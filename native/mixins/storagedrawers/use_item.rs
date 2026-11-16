fn sd_use_item(game: &mut crate::game::Game<'_>, stack: &horizon_client::items::ItemStack, claimed: &mut bool) {
crate::game::sd_integration::use_item(game,stack,claimed);
}
