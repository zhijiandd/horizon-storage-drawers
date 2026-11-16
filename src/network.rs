use std::collections::{BTreeMap,BTreeSet,VecDeque};
use crate::storage::*;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Position { pub dimension:String, pub xyz:[i32;3] }
impl Position {
    pub fn new(dimension:&str,xyz:[i32;3])->Self{Self{dimension:dimension.into(),xyz}}
    pub fn neighbors(&self)->impl Iterator<Item=Self>+'_ {
        [[1,0,0],[-1,0,0],[0,1,0],[0,-1,0],[0,0,1],[0,0,-1]].into_iter().filter_map(|d|{
            Some(Self{dimension:self.dimension.clone(),xyz:[self.xyz[0].checked_add(d[0])?,
                self.xyz[1].checked_add(d[1])?,self.xyz[2].checked_add(d[2])?]})
        })
    }
    pub fn in_range(&self,other:&Self,range:u32)->bool{
        self.dimension==other.dimension && (0..3).all(|i|self.xyz[i].abs_diff(other.xyz[i])<=range)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Node { Drawers(Group), Trim, Controller, Port }
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteBinding { pub drawer:Position,pub controller:Position,pub group:bool }
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Network { pub nodes:BTreeMap<Position,Node>,pub remotes:Vec<RemoteBinding>,pub range:u32 }
impl Default for Network{fn default()->Self{Self{nodes:BTreeMap::new(),remotes:Vec::new(),range:50}}}

impl Network {
    /// Traverse only known (loaded) nodes. A second controller terminates traversal.
    pub fn connected(&self,origin:&Position,range:u32)->BTreeSet<Position>{
        if !matches!(self.nodes.get(origin),Some(Node::Controller)){return BTreeSet::new();}
        let mut visited=BTreeSet::new();let mut q=VecDeque::from([origin.clone()]);
        for r in &self.remotes{
            if &r.controller==origin&&origin.in_range(&r.drawer,range){
                if let Some(Node::Drawers(g))=self.nodes.get(&r.drawer){
                    if g.has(Upgrade::Remote)||g.has(Upgrade::RemoteGroup){
                        if r.group{q.push_back(r.drawer.clone());}else{visited.insert(r.drawer.clone());}
                    }
                }
            }
        }
        while let Some(at)=q.pop_front(){
            if visited.contains(&at)||!origin.in_range(&at,range){continue;}
            let Some(node)=self.nodes.get(&at)else{continue;};
            if at!=*origin&&matches!(node,Node::Controller){continue;}
            if self.remotes.iter().any(|r|r.drawer==at&&r.controller!=*origin){continue;}
            visited.insert(at.clone());
            if matches!(node,Node::Drawers(g)if g.has(Upgrade::Remote)&&!g.has(Upgrade::RemoteGroup)){continue;}
            for n in at.neighbors(){if !visited.contains(&n){q.push_back(n);}}
        }visited
    }
    /// Equal counts until a drawer fills; redistribute the remainder into
    /// drawers with space. Participants must opt into balanced fill.
    pub fn rebalance(&mut self,origin:&Position,access:Access){
        let mut connected=self.connected(origin,self.range);
        // A drawer can be soft-bound to multiple controllers. Union their
        // overlapping networks once, rather than balancing them in sequence.
        loop{let before=connected.len();for (p,n) in &self.nodes{if matches!(n,Node::Controller){
            let next=self.connected(p,self.range);if !next.is_disjoint(&connected){connected.extend(next);}
        }}if connected.len()==before{break;}}
        self.rebalance_positions(connected,access);
    }
    pub fn rebalance_all(&mut self,access:Access){
        let mut remaining:BTreeSet<_>=self.nodes.iter().filter_map(|(p,n)|matches!(n,Node::Controller).then_some(p.clone())).collect();
        while let Some(p)=remaining.pop_first(){
            let mut joined=self.connected(&p,self.range);
            loop{let before=joined.len();let include:Vec<_>=remaining.iter().filter_map(|q|{
                let next=self.connected(q,self.range);(!joined.is_disjoint(&next)).then_some((q.clone(),next))
            }).collect();for (q,next) in include{remaining.remove(&q);joined.extend(next);}
            if before==joined.len(){break;}}
            self.rebalance_positions(joined,access);
        }
    }
    fn rebalance_positions(&mut self,connected:BTreeSet<Position>,access:Access){
        let mut sets:BTreeMap<ItemKey,Vec<(Position,usize)>>=BTreeMap::new();
        for p in connected{if let Some(Node::Drawers(g))=self.nodes.get(&p){
            if !g.has(Upgrade::Balance)||g.has(Upgrade::CreativeVending)||g.compact.is_some()||g.attributes.suspended||g.check(access).is_err(){continue;}
            for s in 0..g.drawers.len(){if g.drawers[s].detached{continue;}if let Some(i)=g.item(s){sets.entry(i.clone()).or_default().push((p.clone(),s));}}
        }}
        for (item,slots) in sets{
            let total:u64=slots.iter().map(|(p,s)|match &self.nodes[p]{Node::Drawers(g)=>g.count(*s),_=>0}).sum();
            let caps:Vec<u64>=slots.iter().map(|(p,s)|match &self.nodes[p]{Node::Drawers(g)=>g.capacity(*s,&item),_=>0}).collect();
            let mut counts=vec![0;slots.len()];let mut remaining=total;
            while remaining>0{
                let open:Vec<_>=(0..caps.len()).filter(|&i|counts[i]<caps[i]).collect();if open.is_empty(){break;}
                let each=remaining/open.len()as u64;let extra=remaining%open.len()as u64;
                for (n,i) in open.into_iter().enumerate(){let put=(each+u64::from((n as u64)<extra)).min(caps[i]-counts[i]);counts[i]+=put;remaining-=put;}
            }
            if remaining>0{continue;}
            for ((p,s),count) in slots.into_iter().zip(counts){if let Some(Node::Drawers(g))=self.nodes.get_mut(&p){g.drawers[s].count=count;}}
        }
    }
    pub fn routes(&self,origin:&Position,item:&ItemKey,access:Access)->Vec<(Position,usize)>{
        let mut out=Vec::new();
        for pos in self.connected(origin,self.range){
            let Some(Node::Drawers(g))=self.nodes.get(&pos)else{continue;};
            if g.check(access).is_err(){continue;}
            for slot in 0..g.drawers.len(){
                // Controllers never assign unbound slots; only matching or locked prototypes.
                if g.item(slot).is_some_and(|k|k.same_item(item)){
                    out.push((g.attributes.priority,pos.clone(),slot));
                }
            }
        }
        out.sort_by(|a,b|b.0.cmp(&a.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
        out.into_iter().map(|(_,p,s)|(p,s)).collect()
    }
    pub fn insert(&mut self,origin:&Position,stack:&Stack,access:Access,simulate:bool)->Inserted{
        let mut rest=stack.count;let mut total=Inserted::default();
        for (pos,slot) in self.routes(origin,&stack.item,access){
            if rest==0{break;}
            if let Some(Node::Drawers(g))=self.nodes.get_mut(&pos){
                if let Ok(put)=g.insert(slot,&Stack{item:stack.item.clone(),count:rest},access,simulate){
                    rest-=put.accepted();total.stored+=put.stored;total.voided+=put.voided;
                }
            }
        }total
    }
    pub fn extract(&mut self,origin:&Position,item:&ItemKey,count:u64,access:Access,simulate:bool)->Option<Stack>{
        let mut taken=0;
        for (pos,slot) in self.routes(origin,item,access){
            if taken==count{break;}
            if let Some(Node::Drawers(g))=self.nodes.get_mut(&pos){
                if let Ok(Some(out))=g.extract(slot,count-taken,access,simulate){taken+=out.count;}
            }
        }
        (taken>0).then(||Stack{item:item.clone(),count:taken})
    }
    /// Work on a copy first, making a move all-or-nothing even if a target refuses it.
    pub fn transfer(&mut self,from:&Position,slot:usize,to:&Position,count:u64)->u64{
        if from==to{return 0;}
        let mut transaction=self.clone();
        let Some(Node::Drawers(src))=transaction.nodes.get_mut(from)else{return 0;};
        let Ok(Some(stack))=src.extract(slot,count,Access::Automation,true)else{return 0;};
        let put=transaction.insert(to,&stack,Access::Automation,false);
        if put.accepted()==0{return 0;}
        let Some(Node::Drawers(src))=transaction.nodes.get_mut(from)else{return 0;};
        if src.extract(slot,put.accepted(),Access::Automation,false).is_err(){return 0;}
        *self=transaction;put.accepted()
    }
    pub fn use_key(&mut self,controller:&Position,key:Key,actor:u128)->usize{
        // Java's group controller reads one accessible drawer, then applies
        // the same new state to the entire network, including mixed states.
        let state=|g:&Group|match key{Key::Lock=>Some(g.attributes.locked),Key::Quantify=>Some(g.attributes.quantified),
            Key::Shroud=>Some(g.attributes.shrouded),Key::Suspend=>Some(g.attributes.suspended),_=>None};
        let connected=self.connected(controller,self.range);
        let target=connected.iter().find_map(|p|match self.nodes.get(p){
            Some(Node::Drawers(g))if g.check(Access::Player(actor)).is_ok()=>state(g).map(|value|!value),_=>None});
        let mut changed=0;
        for pos in connected{
            if let Some(Node::Drawers(g))=self.nodes.get_mut(&pos){
                if target.is_some()&&state(g)==target{continue;}
                if g.use_key(key,actor).is_ok(){changed+=1;}
            }
        }changed
    }
}
