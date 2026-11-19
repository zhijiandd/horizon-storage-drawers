fn creative_hover_page(
    game: &Game<'_>, batch: &HudBatch, text: &str, x: f32, y: f32,
) -> core::ops::ControlFlow<()> {
    use core::ops::ControlFlow::{Break, Continue};
    if game.screen != Screen::Creative { return Continue(()); }
    let screen = &game.creative_screen;
    let origin = Game::creative_origin(batch);
    let cursor = (x, y);
    let Some(index) = Creative::hovered_tab(&screen.tabs, origin, cursor) else { return Continue(()); };
    let tab = &screen.tabs[index];
    if screen.on_page(tab) || game.creative_tab_title(tab) != text { return Continue(()); }
    // The first render pass already draws visible tab names. Suppress the
    // second pass only when it found a hidden tab and named that hidden tab.
    let visible_name_matches = screen.tabs.iter().filter(|tab| screen.on_page(tab)).any(|tab| {
        Creative::hovered_tab(std::slice::from_ref(tab), origin, cursor).is_some()
            && game.creative_tab_title(tab) == text
    });
    if visible_name_matches { Continue(()) } else { Break(()) }
}
