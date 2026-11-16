fn sd_auto_restore(game: &mut crate::game::Game<'_>, view: &View, slot: usize, stack: &horizon_client::items::ItemStack) -> std::ops::ControlFlow<()> {
let location=view.locate(slot).map(|(p,s)|([p.0,p.1,p.2],s));
if let Some((p,s))=location {if crate::game::sd_integration::auto_put_back(game,p,s,stack){return std::ops::ControlFlow::Break(());}}
std::ops::ControlFlow::Continue(())
}
