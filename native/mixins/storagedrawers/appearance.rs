// Upstream part geometry and SpriteReplacementModel particle-sprite replacement.
use super::{Game,sd_items as data};
use horizon_client::{containers::ModBlockEntity,items::ItemStack,item_render::{ItemRenderState,LayerGeometry}};
use horizon_assets::{bake::ModelQuad,item_model::{DisplayContext,OwnerState}};
use horizon_world::nbt::Tag;
use std::sync::Arc;
// Generated from Java 1.21 blockstate metadata.
fn parts(block:&str,open:&str)->&'static [(&'static str,&'static str)]{match (block,open){
("storagedrawers:framed_full_drawers_1","open1")=>&[("side","storagedrawers:block/framed/framed_full_drawers_sides"),("trim","storagedrawers:block/framed/framed_full_drawers_trim_1"),("front","storagedrawers:block/framed/framed_full_drawers_front_1"),("shading","storagedrawers:block/framed/framed_full_drawers_front_shading_1")],
("storagedrawers:framed_full_drawers_2","open1")=>&[("side","storagedrawers:block/framed/framed_full_drawers_sides"),("trim","storagedrawers:block/framed/framed_full_drawers_trim_2"),("front","storagedrawers:block/framed/framed_full_drawers_front_2"),("shading","storagedrawers:block/framed/framed_full_drawers_front_shading_2")],
("storagedrawers:framed_full_drawers_4","open1")=>&[("side","storagedrawers:block/framed/framed_full_drawers_sides"),("trim","storagedrawers:block/framed/framed_full_drawers_trim_4"),("front","storagedrawers:block/framed/framed_full_drawers_front_4"),("shading","storagedrawers:block/framed/framed_full_drawers_front_shading_4")],
("storagedrawers:framed_half_drawers_1","open1")=>&[("side","storagedrawers:block/framed/framed_half_drawers_sides"),("trim","storagedrawers:block/framed/framed_half_drawers_trim_1"),("front","storagedrawers:block/framed/framed_half_drawers_front_1"),("shading","storagedrawers:block/framed/framed_half_drawers_front_shading_1")],
("storagedrawers:framed_half_drawers_2","open1")=>&[("side","storagedrawers:block/framed/framed_half_drawers_sides"),("trim","storagedrawers:block/framed/framed_half_drawers_trim_2"),("front","storagedrawers:block/framed/framed_half_drawers_front_2"),("shading","storagedrawers:block/framed/framed_half_drawers_front_shading_2")],
("storagedrawers:framed_half_drawers_4","open1")=>&[("side","storagedrawers:block/framed/framed_half_drawers_sides"),("trim","storagedrawers:block/framed/framed_half_drawers_trim_4"),("front","storagedrawers:block/framed/framed_half_drawers_front_4"),("shading","storagedrawers:block/framed/framed_half_drawers_front_shading_4")],
("storagedrawers:framed_trim","open1")=>&[("side","storagedrawers:block/framed/framed_trim_sides"),("trim","storagedrawers:block/framed/framed_trim_trim")],
("storagedrawers:framed_compacting_drawers_2","open1")=>&[("side","storagedrawers:block/framed/framed_full_drawers_sides"),("trim","storagedrawers:block/framed/framed_full_drawers_trim_2"),("front","storagedrawers:block/framed/framed_full_drawers_front_2"),("shading","storagedrawers:block/framed/framed_full_compdrawers_2_front_shading_open1")],
("storagedrawers:framed_compacting_drawers_2","open2")=>&[("side","storagedrawers:block/framed/framed_full_drawers_sides"),("trim","storagedrawers:block/framed/framed_full_drawers_trim_2"),("front","storagedrawers:block/framed/framed_full_drawers_front_2"),("shading","storagedrawers:block/framed/framed_full_compdrawers_2_front_shading_open2")],
("storagedrawers:framed_compacting_drawers_3","open1")=>&[("side","storagedrawers:block/framed/framed_full_drawers_sides"),("trim","storagedrawers:block/framed/framed_full_drawers_trim_3"),("front","storagedrawers:block/framed/framed_full_drawers_front_3"),("shading","storagedrawers:block/framed/framed_full_compdrawers_3_front_shading_open1")],
("storagedrawers:framed_compacting_drawers_3","open2")=>&[("side","storagedrawers:block/framed/framed_full_drawers_sides"),("trim","storagedrawers:block/framed/framed_full_drawers_trim_3"),("front","storagedrawers:block/framed/framed_full_drawers_front_3"),("shading","storagedrawers:block/framed/framed_full_compdrawers_3_front_shading_open2")],
("storagedrawers:framed_compacting_drawers_3","open3")=>&[("side","storagedrawers:block/framed/framed_full_drawers_sides"),("trim","storagedrawers:block/framed/framed_full_drawers_trim_3"),("front","storagedrawers:block/framed/framed_full_drawers_front_3"),("shading","storagedrawers:block/framed/framed_full_compdrawers_3_front_shading_open3")],
("storagedrawers:framed_compacting_half_drawers_2","open1")=>&[("side","storagedrawers:block/framed/framed_half_drawers_sides"),("trim","storagedrawers:block/framed/framed_half_drawers_trim_2"),("front","storagedrawers:block/framed/framed_half_drawers_front_2"),("shading","storagedrawers:block/framed/framed_half_compdrawers_2_front_shading_open1")],
("storagedrawers:framed_compacting_half_drawers_2","open2")=>&[("side","storagedrawers:block/framed/framed_half_drawers_sides"),("trim","storagedrawers:block/framed/framed_half_drawers_trim_2"),("front","storagedrawers:block/framed/framed_half_drawers_front_2"),("shading","storagedrawers:block/framed/framed_half_compdrawers_2_front_shading_open2")],
("storagedrawers:framed_compacting_half_drawers_3","open1")=>&[("side","storagedrawers:block/framed/framed_half_drawers_sides"),("trim","storagedrawers:block/framed/framed_half_drawers_trim_3"),("front","storagedrawers:block/framed/framed_half_drawers_front_3"),("shading","storagedrawers:block/framed/framed_half_compdrawers_3_front_shading_open1")],
("storagedrawers:framed_compacting_half_drawers_3","open2")=>&[("side","storagedrawers:block/framed/framed_half_drawers_sides"),("trim","storagedrawers:block/framed/framed_half_drawers_trim_3"),("front","storagedrawers:block/framed/framed_half_drawers_front_3"),("shading","storagedrawers:block/framed/framed_half_compdrawers_3_front_shading_open2")],
("storagedrawers:framed_compacting_half_drawers_3","open3")=>&[("side","storagedrawers:block/framed/framed_half_drawers_sides"),("trim","storagedrawers:block/framed/framed_half_drawers_trim_3"),("front","storagedrawers:block/framed/framed_half_drawers_front_3"),("shading","storagedrawers:block/framed/framed_half_compdrawers_3_front_shading_open3")],
("storagedrawers:framed_controller","open1")=>&[("side","storagedrawers:block/framed/framed_full_drawers_sides"),("trim","storagedrawers:block/framed/framed_full_drawers_trim_2"),("front","storagedrawers:block/framed/framed_full_drawers_front_2"),("shading","storagedrawers:block/framed/framed_controller_shading")],
("storagedrawers:framed_controller_io","open1")=>&[("side","storagedrawers:block/framed/framed_trim_sides"),("trim","storagedrawers:block/framed/framed_trim_trim"),("shading","storagedrawers:block/framed/framed_controller_io_shading")],
_=>&[],}}
fn decoration(role:&str,half:bool,slot:usize)->Option<&'static str>{Some(match (role,half,slot){
("lock",false,0)=>"storagedrawers:block/full_drawers_lock",
("claim",false,0)=>"storagedrawers:block/full_drawers_claim",
("lock_claim",false,0)=>"storagedrawers:block/full_drawers_lock_claim",
("right_label",false,1)=>"storagedrawers:block/full_drawers_right_label_1",
("right_label",false,2)=>"storagedrawers:block/full_drawers_right_label_2",
("right_label",false,3)=>"storagedrawers:block/full_drawers_right_label_3",
("right_label",false,4)=>"storagedrawers:block/full_drawers_right_label_4",
("right_label",false,5)=>"storagedrawers:block/full_drawers_right_label_5",
("right_label",false,6)=>"storagedrawers:block/full_drawers_right_label_6",
("lock",true,0)=>"storagedrawers:block/half_drawers_lock",
("claim",true,0)=>"storagedrawers:block/half_drawers_claim",
("lock_claim",true,0)=>"storagedrawers:block/half_drawers_lock_claim",
("right_label",true,1)=>"storagedrawers:block/half_drawers_right_label_1",
("right_label",true,2)=>"storagedrawers:block/half_drawers_right_label_2",
("right_label",true,3)=>"storagedrawers:block/half_drawers_right_label_3",
("right_label",true,4)=>"storagedrawers:block/half_drawers_right_label_4",
("right_label",true,5)=>"storagedrawers:block/half_drawers_right_label_5",
("right_label",true,6)=>"storagedrawers:block/half_drawers_right_label_6",
_=>return None,})}
pub(super) fn models(out:&mut Vec<String>){out.extend(["storagedrawers:block/framed/framed_controller_io_shading".to_owned(),"storagedrawers:block/framed/framed_controller_shading".to_owned(),"storagedrawers:block/framed/framed_full_compdrawers_2_front_shading_open1".to_owned(),"storagedrawers:block/framed/framed_full_compdrawers_2_front_shading_open2".to_owned(),"storagedrawers:block/framed/framed_full_compdrawers_3_front_shading_open1".to_owned(),"storagedrawers:block/framed/framed_full_compdrawers_3_front_shading_open2".to_owned(),"storagedrawers:block/framed/framed_full_compdrawers_3_front_shading_open3".to_owned(),"storagedrawers:block/framed/framed_full_drawers_front_1".to_owned(),"storagedrawers:block/framed/framed_full_drawers_front_2".to_owned(),"storagedrawers:block/framed/framed_full_drawers_front_3".to_owned(),"storagedrawers:block/framed/framed_full_drawers_front_4".to_owned(),"storagedrawers:block/framed/framed_full_drawers_front_shading_1".to_owned(),"storagedrawers:block/framed/framed_full_drawers_front_shading_2".to_owned(),"storagedrawers:block/framed/framed_full_drawers_front_shading_4".to_owned(),"storagedrawers:block/framed/framed_full_drawers_sides".to_owned(),"storagedrawers:block/framed/framed_full_drawers_trim_1".to_owned(),"storagedrawers:block/framed/framed_full_drawers_trim_2".to_owned(),"storagedrawers:block/framed/framed_full_drawers_trim_3".to_owned(),"storagedrawers:block/framed/framed_full_drawers_trim_4".to_owned(),"storagedrawers:block/framed/framed_half_compdrawers_2_front_shading_open1".to_owned(),"storagedrawers:block/framed/framed_half_compdrawers_2_front_shading_open2".to_owned(),"storagedrawers:block/framed/framed_half_compdrawers_3_front_shading_open1".to_owned(),"storagedrawers:block/framed/framed_half_compdrawers_3_front_shading_open2".to_owned(),"storagedrawers:block/framed/framed_half_compdrawers_3_front_shading_open3".to_owned(),"storagedrawers:block/framed/framed_half_drawers_front_1".to_owned(),"storagedrawers:block/framed/framed_half_drawers_front_2".to_owned(),"storagedrawers:block/framed/framed_half_drawers_front_3".to_owned(),"storagedrawers:block/framed/framed_half_drawers_front_4".to_owned(),"storagedrawers:block/framed/framed_half_drawers_front_shading_1".to_owned(),"storagedrawers:block/framed/framed_half_drawers_front_shading_2".to_owned(),"storagedrawers:block/framed/framed_half_drawers_front_shading_4".to_owned(),"storagedrawers:block/framed/framed_half_drawers_sides".to_owned(),"storagedrawers:block/framed/framed_half_drawers_trim_1".to_owned(),"storagedrawers:block/framed/framed_half_drawers_trim_2".to_owned(),"storagedrawers:block/framed/framed_half_drawers_trim_3".to_owned(),"storagedrawers:block/framed/framed_half_drawers_trim_4".to_owned(),"storagedrawers:block/framed/framed_trim_sides".to_owned(),"storagedrawers:block/framed/framed_trim_trim".to_owned(),"storagedrawers:block/full_drawers_claim".to_owned(),"storagedrawers:block/full_drawers_hopper".to_owned(),"storagedrawers:block/full_drawers_lock".to_owned(),"storagedrawers:block/full_drawers_lock_claim".to_owned(),"storagedrawers:block/full_drawers_missing_1_1".to_owned(),"storagedrawers:block/full_drawers_missing_2_1".to_owned(),"storagedrawers:block/full_drawers_missing_2_2".to_owned(),"storagedrawers:block/full_drawers_missing_4_1".to_owned(),"storagedrawers:block/full_drawers_missing_4_2".to_owned(),"storagedrawers:block/full_drawers_missing_4_3".to_owned(),"storagedrawers:block/full_drawers_missing_4_4".to_owned(),"storagedrawers:block/full_drawers_right_label_1".to_owned(),"storagedrawers:block/full_drawers_right_label_2".to_owned(),"storagedrawers:block/full_drawers_right_label_3".to_owned(),"storagedrawers:block/full_drawers_right_label_4".to_owned(),"storagedrawers:block/full_drawers_right_label_5".to_owned(),"storagedrawers:block/full_drawers_right_label_6".to_owned(),"storagedrawers:block/half_drawers_claim".to_owned(),"storagedrawers:block/half_drawers_lock".to_owned(),"storagedrawers:block/half_drawers_lock_claim".to_owned(),"storagedrawers:block/half_drawers_missing_1_1".to_owned(),"storagedrawers:block/half_drawers_missing_2_1".to_owned(),"storagedrawers:block/half_drawers_missing_2_2".to_owned(),"storagedrawers:block/half_drawers_missing_4_1".to_owned(),"storagedrawers:block/half_drawers_missing_4_2".to_owned(),"storagedrawers:block/half_drawers_missing_4_3".to_owned(),"storagedrawers:block/half_drawers_missing_4_4".to_owned(),"storagedrawers:block/half_drawers_right_label_1".to_owned(),"storagedrawers:block/half_drawers_right_label_2".to_owned(),"storagedrawers:block/half_drawers_right_label_3".to_owned(),"storagedrawers:block/half_drawers_right_label_4".to_owned(),"storagedrawers:block/half_drawers_right_label_5".to_owned(),"storagedrawers:block/half_drawers_right_label_6".to_owned()]);}
fn missing(block:&str,slot:usize)->String{
    let depth=if block.contains("half"){"half"}else{"full"};let n=block.rsplit('_').next().unwrap_or("1");
    format!("storagedrawers:block/{depth}_drawers_missing_{n}_{}",slot+1)
}
fn particle(game:&Game<'_>,frame:&Tag,role:&str)->Option<(String,bool)>{
    let item=frame.get(role).or_else(||frame.get("side")).and_then(horizon_items::item_nbt::item_from_nbt)?;
    let state=game.items().get(&item.item)?.block?;
    let layer=game.pack.render.model_catalog.particle(state)?;
    let translucent=game.res.behaviours().facts(state).flags&horizon_world::level::data::state_flags::CAN_OCCLUDE==0;
    Some((game.pack.layer_names.get(&layer)?.clone(),translucent))
}
fn quads(game:&Game<'_>,block:&str,open:&str,frame:Option<&Tag>)->Vec<ModelQuad>{
    let mut out=Vec::new();for &(role,model) in parts(block,open){
        let Some(baked)=game.pack.render.model_catalog.standalone(model)else{continue;};
        let replacement=if role=="shading"{None}else{frame.and_then(|f|particle(game,f,role))};
        for f in &baked.faces{let mut q=f.quad.clone();
            if let Some((sprite,translucent))=&replacement{q.material.sprite=sprite.clone();q.material.force_translucent=*translucent;}
            if role=="shading"{q.material.force_translucent=true;}
            q.render_type=Some(if q.material.force_translucent{horizon_assets::bake::RenderType::Translucent}else{horizon_assets::bake::RenderType::CutoutMipped});
            out.push(q);
        }
    }out
}
fn open(group:Option<&storagedrawers_engine::Group>)->&'static str{match group.and_then(|g|g.compact.as_ref()).map_or(1,|c|c.views.len()){3=>"open3",2=>"open2",_=>"open1"}}
fn badges(game:&Game<'_>,block:&str,g:&storagedrawers_engine::Group)->Vec<ModelQuad>{
    use storagedrawers_engine::Upgrade as U;
    let mut out=Vec::new();let half=block.contains("half");
    let mut append=|role:&str,slot:usize,icon:Option<&str>|{
        let Some(model)=decoration(role,half,slot).and_then(|m|game.pack.render.model_catalog.standalone(m))else{return;};
        for face in &model.faces{let mut q=face.quad.clone();
            if let Some(icon)=icon{q.material.sprite=format!("storagedrawers:block/indicator/{icon}");}
            q.material.force_translucent=false;q.render_type=Some(horizon_assets::bake::RenderType::CutoutMipped);out.push(q);
        }
    };
    let a=&g.attributes;
    if a.locked&&a.owner.is_some(){append("lock_claim",0,None);}
    else if a.locked{append("lock",0,None);}else if a.owner.is_some(){append("claim",0,None);}
    // DrawerModelDecorator.emitDecoratedQuads order and original label models.
    let mut icons=Vec::new();if g.has(U::Void){icons.push("void_icon");}
    match a.priority{-2=>icons.push("priority_n2_icon"),-1=>icons.push("priority_n1_icon"),
        1=>icons.push("priority_p1_icon"),2=>icons.push("priority_p2_icon"),_=>{}}
    if [U::Magnet1,U::Magnet2,U::Magnet3].iter().any(|&u|g.has(u)){icons.push("magnet_icon");}
    if a.shrouded{icons.push("shroud_icon");}if a.suspended{icons.push("suspend_icon");}
    for (slot,icon) in icons.into_iter().enumerate(){append("right_label",slot+1,Some(icon));}
    out
}
fn hopper_quads(game:&Game<'_>)->Vec<ModelQuad>{
    game.pack.render.model_catalog.standalone("storagedrawers:block/full_drawers_hopper").map(|m|m.faces.iter().map(|f|{
        let mut q=f.quad.clone();q.material.force_translucent=false;
        q.render_type=Some(horizon_assets::bake::RenderType::CutoutMipped);q
    }).collect()).unwrap_or_default()
}
// The centre of an opaque placed block has no sky light. Terrain faces are
// lit from the space outside the face, not from that dark interior cell.
fn face_light(game:&Game<'_>,pos:[i32;3],q:&ModelQuad)->u32{
    use horizon_assets::model::Direction as D;
    let d=match q.facing{D::Up=>[0,1,0],D::Down=>[0,-1,0],D::North=>[0,0,-1],D::South=>[0,0,1],D::East=>[1,0,0],D::West=>[-1,0,0]};
    let at:[i32;3]=std::array::from_fn(|i|pos[i]+d[i]);
    let light=horizon_client::block_entity_render::vertex_light(game.world.light(at[0],at[1],at[2]));
    (light&0xFF)|(((light>>8)&0xFF).max(u32::from(q.light_emission)*16)<<8)
}
fn visible(game:&Game<'_>,pos:[i32;3],q:&ModelQuad)->bool{
    use horizon_assets::model::Direction as D;
    use horizon_core::geometry::{Direction,Aabb,Vec3,ShapeOp,VoxelShape};
    let Some(face)=q.cull else{return true;};
    let face=match face{D::Down=>Direction::Down,D::Up=>Direction::Up,D::North=>Direction::North,
        D::South=>Direction::South,D::West=>Direction::West,D::East=>Direction::East};
    let offset=face.normal();let p:[i32;3]=std::array::from_fn(|i|pos[i]+offset[i]);
    let neighbor=game.world.block(p[0],p[1],p[2]);
    let Some((id,shape))=game.pack.context.tables.visibility.occlusion_face(neighbor,face.opposite())else{return true;};
    if id==0{return true;}if id==1{return false;}
    // Use the same neighbour occlusion shapes as terrain, restricted to this
    // declared cullface's rectangle; a partial neighbour must not hide it all.
    let mut min:[f64;3]=std::array::from_fn(|i|q.positions.iter().map(|p|f64::from(p[i])).fold(f64::INFINITY,f64::min));
    let mut max:[f64;3]=std::array::from_fn(|i|q.positions.iter().map(|p|f64::from(p[i])).fold(f64::NEG_INFINITY,f64::max));
    let axis=offset.iter().position(|&v|v!=0).unwrap();min[axis]=0.;max[axis]=1.;
    let Ok(own)=VoxelShape::from_box(Aabb::new(Vec3::new(min[0],min[1],min[2]),Vec3::new(max[0],max[1],max[2])))else{return true;};
    own.join_is_not_empty(shape,ShapeOp::ONLY_FIRST).unwrap_or(true)
}
fn draw_quads(game:&Game<'_>,pos:[i32;3],draw:&mut super::native_block_entity_draw::BlockEntityDraw,quads:Vec<ModelQuad>){
    use horizon_client::item_render::{WorldSink,ItemSink,ItemQuad,Foil};
    use horizon_assets::model::Direction as D;
    let pose=draw.pose();
    for q in quads{
        if !visible(game,pos,&q){continue;}
        let Some(sprite)=game.gui.atlas.sprites.get(&q.material.sprite)else{continue;};
        let shade:u32=if !q.shade{255}else{match q.facing{D::Up=>255,D::Down=>127,D::North|D::South=>204,D::West|D::East=>153}};
        let mut sink=WorldSink{out:draw.vertices_mut(),light:face_light(game,pos,&q),lights:None,glint:None,tier:0};
        sink.quad(ItemQuad{
            pos:q.positions.map(|p|pose.transform_point3(glam::Vec3::from_array(p))),
            uv:q.uv.map(|[u,v]|[sprite.u0+u*(sprite.u1-sprite.u0),sprite.v0+v*(sprite.v1-sprite.v0)]),
            foil_uv:q.uv,gui_glint_uv:None,normal:glam::Vec3::ZERO,
            color:0xFF00_0000|shade<<16|shade<<8|shade,foil:Foil::None,light_emission:q.light_emission,
            cull_back:true,translucent:match q.render_type{
                Some(horizon_assets::bake::RenderType::Translucent|horizon_assets::bake::RenderType::Tripwire)=>true,
                Some(horizon_assets::bake::RenderType::Solid|horizon_assets::bake::RenderType::Cutout|horizon_assets::bake::RenderType::CutoutMipped)=>false,
                _=>q.material.force_translucent||game.gui.atlas.translucent.contains(&q.material.sprite)},oit:false,
        });
    }
}
pub(super) fn render(game:&Game<'_>,pos:[i32;3],block:&str,m:&ModBlockEntity,draw:&mut super::native_block_entity_draw::BlockEntityDraw){
    if block=="storagedrawers:framing_table"{super::sd_framing::render(game,pos,m,draw);return;}
    let g=match m.nbt.get(&"sd_state".into()){Some(Tag::ByteArray(bytes))=>storagedrawers_engine::codec::decode(bytes).ok(),_=>None};
    let facing=game.native_block_property(pos,"facing").unwrap_or_else(||"north".into());
    let frame=m.nbt.get(&"sd_frame".into());
    if block.starts_with("storagedrawers:framed_")&&frame.is_none(){
        let state=game.world.block(pos[0],pos[1],pos[2]);let pose=draw.pose();
        let grass=game.pack.item_renderer.grass_color(0.5,1.)as u32;
        game.tinted_block_model_vertices(state,42,pose,|q|face_light(game,pos,q),false,
            |i|game.pack.context.tables.default_tint(state,i,grass),draw.vertices_mut());
    }
    // Geometry remains in world orientation for directional terrain light.
    let angle:f32=match facing.as_str(){"east"=>-90.,"south"=>-180.,"west"=>-270.,_=>0.};
    let mut geometry=if frame.is_some(){quads(game,block,open(g.as_ref()),frame)}else{Vec::new()};
    if let Some(g)=&g{geometry.extend(badges(game,block,g));}
    if !geometry.is_empty(){
        // Rotate both positions and facing; rotating only the pose would
        // sample north light even when the drawer faces east or west.
        let matrix=glam::Mat4::from_translation(glam::Vec3::splat(0.5))*glam::Mat4::from_rotation_y(angle.to_radians())*glam::Mat4::from_translation(glam::Vec3::splat(-0.5));
        for q in &mut geometry{
            q.positions=q.positions.map(|p|matrix.transform_point3(glam::Vec3::from_array(p)).to_array());
            use horizon_assets::model::Direction as D;
            for _ in 0..match facing.as_str(){"east"=>1,"south"=>2,"west"=>3,_=>0}{
                let turn=|d|match d{D::North=>D::East,D::East=>D::South,D::South=>D::West,D::West=>D::North,v=>v};
                q.facing=turn(q.facing);q.cull=q.cull.map(turn);
            }
        }
        draw_quads(game,pos,draw,geometry);
    }
    if g.as_ref().is_some_and(|g|g.has(storagedrawers_engine::Upgrade::Hopper)){
        draw_quads(game,pos,draw,hopper_quads(game));
    }
    draw.push();draw.rotate_about([0.,1.,0.],match facing.as_str(){"east"=>-90.,"south"=>-180.,"west"=>-270.,_=>0.},[0.5,0.5,0.5]);
    if let Some(g)=&g{if g.compact.is_none(){for (slot,d) in g.drawers.iter().enumerate(){if d.detached{game.native_draw_model(draw,&missing(block,slot),0xFFFFFF);}}}}
    draw.pop();
}
pub(super) fn item(game:&Game<'_>,stack:&ItemStack,display:DisplayContext,owner:Option<&OwnerState>,seed:i32,hand:horizon_client::cit::Hand)->Option<ItemRenderState>{
    if !stack.item.starts_with("storagedrawers:"){return None;}
    let frame=data::frame(stack);let g=data::group(stack);
    let detached=g.as_ref().is_some_and(|g|g.drawers.iter().any(|d|d.detached));
    let mut extra=g.as_ref().map(|g|badges(game,&stack.item,g)).unwrap_or_default();
    if g.as_ref().is_some_and(|g|g.has(storagedrawers_engine::Upgrade::Hopper)){extra.extend(hopper_quads(game));}
    if frame.is_none()&&!detached&&extra.is_empty(){return None;}
    let mut plain=stack.clone();let mut c=data::custom(&plain);c.remove(&"sd_frame".into());c.remove(&"sd_contents".into());data::set_custom(&mut plain,c);
    let mut state=game.resolve_item_in_hand(&plain,display,owner,seed,hand);
    let mut geometry=if frame.is_some(){quads(game,&stack.item,open(g.as_ref()),frame.as_ref())}else{Vec::new()};
    geometry.extend(extra);
    if let Some(g)=&g{if g.compact.is_none(){for (slot,d) in g.drawers.iter().enumerate(){if d.detached{
        if let Some(model)=game.pack.render.model_catalog.standalone(&missing(&stack.item,slot)){geometry.extend(model.faces.iter().map(|f|f.quad.clone()));}
    }}}}
    for layer in &mut state.layers{if let LayerGeometry::Quads(model)=&layer.geometry{
        let mut model=(**model).clone();if frame.is_some(){model.quads=geometry.clone();}else{model.quads.extend(geometry.clone());}
        layer.extents=Arc::new(model.quads.iter().flat_map(|q|q.positions.map(glam::Vec3::from_array)).collect());
        layer.geometry=LayerGeometry::Quads(Arc::new(model));break;
    }}Some(state)
}
