# ChemCore Selector v1

Protocol id: `chemcore-selector.v1`.

Selectors identify document regions and editable entities for CLI commands.
They are stable strings intended for agent storage, logging, and follow-up
queries.

## Stable Forms

```text
all
object:<scene-object-id>
molecule:<zero-based-molecule-index>
node:<node-id>
bond:<bond-id>
bounds:<minX>,<minY>,<maxX>,<maxY>
selection:<selector;selector>
```

Commands that accept multiple targets may also accept repeated `--target`
arguments or a JSON array in session mode. Multi-target capture and context use
the minimum union bounds of the selected targets, matching the GUI selection
box.

## Stability Rules

- The selector prefixes above are stable in v1.
- `object`, `node`, and `bond` ids are document ids. They remain stable across
  read-only inspection of a document.
- `molecule:<index>` is stable for a loaded document revision, but edits that
  add, delete, or reorder molecule objects may change later molecule indices.
- `bounds` uses world-space points and requires `maxX > minX` and `maxY > minY`.
- `selection` members may be any non-`all` selector. Use `all` by itself.

## Discovery Flow

Use:

```powershell
chemcore-cli targets input.cdxml --out targets.json --pretty
chemcore-cli context input.cdxml --target object:obj_001 --out context.json --pretty
chemcore-cli detail input.cdxml --target object:obj_001 --out detail.json --pretty
```

Automation should discover selectors from `targets` or `context` instead of
guessing ids.
