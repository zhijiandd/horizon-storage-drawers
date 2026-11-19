fn virtual_slots(
    menu: &mut horizon_client::menu::Menu,
    clicks: &[(i16, i32, Input)],
) -> ::core::ops::ControlFlow<Option<(Vec<Predicted>, bool)>> {
    if let horizon_client::menu::Kind::Mod(kind) = menu.kind()
        && matches!(kind.id, "storagedrawers:drawers" | "storagedrawers:framing")
    {
        if let [(slot, 0 | 1, Input::Pickup)] = clicks {
            let slot = usize::try_from(*slot).ok();
            let player_slot = slot.is_some_and(|slot| (kind.slots..kind.slots + 36).contains(&slot));
            if player_slot { return ::core::ops::ControlFlow::Continue(()); }
        }
        return ::core::ops::ControlFlow::Break(None);
    }
    ::core::ops::ControlFlow::Continue(())
}
