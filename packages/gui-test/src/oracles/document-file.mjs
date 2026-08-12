import { spawn } from "node:child_process";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { repositoryRoot } from "../protocol/paths.mjs";

const defaultCliPath = join(repositoryRoot, "target", "release", "chemsema-cli.exe");
const outputLimit = 16 * 1024 * 1024;

function runCli(executable, args, { timeoutMs = 30000 } = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(executable, args, { windowsHide: true, stdio: ["ignore", "pipe", "pipe"] });
    let stdout = Buffer.alloc(0);
    let stderr = Buffer.alloc(0);
    let failure = null;
    const append = (current, chunk) => {
      const next = Buffer.concat([current, chunk]);
      if (next.length > outputLimit) {
        failure = new Error("Independent document oracle exceeded its 16 MiB output limit.");
        child.kill();
      }
      return next;
    };
    child.stdout.on("data", (chunk) => { stdout = append(stdout, chunk); });
    child.stderr.on("data", (chunk) => { stderr = append(stderr, chunk); });
    child.on("error", (error) => { failure = error; });
    const timer = setTimeout(() => {
      failure = new Error(`Independent document oracle exceeded ${timeoutMs} ms.`);
      child.kill();
    }, timeoutMs);
    child.on("close", (status) => {
      clearTimeout(timer);
      if (failure) return reject(failure);
      if (status !== 0) {
        const diagnostic = stderr.toString("utf8").trim() || stdout.toString("utf8").trim() || `exit ${status}`;
        const boundedDiagnostic = diagnostic.length > 4096 ? `${diagnostic.slice(0, 4096)}\n[truncated]` : diagnostic;
        return reject(new Error(`Independent document oracle failed: ${boundedDiagnostic}`));
      }
      try { resolve(JSON.parse(stdout.toString("utf8"))); } catch (error) {
        reject(new Error(`Independent document oracle returned invalid JSON: ${error.message}`));
      }
    });
  });
}

export function evaluateDocumentReports({ inspect, validation }, expected) {
  const counts = inspect?.summary?.counts || {};
  const observed = {
    formatName: inspect?.summary?.format?.name ?? null,
    formatVersion: inspect?.summary?.format?.version ?? null,
    nodes: counts.nodes ?? null,
    bonds: counts.bonds ?? null,
    molecules: counts.molecules ?? null,
    objects: counts.objects ?? null,
    validationSchema: validation?.schema ?? null,
    validationOk: validation?.ok === true,
    issueCount: Array.isArray(validation?.issues) ? validation.issues.length : null,
  };
  const passed = observed.formatName === "chemsema"
    && observed.formatVersion === "0.2"
    && observed.validationSchema === "chemsema.validation-report.v1"
    && observed.validationOk
    && observed.issueCount === 0
    && Object.entries(expected).every(([name, value]) => observed[name] === value);
  return { passed, observed };
}

export function evaluateDocumentBondProperties(bytes, expected) {
  let document;
  try {
    document = JSON.parse(Buffer.isBuffer(bytes) ? bytes.toString("utf8") : String(bytes));
  } catch {
    return { passed: false, observed: [] };
  }
  const resources = document?.resources && typeof document.resources === "object"
    ? Object.values(document.resources)
    : [];
  const bonds = resources.flatMap((resource) => Array.isArray(resource?.data?.bonds) ? resource.data.bonds : []);
  const observed = expected.map((entry) => {
    const bond = bonds.find((candidate) => candidate?.id === entry.id);
    return {
      id: entry.id,
      found: !!bond,
      order: bond?.order ?? null,
      mainLineStyle: bond?.lineStyles?.main ?? null,
      leftLineStyle: bond?.lineStyles?.left ?? null,
      rightLineStyle: bond?.lineStyles?.right ?? null,
      mainLineWeight: bond?.lineWeights?.main ?? null,
      doublePlacement: bond?.double?.placement ?? null,
      stereoKind: bond?.stereo?.kind ?? null,
      wideEnd: bond?.stereo?.wideEnd ?? bond?.stereo?.wide_end ?? null,
      queryOrders: bond?.properties?.queryOrders ?? bond?.properties?.query_orders ?? [],
      topology: bond?.properties?.topology ?? "unspecified",
      reactionParticipation: bond?.properties?.reactionParticipation ?? bond?.properties?.reaction_participation ?? null,
      absoluteStereo: bond?.properties?.absoluteStereo ?? bond?.properties?.absolute_stereo ?? "unspecified",
      showQuery: bond?.properties?.showQuery ?? bond?.properties?.show_query ?? null,
      showReaction: bond?.properties?.showReaction ?? bond?.properties?.show_reaction ?? null,
      showStereo: bond?.properties?.showStereo ?? bond?.properties?.show_stereo ?? null,
    };
  });
  const passed = observed.every((actual, index) => actual.found
    && Object.entries(expected[index]).every(([name, value]) => name === "id"
      || (Array.isArray(value)
        ? Array.isArray(actual[name]) && value.length === actual[name].length && value.every((entry, entryIndex) => entry === actual[name][entryIndex])
        : actual[name] === value)));
  return { passed, observed };
}

export function evaluateDocumentNodeProperties(bytes, expected) {
  let document;
  try {
    document = JSON.parse(Buffer.isBuffer(bytes) ? bytes.toString("utf8") : String(bytes));
  } catch {
    return { passed: false, observed: [] };
  }
  const resources = document?.resources && typeof document.resources === "object"
    ? Object.values(document.resources)
    : [];
  const nodes = resources.flatMap((resource) => Array.isArray(resource?.data?.nodes) ? resource.data.nodes : []);
  const observed = expected.map((entry) => {
    const node = nodes.find((candidate) => candidate?.id === entry.id);
    return {
      id: entry.id,
      found: !!node,
      element: node?.element ?? null,
      atomicNumber: node?.atomicNumber ?? node?.atomic_number ?? null,
      charge: node?.charge ?? null,
      numHydrogens: node?.numHydrogens ?? node?.num_hydrogens ?? null,
      numHydrogensOverride: !node
        ? null
        : Object.prototype.hasOwnProperty.call(node.meta ?? {}, "numHydrogensOverride")
          ? node.meta.numHydrogensOverride
          : null,
      radicalCount: !node
        ? null
        : Object.prototype.hasOwnProperty.call(node.meta ?? {}, "radicalCount")
          ? node.meta.radicalCount
          : 0,
      isotopeMass: node?.atomProperties?.isotopeMass ?? node?.atom_properties?.isotope_mass ?? null,
      isotopicAbundance: node?.atomProperties?.isotopicAbundance ?? node?.atom_properties?.isotopic_abundance ?? "unspecified",
      atomRadical: node?.atomProperties?.radical ?? "none",
      atomNumber: node?.atomProperties?.atomNumber ?? node?.atom_properties?.atom_number ?? null,
      showAtomNumber: node?.atomProperties?.showAtomNumber ?? node?.atom_properties?.show_atom_number ?? null,
      atomStereo: node?.atomProperties?.cipStereo ?? node?.atom_properties?.cip_stereo ?? null,
      showAtomStereo: node?.atomProperties?.showAtomStereo ?? node?.atom_properties?.show_atom_stereo ?? null,
      reactionChange: node?.atomProperties?.reactionChange ?? node?.atom_properties?.reaction_change ?? false,
      reactionStereo: node?.atomProperties?.reactionStereo ?? node?.atom_properties?.reaction_stereo ?? "unspecified",
      showAtomQuery: node?.atomProperties?.showAtomQuery ?? node?.atom_properties?.show_atom_query ?? null,
      ringBondCount: node?.atomProperties?.ringBondCount ?? node?.atom_properties?.ring_bond_count ?? "unspecified",
      unsaturatedBonds: node?.atomProperties?.unsaturatedBonds ?? node?.atom_properties?.unsaturated_bonds ?? "unspecified",
      queryTranslation: node?.atomProperties?.translation ?? "equal",
      abnormalValence: node?.atomProperties?.abnormalValence ?? node?.atom_properties?.abnormal_valence ?? false,
      elementList: node?.atomProperties?.elementList ?? node?.atom_properties?.element_list ?? [],
      elementListExcluded: node?.atomProperties?.elementListExcluded ?? node?.atom_properties?.element_list_excluded ?? false,
      genericList: node?.atomProperties?.genericList ?? node?.atom_properties?.generic_list ?? [],
      genericListExcluded: node?.atomProperties?.genericListExcluded ?? node?.atom_properties?.generic_list_excluded ?? false,
      freeSites: node?.atomProperties?.freeSites ?? node?.atom_properties?.free_sites ?? null,
      substituentsUpTo: node?.atomProperties?.substituentsUpTo ?? node?.atom_properties?.substituents_up_to ?? null,
      substituentsExactly: node?.atomProperties?.substituentsExactly ?? node?.atom_properties?.substituents_exactly ?? null,
      showTerminalCarbonLabel: node?.atomProperties?.showTerminalCarbonLabel ?? node?.atom_properties?.show_terminal_carbon_label ?? null,
      showNonTerminalCarbonLabel: node?.atomProperties?.showNonTerminalCarbonLabel ?? node?.atom_properties?.show_non_terminal_carbon_label ?? null,
      labelText: node?.label?.text ?? null,
      labelSourceText: node?.label?.sourceText ?? node?.label?.source_text ?? null,
    };
  });
  const passed = observed.every((actual, index) => actual.found
    && Object.entries(expected[index]).every(([name, value]) => name === "id"
      || (Array.isArray(value)
        ? Array.isArray(actual[name]) && JSON.stringify(actual[name]) === JSON.stringify(value)
        : actual[name] === value)));
  return { passed, observed };
}

export function evaluateDocumentArrowProperties(bytes, expected) {
  let document;
  try {
    document = JSON.parse(Buffer.isBuffer(bytes) ? bytes.toString("utf8") : String(bytes));
  } catch {
    return { passed: false, observed: [] };
  }
  const scene = Array.isArray(document?.entities?.scene) ? document.entities.scene : [];
  const styles = document?.styles && typeof document.styles === "object" ? document.styles : {};
  const observed = expected.map((entry) => {
    const object = scene.find((candidate) => candidate?.id === entry.id && candidate?.type === "line");
    const arrow = object?.payload?.arrowHead || {};
    return {
      id: entry.id,
      found: !!object,
      kind: arrow.kind ?? null,
      curve: arrow.curve ?? null,
      length: arrow.length ?? null,
      head: arrow.head ?? null,
      tail: arrow.tail ?? null,
      bold: arrow.bold ?? null,
      noGo: arrow.noGo ?? null,
      stroke: styles[object?.styleRef]?.stroke ?? object?.payload?.stroke ?? null,
    };
  });
  const passed = observed.every((actual, index) => actual.found
    && Object.entries(expected[index]).every(([name, value]) => name === "id" || actual[name] === value));
  return { passed, observed };
}

export function evaluateDocumentTextProperties(bytes, expected) {
  let document;
  try {
    document = JSON.parse(Buffer.isBuffer(bytes) ? bytes.toString("utf8") : String(bytes));
  } catch {
    return { passed: false, observed: [] };
  }
  const scene = Array.isArray(document?.entities?.scene) ? document.entities.scene : [];
  const styles = document?.styles && typeof document.styles === "object" ? document.styles : {};
  const observed = expected.map((entry) => {
    const object = scene.find((candidate) => candidate?.id === entry.id && candidate?.type === "text");
    const runSets = ["runs", "sourceRuns", "displayRuns"]
      .filter((key) => Array.isArray(object?.payload?.[key]))
      .map((key) => ({ key, runs: object.payload[key] }));
    const allRuns = runSets.flatMap(({ runs }) => runs);
    const uniformRunValue = (read) => {
      const values = allRuns.map(read);
      return values.length > 0 && values.every((value) => value === values[0]) ? values[0] : null;
    };
    return {
      id: entry.id,
      found: !!object,
      text: object?.payload?.text ?? null,
      fontFamily: object?.payload?.fontFamily ?? null,
      fontSize: object?.payload?.fontSize ?? null,
      align: object?.payload?.align ?? null,
      lineHeight: object?.payload?.lineHeight ?? null,
      lineHeightMode: object?.payload?.lineHeightMode ?? null,
      bold: allRuns.length > 0 && allRuns.every((run) => run?.fontWeight === 700),
      italic: allRuns.length > 0 && allRuns.every((run) => run?.fontStyle === "italic"),
      underline: allRuns.length > 0 && allRuns.every((run) => run?.underline === true),
      outline: allRuns.length > 0 && allRuns.every((run) => run?.outline === true),
      shadow: allRuns.length > 0 && allRuns.every((run) => run?.shadow === true),
      script: uniformRunValue((run) => run?.script ?? "normal"),
      fill: styles[object?.styleRef]?.fill ?? object?.payload?.fill ?? null,
      runFontFamilies: Object.fromEntries(runSets.map(({ key, runs: values }) => [
        key,
        values.map((run) => run?.fontFamily ?? null),
      ])),
      runFontSizes: Object.fromEntries(runSets.map(({ key, runs: values }) => [
        key,
        values.map((run) => run?.fontSize ?? null),
      ])),
      runFills: Object.fromEntries(runSets.map(({ key, runs: values }) => [
        key,
        values.map((run) => run?.fill ?? null),
      ])),
    };
  });
  const passed = observed.every((actual, index) => actual.found
    && Object.entries(expected[index]).every(([name, value]) => name === "id" || actual[name] === value)
    && (!Object.hasOwn(expected[index], "fontFamily") || Object.values(actual.runFontFamilies)
      .every((families) => families.every((family) => family === expected[index].fontFamily)))
    && (!Object.hasOwn(expected[index], "fontSize") || Object.values(actual.runFontSizes)
      .every((sizes) => sizes.every((size) => size === expected[index].fontSize)))
    && (!Object.hasOwn(expected[index], "fill") || Object.values(actual.runFills)
      .every((fills) => fills.every((fill) => fill === expected[index].fill))));
  return { passed, observed };
}

export function evaluateDocumentShapeProperties(bytes, expected) {
  let document;
  try {
    document = JSON.parse(Buffer.isBuffer(bytes) ? bytes.toString("utf8") : String(bytes));
  } catch {
    return { passed: false, observed: [] };
  }
  const scene = Array.isArray(document?.entities?.scene) ? document.entities.scene : [];
  const styles = document?.styles && typeof document.styles === "object" ? document.styles : {};
  const observed = expected.map((entry) => {
    const object = scene.find((candidate) => candidate?.id === entry.id && candidate?.type === "shape");
    const style = styles[object?.styleRef] || {};
    return {
      id: entry.id,
      found: !!object,
      kind: object?.payload?.kind ?? null,
      fill: style.fill ?? null,
      stroke: style.stroke ?? null,
      strokeWidth: style.strokeWidth ?? null,
      dashArray: Array.isArray(style.dashArray) ? style.dashArray : null,
      shaded: style.shaded === true,
      shadow: style.shadow === true,
      shadowSize: style.shadowSize ?? null,
    };
  });
  const passed = observed.every((actual, index) => actual.found
    && Object.entries(expected[index]).every(([name, value]) => name === "id" || JSON.stringify(actual[name]) === JSON.stringify(value)));
  return { passed, observed };
}

export function evaluateDocumentOrbitalProperties(bytes, expected) {
  let document;
  try {
    document = JSON.parse(Buffer.isBuffer(bytes) ? bytes.toString("utf8") : String(bytes));
  } catch {
    return { passed: false, observed: [] };
  }
  const scene = Array.isArray(document?.entities?.scene) ? document.entities.scene : [];
  const styles = document?.styles && typeof document.styles === "object" ? document.styles : {};
  const finitePoint = (value) => Array.isArray(value) && value.length === 2 && value.every(Number.isFinite);
  const observed = expected.map((entry) => {
    const object = scene.find((candidate) => candidate?.id === entry.id
      && candidate?.type === "shape"
      && candidate?.payload?.kind === "orbital");
    const payload = object?.payload || {};
    const style = styles[object?.styleRef] || {};
    const axisStart = payload.axisStart;
    const axisEnd = payload.axisEnd;
    const bbox = Array.isArray(payload.bbox) ? payload.bbox : null;
    const axisLength = finitePoint(axisStart) && finitePoint(axisEnd)
      ? Math.hypot(axisEnd[0] - axisStart[0], axisEnd[1] - axisStart[1])
      : null;
    const geometryValid = finitePoint(axisStart)
      && finitePoint(axisEnd)
      && Math.hypot(axisEnd[0] - axisStart[0], axisEnd[1] - axisStart[1]) > 0
      && !Object.hasOwn(payload, "center")
      && !Object.hasOwn(payload, "majorAxisEnd")
      && !Object.hasOwn(payload, "minorAxisEnd");
    return {
      id: entry.id,
      found: !!object,
      kind: payload.kind ?? null,
      template: payload.orbitalTemplate ?? null,
      orbitalStyle: payload.orbitalStyle ?? null,
      phase: payload.orbitalPhase ?? null,
      color: payload.orbitalColor ?? null,
      geometryValid,
      axisLength,
      bboxSize: bbox?.length === 4 ? [bbox[2], bbox[3]] : null,
      angle: payload.angle ?? null,
      fill: style.fill ?? null,
      stroke: style.stroke ?? null,
      strokeWidth: style.strokeWidth ?? null,
      dashArray: Array.isArray(style.dashArray) ? style.dashArray : null,
      shaded: style.shaded === true,
    };
  });
  const passed = observed.every((actual, index) => actual.found
    && actual.geometryValid
    && Object.entries(expected[index]).every(([name, value]) => name === "id"
      || JSON.stringify(actual[name]) === JSON.stringify(value)));
  return { passed, observed };
}

export function evaluateDocumentChromatographyProperties(bytes, expected) {
  let document;
  try {
    document = JSON.parse(Buffer.isBuffer(bytes) ? bytes.toString("utf8") : String(bytes));
  } catch {
    return { passed: false, observed: [] };
  }
  const scene = Array.isArray(document?.entities?.scene) ? document.entities.scene : [];
  const styles = document?.styles && typeof document.styles === "object" ? document.styles : {};
  const observed = expected.map((entry) => {
    const object = scene.find((candidate) => candidate?.id === entry.id
      && candidate?.type === "shape"
      && candidate?.payload?.kind === entry.kind);
    const style = styles[object?.styleRef] || {};
    if (entry.kind === "tlcPlate") {
      const lanes = Array.isArray(object?.payload?.lanes) ? object.payload.lanes : [];
      return {
        id: entry.id,
        found: !!object,
        kind: object?.payload?.kind ?? null,
        laneCount: lanes.length,
        offsetsValid: lanes.length > 0 && lanes.every((lane, index) => Number.isFinite(lane?.offset)
          && lane.offset > 0 && lane.offset < 1 && (index === 0 || lane.offset > lanes[index - 1].offset)),
        firstMarkValues: lanes.map((lane) => lane?.spots?.[0]?.rf ?? null),
        markColors: lanes.flatMap((lane) => lane?.spots || []).map((spot) => spot?.color ?? null),
        color: style.stroke ?? null,
        showOrigin: object?.payload?.showOrigin ?? null,
        showSolventFront: object?.payload?.showSolventFront ?? null,
        showBorders: object?.payload?.showBorders ?? null,
        showSideTicks: object?.payload?.showSideTicks ?? null,
      };
    }
    const gel = object?.payload?.gelElectrophoresis;
    const lanes = Array.isArray(gel?.lanes) ? gel.lanes : [];
    return {
      id: entry.id,
      found: !!object,
      kind: object?.payload?.kind ?? null,
      laneCount: lanes.length,
      offsetsValid: lanes.length > 0,
      firstMarkValues: lanes.map((lane) => lane?.bands?.[0]?.value ?? null),
      markColors: lanes.flatMap((lane) => lane?.bands || []).map((band) => band?.color ?? null),
      color: gel?.color ?? null,
      showOrigin: null,
      showSolventFront: null,
      showBorders: null,
      showSideTicks: null,
    };
  });
  const passed = observed.every((actual, index) => actual.found
    && actual.offsetsValid
    && actual.markColors.length >= actual.laneCount
    && actual.markColors.every((color) => color === expected[index].color)
    && Object.entries(expected[index]).every(([name, value]) => name === "id"
      || JSON.stringify(actual[name]) === JSON.stringify(value)));
  return { passed, observed };
}

export function evaluateDocumentSymbolProperties(bytes, expected) {
  let document;
  try {
    document = JSON.parse(Buffer.isBuffer(bytes) ? bytes.toString("utf8") : String(bytes));
  } catch {
    return { passed: false, observed: [] };
  }
  const scene = Array.isArray(document?.entities?.scene) ? document.entities.scene : [];
  const styles = document?.styles && typeof document.styles === "object" ? document.styles : {};
  const observed = expected.map((entry) => {
    const object = scene.find((candidate) => candidate?.id === entry.id && candidate?.type === "symbol");
    const style = styles[object?.styleRef] || {};
    return {
      id: entry.id,
      found: !!object,
      kind: object?.payload?.kind ?? null,
      payloadFill: object?.payload?.fill ?? null,
      styleFill: style.fill ?? null,
      styleKind: style.kind ?? null,
      symbolStyle: object?.payload?.symbolStyle ?? null,
      chemicalRole: object?.payload?.chemicalRole ?? null,
      chargeDelta: object?.payload?.chargeDelta ?? null,
      radicalDelta: object?.payload?.radicalDelta ?? null,
      attachedAtomId: object?.payload?.attachedAtomId ?? null,
      attachmentSource: object?.payload?.attachmentSource ?? null,
    };
  });
  const passed = observed.every((actual, index) => actual.found
    && Object.entries(expected[index]).every(([name, value]) => name === "id" || JSON.stringify(actual[name]) === JSON.stringify(value)));
  return { passed, observed };
}

export function evaluateDocumentBracketProperties(bytes, expected) {
  let document;
  try {
    document = JSON.parse(Buffer.isBuffer(bytes) ? bytes.toString("utf8") : String(bytes));
  } catch {
    return { passed: false, observed: [] };
  }
  const roots = Array.isArray(document?.entities?.scene) ? document.entities.scene : [];
  const objects = [];
  const visit = (object) => {
    if (!object || typeof object !== "object") return;
    objects.push(object);
    if (Array.isArray(object.children)) object.children.forEach(visit);
  };
  roots.forEach(visit);
  const observed = expected.map((entry) => {
    const group = objects.find((object) => object?.id === entry.id && object?.type === "group");
    const childIds = Array.isArray(document?.hierarchy?.children?.[entry.id])
      ? document.hierarchy.children[entry.id]
      : [];
    const children = entry.children.map((childEntry) => {
      const child = objects.find((object) => object?.id === childEntry.id && object?.type === "bracket");
      return {
        id: childEntry.id,
        found: !!child,
        kind: child?.payload?.kind ?? null,
        side: child?.payload?.side ?? null,
        stroke: child?.payload?.stroke ?? null,
      };
    });
    return { id: entry.id, found: !!group, childIds, children };
  });
  const passed = observed.every((actual, index) => {
    const wanted = expected[index];
    return actual.found
      && JSON.stringify(actual.childIds) === JSON.stringify(wanted.children.map((child) => child.id))
      && wanted.children.every((child, childIndex) => actual.children[childIndex]?.found
        && Object.entries(child).every(([name, value]) => name === "id" || actual.children[childIndex][name] === value));
  });
  return { passed, observed };
}

export function evaluateDocumentTableProperties(bytes, expected) {
  let document;
  try {
    document = JSON.parse(Buffer.isBuffer(bytes) ? bytes.toString("utf8") : String(bytes));
  } catch {
    return { passed: false, observed: [] };
  }
  const scene = Array.isArray(document?.entities?.scene) ? document.entities.scene : [];
  const observed = expected.map((entry) => {
    const object = scene.find((candidate) => candidate?.id === entry.id && candidate?.type === "table");
    const table = object?.payload?.table;
    const cells = Array.isArray(table?.cells) ? table.cells : [];
    return {
      id: entry.id,
      found: !!object,
      rows: table?.rows ?? null,
      columns: table?.columns ?? null,
      rowGuides: Array.isArray(table?.rowGuides) ? table.rowGuides : [],
      columnGuides: Array.isArray(table?.columnGuides) ? table.columnGuides : [],
      cellCount: cells.length,
      uniqueCellIds: new Set(cells.map((cell) => cell?.id).filter(Boolean)).size,
      cells: entry.cells.map((wanted) => {
        const cell = cells.find((candidate) => candidate?.row === wanted.row && candidate?.column === wanted.column);
        return {
          row: wanted.row,
          column: wanted.column,
          found: !!cell,
          horizontalAlignment: cell?.horizontalAlignment ?? null,
          verticalAlignment: cell?.verticalAlignment ?? null,
          borders: cell?.borders ?? {},
        };
      }),
    };
  });
  const strictlyIncreasing = (values) => values.every((value, index) => Number.isFinite(value)
    && (index === 0 || value > values[index - 1]));
  const passed = observed.every((actual, index) => {
    const wanted = expected[index];
    return actual.found
      && actual.rows === wanted.rows
      && actual.columns === wanted.columns
      && actual.rowGuides.length === wanted.rows + 1
      && actual.columnGuides.length === wanted.columns + 1
      && strictlyIncreasing(actual.rowGuides)
      && strictlyIncreasing(actual.columnGuides)
      && actual.cellCount === wanted.rows * wanted.columns
      && actual.uniqueCellIds === actual.cellCount
      && wanted.cells.every((cell, cellIndex) => {
        const found = actual.cells[cellIndex];
        return found?.found
          && found.horizontalAlignment === cell.horizontalAlignment
          && found.verticalAlignment === cell.verticalAlignment
          && Object.entries(cell.borders).every(([side, border]) => JSON.stringify(found.borders?.[side]) === JSON.stringify(border));
      });
  });
  return { passed, observed };
}

export function validationLevelForDocumentBytes(bytes) {
  try {
    const document = JSON.parse(Buffer.isBuffer(bytes) ? bytes.toString("utf8") : String(bytes));
    const resources = document?.resources && typeof document.resources === "object"
      ? Object.values(document.resources)
      : [];
    const hasChemicalGraph = resources.some((resource) =>
      resource?.type === "molecule_fragment2d"
      && Array.isArray(resource?.data?.nodes)
      && resource.data.nodes.length > 0
    );
    return hasChemicalGraph ? "chemical" : "structural";
  } catch {
    return "structural";
  }
}

export async function inspectDocumentBytes(bytes, { cliPath = defaultCliPath } = {}) {
  if (!Buffer.isBuffer(bytes) || bytes.length === 0 || bytes.length > 64 * 1024 * 1024) {
    throw new Error("Independent document oracle requires a nonempty CCJS payload up to 64 MiB.");
  }
  const root = await mkdtemp(join(tmpdir(), "chemsema-document-oracle-"));
  try {
    const documentPath = join(root, "saved-document.ccjs");
    await writeFile(documentPath, bytes, { flag: "wx" });
    const validationLevel = validationLevelForDocumentBytes(bytes);
    const [inspect, validation] = await Promise.all([
      runCli(cliPath, ["inspect", documentPath, "--include", "summary,objects,molecules,resources"]),
      runCli(cliPath, ["validate", documentPath, "--level", validationLevel]),
    ]);
    return { inspect, validation, validationLevel };
  } finally {
    await rm(root, { recursive: true, force: true });
  }
}
