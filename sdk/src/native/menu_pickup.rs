fn player_inventory_pickup(
    game: &mut Game<'_>, uuid: u128, window: i32, slot: i32, button: i32, mode: i32,
) -> core::ops::ControlFlow<()> {
    use horizon_client::{lan::menu::Slots, menu::{Input, Kind, Menu}};
    use core::ops::ControlFlow::{Break, Continue};

    if mode != 0 || !matches!(button, 0 | 1) { return Continue(()); }
    let Some((index, conn, kind)) = game.lan.as_ref().and_then(|lan| {
        let index = lan.hub.guests.iter().position(|guest| guest.uuid == uuid && !guest.dead)?;
        let guest = &lan.hub.guests[index];
        let opened = guest.menu.as_ref()?;
        let kind = opened.mod_type?;
        if opened.window != window || opened.slots != Slots::Native
            || !matches!(kind.id, "storagedrawers:framing" | "storagedrawers:drawers")
            || !(kind.slots as i32..kind.slots as i32 + 36).contains(&slot)
        { return None; }
        Some((index, guest.conn, kind))
    }) else { return Continue(()); };
    if storagedrawers::menu_drag_pending(kind.id, uuid) { return Continue(()); }

    // An ordinary backpack click uses Horizon's own inventory and cursor
    // handling. Only gestures involving the table's storage need its rules.
    let mut menu = Menu::new(Kind::Mod(kind));
    menu.set_registry_id(Some(kind.id));
    game.guest_menu_op(index, &mut menu, |menu, env| {
        let _ = menu.click(env, slot, button, Input::Pickup);
    });
    if let Some(lan) = game.lan.as_mut() { lan.hub.resend_menu(conn, window, &game.world); }
    Break(())
}
