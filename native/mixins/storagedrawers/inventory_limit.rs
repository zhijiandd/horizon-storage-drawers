fn sd_inventory_limit(inventory: &mut [Option<horizon_client::items::ItemStack>], offhand: &mut Option<horizon_client::items::ItemStack>, selected: usize, max: u32, stack: &mut horizon_client::items::ItemStack) -> std::ops::ControlFlow<bool> {
if max!=1&&crate::game::sd_integration::single_stack_drawer(stack){
    return std::ops::ControlFlow::Break(crate::game::inventory_add(inventory,offhand,selected,1,stack));
}
std::ops::ControlFlow::Continue(())
}
