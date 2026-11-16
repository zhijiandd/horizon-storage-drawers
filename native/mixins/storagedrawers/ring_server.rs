fn sd_ring_server(view: &mut crate::lan::menu::View<'_>, slot: i32, button: i32, rules: &dyn crate::lan::menu::Rules) -> std::ops::ControlFlow<()> {
if button==1{if let Ok(i)=usize::try_from(slot){if i<view.len()&&view.enabled(i)&&view.may_pickup(i,rules){
    if let Some(mut ring)=view.get(i){if view.may_place(i,&ring,rules){
        let mut cursor=view.guest.carried.clone();if crate::crafting::sd_items::ring_click(&mut ring,&mut cursor){
            view.set(i,Some(ring));view.guest.carried=cursor;return std::ops::ControlFlow::Break(());
        }
    }}
}}}
std::ops::ControlFlow::Continue(())
}
