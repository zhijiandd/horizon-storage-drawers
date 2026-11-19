// Two-part framing table, with a shared, persistent four-slot inventory.
use super::{Game,sd_items as data};
use horizon_client::{items::ItemStack,containers::{BlockEntity,ModBlockEntity},text::Text};
use horizon_world::{nbt::{Tag,Compound}};
use std::{cell::RefCell,collections::BTreeMap};
type Pos=[i32;3];
thread_local!{
    static OPEN:RefCell<Vec<(u128,String,Pos)>>=const{RefCell::new(Vec::new())};
    static MENUS:RefCell<BTreeMap<u128,(i32,String,Pos)>>=const{RefCell::new(BTreeMap::new())};
    static DRAGS:RefCell<BTreeMap<u128,(i32,Vec<i32>,ItemStack)>>=const{RefCell::new(BTreeMap::new())};
    static DROP_CLAIMS:RefCell<BTreeMap<(String,Pos),i64>>=const{RefCell::new(BTreeMap::new())};
    static CLEANUP:RefCell<Vec<(String,Pos,String,String)>>=const{RefCell::new(Vec::new())};
}
fn table(game:&Game<'_>,p:Pos)->bool{game.native_block(p)=="storagedrawers:framing_table"}
fn entity<'a>(game:&'a Game<'_>,p:Pos)->Option<&'a ModBlockEntity>{match game.world.block_entities.get(&(p[0],p[1],p[2])){Some(BlockEntity::Mod(m))if m.id=="storagedrawers:framing_table"=>Some(m),_=>None}}
fn left(facing:&str)->Pos{match facing{"south"=>[1,0,0],"east"=>[0,0,-1],"west"=>[0,0,1],_=>[-1,0,0]}}
fn other(p:Pos,facing:&str,part:&str)->Pos{let d=left(facing);std::array::from_fn(|i|p[i]+if part=="right"{d[i]}else{-d[i]})}
fn master(game:&Game<'_>,p:Pos)->Pos{if game.native_block_property(p,"part").as_deref()==Some("left"){other(p,&game.native_block_property(p,"facing").unwrap_or_default(),"left")}else{p}}
fn actor_facing(game:&Game<'_>)->&'static str{
    let yaw=game.native_acting_player().map_or(game.yaw,|p|p.yaw);
    ["south","west","north","east"][((yaw/90.+0.5).floor()as i32).rem_euclid(4)as usize]
}
pub(super) fn may_place(game:&Game<'_>,stack:&ItemStack,pos:Pos)->bool{
    if &*stack.item!="storagedrawers:framing_table"{return true;}
    let p=other(pos,actor_facing(game),"right");
    let state=game.world.block(p[0],p[1],p[2]);
    game.border.geometry().is_within_bounds(p[0]as f64+0.5,p[2]as f64+0.5,0.)&&game.world.loaded(p[0],p[2])&&game.res.behaviours().facts(state).flags&horizon_world::level::data::state_flags::REPLACEABLE!=0
}
pub(super) fn placed(game:&mut Game<'_>,stack:&ItemStack,p:Pos){
    if &*stack.item!="storagedrawers:framing_table"{return;}
    let facing=actor_facing(game);game.native_set_block_property(p,"facing",facing);game.native_set_block_property(p,"part","right");
    let q=other(p,facing,"right");CLEANUP.with_borrow_mut(|queue|queue.retain(|(dim,at,_,_)|dim!=game.dim.id()||(*at!=p&&*at!=q)));DROP_CLAIMS.with_borrow_mut(|claims|{claims.remove(&(game.dim.id().into(),p));claims.remove(&(game.dim.id().into(),q));});let registry=&game.pack.assets.registry;
    let id=registry.default_state("storagedrawers:framing_table").and_then(|id|registry.state(id)).cloned().and_then(|mut s|{
        s.properties.insert("facing".into(),facing.into());s.properties.insert("part".into(),"left".into());registry.id(&s)
    });
    if let Some(id)=id{game.native_set_block(game.dim.id(),q,id,3);}
}
pub(super) fn before_remove(level:&mut super::GameLevel<'_>,p:horizon_core::position::BlockPos,old:horizon_world::registry::BlockStateId){
    // setBlock has already replaced world.block(p); read the removed state.
    let Some(state)=level.registry.state(old)else{return;};
    if state.name!="storagedrawers:framing_table"{return;}
    let facing=state.properties.get("facing").cloned().unwrap_or_default();let part=state.properties.get("part").cloned().unwrap_or_default();
    CLEANUP.with_borrow_mut(|v|v.push((match level.dimension{horizon_world::level::Dimension::Overworld=>"minecraft:overworld",horizon_world::level::Dimension::Nether=>"minecraft:the_nether",horizon_world::level::Dimension::End=>"minecraft:the_end"}.into(),other([p.x,p.y,p.z],&facing,&part),facing,if part=="right"{"left"}else{"right"}.into())));
}
pub(super) fn right_click(game:&mut Game<'_>,p:Pos,_face:Pos,cancel:&mut bool)->bool{
    if !table(game,p){return false;}
    if game.hand!=super::Hand::Main{return true;}
    if game.native_sneaking()&&(game.held().is_some()||game.offhand.is_some()){return true;}
    *cancel=true;
    if let Some(player)=game.native_acting_player(){OPEN.with_borrow_mut(|q|q.push((player.uuid,game.dim.id().into(),master(game,p))));}true
}
pub(super) fn close(uuid:u128,window:i32){MENUS.with_borrow_mut(|m|{if m.get(&uuid).is_some_and(|v|v.0==window){m.remove(&uuid);DRAGS.with_borrow_mut(|d|{d.remove(&uuid);});}});}
fn inputs(game:&Game<'_>,p:Pos)->Option<Vec<Option<ItemStack>>>{let mut v=entity(game,p)?.items.clone();v.resize(4,None);v.truncate(4);Some(v)}
fn supports_front(item:&str)->bool{!item.ends_with("_trim")&&!item.ends_with("controller_io")}
fn result(v:&[Option<ItemStack>])->Option<ItemStack>{
    let target=v[0].as_ref()?;let side=v[1].as_ref()?;let front=if supports_front(&target.item){v[3].as_ref()}else{None};
    let mut out=data::make_frame(target,side,v[2].as_ref(),front)?;
    out.count=[Some(target),Some(side),v[2].as_ref(),front].into_iter().flatten().map(|s|s.count).min()?;
    (out.count>0).then_some(out)
}
fn shown(v:&[Option<ItemStack>])->Vec<Option<ItemStack>>{let mut slots=v.to_vec();slots.push(result(v));slots.resize(9,None);slots}
pub(super) fn material(game:&Game<'_>,s:&ItemStack)->bool{
    let Some(state)=game.items().get(&s.item).and_then(|i|i.block)else{return false;};
    let Some(block)=game.pack.assets.registry.state(state).map(|s|s.name.as_str())else{return false;};let facts=game.res.behaviours().facts(state);
    let config=super::sd_integration::feature_config();
    if config["framing_solid_materials"].as_bool().unwrap_or(true)&&facts.flags&horizon_world::level::data::state_flags::FULL_COLLISION==0{return false;}
    if config["framing_opaque_materials"].as_bool().unwrap_or(false)&&facts.flags&horizon_world::level::data::state_flags::CAN_OCCLUDE==0{return false;}
    !config["framing_material_denylist"].as_array().is_some_and(|v|v.iter().any(|x|x.as_str().is_some_and(|x|x==block||block.split_once(':').is_some_and(|(n,_)|n==x))))
}
fn accepts(game:&Game<'_>,v:&[Option<ItemStack>],slot:usize,s:&ItemStack)->bool{
    horizon_client::menu::sd_rules::frame_accepts(v,slot,s,material(game,s))
}
fn unpack(v:&mut Vec<Option<ItemStack>>){
    let Some(input)=v[0].as_ref()else{return;};let Some((base,materials))=data::unpack_frame(input)else{return;};
    if materials.iter().enumerate().any(|(i,s)|s.is_some()&&v[i+1].is_some()){return;}
    v[0]=Some(base);for (i,material) in materials.into_iter().enumerate(){if material.is_some(){v[i+1]=material;}}
}
fn take_result(v:&mut [Option<ItemStack>],amount:u32)->Option<ItemStack>{
    let mut out=result(v)?;out.count=out.count.min(amount);if out.count==0{return None;}
    for i in 0..4{if i==3&&!supports_front(&out.item){continue;}if let Some(s)=&mut v[i]{s.count-=out.count;if s.count==0{v[i]=None;}}}Some(out)
}
pub(super) fn open_menu(game:&mut Game<'_>,uuid:u128,p:Pos,menu:&str,title:Text,items:Vec<Option<ItemStack>>,hidden:Vec<usize>)->Option<i32>{
    use horizon_client::lan::{data::component_nbt,menu::Slots};
    let conn=game.native_conn(uuid)?;let mut host=game.lan.take()?;
    game.guest_close_all(&mut host,conn);
    // Common menu opening sends the window's authoritative position before
    // OpenScreen. Slots::Native keeps this table's atomic crafting handler.
    host.hub.open_container(conn,menu,component_nbt(&title.component()),
        (p[0],p[1],p[2]),Slots::Native,items.clone(),Some(Vec::new()));
    let window=host.hub.guests.iter_mut().find(|g|g.conn==conn).and_then(|g|{
        let m=g.menu.as_ref().filter(|m|m.slots==Slots::Native&&m.pos==Some((p[0],p[1],p[2])))?;
        let window=m.window;g.native_menu=items;Some(window)
    });
    if window.is_some(){host.hub.hide_menu_slots(conn,hidden);}
    game.lan=Some(host);window
}
pub(super) fn tick(game:&mut Game<'_>){
    let cleanup=CLEANUP.with_borrow_mut(std::mem::take);for (dimension,p,facing,part) in cleanup{
        if dimension!=game.dim.id(){CLEANUP.with_borrow_mut(|v|v.push((dimension,p,facing,part)));continue;}
        if table(game,p)&&game.native_block_property(p,"facing").as_deref()==Some(&facing)&&game.native_block_property(p,"part").as_deref()==Some(&part){
            if let Some(air)=game.pack.assets.registry.default_state("minecraft:air"){game.native_set_block(game.dim.id(),p,air,35);}
        }
    }
    for (uuid,dimension,p) in OPEN.with_borrow_mut(std::mem::take){
        if dimension!=game.dim.id()||!table(game,p){continue;}
        let v=inputs(game,p).unwrap_or_else(||vec![None;4]);
        if let Some(window)=open_menu(game,uuid,p,"storagedrawers:framing",Text::tr("block.storagedrawers.framing_table",&[]),shown(&v),(5..9).collect()){
            MENUS.with_borrow_mut(|m|{m.insert(uuid,(window,dimension,p));});DRAGS.with_borrow_mut(|d|{d.remove(&uuid);});
        }
    }
    let menus=MENUS.with_borrow(|m|m.clone());for (uuid,(window,dimension,p)) in menus{
        if dimension!=game.dim.id(){continue;}
        let open=game.lan.as_ref().and_then(|h|h.hub.guests.iter().find(|p|p.uuid==uuid)).and_then(|p|p.menu.as_ref()).is_some_and(|m|m.window==window);
        if !open{close(uuid,window);continue;}
        let valid=table(game,p)&&game.native_server_players().iter().any(|v|v.uuid==uuid&&(0..3).map(|i|(v.position[i]-p[i]as f64-0.5).powi(2)).sum::<f64>()<=64.);
        if !valid{game.native_close_menu(uuid);close(uuid,window);continue;}
        if let Some(v)=inputs(game,p){let items=shown(&v);if game.native_menu_contents(uuid).as_ref()!=Some(&items){game.native_set_menu_items(uuid,items);}}
    }
}
pub(super) fn screen(block:&str,out:&mut Option<horizon_client::mod_api::NativeScreen>){
    if block!="storagedrawers:framing_table"{return;}
    let mut slots=vec![None;9];for (i,p) in [[23.,35.],[50.,17.],[102.,17.],[50.,53.],[133.,35.]].into_iter().enumerate(){slots[i]=Some(p);}
    *out=Some(horizon_client::mod_api::NativeScreen{width:176.,height:166.,slots,inventory_y:84.,inventory_x:8.,own_labels:true,own_background:true});
}
pub(super) fn screen_draw(game:&Game<'_>,block:&str,batch:&mut horizon_client::hud::HudBatch,[x,y]:[f32;2]){
    if block!="storagedrawers:framing_table"{return;}
    game.native_sprite(batch,"storagedrawers:gui/framing",[0.,0.,176.,166.],[x,y,176.,166.]);
    for (key,p) in [("block.storagedrawers.framing_table",[x+8.,y+6.]),("container.inventory",[x+8.,y+73.])]{game.native_text(batch,&Text::tr(key,&[]).plain(),p,0x404040,false);}
}
// All clicks read and commit the actual block inventory, so simultaneous
// viewers cannot craft the same input twice or overwrite another viewer.
pub(super) fn menu_click(game:&mut Game<'_>,uuid:u128,window:i32,slot:i32,button:i32,mode:i32)->bool{
    let Some((expected,dimension,p))=MENUS.with_borrow(|m|m.get(&uuid).cloned())else{return false;};
    if window!=expected||dimension!=game.dim.id(){return true;}
    let Some(player)=game.native_server_players().into_iter().find(|v|v.uuid==uuid)else{return true;};
    if !table(game,p)||(0..3).map(|i|(player.position[i]-p[i]as f64-0.5).powi(2)).sum::<f64>()>64.{game.native_close_menu(uuid);close(uuid,window);return true;}
    let Some(mut v)=inputs(game,p)else{return true;};
    let Some(host)=game.lan.as_ref()else{return true;};let Some(player_record)=host.hub.guests.iter().find(|v|v.uuid==uuid)else{return true;};
    let mut inv=player_record.inventory.clone();let mut cursor=player_record.carried.clone();let mut offhand=player_record.offhand.clone();
    let old=(v.clone(),inv.clone(),cursor.clone(),offhand.clone());let creative=player.game_mode==1;let max=|s:&ItemStack|game.stack_max_size(s);let mut drops=Vec::new();
    let initial_target = v[0].clone();
    let drag=DRAGS.with_borrow_mut(|d|d.remove(&uuid));
    let mut action=mode;let mut clicked=slot;let mut mouse=button;
    if mode!=5&&drag.is_some(){action=-1;}
    if mode==5{
        let (stage,kind)=(button&3,(button>>2)&3);action=-1;
        if stage==0&&drag.is_none()&&matches!(kind,0|1|2)&&(kind<2||creative){if let Some(s)=cursor.clone(){DRAGS.with_borrow_mut(|d|{d.insert(uuid,(kind,Vec::new(),s));});}}
        else if stage==1{if let Some((ty,mut slots,source))=drag.filter(|(ty,_,s)|*ty==kind&&cursor.as_ref()==Some(s)){
            if !slots.contains(&slot)&&(kind==2||source.count as usize>slots.len())&&can_put(game,&v,&inv,slot,&source){slots.push(slot);}
            DRAGS.with_borrow_mut(|d|{d.insert(uuid,(ty,slots,source));});
        }}else if stage==2{if let Some((_,slots,source))=drag.filter(|(ty,_,s)|*ty==kind&&cursor.as_ref()==Some(s)){
            if slots.len()==1&&kind<2{action=0;clicked=slots[0];mouse=kind;}
            else if !slots.is_empty(){let each=if kind==0{source.count/slots.len()as u32}else if kind==1{1}else{max(&source)};let mut left=source.count;
                for slot in slots{let amount=if kind==2{each}else{each.min(left)};let put=put(game,&mut v,&mut inv,slot,&source,amount);if kind!=2{left=left.saturating_sub(put);}}
                let mut source=source;source.count=left;cursor=(left>0).then_some(source);
            }
        }}
    }
    if matches!(action,0|1)&&!matches!(mouse,0|1){action=-1;}
    if clicked==-999&&matches!(action,0|1){if let Some(mut source)=cursor.take(){let mut out=source.clone();out.count=if mouse==1{1}else{source.count};source.count-=out.count;cursor=(source.count>0).then_some(source);drops.push(out);}}
    else if clicked==4{
        if action==0{
            if let Some(out)=result(&v){let room=cursor.as_ref().map_or(max(&out),|c|if c.same_item_same_components(&out){max(c).saturating_sub(c.count)}else{0});
                let n=room.min(if cursor.is_none()&&mouse==1{out.count.div_ceil(2)}else{out.count});
                if let Some(out)=take_result(&mut v,n){if let Some(c)=cursor.as_mut(){c.count+=out.count;}else{cursor=Some(out);}}
            }
        }else if action==1{
            while let Some(out)=result(&v){let before=inv.clone();let rest=fit_inventory(&mut inv,out.clone(),true,&max);let n=out.count-rest.as_ref().map_or(0,|s|s.count);
                if n==0{inv=before;break;}take_result(&mut v,n);}
        }else if action==2&&matches!(mouse,0..=8|40){
            let dest=if mouse==40{&mut offhand}else{&mut inv[mouse as usize]};if dest.is_none(){if let Some(out)=result(&v){*dest=take_result(&mut v,out.count.min(max(&out)));}}
        }else if action==3&&creative&&cursor.is_none(){cursor=result(&v).map(|mut s|{s.count=max(&s);s});}
        else if action==4&&cursor.is_none(){if let Some(out)=take_result(&mut v,if mouse==0{1}else{u32::MAX}){drops.push(out);}}
    }else if (0..4).contains(&clicked)||(9..45).contains(&clicked){
        if action==0{
            let mut ring=get(&v,&inv,clicked);let ring_click=mouse==1&&ring.as_mut().is_some_and(|ring|data::ring_click(ring,&mut cursor));
            if ring_click{set(&mut v,&mut inv,clicked,ring);}
            else if let Some(c)=cursor.as_ref(){if can_put(game,&v,&inv,clicked,c){let n=if mouse==1{1}else{c.count};let input=c.clone();let moved=put(game,&mut v,&mut inv,clicked,&input,n);
                if moved>0{let c=cursor.as_mut().unwrap();c.count-=moved;if c.count==0{cursor=None;}}
                else if get(&v,&inv,clicked).as_ref().is_some_and(|s|!s.same_item_same_components(&input))&&input.count<=max(&input){let old=get(&v,&inv,clicked);set(&mut v,&mut inv,clicked,Some(input));cursor=old;}
            }}else if let Some(mut out)=get(&v,&inv,clicked){let n=if mouse==1{out.count.div_ceil(2)}else{out.count};cursor=take(&mut v,&mut inv,clicked,n);out.count=0;}
        }else if action==1{
            if clicked<4{if let Some(out)=get(&v,&inv,clicked){let rest=fit_inventory(&mut inv,out,true,&max);set(&mut v,&mut inv,clicked,rest);}}
            else{let i=inventory_slot(clicked).unwrap();if let Some(input)=inv[i].clone(){
                let valid=data::framed_id(&input.item).is_some()||material(game,&input);
                let range=if data::framed_id(&input.item).is_some(){0..1}else if material(game,&input){1..4}else{0..0};let mut left=input.count;
                for target in range{let mut source=input.clone();source.count=left;left-=put(game,&mut v,&mut inv,target,&source,left);}
                inv[i]=if left>0{let mut input=input;input.count=left;Some(input)}else{None};
                if !valid&&left>0{let mut stack=inv[i].take().unwrap();let order=if i<9{(9..36).collect::<Vec<_>>()}else{(0..9).collect()};merge_into(&mut inv,&mut stack,&order,&max);inv[i]=(stack.count>0).then_some(stack);}
            }}
        }else if action==2&&matches!(mouse,0..=8|40){
            let source=if mouse==40{offhand.clone()}else{inv[mouse as usize].clone()};let old=get(&v,&inv,clicked);
            if source.as_ref().is_none_or(|s|can_put(game,&v,&inv,clicked,s)){
                set(&mut v,&mut inv,clicked,source);if mouse==40{offhand=old;}else{inv[mouse as usize]=old;}
            }
        }else if action==3&&creative&&cursor.is_none(){cursor=get(&v,&inv,clicked).map(|mut s|{s.count=max(&s);s});}
        else if action==4&&cursor.is_none(){if let Some(out)=take(&mut v,&mut inv,clicked,if mouse==0{1}else{u32::MAX}){drops.push(out);}}
    }
    if action==6&&cursor.is_some()&&((0..5).contains(&clicked)||(9..45).contains(&clicked))&&(if clicked==4{result(&v).is_none()}else{get(&v,&inv,clicked).is_none()}){
        let c=cursor.as_mut().unwrap();let order:Vec<i32>=if mouse==0{(0..45).collect()}else{(0..45).rev().collect()};
        for full in [false,true]{for &s in &order{if c.count>=max(c){break;}if let Some(item)=get(&v,&inv,s){if item.same_item_same_components(c)&&(item.count==max(&item))==full{
            if let Some(out)=take(&mut v,&mut inv,s,max(c)-c.count){c.count+=out.count;}
        }}}}
    }
    if v[0] != initial_target { unpack(&mut v); }
    if (v.clone(),inv.clone(),cursor.clone(),offhand.clone())!=old{
        game.native_with_mod_block_entity(p,|m|{m.items=v.clone();});game.native_block_entity_sync(p);
        if let Some(host)=game.lan.as_mut(){if let Some(i)=host.hub.guests.iter().position(|v|v.uuid==uuid){let g=&mut host.hub.guests[i];g.inventory=inv;g.carried=cursor;g.offhand=offhand;if g.offhand!=old.3{host.hub.send_inventory(i);}}}
        if !drops.is_empty(){if let Some(host)=game.lan.take(){if let Some(i)=host.hub.guests.iter().position(|v|v.uuid==uuid){game.drop_guest_items(&host,i,drops);}game.lan=Some(host);}}
    }
    game.native_set_menu_items(uuid,shown(&v));if let Some(conn)=game.native_conn(uuid){if let Some(host)=game.lan.as_mut(){host.hub.resend_menu(conn,window,&game.world);}}true
}
fn inventory_slot(slot:i32)->Option<usize>{super::native_menus::native_menu_inventory_slot(9,slot)}
fn get(v:&[Option<ItemStack>],inv:&[Option<ItemStack>],slot:i32)->Option<ItemStack>{if (0..4).contains(&slot){v[slot as usize].clone()}else{inventory_slot(slot).and_then(|i|inv[i].clone())}}
fn set(v:&mut [Option<ItemStack>],inv:&mut [Option<ItemStack>],slot:i32,item:Option<ItemStack>){if (0..4).contains(&slot){v[slot as usize]=item;}else if let Some(i)=inventory_slot(slot){inv[i]=item;}}
fn take(v:&mut [Option<ItemStack>],inv:&mut [Option<ItemStack>],slot:i32,amount:u32)->Option<ItemStack>{
    let mut item=get(v,inv,slot)?;let mut out=item.clone();out.count=amount.min(item.count);item.count-=out.count;set(v,inv,slot,(item.count>0).then_some(item));Some(out)
}
fn can_put(game:&Game<'_>,v:&[Option<ItemStack>],inv:&[Option<ItemStack>],slot:i32,s:&ItemStack)->bool{
    if (0..4).contains(&slot){accepts(game,v,slot as usize,s)}else{inventory_slot(slot).is_some_and(|i|i<inv.len())}
}
fn put(game:&Game<'_>,v:&mut Vec<Option<ItemStack>>,inv:&mut [Option<ItemStack>],slot:i32,input:&ItemStack,amount:u32)->u32{
    if amount==0||!can_put(game,v,inv,slot,input){return 0;}let existing=get(v,inv,slot);
    if existing.as_ref().is_some_and(|s|!s.same_item_same_components(input)){return 0;}
    let count=existing.as_ref().map_or(0,|s|s.count);let put=amount.min(input.count).min(game.stack_max_size(input).saturating_sub(count));
    if put>0{let mut out=input.clone();out.count=count+put;set(v,inv,slot,Some(out));if slot==0{unpack(v);}}put
}
fn merge_into(inv:&mut [Option<ItemStack>],source:&mut ItemStack,order:&[usize],max:&impl Fn(&ItemStack)->u32){
    for &i in order{if let Some(s)=inv[i].as_mut().filter(|s|s.same_item_same_components(source)){let n=source.count.min(max(s).saturating_sub(s.count));s.count+=n;source.count-=n;}}
    for &i in order{if source.count>0&&inv[i].is_none(){let mut out=source.clone();out.count=source.count.min(max(source));source.count-=out.count;inv[i]=Some(out);}}
}
fn fit_inventory(inv:&mut [Option<ItemStack>],mut source:ItemStack,reverse:bool,max:&impl Fn(&ItemStack)->u32)->Option<ItemStack>{
    let order=if reverse{(0..9).rev().chain((9..36).rev()).collect::<Vec<_>>()}else{(0..36).collect()};merge_into(inv,&mut source,&order,max);(source.count>0).then_some(source)
}

pub(super) fn use_on(game:&mut Game<'_>,stack:&ItemStack,pos:Pos,face:Pos,claimed:&mut bool){
    if *claimed||&*stack.item!="storagedrawers:framing_table"{return;}
    let state=game.world.block(pos[0],pos[1],pos[2]);
    let target=if game.res.behaviours().facts(state).flags&horizon_world::level::data::state_flags::REPLACEABLE!=0{pos}else{std::array::from_fn(|i|pos[i]+face[i])};
    if !may_place(game,stack,target){*claimed=true;}
}
pub(super) fn update_tag(entity:&ModBlockEntity,tag:&mut Compound){
    if entity.id=="storagedrawers:framing_table"{tag.insert("sd_table_items".into(),Tag::List(entity.items.iter().map(|s|s.as_ref().map_or(Tag::Compound(Compound::new()),horizon_items::item_nbt::item_to_nbt)).collect()));}
}
pub(super) fn render(game:&Game<'_>,p:Pos,m:&ModBlockEntity,draw:&mut super::native_block_entity_draw::BlockEntityDraw){
    if game.native_block_property(p,"part").as_deref()!=Some("right"){return;}
    let v=match m.nbt.get(&"sd_table_items".into()){Some(Tag::List(v))=>v.iter().map(horizon_items::item_nbt::item_from_nbt).collect::<Vec<_>>(),_=>m.items.clone()};
    if v.len()<4{return;}
    let facing=game.native_block_property(p,"facing").unwrap_or_default();
    let (angle,offset)=match facing.as_str(){"north"=>(180.,[-0.5,0.8,0.]),"east"=>(90.,[-0.5,0.8,1.]),"west"=>(270.,[0.5,0.8,0.]),_=>(0.,[0.5,0.8,1.])};
    let output=result(&v);
    for (item,scale,at) in [(v[1].as_ref(),1.,[1.15,0.15,-0.275]),(v[2].as_ref(),1.,[-0.15,0.15,-0.275]),(v[3].as_ref(),1.,[1.15,0.15,-0.725]),(output.as_ref(),1.6,[0.5,0.1,-0.5])]{
        if let Some(item)=item{draw.push();draw.rotate_about([0.,1.,0.],angle,[0.,0.,0.]);draw.translate(offset);draw.translate(at);draw.scale([scale;3]);game.native_draw_item(draw,item,horizon_assets::item_model::DisplayContext::Ground,None,0);draw.pop();}
    }
}

// Both halves use the ordinary loot pipeline. Destruction only removes the
// counterpart; it never independently spawns a second table item.
pub(super) fn destroy(game:&mut Game<'_>,pos:Pos,canceled:&mut bool){
    if *canceled||!table(game,pos){return;}
    let facing=game.native_block_property(pos,"facing").unwrap_or_default();let part=game.native_block_property(pos,"part").unwrap_or_default();
    let counterpart=other(pos,&facing,&part);let expected=if part=="right"{"left"}else{"right"};
    if table(game,counterpart)&&game.native_block_property(counterpart,"facing").as_deref()==Some(&facing)&&game.native_block_property(counterpart,"part").as_deref()==Some(expected){
        if part=="left"{if let Some(components)=entity(game,counterpart).and_then(|m|m.nbt.get(&"sd_item_components".into())).cloned(){
            game.native_with_mod_block_entity(pos,|m|{m.nbt.insert("sd_item_components".into(),components);});
        }}
        if let Some(air)=game.pack.assets.registry.default_state("minecraft:air"){game.native_set_block(game.dim.id(),counterpart,air,35);}
    }
}
pub(super) fn loot(game:&Game<'_>,context:&horizon_client::mod_api::NativeLootDrops<'_>,drops:&mut Vec<ItemStack>){
    let Some((p,id))=context.block else{return;};let Some(state)=game.pack.assets.registry.state(id)else{return;};
    if state.name!="storagedrawers:framing_table"||!drops.iter().any(|s|&*s.item=="storagedrawers:framing_table"&&s.count>0){return;}
    let facing=state.properties.get("facing").map_or("north",String::as_str);let part=state.properties.get("part").map_or("right",String::as_str);
    let master=if part=="left"{other(p,facing,part)}else{p};let key=(context.dimension.to_owned(),master);let time=game.world.game_time;
    let duplicate=DROP_CLAIMS.with_borrow_mut(|claims|{
        claims.retain(|_,t|*t<=time&&*t>=time.saturating_sub(2));
        if claims.contains_key(&key){true}else{claims.insert(key,time);false}
    });
    let mut kept=false;drops.retain_mut(|s|{
        if &*s.item!="storagedrawers:framing_table"{return true;}
        if duplicate||kept{return false;}s.count=1;kept=true;true
    });
}
