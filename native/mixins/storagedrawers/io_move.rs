fn sd_io_move(game: &mut crate::game::Game<'_>, dimension: &str, from: [i32;3], to: [i32;3], to_face: horizon_core::geometry::Direction, item: Option<&str>, max: u32) -> std::ops::ControlFlow<u32> {
if dimension==game.native_dimension(){if let Some(moved)=crate::game::sd_integration::io_move(game,from,to,to_face,item,max){return std::ops::ControlFlow::Break(moved);}}
std::ops::ControlFlow::Continue(())
}
