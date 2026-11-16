fn sd_auto_insert(game: &mut crate::game::Game<'_>, view: &View, slot: usize, stack: &horizon_client::items::ItemStack, face: Option<horizon_core::geometry::Direction>) -> std::ops::ControlFlow<(Option<ItemStack>,bool)> {
let location=view.locate(slot).map(|(p,s)|([p.0,p.1,p.2],s));
if let Some((p,s))=location {if let Some(out)=crate::game::sd_integration::auto_insert(game,p,s,stack,face){return std::ops::ControlFlow::Break(out);}}
std::ops::ControlFlow::Continue(())
}
