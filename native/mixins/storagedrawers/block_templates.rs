// Only copies made for this mod are changed; vanilla templates stay intact.
use super::{J,Rename};
pub(super) fn adjust(doc:&mut J,rename:&Rename){
    let (from,to)=(&rename.from,&rename.to);
    let Some(id)=to.strip_prefix("storagedrawers:")else{return;};
    let button=id.starts_with("keybutton_");
    let metadata=matches!(doc.get("name"),Some(J::Str(n))if n==from||n==to);
    if metadata&&from=="minecraft:stone"&&!id.ends_with("_trim"){
        // Reuse only passive horizontal placement, never a workstation menu.
        doc.set("class",J::Arr(["GlazedTerracottaBlock","HorizontalDirectionalBlock","Block"]
            .into_iter().map(|s|J::Str(s.into())).collect()));
    }
    if button&&from=="minecraft:stone_button"{
        if metadata{
            if let Some(v)=doc.at("fields/ticksToStayPressed"){*v=J::Num(10.into());}
            for (path,value) in [("fields/type/name","storagedrawers_key_button"),
                ("fields/type/buttonClickOn","minecraft:block.wooden_button.click_on"),
                ("fields/type/buttonClickOff","minecraft:block.wooden_button.click_off")]{
                if let Some(v)=doc.at(path){*v=J::Str(value.into());}
            }
            for path in ["fields/type/canOpenByWindCharge","fields/type/canButtonBeActivatedByArrows"]{
                if let Some(v)=doc.at(path){*v=J::Bool(false);}
            }
        }
        if matches!(doc.get("item"),Some(J::Str(n))if n==from||n==to)&&doc.get("push").is_some(){
            doc.set("push",J::Str("PUSH_PULL".into()));
        }
    }
    // Compact behavior-state rows: sturdy, flags, fluid..., weak, strong.
    // The Java key button emits no signal and has its declared collision.
    if let J::Arr(row)=doc{if row.len()==9&&matches!(row.get(4),Some(J::Bool(_))){
        if button&&from=="minecraft:stone_button"{
            if let J::Num(flags)=&row[1]{if let Some(flags)=flags.as_u64(){row[1]=J::Num((flags&!40).into());}}
            row[5]=J::Num(0.into());row[6]=J::Num(0.into());
        }else if from=="minecraft:stone"&&(id.contains("drawers_")||id=="framed_trim"||id=="framing_table"){
            // Drawer blocks do not suffocate entities or conduct redstone.
            if let J::Num(flags)=&row[1]{if let Some(flags)=flags.as_u64(){
                let analog=if id.contains("drawers_"){16}else{0};
                row[1]=J::Num(((flags&!68)|analog).into());
            }}
        }
    }}
}
