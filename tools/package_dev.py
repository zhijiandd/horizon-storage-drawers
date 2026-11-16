"""Create explicitly labelled development artifacts; never include game assets or build caches."""
from pathlib import Path
from zipfile import ZipFile, ZIP_DEFLATED
import hashlib, json, shutil

root = Path(__file__).resolve().parents[1]
dist = root / 'dist'
dist.mkdir(exist_ok=True)
ref = root.parent / 'references' / 'StorageDrawers-1.21'
shutil.copy2(ref / 'LICENSE', root / 'LICENSE-StorageDrawers.txt')
bundle = root / 'native' / 'mixins' / 'storagedrawers'
shutil.copy2(ref / 'LICENSE', bundle / 'LICENSE-StorageDrawers.txt')
shutil.copy2(root / 'README.md', bundle / 'README.md')

def write_zip(path, entries):
    with ZipFile(path, 'w', ZIP_DEFLATED, compresslevel=9) as archive:
        seen=set()
        for name, source in sorted(entries):
            if name in seen: raise ValueError(f'Duplicate path: {name}')
            seen.add(name)
            archive.write(source,name)
    with ZipFile(path) as archive:
        if archive.testzip() is not None: raise ValueError('Corrupt archive')
    return {'file':str(path),'bytes':path.stat().st_size,'sha256':hashlib.file_digest(path.open('rb'),'sha256').hexdigest()}

entries=[('horizon-native.json',root/'native/horizon-native.json')]
for path in (root/'content').rglob('*'):
    if path.is_file() and (path.name=='horizon-mod.toml' or path.relative_to(root/'content').parts[0] in ['assets','data']):
        entries.append((path.relative_to(root/'content').as_posix(),path))
for path in bundle.rglob('*'):
    if path.is_file(): entries.append(('mixins/storagedrawers/'+path.relative_to(bundle).as_posix(),path))
mod=write_zip(dist/'storagedrawers-horizon-0.1.0-dev.hmod',entries)

source=[]
for folder in ['src','tests','tools','docs','native','content']:
    for path in (root/folder).rglob('*'):
        if path.is_file() and '__pycache__' not in path.parts:source.append((path.relative_to(root).as_posix(),path))
for name in ['Cargo.toml','Cargo.lock','README.md','LICENSE-StorageDrawers.txt','.gitignore']:
    path=root/name
    if path.exists(): source.append((name,path))
src=write_zip(dist/'storagedrawers-horizon-0.1.0-dev-source.zip',source)
(dist/'SHA256SUMS').write_text('\n'.join(f"{x['sha256']}  {Path(x['file']).name}" for x in [mod,src])+'\n',encoding='ascii')
print(json.dumps({'development_only':True,'mod':mod,'source':src},ensure_ascii=False,indent=2))
