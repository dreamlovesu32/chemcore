import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { launchBrowser } from "./playwright-browser.mjs";

function parseArgs(argv) {
  const args = {
    outDir: "tmp/svg-pixel-compare",
    baseScale: 4,
    searchLimit: 24,
    threshold: 740,
    labelLeft: "ChemDraw",
    labelRight: "ChemSema",
    inputs: [],
  };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--out") {
      args.outDir = argv[++index];
    } else if (arg === "--base-scale") {
      args.baseScale = Number(argv[++index]);
    } else if (arg === "--search-limit") {
      args.searchLimit = Number(argv[++index]);
    } else if (arg === "--threshold") {
      args.threshold = Number(argv[++index]);
    } else if (arg === "--left-label") {
      args.labelLeft = argv[++index];
    } else if (arg === "--right-label") {
      args.labelRight = argv[++index];
    } else if (arg === "--help" || arg === "-h") {
      args.help = true;
    } else {
      args.inputs.push(arg);
    }
  }
  return args;
}

function encodeSvg(svg) {
  return Buffer.from(svg, "utf8").toString("base64");
}

export async function compareSvgPixels(options) {
  const outDir = path.resolve(options.outDir ?? "tmp/svg-pixel-compare");
  const leftPath = path.resolve(options.leftPath);
  const rightPath = path.resolve(options.rightPath);
  const leftLabel = options.leftLabel ?? "Left";
  const rightLabel = options.rightLabel ?? "Right";
  const baseScale = Number(options.baseScale ?? 4);
  const searchLimit = Number(options.searchLimit ?? 24);
  const threshold = Number(options.threshold ?? 740);
  const rightScaleMultiplier = options.rightScaleMultiplier == null
    ? null
    : Number(options.rightScaleMultiplier);

  const [leftSvg, rightSvg] = await Promise.all([
    fs.readFile(leftPath, "utf8"),
    fs.readFile(rightPath, "utf8"),
  ]);

  await fs.mkdir(outDir, { recursive: true });

  const browser = await launchBrowser({ headless: true });
  const page = await browser.newPage();

  try {
    const result = await page.evaluate(
      async ({
        leftSvg,
        rightSvg,
        leftLabel,
        rightLabel,
        baseScale,
        searchLimit,
        threshold,
        rightScaleMultiplier,
      }) => {
        function svgDataUrl(svg) {
          return `data:image/svg+xml;base64,${btoa(unescape(encodeURIComponent(svg)))}`;
        }

        async function rasterize(svg, scale) {
          const image = new Image();
          image.decoding = "sync";
          image.src = svgDataUrl(svg);
          await image.decode();
          const width = Math.max(1, Math.round(image.naturalWidth * scale));
          const height = Math.max(1, Math.round(image.naturalHeight * scale));
          const canvas = document.createElement("canvas");
          canvas.width = width;
          canvas.height = height;
          const context = canvas.getContext("2d", { willReadFrequently: true });
          context.fillStyle = "#ffffff";
          context.fillRect(0, 0, width, height);
          context.drawImage(image, 0, 0, width, height);
          return { canvas, context, width, height };
        }

        function rgba(canvas) {
          const context = canvas.getContext("2d", { willReadFrequently: true });
          return context.getImageData(0, 0, canvas.width, canvas.height);
        }

        function inkMask(imageData, threshold) {
          const mask = new Uint8Array(imageData.width * imageData.height);
          for (let index = 0; index < mask.length; index += 1) {
            const offset = index * 4;
            const r = imageData.data[offset];
            const g = imageData.data[offset + 1];
            const b = imageData.data[offset + 2];
            const a = imageData.data[offset + 3];
            mask[index] = a > 0 && r + g + b < threshold ? 1 : 0;
          }
          return mask;
        }

        function maskBBox(mask, width, height) {
          let left = width;
          let top = height;
          let right = -1;
          let bottom = -1;
          for (let y = 0; y < height; y += 1) {
            for (let x = 0; x < width; x += 1) {
              if (!mask[y * width + x]) continue;
              if (x < left) left = x;
              if (y < top) top = y;
              if (x > right) right = x;
              if (y > bottom) bottom = y;
            }
          }
          if (right < left || bottom < top) {
            return { left: 0, top: 0, right: 0, bottom: 0, width: 0, height: 0 };
          }
          return {
            left,
            top,
            right,
            bottom,
            width: right - left + 1,
            height: bottom - top + 1,
          };
        }

        function panelFromRaster(raster, bbox, panelWidth, panelHeight, pad) {
          const canvas = document.createElement("canvas");
          canvas.width = panelWidth;
          canvas.height = panelHeight;
          const context = canvas.getContext("2d", { willReadFrequently: true });
          context.fillStyle = "#ffffff";
          context.fillRect(0, 0, panelWidth, panelHeight);
          context.drawImage(raster.canvas, Math.round(pad - bbox.left), Math.round(pad - bbox.top));
          return canvas;
        }

        function bestShift(leftMask, rightMask, width, height, limit) {
          let best = { iou: -1, dx: 0, dy: 0, overlap: 0, union: 0 };
          for (let dy = -limit; dy <= limit; dy += 1) {
            for (let dx = -limit; dx <= limit; dx += 1) {
              const oursY0 = Math.max(0, dy);
              const refY0 = Math.max(0, -dy);
              const oursX0 = Math.max(0, dx);
              const refX0 = Math.max(0, -dx);
              const h = height - Math.abs(dy);
              const w = width - Math.abs(dx);
              if (h <= 0 || w <= 0) continue;

              let overlap = 0;
              let union = 0;
              for (let y = 0; y < h; y += 1) {
                const oursRow = (oursY0 + y) * width;
                const refRow = (refY0 + y) * width;
                for (let x = 0; x < w; x += 1) {
                  const leftInk = leftMask[refRow + refX0 + x];
                  const rightInk = rightMask[oursRow + oursX0 + x];
                  if (leftInk || rightInk) {
                    union += 1;
                    if (leftInk && rightInk) overlap += 1;
                  }
                }
              }
              const iou = union === 0 ? 1 : overlap / union;
              if (
                iou > best.iou ||
                (iou === best.iou && Math.abs(dx) + Math.abs(dy) < Math.abs(best.dx) + Math.abs(best.dy))
              ) {
                best = { iou, dx, dy, overlap, union };
              }
            }
          }
          return best;
        }

        function shiftedMask(source, width, height, dx, dy) {
          const shifted = new Uint8Array(width * height);
          for (let y = 0; y < height; y += 1) {
            const sourceY = y - dy;
            if (sourceY < 0 || sourceY >= height) continue;
            for (let x = 0; x < width; x += 1) {
              const sourceX = x - dx;
              if (sourceX < 0 || sourceX >= width) continue;
              shifted[y * width + x] = source[sourceY * width + sourceX];
            }
          }
          return shifted;
        }

        function distanceField(mask, width, height) {
          const distance = new Float64Array(width * height);
          distance.fill(Number.POSITIVE_INFINITY);
          for (let index = 0; index < mask.length; index += 1) {
            if (mask[index]) distance[index] = 0;
          }
          const diagonal = Math.SQRT2;
          for (let y = 0; y < height; y += 1) {
            for (let x = 0; x < width; x += 1) {
              const index = y * width + x;
              let value = distance[index];
              if (x > 0) value = Math.min(value, distance[index - 1] + 1);
              if (y > 0) value = Math.min(value, distance[index - width] + 1);
              if (x > 0 && y > 0) {
                value = Math.min(value, distance[index - width - 1] + diagonal);
              }
              if (x + 1 < width && y > 0) {
                value = Math.min(value, distance[index - width + 1] + diagonal);
              }
              distance[index] = value;
            }
          }
          for (let y = height - 1; y >= 0; y -= 1) {
            for (let x = width - 1; x >= 0; x -= 1) {
              const index = y * width + x;
              let value = distance[index];
              if (x + 1 < width) value = Math.min(value, distance[index + 1] + 1);
              if (y + 1 < height) value = Math.min(value, distance[index + width] + 1);
              if (x + 1 < width && y + 1 < height) {
                value = Math.min(value, distance[index + width + 1] + diagonal);
              }
              if (x > 0 && y + 1 < height) {
                value = Math.min(value, distance[index + width - 1] + diagonal);
              }
              distance[index] = value;
            }
          }
          return distance;
        }

        function directedInkDistances(sourceMask, targetMask, width, height) {
          const field = distanceField(targetMask, width, height);
          const values = [];
          for (let index = 0; index < sourceMask.length; index += 1) {
            if (sourceMask[index]) values.push(field[index]);
          }
          values.sort((left, right) => left - right);
          const quantile = (fraction) => values.length === 0
            ? 0
            : values[Math.min(
              values.length - 1,
              Math.floor((values.length - 1) * fraction),
            )];
          return {
            count: values.length,
            p95: quantile(0.95),
            p99: quantile(0.99),
            p999: quantile(0.999),
            maximum: values.at(-1) ?? 0,
          };
        }

        function imageToCanvas(imageData) {
          const canvas = document.createElement("canvas");
          canvas.width = imageData.width;
          canvas.height = imageData.height;
          canvas.getContext("2d").putImageData(imageData, 0, 0);
          return canvas;
        }

        function makeOverlay(leftCanvas, rightCanvas, shift, threshold) {
          const width = leftCanvas.width;
          const height = leftCanvas.height;
          const leftImage = rgba(leftCanvas);
          const rightImage = rgba(rightCanvas);
          const leftMask = inkMask(leftImage, threshold);
          const rightMask = inkMask(rightImage, threshold);
          const overlay = new ImageData(width, height);
          const diff = new ImageData(width, height);

          let overlapPixels = 0;
          let leftOnlyPixels = 0;
          let rightOnlyPixels = 0;
          let differentPixels = 0;

          for (let index = 0; index < overlay.data.length; index += 4) {
            overlay.data[index] = 255;
            overlay.data[index + 1] = 255;
            overlay.data[index + 2] = 255;
            overlay.data[index + 3] = 255;
            diff.data[index] = 255;
            diff.data[index + 1] = 255;
            diff.data[index + 2] = 255;
            diff.data[index + 3] = 255;
          }

          for (let y = 0; y < height; y += 1) {
            for (let x = 0; x < width; x += 1) {
              const leftInk = leftMask[y * width + x];
              const shiftedX = x - shift.dx;
              const shiftedY = y - shift.dy;
              const rightInk =
                shiftedX >= 0 && shiftedX < width && shiftedY >= 0 && shiftedY < height
                  ? rightMask[shiftedY * width + shiftedX]
                  : 0;
              const offset = (y * width + x) * 4;
              if (leftInk && rightInk) {
                overlapPixels += 1;
                overlay.data[offset] = 0;
                overlay.data[offset + 1] = 0;
                overlay.data[offset + 2] = 0;
              } else if (leftInk) {
                leftOnlyPixels += 1;
                differentPixels += 1;
                overlay.data[offset] = 0;
                overlay.data[offset + 1] = 102;
                overlay.data[offset + 2] = 255;
                diff.data[offset] = 0;
                diff.data[offset + 1] = 102;
                diff.data[offset + 2] = 255;
              } else if (rightInk) {
                rightOnlyPixels += 1;
                differentPixels += 1;
                overlay.data[offset] = 255;
                overlay.data[offset + 1] = 59;
                overlay.data[offset + 2] = 48;
                diff.data[offset] = 255;
                diff.data[offset + 1] = 59;
                diff.data[offset + 2] = 48;
              }
            }
          }

          return {
            overlayCanvas: imageToCanvas(overlay),
            diffCanvas: imageToCanvas(diff),
            overlapPixels,
            leftOnlyPixels,
            rightOnlyPixels,
            differentPixels,
          };
        }

        function shiftedCanvas(sourceCanvas, dx, dy) {
          const canvas = document.createElement("canvas");
          canvas.width = sourceCanvas.width;
          canvas.height = sourceCanvas.height;
          const context = canvas.getContext("2d");
          context.fillStyle = "#ffffff";
          context.fillRect(0, 0, canvas.width, canvas.height);
          context.drawImage(sourceCanvas, dx, dy);
          return canvas;
        }

        function labelPanel(canvas, title) {
          const padding = 18;
          const titleHeight = 42;
          const output = document.createElement("canvas");
          output.width = canvas.width + padding * 2;
          output.height = canvas.height + titleHeight + padding * 2;
          const context = output.getContext("2d");
          context.fillStyle = "#ffffff";
          context.fillRect(0, 0, output.width, output.height);
          context.fillStyle = "#111111";
          context.font = "600 22px Arial";
          context.fillText(title, padding, 30);
          context.strokeStyle = "#d0d7de";
          context.lineWidth = 1;
          context.strokeRect(padding - 0.5, titleHeight - 0.5, canvas.width + 1, canvas.height + 1);
          context.drawImage(canvas, padding, titleHeight);
          return output;
        }

        function montage(panels, footerLines) {
          const gap = 24;
          const footerHeight = 26 * footerLines.length + 40;
          const width = panels.reduce((sum, panel) => sum + panel.width, 0) + gap * (panels.length - 1);
          const height = Math.max(...panels.map((panel) => panel.height)) + footerHeight;
          const canvas = document.createElement("canvas");
          canvas.width = width;
          canvas.height = height;
          const context = canvas.getContext("2d");
          context.fillStyle = "#ffffff";
          context.fillRect(0, 0, width, height);

          let x = 0;
          for (const panel of panels) {
            context.drawImage(panel, x, 0);
            x += panel.width + gap;
          }

          const footerTop = Math.max(...panels.map((panel) => panel.height)) + 16;
          context.fillStyle = "#111111";
          context.font = "16px Consolas, 'Courier New', monospace";
          footerLines.forEach((line, index) => {
            context.fillText(line, 0, footerTop + index * 24);
          });
          return canvas;
        }

        const leftRaster = await rasterize(leftSvg, baseScale);
        const rightInitialRaster = await rasterize(rightSvg, baseScale);
        const leftInitialMask = inkMask(rgba(leftRaster.canvas), threshold);
        const rightInitialMask = inkMask(rgba(rightInitialRaster.canvas), threshold);
        const leftInitialBox = maskBBox(leftInitialMask, leftRaster.width, leftRaster.height);
        const rightInitialBox = maskBBox(rightInitialMask, rightInitialRaster.width, rightInitialRaster.height);

        const measuredWidthScale =
          rightInitialBox.width > 0 ? leftInitialBox.width / rightInitialBox.width : 1;
        const measuredHeightScale =
          rightInitialBox.height > 0 ? leftInitialBox.height / rightInitialBox.height : 1;
        const rightScale = rightScaleMultiplier == null
          ? baseScale * (measuredWidthScale + measuredHeightScale) * 0.5
          : baseScale * rightScaleMultiplier;
        const rightRaster = await rasterize(rightSvg, rightScale);

        const leftMask = inkMask(rgba(leftRaster.canvas), threshold);
        const rightMask = inkMask(rgba(rightRaster.canvas), threshold);
        const leftBox = maskBBox(leftMask, leftRaster.width, leftRaster.height);
        const rightBox = maskBBox(rightMask, rightRaster.width, rightRaster.height);

        const pad = 24;
        const panelWidth = Math.max(leftBox.width, rightBox.width) + pad * 2;
        const panelHeight = Math.max(leftBox.height, rightBox.height) + pad * 2;
        const leftPanel = panelFromRaster(leftRaster, leftBox, panelWidth, panelHeight, pad);
        const rightPanel = panelFromRaster(rightRaster, rightBox, panelWidth, panelHeight, pad);
        const leftPanelMask = inkMask(rgba(leftPanel), threshold);
        const rightPanelMask = inkMask(rgba(rightPanel), threshold);
        const shift = bestShift(leftPanelMask, rightPanelMask, panelWidth, panelHeight, searchLimit);
        const alignedRightMask = shiftedMask(
          rightPanelMask,
          panelWidth,
          panelHeight,
          shift.dx,
          shift.dy,
        );
        const detailDistancePixels = {
          leftToRight: directedInkDistances(
            leftPanelMask,
            alignedRightMask,
            panelWidth,
            panelHeight,
          ),
          rightToLeft: directedInkDistances(
            alignedRightMask,
            leftPanelMask,
            panelWidth,
            panelHeight,
          ),
        };
        const rightAlignedPanel = shiftedCanvas(rightPanel, shift.dx, shift.dy);

        const overlay = makeOverlay(leftPanel, rightPanel, shift, threshold);
        const leftLabeled = labelPanel(leftPanel, leftLabel);
        const rightLabeled = labelPanel(
          rightAlignedPanel,
          `${rightLabel} (${
            rightScaleMultiplier == null ? "scaled" : "fixed-scale"
          }/aligned)`,
        );
        const overlayLabeled = labelPanel(
          overlay.overlayCanvas,
          "Overlay (black=overlap, blue=left-only, red=right-only)",
        );
        const diffLabeled = labelPanel(overlay.diffCanvas, "Difference Mask");

        const unionPixels = overlay.overlapPixels + overlay.leftOnlyPixels + overlay.rightOnlyPixels;
        const footerLines = [
          `left=${leftLabel}  right=${rightLabel}`,
          `baseScale=${baseScale.toFixed(2)}  rightScale=${rightScale.toFixed(4)}  scaleMode=${rightScaleMultiplier == null ? "content-normalized" : "fixed-document-unit"}`,
          `measured widthScale=${measuredWidthScale.toFixed(4)}  heightScale=${measuredHeightScale.toFixed(4)}`,
          `bbox left=${leftBox.width}x${leftBox.height}  right=${rightBox.width}x${rightBox.height}  panel=${panelWidth}x${panelHeight}`,
          `bestShift dx=${shift.dx}  dy=${shift.dy}  IoU=${shift.iou.toFixed(6)}  overlap=${overlay.overlapPixels}  union=${unionPixels}`,
          `pixels overlap=${overlay.overlapPixels}  leftOnly=${overlay.leftOnlyPixels}  rightOnly=${overlay.rightOnlyPixels}  different=${overlay.differentPixels}`,
          `detail p99=${Math.max(detailDistancePixels.leftToRight.p99, detailDistancePixels.rightToLeft.p99).toFixed(3)}px  p999=${Math.max(detailDistancePixels.leftToRight.p999, detailDistancePixels.rightToLeft.p999).toFixed(3)}px  max=${Math.max(detailDistancePixels.leftToRight.maximum, detailDistancePixels.rightToLeft.maximum).toFixed(3)}px`,
        ];
        const montageCanvas = montage(
          [leftLabeled, rightLabeled, overlayLabeled, diffLabeled],
          footerLines,
        );

        return {
          summary: {
            leftLabel,
            rightLabel,
            baseScale,
            rightScale,
            widthScale: measuredWidthScale,
            heightScale: measuredHeightScale,
            scaleMode: rightScaleMultiplier == null
              ? "content-normalized"
              : "fixed-document-unit",
            rightScaleMultiplier,
            threshold,
            searchLimit,
            leftBox,
            rightBox,
            panelWidth,
            panelHeight,
            bestShift: shift,
            overlapPixels: overlay.overlapPixels,
            leftOnlyPixels: overlay.leftOnlyPixels,
            rightOnlyPixels: overlay.rightOnlyPixels,
            differentPixels: overlay.differentPixels,
            unionPixels,
            detailDistancePixels,
          },
          pngs: {
            leftPanel: leftPanel.toDataURL("image/png").split(",")[1],
            rightPanel: rightAlignedPanel.toDataURL("image/png").split(",")[1],
            overlay: overlay.overlayCanvas.toDataURL("image/png").split(",")[1],
            diff: overlay.diffCanvas.toDataURL("image/png").split(",")[1],
            montage: montageCanvas.toDataURL("image/png").split(",")[1],
          },
        };
      },
      {
        leftSvg,
        rightSvg,
        leftLabel,
        rightLabel,
        baseScale,
        searchLimit,
        threshold,
        rightScaleMultiplier,
      },
    );

    const leftPanelPath = path.join(outDir, "left-panel.png");
    const rightPanelPath = path.join(outDir, "right-panel.png");
    const overlayPath = path.join(outDir, "overlay.png");
    const diffPath = path.join(outDir, "diff-mask.png");
    const montagePath = path.join(outDir, "montage.png");
    const summaryPath = path.join(outDir, "summary.json");

    await Promise.all([
      fs.writeFile(leftPanelPath, Buffer.from(result.pngs.leftPanel, "base64")),
      fs.writeFile(rightPanelPath, Buffer.from(result.pngs.rightPanel, "base64")),
      fs.writeFile(overlayPath, Buffer.from(result.pngs.overlay, "base64")),
      fs.writeFile(diffPath, Buffer.from(result.pngs.diff, "base64")),
      fs.writeFile(montagePath, Buffer.from(result.pngs.montage, "base64")),
      fs.writeFile(
        summaryPath,
        `${JSON.stringify(
          {
            ...result.summary,
            inputs: {
              leftPath,
              rightPath,
            },
            outputs: {
              leftPanelPath,
              rightPanelPath,
              overlayPath,
              diffPath,
              montagePath,
            },
          },
          null,
          2,
        )}\n`,
      ),
    ]);

    return {
      ...result.summary,
      inputs: {
        leftPath,
        rightPath,
      },
      outputs: {
        leftPanelPath,
        rightPanelPath,
        overlayPath,
        diffPath,
        montagePath,
        summaryPath,
      },
    };
  } finally {
    await browser.close();
  }
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (args.help || args.inputs.length !== 2) {
    console.log(
      "Usage: node scripts/compare-svg-pixels.mjs [--out dir] [--base-scale 4] [--search-limit 24] [--left-label ChemDraw] [--right-label ChemSema] <left.svg> <right.svg>",
    );
    if (args.help) return;
    process.exit(args.inputs.length === 2 ? 0 : 1);
  }

  const report = await compareSvgPixels({
    outDir: args.outDir,
    leftPath: args.inputs[0],
    rightPath: args.inputs[1],
    leftLabel: args.labelLeft,
    rightLabel: args.labelRight,
    baseScale: args.baseScale,
    searchLimit: args.searchLimit,
    threshold: args.threshold,
  });
  console.log(JSON.stringify(report, null, 2));
}

if (path.resolve(process.argv[1] ?? "") === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.stack ?? error.message : String(error));
    process.exit(1);
  });
}
