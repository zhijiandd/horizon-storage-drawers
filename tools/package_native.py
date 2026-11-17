"""Package the single-file native mod; Mojang resources and compiled game stay local."""
from pathlib import Path
from zipfile import ZipFile,ZIP_DEFLATED
import argparse,hashlib,json,shutil
ROOT=Path(__file__).resolve().parents[1]
BUNDLE=ROOT/'native/mixins/storagedrawers'
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--output-dir',type=Path,default=ROOT/'dist',help='destination for the package and SHA256SUMS')
parser.add_argument('--overwrite',action='store_true',help='replace an existing package at the destination')
args=parser.parse_args()
DIST=args.output_dir.expanduser().resolve();DIST.mkdir(parents=True,exist_ok=True)
manifest=json.loads((ROOT/'native/horizon-native.json').read_text(encoding='utf-8'))
version=manifest['version']
path=DIST/f'storagedrawers-horizon-{version}.hnmod'
if path.exists() and not args.overwrite:
    parser.error(f'{path} already exists; use a new --output-dir or explicitly pass --overwrite')
shutil.copy2(ROOT/'LICENSE-StorageDrawers.txt',BUNDLE/'LICENSE-StorageDrawers.txt')
shutil.copy2(ROOT/'LICENSE',BUNDLE/'LICENSE')
shutil.copy2(ROOT/'README.md',BUNDLE/'README.md')
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
