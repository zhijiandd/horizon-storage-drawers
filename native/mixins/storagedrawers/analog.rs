fn sd_analog(level: &crate::game::GameLevel<'_>, pos: horizon_core::position::BlockPos, output: &mut i32) {
crate::game::sd_integration::analog(level,[pos.x,pos.y,pos.z],output);
}
