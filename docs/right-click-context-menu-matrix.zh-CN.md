# Right-Click Context Menu Matrix

This document defines Chemcore context-menu targets and menu content. Menu labels are intentionally written in English because they are the product-facing strings. Notes can stay in Chinese while the behavior is still being designed.

## Design Principles

- Right-click uses a dedicated context hit-test.
- Context hit-test must not change the active drawing tool, show hover focus dots, or start drag/edit interactions.
- If the hit target is already inside the current selection, commands act on the current selection.
- If the hit target is not selected, right-click may create a temporary context selection for the menu. After the command finishes, drawing-tool virtual selection should disappear unless the command naturally changes selection.
- Canvas menu is used only when the hit-test finds no editable target.
- Separators should group commands by intent, not by ChemDraw's historical layout.

## Shared Menu Groups

These groups are reused below to keep object menus consistent.

### Clipboard Group

```text
Cut
Copy
Paste
Delete
```

Disabled states:

- `Cut`: disabled when there is no selection or the target cannot be cut.
- `Copy`: disabled when there is no selection or the target cannot be copied.
- `Paste`: disabled when the clipboard has no supported Chemcore content.
- `Delete`: disabled when there is no deletable target.

### Selection Group

```text
Select All
```

Disabled states:

- `Select All`: disabled when the document has no selectable content, or the active editable text is already fully selected.

### Arrange Group

```text
Bring Forward
Send Backward
Bring to Front
Send to Back
Center on Page
```

Notes:

- `Center on Page` is only shown for standalone graphic/text/symbol objects where the operation is meaningful.
- For multi-selection, `Center on Page` can be hidden until we support predictable multi-object page centering.

### Transform Group

```text
Flip Horizontal
Flip Vertical
Rotate...
Scale...
```

Notes:

- `Rotate...` opens a small input panel. Counterclockwise angles are positive.
- `Scale...` opens a small input panel.
- For molecule or mixed selection, `Scale...` can include `Scale to Bond Length` with default target `1.058 cm`.
- For pure shape/text/graphic selections, `Scale...` only needs percentage scaling.

### Color Group

```text
Color
  Black
  Red
  Blue
  Green
  Yellow
  Orange
  Purple
  Gray
  Other...
```

Notes:

- Use the current eight default colors plus `Other...`.
- `Other...` opens the color picker.
- Later we may split `Stroke Color` and `Fill Color`, but first implementation can keep a single `Color` submenu.

### Grouping Group

```text
Group
  Group
  Ungroup
```

Disabled states:

- `Group`: disabled unless at least two groupable top-level targets are selected.
- `Ungroup`: disabled unless at least one selected target is a group.

### Object Settings Group

```text
Object Settings...
```

Notes:

- Placeholder for later panels.
- Keep it in the spec but it can be disabled or omitted in first implementation.

## Global Contexts

### 01. Canvas

Hit range: empty canvas; no editable object is hit.

```text
Cut
Copy
Paste
Select All
```

Disabled states:

- `Cut`: no selection.
- `Copy`: no selection.
- `Paste`: no supported clipboard payload.
- `Select All`: no selectable document content.

Notes:

- No `Undo` / `Redo`.
- No document properties menu for now.

### 02. Selection

Hit range: current selection content, selection box, or a selected object.

```text
Cut
Copy
Paste
Delete
----
Bring Forward
Send Backward
Bring to Front
Send to Back
----
Flip Horizontal
Flip Vertical
Rotate...
Scale...
----
Color
  Black
  Red
  Blue
  Green
  Yellow
  Orange
  Purple
  Gray
  Other...
Group
  Group
  Ungroup
----
Object Settings...
```

Notes:

- If the selection is the whole molecule/component, do not show internal molecule selection dots, but use this selection menu.
- If the selection contains only molecule atoms/bonds, arrange commands may be hidden or disabled until z-order applies cleanly to molecule components.

### 03. Group

Hit range: group selection box or grouped content when the group is treated as a single object.

```text
Cut
Copy
Paste
Delete
----
Bring Forward
Send Backward
Bring to Front
Send to Back
----
Flip Horizontal
Flip Vertical
Rotate...
Scale...
----
Color
  Black
  Red
  Blue
  Green
  Yellow
  Orange
  Purple
  Gray
  Other...
Group
  Ungroup
----
Object Settings...
```

Notes:

- Same as selection menu, but `Ungroup` should be enabled.
- First implementation should not drill into child objects from a group right-click.

## Molecule Contexts

### 04. Atom / Node

Hit range: atom endpoint, element label anchor, implicit carbon endpoint, or node focus area.

```text
Cut
Copy
Paste
Delete
----
Edit Label
Expand Label
Chemical Check
----
Color
  Black
  Red
  Blue
  Green
  Yellow
  Orange
  Purple
  Gray
  Other...
Object Settings...
```

Disabled states:

- `Expand Label`: disabled when the atom label cannot be expanded to structure.
- `Chemical Check`: checkable. Default checked for atom labels and molecule editing.

Notes:

- `Edit Label` behaves like clicking the atom with the text tool.
- `Chemical Check` controls red validation boxes, including valence and label legality.

### 05. Bond

Hit range: selectable area around single, double, triple, wedge, hashed, bold, dashed, or related bond geometry.

```text
Cut
Copy
Paste
Delete
----
Bond Type
  Single
    Plain
    Dashed
    Hashed
    Hashed Wedged
    Bold
    Bold Wedged
  Double
    Left
    Right
    Center
    Bold
    Dashed
    Double Dashed
  Triple
    Plain
----
Color
  Black
  Red
  Blue
  Green
  Yellow
  Orange
  Purple
  Gray
  Other...
Object Settings...
```

Notes:

- The checked item under `Bond Type` should reflect the current bond style.
- For mixed bond selection, show submenu but leave no single style checked.

### 06. Atom Label / Endpoint Label

Hit range: atom label text box, glyph area, or editable endpoint label area.

```text
Cut
Copy
Paste
Delete
----
Edit Label
Expand Label
----
Font
Style
  Bold
  Italic
  Underline
  Superscript
  Subscript
  Formula
Size
Alignment
  Left
  Center
  Right
  Justified
----
Chemical Check
Color
  Black
  Red
  Blue
  Green
  Yellow
  Orange
  Purple
  Gray
  Other...
Object Settings...
```

Notes:

- `Formula` is the ChemDraw-like name for chemical text formatting.
- `Chemical Check` defaults checked for atom labels.
- This is distinct from standalone `Text Object`.

### 07. Molecule Component

Hit range: whole connected component selection, or context hit-test promoted from atom/bond/label to component.

```text
Cut
Copy
Paste
Delete
----
Flip Horizontal
Flip Vertical
Rotate...
Scale...
----
Chemical Check
Color
  Black
  Red
  Blue
  Green
  Yellow
  Orange
  Purple
  Gray
  Other...
Object Settings...
```

Notes:

- This menu is intentionally closer to selection behavior than to a single atom or bond menu.
- Arrange commands stay hidden unless the component maps cleanly to a top-level scene object.

## Graphic Contexts

### 08. Arrow / Line

Hit range: arrow body, arrow head, arrow tail, curved arrow path, no-go mark, or line selection area.

```text
Cut
Copy
Paste
Delete
----
Line Style
  Plain
  Dashed
  Bold
----
Arrowheads
  Full Arrow at Start
  Full Arrow at End
  Half Arrow at Start Left
  Half Arrow at Start Right
  Half Arrow at End Left
  Half Arrow at End Right
----
Bring Forward
Send Backward
Bring to Front
Send to Back
----
Flip Horizontal
Flip Vertical
Rotate...
Scale...
----
Color
  Black
  Red
  Blue
  Green
  Yellow
  Orange
  Purple
  Gray
  Other...
Object Settings...
```

Notes:

- Current document object type is `line`.
- Submenu checked states should reflect the selected arrow/line style.

### 09. Text Object

Hit range: standalone text object bounding box or glyph area.

```text
Cut
Copy
Paste
Delete
----
Edit Text
----
Font
Style
  Bold
  Italic
  Underline
  Superscript
  Subscript
  Formula
Size
Alignment
  Left
  Center
  Right
  Justified
Line Spacing...
----
Bring Forward
Send Backward
Bring to Front
Send to Back
Center on Page
----
Color
  Black
  Red
  Blue
  Green
  Yellow
  Orange
  Purple
  Gray
  Other...
Object Settings...
```

Notes:

- `Edit Text` opens the text editor for the object.
- `Chemical Check` is not shown here by default; it is shown inside active text editing context if needed.

### 10. Rectangle

Hit range: rectangle border, interior, or selection box.

```text
Cut
Copy
Paste
Delete
----
Shape Style
  Plain
  Dashed
  Filled
  Shaded
  Faded
  Shadowed
----
Bring Forward
Send Backward
Bring to Front
Send to Back
Center on Page
----
Flip Horizontal
Flip Vertical
Rotate...
Scale...
----
Color
  Black
  Red
  Blue
  Green
  Yellow
  Orange
  Purple
  Gray
  Other...
Object Settings...
```

Notes:

- Current document object type is `shape`.
- `Scale...` only needs percentage scaling for pure shape targets.

### 11. Rounded Rectangle

Same menu as `Rectangle`.

### 12. Ellipse

Same menu as `Rectangle`.

### 13. Circle

Same menu as `Rectangle`.

Notes:

- Circle hover focus follows the mouse on the edge. Context menu must not depend on that hover state.

### 14. Other Shape

Same menu as `Rectangle`.

Notes:

- Reserved for future CDXML graphic types.

## Annotation Contexts

### 15. Bracket

Hit range: bracket line, bracket endpoints, or bracket selection box.

```text
Cut
Copy
Paste
Delete
----
Bracket Type
  Parentheses
  Square Brackets
  Braces
----
Bring Forward
Send Backward
Bring to Front
Send to Back
----
Flip Horizontal
Flip Vertical
Rotate...
Scale...
----
Color
  Black
  Red
  Blue
  Green
  Yellow
  Orange
  Purple
  Gray
  Other...
Object Settings...
```

Notes:

- Current document object type is `bracket`.

### 16. Symbol

Hit range: charge, radical, lone pair, electron, or other symbol object.

```text
Cut
Copy
Paste
Delete
----
Bring Forward
Send Backward
Bring to Front
Send to Back
Center on Page
----
Flip Horizontal
Flip Vertical
Rotate...
Scale...
----
Color
  Black
  Red
  Blue
  Green
  Yellow
  Orange
  Purple
  Gray
  Other...
Object Settings...
```

Notes:

- Current document object type is `symbol`.

## Editing Contexts

### 17. Active Text Editor

Hit range: inside the active text editor.

When there is selected text:

```text
Cut
Copy
Paste
Delete
Select All
----
Font
Style
  Bold
  Italic
  Underline
  Superscript
  Subscript
  Formula
Size
Alignment
  Left
  Center
  Right
  Justified
Line Spacing...
----
Chemical Check
Color
  Black
  Red
  Blue
  Green
  Yellow
  Orange
  Purple
  Gray
  Other...
```

When there is no selected text:

```text
Paste
Select All
----
Font
Style
  Bold
  Italic
  Underline
  Superscript
  Subscript
  Formula
Size
Alignment
  Left
  Center
  Right
  Justified
Line Spacing...
----
Chemical Check
Color
  Black
  Red
  Blue
  Green
  Yellow
  Orange
  Purple
  Gray
  Other...
```

Disabled states:

- `Select All`: disabled when the entire editable text is already selected.
- `Chemical Check`: checkable, default unchecked for standalone text editing.

Notes:

- We can decide later whether to preserve native browser text menu or fully replace it.

### 18. Selection Handle

Hit range: the 8 resize handles or rotate handle.

Behavior:

- No dedicated handle menu.
- Use the same menu as the current selection.

### 19. Virtual Focus Target in Drawing Tools

Hit range: an object that is not visibly hover-focused in the current drawing tool, but would be selectable by context hit-test.

Behavior:

- Treat the right-click target as selected for the menu.
- Do not show hover focus dots.
- Do not permanently switch tools.
- After the menu command completes, the temporary selection should disappear if the user was in a drawing tool and the command did not naturally create or modify selection.
