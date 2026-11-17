from pathlib import Path
import json

ROOT=Path(__file__).resolve().parents[1]
B=ROOT/'native/mixins/storagedrawers'
REV='9f42064c11efe7ef3e5ed2baad7ddc2a3b36eea0'
lines=['[mod]','id="storagedrawers"','version="0.2.7"','', '[compatibility]',f'horizon_revision="{REV}"','',
    '[[module]]','id="integration"','file="crates/horizon-client/src/game.rs"','name="sd_integration"','source="integration.rs"','',
    '[[module]]','id="redstone"','file="crates/horizon-client/src/game.rs"','name="sd_redstone"','source="redstone.rs"','',
    '[[module]]','id="geometry"','file="crates/horizon-client/src/game.rs"','name="sd_geometry"','source="geometry.rs"','',
    '[[module]]','id="block_templates"','file="crates/horizon-client/src/mod_content.rs"','name="sd_block_templates"','source="block_templates.rs"','',
    '[[module]]','id="stack_limits"','file="crates/horizon-client/src/lan/menu.rs"','name="sd_stack_limits"','source="stack_limits.rs"','',
    '[[module]]','id="framing"','file="crates/horizon-client/src/game.rs"','name="sd_framing"','source="framing.rs"','',
    '[[module]]','id="appearance"','file="crates/horizon-client/src/game.rs"','name="sd_appearance"','source="appearance.rs"','',
    '[[module]]','id="items"','file="crates/horizon-client/src/game.rs"','name="sd_items"','source="items.rs"','',
    '[[module]]','id="recipe_items"','file="crates/horizon-client/src/crafting.rs"','name="sd_items"','source="recipe_items.rs"','',
    '[[module]]','id="client_ui_rules"','file="crates/horizon-client/src/menu.rs"','name="sd_rules"','source="menu_rules.rs"','visibility="pub"','',
    '[[crate]]','id="recipe_engine"','path="recipe_engine"','dependent="horizon-client-lib"','']

def inject(name,target,args,body,returns=None,hook=False,kind='HEAD',output=None):
    lines.extend(['[[injection]]',f'id="{name}"',f'kind="{kind}"'])
    if hook:lines.extend([f'hook="{target}"','hook_version=1'])
    else:lines.append(f'target="{target}"')
    if returns:lines.append('cancellable=true')
    lines.extend([f'helper="{name}.rs"','bindings = ['])
    for param,ty,value in args:
        lines.append('  {name='+json.dumps(param)+',ty='+json.dumps(ty)+',value='+json.dumps(value)+'},')
    lines.extend([']',''])
    signature=', '.join(f'{p}: {t}' for p,t,v in args)
    (B/f'{name}.rs').write_text(f'fn sd_{name}({signature})'+(f' -> std::ops::ControlFlow<{returns}>'if returns else f' -> {output}'if output else'')+' {\n'+body+'\n}\n',encoding='utf-8')

game=('game',"&mut crate::game::Game<'_>",'&mut *self')
pos=('pos','[i32;3]','pos')
inject('predict_components',"horizon_client::game::block_items::Game<'_>::apply_placed_item",[
    ('value','()','$value'),game,pos,('stack','&horizon_client::items::ItemStack','stack')],
    'crate::game::sd_integration::predicted_item(game,pos,stack);value',kind='RETURN',output='()')
inject('prepare_menu_rules',"horizon_client::game::remote::Game<'_>::remote_menu_ops",[
    ('game',"&crate::game::Game<'_>",'&*self')],
    'crate::game::sd_integration::prepare_menu_rules(game);')
inject('prepare_quick_rules',"horizon_client::game::remote::Game<'_>::remote_quick_move_all",[
    ('game',"&crate::game::Game<'_>",'&*self')],
    'crate::game::sd_integration::prepare_menu_rules(game);')
inject('attach_menu_rules','horizon_client_lib::menu::Menu::set_slot',[
    ('menu','&mut crate::menu::Menu','&mut *self')],
    'crate::menu::sd_rules::initialize(menu);')
for name,method,args,result,call in [
    ('ui_may_place','may_place',[('slot','usize','s'),('stack','&crate::items::ItemStack','stack')],'bool','may_place(menu,slot,stack)'),
    ('ui_may_pickup','may_pickup',[('slot','usize','s')],'bool','may_pickup(menu,slot)'),
    ('ui_slot_max','slot_max',[('env',"&crate::menu::Env<'_>",'env'),('slot','usize','s'),('stack','&crate::items::ItemStack','stack')],'u32','slot_max(menu,env,slot,stack)')]:
    inject(name,'horizon_client_lib::menu::Menu::'+method,[('menu','&crate::menu::Menu','&*self')]+args,
        f'if let Some(value)=crate::menu::sd_rules::{call}{{return std::ops::ControlFlow::Break(value);}}\nstd::ops::ControlFlow::Continue(())',result)
inject('ui_input','horizon_client_lib::menu::Menu::click',[
    ('menu','&mut crate::menu::Menu','&mut *self'),('input','crate::menu::Input','input')],
    'crate::menu::sd_rules::input(menu,input);')
inject('ui_quick_move','horizon_client_lib::menu::Menu::quick_move',[('menu','&mut crate::menu::Menu','&mut *self')],
    'crate::menu::sd_rules::quick_move(menu);')
inject('custom_craft','horizon_client_lib::crafting::Recipes::find_with',[
    ('_recipes','&crate::crafting::Recipes','&*self'),
    ('grid','impl AsRef<[Option<crate::items::ItemStack>]>','grid'),('size','usize','size')],
    '''if let Some((id,result))=crate::crafting::sd_items::craft(grid.as_ref(),size){
    if let Some(recipe)=_recipes.custom.values().flatten().find(|r|r.id==id){return std::ops::ControlFlow::Break(Some((recipe.id.as_str(),result)));}
}
std::ops::ControlFlow::Continue(())''','Option<(&str,ItemStack)>')
inject('custom_recipe_matches','horizon_client_lib::crafting::Recipes::recipe_matches',[
    ('recipes','&crate::crafting::Recipes','&*self'),
    ('id','&str','id'),('grid','&[Option<crate::items::ItemStack>]','grid'),('size','usize','size')],
    '''if id.starts_with("storagedrawers:"){
    if let Some((recipe,_))=crate::crafting::sd_items::craft(grid,size){return std::ops::ControlFlow::Break(recipe==id&&recipes.custom.values().flatten().any(|r|r.id==id));}
}
std::ops::ControlFlow::Continue(())''','bool')
inject('use_item','use_item',[game,('stack','&horizon_client::items::ItemStack','stack'),('claimed','&mut bool','&mut *claimed')],
    'crate::game::sd_integration::use_item(game,stack,claimed);',hook=True)
inject('table_destroy','destroy_block',[game,pos,('canceled','&mut bool','&mut *canceled')],
    'crate::game::sd_framing::destroy(game,pos,canceled);',hook=True)
inject('table_place_check','use_item_on',[game,('stack','&horizon_client::items::ItemStack','stack'),pos,('face','[i32;3]','face'),('claimed','&mut bool','&mut *claimed')],
    'crate::game::sd_framing::use_on(game,stack,pos,face,claimed);',hook=True)
inject('ring_server','horizon_client_lib::lan::menu::pickup',[
    ('view',"&mut crate::lan::menu::View<'_>",'&mut *view'),('slot','i32','slot'),('button','i32','button'),('rules','&dyn crate::lan::menu::Rules','rules')],
    '''if button==1{if let Ok(i)=usize::try_from(slot){if i<view.len()&&view.enabled(i)&&view.may_pickup(i,rules){
    if let Some(mut ring)=view.get(i){if view.may_place(i,&ring,rules){
        let mut cursor=view.guest.carried.clone();if crate::crafting::sd_items::ring_click(&mut ring,&mut cursor){
            view.set(i,Some(ring));view.guest.carried=cursor;return std::ops::ControlFlow::Break(());
        }
    }}
}}}
std::ops::ControlFlow::Continue(())''','()')
inject('ring_client','horizon_client_lib::menu::bundle::Menu::bundle_click',[
    ('menu','&mut crate::menu::Menu','&mut *self'),('env',"&crate::menu::Env<'_>",'env'),('slot','usize','s'),('primary','bool','primary')],
    '''if !primary&&menu.may_pickup(env,slot){if let Some(mut ring)=menu.slot(slot).cloned(){if menu.may_place(env,slot,&ring){
    let mut cursor=menu.carried.clone();if crate::crafting::sd_items::ring_click(&mut ring,&mut cursor){
        menu.set_by_player(env,slot,Some(ring));menu.carried=cursor;menu.inventory_slots_changed(env);return std::ops::ControlFlow::Break(true);
    }
}}}
std::ops::ControlFlow::Continue(())''','bool')
inject('content',"horizon_client::game::mods::content_root",[
    ('root','&std::path::Path','root'),('enabled','&std::collections::BTreeSet<String>','enabled'),
    ('disabled','&std::collections::BTreeSet<String>','disabled'),
    ('dev','&[std::path::PathBuf]','dev')],
    'crate::game::sd_integration::content_root(root,enabled,disabled,dev)','std::path::PathBuf')
inject('listed','mods',[('mods','&mut Vec<horizon_client::mod_api::NativeMod>','&mut *mods')],
    'crate::game::sd_integration::listed(mods);',hook=True)
inject('models','standalone_models',[('models','&mut Vec<String>','&mut *models')],
    'crate::game::sd_integration::models(models);',hook=True)
inject('init','block_entity',[('block','&str','block'),('out','&mut Option<(String,horizon_world::nbt::Compound)>','&mut *entity')],
    'crate::game::sd_integration::init(block,out);',hook=True)
inject('container','block_entity_container',[('block','&str','block'),('out','&mut Option<horizon_client::containers::ModContainer>','&mut *container')],
    'crate::game::sd_integration::container(block,out);',hook=True)
inject('screen','screen',[('block','&str','block'),('screen','&mut Option<horizon_client::mod_api::NativeScreen>','&mut *screen')],
    'crate::game::sd_integration::screen(block,screen);',hook=True)
inject('open_screen',"horizon_client::game::native_hooks::Game<'_>::open_native_screen",[game,
    ('key','(i32,i32,i32)','key'),('target','Option<(i32,i32,i32)>','target'),('window','i32','window'),
    ('menu','&str','menu'),('extra','&Option<horizon_protocol::menu_extra::Message>','&extra')],
    '''if matches!(menu,"minecraft:generic_9x2"|"minecraft:generic_9x1")&&game.native_on("storagedrawers") {
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
std::ops::ControlFlow::Continue(())''','()')
inject('screen_draw','screen_draw',[('game',"&crate::game::Game<'_>",'&*self'),('block','&str','block'),
    ('batch','&mut horizon_client::hud::HudBatch','&mut *batch'),('origin','[f32;2]','origin')],
    'crate::game::sd_integration::prepare_menu_rules(game);crate::game::sd_integration::screen_draw(game,block,batch,origin);',hook=True)
inject('tooltip','tooltip',[('item','&horizon_client::items::ItemStack','stack'),
    ('lines','&mut Vec<(horizon_client::text::Text,u32)>','&mut *lines')],
    'crate::game::sd_integration::tooltip(item,lines);',hook=True)
inject('ticks','block_entity_ticks',[('block','&str','block'),('ticks','&mut bool','&mut *ticks')],
    'crate::game::sd_integration::ticking(block,ticks);',hook=True)
inject('update_tag','block_entity_update_tag',[('entity','&horizon_client::containers::ModBlockEntity','entity'),('tag','&mut Option<horizon_world::nbt::Compound>','&mut *tag')],
    'crate::game::sd_integration::update_tag(entity,tag);',hook=True)
for name,hook in [('right_click','right_click_block'),('left_click','left_click_block')]:
    inject(name,hook,[game,pos,('face','[i32;3]','face'),('cancel','&mut bool','&mut *canceled')],
        f'crate::game::sd_integration::{name}(game,pos,face,cancel);',hook=True)
inject('tick','tick',[game],'crate::game::sd_integration::tick(game);',hook=True)
inject('menu_click','menu_clicked',[game,('uuid','u128','uuid'),('window','i32','window'),('slot','i32','slot'),('button','i32','button'),('mode','i32','mode')],
    'crate::game::sd_integration::menu_click(game,uuid,window,slot,button,mode);',hook=True)
inject('menu_close','menu_closed',[('uuid','u128','uuid'),('window','i32','window')],
    'crate::game::sd_integration::close_menu(uuid,window);',hook=True)
view_arg=('view','&View','view')
st=('stack','&horizon_client::items::ItemStack','&stack')
for name,method,result,body in [
    ('auto_insert','move_in_item','(Option<ItemStack>,bool)',
     'if let Some((p,s))=location {if let Some(out)=crate::game::sd_integration::auto_insert(game,p,s,stack,face){return std::ops::ControlFlow::Break(out);}}'),
    ('auto_take','take_one','Option<ItemStack>',
     'if let Some((p,s))=location {if let Some(out)=crate::game::sd_integration::auto_take(game,p,s,1){return std::ops::ControlFlow::Break(out);}}'),
    ('auto_restore','put_back','()',
     'if let Some((p,s))=location {if crate::game::sd_integration::auto_put_back(game,p,s,stack){return std::ops::ControlFlow::Break(());}}')]:
    args=[game,view_arg,('slot','usize','slot')]
    if name!='auto_take':args.append(st if name=='auto_insert'else('stack',st[1],'&one'))
    if name=='auto_insert':args.append(('face','Option<horizon_core::geometry::Direction>','face'))
    inject(name,f"horizon_client::game::machines::Game<'_>::{method}",args,'let location=view.locate(slot).map(|(p,s)|([p.0,p.1,p.2],s));\n'+body+'\nstd::ops::ControlFlow::Continue(())',result)
inject('io_view',"horizon_client::game::machines::Containers<'_>::view",[
    ('containers',"&crate::game::machines::Containers<'_>",'&*self'),('pos','(i32,i32,i32)','pos')],
    '''if let Some(BlockEntity::Mod(m))=containers.world.block_entities.get(&pos){
    if containers.name(containers.world.block(pos.0,pos.1,pos.2))!=m.id{return std::ops::ControlFlow::Break(None);}
    if let Some(container)=crate::game::sd_integration::io_view(m){
        let n=container.slots;
        return std::ops::ControlFlow::Break(Some(View{parts:vec![(pos,n)],kind:Kind::Mod,container:Some(Arc::new(container))}));
    }
}
std::ops::ControlFlow::Continue(())''','Option<View>')
inject('io_query',"horizon_client::game::machines::native_items::Game<'_>::native_container_slots",[
    ('game',"&crate::game::Game<'_>",'&*self'),('dimension','&str','dimension'),pos],
    '''if game.native_simulates()&&dimension==game.native_dimension(){
    if let Some(slots)=crate::game::sd_integration::io_slots(game,pos){return std::ops::ControlFlow::Break(Some(slots));}
}
std::ops::ControlFlow::Continue(())''','Option<Vec<NativeContainerSlot>>')
inject('io_bulk_insert',"horizon_client::game::machines::native_items::Game<'_>::insert_stack_through",[
    game,pos,('face','horizon_core::geometry::Direction','face'),('stack','&horizon_client::items::ItemStack','&stack')],
    '''if let Some((rest,_))=crate::game::sd_integration::io_insert(game,pos,0,stack,Some(face),false){return std::ops::ControlFlow::Break(rest);}
std::ops::ControlFlow::Continue(())''','Option<ItemStack>')
inject('io_accepts',"horizon_client::game::native_hooks::Game<'_>::native_container_accepts",[
    ('game',"&crate::game::Game<'_>",'&*self'),pos,('slot','usize','slot'),('stack','&horizon_client::items::ItemStack','stack'),('cancel','&mut bool','&mut *cancel')],
    'crate::game::sd_integration::io_accepts(game,pos,slot,stack,cancel);')
inject('io_can_place',"horizon_client::game::machines::Game<'_>::block_can_place",[
    ('game',"&crate::game::Game<'_>",'&*self'),pos,('slot','usize','slot'),('stack','&horizon_client::items::ItemStack','stack')],
    '''if let Some(handler)=crate::game::sd_integration::item_handler(game,pos){
    return std::ops::ControlFlow::Break(game.native_on("storagedrawers")&&handler.simulate_insert(game,slot,stack).count<stack.count);
}
std::ops::ControlFlow::Continue(())''','bool')
inject('io_move',"horizon_client::game::machines::native_items::Game<'_>::native_move_items",[
    game,('dimension','&str','dimension'),('from','[i32;3]','from'),('to','[i32;3]','to'),('to_face','horizon_core::geometry::Direction','to_face'),('item','Option<&str>','item'),('max','u32','max')],
    '''if dimension==game.native_dimension(){if let Some(moved)=crate::game::sd_integration::io_move(game,from,to,to_face,item,max){return std::ops::ControlFlow::Break(moved);}}
std::ops::ControlFlow::Continue(())''','u32')
inject('native_extract',"horizon_client::game::machines::native_items::Game<'_>::native_extract_item",
    [game,('dimension','&str','dimension'),pos,('slot','usize','slot'),('count','u32','count')],
    'if dimension==game.native_dimension(){if let Some(out)=crate::game::sd_integration::auto_take(game,pos,slot,count){return std::ops::ControlFlow::Break(out);}}\nstd::ops::ControlFlow::Continue(())',
    'Option<ItemStack>')
inject('predict_break',"horizon_client::game::remote::Game<'_>::predict_break",[game,pos],
    'if crate::game::sd_integration::suppress_break_prediction(game,pos){return std::ops::ControlFlow::Break(());}\nstd::ops::ControlFlow::Continue(())','()')
inject('drawer_behaviour','horizon_client_lib::mod_content::J::rename',[
    ('doc','&mut crate::mod_content::J','&mut *self'),('rename','&crate::mod_content::Rename','r')],
    'crate::mod_content::sd_block_templates::adjust(doc,rename);')
inject('predict_drawer_use',"horizon_client::game::remote::Game<'_>::remote_block_reacts",[
    ('game',"&crate::game::Game<'_>",'&*self'),('hit','&crate::game::Hit','hit'),
    ('held','Option<&horizon_client::items::ItemStack>','held')],
    '''let sneaking=game.remote_shift_down();
if crate::game::sd_integration::predict_drawer_use(game,hit,held,sneaking){
    return std::ops::ControlFlow::Break((InteractionResult::Consume,DryUse::default()));
}
std::ops::ControlFlow::Continue(())''','(InteractionResult,DryUse)')
inject('inventory_limit','horizon_client::game::inventory_add',[
    ('inventory','&mut [Option<horizon_client::items::ItemStack>]','&mut *inventory'),
    ('offhand','&mut Option<horizon_client::items::ItemStack>','&mut *offhand'),
    ('selected','usize','selected'),('max','u32','max'),('stack','&mut horizon_client::items::ItemStack','&mut *stack')],
    '''if max!=1&&crate::game::sd_integration::single_stack_drawer(stack){
    return std::ops::ControlFlow::Break(crate::game::inventory_add(inventory,offhand,selected,1,stack));
}
std::ops::ControlFlow::Continue(())''','bool')
inject('menu_stack_limit',"horizon_client_lib::lan::menu::View<'_>::slot_max",[
    ('stack','&crate::items::ItemStack','stack')],
    '''if crate::lan::menu::sd_stack_limits::single_stack_drawer(stack){return std::ops::ControlFlow::Break(1);}
std::ops::ControlFlow::Continue(())''','u32')
inject('menu_collect_limit','horizon_client_lib::lan::menu::pickup_all',[
    ('view',"&crate::lan::menu::View<'_>",'&*view')],
    '''if view.guest.carried.as_ref().is_some_and(|s|s.count>=1&&crate::lan::menu::sd_stack_limits::single_stack_drawer(s)){
    return std::ops::ControlFlow::Break(());
}
std::ops::ControlFlow::Continue(())''','()')
inject('menu_clone_limit','horizon_client_lib::lan::menu::click',[
    ('view',"&mut crate::lan::menu::View<'_>",'&mut *view'),
    ('drag','&mut Option<(u8,Vec<usize>)>','&mut *drag'),('slot','i32','slot'),('mode','i32','mode')],
    '''if mode==crate::lan::menu::input::CLONE&&view.guest.creative()&&view.guest.carried.is_none(){
    if let Ok(i)=usize::try_from(slot){if i<view.len()&&view.enabled(i){if let Some(mut stack)=view.get(i){
        if crate::lan::menu::sd_stack_limits::single_stack_drawer(&stack){
            *drag=None;stack.count=1;view.guest.carried=Some(stack);
            return std::ops::ControlFlow::Break((Vec::new(),Vec::new()));
        }
    }}}
}
std::ops::ControlFlow::Continue(())''','(Vec<ItemStack>,Vec<usize>)')
inject('block_tick','block_entity_tick',[game,pos,('entity','&mut horizon_client::containers::ModBlockEntity','&mut *entity')],
    'crate::game::sd_integration::block_tick(game,pos,entity);',hook=True)
inject('placed','place_block',[game,('stack','&horizon_client::items::ItemStack','stack'),pos],
    'crate::game::sd_integration::placed(game,stack,pos);',hook=True)
inject('before_remove',"horizon_client::game::spill::GameLevel<'_>::remove_entity",[('level',"&mut crate::game::GameLevel<'_>",'&mut *self'),('pos','horizon_core::position::BlockPos','pos'),('old','horizon_world::registry::BlockStateId','old')],
    'crate::game::sd_integration::before_remove(level,pos,old);')
inject('render','block_entity_submit',[('game',"&crate::game::Game<'_>",'&*self'),pos,('block','&str','block'),('entity','&horizon_client::containers::ModBlockEntity','entity'),('draw',"&mut crate::game::native_block_entity_draw::BlockEntityDraw",'&mut *draw')],
    'crate::game::sd_integration::render(game,pos,block,entity,draw);',hook=True)
inject('table_loot',"horizon_client::game::native_hooks::Game<'_>::native_loot_drops",[
    ('game',"&crate::game::Game<'_>",'&*self'),('context',"&horizon_client::mod_api::NativeLootDrops<'_>",'context'),('drops','&mut Vec<horizon_client::items::ItemStack>','&mut *drops')],
    'crate::game::sd_framing::loot(game,context,drops);')
inject('framing_bounds',"horizon_client::game::native_hooks::Game<'_>::native_block_entity_render_bounding_box",[
    ('block','&str','block'),('bounds','&mut [f32;6]','&mut *bounds')],
    'if block=="storagedrawers:framing_table"{*bounds=[-1.,0.,-1.,2.,2.,2.];}')
inject('item_appearance',"horizon_client::game::item_draw::Game<'_>::resolve_item_in_hand",[
    ('game',"&crate::game::Game<'_>",'&*self'),('stack','&horizon_client::items::ItemStack','stack'),
    ('display','horizon_assets::item_model::DisplayContext','display'),('owner','Option<&horizon_assets::item_model::OwnerState>','owner'),('seed','i32','seed'),('hand','horizon_client::cit::Hand','hand')],
    '''if let Some(state)=crate::game::sd_appearance::item(game,stack,display,owner,seed,hand){return std::ops::ControlFlow::Break(state);}
std::ops::ControlFlow::Continue(())''','ItemRenderState')
inject('signal','block_signal',[('level',"&crate::game::GameLevel<'_>",'&*self'),('pos','horizon_core::position::BlockPos','pos'),
    ('face','horizon_core::geometry::Direction','face'),('direct','bool','direct'),('signal','&mut i32','&mut *signal')],
    'crate::game::sd_redstone::signal(level,[pos.x,pos.y,pos.z],face,direct,signal);',hook=True)
level=('level',"&crate::game::GameLevel<'_>",'&*self')
state=('state','horizon_world::registry::BlockStateId','state')
inject('signal_source',"horizon_client::game::native_hooks::GameLevel<'_>::native_is_signal_source",[
    level,state,('source','&mut bool','&mut *source')],
    'crate::game::sd_redstone::signal_source(level,state,source);')
inject('has_analog',"horizon_client::game::native_hooks::GameLevel<'_>::native_has_analog_output",[
    level,state,('output','&mut bool','&mut *output')],
    'crate::game::sd_redstone::has_analog(level,state,output);')
inject('analog',"horizon_client::game::native_hooks::GameLevel<'_>::native_analog_output",[
    level,('pos','horizon_core::position::BlockPos','pos'),('output','&mut i32','&mut *output')],
    'crate::game::sd_redstone::analog(level,[pos.x,pos.y,pos.z],output);')
inject('item_contact',"horizon_client::game::native_hooks::Game<'_>::native_block_entity_inside",[
    ('dimension','&str','dimension'),('pos','[i32;3]','position'),('kind','&str','kind'),('entity','i32','entity')],
    'if kind=="minecraft:item"{crate::game::sd_integration::item_contact(dimension,pos,entity as u32);}')
(B/'horizon-mixin.toml').write_text('\n'.join(lines)+'\n',encoding='utf-8')
print('Generated native hook manifest and helpers')

