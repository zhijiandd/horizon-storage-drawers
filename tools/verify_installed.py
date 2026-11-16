"""Read-only verification of the installed native build and packaged content."""
from pathlib import Path
from zipfile import ZipFile
import hashlib,json,tomllib
root=Path(__file__).resolve().parents[1]
game=Path(r'F:\Games\Horizon\game');home=Path(r'F:\Games\Horizon\data')
version=json.loads((root/'native/horizon-native.json').read_text('utf-8'))['version']
selected=json.loads((home/'instances.json').read_text('utf-8'))['instances']['horizon']['version']
built=home/'versions'/selected
meta=json.loads((built/'native-mods.json').read_text('utf-8'))
assert not meta['left_out'],meta['left_out']
assert (built/'.installed').exists()and(built/'horizon-client.exe').stat().st_size>0
flags=dict(line.split(':',1)for line in(game/'horizon-mods.txt').read_text('utf-8').splitlines()if ':'in line)
actual={};digests={}
for p in sorted((game/'horizon-native-mods').glob('*.hnmod')):
 with ZipFile(p)as z:
  manifest=json.loads(z.read('horizon-native.json'));mod=manifest['id']
  if flags.get('mod.'+mod,'true')=='false':continue
  assert mod not in actual,('duplicate',mod)
  actual[mod]=manifest['version'];h=hashlib.sha256()
  for name in sorted(n for n in z.namelist()if not n.endswith('/')):
   body=z.read(name);h.update(name.encode());h.update(len(body).to_bytes(8,'little'));h.update(body)
  digests[mod]=h.hexdigest()
assert actual=={m['id']:m['version']for m in meta['mods']},(actual,meta['mods'])
assert digests=={m['id']:m['sha256']for m in meta['mods']},'installed sources do not match the selected build'
assert actual['storagedrawers']==version
artifact=root/f'dist/storagedrawers-horizon-{version}.hnmod'
assert artifact.read_bytes()==(game/'horizon-native-mods/storagedrawers.hnmod').read_bytes()
with ZipFile(artifact)as z:
 mixin=tomllib.loads(z.read('mixins/storagedrawers/horizon-mixin.toml').decode())
 with ZipFile(__import__('io').BytesIO(z.read('mixins/storagedrawers/data/content.hmod')))as content:
  for name in content.namelist():
   if name.endswith('.json'):json.loads(content.read(name))
  table=json.loads(content.read('data/storagedrawers/horizon/block/framing_table.json'))
  assert table['default_state']['part']=='right'
  assert len(table['collision_shapes']['values'])==8
  framed=json.loads(content.read('data/storagedrawers/horizon/block/framed_full_drawers_1.json'))
  assert framed['properties']['render_shape']=='ENTITYBLOCK_ANIMATED'
  assert framed['properties']['occludes'] is False
  assert framed['default_state']['hopper']=='false'
  assert len(framed['collision_shapes']['values'])==8
  trim=json.loads(content.read('data/storagedrawers/tags/item/trim.json'))
  assert len(trim['values'])==11
  loot=json.loads(content.read('data/storagedrawers/loot_table/blocks/framing_table.json'))
  assert loot['pools'][0]['conditions']==[{'type':'minecraft:survives_explosion'}]
  menu=json.loads(content.read('data/storagedrawers/horizon/menu/framing.json'))
  assert menu['slots']==9 and menu['results']==[4]
  drawers=json.loads(content.read('data/storagedrawers/horizon/menu/drawers.json'))
  assert drawers['slots']==18
audit=json.loads((root/f'docs/block-template-audit-{version}.json').read_text('utf-8'))
assert audit['version']==version and audit['model_states_checked']>0
probe=json.loads((root/f'docs/button-resources-{version}.json').read_text('utf-8'))
assert probe['version']==version and probe['button_states']==72 and probe['prepared_states']==audit['model_states_checked']
assert probe['drawers_menu']=='storagedrawers:drawers' and probe['framing_menu']=='storagedrawers:framing'
assert all(probe['client_ui'][check] for check in ('invalid_all_own_slots','shift_swap_drag_rejected',
 'valid_upgrade_and_glass','duplicate_upgrade_rejected','capacity_removal_rejected',
 'same_single_storage_unchanged','missing_snapshot_rejected','vanilla_chest_unchanged'))
report={'version':selected,'mods':actual,'source_digests':digests,'left_out':[],
 'compiled':True,'installed':True,'selected_for':'horizon','game_tested':False,
 'checks':[f"{len(mixin['injection'])} native injection points compiled",'all enabled installed mods included','installed source digests match selected build','installed mod exactly matches distributed artifact','packaged JSON and table/framed declarations validated','focused client slot admission checks passed without a game process'],
 'artifact':str(artifact),'artifact_sha256':hashlib.sha256(artifact.read_bytes()).hexdigest(),
 'block_template_audit':{'blocks':len(audit['blocks']),'model_states_checked':audit['model_states_checked']},
 'resource_probe':probe,
 'pending':[f'user in-game verification of {version}: custom framed appearance on first placement frame, including offhand and last item; immediate rejection of invalid drawer/framing slots during click, Shift, number/offhand swap and drag; valid upgrade install/remove and framing output']}
(root/f'docs/build-all-{version}.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n','utf-8')
print(json.dumps({'selected':selected,'mods':actual,'left_out':[],'source_digests_match':True,'artifact':str(artifact)},ensure_ascii=False,indent=2))
