use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepotInfo {
    pub depot_id: u64,
    pub depot_key: Option<String>,
    pub manifest_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LuaParseResult {
    pub main_app_id: Option<u64>,
    pub depots: Vec<DepotInfo>,
}

/// Parse `.lua` file content, extracting `addappid()` and `setManifestid()` calls.
pub fn parse_lua_file(content: &str) -> LuaParseResult {
    let mut result = LuaParseResult {
        main_app_id: None,
        depots: Vec::new(),
    };

    // Map to collect depot data by depotId
    let mut depot_map: HashMap<u64, DepotInfo> = HashMap::new();

    // Match addappid calls
    // Pattern 1: addappid(appId) — main app, no key
    // Pattern 2: addappid(depotId, 0, "hexKey") — depot with key
    let add_app_id_re =
        Regex::new(r#"(?i)addappid\((\d+)(?:\s*,\s*(\d+)\s*,\s*"([a-f0-9]+)")?\)"#).unwrap();

    for cap in add_app_id_re.captures_iter(content) {
        let id: u64 = cap[1].parse().unwrap_or(0);
        let has_key = cap.get(3).is_some();

        if !has_key {
            // First addappid without a key is the main app ID
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
                });
        }
    }

    // Match setManifestid calls
    // Pattern: setManifestid(depotId, "manifestId")
    let set_manifest_re = Regex::new(r#"(?i)setManifestid\((\d+)\s*,\s*"(\d+)"\)"#).unwrap();

    for cap in set_manifest_re.captures_iter(content) {
        let depot_id: u64 = cap[1].parse().unwrap_or(0);
        let manifest_id = cap[2].to_string();

        depot_map
            .entry(depot_id)
            .and_modify(|d| d.manifest_id = Some(manifest_id.clone()))
            .or_insert(DepotInfo {
                depot_id,
                depot_key: None,
                manifest_id: Some(manifest_id),
            });
    }

    // Convert map to vec
    result.depots = depot_map.into_values().collect();

    // If no mainAppId was found, use the smallest depotId as fallback
    if result.main_app_id.is_none() && !result.depots.is_empty() {
        result.main_app_id = result.depots.iter().map(|d| d.depot_id).min();
    }

    result
}
