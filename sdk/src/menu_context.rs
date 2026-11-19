use crate::{engine as sd,compat::Stack,state,compacting};
use mod_api::*;
pub struct Context<'a>{pub host:&'a dyn Host,pub at:BlockSide}
impl Context<'_>{
    pub fn stack_max_size(&self,s:&Stack)->u32{s.to_api(self.host).map_or(64,|s|state::max_stack(self.host,&s))}
}
pub fn inventory_slot(own:i32,slot:i32)->Option<usize>{
    let i=slot-own;if (0..27).contains(&i){Some(i as usize+9)}else if (27..36).contains(&i){Some((i-27)as usize)}else{None}
}
pub fn upgrade(id:&str)->Option<sd::Upgrade>{sd::Upgrade::from_id(id)}
pub use crate::state::slots;
pub fn stack(ctx:&Context,key:&sd::ItemKey,count:u64)->Option<Stack>{
    Stack::from_api(ctx.host,&state::stack(ctx.host,key,count.try_into().ok()?)?)
}
pub fn group_key(ctx:&Context,g:&sd::Group,input:&Stack)->sd::ItemKey{
    input.to_api(ctx.host).map_or_else(||sd::ItemKey::plain(&input.item,64),|s|state::group_key(ctx.host,g,&s))
}
pub fn seed(ctx:&Context,block:&str,g:&mut sd::Group,input:&Stack,access:sd::Access)->bool{
    input.to_api(ctx.host).is_some_and(|s|compacting::seed(ctx.host,ctx.at,block,g,&s,access))
}
pub fn insert_group(ctx:&Context,g:&mut sd::Group,slot:usize,input:&sd::Stack,access:sd::Access,_:bool)->Result<sd::Inserted,sd::Error>{
    let Some(s)=state::stack(ctx.host,&input.item,1)else{return Err(sd::Error::Incompatible)};
    Ok(state::insert(ctx.host,g,&[slot],&s,input.count,access))
}
