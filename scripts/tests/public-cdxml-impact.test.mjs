import assert from "node:assert/strict";
import test from "node:test";
import {
  mapWithConcurrency,
  mergeIncrementalManifestItems,
} from "../render-public-cdxml-visual-review.mjs";
import { classifyBaselineChanges } from "../public-cdxml-visual-gate.mjs";
import { featuresFromCdxml, selectAffectedCases } from "../public-cdxml-impact.mjs";
import {
  GALLERY_PROVENANCE_SCHEMA,
  publicCdxmlCliCandidates,
  provenanceMismatches,
} from "../public-cdxml-provenance.mjs";
import { bracketWorldEndpoints } from "../public-cdxml-bracket-geometry.mjs";
import { compareVisualGeometry } from "../public-cdxml-semantic-geometry.mjs";
import { visibleInterchangeBondCount } from "../public-cdxml-source-topology.mjs";

test("bracket semantic geometry follows ordered rotated spine endpoints", () => {
  const endpoints = bracketWorldEndpoints({
    bbox: [0, 0, 2, 10],
    translate: [5, 7],
    rotate: 90,
    kind: "square",
    side: "left",
  });
  assert.ok(endpoints);
  assert.deepEqual(endpoints.top, [11, 11]);
  assert.deepEqual(endpoints.bottom, [1, 11]);
});

test("visual geometry matches repeated text by tolerance instead of coordinate sort order", () => {
  const item = (position) => ({
    key: "[\"3\",\"left\"]",
    position,
    box: [position[0], position[1], position[0] + 2, position[1] + 5],
    lineHeight: [5],
  });
  const before = [
    item([202.48, 632.07]),
    item([202.48, 705.28]),
  ];
  const after = [
    item([202.47, 705.28]),
    item([202.48, 632.07]),
  ];
  assert.equal(compareVisualGeometry(before, after), true);
});

test("visual geometry still rejects unmatched repeated text", () => {
  const before = [{
    key: "caption",
    position: [10, 10],
    box: [10, 10, 20, 20],
    lineHeight: [8],
  }];
  const after = [{
    key: "caption",
    position: [10, 12],
    box: [10, 12, 20, 22],
    lineHeight: [8],
  }];
  assert.equal(compareVisualGeometry(before, after), false);
});

test("source topology ignores bonds inside node-owned fragments", () => {
  const bond = (id) => ({ name: "b", id, children: [] });
  const root = {
    name: "CDXML",
    children: [{
      name: "page",
      children: [
        {
          name: "fragment",
          children: [bond("visible-1"), bond("visible-2")],
        },
        {
          name: "fragment",
          children: [{
            name: "n",
            children: [{
              name: "fragment",
              children: [bond("embedded-definition")],
            }],
          }],
        },
      ],
    }],
  };
  assert.equal(visibleInterchangeBondCount(root), 2);
});

test("CDXML feature extraction recognizes visual rule families", () => {
  const features = featuresFromCdxml(`
    <CDXML><fragment><n id="1" NodeType="Nickname"><t><s>Me</s></t></n>
    <n id="2" EnhancedStereoType="Or"/><b B="1" E="2" Display="WedgedHashBegin">
    <objecttag Name="query"><t><s>Rxn</s></t></objecttag></b></fragment></CDXML>
  `);
  for (const expected of ["bond", "text", "nickname", "enhanced-stereo", "hashed-wedge", "object-tag", "query"]) {
    assert.ok(features.includes(expected), expected);
  }
});

test("CDXML query extraction distinguishes content from visibility defaults", () => {
  const visibilityOnly = featuresFromCdxml(`
    <CDXML ShowAtomQuery="yes" ShowBondQuery="yes"><page><fragment>
      <n id="1" p="0 0" Element="6"/>
    </fragment></page></CDXML>
  `);
  assert.ok(visibilityOnly.includes("query-visibility"));
  assert.ok(!visibilityOnly.includes("query"));

  for (const source of [
    `<CDXML><n ImplicitHydrogens="yes"/></CDXML>`,
    `<CDXML><n FreeSites="2" RingBondCount="AsDrawn"/></CDXML>`,
    `<CDXML><n UnsaturatedBonds="MustBePresent" SubstituentsExactly="2"/></CDXML>`,
    `<CDXML><n Translation="Broad" IsotopicAbundance="Any"/></CDXML>`,
    `<CDXML><n NodeType="ElementList" ElementList="6 7 8"/></CDXML>`,
    `<CDXML><objecttag Name="query"><t><s>R</s></t></objecttag></CDXML>`,
  ]) {
    assert.ok(featuresFromCdxml(source).includes("query"), source);
  }
});

test("affected selection combines feature hits and historical regressions", () => {
  const featureIndex = {
    cases: [
      { caseId: "0001", relativeCdxml: "a.cdxml", format: "cdxml", features: ["hashed-wedge"] },
      { caseId: "0002", relativeCdxml: "b.cdxml", format: "cdxml", features: ["text"] },
      { caseId: "0003", relativeCdxml: "c.cdx", format: "cdx", features: ["hashed-wedge"] },
    ],
  };
  const impactMap = {
    rules: [{
      name: "hash",
      pathSubstrings: ["bond_metrics.rs"],
      features: ["hashed-wedge"],
      regressionCases: ["0002"],
    }],
    productionPathPrefixes: ["crates/"],
    ignoredPathPrefixes: [],
    unknownProductionChange: "full",
  };
  const result = selectAffectedCases({
    changedFiles: ["crates/engine/bond_metrics.rs"],
    featureIndex,
    impactMap,
  });
  assert.deepEqual(result.selected.map((entry) => entry.caseId), ["0001", "0002", "0003"]);
  assert.equal(result.forceFull, false);
});

test("unknown production changes conservatively force a full selection", () => {
  const result = selectAffectedCases({
    changedFiles: ["crates/engine/new_renderer.rs"],
    featureIndex: {
      cases: [
        { caseId: "0001", relativeCdxml: "a.cdxml", format: "cdxml", features: [] },
        { caseId: "0002", relativeCdxml: "b.cdx", format: "cdx", features: [] },
      ],
    },
    impactMap: {
      rules: [],
      productionPathPrefixes: ["crates/"],
      ignoredPathPrefixes: [],
      unknownProductionChange: "full",
    },
  });
  assert.equal(result.forceFull, true);
  assert.equal(result.selected.length, 2);
});

test("incremental manifest replacement preserves full gallery order", () => {
  const retained = [{ id: "a", value: 1, label: "001 — a" }, { id: "b", value: 2, label: "002 — b" }];
  const updated = [{ id: "b", value: 3, label: "001 — b" }, { id: "c", value: 4, label: "002 — c" }];
  assert.deepEqual(mergeIncrementalManifestItems(retained, updated), [
    { id: "a", value: 1, label: "001 — a" },
    { id: "b", value: 3, label: "002 — b" },
    { id: "c", value: 4, label: "002 — c" },
  ]);
});

test("visual workers run concurrently while preserving manifest order", async () => {
  let active = 0;
  let maximumActive = 0;
  const results = await mapWithConcurrency([30, 10, 20, 0], 2, async (value) => {
    active += 1;
    maximumActive = Math.max(maximumActive, active);
    await new Promise((resolve) => setTimeout(resolve, value));
    active -= 1;
    return value;
  });
  assert.deepEqual(results, [30, 10, 20, 0]);
  assert.equal(maximumActive, 2);
});

test("gallery provenance invalidates stale source, CLI, corpus, and report inputs", () => {
  const recorded = {
    schema: GALLERY_PROVENANCE_SCHEMA,
    repository: { identity: "repo-a" },
    cli: { sha256: "cli-a", buildIdentity: "repo-a" },
    corpus: {
      manifestSha256: "manifest-a",
      sources: [{ id: "rdkit", actualRevision: "revision-a" }],
    },
    roundtripReport: { sha256: "report-a" },
  };
  assert.deepEqual(provenanceMismatches(recorded, structuredClone(recorded)), []);
  const current = structuredClone(recorded);
  current.repository.identity = "repo-b";
  current.cli.sha256 = "cli-b";
  current.cli.buildIdentity = "repo-b";
  current.corpus.manifestSha256 = "manifest-b";
  current.corpus.sources[0].actualRevision = "revision-b";
  current.roundtripReport.sha256 = "report-b";
  assert.deepEqual(provenanceMismatches(recorded, current), [
    "repository-state",
    "cli-binary",
    "cli-build-identity",
    "corpus-manifest",
    "corpus-source:rdkit",
    "roundtrip-report",
  ]);
});

test("public CDXML gates prefer the release CLI produced by the canonical builder", () => {
  assert.deepEqual(
    publicCdxmlCliCandidates("D:\\repo", null, "win32"),
    [
      "D:\\repo\\target\\release\\chemsema-cli.exe",
      "D:\\repo\\target\\debug\\chemsema-cli.exe",
    ],
  );
  assert.equal(
    publicCdxmlCliCandidates("D:\\repo", "D:\\custom\\cli.exe", "win32")[0],
    "D:\\custom\\cli.exe",
  );
});

test("baseline mode blocks regressions without requiring historical failures to turn green", () => {
  const baseline = new Map([
    ["old-failure.cdxml", { status: "fail" }],
    ["regression.cdxml", { status: "pass" }],
    ["improvement.cdxml", { status: "fail" }],
  ]);
  const delta = classifyBaselineChanges([
    { relativeCdxml: "old-failure.cdxml", status: "fail" },
    { relativeCdxml: "regression.cdxml", status: "fail" },
    { relativeCdxml: "improvement.cdxml", status: "pass" },
  ], baseline);
  assert.equal(delta.regressions.length, 1);
  assert.equal(delta.improvements.length, 1);
  assert.equal(delta.changes.length, 2);
});
