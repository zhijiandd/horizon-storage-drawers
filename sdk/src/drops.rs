//! Saved drawer data follows the normal block drop. Display slots never spill.
use crate::{compat, engine as sd, framing, state};
use mod_api::*;
use std::{cell::RefCell, collections::{BTreeMap, BTreeSet}};

type Address = (String, [i32; 3]);
const TABLE: &str = "storagedrawers:framing_table";

thread_local! {
    static REMOVING: RefCell<BTreeMap<[i32; 3], (Address, String)>> = const { RefCell::new(BTreeMap::new()) };
    static PACKED: RefCell<BTreeMap<Address, (String, Compound)>> = const { RefCell::new(BTreeMap::new()) };
    static TABLE_PAIRS: RefCell<BTreeMap<Address, Option<framing::Partner>>> = const { RefCell::new(BTreeMap::new()) };
    static TABLE_DROPPED: RefCell<BTreeSet<Address>> = const { RefCell::new(BTreeSet::new()) };
}

pub fn clear() {
    REMOVING.with_borrow_mut(BTreeMap::clear);
    PACKED.with_borrow_mut(BTreeMap::clear);
    TABLE_PAIRS.with_borrow_mut(BTreeMap::clear);
    TABLE_DROPPED.with_borrow_mut(BTreeSet::clear);
}

pub fn changed(level: &dyn LevelHost, event: &BlockChangedEvent) {
    let Some(old) = level.state_block(event.old) else { return };
    let new = level.state_block(event.new);
    if new == Some(old) { return; }
    let pos = [event.pos.x, event.pos.y, event.pos.z];
    let address = (event.dimension.key().id().to_owned(), pos);
    PACKED.with_borrow_mut(|map| { map.remove(&address); });
    if old == TABLE {
        let partner = framing::partner(level, pos, event.old);
        TABLE_PAIRS.with_borrow_mut(|pairs| { pairs.insert(address.clone(), partner); });
    } else if new == Some(TABLE) {
        TABLE_DROPPED.with_borrow_mut(|pairs| { pairs.remove(&address); });
    }
    if old.starts_with("storagedrawers:") {
        REMOVING.with_borrow_mut(|map| { map.insert(pos, (address, old.to_owned())); });
    }
}

pub fn pre_remove(event: &PreRemoveSideEffectsEvent<'_>) {
    let pos = [event.pos.x, event.pos.y, event.pos.z];
    let Some((address, block)) = REMOVING.with_borrow_mut(|map| map.remove(&pos)) else { return };
    let nbt = &event.entity.nbt;
    if event.entity.id() != block { return; }
    if block == TABLE { return; }
    let patch = packed(nbt, None);
    if !patch.is_empty() {
        PACKED.with_borrow_mut(|map| { map.insert(address, (block, patch)); });
    }
}

fn packed(nbt: &Compound, mode: Option<&str>) -> Compound {
    let mut patch = match nbt.get(&"sd_item_components".into()) {
        Some(Tag::Compound(data)) => data.clone(),
        _ => Compound::new(),
    };
    let mut custom = match patch.remove(&"minecraft:custom_data".into()) {
        Some(Tag::Compound(data)) => data,
        _ => Compound::new(),
    };
    for key in ["sd_contents", "sd_upgrade_slots", "sd_upgrade_items", "sd_remote", "sd_frame"] {
        custom.remove(&key.into());
    }
    if let Some(frame) = nbt.get(&"sd_frame".into()) {
        custom.insert("sd_frame".into(), frame.clone());
    }
    if let Some(group) = state::group(nbt) {
        let config = sd::config::features();
        let mode = mode.or_else(|| config["drop_mode"].as_str()).unwrap_or("KEEP");
        patch.remove(&"minecraft:max_stack_size".into());
        let has_inventory = group.has_inventory_data();
        let packed = has_inventory || !group.upgrades.is_empty() || group.frame.is_some()
            || group.attributes != sd::Attributes::default();
        if packed && !matches!(mode, "DROP" | "VOID") {
            if let Some(bytes) = nbt.get(&state::SAVE.into()) {
                custom.insert("sd_contents".into(), bytes.clone());
            }
            for key in ["sd_upgrade_slots", "sd_upgrade_items", "sd_remote"] {
                if let Some(value) = nbt.get(&key.into()) {
                    custom.insert(key.into(), value.clone());
                }
            }
            if has_inventory {
                patch.insert("minecraft:max_stack_size".into(), Tag::Int(1));
            }
        }
    }
    if !custom.is_empty() {
        patch.insert("minecraft:custom_data".into(), Tag::Compound(custom));
    }
    patch
}

pub fn block_drops(level: &dyn LevelHost, event: &mut BlockDropsEvent<'_>) {
    let Some(block) = level.state_block(event.state) else { return };
    if !block.starts_with("storagedrawers:") { return; }
    let dim = event.dimension.key().id();
    let pos = [event.pos.x, event.pos.y, event.pos.z];
    let address = (dim.to_owned(), pos);
    let Some(item) = crate::compat::registered_item(block) else { return };
    if block == TABLE {
        let canonical = TABLE_PAIRS.with_borrow(|pairs| {
            pairs.get(&address).and_then(Option::as_ref).map_or(pos, |p| pos.min(p.pos))
        });
        if event.drops.iter().any(|drop| drop.stack.item() == item) {
            let first = TABLE_DROPPED.with_borrow_mut(|seen| seen.insert((dim.into(), canonical)));
            if !first { event.drops.retain(|drop| drop.stack.item() != item); }
        }
        return;
    }
    let queued = PACKED.with_borrow_mut(|map| map.remove(&address)).filter(|(id, _)| id == block);
    let patch = queued.map(|(_, patch)| patch).or_else(|| {
        event.block_entity.and_then(|entity| level.block_entity_data(entity)).map(|nbt| packed(&nbt, None))
    });
    let Some(patch) = patch.filter(|p| !p.is_empty()) else { return };
    if let Some(body) = event.drops.iter_mut().find(|drop| drop.stack.item() == item) {
        let mut components = body.stack.components().map(|c| c.as_nbt().clone()).unwrap_or_default();
        components.extend(patch);
        let _ = body.stack.set_components(components);
    }
}

pub fn pick(host: &dyn Host, event: &CloneItemStackEvent<'_>) -> Option<ItemStack> {
    if event.block == TABLE || !event.block.starts_with("storagedrawers:") { return None; }
    let level = host.level(event.dimension.key().id())?;
    let pos = [event.pos.x, event.pos.y, event.pos.z];
    let nbt = host.block_entity(level, pos)?.data(host)?;
    let key = crate::compat::registered_item(event.block)?;
    let mut stack = ItemStack::of(key);
    let patch = if event.include_data {
        packed(&nbt, Some("KEEP"))
    } else {
        let frame = nbt.get(&"sd_frame".into())?.clone();
        Compound::from([("minecraft:custom_data".into(), Tag::compound([("sd_frame", frame)]))])
    };
    stack.set_components(patch).ok()?;
    Some(stack)
}

pub fn spill_contents(nbt: &Compound) -> (Vec<Tag>, Vec<sd::Stack>) {
    if sd::config::features()["drop_mode"].as_str() != Some("DROP") { return Default::default(); }
    let Some(mut group) = state::group(nbt) else { return Default::default(); };
    let (_, upgrades) = state::upgrade_state(nbt, &group);
    let upgrades = upgrades.iter().filter(|s| crate::items::upgrade(s) != Some(sd::Upgrade::CreativeVending))
        .map(compat::item_to_nbt).collect();
    let mut stock = Vec::new();
    if group.has(sd::Upgrade::CreativeVending) { return (upgrades, stock); }
    group.attributes.suspended = true;
    let mut remaining = sd::config::uint("drop_stack_limit", 32, 0, 65536);
    while remaining > 0 {
        let before = remaining;
        for slot in 0..group.drawers.len() {
            if remaining == 0 { break; }
            let amount = group.item(slot).map_or(0, |key| u64::from(key.max_stack));
            if let Ok(Some(out)) = group.extract(slot, amount, sd::Access::Administrator, false) {
                stock.push(out);
                remaining -= 1;
            }
        }
        if before == remaining { break; }
    }
    (upgrades, stock)
}

pub fn settle(host: &mut dyn ServerHost, level: LevelRef) {
    let dim = level.dimension_id();
    let pairs = TABLE_PAIRS.with_borrow_mut(|pairs| {
        let keys: Vec<_> = pairs.keys().filter(|(d, _)| d == dim).cloned().collect();
        keys.into_iter().filter_map(|key| pairs.remove(&key).flatten().map(|p| (key.1, p))).collect::<Vec<_>>()
    });
    for (removed, partner) in pairs {
        if level.block(host, removed).is_some_and(|b| b.id(host) == TABLE) { continue; }
        framing::remove_partner(host, level, partner);
    }
    TABLE_DROPPED.with_borrow_mut(|pairs| pairs.retain(|(d, _)| d != dim));
    PACKED.with_borrow_mut(|map| map.retain(|(d, _), _| d != dim));
    REMOVING.with_borrow_mut(|map| map.retain(|_, ((d, _), _)| d != dim));
}

pub fn player_destroyed(host:&mut dyn ServerHost,event:&PlayerDestroyedBlockEvent){
    if event.state.block().id(host)!=TABLE {return;}
    let Some(level)=host.level(host.dimension())else{return};
    let address=(level.dimension_id().to_owned(),event.pos);
    let partner=TABLE_PAIRS.with_borrow(|pairs|pairs.get(&address).cloned().flatten());
    if let Some(partner)=partner {framing::remove_partner(host,level,partner);}
}
