fn sd_predict_break(game: &mut crate::game::Game<'_>, pos: [i32;3]) -> std::ops::ControlFlow<()> {
if crate::game::sd_integration::suppress_break_prediction(game,pos){return std::ops::ControlFlow::Break(());}
std::ops::ControlFlow::Continue(())
}
