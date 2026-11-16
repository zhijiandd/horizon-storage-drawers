fn sd_item_appearance(game: &crate::game::Game<'_>, stack: &horizon_client::items::ItemStack, display: horizon_assets::item_model::DisplayContext, owner: Option<&horizon_assets::item_model::OwnerState>, seed: i32, hand: horizon_client::cit::Hand) -> std::ops::ControlFlow<ItemRenderState> {
if let Some(state)=crate::game::sd_appearance::item(game,stack,display,owner,seed,hand){return std::ops::ControlFlow::Break(state);}
std::ops::ControlFlow::Continue(())
}
