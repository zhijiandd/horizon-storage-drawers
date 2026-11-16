fn sd_loot(game: &mut crate::game::Game<'_>, pos: [i32;3], drops: &mut Vec<horizon_client::items::ItemStack>) {
crate::game::sd_integration::loot(game,pos,drops);
}
