fn sd_item_contact(dimension: &str, pos: [i32;3], kind: &str, entity: i32) {
if kind=="minecraft:item"{crate::game::sd_integration::item_contact(dimension,pos,entity as u32);}
}
