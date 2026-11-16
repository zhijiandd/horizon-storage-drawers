"""Build all installed native mods; reject the launcher's partial-build fallback."""
from pathlib import Path
import json
import os
import subprocess
import tempfile
import hashlib
from zipfile import ZipFile

root = Path(__file__).resolve().parents[1]
os.environ.update(HORIZON_LAUNCHER_HOME=r'F:\Games\Horizon\data',
                  HORIZON_HOME=r'F:\Games\Horizon\game',
                  HORIZON_LAUNCHER_NO_SELF_UPDATE='1',
                  TEMP=r'F:\HorizonMods\tooling\temp', TMP=r'F:\HorizonMods\tooling\temp')
version = json.loads((root / 'native/horizon-native.json').read_text('utf-8'))['version']
game = Path(r'F:\Games\Horizon\game')
flags = dict(line.split(':', 1) for line in (game / 'horizon-mods.txt').read_text('utf-8').splitlines() if ':' in line)
# An explicit checked bundle folder prevents the launcher's registry fallback
# from replacing an installed local compatibility patch with an older release.
snapshot = Path(tempfile.mkdtemp(prefix=f'build-all-{version}-', dir=root / 'scratch'))
packages = []
for package in sorted((game / 'horizon-native-mods').glob('*.hnmod')):
    with ZipFile(package) as archive:
        mod_id = json.loads(archive.read('horizon-native.json'))['id']
        if flags.get('mod.' + mod_id, 'true') == 'false':
            continue
        packages.append((package,package.read_bytes(),json.loads(archive.read('horizon-native.json'))))
        destination = snapshot / mod_id
        for name in archive.namelist():
            if name.endswith('/'):
                continue
            target = destination / name
            if not target.resolve().is_relative_to(destination.resolve()):
                raise ValueError('Unsafe archive path')
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(archive.read(name))
args = [r'F:\Games\Horizon\launcher\horizon-launcher.exe', 'native-build', str(snapshot),
        '--instance', 'horizon', '--version', 'nightly-2026-11-13-9f42064',
        '--profile', 'dev', '--trust-native-code']
process = subprocess.Popen(args, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                           text=True, encoding='utf-8', errors='replace')
announced_compiling = False
with (root / f'docs/build-all-{version}.log').open('w', encoding='utf-8') as log:
    for line in process.stdout:
        log.write(line)
        log.flush()
        if 'Compiling the game' not in line or not announced_compiling:
            print(line.rstrip(), flush=True)
        if 'Compiling the game' in line:
            announced_compiling = True
        if 'was left out' in line or '已略过' in line:
            subprocess.run(['taskkill', '/PID', str(process.pid), '/T', '/F'],
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            raise SystemExit('Stopped: every installed mod must be included.')
result = process.wait()
if result == 0:
    chosen_path = game / 'mod-browser/chosen-versions.json'
    chosen = json.loads(chosen_path.read_text('utf-8')) if chosen_path.exists() else {}
    browser_path = game / 'mod-browser.json'
    browser = json.loads(browser_path.read_text('utf-8')) if browser_path.exists() else {'entries':[]}
    restored = False
    for path, content, manifest in packages:
        if path.read_bytes() != content:
            path.write_bytes(content)
            chosen[manifest['id']] = manifest['version']
            for entry in browser['entries']:
                if entry.get('project') == manifest['id']:
                    entry['version'] = entry['number'] = manifest['version']
                    entry['checksum'] = 'SHA-256:' + hashlib.sha256(content).hexdigest()
                    entry['verdict'] = 'Unsigned'
            restored = True
    if restored:
        chosen_path.parent.mkdir(parents=True,exist_ok=True)
        chosen_path.write_text(json.dumps(chosen,indent=2)+'\n',encoding='utf-8')
        browser_path.write_text(json.dumps(browser,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
        print('Preserved the installed local compatibility packages and their chosen versions.',flush=True)
print('BUILD_EXIT=' + str(result), flush=True)
raise SystemExit(result)
