# chemsema Format v0.1

## Scope

This document defines the first persisted document format for `chemsema`.

Version `0.1` is intentionally narrow. It is a document/object format for
rendering and future editing.

Its immediate purpose is:

- to represent a single chemistry page
- to support read-only rendering
- to receive converted data from CDXML extraction
- to act as the base for future runtime and editing logic

## Format Overview

The file is a JSON document with six top-level sections:

- `format`
- `document`
- `styles`
- `objects`
- `resources`
- `interchange` (optional)

At a high level:

- `document` defines global metadata and page settings
- `styles` stores reusable rendering styles
- `objects` stores the scene graph nodes
- `resources` stores reusable chemistry payloads such as `molecule_fragment2d`
- `interchange` losslessly stores exchange-format objects and fields that do not yet have source-independent semantics; it is editable and participates in export, so it is not metadata

## Top-Level Structure

```json
{
  "format": {
    "name": "chemsema",
    "version": "0.1",
    "unit": "pt"
  },
  "document": {},
  "styles": {},
  "objects": [],
  "resources": {},
  "interchange": {}
}
```

## Complete Interchange Field Layer

The complete CDX/CDXML field universe is larger than the current cross-format scene model. Fields without native drawing semantics must not be discarded or hidden in non-exported `meta`. Importers retain them in top-level `interchange` as an object tree with named properties.

Each object records `name`, optional `formatTag`, optional `id`, and `children`. Each property records its canonical `name`, zero-based `order`, editable lexical `value`, and explicit `valueType`; CDX properties additionally record `cdxTag`, `cdxType`, and exact `rawBase64`. Repeated CDX properties use stable storage keys such as `Name#2` while retaining the canonical internal `name` and source order. Public lexical CDX types are edited through `value`; `Unformatted`, `varies`, and structurally complex binary types are edited through `rawBase64`.

When a source-independent CCJS field exists, that native field remains authoritative and the exporter re-encodes it. The interchange tree only restores objects and properties that are not otherwise modeled. The exhaustive machine-readable contract is `schemas/cdx-cdxml-verification-v1.json`.

## Coordinate System

Version `0.1` uses a single world coordinate system:

- origin: top-left
- x increases to the right
- y increases downward
- units: typographic points (`pt`, 1/72 inch), stored as `format.unit = "pt"`

No backend-specific pixel assumptions belong in the file.

## Object Identity

Every object must have a globally unique `id` within the document.

Rules:

- object ids are strings
- style ids are strings
- resource ids are strings
- references are by id, never by array position

## Containment Rules

Version `0.1` uses a strict tree for object ownership.

Rules:

- Every object must belong to exactly one container
- A container is either the top-level `objects` array or one `group.children` list
- An object may have at most one direct parent group
- An object may appear in exactly one container
- A direct parent group owns each grouped object

This keeps ownership, traversal, selection, and editing behavior deterministic.

## Object Model

Each scene object shares a common envelope:

```json
{
  "id": "obj_001",
  "type": "molecule",
  "name": "optional human label",
  "visible": true,
  "locked": false,
  "zIndex": 10,
  "transform": {
    "translate": [0, 0],
    "rotate": 0,
    "scale": [1, 1]
  },
  "styleRef": "style_default",
  "meta": {},
  "payload": {}
}
```

### Common Fields

- `id`: unique object id
- `type`: one of the supported object types
- `name`: optional label for debugging or UI
- `visible`: whether the object participates in rendering
- `locked`: whether the object is editable
- `zIndex`: global stacking key
- `transform`: local transform
- `styleRef`: optional style id
- `meta`: non-render-critical metadata
- `payload`: type-specific data

### Supported Object Types in v0.1

- `molecule`
- `text`
- `line`
- `curve`
- `bracket`
- `symbol`
- `shape`
- `image`
- `spectrum`
- `group`

Other graphical primitives can be added later.

## Object Type Baseline

Version `0.1` should start from a small but stable set of first-class object
types:

- `molecule`: chemistry-bearing 2D structure
- `text`: positioned rich text
- `line`: straight/curved stroke objects, including arrows
- `curve`: CDXML/CDX cubic Bezier curve objects with their native control points
- `bracket`: bracket-like graphical objects
- `symbol`: independently selectable ChemDraw symbol objects
- `shape`: simple filled or stroked regions
- `image`: placed raster image backed by an explicit image resource
- `spectrum`: sampled scientific spectrum with explicit axes and unit semantics
- `group`: logical grouping and shared transform

This split is intentional.

- `molecule` owns chemistry semantics
- `text`, `line`, `bracket`, `shape`, `image`, and `spectrum` are document graphics
- `group` owns containment and transform only

Important: labels that belong to a `molecule` are molecule-owned structure labels.
Examples include `CN`, `Ph`, `N3`, `t-Bu`, `HN`, or stacked hetero labels such as
`H` over `N`. These are structure labels with:

- an attachment anchor inside the label
- orientation relative to the attached bond
- chemistry-aware ordering rules
- optional inline sub/superscript formatting
- optional multiline run data such as `lineRuns`, when a structure label is
  displayed as stacked lines but still needs per-token styling like the
  subscript `2` in `SO2`
- normalized display runs should preserve semantic inline formatting such as
  weight, italic, underline, outline, shadow, subscript, and superscript; a
  source-format bitmask such as CDXML `face` must not be stored in native JSON
- structure-label source runs may still be preserved for import fidelity in
  `label.meta.sourceRuns`; other raw source-format fields may live under
  `meta.import.<source>`, but fields such as `face` that have an explicit native
  semantic mapping must be discarded after decoding

They should live inside molecule resources or molecule-specific payloads.

Viewer note: a renderer may apply small bounded optical adjustments at display
time, for example to separate attached-group labels from nearby atom labels.
These adjustments are viewer behavior only. Stored document geometry remains
authoritative.

Brackets are kept separate from `molecule` in `v0.1`. They often appear around
chemistry and remain document objects first. Chemical meaning, if needed later,
can be added through metadata.

## Transform

All objects may carry a local transform:

```json
"transform": {
  "translate": [120, 40],
  "rotate": 0,
  "scale": [1, 1]
}
```

Rules:

- `translate` is required
- `rotate` defaults to `0`
- `scale` defaults to `[1, 1]`

For `v0.1`, transforms are applied in local-to-world order:

1. scale
2. rotate
3. translate

## Styles

Styles are stored separately and referenced by `styleRef`.

Example:

```json
"styles": {
  "style_text_default": {
    "kind": "text",
    "fontFamily": "Helvetica",
    "fontSize": 12,
    "fontWeight": 400,
    "fill": "#111111",
    "stroke": null
  },
  "style_line_default": {
    "kind": "stroke",
    "stroke": "#222222",
    "strokeWidth": 1.5,
    "lineCap": "round",
    "lineJoin": "round"
  }
}
```

Version `0.1` keeps style taxonomy narrow. The renderer should expect styles to
describe either:

- text appearance
- stroke/fill appearance
- molecule appearance

## Resources

`resources` hold reusable content blobs shared by scene objects.

Version `0.1` defines two resource types explicitly:

- `molecule_fragment2d`
- `image`

Example:

```json
"resources": {
  "mol_a": {
    "type": "molecule_fragment2d",
    "encoding": "chemsema.molecule.fragment2d",
    "data": {}
  }
}
```

This keeps molecule objects small and makes repeated references possible.

An `image` resource stores validated raster bytes and decoded pixel dimensions:

```json
"image_a": {
  "type": "image",
  "encoding": "base64",
  "data": {
    "mimeType": "image/png",
    "dataBase64": "iVBORw0KGgo...",
    "pixelWidth": 640,
    "pixelHeight": 480,
    "sourceName": "scheme.png"
  }
}
```

Native insertion accepts PNG, JPEG, GIF, and BMP. Payload bytes, MIME signature,
declared dimensions, byte size, and pixel count are validated before insertion.

## Image Object

An image object places a raster resource in the scene. Its local `bbox` defines
the displayed rectangle. Edge resize handles stretch one axis; corner handles
preserve aspect ratio. Rotation, movement, grouping, stacking, locking,
visibility, copy/paste, deletion, and undo use the normal scene-object rules.

```json
{
  "id": "obj_image_1",
  "type": "image",
  "visible": true,
  "locked": false,
  "zIndex": 30,
  "transform": {
    "translate": [120, 80],
    "rotate": 15,
    "scale": [1, 1]
  },
  "meta": { "kind": "image" },
  "payload": {
    "resourceRef": "image_a",
    "bbox": [0, 0, 160, 120],
    "fit": "stretch",
    "opacity": 1
  }
}
```

CDX/CDXML raster payloads map to this object without changing their bytes.
Unsupported compound payloads such as OLE, EMF, WMF, TIFF, PDF, or PICT remain
opaque resources and render a sized diagnostic placeholder instead of silently
disappearing. Their original bytes remain authoritative for round-trip export.

## Spectrum Object

A spectrum object stores evenly spaced X samples and their Y values directly in
the scene object. Imported spectra and generated prediction results use this
same object; the object itself does not own or invoke a prediction service.

```json
{
  "id": "obj_spectrum_1",
  "type": "spectrum",
  "visible": true,
  "locked": false,
  "zIndex": 18,
  "transform": {
    "translate": [40, 40],
    "rotate": 0,
    "scale": [1, 1]
  },
  "styleRef": "style_spectrum_1",
  "payload": {
    "bbox": [0, 0, 240, 120],
    "spectrum": {
      "class": "nmr",
      "xLow": 0,
      "xSpacing": 0.5,
      "xType": "parts-per-million",
      "xAxisLabel": "ppm",
      "yLow": 0,
      "yScale": 1,
      "yType": "arbitrary-units",
      "yAxisLabel": "intensity",
      "dataPoints": [0, 0.1, 0.8, 0.2, 0]
    }
  }
}
```

Rules:

- `dataPoints` is a required non-empty finite-number array.
- Decoded Y value `i` is `yLow + dataPoints[i] * yScale`.
- `xLow` identifies the first sample and `xSpacing` the evenly spaced interval;
  the opposite axis endpoint is `xLow + dataPoints.length * xSpacing`.
- CDXML may retain `yLow` and `yScale` as a storage transform. CDX stores the
  decoded `FLOAT64` array, so CDX import uses `yLow = 0` and `yScale = 1`.
- The first sample is drawn at the right edge, matching ChemDraw spectrum
  orientation. Resizing changes `bbox`, not the sample values.
- Spectrum objects may move, resize, group, reorder, recolor, copy/paste,
  delete, lock, and hide. They do not rotate; `rotate` must be `0` and `scale`
  must remain `[1, 1]`.
- Rendering uses bounded extrema-preserving downsampling. The stored samples
  always remain authoritative and unchanged.

`class` values are `unknown`, `chromatogram`, `infrared`, `uv-vis`,
`x-ray-diffraction`, `mass-spectrum`, `nmr`, `raman`, `fluorescence`, and
`atomic`.

`xType` values are `unknown`, `wavenumbers`, `microns`, `hertz`, `mass-units`,
`parts-per-million`, and `other`.

`yType` values are `unknown`, `absorbance`, `transmittance`,
`percent-transmittance`, `other`, and `arbitrary-units`.

## Molecule Object

The molecule object represents a placed chemistry-bearing structure on the page.

Example:

```json
{
  "id": "obj_mol_1",
  "type": "molecule",
  "visible": true,
  "locked": false,
  "zIndex": 10,
  "transform": {
    "translate": [96, 72],
    "rotate": 0,
    "scale": [1, 1]
  },
  "styleRef": "style_molecule_default",
  "meta": {
    "source": "editor",
    "collapsed": false
  },
  "payload": {
    "resourceRef": "mol_a",
    "bbox": [0, 0, 88, 42],
    "role": "substrate"
  }
}
```

### Molecule Payload Fields

- `resourceRef`: required, points to a `molecule_fragment2d` resource
- `bbox`: optional local bounding box
- `role`: optional semantic hint such as `substrate`, `product`, `ligand`

Version `0.1` stores reaction semantics only as lightweight `role` hints.

## Text Object

The text object represents positioned rich text content.

Example:

```json
{
  "id": "obj_text_1",
  "type": "text",
  "visible": true,
  "locked": false,
  "zIndex": 20,
  "transform": {
    "translate": [220, 88],
    "rotate": 0,
    "scale": [1, 1]
  },
  "styleRef": "style_text_default",
  "meta": {},
  "payload": {
    "text": "PhB(OH)2 (1.0 equiv)",
    "box": [0, 0, 120, 18],
    "align": "left",
    "valign": "middle"
  }
}
```

### Text Payload Fields

- `text`: required plain text content
- `box`: optional local text box
- `align`: `left | center | right`
- `valign`: `top | middle | bottom`
- `runs`: optional rich text runs for inline formatting
- `anchorOffsetX`: optional horizontal offset from the rendered text-box anchor to the
  source text anchor; this is a geometric length, not a source-format enum
- `baselineOffset`: optional vertical offset from the text-box top to the text baseline

### Rich Text Support

Version `0.1` text should be able to represent:

- font family
- font size
- font weight / italic
- underline / outline / shadow
- superscript
- subscript
- symbols and special characters

Recommended inline model:

```json
"runs": [
  {
    "text": "SO",
    "fontFamily": "Arial",
    "fontSize": 10,
    "fill": "#000000",
    "fontWeight": 700,
    "fontStyle": "normal",
    "underline": false,
    "outline": false,
    "shadow": false,
    "script": "normal"
  },
  {
    "text": "4",
    "fontFamily": "Arial",
    "fontSize": 10,
    "fill": "#000000",
    "fontWeight": 700,
    "fontStyle": "normal",
    "underline": false,
    "outline": false,
    "shadow": false,
    "script": "subscript"
  }
]
```

`script` is one of `normal | subscript | superscript | chemical`. CDXML `face`
is decoded into `fontWeight`, `fontStyle`, `underline`, `outline`, `shadow`, and
`script`; `font` and `color` are decoded into `fontFamily` and `fill`. The native
format never stores the source `face` bitmask. Export reconstructs it from the
semantic fields.

`fontFamily` is an open family-name string, not an enum. A UI may suggest common
installed families, but imported or user-entered names must remain round-trippable.

#### Text run fields

| Field | Type | Required | Semantics |
| --- | --- | --- | --- |
| `text` | string | yes | Text owned by this run |
| `fontFamily` | string | no | Open font-family name; inherit the enclosing text style when absent |
| `fontSize` | number | no | Positive size in document units; inherit when absent |
| `fill` | string | no | Text color; inherit when absent |
| `fontWeight` | number | no | Semantic weight such as `400` or `700`; inherit when absent |
| `fontStyle` | string | no | `normal` or `italic`; inherit when absent |
| `underline` | boolean | no | Underline decoration; inherit when absent |
| `outline` | boolean | no | Draw glyph outlines instead of solid glyph fills; inherit when absent |
| `shadow` | boolean | no | Draw the text shadow effect; inherit when absent |
| `script` | string | no | `normal`, `subscript`, `superscript`, or `chemical`; inherit when absent |

`style.labelStyle` and `style.captionStyle` use the same fields without `text`.
Their canonical values are explicit: `fontFamily`, `fontSize`, `fill`,
`fontWeight`, `fontStyle`, `underline`, `outline`, `shadow`, `script`,
`lineHeight`, and `lineHeightMode`. `lineHeight` is the resolved positive
baseline advance in document points. `lineHeightMode` is `fixed`, `auto`, or
`variable`; it records how new multiline content derives its advances without
storing a source-format sentinel.
Readers of older CCJS documents must default missing `outline` and `shadow` to
`false`. Writers must emit semantic fields and must not emit `face`.

## Molecule Fragment2D

`molecule_fragment2d` resources store nodes and bonds in local coordinates.
Fields should describe chemistry and rendering intent directly.

In addition to `nodes` and `bonds`, a fragment may contain:

- `stereo`: source-independent `StereoElementV2` records for enhanced,
  nontraditional, conformational, or otherwise identity-bearing
  stereochemistry that cannot be reconstructed from bond glyphs alone;
- `interactions`: source-independent `MultiCenterInteractionV2` records for
  coordination and delocalized multicenter relationships.

Both fields default to empty for older CCJS documents. Their atom and bond
references use IDs local to the fragment. Copy, paste, component splitting, and
deletion must remap or prune all references atomically. CDXML
`EnhancedStereoType`/`EnhancedStereoGroupNum` map to `stereo`;
`NodeType="MultiAttachment"` plus one or more proxy bonds to distinct acceptor
atoms maps to one donor center and one or more acceptor centers in
`interactions`. Existing proxy geometry is preserved for imported-document
round trips. Native graph-only coordination data is not advertised as a
lossless CDX/CDXML export until an adapter explicitly constructs the required
proxy geometry.
The CDXML proxy node is retained only for drawing and source round-trip and is
not an atom in ChemicalGraph. Missing attachment atoms, repeated attachments,
or invalid/repeated acceptors are import errors rather than lossy
normalizations. An unbonded MultiAttachment remains a drawing marker and does
not invent a molecular interaction.

Example node label:

```json
{
  "id": "n1",
  "element": "N",
  "atomicNumber": 7,
  "position": [47.4, 29.96],
  "charge": 0,
  "numHydrogens": 0,
  "atomProperties": {
    "isotopeMass": 15,
    "isotopicAbundance": "enriched",
    "radical": "doublet",
    "atomNumber": "7",
    "showAtomNumber": true,
    "cipStereo": "R",
    "showAtomStereo": true
  },
  "label": {
    "text": "N",
    "sourceText": "N",
    "position": [43.79, 33.86],
    "box": [43.79, 25.52, 51.01, 33.86],
    "layout": "default",
    "anchor": "start",
    "lineHeight": 8.9,
    "lineHeightMode": "variable",
    "runs": [
      {
        "text": "N",
        "fontFamily": "Arial",
        "fontSize": 10,
        "fill": "#000000",
        "fontWeight": 400,
        "fontStyle": "normal",
        "script": "normal"
      }
    ]
  }
}
```

External connection points use one source-independent object rather than a
boolean flag or `atomProperties.atomNumber`:

```json
{
  "id": "e1",
  "element": "",
  "atomicNumber": 0,
  "position": [120, 80],
  "charge": 0,
  "numHydrogens": 0,
  "externalConnection": {
    "type": "polymer-bead",
    "number": 2
  }
}
```

`type` is one of `unspecified`, `diamond`, `star`, `polymer-bead`, `wavy`,
`residue`, `peptide`, `dna`, `rna`, `terminus`, `sulfide`, `nucleotide`, or
`unlinked-branch`. `number` is an optional connection-relation number, not the
ordinal that ChemDraw draws inside a diamond. Readers migrate legacy
`isExternalConnectionPoint: true` to `{ "type": "unspecified" }`; canonical
writers must not emit the legacy boolean.

`atomProperties` is the source-independent home for editable atom decorations.
It must not store CDXML object tags or cached text boxes as its meaning.

| Field | Type | Semantics |
| --- | --- | --- |
| `isotopeMass` | positive integer | Absolute isotope mass number |
| `isotopicAbundance` | string | `unspecified`, `any`, `natural`, `enriched`, `deficient`, or `nonnatural` |
| `radical` | string | `none`, `singlet`, `doublet`, or `triplet` |
| `atomNumber` | string | User-visible atom number; distinct from reaction atom mapping |
| `showAtomNumber` | boolean | Per-atom override for the atom-number indicator |
| `cipStereo` | string | Absolute CIP descriptor such as `R`, `S`, `r`, or `s` |
| `showAtomStereo` | boolean | Per-atom override for the stereochemistry indicator |
| `atomNumberPosition` | object | Optional `auto`, angle, offset, or absolute placement intent |
| `stereoPosition` | object | Optional `auto`, angle, offset, or absolute placement intent |
| `elementList` | integer array | Allowed atomic numbers for an element-list query |
| `elementListExcluded` | boolean | Treat `elementList` as an excluded (`NOT`) list |
| `genericList` | string array | Allowed generic nicknames; may coexist with `elementList` |
| `genericListExcluded` | boolean | Treat `genericList` as an excluded (`NOT`) list |
| `freeSites` | integer | Maximum additional substituent sites |
| `showAtomQuery` | boolean | Per-atom override for query-indicator visibility |
| `ringBondCount` | string | `unspecified`, `no-ring-bonds`, `as-drawn`, `simple-ring`, `fusion`, or `spiro-or-higher` |
| `unsaturatedBonds` | string | `unspecified`, `must-be-absent`, or `must-be-present` |
| `substituentsUpTo` | integer | Maximum number of non-hydrogen neighboring bonds |
| `substituentsExactly` | integer | Exact number of non-hydrogen neighboring bonds |
| `translation` | string | `equal`, `broad`, `narrow`, or `any` query translation |
| `abnormalValence` | boolean | Permit the actual drawn valence without normal-valence diagnostics |
| `reactionChange` | boolean | The atom changes during a reaction |
| `reactionStereo` | string | `unspecified`, `inversion`, or `retention` |
| `showTerminalCarbonLabel` | boolean | Per-node override for terminal carbon labels |
| `showNonTerminalCarbonLabel` | boolean | Per-node override for nonterminal carbon labels |

Missing fields use the document/style defaults. `isotopicAbundance` and
`radical` default to `unspecified` and `none`; query enums default to
`unspecified`/`equal`, list-exclusion and abnormal-valence flags default to
`false`, and the remaining fields default to absent. Attached electron-symbol objects remain independently selectable, but
their attachment contributes to the atom's effective radical chemistry.

`nmrAssignments` is the source-independent, editable home for predicted or
imported atom assignments. Each entry contains `nucleus` (`1H`, `13C`, or
`unknown`), `shiftPpm`, `rangeLowPpm`, `rangeHighPpm`, `quality` (`good`,
`medium`, `rough`, or `unknown`), and a normal `NodeLabel` in `label`.
ChemDraw `/CS/CD/assign` object tags map to this field; they must not create a
second free-text object.

Abbreviation labels keep the original drawing data and add machine-readable
semantics under `meta.labelRecognition`. Readers that only need visual
round-trip can ignore `meta`; readers that need functional group semantics can
consume `expansion`:

```json
{
  "id": "n3",
  "element": "C",
  "atomicNumber": 6,
  "position": [82.0, 48.0],
  "charge": 0,
  "numHydrogens": 0,
  "isPlaceholder": true,
  "label": {
    "text": "CO2Et",
    "sourceText": "CO2Et",
    "position": [82.0, 48.0],
    "box": [82.0, 39.6, 112.0, 50.4],
    "runs": []
  },
  "meta": {
    "labelRecognition": {
      "kind": "functional-group",
      "status": "recognized",
      "source": "valence-parser",
      "label": "CO2Et",
      "canonicalLabel": "CO2Et",
      "groupKind": "valence-fragment",
      "formula": "-C(=O)OEt",
      "anchorAtom": "C",
      "components": [
        { "label": "C", "kind": "atom" },
        { "label": "O", "kind": "atom", "parentIndex": 0, "bondOrderToParent": 2 },
        { "label": "O", "kind": "atom", "parentIndex": 0, "bondOrderToParent": 1 },
        { "label": "Et", "kind": "terminal", "parentIndex": 2, "bondOrderToParent": 1 }
      ],
      "expansion": {
        "schema": "chemsema.functionalGroupExpansion.v1",
        "connectionKind": "terminal",
        "complete": true,
        "atoms": [
          { "id": "c1", "element": "C", "numHydrogens": 0 },
          { "id": "o1", "element": "O", "numHydrogens": 0 },
          { "id": "o2", "element": "O", "numHydrogens": 0 },
          { "id": "c2", "element": "C", "numHydrogens": 2 },
          { "id": "c3", "element": "C", "numHydrogens": 3 }
        ],
        "bonds": [
          { "begin": "c1", "end": "o1", "order": 2 },
          { "begin": "c1", "end": "o2", "order": 1 },
          { "begin": "o2", "end": "c2", "order": 1 },
          { "begin": "c2", "end": "c3", "order": 1 }
        ],
        "attachments": [
          { "role": "external", "atomId": "c1" }
        ]
      }
    }
  }
}
```

`expansion` is an additional semantic layer on top of the main molecule graph.
Its atom ids are local to the expansion. Bridge labels use
`left` and `right` attachment roles. `complete: false` means the label was
recognized, but the current expansion contains a partial or opaque component.
Atoms may also carry `formalCharge` for valence-parser exceptions such as
`BH3`, `NH3`, `OH2`, and `OH3`. Current `groupKind` values are
`terminal-fragment`, `valence-fragment`, `bridge-fragment`, and
`chemical-text`; `chemical-text` carries no `expansion`.

Example bonds:

```json
{
  "id": "b1",
  "begin": "n1",
  "end": "n2",
  "order": 1,
  "strokeWidth": 0.6,
  "boldWidth": 2.0,
  "wedgeWidth": 3.0,
  "hashSpacing": 2.5,
  "bondSpacing": 18.0,
  "marginWidth": 1.6,
  "properties": {
    "queryOrders": ["single", "double"],
    "topology": "ring",
    "reactionParticipation": "make-and-change",
    "absoluteStereo": "e",
    "showQuery": true,
    "showReaction": true,
    "showStereo": true
  },
  "stereo": {
    "kind": "solid-wedge",
    "wideEnd": "end"
  }
}
```

```json
{
  "id": "b2",
  "begin": "n2",
  "end": "n3",
  "order": 2,
  "double": {
    "placement": "right"
  }
}
```

Molecule label fields:

- `text`: normalized display text
- `sourceText`: optional original label text before chemistry-oriented
  reordering
- `position`: local label point
- `box`: local label bounding box
- `layout`: label layout mode such as `default`, `attached-group`,
  `attached-group-above`, or `centered-atom`
- `anchor`: connection anchor inside the label, usually `start | center | end`
- `runs`: normalized display runs
- `lineRuns`: optional normalized runs per rendered line
- `lines`: optional rendered-line text, usually paired with `lineRuns`
- `lineHeight`: resolved positive default baseline advance in document points;
  it is present for single-line labels too
- `lineHeightMode`: `fixed`, `auto`, or `variable`; this is explicit and must
  not be inferred from the numeric advance
- `lineAdvances`: optional positive per-transition baseline advances for
  variable-height multiline labels; entry 0 advances line 0 to line 1
- `glyphPolygons`: optional per-glyph optical polygons in local coordinates; when
  present, renderers may use them for label knockout and bond clipping with
  finer precision than the coarse label `box`
- `meta.sourceRuns`: optional source runs for reopening the structure-label editor
  and regenerating direction-dependent display text

For CDXML/CDX imports, the source `<t BoundingBox>` is preserved only as
`meta.import.cdxml.boundingBox`. The native active `box` / `boxField` is rebuilt
from the current label runs, baseline, alignment, and shared glyph metrics.
Imported bounding boxes are cached source evidence and may be stale; they must
not override current ChemSema label geometry.

CDXML/CDX root drawing defaults are preserved under
`document.meta.import.cdxml.defaults`. Numeric physical values such as bond
length, chain angle in degrees, line widths, spacing, margins, font sizes, and
print margins remain numbers. Source-format codes do not enter native JSON:
font ids become `fontFamily`, face bitmasks become explicit `fontWeight`,
`fontStyle`, `underline`, `outline`, `shadow`, and `script` fields, and color-table ids become hex
colors. Active text defaults live in `style.labelStyle` and
`style.captionStyle`; numeric drawing defaults remain in `style.defaults`.
CDX/CDXML export rebuilds font, face, and color-table ids from these semantic
values. Known colors reuse their color-table entries instead of persisting the
source id in CCJS.

Bond fields:

- `order`: numeric bond order
- `properties.queryOrders`: zero or more official query bond orders,
  `single | aromatic | double | triple`; two or more values describe a query
  bond whose mnemonic is its visible bond-type expression
- `properties.topology`: `unspecified | ring | chain | ring-or-chain`
- `properties.reactionParticipation`: `unspecified | reaction-center |
  make-or-break | change-type | make-and-change | not-reaction-center |
  no-change | unmapped`
- `properties.absoluteStereo`: `unspecified | none | e | z`
- `properties.showQuery`, `showReaction`, `showStereo`: optional per-bond
  visibility overrides; absence inherits the corresponding document default
- `strokeWidth`: normal bond stroke width in pt
- `boldWidth`: bold bond template width in pt
- `wedgeWidth`: solid and hollow wedge wide-end template width in pt; CDXML source templates derive it as `1.5 * BoldWidth`
- `labelClipMargin`: legacy compatibility field; new documents must not emit it,
  and renderers ignore it because glyph polygons define the clipping boundary
- `hashSpacing`: hash / hashed wedge template spacing in pt
- `bondSpacing`: double-bond spacing percentage, matching CDXML `BondSpacing`
- `marginWidth`: source margin width in pt. It drives glyph-polygon expansion for
  bond-vs-label retreat and also applies to bond-vs-bond crossing knockout where
  applicable.
- `lineStyles`: line patterns for `main | left | right`, each one of
  `solid | dashed | wavy`
- `lineWeights`: line weights for `main | left | right`, each one of
  `normal | bold`
- `stereo.kind`: `solid-wedge | hashed-wedge | hollow-wedge`
- `stereo.wideEnd`: `begin | end`
- `double.placement`: `left | right | center`
- `double.centerExitSide`: optional side preference for center double bonds at
  branched endpoints
- `double.frozen`: optional boolean that prevents automatic double-bond side
  inference from replacing an imported or user-chosen placement
- `meta.endpointAttachments.begin | end`: optional semantic internal-label
  attachment object with `target: "label-character"`, numeric
  `characterIndex`, and the corresponding `character`. CDX/CDXML import
  decodes `BeginAttach` / `EndAttach` into this object; export encodes only the
  character index back to the source format.

Current built-in template values:

| Field | Default | ACS Document 1996 |
| --- | ---: | ---: |
| `strokeWidth` | `1.0` | `0.6` |
| `boldWidth` | `4.0` | `2.0` |
| `wedgeWidth` | `6.0` | `3.0` |
| `hashSpacing` | `2.7` | `2.5` |
| `bondSpacing` | `12.0` | `18.0` |
| `marginWidth` | `2.0` | `1.6` |
| `chainAngle` | `120.0` | `120.0` |

## Line Object

The line object represents stroke-based linear geometry on the page.

It should cover:

- straight lines
- dashed lines
- polylines
- curved lines
- half arrows
- full arrows

Example:

```json
{
  "id": "obj_line_1",
  "type": "line",
  "visible": true,
  "locked": false,
  "zIndex": 15,
  "transform": {
    "translate": [0, 0],
    "rotate": 0,
    "scale": [1, 1]
  },
  "styleRef": "style_line_default",
  "meta": {},
  "payload": {
    "kind": "polyline",
    "points": [[260, 120], [380, 120]],
    "head": "end",
    "tail": "none",
    "arrowHead": {
      "kind": "solid",
      "head": "full",
      "tail": "none",
      "length": 18,
      "centerLength": 14,
      "width": 5
    }
  }
}
```

### Line Payload Fields

- `kind`: required geometry kind such as `line | polyline | curve`
- `points`: required control points in local coordinates
- `head`: `none | start | end | both`
- `tail`: `none | start | end | both`
- `arrowHead`: optional arrow decoration data; omitted or `null` means plain line
- `curve`: optional curve metadata for bezier or arc-like lines
- `arrowGeometry`: optional circular-arc reference geometry for curved arrows,
  with `center`, `majorAxisEnd`, and `minorAxisEnd`

`arrowHead` size fields follow ChemDraw's stroke-width-relative meanings. At render time the real size is the stored value multiplied by the current stroke width; CDXML export writes the stored value back multiplied by `100`:

- `length` maps to CDXML `HeadSize / 100`; the rendered head length is `length * strokeWidth`
- `centerLength` maps to CDXML `ArrowheadCenterSize / 100`; the rendered notch position is `centerLength * strokeWidth`
- `width` maps to CDXML `ArrowheadWidth / 100`; the rendered broad-end half-width parameter is `width * strokeWidth`. For solid arrowheads, ChemDraw treats this as the broad-end half-width parameter: the rendered outline uses an outer half-width of about `width * strokeWidth + 0.05` and an inner Bezier control offset of `7/16` of that half-width. For open and hollow arrowheads, this value is the extra head-width parameter relative to the shaft half-width
- `curve` maps to CDXML `AngularSize`; negative and positive values represent opposite bend directions
- `curveSpacing` maps to CDXML `CurveSpacing / 100`
- `noGo` maps to CDXML `NoGo` and may be `none | cross | hash`
- `dipole` adds the perpendicular dipole bar at the tail and maps to CDXML `Dipole=yes`
- `closed` preserves CDXML's closed-curve flag when present
- `source` and `target` preserve the CDXML `ArrowSource` and `ArrowTarget` object references
- `kind` may be `solid | hollow | open | equilibrium | unequal-equilibrium`
- `bold` marks a bold arrow stroke
- `shaftSpacing` stores the spacing between equilibrium-arrow shafts
- `equilibriumRatio` stores the long/short ratio for `kind: "unequal-equilibrium"` and is removed for equal equilibrium arrows
- `hollow` and `open` arrow kinds use their own size templates

Line appearance belongs primarily in styles, including:

- stroke color
- stroke width
- dash pattern
- line cap
- line join

Arrow semantics are therefore modeled as line-end decoration on the same `line`
object type.

CDX has two arrow representations. Legacy `Graphic` objects use the bit-field
`ArrowType` (`NoHead`, `HalfHead`, `FullHead`, `Resonance`, `Equilibrium`,
`Hollow`, `RetroSynthetic`, plus `NoGo`/`Dipole` modifiers). Modern `Arrow`
objects use independent `ArrowheadHead`, `ArrowheadTail`, and `ArrowheadType`
properties. Import gives the modern endpoint properties precedence when both
representations are present. Legacy arc graphics store an authored arc endpoint
followed by the circle center in `BoundingBox`; the other endpoint is recovered
from the signed `AngularSize` sweep.

## Bracket Object

The bracket object represents standalone bracket graphics.

It should cover:

- parenthesis: `(`
- square bracket: `[]`
- curly brace: `{}`

Example:

```json
{
  "id": "obj_bracket_1",
  "type": "bracket",
  "visible": true,
  "locked": false,
  "zIndex": 12,
  "transform": {
    "translate": [0, 0],
    "rotate": 0,
    "scale": [1, 1]
  },
  "styleRef": "style_bracket_default",
  "meta": {
    "semanticRole": "annotation"
  },
  "payload": {
    "kind": "square",
    "side": "left",
    "box": [180, 60, 12, 80]
  }
}
```

### Bracket Payload Fields

- `kind`: `round | square | curly`
- `side`: `left | right | pair`
- `box`: required local box used to fit the bracket geometry

Brackets are document graphics in `v0.1`. If a bracket later carries polymer,
repeat-unit, or grouping semantics, that meaning should be added through
explicit metadata or future chemistry-specific objects.

## Shape Object

The shape object represents simple filled or stroked regions.

It should cover:

- `circle`
- `ellipse`
- `rect`
- `roundRect`

Example:

```json
{
  "id": "obj_shape_1",
  "type": "shape",
  "visible": true,
  "locked": false,
  "zIndex": 8,
  "transform": {
    "translate": [0, 0],
    "rotate": 0,
    "scale": [1, 1]
  },
  "styleRef": "style_shape_default",
  "meta": {},
  "payload": {
    "kind": "roundRect",
    "bbox": [0, 0, 160, 64],
    "cornerRadius": 8
  }
}
```

### Shape Payload Fields

- `kind`: `circle | ellipse | rect | roundRect`
- `bbox`: local bounding box for rectangles and rounded rectangles; CDXML import maps this from `BoundingBox`
- `cornerRadius`: optional corner radius for `roundRect`, mapped from CDXML `CornerRadius / 100`
- `center` / `majorAxisEnd` / `minorAxisEnd`: actual circle and ellipse axis points, mapped from CDXML `Center3D`, `MajorAxisEnd3D`, and `MinorAxisEnd3D`

Shape appearance belongs primarily in styles, including:

- fill color
- stroke color
- stroke width
- dash pattern
- filled vs unfilled
- `shaded`, mapped from CDXML `Shaded`
- `shadow`, mapped from CDXML `Shadow` / `Shadowed`
- `shadowSize`, mapped from CDXML `ShadowSize / 100`; it is a dimensionless multiplier of `strokeWidth`, so the rendered shadow offset is `shadowSize × strokeWidth`

## Group Object

The group object organizes children and shared transforms.

Example:

```json
{
  "id": "obj_group_1",
  "type": "group",
  "visible": true,
  "locked": false,
  "zIndex": 5,
  "transform": {
    "translate": [0, 0],
    "rotate": 0,
    "scale": [1, 1]
  },
  "styleRef": null,
  "meta": {
    "kind": "reaction_block"
  },
  "payload": {
    "children": ["obj_mol_1", "obj_line_1", "obj_text_1"]
  }
}
```

### Group Payload Fields

- `children`: required ordered list of child object ids

Children inherit the group transform.

## Group Semantics

In `v0.1`, `group` is intentionally narrow.

- A `group` owns an ordered child list
- A `group` scopes shared transforms
- Stacking order decides front/back visibility for overlaps
- A `group` can be purely structural
- Top-level `objects` should contain only root objects with no parent group

This keeps grouping and overlap handling separate.

## Document Section

The `document` section stores global metadata and page settings.

Example:

```json
"document": {
  "id": "doc_001",
  "title": "example reaction page",
  "page": {
    "width": 1024,
    "height": 768,
    "background": "#ffffff"
  },
  "meta": {
    "createdBy": "chemsema"
  }
}
```

### Document Fields

- `id`: document id
- `title`: optional title
- `page.width`: required
- `page.height`: required
- `page.background`: optional
- `meta`: optional general metadata

## Example Rendering Order

Objects are painted by:

1. ascending `zIndex`
2. stable sibling order as tiebreaker

Later-painted objects appear in front of earlier-painted objects where they
overlap.

Groups scope transforms and ownership while preserving child ordering.

## Overlap and Stacking

If two visible objects partially overlap, front/back display order is determined
only by stacking order, never by object type or overlap area.

Rules:

- Higher `zIndex` objects appear in front of lower `zIndex` objects
- If two objects have the same `zIndex`, later sibling order appears in front
- Rendering is defined as ordered painting; later paint covers earlier paint
- `group` membership preserves these rules

## Constraints for v0.1

Later versions may add:

- multiple pages
- native decoding for opaque compound embedded assets
- native reaction graph semantics
- query chemistry semantics
- editing history
- viewport state
- selection state
- collaborative metadata

Those capabilities belong in future versions once the base model is proven.

## File Extension

The native document extensions are:

- `.ccjz`: the default save format, containing gzip-compressed ChemSema JSON.
- `.ccjs`: the readable/debug format, containing uncompressed ChemSema JSON.

This keeps the JSON payload inspectable without making the default file unnecessarily large:

- production saves and Save As should default to `.ccjz`
- manual diffs and import/export debugging should use `.ccjs`

## Compatibility Promise

Version `0.1` is an unstable development format.

The current promise is:

- fields should be explicit
- ids should remain stable once generated
- migration should be possible by versioned transforms

Backward compatibility will be defined after the format stabilizes.
