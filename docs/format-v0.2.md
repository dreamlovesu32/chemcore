# CCJS Document Format v0.2

Status: current writing specification. Machine-readable schema: [`schemas/ccjs-v0.2.schema.json`](../schemas/ccjs-v0.2.schema.json). The detailed normative contract is in [the Chinese specification](format-v0.2.zh-CN.md); design rationale and remaining stable-release gates are in the [architecture rationale](ccjs-architecture-and-format-rationale.zh-CN.md) and [stability contract](ccjs-v0.2-stability-architecture.zh-CN.md).

CCJS is ChemSema's source-neutral editable chemical-document snapshot. It represents page scene entities, molecular graphs, styles, resources, containment, typed relations, reaction semantics, and lossless interchange data. It is not a JSON spelling of CDXML.

```json
{
  "format": { "name": "chemsema", "version": "0.2", "unit": "pt", "profile": "snapshot" },
  "document": {},
  "style": {},
  "styles": {},
  "entities": { "scene": [] },
  "hierarchy": { "roots": [], "children": {} },
  "relations": [],
  "orders": { "reading": [] },
  "reactionSchemes": [],
  "chemicalProperties": [],
  "resources": {}
}
```

Normative invariants:

- Writers emit exactly the v0.2 header. Readers reject an unknown name, version, unit, or profile.
- `entities.scene` is flat. Every scene entity has a non-empty unique ID and no `children` field.
- Every scene ID occurs exactly once in containment: in roots or one parent's children.
- Containment is acyclic, all entities are reachable, and only a group may own children.
- References use IDs, never array positions. Style and resource references resolve.
- `zIndex` is the only persisted paint authority; stable IDs break ties.
- `orders.reading` contains existing unique scene IDs and does not control painting.
- Relations express typed cross-entity semantics, never containment. Registered kinds are `bracket-repeat-label`, `analysis-caption`, `atom-symbol`, `chemical-property-display`, and `annotation-basis`.
- Spatial and reverse indexes are derived revision-scoped caches, never snapshot truth.
- v0.1 is accepted only through explicit migration; successful writes always produce v0.2.

Byte truncation is not a subset mechanism. A portable subset is a new self-contained v0.2 snapshot containing the selected entities and the closure of required hierarchy, styles, resources, relations, and chemical semantics. Incremental UI updates use [Document Patch v1](protocol/document-patch-v1.md), not a reread of the file.

`interchange` retains editable CDX/CDXML objects and properties not yet represented natively. Native fields remain authoritative. `.ccjs` is text JSON. `.ccjz` uses [CCJZ Container v1](protocol/ccjz-container-v1.md): a deterministic, hashed, random-access ZIP container with scene chunks and content-addressed resources. Legacy gzip `.ccjz` remains readable but is never written by current implementations.
