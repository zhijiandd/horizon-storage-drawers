"""Exercise the real integrated server through Horizon's control CLI, in a scratch world only."""
from pathlib import Path
import json,re,struct,subprocess,sys,time
from concurrent.futures import ThreadPoolExecutor

ROOT=Path(__file__).resolve().parents[1]
CTL=ROOT.parent/'tooling/horizon-target/fast/horizon-ctl.exe'
RUN=Path((ROOT/'docs/active-regression-root.txt').read_text().strip())
assert RUN.resolve().is_relative_to((ROOT/'scratch').resolve())
SOCKET=RUN/'control.json'
TRANSCRIPT=[]

def ctl(*args):
    p=subprocess.run([str(CTL),'--socket',str(SOCKET),*map(str,args)],capture_output=True,encoding='utf-8',timeout=35)
    lines=p.stdout.strip().splitlines()
    value=json.loads(lines[-1]) if lines else {'error':p.stderr.strip()}
    TRANSCRIPT.append({'args':args,'result':value})
    if p.returncode or value.get('error'):raise RuntimeError(f'{args}: {value}')
    return value

def command(line):return ctl('command',line)
def pause(ticks=3):ctl('wait',ticks)
def pos_command(verb,pos,end=''):return command(f'{verb} {" ".join(map(str,pos))} {end}'.strip())
def give(item,count=1):command(f'give @s {item} {count}');pause(2);ctl('select',item)
def clear():command('clear @s');pause(3)
def interact(pos=(0,100,0),face='north'):return ctl('interact',*pos,face)
def aim(x,y,z):ctl('look-at',x,y,z);pause(2)
def blocks(pos,name):pos_command('setblock',pos,'storagedrawers:'+name+'[facing=north]');pause()

def read_drawer(pos=(0,100,0)):
    text=' '.join(pos_command('data get block',pos,'sd_state').get('chat',[]))
    raw=re.search(r'\[B;([^]]+)\]',text)
    if not raw:raise AssertionError(f'No drawer save returned: {text[:200]}')
    data=bytearray(int(x)&255 for x in re.findall(r'-?\d+',raw[1]))
    assert data[:8]==b'SDRH\x01\0\0\0'
    at=8
    def take(n):
        nonlocal at
        if at+n>len(data):
            def read_byte(index):
                answer=pos_command('data get block',pos,f'sd_state[{index}]')
                text=' '.join(answer.get('chat',[]))
                value=re.search(r'(-?\d+)[bB]$',text)
                if not value:raise AssertionError(f'Cannot read byte {index}: {text}')
                return int(value[1])&255
            with ThreadPoolExecutor(max_workers=8) as pool:
                data.extend(pool.map(read_byte,range(len(data),at+n)))
        b=data[at:at+n];at+=n;return b
    def byte():return take(1)[0]
    def number():return struct.unpack('<Q',take(8))[0]
    def text():return take(number()).decode()
    def item():return {'id':text(),'components':text(),'max_stack':number()}
    result={'slots':byte(),'half':bool(byte()),'drawers':[]}
    for _ in range(result['slots']):
        detached=bool(byte());i=item() if byte() else None
        result['drawers'].append({'detached':detached,'item':i,'count':number()})
    if byte():
        result['views']=[{'item':item(),'factor':number()} for _ in range(byte())]
        result['units']=number()
    result['upgrades']=[text() for _ in range(byte())]
    for flag in ['locked','quantified','shrouded','suspended']:result[flag]=bool(byte())
    result['priority']=byte()-2
    result['owner']=int.from_bytes(take(16),'little') if byte() else None
    return result

def total(item):return ctl('inventory').get('counts',{}).get(item,0)
def test(name,fn):
    try:fn();REPORT.append({'name':name,'passed':True});print('PASS',name,flush=True)
    except Exception as e:REPORT.append({'name':name,'passed':False,'error':str(e)});print('FAIL',name,str(e),flush=True);raise
    finally:
        (ROOT/'docs/game-regression-results.json').write_text(json.dumps({'run':str(RUN),'tests':REPORT},ensure_ascii=False,indent=2),encoding='utf-8')
        (RUN/'regression-transcript.json').write_text(json.dumps(TRANSCRIPT,ensure_ascii=False,indent=2),encoding='utf-8')

REPORT=[]
def setup():
    ctl('options-set','tutorialStep','none');ctl('options-set','lang','zh_cn')
    command('gamemode creative');command('tp @s 0.5 100 -3.5');pause(20)
    command('fill -7 99 -7 7 99 7 minecraft:stone');command('fill -6 100 -2 6 104 3 minecraft:air');pause(10)
    command('tp @s 0.5 100 -3.5');pause(10)
    command('time set noon');command('weather clear');clear()

def standard():
    blocks((0,100,0),'oak_full_drawers_1');give('minecraft:diamond',64);interact();pause()
    assert read_drawer()['drawers'][0]['count']==64
    assert total('minecraft:diamond')==0
    aim(.5,100.5,.0625);ctl('mouse','left','click');pause()
    assert read_drawer()['drawers'][0]['count']==63
    assert total('minecraft:diamond')==1
    ctl('key','shift','down');ctl('mouse','left','click');ctl('key','shift','up');pause()
    assert read_drawer()['drawers'][0]['count']==0
    assert total('minecraft:diamond')==64

def double_click():
    clear();give('minecraft:diamond',256);interact();interact();pause()
    assert read_drawer()['drawers'][0]['count']==256
    assert total('minecraft:diamond')==0

def upgrades():
    clear();give('storagedrawers:iron_storage_upgrade',1);interact();pause()
    assert read_drawer()['upgrades']==['iron_storage_upgrade']
    assert total('storagedrawers:iron_storage_upgrade')==1,'Creative upgrade consumed'
    command('gamemode survival');pause();give('storagedrawers:gold_storage_upgrade',1);interact();pause()
    assert total('storagedrawers:gold_storage_upgrade')==0
    assert read_drawer()['upgrades']==['iron_storage_upgrade','gold_storage_upgrade']
    command('gamemode creative');pause()
    give('storagedrawers:fill_level_upgrade',1);interact();pause()
    give('storagedrawers:quantify_key',1);interact();pause(6)
    assert read_drawer()['quantified']
    give('storagedrawers:drawer_key',1);interact();pause(6)
    assert read_drawer()['locked']

def compacting():
    clear();blocks((2,100,0),'compacting_drawers_3')
    command('tp @s 2.5 100 -3.5');pause(8)
    give('minecraft:iron_ingot',64);interact((2,100,0));pause()
    g=read_drawer((2,100,0));assert g['units']==576
    assert [v['item']['id'] for v in g['views']]==['minecraft:iron_block','minecraft:iron_ingot','minecraft:iron_nugget']
    aim(2.5,100.75,.0625);ctl('mouse','left','click');pause()
    assert read_drawer((2,100,0))['units']==495
    assert total('minecraft:iron_block')==1
    assert 'slots=open3' in ctl('block',2,100,0)['state']

def four_slots():
    clear();blocks((-2,100,0),'spruce_full_drawers_4')
    command('tp @s -1.5 100 -3.5');pause(8)
    points=[(-1.25,100.75),(-1.75,100.75),(-1.25,100.25),(-1.75,100.25)]
    items=['minecraft:diamond','minecraft:emerald','minecraft:redstone','minecraft:coal']
    for (x,y),item in zip(points,items):
        give(item,16);aim(x,y,.0625);ctl('mouse','right','click');pause()
    g=read_drawer((-2,100,0));assert [d['item']['id'] for d in g['drawers']]==items
    assert [d['count'] for d in g['drawers']]==[16]*4
    clear();aim(*points[0],.0625);ctl('mouse','left','click');pause()
    assert total('minecraft:diamond')==1

if __name__=='__main__':
    test('scratch world setup',setup)
    test('single and stack withdrawal conserve stock',standard)
    test('double click deposits matching inventory',double_click)
    test('creative and survival upgrades, quantity and lock',upgrades)
    test('compacting ingot insertion and block withdrawal',compacting)
    test('four drawer face regions match rendered slots',four_slots)
