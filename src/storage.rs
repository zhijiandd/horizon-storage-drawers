use std::collections::BTreeSet;

pub const MAX_COUNT: u64 = i32::MAX as u64;
pub const UPGRADE_SLOTS: usize = 7;

/// `components` is a canonical encoding of the COMPLETE component patch, not a display name.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ItemKey {
    pub id: String,
    pub components: String,
    pub max_stack: u32,
}
impl ItemKey {
    pub fn plain(id: &str, max_stack: u32) -> Self {
        Self { id: id.into(), components: String::new(), max_stack: max_stack.clamp(1, 99) }
    }
    pub fn same_item(&self, other: &Self) -> bool {
        self.id == other.id && self.components == other.components
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stack { pub item: ItemKey, pub count: u64 }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Access { Player(u128), Automation, Administrator }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Upgrade {
    Obsidian, Copper, Iron, Gold, Emerald, Diamond, Netherite,
    OneStack, Void, CreativeStorage, CreativeVending, Conversion,
    Redstone, MinRedstone, MaxRedstone, Illumination, FillLevel,
    Balance, Portability, Hopper, Magnet1, Magnet2, Magnet3,
    Remote, RemoteGroup,
}
impl Upgrade {
    pub const ALL: [Self; 25] = [Self::Obsidian, Self::Copper, Self::Iron, Self::Gold,
        Self::Emerald, Self::Diamond, Self::Netherite, Self::OneStack, Self::Void,
        Self::CreativeStorage, Self::CreativeVending, Self::Conversion, Self::Redstone,
        Self::MinRedstone, Self::MaxRedstone, Self::Illumination, Self::FillLevel,
        Self::Balance, Self::Portability, Self::Hopper, Self::Magnet1, Self::Magnet2,
        Self::Magnet3, Self::Remote, Self::RemoteGroup];
    pub fn multiplier(self) -> Option<u64> {
        Some(match self { Self::Obsidian=>2, Self::Copper=>4, Self::Iron=>8,
            Self::Gold=>16, Self::Emerald=>64, Self::Diamond=>256, Self::Netherite=>2048,
            _=>return None })
    }
    pub fn id(self) -> &'static str {
        match self { Self::Obsidian=>"obsidian_storage_upgrade", Self::Copper=>"copper_storage_upgrade",
            Self::Iron=>"iron_storage_upgrade", Self::Gold=>"gold_storage_upgrade",
            Self::Emerald=>"emerald_storage_upgrade", Self::Diamond=>"diamond_storage_upgrade",
            Self::Netherite=>"netherite_storage_upgrade", Self::OneStack=>"one_stack_upgrade",
            Self::Void=>"void_upgrade", Self::CreativeStorage=>"creative_storage_upgrade",
            Self::CreativeVending=>"creative_vending_upgrade", Self::Conversion=>"conversion_upgrade",
            Self::Redstone=>"redstone_upgrade", Self::MinRedstone=>"min_redstone_upgrade",
            Self::MaxRedstone=>"max_redstone_upgrade", Self::Illumination=>"illumination_upgrade",
            Self::FillLevel=>"fill_level_upgrade", Self::Balance=>"balanced_fill_upgrade",
            Self::Portability=>"portability_upgrade", Self::Hopper=>"hopper_upgrade",
            Self::Magnet1=>"magnet_upgrade", Self::Magnet2=>"magnet_upgrade_2",
            Self::Magnet3=>"magnet_upgrade_3", Self::Remote=>"remote_upgrade",
            Self::RemoteGroup=>"remote_group_upgrade" }
    }
    pub fn from_id(id: &str) -> Option<Self> {
        let id = id.strip_prefix("storagedrawers:").unwrap_or(id);
        let id=match id{"balance_fill_upgrade"=>"balanced_fill_upgrade","remote_upgrade_bound"=>"remote_upgrade",
            "remote_group_upgrade_bound"=>"remote_group_upgrade",v=>v};
        Self::ALL.into_iter().find(|u| u.id()==id)
    }
    pub(crate) fn exclusive(self, other: Self) -> bool {
        if self.multiplier().is_some() || matches!(self,Self::Magnet1|Self::Magnet2|Self::Magnet3) { return false; }
        self==other || matches!((self,other),
            (Self::Redstone|Self::MinRedstone|Self::MaxRedstone, Self::Redstone|Self::MinRedstone|Self::MaxRedstone)
            | (Self::CreativeStorage|Self::CreativeVending, Self::CreativeStorage|Self::CreativeVending)
            | (Self::Remote|Self::RemoteGroup, Self::Remote|Self::RemoteGroup))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Attributes {
    pub locked: bool, pub quantified: bool, pub shrouded: bool, pub suspended: bool,
    pub priority: i8, pub owner: Option<u128>, pub trusted: BTreeSet<u128>,
}
impl Attributes {
    pub fn permits(&self, access: Access) -> bool {
        match access {
            Access::Administrator=>true,
            Access::Automation=>!self.suspended,
            Access::Player(id)=>self.owner.is_none_or(|o| o==id || self.trusted.contains(&id)),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key { Lock, Quantify, Shroud, Suspend, Priority, Personal, UnlockPersonal }
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Keyring { pub keys: Vec<Key>, pub selected: usize }
impl Keyring {
    pub fn add(&mut self, key: Key) -> bool {
        if self.keys.contains(&key) {return false;} self.keys.push(key); true
    }
    pub fn cycle(&mut self, backwards: bool) -> Option<Key> {
        if self.keys.is_empty(){return None;}
        let n=self.keys.len(); self.selected=(self.selected+n+if backwards{n-1}else{1})%n;
        self.keys.get(self.selected).copied()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Drawer { pub item: Option<ItemKey>, pub count: u64, pub detached: bool }

/// The first view is the most compact item. Factors convert one of that view to base units.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Compacting { pub views: Vec<(ItemKey,u64)>, pub units: u64 }
impl Compacting {
    pub fn new(views: Vec<(ItemKey,u64)>) -> Result<Self, Error> {
        if !(1..=3).contains(&views.len()) || views.last().is_none_or(|v| v.1!=1)
            || views.iter().any(|v| v.1==0 || v.1>MAX_COUNT)
            || views.windows(2).any(|v| v[0].1<=v[1].1 || v[0].1%v[1].1!=0)
            || views.iter().enumerate().any(|(i,v)| views[..i].iter().any(|w| w.0.same_item(&v.0))) {
            return Err(Error::InvalidRecipe);
        }
        Ok(Self { views, units:0 })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame { pub side: String, pub trim: String, pub front: String }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Group {
    pub drawers: Vec<Drawer>, pub half: bool, pub compact: Option<Compacting>,
    pub upgrades: Vec<Upgrade>, pub attributes: Attributes, pub frame: Option<Frame>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error { Denied, Suspended, Slot, Detached, Incompatible, Full, InvalidRecipe,
    UpgradeSlots, DuplicateUpgrade, CapacityInUse, InvalidData, UnsupportedVersion, TooLarge }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Inserted { pub stored: u64, pub voided: u64 }
impl Inserted { pub fn accepted(self)->u64 {self.stored+self.voided} }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DetachedDrawer { pub drawer: Drawer, pub capacity: u64 }

impl Group {
    pub fn standard(slots: usize, half: bool) -> Result<Self, Error> {
        if ![1,2,4].contains(&slots){return Err(Error::Slot);}
        Ok(Self {drawers:vec![Drawer::default();slots],half,compact:None,upgrades:Vec::new(),
            attributes:Attributes::default(),frame:None})
    }
    pub fn compacting(views: Vec<(ItemKey,u64)>, half:bool)->Result<Self,Error>{
        let compact=Compacting::new(views)?;
        Ok(Self {drawers:vec![Drawer::default();compact.views.len()], half, compact:Some(compact),
            upgrades:Vec::new(), attributes:Attributes::default(), frame:None})
    }
    pub fn has(&self,u:Upgrade)->bool {self.upgrades.contains(&u)}
    pub fn multiplier(&self)->u64 {
        self.upgrades.iter().filter_map(|u|u.multiplier()).sum::<u64>().max(1)
    }
    pub fn base_stacks(&self)->u64 {
        let total=if self.half{16}else{32};
        if self.compact.is_some(){total}else{total/self.drawers.len() as u64}
    }
    pub fn item(&self,slot:usize)->Option<&ItemKey>{
        if self.drawers.get(slot)?.detached{return None;}
        if let Some(c)=&self.compact {Some(&c.views.get(slot)?.0)}else{self.drawers[slot].item.as_ref()}
    }
    pub fn count(&self,slot:usize)->u64 {
        if self.drawers.get(slot).is_none_or(|d|d.detached){return 0;}
        if self.has(Upgrade::CreativeVending)&&self.item(slot).is_some(){return MAX_COUNT;}
        if let Some(c)=&self.compact {c.units/c.views[slot].1}else{self.drawers[slot].count}
    }
    pub fn capacity(&self,slot:usize,item:&ItemKey)->u64{
        if slot>=self.drawers.len(){return 0;}
        if self.has(Upgrade::CreativeStorage)||self.has(Upgrade::CreativeVending){
            return self.compact.as_ref().map_or(MAX_COUNT,|c|MAX_COUNT/c.views[slot].1);
        }
        let stacks=if self.has(Upgrade::OneStack){self.multiplier()}else{self.base_stacks().saturating_mul(self.multiplier())};
        let cap=stacks.saturating_mul(u64::from(self.compact.as_ref().map_or(item.max_stack,|c|c.views[0].0.max_stack)));
        if let Some(c)=&self.compact {
            cap.saturating_mul(c.views[0].1).min(MAX_COUNT)/c.views[slot].1
        }else{cap.min(MAX_COUNT)}
    }
    fn compact_unit_capacity(&self)->u64{
        let Some(c)=&self.compact else{return 0;};
        self.capacity(0,&c.views[0].0).saturating_mul(c.views[0].1)
    }
    pub fn check(&self,access:Access)->Result<(),Error>{
        if !self.attributes.permits(access){
            return Err(if access==Access::Automation && self.attributes.suspended{Error::Suspended}else{Error::Denied});
        } Ok(())
    }
    fn match_slot(&self,slot:usize,item:&ItemKey,conversion:&impl Fn(&ItemKey,&ItemKey)->bool)->Result<(),Error>{
        let d=self.drawers.get(slot).ok_or(Error::Slot)?;
        if d.detached{return Err(Error::Detached);}
        if let Some(existing)=self.item(slot) {
            if !existing.same_item(item) && !(self.has(Upgrade::Conversion)&&conversion(existing,item)) {
                return Err(Error::Incompatible);
            }
        } Ok(())
    }
    pub fn insert(&mut self,slot:usize,stack:&Stack,access:Access,simulate:bool)->Result<Inserted,Error>{
        self.insert_with_conversion(slot,stack,access,simulate,&|_,_|false)
    }
    pub fn insert_with_conversion(&mut self,slot:usize,stack:&Stack,access:Access,simulate:bool,
        conversion:&impl Fn(&ItemKey,&ItemKey)->bool)->Result<Inserted,Error>{
        self.check(access)?; self.match_slot(slot,&stack.item,conversion)?;
        if stack.count==0{return Ok(Inserted::default());}
        let cap=self.capacity(slot,self.item(slot).unwrap_or(&stack.item));
        let room=if let Some(c)=&self.compact {
            self.compact_unit_capacity().saturating_sub(c.units)/c.views[slot].1
        }else{cap.saturating_sub(self.count(slot))};
        let stored=stack.count.min(room);
        let voided=if self.has(Upgrade::Void){stack.count-stored}else{0};
        if !simulate && stored>0{
            if let Some(c)=&mut self.compact {c.units+=stored*c.views[slot].1;}
            else{let d=&mut self.drawers[slot]; if d.item.is_none(){d.item=Some(stack.item.clone());} d.count+=stored;}
        }
        if !simulate && self.has(Upgrade::Balance){self.balance();}
        Ok(Inserted{stored,voided})
    }
    pub fn extract(&mut self,slot:usize,count:u64,access:Access,simulate:bool)->Result<Option<Stack>,Error>{
        self.check(access)?;
        let d=self.drawers.get(slot).ok_or(Error::Slot)?;
        if d.detached{return Err(Error::Detached);}
        let Some(item)=self.item(slot).cloned() else{return Ok(None);};
        let vending=self.has(Upgrade::CreativeVending);
        let taken=count.min(if vending {MAX_COUNT}else{self.count(slot)});
        if taken==0{return Ok(None);}
        if !simulate&&!vending{
            if let Some(c)=&mut self.compact{c.units-=taken*c.views[slot].1;}
            else{self.drawers[slot].count-=taken; self.clear_empty(slot);}
            if self.has(Upgrade::Balance){self.balance();}
        }
        Ok(Some(Stack{item,count:taken}))
    }
    fn clear_empty(&mut self,slot:usize){
        if self.drawers[slot].count==0&&!self.attributes.locked{self.drawers[slot].item=None;}
    }
    pub fn add_upgrade(&mut self,u:Upgrade,access:Access)->Result<(),Error>{
        self.check(access)?;
        if self.upgrades.len()>=UPGRADE_SLOTS{return Err(Error::UpgradeSlots);}
        if self.upgrades.iter().any(|&v|u.exclusive(v)){return Err(Error::DuplicateUpgrade);}
        let mut changed=self.clone(); changed.upgrades.push(u);
        if !changed.within_capacity(){return Err(Error::CapacityInUse);}
        *self=changed; Ok(())
    }
    /// Shared SlotUpgrade admission, used before both client prediction and
    /// authoritative inventory changes. Component equality is checked by the caller.
    pub fn can_place_upgrade(&self,index:Option<usize>,u:Upgrade,count:u32,creative:bool,access:Access)->bool{
        if count==0{return false;}
        let mut changed=self.clone();
        if let Some(index)=index{
            let Some(&old)=self.upgrades.get(index)else{return false;};
            if count!=1||!creative&&matches!(old,Upgrade::CreativeStorage|Upgrade::CreativeVending)
                ||u.multiplier().is_none()||old.multiplier().is_none()&&old!=Upgrade::OneStack{return false;}
            if changed.remove_upgrade(index,access).is_err(){return false;}
        }
        changed.add_upgrade(u,access).is_ok()
    }
    pub fn remove_upgrade(&mut self,slot:usize,access:Access)->Result<Upgrade,Error>{
        self.check(access)?;
        let u=*self.upgrades.get(slot).ok_or(Error::Slot)?;
        let mut changed=self.clone(); changed.upgrades.remove(slot);
        if !changed.within_capacity(){return Err(Error::CapacityInUse);}
        *self=changed; Ok(u)
    }
    pub fn within_capacity(&self)->bool{
        if let Some(c)=&self.compact{return c.units<=self.compact_unit_capacity();}
        self.drawers.iter().enumerate().all(|(s,d)|d.item.as_ref().is_none_or(|i|d.count<=self.capacity(s,i)))
    }
    pub fn use_key(&mut self,key:Key,actor:u128)->Result<(),Error>{
        self.check(Access::Player(actor))?;
        match key{
            Key::Lock=>{self.attributes.locked=!self.attributes.locked;if !self.attributes.locked{
                for s in 0..self.drawers.len(){self.clear_empty(s);}}},
            Key::Quantify=>self.attributes.quantified=!self.attributes.quantified,
            Key::Shroud=>self.attributes.shrouded=!self.attributes.shrouded,
            Key::Suspend=>self.attributes.suspended=!self.attributes.suspended,
            Key::Priority=>self.attributes.priority=match self.attributes.priority{0=>1,1=>2,2=>-1,-1=>-2,_=>0},
            Key::Personal=>self.attributes.owner=if self.attributes.owner==Some(actor){None}else{Some(actor)},
            Key::UnlockPersonal=>{self.attributes.owner=None;self.attributes.trusted.clear();},
        } Ok(())
    }
    pub fn balance(&mut self){
        if self.compact.is_some(){return;}
        let mut done=BTreeSet::new();
        for i in 0..self.drawers.len(){
            if done.contains(&i)||self.drawers[i].detached{continue;}
            let Some(item)=self.drawers[i].item.clone()else{continue;};
            let slots:Vec<_>=(i..self.drawers.len()).filter(|&s|!self.drawers[s].detached&&
                self.drawers[s].item.as_ref().is_some_and(|k|k.same_item(&item))).collect();
            let total:u64=slots.iter().map(|&s|self.drawers[s].count).sum();
            for (idx,&s) in slots.iter().enumerate(){self.drawers[s].count=total/slots.len() as u64+
                u64::from((idx as u64)<total%slots.len() as u64);done.insert(s);}
        }
    }
    pub fn redstone(&self)->u8{
        if ![Upgrade::Redstone,Upgrade::MinRedstone,Upgrade::MaxRedstone].iter().any(|&u|self.has(u)){return 0;}
        let fractions:Vec<f64>=(0..self.drawers.len()).filter(|&s|!self.drawers[s].detached).map(|s|{
            self.item(s).map_or(0.,|i|{let cap=self.capacity(s,i);if cap==0{0.}else{self.count(s)as f64/cap as f64}})
        }).collect();
        if fractions.is_empty(){return 0;}
        let signal=|ratio:f64|->u8{if ratio>=1.{15}else{(ratio*14.).ceil()as u8}};
        if self.has(Upgrade::MinRedstone){
            return signal(fractions.into_iter().fold(1.,f64::min));
        }
        if self.has(Upgrade::MaxRedstone){return signal(fractions.into_iter().fold(0.,f64::max));}
        signal(fractions.iter().sum::<f64>()/fractions.len()as f64)
    }
    pub fn detach(&mut self,slot:usize,access:Access)->Result<DetachedDrawer,Error>{
        self.check(access)?;
        if self.compact.is_some(){return Err(Error::Incompatible);}
        let d=self.drawers.get(slot).ok_or(Error::Slot)?;
        if d.detached{return Err(Error::Detached);}
        let cap=d.item.as_ref().map_or(0,|i|self.capacity(slot,i));
        let drawer=std::mem::replace(&mut self.drawers[slot],Drawer{detached:true,..Drawer::default()});
        Ok(DetachedDrawer{drawer,capacity:cap})
    }
    pub fn attach(&mut self,slot:usize,drawer:&DetachedDrawer,access:Access)->Result<(),Error>{
        self.check(access)?;
        if self.compact.is_some(){return Err(Error::Incompatible);}
        let old=self.drawers.get(slot).ok_or(Error::Slot)?;
        if !old.detached{return Err(Error::Incompatible);}
        if let Some(i)=&drawer.drawer.item{
            let cap=self.capacity(slot,i);
            if drawer.drawer.count>cap{return Err(Error::CapacityInUse);}
        }
        self.drawers[slot]=drawer.drawer.clone();self.drawers[slot].detached=false;Ok(())
    }
    pub fn magnet_radius(&self)->u32{
        self.magnet_range()[0]
    }
    pub fn magnet_range(&self)->[u32;3]{
        let mut range=[0,0,0];
        for upgrade in &self.upgrades{let r=match upgrade{Upgrade::Magnet1=>[1,1,0],Upgrade::Magnet2=>[4,2,0],Upgrade::Magnet3=>[8,3,0],_=>[0,0,0]};
            for i in 0..3{range[i]+=r[i];}}
        [range[0].min(24),range[1].min(8),range[2].min(0)]
    }
    pub fn magnet_interval(&self)->u64{
        if self.has(Upgrade::Magnet3){5}else if self.has(Upgrade::Magnet2){10}else{20}
    }
}

