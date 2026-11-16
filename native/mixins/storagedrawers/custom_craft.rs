fn sd_custom_craft(_recipes: &crate::crafting::Recipes, grid: impl AsRef<[Option<crate::items::ItemStack>]>, size: usize) -> std::ops::ControlFlow<Option<(&str,ItemStack)>> {
if let Some((id,result))=crate::crafting::sd_items::craft(grid.as_ref(),size){
    if let Some(recipe)=_recipes.custom.values().flatten().find(|r|r.id==id){return std::ops::ControlFlow::Break(Some((recipe.id.as_str(),result)));}
}
std::ops::ControlFlow::Continue(())
}
