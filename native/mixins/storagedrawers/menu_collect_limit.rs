fn sd_menu_collect_limit(view: &crate::lan::menu::View<'_>) -> std::ops::ControlFlow<()> {
if view.guest.carried.as_ref().is_some_and(|s|s.count>=1&&crate::lan::menu::sd_stack_limits::single_stack_drawer(s)){
    return std::ops::ControlFlow::Break(());
}
std::ops::ControlFlow::Continue(())
}
