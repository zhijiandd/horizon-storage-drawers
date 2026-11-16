fn sd_io_can_place(game: &crate::game::Game<'_>, pos: [i32;3], slot: usize, stack: &horizon_client::items::ItemStack) -> std::ops::ControlFlow<bool> {
if let Some(handler)=crate::game::sd_integration::item_handler(game,pos){
    return std::ops::ControlFlow::Break(game.native_on("storagedrawers")&&handler.simulate_insert(game,slot,stack).count<stack.count);
}
std::ops::ControlFlow::Continue(())
}
