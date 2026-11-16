fn sd_screen_draw(game: &crate::game::Game<'_>, block: &str, batch: &mut horizon_client::hud::HudBatch, origin: [f32;2]) {
crate::game::sd_integration::prepare_menu_rules(game);crate::game::sd_integration::screen_draw(game,block,batch,origin);
}
