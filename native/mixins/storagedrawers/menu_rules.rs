// Owned rules for one predicted menu; no global permission/slot decision cache.
use super::{Menu,Kind,Input,Env,MAIN,PLAYER};
use crate::{items::ItemStack,crafting::sd_items as data};
use horizon_world::nbt::{self,NamedTag,Tag,Limits};
use storagedrawers_recipe_engine as sd;
use std::{cell::RefCell,collections::BTreeSet};

#[derive(Clone)]
pub enum Mode{
    Drawers{group:sd::Group,deposits:Vec<(ItemStack,Vec<u32>)>},
    Framing{materials:BTreeSet<String>},
}
#[derive(Clone)]
pub struct Context{mode:Mode,quick_move:bool}
const RULES:i32=0x53445231;
fn ours(menu:&Menu)->bool{matches!(menu.kind,Kind::Mod(t)if matches!(t.id,"storagedrawers:drawers"|"storagedrawers:framing"))}
thread_local!{static PENDING:RefCell<Option<Context>>=const{RefCell::new(None)};}
pub fn pending(context:Option<Context>){PENDING.with_borrow_mut(|p|*p=context);}
pub fn drawers(bytes:&[u8],deposits:Vec<(ItemStack,Vec<u32>)>)->Option<Context>{
    Some(Context{mode:Mode::Drawers{group:sd::codec::decode(bytes).ok()?,deposits},quick_move:false})
}
pub fn framing(materials:BTreeSet<String>)->Context{Context{mode:Mode::Framing{materials},quick_move:false}}
pub fn initialize(menu:&mut Menu){
    if menu.data.first()!=Some(&RULES){attach(menu,menu.kind);}
}
pub fn attach(menu:&mut Menu,kind:Kind){
    if !matches!(kind,Kind::Mod(t)if matches!(t.id,"storagedrawers:drawers"|"storagedrawers:framing")){return;}
    let context=PENDING.with_borrow(Clone::clone).filter(|c|match (&c.mode,kind){
        (Mode::Drawers{..},Kind::Mod(t))=>t.id=="storagedrawers:drawers",
        (Mode::Framing{..},Kind::Mod(t))=>t.id=="storagedrawers:framing",_=>false});
    let Some(context)=context else{return;};
    // Native menus own their ContainerData. These two menu types use it for
    // the complete local validation snapshot, including original item components.
    let tag=match context.mode{
        Mode::Drawers{group,deposits}=>{
            let Ok(bytes)=sd::codec::encode(&group)else{return;};
            Tag::compound([("mode",Tag::Byte(1)),("group",Tag::ByteArray(bytes)),("deposits",Tag::List(deposits.into_iter().map(|(stack,limits)|
                Tag::compound([("item",horizon_items::item_nbt::item_to_nbt(&stack)),("limits",Tag::IntArray(limits.into_iter().map(|v|v as i32).collect()))])).collect()))])
        }
        Mode::Framing{materials}=>Tag::compound([("mode",Tag::Byte(2)),("materials",Tag::List(materials.into_iter().map(|v|Tag::String(v.as_str().into())).collect()))]),
    };
    if let Ok(bytes)=nbt::write_named(&NamedTag{name:"".into(),tag},Limits::tables()){
        menu.data=std::iter::once(RULES).chain(std::iter::once(0)).chain(bytes.into_iter().map(i32::from)).collect();
    }
}
fn context(menu:&Menu)->Option<Context>{
    if menu.data.first()!=Some(&RULES){return None;}
    let bytes:Vec<u8>=menu.data.get(2..)?.iter().map(|&v|v as u8).collect();
    let (root,_)=nbt::read_named(&bytes,Limits::tables()).ok()?;
    let mode=match root.tag.get("mode")?{
        Tag::Byte(1)=>{
            let Tag::ByteArray(bytes)=root.tag.get("group")?else{return None;};
            let Tag::List(entries)=root.tag.get("deposits")?else{return None;};
            let deposits=entries.iter().filter_map(|e|{
                let stack=horizon_items::item_nbt::item_from_nbt(e.get("item")?)?;
                let Tag::IntArray(limits)=e.get("limits")?else{return None;};
                Some((stack,limits.iter().map(|&v|v.max(0)as u32).collect()))
            }).collect();Mode::Drawers{group:sd::codec::decode(bytes).ok()?,deposits}
        }
        Tag::Byte(2)=>{
            let Tag::List(values)=root.tag.get("materials")?else{return None;};
            Mode::Framing{materials:values.iter().filter_map(|v|match v{Tag::String(s)=>Some(s.to_string_lossy()),_=>None}).collect()}
        }
        _=>return None,
    };Some(Context{mode,quick_move:menu.data.get(1)==Some(&1)})
}
fn upgrade(stack:&ItemStack)->Option<sd::Upgrade>{sd::Upgrade::from_id(stack.item.strip_suffix("_bound").unwrap_or(&stack.item))}
fn drawer_group(menu:&Menu,original:&sd::Group)->(sd::Group,Vec<usize>){
    let mut group=original.clone();group.upgrades.clear();let mut positions=Vec::new();
    for slot in 0..7{if let Some(u)=menu.slot(slot).and_then(upgrade){positions.push(slot);group.upgrades.push(u);}}
    (group,positions)
}
pub fn frame_accepts(inputs:&[Option<ItemStack>],slot:usize,stack:&ItemStack,material:bool)->bool{
    if slot>=4{return false;}
    if slot!=0{return material;}
    if data::framed_id(&stack.item).is_none(){return false;}
    if let Some((_,materials))=data::unpack_frame(stack){
        if materials.iter().enumerate().any(|(i,s)|s.is_some()&&inputs.get(i+1).is_some_and(Option::is_some)){return false;}
    }true
}
pub fn may_place(menu:&Menu,slot:usize,stack:&ItemStack)->Option<bool>{
    if !ours(menu){return None;}
    let cell=menu.slots.get(slot)?;if cell.container==PLAYER{return None;}if cell.container!=MAIN{return Some(false);}
    let Some(context)=context(menu)else{
        return matches!(menu.kind,Kind::Mod(t)if matches!(t.id,"storagedrawers:drawers"|"storagedrawers:framing")).then_some(false);
    };
    Some(match &context.mode{
        Mode::Framing{materials}=>frame_accepts(&menu.containers[MAIN],cell.index,stack,materials.contains(&*stack.item)),
        Mode::Drawers{group,deposits}=>{
            let (g,positions)=drawer_group(menu,group);
            if cell.index<7{
                let index=positions.iter().position(|&p|p==cell.index);
                upgrade(stack).is_some_and(|u|
                    g.can_place_upgrade(index,u,stack.count,menu.creative,sd::Access::Administrator))
            }else if !context.quick_move&&(9..9+g.drawers.len()).contains(&cell.index){
                deposits.iter().find(|(s,_)|s.same_item_same_components(stack))
                    .and_then(|(_,limits)|limits.get(cell.index-9)).is_some_and(|&n|n>0)
            }else{false}
        }
    })
}
pub fn may_pickup(menu:&Menu,slot:usize)->Option<bool>{
    if !ours(menu){return None;}
    let cell=menu.slots.get(slot)?;if cell.container==PLAYER{return None;}if cell.container!=MAIN{return Some(false);}
    let Some(context)=context(menu)else{
        return matches!(menu.kind,Kind::Mod(t)if matches!(t.id,"storagedrawers:drawers"|"storagedrawers:framing")).then_some(false);
    };
    Some(match &context.mode{
        Mode::Framing{..}=>cell.index<5,
        Mode::Drawers{group,..}=>{
            if cell.index>=7{return Some(false);}
            let (mut g,positions)=drawer_group(menu,group);
            positions.iter().position(|&p|p==cell.index).is_none_or(|i|
                (menu.creative||!matches!(g.upgrades[i],sd::Upgrade::CreativeStorage|sd::Upgrade::CreativeVending))
                    &&g.remove_upgrade(i,sd::Access::Administrator).is_ok())
        }
    })
}
pub fn slot_max(menu:&Menu,env:&Env,slot:usize,stack:&ItemStack)->Option<u32>{
    let context=context(menu)?;
    let cell=menu.slots.get(slot)?;if cell.container!=MAIN{return None;}
    let item_max=env.items.get(&stack.item).map_or(64,|i|i.max_stack);
    Some(match &context.mode{
        Mode::Drawers{deposits,..}if cell.index>=9=>{
            let free=deposits.iter().find(|(s,_)|s.same_item_same_components(stack))
                .and_then(|(_,limits)|limits.get(cell.index-9)).copied().unwrap_or(0);
            menu.slot(slot).map_or(0,|s|s.count).saturating_add(free.min(item_max)).clamp(1,99)
        }
        Mode::Drawers{..}=>1,Mode::Framing{..}=>item_max,
    })
}
pub fn input(menu:&mut Menu,input:Input){
    if ours(menu)&&menu.data.first()==Some(&RULES){menu.data[1]=i32::from(matches!(input,Input::QuickMove));}
}
pub fn quick_move(menu:&mut Menu){
    if ours(menu)&&menu.data.first()==Some(&RULES){menu.data[1]=1;}
}
