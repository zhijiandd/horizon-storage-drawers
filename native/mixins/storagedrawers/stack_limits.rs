// This module lives in the reusable client library, outside Game's adapter.
// Keep the compatibility correction specific to portable, loaded drawers.
pub(super) fn single_stack_drawer(stack:&crate::items::ItemStack)->bool{
    stack.item.strip_prefix("storagedrawers:").is_some_and(|id|id.contains("drawers_"))
        &&matches!(stack.data().and_then(|d|d.other("minecraft:max_stack_size")),Some(horizon_world::nbt::Tag::Int(1)))
}
