//! Bounded, versioned, lossless saved inventory. Invalid saves never become empty drawers.
use crate::storage::*;
use std::collections::BTreeSet;

const MAGIC:&[u8;8]=b"SDRH\x01\0\0\0";
const LIMIT:usize=4*1024*1024;
fn checksum(bytes:&[u8])->u64{
    bytes.iter().fold(0xcbf29ce484222325u64,|h,b|(h^u64::from(*b)).wrapping_mul(0x100000001b3))
}
struct Writer(Vec<u8>);
impl Writer{
    fn byte(&mut self,n:u8){self.0.push(n);}
    fn number(&mut self,n:u64){self.0.extend(n.to_le_bytes());}
    fn text(&mut self,s:&str){self.number(s.len() as u64);self.0.extend(s.as_bytes());}
    fn owner(&mut self,o:Option<u128>){self.byte(u8::from(o.is_some()));if let Some(o)=o{self.0.extend(o.to_le_bytes());}}
    fn item(&mut self,i:&ItemKey){self.text(&i.id);self.text(&i.components);self.number(u64::from(i.max_stack));}
}
struct Reader<'a>{bytes:&'a[u8],at:usize}
impl<'a> Reader<'a>{
    fn take(&mut self,n:usize)->Result<&'a[u8],Error>{
        let end=self.at.checked_add(n).filter(|&e|e<=self.bytes.len()).ok_or(Error::InvalidData)?;
        let out=&self.bytes[self.at..end];self.at=end;Ok(out)
    }
    fn byte(&mut self)->Result<u8,Error>{Ok(self.take(1)?[0])}
    fn flag(&mut self)->Result<bool,Error>{match self.byte()?{0=>Ok(false),1=>Ok(true),_=>Err(Error::InvalidData)}}
    fn number(&mut self)->Result<u64,Error>{Ok(u64::from_le_bytes(self.take(8)?.try_into().map_err(|_|Error::InvalidData)?))}
    fn length(&mut self,max:usize)->Result<usize,Error>{
        let n=usize::try_from(self.number()?).map_err(|_|Error::TooLarge)?;
        if n>max{return Err(Error::TooLarge);}Ok(n)
    }
    fn text(&mut self)->Result<String,Error>{let n=self.length(1024*1024)?;String::from_utf8(self.take(n)?.to_vec()).map_err(|_|Error::InvalidData)}
    fn owner(&mut self)->Result<Option<u128>,Error>{
        if !self.flag()?{return Ok(None);}Ok(Some(u128::from_le_bytes(self.take(16)?.try_into().map_err(|_|Error::InvalidData)?)))
    }
    fn item(&mut self)->Result<ItemKey,Error>{
        let id=self.text()?;let components=self.text()?;let max_stack=self.number()?;
        if !(1..=99).contains(&max_stack)||!valid_id(&id){return Err(Error::InvalidData);}
        Ok(ItemKey{id,components,max_stack:max_stack as u32})
    }
}
fn valid_id(id:&str)->bool{
    id.len()<=512&&id.split_once(':').is_some_and(|(n,p)|!n.is_empty()&&!p.is_empty()&&
        n.bytes().all(|b|b.is_ascii_lowercase()||b.is_ascii_digit()||b"_.-".contains(&b))&&
        p.bytes().all(|b|b.is_ascii_lowercase()||b.is_ascii_digit()||b"_./-".contains(&b)))
}
pub fn encode(g:&Group)->Result<Vec<u8>,Error>{
    let mut w=Writer(MAGIC.to_vec());
    w.byte(g.drawers.len() as u8);w.byte(u8::from(g.half));
    for d in &g.drawers{
        w.byte(u8::from(d.detached));w.byte(u8::from(d.item.is_some()));
        if let Some(i)=&d.item{w.item(i);}w.number(d.count);
    }
    w.byte(u8::from(g.compact.is_some()));
    if let Some(c)=&g.compact{w.byte(c.views.len() as u8);for (i,f) in &c.views{w.item(i);w.number(*f);}w.number(c.units);}
    w.byte(g.upgrades.len() as u8);for u in &g.upgrades{w.text(u.id());}
    let a=&g.attributes;
    for v in [a.locked,a.quantified,a.shrouded,a.suspended]{w.byte(u8::from(v));}
    w.byte((a.priority+2) as u8);w.owner(a.owner);
    w.number(a.trusted.len() as u64);for t in &a.trusted{w.0.extend(t.to_le_bytes());}
    w.byte(u8::from(g.frame.is_some()));
    if let Some(f)=&g.frame{w.text(&f.side);w.text(&f.trim);w.text(&f.front);}
    if w.0.len()>LIMIT{return Err(Error::TooLarge);}
    w.number(checksum(&w.0));Ok(w.0)
}
pub fn decode(bytes:&[u8])->Result<Group,Error>{
    if bytes.len()>LIMIT+8{return Err(Error::TooLarge);}
    if bytes.len()<16{return Err(Error::InvalidData);}
    if &bytes[..4]!=&MAGIC[..4]{return Err(Error::InvalidData);}
    if &bytes[..8]!=MAGIC{return Err(Error::UnsupportedVersion);}
    let end=bytes.len()-8;
    if checksum(&bytes[..end])!=u64::from_le_bytes(bytes[end..].try_into().map_err(|_|Error::InvalidData)?){return Err(Error::InvalidData);}
    let mut r=Reader{bytes:&bytes[..end],at:8};
    let slots=usize::from(r.byte()?);if ![1,2,3,4].contains(&slots){return Err(Error::Slot);}
    let half=r.flag()?;let mut drawers=Vec::new();
    for _ in 0..slots{
        let detached=r.flag()?;let item=if r.flag()?{Some(r.item()?)}else{None};let count=r.number()?;
        if count>MAX_COUNT||(count>0&&item.is_none())||(detached&&(count>0||item.is_some())){return Err(Error::InvalidData);}
        drawers.push(Drawer{item,count,detached});
    }
    let compact=if r.flag()?{
        let n=usize::from(r.byte()?);if n!=slots{return Err(Error::InvalidRecipe);}
        let mut views=Vec::new();for _ in 0..n{views.push((r.item()?,r.number()?));}
        let mut c=Compacting::new(views)?;c.units=r.number()?;Some(c)
    }else{if slots==3{return Err(Error::Slot);}None};
    let mut g=Group{drawers,half,compact,upgrades:Vec::new(),attributes:Attributes::default(),frame:None};
    let n=usize::from(r.byte()?);if n>UPGRADE_SLOTS{return Err(Error::UpgradeSlots);}
    for _ in 0..n{
        let u=Upgrade::from_id(&r.text()?).ok_or(Error::InvalidData)?;
        if g.upgrades.iter().any(|&v|u.exclusive(v)){return Err(Error::DuplicateUpgrade);}
        g.upgrades.push(u);
    }
    let a=&mut g.attributes;
    a.locked=r.flag()?;a.quantified=r.flag()?;a.shrouded=r.flag()?;a.suspended=r.flag()?;
    let priority=r.byte()?;if priority>4{return Err(Error::InvalidData);}a.priority=priority as i8-2;
    a.owner=r.owner()?;let n=r.length(4096)?;a.trusted=BTreeSet::new();
    for _ in 0..n{a.trusted.insert(u128::from_le_bytes(r.take(16)?.try_into().map_err(|_|Error::InvalidData)?));}
    if r.flag()?{g.frame=Some(Frame{side:r.text()?,trim:r.text()?,front:r.text()?});}
    if r.at!=end||!g.within_capacity(){return Err(Error::InvalidData);}Ok(g)
}
