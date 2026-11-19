// Shared item payload/recipe implementation, compiled into both game and library.
use horizon_items::{items::ItemStack,item_nbt::{item_from_nbt,item_to_nbt}};
use horizon_world::nbt::{Compound,Tag};
use horizon_client::menu::sd_rules::engine as sd;

pub(crate) fn kind(id:&str)->&str{id.strip_prefix("storagedrawers:").unwrap_or("")}
pub(crate) fn drawer(id:&str)->bool{kind(id).contains("drawers_")}
pub(crate) fn custom(s:&ItemStack)->Compound{match s.data().and_then(|d|d.other("minecraft:custom_data")){Some(Tag::Compound(c))=>c.clone(),_=>Compound::new()}}
pub(crate) fn set_custom(s:&mut ItemStack,c:Compound){s.data_mut().set_other("minecraft:custom_data",(!c.is_empty()).then_some(Tag::Compound(c)));}
pub(crate) fn upgrade(s:&ItemStack)->Option<sd::Upgrade>{sd::Upgrade::from_id(&s.item)}
pub(crate) fn group(s:&ItemStack)->Option<sd::Group>{
    let c=custom(s);
    if let Some(Tag::ByteArray(bytes))=c.get(&"sd_contents".into()){return sd::codec::decode(bytes).ok();}
    if !drawer(&s.item){return None;}
    let n=if kind(&s.item).contains("compacting"){1}else{s.item.rsplit('_').next()?.parse().ok()?};
    sd::Group::standard(n,kind(&s.item).contains("half")).ok()
}
pub(crate) fn positions(c:&Compound,g:&sd::Group)->Vec<usize>{
    let saved=match c.get(&"sd_upgrade_slots".into()){Some(Tag::IntArray(v))=>v.as_slice(),_=>&[]};
    let mut result=Vec::new();for i in 0..g.upgrades.len(){
        let p=saved.get(i).and_then(|&v|usize::try_from(v).ok()).filter(|p|*p<7&&!result.contains(p))
            .unwrap_or_else(||(0..7).find(|p|!result.contains(p)).unwrap());result.push(p);
    }result
}
pub(crate) fn upgrade_items(c:&Compound,g:&sd::Group,positions:&[usize])->Vec<ItemStack>{
    let items=match c.get(&"sd_upgrade_items".into()){Some(Tag::List(v))=>v.as_slice(),_=>&[]};
    g.upgrades.iter().zip(positions).map(|(&u,&pos)|{
        let mut item=items.iter().find(|v|matches!(v.get("slot"),Some(Tag::Int(p))if *p==pos as i32))
            .and_then(|v|item_from_nbt(v.get("item")?)).filter(|s|upgrade(s)==Some(u))
            .unwrap_or_else(||ItemStack::new(&format!("storagedrawers:{}",u.id()),1));item.count=1;canonical_upgrade(item)
    }).collect()
}
pub(crate) fn canonical_upgrade(mut item:ItemStack)->ItemStack{
    if &*item.item=="storagedrawers:balanced_fill_upgrade"{item.item="storagedrawers:balance_fill_upgrade".into();}item
}
pub(crate) fn save_group(s:&mut ItemStack,g:&sd::Group,positions:&[usize],items:&[ItemStack])->Option<()>{
    if positions.len()!=g.upgrades.len()||items.len()!=positions.len(){return None;}
    let mut c=custom(s);c.insert("sd_contents".into(),Tag::ByteArray(sd::codec::encode(g).ok()?));
    c.insert("sd_upgrade_slots".into(),Tag::IntArray(positions.iter().map(|&p|p as i32).collect()));
    c.insert("sd_upgrade_items".into(),Tag::List(positions.iter().zip(items).map(|(&p,s)|Tag::compound([
        ("slot",Tag::Int(p as i32)),("item",item_to_nbt(s))])).collect()));
    set_custom(s,c);
    s.data_mut().set_other("minecraft:max_stack_size",g.has_inventory_data().then_some(Tag::Int(1)));Some(())
}
pub(crate) fn stock(item:&sd::ItemKey,count:u64)->Option<ItemStack>{
    let mut s=if item.components.is_empty(){ItemStack::new(&item.id,1)}else{
        let (tag,n)=horizon_world::nbt::parse_snbt(&item.components).ok()?;if n!=item.components.len(){return None;}item_from_nbt(&tag)?
    };s.count=u32::try_from(count).ok()?;Some(s)
}
pub(crate) fn binding(s:&ItemStack)->Option<([i32;3],String)>{
    if !matches!(upgrade(s), Some(sd::Upgrade::Remote | sd::Upgrade::RemoteGroup)) { return None; }
    let c=custom(s);let Tag::IntArray(v)=c.get(&"sd_controller".into())?else{return None;};
    let xyz:[i32;3]=v.as_slice().try_into().ok()?;
    let dim=match c.get(&"sd_dimension".into()){Some(Tag::String(d))=>d.to_string_lossy(),_=>"minecraft:overworld".into()};Some((xyz,dim))
}
pub(crate) fn bound(s:&ItemStack,pos:[i32;3],dim:&str)->ItemStack{
    let mut out=s.clone();let group=upgrade(s)==Some(sd::Upgrade::RemoteGroup);
    out.item=format!("storagedrawers:remote_{}upgrade_bound",if group{"group_"}else{""}).into();
    let mut c=custom(&out);c.insert("sd_controller".into(),Tag::IntArray(pos.to_vec()));c.insert("sd_dimension".into(),Tag::String(dim.into()));set_custom(&mut out,c);out
}
pub(crate) fn unbound(s:&ItemStack)->ItemStack{
    let mut out=s.clone();if let Some(u)=upgrade(s){out.item=format!("storagedrawers:{}",u.id()).into();}
    let mut c=custom(&out);c.remove(&"sd_controller".into());c.remove(&"sd_dimension".into());set_custom(&mut out,c);out
}
// Detached payload stores stack capacity independently of the held prototype.
pub(crate) fn detached(s:&ItemStack)->Option<(Option<ItemStack>,u64,bool)>{
    if !matches!(kind(&s.item),"detached_drawer"|"detached_drawer_full"){return None;}
    let c=custom(s);let Some(Tag::Compound(data))=c.get(&"sd_detached".into())else{return Some((None,sd::config::detached_stacks(),false));};
    let item=data.get(&"item".into()).and_then(item_from_nbt).filter(|s|s.count>0);
    let capacity=match data.get(&"capacity".into()){Some(Tag::Long(v))if *v>0=>*v as u64,_=>sd::config::detached_stacks()};
    let heavy=matches!(data.get(&"heavy".into()),Some(Tag::Byte(1)));Some((item,capacity,heavy))
}
pub(crate) fn make_detached(item:Option<ItemStack>,capacity:u64,heavy:bool)->ItemStack{
    let loaded=item.as_ref().is_some_and(|s|s.count>0);let mut out=ItemStack::new(if loaded{"storagedrawers:detached_drawer_full"}else{"storagedrawers:detached_drawer"},1);
    let mut data=Compound::new();if let Some(s)=item{data.insert("item".into(),item_to_nbt(&s));}
    data.insert("capacity".into(),Tag::Long(capacity.min(i64::MAX as u64)as i64));data.insert("heavy".into(),Tag::Byte(i8::from(heavy)));
    let mut c=custom(&out);c.insert("sd_detached".into(),Tag::Compound(data));set_custom(&mut out,c);
    if loaded{out.data_mut().set_other("minecraft:max_stack_size",Some(Tag::Int(1)));}out
}
pub(crate) fn framed_id(id:&str)->Option<String>{
    let k=kind(id);let name=if k.starts_with("framed_"){k.to_owned()}
    else if k.contains("compacting_"){format!("framed_{k}")}
    else if let Some(i)=k.find("_full_drawers_"){format!("framed{}",&k[i..])}
    else if let Some(i)=k.find("_half_drawers_"){format!("framed{}",&k[i..])}
    else if k.ends_with("_trim"){"framed_trim".into()}
    else if matches!(k,"controller"|"controller_io"){format!("framed_{k}")}
    else{return None;};Some(format!("storagedrawers:{name}"))
}
pub(crate) fn frame(s:&ItemStack)->Option<Tag>{custom(s).get(&"sd_frame".into()).cloned()}
pub(crate) fn make_frame(input:&ItemStack,side:&ItemStack,trim:Option<&ItemStack>,front:Option<&ItemStack>)->Option<ItemStack>{
    let id=framed_id(&input.item)?;let mut result=input.clone();result.item=id.into();
    let mut data=Compound::new();data.insert("base".into(),item_to_nbt(&ItemStack::new(&input.item,1)));
    for (key,item) in [("side",Some(side)),("trim",trim),("front",front)]{if let Some(item)=item{let mut one=item.clone();one.count=1;data.insert(key.into(),item_to_nbt(&one));}}
    let mut c=custom(&result);c.insert("sd_frame".into(),Tag::Compound(data));set_custom(&mut result,c);Some(result)
}
pub(crate) fn unpack_frame(input:&ItemStack)->Option<(ItemStack,[Option<ItemStack>;3])>{
    let data=frame(input)?;let base=item_from_nbt(data.get("base")?)?;
    let mut result=input.clone();result.item=base.item;let mut c=custom(&result);c.remove(&"sd_frame".into());set_custom(&mut result,c);
    let materials=["side","trim","front"].map(|k|data.get(k).and_then(item_from_nbt).map(|mut s|{s.count=input.count;s}));Some((result,materials))
}
pub(crate) fn ring_key(id: &str) -> bool {
    matches!(id.strip_prefix("storagedrawers:"), Some(
        "drawer_key" | "quantify_key" | "shroud_key" | "suspend_key" | "personal_key"
        | "personal_key_unlock" | "personal_key_ftb" | "personal_key_cofh" | "drawer_puller"
        | "priority_key" | "priority_key_p1" | "priority_key_p2" | "priority_key_n1" | "priority_key_n2"
    ))
}
pub(crate) fn ring_variant(keys:&[ItemStack],active:usize)->String{
    keys.get(active).map_or_else(||"storagedrawers:keyring".into(),|s|{
        let k=kind(&s.item);let suffix=match k{"drawer_key"=>"drawer","quantify_key"=>"quantify","shroud_key"=>"shroud","suspend_key"=>"suspend",
            "personal_key"=>"personal","personal_key_unlock"=>"personal_unlock","personal_key_ftb"=>"personal_ftb","personal_key_cofh"=>"personal_cofh","drawer_puller"=>"puller",v=>v};
        let suffix=suffix.strip_prefix("priority_key").map_or_else(||suffix.to_owned(),|s|format!("priority{s}"));
        format!("storagedrawers:keyring_{suffix}")
    })
}
pub(crate) fn ring_items(s:&ItemStack)->Vec<ItemStack>{
    let c=custom(s);match c.get(&"sd_keys".into()){Some(Tag::List(v))=>v.iter().filter_map(item_from_nbt).filter(|s|ring_key(&s.item)).collect(),_=>Vec::new()}
}
pub(crate) fn save_ring(s:&mut ItemStack,keys:&[ItemStack],active:usize){
    s.item=ring_variant(keys,active).into();let mut c=custom(s);c.insert("sd_keys".into(),Tag::List(keys.iter().map(item_to_nbt).collect()));
    c.insert("sd_active_key".into(),Tag::Int(active as i32));set_custom(s,c);
}
pub(crate) fn active_key(s:&ItemStack)->Option<ItemStack>{
    let keys=ring_items(s);let c=custom(s);let active=match c.get(&"sd_active_key".into()){Some(Tag::Int(v))=>usize::try_from(*v).unwrap_or(0),_=>0};keys.get(active).cloned()
}
pub(crate) fn craft(grid:&[Option<ItemStack>],size:usize)->Option<(&'static str,ItemStack)>{
    if !matches!(size,2|3)||grid.len()<size*size{return None;}
    let items:Vec<_>=grid.iter().take(size*size).filter_map(Option::as_ref).filter(|s|s.count>0).collect();
    if items.is_empty(){return None;}
    // Only actual occupied slots are ingredients, once each, as Java CraftingInput.
    if items.len()>=2&&items.iter().filter(|s|drawer(&s.item)).count()==1&&items.iter().all(|s|drawer(&s.item)||upgrade(s).is_some()){
        let input=*items.iter().find(|s|drawer(&s.item))?;let mut out=input.clone();out.count=1;let mut g=group(input)?;
        let c=custom(input);let mut slots=positions(&c,&g);let mut stacks=upgrade_items(&c,&g,&slots);
        for s in &items{if drawer(&s.item){continue;}let u=upgrade(s)?;if u==sd::Upgrade::OneStack{return None;}
            let free=(0..7).find(|p|!slots.contains(p))?;g.add_upgrade(u,sd::Access::Administrator).ok()?;
            slots.push(free);let mut installed=(*s).clone();installed.count=1;stacks.push(installed);
        }save_group(&mut out,&g,&slots,&stacks)?;return Some(("storagedrawers:add_upgrade",out));
    }
    if items.iter().filter(|s|detached(s).is_some()).count()==1&&items.iter().all(|s|detached(s).is_some()||upgrade(s).is_some_and(|u|u.multiplier().is_some())){
        let input=*items.iter().find(|s|detached(s).is_some())?;let (stock,capacity,heavy)=detached(input)?;
        let added: u64 = items.iter().filter_map(|s| upgrade(s).and_then(sd::Upgrade::multiplier)).sum();
        let base = sd::Group::standard(1, false).ok()?.base_stacks();
        let upgraded = capacity.saturating_add(base.saturating_mul(added)).min(sd::MAX_COUNT);
        let payload = make_detached(if added == 0 { None } else { stock }, upgraded, heavy && added > 0);
        let mut out = if added == 0 {
            payload
        } else {
            let mut out = input.clone();
            let mut components = custom(&out);
            components.insert("sd_detached".into(), custom(&payload).remove(&"sd_detached".into())?);
            set_custom(&mut out, components);
            out
        };
        // Each craft consumes one drawer and one item from each upgrade cell.
        // Copy components, never the count of the input ingredient stack.
        out.count=1;
        return Some(("storagedrawers:add_detached_upgrade",out));
    }
    if size==3&&items.len()==5&&grid[0].is_none()&&grid[2].is_none()&&grid[6].is_none()&&grid[8].is_none()
        &&[1,3,5,7].into_iter().all(|i|grid[i].as_ref().is_some_and(|s|&*s.item=="minecraft:iron_nugget")){
        let key=grid[4].as_ref()?;if ring_key(&key.item){let mut key=key.clone();key.count=1;let mut out=ItemStack::new("storagedrawers:keyring",1);save_ring(&mut out,&[key],0);return Some(("storagedrawers:keyring",out));}
    }
    if items.len()==1&&&*items[0].item=="storagedrawers:personal_key"{return Some(("storagedrawers:personal_key_cycle",ItemStack::new("storagedrawers:personal_key",1)));}
    if size==3&&items.len()==3{for row in 0..3{
        let i=row*3;if grid[i].as_ref().is_some_and(|s|&*s.item=="minecraft:ender_pearl")&&grid[i+2].as_ref().is_some_and(|s|&*s.item=="minecraft:ender_pearl"){
            let source=grid[i+1].as_ref()?;if &*source.item=="storagedrawers:remote_upgrade_bound"&&binding(source).is_some(){
                let mut out=source.clone();out.item="storagedrawers:remote_group_upgrade_bound".into();out.count=1;return Some(("storagedrawers:remote_group_upgrade_bound",out));
            }
        }
    }}None
}

// Java overrideOtherStackedOnMe: a secondary click on the ring itself.
pub(crate) fn ring_click(ring:&mut ItemStack,cursor:&mut Option<ItemStack>)->bool{
    if !kind(&ring.item).starts_with("keyring")||ring.count!=1{return false;}
    let mut keys=ring_items(ring);
    let active=active_key(ring);
    if let Some(held)=cursor.as_mut(){
        if ring_key(&held.item)&&!keys.iter().any(|s|s.same_item_same_components(held)){
            let mut one=held.clone();one.count=1;keys.insert(0,one);held.count-=1;
            if held.count==0{*cursor=None;}
        }
        let index=active.as_ref().and_then(|a|keys.iter().position(|s|s.same_item_same_components(a))).unwrap_or(0);
        save_ring(ring,&keys,index);
    }else{
        if !keys.is_empty(){*cursor=Some(keys.remove(0));}
        let next=active.as_ref().and_then(|a|keys.iter().position(|s|s.item==a.item)).map_or(0,|i|(i+1)%keys.len());
        save_ring(ring,&keys,next);
    }true
}
