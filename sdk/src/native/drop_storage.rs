fn drop_storage(entity: &crate::containers::ModBlockEntity, drops: &mut Vec<EngineStack>) {
    if !entity.id().starts_with("storagedrawers:") { return; }
    let (upgrades, stock) = storagedrawers::spill_contents(&entity.nbt);
    drops.extend(upgrades.iter().filter_map(horizon_items::item_nbt::item_from_nbt));
    drops.extend(stock.iter().filter_map(|stack|
        crate::game::native_block_entity_draw::sd_labels::stack(&stack.item, stack.count)
    ));
}
