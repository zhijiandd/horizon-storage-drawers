"""Stage installed bundles for a compatibility-only patch, keeping originals."""
from pathlib import Path
from zipfile import ZipFile
import json
import re

ROOT = Path(__file__).resolve().parents[1]
INSTALLED = Path(r'F:\Games\Horizon\game\horizon-native-mods')
STAGED = ROOT / 'scratch/all-mods-9f-compat'
BACKUPS = ROOT / 'docs/compat-backups'
BACKUPS.mkdir(parents=True, exist_ok=True)
for package in sorted(INSTALLED.glob('*.hnmod')):
    original = BACKUPS / 'ae2-15.4.18-original.hnmod'
    source = original if package.stem == 'ae2' and original.exists() else package
    with ZipFile(source) as archive:
        manifest = json.loads(archive.read('horizon-native.json'))
        mod_id = manifest['id']
        prefix = f'mixins/{mod_id}/'
        destination = STAGED / mod_id
        for name in archive.namelist():
            if not name.startswith(prefix) or name.endswith('/'):
                continue
            target = destination / name.removeprefix(prefix)
            if not target.resolve().is_relative_to(destination.resolve()):
                raise ValueError('Unsafe archive path')
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(archive.read(name))
        if mod_id == 'ae2':
            backup = BACKUPS / 'ae2-15.4.18-original.hnmod'
            if not backup.exists():
                backup.write_bytes(package.read_bytes())
            path = destination / 'horizon-mixin.toml'
            text = path.read_text('utf-8')
            replacements = {
                "horizon_client::game::Game<'_>::clicked_outside":
                    "horizon_client::game::inventory_input::Game<'_>::clicked_outside",
                "horizon_client::game::machines::Game<'_>::hopper_eject":
                    "horizon_client::game::machines::Game<'_>::hopper_eject_with_state",
            }
            for old, new in replacements.items():
                assert text.count(old) == 1
                text = text.replace(old, new)
            # The resource loader gained the disabled-content set on this base.
            pattern = r'(?ms)^\[\[injection\]\].*?(?=^\[\[|\Z)'
            def content_binding(match):
                chunk = match.group(0)
                if not re.search(r'id\s*=\s*"me-embedded-content"', chunk):
                    return chunk
                addition = '  { name="disabled", ty="&std::collections::BTreeSet<String>", value="disabled" },\n'
                chunk, count = re.subn(r'(?m)^\](\s*)$', addition + r']\1', chunk)
                assert count == 1
                return chunk
            text = re.sub(pattern, content_binding, text)
            path.write_text(text, encoding='utf-8')
            helper = destination / 'me_embedded_content.rs'
            body = helper.read_text('utf-8')
            old = 'dev: &[std::path::PathBuf])'
            assert body.count(old) == 1
            body = body.replace(old, 'dev: &[std::path::PathBuf], disabled: &std::collections::BTreeSet<String>)')
            old = 'content_root(root, enabled, &with)'
            assert body.count(old) == 1
            helper.write_text(body.replace(old, 'content_root(root, enabled, disabled, &with)'), encoding='utf-8')
            implementation = destination / 'me.rs'
            body = implementation.read_text('utf-8')
            old = 's.tiny_tnts.push(super::tnt::PrimedTnt {'
            assert body.count(old) == 2
            body = body.replace(old, old + '''
                used_portal: false,
                fall_distance: 0.0,
                stuck: horizon_core::geometry::Vec3::ZERO,
                in_water: false,''')
            old = 'game.world.block_names.insert(key, item_words(&id));'
            assert body.count(old) == 1
            body = body.replace(old, 'game.world.block_names.insert(key, horizon_world::nbt::Tag::String(item_words(&id).as_str().into()));')
            implementation.write_text(body, encoding='utf-8')
        path = destination / 'horizon-mixin.toml'
        text = path.read_text('utf-8')
        text, count = re.subn(r'(?m)^(horizon_revision\s*=\s*)"[a-f0-9]{40}"',
                            r'\1"9f42064c11efe7ef3e5ed2baad7ddc2a3b36eea0"', text)
        assert count == 1
        path.write_text(text, encoding='utf-8')
print(STAGED)
