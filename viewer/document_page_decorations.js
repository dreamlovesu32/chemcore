export function pageTextAnnotations(documentData, layout, page, now = new Date()) {
  const dynamic = {
    f: documentData?.document?.title || "Untitled",
    p: String(page.pageNumber),
    d: new Intl.DateTimeFormat(undefined, { dateStyle: "short" }).format(now),
    t: new Intl.DateTimeFormat(undefined, { timeStyle: "short" }).format(now),
  };
  const annotations = [];
  for (const [role, source, baseline] of [
    ["header", layout.header, page.y + Number(layout.headerPosition || 0)],
    ["footer", layout.footer, page.y + page.height - Number(layout.footerPosition || 0)],
  ]) {
    if (!source) continue;
    const sections = headerFooterSections(source, dynamic);
    for (const [anchor, text, x] of [
      ["start", sections.left, page.x + 6],
      ["middle", sections.center, page.x + page.width / 2],
      ["end", sections.right, page.x + page.width - 6],
    ]) {
      if (text) {
        annotations.push({
          role,
          pageNumber: page.pageNumber,
          anchor,
          text,
          x,
          y: baseline,
        });
      }
    }
  }
  return annotations;
}

export function headerFooterSections(source, dynamic) {
  const sections = { left: "", center: "", right: "" };
  let target = "left";
  for (const chunk of String(source).split(/(&[lcr])/i)) {
    if (/^&[lcr]$/i.test(chunk)) {
      target = { l: "left", c: "center", r: "right" }[chunk[1].toLowerCase()];
    } else {
      sections[target] += chunk.replace(
        /&([fpdt])/gi,
        (_, token) => dynamic[token.toLowerCase()] || "",
      );
    }
  }
  return sections;
}

export function pageTrimMarkSegments(x, y, width, height) {
  const inset = 3;
  const length = 8;
  return [
    [x + inset, y, x + inset + length, y],
    [x, y + inset, x, y + inset + length],
    [x + width - inset - length, y, x + width - inset, y],
    [x + width, y + inset, x + width, y + inset + length],
    [x + inset, y + height, x + inset + length, y + height],
    [x, y + height - inset - length, x, y + height - inset],
    [x + width - inset - length, y + height, x + width - inset, y + height],
    [x + width, y + height - inset - length, x + width, y + height - inset],
  ];
}
