"""Install the locally compiled single-mod build, using Horizon's native version metadata format."""
from pathlib import Path
from zipfile import ZipFile
import hashlib,json,os,shutil,struct

ROOT=Path(__file__).resolve().parents[1]
HOME=Path(r'F:\Games\Horizon').resolve()
VERSIONS=HOME/'data/versions'
PREVIOUS=json.loads((ROOT/'docs/install-plan.json').read_text(encoding='utf-8'))
BASE=PREVIOUS['version']
EXE=ROOT.parent/'tooling/horizon-target/fast/horizon-client.exe'
manifest=json.loads((ROOT/'native/horizon-native.json').read_text(encoding='utf-8'))
MOD=ROOT/f"dist/storagedrawers-horizon-{manifest['version']}.hnmod"
with ZipFile(MOD) as z:
    files={n:z.read(n) for n in sorted(z.namelist()) if not n.endswith('/')}
    assert json.loads(files['horizon-native.json'])==manifest
def digest(entries):
    h=hashlib.sha256()
    for name,data in entries:
        h.update(name.encode());h.update(struct.pack('<Q',len(data)));h.update(data)
    return h.hexdigest()
bundle_prefix='mixins/storagedrawers/'
bundle=[(n.removeprefix(bundle_prefix),b) for n,b in files.items() if n.startswith(bundle_prefix)]
h=hashlib.sha256()
for part in [manifest['mixins'],'fast','profile.fast.package.horizon-client.opt-level=0','horizon-client/mods','1.99.1',manifest['horizon']]:
    h.update(part.encode());h.update(b'\0')
h.update(b'storagedrawers\0')
for name,data in bundle:
    h.update(name.encode());h.update(struct.pack('<Q',len(data)));h.update(data)
version=f"native-{manifest['horizon'][:7]}-{h.hexdigest()[:12]}"
destination=VERSIONS/version
assert destination.resolve().is_relative_to(VERSIONS.resolve())
assert not destination.exists(),f'Existing destination: {destination}'
assert EXE.is_file() and (VERSIONS/BASE/'.installed').exists()
def copy(src,dst):
    try:os.link(src,dst)
    except OSError:shutil.copy2(src,dst)
skip={'horizon-client.exe','.installed','native-mods.json','native-data'}
shutil.copytree(VERSIONS/BASE,destination,copy_function=copy,
    ignore=lambda p,n:[x for x in n if Path(p)==VERSIONS/BASE and x in skip])
shutil.copy2(EXE,destination/'horizon-client.exe')
origin=json.loads((VERSIONS/BASE/'native-mods.json').read_text(encoding='utf-8'))['base']
about={'base':origin,'horizon':manifest['horizon'],'base_horizon':manifest['horizon'],
    'pinned_by':[],'on_base':True,'horizon_mixins':manifest['mixins'],'hooks':None,
    'rust':'1.99.1','profile':'dev','target':'client','rust_target':'x86_64-pc-windows-gnu',
    'mods':[{'id':'storagedrawers','version':manifest['version'],'sha256':digest(files.items())}]}
(destination/'native-mods.json').write_text(json.dumps(about,indent=2)+'\n',encoding='utf-8')
(destination/'.installed').write_text('',encoding='ascii')
plan={'version':version,'destination':str(destination),'mod':str(MOD),
    'binary_sha256':hashlib.sha256(EXE.read_bytes()).hexdigest(),'native_mod_digest':about['mods'][0]['sha256']}
(ROOT/'docs/install-plan.json').write_text(json.dumps(plan,indent=2)+'\n',encoding='utf-8')
print(json.dumps(plan,indent=2))
