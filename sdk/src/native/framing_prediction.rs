fn framing_prediction(placed: Option<DryPlace>, game: &Game<'_>, stack: &ItemStack) -> Option<DryPlace> {
    if &*stack.item != "storagedrawers:framing_table" { return placed; }
    let (pos, mut changes) = placed?;
    let (facing, [x, y, z]) = storagedrawers::table_placement([pos.x, pos.y, pos.z], game.yaw);
    if !game.world.is_loaded_at(x, y, z)
        || !game.border.geometry().is_within_bounds(f64::from(x), f64::from(z), 0.)
        || !game.res.facts().get(game.world.block(x, y, z)).replaceable
    { return None; }

    let state = changes.iter_mut().find(|(at, _)| *at == pos).map(|(_, state)| state)?;
    let right = game.with_props(*state, &[("facing", facing), ("part", "right")])?;
    let left = game.with_props(right, &[("part", "left")])?;
    *state = right;
    changes.push((horizon_core::position::BlockPos::new(x, y, z), left));
    Some((pos, changes))
}
