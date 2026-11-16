fn sd_io_accepts(game: &crate::game::Game<'_>, pos: [i32;3], slot: usize, stack: &horizon_client::items::ItemStack, cancel: &mut bool) {
crate::game::sd_integration::io_accepts(game,pos,slot,stack,cancel);
}
