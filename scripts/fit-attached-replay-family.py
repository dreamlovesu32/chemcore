import json
import sys
from pathlib import Path
from sklearn.compose import ColumnTransformer
from sklearn.pipeline import Pipeline
from sklearn.preprocessing import OneHotEncoder
from sklearn.tree import DecisionTreeRegressor, export_text

if len(sys.argv) != 2:
    print('usage: fit-attached-replay-family.py <attached-knockout-geometry.json>')
    raise SystemExit(2)

rows = json.loads(Path(sys.argv[1]).read_text(encoding='utf-8'))['rows']
X=[]; y=[]
for r in rows:
    if not str(r.get('layout','')).startswith('attached-group'):
        continue
    gap_right = -r['overhangToComponent']['right']
    replay_l1 = sum(abs(v) for v in r['replayDeltaDims'] + r['replayDeltaTopLeft'])
    X.append({
        'text': r['text'],
        'fill': r['fill'],
        'cdxmlLabelAlignment': r['cdxmlLabelAlignment'] or 'None',
        'cdxmlLabelJustification': r['cdxmlLabelJustification'] or 'None',
        'componentHalfX': r['componentHalfX'],
        'componentHalfY': r['componentHalfY'],
        'componentQuadrant': r['componentQuadrant'],
        'primaryNeighborBucket': r['primaryNeighborBucket'],
        'nodeType': r['nodeType'] or 'None',
        'sideX': r['sideX'],
        'sideY': r['sideY'],
        'gapRight': gap_right,
    })
    y.append(replay_l1)

features = list(X[0].keys())
cat = [f for f in features if f != 'gapRight']
rows_matrix = [[x[f] for f in features] for x in X]
pre = ColumnTransformer([
    ('cat', OneHotEncoder(handle_unknown='ignore'), [features.index(f) for f in cat]),
    ('num', 'passthrough', [features.index('gapRight')]),
])
reg = DecisionTreeRegressor(max_depth=3, min_samples_leaf=1, random_state=0)
pipe = Pipeline([('pre', pre), ('reg', reg)])
pipe.fit(rows_matrix, y)
feature_names = list(pipe.named_steps['pre'].get_feature_names_out())
report = {
    'featureOrder': features,
    'r2': pipe.score(rows_matrix, y),
    'tree': export_text(pipe.named_steps['reg'], feature_names=feature_names),
}
print(json.dumps(report, ensure_ascii=False, indent=2))
