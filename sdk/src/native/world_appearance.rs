fn appearance(
    game: &super::Game<'_>,
    pos: [i32; 3],
    block: &str,
    entity: &crate::containers::ModBlockEntity,
    draw: &mut super::native_block_entity_draw::BlockEntityDraw,
) {
    crate::game::native_block_entity_draw::sd_labels::render(game, pos, block, entity, draw);
}
