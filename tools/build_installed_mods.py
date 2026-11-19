"""Build all installed native mods; reject the launcher's partial-build fallback."""
from pathlib import Path
import json
import os
import subprocess
import tempfile
import hashlib
import argparse
import signal
from zipfile import ZipFile

root = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--game-dir', type=Path, default=os.environ.get('HORIZON_HOME'), required='HORIZON_HOME' not in os.environ)
parser.add_argument('--launcher-home', type=Path, default=os.environ.get('HORIZON_LAUNCHER_HOME'), required='HORIZON_LAUNCHER_HOME' not in os.environ)
parser.add_argument('--launcher', type=Path, required=True, help='path to the Horizon launcher executable')
parser.add_argument('--base-version', required=True, help='installed Horizon base version to build against')
parser.add_argument('--instance', default='horizon')
parser.add_argument('--profile', help='optional launcher-supported build profile; omit to use its default')
parser.add_argument('--offline', action='store_true', help='use only cached Cargo dependencies')
args = parser.parse_args()
game = Path(args.game_dir).expanduser().resolve()
launcher_home = Path(args.launcher_home).expanduser().resolve()
launcher = args.launcher.expanduser().resolve()
if not launcher.is_file() or not game.is_dir() or not launcher_home.is_dir():
    parser.error('launcher executable, game directory, and launcher home must exist')
help_result = subprocess.run([str(launcher), 'native-build', '--help'], capture_output=True,
                             text=True, encoding='utf-8', errors='replace', timeout=30)
if 'horizon-mixin.toml' not in help_result.stdout:
    parser.error('This package uses legacy source mixins. The selected launcher uses the new SDK format; '
                 'pass a compatible legacy build tool via --launcher. Do not replace the installed launcher.')
os.environ.update(HORIZON_LAUNCHER_HOME=str(launcher_home), HORIZON_HOME=str(game),
                  HORIZON_LAUNCHER_NO_SELF_UPDATE='1')
if args.offline:
    os.environ['CARGO_NET_OFFLINE'] = 'true'
version = json.loads((root / 'native/horizon-native.json').read_text('utf-8'))['version']
flags = dict(line.split(':', 1) for line in (game / 'horizon-mods.txt').read_text('utf-8').splitlines() if ':' in line)
# An explicit checked bundle folder prevents the launcher's registry fallback
# from replacing an installed local compatibility patch with an older release.
(root / 'scratch').mkdir(exist_ok=True)
(root / 'docs').mkdir(exist_ok=True)
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
command = [str(launcher), 'native-build', str(snapshot),
        '--instance', args.instance, '--version', args.base_version,
        '--trust-native-code']
if args.profile:
    command.extend(['--profile', args.profile])
process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                           text=True, encoding='utf-8', errors='replace', start_new_session=os.name != 'nt')
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
            if os.name == 'nt':
                subprocess.run(['taskkill', '/PID', str(process.pid), '/T', '/F'],
                               stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            else:
                os.killpg(process.pid, signal.SIGTERM)
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
