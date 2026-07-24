# ChemicalGraph single-molecule contract

## Scope

ChemicalGraph is ChemSema's strict semantic interchange representation for one
determined molecular entity or one discrete, integer-ratio molecular
composition. Naming and NMR consume this representation. CCJS remains the
editable document model.

ChemicalGraph deliberately does not represent drawing coordinates, fonts,
colors, captions, reactions, query structures, Markush structures, polymers,
mixtures with fractional composition, or document layout. Those belong to
CCJS or to a future profile with a different schema identifier.

## Field classification

| Class | ChemicalGraph fields | Rule |
| --- | --- | --- |
| Authoritative molecular facts | atomic number, isotope, formal charge, radical state, resolved implicit-H count, bond kind and dative direction, stereo elements, component count, multicenter interactions | Serialized, validated, and included in identity |
| Derived normalization facts | aromatic representation, resolved valence and implicit-H interpretation, component connectivity | The V2 schema fixes their interpretation; producers must resolve them before export |
| Provenance and declared limits | `assumptions` | Serialized and validated, but never used as a hidden substitute for missing molecular facts |
| Presentation | coordinates, bounding boxes, labels, fonts, colors, bond glyphs | Forbidden from ChemicalGraph; retained in CCJS |

## V2 compatibility decision

The established identifier `chemsema-nomenclature/chemical-graph/2` already
means the V1 fields plus `interactions`. ChemSema therefore keeps that wire
shape:

- an omitted `semantics` object is accepted as the fixed V2 semantics;
- when emitted, `semantics` is informational and must equal the one fixed V2
  profile; it cannot select a different identity model;
- interaction electron count is not added to V2 because it is a variable
  molecular fact and older V2 readers would silently ignore it;
- additive fields are allowed in V2 only when ignoring them cannot change
  molecular identity or interpretation;
- any future variable identity field requires a new schema identifier and an
  explicit adapter.

## Normalization contract

Every producer must:

1. reject query, pseudo, external-connection, reaction, and polymer semantics;
2. resolve each atom to a determined element, isotope, charge, radical state,
   and implicit-hydrogen count;
3. represent aromatic bonds explicitly and never infer identity from drawing
   glyphs;
4. convert source multicenter conventions into native `interactions`;
5. convert supported source stereo into native stereo elements;
6. reject malformed or unsupported identity-bearing source data instead of
   dropping it or selecting a fallback;
7. compute components using both pairwise bonds and multicenter interactions.

## Adapter acceptance matrix

| Source or target | Accepted | Explicitly rejected |
| --- | --- | --- |
| CCJS molecule fragment | One complete selected molecule; determined atoms; supported bonds, stereo, and native interactions | Partial selection, queries, external connection points, polymers, or malformed semantic references |
| CDXML/CDX | Determined molecule, enhanced stereo, dative bonds, and validated `MultiAttachment` coordination patterns | Ambiguous/malformed multi-attachment topology and unsupported identity-bearing object types |
| SMILES | Determined connected molecule supported by the chemistry parser | Query SMARTS, document semantics, or source meaning not expressible by the parsed graph |
| Nomenclature/NMR request | Validated V2 only | Invalid schema, unresolved source semantics, or silent downgrade |

## Identity and equality

ChemicalGraph identity ignores local IDs, vector ordering, component ordering,
and drawing geometry. It includes all authoritative molecular facts above.
Two different drawings of the same normalized graph must compare equal.
Changing an isotope, charge, radical, hydrogen count, stereo descriptor,
component count, dative direction, or multicenter interaction must change
identity.
