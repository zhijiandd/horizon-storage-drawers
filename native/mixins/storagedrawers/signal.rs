fn sd_signal(level: &crate::game::GameLevel<'_>, pos: horizon_core::position::BlockPos, face: horizon_core::geometry::Direction, direct: bool, signal: &mut i32) {
crate::game::sd_integration::signal(level,[pos.x,pos.y,pos.z],face,direct,signal);
}
