from __future__ import annotations

import argparse
import json
import math
from collections import Counter, defaultdict, deque
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable


@dataclass
class BBox:
    left: float
    top: float
    right: float
    bottom: float

    @property
    def width(self) -> float:
        return self.right - self.left

    @property
    def height(self) -> float:
        return self.bottom - self.top

    @property
    def center(self) -> list[float]:
        return [(self.left + self.right) / 2.0, (self.top + self.bottom) / 2.0]

    def expand_to_include(self, other: "BBox") -> "BBox":
        return BBox(
            left=min(self.left, other.left),
            top=min(self.top, other.top),
            right=max(self.right, other.right),
            bottom=max(self.bottom, other.bottom),
        )

    def as_list(self) -> list[float]:
        return [self.left, self.top, self.right, self.bottom]


def load_document(path: Path) -> dict:
    payload = json.loads(path.read_text(encoding="utf8"))
    if "chemcoreDocumentJson" in payload:
        return json.loads(payload["chemcoreDocumentJson"])
    return payload


def bbox_from_points(points: Iterable[Iterable[float]]) -> BBox | None:
    pts = [tuple(map(float, p)) for p in points]
    if not pts:
        return None
    xs = [p[0] for p in pts]
    ys = [p[1] for p in pts]
    return BBox(min(xs), min(ys), max(xs), max(ys))


def bbox_from_list(values: list[float] | None) -> BBox | None:
    if not values or len(values) != 4:
        return None
    return BBox(float(values[0]), float(values[1]), float(values[2]), float(values[3]))


def transform_point(point: Iterable[float], transform: dict) -> list[float]:
    x, y = map(float, point)
    translate = transform.get("translate", [0.0, 0.0])
    scale = transform.get("scale", [1.0, 1.0])
    rotate_deg = float(transform.get("rotate", 0.0))
    x *= float(scale[0])
    y *= float(scale[1])
    if rotate_deg:
        theta = math.radians(rotate_deg)
        cos_t = math.cos(theta)
        sin_t = math.sin(theta)
        x, y = (x * cos_t - y * sin_t, x * sin_t + y * cos_t)
    x += float(translate[0])
    y += float(translate[1])
    return [x, y]


def transform_bbox(box: BBox, transform: dict) -> BBox:
    corners = [
        transform_point((box.left, box.top), transform),
        transform_point((box.right, box.top), transform),
        transform_point((box.right, box.bottom), transform),
        transform_point((box.left, box.bottom), transform),
    ]
    return bbox_from_points(corners)  # type: ignore[return-value]


def union_boxes(boxes: Iterable[BBox]) -> BBox | None:
    iterator = iter(boxes)
    try:
        first = next(iterator)
    except StopIteration:
        return None
    acc = first
    for box in iterator:
        acc = acc.expand_to_include(box)
    return acc


def slot_name(index: int) -> str:
    return f"component_{index + 1:02d}"


def component_bbox_for_nodes(component_nodes: list[dict]) -> BBox:
    return bbox_from_points(node["position"] for node in component_nodes)  # type: ignore[return-value]


def label_boxes_for_nodes(component_nodes: list[dict]) -> list[BBox]:
    boxes: list[BBox] = []
    for node in component_nodes:
        label = node.get("label")
        if not label:
            continue
        label_box = bbox_from_list(label.get("box"))
        glyph_box = None
        glyph_polygons = label.get("glyphPolygons") or []
        glyph_points: list[list[float]] = []
        for polygon in glyph_polygons:
            glyph_points.extend(polygon)
        if glyph_points:
            glyph_box = bbox_from_points(glyph_points)
        for candidate in [label_box, glyph_box]:
            if candidate:
                boxes.append(candidate)
    return boxes


def classify_component(index: int, component_nodes: list[dict], center_world: list[float]) -> dict:
    element_counts = Counter(node.get("element", "?") for node in component_nodes)
    label_texts = [
        node["label"]["text"]
        for node in component_nodes
        if node.get("label") and node["label"].get("text")
    ]
    label_fills = Counter(
        node["label"].get("fill", "#000000")
        for node in component_nodes
        if node.get("label")
    )
    label_fill_keys = set(label_fills.keys())
    label_text_counter = Counter(label_texts)
    role_guess = slot_name(index)
    if any(fill != "#000000" for fill in label_fill_keys):
        role_guess = "top_right_product_component"
    elif label_text_counter.get("Ph", 0) >= 4 and (
        "CN" in label_text_counter or "NC" in label_text_counter
    ):
        role_guess = "bottom_right_catalyst_component"
    elif "S" in label_text_counter:
        role_guess = "bottom_center_reagent_component"
    elif label_texts and set(label_texts).issubset({"O", "N"}):
        role_guess = "bottom_left_ligand_component"
    elif not label_texts and center_world[0] < 600:
        role_guess = "top_left_substrate_component"
    return {
        "slot": slot_name(index),
        "roleGuess": role_guess,
        "element_counts": dict(element_counts),
        "label_texts": label_texts,
        "label_fills": dict(label_fills),
    }


def molecule_component_report(molecule_obj: dict, resource: dict) -> dict:
    data = resource["data"]
    nodes = data["nodes"]
    bonds = data["bonds"]
    node_map = {node["id"]: node for node in nodes}
    adjacency: dict[str, set[str]] = defaultdict(set)
    bond_ids_by_pair: dict[tuple[str, str], list[str]] = defaultdict(list)
    for bond in bonds:
        a = bond["begin"]
        b = bond["end"]
        adjacency[a].add(b)
        adjacency[b].add(a)
        bond_ids_by_pair[tuple(sorted((a, b)))].append(bond["id"])

    seen: set[str] = set()
    components: list[list[str]] = []
    for node_id in node_map:
        if node_id in seen:
            continue
        queue = deque([node_id])
        seen.add(node_id)
        ids: list[str] = []
        while queue:
            current = queue.popleft()
            ids.append(current)
            for neighbor in adjacency[current]:
                if neighbor not in seen:
                    seen.add(neighbor)
                    queue.append(neighbor)
        components.append(ids)

    component_rows: list[dict] = []
    for comp_ids in components:
        comp_nodes = [node_map[node_id] for node_id in comp_ids]
        comp_node_box = component_bbox_for_nodes(comp_nodes)
        label_boxes = label_boxes_for_nodes(comp_nodes)
        comp_label_box = union_boxes(label_boxes)
        comp_world_boxes = [comp_node_box]
        if comp_label_box:
            comp_world_boxes.append(comp_label_box)
        comp_local_union = union_boxes(comp_world_boxes) or comp_node_box
        world_union = transform_bbox(comp_local_union, molecule_obj["transform"])
        component_bond_ids: set[str] = set()
        for node_id in comp_ids:
            for neighbor in adjacency[node_id]:
                for bond_id in bond_ids_by_pair[tuple(sorted((node_id, neighbor)))]:
                    component_bond_ids.add(bond_id)
        classification = classify_component(len(component_rows), comp_nodes, world_union.center)
        component_rows.append(
            {
                "slot": classification["slot"],
                "roleGuess": classification["roleGuess"],
                "nodeCount": len(comp_ids),
                "bondCount": len(component_bond_ids),
                "nodeBoxLocal": comp_node_box.as_list(),
                "labelBoxLocal": comp_label_box.as_list() if comp_label_box else None,
                "unionBoxLocal": comp_local_union.as_list(),
                "worldBox": world_union.as_list(),
                "centerWorld": world_union.center,
                "elementCounts": classification["element_counts"],
                "labelTexts": classification["label_texts"],
                "labelFills": classification["label_fills"],
            }
        )

    component_rows.sort(key=lambda row: (row["centerWorld"][1], row["centerWorld"][0]))
    for idx, row in enumerate(component_rows):
        row["slot"] = slot_name(idx)
    component_union = union_boxes(
        bbox_from_list(row["worldBox"]) for row in component_rows if row.get("worldBox")
    )
    molecule_top_box = transform_bbox(
        bbox_from_list(molecule_obj["payload"]["bbox"]), molecule_obj["transform"]
    )
    return {
        "resourceRef": molecule_obj["payload"]["resourceRef"],
        "componentCount": len(component_rows),
        "components": component_rows,
        "edgeContributors": edge_contributors(component_rows),
        "componentUnionWorldBox": component_union.as_list() if component_union else None,
        "topLevelMoleculeWorldBox": molecule_top_box.as_list() if molecule_top_box else None,
        "componentUnionMinusTopLevel": (
            {
                "left": component_union.left - molecule_top_box.left,
                "top": component_union.top - molecule_top_box.top,
                "right": component_union.right - molecule_top_box.right,
                "bottom": component_union.bottom - molecule_top_box.bottom,
            }
            if component_union and molecule_top_box
            else None
        ),
    }


def top_level_object_report(obj: dict) -> dict:
    transform = obj.get("transform", {"translate": [0.0, 0.0], "scale": [1.0, 1.0], "rotate": 0.0})
    payload = obj.get("payload", {})
    box = None
    if obj.get("type") == "text":
        box = bbox_from_list(payload.get("box"))
    elif obj.get("type") == "line":
        box = bbox_from_list(payload.get("arrowGeometry", {}).get("boundingBox"))
    elif obj.get("type") == "molecule":
        box = bbox_from_list(payload.get("bbox"))
    if not box:
        return {
            "id": obj.get("id"),
            "type": obj.get("type"),
            "worldBox": None,
        }
    world_box = transform_bbox(box, transform)
    report = {
        "id": obj.get("id"),
        "type": obj.get("type"),
        "worldBox": world_box.as_list(),
        "centerWorld": world_box.center,
        "widthWorld": world_box.width,
        "heightWorld": world_box.height,
    }
    if obj.get("type") == "text":
        payload_text = payload.get("text", "")
        report.update(
            {
                "align": payload.get("align"),
                "lines": payload_text.count("\n") + 1,
                "scripts": sorted({run.get("script", "normal") for run in payload.get("runs", [])}),
                "textPreview": payload_text.replace("\n", " ⏎ "),
            }
        )
    return report


def edge_contributors(boxed_rows: list[dict]) -> dict:
    usable = [row for row in boxed_rows if row.get("worldBox")]
    if not usable:
        return {}

    def metric(row: dict, which: str) -> float:
        box = row["worldBox"]
        if which == "left":
            return box[0]
        if which == "top":
            return box[1]
        if which == "right":
            return box[2]
        if which == "bottom":
            return box[3]
        raise ValueError(which)

    return {
        "leftmost": min(usable, key=lambda row: metric(row, "left")),
        "topmost": min(usable, key=lambda row: metric(row, "top")),
        "rightmost": max(usable, key=lambda row: metric(row, "right")),
        "bottommost": max(usable, key=lambda row: metric(row, "bottom")),
    }


def add_document_edge_distances(rows: list[dict], document_box: BBox | None) -> None:
    if not document_box:
        return
    for row in rows:
        box = row.get("worldBox")
        if not box:
            continue
        row["distanceToDocumentEdges"] = {
            "left": float(box[0]) - document_box.left,
            "top": float(box[1]) - document_box.top,
            "right": document_box.right - float(box[2]),
            "bottom": document_box.bottom - float(box[3]),
        }


def main() -> None:
    parser = argparse.ArgumentParser(description="Dump geometry features from a chemcore payload/doc JSON.")
    parser.add_argument("input_json")
    parser.add_argument("--output")
    args = parser.parse_args()

    doc = load_document(Path(args.input_json))
    top_rows = [top_level_object_report(obj) for obj in doc.get("objects", [])]
    molecule_obj = next((obj for obj in doc.get("objects", []) if obj.get("type") == "molecule"), None)
    molecule_report = None
    if molecule_obj:
        resource_ref = molecule_obj["payload"]["resourceRef"]
        molecule_report = molecule_component_report(molecule_obj, doc["resources"][resource_ref])

    all_boxes: list[BBox] = []
    for row in top_rows:
        if row.get("worldBox"):
            all_boxes.append(bbox_from_list(row["worldBox"]))  # type: ignore[arg-type]
    document_world_box = union_boxes(box for box in all_boxes if box is not None)
    add_document_edge_distances(top_rows, document_world_box)
    if molecule_report:
        add_document_edge_distances(molecule_report["components"], document_world_box)

    report = {
        "input": args.input_json,
        "topLevelObjects": top_rows,
        "topLevelEdgeContributors": edge_contributors(top_rows),
        "documentWorldBox": document_world_box.as_list() if document_world_box else None,
        "moleculeComponents": molecule_report,
    }

    text = json.dumps(report, indent=2, ensure_ascii=False)
    if args.output:
        Path(args.output).write_text(text, encoding="utf8")
    else:
        print(text)


if __name__ == "__main__":
    main()
