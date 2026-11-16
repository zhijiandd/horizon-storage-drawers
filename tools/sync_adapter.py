"""Update only the transformed integration module during local iteration; leave game sources intact."""
from pathlib import Path
import shutil
root=Path(__file__).resolve().parents[1]
stage=root/'scratch/game-dev2'
game=stage/'crates/horizon-client/src/game.rs'
text=game.read_text(encoding='utf-8')
marker='pub(crate) mod sd_integration {'
at=text.index(marker)
# This injected module is last. Keep all earlier hook transformations byte-for-byte.
assert text.rstrip().endswith('}') and text.count(marker)==1
module=(root/'native/mixins/storagedrawers/integration.rs').read_text(encoding='utf-8')
game.write_text(text[:at]+marker+'\n'+module+'\n}\n',encoding='utf-8')
shutil.copy2(root/'native/mixins/storagedrawers/data/content.hmod',stage/'native-data/storagedrawers/content.hmod')
print('Updated transformed adapter and embedded content')
