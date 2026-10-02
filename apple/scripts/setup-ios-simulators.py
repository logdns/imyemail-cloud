#!/usr/bin/env python3
"""Create/reuse two isolated development simulators; never erase an existing device."""
import json
import subprocess
from pathlib import Path


def simctl(*args):
    return subprocess.check_output(['xcrun', 'simctl', *args], text=True)


state = json.loads(simctl('list', '--json'))
runtimes = [r for r in state['runtimes'] if r.get('isAvailable') and r['identifier'].endswith('iOS-' + r['version'].replace('.', '-'))]
if not runtimes:
    raise SystemExit('Install an iOS Simulator runtime in Xcode first.')
runtime = max(runtimes, key=lambda r: tuple(int(n) for n in r['version'].split('.')))
devices = state['devices'].get(runtime['identifier'], [])
result = {'runtime': runtime['identifier']}
for family in ['iPhone', 'iPad']:
    name = 'chck iOS Dev ' + family
    existing = next((d for d in devices if d['name'] == name and d.get('isAvailable')), None)
    if existing:
        udid = existing['udid']
    else:
        # Use a device type supported by the selected runtime, not an arbitrary newest model.
        supported = runtime.get('supportedDeviceTypes', state['devicetypes'])
        candidates = [d for d in supported if d['name'].startswith(family)]
        if not candidates:
            raise SystemExit('No supported ' + family + ' device type.')
        device_type = candidates[0]['identifier']
        udid = simctl('create', name, device_type, runtime['identifier']).strip()
    result[family.lower()] = udid
path = Path(__file__).resolve().parents[1] / '.build-ios-devices.json'
path.write_text(json.dumps(result, indent=2) + '\n')
print(path.read_text(), end='')
