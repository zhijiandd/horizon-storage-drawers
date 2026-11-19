// RecipeManager-style discovery. No cached recipe IDs: data-pack reloads and
// other mods' reversible crafting recipes participate in the same lookup.
use super::Game;
use horizon_client::{crafting::{Ingredient,Recipe,Recipes,Template},items::ItemStack};
use horizon_client::menu::sd_rules::engine as sd;

fn shape(recipe:&Recipe)->Option<(Vec<&Ingredient>,&Template,usize)>{
    match recipe{
        Recipe::Shaped{width,height,cells,result} if width==height&&[1,2,3].contains(width)&&cells.iter().all(Option::is_some)=>
            Some((cells.iter().flatten().collect(),result,*width)),
        Recipe::Shapeless{ingredients,result} if [1,4,9].contains(&ingredients.len())=>
            Some((ingredients.iter().collect(),result,match ingredients.len(){9=>3,4=>2,_=>1})),
        _=>None,
    }
}
fn same(a:&ItemStack,b:&ItemStack)->bool{a.same_item_same_components(b)}
fn plain(game:&Game<'_>,id:&str)->Option<ItemStack>{
    game.items().get(id)?;Some(ItemStack::new(id,1))
}
fn registry(game:&Game<'_>)->Vec<(ItemStack,ItemStack,u64)>{
    let mut rules=Vec::new();
    if sd::config::boolean("extra_compacting_rules",true){
        for (upper,lower,factor) in [("clay","clay_ball",4),("snow_block","snowball",4),
            ("glowstone","glowstone_dust",4),("bricks","brick",4),("nether_bricks","nether_brick",4),
            ("nether_wart_block","nether_wart",9),("quartz_block","quartz",4),
            ("melon","melon_slice",9),("bamboo_block","bamboo",9)]{
            if let (Some(u),Some(l))=(plain(game,&format!("minecraft:{upper}")),plain(game,&format!("minecraft:{lower}"))){rules.push((u,l,factor));}
        }
    }
    if let Some(entries)=sd::config::features()["compacting_rules"].as_array(){for entry in entries{
        let Some(rule)=entry.as_str()else{continue;};let parts:Vec<_>=rule.split(',').map(str::trim).collect();
        if parts.len()!=3{continue;}let Some(factor)=parts[2].parse::<u64>().ok().filter(|f|(2..=sd::MAX_COUNT).contains(f))else{continue;};
        if let (Some(u),Some(l))=(plain(game,parts[0]),plain(game,parts[1])){if !same(&u,&l){
            rules.retain(|(old_u,old_l,_)|!same(old_u,&u)&&!same(old_l,&l));rules.push((u,l,factor));
        }}
    }}rules
}
fn matches(recipes:&Recipes,id:&str,item:&ItemStack,size:usize)->bool{
    let mut one=item.clone();one.count=1;let grid=vec![Some(one);size*size];
    recipes.recipe_matches(id,&grid,size)&&recipes.remainders(id,&grid,size).iter().all(Option::is_none)
}
fn inverse(recipes:&Recipes,upper:&ItemStack,lower:&ItemStack,factor:u64)->bool{
    recipes.crafting.iter().any(|(id,r)|{
        let Some((ingredients,result,size))=shape(r)else{return false;};
        let out=result.create();size==1&&ingredients[0].test(upper)&&u64::from(out.count)==factor&&same(&out,lower)&&matches(recipes,id,upper,1)
    })
}
fn choose(input:&ItemStack,candidates:Vec<(ItemStack,u64)>)->Option<(ItemStack,u64)>{
    let namespace=input.item.split_once(':').map(|v|v.0);
    candidates.iter().find(|(s,_)|s.item.split_once(':').map(|v|v.0)==namespace).cloned().or_else(||candidates.into_iter().next())
}
fn higher(game:&Game<'_>,input:&ItemStack,rules:&[(ItemStack,ItemStack,u64)])->Option<(ItemStack,u64)>{
    if let Some((u,_,f))=rules.iter().find(|(_,l,_)|same(l,input)){return Some((u.clone(),*f));}
    let recipes=game.recipes();
    for size in [3,2]{
        let mut forward_found=false;let mut candidates=Vec::new();
        for (id,r) in &recipes.crafting{let Some((ingredients,result,n))=shape(r)else{continue;};
            if n!=size||!ingredients.iter().all(|i|i.test(input))||!matches(recipes,id,input,size){continue;}
            forward_found=true;let out=result.create();let factor=(size*size)as u64;
            if out.count==1&&!same(&out,input)&&inverse(recipes,&out,input,factor){candidates.push((out,factor));}
        }
        // Java only falls back to 2x2 when no 3x3 forward recipe exists.
        if forward_found{return choose(input,candidates);}
    }None
}
fn lower(game:&Game<'_>,input:&ItemStack,rules:&[(ItemStack,ItemStack,u64)])->Option<(ItemStack,u64)>{
    if let Some((_,l,f))=rules.iter().find(|(u,_,_)|same(u,input)){return Some((l.clone(),*f));}
    let recipes=game.recipes();let mut candidates=Vec::new();
    for (id,r) in &recipes.crafting{let Some((ingredients,result,size))=shape(r)else{continue;};
        let out=result.create();if size==1||out.count!=1||!same(&out,input){continue;}
        for candidate in &ingredients[0].order{let Some(item)=plain(game,candidate)else{continue;};
            if same(&item,input)||!ingredients.iter().all(|i|i.test(&item))||!matches(recipes,id,&item,size){continue;}
            let factor=(size*size)as u64;
            if inverse(recipes,input,&item,factor){candidates.push((item,factor));}
        }
    }choose(input,candidates)
}
pub(super) fn views(game:&Game<'_>,input:&ItemStack,slots:usize)->Option<Vec<(ItemStack,u64)>>{
    let mut input=input.clone();input.count=1;let rules=registry(game);
    let enabled=sd::config::boolean("compacting_enabled",true);
    let mut uppers=Vec::new();let mut cursor=input.clone();let mut seen=vec![input.clone()];
    if enabled{for _ in 1..slots{let Some((next,factor))=higher(game,&cursor,&rules)else{break;};
        if seen.iter().any(|s|same(s,&next)){break;}seen.push(next.clone());uppers.push((next.clone(),factor));cursor=next;
    }}
    let mut chain:Vec<(ItemStack,u64)>=uppers.into_iter().rev().collect();chain.push((input.clone(),1));
    cursor=input;
    if enabled{while chain.len()<slots{let Some((next,factor))=lower(game,&cursor,&rules)else{break;};
        if seen.iter().any(|s|same(s,&next)){break;}seen.push(next.clone());
        // Each pair's factor is its ratio to the following item.
        chain.last_mut()?.1=factor;chain.push((next.clone(),1));cursor=next;
    }}
    let mut factor=1u64;
    for (_,ratio) in chain.iter_mut().rev(){factor=factor.checked_mul(*ratio)?;if factor>sd::MAX_COUNT{return None;}*ratio=factor;}
    Some(chain)
}
