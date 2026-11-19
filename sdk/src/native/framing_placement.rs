fn framing_placement(
    level: &GameLevel<'_>,
    state: BlockStateId,
    context: &PlacementContext,
    answer: &mut Option<Option<BlockStateId>>,
) -> core::ops::ControlFlow<()> {
    if !level.native_authoritative || level.native_state_block(state) != Some("storagedrawers:framing_table") {
        return core::ops::ControlFlow::Continue(());
    }
    let (facing, [x, y, z]) = storagedrawers::table_placement(
        [context.pos.x, context.pos.y, context.pos.z], context.yaw,
    );
    let can_place_left = level.world.is_loaded_at(x, y, z)
        && level.within_world_border(BlockPos::new(x, y, z))
        && level.facts.get(level.world.block(x, y, z)).replaceable;
    *answer = Some(can_place_left.then(|| level.native_state_with_property(state, "facing", facing)).flatten());
    core::ops::ControlFlow::Break(())
}
