import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

function parseArgs(argv) {
  const args = { scale: 3 };
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === "--manifest") args.manifest = argv[++i];
    else if (arg === "--scale") args.scale = Number(argv[++i]);
    else if (arg === "--help" || arg === "-h") args.help = true;
    else throw new Error(`Unknown argument: ${arg}`);
  }
  return args;
}

function attrValue(svg, name) {
  const match = svg.match(new RegExp(`\\b${name}\\s*=\\s*["']([^"']+)["']`, "i"));
  return match?.[1] ?? null;
}

function parseLength(value) {
  if (!value) return null;
  const match = String(value).trim().match(/^([+-]?\d+(?:\.\d+)?)(px|pt|in|cm|mm)?$/i);
  if (!match) return null;
  const number = Number(match[1]);
  const unit = (match[2] || "px").toLowerCase();
  if (!Number.isFinite(number) || number <= 0) return null;
  if (unit === "pt") return number * (96 / 72);
  if (unit === "in") return number * 96;
  if (unit === "cm") return number * (96 / 2.54);
  if (unit === "mm") return number * (96 / 25.4);
  return number;
}

function svgSize(svg) {
  const width = parseLength(attrValue(svg, "width"));
  const height = parseLength(attrValue(svg, "height"));
  if (width && height) return { width, height };
  const viewBox = attrValue(svg, "viewBox");
  if (viewBox) {
    const parts = viewBox.trim().split(/[,\s]+/).map(Number);
    if (parts.length === 4 && parts.every(Number.isFinite) && parts[2] > 0 && parts[3] > 0) {
      return { width: parts[2], height: parts[3] };
    }
  }
  return { width: 640, height: 480 };
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function svgDataUrl(svg) {
  return `data:image/svg+xml;base64,${Buffer.from(svg, "utf8").toString("base64")}`;
}

function fitSize(size, maxWidth, maxHeight) {
  const scale = Math.min(maxWidth / size.width, maxHeight / size.height, 1);
  return {
    width: Math.max(1, Math.ceil(size.width * scale)),
    height: Math.max(1, Math.ceil(size.height * scale)),
  };
}

async function renderOne(page, item) {
  const [chemdrawSvg, chemcoreSvg] = await Promise.all([
    fs.readFile(item.chemdrawSvg, "utf8"),
    fs.readFile(item.chemcoreSvg, "utf8"),
  ]);
  const leftRaw = svgSize(chemdrawSvg);
  const rightRaw = svgSize(chemcoreSvg);
  const maxPanelWidth = 1800;
  const maxPanelHeight = 1400;
  const left = fitSize(leftRaw, maxPanelWidth, maxPanelHeight);
  const right = fitSize(rightRaw, maxPanelWidth, maxPanelHeight);
  const panelWidth = Math.max(left.width, right.width, 320);
  const panelHeight = Math.max(left.height, right.height, 240);
  const viewport = {
    width: Math.ceil(panelWidth * 2 + 72),
    height: Math.ceil(panelHeight + 118),
  };
  await page.setViewportSize(viewport);

  const title = item.title || item.stem || path.basename(item.output);
  const source = item.sourcePpt ? `${item.sourcePpt}  /  ${item.embedding || ""}` : item.embedding || "";
  const html = `<!doctype html>
<html>
<head>
<meta charset="utf-8">
<style>
  * { box-sizing: border-box; }
  html, body { margin: 0; background: #fff; color: #111; font-family: "Segoe UI", Arial, sans-serif; }
  body { width: ${viewport.width}px; min-height: ${viewport.height}px; padding: 18px 24px 24px; }
  .header { height: 48px; display: flex; flex-direction: column; justify-content: center; border-bottom: 1px solid #d8d8d8; margin-bottom: 16px; }
  .name { font-size: 18px; line-height: 22px; font-weight: 600; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .source { font-size: 12px; line-height: 16px; color: #555; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .grid { display: grid; grid-template-columns: ${panelWidth}px ${panelWidth}px; gap: 24px; align-items: start; }
  .panel-title { height: 24px; font-size: 14px; line-height: 20px; font-weight: 600; color: #333; }
  .panel-body { width: ${panelWidth}px; height: ${panelHeight}px; display: flex; align-items: center; justify-content: center; background: #fff; border: 1px solid #e0e0e0; }
  img { display: block; object-fit: contain; max-width: ${panelWidth}px; max-height: ${panelHeight}px; }
</style>
</head>
<body>
  <div class="header">
    <div class="name">${escapeHtml(title)}</div>
    <div class="source">${escapeHtml(source)}</div>
  </div>
  <div class="grid">
    <section>
      <div class="panel-title">ChemDraw</div>
      <div class="panel-body"><img src="${svgDataUrl(chemdrawSvg)}" width="${left.width}" height="${left.height}"></div>
    </section>
    <section>
      <div class="panel-title">ChemCore</div>
      <div class="panel-body"><img src="${svgDataUrl(chemcoreSvg)}" width="${right.width}" height="${right.height}"></div>
    </section>
  </div>
</body>
</html>`;

  await page.setContent(html, { waitUntil: "networkidle" });
  await fs.mkdir(path.dirname(item.output), { recursive: true });
  await page.screenshot({ path: item.output, fullPage: true, scale: "device" });
  console.log(`[PNG] ${item.output}`);
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (args.help || !args.manifest) {
    console.log("Usage: node scripts/render-svg-compare-pngs.mjs --manifest manifest.json [--scale 3]");
    return;
  }
  const manifest = JSON.parse(await fs.readFile(args.manifest, "utf8"));
  const scale = Number.isFinite(args.scale) && args.scale > 0 ? args.scale : 3;
  const browser = await chromium.launch();
  const context = await browser.newContext({
    viewport: { width: 1200, height: 800 },
    deviceScaleFactor: scale,
  });
  const page = await context.newPage();
  try {
    for (const item of manifest.items || []) {
      await renderOne(page, item);
    }
  } finally {
    await browser.close();
  }
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.stack || error.message : String(error));
    process.exit(1);
  });
}
