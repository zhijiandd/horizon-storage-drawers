"""Check declared states against upstream model variants, without launching the game."""
from pathlib import Path
from itertools import product
import json

root=Path(__file__).resolve().parents[1]
content=root/'content'
version=json.loads((root/'native/horizon-native.json').read_text('utf-8'))['version']
report={'version':version,'game_launched':False,'blocks':[]}
for path in sorted((content/'data/storagedrawers/horizon/block').glob('*.json')):
    declaration=json.loads(path.read_text('utf-8'))
    assert declaration['template']!='minecraft:loom',path.stem
    if path.stem.startswith('framed_'):assert declaration['properties']['occludes'] is False,path.stem
    if 'drawers_'in path.stem:
        assert declaration['default_state']['hopper']=='false'
        for facing in ['north','south','west','east']:
            boxes=declaration['collision_shapes']['values'][facing+',true']
            contains=lambda p:any(all(box[i]<p[i]<box[i+3]for i in range(3))for box in boxes)
            assert not contains([.5,.90,.5]),('hopper mouth blocked',path.stem,facing)
            assert contains([.5,.70,.5]),('hopper floor missing',path.stem,facing)
            volume=sum((box[3]-box[0])*(box[4]-box[1])*(box[5]-box[2])for box in boxes)
            assert abs(volume-3324/4096)<1e-10,('Java hopper shape volume mismatch',path.stem,facing,volume)
    if path.stem.startswith('keybutton_'):
        values={'face':['floor','wall','ceiling'],'facing':['north','south','west','east'],'powered':['false','true']}
        assert declaration['template']=='minecraft:stone_button'
        assert len(declaration['shapes']['values'])==24
    else:
        assert declaration['template']=='minecraft:stone'
        values={k:v['values']for k,v in declaration['state_properties'].items()}
    states=[dict(zip(values,combo))for combo in product(*values.values())]
    models=json.loads((content/f'assets/storagedrawers/blockstates/{path.stem}.json').read_text('utf-8'))['variants']
    for state in states:
        matching=[model for rule,model in models.items()
                  if all(state.get(k) in v.split('|')for k,v in (pair.split('=',1)for pair in rule.split(',')if pair))]
        assert matching,('no upstream model variant',path.stem,state)
        for choices in matching:
            for model in choices if isinstance(choices,list)else [choices]:
                namespace,name=model['model'].split(':',1)
                if namespace=='storagedrawers':assert (content/f'assets/{namespace}/models/{name}.json').is_file(),model
    report['blocks'].append({'id':path.stem,'template':declaration['template'],'model_states_checked':len(states)})
report['model_states_checked']=sum(row['model_states_checked']for row in report['blocks'])
(root/f'docs/block-template-audit-{version}.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n','utf-8')
print(f"{len(report['blocks'])} blocks, {report['model_states_checked']} model states checked; no loom templates; no game launched.")
