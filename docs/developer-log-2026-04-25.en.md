# Chemcore Developer Log - 2026-04-25

Author: Jiajun Zhang

Time range: 2026-04-24 00:28 to 2026-04-25 00:25, Asia/Shanghai

## Summary

The main outcome today was not another round of local fixes on top of the old bond logic. We replaced bond-contact rendering with a more coherent geometry kernel.

Previously, main bonds, side lines, bold bonds, wedges, dashed bonds, and centered doubles all had partially separate contact rules. Many cases were held together by local heuristics, which is why the viewer could still show spikes, seams, wrong retreats, or preview/final mismatches even when individual cases looked close.

Today we moved that layer onto a contour-driven Rust rendering path. Solid bonds are treated as polygons, bond contact is resolved through contour intersections, and drag preview now renders through the same `render_document` path as final placement. What the user sees while dragging is now much closer to the exact geometry that lands in the document.

## Bond Contact Kernel Rewrite

The largest change today was the fragment-bond contact rewrite.

The new direction is:

- objects with a main bond are first reduced to endpoint contours,
- two-bond contact is resolved from extended inner and outer contour intersections,
- three-way and higher-degree nodes are resolved around the node in angular order,
- and `180°` straight-through cases are handled as explicit exceptions instead of forcing unstable intersections.

The important part is not only that these cases can be computed, but that the renderer now tries to generate the correct bond polygon directly instead of stacking patch-on-patch fixes. In multi-bond nodes, a five-sided bond polygon is no longer a special overlay trick; it now falls naturally out of the endpoint profile logic.

The old legacy molblock rendering branch was also split out of the main render file into its own module so it stops contaminating the new contact path.

## Unified Bond Geometry Model

Several bond types were moved onto one shared geometry model today:

- All solid bonds are handled as polygons rather than `line` primitives. A plain single bond and a bold bond are now the same geometry type with different widths.
- Solid wedges are no longer treated as triangles. They now render as trapezoids with a very short narrow-end cap, which makes later contact and width handling fit the same polygon framework.
- Dashed bonds no longer depend on SVG dash styling. They are built as solid mother polygons and then cut into segments with white knockout gaps.
- Hash bonds and hashed wedges now follow the same idea: a bold mother shape sliced by denser white gaps, with equal black segment lengths and slightly flexible white spacing.

This matters because contact rules no longer depend on whether a shape happened to be rendered as a line or a polygon. Once an object has explicit contours, contact can fall back to the same contour-intersection logic.

## Double-Bond Rules Tightened

Most of today’s double-bond work was about replacing vague behavior with explicit rules.

For side double bonds:

- the outer side line normally keeps its original inset behavior and does not participate in ordinary contact,
- only the inner side line joins against neighboring geometry,
- terminal ends stay equal in length with the main bond,
- non-terminal ends shorten by `offset * sqrt(3) / 3`,
- acute-angle retreat happens only when the connected main bond does not provide a same-side secondary line,
- if the neighbor does provide a same-side secondary line, the two secondary lines still intersect normally,
- and if the neighbor is a centered double bond, retreat uses its center axis as reference.

Centered doubles were also rewritten:

- they are no longer treated as a decorative double-line special case,
- each child line now decides its own endpoint behavior independently,
- each endpoint and each side can extend to neighboring bond contours on its own,
- near-straight cases above `162°` remain unchanged,
- and mixed solid/dashed centered doubles as well as double-dashed centered doubles now share the same centered-double rule set.

This finally makes the boundaries between side double and centered double behavior much clearer, especially at branching contacts.

## Hash Bonds and Multi-Bond Nodes

Hash bonds and hashed wedges received two important rules today.

First, when they touch ordinary main-bond geometry, their mother outlines stay unchanged:

- a `hash bond` remains a standard rectangle,
- a `hashed wedge` remains a standard trapezoid.

They no longer deform like solid bonds to actively fit neighboring geometry. The only intentional exception that remains is their centered-double behavior, where they retreat to the relevant outer reference line.

Second, in multi-bond nodes:

- hash bonds and hashed wedges keep their original shapes,
- the other bonds retreat instead,
- and they leave a small white gap rather than trying to create a perfectly sealed seam.

This keeps the hash family from being visually crushed in dense nodes and gives those nodes a more stable hierarchy.

## Editor Preview and Viewer Cleanup

In addition to backend geometry, today also closed several important frontend gaps.

The biggest one is drag preview. The editor no longer draws a fake overlay bond while dragging. The Rust engine now clones the current document, inserts the temporary bond, and runs `render_document` on that preview document. That means:

- the bond shown during drag,
- the bond committed on mouse release,
- and the final display list emitted by the kernel

all now come from the same geometry path.

The temporary debug panel that showed cursor coordinates and polygon vertices in the lower-right corner was removed once the geometry stabilized. The primary bond button in the left toolbar now also mirrors the currently selected bond subtype.

We also cleaned up frontend handling of `strokeWidth: 0`, polygon stroking, and shared-edge style overrides. Several cases where the backend SVG looked correct but the live viewer still showed spikes or white seams were ultimately traced to the frontend accidentally stroking polygons that were supposed to have no stroke.

## Verification

The main verification commands used today were:

```bash
cargo test
npm run build:engine-wasm
node --check viewer/app.js
```

`cargo test` now covers:

- bond-tool interaction behavior,
- render-document geometry regressions,
- multi-bond nodes, centered doubles, hash bonds, wedges, and related contact cases.

This round was also accepted through direct viewer interaction checks for:

- two-bond contacts,
- three-way and higher-degree contacts,
- side-double and centered-double edge cases,
- hash bond and hashed wedge behavior against ordinary bonds, centered doubles, and multi-bond nodes,
- and consistency between drag preview and final placement.

## Commit Timeline

| Commit | Summary |
| --- | --- |
| `1951a84` | Rewrote the bond contact rendering kernel and moved drag preview onto the same Rust render path. |

## Remaining Risks and Next Steps

The biggest gain today is that bond contact is now much closer to a backend geometry definition instead of a viewer-side visual patch. But a few follow-up risks are still clear:

- centered doubles, side doubles, and the hash family now live in one framework, but their rule surface is still large enough that more example-driven regression coverage is needed,
- the viewer and the Rust display list are now much closer, and the next step should keep reducing any frontend reinterpretation of geometry,
- and the contact rules are now stable enough that they should eventually be written down as standalone rendering documentation rather than only living in tests and branch logic.

The architectural conclusion for the day is clear: stable bond rendering will not come from layering more viewer-side fixes. It has to come from defining bond geometry, endpoint closure, and node contact allocation centrally inside the Rust kernel.
