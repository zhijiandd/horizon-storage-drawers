"""Stage the SDK port with the authoritative inventory core and flat resources.

Staging never installs a mod or changes an instance. The SDK adapter is developed
separately from the working legacy adapter until its migration is complete.
"""
from pathlib import Path
import argparse
import shutil
import json
import re

ROOT = Path(__file__).resolve().parents[1]
LEGACY = ROOT / 'native/mixins/storagedrawers'


def stage_renderer(destination: Path, legacy: str, framing: str):
    """Reuse the original model parts, lighting and front label renderer."""
    native = destination / 'src/native'
    appearance = (LEGACY / 'appearance.rs').read_text('utf-8')
    appearance = appearance.replace('use super::{Game,sd_items as data};', 'use super::{Game,sd_items as data,BlockEntityDraw};')
    appearance = appearance.replace('horizon_world::nbt', 'horizon_core::nbt')
    appearance = appearance.replace('use horizon_client::menu::sd_rules::engine as storagedrawers_engine;',
                                    'use storagedrawers::engine as storagedrawers_engine;')
    appearance = appearance.replace('super::native_block_entity_draw::BlockEntityDraw', 'BlockEntityDraw')
    appearance = appearance.replace('game.native_block_property(', 'super::sd_labels::block_property(game,')
    appearance = appearance.replace('!m.id.contains(', '!m.id().contains(')
    appearance = appearance.replace('super::sd_framing::render(', 'super::sd_labels::render_table(')
    appearance = appearance.replace('game.native_draw_model(draw,&missing(block,slot),0xFFFFFF);',
        'if let Ok(id)=mod_api::Identifier::parse(&missing(block,slot))\n'
        '    && let Ok(model)=game.native_model(&id){\n'
        '    let _=game.native_draw_model(draw,model,0xFFFFFFFF);\n'
        '}')
    appearance = appearance.replace('display:DisplayContext,owner:Option<&OwnerState>,seed:i32,hand:horizon_client::cit::Hand',
                                    'mut state:ItemRenderState')
    start = appearance.index('    let mut plain=stack.clone();')
    end = appearance.index('    let mut geometry=', start)
    appearance = appearance[:start] + appearance[end:]
    names = sorted(set(re.findall(r'"(storagedrawers:block/[^"]+)"\.to_owned\(\)', appearance)))
    (destination / 'src/render_models.rs').write_text('pub const MODELS: &[&str] = &' + json.dumps(names) + ';\n', 'utf-8')
    start = appearance.index('pub(super) fn models(')
    end = appearance.index('\n', start)
    appearance = appearance[:start] + appearance[end+1:]
    appearance = appearance.replace('pub(super) fn item(', 'pub fn item(')
    appearance = appearance.replace('->Option<ItemRenderState>{', '->ItemRenderState{')
    start = appearance.index('pub fn item(')
    appearance = appearance[:start] + appearance[start:].replace('return None;', 'return state;').replace('}}Some(state)', '}}state')
    (native / 'appearance.rs').write_text(appearance, 'utf-8')
    for name in ('geometry.rs', 'lighting.rs'):
        shutil.copy2(LEGACY / name, native / name)
    items = (LEGACY / 'items.rs').read_text('utf-8')
    items = items.replace('use horizon_client::menu::sd_rules::engine as sd;', 'use storagedrawers::engine as sd;')
    items = items.replace('horizon_world::nbt', 'horizon_core::nbt')
    (native / 'items.rs').write_text(items, 'utf-8')
    render = legacy[legacy.index('pub(super) fn render('):legacy.index('pub(super) fn before_remove(')]
    render = render.replace('    if !game.native_on(ID){return;}',
                            '    if !block.starts_with("storagedrawers:"){return;}')
    render = render.replace('super::native_block_entity_draw::BlockEntityDraw', 'BlockEntityDraw')
    render = render.replace('game.native_text_width(&text)', 'game.native_draw_text_width(draw,&Text::literal(text.clone()))')
    render = render.replace('false,0);', 'false,None);')
    render = render.replace('pub(super) fn render(', 'pub fn render(')
    table = framing[framing.index('fn supports_front('):framing.index('fn shown(')]
    table += framing[framing.index('pub(super) fn render('):framing.index('// Both halves use the ordinary loot pipeline.')]
    table = table.replace('pub(super) fn render(', 'pub(super) fn render_table(')
    table = table.replace('super::native_block_entity_draw::BlockEntityDraw', 'BlockEntityDraw')
    table = table.replace('game.native_block_property(', 'block_property(game,')
    start = table.index('    let v=match m.nbt')
    end = table.index('    if v.len()', start)
    table = table[:start] + '''    let mut v=vec![None;4];
    if let Some(Tag::List(inputs))=m.nbt.get(&"sd_frame_inputs".into()){
        for input in inputs{
            let Some(Tag::Int(index))=input.get("Slot")else{continue};
            let Some(slot)=v.get_mut(*index as usize)else{continue};
            *slot=input.get("Item").and_then(horizon_items::item_nbt::item_from_nbt);
        }
    }else if let Some(Tag::List(items))=m.nbt.get(&"sd_table_items".into()){
        for (slot,item) in v.iter_mut().zip(items){*slot=horizon_items::item_nbt::item_from_nbt(item);}
    }
''' + table[end:]
    stack = legacy[legacy.index('fn stack(item:'):legacy.index('// Keep physical slots separate')]
    stack = stack.replace('fn stack(', 'pub fn stack(')
    stack = stack.replace('horizon_world::nbt', 'horizon_core::nbt')
    (native / 'labels.rs').write_text('''use super::{Game,BlockEntityDraw,sd_items as data};
use horizon_client::{containers::ModBlockEntity, items::ItemStack, text::Text};
use horizon_core::nbt::Tag;
use storagedrawers::engine as sd;
type Pos=[i32;3];
fn kind(block:&str)->&str{block.strip_prefix("storagedrawers:").unwrap_or("")}
fn drawer(block:&str)->bool{kind(block).contains("drawers_")}
fn slots(block:&str)->usize{block.rsplit('_').next().and_then(|s|s.parse().ok()).filter(|n|[1,2,3,4].contains(n)).unwrap_or(1)}
fn group(entity:&ModBlockEntity)->Result<sd::Group,()>{
    let Some(Tag::ByteArray(bytes))=entity.nbt.get(&"sd_state".into())else{return Err(())};
    sd::codec::decode(bytes).map_err(|_|())
}
pub(super) fn block_property(game:&Game<'_>,pos:Pos,key:&str)->Option<String>{
    let state=game.world.block(pos[0],pos[1],pos[2]);
    game.pack.assets.registry.state(state)?.properties.get(key).cloned()
}
fn facing(game:&Game<'_>,pos:Pos)->&'static str{
    match block_property(game,pos,"facing").as_deref(){Some("east")=>"east",Some("south")=>"south",Some("west")=>"west",_=>"north"}
}
''' + stack + render + table, 'utf-8')


def stage(destination: Path, api_source: Path | None = None) -> Path:
    destination = destination.resolve()
    destination.mkdir(parents=True, exist_ok=True)
    shutil.copytree(ROOT / 'sdk', destination, dirs_exist_ok=True)
    shutil.copytree(ROOT / 'native/mixins/storagedrawers/recipe_engine/src',
                    destination / 'src/engine', dirs_exist_ok=True)
    (destination / 'src/engine/Cargo.toml').write_text(
        '[package]\nname="storagedrawers-recipe-engine"\nversion="0.1.0"\nedition="2024"\n'
        '[lib]\npath="lib.rs"\n[dependencies]\nserde_json="1"\n', encoding='utf-8')
    content = ROOT / 'content/data/storagedrawers/horizon'
    blocks = ['storagedrawers:' + p.stem for p in sorted((content / 'block').glob('*.json'))]
    tools = ['storagedrawers:' + p.stem for p in sorted((content / 'item').glob('*.json'))
             if p.stem.startswith(('keyring', 'priority_key', 'detached_drawer'))]
    (destination / 'src/blocks.rs').write_text(
        'pub const BLOCKS: &[&str] = &' + json.dumps(blocks) + ';\n'
        'pub const TOOLS: &[&str] = &' + json.dumps(tools) + ';\n', encoding='utf-8')
    items = (ROOT / 'native/mixins/storagedrawers/items.rs').read_text('utf-8')
    items = items.replace('use horizon_items::{items::ItemStack,item_nbt::{item_from_nbt,item_to_nbt}};',
                          'use crate::compat::{Stack as ItemStack,item_from_nbt,item_to_nbt};')
    items = items.replace('use horizon_world::nbt::{Compound,Tag};', 'use mod_api::{Compound,Tag};')
    items = items.replace('use horizon_client::menu::sd_rules::engine as sd;', 'use crate::engine as sd;')
    # This adapter-specific helper is unused by the pure item/recipe implementation.
    start = items.index('pub(crate) fn stock(')
    end = items.index('pub(crate) fn binding(', start)
    items = items[:start] + items[end:]
    (destination / 'src/items.rs').write_text(items, encoding='utf-8')
    # Keep the mature, pure menu state machine in one authoritative source. The
    # SDK context only supplies item limits, compaction and component boundaries.
    legacy = (ROOT / 'native/mixins/storagedrawers/integration.rs').read_text('utf-8')
    logic = legacy[legacy.index('fn may_take_upgrade('):legacy.index('fn apply_menu_click(')]
    logic += legacy[legacy.index('fn cursor_click('):legacy.index('// entityInside fires')]
    logic = logic.replace("&Game<'_>", "&Context<'_>")
    logic = logic.replace('super::native_menus::native_menu_inventory_slot', 'inventory_slot')
    logic = logic.replace('super::sd_items::', 'crate::items::')
    logic = logic.replace('struct DrawerMenu{', 'pub(crate) struct DrawerMenu{')
    logic = logic.replace('    g:sd::Group,positions:Vec<usize>,upgrades:Vec<ItemStack>,',
        '    pub(crate) g:sd::Group,pub(crate) positions:Vec<usize>,pub(crate) upgrades:Vec<ItemStack>,')
    logic = logic.replace('    inv:Vec<Option<ItemStack>>,offhand:Option<ItemStack>,carried:Option<ItemStack>,',
        '    pub(crate) inv:Vec<Option<ItemStack>>,pub(crate) offhand:Option<ItemStack>,pub(crate) carried:Option<ItemStack>,')
    logic = logic.replace('    fn ', '    pub(crate) fn ')
    logic = logic.replace('fn stack(&self,slot:i32)', "fn stack(&self,game:&Context<'_>,slot:i32)")
    logic = logic.replace('self.stack(slot)', 'self.stack(game,slot)')
    logic = logic.replace('stack(i,1)', 'stack(game,i,1)')
    logic = logic.replace('let key=key(game,input);', 'let key=group_key(game,&g,input);')
    (destination / 'src/menu_logic.rs').write_text(
        'use crate::menu_context::*;\nuse crate::engine as sd;\nuse crate::compat::Stack as ItemStack;\n' + logic, encoding='utf-8')
    framing = (ROOT / 'native/mixins/storagedrawers/framing.rs').read_text('utf-8')
    stage_renderer(destination, legacy, framing)
    helpers = framing[framing.index('fn supports_front('):framing.index('pub(super) fn material(')]
    helpers += framing[framing.index('fn unpack('):framing.index('pub(super) fn open_menu(')]
    helpers += framing[framing.index('fn inventory_slot('):framing.index('pub(super) fn use_on(')]
    start = framing.index('    let initial_target', framing.index('pub(super) fn menu_click('))
    end = framing.index('    if (v.clone()',start)
    handle = framing[start:end]
    header = '''use crate::compat::Stack as ItemStack;
use crate::menu_context::Context;
use crate::items as data;
use crate::framing::{accepts,material};
use std::{cell::RefCell,collections::BTreeMap};
thread_local!{static DRAGS:RefCell<BTreeMap<u128,(i32,Vec<i32>,ItemStack)>>=const{RefCell::new(BTreeMap::new())};}
pub(crate) fn clear(uuid:u128){DRAGS.with_borrow_mut(|d|d.remove(&uuid));}
pub(crate) fn drag_pending(uuid:u128)->bool{DRAGS.with_borrow(|drags|drags.contains_key(&uuid))}
pub(crate) fn handle(game:&Context<'_>, uuid:u128, slot:i32,button:i32,mode:i32,creative:bool,
    mut v:Vec<Option<ItemStack>>,mut inv:Vec<Option<ItemStack>>,mut cursor:Option<ItemStack>,mut offhand:Option<ItemStack>)
    ->(Vec<Option<ItemStack>>,Vec<Option<ItemStack>>,Option<ItemStack>,Option<ItemStack>,Vec<ItemStack>){
    let max=|s:&ItemStack|game.stack_max_size(s);let mut drops=Vec::new();
'''
    text = header + handle + '    (v,inv,cursor,offhand,drops)\n}\n' + helpers
    text = text.replace("&Game<'_>", "&Context<'_>")
    text = text.replace('super::native_menus::native_menu_inventory_slot', 'crate::menu_context::inventory_slot')
    for name in ('result','shown','unpack'):
        text = text.replace('fn '+name+'(', 'pub(crate) fn '+name+'(')
    (destination / 'src/frame_logic.rs').write_text(text, encoding='utf-8')
    for name in ('assets', 'data'):
        shutil.copytree(ROOT / 'content' / name, destination / name, dirs_exist_ok=True)
    # The Java loaders supply these common tags. Contribute the vanilla
    # entries on Horizon while leaving entries from other mods in place.
    conventions = {
        'ingots/iron': ['minecraft:iron_ingot'],
        'ingots/gold': ['minecraft:gold_ingot'],
        'ingots/copper': ['minecraft:copper_ingot'],
        'ingots/netherite': ['minecraft:netherite_ingot'],
        'gems/diamond': ['minecraft:diamond'],
        'gems/emerald': ['minecraft:emerald'],
        'nuggets/gold': ['minecraft:gold_nugget'],
        'dusts/glowstone': ['minecraft:glowstone_dust'],
        'dusts/redstone': ['minecraft:redstone'],
        'obsidians': ['minecraft:obsidian', 'minecraft:crying_obsidian'],
        'rods/wooden': ['minecraft:stick'],
    }
    for tag, items in conventions.items():
        file = destination / f'data/c/tags/item/{tag}.json'
        file.parent.mkdir(parents=True, exist_ok=True)
        file.write_text(json.dumps({'replace': False, 'values': items}) + '\n', encoding='utf-8')
    # Passive horizontal placement replaces the old metadata-injection helper.
    # It has no workstation interaction or inherited menu to interfere with ours.
    for file in (destination / 'data/storagedrawers/horizon/block').glob('*.json'):
        value = json.loads(file.read_text('utf-8'))
        if value.get('template') == 'minecraft:stone' and not file.stem.endswith('_trim'):
            value['template'] = 'minecraft:white_glazed_terracotta'
            # The template already owns this property. Redeclaring it is a
            # fatal registry error when the actual resource pack is applied.
            value.get('state_properties', {}).pop('facing', None)
        if 'drawers_' in file.stem:
            value.setdefault('properties', {})['dynamic_light'] = True
        file.write_text(json.dumps(value,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    for name in ('LICENSE', 'LICENSE-StorageDrawers.txt'):
        shutil.copy2(ROOT / name, destination / name)
    if api_source:
        manifest = destination / 'Cargo.toml'
        text = manifest.read_text('utf-8')
        text = text.replace('path=".horizon/sdk/api/crates/horizon-mod-api"',
                            'path=' + '"' + (api_source.resolve() / 'crates/horizon-mod-api').as_posix() + '"')
        manifest.write_text(text + '\n[workspace]\n', encoding='utf-8')
    return destination


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--api-source', type=Path, help='local SDK API source for development checks only')
    args = parser.parse_args()
    print(stage(args.output, args.api_source))
