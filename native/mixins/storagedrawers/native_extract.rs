fn sd_native_extract(game: &mut crate::game::Game<'_>, dimension: &str, pos: [i32;3], slot: usize, count: u32) -> std::ops::ControlFlow<Option<ItemStack>> {
if dimension==game.native_dimension(){if let Some(out)=crate::game::sd_integration::auto_take(game,pos,slot,count){return std::ops::ControlFlow::Break(out);}}
std::ops::ControlFlow::Continue(())
}
