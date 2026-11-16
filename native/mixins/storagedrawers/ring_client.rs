fn sd_ring_client(menu: &mut crate::menu::Menu, env: &crate::menu::Env<'_>, slot: usize, primary: bool) -> std::ops::ControlFlow<bool> {
if !primary&&menu.may_pickup(env,slot){if let Some(mut ring)=menu.slot(slot).cloned(){if menu.may_place(env,slot,&ring){
    let mut cursor=menu.carried.clone();if crate::crafting::sd_items::ring_click(&mut ring,&mut cursor){
        menu.set_by_player(env,slot,Some(ring));menu.carried=cursor;menu.inventory_slots_changed(env);return std::ops::ControlFlow::Break(true);
    }
}}}
std::ops::ControlFlow::Continue(())
}
