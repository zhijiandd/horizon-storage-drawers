//! One configuration source for the storage core, recipes and game adapter.
//! Read once per process; changing server rules requires a restart.
use serde_json::{Value, json};
use std::{path::Path, sync::OnceLock};

pub const PATH: &str = "horizon-mods/.horizon-mod-data/storagedrawers/features.json";

pub fn defaults() -> Value {
    let mut tags = Vec::new();
    for kind in ["storage_blocks", "ingots", "nuggets"] {
        for material in ["aluminum", "constantan", "steel", "uranium", "invar", "tin", "lead",
            "silver", "platinum", "nickel", "osmium", "bronze", "electrum"] {
            tags.push(format!("c:{kind}/{material}"));
        }
    }
    json!({
        "conversion_tag_allowlist":tags,"conversion_tag_denylist":[],"conversion_equivalence_groups":[],
        "controller_range":50,"remote_range":0,"remote_group_range":0,
        "framing_solid_materials":true,"framing_opaque_materials":false,"framing_material_denylist":[],
        "detached_force_capacity_check":false,"detached_heavy":false,"redstone_analog_output":true,
        "store_denylist":["storagedrawers:creative_vending_upgrade"],
        "base_stack_storage":1,"full_drawer_units":[32,16,8],"half_drawer_units":[16,8,4],
        "compacting_units":32,"half_compacting_units":16,"detached_units":32,
        "storage_multipliers":[2,4,8,16,64,256,2048],
        "compacting_enabled":true,"extra_compacting_rules":true,"compacting_rules":[],
        "magnet_additive_range":true,"magnet_max_range":[24,8,0],
        "magnet_ranges":[[1,1,0],[4,2,0],[8,3,0]],
        "magnet_active_ticks":[20,10,5],"magnet_idle_ticks":[20,20,20],
        "drop_mode":"KEEP","drop_stack_limit":32,
        "label_render_distance":20.0,"quantity_render_distance":10.0,"quantity_fade_distance":20.0,
        "illumination_level":13,"minimum_illumination_level":1,"framed_translucent_materials":true
    })
}

pub fn merged(user: Value) -> Value {
    let mut result = defaults();
    if let Value::Object(values) = user {
        for (key, value) in values { result[&key] = value; }
    }
    result
}

pub fn features() -> &'static Value {
    static FEATURES: OnceLock<Value> = OnceLock::new();
    FEATURES.get_or_init(|| {
        let user = std::fs::read(PATH).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null);
        merged(user)
    })
}

/// The game adapter materializes new defaults, preserving existing overrides.
/// A malformed file is never overwritten.
pub fn write_defaults() {
    let path = Path::new(PATH);
    if path.exists() && std::fs::read(path).ok().and_then(|b| serde_json::from_slice::<Value>(&b).ok())
        .is_none_or(|v| !v.is_object()) { return; }
    if let Some(parent) = path.parent() {
        if std::fs::create_dir_all(parent).is_ok() {
            if let Ok(bytes) = serde_json::to_vec_pretty(features()) {
                if std::fs::read(path).ok().as_deref() != Some(bytes.as_slice()) { let _ = std::fs::write(path, bytes); }
            }
        }
    }
}

pub fn number(value: &Value, default: u64, min: u64, max: u64) -> u64 {
    value.as_u64().unwrap_or(default).clamp(min, max)
}
pub fn uint(key: &str, default: u64, min: u64, max: u64) -> u64 { number(&features()[key], default, min, max) }
pub fn boolean(key: &str, default: bool) -> bool { features()[key].as_bool().unwrap_or(default) }
pub fn distance(key: &str, default: f64) -> f64 {
    features()[key].as_f64().filter(|v| v.is_finite() && *v >= 0.).unwrap_or(default)
}
pub fn denied_by(value: &Value, id: &str) -> bool {
    value.as_array().is_some_and(|list| list.iter().filter_map(Value::as_str).any(|entry| {
        if entry.contains(':') { entry == id } else { id.split_once(':').is_some_and(|(ns, _)| ns == entry) }
    }))
}
pub fn can_store(id: &str) -> bool { !denied_by(&features()["store_denylist"], id) }
pub fn detached_stacks() -> u64 {
    uint("base_stack_storage",1,1,65536).saturating_mul(uint("detached_units",32,1,65536))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn defaults_and_existing_overrides_merge_without_losing_new_rules() {
        let v = merged(json!({"controller_range":72,"extension_setting":true}));
        assert_eq!(v["controller_range"],72); assert_eq!(v["extension_setting"],true);
        assert_eq!(v["store_denylist"][0],"storagedrawers:creative_vending_upgrade");
    }
    #[test] fn storage_blacklist_accepts_both_namespace_and_exact_item_entries() {
        let deny=json!(["blockedmod","minecraft:stone"]);
        assert!(denied_by(&deny,"blockedmod:anything")); assert!(denied_by(&deny,"minecraft:stone"));
        assert!(!denied_by(&deny,"minecraft:stone_slab"));
    }
}
