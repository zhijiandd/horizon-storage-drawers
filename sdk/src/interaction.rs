use crate::{engine as sd,state,items,compat::Stack,network,compacting,menu};
use mod_api::*;
use std::{cell::RefCell,collections::BTreeMap};
thread_local!{
    static CLICKS:RefCell<BTreeMap<u128,(String,[i32;3],i64)>>=const{RefCell::new(BTreeMap::new())};
}
pub fn clear() { CLICKS.with_borrow_mut(BTreeMap::clear); }

pub fn key_type(id:&str)->Option<sd::Key>{Some(match items::kind(id){
    "drawer_key"=>sd::Key::Lock,"quantify_key"=>sd::Key::Quantify,"shroud_key"=>sd::Key::Shroud,
    "suspend_key"=>sd::Key::Suspend,"personal_key"=>sd::Key::Personal,"personal_key_unlock"=>sd::Key::UnlockPersonal,
    s if s.starts_with("priority_key")=>sd::Key::Priority,_=>return None,
})}
pub fn facing(host:&dyn Host,at:BlockSide)->String{at.level.block_property(host,at.pos,"facing").unwrap_or_else(||"north".into())}
pub fn front(host:&dyn Host,at:BlockSide,face:[i32;3])->bool{
    face==match facing(host,at).as_str(){"east"=>[1,0,0],"west"=>[-1,0,0],"south"=>[0,0,1],_=>[0,0,-1]}
}
fn slot_for(host:&dyn Host,at:BlockSide,n:usize,point:[f64;3])->Option<usize>{
    let facing=facing(host,at);let y=point[1]-f64::from(at.pos[1]);
    let h=if matches!(facing.as_str(),"north"|"south"){point[0]-f64::from(at.pos[0])}else{point[2]-f64::from(at.pos[2])};
    if !(1./16.<y&&y<15./16.&&1./16.<h&&h<15./16.){return None}
    let left=if matches!(facing.as_str(),"south"|"west"){h<0.5}else{h>0.5};
    let bottom=y<=0.5;let right=!left;
    Some(match n{2=>usize::from(bottom),3=>if !bottom{0}else{1+usize::from(right)},4=>2*usize::from(bottom)+usize::from(right),_=>0})
}
pub fn drawer_panel(host: &dyn Host, at: BlockSide, face: [i32; 3], point: [f64; 3]) -> Option<usize> {
    let block = state::block(host, at)?;
    if !state::is_drawer(&block) || !front(host, at, face) { return None; }
    slot_for(host, at, state::slots(&block), point)
}
pub fn validate_remote(host:&dyn Host,at:BlockSide,input:&Stack)->Stack{
    let Some((pos,dim))=items::binding(input)else{return input.clone()};
    let group=items::upgrade(input)==Some(sd::Upgrade::RemoteGroup);
    let range=sd::remote_range(network::range(),group);
    let origin=sd::Position::new(at.level.dimension_id(),at.pos);let target=sd::Position::new(&dim,pos);
    let wrong=!origin.in_range(&target,range)||at.level.is_loaded(host,pos)&&!state::block(host,BlockSide{pos,..at}).is_some_and(|b|state::is_controller(&b));
    if wrong{items::unbound(input)}else{input.clone()}
}
fn sneak(host:&dyn Host,p:PlayerRef)->bool{p.entity().is_shift_key_down(host)==Some(true)}
fn held_slot(host:&dyn Host,p:PlayerRef,hand:Hand)->Option<u32>{Some(if hand==Hand::Main{p.selected_slot(host)?as u32}else{40})}
fn consume(host:&mut dyn ServerHost,p:PlayerRef,hand:Hand,stack:&ItemStack,n:u32)->Result<(),()>{
    let slot=held_slot(host,p,hand).ok_or(())?;
    p.set_inventory_slot(host,slot,stack.clone().shrink(n)).map_err(|_|())
}
fn feedback(host:&mut dyn ServerHost,p:PlayerRef,id:&str){let _=p.action_bar(host,Text::translate(id,Vec::new()));}
fn bulk(host:&dyn Host,p:PlayerRef,at:BlockSide)->bool{
    let now=at.level.game_time(host).unwrap_or(0);
    CLICKS.with_borrow_mut(|m|m.insert(p.uuid(),(at.level.dimension_id().into(),at.pos,now)))
        .is_some_and(|(d,pos,t)|d==at.level.dimension_id()&&pos==at.pos&&(0..10).contains(&now.saturating_sub(t)))
}
pub fn claims(host:&dyn Host,click:&BlockClick)->bool{
    let Some(p)=host.player(click.player)else{return false};let Some(level)=p.level(host)else{return false};let at=BlockSide{level,pos:click.pos,side:None};
    let Some(block)=state::block(host,at)else{return false};if !block.starts_with("storagedrawers:"){return false}
    let drawer = state::is_drawer(&block);
    let controller = state::is_controller(&block);
    let port = state::is_port(&block);
    let framing = block == "storagedrawers:framing_table";
    if !(drawer || controller || port || framing) { return false; }
    let sneaking=sneak(host,p);
    let held=click.stack.and_then(|s|Stack::from_api(host,s));
    if sneaking && held.as_ref().is_some_and(|s| items::kind(&s.item).starts_with("keyring")) { return false; }
    let tool=held.as_ref().and_then(items::active_key).or_else(||held.clone());
    if tool.as_ref().and_then(|s| key_type(&s.item)).is_some_and(|key| drawer || controller && key != sd::Key::Priority) { return true; }
    if click.hand != Hand::Main { return false; }
    if sneaking{if let Some(living)=host.living(p.entity()){
        if [Hand::Main,Hand::Off].into_iter().filter_map(|h|living.hand_item(host,h)).any(|s|host.item_block(s.item()).is_some()){return false}
    }}
    if framing { return true; }
    if controller { return front(host,at,click.hit.face); }
    if !drawer { return false; }
    (sneaking&&held.is_none())||(!sneaking&&held.as_ref().is_some_and(|s|items::upgrade(s).is_some()))
        ||front(host,at,click.hit.face)&&slot_for(host,at,state::slots(&block),click.hit.point).is_some()
}
pub fn right(host:&mut dyn ServerHost,click:&BlockClick)->bool{
    if !claims(host,click){return false}
    let Some(player)=host.player(click.player)else{return true};let Some(level)=player.level(host)else{return true};
    let at=BlockSide{level,pos:click.pos,side:None};let Some(block)=state::block(host,at)else{return true};
    if block=="storagedrawers:framing_table"{crate::framing::open(host,player,at);return true}
    let access=sd::Access::Player(player.uuid());let sneaking=sneak(host,player);
    let held=click.stack.and_then(|s|Stack::from_api(host,s));
    let tool=held.as_ref().and_then(items::active_key).or_else(||held.clone());
    let controller = state::is_controller(&block);
    if let Some(key)=tool.as_ref().and_then(|s|key_type(&s.item))
        && (state::is_drawer(&block) || controller && key != sd::Key::Priority)
    {
        let cooldown = click.stack.filter(|_|controller || !matches!(key,sd::Key::Personal|sd::Key::UnlockPersonal))
            .map(|stack| host.item_cooldown_group(stack));
        if cooldown.as_ref().is_some_and(|group| player.has_item_cooldown(host, group) == Some(true)) { return true; }
        let priority=tool.as_ref().map_or(0,|s|match items::kind(&s.item){"priority_key_p1"=>1,"priority_key_p2"=>2,"priority_key_n1"=>-1,"priority_key_n2"=>-2,_=>0});
        let applied = if controller {
            let mut net=network::snapshot(host,level);
            let origin=sd::Position::new(level.dimension_id(),at.pos);
            if !matches!(net.nodes.get(&origin),Some(sd::Node::Controller)){return true;}
            net.use_key(&origin,key,player.uuid());
            network::commit(host,level,&net).is_ok()
        }else if let Some((entity,nbt,mut g))=state::read(host,at){
            let old=g.attributes.priority;
            if g.use_key(key,player.uuid()).is_ok(){
                if priority!=0{g.attributes.priority=if old==priority{0}else{priority};}
                if key == sd::Key::Lock { compacting::reset(&mut g); }
                state::write(host,entity,nbt,&g)
            }else{false}
        }else{false};
        if applied && let Some(group) = cooldown { player.set_item_cooldown(host, &group, 5); }
        return true
    }
    if controller{
        if let Some(s)=held.as_ref().filter(|s|matches!(items::upgrade(s),Some(sd::Upgrade::Remote|sd::Upgrade::RemoteGroup))){
            if let Some(bound)=items::bound(s,at.pos,level.dimension_id()).to_api(host){
                if let Some(slot)=held_slot(host,player,click.hand){let _=player.set_inventory_slot(host,slot,Some(bound));}
            }return true
        }
        let mut net=network::snapshot(host,level);let origin=sd::Position::new(level.dimension_id(),at.pos);
        let all=bulk(host,player,at);let selected=held_slot(host,player,click.hand);
        let inventory=player.inventory_slots(host).unwrap_or_default();
        let _:Result<(),()>=host.transaction(|host,_|{
            for (slot,s) in inventory {if (all&&slot<36)||Some(slot)==selected{
                let put=network::insert(host,level,&mut net,&origin,&s,s.count().into(),access);
                if put.accepted()>0{player.set_inventory_slot(host,slot,s.shrink(put.accepted()as u32)).map_err(|_|())?;}
            }}network::commit(host,level,&net)
        });return true
    }
    let Some((entity,mut nbt,mut g))=state::read(host,at)else{return true};
    if g.check(access).is_err(){return true}
    if sneaking&&held.is_none(){menu::open(host,player,at);return true}
    let (mut positions,mut upgrades)=state::upgrade_state(&nbt,&g);
    if !sneaking{if let Some(input)=held.as_ref(){if let Some(u)=items::upgrade(input){
        if g.upgrades_locked_by_missing_drawer() {
            feedback(host,player,"message.storagedrawers.missing_slots_upgrade");
            return true;
        }
        if matches!(u,sd::Upgrade::Remote|sd::Upgrade::RemoteGroup)&&items::binding(input).is_some(){
            if let Some(i)=g.upgrades.iter().position(|u|matches!(u,sd::Upgrade::Remote|sd::Upgrade::RemoteGroup)){
                if let Some((p,d))=items::binding(input){upgrades[i]=validate_remote(host,at,&items::bound(&upgrades[i],p,&d));
                    if state::save_upgrades(&mut nbt,&g,&positions,&upgrades){state::write(host,entity,nbt,&g);}}
                return true
            }
        }
        let result = g.add_upgrade(u,access);
        if result.is_ok(){
            positions.push((0..sd::UPGRADE_SLOTS).find(|p|!positions.contains(p)).unwrap());
            let mut installed=validate_remote(host,at,input);installed.count=1;upgrades.push(installed);
            let saved = state::save_upgrades(&mut nbt,&g,&positions,&upgrades) && host.transaction(|host,_|{
                if player.game_mode(host) != Some(GameMode::Creative) {
                    consume(host, player, click.hand, click.stack.ok_or(())?, 1)?;
                }
                state::write(host, entity, nbt, &g).then_some(()).ok_or(())
            }).is_ok();
            if !saved { feedback(host, player, "message.storagedrawers.cannot_add_upgrade"); }
        } else {
            let message = match result {
                Err(sd::Error::UpgradeSlots) => "message.storagedrawers.max_upgrades",
                _ => "message.storagedrawers.cannot_add_upgrade",
            };
            feedback(host, player, message);
        }
        return true
    }}}
    let Some(clicked)=slot_for(host,at,state::slots(&block),click.hit.point)else{return false};
    if let Some(tool)=tool.as_ref(){let pulling=items::kind(&tool.item)=="drawer_puller";
        if pulling||items::detached(tool).is_some(){
            if g.compact.is_some(){return true}
            if pulling{
                let cap = g.upgraded_stacks();
                if let Ok(out)=g.detach(clicked,access){
                    let stock=out.drawer.item.as_ref().and_then(|i|state::stack(host,i,out.drawer.count.try_into().ok()?)).and_then(|s|Stack::from_api(host,&s));
                    if out.drawer.count > 0 && stock.is_none() { return true; }
                    let item=items::make_detached(stock,cap,sd::config::boolean("detached_heavy",false)&&!g.has(sd::Upgrade::Portability));
                    if let Some(item)=item.to_api(host){let _:Result<(),()>=host.transaction(|host,_|{
                        if !state::write(host,entity,nbt,&g){return Err(())}player.give_stack(host,item).map_err(|_|())
                    });}
                }
            }else if let Some((stock,capacity,_))=items::detached(tool){
                let cap = g.upgraded_stacks();
                if sd::config::boolean("detached_force_capacity_check",false)&&capacity<cap{return true}
                let item = match stock.as_ref() {
                    Some(stack) => {
                        let Some(stack) = stack.to_api(host) else { return true; };
                        Some(state::key(host, &stack))
                    }
                    None => None,
                };
                let drawer=sd::DetachedDrawer{drawer:sd::Drawer{item,count:stock.as_ref().map_or(0,|s|s.count.into()),detached:false},capacity};
                if g.attach(clicked,&drawer,access).is_ok(){let _:Result<(),()>=host.transaction(|host,_|{
                    if !state::write(host,entity,nbt,&g){return Err(())}consume(host,player,click.hand,click.stack.ok_or(())?,1)
                });}
            }return true
        }
    }
    if let Some(s)=click.stack{if !compacting::seed(host,at,&block,&mut g,s,access){return true}}
    let all=bulk(host,player,at)&&g.item(clicked.min(g.drawers.len()-1)).is_some();
    let selected=held_slot(host,player,click.hand);let inventory=player.inventory_slots(host).unwrap_or_default();
    let _:Result<(),()>=host.transaction(|host,_|{
        for (slot,s) in inventory {if (all&&slot<36)||Some(slot)==selected{
            let key=state::group_key(host,&g,&s);
            let target=if let Some(c)=&g.compact{let Some(i)=c.views.iter().position(|(k,_)|k.same_item(&key))else{continue};i}else{clicked};
            let put=state::insert(host,&mut g,&[target],&s,s.count().into(),access);
            if put.accepted()>0{player.set_inventory_slot(host,slot,s.shrink(put.accepted()as u32)).map_err(|_|())?;}
        }}if state::write(host,entity,nbt,&g){Ok(())}else{Err(())}
    });true
}
pub fn left(host:&mut dyn ServerHost,event:&mut LeftClickBlockEvent)->bool{
    let Some(p)=host.players().acting_player()else{return false};let Some(level)=p.level(host)else{return false};
    let at=BlockSide{level,pos:event.pos,side:None};let Some(block)=state::block(host,at)else{return false};
    if !state::is_drawer(&block)||!front(host,at,event.face){return false}
    let Some(eye)=p.entity().eye_position(host)else{return false};let Some([yaw,pitch])=p.entity().rotation(host)else{return false};
    let (yaw,pitch)=(f64::from(yaw).to_radians(),f64::from(pitch).to_radians());
    let dir=[-yaw.sin()*pitch.cos(),-pitch.sin(),yaw.cos()*pitch.cos()];
    let axis = if event.face[0] != 0 { 0 } else { 2 };
    if dir[axis] * f64::from(event.face[axis]) >= -1e-8 { return false; }
    let Some(boxes) = host.state_outline(event.state) else { return false; };
    let plane = axis + if event.face[axis] > 0 { 3 } else { 0 };
    let Some((_, point)) = boxes.into_iter().filter_map(|bounds| {
        let t = (f64::from(at.pos[axis]) + bounds[plane] - eye[axis]) / dir[axis];
        if !t.is_finite() || t < 0. { return None; }
        let point = std::array::from_fn(|i| eye[i] + dir[i] * t);
        (0..3).filter(|&i| i != axis).all(|i| {
            let value = point[i] - f64::from(at.pos[i]);
            value >= bounds[i] && value <= bounds[i + 3]
        }).then_some((t, point))
    }).min_by(|a, b| a.0.total_cmp(&b.0)) else { return false; };
    let Some(slot)=slot_for(host,at,state::slots(&block),point)else{return false};
    let creative=p.game_mode(host)==Some(GameMode::Creative);
    let Some((entity,nbt,mut g))=state::read(host,at)else{return creative};
    let count=if sneak(host,p){g.item(slot).map_or(64,|k|k.max_stack)}else{1};
    if let Ok(Some(out))=g.extract(slot,count.into(),sd::Access::Player(p.uuid()),false){
        if let Some(s)=state::stack(host,&out.item,out.count as u32){compacting::reset(&mut g);let _:Result<(),()>=host.transaction(|host,_|{
            if !state::write(host,entity,nbt,&g){return Err(())}p.give_stack(host,s).map_err(|_|())
        });}
    }creative
}
pub fn placed(host:&mut dyn ServerHost,event:&mut OnBlockPlacedByEvent){
    let Some(level)=host.level(host.dimension())else{return};let at=BlockSide{level,pos:event.pos,side:None};
    if event.stack.item().id(host) == "storagedrawers:framing_table" {
        crate::framing::placed(host, at, event.state);
        return;
    }
    let Some(input)=Stack::from_api(host,event.stack)else{return};
    let Some(entity)=host.block_entity(level,event.pos)else{return};let Some(mut nbt)=entity.data(host)else{return};
    if let Some(components)=event.stack.components(){
        nbt.insert("sd_item_components".into(),Tag::Compound(components.as_nbt().clone()));
    }
    nbt.insert("sd_dimension".into(),Tag::String(level.dimension_id().into()));
    if let Some(frame)=items::frame(&input){nbt.insert("sd_frame".into(),frame);}
    if let Some(g)=items::group(&input){
        let custom=items::custom(&input);let (positions,upgrades)=state::upgrade_state(&custom,&g);
        if state::save_upgrades(&mut nbt,&g,&positions,&upgrades){state::write(host,entity,nbt,&g);}
    }else{let _=entity.replace_data(host,nbt);}
    state::remember(host,entity);
}

pub fn use_item(host: HostSide, event: &UseItemEvent<'_>) -> Option<InteractionResult> {
    let shared: &dyn Host = match &host {
        HostSide::Server(host) => &**host,
        HostSide::Client(host) => &**host,
    };
    let player = shared.player(event.player)?;
    if !sneak(shared, player) { return None; }
    let mut stack = Stack::from_api(shared, event.stack)?;
    let id = items::kind(&stack.item);
    if id.starts_with("priority_key") {
        let next = match id {
            "priority_key" => "priority_key_p1",
            "priority_key_p1" => "priority_key_p2",
            "priority_key_p2" => "priority_key_n1",
            "priority_key_n1" => "priority_key_n2",
            _ => "priority_key",
        };
        stack.item = format!("storagedrawers:{next}");
    } else if id.starts_with("keyring") {
        let keys = items::ring_items(&stack);
        if keys.is_empty() { return None; }
        let active = items::active_key(&stack)
            .and_then(|key| keys.iter().position(|item| item == &key)).unwrap_or(0);
        items::save_ring(&mut stack, &keys, (active + 1) % keys.len());
    } else {
        return None;
    }
    if let HostSide::Server(host) = host {
        let slot = held_slot(host, player, event.hand)?;
        let stack = stack.to_api(host)?;
        player.set_inventory_slot(host, slot, Some(stack)).ok()?;
    }
    Some(InteractionResult::Success)
}

pub fn ring_click(event: &mut StackedOnOther<'_>) -> bool {
    if event.primary || !event.clicked.allow_modification() { return false; }
    let Some(old) = event.clicked.item() else { return false; };
    let Some(mut ring) = Stack::from_recipe(&old) else { return false; };
    if !items::kind(&ring.item).starts_with("keyring") { return false; }
    let mut cursor = match event.carried.get() {
        Some(stack) => match Stack::from_recipe(stack) { Some(stack) => Some(stack), None => return false },
        None => None,
    };
    if !items::ring_click(&mut ring, &mut cursor) { return false; }
    let Some(output) = ring.to_recipe() else { return false; };
    let cursor = match cursor {
        Some(stack) => match stack.to_recipe() { Some(stack) => Some(stack), None => return false },
        None => None,
    };
    if !event.clicked.may_place(&output) { return false; }
    let Some(removed) = event.clicked.safe_take(1, 1) else { return false; };
    if event.clicked.safe_insert(Some(output), 1).is_some() {
        let _ = event.clicked.safe_insert(Some(removed), 1);
        return true;
    }
    event.carried.set(cursor);
    event.cancellation_result = true;
    true
}

pub fn press_button(host: &mut dyn ServerHost, click: &BlockClick) {
    if click.hand != Hand::Main { return; }
    let Some(player) = host.player(click.player) else { return; };
    let Some(level) = player.level(host) else { return; };
    let at = BlockSide { level, pos: click.pos, side: None };
    let Some(block) = state::block(host, at) else { return; };
    let key = match block.as_str() {
        "storagedrawers:keybutton_drawer" => sd::Key::Lock,
        "storagedrawers:keybutton_quantify" => sd::Key::Quantify,
        "storagedrawers:keybutton_concealment" => sd::Key::Shroud,
        _ => return,
    };
    if level.block_property(host, at.pos, "powered").as_deref() == Some("true") { return; }
    if sneak(host, player) && host.living(player.entity()).is_some_and(|living|
        [Hand::Main, Hand::Off].into_iter().any(|hand| living.hand_item(host, hand).is_some()))
    {
        return;
    }
    let offset = match level.block_property(host, at.pos, "face").as_deref() {
        Some("floor") => [0, -1, 0],
        Some("ceiling") => [0, 1, 0],
        _ => match facing(host, at).as_str() {
            "south" => [0, 0, -1],
            "west" => [1, 0, 0],
            "east" => [-1, 0, 0],
            _ => [0, 0, 1],
        },
    };
    let backing = sd::Position::new(level.dimension_id(), std::array::from_fn(|i| at.pos[i] + offset[i]));
    let mut net = network::snapshot(host, level);
    if matches!(net.nodes.get(&backing), Some(sd::Node::Controller | sd::Node::Port))
        && let Some(root) = network::root(&net, &backing)
    {
        net.use_key(&root, key, player.uuid());
        let _ = network::commit(host, level, &net);
    }
}

pub fn validate_inventory_bindings(host: &mut dyn ServerHost, level: LevelRef) {
    let heavy = sd::config::boolean("detached_heavy", false);
    let now = level.game_time(host).unwrap_or_default();
    for player in level.online_players(host).unwrap_or_default() {
        let mut burdened = false;
        for (slot, input) in player.inventory_slots(host).unwrap_or_default() {
            let Some(stack) = Stack::from_api(host, &input) else { continue; };
            if let Some((pos, dim)) = items::binding(&stack)
                && (dim != level.dimension_id() || level.is_loaded(host, pos)
                    && !state::block(host, BlockSide { level, pos, side: None }).is_some_and(|id| state::is_controller(&id)))
                && let Some(unbound) = items::unbound(&stack).to_api(host)
            {
                let _ = player.set_inventory_slot(host, slot, Some(unbound));
            }
            if heavy && now % 60 == 0 && let Some((Some(stock), _, true)) = items::detached(&stack)
                && let Some(item) = crate::compat::registered_item(&stock.item)
            {
                burdened |= stock.count > host.max_stack_size(item).unwrap_or(64);
            }
        }
        if burdened && let Some(living) = host.living(player.entity())
            && let Some(effect) = host.effect_key("minecraft:slowness")
        {
            living.add_effect(host, &ActiveEffect::new(effect, 100, 3, true, true, true));
        }
    }
}

pub fn tooltip(host: &dyn ClientHost, event: &mut ItemTooltipEvent<'_>) {
    let id = event.stack.item().id(host);
    if !id.starts_with("storagedrawers:") { return; }
    let Some(stack) = Stack::from_api(host, event.stack) else { return; };
    let kind = items::kind(&id);
    let description_id = if kind.starts_with("keyring") { "keyring" }
        else if kind == "detached_drawer_full" { "detached_drawer" }
        else if items::upgrade(&stack).and_then(sd::Upgrade::multiplier).is_some() { "storage_upgrade" }
        else { kind };
    let key = format!("item.storagedrawers.{description_id}.desc");
    if let Some(mut description) = host.translate(&key).filter(|text| text != &key) {
        if let Some(multiplier) = items::upgrade(&stack).and_then(sd::Upgrade::multiplier) {
            description = description.replacen("%d", &multiplier.to_string(), 1);
        }
        for line in description.split("\\n").flat_map(str::lines) {
            event.lines.push((Text::literal(line), 0xffa0a0a0));
        }
    }
    if matches!(items::upgrade(&stack), Some(sd::Upgrade::Remote | sd::Upgrade::RemoteGroup)) {
        let key = format!("item.storagedrawers.{kind}.bound");
        if let Some(mut message) = host.translate(&key).filter(|text| text != &key) {
            if let Some((pos, _)) = items::binding(&stack) {
                for coordinate in pos { message = message.replacen("%d", &coordinate.to_string(), 1); }
            }
            event.lines.push((Text::literal(message), 0xffffff55));
        }
    }
    if let Some((Some(stock), _, true)) = items::detached(&stack)
        && stock.to_api(host).is_some_and(|item| stock.count > state::max_stack(host, &item))
    {
        event.lines.push((Text::translate("tooltip.storagedrawers.drawers.too_heavy", Vec::new()), 0xffff5555));
    }
    if state::is_drawer(&id) {
        let base = if kind.contains("compacting") {
            let half = kind.contains("half");
            sd::config::uint(if half { "half_compacting_units" } else { "compacting_units" }, if half { 16 } else { 32 }, 1, 65536)
                .saturating_mul(sd::config::uint("base_stack_storage", 1, 1, 65536))
        } else { items::group(&stack).map_or(0, |group| group.base_stacks()) };
        if let Some(line) = host.translate("tooltip.storagedrawers.drawers.capacity") {
            event.lines.push((Text::literal(line.replacen("%d", &base.to_string(), 1)), 0xffa0a0a0));
        }
        if items::custom(&stack).contains_key(&"sd_contents".into())
            || stack.data().is_some_and(|data| data.other("minecraft:block_entity_data").is_some())
        {
            event.lines.push((Text::translate("tooltip.storagedrawers.drawers.sealed", Vec::new()), 0xffffff55));
        }
    }
}
