function wildcardPatternToRegExp(pattern) {
  const escaped = pattern.replace(/[.+^${}()|[\]\\]/g, "\\$&").replaceAll("**", "\u0000").replaceAll("*", "[^/]*").replaceAll("\u0000", ".*");
  return new RegExp(`^${escaped}$`);
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
    .filter((node) => node.kind === "source" && (node.patterns || []).some((pattern) => {
      const regex = wildcardPatternToRegExp(pattern.replaceAll("\\", "/"));
      return changedPaths.some((path) => regex.test(path.replaceAll("\\", "/")));
    }))
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
