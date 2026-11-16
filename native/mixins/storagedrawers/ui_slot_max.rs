fn sd_ui_slot_max(menu: &crate::menu::Menu, env: &crate::menu::Env<'_>, slot: usize, stack: &crate::items::ItemStack) -> std::ops::ControlFlow<u32> {
if let Some(value)=crate::menu::sd_rules::slot_max(menu,env,slot,stack){return std::ops::ControlFlow::Break(value);}
std::ops::ControlFlow::Continue(())
}
