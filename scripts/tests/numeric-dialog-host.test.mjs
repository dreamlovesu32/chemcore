import assert from "node:assert/strict";
import test from "node:test";
import { numericDialogMarkup } from "../../viewer/numeric_dialog_host.js";

test("numeric dialogs expose a named modal and a uniquely named value field", () => {
  const markup = numericDialogMarkup({
    title: "Line Spacing",
    field: { label: "Line Spacing", value: 12, unit: "pt" },
  });
  assert.match(markup, /role="dialog"/);
  assert.match(markup, /aria-modal="true"/);
  assert.match(markup, /aria-label="Line Spacing"/);
  assert.match(markup, /<input[^>]+name="value"[^>]+aria-label="Line Spacing"/);
  assert.match(markup, /value="12"/);
});

test("numeric dialog accessible names and values are HTML escaped", () => {
  const markup = numericDialogMarkup({
    title: "Scale <selection>",
    field: { label: "Value & unit", value: "\"unsafe\"", inputMode: "text" },
  });
  assert.match(markup, /aria-label="Scale &lt;selection&gt;"/);
  assert.match(markup, /aria-label="Value &amp; unit"/);
  assert.match(markup, /value="&quot;unsafe&quot;"/);
});
