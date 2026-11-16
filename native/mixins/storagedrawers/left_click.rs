fn sd_left_click(game: &mut crate::game::Game<'_>, pos: [i32;3], face: [i32;3], cancel: &mut bool) {
crate::game::sd_integration::left_click(game,pos,face,cancel);
}
