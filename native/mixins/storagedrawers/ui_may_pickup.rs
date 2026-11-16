fn sd_ui_may_pickup(menu: &crate::menu::Menu, slot: usize) -> std::ops::ControlFlow<bool> {
if let Some(value)=crate::menu::sd_rules::may_pickup(menu,slot){return std::ops::ControlFlow::Break(value);}
std::ops::ControlFlow::Continue(())
}
