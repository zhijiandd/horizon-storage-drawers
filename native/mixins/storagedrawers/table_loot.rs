fn sd_table_loot(game: &crate::game::Game<'_>, context: &horizon_client::mod_api::NativeLootDrops<'_>, drops: &mut Vec<horizon_client::items::ItemStack>) {
crate::game::sd_framing::loot(game,context,drops);
}
