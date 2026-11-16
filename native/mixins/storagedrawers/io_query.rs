fn sd_io_query(game: &crate::game::Game<'_>, dimension: &str, pos: [i32;3]) -> std::ops::ControlFlow<Option<Vec<NativeContainerSlot>>> {
if game.native_simulates()&&dimension==game.native_dimension(){
    if let Some(slots)=crate::game::sd_integration::io_slots(game,pos){return std::ops::ControlFlow::Break(Some(slots));}
}
std::ops::ControlFlow::Continue(())
}
