"""Generate the distributable native bundle and vanilla-format content assets."""
from pathlib import Path
import json, re, shutil, zipfile, hashlib

ROOT=Path(__file__).resolve().parents[1]
REF=ROOT.parent/'references'/'StorageDrawers-1.21'
JAVA=REF/'common/src/main/java/com/jaquadro/minecraft/storagedrawers'
RES=REF/'common/src/main/resources'
BUNDLE=ROOT/'native/mixins/storagedrawers'
CONTENT=ROOT/'content'
REV='9f42064c11efe7ef3e5ed2baad7ddc2a3b36eea0'
VERSION='0.2.7'

def normalize_loot(value):
    # Horizon's runtime condition codec uses `type`; copied Java loot uses
    # `condition`. Keep the container keys `condition`/`conditions` intact.
    if isinstance(value,list):return [normalize_loot(v)for v in value]
    if isinstance(value,dict):
        out={k:normalize_loot(v)for k,v in value.items()}
        if isinstance(out.get('condition'),str)and 'type'not in out:out['type']=out.pop('condition')
        return out
    return value

def dump(path,obj):
    if '/loot_table/'in path.as_posix():obj=normalize_loot(obj)
    path.parent.mkdir(parents=True,exist_ok=True)
    path.write_text(json.dumps(obj,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')

def generate():
    BUNDLE.mkdir(parents=True,exist_ok=True)
    engine=BUNDLE/'engine'
    engine.mkdir(exist_ok=True)
    (engine/'Cargo.toml').write_text(f'[package]\nname="storagedrawers-engine"\nversion="{VERSION}"\nedition="2024"\nlicense="MIT"\npublish=false\n',encoding='utf-8')
    shutil.copytree(ROOT/'src',engine/'src',dirs_exist_ok=True)
    recipe_engine=BUNDLE/'recipe_engine';recipe_engine.mkdir(exist_ok=True)
    (recipe_engine/'Cargo.toml').write_text(f'[package]\nname="storagedrawers-recipe-engine"\nversion="{VERSION}"\nedition="2024"\nlicense="MIT"\npublish=false\n',encoding='utf-8')
    shutil.copytree(ROOT/'src',recipe_engine/'src',dirs_exist_ok=True)
    (BUNDLE/'recipe_items.rs').write_text((BUNDLE/'items.rs').read_text('utf-8').replace('use storagedrawers_engine as sd;','use storagedrawers_recipe_engine as sd;'),encoding='utf-8')
    CONTENT.mkdir(exist_ok=True)
    (CONTENT/'horizon-mod.toml').write_text(f'id="storagedrawers"\nname="储物抽屉"\nversion="{VERSION}"\ndescription="储物抽屉 Java 1.21 的 Horizon 移植"\nauthors=["Justin Aquadro", "Horizon port contributors"]\napi="0.2"\nside="both"\ncapabilities=["registries"]\n',encoding='utf-8')
    shutil.copytree(RES/'assets/storagedrawers/textures',CONTENT/'assets/storagedrawers/textures',dirs_exist_ok=True)
    shutil.copytree(RES/'assets/storagedrawers/lang',CONTENT/'assets/storagedrawers/lang',dirs_exist_ok=True)
    # The upstream models include the inset drawer face, frame and item transforms.
    # Replacing them with generic cubes loses both appearance and the label's depth.
    for folder in ['models','blockstates']:
        shutil.copytree(RES/f'assets/storagedrawers/{folder}',CONTENT/f'assets/storagedrawers/{folder}',dirs_exist_ok=True)
    shutil.copy2(REF/'LICENSE',CONTENT/'LICENSE')
    shutil.copytree(RES/'data/storagedrawers/tags',CONTENT/'data/storagedrawers/tags',dirs_exist_ok=True)
    woods=['oak','spruce','birch','jungle','acacia','dark_oak','mangrove','cherry','bamboo','crimson','warped','framed']
    blocks=[]
    for wood in woods:
        for depth in ['full','half']:
            for slots in [1,2,4]:blocks.append(f'{wood}_{depth}_drawers_{slots}')
        blocks.append(f'{wood}_trim')
    blocks += [f'{frame}compacting_{half}drawers_{n}' for frame in ['', 'framed_'] for half in ['', 'half_'] for n in [2,3]]
    blocks += ['controller','controller_io','framed_controller','framed_controller_io','framing_table',
        'keybutton_drawer','keybutton_quantify','keybutton_concealment']
    items=list(dict.fromkeys(re.findall(r'ITEMS\.register\("([a-z0-9_]+)"',(JAVA/'core/ModItems.java').read_text())))
    for name in blocks:
        stone='compacting' in name or 'controller' in name
        # A passive cube supplies light/shape facts without a workstation's
        # use, inventory or fire behavior. Material properties are explicit.
        declaration={'template':'minecraft:stone',
            'properties':{'hardness':4.0 if stone else 3.0,'explosion_resistance':5.0 if stone else 4.0,
                'sounds':'minecraft:stone' if stone else 'minecraft:oak_planks','requires_tool':False,'map_color':0},
            'default_state':{}}
        # Retain trim's old facing property so existing world palettes remain
        # valid, although its passive cube model does not use orientation.
        declaration['state_properties']={'facing':{'type':'enum','values':['north','south','west','east']}}
        declaration['default_state']['facing']='north'
        if 'compacting' in name:
            declaration['state_properties']['slots']={'type':'enum','values':[f'open{i}'for i in range(1,int(name[-1])+1)]}
            declaration['default_state']['slots']='open1'
        if 'drawers_' in name:
            declaration['state_properties']['hopper']={'type':'enum','values':['false','true']}
            declaration['default_state']['hopper']='false'
            half='_half_' in name or 'compacting_half_' in name
            full_north=[
                [0,0,1/16,1,1,1], [0,0,0,1/16,1,1/16], [15/16,0,0,1,1,1/16],
                [1/16,0,0,15/16,1/16,1/16], [1/16,15/16,0,15/16,1,1/16]]
            north=[[0,0,0.5,1,1,1]] if half else full_north
            def subtract(box,cut):
                lo=[max(box[i],cut[i])for i in range(3)];hi=[min(box[i+3],cut[i+3])for i in range(3)]
                if any(lo[i]>=hi[i]for i in range(3)):return [box]
                out=[];remaining=box[:]
                for axis in range(3):
                    if remaining[axis]<lo[axis]:
                        piece=remaining[:];piece[axis+3]=lo[axis];out.append(piece);remaining[axis]=lo[axis]
                    if hi[axis]<remaining[axis+3]:
                        piece=remaining[:];piece[axis]=hi[axis];out.append(piece);remaining[axis+3]=hi[axis]
                return out
            # BlockDrawers uses FULL minus HOPPER_INSIDE even for half drawers.
            hopper_north=[part for box in full_north for part in subtract(box,[2/16,12/16,2/16,14/16,1,14/16])]
            def rotate_box(box,facing):
                x1,y1,z1,x2,y2,z2=box
                def rotate(x,z):
                    return {'north':(x,z),'south':(1-x,1-z),'east':(1-z,x),'west':(z,1-x)}[facing]
                corners=[rotate(x,z) for x in [x1,x2] for z in [z1,z2]]
                return [min(x for x,z in corners),y1,min(z for x,z in corners),
                        max(x for x,z in corners),y2,max(z for x,z in corners)]
            declaration['collision_shapes']={'properties':['facing','hopper'],'values':{
                f'{face},{hopper}':[rotate_box(box,face)for box in (hopper_north if hopper=='true' else north)]
                for face in ['north','south','west','east']for hopper in ['false','true']}}
            declaration['shapes']=declaration['collision_shapes']
            if half:declaration['properties']['occludes']=False
        if name.startswith('framed_'):
            declaration['properties']['render_shape']='ENTITYBLOCK_ANIMATED'
            # All upstream framed registrations call noOcclusion(), even when
            # their collision is a full block. Otherwise glass hides terrain.
            declaration['properties']['occludes']=False
        if name=='framing_table':
            declaration['state_properties']['part']={'type':'enum','values':['left','right']}
            declaration['default_state']['part']='right'
            declaration['properties']['occludes']=False
            declaration['properties']['requires_tool']=False
            bottoms={'north':[1,0,0,15,16,15],'south':[1,0,1,15,16,16],'west':[1,0,1,16,16,15],'east':[0,0,1,15,16,15]}
            orientation={('north','left'):'west',('north','right'):'east',('south','left'):'east',('south','right'):'west',('west','left'):'north',('west','right'):'south',('east','left'):'south',('east','right'):'north'}
            declaration['shapes']={'properties':['facing','part'],'values':{f'{f},{p}':[[0,14/16,0,1,1,1],[v/16 for v in bottoms[d]]]for (f,p),d in orientation.items()}}
            declaration['collision_shapes']=declaration['shapes']
        if name.startswith('keybutton_'):
            # Button placement/support are shared vanilla mechanics. Its state
            # schema also supplies every upstream model variant (3 x 4 x 2).
            declaration={'template':'minecraft:stone_button','properties':{
                'hardness':0.0,'explosion_resistance':0.0,'sounds':'minecraft:stone',
                'requires_tool':False,'occludes':False},
                'default_state':{'face':'wall','facing':'north','powered':'false'}}
            def button_box(face,facing,powered):
                thick=1 if powered=='true' else 2
                if face=='floor':box=[3,0,3,13,thick,13]
                elif face=='ceiling':box=[3,16-thick,3,13,16,13]
                else:box={'north':[3,3,16-thick,13,13,16],'south':[3,3,0,13,13,thick],
                    'west':[16-thick,3,3,16,13,13],'east':[0,3,3,thick,13,13]}[facing]
                return [[v/16 for v in box]]
            declaration['shapes']={'properties':['face','facing','powered'],'values':{
                f'{face},{facing},{powered}':button_box(face,facing,powered)
                for face in ['floor','wall','ceiling'] for facing in ['north','south','west','east'] for powered in ['false','true']}}
            declaration['collision_shapes']=declaration['shapes']
        dump(CONTENT/f'data/storagedrawers/horizon/block/{name}.json',declaration)
        assert (CONTENT/f'assets/storagedrawers/blockstates/{name}.json').exists(), name
        assert (CONTENT/f'assets/storagedrawers/models/item/{name}.json').exists(), name
        dump(CONTENT/f'data/storagedrawers/loot_table/blocks/{name}.json',{'type':'minecraft:block','pools':[{'rolls':1,'entries':[{'type':'minecraft:item','name':'storagedrawers:'+name}]}]})
    # Both halves roll ordinary loot. The shared block-loot adapter claims
    # one table per pair, also when an explosion rolls both halves.
    dump(CONTENT/'data/storagedrawers/loot_table/blocks/framing_table.json',{'type':'minecraft:block','pools':[{'rolls':1,'conditions':[{'condition':'minecraft:survives_explosion'}],'entries':[{'type':'minecraft:item','name':'storagedrawers:framing_table'}]}]})
    for name in items:
        dump(CONTENT/f'data/storagedrawers/horizon/item/{name}.json',{'template':'minecraft:stick','max_stack_size':1 if name.startswith('keyring') or name=='detached_drawer_full' else 64})
        assert (CONTENT/f'assets/storagedrawers/models/item/{name}.json').exists(), name
    for name in blocks+items:
        dump(CONTENT/f'assets/storagedrawers/items/{name}.json',{'model':{'type':'minecraft:model','model':'storagedrawers:item/'+name}})
    messages={
        'en_us':{'storagedrawers.menu.upgrades':'Drawer upgrades',
            'storagedrawers.mod.title':'Storage Drawers',
            'storagedrawers.mod.description':'Drawers, compacting storage and controller networks.',
            'storagedrawers.tooltip.drawer_use':'Sneak + use with an empty hand: upgrades. Sneak + use with a block: place it.',
            'storagedrawers.tooltip.stored':'Stored: %s',
            'storagedrawers.tooltip.capacity':'Capacity: %s',
            'storagedrawers.tooltip.multiplier':'Base capacity multiplier: %s',
            'storagedrawers.message.denied':'You do not have access to this drawer.',
            'storagedrawers.message.bound':'Remote upgrade bound to this controller.',
            'storagedrawers.message.corrupt':'Cannot read this drawer: its saved data has been preserved.',
            'storagedrawers.message.upgrade_failed':'This upgrade cannot be installed here.',
            'storagedrawers.message.no_compaction':'No supported reversible compacting recipe for this item.'},
        'zh_cn':{'storagedrawers.menu.upgrades':'抽屉升级',
            'storagedrawers.mod.title':'储物抽屉',
            'storagedrawers.mod.description':'储物抽屉、压缩储物与控制器网络。',
            'storagedrawers.tooltip.drawer_use':'空手潜行右键打开升级界面；手持方块潜行右键正常放置。',
            'storagedrawers.tooltip.stored':'存储数量：%s',
            'storagedrawers.tooltip.capacity':'单格容量：%s',
            'storagedrawers.tooltip.multiplier':'基础容量倍率：%s 倍',
            'item.storagedrawers.fill_level_upgrade':'储量显示升级',
            'item.storagedrawers.fill_level_upgrade.desc':'在抽屉正面显示容量使用进度',
            'item.storagedrawers.one_stack_upgrade':'单组容量升级',
            'item.storagedrawers.one_stack_upgrade.desc':'将每格基础容量限制为一组；容量升级仍然生效',
            'item.storagedrawers.illumination_upgrade.desc':'提高抽屉正面物品标签的亮度',
            'item.storagedrawers.drawer_key':'抽屉锁定钥匙',
            'item.storagedrawers.drawer_key.desc':'右键切换锁定；取空后保留物品种类',
            'item.storagedrawers.quantify_key':'数量显示钥匙',
            'item.storagedrawers.quantify_key.desc':'右键切换抽屉正面的数量显示',
            'item.storagedrawers.shroud_key':'隐藏标签钥匙',
            'item.storagedrawers.shroud_key.desc':'右键切换物品标签的显示与隐藏',
            'item.storagedrawers.suspend_key':'自动化暂停钥匙',
            'item.storagedrawers.suspend_key.desc':'右键暂停或恢复漏斗等自动化存取',
            'storagedrawers.message.denied':'你没有此抽屉的使用权限。',
            'storagedrawers.message.bound':'远程升级已绑定此控制器。',
            'storagedrawers.message.corrupt':'无法读取此抽屉；已保留原始存档数据。',
            'storagedrawers.message.upgrade_failed':'此处无法安装该升级。',
            'storagedrawers.message.no_compaction':'此物品没有受支持的可逆压缩配方。'}}
    english=json.loads((RES/'assets/storagedrawers/lang/en_us.json').read_text(encoding='utf-8'))
    # Java ItemDrawers composes these names in code; Horizon needs full translation keys.
    for path in (CONTENT/'assets/storagedrawers/lang').glob('*.json'):
        current=json.loads(path.read_text(encoding='utf-8'))
        base={**english,**current}
        for wood in woods:
            material=base[f'block.storagedrawers.mat.{wood}']
            if path.stem=='zh_cn':
                material={'oak':'橡木','spruce':'云杉木','birch':'白桦木','jungle':'丛林木','acacia':'金合欢木',
                    'dark_oak':'深色橡木','mangrove':'红树木','cherry':'樱花木','bamboo':'竹',
                    'crimson':'绯红木','warped':'诡异木','framed':'镶框'}[wood]
                current[f'block.storagedrawers.mat.{wood}']=material
            for shape in ['full_drawers_1','full_drawers_2','full_drawers_4','half_drawers_1','half_drawers_2','half_drawers_4','trim']:
                if path.stem=='zh_cn':
                    suffix='桥接方块' if shape=='trim' else ('半深储物抽屉' if shape.startswith('half') else '储物抽屉')+'（'+{'1':'单格','2':'双格','4':'四格'}[shape[-1]]+'）'
                    current[f'block.storagedrawers.{wood}_{shape}']=material+suffix
                else:current[f'block.storagedrawers.{wood}_{shape}']=base[f'block.storagedrawers.type.{shape}'].replace('%s',material)
        for name in items:
            key=f'item.storagedrawers.{name}'
            if key not in current:
                source='detached_drawer' if name=='detached_drawer_full' else 'keyring' if name.startswith('keyring_') else name
                current[key]=base.get(f'item.storagedrawers.{source}',english.get(key,name.replace('_',' ').title()))
        current.update(messages.get(path.stem,messages['en_us']))
        dump(path,current)
    for tool in ['axe','pickaxe']:
        source=RES/f'data/minecraft/tags/block/mineable/{tool}.json'
        dump(CONTENT/f'data/minecraft/tags/block/mineable/{tool}.json',json.loads(source.read_text('utf-8')))
    dump(CONTENT/'data/c/tags/item/chests/wooden.json',{'replace':False,'values':['minecraft:chest','minecraft:trapped_chest']})
    excluded=set(re.findall(r'EXCLUDE_ITEMS_CREATIVE_TAB.add\(([A-Z0-9_]+)\)',(JAVA/'core/ModItems.java').read_text()))
    hidden={name for symbol,name in re.findall(r'([A-Z0-9_]+)\s*=\s*ITEMS.register\("([a-z0-9_]+)"',(JAVA/'core/ModItems.java').read_text()) if symbol in excluded}
    dump(CONTENT/'data/storagedrawers/horizon/creative_tab/drawers.json',{'title':'itemGroup.storagedrawers','icon':'storagedrawers:oak_full_drawers_1','items':['storagedrawers:'+x for x in blocks+items if x not in hidden]})
    # A named menu with server-sent block position: its screen must never
    # depend on the client's last click or fall back to a chest layout.
    dump(CONTENT/'data/storagedrawers/horizon/menu/framing.json',{'slots':9,'results':[4]})
    dump(CONTENT/'data/storagedrawers/horizon/menu/drawers.json',{'slots':18})
    hopper_model=json.loads((RES/'assets/storagedrawers/models/block/full_drawers_hopper.json').read_text('utf-8'))
    # Upstream's "top" cullface alias is named "up" in Horizon's model codec.
    hopper_model['elements'][0]['faces']['up']['cullface']='up'
    dump(CONTENT/'assets/storagedrawers/models/block/full_drawers_hopper.json',hopper_model)
    # Copy only vanilla recipes; Java custom recipe serializers need explicit native implementations.
    recipes=REF/'neoforge/src/main/resources/data/storagedrawers/recipe'
    converted=[];unsupported=[]
    def ingredient(v):
        if isinstance(v,dict):
            if set(v)=={'item'}:return v['item']
            if set(v)=={'tag'}:return '#'+v['tag']
            return {k:ingredient(x) for k,x in v.items() if k not in ['conditions','neoforge:conditions']}
        if isinstance(v,list):return [ingredient(x)for x in v]
        return v
    for path in recipes.rglob('*.json'):
        obj=json.loads(path.read_text());kind=obj.get('type','')
        if not kind.startswith('minecraft:'):
            dump(CONTENT/'data/storagedrawers/recipe'/path.relative_to(recipes),obj)
            unsupported.append({'file':str(path.relative_to(recipes)),'type':kind});continue
        obj=ingredient(obj)
        result=obj.get('result')
        if isinstance(result,dict) and 'item'in result:result['id']=result.pop('item')
        dump(CONTENT/'data/storagedrawers/recipe'/path.relative_to(recipes),obj);converted.append(str(path.relative_to(recipes)))
    dump(ROOT/'docs/generated-content.json',{'blocks':blocks,'items':items,'recipes':converted,'native_custom_recipes':unsupported,'custom_recipes_pending':[]})
    dump(ROOT/'native/horizon-native.json',{'schema':1,'id':'storagedrawers','version':VERSION,'game':'26.3','horizon':REV,'mixins':'5a64af938ec86b00d53d16a408aabc0038fb3e57'})
    # Geometry from the original Java renderer's model metadata, in model pixels.
    geometry=['// Generated from Storage Drawers 1.21 geometry; MIT, Justin Aquadro.',
        'pub(super) fn areas(compact:bool,half:bool,n:usize)->[&\'static [[[f32;3];2]];4]{',
        'match (compact,half,n){']
    def rust_array(v):
        if isinstance(v,list):return '['+','.join(rust_array(x) for x in v)+']'
        return repr(float(v))
    for compact,counts in [(False,[1,2,4]),(True,[2,3])]:
        for half in [False,True]:
            for n in counts:
                prefix=('half' if half else 'full')+('_comp_drawers' if compact else '_drawers')
                arrays=[]
                for area in ['icon','count','ind','indbase']:
                    p=RES/f'assets/storagedrawers/models/block/geometry/{prefix}_{area}_area_{n}.json'
                    elems=json.loads(p.read_text())['elements']
                    arrays.append('&'+rust_array([[e['from'],e['to']] for e in elems]))
                geometry.append(f'({str(compact).lower()},{str(half).lower()},{n})=>['+','.join(arrays)+'],')
    geometry.extend(['_=>areas(false,half,1),','}}'])
    (BUNDLE/'geometry.rs').write_text('\n'.join(geometry)+'\n',encoding='utf-8')
    sprites=sorted((RES/'assets/storagedrawers/textures/block/indicator').glob('*.png'))
    dump(CONTENT/'assets/storagedrawers/models/block/sd_sprites.json',{
        'textures':{f's{i}':f'storagedrawers:block/indicator/{p.stem}' for i,p in enumerate(sprites)},
        'elements':[{'from':[0,0,0],'to':[16,16,0],
            'faces':{'north':{'texture':f'#s{i}'}}} for i in range(len(sprites))]})
    # Select exactly the upstream's side/trim/front/overlay geometry. Material
    # replacement uses its particle sprite, as SpriteReplacementModel does.
    def part_model(meta, props):
        variants=json.loads((RES/f'assets/storagedrawers/blockstates/{meta}.json').read_text())['variants']
        for key,variant in variants.items():
            values=dict(x.split('=')for x in key.split(',')if x)
            if all(props.get(k)==v for k,v in values.items()):return variant['model']
        raise ValueError((meta,props))
    rows=[];all_models={'storagedrawers:block/full_drawers_hopper'}
    for name in blocks:
        if not name.startswith('framed_'):continue
        if 'compacting_' in name: stem=f'meta_framed_compdrawers_{name[-1]}'
        elif 'drawers_' in name:stem='meta_framed_drawers'
        elif name=='framed_trim':stem='meta_framed_trim'
        else:stem='meta_'+name
        for opened in (['open'+str(i)for i in range(1,int(name[-1])+1)] if 'compacting_' in name else ['open1']):
            props={'facing':'north','half':str('half' in name).lower(),'slots':opened if 'compacting_' in name else name[-1]}
            parts=[]
            for role in ['side','trim','front','shading']:
                if not (RES/f'assets/storagedrawers/blockstates/{stem}_{role}.json').exists():continue
                model=part_model(stem+'_'+role,props);parts.append((role,model));all_models.add(model)
            rows.append((name,opened,parts))
    for depth in ['full','half']:
        for n in [1,2,4]:
            for slot in range(1,n+1):all_models.add(f'storagedrawers:block/{depth}_drawers_missing_{n}_{slot}')
    decoration_models=[]
    for half in [False,True]:
        props={'facing':'north','half':str(half).lower()}
        for role in ['lock','claim','lock_claim']:
            meta={'lock':'meta_locked','claim':'meta_claimed','lock_claim':'meta_locked_claimed'}[role]
            model=part_model(meta,props);all_models.add(model)
            decoration_models.append((role,half,0,model))
        for slot in range(1,7):
            model=part_model('meta_right_label',{**props,'slot':str(slot)});all_models.add(model)
            decoration_models.append(('right_label',half,slot,model))
    appearance=['// Generated from Java 1.21 blockstate metadata.','fn parts(block:&str,open:&str)->&\'static [(&\'static str,&\'static str)]{match (block,open){']
    for name,opened,parts in rows:
        appearance.append(f'("storagedrawers:{name}","{opened}")=>&['+','.join(f'("{r}","{m}")'for r,m in parts)+'],')
    appearance += ['_=>&[],}}',"fn decoration(role:&str,half:bool,slot:usize)->Option<&'static str>{Some(match (role,half,slot){"]
    appearance += [f'("{role}",{str(half).lower()},{slot})=>"{model}",'for role,half,slot,model in decoration_models]
    appearance += ['_=>return None,})}', 'pub(super) fn models(out:&mut Vec<String>){out.extend(['+','.join(json.dumps(m)+'.to_owned()'for m in sorted(all_models))+']);}']
    (BUNDLE/'appearance_parts.rs').write_text('\n'.join(appearance)+'\n','utf-8')
    (BUNDLE/'appearance.rs').write_text((BUNDLE/'appearance_base.rs').read_text('utf-8').replace('// GENERATED_PARTS','\n'.join(appearance)),'utf-8')
    data=BUNDLE/'data';data.mkdir(exist_ok=True)
    with zipfile.ZipFile(data/'content.hmod','w',zipfile.ZIP_DEFLATED) as archive:
        for path in sorted(CONTENT.rglob('*')):
            if path.is_file():archive.write(path,path.relative_to(CONTENT).as_posix())
    print(f'Generated {len(blocks)} blocks, {len(items)} items, {len(converted)} ordinary recipes and {len(unsupported)} native custom recipes.')

if __name__=='__main__':generate()

