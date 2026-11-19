//! SDK adapter. Not released until all legacy entry points have been migrated.
pub use storagedrawers_recipe_engine as engine;
mod state;
mod storage;
mod compat;
mod items;
mod recipes;
mod network;
mod compacting;
mod interaction;
mod menu_context;
mod menu_logic;
mod menu;
mod framing;
mod frame_logic;
mod runtime;
mod drops;
mod render_models;
mod tooltips;
pub use drops::spill_contents;
pub use framing::framing_materials;
pub use framing::table_placement;
pub use interaction::drawer_panel;
pub use menu::click_player as menu_click_player;

pub fn menu_drag_pending(id: &str, player: u128) -> bool {
    if id == "storagedrawers:framing" { frame_logic::drag_pending(player) } else { menu::drag_pending(player) }
}

use mod_api::*;

#[derive(Default)]
struct Mod;
struct DrawerEntity;
struct KeyButton;
struct Item;

#[mod_api::horizon_mod]
impl Hooks for Mod {
    fn register(&self, r: &mut dyn Registrar, _: &RegistrationContext) {
        recipes::register(r);
        menu::register(r);
        framing::register(r);
        for id in state::BLOCKS {
            let block = r.block_key(id);
            let item=r.item_key(id);
            r.item_part(item,Item);
            if id.starts_with("storagedrawers:keybutton_"){
                r.block_part(block,KeyButton);
                continue;
            }
            // BlockEntityPart already contributes its BlockPart for these blocks.
            let kind = r.block_entity_type(id).blocks(&[id]).build();
            r.block_entity_part(&kind, DrawerEntity).expect("Storage Drawers block entity registration");
            // The client uses this association to find a screen for OpenScreen.
            // Drawer display slots belong to the native menu, not the block inventory.
            let container = if state::is_drawer(id) {
                Some((0, "storagedrawers:drawers"))
            } else if *id == "storagedrawers:framing_table" {
                Some((4, "storagedrawers:framing"))
            } else {
                None
            };
            if let Some((slots, menu)) = container {
                r.block_container(&block, ModContainer {
                    slots,
                    faces: std::array::from_fn(|_| Vec::new()),
                    inputs: Vec::new(),
                    outputs: Vec::new(),
                    menu: Some(menu.into()),
                });
            }
            if state::is_drawer(id) || state::is_controller(id) || state::is_port(id) {
                r.block_capability_before_game(&ITEMS, &block, |_, at, _, _| {
                    Some(Box::new(storage::DrawerItems(at)))
                });
            }
        }
        for id in state::TOOLS {
            let item=r.item_key(id);
            r.item_part(item,Item);
        }
    }
    fn right_click_block(&self,host:HostSide,event:&mut RightClickBlockEvent)->Flow{
        let used=match host{
            HostSide::Server(h)=>{interaction::press_button(h,event.click);interaction::right(h,event.click)},
            HostSide::Client(h)=>interaction::claims(h,event.click)
        };
        if used{event.cancelled=true;*event.cancellation_result=InteractionResult::Success;Flow::Cancel}else{Flow::Continue}
    }
    fn left_click_block(&self,host:&mut dyn ServerHost,event:&mut LeftClickBlockEvent)->Flow{
        if interaction::left(host,event){event.cancelled=true;Flow::Cancel}else{Flow::Continue}
    }
    fn menu_clicked(&self,host:&mut dyn ServerHost,event:&mut MenuClickedEvent){
        menu::click(host,event);framing::click(host,event);
    }
    fn menu_closed(&self,_:&mut dyn ServerHost,event:&mut MenuClosedEvent){menu::close(event);framing::close(event);}
    fn level_tick_post(&self,host:HostSide,event:&mut LevelTickPostEvent){
        if let HostSide::Server(host)=host { runtime::tick(host,event.level); }
    }
    fn block_changed(&self,level:&dyn LevelHost,event:&mut BlockChangedEvent){
        drops::changed(level,event);
    }
    fn player_destroyed_block(&self,host:&mut dyn ServerHost,event:&mut PlayerDestroyedBlockEvent){
        drops::player_destroyed(host,event);
    }
    fn block_drops(&self,level:&dyn LevelHost,event:&mut BlockDropsEvent<'_>)->Flow{
        drops::block_drops(level,event);
        Flow::Continue
    }
    fn item_stacked_on_other(&self,_:&dyn LoopContext,event:&mut ItemStackedOnOtherEvent<'_, '_>)->Flow{
        if interaction::ring_click(event.event){Flow::Cancel}else{Flow::Continue}
    }
    fn item_tooltip(&self,host:&dyn ClientHost,event:&mut ItemTooltipEvent<'_>){
        interaction::tooltip(host,event);
    }
    fn server_about_to_start(&self,_:&mut dyn ServerHost,_:&mut ServerAboutToStartEvent){
        engine::config::write_defaults();
        interaction::clear();
        state::clear();
        runtime::clear();
        drops::clear();
    }
    fn standalone_models(&self,_:&RegistrationContext,event:&mut StandaloneModelsEvent<'_>){
        event.models.extend(render_models::MODELS.iter().map(|id|(*id).to_owned()));
    }
}

#[mod_api::horizon_part]
impl BlockPart for KeyButton {
    fn is_signal_source(&self,_:&dyn LevelHost,event:&mut IsSignalSourceEvent){
        event.source=false;
    }
    fn block_signal(&self,_:&dyn LevelHost,event:&mut BlockSignalEvent){
        event.signal=0;
    }
}

#[mod_api::horizon_part]
impl BlockPart for DrawerEntity {
    fn has_analog_output(&self, level: &dyn LevelHost, event: &mut HasAnalogOutputEvent) {
        event.output = level.state_block(event.state).is_some_and(state::is_drawer);
    }

    fn analog_output(&self, level: &dyn LevelHost, event: &mut AnalogOutputEvent) {
        event.output = state::analog(level, event);
    }
    fn entity_inside(&self,level:&mut dyn LevelRandomHost,event:&mut EntityInsideEvent<'_>){
        runtime::contact(level,event);
    }
    fn clone_item_stack(&self,host:&dyn Host,event:&mut CloneItemStackEvent<'_>)->Option<ItemStack>{
        drops::pick(host,event)
    }
    fn light_emission(&self,host:&dyn Host,event:&mut LightEmissionEvent)->Option<u8>{
        let level=host.level(event.dimension.key().id())?;
        let pos=[event.pos.x,event.pos.y,event.pos.z];
        let (_,_,group)=state::read(host,BlockSide{level,pos,side:None})?;
        group.has(engine::Upgrade::Illumination)
            .then(||engine::config::uint("illumination_level",13,0,15) as u8)
    }
}

#[mod_api::horizon_part]
impl BlockEntityPart for DrawerEntity {
    fn screen(&self,_:&dyn Host,event:&mut ScreenEvent)->Option<Screen>{
        if state::is_drawer(event.block){Some(menu::layout(event.block))}
        else if event.block=="storagedrawers:framing_table"{Some(framing::layout())}else{None}
    }
    fn screen_draw(&self,host:&dyn ClientHost,event:&mut ScreenDrawEvent){
        if state::is_drawer(event.block){menu::draw(host,event)}else if event.block=="storagedrawers:framing_table"{framing::draw(host,event)}
    }
    fn block_entity_on_load(&self,host:HostSide,event:&mut BlockEntityOnLoadEvent){
        match host {
            HostSide::Server(h)=>state::remember(h,event.entity),
            HostSide::Client(h)=>state::remember(h,event.entity),
        }
    }
    fn block_entity(&self, _: LevelSide, event: &mut BlockEntityEvent<'_>) -> Option<Compound> {
        state::initial(event.block)
    }

    fn block_entity_update_tag(&self, event: &mut BlockEntityUpdateTagEvent<'_>) -> Option<Compound> {
        let mut tag=event.entity.nbt.clone();
        if event.entity.id()=="storagedrawers:framing_table"{
            tag.insert("sd_frame_inputs".into(),Tag::List(event.entity.items.iter().enumerate().filter_map(|(index,stack)|{
                let stack=compat::Stack::from_recipe(stack.as_ref()?)?;
                Some(Tag::compound([("Slot",Tag::Int(index as i32)),("Item",compat::item_to_nbt(&stack))]))
            }).collect()));
        }
        Some(tag)
    }
    fn pre_remove_side_effects(&self,_:&mut dyn LevelRandomHost,event:&mut PreRemoveSideEffectsEvent<'_>){
        drops::pre_remove(event);
    }
}

#[mod_api::horizon_part]
impl ItemPart for Item{
    fn on_block_placed_by(&self,host:&mut dyn ServerHost,event:&mut OnBlockPlacedByEvent){interaction::placed(host,event);}
    fn use_item(&self,host:HostSide,event:&mut UseItemEvent<'_>)->Option<InteractionResult>{
        interaction::use_item(host,event)
    }
    fn tooltip_component(&self, host: &dyn ClientHost, event: &mut TooltipComponentEvent<'_>) -> Option<[f32; 2]> {
        tooltips::size(host, event.stack)
    }
    fn tooltip_component_draw(&self, host: &dyn ClientHost, event: &mut TooltipComponentDrawEvent<'_>) {
        tooltips::draw(host, event);
    }
}
