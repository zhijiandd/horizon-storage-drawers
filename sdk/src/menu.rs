//! Display slots never own stock. Every gesture commits the core and player together.
use crate::{engine as sd,state,compat::Stack,menu_logic::DrawerMenu,menu_context::Context,interaction};
use mod_api::*;
use std::{cell::{Cell,RefCell},collections::BTreeMap};
#[derive(Clone)]
struct Open{window:i32,dimension:String,pos:[i32;3]}
struct Drag{kind:i32,slots:Vec<i32>,source:Stack}
thread_local!{
    static OPEN:RefCell<BTreeMap<u128,Open>>=const{RefCell::new(BTreeMap::new())};
    static DRAG:RefCell<BTreeMap<u128,Drag>>=const{RefCell::new(BTreeMap::new())};
    static CLICK_PLAYER: Cell<Option<u128>> = const { Cell::new(None) };
}

pub fn click_player() -> Option<u128> { CLICK_PLAYER.with(Cell::get) }

pub fn click_transaction<H: ServerHost + ?Sized, T, E>(
    host: &mut H,
    player: u128,
    apply: impl FnOnce(&mut H, &Transaction) -> Result<T, E>,
) -> Result<T, E> {
    struct Restore(Option<u128>);
    impl Drop for Restore {
        fn drop(&mut self) { CLICK_PLAYER.with(|active| active.set(self.0)); }
    }
    let _scope = Restore(CLICK_PLAYER.with(|active| active.replace(Some(player))));
    host.transaction(apply)
}
pub struct Rules;
pub fn drag_pending(player: u128) -> bool { DRAG.with_borrow(|drags| drags.contains_key(&player)) }
pub fn register(r:&mut dyn Registrar){r.menu_part("storagedrawers:drawers",Rules);}
pub fn open(host:&mut dyn ServerHost,player:PlayerRef,at:BlockSide){
    let Some((_,nbt,g))=state::read(host,at)else{return};
    let Some(key)=host.menu_keys().into_iter().find(|k|k.id(host)=="storagedrawers:drawers")else{return};
    let Some(block)=state::block(host,at)else{return};
    let Ok(Some(window))=player.open_mod_menu(host,key,Text::translate(format!("block.storagedrawers.{}",crate::items::kind(&block)),Vec::new()),Vec::new(),Some(at.pos),MenuReplace::CloseFirst)else{return};
    OPEN.with_borrow_mut(|m|m.insert(player.uuid(),Open{window,dimension:at.level.dimension_id().into(),pos:at.pos}));
    let _=host.menu_set_items(player.uuid(),window,projection(host,&g,&nbt));
}
pub fn close(event:&MenuClosedEvent){
    OPEN.with_borrow_mut(|m|{if m.get(&event.uuid).is_some_and(|o|o.window==event.window){m.remove(&event.uuid);}});
    DRAG.with_borrow_mut(|m|m.remove(&event.uuid));
}
pub fn within_reach(host:&dyn Host,uuid:u128,level:LevelRef,pos:[i32;3])->bool{
    host.player(uuid).filter(|p|p.level(host)==Some(level)).and_then(|p|p.entity().position(host))
        .is_some_and(|p|(0..3).map(|i|(p[i]-f64::from(pos[i])-0.5).powi(2)).sum::<f64>()<=64.)
}

pub fn refresh_slots(host:&mut dyn ServerHost,uuid:u128,window:i32,menu:MenuRef,slots:Vec<Option<ItemStack>>){
    if slots.iter().enumerate().any(|(i,stack)|menu.slot(host,i).as_ref()!=Some(stack)){
        let _=host.menu_set_items(uuid,window,slots);
    }
}

pub fn refresh(host:&mut dyn ServerHost,level:LevelRef){
    let opened=OPEN.with_borrow(|map|map.clone());
    for (uuid,open) in opened{
        if open.dimension!=level.dimension_id(){continue;}
        let Some(menu)=host.menu(uuid,open.window)else{OPEN.with_borrow_mut(|m|{m.remove(&uuid);});continue};
        let at=BlockSide{level,pos:open.pos,side:None};
        if let Some((_,nbt,group))=state::read(host,at)
            && within_reach(host,uuid,level,open.pos) && group.check(sd::Access::Player(uuid)).is_ok(){
            let slots=projection(host,&group,&nbt);
            refresh_slots(host,uuid,open.window,menu,slots);
        }else{host.close_menu(uuid);}
    }
}
pub fn projection(host:&dyn Host,g:&sd::Group,nbt:&Compound)->Vec<Option<ItemStack>>{
    let (positions,upgrades)=state::upgrade_state(nbt,g);let mut slots=vec![None;18];
    for (p,s) in positions.into_iter().zip(upgrades){slots[p]=s.to_api(host);}
    for i in 0..g.drawers.len(){slots[9+i]=g.item(i).and_then(|key| state::stack(host,key,1));}
    slots
}
pub fn cursor(host:&mut dyn ServerHost,tx:&Transaction,menu:MenuRef,value:Option<ItemStack>)->Result<(),()>{
    let place=menu.cursor();
    let old = menu.carried(host).ok_or(())?;
    if old == value { return Ok(()); }
    let shared = match (&old, &value) {
        (Some(old), Some(value)) if old.is_stackable_with(value) => old.count().min(value.count()),
        _ => 0,
    };
    if let Some(old) = &old {
        let remove = old.count() - shared;
        if remove > 0 && place.extract(host, tx, &ItemResource::of(old), remove) != remove { return Err(()); }
    }
    if let Some(stack) = &value {
        let add = stack.count() - shared;
        if add > 0 && place.insert(host, tx, &ItemResource::of(stack), add) != add { return Err(()); }
    }
    // ItemPlace's generic insert may route overflow to the inventory. A menu
    // move must land on the cursor exactly, or roll back the entire click.
    (menu.carried(host) == Some(value)).then_some(()).ok_or(())
}
fn converted(host:&dyn Host,input:Option<&Stack>)->Result<Option<ItemStack>,()>{input.map(|s|s.to_api(host).ok_or(())).transpose()}
pub fn click(host:&mut dyn ServerHost,e:&MenuClickedEvent){
    let Some(open)=OPEN.with_borrow(|m|m.get(&e.uuid).cloned()).filter(|o|o.window==e.window)else{return};
    let Some(menu)=host.menu(e.uuid,e.window)else{return};
    let Some(player)=host.player(e.uuid)else{return};let Some(level)=player.level(host).filter(|l|l.dimension_id()==open.dimension)else{host.close_menu(e.uuid);return};
    if player.entity().position(host).is_none_or(|p|(0..3).map(|i|(p[i]-f64::from(open.pos[i])-0.5).powi(2)).sum::<f64>()>64.){host.close_menu(e.uuid);return}
    let at=BlockSide{level,pos:open.pos,side:None};let Some((entity,mut nbt,g))=state::read(host,at)else{host.close_menu(e.uuid);return};
    let access=sd::Access::Player(e.uuid);if g.check(access).is_err(){host.close_menu(e.uuid);return}
    let block=state::block(host,at).unwrap_or_default();let creative=player.game_mode(host)==Some(GameMode::Creative);
    let (positions,upgrades)=state::upgrade_state(&nbt,&g);
    let Some(inventory)=player.inventory_slots(host)else{return};let mut inv=vec![None;36];let mut offhand=None;
    for (i,s) in inventory{let Some(s)=Stack::from_api(host,&s)else{return};if i<36{inv[i as usize]=Some(s)}else if i==40{offhand=Some(s)}}
    let Some(carried) = menu.carried(host) else { return; };
    let carried=carried.as_ref().map(|s|Stack::from_api(host,s)).transpose_option();
    let Some(carried)=carried else{return};
    let mut state=DrawerMenu{g,positions,upgrades,inv,offhand,carried};let before=state.clone();let mut drops=Vec::new();
    let drag=DRAG.with_borrow_mut(|d|d.remove(&e.uuid));
    let (slot,button,mode)=(e.slot,e.button,e.mode);
    if mode!=5&&drag.is_some(){return}
    if !(0..=6).contains(&mode)||matches!(mode,0|1|6)&&!matches!(button,0|1){return}
    {
        let ctx=Context{host,at};
        if mode==5{
            let (stage,kind)=(button&3,(button>>2)&3);if button<0||button>10{return}
            match stage{
                0=>{if drag.is_none()&&matches!(kind,0|1|2)&&(kind!=2||creative){if let Some(source)=state.carried.clone(){DRAG.with_borrow_mut(|d|d.insert(e.uuid,Drag{kind,slots:Vec::new(),source}));}}return}
                1=>{if let Some(mut drag)=drag.filter(|d|d.kind==kind&&state.carried.as_ref()==Some(&d.source)){
                    if !drag.slots.contains(&slot)&&(kind==2||drag.source.count as usize>drag.slots.len())&&state.drag_accepts(&ctx,&block,slot,&drag.source,access){drag.slots.push(slot)}
                    DRAG.with_borrow_mut(|d|d.insert(e.uuid,drag));}return}
                2=>{let Some(drag)=drag.filter(|d|d.kind==kind&&state.carried.as_ref()==Some(&d.source))else{return};if drag.slots.is_empty(){return}
                    if drag.slots.len()==1&&kind<2{state.pickup(&ctx,&block,drag.slots[0],kind,access,creative)}
                    else{let each=match kind{0=>drag.source.count/drag.slots.len()as u32,1=>1,_=>ctx.stack_max_size(&drag.source)};let mut remaining=drag.source.count;
                        for slot in drag.slots{let amount=if kind==2{each}else{each.min(remaining)};let put=state.drag_put(&ctx,&block,slot,&drag.source,amount,access);if kind!=2{remaining=remaining.saturating_sub(put);}}
                        let mut source=drag.source;source.count=remaining;state.carried=(remaining>0).then_some(source);
                    }
                }_=>return,
            }
        }else if slot==-999&&matches!(mode,0|1){
            if let Some(mut source)=state.carried.take(){let mut drop=source.clone();drop.count=if button==1{1}else{source.count};source.count-=drop.count;state.carried=(source.count>0).then_some(source);drops.push(drop);}
        }else if state.visible(&block,slot){match mode{
            0=>state.pickup(&ctx,&block,slot,button,access,creative),1=>state.quick_move(&ctx,slot,access,creative),
            2=>state.swap(&ctx,&block,slot,button,access,creative),
            3=>{if creative&&state.carried.is_none(){state.carried=state.stack(&ctx,slot).map(|mut s|{s.count=ctx.stack_max_size(&s);s})}}
            4=>{if state.carried.is_none(){if let Some(drop)=state.take(slot,if button==0{1}else{u32::MAX},access,creative){drops.push(drop)}}}
            6=>state.collect(&ctx,&block,slot,button,access,creative),_=>{},
        }}
    }
    state.upgrades=state.upgrades.iter().map(|s|interaction::validate_remote(host,at,s)).collect();
    if state!=before&&state::save_upgrades(&mut nbt,&state.g,&state.positions,&state.upgrades){
        let result:Result<(),()>=click_transaction(host,e.uuid,|host,tx|{
            if state.carried!=before.carried{let value=converted(host,state.carried.as_ref())?;cursor(host,tx,menu,value)?;}
            if !state::write(host,entity,nbt.clone(),&state.g){return Err(())}
            for i in 0..36{if state.inv[i]!=before.inv[i]{let value=converted(host,state.inv[i].as_ref())?;player.set_inventory_slot(host,i as u32,value).map_err(|_|())?;}}
            if state.offhand!=before.offhand{let value=converted(host,state.offhand.as_ref())?;player.set_inventory_slot(host,40,value).map_err(|_|())?;}
            Ok(())
        });
        if result.is_ok(){if let Some(living)=host.living(player.entity()){for drop in drops{if let Some(s)=drop.to_api(host){let _=living.drop(host,s,false,true);}}}}
    }
    if let Some((_,nbt,g))=state::read(host,at){let _=host.menu_set_items(e.uuid,e.window,projection(host,&g,&nbt));}
}
trait TransposeOption<T>{fn transpose_option(self)->Option<Option<T>>;}
impl<T> TransposeOption<T> for Option<Option<T>>{fn transpose_option(self)->Option<Option<T>>{match self{None=>Some(None),Some(Some(v))=>Some(Some(v)),Some(None)=>None}}}
#[mod_api::horizon_part]
impl MenuPart for Rules{
    fn menu_slot_rules(&self,host:&dyn Host,e:&mut MenuSlotRulesEvent){
        if e.container==MenuContainer::Player{return}
        e.may_place=false;
        let Some(pos)=e.context.pos else{return};let Some(level)=host.level(host.dimension())else{return};let at=BlockSide{level,pos,side:None};
        let Some((_,nbt,g))=state::read(host,at)else{return};let Some(uuid)=e.context.player else{return};let access=sd::Access::Player(uuid);
        if g.check(access).is_err(){return}
        if e.slot<7{
            let Some(u)=sd::Upgrade::from_id(&e.stack.item().id(host))else{return};
            let (positions,_)=state::upgrade_state(&nbt,&g);let index=positions.iter().position(|p|*p==e.slot);
            let creative=host.player(uuid).and_then(|p|p.game_mode(host))==Some(GameMode::Creative);
            e.may_place=g.can_place_upgrade(index,u,e.stack.count(),creative,access);e.max_stack=1;
        }else if (9..13).contains(&e.slot){
            let Some(block)=state::block(host,at)else{return};let Some(input)=Stack::from_api(host,e.stack)else{return};
            let mut trial=DrawerMenu{g,positions:Vec::new(),upgrades:Vec::new(),inv:Vec::new(),offhand:None,carried:None};
            e.may_place=trial.deposit(&Context{host,at},&block,e.slot as i32,&input,1,access)>0;
        }
    }
}
pub fn layout(block:&str)->Screen{
    let mut slots=vec![None;18];for i in 0..7{slots[i]=Some([26.+i as f32*18.,86.]);}
    let cells:&[[f32;2]]=match state::slots(block){
        1=>&[[80.,36.]],2 if block.contains("compacting")=>&[[80.,23.],[67.,49.]],2=>&[[80.,23.],[80.,49.]],
        3=>&[[80.,23.],[67.,49.],[93.,49.]],_=>&[[67.,23.],[93.,23.],[67.,49.],[93.,49.]],
    };for (i,p) in cells.iter().enumerate(){slots[9+i]=Some(*p);}
    Screen{width:176.,height:199.,slots,inventory_y:117.,inventory_x:8.,own_labels:true,own_background:true,..Screen::default()}
}
pub fn draw(host:&dyn ClientHost,e:&mut ScreenDrawEvent){
    let [x,y]=e.origin;let variant=if e.block.contains("compacting"){if state::slots(e.block)==2{"comp_2"}else{"comp"}}else{match state::slots(e.block){2=>"2",4=>"4",_=>"1"}};
    e.draw.sprite(&format!("storagedrawers:gui/drawers_{variant}"),[0.,0.,176.,199.],[x,y,176.,199.],None);
    for (key,at) in [(format!("block.storagedrawers.{}",crate::items::kind(e.block)),[x+8.,y+6.]),("container.storagedrawers.upgrades".into(),[x+8.,y+75.]),("container.inventory".into(),[x+8.,y+105.])]{
        e.draw.text(&host.translate(&key).unwrap_or(key),at,0xff404040,false);
    }
    let Some(pos)=host.screen_pos()else{return};
    let Some(level)=host.local_player().and_then(|player|player.level(host))else{return};
    let Some((_,_,group))=state::read(host,BlockSide{level,pos,side:None})else{return};
    let stacks = group.upgraded_stacks_for(e.block.contains("compacting"));
    let stacks = i32::try_from(stacks).unwrap_or(i32::MAX / 64).to_string();
    let width = e.draw.text_width(&stacks);
    e.draw.text(&stacks,[x+161.-width,y+42.],0xff404040,false);
    let screen=layout(e.block);
    for index in 0..group.drawers.len(){
        if group.item(index).is_none(){continue;}
        let Some(at)=screen.slots.get(9+index).copied().flatten()else{continue};
        let value=quantity(group.count(index));
        let width=e.draw.text_width(&value);
        e.draw.text(&value,[x+at[0]+8.-width/2.,y+at[1]+18.],0xff404040,false);
    }
}
fn quantity(n:u64)->String{
    let n_float=n as f64;
    if n>=1_000_000_000{format!("{:.1}G",n_float/1e9)}
    else if n>=100_000_000{format!("{:.0}M",n_float/1e6)}
    else if n>=1_000_000{format!("{:.1}M",n_float/1e6)}
    else if n>=100_000{format!("{:.0}K",n_float/1e3)}
    else if n>=10_000{format!("{:.1}K",n_float/1e3)}
    else{n.to_string()}
}
