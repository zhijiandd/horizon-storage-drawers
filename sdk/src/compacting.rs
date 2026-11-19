//! Resolve reversible recipes against the current server recipe manager.
use crate::{engine as sd,state};
use mod_api::*;

fn shape(recipe:&Recipe)->Option<usize>{
    let json=&recipe.json;
    match json.get("type")?.as_str()? {
        "minecraft:crafting_shaped"=>{
            let rows=json.get("pattern")?.as_array()?;
            let n=rows.len();
            ([1,2,3].contains(&n)&&rows.iter().all(|r|r.as_str().is_some_and(|r|r.chars().count()==n&&!r.contains(' ')))).then_some(n)
        }
        "minecraft:crafting_shapeless"=>match json.get("ingredients")?.as_array()?.len(){1=>Some(1),4=>Some(2),9=>Some(3),_=>None},
        _=>None,
    }
}
fn same(a:&ItemStack,b:&ItemStack)->bool{a.same_item(b)&&a.same_components(b)}
fn plain(host:&dyn Host,id:&str)->Option<ItemStack>{Some(ItemStack::of(host.item_key(&Identifier::parse(id).ok()?)?))}
fn rules(host:&dyn Host)->Vec<(ItemStack,ItemStack,u64)>{
    let mut out=Vec::new();
    if sd::config::boolean("extra_compacting_rules",true){
        for (u,l,f) in [("clay","clay_ball",4),("snow_block","snowball",4),("glowstone","glowstone_dust",4),
            ("bricks","brick",4),("nether_bricks","nether_brick",4),("nether_wart_block","nether_wart",9),
            ("quartz_block","quartz",4),("melon","melon_slice",9),("bamboo_block","bamboo",9)]{
            if let (Some(u),Some(l))=(plain(host,&format!("minecraft:{u}")),plain(host,&format!("minecraft:{l}"))){out.push((u,l,f));}
        }
    }
    if let Some(entries)=sd::config::features()["compacting_rules"].as_array(){for entry in entries{
        let Some(s)=entry.as_str()else{continue};let p:Vec<_>=s.split(',').map(str::trim).collect();
        if p.len()!=3{continue}let Some(f)=p[2].parse::<u64>().ok().filter(|f|(2..=sd::MAX_COUNT).contains(f))else{continue};
        if let (Some(u),Some(l))=(plain(host,p[0]),plain(host,p[1])){if !same(&u,&l){
            out.retain(|(a,b,_)|!same(a,&u)&&!same(b,&l));out.push((u,l,f));
        }}
    }}out
}
fn matched(host:&dyn Host,level:LevelRef,input:&ItemStack,size:usize,output:&ItemStack)->bool{
    let mut one=input.clone();let _=one.set_count(1);
    level.recipe_for(host,&Identifier::parse("minecraft:crafting").unwrap(),
        &RecipeInput::Crafting{width:size as u32,height:size as u32,slots:vec![Some(one);size*size]})
        .ok().flatten().is_some_and(|r|same(&r.output,output)&&r.output.count()==output.count()&&r.remainders.iter().all(Option::is_none))
}
fn inverse(host:&dyn Host,level:LevelRef,upper:&ItemStack,lower:&ItemStack,factor:u64)->bool{
    let mut result=lower.clone();let Ok(f)=u32::try_from(factor)else{return false};if result.set_count(f).is_err(){return false}
    matched(host,level,upper,1,&result)
}
fn choose(host:&dyn Host,input:&ItemStack,candidates:Vec<(ItemStack,u64)>)->Option<(ItemStack,u64)>{
    let id=input.item().id(host);let ns=id.split_once(':')?.0;
    candidates.iter().find(|(s,_)|s.item().id(host).split_once(':').is_some_and(|(n,_)|n==ns)).cloned()
        .or_else(||candidates.into_iter().next())
}
fn higher(host:&dyn Host,level:LevelRef,input:&ItemStack,rules:&[(ItemStack,ItemStack,u64)])->Option<(ItemStack,u64)>{
    if let Some((u,_,f))=rules.iter().find(|(_,l,_)|same(l,input)){return Some((u.clone(),*f))}
    for size in [3,2]{let mut found=false;let mut candidates=Vec::new();
        for r in host.recipes_of_type("minecraft:crafting").iter(){
            if shape(r)!=Some(size)||!r.ingredients.iter().all(|i|i.test(host,input)){continue}
            let Some(out)=r.results.first()else{continue};
            if !matched(host,level,input,size,out){continue}found=true;
            if out.count()==1&&!same(out,input)&&inverse(host,level,out,input,(size*size)as u64){candidates.push((out.clone(),(size*size)as u64));}
        }
        if found{return choose(host,input,candidates)}
    }None
}
fn lower(host:&dyn Host,level:LevelRef,input:&ItemStack,rules:&[(ItemStack,ItemStack,u64)])->Option<(ItemStack,u64)>{
    if let Some((_,l,f))=rules.iter().find(|(u,_,_)|same(u,input)){return Some((l.clone(),*f))}
    let mut candidates=Vec::new();
    for r in host.recipes_of_type("minecraft:crafting").iter(){
        let Some(size)=shape(r).filter(|n|*n>1)else{continue};
        if !r.results.first().is_some_and(|s|s.count()==1&&same(s,input)){continue}
        let Some(first)=r.ingredients.first()else{continue};
        for id in &first.order{let Some(item)=plain(host,id)else{continue};
            if same(&item,input)||!r.ingredients.iter().all(|i|i.test(host,&item))||!matched(host,level,&item,size,input){continue}
            let factor=(size*size)as u64;if inverse(host,level,input,&item,factor){candidates.push((item,factor));}
        }
    }choose(host,input,candidates)
}
pub fn seed(host:&dyn Host,at:BlockSide,block:&str,g:&mut sd::Group,input:&ItemStack,access:sd::Access)->bool{
    if !sd::config::can_store(&input.item().id(host))||g.check(access).is_err(){return false}
    if !block.contains("compacting")||g.compact.is_some(){return true}
    if !g.can_initialize(access)||g.count(0)>0{return false}
    let mut input=input.clone();let _=input.set_count(1);let rules=rules(host);let enabled=sd::config::boolean("compacting_enabled",true);
    let slots=state::slots(block);let mut cursor=input.clone();let mut seen=vec![input.clone()];let mut upper=Vec::new();
    if enabled{for _ in 1..slots{let Some((next,f))=higher(host,at.level,&cursor,&rules)else{break};if seen.iter().any(|s|same(s,&next)){break}
        seen.push(next.clone());upper.push((next.clone(),f));cursor=next;
    }}
    let mut chain:Vec<_>=upper.into_iter().rev().collect();chain.push((input.clone(),1));cursor=input;
    if enabled{while chain.len()<slots{let Some((next,f))=lower(host,at.level,&cursor,&rules)else{break};if seen.iter().any(|s|same(s,&next)){break}
        seen.push(next.clone());chain.last_mut().unwrap().1=f;chain.push((next.clone(),1));cursor=next;
    }}
    let mut factor=1u64;let mut views=Vec::new();for (s,ratio) in chain.into_iter().rev(){
        let Some(next)=factor.checked_mul(ratio).filter(|f|*f<=sd::MAX_COUNT)else{return false};factor=next;views.push((state::key(host,&s),factor));
    }views.reverse();
    let Ok(mut compact)=sd::Group::compacting(views,g.half)else{return false};
    compact.upgrades=g.upgrades.clone();compact.attributes=g.attributes.clone();compact.frame=g.frame.clone();*g=compact;true
}
pub fn reset(g:&mut sd::Group){
    if g.compact.as_ref().is_some_and(|c|c.units==0)&&!g.attributes.locked&&!g.has(sd::Upgrade::CreativeVending){
        if let Ok(mut empty)=sd::Group::standard(1,g.half){empty.upgrades=g.upgrades.clone();empty.attributes=g.attributes.clone();empty.frame=g.frame.clone();*g=empty;}
    }
}
