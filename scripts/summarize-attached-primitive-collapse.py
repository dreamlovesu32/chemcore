import json
import sys
from collections import Counter
from pathlib import Path

if len(sys.argv) != 3:
    print('usage: summarize-attached-primitive-collapse.py <payload.json> <all-text-primitives.json>')
    raise SystemExit(2)

payload_path = Path(sys.argv[1])
primitives_path = Path(sys.argv[2])
outer = json.loads(payload_path.read_text(encoding='utf-8'))
doc = json.loads(outer['chemcoreDocumentJson'])
prims = json.loads(primitives_path.read_text(encoding='utf-8-sig'))
prims_by_node = {p['nodeId']: p for p in prims if p.get('nodeId')}
rows = []
for n in doc['resources']['mol_cdxml_merged']['data']['nodes']:
    lab = n.get('label')
    if not lab:
        continue
    prim = prims_by_node.get(n['id'])
    rows.append({
        'nodeId': n['id'],
        'labelText': lab.get('text'),
        'layout': lab.get('layout'),
        'align': lab.get('align'),
        'anchor': lab.get('anchor'),
        'cdxmlLabelAlignment': lab.get('meta', {}).get('import', {}).get('cdxml', {}).get('labelAlignment'),
        'cdxmlLabelJustification': lab.get('meta', {}).get('import', {}).get('cdxml', {}).get('labelJustification'),
        'primitivePresent': prim is not None,
        'primitiveTextAnchor': None if prim is None else prim.get('textAnchor'),
        'primitiveBoxWidth': None if prim is None else prim.get('boxWidth'),
        'primitiveTextEmpty': None if prim is None else prim.get('text') == '',
        'primitiveRunCount': None if prim is None else len(prim.get('runs', [])),
        'primitiveRunTexts': None if prim is None else [r['text'] for r in prim.get('runs', [])],
    })

attached = [r for r in rows if str(r['layout']).startswith('attached-group')]
summary = {
    'attached_count': len(attached),
    'text_anchor_counts': dict(Counter(r['primitiveTextAnchor'] for r in attached)),
    'box_width_counts': dict(Counter(str(r['primitiveBoxWidth']) for r in attached)),
    'text_empty_counts': dict(Counter(r['primitiveTextEmpty'] for r in attached)),
    'alignment_cross': {
        f"{k[0]} -> {k[1]}": v
        for k, v in Counter((r['cdxmlLabelAlignment'], r['primitiveTextAnchor']) for r in attached).items()
    },
    'layout_cross': {
        f"{k[0]} -> {k[1]}": v
        for k, v in Counter((r['layout'], r['primitiveTextAnchor']) for r in attached).items()
    },
    'non_start_examples': [r for r in attached if r['primitiveTextAnchor'] != 'start'][:10],
    'non_null_box_examples': [r for r in attached if r['primitiveBoxWidth'] is not None][:10],
    'non_empty_text_examples': [r for r in attached if r['primitiveTextEmpty'] is False][:10],
}
print(json.dumps(summary, ensure_ascii=False, indent=2))
