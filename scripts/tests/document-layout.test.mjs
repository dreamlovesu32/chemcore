import test from "node:test";
import assert from "node:assert/strict";

import { resolveLayout } from "../../viewer/document_layout_host.js";
import { buildImagePagesPdf } from "../../viewer/export_preview.js";
import {
  headerFooterSections,
  pageTextAnnotations,
  pageTrimMarkSegments,
} from "../../viewer/document_page_decorations.js";

const A4 = { width: 595.275590551, height: 841.88976378 };

function layout(overrides = {}) {
  return {
    drawingSpace: "pages",
    paper: A4,
    widthPages: 1,
    heightPages: 1,
    autoPaginate: true,
    pageOrigin: null,
    margins: [36, 36, 36, 36],
    pageOverlap: 0,
    printTrimMarks: false,
    header: "",
    headerPosition: 36,
    footer: "",
    footerPosition: 36,
    magnificationPercent: 100,
    pageDefinition: "undefined",
    splitters: [],
    legacySplitterPositionIds: [],
    fixInPlaceExtent: null,
    fixInPlaceGap: null,
    ...overrides,
  };
}

test("initial paper layout centers content on the minimum A4 tile grid", () => {
  const resolved = resolveLayout(layout(), {
    minX: 100,
    minY: 200,
    maxX: 900,
    maxY: 500,
  });
  assert.equal(resolved.widthPages, 2);
  assert.equal(resolved.heightPages, 1);
  assert.equal(resolved.anchorOrigin[0], 100 - (A4.width * 2 - 800) / 2);
  assert.equal(resolved.anchorOrigin[1], 200 - (A4.height - 300) / 2);
  assert.deepEqual(resolved.prependedPages, [0, 0]);
});

test("edits preserve the original page and prepend or append sheets as needed", () => {
  const anchored = layout({
    paper: { width: 100, height: 120 },
    pageOrigin: [30, 45],
  });
  const resolved = resolveLayout(anchored, {
    minX: -20,
    minY: -90,
    maxX: 260,
    maxY: 250,
  });
  assert.deepEqual(resolved.anchorOrigin, [30, 45]);
  assert.deepEqual(resolved.origin, [-70, -195]);
  assert.deepEqual(resolved.prependedPages, [1, 2]);
  assert.equal(resolved.widthPages, 4);
  assert.equal(resolved.heightPages, 4);
});

test("poster overlap changes sheet step without changing the fixed page anchor", () => {
  const resolved = resolveLayout(layout({
    drawingSpace: "poster",
    paper: { width: 100, height: 100 },
    pageOverlap: 10,
    pageOrigin: [0, 0],
  }), {
    minX: -1,
    minY: 0,
    maxX: 181,
    maxY: 99,
  });
  assert.deepEqual(resolved.origin, [-90, 0]);
  assert.deepEqual(resolved.anchorOrigin, [0, 0]);
  assert.equal(resolved.widthPages, 3);
  assert.equal(resolved.totalWidth, 280);
});

test("multi-page PDF contains one physical page object and MediaBox per tile", () => {
  const bytes = buildImagePagesPdf([
    {
      jpegBytes: new Uint8Array([0xff, 0xd8, 0xff, 0xd9]),
      imageWidth: 2,
      imageHeight: 2,
      pageWidthPt: 595.28,
      pageHeightPt: 841.89,
    },
    {
      jpegBytes: new Uint8Array([0xff, 0xd8, 0xff, 0xd9]),
      imageWidth: 2,
      imageHeight: 2,
      pageWidthPt: 792,
      pageHeightPt: 612,
    },
  ]);
  const text = Buffer.from(bytes).toString("latin1");
  assert.match(text, /\/Count 2\b/);
  assert.match(text, /\/MediaBox \[0 0 595\.28 841\.89\]/);
  assert.match(text, /\/MediaBox \[0 0 792 612\]/);
  assert.equal((text.match(/\/Type \/Page\b/g) || []).length, 2);
  assert.match(text, /startxref\n\d+\n%%EOF\n$/);
});

test("paper view and PDF share one header, footer, and trim-mark rule", () => {
  const now = new Date("2026-07-28T12:34:00Z");
  const configured = layout({
    header: "&lChemSema&cPage &p&r&f",
    headerPosition: 24,
    footer: "&cVerified &d &t",
    footerPosition: 25,
    printTrimMarks: true,
  });
  const page = {
    x: 100,
    y: 200,
    width: 300,
    height: 400,
    pageNumber: 2,
  };
  const annotations = pageTextAnnotations(
    { document: { title: "layout.ccjs" } },
    configured,
    page,
    now,
  );
  assert.deepEqual(
    annotations.slice(0, 3).map(({ role, pageNumber, anchor, text, x, y }) => ({
      role, pageNumber, anchor, text, x, y,
    })),
    [
      {
        role: "header",
        pageNumber: 2,
        anchor: "start",
        text: "ChemSema",
        x: 106,
        y: 224,
      },
      {
        role: "header",
        pageNumber: 2,
        anchor: "middle",
        text: "Page 2",
        x: 250,
        y: 224,
      },
      {
        role: "header",
        pageNumber: 2,
        anchor: "end",
        text: "layout.ccjs",
        x: 394,
        y: 224,
      },
    ],
  );
  assert.equal(annotations.at(-1).role, "footer");
  assert.equal(annotations.at(-1).y, 575);
  assert.match(annotations.at(-1).text, /^Verified /);

  assert.deepEqual(
    headerFooterSections("&c&& &p", { p: "7" }),
    { left: "", center: "&& 7", right: "" },
  );
  const trimMarks = pageTrimMarkSegments(page.x, page.y, page.width, page.height);
  assert.equal(trimMarks.length, 8);
  for (const [x1, y1, x2, y2] of trimMarks) {
    assert(x1 >= page.x && x1 <= page.x + page.width);
    assert(x2 >= page.x && x2 <= page.x + page.width);
    assert(y1 >= page.y && y1 <= page.y + page.height);
    assert(y2 >= page.y && y2 <= page.y + page.height);
  }
});
