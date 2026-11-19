//! World updates use the same inventory and transaction as hand and pipe IO.
use crate::{compacting, drops, engine as sd, framing, interaction, menu, network, state};
use mod_api::*;
use std::{cell::RefCell, collections::{BTreeMap, BTreeSet}};

type Address = (String, [i32; 3]);

thread_local! {
    static CONTACTS: RefCell<BTreeMap<Address, BTreeSet<u32>>> = const { RefCell::new(BTreeMap::new()) };
    static LIGHT: RefCell<BTreeMap<Address, bool>> = const { RefCell::new(BTreeMap::new()) };
    static SUCTION_TICKS: RefCell<BTreeMap<Address, (BlockEntityRef, i64)>> = const { RefCell::new(BTreeMap::new()) };
}

pub fn clear() {
    CONTACTS.with_borrow_mut(BTreeMap::clear);
    LIGHT.with_borrow_mut(BTreeMap::clear);
    SUCTION_TICKS.with_borrow_mut(BTreeMap::clear);
}

pub fn contact(level: &dyn LevelHost, event: &EntityInsideEvent<'_>) {
    if event.kind != "minecraft:item" || !level.state_block(event.state).is_some_and(state::is_drawer) {
        return;
    }
    let pos = [event.pos.x, event.pos.y, event.pos.z];
    // The collision callback's level view has no block-entity read access.
    if level.state_with_property(event.state, "hopper", "true") == Some(event.state)
        && let Ok(id) = u32::try_from(event.entity)
    {
        CONTACTS.with_borrow_mut(|contacts| {
            contacts.entry((event.dimension.into(), pos)).or_default().insert(id);
        });
    }
}

fn collect(host: &mut dyn ServerHost, at: BlockSide, block: &str, item: ItemEntityRef) -> bool {
    let Some(input) = item.stack(host) else { return false };
    let Some((entity, nbt, mut group)) = state::read(host, at) else { return false };
    if !compacting::seed(host, at, block, &mut group, &input, sd::Access::Automation) {
        return false;
    }
    let targets: Vec<_> = (0..group.drawers.len()).collect();
    let count = state::insert(host, &mut group, &targets, &input, input.count().into(), sd::Access::Automation).accepted() as u32;
    if count == 0 { return false; }

    let result: Result<(), ()> = host.transaction(|host, _| {
        if !state::write(host, entity, nbt, &group) { return Err(()); }
        let replaced = match input.shrink(count) {
            Some(remainder) => matches!(item.set_stack(host, remainder), Ok(true)),
            None => item.entity().discard(host),
        };
        if replaced { Ok(()) } else { Err(()) }
    });
    result.is_ok()
}

fn collect_contacts(host: &mut dyn ServerHost, level: LevelRef) {
    let contacts = CONTACTS.with_borrow_mut(|contacts| {
        let keys: Vec<_> = contacts.keys().filter(|(dim, _)| dim == level.dimension_id()).cloned().collect();
        keys.into_iter().filter_map(|key| contacts.remove(&key).map(|ids| (key.1, ids))).collect::<Vec<_>>()
    });
    for (pos, ids) in contacts {
        let at = BlockSide { level, pos, side: None };
        let Some(block) = state::block(host, at) else { continue };
        let active = state::read(host, at).is_some_and(|(_, _, group)| {
            group.has(sd::Upgrade::Hopper) && !group.attributes.suspended
        });
        if !active { continue; }
        let [min, max] = hopper_bounds(pos);
        for id in ids {
            let Some(entity) = host.entity(id) else { continue };
            let Some([lo, hi]) = entity.bounding_box(host) else { continue };
            if (0..3).all(|axis| lo[axis] < max[axis] && hi[axis] > min[axis])
                && let Some(item) = host.item_entity(entity)
            {
                collect(host, at, &block, item);
            }
        }
    }
}

fn hopper_bounds(pos: [i32; 3]) -> [[f64; 3]; 2] {
    let [x, y, z] = pos.map(f64::from);
    [[x, y + 11. / 16., z], [x + 1., y + 2., z + 1.]]
}

fn hopper_blocked(host: &dyn Host, at: BlockSide) -> bool {
    let above = [at.pos[0], at.pos[1] + 1, at.pos[2]];
    let [x, y, z] = above.map(f64::from);
    let cube = [x, y, z, x + 1., y + 1., z + 1.];
    at.level.block(host, above).is_some_and(|key| {
        host.block_in_tag(key, "minecraft:does_not_block_hoppers") != Some(true)
            && at.level.collision_boxes(host, cube).is_some_and(|boxes| boxes.contains(&cube))
    })
}

fn suction_tick(host: &mut dyn ServerHost, at: BlockSide, block: &str, entity: BlockEntityRef, group: &sd::Group, now: i64) {
    use sd::Upgrade::{Magnet1, Magnet2, Magnet3};
    let magnet = [Magnet1, Magnet2, Magnet3].into_iter().any(|u| group.has(u));
    let address = (at.level.dimension_id().to_owned(), at.pos);
    if !magnet && !group.has(sd::Upgrade::Hopper) {
        SUCTION_TICKS.with_borrow_mut(|ticks| ticks.remove(&address));
        return;
    }
    if at.level.should_tick_blocks_at(host, at.pos) != Some(true) { return; }
    let due = SUCTION_TICKS.with_borrow_mut(|ticks| {
        let tick = ticks.entry(address.clone()).or_insert((entity, now.saturating_add(1)));
        if tick.0 != entity { *tick = (entity, now.saturating_add(1)); }
        tick.1
    });
    if now < due { return; }

    let collected = if group.attributes.suspended || !magnet && hopper_blocked(host, at) {
        false
    } else {
        let [min, max] = if magnet {
            let [horizontal, up, down] = group.magnet_range().map(f64::from);
            let [x, y, z] = at.pos.map(f64::from);
            [[x - horizontal, y - down, z - horizontal], [x + 1. + horizontal, y + 1. + up, z + 1. + horizontal]]
        } else {
            // Items at rest can miss Horizon's entity-inside sweep.
            hopper_bounds(at.pos)
        };
        at.level.entities_in(host, min, max).unwrap_or_default().into_iter().any(|entity| {
            host.item_entity(entity).is_some_and(|item| collect(host, at, block, item))
        })
    };
    let interval = if !magnet {
        20
    } else if collected {
        group.magnet_interval()
    } else {
        let jitter = (now as u64).wrapping_mul(6364136223846793005)
            .wrapping_add(at.pos[0] as u64).wrapping_add(at.pos[2] as u64) % 5;
        group.magnet_idle_interval() + jitter
    };
    SUCTION_TICKS.with_borrow_mut(|ticks| {
        ticks.insert(address, (entity, now.saturating_add(interval as i64)));
    });
}

pub fn tick(host: &mut dyn ServerHost, level: LevelRef) {
    let Some(now) = level.game_time(host) else { return };
    let dim = level.dimension_id();
    drops::settle(host, level);
    collect_contacts(host, level);
    if now % 100 == 0 {
        state::prune(host, level);
        LIGHT.with_borrow_mut(|light| light.retain(|(d, p), _| d != dim || level.is_loaded(host, *p)));
        SUCTION_TICKS.with_borrow_mut(|ticks| ticks.retain(|(d, _), (entity, _)| {
            d != dim || entity.position(host).is_some()
        }));
    }
    let mut balances = false;
    for (dimension, pos, entity) in state::addresses() {
        if dimension != dim || entity.level(host) != Some(level) { continue; }
        let Some(nbt) = entity.data(host) else { continue };
        let Some(Tag::String(id)) = nbt.get(&"sd_block".into()) else { continue };
        let block = id.to_string_lossy();
        if !state::is_drawer(&block) { continue; }
        let Some(group) = state::group(&nbt) else { continue };
        balances |= group.has(sd::Upgrade::Balance);
        let at = BlockSide { level, pos, side: None };
        let has_hopper = group.has(sd::Upgrade::Hopper);
        let hopper = if has_hopper { "true" } else { "false" };
        if level.block_property(host, pos, "hopper").is_some_and(|value| value != hopper)
            && let Some(current) = level.block_state(host, pos)
            && let Ok(next) = current.with(&host.block_states(), "hopper", hopper)
        {
            let _ = level.set_block(host, pos, next, BlockUpdateFlags::CLIENTS | BlockUpdateFlags::NEIGHBORS);
        }
        let enabled = group.has(sd::Upgrade::Illumination);
        let changed = LIGHT.with_borrow_mut(|light| light.insert((dimension, pos), enabled) != Some(enabled));
        if changed { level.check_light(host, pos); }
        if now % 20 == 0 {
            let (positions, upgrades) = state::upgrade_state(&nbt, &group);
            let validated: Vec<_> = upgrades.iter().map(|stack| interaction::validate_remote(host, at, stack)).collect();
            if validated != upgrades {
                let mut nbt = nbt.clone();
                if state::save_upgrades(&mut nbt, &group, &positions, &validated) {
                    state::write(host, entity, nbt, &group);
                }
            }
        }
        suction_tick(host, at, &block, entity, &group, now);
    }
    if balances {
        let mut network = network::snapshot(host, level);
        network.rebalance_all(sd::Access::Automation);
        let _ = network::commit(host, level, &network);
    }
    if now % 20 == 0 { interaction::validate_inventory_bindings(host, level); }
    menu::refresh(host, level);
    framing::refresh(host, level);
}
