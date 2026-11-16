fn sd_drawer_use_pass(block: &str) -> std::ops::ControlFlow<InteractionResult> {
// The native right-click hook handles the drawer's front and upgrade/key gestures.
// Every unclaimed use must reach ItemStack.useOn, never the loom template's menu.
if block.strip_prefix("storagedrawers:").is_some_and(|id|id.contains("drawers_")){
    return std::ops::ControlFlow::Break(InteractionResult::Pass);
}
std::ops::ControlFlow::Continue(())
}
