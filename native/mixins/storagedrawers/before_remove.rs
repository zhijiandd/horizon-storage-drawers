fn sd_before_remove(level: &mut crate::game::GameLevel<'_>, pos: horizon_core::position::BlockPos, old: horizon_world::registry::BlockStateId) {
crate::game::sd_integration::before_remove(level,pos,old);
}
