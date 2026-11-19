//! Owned recipe values. API values are converted only at an item boundary.
use mod_api::{Compound, Tag, Host, ItemStack};

#[derive(Clone, Debug, PartialEq)]
pub struct Stack { pub item: String, pub count: u32, data: Data }
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Data(Compound);
impl Data {
    pub fn other(&self, key: &str) -> Option<&Tag> { self.0.get(&key.into()) }
    pub fn set_other(&mut self, key: &str, value: Option<Tag>) {
        if let Some(v) = value { self.0.insert(key.into(), v); } else { self.0.remove(&key.into()); }
    }
}
impl Stack {
    pub fn new(item: &str, count: u32) -> Self { Self { item: item.into(), count, data: Data::default() } }
    pub fn data(&self) -> Option<&Data> { Some(&self.data) }
    pub fn data_mut(&mut self) -> &mut Data { &mut self.data }
    pub fn same_item_same_components(&self, other: &Self) -> bool { self.item == other.item && self.data == other.data }
    pub fn from_api(host: &dyn Host, stack: &ItemStack) -> Option<Self> { item_from_nbt(&Tag::Compound(stack.to_nbt(host)?)) }
    pub fn to_api(&self, host: &dyn Host) -> Option<ItemStack> {
        match item_to_nbt(self) { Tag::Compound(c) => ItemStack::from_nbt(host, &c), _ => None }
    }
}
pub fn item_from_nbt(tag: &Tag) -> Option<Stack> {
    let Tag::String(id) = tag.get("id")? else { return None };
    let count = match tag.get("count") { Some(Tag::Int(n)) => u32::try_from(*n).ok()?, Some(Tag::Long(n)) => u32::try_from(*n).ok()?, _ => 1 };
    let data = match tag.get("components") { Some(Tag::Compound(c)) => c.clone(), _ => Compound::new() };
    Some(Stack { item: id.to_string_lossy(), count, data: Data(data) })
}
pub fn item_to_nbt(stack: &Stack) -> Tag {
    let mut data = Compound::new();
    data.insert("id".into(), Tag::String(stack.item.as_str().into()));
    data.insert("count".into(), i32::try_from(stack.count).map_or_else(|_|Tag::Long(i64::from(stack.count)),Tag::Int));
    if !stack.data.0.is_empty() { data.insert("components".into(), Tag::Compound(stack.data.0.clone())); }
    Tag::Compound(data)
}

// Crafting can run before world entry or a resource reload. Use the same
// load-owned item keys that Horizon uses to convert the crafting grid.
pub fn registered_item(id:&str)->Option<mod_api::ItemKey>{
    mod_api::__private::item_key(id)
}
impl Stack {
    pub fn from_recipe(stack: &ItemStack) -> Option<Self> {
        let id=mod_api::__private::item_key_id(stack.item())?.to_owned();
        let mut patch=stack.components().map(|c|c.as_nbt().clone()).unwrap_or_default();
        if let Some(d)=stack.damage_component() {
            patch.insert("minecraft:damage".into(),i32::try_from(d).map_or_else(|_|Tag::Long(i64::from(d)),Tag::Int));
        }
        Some(Self{item:id,count:stack.count(),data:Data(patch)})
    }
    pub fn to_recipe(&self) -> Option<ItemStack> {
        let mut stack=ItemStack::new(registered_item(&self.item)?,self.count)?;
        stack.set_components(self.data.0.clone()).ok()?;
        Some(stack)
    }
}
