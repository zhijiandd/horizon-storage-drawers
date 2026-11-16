fn sd_open_screen(game: &mut crate::game::Game<'_>, key: (i32,i32,i32), target: Option<(i32,i32,i32)>, window: i32, menu: &str, extra: &Option<horizon_protocol::menu_extra::Message>) -> std::ops::ControlFlow<()> {
if matches!(menu,"minecraft:generic_9x2"|"minecraft:generic_9x1")&&game.native_on("storagedrawers") {
    if let Some(t)=target {
        let block=game.native_block([t.0,t.1,t.2]);
        if block.starts_with("storagedrawers:") {
            let mut screen=None;
            crate::game::sd_integration::screen(&block,&mut screen);
            if let Some(screen)=screen {
                game.mod_screen=Some(OpenNativeScreen{key,window,block,screen,data:Vec::new(),extra:extra.clone()});
                return std::ops::ControlFlow::Break(());
            }
        }
    }
}
std::ops::ControlFlow::Continue(())
}
