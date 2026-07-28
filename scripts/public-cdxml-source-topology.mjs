export function visibleInterchangeBondCount(root) {
  if (!root || typeof root !== "object") return 0;

  function visit(node, insideNode) {
    if (!node || typeof node !== "object") return 0;
    const name = String(node.name || "").toLowerCase();
    let count = name === "b" && !insideNode ? 1 : 0;
    const childInsideNode = insideNode || name === "n";
    for (const child of node.children || []) {
      count += visit(child, childInsideNode);
    }
    return count;
  }

  return visit(root, false);
}
