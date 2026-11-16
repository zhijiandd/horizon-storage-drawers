fn sd_predict_drawer_use(game: &crate::game::Game<'_>, hit: &crate::game::Hit, held: Option<&horizon_client::items::ItemStack>) -> std::ops::ControlFlow<(InteractionResult,DryUse)> {
let sneaking=game.remote_shift_down();
if crate::game::sd_integration::predict_drawer_use(game,hit,held,sneaking){
    return std::ops::ControlFlow::Break((InteractionResult::Consume,DryUse::default()));
}
std::ops::ControlFlow::Continue(())
}
