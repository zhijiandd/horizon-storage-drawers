fn predicted_components(game: &mut Game<'_>, pos: [i32; 3], stack: &ItemStack, result: ()) {
    if !stack.item.starts_with("storagedrawers:") { return result; }
    let Some(data) = stack.data().and_then(|data| data.other("minecraft:custom_data")) else {
        return result;
    };
    let key = (pos[0], pos[1], pos[2]);
    let Some(BlockEntity::Mod(entity)) = game.world.block_entities.get_mut(&key) else {
        return result;
    };
    if &*stack.item != entity.id() { return result; }
    for (item, saved) in [
        ("sd_frame", "sd_frame"), ("sd_contents", "sd_state"),
        ("sd_upgrade_slots", "sd_upgrade_slots"), ("sd_upgrade_items", "sd_upgrade_items"),
    ] {
        if let Some(value) = data.get(item) {
            entity.nbt.insert(saved.into(), value.clone());
        }
    }
    result
}
