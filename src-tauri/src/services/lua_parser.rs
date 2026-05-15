use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::OnceLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepotInfo {
    pub depot_id: u64,
    pub depot_key: Option<String>,
    pub manifest_id: Option<String>,
    pub size_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LuaParseResult {
    pub main_app_id: Option<u64>,
    pub depots: Vec<DepotInfo>,
}

// Matches `addappid(id)` (main app) or `addappid(id, 0, "hexKey")` (depot with key).
fn add_app_id_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(r#"(?i)addappid\((\d+)(?:\s*,\s*(\d+)\s*,\s*"([a-f0-9]+)")?\)"#)
            .expect("addappid pattern is a valid regex")
    })
}

// Matches `setManifestid(depotId, "manifestId")` and the Hubcap variant
// `setManifestid(depotId, "manifestId", sizeBytes)`.
fn set_manifest_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(r#"(?i)setManifestid\((\d+)\s*,\s*"(\d+)"(?:\s*,\s*(\d+))?[^)]*\)"#)
            .expect("setManifestid pattern is a valid regex")
    })
}

/// Parse `.lua` file content, extracting `addappid()` and `setManifestid()` calls.
///
/// Returns `Err` when no app id and no depot entries were found. In that case the
/// file is either not in the expected format or its contents are corrupted.
pub fn parse_lua_file(content: &str) -> Result<LuaParseResult, String> {
    let mut result = LuaParseResult {
        main_app_id: None,
        depots: Vec::new(),
    };

    let mut depot_map: HashMap<u64, DepotInfo> = HashMap::new();

    for cap in add_app_id_pattern().captures_iter(content) {
        let id: u64 = match cap[1].parse() {
            Ok(n) => n,
            Err(_) => continue,
        };
        let has_key = cap.get(3).is_some();

        if !has_key {
            if result.main_app_id.is_none() {
                result.main_app_id = Some(id);
            }
        } else {
            let depot_key = cap[3].to_string();
            depot_map
                .entry(id)
                .and_modify(|d| d.depot_key = Some(depot_key.clone()))
                .or_insert(DepotInfo {
                    depot_id: id,
                    depot_key: Some(depot_key),
                    manifest_id: None,
                    size_bytes: None,
                });
        }
    }

    for cap in set_manifest_pattern().captures_iter(content) {
        let depot_id: u64 = match cap[1].parse() {
            Ok(n) => n,
            Err(_) => continue,
        };
        let manifest_id = cap[2].to_string();
        let size_bytes = cap.get(3).and_then(|m| m.as_str().parse::<u64>().ok());

        depot_map
            .entry(depot_id)
            .and_modify(|d| {
                d.manifest_id = Some(manifest_id.clone());
                if size_bytes.is_some() {
                    d.size_bytes = size_bytes;
                }
            })
            .or_insert(DepotInfo {
                depot_id,
                depot_key: None,
                manifest_id: Some(manifest_id),
                size_bytes,
            });
    }

    result.depots = depot_map.into_values().collect();

    // Fallback: if the script has no explicit main app id, use the smallest depot id.
    if result.main_app_id.is_none() && !result.depots.is_empty() {
        result.main_app_id = result.depots.iter().map(|d| d.depot_id).min();
    }

    if result.main_app_id.is_none() && result.depots.is_empty() {
        return Err(
            "The file does not contain any recognised entries. Expected at least one addappid(...) or setManifestid(...) call."
                .to_string(),
        );
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_classic_two_arg_setmanifestid() {
        let lua = r#"addappid(2835590)
addappid(2835591,1,"2FFB82F62CC851ACCC98FCF3FC0E60C0304099E14D3FF50AE18575AC22C50372")
setManifestid(2835591,"433998940059644752")
"#;
        let result = parse_lua_file(lua).unwrap();
        assert_eq!(result.main_app_id, Some(2835590));
        let depot = result.depots.iter().find(|d| d.depot_id == 2835591).unwrap();
        assert_eq!(depot.manifest_id.as_deref(), Some("433998940059644752"));
        assert!(depot.size_bytes.is_none());
        assert!(depot.depot_key.is_some());
    }

    #[test]
    fn parses_hubcap_three_arg_setmanifestid_with_size() {
        let lua = r#"addappid(1392860, 1, "8629cb2029941f570bf6fa875b80ed70fb66f92674c8f43d25937825732ae4a2")
addappid(1392863, 1, "2a40e8e4f16a349cec1ad4cdd732e6a88afb1d0676c8de3061c87c7d815343d0")
setManifestid(1392863, "4398981003384795747", 13789896785)
"#;
        let result = parse_lua_file(lua).unwrap();
        let depot = result.depots.iter().find(|d| d.depot_id == 1392863).unwrap();
        assert_eq!(depot.manifest_id.as_deref(), Some("4398981003384795747"));
        assert_eq!(depot.size_bytes, Some(13789896785));
        assert_eq!(
            depot.depot_key.as_deref(),
            Some("2a40e8e4f16a349cec1ad4cdd732e6a88afb1d0676c8de3061c87c7d815343d0")
        );
    }
}