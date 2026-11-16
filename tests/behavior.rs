use horizon_storage_drawers_core::*;
fn stone()->ItemKey{ItemKey::plain("minecraft:stone",64)}
fn stack(n:u64)->Stack{Stack{item:stone(),count:n}}
fn compact()->Group{Group::compacting(vec![
    (ItemKey::plain("minecraft:iron_block",64),81),
    (ItemKey::plain("minecraft:iron_ingot",64),9),
    (ItemKey::plain("minecraft:iron_nugget",64),1)],false).unwrap()}
#[test]fn capacities_match_java_121(){
    for (n,half,cap) in [(1,false,2048),(2,false,1024),(4,false,512),(1,true,1024),(2,true,512),(4,true,256)]{
        assert_eq!(Group::standard(n,half).unwrap().capacity(0,&stone()),cap);
    }
}
#[test]fn storage_multipliers_add_instead_of_multiply(){
    let mut g=Group::standard(1,false).unwrap();
    g.add_upgrade(Upgrade::Iron,Access::Administrator).unwrap();
    g.add_upgrade(Upgrade::Gold,Access::Administrator).unwrap();
    assert_eq!(g.capacity(0,&stone()),2048*24);
}
#[test]fn nonstackables_and_small_stacks_use_their_own_limit(){
    let g=Group::standard(1,false).unwrap();
    assert_eq!(g.capacity(0,&ItemKey::plain("minecraft:ender_pearl",16)),512);
    assert_eq!(g.capacity(0,&ItemKey::plain("minecraft:diamond_sword",1)),32);
}
#[test]fn component_distinct_items_do_not_merge(){
    let mut g=Group::standard(1,false).unwrap();let mut named=stone();named.components="{name:\"A\"}".into();
    g.insert(0,&Stack{item:named,count:3},Access::Automation,false).unwrap();
    assert_eq!(g.insert(0,&stack(2),Access::Automation,false),Err(Error::Incompatible));assert_eq!(g.count(0),3);
}
#[test]fn only_overflow_is_voided_and_simulation_never_mutates(){
    let mut g=Group::standard(1,false).unwrap();g.add_upgrade(Upgrade::Void,Access::Administrator).unwrap();let original=g.clone();
    assert_eq!(g.insert(0,&stack(2100),Access::Automation,true).unwrap(),Inserted{stored:2048,voided:52});assert_eq!(g,original);
    let out=g.insert(0,&stack(2100),Access::Automation,false).unwrap();assert_eq!(out.accepted(),2100);assert_eq!(g.count(0),2048);
}
#[test]fn incompatible_items_are_never_voided(){
    let mut g=Group::standard(1,false).unwrap();g.add_upgrade(Upgrade::Void,Access::Administrator).unwrap();
    g.insert(0,&stack(1),Access::Automation,false).unwrap();
    assert_eq!(g.insert(0,&Stack{item:ItemKey::plain("minecraft:dirt",64),count:90},Access::Automation,false),Err(Error::Incompatible));
}
#[test]fn lock_retains_empty_prototype_unlock_releases_it(){
    let mut g=Group::standard(1,false).unwrap();g.insert(0,&stack(1),Access::Automation,false).unwrap();g.use_key(Key::Lock,7).unwrap();
    g.extract(0,1,Access::Automation,false).unwrap();assert!(g.item(0).is_some());g.use_key(Key::Lock,7).unwrap();assert!(g.item(0).is_none());
}
#[test]fn extracting_empty_and_zero_items_is_a_noop(){
    let mut g=Group::standard(1,false).unwrap();assert_eq!(g.extract(0,100,Access::Automation,false).unwrap(),None);
    assert_eq!(g.insert(0,&stack(0),Access::Automation,false).unwrap().accepted(),0);assert!(g.item(0).is_none());
}
#[test]fn compacting_views_share_units_without_rounding_loss(){
    let mut g=compact();let nugget=g.item(2).unwrap().clone();
    g.insert(2,&Stack{item:nugget,count:91},Access::Automation,false).unwrap();
    assert_eq!((g.count(0),g.count(1),g.count(2)),(1,10,91));
    assert_eq!(g.extract(0,1,Access::Automation,false).unwrap().unwrap().count,1);
    assert_eq!((g.count(0),g.count(1),g.count(2)),(0,1,10));
    g.extract(1,1,Access::Automation,false).unwrap();assert_eq!(g.count(2),1);
}
#[test]fn repeated_compacting_extracts_never_duplicate_items(){
    let mut g=compact();let item=g.item(2).unwrap().clone();g.insert(2,&Stack{item,count:81},Access::Automation,false).unwrap();
    g.extract(0,1,Access::Automation,false).unwrap();assert!(g.extract(1,1,Access::Automation,false).unwrap().is_none());assert!(g.extract(2,1,Access::Automation,false).unwrap().is_none());
}
#[test]fn compacting_recipe_must_be_reversible_and_unambiguous(){
    assert_eq!(Compacting::new(vec![(stone(),9),(stone(),1)]),Err(Error::InvalidRecipe));
    assert!(Compacting::new(vec![(stone(),9),(ItemKey::plain("minecraft:dirt",64),2)]).is_err());
}
#[test]fn removal_cannot_delete_stock_or_uninstall_creative_capacity_in_use(){
    let mut g=Group::standard(1,false).unwrap();g.add_upgrade(Upgrade::Iron,Access::Administrator).unwrap();
    g.insert(0,&stack(3000),Access::Automation,false).unwrap();let old=g.clone();
    assert_eq!(g.remove_upgrade(0,Access::Administrator),Err(Error::CapacityInUse));assert_eq!(g,old);
    assert_eq!(g.add_upgrade(Upgrade::OneStack,Access::Administrator),Err(Error::CapacityInUse));
}
#[test]fn vending_is_infinite_but_requires_a_prototype(){
    let mut g=Group::standard(1,false).unwrap();g.add_upgrade(Upgrade::CreativeVending,Access::Administrator).unwrap();
    assert!(g.extract(0,64,Access::Automation,false).unwrap().is_none());g.insert(0,&stack(1),Access::Automation,false).unwrap();
    for _ in 0..100{assert_eq!(g.extract(0,64,Access::Automation,false).unwrap().unwrap().count,64);}assert_eq!(g.count(0),MAX_COUNT);
}
#[test]fn java_redstone_midpoint_is_seven_and_almost_full_is_fourteen(){
    let mut g=Group::standard(1,false).unwrap();g.add_upgrade(Upgrade::Redstone,Access::Administrator).unwrap();
    g.insert(0,&stack(1024),Access::Automation,false).unwrap();assert_eq!(g.redstone(),7);
    g.insert(0,&stack(1023),Access::Automation,false).unwrap();assert_eq!(g.redstone(),14);
}
#[test]fn combined_signal_averages_slots_with_different_stack_sizes(){
    let mut g=Group::standard(2,false).unwrap();g.add_upgrade(Upgrade::Redstone,Access::Administrator).unwrap();
    g.insert(0,&Stack{item:ItemKey::plain("minecraft:diamond_sword",1),count:16},Access::Automation,false).unwrap();
    g.insert(1,&stack(0),Access::Automation,false).unwrap();assert_eq!(g.redstone(),7);
}
#[test]fn compacting_pooled_count_stays_in_java_integer_range(){
    let mut g=compact();for _ in 0..7{g.add_upgrade(Upgrade::Netherite,Access::Administrator).unwrap();}
    let nugget=g.item(2).unwrap().clone();assert_eq!(g.capacity(2,&nugget),MAX_COUNT);
    g.insert(2,&Stack{item:nugget,count:u64::MAX},Access::Automation,false).unwrap();
    assert_eq!(g.compact.as_ref().unwrap().units,MAX_COUNT/81*81);
}
#[test]fn personal_key_toggles_ownership_for_its_owner(){
    let mut g=Group::standard(1,false).unwrap();g.use_key(Key::Personal,10).unwrap();assert_eq!(g.attributes.owner,Some(10));
    g.use_key(Key::Personal,10).unwrap();assert_eq!(g.attributes.owner,None);
}
#[test]fn compacting_supports_an_item_without_a_recipe(){
    let mut g=Group::compacting(vec![(stone(),1)],false).unwrap();g.insert(0,&stack(64),Access::Automation,false).unwrap();
    assert_eq!(codec::decode(&codec::encode(&g).unwrap()).unwrap(),g);
}
#[test]fn ownership_and_suspend_do_not_grant_players_administrator_access(){
    let mut g=Group::standard(1,false).unwrap();g.use_key(Key::Personal,10).unwrap();
    assert_eq!(g.use_key(Key::UnlockPersonal,20),Err(Error::Denied));assert_eq!(g.insert(0,&stack(1),Access::Player(20),false),Err(Error::Denied));
    g.use_key(Key::Suspend,10).unwrap();assert_eq!(g.insert(0,&stack(1),Access::Automation,false),Err(Error::Suspended));
    assert!(g.insert(0,&stack(1),Access::Player(10),false).is_ok());
}
#[test]fn detached_drawers_are_not_copied_or_truncated(){
    let mut g=Group::standard(1,false).unwrap();g.insert(0,&stack(2000),Access::Automation,false).unwrap();
    let token=g.detach(0,Access::Player(1)).unwrap();assert_eq!(g.count(0),0);assert!(g.detach(0,Access::Player(1)).is_err());
    let mut small=Group::standard(4,false).unwrap();assert_eq!(small.attach(0,&token,Access::Player(1)),Err(Error::CapacityInUse));
    g.attach(0,&token,Access::Player(1)).unwrap();assert_eq!(g.count(0),2000);
}
#[test]fn balancing_conserves_exact_count(){
    let mut g=Group::standard(4,false).unwrap();for s in 0..4{g.insert(s,&stack(1),Access::Automation,false).unwrap();}
    g.insert(0,&stack(99),Access::Automation,false).unwrap();g.balance();
    assert_eq!((0..4).map(|s|g.count(s)).collect::<Vec<_>>(),vec![26,26,26,25]);
}
#[test]fn redstone_has_zero_empty_and_fifteen_full(){
    let mut g=Group::standard(1,false).unwrap();g.add_upgrade(Upgrade::Redstone,Access::Administrator).unwrap();assert_eq!(g.redstone(),0);
    g.insert(0,&stack(1),Access::Automation,false).unwrap();assert_eq!(g.redstone(),1);
    g.insert(0,&stack(2047),Access::Automation,false).unwrap();assert_eq!(g.redstone(),15);
}
fn pos(x:i32)->Position{Position::new("minecraft:overworld",[x,64,0])}
fn network()->Network{
    let mut n=Network::default();n.nodes.insert(pos(0),Node::Controller);n.nodes.insert(pos(1),Node::Trim);
    for x in [2,3]{let mut g=Group::standard(1,false).unwrap();g.insert(0,&stack(1),Access::Automation,false).unwrap();n.nodes.insert(pos(x),Node::Drawers(g));}n
}
#[test]fn controller_uses_connected_matching_drawers_in_priority_order(){
    let mut n=network();if let Some(Node::Drawers(g))=n.nodes.get_mut(&pos(3)){g.attributes.priority=2;}
    let added=n.insert(&pos(0),&stack(2048),Access::Automation,false);assert_eq!(added.stored,2048);
    let Some(Node::Drawers(a))=n.nodes.get(&pos(2))else{panic!()};let Some(Node::Drawers(b))=n.nodes.get(&pos(3))else{panic!()};
    assert_eq!((a.count(0),b.count(0)),(2,2048));
}
#[test]fn disconnected_and_other_dimension_nodes_are_not_reached(){
    let mut n=network();n.nodes.remove(&pos(1));assert_eq!(n.insert(&pos(0),&stack(100),Access::Automation,false).accepted(),0);
    n.nodes.insert(Position::new("minecraft:the_nether",[1,64,0]),Node::Trim);assert_eq!(n.connected(&pos(0),50).len(),1);
}
#[test]fn remote_bindings_require_an_upgrade_and_loaded_target(){
    let mut n=network();let mut g=Group::standard(1,false).unwrap();g.add_upgrade(Upgrade::Remote,Access::Administrator).unwrap();
    g.insert(0,&stack(1),Access::Automation,false).unwrap();n.nodes.insert(pos(10),Node::Drawers(g));
    n.remotes.push(RemoteBinding{controller:pos(0),drawer:pos(10),group:false});assert!(n.connected(&pos(0),50).contains(&pos(10)));
    n.nodes.remove(&pos(10));assert!(!n.connected(&pos(0),50).contains(&pos(10)));
}
#[test]fn network_transfer_to_full_destination_keeps_source_intact(){
    let mut n=network();for x in [2,3]{if let Some(Node::Drawers(g))=n.nodes.get_mut(&pos(x)){g.insert(0,&stack(2047),Access::Automation,false).unwrap();}}
    let original=n.clone();assert_eq!(n.transfer(&pos(2),0,&pos(0),64),0);assert_eq!(n,original);
}
#[test]fn saved_state_roundtrips_components_owner_upgrades_and_shared_units(){
    let mut g=compact();g.add_upgrade(Upgrade::Netherite,Access::Administrator).unwrap();g.use_key(Key::Lock,1).unwrap();
    g.use_key(Key::Personal,123).unwrap();g.frame=Some(Frame{side:"minecraft:stone".into(),trim:"minecraft:gold_block".into(),front:"minecraft:glass".into()});
    let k=g.item(2).unwrap().clone();g.insert(2,&Stack{item:k,count:99999},Access::Automation,false).unwrap();
    assert_eq!(codec::decode(&codec::encode(&g).unwrap()).unwrap(),g);
}
#[test]fn corrupt_or_truncated_save_is_rejected_without_resetting_inventory(){
    let mut g=Group::standard(1,false).unwrap();g.insert(0,&stack(1000),Access::Automation,false).unwrap();let mut saved=codec::encode(&g).unwrap();
    for len in [0,3,8,20,saved.len()-1]{assert!(codec::decode(&saved[..len]).is_err());}
    saved[20]^=1;assert_eq!(codec::decode(&saved),Err(Error::InvalidData));
}
#[test]fn deterministic_mixed_transfers_conserve_all_base_units(){
    let mut g=compact();let mut seed=17u64;let mut inserted=0u64;let mut extracted=0u64;
    for _ in 0..10_000{
        seed=seed.wrapping_mul(6364136223846793005).wrapping_add(1);let slot=(seed%3)as usize;let factor=[81,9,1][slot];
        let amount=(seed>>8)%100+1;
        if seed&8==0{let item=g.item(slot).unwrap().clone();let out=g.insert(slot,&Stack{item,count:amount},Access::Automation,false).unwrap();inserted+=out.stored*factor;}
        else if let Some(s)=g.extract(slot,amount,Access::Automation,false).unwrap(){extracted+=s.count*factor;}
        assert_eq!(g.compact.as_ref().unwrap().units,inserted-extracted);
    }
}
