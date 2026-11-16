fn sd_custom_recipe_matches(recipes: &crate::crafting::Recipes, id: &str, grid: &[Option<crate::items::ItemStack>], size: usize) -> std::ops::ControlFlow<bool> {
if id.starts_with("storagedrawers:"){
    if let Some((recipe,_))=crate::crafting::sd_items::craft(grid,size){return std::ops::ControlFlow::Break(recipe==id&&recipes.custom.values().flatten().any(|r|r.id==id));}
}
std::ops::ControlFlow::Continue(())
}
