//! All pipes share the saved core inventory and the host transaction journal.
use crate::{engine as sd,state,network,compacting};
use mod_api::*;
pub struct DrawerItems(pub BlockSide);
struct View { net:sd::Network,root:Option<sd::Position>,entries:Vec<(sd::Position,usize)> }
impl DrawerItems {
    fn view(&self,host:&dyn Host)->Option<View>{
        let block=state::block(host,self.0)?;
        let at=sd::Position::new(self.0.level.dimension_id(),self.0.pos);
        let mut net=if state::is_drawer(&block){sd::Network::default()}else{network::snapshot(host,self.0.level)};
        if let Some((_,_,g))=state::read(host,self.0){net.nodes.insert(at.clone(),sd::Node::Drawers(g));}
        let root=if state::is_controller(&block)||state::is_port(&block){network::root(&net,&at)}else{None};
        let connected=if state::is_drawer(&block){std::collections::BTreeSet::from([at])}
            else{root.as_ref().map(|r|net.connected(r,network::range())).unwrap_or_default()};
        let entries=connected.into_iter().flat_map(|p|match net.nodes.get(&p){
            Some(sd::Node::Drawers(g))=>(0..g.drawers.len()).map(|i|(p.clone(),i)).collect::<Vec<_>>(),_=>Vec::new(),
        }).collect();Some(View{net,root,entries})
    }
    fn put(&self,host:&dyn Host,view:&mut View,index:usize,resource:&ItemResource,amount:u32)->u32{
        let input=resource.as_stack();
        if index==0{if let Some(root)=view.root.clone(){return network::insert(host,self.0.level,&mut view.net,&root,input,amount.into(),sd::Access::Automation).accepted()as u32}}
        let (p,slot)=if index==0{(sd::Position::new(self.0.level.dimension_id(),self.0.pos),None)}
            else{let Some((p,s))=view.entries.get(index-1)else{return 0};(p.clone(),Some(*s))};
        let Some(sd::Node::Drawers(g))=view.net.nodes.get_mut(&p)else{return 0};
        let at=BlockSide{pos:p.xyz,..self.0};let Some(block)=state::block(host,at)else{return 0};
        let was_unseeded = g.compact.is_none();
        if !compacting::seed(host,at,&block,g,input,sd::Access::Automation){return 0}
        let targets:Vec<_> = if let Some(slot) = slot {
            if was_unseeded && let Some(compact) = &g.compact {
                let key = state::group_key(host,g,input);
                let Some(tier) = compact.views.iter().position(|(item,_)| item.same_item(&key)) else { return 0; };
                vec![tier]
            } else {
                vec![slot]
            }
        } else {
            (0..g.drawers.len()).collect()
        };
        state::insert(host,g,&targets,input,amount.into(),sd::Access::Automation).accepted()as u32
    }
}
impl ResourceHandler<ItemResource> for DrawerItems {
    fn size(&self,host:&mut dyn ServerHost)->usize{self.view(host).map_or(0,|v|v.entries.len()+1)}
    fn resource(&self,host:&mut dyn ServerHost,index:usize)->Option<ItemResource>{
        let v=self.view(host)?;let (p,slot)=v.entries.get(index.checked_sub(1)?)?;
        let sd::Node::Drawers(g)=v.net.nodes.get(p)?else{return None};
        if g.check(sd::Access::Automation).is_err(){return None}
        Some(ItemResource::of(&state::stack(host,g.item(*slot)?,1)?))
    }
    fn amount(&self,host:&mut dyn ServerHost,index:usize)->u32{
        let Some(v)=self.view(host)else{return 0};let Some(i)=index.checked_sub(1)else{return 0};
        let Some((p,slot))=v.entries.get(i)else{return 0};let Some(sd::Node::Drawers(g))=v.net.nodes.get(p)else{return 0};
        if g.check(sd::Access::Automation).is_err(){return 0}
        if g.has(sd::Upgrade::CreativeVending)&&g.item(*slot).is_some(){u32::MAX}else{g.count(*slot).min(u64::from(u32::MAX))as u32}
    }
    fn capacity(&self,host:&mut dyn ServerHost,index:usize,resource:&ItemResource)->u32{
        let Some(v)=self.view(host)else{return 0};
        if index==0{return if v.root.is_some()||!v.entries.is_empty(){u32::MAX}else{0}}
        let Some((pos,slot))=v.entries.get(index-1)else{return 0};
        let Some(sd::Node::Drawers(group))=v.net.nodes.get(pos)else{return 0};
        if group.drawers[*slot].detached{return 0}
        group.capacity(*slot,&state::key(host,resource.as_stack())).min(u64::from(u32::MAX)) as u32
    }
    fn is_valid(&self,host:&mut dyn ServerHost,index:usize,resource:&ItemResource)->bool{
        if !sd::config::can_store(&resource.as_stack().item().id(host)){return false}
        let Some(v)=self.view(host)else{return false};
        if index==0{return v.root.is_some()||!v.entries.is_empty()}
        let Some((pos,slot))=v.entries.get(index-1)else{return false};
        let Some(sd::Node::Drawers(group))=v.net.nodes.get(pos)else{return false};
        !group.drawers[*slot].detached && group.check(sd::Access::Automation).is_ok()
            && match group.item(*slot){
                Some(target)=>target.same_item(&state::conversion(host,group,*slot,resource.as_stack())),
                None=>!group.attributes.locked,
            }
    }
    fn insert(&self,host:&mut dyn ServerHost,tx:&Transaction,index:usize,resource:&ItemResource,amount:u32)->u32{
        if host.transaction_depth(tx).is_none(){return 0}
        let Some(mut v)=self.view(host)else{return 0};let put=self.put(host,&mut v,index,resource,amount);
        if put>0&&network::commit(host,self.0.level,&v.net).is_ok(){put}else{0}
    }
    fn extract(&self,host:&mut dyn ServerHost,tx:&Transaction,index:usize,resource:&ItemResource,amount:u32)->u32{
        if host.transaction_depth(tx).is_none(){return 0}
        let Some(mut v)=self.view(host)else{return 0};let Some(i)=index.checked_sub(1)else{return 0};
        let Some((p,slot))=v.entries.get(i).cloned()else{return 0};
        let Some(sd::Node::Drawers(g))=v.net.nodes.get_mut(&p)else{return 0};
        let key=state::group_key(host,g,resource.as_stack());if !g.item(slot).is_some_and(|k|k.same_item(&key)){return 0}
        let Ok(Some(out))=g.extract(slot,amount.into(),sd::Access::Automation,false)else{return 0};
        compacting::reset(g);
        if network::commit(host,self.0.level,&v.net).is_ok(){out.count as u32}else{0}
    }
    fn insert_any(&self,host:&mut dyn ServerHost,tx:&Transaction,resource:&ItemResource,amount:u32)->u32{
        self.insert(host,tx,0,resource,amount)
    }
}
