use flate2::read::ZlibDecoder;
use std::io::Read;

use crate::services::lua_parser::{parse_lua_file, LuaParseResult};

/// Parse a `.st` binary file buffer.
///
/// Format:
///   Header: 12 bytes = [xorkey (u32 LE), size (u32 LE), xorkeyverify (u32 LE)]
///   xorkey = (xorkey XOR 0xFFFEA4C8) AND 0xFF
///   Data: content[12 .. 12+size], XOR each byte with xorkey
///   Then zlib decompress
///   Then skip first 512 bytes, rest is lua-like content
pub fn parse_st_file(buffer: &[u8]) -> Result<LuaParseResult, String> {
    if buffer.len() < 12 {
        return Err(format!(
            ".st file too small: {} bytes (need at least 12 for header)",
            buffer.len()
        ));
    }

    // Read header (3x uint32 little-endian)
    let xor_key_raw = u32::from_le_bytes(buffer[0..4].try_into().unwrap());
    let size = u32::from_le_bytes(buffer[4..8].try_into().unwrap()) as usize;
    // xor_key_verify at bytes 8..12 not used

    // Derive XOR key
    let xor_key = ((xor_key_raw ^ 0xFFFEA4C8) & 0xFF) as u8;

    // Validate size
    if 12 + size > buffer.len() {
        return Err(format!(
            ".st file data size ({}) exceeds buffer length ({})",
            size,
            buffer.len() - 12
        ));
    }

    // Extract and XOR decrypt data bytes
    let encrypted_data = &buffer[12..12 + size];
    let decrypted_data: Vec<u8> = encrypted_data.iter().map(|b| b ^ xor_key).collect();

    // Zlib decompress
    let mut decoder = ZlibDecoder::new(&decrypted_data[..]);
    let mut decompressed = Vec::new();
    decoder
        .read_to_end(&mut decompressed)
        .map_err(|e| format!("Failed to decompress .st data: {}", e))?;

    // Skip first 512 bytes
    if decompressed.len() <= 512 {
        return Err(format!(
            ".st decompressed data too small: {} bytes (need >512)",
            decompressed.len()
        ));
    }

    let lua_content = String::from_utf8_lossy(&decompressed[512..]).to_string();

    // Parse with lua_parser
    Ok(parse_lua_file(&lua_content))
}
