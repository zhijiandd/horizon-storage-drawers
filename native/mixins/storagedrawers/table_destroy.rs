fn sd_table_destroy(game: &mut crate::game::Game<'_>, pos: [i32;3], canceled: &mut bool) {
crate::game::sd_framing::destroy(game,pos,canceled);
}
