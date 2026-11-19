use crate::{engine as sd,state,items,compacting};
use mod_api::*;
pub fn range()->u32{sd::config::features()["controller_range"].as_u64().unwrap_or(50).clamp(1,512)as u32}
pub fn snapshot(host:&dyn Host,level:LevelRef)->sd::Network{
    let mut net=sd::Network::default();net.range=range();
    for (dim,pos,entity) in state::addresses(){
        if dim!=level.dimension_id(){continue}
        let Some(nbt)=entity.data(host)else{continue};
        if entity.level(host)!=Some(level){continue}
        let id=match nbt.get(&"sd_block".into()){Some(Tag::String(id))=>id.to_string_lossy(),_=>continue};
        let at=sd::Position::new(&dim,pos);
        let node=if state::is_drawer(&id){let Some(g)=state::group(&nbt)else{continue};
            let (_,upgrades)=state::upgrade_state(&nbt,&g);
            for upgrade in upgrades{if let Some((target,d))=items::binding(&upgrade){
                let group=items::upgrade(&upgrade)==Some(sd::Upgrade::RemoteGroup);
                let controller=sd::Position::new(&d,target);
                if at.in_range(&controller, sd::remote_range(net.range, group)){
                    net.remotes.push(sd::RemoteBinding{drawer:at.clone(),controller,group});
                }
            }}sd::Node::Drawers(g)
        }else if state::is_controller(&id){sd::Node::Controller}else if state::is_port(&id){sd::Node::Port}
        else if id.ends_with("_trim"){sd::Node::Trim}else{continue};
        net.nodes.insert(at,node);
    }net
}
pub fn root(net:&sd::Network,at:&sd::Position)->Option<sd::Position>{
    if matches!(net.nodes.get(at),Some(sd::Node::Controller)){return Some(at.clone())}
    net.nodes.iter().filter(|(_,n)|matches!(n,sd::Node::Controller)).find_map(|(p,_)|
        net.connected(p,range()).contains(at).then_some(p.clone()))
}
pub fn commit(host:&mut dyn ServerHost,level:LevelRef,net:&sd::Network)->Result<(),()>{
    host.transaction(|host,_|{
        for (p,node) in &net.nodes {if let sd::Node::Drawers(g)=node{
            let at=BlockSide{level,pos:p.xyz,side:None};
            let Some((entity,nbt,old))=state::read(host,at)else{return Err(())};
            if old != *g {
                if old.attributes.locked && !g.attributes.locked {
                    let mut unlocked = g.clone();
                    compacting::reset(&mut unlocked);
                    if !state::write(host, entity, nbt, &unlocked) { return Err(()); }
                } else if !state::write(host, entity, nbt, g) {
                    return Err(());
                }
            }
        }}Ok(())
    })
}
pub fn insert(host:&dyn Host,level:LevelRef,net:&mut sd::Network,origin:&sd::Position,input:&ItemStack,amount:u64,access:sd::Access)->sd::Inserted{
    let input_id = input.item().id(host);
    let mut routes=Vec::new();
    for p in net.connected(origin,range()){if let Some(sd::Node::Drawers(g))=net.nodes.get(&p){
        if g.attributes.suspended || g.check(access).is_err(){continue}for slot in 0..g.drawers.len(){
            if g.drawers[slot].detached{continue}
            if let Some(target)=g.item(slot){if target.same_item(&state::conversion(host,g,slot,input)){
                if target.id == input_id {
                    routes.push((0,g.attributes.priority,p.clone(),slot));
                    routes.push((1,g.attributes.priority,p.clone(),slot));
                } else {
                    routes.push((2,g.attributes.priority,p.clone(),slot));
                }
            }}else if !matches!(access,sd::Access::Player(_))&&!g.attributes.locked{routes.push((2,g.attributes.priority,p.clone(),slot));}
        }
    }}
    routes.sort_by(|a,b|a.0.cmp(&b.0).then(b.1.cmp(&a.1)).then(a.2.cmp(&b.2)).then(a.3.cmp(&b.3)));
    let mut remaining=amount;let mut result=sd::Inserted::default();
    for (phase,_,p,slot) in routes{if remaining==0{break}
        let Some(sd::Node::Drawers(g))=net.nodes.get_mut(&p)else{continue};
        let at=BlockSide{level,pos:p.xyz,side:None};let Some(block)=state::block(host,at)else{continue};
        let mut seeded = (g.compact.is_none() && block.contains("compacting")).then(|| g.clone());
        let trial = seeded.as_mut().unwrap_or(&mut *g);
        if !compacting::seed(host,at,&block,trial,input,access){continue}
        let key=state::group_key(host,trial,input);
        let target=trial.compact.as_ref().and_then(|c|c.views.iter().position(|(k,_)|k.same_item(&key))).unwrap_or(slot);
        let offer=if phase==0{trial.remaining_capacity(target,&key).min(remaining)}else{remaining};
        let put=state::insert(host,trial,&[target],input,offer,access);
        if put.accepted() > 0 && let Some(seeded) = seeded { *g = seeded; }
        remaining-=put.accepted();result.stored+=put.stored;result.voided+=put.voided;
    }result
}
