fn sd_ui_may_place(menu: &crate::menu::Menu, slot: usize, stack: &crate::items::ItemStack) -> std::ops::ControlFlow<bool> {
if let Some(value)=crate::menu::sd_rules::may_place(menu,slot,stack){return std::ops::ControlFlow::Break(value);}
std::ops::ControlFlow::Continue(())
}
