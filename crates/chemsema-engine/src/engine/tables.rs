use super::*;
use serde_json::json;
use std::collections::BTreeMap;

impl Engine {
    fn table_hit_at_point(&self, point: Point) -> Option<crate::HoverTableCell> {
        self.state
            .document
            .scene_objects()
            .into_iter()
            .rev()
            .filter(|object| object.visible && object.object_type == "table")
            .find_map(|object| {
                self.table_cell_at_point(&object.id, point)
                    .map(|(row, column, bounds)| crate::HoverTableCell {
                        object_id: object.id.clone(),
                        row,
                        column,
                        bounds,
                    })
            })
    }

    pub(super) fn table_cell_at_point(
        &self,
        object_id: &str,
        point: Point,
    ) -> Option<(usize, usize, [f64; 4])> {
        let object = self
            .state
            .document
            .find_scene_object(object_id)
            .filter(|object| object.object_type == "table")?;
        let table = object.payload.table.as_ref()?;
        let local_x = point.x - object.transform.translate[0];
        let local_y = point.y - object.transform.translate[1];
        let column = table
            .column_guides
            .windows(2)
            .position(|pair| local_x >= pair[0] && local_x <= pair[1])?;
        let row = table
            .row_guides
            .windows(2)
            .position(|pair| local_y >= pair[0] && local_y <= pair[1])?;
        Some((
            row,
            column,
            [
                object.transform.translate[0] + table.column_guides[column],
                object.transform.translate[1] + table.row_guides[row],
                object.transform.translate[0] + table.column_guides[column + 1],
                object.transform.translate[1] + table.row_guides[row + 1],
            ],
        ))
    }

    pub(super) fn pointer_down_table(&mut self, event: PointerEvent) {
        let point = event.point();
        if let Some(hit) = self.table_hit_at_point(point) {
            self.clear_interaction();
            self.state.overlay.hover_table_cell = Some(hit);
            return;
        }
        self.clear_interaction();
        self.state.selection = SelectionState::default();
        self.shape_drag = Some(ShapeDragState {
            pointer_start: point,
            start: point,
            current: point,
            anchor: ShapeDrawAnchor {
                kind: ShapeDrawAnchorKind::Free,
                point,
                bounds: None,
            },
            has_dragged: false,
        });
    }

    pub(super) fn pointer_move_table(&mut self, event: PointerEvent) {
        let point = event.point();
        self.state.overlay = OverlayState::default();
        let Some(mut drag) = self.shape_drag.take() else {
            self.state.overlay.hover_table_cell = self.table_hit_at_point(point);
            return;
        };
        drag.current = point;
        drag.has_dragged =
            drag.has_dragged || drag.pointer_start.distance(point) >= DRAG_START_THRESHOLD;
        if drag.has_dragged {
            self.state.overlay.preview = Some(BondPreview {
                start: drag.start,
                end: point,
            });
        }
        self.shape_drag = Some(drag);
    }

    pub(super) fn pointer_up_table(&mut self, event: PointerEvent) {
        let Some(mut drag) = self.shape_drag.take() else {
            return;
        };
        drag.current = event.point();
        drag.has_dragged = drag.pointer_start.distance(drag.current) >= DRAG_START_THRESHOLD;
        self.state.overlay = OverlayState::default();
        if !drag.has_dragged {
            return;
        }
        let begin = [drag.start.x, drag.start.y];
        let end = [drag.current.x, drag.current.y];
        self.pending_dialog = Some(json!({
            "kind": "insert-table",
            "title": "Insert Table",
            "bounds": { "begin": begin, "end": end },
            "fields": [
                {
                    "key": "rows",
                    "label": "Rows",
                    "value": 2,
                    "inputMode": "numeric",
                    "valueKind": "integer",
                    "minimum": 1,
                    "maximum": 100
                },
                {
                    "key": "columns",
                    "label": "Columns",
                    "value": 2,
                    "inputMode": "numeric",
                    "valueKind": "integer",
                    "minimum": 1,
                    "maximum": 100
                }
            ]
        }));
    }

    pub(super) fn table_preview_document(&self) -> Option<ChemSemaDocument> {
        if self.state.tool.active_tool != Tool::Table {
            return None;
        }
        let drag = self.shape_drag.as_ref().filter(|drag| drag.has_dragged)?;
        let mut document = self.preview_document_shell();
        document.objects.push(self.table_scene_object(
            drag.start,
            drag.current,
            1,
            1,
            "__preview_table".to_string(),
        )?);
        Some(document)
    }

    pub(super) fn insert_table(
        &mut self,
        begin: Point,
        end: Point,
        rows: usize,
        columns: usize,
    ) -> bool {
        if !(1..=100).contains(&rows) || !(1..=100).contains(&columns) {
            return false;
        }
        let object_id = self.next_id("obj_table");
        let Some(object) = self.table_scene_object(begin, end, rows, columns, object_id.clone())
        else {
            return false;
        };
        self.push_undo_snapshot();
        self.state.document.objects.push(object);
        self.note_pending_select_target(PendingSelectTarget::GraphicObject(object_id));
        true
    }

    pub(super) fn edit_table(
        &mut self,
        object_id: &str,
        row: usize,
        column: usize,
        action: &str,
    ) -> bool {
        let Some(original) = self
            .state
            .document
            .find_scene_object(object_id)
            .filter(|object| object.object_type == "table")
            .cloned()
        else {
            return false;
        };
        let Some(mut table) = original.payload.table.clone() else {
            return false;
        };
        if row >= table.rows || column >= table.columns {
            return false;
        }
        let mut removed_content_ids = Vec::new();
        let align_after = action.starts_with("align-");
        let content_bounds = (action == "size-to-fit-contents")
            .then(|| self.table_cell_content_bounds(&table, row, column))
            .flatten();
        let changed = match action {
            "add-row-before" => insert_table_row(&mut table, object_id, row),
            "add-row-after" => insert_table_row(&mut table, object_id, row + 1),
            "add-column-before" => insert_table_column(&mut table, object_id, column),
            "add-column-after" => insert_table_column(&mut table, object_id, column + 1),
            "delete-row" if table.rows > 1 => {
                removed_content_ids.extend(
                    table
                        .cells
                        .iter()
                        .filter(|cell| cell.row == row)
                        .flat_map(|cell| cell.content_object_ids.iter().cloned()),
                );
                delete_table_row(&mut table, row)
            }
            "delete-column" if table.columns > 1 => {
                removed_content_ids.extend(
                    table
                        .cells
                        .iter()
                        .filter(|cell| cell.column == column)
                        .flat_map(|cell| cell.content_object_ids.iter().cloned()),
                );
                delete_table_column(&mut table, column)
            }
            "clear-contents" => {
                let Some(cell) = table
                    .cells
                    .iter_mut()
                    .find(|cell| cell.row == row && cell.column == column)
                else {
                    return false;
                };
                removed_content_ids.append(&mut cell.content_object_ids);
                !removed_content_ids.is_empty()
            }
            "size-to-fit-contents" => {
                let Some(bounds) = content_bounds else {
                    return false;
                };
                let desired_width = (bounds[2] - bounds[0] + 8.0).max(12.0);
                let desired_height = (bounds[3] - bounds[1] + 8.0).max(12.0);
                let current_width = table.column_guides[column + 1] - table.column_guides[column];
                let current_height = table.row_guides[row + 1] - table.row_guides[row];
                let delta_x = desired_width - current_width;
                let delta_y = desired_height - current_height;
                for guide in table.column_guides.iter_mut().skip(column + 1) {
                    *guide = crate::round2(*guide + delta_x);
                }
                for guide in table.row_guides.iter_mut().skip(row + 1) {
                    *guide = crate::round2(*guide + delta_y);
                }
                delta_x.abs() > crate::EPSILON || delta_y.abs() > crate::EPSILON
            }
            "align-left" | "align-center" | "align-right" => {
                let alignment = match action {
                    "align-center" => crate::TableHorizontalAlignment::Center,
                    "align-right" => crate::TableHorizontalAlignment::Right,
                    _ => crate::TableHorizontalAlignment::Left,
                };
                set_cell_horizontal_alignment(&mut table, row, column, alignment)
            }
            "align-top" | "align-middle" | "align-bottom" => {
                let alignment = match action {
                    "align-top" => crate::TableVerticalAlignment::Top,
                    "align-bottom" => crate::TableVerticalAlignment::Bottom,
                    _ => crate::TableVerticalAlignment::Middle,
                };
                set_cell_vertical_alignment(&mut table, row, column, alignment)
            }
            "borders-none" | "borders-box" | "borders-all" => {
                let visible = action != "borders-none";
                let border = crate::TableBorder {
                    visible,
                    ..table.default_border.clone()
                };
                let Some(cell) = table
                    .cells
                    .iter_mut()
                    .find(|cell| cell.row == row && cell.column == column)
                else {
                    return false;
                };
                let next = Some(border);
                let changed = cell.borders.top != next
                    || cell.borders.left != next
                    || cell.borders.bottom != next
                    || cell.borders.right != next;
                cell.borders.top = next.clone();
                cell.borders.left = next.clone();
                cell.borders.bottom = next.clone();
                cell.borders.right = next;
                changed
            }
            "border-solid" | "border-dashed" => {
                let next = if action == "border-dashed" {
                    crate::TableLineStyle::Dashed
                } else {
                    crate::TableLineStyle::Solid
                };
                let Some(cell) = table
                    .cells
                    .iter_mut()
                    .find(|cell| cell.row == row && cell.column == column)
                else {
                    return false;
                };
                let mut changed = false;
                for border in [
                    &mut cell.borders.top,
                    &mut cell.borders.left,
                    &mut cell.borders.bottom,
                    &mut cell.borders.right,
                ] {
                    let value = border.get_or_insert_with(|| table.default_border.clone());
                    changed |= value.line_style != next;
                    value.line_style = next;
                }
                changed
            }
            _ => false,
        };
        if !changed {
            return false;
        }
        self.push_undo_snapshot();
        if !removed_content_ids.is_empty() {
            self.state
                .document
                .objects
                .retain(|object| !removed_content_ids.contains(&object.id));
        }
        let Some(object) = self.state.document.find_scene_object_mut(object_id) else {
            return false;
        };
        let width = *table.column_guides.last().unwrap_or(&0.0);
        let height = *table.row_guides.last().unwrap_or(&0.0);
        object.payload.bbox = Some([0.0, 0.0, width, height]);
        object.payload.table = Some(table);
        if align_after {
            self.align_table_cell_contents(object_id, row, column);
        }
        true
    }

    fn table_cell_content_bounds(
        &self,
        table: &crate::TableData,
        row: usize,
        column: usize,
    ) -> Option<[f64; 4]> {
        let cell = table
            .cells
            .iter()
            .find(|cell| cell.row == row && cell.column == column)?;
        cell.content_object_ids
            .iter()
            .filter_map(|id| self.state.document.find_scene_object(id))
            .filter_map(|object| {
                select::object_selection_bounds_for_render(&self.state.document, object)
            })
            .reduce(|left, right| {
                [
                    left[0].min(right[0]),
                    left[1].min(right[1]),
                    left[2].max(right[2]),
                    left[3].max(right[3]),
                ]
            })
    }

    fn align_table_cell_contents(&mut self, object_id: &str, row: usize, column: usize) {
        let Some(table_object) = self.state.document.find_scene_object(object_id).cloned() else {
            return;
        };
        let Some(table) = table_object.payload.table.as_ref() else {
            return;
        };
        let Some(cell) = table
            .cells
            .iter()
            .find(|cell| cell.row == row && cell.column == column)
            .cloned()
        else {
            return;
        };
        let Some(bounds) = self.table_cell_content_bounds(table, row, column) else {
            return;
        };
        let cell_bounds = [
            table_object.transform.translate[0] + table.column_guides[column],
            table_object.transform.translate[1] + table.row_guides[row],
            table_object.transform.translate[0] + table.column_guides[column + 1],
            table_object.transform.translate[1] + table.row_guides[row + 1],
        ];
        let content_width = bounds[2] - bounds[0];
        let content_height = bounds[3] - bounds[1];
        let target_left = match cell.horizontal_alignment {
            crate::TableHorizontalAlignment::Left => cell_bounds[0] + 4.0,
            crate::TableHorizontalAlignment::Center => {
                (cell_bounds[0] + cell_bounds[2] - content_width) * 0.5
            }
            crate::TableHorizontalAlignment::Right => cell_bounds[2] - content_width - 4.0,
        };
        let target_top = match cell.vertical_alignment {
            crate::TableVerticalAlignment::Top => cell_bounds[1] + 4.0,
            crate::TableVerticalAlignment::Middle => {
                (cell_bounds[1] + cell_bounds[3] - content_height) * 0.5
            }
            crate::TableVerticalAlignment::Bottom => cell_bounds[3] - content_height - 4.0,
        };
        let delta_x = target_left - bounds[0];
        let delta_y = target_top - bounds[1];
        for id in cell.content_object_ids {
            let Some(original) = self.state.document.find_scene_object(&id).cloned() else {
                continue;
            };
            let next = select::translated_scene_object(&original, delta_x, delta_y);
            if let Some(object) = self.state.document.find_scene_object_mut(&id) {
                *object = next;
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn set_table_borders(
        &mut self,
        object_id: &str,
        row: usize,
        column: usize,
        sides: &[String],
        line_style: crate::TableLineStyle,
        width: f64,
        color: &str,
    ) -> bool {
        if !width.is_finite() || width < 0.0 || !is_hex_color(color) {
            return false;
        }
        let Some(original) = self
            .state
            .document
            .find_scene_object(object_id)
            .filter(|object| object.object_type == "table")
            .cloned()
        else {
            return false;
        };
        let Some(mut table) = original.payload.table.clone() else {
            return false;
        };
        let default = crate::TableBorder {
            visible: width > crate::EPSILON,
            line_style,
            width,
            color: color.to_ascii_lowercase(),
        };
        let hidden = crate::TableBorder {
            visible: false,
            ..default.clone()
        };
        let Some(cell) = table
            .cells
            .iter_mut()
            .find(|cell| cell.row == row && cell.column == column)
        else {
            return false;
        };
        let selected = |side: &str| sides.iter().any(|value| value == side);
        let next = crate::TableCellBorders {
            top: Some(if selected("top") {
                default.clone()
            } else {
                hidden.clone()
            }),
            left: Some(if selected("left") {
                default.clone()
            } else {
                hidden.clone()
            }),
            bottom: Some(if selected("bottom") {
                default.clone()
            } else {
                hidden.clone()
            }),
            right: Some(if selected("right") { default } else { hidden }),
        };
        if cell.borders == next {
            return false;
        }
        self.push_undo_snapshot();
        let Some(object) = self.state.document.find_scene_object_mut(object_id) else {
            return false;
        };
        let Some(table) = object.payload.table.as_mut() else {
            return false;
        };
        let Some(cell) = table
            .cells
            .iter_mut()
            .find(|cell| cell.row == row && cell.column == column)
        else {
            return false;
        };
        cell.borders = next;
        true
    }

    fn table_scene_object(
        &self,
        begin: Point,
        end: Point,
        rows: usize,
        columns: usize,
        object_id: String,
    ) -> Option<SceneObject> {
        let left = begin.x.min(end.x);
        let top = begin.y.min(end.y);
        let width = (end.x - begin.x).abs();
        let height = (end.y - begin.y).abs();
        if width <= crate::EPSILON || height <= crate::EPSILON {
            return None;
        }
        let row_guides = (0..=rows)
            .map(|index| crate::round2(height * index as f64 / rows as f64))
            .collect();
        let column_guides = (0..=columns)
            .map(|index| crate::round2(width * index as f64 / columns as f64))
            .collect();
        let cells = (0..rows)
            .flat_map(|row| {
                (0..columns).map({
                    let object_id = object_id.clone();
                    move |column| crate::TableCell {
                        id: format!("{object_id}_cell_{row}_{column}"),
                        row,
                        column,
                        content_object_ids: Vec::new(),
                        borders: Default::default(),
                        horizontal_alignment: Default::default(),
                        vertical_alignment: Default::default(),
                    }
                })
            })
            .collect();
        Some(SceneObject {
            id: object_id,
            object_type: "table".to_string(),
            name: format!("{rows} by {columns} table"),
            visible: true,
            locked: false,
            z_index: self.next_shape_z_index(),
            transform: crate::Transform {
                translate: [crate::round2(left), crate::round2(top)],
                rotate: 0.0,
                scale: [1.0, 1.0],
            },
            style_ref: None,
            link_policy: Default::default(),
            meta: json!({"source": "authored"}),
            payload: crate::ObjectPayload {
                resource_ref: None,
                bbox: Some([0.0, 0.0, crate::round2(width), crate::round2(height)]),
                spectrum: None,
                geometry: None,
                constraint: None,
                table: Some(crate::TableData {
                    rows,
                    columns,
                    row_guides,
                    column_guides,
                    cells,
                    default_border: crate::TableBorder {
                        width: self.options.graphic_stroke_width,
                        ..Default::default()
                    },
                }),
                stoichiometry_grid: None,
                extra: BTreeMap::new(),
            },
            children: Vec::new(),
        })
    }
}

fn insert_table_row(table: &mut crate::TableData, object_id: &str, insertion: usize) -> bool {
    if insertion > table.rows {
        return false;
    }
    let source = insertion.min(table.rows.saturating_sub(1));
    let height = table.row_guides[source + 1] - table.row_guides[source];
    let insertion_position = table.row_guides[insertion];
    for guide in table.row_guides.iter_mut().skip(insertion) {
        *guide = crate::round2(*guide + height);
    }
    table.row_guides.insert(insertion, insertion_position);
    for cell in &mut table.cells {
        if cell.row >= insertion {
            cell.row += 1;
        }
    }
    for column in 0..table.columns {
        table.cells.push(crate::TableCell {
            id: format!(
                "{object_id}_cell_{insertion}_{column}_{}",
                table.cells.len()
            ),
            row: insertion,
            column,
            content_object_ids: Vec::new(),
            borders: Default::default(),
            horizontal_alignment: Default::default(),
            vertical_alignment: Default::default(),
        });
    }
    table.rows += 1;
    table.cells.sort_by_key(|cell| (cell.row, cell.column));
    true
}

fn insert_table_column(table: &mut crate::TableData, object_id: &str, insertion: usize) -> bool {
    if insertion > table.columns {
        return false;
    }
    let source = insertion.min(table.columns.saturating_sub(1));
    let width = table.column_guides[source + 1] - table.column_guides[source];
    let insertion_position = table.column_guides[insertion];
    for guide in table.column_guides.iter_mut().skip(insertion) {
        *guide = crate::round2(*guide + width);
    }
    table.column_guides.insert(insertion, insertion_position);
    for cell in &mut table.cells {
        if cell.column >= insertion {
            cell.column += 1;
        }
    }
    for row in 0..table.rows {
        table.cells.push(crate::TableCell {
            id: format!("{object_id}_cell_{row}_{insertion}_{}", table.cells.len()),
            row,
            column: insertion,
            content_object_ids: Vec::new(),
            borders: Default::default(),
            horizontal_alignment: Default::default(),
            vertical_alignment: Default::default(),
        });
    }
    table.columns += 1;
    table.cells.sort_by_key(|cell| (cell.row, cell.column));
    true
}

fn delete_table_row(table: &mut crate::TableData, row: usize) -> bool {
    let height = table.row_guides[row + 1] - table.row_guides[row];
    table.row_guides.remove(row + 1);
    for guide in table.row_guides.iter_mut().skip(row + 1) {
        *guide = crate::round2(*guide - height);
    }
    table.cells.retain(|cell| cell.row != row);
    for cell in &mut table.cells {
        if cell.row > row {
            cell.row -= 1;
        }
    }
    table.rows -= 1;
    true
}

fn delete_table_column(table: &mut crate::TableData, column: usize) -> bool {
    let width = table.column_guides[column + 1] - table.column_guides[column];
    table.column_guides.remove(column + 1);
    for guide in table.column_guides.iter_mut().skip(column + 1) {
        *guide = crate::round2(*guide - width);
    }
    table.cells.retain(|cell| cell.column != column);
    for cell in &mut table.cells {
        if cell.column > column {
            cell.column -= 1;
        }
    }
    table.columns -= 1;
    true
}

fn set_cell_horizontal_alignment(
    table: &mut crate::TableData,
    row: usize,
    column: usize,
    alignment: crate::TableHorizontalAlignment,
) -> bool {
    let Some(cell) = table
        .cells
        .iter_mut()
        .find(|cell| cell.row == row && cell.column == column)
    else {
        return false;
    };
    let changed = cell.horizontal_alignment != alignment;
    cell.horizontal_alignment = alignment;
    changed
}

fn set_cell_vertical_alignment(
    table: &mut crate::TableData,
    row: usize,
    column: usize,
    alignment: crate::TableVerticalAlignment,
) -> bool {
    let Some(cell) = table
        .cells
        .iter_mut()
        .find(|cell| cell.row == row && cell.column == column)
    else {
        return false;
    };
    let changed = cell.vertical_alignment != alignment;
    cell.vertical_alignment = alignment;
    changed
}

fn is_hex_color(value: &str) -> bool {
    value.len() == 7
        && value.starts_with('#')
        && value[1..]
            .chars()
            .all(|character| character.is_ascii_hexdigit())
}
