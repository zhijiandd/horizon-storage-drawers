//! Real recipe serializers: the engine owns ingredient consumption and shift crafting.
use crate::{compat::Stack,items};
use mod_api::*;
const KINDS: &[&str]=&["add_upgrade","add_detached_upgrade","keyring","personal_key_cycle","remote_group_upgrade_bound"];
struct Special(&'static str);
pub fn register(r: &mut dyn Registrar) {
    for &recipe in KINDS {
        // The recipe's ID is distinct from the serializer named by its data.
        let serializer = if recipe == "remote_group_upgrade_bound" { "remote_group_upgrade" } else { recipe };
        r.register_crafting_recipe_serializer(&format!("storagedrawers:{serializer}"), Special(recipe));
    }
}
fn result(grid: CraftingGrid, recipe: &str) -> Option<ItemStack> {
    let size=if grid.width<=2&&grid.height<=2{2}else{3};
    let mut cells=vec![None;size*size];
    for y in 0..grid.height as usize { for x in 0..grid.width as usize {
        cells[y*size+x]=match grid.cells.get(y*grid.width as usize+x)?.as_ref() {
            Some(s)=>Some(Stack::from_recipe(s)?),None=>None,
        };
    }}
    let (id, output) = items::craft(&cells,size)?;
    if id.strip_prefix("storagedrawers:") != Some(recipe) { return None; }
    output.to_recipe()
}
impl CraftingRecipeKind for Special {
    type Value=();
    fn codec(&self)->Option<CraftingRecipeCodec<()>> {Some(CraftingRecipeCodec::wire())}
    fn decode(&self,json:&str)->Result<(),String> {
        serde_json::from_str::<serde_json::Value>(json).map(|_|()).map_err(|e|e.to_string())
    }
    fn write(&self,_:&())->Result<String,String>{Ok("{}".into())}
    fn matches(&self,_:&(),grid:CraftingGrid)->bool{
        result(grid, self.0).is_some()
    }
    fn assemble(&self,_:&(),grid:CraftingGrid)->Option<ItemStack>{
        result(grid, self.0)
    }
}
