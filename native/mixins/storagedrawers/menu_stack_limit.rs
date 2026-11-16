fn sd_menu_stack_limit(stack: &crate::items::ItemStack) -> std::ops::ControlFlow<u32> {
if crate::lan::menu::sd_stack_limits::single_stack_drawer(stack){return std::ops::ControlFlow::Break(1);}
std::ops::ControlFlow::Continue(())
}
