fn sd_menu_clone_limit(view: &mut crate::lan::menu::View<'_>, drag: &mut Option<(u8,Vec<usize>)>, slot: i32, mode: i32) -> std::ops::ControlFlow<(Vec<ItemStack>,Vec<usize>)> {
if mode==crate::lan::menu::input::CLONE&&view.guest.creative()&&view.guest.carried.is_none(){
    if let Ok(i)=usize::try_from(slot){if i<view.len()&&view.enabled(i){if let Some(mut stack)=view.get(i){
        if crate::lan::menu::sd_stack_limits::single_stack_drawer(&stack){
            *drag=None;stack.count=1;view.guest.carried=Some(stack);
            return std::ops::ControlFlow::Break((Vec::new(),Vec::new()));
        }
    }}}
}
std::ops::ControlFlow::Continue(())
}
