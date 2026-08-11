# ChemicalGraphV2 semantic contract

## Scope

`chemsema-nomenclature/chemical-graph/2` is ChemSema's presentation-independent
semantic graph for determined molecular entities, molecular fragments, and
discrete integer compositions. It is the shared input boundary for
nomenclature, NMR prediction, identity comparison, and external-format
adapters.

It is not a drawing document, query language, reaction model, polymer model, or
general substance formulation. Coordinates, fonts, colors, captions, selection
state, and cached geometry must never enter this graph.

The normative machine schema is
[`schemas/chemical-graph-v2.schema.json`](../schemas/chemical-graph-v2.schema.json).
The Rust model is authoritative and the test suite requires the checked-in
schema to match it byte-semantically.
Normative positive and negative examples live in
[`fixtures/chemical-graph-v2`](../fixtures/chemical-graph-v2).

## Fixed semantics

Every newly emitted graph declares:

- `profile`: `molecular-entity`, `molecular-fragment`, or
  `discrete-composition`;
- `aromaticityModel`: currently `explicit-aromatic-bonds`;
- `hydrogenModel`: currently `resolved-counts`;
- `valenceModel`: currently `chem-sema2026`;
- `normalization`: currently
  `chemsema-chemical-graph-normalization/1`.

The original V2 wire contract did not contain the `semantics` object. Readers
therefore accept its omission as exactly the fixed values above. The object is
not a per-document switch; any present value must match the supported V2
contract.

An aromatic encoding and an alternating Kekule encoding are not silently
identical. An importing adapter must normalize them under an explicitly
supported aromaticity model before constructing V2. Resolved implicit hydrogen
counts participate in identity.

`molecular-entity` requires exactly one connected component with count one.
`molecular-fragment` also requires one connected component with count one and
at least one structured `freeValences` entry. Each entry identifies the atom
and the missing bond order (`single`, `double`, or `triple`). Equal repeated
entries are significant: two single free valences are not collapsed into one
double free valence.
`discrete-composition` allows multiple connected components with positive
integer counts. Fractional occupancy, nonstoichiometric solids, Markush/query
structures, polymers, and reactions are explicit unsupported boundaries rather
than hidden approximations.

## Identity and normalization

`validate()` rejects unknown schemas, unknown JSON fields, missing references,
duplicate ids, duplicate pairwise bonds, invalid dative directions, malformed
stereo, disconnected declared components, invalid interaction roles, and empty
or duplicate assumptions.

`normalized()` gives deterministic array/set ordering while preserving source
ids. It is useful for stable transport but is not a canonical molecular
identifier.

`is_isomorphic_to()` is the exact identity operation. It compares atom
attributes, resolved hydrogens, pairwise bond kind and dative direction,
components and counts, stereo elements and enhanced groups, and multicenter
interactions. Source ids, array order, component/interaction ids, and audit
assumptions do not affect identity. Free-valence atom placement, order, and
multiplicity do affect identity.

Consequently, V2 does not promise that one molecule has only one JSON text, and
it does not treat a SMILES string as an identity key. Two different SMILES can
map to isomorphic V2 graphs only after the importing adapter has resolved them
under the declared aromaticity, hydrogen, valence, charge, and stereo semantics.
Applications that need a database key must use graph isomorphism or a separately
versioned canonical-identity algorithm; hashing `normalized()` JSON is not
sufficient because source atom ids are preserved.

## Molecular-fragment example

The `propan-2-yl` fragment stores the carbon skeleton normally and puts its one
free single valence on the central carbon:

```json
{
  "schema": "chemsema-nomenclature/chemical-graph/2",
  "semantics": {
    "profile": "molecular-fragment",
    "aromaticityModel": "explicit-aromatic-bonds",
    "hydrogenModel": "resolved-counts",
    "valenceModel": "chem-sema2026",
    "normalization": "chemsema-chemical-graph-normalization/1"
  },
  "atoms": [
    {"id":"c1","atomicNumber":6,"isotope":null,"formalCharge":0,"radical":"none","implicitHydrogens":3},
    {"id":"c2","atomicNumber":6,"isotope":null,"formalCharge":0,"radical":"none","implicitHydrogens":1},
    {"id":"c3","atomicNumber":6,"isotope":null,"formalCharge":0,"radical":"none","implicitHydrogens":3}
  ],
  "bonds": [
    {"id":"b1","atoms":["c1","c2"],"kind":"single","dativeDirection":null},
    {"id":"b2","atoms":["c2","c3"],"kind":"single","dativeDirection":null}
  ],
  "freeValences": [{"atom":"c2","order":"single"}],
  "stereo": [],
  "components": [{"id":"component-1","atoms":["c1","c2","c3"],"count":1}],
  "assumptions": [],
  "interactions": []
}
```

The complete normative fixture is
[`fixtures/chemical-graph-v2/valid/propan-2-yl.json`](../fixtures/chemical-graph-v2/valid/propan-2-yl.json).

## Stereo and multicenter interactions

Tetrahedral and double-bond stereo use semantic references rather than drawing
geometry. Extended stereo descriptors are structured by class; arbitrary
descriptor strings are not accepted. Coordination geometries carry a positive
permutation index, and fullerene/ring-assembly descriptors carry validated
locants.

Pairwise dative bonds use a typed donor/acceptor direction. Its V2 JSON wire
form remains `donorId->acceptorId` for compatibility; parsing validates both
endpoints instead of treating it as an arbitrary string. Interactions
whose identity cannot be represented by a pairwise bond use:

- `coordination`: exactly one donor center and one or more acceptor centers;
- `delocalized-bond`: at least three atoms in shared centers.

V2 does not carry an interaction electron count. Electron count is a variable
molecular fact, so adding it under the established V2 schema would let older
readers silently ignore identity-bearing data. A future version may add it only
with a new schema identifier and an explicit adapter.
Nomenclature symbols such as eta, kappa, and mu are derived by naming rules and
are not stored as drawing strings.

## Product interfaces

- Rust: `Engine::chemical_graph_v2_json()`;
- WebAssembly: `chemicalGraphV2Json()`;
- CLI:
  `chemsema-cli chemistry input.ccjs --format chemical-graph-v2 --pretty`;
- nomenclature provider envelope:
  `Engine::nomenclature_request_json()` /
  `nomenclatureRequestJson()`, governed by
  [`schemas/nomenclature-request-v1.schema.json`](../schemas/nomenclature-request-v1.schema.json);
- NMR: `nmr_prediction_request_json()` embeds the same validated graph instead
  of rebuilding a second representation.

Nomenclature and NMR providers must reject a schema or normalization contract
they do not understand. They must not ignore unknown identity-relevant fields.
NMR requests are for complete molecular entities; molecular-fragment graphs are
intended for nomenclature and structure-editing boundaries.

## Adapter loss policy

| External representation | Intended use | Required boundary |
| --- | --- | --- |
| V3000 Mol/SDF | broad structure interchange | reject or report query atoms, S-groups, polymers, variable attachment, and unsupported enhanced stereo |
| CommonChem/rdkitjson | toolkit JSON bridge | require an explicit mapping for toolkit-specific aromaticity and stereo |
| SMILES/CXSMILES | compact transport | record the parser/aromaticity/valence model; reject unrepresentable document semantics |
| InChI/InChIKey | identity and search | never treat it as a lossless editable-graph round trip |
| CML | scientific data exchange | accept only a declared convention/profile; reject silently ignored identity extensions |
| CDX/CDXML | ChemDraw document fidelity | keep drawing and document fields in CCJS; export only resolved chemical facts to V2 |

Every adapter must return either a validated graph or an explicit unsupported/
partial result with a loss ledger. Silent field dropping is prohibited.
The Rust API `ChemicalGraphV2::assess_mapping_to()` produces the versioned
`chemsema.chemical-graph-mapping-report.v1` ledger for the implemented
ChemicalGraph, CDX/CDXML, SMILES, and SDF V2000 boundaries.
Imported CCJS documents can preserve existing CDXML MultiAttachment proxy
geometry during source round-trip. A graph-only CDX/CDXML mapping cannot assume
that presentation geometry exists, so the mapping report rejects coordination
interactions until an adapter explicitly constructs that document encoding.
The current CDX/CDXML, SMILES, and SDF V2000 mapping reports likewise reject a
fragment as lossless until that adapter has a verified structured-free-valence
encoding.
