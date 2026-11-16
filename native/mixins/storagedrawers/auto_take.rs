fn sd_auto_take(game: &mut crate::game::Game<'_>, view: &View, slot: usize) -> std::ops::ControlFlow<Option<ItemStack>> {
let location=view.locate(slot).map(|(p,s)|([p.0,p.1,p.2],s));
if let Some((p,s))=location {if let Some(out)=crate::game::sd_integration::auto_take(game,p,s,1){return std::ops::ControlFlow::Break(out);}}
std::ops::ControlFlow::Continue(())
}
