# ChemSema Document Patch v1

Protocol id: `chemsema.document.patch.v1`. Document Patch is revision-bounded UI synchronization. It is not persisted in `.ccjs` or `.ccjz` and is not undo history.

```json
{
  "beforeRevision": 4,
  "revision": 5,
  "upsertEntities": [{
    "entity": { "id": "obj_1", "type": "text", "payload": {} },
    "parentId": "group_1",
    "childIds": []
  }],
  "deletedEntityIds": [],
  "hierarchyRoots": ["group_1"],
  "upsertResources": {},
  "relationScopeEntityIds": ["obj_1"],
  "relations": [],
  "upsertStyles": {},
  "deletedStyleIds": [],
  "logicalObjects": {},
  "reactionSchemes": [],
  "chemicalProperties": [],
  "orders": { "reading": [] }
}
```

Rules:

- Apply only to the matching beforeRevision; a gap requires a full snapshot refresh.
- Entity records are flat. Parent and child IDs update the disposable runtime hierarchy.
- `hierarchyRoots` is present when scene structure may have changed.
- Replace relations touching the relation scope with the supplied current relations.
- Upsert resource and style records before requesting target rendering.
- The semantic sections and `orders` are optional replacement snapshots and
  appear only when the corresponding command target class may have changed.
- Remove deleted IDs, apply upserts, then rebuild affected membership.
- Request render primitives only for command-result target IDs.
- Clients may fall back to full synchronization when the backend lacks this protocol.
