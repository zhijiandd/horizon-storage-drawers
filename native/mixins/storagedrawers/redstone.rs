// Comparator and optional direct redstone output for drawer blocks.
use super::sd_integration::{drawer,feature_config,group};
use horizon_client::containers::BlockEntity;
use horizon_core::geometry::Direction;

type Pos=[i32;3];
fn redstone_analog()->bool{feature_config()["redstone_analog_output"].as_bool().unwrap_or(true)}
pub(super) fn signal_source(level:&super::GameLevel<'_>,state:horizon_world::registry::BlockStateId,source:&mut bool){
    if level.registry.state(state).is_some_and(|s|drawer(&s.name)){*source=!redstone_analog();}
}
pub(super) fn has_analog(level:&super::GameLevel<'_>,state:horizon_world::registry::BlockStateId,output:&mut bool){
    if level.registry.state(state).is_some_and(|s|drawer(&s.name)){*output=true;}
}
pub(super) fn signal(level:&super::GameLevel<'_>,pos:Pos,face:Direction,direct:bool,signal:&mut i32){
    let state=level.world.block(pos[0],pos[1],pos[2]);
    if !level.registry.state(state).is_some_and(|s|drawer(&s.name)){return;}
    *signal=0;if redstone_analog()||direct&&face!=Direction::Up{return;}
    analog(level,pos,signal);
}
pub(super) fn analog(level:&super::GameLevel<'_>,pos:Pos,signal:&mut i32){
    let state=level.world.block(pos[0],pos[1],pos[2]);
    if !level.registry.state(state).is_some_and(|s|drawer(&s.name)){return;}*signal=0;
    if let Some(BlockEntity::Mod(m))=level.world.block_entities.get(&(pos[0],pos[1],pos[2])){
        if drawer(&m.id){if let Ok(g)=group(m){*signal=i32::from(g.redstone());}}
    }
}
