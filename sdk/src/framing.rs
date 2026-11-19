//! Shared four-slot inventory for both halves of the framing table.
use crate::{engine as sd,state,compat::Stack,items,menu,menu_context::Context,frame_logic};
use mod_api::*;
use std::{cell::RefCell,collections::BTreeMap};
const TABLE:&str="storagedrawers:framing_table";
thread_local!{
    static OPEN:RefCell<BTreeMap<u128,(i32,String,[i32;3])>>=const{RefCell::new(BTreeMap::new())};
    static MATERIALS:RefCell<BTreeMap<String,(String,bool,bool)>>=const{RefCell::new(BTreeMap::new())};
}
pub fn framing_materials(materials:BTreeMap<String,(String,bool,bool)>){
    MATERIALS.with_borrow_mut(|table|*table=materials);
}

pub fn table_placement(pos: [i32; 3], yaw: f32) -> (&'static str, [i32; 3]) {
    let facing = ["south", "west", "north", "east"][((yaw / 90. + 0.5).floor() as i32 & 3) as usize];
    (facing, other(pos, facing, "right"))
}
pub struct Rules;
pub fn register(r:&mut dyn Registrar){r.menu_part("storagedrawers:framing",Rules);}
fn other(pos:[i32;3],facing:&str,part:&str)->[i32;3]{
    let d=match facing{"south"=>[1,0,0],"east"=>[0,0,-1],"west"=>[0,0,1],_=>[-1,0,0]};
    std::array::from_fn(|i|pos[i]+if part=="right"{d[i]}else{-d[i]})
}
fn master(host:&dyn Host,at:BlockSide)->BlockSide{
    if at.level.block_property(host,at.pos,"part").as_deref()==Some("left"){
        BlockSide{pos:other(at.pos,&crate::interaction::facing(host,at),"left"),..at}
    }else{at}
}
fn inputs(host:&dyn Host,entity:BlockEntityRef)->Option<Vec<Option<Stack>>>{
    (0..4).map(|i|entity.item_slot(host,i).ok()?.map(|s|Stack::from_api(host,&s)).transpose_option()).collect()
}
trait TransposeOption<T>{fn transpose_option(self)->Option<Option<T>>;}
impl<T> TransposeOption<T> for Option<Option<T>>{fn transpose_option(self)->Option<Option<T>>{match self{None=>Some(None),Some(Some(v))=>Some(Some(v)),Some(None)=>None}}}
pub fn material(ctx:&Context,s:&Stack)->bool{
    let Some(s)=s.to_api(ctx.host)else{return false};let Some(block)=ctx.host.item_block(s.item())else{return false};
    let id=block.id(ctx.host);let cfg=sd::config::features();
    if cfg["framing_material_denylist"].as_array().is_some_and(|v|v.iter().filter_map(|s|s.as_str()).any(|s|s==id||id.split_once(':').is_some_and(|(n,_)|n==s))){return false}
    let solid=sd::config::boolean("framing_solid_materials",true);
    let opaque=sd::config::boolean("framing_opaque_materials",false);
    if !solid&&!opaque{return true;}
    let item=s.item().id(ctx.host);
    MATERIALS.with_borrow(|table|table.get(&item)
        .is_some_and(|(block,full,occludes)|block==&id&&(!solid||*full)&&(!opaque||*occludes)))
}
pub(crate) fn accepts(ctx:&Context,v:&[Option<Stack>],slot:usize,s:&Stack)->bool{
    if slot>=4{return false}if slot!=0{return material(ctx,s)}
    if items::framed_id(&s.item).is_none(){return false}
    if let Some((_,materials))=items::unpack_frame(s){if materials.iter().enumerate().any(|(i,s)|s.is_some()&&v.get(i+1).is_some_and(Option::is_some)){return false}}
    true
}
pub fn placed(host:&mut dyn ServerHost,at:BlockSide,state:BlockState){
    let facing=crate::interaction::facing(host,at);let pos=other(at.pos,&facing,"right");
    if !at.level.is_loaded(host,pos){return}
    let Some(old)=at.level.block_state(host,pos)else{return};
    if host.block_states().replaceable(old)!=Some(true){return}
    let Ok(state) = state.with(&host.block_states(), "part", "left") else { return; };
    let _=at.level.set_block(host,pos,state,BlockUpdateFlags::NEIGHBORS|BlockUpdateFlags::CLIENTS);
    if let Some(entity)=host.block_entity(at.level,pos){state::remember(host,entity);}
}
pub fn open(host:&mut dyn ServerHost,player:PlayerRef,at:BlockSide){
    let at=master(host,at);let Some(entity)=host.block_entity(at.level,at.pos)else{return};
    let Some(v)=inputs(host,entity)else{return};
    let Some(key)=host.menu_keys().into_iter().find(|k|k.id(host)=="storagedrawers:framing")else{return};
    let Ok(Some(window))=player.open_mod_menu(host,key,Text::translate("block.storagedrawers.framing_table",Vec::new()),Vec::new(),Some(at.pos),MenuReplace::CloseFirst)else{return};
    OPEN.with_borrow_mut(|m|m.insert(player.uuid(),(window,at.level.dimension_id().into(),at.pos)));
    let _=host.menu_set_items(player.uuid(),window,projection(host,&v));
}
fn projection(host:&dyn Host,v:&[Option<Stack>])->Vec<Option<ItemStack>>{
    frame_logic::shown(v).iter().map(|s|s.as_ref().and_then(|s|s.to_api(host))).collect()
}
pub fn close(e:&MenuClosedEvent){
    OPEN.with_borrow_mut(|m|{if m.get(&e.uuid).is_some_and(|v|v.0==e.window){m.remove(&e.uuid);}});frame_logic::clear(e.uuid);
}
pub fn click(host:&mut dyn ServerHost,e:&MenuClickedEvent){
    let Some((window,dim,pos))=OPEN.with_borrow(|m|m.get(&e.uuid).cloned()).filter(|v|v.0==e.window)else{return};
    let Some(menu)=host.menu(e.uuid,window)else{return};let Some(player)=host.player(e.uuid)else{return};
    let Some(level)=player.level(host).filter(|l|l.dimension_id()==dim)else{host.close_menu(e.uuid);return};let at=BlockSide{level,pos,side:None};
    if state::block(host,at).as_deref()!=Some(TABLE)||player.entity().position(host).is_none_or(|p|(0..3).map(|i|(p[i]-f64::from(pos[i])-0.5).powi(2)).sum::<f64>()>64.){host.close_menu(e.uuid);return}
    let Some(entity)=host.block_entity(level,pos)else{return};let Some(v)=inputs(host,entity)else{return};
    let Some(inventory)=player.inventory_slots(host)else{return};let mut inv=vec![None;36];let mut offhand=None;
    for (i,s) in inventory{let Some(s)=Stack::from_api(host,&s)else{return};if i<36{inv[i as usize]=Some(s)}else if i==40{offhand=Some(s)}}
    let Some(carried) = menu.carried(host) else { return; };
    let Some(cursor)=carried.as_ref().map(|s|Stack::from_api(host,s)).transpose_option()else{return};
    let old=(v.clone(),inv.clone(),cursor.clone(),offhand.clone());
    let creative=player.game_mode(host)==Some(GameMode::Creative);
    let (v,inv,cursor,offhand,drops)=frame_logic::handle(&Context{host,at},e.uuid,e.slot,e.button,e.mode,creative,v,inv,cursor,offhand);
    if (&v,&inv,&cursor,&offhand)!=(&old.0,&old.1,&old.2,&old.3){
        let convert=|host:&dyn Host,s:Option<&Stack>|s.map(|s|s.to_api(host).ok_or(())).transpose();
        let result:Result<(),()>=menu::click_transaction(host,e.uuid,|host,tx|{
            if cursor!=old.2{let value=convert(host,cursor.as_ref())?;menu::cursor(host,tx,menu,value)?;}
            for i in 0..4{if v[i]!=old.0[i]{let value=convert(host,v[i].as_ref())?;entity.set_item_slot(host,tx,i as u32,value).map_err(|_|())?;}}
            for i in 0..36{if inv[i]!=old.1[i]{let value=convert(host,inv[i].as_ref())?;player.set_inventory_slot(host,i as u32,value).map_err(|_|())?;}}
            if offhand!=old.3{let value=convert(host,offhand.as_ref())?;player.set_inventory_slot(host,40,value).map_err(|_|())?;}
            Ok(())
        });
        if result.is_ok(){host.block_entity_sync(pos);if let Some(living)=host.living(player.entity()){for drop in drops{if let Some(s)=drop.to_api(host){let _=living.drop(host,s,false,true);}}}}
    }
    if let Some(v)=inputs(host,entity){let _=host.menu_set_items(e.uuid,window,projection(host,&v));}
}
#[mod_api::horizon_part]
impl MenuPart for Rules{
    fn menu_slot_rules(&self,host:&dyn Host,e:&mut MenuSlotRulesEvent){
        if e.container==MenuContainer::Player{return}e.may_place=false;
        if e.slot>=4{return}let Some(pos)=e.context.pos else{return};let Some(level)=host.level(host.dimension())else{return};
        let at=BlockSide{level,pos,side:None};let Some(s)=Stack::from_api(host,e.stack)else{return};
        let v:Vec<_>=(0..4).map(|i|e.context.slot(MenuContainer::Main,i).and_then(|s|Stack::from_api(host,s))).collect();
        e.may_place=accepts(&Context{host,at},&v,e.slot,&s);
    }
}
pub fn layout()->Screen{
    let mut slots=vec![None;9];for (i,p) in [[23.,35.],[50.,17.],[102.,17.],[50.,53.],[133.,35.]].into_iter().enumerate(){slots[i]=Some(p)}
    Screen{width:176.,height:166.,slots,inventory_y:84.,inventory_x:8.,own_labels:true,own_background:true,..Screen::default()}
}
pub fn draw(host:&dyn ClientHost,e:&mut ScreenDrawEvent){
    let [x,y]=e.origin;e.draw.sprite("storagedrawers:gui/framing",[0.,0.,176.,166.],[x,y,176.,166.],None);
    for (key,p) in [("block.storagedrawers.framing_table",[x+8.,y+6.]),("container.inventory",[x+8.,y+73.])]{
        e.draw.text(&host.translate(key).unwrap_or_else(||key.into()),p,0xff404040,false);
    }
}
#[derive(Clone)]
pub struct Partner {
    pub pos: [i32;3],
    state: BlockState,
}

pub fn partner(level:&dyn LevelHost,pos:[i32;3],state:BlockState)->Option<Partner>{
    let name=level.state_string(state)?;
    let properties=name.split_once('[')?.1.strip_suffix(']')?;
    let value=|key|properties.split(',').filter_map(|p|p.split_once('=')).find_map(|(k,v)|(k==key).then_some(v));
    let facing=value("facing")?;
    let part=value("part")?;
    let expected=level.state_with_property(state,"part",if part=="left"{"right"}else{"left"})?;
    let pos=other(pos,facing,part);
    (level.block_state(pos)==Some(expected)).then_some(Partner{pos,state:expected})
}

pub fn remove_partner(host:&mut dyn ServerHost,level:LevelRef,partner:Partner){
    if level.block_state(host,partner.pos)==Some(partner.state) && let Some(air)=host.block_state_id("minecraft:air"){
        let _=level.set_block(host,partner.pos,air,BlockUpdateFlags::CLIENTS|BlockUpdateFlags::NEIGHBORS|BlockUpdateFlags::SUPPRESS_DROPS);
    }
}

pub fn refresh(host:&mut dyn ServerHost,level:LevelRef){
    let opened=OPEN.with_borrow(|map|map.clone());
    for (uuid,(window,dim,pos)) in opened{
        if dim!=level.dimension_id(){continue;}
        let Some(menu)=host.menu(uuid,window)else{OPEN.with_borrow_mut(|m|{m.remove(&uuid);});continue};
        if !super::menu::within_reach(host,uuid,level,pos) || level.block(host,pos).is_none_or(|b|b.id(host)!=TABLE){
            host.close_menu(uuid);
            continue;
        }
        let Some(entity)=host.block_entity(level,pos)else{continue};
        let Some(stacks)=inputs(host,entity)else{continue};
        let slots=projection(host,&stacks);
        super::menu::refresh_slots(host,uuid,window,menu,slots);
    }
}
