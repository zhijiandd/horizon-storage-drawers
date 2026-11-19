fn drawer_break_prediction(game: &Game<'_>, pos: [i32; 3]) -> core::ops::ControlFlow<()> {
    use mod_api::Host;

    if game.creative
        && let Some(hit) = game.pick().filter(|hit| hit.pos == pos)
        && let Some(level) = game.level(game.dimension())
        && storagedrawers::drawer_panel(
            game,
            mod_api::BlockSide { level, pos, side: None },
            hit.face,
            hit.point,
        ).is_some()
    {
        // remote_mining still sends START. Only the local block removal is skipped.
        return core::ops::ControlFlow::Break(());
    }
    core::ops::ControlFlow::Continue(())
}
