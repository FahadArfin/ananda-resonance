"""Packaged native MCP acceptance. No browser, GUI automation or Node required."""
import json, os, pathlib, socket, subprocess, sys, time

exe = sys.argv[1] if len(sys.argv) > 1 else r'C:\Users\fahad\Applications\Ananda Control\ananda-control.exe'
process = subprocess.Popen([exe, '--mcp'], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, encoding='utf-8')
sequence = 0
checks = []
def rpc(method, params=None):
    global sequence
    sequence += 1
    process.stdin.write(json.dumps({'jsonrpc':'2.0','id':sequence,'method':method,'params':params or {}})+'\n'); process.stdin.flush()
    response = json.loads(process.stdout.readline())
    assert response['id'] == sequence, response
    return response
def call(name, args=None, fail=False):
    reply = rpc('tools/call', {'name':name,'arguments':args or {}})
    if fail:
        assert 'error' in reply or reply['result'].get('isError'), reply
        return reply
    assert 'error' not in reply and not reply['result'].get('isError'), reply
    return reply['result']['structuredContent']
def passed(name):
    checks.append(name); print('PASS',name,flush=True)

settings = None
original = None
temp_ids = []
try:
    assert rpc('initialize',{'protocolVersion':'2025-11-25','capabilities':{},'clientInfo':{'name':'acceptance','version':'1'}})['result']['protocolVersion'] == '2025-11-25'
    process.stdin.write('{"jsonrpc":"2.0","method":"notifications/initialized"}\n'); process.stdin.flush()
    assert len(rpc('tools/list')['result']['tools']) == 17
    passed('MCP initialize, notification and 17 tools')
    original = call('get_status'); settings = call('get_settings')['settings']
    assert original['ownershipOk'] and original['integrationInstalled']
    assert not original['mainWindowOpen'], original
    passed('Native status with main webview closed')
    profiles = call('list_profiles')['profiles']; assert len(profiles) == 6
    imported_values = {p['id']: call('get_profile',{'profile':p['id']}) for p in profiles}
    call('update_settings',{'notifications':False})
    result = call('manage_profile',{'action':'duplicate','profile':'music','name':'MCP acceptance temporary'})
    test_id = result['profileId']; temp_ids.append(test_id)
    p = call('set_preamp',{'profile':test_id,'gainDb':-8}); assert p['preamp'] == -8
    p = call('edit_filter',{'profile':test_id,'filterId':p['filters'][0]['id'],'gain':1.25}); assert p['filters'][0]['gain'] == 1.25
    p = call('add_filter',{'profile':test_id,'kind':'HPQ','frequency':25,'gain':0,'q':0.707}); fid=p['filters'][-1]['id']
    call('remove_filter',{'profile':test_id,'filterId':fid})
    passed('Create, patch preamp/filter, add and remove filter')
    for tool, args in [('set_preamp',{'profile':test_id,'gainDb':99}),('edit_filter',{'profile':test_id,'filterId':fid,'q':0}),('update_settings',{'agentControl':False}),('import_profile',{'name':'unsafe','format':'apo','content':'Preamp: -3 dB\nInclude: secret.txt'}),('get_status',{'path':'C:/secret'})]: call(tool,args,True)
    assert len(call('list_profiles')['profiles']) == 7
    passed('Invalid bounds, permission changes and unsafe imports rejected without partial import')
    exported = call('export_profile',{'profile':test_id,'format':'json'})
    copied = call('import_profile',{'name':'MCP roundtrip temporary','format':'json','content':exported['content']}); temp_ids.append(copied['id'])
    assert copied['filters'] == call('get_profile',{'profile':test_id})['filters']
    passed('Native JSON roundtrip')
    times=[]
    for profile in ['movies','stock',original['activeProfile']]:
        result=call('activate_profile',{'profile':profile}); assert result['activeProfile'] == profile and not result['mainWindowOpen']; times.append(result['status']['lastCommitMs'])
    call('set_eq',{'enabled':False}); assert call('get_status')['activeProfile']=='stock'
    call('set_eq',{'enabled':True}); assert call('get_status')['activeProfile']!='stock'
    passed('Tray-only profile switching and EQ off/on; commits ms '+str(times))
    endpoint=json.loads((pathlib.Path(os.environ['APPDATA'])/'com.anandacontrol.desktop/agent-bridge.json').read_text())
    with socket.create_connection(('127.0.0.1',endpoint['port']),timeout=3) as sock:
        sock.sendall(b'{"token":"wrong","name":"set_eq","arguments":{"enabled":false}}\n')
        assert 'Unauthorized' in json.loads(sock.makefile().readline())['error']
    passed('Unauthenticated native connection rejected')
    for p in profiles: assert call('get_profile',{'profile':p['id']}) == imported_values[p['id']]
    passed('All six original profiles preserved exactly')
finally:
    if original:
        # Restore both remembered enabled profile and current bypass/selection.
        call('activate_profile',{'profile':original['lastEnabledProfile']})
        call('activate_profile',{'profile':original['activeProfile']})
    for ident in temp_ids: call('manage_profile',{'action':'delete','profile':ident})
    if settings: call('update_settings',{'notifications':settings['notifications']})
    process.stdin.close(); process.wait(timeout=5)
pathlib.Path('test-results').mkdir(exist_ok=True)
pathlib.Path('test-results/mcp-acceptance.json').write_text(json.dumps({'checks':checks,'passed':len(checks)},indent=2))
