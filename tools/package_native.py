"""Package the single-file native mod; Mojang resources and compiled game stay local."""
from pathlib import Path
from zipfile import ZipFile,ZIP_DEFLATED
import hashlib,json,shutil
ROOT=Path(__file__).resolve().parents[1]
BUNDLE=ROOT/'native/mixins/storagedrawers'
DIST=ROOT/'dist';DIST.mkdir(exist_ok=True)
manifest=json.loads((ROOT/'native/horizon-native.json').read_text(encoding='utf-8'))
version=manifest['version']
shutil.copy2(ROOT.parent/'references/StorageDrawers-1.21/LICENSE',BUNDLE/'LICENSE-StorageDrawers.txt')
shutil.copy2(ROOT/'README.md',BUNDLE/'README.md')
path=DIST/f'storagedrawers-horizon-{version}.hnmod'
with ZipFile(path,'w',ZIP_DEFLATED,compresslevel=9) as archive:
    archive.write(ROOT/'native/horizon-native.json','horizon-native.json')
    for file in sorted(BUNDLE.rglob('*')):
        if file.is_file():archive.write(file,'mixins/storagedrawers/'+file.relative_to(BUNDLE).as_posix())
with ZipFile(path) as archive:
    assert archive.testzip() is None
    assert len(archive.namelist())<512
    assert sum(x.file_size for x in archive.infolist())<64*1024*1024
sha=hashlib.sha256(path.read_bytes()).hexdigest()
(DIST/'SHA256SUMS').write_text(f'{sha}  {path.name}\n',encoding='ascii')
print(json.dumps({'file':str(path),'bytes':path.stat().st_size,'sha256':sha},indent=2))
