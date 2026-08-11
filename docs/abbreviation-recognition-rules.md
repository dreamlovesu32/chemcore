# ChemSema Abbreviation Recognition Rules

This document defines the current kernel behavior for recognizing structural endpoint labels, functional-group abbreviations, and formula-like labels. Ordinary free text is handled by text-object rules.

The recognition entry point must receive connection-count context. The same string may have different meanings under different connection counts, and may become invalid when the context changes.

## Connection Count Routing

| External connection count | Behavior |
| ---: | --- |
| `0` | Chemical text validation only. Chemical text understood by the valence tokenizer is marked as `groupKind: "chemical-text"` and does not produce an `expansion`. |
| `1` | Terminal substituent. The valence-driven parser runs first, followed by named functional-group templates. |
| `2` | Bridging label. Only open bridging fragments, or `N` plus a monovalent terminal substituent, are accepted. |
| `>=3` | Currently not recognized as a functional group. |

On successful recognition, both the node and the node label receive `meta.labelRecognition`. On failure, the chemical structure label keeps the original text and receives invalid metadata; the rendering layer shows a red diagnostic box.

## Metadata Shape

Successful metadata uses a common top-level shape:

```json
{
  "kind": "functional-group",
  "status": "recognized",
  "label": "CO2Et",
  "canonicalLabel": "CO2Et",
  "groupKind": "valence-fragment",
  "source": "valence-parser",
  "formula": "-C(=O)OEt",
  "anchorAtom": "C",
  "components": [],
  "expansion": {}
}
```

Field rules:

- `label` preserves the user's input.
- `canonicalLabel` stores the normalized label, such as `COOH -> CO2H`, `OCH3 -> OMe`, `Tos -> Ts`, `FMOC -> Fmoc`, and `t-Bu -> tBu`.
- Current `groupKind` values are `terminal-fragment`, `valence-fragment`, `bridge-fragment`, and `chemical-text`.
- `source: "valence-parser"` is written only on `valence-fragment`.
- `chemical-text` does not produce an `expansion`.
- `expansion.schema` is fixed as `chemsema.functionalGroupExpansion.v1`.
- `expansion.connectionKind` is currently `terminal` or `bridge`.
- `expansion.atoms[].id` is a local id inside the expansion and does not pollute main molecular graph node ids.
- `expansion.atoms[]` may carry `numHydrogens`, `label`, and `formalCharge`.
- `expansion.attachments` uses `external` for terminal external attachment, and `left` / `right` for bridge attachments.
- `complete: false` means the label is legally recognized, but the expansion contains incomplete or placeholder topology.

Invalid metadata uses:

```json
{
  "kind": "functional-label",
  "status": "invalid",
  "diagnostic": "uninterpretable-label",
  "label": "NotAGroup"
}
```

`diagnostic` distinguishes invalid-label causes:

- `invalid-valence`: the valence tokenizer can read chemical tokens, but the
  connection count or valence assignment is impossible, such as `NMe4` or the
  reversed source form `TFAO`.
- `uninterpretable-label`: the current chemical tokenizer cannot interpret the
  text, such as `OXYZ`.

Invalid metadata is a chemical semantic diagnostic only. It must not change
attached-label display grouping, reversal, or anchor selection.

## Named Terminal Templates

The following named templates can be recognized as terminal substituents with one external bond. The external bond connects to `anchorAtom`. Some templates have complete topological expansions; complex templates that are not fully expanded still keep legal recognition metadata and are marked `complete: false` in their expansion.

| canonical | aliases | Name | formula / structure | anchorAtom |
| --- | --- | --- | --- | --- |
| `R` | `R'`, `R''` | generic substituent | `R` | `R` |
| `Ar` | - | generic aromatic substituent | `Ar` | `Ar` |
| `Me` | `CH3` | methyl | `-CH3` | `C` |
| `Et` | `C2H5` | ethyl | `-CH2CH3` | `C` |
| `Pr` | - | propyl | `-CH2CH2CH3` | `C` |
| `nPr` | `n-Pr` | n-propyl | `-CH2CH2CH3` | `C` |
| `iPr` | `i-Pr` | isopropyl | `-CH(CH3)2` | `C` |
| `Bu` | - | butyl | `-CH2CH2CH2CH3` | `C` |
| `nBu` | `n-Bu` | n-butyl | `-CH2CH2CH2CH3` | `C` |
| `iBu` | `i-Bu` | isobutyl | `-CH2CH(CH3)2` | `C` |
| `sBu` | `s-Bu` | sec-butyl | `-CH(CH3)CH2CH3` | `C` |
| `tBu` | `t-Bu` | tert-butyl | `-C(CH3)3` | `C` |
| `Ph` | - | phenyl | `-C6H5` | `C` |
| `PhCOOH` | - | benzoic acid substituent | `PhCOOH` | `C` |
| `Bn` | - | benzyl | `-CH2Ph` | `C` |
| `Bz` | - | benzoyl | `-C(=O)Ph` | `C` |
| `Ac` | - | acetyl | `-C(=O)CH3` | `C` |
| `TFA` | - | trifluoroacetyl | `-C(=O)CF3` | `C` |
| `Piv` | - | pivaloyl | `-C(=O)tBu` | `C` |
| `CHO` | - | formyl | `-C(=O)H` | `C` |
| `CN` | - | cyano | `-C#N` | `C` |
| `NCO` | - | isocyanato | `-N=C=O` | `N` |
| `NCS` | - | isothiocyanato | `-N=C=S` | `N` |
| `SCN` | - | thiocyanato | `-S-C#N` | `S` |
| `NO2` | - | nitro | `-N(=O)O` | `N` |
| `N3` | - | azido | `-N3` | `N` |
| `H` | - | hydrogen terminator | `-H` | `H` |
| `F` | - | fluoro | `-F` | `F` |
| `Cl` | - | chloro | `-Cl` | `Cl` |
| `Br` | - | bromo | `-Br` | `Br` |
| `I` | - | iodo | `-I` | `I` |
| `OH` | - | hydroxy | `-OH` | `O` |
| `NH2` | - | amino | `-NH2` | `N` |
| `Ts` | `Tos` | tosyl | `-S(=O)2-p-Tol` | `S` |
| `Bs` | - | brosyl | `-S(=O)2-p-BrPh` | `S` |
| `Ms` | - | mesyl | `-S(=O)2CH3` | `S` |
| `Tf` | - | triflyl | `-S(=O)2CF3` | `S` |
| `SO3H` | - | sulfonic acid | `-S(=O)2OH` | `S` |
| `SO2H` | - | sulfinic acid style label | `-S(=O)OH` | `S` |
| `SO3` | - | sulfonate fragment | `-S(=O)3-` | `S` |
| `SO4` | - | sulfate fragment | `SO4` | `S` |
| `SO4H` | - | sulfate monoacid | `SO4H` | `O` |
| `PO2` | - | phosphoryl fragment | `PO2` | `P` |
| `PO3` | - | phosphate fragment | `PO3` | `P` |
| `PO3H2` | - | phosphonic acid | `-P(=O)(OH)2` | `P` |
| `PO4` | - | phosphate | `PO4` | `P` |
| `PO4H2` | - | phosphate acid form | `PO4H2` | `O` |
| `Boc` | - | tert-butyloxycarbonyl | `-C(=O)O-tBu` | `C` |
| `Cbz` | - | benzyloxycarbonyl | `-C(=O)OCH2Ph` | `C` |
| `Fmoc` | `FMOC` | fluorenylmethoxycarbonyl | `-C(=O)OCH2-fluorenyl` | `C` |
| `TMS` | - | trimethylsilyl | `-Si(CH3)3` | `Si` |
| `TBDMS` | - | tert-butyldimethylsilyl | `-Si(CH3)2tBu` | `Si` |
| `TBDPS` | - | tert-butyldiphenylsilyl | `-Si(Ph)2tBu` | `Si` |
| `CCl3` | - | trichloromethyl | `-CCl3` | `C` |
| `CF3` | - | trifluoromethyl | `-CF3` | `C` |
| `CPh3` | - | trityl | `-CPh3` | `C` |
| `Cp` | - | cyclopentadienyl | `Cp` | `C` |
| `Cy` | - | cyclohexyl | `-C6H11` | `C` |
| `Mes` | - | mesityl | `2,4,6-trimethylphenyl` | `C` |
| `NHPh` | - | anilino | `-NHPh` | `N` |
| `Indole` | - | indolyl / indole template | `Indole` | `C` |
| `ster` | - | generic steric label | `ster` | `C` |

## Valence-Driven Formula-Like Labels

With one external bond, the kernel first tries the valence-driven parser. The parser tokenizes labels into elements, counts, parenthesized groups, and monovalent named templates, then assigns bond orders from left to right and produces `groupKind: "valence-fragment"`.

Typical results:

```text
CH3                  -> -CH3
CN                   -> -C#N
CF3                  -> -CF3
COCl                 -> -C(=O)Cl
COBr                 -> -C(=O)Br
CONH2                -> -C(=O)NH2
COOH                 -> canonical CO2H, formula -C(=O)OH
CO2Et                -> -C(=O)OEt
CO2Boc               -> -C(=O)OBoc
COOSO2Me             -> -C(=O)OS(=O)2Me
CH2COOCH2SO2NHCl     -> -CH2C(=O)OCH2S(=O)2NHCl
B(OH)2               -> boronic-acid style terminal fragment
```

Named templates can also be used as monovalent terminal tokens by the valence parser. For example, in `CH2Boc`, `Boc` consumes one connection site on the previous atom while its internal topology still uses the `Boc` template expansion.

The current tokenizer supports:

- Standard case-sensitive element symbols such as `Cl`, `Si`, and `Na`.
- Numeric repetition after elements, such as `H3` and `O2`.
- Parenthesized groups and repetition counts after groups; repetition counts must be `1..=32`.
- Monovalent named templates as terminal tokens.

Current valence exceptions:

- Alkali metals are treated as valence 1, and alkaline-earth metals as valence 2.
- Transition metals and several metal labels are handled as unconstrained valence, mainly for chemical text validation.
- Second-period `B`, `C`, `N`, `O`, and `F` do not use hidden expanded-valence or hidden-charge fallbacks. Without explicit charge evidence, labels such as `BH3`, `NH3`, and `OH2` are invalid in one-bond terminal context and should render with diagnostics instead of silently recording `formalCharge`.
- `S` follows local writing conventions by recognizing `SO2` as two `S=O` bonds first, then considering other feasible valence states.

Element labels with explicit parenthesized Roman oxidation states, such as `Cu(II)` or `Fe(III)`, are recognized as chemical text labels, not functional-group expansions. Their source text stays unchanged, but normal attached-label reversal still applies, so a right-side/right-aligned `Cu(II)` renders visibly as `(II)Cu`.

Metal-leading labels and reagent text are intentionally tolerant. If a chemical label starts with a metal element token, ChemSema preserves it as recognized `chemical-text` rather than marking it invalid just because the ordinary organic functional-group parser cannot expand it. This covers coordination and salt text such as `Cu(NO3)2` while still avoiding a generated expansion. Unknown nonmetal-leading strings must not be promoted to recognized chemical text merely because they contain a metal symbol such as `Y` or `Na` later in the string.

The following patterns are currently not relaxed:

```text
BH3
BCl3
NH3
NMe4
OH2
OH3
OCl3
OCl4
```

## Bridging Labels

With two external bonds, the following open fragments may be used as standalone bridging labels:

| label | aliases | formula | left / right attachment |
| --- | --- | --- | --- |
| `CO2` | `COO` | `-C(=O)O-` | `C` / `O` |
| `OCO` | - | `-O-C(=O)-` | `O` / `C` |
| `SO2` | - | `-S(=O)2-` | `S` / `S` |
| `SO` | - | `-S(=O)-` | `S` / `S` |
| `CH2` | - | `-CH2-` | `C` / `C` |
| `NH` | - | `-NH-` | `N` / `N` |
| `CO` | - | `-C(=O)-` | `C` / `C` |
| `O` | - | `-O-` | `O` / `O` |

In addition, `N` plus a monovalent terminal substituent may be used as a substituted nitrogen bridge:

```text
NMe  -> -N(Me)-
NTs  -> -N(Ts)-
NTos -> canonical NTs
NCl  -> -N(Cl)-
```

Two-bond context does not accept ordinary terminal templates such as `Boc`, `CN`, `NO2`, or `CO2Et`.

## Label Display And Reversal

Structural label display first splits text into chemically meaningful groups, then chooses group order according to the connection direction.

Grouping rules:

- Named abbreviations that contain lowercase letters are treated as one group, such as `Ph`, `Boc`, `iPr`, and `tBu`.
- `R`, `TFA`, `TMS`, `TBDMS`, and `TBDPS` are treated as whole letter groups.
- Numeric suffixes stay inside their grouped unit.
- The connection point of `TMS` is `Si`, and only one external connection point is allowed.

Therefore, when connected on the right:

```text
OTMS -> TMSO
OTFA -> TFAO
OTAA -> AATO
OXYZ -> ZYXO
```

`TMS` and `TFA` remain whole letter groups. `TAA` and `XYZ` are not currently
known whole abbreviations, so they continue to split into uppercase tokens. Even
when the semantic layer marks a label invalid, the display layer still uses
tokenizer grouping and reversal instead of falling back to whole-label layout.

The abbreviation table is extended from two open-source reference families:
RDKit's default abbreviation list as a conservative depiction baseline, and
Open Babel/OSRA `superatom.txt` as a source of left/right aliases and additional
superatom candidates. New entries should be added in small batches only after a
ChemDraw probe, an open-source reference, or a gate failure justifies them; do
not bulk-import the entire table.

Terminal templates such as `iPr`, `nBu`, and `tBu`, which begin with a lowercase letter and contain later uppercase letters, use whole-label layout: selection and anchors treat the whole label as one indivisible structural label.

Multi-connection direction uses ChemDraw's largest-open-sector rule. Sort the
actual connection directions, find the largest angular gap, and classify the
bisector of the occupied complementary arc. The two horizontal flow sectors
each span 135 degrees (`292.5°..67.5°` for reverse and
`112.5°..247.5°` for forward); the two 45-degree vertical sectors produce
stack-above and stack-below. Gaps within `0.001°` are an intentional symmetry
tie: an open gap centered on the positive horizontal axis wins first,
otherwise the gap nearest the upward axis wins, with the clockwise gap winning
an exact distance tie. Exactly opposite pairs use the undirected-axis
boundaries `22.5°/90°/157.5°`. A three-way split remains in its trigonal
120-degree phase branch while every realized angular gap is within `3°` of
`120°`; silent probes place the transition between `2.95°` and `3.05°`
across 8/10/14 pt labels and 10/14.35/24 pt bonds. Outside that fixed window,
the unique largest gap is authoritative. Silent ChemDraw SVG/CDXML probes cover
the complete 30-degree grids for
two, three, and four connections (1,079 cases), boundary sweeps, multiple
label sizes, and multiple bond lengths. Source-coordinate quantization is
resolved before this rule; file names or nominal request angles never enter
the decision. A terminal label instead follows the complete left/right
half-plane; only an effectively vertical single connection delegates direction
to collision resolution.

## Relationship To Element Implicit Hydrogen

Abbreviation recognition occurs before simple-element implicit hydrogen logic. After a functional group is recognized, implicit-hydrogen rules use the functional-group expansion as input.

Examples:

- `NO2` is recognized as a nitro group.
- `CN` is recognized as a cyano group.
- `TMS` is a monovalent trimethylsilyl group whose connection point is `Si`.
- `CO2Et`, `COOSO2Me`, and `CH2CH2CH3` are interpreted by the valence parser.

Ordinary element labels and automatic hydrogen rules are described in `docs/implicit-hydrogen-rules.md`.
More complete valence-parser rules are described in `docs/valence-label-recognition-rules.md`.
