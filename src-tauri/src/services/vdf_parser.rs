use regex::Regex;
use std::collections::HashMap;

const SEAN_WHO_XOR_KEY: &[u8] = b"Scalping dogs, I'll fuck you";

/// Parse a Key.vdf file content into a depot-key map.
///
/// # Arguments
/// * `vdf_content` - The VDF file content as string
/// * `repo` - Optional repo name to handle special decryption (sean-who uses XOR)
///
/// # Returns
/// HashMap of depot_id (String) -> depot_key (hex String)
pub fn parse_key_vdf(vdf_content: &str, repo: Option<&str>) -> HashMap<String, String> {
    let mut result = HashMap::new();

    // Regex to match depot blocks with DecryptionKey
    // Matches patterns like:
    //   "1995891"
    //   {
    //       "DecryptionKey" "hexvalue"
    //   }
    let depot_block_re =
        Regex::new(r#"(?si)"(\d+)"\s*\{[^}]*"DecryptionKey"\s+"([^"]+)"[^}]*\}"#).unwrap();

    for cap in depot_block_re.captures_iter(vdf_content) {
        let depot_id = cap[1].to_string();
        let mut depot_key = cap[2].to_string();

        // sean-who/ManifestAutoUpdate uses XOR encryption on depot keys
        if let Some(r) = repo {
            if r.contains("sean-who") {
                depot_key = xor_decrypt_hex(&depot_key, SEAN_WHO_XOR_KEY);
            }
        }

        result.insert(depot_id, depot_key);
    }

    result
}

/// XOR decrypt a hex-encoded key using a repeating XOR key.
/// The hex string is first converted to bytes, XOR'd, then converted back to hex.
pub fn xor_decrypt_hex(hex_string: &str, xor_key: &[u8]) -> String {
    let bytes = match hex_decode(hex_string) {
        Some(b) => b,
        None => return hex_string.to_string(),
    };

    let result: Vec<u8> = bytes
        .iter()
        .enumerate()
        .map(|(i, &b)| b ^ xor_key[i % xor_key.len()])
        .collect();

    hex_encode(&result)
}

fn hex_decode(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}
