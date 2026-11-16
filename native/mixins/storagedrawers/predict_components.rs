fn sd_predict_components(value: (), game: &mut crate::game::Game<'_>, pos: [i32;3], stack: &horizon_client::items::ItemStack) -> () {
crate::game::sd_integration::predicted_item(game,pos,stack);value
}
