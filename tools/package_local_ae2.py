"""Package the local 9f42064 compatibility changes; preserve all other files."""
from pathlib import Path
from zipfile import ZipFile, ZIP_DEFLATED
import hashlib
import json

root = Path(__file__).resolve().parents[1]
original = root / 'docs/compat-backups/ae2-15.4.18-original.hnmod'
stage = root / 'scratch/all-mods-9f-compat/ae2'
output = root / 'dist/ae2-15.4.18-local-9f.hnmod'
changed = {'mixins/ae2/' + name: (stage / name).read_bytes()
           for name in ['horizon-mixin.toml', 'me_embedded_content.rs', 'me.rs']}
with ZipFile(original) as archive:
    manifest = json.loads(archive.read('horizon-native.json'))
    manifest['horizon'] = '9f42064c11efe7ef3e5ed2baad7ddc2a3b36eea0'
    changed['horizon-native.json'] = (json.dumps(manifest, indent=2) + '\n').encode()
    with ZipFile(output, 'w', ZIP_DEFLATED, compresslevel=9) as package:
        for name in archive.namelist():
            package.writestr(name, changed.get(name, archive.read(name)))
with ZipFile(output) as package:
    assert package.testzip() is None
installed = Path(r'F:\Games\Horizon\game\horizon-native-mods\ae2.hnmod')
installed.write_bytes(output.read_bytes())
report = {'base': manifest['horizon'], 'ae2': manifest['version'],
          'patched_files': list(changed), 'backup': str(original),
          'sha256': hashlib.sha256(output.read_bytes()).hexdigest()}
(root / 'docs/ae2-local-9f-compat.json').write_text(json.dumps(report, indent=2), encoding='utf-8')
print('Installed local AE2 compatibility package.')
