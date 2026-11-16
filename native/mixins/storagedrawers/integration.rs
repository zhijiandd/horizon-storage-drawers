// Pinned game adapter. Saved core state is authoritative; item slots are projections only.
use super::Game;
use storagedrawers_engine as sd;
use horizon_client::{containers::{BlockEntity,ModBlockEntity,ModContainer},items::ItemStack,text::Text};
use horizon_world::nbt::{Compound,Tag};
use horizon_core::geometry::Direction;
use std::{cell::RefCell,collections::{BTreeMap,BTreeSet}};

const ID:&str="storagedrawers";
const SAVE:&str="sd_state";
const UPGRADE_POS:&str="sd_upgrade_slots";
const UPGRADE_ITEMS:&str="sd_upgrade_items";
type Pos=[i32;3];

// A native package is one installable file. Its data payload contains its content companion.
// Run this before resources load, rather than expecting a second manual content installation.
pub(super) fn content_root(root:&std::path::Path,enabled:&BTreeSet<String>,disabled:&BTreeSet<String>,dev:&[std::path::PathBuf])
    ->std::ops::ControlFlow<std::path::PathBuf>{
    let bytes=horizon_client::native_data!("storagedrawers","content.hmod");
    let Ok(here)=std::env::current_dir()else{return std::ops::ControlFlow::Continue(());};
    let dir=here.join(".horizon-cache/native-content");
    let path=dir.join("storagedrawers-0.2.7.hmod");
    if dev.contains(&path){return std::ops::ControlFlow::Continue(());}
    let ready=std::fs::read(&path).is_ok_and(|saved|saved==bytes.as_slice());
    if !ready&&(std::fs::create_dir_all(&dir).and_then(|()|std::fs::write(&path,bytes))).is_err(){
        eprintln!("Storage Drawers: cannot materialize bundled resources at {}",path.display());
        return std::ops::ControlFlow::Continue(());
    }
    let mut with=dev.to_vec();with.push(path);
    std::ops::ControlFlow::Break(super::mods::content_root(root,enabled,disabled,&with))
}
pub(super) fn listed(mods:&mut Vec<horizon_client::mod_api::NativeMod>){
    mods.push(horizon_client::mod_api::NativeMod{id:ID.into(),title:Text::tr("storagedrawers.mod.title",&[]).plain(),
        description:Text::tr("storagedrawers.mod.description",&[]).plain(),keys:Vec::new()});
}
pub(super) fn models(models:&mut Vec<String>){super::sd_appearance::models(models);models.push("storagedrawers:block/sd_sprites".into());}
thread_local!{
    static MENUS:RefCell<BTreeMap<u128,(i32,String,Pos)>>=const{RefCell::new(BTreeMap::new())};
    static OPEN:RefCell<Vec<(u128,String,Pos)>>=const{RefCell::new(Vec::new())};
    static CLICKS:RefCell<BTreeMap<u128,(String,Pos,i64)>>=const{RefCell::new(BTreeMap::new())};
    static KEY_TIMES:RefCell<BTreeMap<u128,i64>>=const{RefCell::new(BTreeMap::new())};
    static DRAGS:RefCell<BTreeMap<u128,MenuDrag>>=const{RefCell::new(BTreeMap::new())};
    static HOPPER_CONTACTS:RefCell<BTreeMap<(String,Pos),BTreeSet<u32>>>=const{RefCell::new(BTreeMap::new())};
}
struct MenuDrag{kind:i32,slots:Vec<i32>,source:ItemStack}
fn ours(id:&str)->bool{id.starts_with("storagedrawers:")}
fn name_at(game:&Game<'_>,pos:Pos)->String{game.native_block(pos)}
fn kind(block:&str)->&str{block.strip_prefix("storagedrawers:").unwrap_or("")}
fn drawer(block:&str)->bool{kind(block).contains("drawers_")}
fn controller(block:&str)->bool{matches!(kind(block),"controller"|"framed_controller")}
fn port(block:&str)->bool{matches!(kind(block),"controller_io"|"framed_controller_io")}
fn trim(block:&str)->bool{kind(block).ends_with("_trim")}
fn button_key(block:&str)->Option<sd::Key>{Some(match kind(block){
    "keybutton_drawer"=>sd::Key::Lock,"keybutton_quantify"=>sd::Key::Quantify,
    "keybutton_concealment"=>sd::Key::Shroud,_=>return None})}
fn slots(block:&str)->usize{block.rsplit('_').next().and_then(|s|s.parse().ok()).filter(|n|[1,2,3,4].contains(n)).unwrap_or(1)}
fn entity<'a>(game:&'a Game<'_>,pos:Pos)->Option<&'a ModBlockEntity>{
    match game.world.block_entities.get(&(pos[0],pos[1],pos[2])){Some(BlockEntity::Mod(m))if ours(&m.id)=>Some(m),_=>None}
}
fn group(entity:&ModBlockEntity)->Result<sd::Group,sd::Error>{
    match entity.nbt.get(&SAVE.into()){Some(Tag::ByteArray(bytes))=>sd::codec::decode(bytes),_=>Err(sd::Error::InvalidData)}
}
fn block_name(entity:&ModBlockEntity)->String{
    match entity.nbt.get(&"sd_block".into()){Some(Tag::String(s))=>s.to_string_lossy(),_=>entity.id.clone()}
}
fn key(game:&Game<'_>,stack:&ItemStack)->sd::ItemKey{
    let mut one=stack.clone();one.count=1;
    sd::ItemKey{id:stack.item.to_string(),components:horizon_client::snbt::to_snbt(&horizon_items::item_nbt::item_to_nbt(&one)),
        max_stack:game.stack_max_size(stack).clamp(1,99)}
}
fn conversion_key(game:&Game<'_>,g:&sd::Group,slot:usize,input:&sd::ItemKey)->sd::ItemKey{
    if !g.has(sd::Upgrade::Conversion){return input.clone();}
    let Some(target)=g.item(slot)else{return input.clone();};if target.id==input.id{return input.clone();}
    let config=feature_config();
    let equivalent=config["conversion_equivalence_groups"].as_array().is_some_and(|groups|groups.iter().any(|group|{
        group.as_array().is_some_and(|v|v.iter().any(|i|i.as_str()==Some(&input.id))&&v.iter().any(|i|i.as_str()==Some(&target.id)))
    }));
    let tagged=game.items().item_tags.iter().any(|(tag,items)|{
        if !items.contains(input.id.as_str())||!items.contains(target.id.as_str()){return false;}
        let denied=config["conversion_tag_denylist"].as_array().is_some_and(|v|v.iter().any(|t|t.as_str()==Some(tag.as_str())));
        let allowed=config["conversion_tag_allowlist"].as_array().is_some_and(|v|v.iter().any(|t|t.as_str()==Some(tag.as_str())));
        !denied&&allowed
    });
    if equivalent||tagged{target.clone()}else{input.clone()}
}
fn insert_group(game:&Game<'_>,g:&mut sd::Group,slot:usize,input:&sd::Stack,access:sd::Access,simulate:bool)->Result<sd::Inserted,sd::Error>{
    let item=conversion_key(game,g,slot,&input.item);g.insert(slot,&sd::Stack{item,count:input.count},access,simulate)
}
pub(super) fn feature_config()->&'static serde_json::Value{
    static CONFIG:std::sync::OnceLock<serde_json::Value>=std::sync::OnceLock::new();
    CONFIG.get_or_init(||{
        let mut tags=Vec::new();for kind in ["storage_blocks","ingots","nuggets"]{for material in ["aluminum","constantan","steel","uranium","invar","tin","lead","silver","platinum","nickel","osmium","bronze","electrum"]{tags.push(format!("c:{kind}/{material}"));}}
        let mut config=serde_json::json!({"conversion_tag_allowlist":tags,"conversion_tag_denylist":[],"conversion_equivalence_groups":[],
            "controller_range":50,"remote_range":0,"remote_group_range":0,"framing_solid_materials":true,"framing_opaque_materials":false,
            "framing_material_denylist":[],"detached_force_capacity_check":false,"detached_heavy":false,"redstone_analog_output":true});
        let path=std::path::Path::new("horizon-mods/.horizon-mod-data/storagedrawers/features.json");
        if let Ok(bytes)=std::fs::read(path){if let Ok(serde_json::Value::Object(values))=serde_json::from_slice::<serde_json::Value>(&bytes){
            for (k,v) in values{config[&k]=v;}
        }}else if let Some(parent)=path.parent(){if std::fs::create_dir_all(parent).is_ok(){let _=std::fs::write(path,serde_json::to_vec_pretty(&config).unwrap());}}
        config
    })
}
fn controller_range()->u32{feature_config()["controller_range"].as_u64().unwrap_or(50).clamp(1,512)as u32}
// Called after vanilla's predicted BlockItem component application, before
// this placement can be rendered. The held stack is supplied by that API,
// including off-hand placement and a last item consumed by the placement.
pub(super) fn predicted_item(game:&mut Game<'_>,pos:Pos,input:&ItemStack){
    if game.remote.is_none()||!ours(&input.item)||name_at(game,pos)!=&*input.item{return;}
    let frame=super::sd_items::frame(input);let contents=super::sd_items::group(input);
    let custom=super::sd_items::custom(input);
    let Some(BlockEntity::Mod(m))=game.world.block_entities.get_mut(&(pos[0],pos[1],pos[2]))else{return;};
    if m.id!=&*input.item{return;}
    if let Some(frame)=frame{m.nbt.insert("sd_frame".into(),frame);}
    if let Some(g)=contents{
        let positions=super::sd_items::positions(&custom,&g);let items=super::sd_items::upgrade_items(&custom,&g,&positions);
        save_upgrade_state(m,&g,&positions,&items);
    }
}
// Build a rule snapshot for the next Menu instance, using the same server
// predicates. The snapshot becomes owned by that instance, not its window ID.
pub(super) fn prepare_menu_rules(game:&Game<'_>){
    use horizon_client::menu::sd_rules;
    let context=(||{
        let target=game.native_screen_pos()?;
        let (_,menu)=game.remote_menu();
        if !matches!(menu.as_str(),"storagedrawers:drawers"|"storagedrawers:framing"){return None;}
        let block=name_at(game,target);if !ours(&block){return None;}
        let mut candidates:Vec<ItemStack>=game.inventory.iter().chain(std::iter::once(&game.offhand)).chain(std::iter::once(&game.carried)).flatten().cloned().collect();
        if let Some(m)=game.container.and_then(|p|game.world.block_entities.get(&p)){candidates.extend(m.slots().iter().flatten().cloned());}
        let mut unique:Vec<ItemStack>=Vec::new();for item in candidates{
            if !unique.iter().any(|s|s.same_item_same_components(&item)){unique.push(item);}
        }
        if menu=="storagedrawers:framing"&&block=="storagedrawers:framing_table"{
            let materials=unique.iter().filter(|s|super::sd_framing::material(game,s)).map(|s|s.item.to_string()).collect();
            return Some(sd_rules::framing(materials));
        }
        if menu!="storagedrawers:drawers"||!drawer(&block){return None;}
        let g=entity(game,target).and_then(|m|group(m).ok())?;
        let mut deposits=Vec::new();for input in unique{
            let mut probe=input.clone();probe.count=u32::MAX;
            let limits=(0..g.drawers.len()).map(|slot|{
                let mut state=DrawerMenu{g:g.clone(),positions:Vec::new(),upgrades:Vec::new(),inv:Vec::new(),offhand:None,carried:None};
                state.deposit(game,&block,9+slot as i32,&probe,u32::MAX,sd::Access::Administrator)
            }).collect();deposits.push((input,limits));
        }
        sd_rules::drawers(&sd::codec::encode(&g).ok()?,deposits)
    })();
    sd_rules::pending(context);
}
fn network_insert(game:&Game<'_>,net:&mut sd::Network,origin:&sd::Position,input:&sd::Stack,access:sd::Access,simulate:bool)->sd::Inserted{
    // Use a transaction for both real and simulated routing. Java first fills
    // matching stock, then overflow/void capacity, then unassigned drawers for
    // automation. Player controller clicks intentionally do not assign empties.
    let mut working=net.clone();let mut routes=Vec::new();
    for p in working.connected(origin,controller_range()){if let Some(sd::Node::Drawers(g))=working.nodes.get(&p){
        if g.check(access).is_err(){continue;}
        for slot in 0..g.drawers.len(){
            if g.drawers[slot].detached{continue;}
            if let Some(target)=g.item(slot){
                if target.same_item(&conversion_key(game,g,slot,&input.item)){
                    routes.push((0,g.attributes.priority,p.clone(),slot));routes.push((1,g.attributes.priority,p.clone(),slot));
                }
            }else if !matches!(access,sd::Access::Player(_))&&!g.attributes.locked{
                routes.push((2,g.attributes.priority,p.clone(),slot));
            }
        }
    }}
    routes.sort_by(|a,b|a.0.cmp(&b.0).then(b.1.cmp(&a.1)).then(a.2.cmp(&b.2)).then(a.3.cmp(&b.3)));
    let mut remaining=input.count;let mut result=sd::Inserted::default();
    for (phase,_,p,slot) in routes{
        if remaining==0{break;}
        let Some(sd::Node::Drawers(original))=working.nodes.get(&p)else{continue;};let mut g=original.clone();let mut target=slot;
        let block=name_at(game,p.xyz);
        if kind(&block).contains("compacting")&&g.compact.is_none(){
            let Some(stock)=stack(&input.item,1)else{continue;};seed(game,&block,&mut g,&stock);
            let Some(c)=&g.compact else{continue;};let Some(slot)=c.views.iter().position(|(i,_)|i.same_item(&input.item))else{continue;};target=slot;
        }
        let offered=if phase==0{g.item(target).map_or(0,|i|g.capacity(target,i).saturating_sub(g.count(target))).min(remaining)}else{remaining};
        if offered==0{continue;}
        if let Ok(put)=insert_group(game,&mut g,target,&sd::Stack{item:input.item.clone(),count:offered},access,false){
            if put.accepted()>0{working.nodes.insert(p,sd::Node::Drawers(g));remaining-=put.accepted();result.stored+=put.stored;result.voided+=put.voided;}
        }
    }
    if !simulate{*net=working;}result
}
pub(super) fn single_stack_drawer(stack:&ItemStack)->bool{
    drawer(&stack.item)&&matches!(stack.data().and_then(|d|d.other("minecraft:max_stack_size")),Some(Tag::Int(1)))
}
fn stack(item:&sd::ItemKey,count:u64)->Option<ItemStack>{
    let mut s=if item.components.is_empty(){ItemStack::new(&item.id,1)}else{
        let (tag,n)=horizon_world::nbt::parse_snbt(&item.components).ok()?;
        if n!=item.components.len(){return None;}horizon_items::item_nbt::item_from_nbt(&tag)?
    };s.count=u32::try_from(count).ok()?;Some(s)
}
// Keep physical slots separate from the packed, backwards-compatible core save.
// Old worlds have no sidecar and retain their original left-to-right positions.
fn upgrade_positions(entity:&ModBlockEntity,g:&sd::Group)->Vec<usize>{
    positions_from_nbt(&entity.nbt,g)
}
fn positions_from_nbt(nbt:&Compound,g:&sd::Group)->Vec<usize>{
    let saved=match nbt.get(&UPGRADE_POS.into()){Some(Tag::IntArray(v))=>v.as_slice(),_=>&[]};
    let mut positions=Vec::new();
    for i in 0..g.upgrades.len(){
        let pos=saved.get(i).and_then(|&p|usize::try_from(p).ok()).filter(|p|*p<sd::UPGRADE_SLOTS&&!positions.contains(p))
            .unwrap_or_else(||(0..sd::UPGRADE_SLOTS).find(|p|!positions.contains(p)).unwrap());
        positions.push(pos);
    }positions
}
fn save_positions(entity:&mut ModBlockEntity,g:&sd::Group,positions:&[usize])->bool{
    let items=upgrade_items_from_nbt(&entity.nbt,g,positions);
    save_upgrade_state(entity,g,positions,&items)
}
fn upgrade_items_from_nbt(nbt:&Compound,g:&sd::Group,positions:&[usize])->Vec<ItemStack>{
    let saved=match nbt.get(&UPGRADE_ITEMS.into()){Some(Tag::List(v))=>v.as_slice(),_=>&[]};
    g.upgrades.iter().zip(positions).map(|(&u,&physical)|{
        saved.iter().find(|entry|matches!(entry.get("slot"),Some(Tag::Int(p))if *p==physical as i32))
            .and_then(|entry|horizon_items::item_nbt::item_from_nbt(entry.get("item")?))
            .filter(|s|s.count>0&&upgrade(&s.item)==Some(u))
            .map(|mut s|{s.count=1;s}).unwrap_or_else(||{
                let mut s=upgrade_stack(u);
                if matches!(u,sd::Upgrade::Remote|sd::Upgrade::RemoteGroup){if let Some(remote)=nbt.get(&"sd_remote".into()){
                    s.data_mut().set_other("minecraft:custom_data",Some(remote.clone()));
                }}s
            })
    }).collect()
}
fn encode_upgrade_items(positions:&[usize],items:&[ItemStack])->Tag{
    Tag::List(positions.iter().zip(items).map(|(&pos,item)|Tag::compound([
        ("slot",Tag::Int(pos as i32)),("item",horizon_items::item_nbt::item_to_nbt(item))])).collect())
}
fn save_upgrade_state(entity:&mut ModBlockEntity,g:&sd::Group,positions:&[usize],items:&[ItemStack])->bool{
    if positions.len()!=g.upgrades.len()||positions.iter().any(|&p|p>=sd::UPGRADE_SLOTS)
        ||positions.iter().collect::<BTreeSet<_>>().len()!=positions.len()||items.len()!=g.upgrades.len()
        ||items.iter().zip(&g.upgrades).any(|(s,&u)|s.count!=1||upgrade(&s.item)!=Some(u)){return false;}
    let Ok(bytes)=sd::codec::encode(g)else{return false;};
    entity.nbt.insert(SAVE.into(),Tag::ByteArray(bytes));
    entity.nbt.insert(UPGRADE_POS.into(),Tag::IntArray(positions.iter().map(|&p|p as i32).collect()));
    entity.nbt.insert(UPGRADE_ITEMS.into(),encode_upgrade_items(positions,items));
    entity.nbt.remove(&"sd_remote".into());
    project(entity,g);true
}
fn save(entity:&mut ModBlockEntity,g:&sd::Group)->bool{
    let old=group(entity).ok();
    let old_positions=old.as_ref().map(|g|upgrade_positions(entity,g)).unwrap_or_default();
    let mut used=BTreeSet::new();
    let mut positions:Vec<Option<usize>>=g.upgrades.iter().map(|u|{
        let index=old.as_ref()?.upgrades.iter().enumerate().find(|(i,v)|*v==u&&!used.contains(i))?.0;
        used.insert(index);Some(old_positions[index])
    }).collect();
    let mut taken:BTreeSet<usize>=positions.iter().flatten().copied().collect();
    for p in &mut positions{if p.is_none(){let Some(free)=(0..sd::UPGRADE_SLOTS).find(|p|!taken.contains(p))else{return false;};
        taken.insert(free);*p=Some(free);}}
    save_positions(entity,g,&positions.into_iter().flatten().collect::<Vec<_>>())
}
const IO_LAYOUT:i32=2;
fn io_count(g:&sd::Group,slot:usize)->u64{if g.has(sd::Upgrade::CreativeVending){sd::MAX_COUNT}else{g.count(slot)}}
fn project(entity:&mut ModBlockEntity,g:&sd::Group){
    // Java's handler slot 0 is an empty insertion-only port, not real stock.
    let count=slots(&block_name(entity));
    entity.items=std::iter::once(None).chain((0..count).map(|slot|{
        if g.check(sd::Access::Automation).is_err()||slot>=g.drawers.len()||g.drawers[slot].detached{return None;}
        g.item(slot).and_then(|i|stack(i,io_count(g,slot))).filter(|s|s.count>0)
    })).collect();
    entity.nbt.insert("sd_io_layout".into(),Tag::Int(IO_LAYOUT));
}
fn set_group(game:&mut Game<'_>,pos:Pos,g:&sd::Group)->bool{
    set_group_positions(game,pos,g,None,None)
}
fn set_group_positions(game:&mut Game<'_>,pos:Pos,g:&sd::Group,positions:Option<&[usize]>,items:Option<&[ItemStack]>)->bool{
    let result=game.native_with_mod_block_entity(pos,|m|match (positions,items){
        (Some(p),Some(items))=>save_upgrade_state(m,g,p,items),(Some(p),None)=>save_positions(m,g,p),_=>save(m,g)}).unwrap_or(false);
    if result{
        if drawer(&name_at(game,pos)){game.native_set_block_property(pos,"hopper",if g.has(sd::Upgrade::Hopper){"true"}else{"false"});}
        if kind(&name_at(game,pos)).contains("compacting"){
            let value=match g.compact.as_ref().map_or(1,|c|c.views.len()){3=>"open3",2=>"open2",_=>"open1"};
            game.native_set_block_property(pos,"slots",value);
        }
        game.native_block_entity_sync(pos);let dim=game.dim.id();game.native_update_signal_neighbors(dim,pos);game.native_update_output_neighbors(dim,pos);
    }result
}
fn feedback(game:&mut Game<'_>,translation:&str){game.native_feedback(Text::tr(translation,&[]));}
fn upgrade(id:&str)->Option<sd::Upgrade>{sd::Upgrade::from_id(id.strip_suffix("_bound").unwrap_or(id))}
fn key_type(id:&str)->Option<sd::Key>{
    Some(match kind(id){"drawer_key"|"keyring_drawer"=>sd::Key::Lock,"quantify_key"|"keyring_quantify"=>sd::Key::Quantify,
        "shroud_key"|"keyring_shroud"=>sd::Key::Shroud,"suspend_key"|"keyring_suspend"=>sd::Key::Suspend,
        "personal_key"|"keyring_personal"=>sd::Key::Personal,"personal_key_unlock"|"keyring_personal_unlock"=>sd::Key::UnlockPersonal,
        s if s.starts_with("priority_key")||s.starts_with("keyring_priority")=>sd::Key::Priority,_=>return None})
}
pub(super) fn init(block:&str,out:&mut Option<(String,Compound)>){
    if !ours(block)||button_key(block).is_some()||out.is_some(){return;}
    let mut nbt=Compound::new();nbt.insert("sd_block".into(),Tag::String(block.into()));
    if drawer(block){
        let n=if kind(block).contains("compacting"){1}else{slots(block)};
        if let Ok(g)=sd::Group::standard(n,kind(block).contains("half")){
            if let Ok(bytes)=sd::codec::encode(&g){nbt.insert(SAVE.into(),Tag::ByteArray(bytes));}
        }
    }*out=Some((block.into(),nbt));
}
pub(super) fn container(block:&str,out:&mut Option<ModContainer>){
    if block=="storagedrawers:framing_table"{*out=Some(ModContainer{slots:4,faces:std::array::from_fn(|_|Vec::new()),inputs:Vec::new(),outputs:Vec::new(),menu:Some("storagedrawers:framing".into())});return;}
    if !(drawer(block)||controller(block)||port(block)){return;}
    let n=if drawer(block){slots(block)+1}else{1};let indices:Vec<_>=(0..n).collect();
    *out=Some(ModContainer{slots:n,faces:std::array::from_fn(|_|indices.clone()),inputs:indices,outputs:(1..n).collect(),
        menu:drawer(block).then(||"storagedrawers:drawers".into())});
}
pub(super) fn screen(block:&str,out:&mut Option<horizon_client::mod_api::NativeScreen>){
    super::sd_framing::screen(block,out);
    if !drawer(block){return;}
    let mut positions=vec![None;18];
    for i in 0..7{positions[i]=Some([26.+i as f32*18.,86.]);}
    let cells:&[[f32;2]]=match slots(block){
        1=>&[[80.,36.]],2=>&[[80.,23.],[80.,49.]],
        3=>&[[80.,23.],[67.,49.],[93.,49.]],
        _=>&[[67.,23.],[93.,23.],[67.,49.],[93.,49.]],
    };
    for (i,&at) in cells.iter().enumerate(){positions[9+i]=Some(at);}
    *out=Some(horizon_client::mod_api::NativeScreen{width:176.,height:199.,slots:positions,
        inventory_y:117.,inventory_x:8.,own_labels:true,own_background:true});
}
pub(super) fn screen_draw(game:&Game<'_>,block:&str,batch:&mut horizon_client::hud::HudBatch,origin:[f32;2]){
    super::sd_framing::screen_draw(game,block,batch,origin);
    if !drawer(block){return;}
    let [x,y]=origin;
    let variant=if kind(block).contains("compacting"){if slots(block)==2{"comp_2"}else{"comp"}}
        else{match slots(block){2=>"2",4=>"4",_=>"1"}};
    game.native_sprite(batch,&format!("storagedrawers:gui/drawers_{variant}"),[0.,0.,176.,199.],[x,y,176.,199.]);
    for (key,at) in [(format!("block.storagedrawers.{}",kind(block)),[x+8.,y+6.]),
        ("container.storagedrawers.upgrades".into(),[x+8.,y+75.]),("container.inventory".into(),[x+8.,y+105.])]{
        game.native_text(batch,&Text::tr(&key,&[]).plain(),at,0x404040,false);
    }
    let mut layout=None;screen(block,&mut layout);
    if let (Some(layout),Some(menu))=(layout,game.container.and_then(|p|game.world.block_entities.get(&p))){
        for i in 0..slots(block){
            let Some(item)=menu.slots().get(9+i).and_then(Option::as_ref)else{continue;};
            let Some(Tag::Long(count))=item.data().and_then(|d|d.other("minecraft:custom_data")).and_then(|d|d.get("sd_count"))else{continue;};
            let Some(at)=layout.slots[9+i]else{continue;};
            let value=count_text((*count).max(0)as u64);let width=game.native_text_width(&value);
            game.native_text(batch,&value,[x+at[0]+8.-width/2.,y+at[1]+18.],0x404040,false);
        }
    }
}
pub(super) fn tooltip(item:&ItemStack,lines:&mut Vec<(Text,u32)>){
    if !ours(&item.item){return;}
    let key=format!("item.storagedrawers.{}.desc",kind(&item.item));
    let text=Text::tr(&key,&[]).plain();
    if text!=key&&!text.contains('%'){
        for line in text.split("\\n").flat_map(str::lines){lines.push((Text::literal(line),0xFFA0A0A0));}
    }
    if let Some((p,dimension))=super::sd_items::binding(item){lines.push((Text::literal(format!("控制器：{}, {}, {} ({dimension})",p[0],p[1],p[2])),0xFFFFFF55));}
    if let Some((stock,capacity,heavy))=super::sd_items::detached(item){
        lines.push((Text::literal(format!("容量：{capacity} 组")),0xFFA0A0A0));
        if let Some(stock)=stock{lines.push((Text::literal(format!("{} × {}",stock.item,stock.count)),0xFFA0A0A0));if heavy&&stock.count>64{lines.push((Text::tr("tooltip.storagedrawers.drawers.too_heavy",&[]),0xFFFF5555));}}
    }
    let keys=super::sd_items::ring_items(item);if !keys.is_empty(){
        let active=super::sd_items::active_key(item);for key in &keys{let selected=active.as_ref().is_some_and(|a|a.same_item_same_components(key));
            let name=Text::tr(&format!("item.storagedrawers.{}",kind(&key.item)),&[]).plain();
            lines.push((Text::literal(format!("{} {name}",if selected{"▶"}else{"·"})),if selected{0xFFFFFF55}else{0xFFA0A0A0}));
        }
    }
    if drawer(&item.item){lines.push((Text::tr("storagedrawers.tooltip.drawer_use",&[]),0xFFA0A0A0));}
    if let Some(multiplier)=upgrade(&item.item).and_then(sd::Upgrade::multiplier){
        let text=Text::tr("storagedrawers.tooltip.multiplier",&[]).plain().replace("%s",&multiplier.to_string());
        lines.push((Text::literal(text),0xFFA0A0A0));
    }
}
pub(super) fn ticking(block:&str,ticks:&mut bool){if drawer(block)||controller(block)||port(block)||block=="storagedrawers:framing_table"{*ticks=true;}}
pub(super) fn update_tag(entity:&ModBlockEntity,tag:&mut Option<Compound>){if ours(&entity.id){let mut data=entity.nbt.clone();super::sd_framing::update_tag(entity,&mut data);*tag=Some(data);}}

fn family(game:&Game<'_>,input:&ItemStack,n:usize,half:bool)->Option<sd::Group>{
    if input.damage!=0||input.extra.is_some(){return None;}
    const FAMILIES:&[&[(&str,u64)]]=&[
        &[("iron_block",81),("iron_ingot",9),("iron_nugget",1)],&[("gold_block",81),("gold_ingot",9),("gold_nugget",1)],
        &[("copper_block",9),("copper_ingot",1)],&[("diamond_block",9),("diamond",1)],&[("emerald_block",9),("emerald",1)],
        &[("lapis_block",9),("lapis_lazuli",1)],&[("redstone_block",9),("redstone",1)],&[("coal_block",9),("coal",1)],
        &[("netherite_block",9),("netherite_ingot",1)],&[("raw_iron_block",9),("raw_iron",1)],
        &[("raw_gold_block",9),("raw_gold",1)],&[("raw_copper_block",9),("raw_copper",1)],
        &[("slime_block",9),("slime_ball",1)],&[("hay_block",9),("wheat",1)],&[("bone_block",9),("bone_meal",1)],
        &[("dried_kelp_block",9),("dried_kelp",1)],&[("snow_block",4),("snowball",1)],&[("clay",4),("clay_ball",1)],
    ];
    let id=input.item.strip_prefix("minecraft:")?;
    let Some(f)=FAMILIES.iter().find(|f|f.iter().any(|i|i.0==id))else{
        return sd::Group::compacting(vec![(key(game,input),1)],half).ok();
    };
    let start=if n==2&&f.len()==3&&id==f[2].0{1}else{0};let f=&f[start..];let base=f.last()?.1;
    let views=f.iter().map(|(i,factor)|(key(game,&ItemStack::new(&format!("minecraft:{i}"),1)),factor/base)).collect();
    sd::Group::compacting(views,half).ok()
}
fn seed(game:&Game<'_>,block:&str,g:&mut sd::Group,input:&ItemStack){
    if kind(block).contains("compacting")&&g.compact.is_none()&&g.count(0)==0{
        if let Some(mut c)=family(game,input,slots(block),g.half){c.upgrades=g.upgrades.clone();c.attributes=g.attributes.clone();c.frame=g.frame.clone();*g=c;}
    }
}
fn facing(game:&Game<'_>,pos:Pos)->&'static str{
    match game.native_block_property(pos,"facing").as_deref(){Some("east")=>"east",Some("south")=>"south",Some("west")=>"west",_=>"north"}
}
fn front(game:&Game<'_>,pos:Pos,face:Pos)->bool{
    face==match facing(game,pos){"east"=>[1,0,0],"south"=>[0,0,1],"west"=>[-1,0,0],_=>[0,0,-1]}
}
fn clicked_slot(game:&Game<'_>,pos:Pos,n:usize)->Option<usize>{
    let p=game.player.as_ref()?;
    let eye=game.eye_position().unwrap_or([p.position.x,p.position.y+1.62,p.position.z]);
    let dir=super::view_vector(p.yaw,p.pitch);
    let half=kind(&name_at(game,pos)).contains("half");
    let inset=if half{0.5}else{1./16.};
    let (axis,plane)=match facing(game,pos){"east"=>(0,pos[0]as f64+1.-inset),"west"=>(0,pos[0]as f64+inset),"south"=>(2,pos[2]as f64+1.-inset),_=>(2,pos[2]as f64+inset)};
    if dir[axis].abs()<=1e-8{return None;}
    let t=(plane-eye[axis])/dir[axis];if !t.is_finite()||t<0.{return None;}
    let y=eye[1]+dir[1]*t-pos[1]as f64;
    let horizontal=if axis==2{eye[0]+dir[0]*t-pos[0]as f64}else{eye[2]+dir[2]*t-pos[2]as f64};
    // Java's FaceSlotBlock excludes the wooden 1-pixel border and treats
    // the exact middle line as the lower/right drawer.
    if !(1./16.<y&&y<15./16.&&1./16.<horizontal&&horizontal<15./16.){return None;}
    let left=if matches!(facing(game,pos),"south"|"west"){horizontal<0.5}else{horizontal>0.5};
    let bottom=y<=0.5;let right=!left;
    Some(match n{2=>usize::from(bottom),3=>if !bottom{0}else{1+usize::from(right)},4=>2*usize::from(bottom)+usize::from(right),_=>0})
}
fn consume_held(game:&mut Game<'_>,n:u32){
    if let Some(s)=game.held_mut().as_mut(){s.count=s.count.saturating_sub(n);if s.count==0{*game.held_mut()=None;}}
}
fn handle_detached(game:&mut Game<'_>,pos:Pos,face:Pos,slot:usize,held:Option<&ItemStack>,access:sd::Access)->bool{
    let Some(held)=held else{return false;};let pulling=kind(&held.item)=="drawer_puller";
    let putting=super::sd_items::detached(held);if !pulling&&putting.is_none(){return false;}
    let Some(m)=entity(game,pos)else{return true;};let Ok(mut g)=group(m)else{return true;};
    if g.compact.is_some()||g.check(access).is_err(){return true;}
    if pulling{
        let cap=if g.has(sd::Upgrade::OneStack){g.multiplier()}else{g.base_stacks()*g.multiplier()};
        let Ok(out)=g.detach(slot,access)else{return true;};
        let stock=out.drawer.item.as_ref().and_then(|i|stack(i,out.drawer.count)).filter(|s|s.count>0);
        let item=super::sd_items::make_detached(stock,cap,feature_config()["detached_heavy"].as_bool().unwrap_or(false)&&!g.has(sd::Upgrade::Portability));
        if set_group(game,pos,&g){if let Some(p)=game.native_acting_player(){if let Some(rest)=game.native_give_stack(p.uuid,item){
            let at=std::array::from_fn(|i|f64::from(pos[i])+0.5+f64::from(face[i])*0.75);game.spawn_item(rest,at,[0.,0.1,0.],10);
        }}}
    }else if let Some((stock,capacity,_))=putting{
        let dest_cap=if g.has(sd::Upgrade::OneStack){g.multiplier()}else{g.base_stacks()*g.multiplier()};
        if feature_config()["detached_force_capacity_check"].as_bool().unwrap_or(false)&&capacity<dest_cap{return true;}
        let drawer=sd::DetachedDrawer{drawer:sd::Drawer{item:stock.as_ref().map(|s|key(game,s)),count:stock.as_ref().map_or(0,|s|u64::from(s.count)),detached:false},capacity};
        if g.attach(slot,&drawer,access).is_ok()&&set_group(game,pos,&g){consume_held(game,1);}
    }true
}
pub(super) fn use_item(game:&mut Game<'_>,stack:&ItemStack,claimed:&mut bool){
    if *claimed||!game.native_on(ID)||!game.native_sneaking(){return;}
    if kind(&stack.item).starts_with("priority_key"){
        let next=match kind(&stack.item){"priority_key"=>"priority_key_p1","priority_key_p1"=>"priority_key_p2","priority_key_p2"=>"priority_key_n1","priority_key_n1"=>"priority_key_n2",_=>"priority_key"};
        let mut out=stack.clone();out.item=format!("storagedrawers:{next}").into();out.count=1;*game.held_mut()=Some(out);*claimed=true;return;
    }
    if !kind(&stack.item).starts_with("keyring"){return;}
    let keys=super::sd_items::ring_items(stack);if keys.is_empty(){return;}
    let c=super::sd_items::custom(stack);let active=match c.get(&"sd_active_key".into()){Some(Tag::Int(v))=>(*v).max(0)as usize,_=>0};
    let mut out=stack.clone();super::sd_items::save_ring(&mut out,&keys,(active+1)%keys.len());*game.held_mut()=Some(out);*claimed=true;
}
fn validate_remote(game:&Game<'_>,pos:Pos,item:&ItemStack)->ItemStack{
    if !matches!(upgrade(&item.item),Some(sd::Upgrade::Remote|sd::Upgrade::RemoteGroup)){return item.clone();}
    let Some((controller_pos,dim))=super::sd_items::binding(item)else{return item.clone();};
    let group=upgrade(&item.item)==Some(sd::Upgrade::RemoteGroup);
    let range=feature_config()[if group{"remote_group_range"}else{"remote_range"}].as_u64().unwrap_or(0);let range=if range==0{controller_range()}else{range.min(u64::from(controller_range()))as u32};
    let wrong=dim!=game.dim.id()||!position(game,pos).in_range(&position(game,controller_pos),range)
        ||game.world.loaded(controller_pos[0],controller_pos[2])&&!controller(&name_at(game,controller_pos));
    if wrong{super::sd_items::unbound(item)}else{item.clone()}
}
pub(super) fn right_click(game:&mut Game<'_>,pos:Pos,face:Pos,cancel:&mut bool){
    let block=name_at(game,pos);if !ours(&block)||!game.native_on(ID)||*cancel{return;}
    if let Some(key)=button_key(&block){
        // Leave the press, swing, sound and scheduled release to the common
        // button implementation. Only its Storage Drawers action is added.
        if game.hand!=super::Hand::Main||game.native_sneaking()&&(game.held().is_some()||game.offhand.is_some())
            ||game.native_block_property(pos,"powered").as_deref()==Some("true"){return;}
        let Some(player)=game.native_acting_player()else{return;};
        let facing=game.native_block_property(pos,"facing").unwrap_or_default();
        let offset=match game.native_block_property(pos,"face").as_deref(){Some("floor")=>[0,-1,0],Some("ceiling")=>[0,1,0],
            _=>match facing.as_str(){"south"=>[0,0,-1],"west"=>[1,0,0],"east"=>[-1,0,0],_=>[0,0,1]}};
        let backing=std::array::from_fn(|i|pos[i]+offset[i]);let target=name_at(game,backing);
        let root=if controller(&target){Some(backing)}else if port(&target){controller_for(game,backing)}else{None};
        if let Some(root)=root{let mut net=network(game,root);net.use_key(&position(game,root),key,player.uuid);commit_network(game,&net);}
        return;
    }
    if super::sd_framing::right_click(game,pos,face,cancel){return;}
    // Let normal item placement handle sneak-use while holding any block.
    // This must run before the drawer claims the click as an inventory insertion.
    let block_in_hand=|s:&ItemStack|game.items().get(&s.item).is_some_and(|i|i.block.is_some());
    if game.native_sneaking()&&(game.held().is_some_and(block_in_hand)||game.offhand.as_ref().is_some_and(block_in_hand)){return;}
    let Some(player)=game.native_acting_player()else{return;};
    let held=game.held().cloned();let access=sd::Access::Player(player.uuid);
    if game.native_sneaking()&&held.as_ref().is_some_and(|s|kind(&s.item).starts_with("keyring")){return;}
    let tool=held.as_ref().and_then(super::sd_items::active_key).or_else(||held.clone());
    let priority=tool.as_ref().map_or(0,|s|match kind(&s.item){"priority_key_p1"=>1,"priority_key_p2"=>2,"priority_key_n1"=>-1,"priority_key_n2"=>-2,_=>0});
    if drawer(&block)&&front(game,pos,face){if let Some(slot)=clicked_slot(game,pos,slots(&block)){
        if handle_detached(game,pos,face,slot,tool.as_ref(),access){*cancel=true;return;}
    }}
    if let Some(k)=tool.as_ref().and_then(|s|key_type(&s.item)){
        *cancel=true;
        if KEY_TIMES.with_borrow_mut(|times|{let old=times.get(&player.uuid).copied();
            if old.is_some_and(|t|game.world.game_time-t<5){true}else{times.insert(player.uuid,game.world.game_time);false}}){return;}
        if controller(&block){let mut net=network(game,pos);let origin=position(game,pos);
            if priority!=0{for p in net.connected(&origin,controller_range()){if let Some(sd::Node::Drawers(g))=net.nodes.get_mut(&p){if g.check(access).is_ok(){g.attributes.priority=if g.attributes.priority==priority{0}else{priority};}}}}
            else{net.use_key(&origin,k,player.uuid);}commit_network(game,&net);
        }
        else if let Some(m)=entity(game,pos){if let Ok(mut g)=group(m){let old_priority=g.attributes.priority;if g.use_key(k,player.uuid).is_ok(){if priority!=0{g.attributes.priority=if old_priority==priority{0}else{priority};}set_group(game,pos,&g);}else{feedback(game,"storagedrawers.message.denied");}}}
        return;
    }
    if controller(&block){
        if !front(game,pos,face){return;}
        *cancel=true;
        if let Some(s)=held.as_ref().filter(|s|matches!(upgrade(&s.item),Some(sd::Upgrade::Remote|sd::Upgrade::RemoteGroup))){
            let bound=super::sd_items::bound(s,pos,game.dim.id());
            *game.held_mut()=Some(bound);feedback(game,"storagedrawers.message.bound");return;
        }
        let mut net=network(game,pos);let origin=sd::Position::new(game.dim.id(),pos);
        let bulk=CLICKS.with_borrow_mut(|c|{let old=c.insert(player.uuid,(game.dim.id().into(),pos,game.world.game_time));old.is_some_and(|(d,p,t)|d==game.dim.id()&&p==pos&&game.world.game_time-t<=10)});
        let indices:Vec<usize>=if bulk{(0..36).collect()}else{vec![game.held_slot()]};
        for i in indices{let Some(s)=game.stack_in(i).cloned()else{continue;};let put=network_insert(game,&mut net,&origin,&sd::Stack{item:key(game,&s),count:u64::from(s.count)},access,false);
            if put.accepted()>0{if let Some(s)=game.stack_in_mut(i).as_mut(){s.count-=put.accepted()as u32;if s.count==0{*game.stack_in_mut(i)=None;}}}}
        commit_network(game,&net);return;
    }
    if !drawer(&block){return;}
    // Upgrades and the empty-hand sneak gesture work on any face. Other side
    // clicks must reach vanilla item use/placement rather than being swallowed.
    let gui=game.native_sneaking()&&held.is_none();
    let installing=!game.native_sneaking()&&held.as_ref().is_some_and(|s|upgrade(&s.item).is_some());
    if !front(game,pos,face)&&!gui&&!installing{return;}
    *cancel=true;
    let Some(m)=entity(game,pos)else{return;};let Ok(mut g)=group(m)else{feedback(game,"storagedrawers.message.corrupt");return;};
    if g.check(access).is_err(){feedback(game,"storagedrawers.message.denied");return;}
    if let Some(s)=held.as_ref().filter(|_|!game.native_sneaking()){if let Some(u)=upgrade(&s.item){
        let mut positions=upgrade_positions(m,&g);
        let mut items=upgrade_items_from_nbt(&m.nbt,&g,&positions);
        if matches!(u,sd::Upgrade::Remote|sd::Upgrade::RemoteGroup)&&super::sd_items::binding(s).is_some(){
            if let Some(i)=g.upgrades.iter().position(|u|matches!(u,sd::Upgrade::Remote|sd::Upgrade::RemoteGroup)){
                if let Some((p,d))=super::sd_items::binding(s){items[i]=validate_remote(game,pos,&super::sd_items::bound(&items[i],p,&d));set_group_positions(game,pos,&g,Some(&positions),Some(&items));}return;
            }
        }
        if g.add_upgrade(u,access).is_ok(){
            let free=(0..sd::UPGRADE_SLOTS).find(|p|!positions.contains(p)).unwrap();
            positions.push(free);let mut installed=validate_remote(game,pos,s);installed.count=1;items.push(installed);
            if set_group_positions(game,pos,&g,Some(&positions),Some(&items))&&player.game_mode!=1{consume_held(game,1);}
            if matches!(u,sd::Upgrade::Remote|sd::Upgrade::RemoteGroup){
                let custom=s.data().and_then(|d|d.other("minecraft:custom_data")).cloned();
                if let Some(custom)=custom{game.native_with_mod_block_entity(pos,|e|{e.nbt.insert("sd_remote".into(),custom);});}
            }
        }else{feedback(game,"storagedrawers.message.upgrade_failed");}return;
    }}
    if game.native_sneaking()&&held.is_none(){OPEN.with_borrow_mut(|q|q.push((player.uuid,game.dim.id().into(),pos)));return;}
    if !front(game,pos,face){return;}
    let Some(clicked)=clicked_slot(game,pos,slots(&block))else{*cancel=false;return;};
    if let Some(s)=&held{seed(game,&block,&mut g,s);}
    if kind(&block).contains("compacting")&&g.compact.is_none(){feedback(game,"storagedrawers.message.no_compaction");return;}
    let bulk=CLICKS.with_borrow_mut(|c|{let old=c.insert(player.uuid,(game.dim.id().into(),pos,game.world.game_time));
        old.is_some_and(|(d,p,t)|d==game.dim.id()&&p==pos&&game.world.game_time-t<10)});
    let indices:Vec<usize>=if bulk&&g.item(clicked.min(g.drawers.len()-1)).is_some(){(0..36).collect()}else{vec![game.held_slot()]};
    let old=g.clone();
    for index in indices{
        let Some(s)=game.stack_in(index).cloned()else{continue;};let k=key(game,&s);
        let slot=if let Some(c)=&g.compact{let Some(s)=c.views.iter().position(|(v,_)|v.same_item(&k))else{continue;};s}else{clicked};
        if let Ok(put)=insert_group(game,&mut g,slot,&sd::Stack{item:k,count:u64::from(s.count)},access,false){
            if put.accepted()>0{if let Some(s)=game.stack_in_mut(index).as_mut(){s.count-=put.accepted()as u32;if s.count==0{*game.stack_in_mut(index)=None;}}}
        }
    }
    if g!=old{set_group(game,pos,&g);}
}
pub(super) fn left_click(game:&mut Game<'_>,pos:Pos,face:Pos,cancel:&mut bool){
    let block=name_at(game,pos);if !drawer(&block)||*cancel||!game.native_on(ID){return;}
    if !front(game,pos,face){return;}
    let Some(slot)=clicked_slot(game,pos,slots(&block))else{return;};
    let Some(p)=game.native_acting_player()else{return;};
    // NeoForge cancels the front-panel creative break only. Survival's
    // initial attack takes an item and then continues normal block mining.
    *cancel=p.game_mode==1;
    let Some(m)=entity(game,pos)else{return;};let Ok(mut g)=group(m)else{return;};
    let n=if game.native_sneaking(){g.item(slot).map_or(64,|i|i.max_stack)}else{1};
    if let Ok(Some(out))=g.extract(slot,u64::from(n),sd::Access::Player(p.uuid),false){
        if let Some(s)=stack(&out.item,out.count){
            reset_compact(&mut g);
            if set_group(game,pos,&g){if let Some(rest)=game.native_give_stack(p.uuid,s){
                let at=std::array::from_fn(|i|f64::from(pos[i])+0.5+f64::from(face[i])*0.75);
                game.spawn_item(rest,at,[f64::from(face[0])*0.1,0.1,f64::from(face[2])*0.1],10);
            }}
        }
    }
}
pub(super) fn suppress_break_prediction(game:&Game<'_>,pos:Pos)->bool{
    if !game.creative||!game.native_on(ID)||!drawer(&name_at(game,pos)){return false;}
    game.pick().is_some_and(|hit|hit.pos==pos&&front(game,pos,hit.face)&&clicked_slot(game,pos,slots(&name_at(game,pos))).is_some())
}
pub(super) fn predict_drawer_use(game:&Game<'_>,hit:&super::Hit,held:Option<&ItemStack>,sneaking:bool)->bool{
    let block=name_at(game,hit.pos);if !game.native_on(ID){return false;}
    if block=="storagedrawers:framing_table"{return !sneaking||(held.is_none()&&game.offhand.is_none());}
    if !drawer(&block){return false;}
    let is_block=|s:&ItemStack|game.items().get(&s.item).is_some_and(|i|i.block.is_some());
    if sneaking&&(held.is_some_and(is_block)||game.offhand.as_ref().is_some_and(is_block)){return false;}
    if sneaking&&held.is_some_and(|s|kind(&s.item).starts_with("keyring")){return false;}
    // Predict consumption only, not inventory changes. Non-front uses pass to
    // ordinary item placement; front insertion must not place a ghost block.
    held.is_some_and(|s|key_type(&s.item).is_some())||sneaking&&held.is_none()
        ||held.is_some_and(|s|kind(&s.item).starts_with("keyring")||matches!(kind(&s.item),"drawer_puller"|"detached_drawer"|"detached_drawer_full"))&&front(game,hit.pos,hit.face)
        ||!sneaking&&held.is_some_and(|s|upgrade(&s.item).is_some())
        ||front(game,hit.pos,hit.face)&&clicked_slot(game,hit.pos,slots(&block)).is_some()
}
fn reset_compact(g:&mut sd::Group){
    if g.compact.as_ref().is_some_and(|c|c.units==0)&&!g.attributes.locked&&!g.has(sd::Upgrade::CreativeVending){
        g.compact=None;g.drawers=vec![sd::Drawer::default()];
    }
}

fn position(game:&Game<'_>,pos:Pos)->sd::Position{sd::Position::new(game.dim.id(),pos)}
fn network(game:&Game<'_>,_origin:Pos)->sd::Network{
    let mut net=sd::Network::default();net.range=controller_range();
    for (&(x,y,z),value) in game.world.block_entities.iter(){
        let p=[x,y,z];if !game.world.loaded(x,z){continue;}
        let BlockEntity::Mod(m)=value else{continue;};let block=block_name(m);
        let node=if drawer(&block){let Ok(g)=group(m)else{continue;};
            let positions=upgrade_positions(m,&g);let items=upgrade_items_from_nbt(&m.nbt,&g,&positions);
            for s in &items{if matches!(upgrade(&s.item),Some(sd::Upgrade::Remote|sd::Upgrade::RemoteGroup)){
                if let Some((target,dimension))=super::sd_items::binding(s){
                    let group=upgrade(&s.item)==Some(sd::Upgrade::RemoteGroup);let range=feature_config()[if group{"remote_group_range"}else{"remote_range"}].as_u64().unwrap_or(0);
                    if dimension==game.dim.id()&&(range==0||position(game,p).in_range(&position(game,target),range.min(u64::from(controller_range()))as u32)){
                        net.remotes.push(sd::RemoteBinding{drawer:position(game,p),controller:position(game,target),group});
                    }
                }
            }}sd::Node::Drawers(g)
        }else if controller(&block){sd::Node::Controller}else if trim(&block){sd::Node::Trim}else if port(&block){sd::Node::Port}else{continue;};
        net.nodes.insert(position(game,p),node);
    }
    net
}
fn commit_network(game:&mut Game<'_>,net:&sd::Network){
    for (p,n) in &net.nodes{if let sd::Node::Drawers(g)=n{if entity(game,p.xyz).and_then(|m|group(m).ok()).as_ref()!=Some(g){set_group(game,p.xyz,g);}}}
}
fn controller_for(game:&Game<'_>,port_pos:Pos)->Option<Pos>{
    game.world.block_entities.iter().filter_map(|(&(x,y,z),m)|{
        if let BlockEntity::Mod(m)=m{if controller(&block_name(m)){
            let p=[x,y,z];let net=network(game,p);if net.connected(&position(game,p),controller_range()).contains(&position(game,port_pos)){return Some(p);}
        }}None
    }).next()
}
// One handler contract for vanilla containers and every native mod caller.
// Slots: 0 is insertion-only; 1..=N are stable physical drawer records.
#[derive(Clone)]
pub(crate) struct ItemHandler{
    address:Pos,dimension:String,root:Option<Pos>,network:sd::Network,entries:Vec<(Pos,usize)>,before:BTreeMap<sd::Position,sd::Group>,suspended:bool,
}
pub(crate) fn item_handler(game:&Game<'_>,pos:Pos)->Option<ItemHandler>{
    let block=name_at(game,pos);if !(drawer(&block)||controller(&block)||port(&block)){return None;}
    let m=entity(game,pos)?;if m.id!=block||!game.world.loaded(pos[0],pos[2]){return None;}
    let net=if drawer(&block){
        let mut net=sd::Network::default();net.nodes.insert(position(game,pos),sd::Node::Drawers(group(m).ok()?));net
    }else{network(game,pos)};
    let (root,entries,suspended)=if drawer(&block){
        let g=group(m).ok()?;(None,(0..slots(&block)).map(|s|(pos,s)).collect(),g.attributes.suspended)
    }else{
        let root=if controller(&block){Some(pos)}else{controller_for(game,pos)};
        let mut entries=Vec::new();if let Some(root)=root{for p in net.connected(&position(game,root),controller_range()){
            if matches!(net.nodes.get(&p),Some(sd::Node::Drawers(_))){for slot in 0..slots(&name_at(game,p.xyz)){entries.push((p.xyz,slot));}}
        }}(root,entries,false)
    };
    let before=net.nodes.iter().filter_map(|(p,n)|match n{sd::Node::Drawers(g)=>Some((p.clone(),g.clone())),_=>None}).collect();
    Some(ItemHandler{address:pos,dimension:game.dim.id().into(),root,network:net,entries,before,suspended})
}
impl ItemHandler{
    pub(crate) fn slots(&self)->usize{if self.suspended{0}else{self.entries.len()+1}}
    fn location(&self,slot:usize)->Option<(Pos,usize)>{if slot==0||self.suspended{return None;}self.entries.get(slot-1).copied()}
    fn group_at(&self,p:Pos)->Option<&sd::Group>{match self.network.nodes.get(&sd::Position::new(&self.dimension,p)){Some(sd::Node::Drawers(g))=>Some(g),_=>None}}
    pub(crate) fn stack(&self,slot:usize)->Option<ItemStack>{
        let (p,s)=self.location(slot)?;let g=self.group_at(p)?;
        if s>=g.drawers.len()||g.drawers[s].detached||g.check(sd::Access::Automation).is_err(){return None;}
        g.item(s).and_then(|i|stack(i,io_count(g,s))).filter(|s|s.count>0)
    }
    pub(crate) fn limit(&self,game:&Game<'_>,slot:usize,prototype:Option<&ItemStack>)->u64{
        if slot>=self.slots(){return 0;}if slot==0{return sd::MAX_COUNT;}
        let Some((p,s))=self.location(slot)else{return 0;};let Some(g)=self.group_at(p)else{return 0;};
        if s>=g.drawers.len()||g.drawers[s].detached||g.check(sd::Access::Automation).is_err(){return 0;}
        let item=prototype.map(|s|key(game,s));let default=sd::ItemKey::plain("minecraft:stone",64);
        g.capacity(s,g.item(s).or(item.as_ref()).unwrap_or(&default))
    }
    pub(crate) fn simulate_insert(&self,game:&Game<'_>,slot:usize,input:&ItemStack)->ItemStack{
        let mut draft=self.clone();let n=draft.insert(game,slot,input);let n=if draft.valid(game){n}else{0};let mut rest=input.clone();rest.count-=n.min(u64::from(input.count))as u32;rest
    }
    pub(crate) fn simulate_extract(&self,slot:usize,count:u32)->Option<ItemStack>{let mut draft=self.clone();draft.extract(slot,count)}
    fn insert(&mut self,game:&Game<'_>,slot:usize,input:&ItemStack)->u64{
        if input.count==0||slot>=self.slots(){return 0;}
        let empty=|slot|self.location(slot).and_then(|(p,s)|self.group_at(p).map(|g|g.item(s).is_none())).unwrap_or(true);
        let scan=slot==0||slot>1&&empty(slot)&&!empty(slot-1);
        let input=sd::Stack{item:key(game,input),count:u64::from(input.count)};
        if scan{if let Some(root)=self.root{return network_insert(game,&mut self.network,&position(game,root),&input,sd::Access::Automation,false).accepted();}}
        let p=if scan{self.address}else{let Some((p,_))=self.location(slot)else{return 0;};p};
        let Some(original)=self.group_at(p)else{return 0;};let mut g=original.clone();
        if let Some(stock)=stack(&input.item,1){seed(game,&name_at(game,p),&mut g,&stock);}
        if kind(&name_at(game,p)).contains("compacting")&&g.compact.is_none(){return 0;}
        let targets:Vec<_>=if scan{(0..g.drawers.len()).collect()}else{vec![self.location(slot).unwrap().1]};
        let mut remaining=input.count;
        // Match Java's repository: use real free capacity before void overflow.
        for relaxed in [false,true]{for &target in &targets{
            if remaining==0{break;}if target>=g.drawers.len()||g.drawers[target].detached{continue;}
            if g.item(target).is_none()&&g.attributes.locked{continue;}
            let converted=conversion_key(game,&g,target,&input.item);
            let amount=if relaxed{remaining}else{g.capacity(target,g.item(target).unwrap_or(&converted)).saturating_sub(g.count(target)).min(remaining)};
            if amount==0{continue;}
            if let Ok(put)=insert_group(game,&mut g,target,&sd::Stack{item:input.item.clone(),count:amount},sd::Access::Automation,false){remaining-=put.accepted();}
        }}
        let accepted=input.count-remaining;if accepted>0{self.network.nodes.insert(position(game,p),sd::Node::Drawers(g));}accepted
    }
    fn extract(&mut self,slot:usize,count:u32)->Option<ItemStack>{
        let (p,s)=self.location(slot)?;let mut g=self.group_at(p)?.clone();
        let out=g.extract(s,u64::from(count),sd::Access::Automation,false).ok().flatten().and_then(|s|stack(&s.item,s.count))?;
        reset_compact(&mut g);
        self.network.nodes.insert(sd::Position::new(&self.dimension,p),sd::Node::Drawers(g));Some(out)
    }
    fn valid(&self,game:&Game<'_>)->bool{
        self.network.nodes.iter().all(|(p,n)|match n{
            sd::Node::Drawers(g)if self.before.get(p)!=Some(g)=>sd::codec::encode(g).is_ok()&&entity(game,p.xyz).and_then(|m|group(m).ok()).as_ref()==self.before.get(p),
            _=>true,
        })
    }
    fn commit(&self,game:&mut Game<'_>)->bool{
        // Preflight the complete transaction before writing any participating group.
        if !self.valid(game){return false;}
        let changed:Vec<_>=self.network.nodes.iter().filter_map(|(p,n)|match n{sd::Node::Drawers(g)if self.before.get(p)!=Some(g)=>Some((p,g)),_=>None}).collect();
        for (p,g) in changed{if !set_group(game,p.xyz,g){return false;}}
        true
    }
}
#[derive(Clone)]
struct IoRollback{time:i64,physical:Pos,before:sd::Group,after:sd::Group,item:ItemStack}
thread_local!{static IO_ROLLBACK:RefCell<BTreeMap<(String,Pos,usize),IoRollback>>=const{RefCell::new(BTreeMap::new())};}
pub(crate) fn io_insert(game:&mut Game<'_>,pos:Pos,slot:usize,input:&ItemStack,face:Option<Direction>,simulate:bool)->Option<(Option<ItemStack>,bool)>{
    let block=name_at(game,pos);if !(drawer(&block)||controller(&block)||port(&block)){return None;}
    let refused=||Some((Some(input.clone()),false));
    if !game.native_on(ID)||!(game.serving.is_some()||game.options.dedicated.is_some()){return refused();}
    let Some(mut handler)=item_handler(game,pos)else{return refused();};
    let Some(m)=entity(game,pos)else{return refused();};let mut cancel=false;game.native_container_accepts(pos,m,slot,input,face,&mut cancel);if cancel{return refused();}
    let accepted=handler.insert(game,slot,input).min(u64::from(input.count));
    if accepted>0&&!simulate{if !handler.commit(game){return refused();}refresh_controllers(game);}
    let mut rest=input.clone();rest.count-=accepted as u32;Some(((rest.count>0).then_some(rest),accepted>0))
}
pub(crate) fn io_extract(game:&mut Game<'_>,pos:Pos,slot:usize,count:u32,simulate:bool)->Option<Option<ItemStack>>{
    let block=name_at(game,pos);if !(drawer(&block)||controller(&block)||port(&block)){return None;}
    if !game.native_on(ID)||!(game.serving.is_some()||game.options.dedicated.is_some()){return Some(None);}
    let Some(mut handler)=item_handler(game,pos)else{return Some(None);};let out=handler.extract(slot,count);
    if out.is_some()&&!simulate{if !handler.commit(game){return Some(None);}refresh_controllers(game);}Some(out)
}
pub(super) fn auto_insert(game:&mut Game<'_>,pos:Pos,slot:usize,input:&ItemStack,face:Option<Direction>)->Option<(Option<ItemStack>,bool)>{io_insert(game,pos,slot,input,face,false)}
pub(super) fn auto_take(game:&mut Game<'_>,pos:Pos,slot:usize,count:u32)->Option<Option<ItemStack>>{
    let address=(game.dim.id().to_owned(),pos,slot);IO_ROLLBACK.with_borrow_mut(|q|{q.remove(&address);});
    let snapshot=item_handler(game,pos).and_then(|h|h.location(slot).and_then(|(p,_)|h.group_at(p).cloned().map(|g|(p,g))));
    let out=io_extract(game,pos,slot,count,false);
    if let (Some((p,before)),Some(Some(item)))=(snapshot,out.as_ref()){
        if let Some(after)=entity(game,p).and_then(|m|group(m).ok()){
            IO_ROLLBACK.with_borrow_mut(|q|{q.insert(address,IoRollback{time:game.world.game_time,physical:p,before,after,item:item.clone()});});
        }
    }out
}
pub(super) fn auto_put_back(game:&mut Game<'_>,pos:Pos,slot:usize,input:&ItemStack)->bool{
    let block=name_at(game,pos);if !(drawer(&block)||controller(&block)||port(&block)){return false;}
    let address=(game.dim.id().to_owned(),pos,slot);let rollback=IO_ROLLBACK.with_borrow_mut(|q|q.remove(&address));
    if let Some(r)=rollback{if r.time==game.world.game_time&&r.item==*input&&entity(game,r.physical).and_then(|m|group(m).ok())==Some(r.after){
        if set_group(game,r.physical,&r.before){refresh_controllers(game);return true;}
    }}
    // A refused move normally restores the snapshot above. If another native
    // callback changed the source meanwhile, retain ownership of any remainder.
    let mut rest=input.clone();
    if let Some(mut handler)=item_handler(game,pos){
        let upgrades:Vec<_>=handler.network.nodes.iter_mut().filter_map(|(p,n)|if let sd::Node::Drawers(g)=n{let old=g.upgrades.clone();g.upgrades.retain(|u|*u!=sd::Upgrade::Void);Some((p.clone(),old))}else{None}).collect();
        let accepted=handler.insert(game,0,input);
        for (p,upgrades) in upgrades{if let Some(sd::Node::Drawers(g))=handler.network.nodes.get_mut(&p){g.upgrades=upgrades;}}
        if handler.commit(game){rest.count-=accepted.min(u64::from(rest.count))as u32;refresh_controllers(game);}
    }
    if rest.count>0{game.spawn_item(rest,pos.map(|v|v as f64+0.5),[0.,0.1,0.],10);}true
}
// Native pipes use this same high-level transfer API. Re-read the handler
// before each unit so a cached display layout cannot select the wrong item.
pub(super) fn io_move(game:&mut Game<'_>,from:Pos,to:Pos,to_face:Direction,item:Option<&str>,max:u32)->Option<u32>{
    let block=name_at(game,from);if !(drawer(&block)||controller(&block)||port(&block)){return None;}
    if !game.native_on(ID)||!(game.serving.is_some()||game.options.dedicated.is_some())||from==to{return Some(0);}
    let mut moved=0;
    while moved<max{
        let Some(source)=item_handler(game,from)else{break;};let target=item_handler(game,to);let mut success=false;
        for slot in 1..source.slots(){
            let Some(stock)=source.stack(slot)else{continue;};if item.is_some_and(|i|i!=&*stock.item){continue;}
            if let Some(target)=&target{
                if source.location(slot).is_some_and(|(p,_)|target.entries.iter().any(|(q,_)|*q==p)){continue;}
                let mut one=stock.clone();one.count=1;if target.simulate_insert(game,0,&one).count>0{continue;}
            }
            let Some(Some(one))=auto_take(game,from,slot,1)else{continue;};
            let rest=game.native_insert_item(to,to_face,one);
            if let Some(rest)=rest.filter(|s|s.count>0){auto_put_back(game,from,slot,&rest);}
            else{
                IO_ROLLBACK.with_borrow_mut(|q|{q.remove(&(game.dim.id().to_owned(),from,slot));});
                moved+=1;success=true;break;
            }
        }
        if !success{break;}
    }Some(moved)
}
pub(super) fn io_accepts(game:&Game<'_>,pos:Pos,slot:usize,input:&ItemStack,cancel:&mut bool){
    if let Some(handler)=item_handler(game,pos){if !game.native_on(ID)||handler.simulate_insert(game,slot,input).count==input.count{*cancel=true;}}
}
pub(super) fn io_view(m:&ModBlockEntity)->Option<ModContainer>{
    if !(drawer(&m.id)||controller(&m.id)||port(&m.id)){return None;}
    let ready=matches!(m.nbt.get(&"sd_io_layout".into()),Some(Tag::Int(IO_LAYOUT)));
    let paused=drawer(&m.id)&&group(m).is_ok_and(|g|g.attributes.suspended);
    let n=if paused{0}else if ready{m.items.len().max(1)}else{1};
    Some(ModContainer{slots:n,faces:std::array::from_fn(|_|(0..n).collect()),inputs:(0..n).collect(),outputs:(1..n).collect(),menu:None})
}
pub(super) fn io_slots(game:&Game<'_>,pos:Pos)->Option<Vec<super::machines::native_items::NativeContainerSlot>>{
    let handler=item_handler(game,pos)?;if !game.native_on(ID){return Some(Vec::new());}Some((0..handler.slots()).map(|slot|{
        let stack=handler.stack(slot);let can_take=stack.is_some();super::machines::native_items::NativeContainerSlot{slot,stack,can_take}
    }).collect())
}
fn refresh_controllers(game:&mut Game<'_>){
    let positions:Vec<_>=game.world.block_entities.iter().filter_map(|(&(x,y,z),e)|match e{BlockEntity::Mod(m)if controller(&block_name(m))||port(&block_name(m))=>Some([x,y,z]),_=>None}).collect();
    for pos in positions{if let Some(handler)=item_handler(game,pos){let items=(0..handler.slots()).map(|s|handler.stack(s)).collect();
        game.native_with_mod_block_entity(pos,|m|{m.items=items;m.nbt.insert("sd_io_layout".into(),Tag::Int(IO_LAYOUT));});
    }}
}

pub(super) fn tick(game:&mut Game<'_>){
    if !(game.serving.is_some()||game.options.dedicated.is_some())||!game.native_on(ID){HOPPER_CONTACTS.with_borrow_mut(BTreeMap::clear);return;}
    super::sd_framing::tick(game);
    IO_ROLLBACK.with_borrow_mut(|q|q.retain(|_,r|r.time>=game.world.game_time));
    let migrate:Vec<_>=game.world.block_entities.iter().filter_map(|(&(x,y,z),e)|{
        let BlockEntity::Mod(m)=e else{return None;};if !drawer(&m.id){return None;}let g=group(m).ok()?;
        let hopper=if g.has(sd::Upgrade::Hopper){"true"}else{"false"};
        (!matches!(m.nbt.get(&"sd_io_layout".into()),Some(Tag::Int(IO_LAYOUT)))||game.native_block_property([x,y,z],"hopper").as_deref()!=Some(hopper)).then_some(([x,y,z],g))
    }).collect();
    for (p,g) in migrate{set_group(game,p,&g);}
    collect_hopper_contacts(game);

    if game.world.game_time%60==0&&feature_config()["detached_heavy"].as_bool().unwrap_or(false){
        let heavy=|s:&ItemStack|super::sd_items::detached(s).is_some_and(|(stock,_,heavy)|heavy&&stock.as_ref().is_some_and(|s|s.count>game.stack_max_size(s)));
        let players:Vec<_>=game.lan.as_ref().map(|h|h.hub.guests.iter().filter(|p|p.dimension==game.dim.id()&&(p.inventory.iter().flatten().any(&heavy)||p.offhand.as_ref().is_some_and(&heavy))).map(|p|p.uuid).collect()).unwrap_or_default();
        for uuid in players{game.native_add_effect(uuid,&super::native_player_state::NativeEffect{id:"minecraft:slowness".into(),amplifier:3,duration:100,ambient:true,particles:true});}
    }
    if game.world.game_time%20==0{
        let fixes:Vec<_>=game.lan.as_ref().map(|host|host.hub.guests.iter().filter(|g|g.dimension==game.dim.id()).map(|g|{
            let validate=|s:&ItemStack|if let Some((p,d))=super::sd_items::binding(s){
                if d!=game.dim.id()||game.world.loaded(p[0],p[2])&&!controller(&name_at(game,p)){super::sd_items::unbound(s)}else{s.clone()}
            }else{s.clone()};
            (g.uuid,g.inventory.iter().map(|s|s.as_ref().map(&validate)).collect::<Vec<_>>(),g.offhand.as_ref().map(&validate),g.carried.as_ref().map(&validate))
        }).collect()).unwrap_or_default();
        if let Some(host)=game.lan.as_mut(){for (uuid,inv,off,carried) in fixes{if let Some(i)=host.hub.guests.iter().position(|p|p.uuid==uuid){
            let g=&mut host.hub.guests[i];if g.inventory!=inv||g.offhand!=off||g.carried!=carried{g.inventory=inv;g.offhand=off;g.carried=carried;host.hub.send_inventory(i);}
        }}}
        let drawers:Vec<_>=game.world.block_entities.iter().filter_map(|(&(x,y,z),e)|match e{BlockEntity::Mod(m)if drawer(&block_name(m))=>Some([x,y,z]),_=>None}).collect();
        for p in drawers{if let Some(m)=entity(game,p){if let Ok(g)=group(m){let positions=upgrade_positions(m,&g);let old=upgrade_items_from_nbt(&m.nbt,&g,&positions);
            let items:Vec<_>=old.iter().map(|s|validate_remote(game,p,s)).collect();if items!=old{set_group_positions(game,p,&g,Some(&positions),Some(&items));}
        }}}
    }
    let controllers:Vec<_>=game.world.block_entities.iter().filter_map(|(&(x,y,z),e)|match e{BlockEntity::Mod(m)if controller(&block_name(m))=>Some([x,y,z]),_=>None}).collect();
    if let Some(&p)=controllers.first(){let mut net=network(game,p);net.rebalance_all(sd::Access::Automation);commit_network(game,&net);}
    let opens=OPEN.with_borrow_mut(std::mem::take);
    for (uuid,dim,pos) in opens{if dim!=game.dim.id(){continue;}
        if let Some(m)=entity(game,pos){if let Ok(g)=group(m){
            if !drawer(&block_name(m))||g.check(sd::Access::Player(uuid)).is_err(){continue;}
            let title=format!("block.storagedrawers.{}",kind(&block_name(m)));let items=menu_items_for(m,&g);
            let hidden=(0..18).filter(|&s|!(s<7||(9..9+g.drawers.len()).contains(&s))).collect();
            if let Some(window)=super::sd_framing::open_menu(game,uuid,pos,"storagedrawers:drawers",Text::tr(&title,&[]),items,hidden){
            DRAGS.with_borrow_mut(|m|{m.remove(&uuid);});
            MENUS.with_borrow_mut(|m|{m.insert(uuid,(window,dim,pos));});
        }}}
    }
    // Automation, another player and block removal also change an open screen.
    let menus=MENUS.with_borrow(|m|m.clone());
    for (uuid,(window,dim,pos)) in menus{if dim!=game.dim.id(){continue;}
        let open=game.lan.as_ref().and_then(|h|h.hub.guests.iter().find(|p|p.uuid==uuid))
            .and_then(|p|p.menu.as_ref()).is_some_and(|m|m.window==window);
        if !open{close_menu(uuid,window);continue;}
        let player=game.native_server_players().into_iter().find(|p|p.uuid==uuid);
        let valid=player.is_some_and(|p|p.dimension==dim&&(0..3).map(|i|(p.position[i]-f64::from(pos[i])-0.5).powi(2)).sum::<f64>()<=64.);
        let data=entity(game,pos).and_then(|m|group(m).ok().filter(|g|drawer(&block_name(m))&&g.check(sd::Access::Player(uuid)).is_ok())
            .map(|g|menu_items_for(m,&g)));
        if !valid||data.is_none(){game.native_close_menu(uuid);close_menu(uuid,window);continue;}
        let items=data.unwrap();
        if game.native_menu_contents(uuid).as_ref()!=Some(&items){game.native_set_menu_items(uuid,items);}
    }
    refresh_controllers(game);
}
fn menu_items_for(entity:&ModBlockEntity,g:&sd::Group)->Vec<Option<ItemStack>>{
    let positions=upgrade_positions(entity,g);let upgrades=upgrade_items_from_nbt(&entity.nbt,g,&positions);
    menu_items(g,&positions,&upgrades)
}
fn menu_items(g:&sd::Group,positions:&[usize],upgrades:&[ItemStack])->Vec<Option<ItemStack>>{
    let mut items=vec![None;18];
    for (u,&pos) in upgrades.iter().zip(positions){items[pos]=Some(u.clone());}
    for slot in 0..g.drawers.len(){items[9+slot]=g.item(slot).and_then(|i|{
        let mut item=stack(i,1)?;
        let lore=Tag::List([("storagedrawers.tooltip.stored",g.count(slot)),("storagedrawers.tooltip.capacity",g.capacity(slot,i))]
            .into_iter().map(|(key,value)|Tag::compound([("translate",Tag::String(key.into())),
                ("with",Tag::List(vec![Tag::String(value.to_string().as_str().into())]))])).collect());
        item.data_mut().set_other("minecraft:lore",Some(lore));
        item.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([("sd_count",Tag::Long(g.count(slot)as i64))])));
        Some(item)
    });}
    items
}
pub(super) fn close_menu(uuid:u128,window:i32){super::sd_framing::close(uuid,window);MENUS.with_borrow_mut(|m|{if m.get(&uuid).is_some_and(|v|v.0==window){m.remove(&uuid);DRAGS.with_borrow_mut(|d|{d.remove(&uuid);});}});}
pub(super) fn menu_click(game:&mut Game<'_>,uuid:u128,window:i32,slot:i32,button:i32,mode:i32){
    if super::sd_framing::menu_click(game,uuid,window,slot,button,mode){return;}
    let Some((expected,dim,pos))=MENUS.with_borrow(|m|m.get(&uuid).cloned())else{return;};
    if window!=expected||dim!=game.dim.id(){return;}
    // Queued clicks may outlive their window; never apply them to a replacement.
    let open=game.lan.as_ref().and_then(|h|h.hub.guests.iter().find(|p|p.uuid==uuid))
        .and_then(|p|p.menu.as_ref()).is_some_and(|m|m.window==window);
    if !open{close_menu(uuid,window);return;}
    apply_menu_click(game,uuid,pos,slot,button,mode);
    // Display-only menus are resent BEFORE the callback by Horizon. A slot-only
    // tick sync cannot repair its old cursor: resend both after our transaction.
    if let Some(conn)=game.native_conn(uuid){if let Some(host)=game.lan.as_mut(){host.hub.resend_menu(conn,window,&game.world);}}
}
fn upgrade_stack(u:sd::Upgrade)->ItemStack{ItemStack::new(&format!("storagedrawers:{}",u.id()),1)}
fn may_take_upgrade(u:sd::Upgrade,creative:bool)->bool{creative||!matches!(u,sd::Upgrade::CreativeStorage|sd::Upgrade::CreativeVending)}
fn upgrade_cursor(g:&mut sd::Group,positions:&mut Vec<usize>,upgrades:&mut Vec<ItemStack>,physical:usize,carried:&mut Option<ItemStack>,access:sd::Access,creative:bool){
    let index=positions.iter().position(|&p|p==physical);
    if let Some(index)=index{if !may_take_upgrade(g.upgrades[index],creative)||g.clone().remove_upgrade(index,access).is_err(){return;}}
    if let Some(input)=carried.as_ref(){
        let Some(u)=upgrade(&input.item)else{return;};
        if !g.can_place_upgrade(index,u,input.count,creative,access){return;}
        if input.count==0||index.is_some_and(|i|upgrades[i].same_item_same_components(input))||index.is_some()&&input.count!=1{return;}
        if index.is_some_and(|i|u.multiplier().is_none()||g.upgrades[i].multiplier().is_none()&&g.upgrades[i]!=sd::Upgrade::OneStack){return;}
        let mut changed=g.clone();let mut mapped=positions.clone();
        let mut changed_items=upgrades.clone();
        let old=index.map(|i|{mapped.remove(i);changed.upgrades.remove(i);changed_items.remove(i)});
        // SlotUpgrade.mayPickup and mayPlace must both allow the exchange.
        if changed.add_upgrade(u,access).is_err(){return;}
        mapped.push(physical);let mut installed=input.clone();installed.count=1;changed_items.push(installed);
        if let Some(old)=old{*carried=Some(old);}
        else{let s=carried.as_mut().unwrap();s.count-=1;if s.count==0{*carried=None;}}
        *g=changed;*positions=mapped;*upgrades=changed_items;
    }else if let Some(index)=index{if g.remove_upgrade(index,access).is_ok(){positions.remove(index);*carried=Some(upgrades.remove(index));}}
}
fn move_inventory(inv:&mut [Option<ItemStack>],index:usize,max:&impl Fn(&ItemStack)->u32){
    let Some(mut input)=inv[index].take()else{return;};
    let range=if index<9{9..36}else{0..9};
    for i in range.clone(){if let Some(s)=inv[i].as_mut().filter(|s|s.same_item_same_components(&input)){
        let n=max(s).saturating_sub(s.count).min(input.count);s.count+=n;input.count-=n;
    }}
    for i in range{if input.count>0&&inv[i].is_none(){let n=max(&input).min(input.count);let mut out=input.clone();out.count=n;inv[i]=Some(out);input.count-=n;}}
    inv[index]=(input.count>0).then_some(input);
}
#[derive(Clone,PartialEq)]
struct DrawerMenu{
    g:sd::Group,positions:Vec<usize>,upgrades:Vec<ItemStack>,
    inv:Vec<Option<ItemStack>>,offhand:Option<ItemStack>,carried:Option<ItemStack>,
}
impl DrawerMenu{
    fn visible(&self,slot:i32)->bool{(0..7).contains(&slot)||(9..9+self.g.drawers.len()as i32).contains(&slot)||(18..54).contains(&slot)}
    fn stack(&self,slot:i32)->Option<ItemStack>{
        if (0..7).contains(&slot){return self.positions.iter().position(|&p|p==slot as usize).map(|i|self.upgrades[i].clone());}
        if let Some(i)=super::native_menus::native_menu_inventory_slot(18,slot){return self.inv[i].clone();}
        if (9..9+self.g.drawers.len()as i32).contains(&slot){return self.g.item((slot-9)as usize).and_then(|i|stack(i,1));}
        None
    }
    fn may_pickup(&self,slot:i32,access:sd::Access,creative:bool)->bool{
        if (0..7).contains(&slot){return self.positions.iter().position(|&p|p==slot as usize).is_none_or(|i|
            may_take_upgrade(self.g.upgrades[i],creative)&&self.g.clone().remove_upgrade(i,access).is_ok());}
        (18..54).contains(&slot)
    }
    fn take(&mut self,slot:i32,amount:u32,access:sd::Access,creative:bool)->Option<ItemStack>{
        if amount==0||!self.may_pickup(slot,access,creative){return None;}
        if (0..7).contains(&slot){
            let i=self.positions.iter().position(|&p|p==slot as usize)?;
            self.g.remove_upgrade(i,access).ok()?;self.positions.remove(i);return Some(self.upgrades.remove(i));
        }
        let i=super::native_menus::native_menu_inventory_slot(18,slot)?;
        let item=self.inv[i].as_mut()?;let mut out=item.clone();out.count=amount.min(item.count);item.count-=out.count;
        if item.count==0{self.inv[i]=None;}Some(out)
    }
    fn deposit(&mut self,game:&Game<'_>,block:&str,slot:i32,input:&ItemStack,amount:u32,access:sd::Access)->u32{
        if !(9..9+self.g.drawers.len()as i32).contains(&slot)||amount==0{return 0;}
        let mut g=self.g.clone();seed(game,block,&mut g,input);let key=key(game,input);
        let target=if let Some(c)=&g.compact{let Some(i)=c.views.iter().position(|(i,_)|i.same_item(&key))else{return 0;};i}else{(slot-9)as usize};
        let accepted=insert_group(game,&mut g,target,&sd::Stack{item:key,count:u64::from(amount.min(input.count))},access,false).map_or(0,|p|p.accepted()as u32);
        if accepted>0{self.g=g;}accepted
    }
    fn pickup(&mut self,game:&Game<'_>,block:&str,slot:i32,button:i32,access:sd::Access,creative:bool){
        let max=|s:&ItemStack|game.stack_max_size(s);
        if (0..7).contains(&slot){
            // A full one-item slot cannot merge. When mayPlace refuses a stack
            // of the same upgrade, vanilla tops up the cursor from the slot.
            if let (Some(item),Some(cursor))=(self.stack(slot),self.carried.as_ref()){
                if item.same_item_same_components(cursor){
                    // A single storage upgrade passes mayPlace but the slot is
                    // already full. A larger stack, or a non-storage upgrade,
                    // is refused by mayPlace and may take the installed item.
                    let refused=cursor.count>1||upgrade(&item.item).is_none_or(|u|u.multiplier().is_none());
                    if refused&&cursor.count<max(cursor){if let Some(taken)=self.take(slot,1,access,creative){self.carried.as_mut().unwrap().count+=taken.count;}}
                    return;
                }
            }
            upgrade_cursor(&mut self.g,&mut self.positions,&mut self.upgrades,slot as usize,&mut self.carried,access,creative);
        }else if let Some(i)=super::native_menus::native_menu_inventory_slot(18,slot){cursor_click(&mut self.inv[i],&mut self.carried,button==1,&max);}
        else if let Some(mut input)=self.carried.clone(){
            let amount=if button==1{1}else{input.count};input.count-=self.deposit(game,block,slot,&input,amount,access);
            self.carried=(input.count>0).then_some(input);
        }
    }
    fn quick_move(&mut self,game:&Game<'_>,slot:i32,access:sd::Access,creative:bool){
        let max=|s:&ItemStack|game.stack_max_size(s);
        if (0..7).contains(&slot){
            let mut changed=self.clone();let Some(input)=changed.take(slot,1,access,creative)else{return;};
            // Java scans the player slots backwards: hotbar 8..0, main 35..9.
            let order:Vec<usize>=(0..9).rev().chain((9..36).rev()).collect();
            if put_inventory(&mut changed.inv,input,&order,&max).is_none(){*self=changed;}
        }else if let Some(i)=super::native_menus::native_menu_inventory_slot(18,slot){
            if let Some(input)=self.inv[i].clone(){if let (Some(u),Some(free))=(upgrade(&input.item),(0..sd::UPGRADE_SLOTS).find(|p|!self.positions.contains(p))){
                if self.g.add_upgrade(u,access).is_ok(){self.positions.push(free);let mut installed=input;installed.count=1;self.upgrades.push(installed);
                    let s=self.inv[i].as_mut().unwrap();s.count-=1;if s.count==0{self.inv[i]=None;}return;}
            }}
            move_inventory(&mut self.inv,i,&max);
        }
    }
    fn swap(&mut self,game:&Game<'_>,block:&str,slot:i32,button:i32,access:sd::Access,creative:bool){
        if !matches!(button,0..=8|40)||!self.visible(slot){return;}
        let mut source=if button==40{self.offhand.clone()}else{self.inv[button as usize].clone()};
        if (0..7).contains(&slot){
            // InventoryUpgrade.canSwapUpgrade checks the original source count.
            if self.stack(slot).is_some()&&source.as_ref().is_some_and(|s|s.count>1){return;}
            upgrade_cursor(&mut self.g,&mut self.positions,&mut self.upgrades,slot as usize,&mut source,access,creative);
        }else if let Some(i)=super::native_menus::native_menu_inventory_slot(18,slot){
            if button!=40&&i==button as usize{return;}
            std::mem::swap(&mut self.inv[i],&mut source);
        }else{
            // Stock slots never give their displayed prototype to a swap.
            if self.stack(slot).is_some(){return;}
            if let Some(input)=source.as_mut(){input.count-=self.deposit(game,block,slot,input,input.count,access);if input.count==0{source=None;}}
        }
        if button==40{self.offhand=source;}else{self.inv[button as usize]=source;}
    }
    fn drag_accepts(&self,game:&Game<'_>,block:&str,slot:i32,input:&ItemStack,access:sd::Access)->bool{
        if !self.visible(slot){return false;}
        if let Some(existing)=self.stack(slot){if !existing.same_item_same_components(input)||existing.count>game.stack_max_size(input){return false;}}
        if (0..7).contains(&slot){return self.stack(slot).is_none()&&upgrade(&input.item).is_some_and(|u|self.g.clone().add_upgrade(u,access).is_ok());}
        if (18..54).contains(&slot){return true;}
        self.clone().deposit(game,block,slot,input,1,access)>0
    }
    fn drag_put(&mut self,game:&Game<'_>,block:&str,slot:i32,input:&ItemStack,amount:u32,access:sd::Access)->u32{
        if amount==0||!self.drag_accepts(game,block,slot,input,access){return 0;}
        if (0..7).contains(&slot){
            let Some(u)=upgrade(&input.item)else{return 0;};if self.g.add_upgrade(u,access).is_err(){return 0;}
            self.positions.push(slot as usize);let mut installed=input.clone();installed.count=1;self.upgrades.push(installed);return 1;
        }
        if let Some(i)=super::native_menus::native_menu_inventory_slot(18,slot){
            let have=self.inv[i].as_ref().map_or(0,|s|s.count);let put=amount.min(game.stack_max_size(input).saturating_sub(have));
            if put>0{let mut placed=input.clone();placed.count=have+put;self.inv[i]=Some(placed);}return put;
        }
        let mut source=input.clone();source.count=amount;self.deposit(game,block,slot,&source,amount,access)
    }
    fn collect(&mut self,game:&Game<'_>,slot:i32,button:i32,access:sd::Access,creative:bool){
        let Some(mut cursor)=self.carried.clone()else{return;};
        if !self.visible(slot)||self.stack(slot).is_some()&&self.may_pickup(slot,access,creative){return;}
        let max=game.stack_max_size(&cursor);let order:Vec<i32>=if button==0{(0..54).collect()}else{(0..54).rev().collect()};
        for full in [false,true]{for &slot in &order{
            if cursor.count>=max{break;}
            let Some(item)=self.stack(slot)else{continue;};
            if !item.same_item_same_components(&cursor)||(item.count==max)!=full{continue;}
            if let Some(taken)=self.take(slot,max-cursor.count,access,creative){cursor.count+=taken.count;}
        }}self.carried=Some(cursor);
    }
}
fn put_inventory(inv:&mut [Option<ItemStack>],mut input:ItemStack,order:&[usize],max:&impl Fn(&ItemStack)->u32)->Option<ItemStack>{
    for &i in order{if let Some(s)=inv[i].as_mut().filter(|s|s.same_item_same_components(&input)){
        let n=max(s).saturating_sub(s.count).min(input.count);s.count+=n;input.count-=n;
    }}
    for &i in order{if input.count>0&&inv[i].is_none(){let mut out=input.clone();out.count=input.count.min(max(&input));input.count-=out.count;inv[i]=Some(out);}}
    (input.count>0).then_some(input)
}
fn apply_menu_click(game:&mut Game<'_>,uuid:u128,pos:Pos,slot:i32,button:i32,mode:i32){
    let drag=DRAGS.with_borrow_mut(|d|d.remove(&uuid));
    // AbstractContainerMenu resets a pending drag before accepting another click.
    if mode!=5&&drag.is_some(){return;}
    if !(0..=6).contains(&mode)||matches!(mode,0|1|6)&&!matches!(button,0|1){return;}
    let Some(player)=game.native_server_players().into_iter().find(|p|p.uuid==uuid)else{return;};
    if player.dimension!=game.dim.id()||(0..3).map(|i|(player.position[i]-f64::from(pos[i])-0.5).powi(2)).sum::<f64>()>64.{game.native_close_menu(uuid);return;}
    let creative=player.game_mode==1;let access=sd::Access::Player(uuid);
    let Some(m)=entity(game,pos)else{game.native_close_menu(uuid);return;};let Ok(g)=group(m)else{return;};
    if !drawer(&block_name(m))||g.check(access).is_err(){return;}
    let block=block_name(m);let positions=upgrade_positions(m,&g);let upgrades=upgrade_items_from_nbt(&m.nbt,&g,&positions);
    let Some(inv)=game.native_player_stacks(uuid).filter(|inv|inv.len()>=36)else{return;};
    let Some(guest)=game.lan.as_ref().and_then(|h|h.hub.guests.iter().find(|p|p.uuid==uuid))else{return;};
    let mut state=DrawerMenu{g,positions,upgrades,inv,offhand:guest.offhand.clone(),carried:guest.carried.clone()};
    let before=state.clone();let mut drops=Vec::new();
    if mode==5{
        let (stage,kind)=(button&3,(button>>2)&3);
        if button<0||button>10{return;}
        match stage{
            0=>{if drag.is_none()&&matches!(kind,0|1|2)&&(kind!=2||creative){if let Some(source)=state.carried.clone(){
                DRAGS.with_borrow_mut(|d|{d.insert(uuid,MenuDrag{kind,slots:Vec::new(),source});});
            }}return;}
            1=>{if let Some(mut drag)=drag.filter(|d|d.kind==kind&&state.carried.as_ref()==Some(&d.source)){
                if !drag.slots.contains(&slot)&&(kind==2||drag.source.count as usize>drag.slots.len())&&state.drag_accepts(game,&block,slot,&drag.source,access){drag.slots.push(slot);}
                DRAGS.with_borrow_mut(|d|{d.insert(uuid,drag);});
            }return;}
            2=>{let Some(drag)=drag.filter(|d|d.kind==kind&&state.carried.as_ref()==Some(&d.source))else{return;};
                if drag.slots.is_empty(){return;}
                if drag.slots.len()==1{if kind<2{state.pickup(game,&block,drag.slots[0],kind,access,creative);}}
                else{
                    let each=match kind{0=>drag.source.count/drag.slots.len()as u32,1=>1,_=>game.stack_max_size(&drag.source)};
                    let mut remaining=drag.source.count;
                    for slot in drag.slots{
                        let amount=if kind==2{each}else{each.min(remaining)};
                        let put=state.drag_put(game,&block,slot,&drag.source,amount,access);remaining=remaining.saturating_sub(put);
                    }
                    let mut source=drag.source;source.count=remaining;state.carried=(remaining>0).then_some(source);
                }
            }
            _=>return,
        }
    }else if slot==-999&&matches!(mode,0|1){
        if let Some(mut source)=state.carried.take(){let mut drop=source.clone();drop.count=if button==1{1}else{source.count};source.count-=drop.count;
            state.carried=(source.count>0).then_some(source);drops.push(drop);}
    }else if state.visible(slot){match mode{
        0=>state.pickup(game,&block,slot,button,access,creative),
        1=>state.quick_move(game,slot,access,creative),
        2=>state.swap(game,&block,slot,button,access,creative),
        3=>{if creative&&state.carried.is_none(){state.carried=state.stack(slot).map(|mut s|{s.count=game.stack_max_size(&s);s});}}
        4=>{if state.carried.is_none(){let amount=if button==0{1}else{u32::MAX};if let Some(drop)=state.take(slot,amount,access,creative){drops.push(drop);}}}
        6=>state.collect(game,slot,button,access,creative),_=>{}
    }}
    state.upgrades=state.upgrades.iter().map(|s|validate_remote(game,pos,s)).collect();
    let changed=state.g!=before.g||state.positions!=before.positions||state.upgrades!=before.upgrades;
    if state!=before&&(!changed||set_group_positions(game,pos,&state.g,Some(&state.positions),Some(&state.upgrades))){
        if let Some(host)=game.lan.as_mut(){if let Some(i)=host.hub.guests.iter().position(|p|p.uuid==uuid){
            let p=&mut host.hub.guests[i];p.inventory=state.inv;p.offhand=state.offhand;p.carried=state.carried;
            // The chest content packet has no off-hand slot. Send window 0
            // before resend_menu marks the inventory as already synchronized.
            if p.offhand!=before.offhand{host.hub.send_inventory(i);}
        }}
        game.native_set_menu_items(uuid,menu_items(&state.g,&state.positions,&state.upgrades));
        // Persist the drawer and all player slots before emitting thrown items.
        if !drops.is_empty(){if let Some(host)=game.lan.take(){
            if let Some(i)=host.hub.guests.iter().position(|p|p.uuid==uuid){game.drop_guest_items(&host,i,drops);}game.lan=Some(host);
        }}
    }
}
fn cursor_click(slot:&mut Option<ItemStack>,carried:&mut Option<ItemStack>,right:bool,max:&impl Fn(&ItemStack)->u32){
    if right{if let Some(ring)=slot.as_mut(){if super::sd_items::ring_click(ring,carried){return;}}}
    match (slot.as_mut(),carried.as_mut()){
        (Some(item),None)=>{let n=if right{item.count.div_ceil(2)}else{item.count};let mut out=item.clone();out.count=n;item.count-=n;
            if item.count==0{*slot=None;}*carried=Some(out);}
        (None,Some(item))=>{let n=if right{1}else{item.count.min(max(item))};let mut out=item.clone();out.count=n;item.count-=n;
            if item.count==0{*carried=None;}*slot=Some(out);}
        (Some(a),Some(b)) if a.same_item_same_components(b)=>{let n=max(a).saturating_sub(a.count).min(if right{1}else{b.count});
            a.count+=n;b.count-=n;if b.count==0{*carried=None;}}
        (Some(_),Some(b)) if b.count<=max(b)=>std::mem::swap(slot,carried),_=>{}
    }
}

// entityInside fires while the item is borrowed by physics. Resolve the
// contact on the next safe server turn using the shared inventory handler.
pub(super) fn item_contact(dimension:&str,pos:Pos,item:u32){
    HOPPER_CONTACTS.with_borrow_mut(|q|{q.entry((dimension.into(),pos)).or_default().insert(item);});
}
fn collect_hopper_contacts(game:&mut Game<'_>){
    let contacts=HOPPER_CONTACTS.with_borrow_mut(std::mem::take);
    for ((dimension,pos),ids) in contacts{
        if dimension!=game.dim.id(){HOPPER_CONTACTS.with_borrow_mut(|q|{q.entry((dimension,pos)).or_default().extend(ids);});continue;}
        let active=entity(game,pos).and_then(|m|group(m).ok()).is_some_and(|g|g.has(sd::Upgrade::Hopper)&&!g.attributes.suspended);
        if !active{continue;}
        // BlockEntityDrawers.SUCK_AABB, not the ordinary hopper's cooldown.
        let min=[pos[0]as f64,pos[1]as f64+11./16.,pos[2]as f64];
        let max=[pos[0]as f64+1.,pos[1]as f64+2.,pos[2]as f64+1.];
        for item in game.native_item_entities(&dimension,min,max){
            if !ids.contains(&item.id){continue;}
            let Some(before)=entity(game,pos).and_then(|m|group(m).ok())else{break;};
            let Some((rest,true))=io_insert(game,pos,0,&item.stack,Some(Direction::Up),false)else{continue;};
            let remainder=rest.unwrap_or_else(||{let mut s=item.stack.clone();s.count=0;s});
            if !game.native_set_item_entity(&dimension,item.id,remainder){set_group(game,pos,&before);refresh_controllers(game);}
        }
    }
}
pub(super) fn block_tick(game:&mut Game<'_>,pos:Pos,m:&mut ModBlockEntity){
    if !drawer(&block_name(m))||!game.native_on(ID){return;}
    let Ok(mut g)=group(m)else{return;};
    if g.attributes.suspended{return;}
    let old=g.clone();let dim=game.dim.id();
    let range=g.magnet_range();let interval=g.magnet_interval();
    if range[0]>0&&game.world.game_time%interval.max(1)as i64==0{
        let r=range[0]as f64;
        let (min,max)=if range[0]>0{([pos[0]as f64-r,pos[1]as f64-range[2]as f64,pos[2]as f64-r],
            [pos[0]as f64+1.+r,pos[1]as f64+1.+range[1]as f64,pos[2]as f64+1.+r])}
            else{([pos[0]as f64,pos[1]as f64+1.,pos[2]as f64],[pos[0]as f64+1.,pos[1]as f64+2.,pos[2]as f64+1.])};
        for item in game.native_item_entities(dim,min,max){
            seed(game,&block_name(m),&mut g,&item.stack);
            let k=key(game,&item.stack);let Some(slot)=(0..g.drawers.len()).find(|&i|g.item(i).map_or(!g.attributes.locked,|v|v.same_item(&conversion_key(game,&g,i,&k))))else{continue;};
            let put=insert_group(game,&mut g,slot,&sd::Stack{item:k,count:u64::from(item.stack.count)},sd::Access::Automation,false).unwrap_or_default();
            if put.accepted()>0{let mut rest=item.stack;rest.count-=put.accepted()as u32;game.native_set_item_entity(dim,item.id,rest);}
        }
    }
    if g!=old{save(m,&g);game.native_block_entity_sync(pos);game.native_update_signal_neighbors(dim,pos);game.native_update_output_neighbors(dim,pos);}
}

pub(super) fn placed(game:&mut Game<'_>,input:&ItemStack,pos:Pos){
    if !ours(&name_at(game,pos)){return;}
    super::sd_framing::placed(game,input,pos);
    if let Some(Tag::Compound(components))=horizon_items::item_nbt::item_to_nbt(input).get("components"){
        game.native_with_mod_block_entity(pos,|m|{m.nbt.insert("sd_item_components".into(),Tag::Compound(components.clone()));});
    }
    if let Some(frame)=super::sd_items::frame(input){game.native_with_mod_block_entity(pos,|m|{m.nbt.insert("sd_frame".into(),frame);});game.native_block_entity_sync(pos);}
    // Horizontal facing is already chosen by the vanilla placement template.
    let data=input.data().and_then(|d|d.other("minecraft:custom_data"));
    if let Some(Tag::ByteArray(bytes))=data.and_then(|d|d.get("sd_contents")){
        if let Ok(g)=sd::codec::decode(bytes){
            let positions=if let Some(Tag::Compound(nbt))=data{positions_from_nbt(nbt,&g)}else{(0..g.upgrades.len()).collect()};
            let upgrades=if let Some(Tag::Compound(nbt))=data{upgrade_items_from_nbt(nbt,&g,&positions)}else{g.upgrades.iter().copied().map(upgrade_stack).collect()};
            set_group_positions(game,pos,&g,Some(&positions),Some(&upgrades));
        }else{feedback(game,"storagedrawers.message.corrupt");}
    }
    if let Some(remote)=data.and_then(|d|d.get("sd_remote")){
        game.native_with_mod_block_entity(pos,|m|{m.nbt.insert("sd_remote".into(),remote.clone());});
    }
}

pub(super) fn render(game:&Game<'_>,pos:Pos,block:&str,m:&ModBlockEntity,draw:&mut super::native_block_entity_draw::BlockEntityDraw){
    if !game.native_on(ID){return;}
    super::sd_appearance::render(game,pos,block,m,draw);
    if !drawer(block){return;}let Ok(g)=group(m)else{return;};
    let offset=match facing(game,pos){"east"=>[1,0,0],"south"=>[0,0,1],"west"=>[-1,0,0],_=>[0,0,-1]};
    draw.light=horizon_client::block_entity_render::vertex_light(game.world.light(pos[0]+offset[0],pos[1],pos[2]+offset[2]));
    let minimum=if g.has(sd::Upgrade::Illumination){13*16}else{16};
    draw.light=(draw.light&0xFF)|((draw.light>>8).max(minimum)<<8);
    draw.push();draw.translate([0.5,0.,0.5]);
    let angle=match facing(game,pos){"south"=>0.,"east"=>90.,"west"=>270.,_=>180.};
    draw.rotate_about([0.,1.,0.],angle,[0.,0.,0.]);
    draw.translate([-0.5,0.,if g.half{-0.06}else{0.44}]);
    let compact=kind(block).contains("compacting");
    let [icons,counts,indicators,bases]=super::sd_geometry::areas(compact,g.half,slots(block));
    for slot in 0..g.drawers.len(){
        let Some(i)=g.item(slot)else{continue;};let Some(s)=stack(i,1)else{continue;};
        if !g.attributes.shrouded{
            let [from,to]=icons[slot];
            draw.push();draw.translate([(from[0]+to[0])/32.,(from[1]+to[1])/32.,0.]);
            draw.scale([(to[0]-from[0])/16.,(to[1]-from[1])/16.,0.001]);
            draw_label_item(game,draw,&s,angle);draw.pop();
            if g.attributes.quantified&&g.count(slot)>0{
                let text=count_text(g.count(slot));let [from,to]=counts[slot];
                draw.push();draw.translate([(from[0]+to[0])/32.,to[1]/16.,0.008]);draw.scale([1./128.,-1./128.,1./128.]);
                let width=game.native_text_width(&text);
                game.native_draw_text(draw,&Text::literal(text),[-width/2.,0.],0xFFFFFF,false,0);draw.pop();
            }
        }
        if g.has(sd::Upgrade::FillLevel)&&!g.attributes.shrouded&&(!compact||slot==0){
            let cap=g.capacity(slot,i);let ratio=if cap>0{g.count(slot)as f32/cap as f32}else{0.};
            let [a,b]=indicators[slot];let [_,base]=bases[slot];
            let end=[if b[0]>base[0]{a[0]+(b[0]-base[0])*ratio}else{b[0]},
                if b[1]>base[1]{a[1]+(b[1]-base[1])*ratio}else{b[1]}];
            let variant=if compact{if slots(block)==2{"comp2"}else{"comp"}}else{match slots(block){2=>"2",4=>"4",_=>"1"}};
            // Original on/off indicator textures cover the complete face; crop their active region.
            face_quad(game,draw,&format!("storagedrawers:block/indicator/indicator_{variant}_off"),[0.,0.,16.,16.],0.004);
            if end[0]>a[0]&&end[1]>a[1]{face_quad(game,draw,&format!("storagedrawers:block/indicator/indicator_{variant}_on"),[a[0],a[1],end[0],end[1]],0.005);}
        }
    }
    draw.pop();
}
// Java's renderer flattens the position matrix without flattening the normal
// matrix, then adjusts the normals for GUI models. World item drawing alone
// uses the inverse transpose of the flattened pose and washes out face shading.
fn draw_label_item(game:&Game<'_>,draw:&mut super::native_block_entity_draw::BlockEntityDraw,item:&ItemStack,angle:f32){
    use glam::{EulerRot,Mat3,Quat};
    use horizon_client::item_render::{ItemQuad,ItemSink,LayerGeometry,WorldSink};
    use horizon_assets::item_model::DisplayContext;
    struct LabelSink<'a>{inner:WorldSink<'a>,normals:Mat3}
    impl ItemSink for LabelSink<'_>{
        fn quad(&mut self,mut quad:ItemQuad){
            quad.normal=(self.normals*quad.normal).normalize_or_zero();self.inner.quad(quad);
        }
    }
    let mut state=game.resolve_item(item,DisplayContext::Gui,None,0);
    if state.is_empty(){return;}
    let three_d=state.uses_block_light();
    // ItemRenderer uses FIXED for custom/special model renderers in the Java port.
    if state.layers.iter().any(|l|matches!(&l.geometry,LayerGeometry::Special{..})){
        state=game.resolve_item(item,DisplayContext::Fixed,None,0);
    }
    let rotation=if three_d{
        // Alignment Y rotation and its inverse cancel for a 3D item label.
        Mat3::from_quat(Quat::from_euler(EulerRot::YXZ,0.36,-0.36,-0.014))
    }else{
        Mat3::from_rotation_y(angle.to_radians())*Mat3::from_rotation_x(-std::f32::consts::FRAC_PI_4)
    };
    // Undo only the label pose's normal transform, retaining each item's own
    // display/local transform, its component tints, foil and material emission.
    let pose=draw.pose();let normals=rotation*Mat3::from_mat4(pose).transpose();
    let block=((draw.light>>8)&0xFF).max(u32::from(state.hdr_light)*16);
    let light=(draw.light&!0xFF00)|(block<<8);
    let inner=WorldSink{out:draw.vertices_mut(),light,
        lights:(state.hdr_light==0).then(||horizon_client::block_entity_render::diffuse_lights(game.nether_lighting())),
        glint:game.glint(),tier:state.hdr_tier};
    let mut sink=LabelSink{inner,normals};
    state.submit(&game.pack.item_renderer,&game.gui.atlas.sprites,&game.item_host(),pose,&mut sink);
}
fn face_quad(game:&Game<'_>,draw:&mut super::native_block_entity_draw::BlockEntityDraw,sprite:&str,area:[f32;4],z:f32){
    let [x1,y1,x2,y2]=area;let corners=[[x1/16.,y2/16.,z],[x1/16.,y1/16.,z],[x2/16.,y1/16.,z],[x2/16.,y2/16.,z]];
    game.native_draw_quad(draw,sprite,corners,[x1,16.-y2,x2,16.-y1],0xFFFFFF);
}
fn count_text(n:u64)->String{
    if n>=1_000_000_000{format!("{:.1}G",n as f64/1e9)}else if n>=100_000_000{format!("{:.0}M",n as f64/1e6)}
    else if n>=1_000_000{format!("{:.1}M",n as f64/1e6)}else if n>=100_000{format!("{:.0}K",n as f64/1e3)}
    else if n>=10_000{format!("{:.1}K",n as f64/1e3)}else{n.to_string()}
}

fn redstone_analog()->bool{feature_config()["redstone_analog_output"].as_bool().unwrap_or(true)}
pub(super) fn signal_source(level:&super::GameLevel<'_>,state:horizon_world::registry::BlockStateId,source:&mut bool){
    if level.registry.state(state).is_some_and(|s|drawer(&s.name)){*source=!redstone_analog();}
}
pub(super) fn has_analog(level:&super::GameLevel<'_>,state:horizon_world::registry::BlockStateId,output:&mut bool){
    if level.registry.state(state).is_some_and(|s|drawer(&s.name)){*output=true;}
}
pub(super) fn signal(level:&super::GameLevel<'_>,pos:Pos,face:Direction,direct:bool,signal:&mut i32){
    let state=level.world.block(pos[0],pos[1],pos[2]);
    if !level.registry.state(state).is_some_and(|s|drawer(&s.name)){return;}
    *signal=0;if redstone_analog()||direct&&face!=Direction::Up{return;}
    analog(level,pos,signal);
}
pub(super) fn analog(level:&super::GameLevel<'_>,pos:Pos,signal:&mut i32){
    let state=level.world.block(pos[0],pos[1],pos[2]);
    if !level.registry.state(state).is_some_and(|s|drawer(&s.name)){return;}*signal=0;
    if let Some(BlockEntity::Mod(m))=level.world.block_entities.get(&(pos[0],pos[1],pos[2])){
        if drawer(&m.id){if let Ok(g)=group(m){*signal=i32::from(g.redstone());}}
    }
}

pub(super) fn before_remove(level:&mut super::GameLevel<'_>,pos:horizon_core::position::BlockPos,old:horizon_world::registry::BlockStateId){
    super::sd_framing::before_remove(level,pos,old);
    let key=(pos.x,pos.y,pos.z);
    let Some(BlockEntity::Mod(m))=level.world.block_entities.get_mut(&key)else{return;};
    let mut original=match m.nbt.get(&"sd_item_components".into()){Some(Tag::Compound(c))=>c.clone(),_=>Compound::new()};
    let mut original_custom=match original.remove(&"minecraft:custom_data".into()){Some(Tag::Compound(c))=>c,_=>Compound::new()};
    for key in ["sd_contents","sd_upgrade_slots","sd_upgrade_items","sd_remote","sd_frame"]{original_custom.remove(&key.into());}
    if !drawer(&m.id){
        // Controller slots are projections of stock stored in other blocks.
        if controller(&m.id)||port(&m.id){m.items.clear();}
        if let Some(frame)=m.nbt.get(&"sd_frame".into()).cloned(){original_custom.insert("sd_frame".into(),frame);}
        if !original_custom.is_empty(){original.insert("minecraft:custom_data".into(),Tag::Compound(original_custom));}
        if !original.is_empty(){level.pending.copied.insert([pos.x,pos.y,pos.z],(m.id.clone(),original));}
        return;
    }
    let Some(Tag::ByteArray(bytes))=m.nbt.get(&SAVE.into()).cloned()else{return;};
    if group(m).ok().is_some_and(|g|g.upgrades.is_empty()&&g.frame.is_none()&&g.attributes==sd::Attributes::default()
        &&!m.nbt.contains_key(&"sd_frame".into())&&original.is_empty()&&original_custom.is_empty()&&g.drawers.iter().all(|d|d.item.is_none()&&!d.detached&&d.count==0)){
        m.items.clear();return;
    }
    let block=m.id.clone();
    let mut data=original_custom;data.insert("sd_contents".into(),Tag::ByteArray(bytes));
    if let Some(frame)=m.nbt.get(&"sd_frame".into()){data.insert("sd_frame".into(),frame.clone());}
    if let Some(positions)=m.nbt.get(&UPGRADE_POS.into()){data.insert(UPGRADE_POS.into(),positions.clone());}
    if let Some(items)=m.nbt.get(&UPGRADE_ITEMS.into()){data.insert(UPGRADE_ITEMS.into(),items.clone());}
    if let Some(remote)=m.nbt.get(&"sd_remote".into()){data.insert("sd_remote".into(),remote.clone());}
    m.items.clear();
    let mut patch=original;patch.remove(&"minecraft:max_stack_size".into());patch.insert("minecraft:custom_data".into(),Tag::Compound(data));
    if group(m).ok().is_some_and(|g|g.drawers.iter().enumerate().any(|(i,d)|d.detached||g.count(i)>0)){
        patch.insert("minecraft:max_stack_size".into(),Tag::Int(1));
    }
    level.pending.copied.insert([pos.x,pos.y,pos.z],(block,patch));
}


