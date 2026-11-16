fn sd_io_bulk_insert(game: &mut crate::game::Game<'_>, pos: [i32;3], face: horizon_core::geometry::Direction, stack: &horizon_client::items::ItemStack) -> std::ops::ControlFlow<Option<ItemStack>> {
if let Some((rest,_))=crate::game::sd_integration::io_insert(game,pos,0,stack,Some(face),false){return std::ops::ControlFlow::Break(rest);}
std::ops::ControlFlow::Continue(())
}
