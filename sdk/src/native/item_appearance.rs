fn appearance(game: &Game<'_>, stack: &ItemStack, state: ItemRenderState) -> ItemRenderState {
    crate::game::native_block_entity_draw::sd_appearance::item(game, stack, state)
}
