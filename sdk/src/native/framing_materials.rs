fn framing_materials(game: &Game<'_>) {
    use horizon_world::level::data::state_flags::{CAN_OCCLUDE, FULL_COLLISION};
    let materials = game.items().items.iter().filter_map(|(id, item)| {
        let state = item.block?;
        let block = game.pack.assets.registry.state(state)?.name.as_str();
        let flags = game.res.behaviours().facts(state).flags;
        Some((id.to_string(), (block.to_owned(), flags & FULL_COLLISION != 0, flags & CAN_OCCLUDE != 0)))
    }).collect();
    storagedrawers::framing_materials(materials);
}
