fn sd_placed(game: &mut crate::game::Game<'_>, stack: &horizon_client::items::ItemStack, pos: [i32;3]) {
crate::game::sd_integration::placed(game,stack,pos);
}
