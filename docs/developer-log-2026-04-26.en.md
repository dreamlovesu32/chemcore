# Chemcore Developer Log - 2026-04-26

Author: Jiajun Zhang

Time range: 2026-04-26 00:00 to 2026-04-26 23:59, Asia/Shanghai

## Summary

Today’s work moved from bond rendering and double-bond style rules into the text editor and attached-label geometry itself. The first half of the day was about turning the text tool from a browser-driven rich-text surface into a more self-owned editor stack. The second half was about making attached-label anchoring, especially terminal double-bond labels, finally agree with the actual rendered bond geometry.

The important outcome was not another set of isolated fixes. Two long-running sources of instability were pushed further into the core:

- text editing now depends much less on `contenteditable`, DOM selection, and browser rich-text behavior;
- label anchoring depends less on viewer heuristics and legacy font constants, and more on kernel glyph geometry and the same double-bond offset rules used in rendering.

By the end of the session the text tool looked much more like a real editor, and terminal double-bond labels finally landed in the visible double-line channel rather than in the familiar “almost right but still visibly off” position.

## Moving the Text Tool Toward a Self-Owned Editor

The day started with a deliberate shift away from patching `contenteditable` behavior and toward reclaiming editor semantics from the browser.

### Text Model

The first step was to stop treating browser rich-text commands as the primary source of truth.

Bold, italic, underline, superscript, subscript, chemical formatting, color, font size, font family, paste behavior, and ordinary text insertion now operate on the editor’s own runs model first, with the visible layer being redrawn from that state.

That means:

- the browser is no longer the document model,
- the real text state is increasingly `sourceRuns + selection`,
- and toolbar commands and commit behavior are organized around the editor’s own model rather than DOM formatting.

### Selection and Caret

The next step was to make selection and caret state explicit instead of continuing to treat the browser selection as the only authority.

The editor now maintains its own selection offsets:

- keyboard input first updates runs and selection,
- pointer hit testing and drag selection update the same offset model,
- and the visible blue highlight and black caret are then rendered from that state.

So browser selection is no longer what drives editor logic. It has been reduced toward an input transport rather than an editing authority.

### Removing contenteditable as the Semantic Layer

The visible editor surface was then restructured into:

- a rendered text display layer,
- a self-drawn blue selection highlight,
- a self-drawn black caret,
- and a hidden `textarea` used only for keyboard and IME input.

This matters because the editor’s appearance and behavior now depend less on DOM editing semantics themselves. That is a necessary direction if the same editing core is expected to survive across Web, desktop, and tablet hosts.

### Shared Glyph Profiles

To reduce “looks one way while editing, another way after commit” drift, glyph profiles, tracking, script scaling, and baseline shifts were pushed further into shared data.

The frontend editor and the Rust label/glyph logic now at least begin from the same profile table rather than carrying two separate sets of hand-tuned constants.

That does not remove every platform-specific difference yet, but it does eliminate one of the easiest sources of long-term divergence.

## Text Editor Module Split

At the same time, the frontend text editor was broken into dedicated modules instead of continuing to grow inside `viewer/app.js`.

Today’s split includes:

- `viewer/text_metrics.js`
- `viewer/text_editor_model.js`
- `viewer/text_editor_render.js`
- `viewer/text_editor_controller.js`

This was not just helper extraction. It separated:

- pure text and glyph metrics,
- runs and selection model operations,
- editor display rendering,
- and the controller logic for opening sessions, handling input, pointer selection, navigation, and formatting commands.

The practical gain is not cosmetic. The text editor had become the highest-risk part of the frontend, and this split reduces the cost of continuing to evolve it.

## Chemical Formatting and Label Editing

Several behavior rules that matter directly to users were also tightened.

### Chemical-by-Default Endpoint Labels

Node and endpoint labels continue to default to chemical semantics, with the important change being that the formatting is meant to happen during editing rather than only after commit.

In label editing:

- digits after element text become subscripts immediately,
- charge markers and associated digits become superscripts immediately,
- and the visible editor state is much closer to the final committed rendering.

### Reopening Existing Labels

Reopening an existing label for editing no longer lets the editor float around based on where the user clicked inside it. It now prefers the label’s own stable editing anchor.

That directly supports the basic editing rule the user kept insisting on: if the text looked a certain way during input, it should continue to look like that after commit and when reopened.

### Separating Plain Text from Attached Labels

The distinction between plain text objects and attached labels was also reinforced:

- plain text boxes start exactly where the user clicks and commit at that position,
- attached node/endpoint labels continue to use bond-aware placement and orientation rules.

That separation prevents ordinary text objects and chemical labels from contaminating each other’s logic.

## Attached-Label Anchor Geometry

The most important geometry work today centered on node labels, especially terminal double-bond labels.

### Terminal Double-Bond Rules

Earlier work had already restored part of the intended rule:

- a terminal side double should anchor its label between the two visible lines,
- and once a substituent is added at that endpoint, anchoring should fall back to the ordinary main-bond anchor.

But after more realistic regression checks, two subtler problems remained:

- some paths had the correct direction but obviously insufficient offset,
- and some single-letter labels still were not visually centered in the expected line channel.

That turned out to be two different problems layered on top of each other.

### Attached Labels Were Not Strictly Consuming the Anchor

The first issue was inside attached-label geometry itself. Even if the target anchor was correct, single-letter labels such as `O` and `N` could still land only approximately because the attached-label layout was still carrying legacy baseline approximations.

That was corrected by:

- generating glyph polygons from the current runs,
- then translating the whole polygon group and box so that the anchor glyph center exactly matches the target anchor.

This made the label’s actual glyph center and the editing anchor consistent again.

### Double-Bond Anchor Offset Did Not Match Rendering

The deeper problem was that the double-bond anchor offset used by label anchoring did not actually match the double-bond offset used by rendering.

The final cause was very specific:

- the rendering side normalizes side-double offsets against `VIEWER_BOND_STROKE = 0.85`,
- while label anchoring had been using a different, larger default stroke basis,
- which meant labels only moved through a fraction of the visible double-line spacing.

Once that was unified, slanted terminal double-bond labels such as `O` finally moved into the actual visible channel between the two rendered lines instead of staying too close to the main bond.

### Left/Right and begin/end

The fix also preserved the earlier correction that `Left` and `Right` must be interpreted relative to the bond’s own `begin -> end` direction, not relative to whichever endpoint is currently being considered.

Without that rule, terminal labels on the `end` side can easily flip onto the opposite visual side even if the editor state itself is correct.

## Fixed Regression Scenes and Scripts

To keep this class of geometry bug from falling back into manual visual checking, a dedicated regression script was added today:

- `scripts/label-anchor-regression.mjs`

This script replays three attached-label scenes and writes cropped output images for them:

- terminal side double: label should sit between the two visible lines,
- center double: label should fall back to the main-bond anchor,
- branched side double: once the endpoint gets another substituent, the label should return to the main-bond anchor.

The output is written to:

- `tmp/label-anchor-regression/terminal-side-double.png`
- `tmp/label-anchor-regression/center-double.png`
- `tmp/label-anchor-regression/branched-double.png`

The earlier text-editor regression script also remained in active use:

- `scripts/text-editor-regression.mjs`

That means today’s improvements are backed by repeatable replays rather than only by local visual judgment.

## Verification

The main verification steps used today were:

```bash
cargo test -p chemcore-engine
./scripts/build-engine-wasm.sh
node --experimental-default-type=module --check viewer/app.js
node --check scripts/label-anchor-regression.mjs
node scripts/label-anchor-regression.mjs
node scripts/text-editor-regression.mjs
```

This covered:

- text-tool input, reopen, and zoom behavior,
- immediate chemical superscript and subscript display,
- stable attached-label anchors,
- terminal side double, center double, and branched double label-anchor regressions,
- and agreement between Rust-side geometry and actual viewer replay.

## Risks and Next Steps

Even after today’s progress, a few risks remain worth watching:

- the text editor is now largely out from under `contenteditable`, but IME behavior, grapheme clusters, and platform-level font differences are still important future risk areas;
- attached-label anchoring and rendered double-bond offsets now agree again, but if bond-render constants continue to evolve, the safest long-term direction is to keep the anchor side and render side on one shared definition rather than merely matching numeric values by convention;
- `viewer/app.js` is much healthier than before, but text editing is still the densest behavior area in the frontend, so continued modularization will remain important.

The architectural takeaway from today is simple: the text editor is starting to behave like a real editor, and attached labels are increasingly landing by kernel-defined geometry instead of by browser behavior and legacy visual heuristics.
