fn sd_io_view(containers: &crate::game::machines::Containers<'_>, pos: (i32,i32,i32)) -> std::ops::ControlFlow<Option<View>> {
if let Some(BlockEntity::Mod(m))=containers.world.block_entities.get(&pos){
    if containers.name(containers.world.block(pos.0,pos.1,pos.2))!=m.id{return std::ops::ControlFlow::Break(None);}
    if let Some(container)=crate::game::sd_integration::io_view(m){
        let n=container.slots;
        return std::ops::ControlFlow::Break(Some(View{parts:vec![(pos,n)],kind:Kind::Mod,container:Some(Arc::new(container))}));
    }
}
std::ops::ControlFlow::Continue(())
}
