"""Compare a Horizon Git checkout with the revision targeted by this mod."""
import argparse
import json
import subprocess
from pathlib import Path

root = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('source', type=Path, help='path to a Horizon Git checkout')
args = parser.parse_args()
expected = json.loads((root / 'native/horizon-native.json').read_text('utf-8'))['horizon']
source = args.source.expanduser().resolve()
result = subprocess.run(['git', '-C', str(source), 'rev-parse', '--verify', 'HEAD'],
                        text=True, capture_output=True)
if result.returncode:
    parser.error(f'{source} is not a readable Git checkout')
actual = result.stdout.strip()
if actual != expected:
    print(f'Horizon revision differs: expected {expected}, found {actual}. Compatibility is not verified.')
    raise SystemExit(1)
print(f'Horizon revision matches: {actual}.')
