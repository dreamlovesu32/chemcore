function wildcardPatternToRegExp(pattern) {
  const escaped = pattern.replace(/[.+^${}()|[\]\\]/g, "\\$&").replaceAll("**", "\u0000").replaceAll("*", "[^/]*").replaceAll("\u0000", ".*");
  return new RegExp(`^${escaped}$`);
}

function sourceMatchesPath(node, path) {
  const normalized = path.replaceAll("\\", "/");
  const patterns = node.patterns || [];
  const positive = patterns.filter((pattern) => !pattern.startsWith("!"));
  const negative = patterns.filter((pattern) => pattern.startsWith("!")).map((pattern) => pattern.slice(1));
  return positive.some((pattern) => wildcardPatternToRegExp(pattern.replaceAll("\\", "/")).test(normalized))
    && !negative.some((pattern) => wildcardPatternToRegExp(pattern.replaceAll("\\", "/")).test(normalized));
}

export function selectImpactedScenarios(graph, changedPaths) {
  const nodes = new Map(graph.nodes.map((node) => [node.id, node]));
  const outgoing = new Map();
  for (const edge of graph.edges) {
    if (!nodes.has(edge.from) || !nodes.has(edge.to)) {
      throw new Error(`Impact edge references an unknown node: ${edge.from} -> ${edge.to}`);
    }
    const next = outgoing.get(edge.from) || [];
    next.push(edge.to);
    outgoing.set(edge.from, next);
  }

  const queue = graph.nodes
    .filter((node) => node.kind === "source" && changedPaths.some((path) => sourceMatchesPath(node, path)))
    .map((node) => node.id);
  const visited = new Set(queue);
  while (queue.length) {
    const current = queue.shift();
    for (const next of outgoing.get(current) || []) {
      if (!visited.has(next)) {
        visited.add(next);
        queue.push(next);
      }
    }
  }
  return [...visited]
    .filter((id) => nodes.get(id)?.kind === "scenario")
    .sort();
}

export function planImpactedScenarios(graph, changedPaths) {
  const sourceNodes = graph.nodes.filter((node) => node.kind === "source");
  const matchedSources = new Set();
  const unmatchedPaths = [];
  for (const changedPath of changedPaths) {
    const normalized = changedPath.replaceAll("\\", "/");
    const matches = sourceNodes.filter((node) => sourceMatchesPath(node, normalized));
    if (!matches.length) {
      unmatchedPaths.push(normalized);
    }
    for (const match of matches) {
      matchedSources.add(match.id);
    }
  }
  const allScenarios = graph.nodes.filter((node) => node.kind === "scenario").map((node) => node.id).sort();
  return {
    changedPaths: changedPaths.map((path) => path.replaceAll("\\", "/")),
    matchedSources: [...matchedSources].sort(),
    unmatchedPaths,
    expandedForUncertainty: unmatchedPaths.length > 0,
    scenarios: unmatchedPaths.length > 0 ? allScenarios : selectImpactedScenarios(graph, changedPaths),
  };
}
