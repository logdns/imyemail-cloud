#!/usr/bin/env python3
"""校验并离线导出六语客户端文案。用法：python3 scripts/export-client-locales.py [--check]"""
from pathlib import Path
import json, re, sys
ROOT = Path(__file__).resolve().parents[1]
CODES = ('en', 'zh-TW', 'zh-CN', 'ja', 'fr', 'es')
catalogs = {code: json.loads((ROOT / 'brand/locales' / (code + '.json')).read_text()) for code in CODES}
keys = set(catalogs['en'])
for code, table in catalogs.items():
    assert set(table) == keys, f'{code}: keys differ'
    for key, value in table.items():
        assert value.strip(), f'{code}: empty {key}'
        assert sorted(re.findall(r'XPH\d+X', key)) == sorted(re.findall(r'XPH\d+X', value)), f'{code}: placeholders differ: {key}'
blob = json.dumps(catalogs, ensure_ascii=False, sort_keys=True, indent=2) + '\n'
outputs = {ROOT / p: blob for p in (
    'apple/Sources/ChckDesign/Localization/catalog.json',
    'windows/Chck.Mail.Core/Localization/catalog.json',
    'linux/data/locales.json',
)}
def escape(value):
    return ''.join({'\\':'\\\\', '\n':'\\n', '\r':'\\r', '\t':'\\t', ' ':'\\ ', ':':'\\:', '=':'\\=', '#':'\\#', '!':'\\!'}.get(c,c) for c in value)
for code, table in catalogs.items():
    outputs[ROOT / f'android/core/common/src/main/resources/locales/{code}.properties'] = '\n'.join(escape(k)+'='+escape(v) for k,v in sorted(table.items()))+'\n'
for path, contents in outputs.items():
    if '--check' in sys.argv:
        assert path.read_text() == contents, f'outdated export: {path}'
    else:
        path.parent.mkdir(parents=True,exist_ok=True)
        path.write_text(contents)
print(f'Validated {len(keys)} entries × {len(CODES)} languages; {len(outputs)} offline resources')
