use crate::engine as sd;
use mod_api::*;
use std::{cell::RefCell, collections::BTreeMap};

include!("blocks.rs");
pub const SAVE: &str = "sd_state";
// Addresses are only live handles, never a second copy of inventory. The host
// checks the world/session/generation on every read, including comparator reads.
thread_local! {
    static LOADED: RefCell<BTreeMap<(String,[i32;3]),Vec<BlockEntityRef>>> = const { RefCell::new(BTreeMap::new()) };
}
pub fn remember(host: &dyn Host, entity: BlockEntityRef) {
    if let (Some(level),Some(pos)) = (entity.level(host),entity.position(host)) {
        LOADED.with_borrow_mut(|m| {
            let entries=m.entry((level.dimension_id().into(),pos)).or_default();
            if !entries.contains(&entity){entries.push(entity);}
        });
    }
}
pub fn addresses() -> Vec<(String,[i32;3],BlockEntityRef)> {
    LOADED.with_borrow(|m|m.iter().flat_map(|((d,p),es)|es.iter().map(|e|(d.clone(),*p,*e))).collect())
}
pub fn clear() { LOADED.with_borrow_mut(BTreeMap::clear); }
pub fn candidates(pos:[i32;3])->Vec<(String,BlockEntityRef)>{
    LOADED.with_borrow(|m|m.iter().filter(|((_,p),_)|*p==pos)
        .flat_map(|((d,_),entries)|entries.iter().map(|e|(d.clone(),*e))).collect())
}
pub fn prune(host:&dyn Host,level:LevelRef){
    LOADED.with_borrow_mut(|m|m.retain(|(dim,_),entries|{
        if dim==level.dimension_id(){entries.retain(|entity|entity.data(host).is_some());}
        !entries.is_empty()
    }));
}
pub fn block(host: &dyn Host, at: BlockSide) -> Option<String> {
    Some(at.level.block(host,at.pos)?.id(host).to_owned())
}

pub fn is_drawer(id: &str) -> bool { id.starts_with("storagedrawers:") && id.contains("drawers_") }
pub fn is_controller(id: &str) -> bool { matches!(id, "storagedrawers:controller" | "storagedrawers:framed_controller") }
pub fn is_port(id: &str) -> bool { matches!(id, "storagedrawers:controller_io" | "storagedrawers:framed_controller_io") }
pub fn slots(id: &str) -> usize {
    id.rsplit('_').next().and_then(|s| s.parse().ok()).filter(|n| [1,2,3,4].contains(n)).unwrap_or(1)
}
pub fn initial(id: &str) -> Option<Compound> {
    let mut nbt = Compound::new();
    nbt.insert("sd_block".into(), Tag::String(id.into()));
    if is_drawer(id) {
        let group = sd::Group::standard(if id.contains("compacting") { 1 } else { slots(id) }, id.contains("half")).ok()?;
        nbt.insert(SAVE.into(), Tag::ByteArray(sd::codec::encode(&group).ok()?));
    }
    Some(nbt)
}
pub fn group(nbt: &Compound) -> Option<sd::Group> {
    match nbt.get(&SAVE.into()) { Some(Tag::ByteArray(bytes)) => sd::codec::decode(bytes).ok(), _ => None }
}
pub fn upgrade_state(nbt:&Compound,g:&sd::Group)->(Vec<usize>,Vec<crate::compat::Stack>){
    let positions=crate::items::positions(nbt,g);
    let items=crate::items::upgrade_items(nbt,g,&positions);
    (positions,items)
}
pub fn save_upgrades(nbt:&mut Compound,g:&sd::Group,positions:&[usize],items:&[crate::compat::Stack])->bool{
    if g.upgrades.len()!=positions.len()||positions.len()!=items.len()||positions.iter().any(|p|*p>=sd::UPGRADE_SLOTS)
        ||positions.iter().collect::<std::collections::BTreeSet<_>>().len()!=positions.len()
        ||items.iter().zip(&g.upgrades).any(|(s,u)|s.count!=1||crate::items::upgrade(s)!=Some(*u)){return false}
    nbt.insert("sd_upgrade_slots".into(),Tag::IntArray(positions.iter().map(|p|*p as i32).collect()));
    nbt.insert("sd_upgrade_items".into(),Tag::List(positions.iter().zip(items).map(|(p,s)|Tag::compound([
        ("slot",Tag::Int(*p as i32)),("item",crate::compat::item_to_nbt(s))])).collect()));
    nbt.remove(&"sd_remote".into());true
}
pub fn conversion(host:&dyn Host,g:&sd::Group,slot:usize,input:&ItemStack)->sd::ItemKey{
    let key=group_key(host,g,input);
    if !g.has(sd::Upgrade::Conversion){return key}
    let Some(target)=g.item(slot)else{return key};if target.id==key.id{return key}
    let cfg=sd::config::features();
    let grouped=cfg["conversion_equivalence_groups"].as_array().is_some_and(|groups|groups.iter().any(|g|
        g.as_array().is_some_and(|v|v.iter().any(|i|i.as_str()==Some(&key.id))&&v.iter().any(|i|i.as_str()==Some(&target.id)))));
    let tagged=cfg["conversion_tag_allowlist"].as_array().is_some_and(|tags|tags.iter().filter_map(|s|s.as_str()).any(|tag|{
        if cfg["conversion_tag_denylist"].as_array().is_some_and(|v|v.iter().any(|s|s.as_str()==Some(tag))){return false}
        let Some(tag)=Identifier::parse(tag).ok().and_then(|t|host.tags().key(&t))else{return false};
        let Some(target)=Identifier::parse(&target.id).ok().and_then(|id|host.item_key(&id))else{return false};
        host.tags().is_in_tag(input.item(),tag)==Some(true)&&host.tags().is_in_tag(target,tag)==Some(true)
    }));
    if grouped||tagged{target.clone()}else{key}
}
pub fn insert(host:&dyn Host,g:&mut sd::Group,indices:&[usize],input:&ItemStack,amount:u64,access:sd::Access)->sd::Inserted{
    if !sd::config::can_store(&input.item().id(host)){return sd::Inserted::default()}
    let item=group_key(host,g,input);
    let conversions:Vec<_>=indices.iter().filter_map(|i|{
        let target=g.item(*i)?;
        target.same_item(&conversion(host,g,*i,input)).then_some(target.clone())
    }).collect();
    g.insert_all(indices,&sd::Stack{item,count:amount},access,false,&|target,_|conversions.iter().any(|k|k.same_item(target)))
}
pub fn read(host: &dyn Host, at: BlockSide) -> Option<(BlockEntityRef, Compound, sd::Group)> {
    let entity = host.block_entity(at.level, at.pos)?;
    remember(host,entity);
    let nbt = entity.data(host)?;
    let group = group(&nbt)?;
    Some((entity, nbt, group))
}
pub fn write(host: &mut dyn ServerHost, entity: BlockEntityRef, mut nbt: Compound, group: &sd::Group) -> bool {
    let Ok(bytes) = sd::codec::encode(group) else { return false };
    nbt.insert(SAVE.into(), Tag::ByteArray(bytes));
    if entity.data(host).as_ref() == Some(&nbt) { return true; }
    matches!(entity.replace_data(host, nbt), Ok(true))
}
pub fn key(host: &dyn Host, stack: &ItemStack) -> sd::ItemKey {
    let mut one = stack.clone();
    let _ = one.set_count(1);
    let components = one.to_nbt(host).map(|nbt| snbt(&Tag::Compound(nbt))).unwrap_or_default();
    sd::ItemKey { id: stack.item().id(host).to_owned(), components,
        max_stack: max_stack(host,stack) }
}
pub fn max_stack(host: &dyn Host, stack: &ItemStack) -> u32 {
    stack.components().and_then(|c|c.as_nbt().get(&"minecraft:max_stack_size".into()))
        .and_then(|t|match t {Tag::Int(v)=>u32::try_from(*v).ok(),_=>None})
        .unwrap_or_else(||host.max_stack_size(stack.item()).unwrap_or(64)).clamp(1,99)
}
// Existing worlds used a different SNBT formatter. Compare decoded stack values
// before selecting a saved key; never relabel or discard old stored components.
pub fn group_key(host: &dyn Host, group: &sd::Group, input: &ItemStack) -> sd::ItemKey {
    for i in 0..group.drawers.len() {
        if let Some(saved) = group.item(i) {
            if stack(host,saved,1).is_some_and(|s|s.same_item(input)&&s.same_components(input)) {
                return saved.clone();
            }
        }
    }
    key(host,input)
}
pub fn stack(host: &dyn Host, key: &sd::ItemKey, count: u32) -> Option<ItemStack> {
    let mut stack = if key.components.is_empty() {
        ItemStack::of(host.item_key(&Identifier::parse(&key.id).ok()?)?)
    } else { host.item_from_snbt(&key.components)? };
    stack.set_count(count).ok()?;
    Some(stack)
}
pub fn analog(level: &dyn LevelHost, event: &AnalogOutputEvent) -> i32 {
    let pos=[event.pos.x,event.pos.y,event.pos.z];
    candidates(pos).into_iter()
        .find_map(|(_,e)|level.block_entity_data(e)).and_then(|nbt|group(&nbt))
        .map_or(0,|g|i32::from(g.redstone()))
}

// The core's existing item identity includes the complete one-item SNBT. Keep
// this serializer deterministic; restored old keys are compared as parsed stacks
// before they are rewritten in the SDK representation.
fn snbt(tag: &Tag) -> String {
    match tag {
        Tag::End => "END".into(),
        Tag::Byte(v) => format!("{v}b"), Tag::Short(v) => format!("{v}s"),
        Tag::Int(v) => v.to_string(), Tag::Long(v) => format!("{v}L"),
        Tag::Float(v) => format!("{:?}f",f32::from_bits(*v)),
        Tag::Double(v) => format!("{:?}d",f64::from_bits(*v)),
        Tag::String(v) => serde_json::to_string(&v.to_string_lossy()).unwrap(),
        Tag::ByteArray(v) => format!("[B;{}]", v.iter().map(|x|format!("{}b", *x as i8)).collect::<Vec<_>>().join(",")),
        Tag::IntArray(v) => format!("[I;{}]", v.iter().map(ToString::to_string).collect::<Vec<_>>().join(",")),
        Tag::LongArray(v) => format!("[L;{}]", v.iter().map(|x|format!("{x}L")).collect::<Vec<_>>().join(",")),
        Tag::List(v) => format!("[{}]", v.iter().map(snbt).collect::<Vec<_>>().join(",")),
        Tag::Compound(v) => format!("{{{}}}", v.iter().map(|(k,v)|format!("{}:{}", serde_json::to_string(&k.to_string_lossy()).unwrap(), snbt(v))).collect::<Vec<_>>().join(",")),
    }
}
