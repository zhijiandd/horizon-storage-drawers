fn sd_block_tick(game: &mut crate::game::Game<'_>, pos: [i32;3], entity: &mut horizon_client::containers::ModBlockEntity) {
crate::game::sd_integration::block_tick(game,pos,entity);
}
