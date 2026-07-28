use crate::{
    round2, Point, DEFAULT_BOND_LENGTH_PT, DEFAULT_BOND_STROKE_PT,
    DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT, DEFAULT_PAGE_HEIGHT_PT, DEFAULT_PAGE_WIDTH_PT,
    DEFAULT_TEXT_BLOCK_PADDING_PT, DEFAULT_TEXT_FONT_SIZE_PT, DEFAULT_TEXT_LINE_HEIGHT_PT, EPSILON,
};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[path = "document/geometry_constraints.rs"]
mod geometry_constraints;
pub use geometry_constraints::{
    AnnotationDisplay, AnnotationPositioningType, ConstraintData, ConstraintType, GeometryData,
    GeometryFeature,
};

pub const DEFAULT_PAGE_WIDTH: f64 = DEFAULT_PAGE_WIDTH_PT;
pub const DEFAULT_PAGE_HEIGHT: f64 = DEFAULT_PAGE_HEIGHT_PT;
pub const DEFAULT_BOND_LENGTH: f64 = DEFAULT_BOND_LENGTH_PT;
pub const DEFAULT_BOND_STROKE: f64 = DEFAULT_BOND_STROKE_PT;

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChemSemaDocument {
    pub format: FormatInfo,
    pub document: DocumentInfo,
    #[serde(default)]
    pub style: DocumentStyleInfo,
    #[serde(default)]
    pub styles: BTreeMap<String, Value>,
    #[serde(default)]
    pub objects: Vec<SceneObject>,
    #[serde(default)]
    pub links: Vec<LinkRelation>,
    #[serde(default, skip_serializing_if = "crate::LogicalObjectData::is_empty")]
    pub logical_objects: crate::LogicalObjectData,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reaction_schemes: Vec<crate::ReactionSchemeData>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub chemical_properties: Vec<ChemicalProperty>,
    #[serde(default)]
    pub resources: BTreeMap<String, Resource>,
    /// Lossless, editable trees for interchange-format information that does
    /// not yet have a source-independent CCJS field.  This is deliberately a
    /// first-class field rather than import metadata: changing a value here is
    /// reflected by the corresponding exporter.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub interchange: BTreeMap<String, InterchangeDocument>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InterchangeDocument {
    pub format: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    pub root: InterchangeObject,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InterchangeObject {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format_tag: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub properties: BTreeMap<String, InterchangeProperty>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub text: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<InterchangeObject>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InterchangeProperty {
    /// Canonical property name. It is explicit because CDX permits repeated
    /// properties; their storage keys use `Name#2`, `Name#3`, ... while this
    /// field remains `Name`.
    pub name: String,
    /// Zero-based order within the containing interchange object. CDX allows
    /// repeated tags and some consumers attach meaning to their sequence.
    #[serde(default)]
    pub order: usize,
    /// CDXML lexical value.  It remains authoritative for exact round trips
    /// and for properties whose public CDX specification calls the type
    /// "Unformatted" or "varies".
    pub value: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cdx_tag: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cdx_type: Option<String>,
    /// Exact CDX bytes are retained whenever a property cannot be losslessly
    /// reconstructed from its public lexical representation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_base64: Option<String>,
}

impl ChemSemaDocument {
    pub fn blank() -> Self {
        let mut styles = BTreeMap::new();
        styles.insert(
            "style_molecule_default".to_string(),
            json!({
                "kind": "molecule",
                "stroke": "#000000",
                "strokeWidth": DEFAULT_BOND_STROKE,
                "fontFamily": "Arial",
                "fontSize": DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT
            }),
        );
        styles.insert(
            "style_arrow_default".to_string(),
            json!({
                "kind": "stroke",
                "stroke": "#000000",
                "strokeWidth": DEFAULT_BOND_STROKE,
                "lineCap": "butt",
                "lineJoin": "miter"
            }),
        );

        let mut resources = BTreeMap::new();
        resources.insert(
            "mol_editor".to_string(),
            Resource {
                resource_type: "molecule_fragment2d".to_string(),
                encoding: "chemsema.molecule.fragment2d".to_string(),
                data: ResourceData::Fragment(MoleculeFragment::blank()),
                meta: Value::Null,
            },
        );

        Self {
            format: FormatInfo {
                name: "chemsema".to_string(),
                version: "0.1".to_string(),
                unit: "pt".to_string(),
            },
            document: DocumentInfo {
                id: "doc_editor_untitled".to_string(),
                title: "Untitled".to_string(),
                page: Page {
                    width: DEFAULT_PAGE_WIDTH,
                    height: DEFAULT_PAGE_HEIGHT,
                    background: "#ffffff".to_string(),
                },
                layout: DocumentLayout::default(),
                meta: Value::Null,
            },
            style: DocumentStyleInfo::default(),
            styles,
            objects: vec![SceneObject {
                id: "obj_editor_molecule".to_string(),
                object_type: "molecule".to_string(),
                name: "molecule".to_string(),
                visible: true,
                locked: false,
                z_index: 10,
                transform: Transform::identity(),
                style_ref: Some("style_molecule_default".to_string()),
                link_policy: LinkPolicy::Auto,
                meta: Value::Null,
                payload: ObjectPayload {
                    resource_ref: Some("mol_editor".to_string()),
                    bbox: Some([0.0, 0.0, DEFAULT_PAGE_WIDTH, DEFAULT_PAGE_HEIGHT]),
                    spectrum: None,
                    geometry: None,
                    constraint: None,
                    table: None,
                    stoichiometry_grid: None,
                    gel_electrophoresis: None,
                    plasmid_map: None,
                    bio_shape: None,
                    extra: BTreeMap::new(),
                },
                children: Vec::new(),
            }],
            links: Vec::new(),
            logical_objects: Default::default(),
            reaction_schemes: Vec::new(),
            chemical_properties: Vec::new(),
            resources,
            interchange: BTreeMap::new(),
        }
    }

    /// Returns an interchange object by its child-index path.  An empty path
    /// addresses the format root; `[0, 2]` addresses the third child of the
    /// root's first child.  Index paths remain unambiguous even for CDXML
    /// elements that legitimately omit or duplicate ids.
    pub fn interchange_object(&self, format: &str, path: &[usize]) -> Option<&InterchangeObject> {
        let mut object = &self.interchange.get(format)?.root;
        for index in path {
            object = object.children.get(*index)?;
        }
        Some(object)
    }

    pub fn interchange_object_mut(
        &mut self,
        format: &str,
        path: &[usize],
    ) -> Option<&mut InterchangeObject> {
        let mut object = &mut self.interchange.get_mut(format)?.root;
        for index in path {
            object = object.children.get_mut(*index)?;
        }
        Some(object)
    }

    /// Edits a named CDX/CDXML property without routing it through `meta`.
    /// The property must already exist so misspellings cannot silently create
    /// a format-invalid field.  CDX exporters use the official type recorded
    /// beside the value; CDXML exporters use its lexical representation.
    pub fn set_interchange_property(
        &mut self,
        format: &str,
        path: &[usize],
        property: &str,
        value: impl Into<String>,
    ) -> Result<(), String> {
        let object = self
            .interchange_object_mut(format, path)
            .ok_or_else(|| format!("interchange object {format}:{path:?} does not exist"))?;
        let field = object.properties.get_mut(property).ok_or_else(|| {
            format!(
                "interchange property {property} does not exist on {format}:{path:?} ({})",
                object.name
            )
        })?;
        field.value = value.into();
        Ok(())
    }

    /// Replaces the exact CDX payload for public types whose lexical form is
    /// unspecified.  This is intentionally separate from value editing.
    pub fn set_interchange_property_raw_base64(
        &mut self,
        path: &[usize],
        property: &str,
        raw_base64: impl Into<String>,
    ) -> Result<(), String> {
        let object = self
            .interchange_object_mut("cdx", path)
            .ok_or_else(|| format!("interchange object cdx:{path:?} does not exist"))?;
        let field = object.properties.get_mut(property).ok_or_else(|| {
            format!(
                "interchange property {property} does not exist on cdx:{path:?} ({})",
                object.name
            )
        })?;
        field.raw_base64 = Some(raw_base64.into());
        Ok(())
    }

    pub fn editable_fragment_mut(&mut self) -> Option<EditableFragmentMut<'_>> {
        let object = first_molecule_object_mut(&mut self.objects)?;
        let resource_ref = object.payload.resource_ref.clone()?;
        let resource = self.resources.get_mut(&resource_ref)?;
        let fragment = resource.data.as_fragment_mut()?;
        Some(EditableFragmentMut { object, fragment })
    }

    pub fn editable_fragment_mut_for_object(
        &mut self,
        object_id: &str,
    ) -> Option<EditableFragmentMut<'_>> {
        let object = find_scene_object_mut(&mut self.objects, object_id)?;
        if object.object_type != "molecule" || !object.visible {
            return None;
        }
        let resource_ref = object.payload.resource_ref.clone()?;
        let resource = self.resources.get_mut(&resource_ref)?;
        let fragment = resource.data.as_fragment_mut()?;
        Some(EditableFragmentMut { object, fragment })
    }

    pub fn editable_fragment(&self) -> Option<EditableFragment<'_>> {
        let object = first_molecule_object(&self.objects)?;
        let resource_ref = object.payload.resource_ref.as_ref()?;
        let resource = self.resources.get(resource_ref)?;
        let fragment = resource.data.as_fragment()?;
        Some(EditableFragment { object, fragment })
    }

    pub fn editable_fragments(&self) -> Vec<EditableFragment<'_>> {
        let mut out = Vec::new();
        collect_editable_fragments(&self.objects, &self.resources, &mut out);
        out
    }

    pub fn scene_objects(&self) -> Vec<&SceneObject> {
        let mut out = Vec::new();
        collect_scene_objects(&self.objects, &mut out);
        out
    }

    pub fn find_scene_object(&self, object_id: &str) -> Option<&SceneObject> {
        find_scene_object(&self.objects, object_id)
    }

    pub fn find_scene_object_mut(&mut self, object_id: &str) -> Option<&mut SceneObject> {
        find_scene_object_mut(&mut self.objects, object_id)
    }

    pub fn ancestor_group_id_for_scene_object(&self, object_id: &str) -> Option<String> {
        find_ancestor_group_id(&self.objects, object_id, None)
    }

    pub fn remove_scene_objects_by_id(
        &mut self,
        object_ids: &std::collections::BTreeSet<&str>,
    ) -> usize {
        let removed = remove_scene_objects_by_id(&mut self.objects, object_ids);
        if removed > 0 {
            let referenced: BTreeSet<String> = self
                .scene_objects()
                .into_iter()
                .filter_map(|object| object.payload.resource_ref.clone())
                .collect();
            self.resources.retain(|id, _| referenced.contains(id));
        }
        removed
    }
}

fn collect_scene_objects<'a>(objects: &'a [SceneObject], out: &mut Vec<&'a SceneObject>) {
    for object in objects {
        out.push(object);
        collect_scene_objects(&object.children, out);
    }
}

fn collect_editable_fragments<'a>(
    objects: &'a [SceneObject],
    resources: &'a BTreeMap<String, Resource>,
    out: &mut Vec<EditableFragment<'a>>,
) {
    for object in objects {
        if object.object_type == "molecule" && object.visible {
            if let Some(resource_ref) = object.payload.resource_ref.as_ref() {
                if let Some(fragment) = resources
                    .get(resource_ref)
                    .and_then(|resource| resource.data.as_fragment())
                {
                    out.push(EditableFragment { object, fragment });
                }
            }
        }
        collect_editable_fragments(&object.children, resources, out);
    }
}

fn find_scene_object<'a>(objects: &'a [SceneObject], object_id: &str) -> Option<&'a SceneObject> {
    for object in objects {
        if object.id == object_id {
            return Some(object);
        }
        if let Some(found) = find_scene_object(&object.children, object_id) {
            return Some(found);
        }
    }
    None
}

fn find_scene_object_mut<'a>(
    objects: &'a mut [SceneObject],
    object_id: &str,
) -> Option<&'a mut SceneObject> {
    for object in objects {
        if object.id == object_id {
            return Some(object);
        }
        if let Some(found) = find_scene_object_mut(&mut object.children, object_id) {
            return Some(found);
        }
    }
    None
}

fn find_ancestor_group_id(
    objects: &[SceneObject],
    object_id: &str,
    ancestor_group_id: Option<&str>,
) -> Option<String> {
    for object in objects {
        if object.id == object_id {
            return ancestor_group_id.map(str::to_string);
        }
        let next_ancestor = if object.object_type == "group" {
            Some(object.id.as_str())
        } else {
            ancestor_group_id
        };
        if let Some(found) = find_ancestor_group_id(&object.children, object_id, next_ancestor) {
            return Some(found);
        }
    }
    None
}

fn first_molecule_object(objects: &[SceneObject]) -> Option<&SceneObject> {
    for object in objects {
        if object.object_type == "molecule" {
            return Some(object);
        }
        if let Some(found) = first_molecule_object(&object.children) {
            return Some(found);
        }
    }
    None
}

fn first_molecule_object_mut(objects: &mut [SceneObject]) -> Option<&mut SceneObject> {
    for object in objects {
        if object.object_type == "molecule" {
            return Some(object);
        }
        if let Some(found) = first_molecule_object_mut(&mut object.children) {
            return Some(found);
        }
    }
    None
}

fn remove_scene_objects_by_id(
    objects: &mut Vec<SceneObject>,
    object_ids: &std::collections::BTreeSet<&str>,
) -> usize {
    let before = objects.len();
    objects.retain(|object| !object_ids.contains(object.id.as_str()));
    let mut removed = before - objects.len();
    for object in objects {
        removed += remove_scene_objects_by_id(&mut object.children, object_ids);
    }
    removed
}

pub fn parse_document_json(json: &str) -> Result<ChemSemaDocument, String> {
    let mut value: Value = serde_json::from_str(json).map_err(|error| error.to_string())?;
    ensure_document_json_pt_unit(&mut value)?;
    migrate_legacy_external_connection_points(&mut value);
    let mut document: ChemSemaDocument =
        serde_json::from_value(value).map_err(|error| error.to_string())?;
    migrate_legacy_bracket_links(&mut document);
    document.document.layout.validate()?;
    validate_scene_object_types(&document.objects)?;
    validate_spectrum_objects(&document.objects)?;
    let scene_ids = document
        .scene_objects()
        .into_iter()
        .map(|object| object.id.clone())
        .collect::<BTreeSet<_>>();
    validate_table_objects(&document.objects, &scene_ids, &mut BTreeSet::new())?;
    validate_stoichiometry_objects(&document, &scene_ids)?;
    validate_gel_electrophoresis_objects(&document.objects)?;
    validate_plasmid_map_objects(&document.objects)?;
    validate_bio_shape_objects(&document.objects)?;
    validate_image_objects(&document)?;
    validate_geometry_constraint_objects(&document)?;
    validate_molecule_fragment_resources(&document)?;
    split_disconnected_molecule_objects(&mut document);
    normalize_text_object_payloads(&mut document);
    normalize_shape_object_payloads(&mut document);
    normalize_arrow_object_payloads(&mut document);
    normalize_fragment_label_payloads(&mut document);
    validate_chemical_properties(&document)?;
    validate_logical_objects(&document)?;
    validate_link_relations(&document)?;
    Ok(document)
}

fn validate_image_objects(document: &ChemSemaDocument) -> Result<(), String> {
    fn visit(document: &ChemSemaDocument, objects: &[SceneObject]) -> Result<(), String> {
        for object in objects {
            if object.payload.extra.contains_key("imageCrop") && object.object_type != "image" {
                return Err(format!(
                    "object {} carries imageCrop but is not an image",
                    object.id
                ));
            }
            if object.object_type == "image" {
                let crop = object.payload.image_crop()?;
                if let Some(crop) = crop {
                    let resource_ref = object
                        .payload
                        .resource_ref
                        .as_deref()
                        .ok_or_else(|| format!("image {} has no resourceRef", object.id))?;
                    let resource = document.resources.get(resource_ref).ok_or_else(|| {
                        format!(
                            "image {} references missing resource {resource_ref}",
                            object.id
                        )
                    })?;
                    let image = resource.display_image().ok_or_else(|| {
                        format!("image {} has no decodable preview to crop", object.id)
                    })?;
                    crop.validate(image.pixel_width, image.pixel_height)
                        .map_err(|error| format!("image {}: {error}", object.id))?;
                }
            }
            visit(document, &object.children)?;
        }
        Ok(())
    }
    visit(document, &document.objects)
}

fn validate_logical_objects(document: &ChemSemaDocument) -> Result<(), String> {
    let scene_ids = document
        .scene_objects()
        .into_iter()
        .map(|object| object.id.clone())
        .collect::<BTreeSet<_>>();
    let node_ids = document
        .editable_fragments()
        .into_iter()
        .flat_map(|entry| entry.fragment.nodes.iter().map(|node| node.id.clone()))
        .collect::<BTreeSet<_>>();
    let bond_ids = document
        .editable_fragments()
        .into_iter()
        .flat_map(|entry| entry.fragment.bonds.iter().map(|bond| bond.id.clone()))
        .collect::<BTreeSet<_>>();
    document
        .logical_objects
        .validate(&scene_ids, &node_ids, &bond_ids)?;
    let mut all_ids = scene_ids
        .iter()
        .chain(node_ids.iter())
        .chain(bond_ids.iter())
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    all_ids.extend(document.logical_objects.all_ids());
    for scheme in &document.reaction_schemes {
        if !all_ids.insert(scheme.id.as_str()) {
            return Err(format!(
                "reaction scheme id '{}' collides with another document entity",
                scheme.id
            ));
        }
        for step in &scheme.steps {
            if !all_ids.insert(step.id.as_str()) {
                return Err(format!(
                    "reaction step id '{}' collides with another document entity",
                    step.id
                ));
            }
        }
    }
    for splitter in &document.document.layout.splitters {
        if !all_ids.insert(splitter.id.as_str()) {
            return Err(format!(
                "document splitter id '{}' collides with another document entity",
                splitter.id
            ));
        }
    }
    Ok(())
}

fn migrate_legacy_bracket_links(document: &mut ChemSemaDocument) {
    let existing = document
        .links
        .iter()
        .flat_map(|relation| {
            relation
                .endpoints
                .iter()
                .map(|endpoint| endpoint.entity_id.clone())
        })
        .collect::<BTreeSet<_>>();
    let pairs = document
        .scene_objects()
        .into_iter()
        .filter_map(|object| {
            let text_id = object
                .meta
                .get("linkedTextObjectId")
                .and_then(Value::as_str)?;
            Some((object.id.clone(), text_id.to_string()))
        })
        .collect::<Vec<_>>();
    let mut next = document.links.len() + 1;
    for (bracket_id, text_id) in pairs {
        if existing.contains(&bracket_id) || existing.contains(&text_id) {
            continue;
        }
        document.links.push(LinkRelation {
            id: format!("link_migrated_{next}"),
            kind: "bracket-repeat-label".to_string(),
            endpoints: vec![
                LinkEndpoint {
                    entity_id: bracket_id,
                    role: "bracket".to_string(),
                },
                LinkEndpoint {
                    entity_id: text_id,
                    role: "label".to_string(),
                },
            ],
            data: json!({"inference": "declared"}),
        });
        next += 1;
    }
    clear_legacy_link_meta(&mut document.objects);
}

fn clear_legacy_link_meta(objects: &mut [SceneObject]) {
    const LEGACY_FIELDS: [&str; 6] = [
        "linkedTextObjectId",
        "bracketLabelTextObjectId",
        "linkKind",
        "linkedBracketObjectId",
        "bracketObjectId",
        "repeatUnitDetached",
    ];
    for object in objects {
        if let Some(meta) = object.meta.as_object_mut() {
            for field in LEGACY_FIELDS {
                meta.remove(field);
            }
            if meta.is_empty() {
                object.meta = Value::Null;
            }
        }
        clear_legacy_link_meta(&mut object.children);
    }
}

fn validate_link_relations(document: &ChemSemaDocument) -> Result<(), String> {
    let scene_ids = document
        .scene_objects()
        .into_iter()
        .map(|object| object.id.as_str())
        .collect::<BTreeSet<_>>();
    let node_ids = document
        .editable_fragments()
        .into_iter()
        .flat_map(|entry| entry.fragment.nodes.iter().map(|node| node.id.as_str()))
        .collect::<BTreeSet<_>>();
    let bond_ids = document
        .editable_fragments()
        .into_iter()
        .flat_map(|entry| entry.fragment.bonds.iter().map(|bond| bond.id.as_str()))
        .collect::<BTreeSet<_>>();
    let mut relation_ids = BTreeSet::new();
    for relation in &document.links {
        if relation.id.trim().is_empty() || !relation_ids.insert(relation.id.as_str()) {
            return Err(format!(
                "link relation id '{}' is empty or duplicated",
                relation.id
            ));
        }
        let endpoint_ids = relation
            .endpoints
            .iter()
            .map(|endpoint| endpoint.entity_id.as_str())
            .collect::<BTreeSet<_>>();
        if endpoint_ids.len() != relation.endpoints.len() {
            return Err(format!(
                "link relation '{}' repeats an endpoint",
                relation.id
            ));
        }
        for endpoint in &relation.endpoints {
            if !scene_ids.contains(endpoint.entity_id.as_str())
                && !node_ids.contains(endpoint.entity_id.as_str())
                && !bond_ids.contains(endpoint.entity_id.as_str())
            {
                return Err(format!(
                    "link relation '{}' references missing entity '{}'",
                    relation.id, endpoint.entity_id
                ));
            }
        }
        let role = |name: &str| {
            relation
                .endpoints
                .iter()
                .find(|endpoint| endpoint.role == name)
        };
        let object_type = |endpoint: &LinkEndpoint| {
            document
                .find_scene_object(&endpoint.entity_id)
                .map(|object| object.object_type.as_str())
        };
        let valid = match relation.kind.as_str() {
            "bracket-repeat-label" => {
                relation.endpoints.len() == 2
                    && role("bracket").is_some_and(|endpoint| {
                        document
                            .find_scene_object(&endpoint.entity_id)
                            .is_some_and(|object| {
                                object.object_type == "bracket"
                                    || (object.object_type == "group"
                                        && object.meta.get("kind").and_then(Value::as_str)
                                            == Some("bracket-group"))
                            })
                    })
                    && role("label").and_then(object_type) == Some("text")
            }
            "analysis-caption" => {
                relation.endpoints.len() == 2
                    && role("source").and_then(object_type) == Some("molecule")
                    && role("caption").and_then(object_type) == Some("text")
            }
            "atom-symbol" => {
                relation.endpoints.len() == 2
                    && role("atom")
                        .is_some_and(|endpoint| node_ids.contains(endpoint.entity_id.as_str()))
                    && role("symbol").and_then(object_type) == Some("symbol")
            }
            "chemical-property-display" => {
                let property_id = relation
                    .data
                    .get("chemicalPropertyId")
                    .and_then(Value::as_str);
                let Some(property) = property_id.and_then(|id| {
                    document
                        .chemical_properties
                        .iter()
                        .find(|property| property.id == id)
                }) else {
                    return Err(format!(
                        "link relation '{}' references a missing chemical property",
                        relation.id
                    ));
                };
                let display = role("display");
                let basis = relation
                    .endpoints
                    .iter()
                    .filter(|endpoint| endpoint.role == "basis")
                    .map(|endpoint| endpoint.entity_id.as_str())
                    .collect::<Vec<_>>();
                display.and_then(object_type) == Some("text")
                    && property.display_object_id.as_deref()
                        == display.map(|endpoint| endpoint.entity_id.as_str())
                    && basis
                        == property
                            .basis_entity_ids
                            .iter()
                            .map(String::as_str)
                            .collect::<Vec<_>>()
            }
            "annotation-basis" => {
                let annotation = role("annotation");
                let basis = relation
                    .endpoints
                    .iter()
                    .filter(|endpoint| endpoint.role == "basis")
                    .map(|endpoint| endpoint.entity_id.as_str())
                    .collect::<Vec<_>>();
                annotation
                    .and_then(|endpoint| document.find_scene_object(&endpoint.entity_id))
                    .is_some_and(|object| {
                        matches!(
                            object.kind(),
                            crate::SceneObjectKind::Geometry | crate::SceneObjectKind::Constraint
                        ) && basis
                            == object
                                .payload
                                .geometry
                                .as_ref()
                                .map(|geometry| geometry.basis_entity_ids.as_slice())
                                .or_else(|| {
                                    object
                                        .payload
                                        .constraint
                                        .as_ref()
                                        .map(|constraint| constraint.basis_entity_ids.as_slice())
                                })
                                .unwrap_or(&[])
                    })
                    && relation.endpoints.len() == basis.len() + 1
            }
            _ => false,
        };
        if !valid {
            return Err(format!(
                "link relation '{}' has invalid kind or endpoint signature '{}'",
                relation.id, relation.kind
            ));
        }
    }
    Ok(())
}

fn validate_chemical_properties(document: &ChemSemaDocument) -> Result<(), String> {
    let scene_ids = document
        .scene_objects()
        .into_iter()
        .map(|object| object.id.as_str())
        .collect::<BTreeSet<_>>();
    let node_ids = document
        .editable_fragments()
        .into_iter()
        .flat_map(|entry| entry.fragment.nodes.iter().map(|node| node.id.as_str()))
        .collect::<BTreeSet<_>>();
    let bond_ids = document
        .editable_fragments()
        .into_iter()
        .flat_map(|entry| entry.fragment.bonds.iter().map(|bond| bond.id.as_str()))
        .collect::<BTreeSet<_>>();
    let mut ids = BTreeSet::new();
    for property in &document.chemical_properties {
        if property.id.trim().is_empty() || !ids.insert(property.id.as_str()) {
            return Err(format!(
                "chemical property id '{}' is empty or duplicated",
                property.id
            ));
        }
        property.property_type.validate()?;
        let mut basis = BTreeSet::new();
        for entity_id in &property.basis_entity_ids {
            if !basis.insert(entity_id.as_str()) {
                return Err(format!(
                    "chemical property '{}' repeats basis entity '{}'",
                    property.id, entity_id
                ));
            }
            if !scene_ids.contains(entity_id.as_str())
                && !node_ids.contains(entity_id.as_str())
                && !bond_ids.contains(entity_id.as_str())
            {
                return Err(format!(
                    "chemical property '{}' references missing basis entity '{}'",
                    property.id, entity_id
                ));
            }
        }
        if let Some(display_id) = property.display_object_id.as_deref() {
            if document
                .find_scene_object(display_id)
                .is_none_or(|object| object.object_type != "text")
            {
                return Err(format!(
                    "chemical property '{}' display '{}' is not a text object",
                    property.id, display_id
                ));
            }
        }
        if !property.is_active
            && property.calculation_state != ChemicalPropertyCalculationState::Static
        {
            return Err(format!(
                "inactive chemical property '{}' must have static calculation state",
                property.id
            ));
        }
    }
    Ok(())
}

fn migrate_legacy_external_connection_points(value: &mut Value) {
    let Some(resources) = value.get_mut("resources").and_then(Value::as_object_mut) else {
        return;
    };
    for resource in resources.values_mut() {
        let Some(nodes) = resource
            .get_mut("data")
            .and_then(|data| data.get_mut("nodes"))
            .and_then(Value::as_array_mut)
        else {
            continue;
        };
        for node in nodes {
            let Some(node) = node.as_object_mut() else {
                continue;
            };
            if let Some(Value::Bool(is_external)) = node.remove("isExternalConnectionPoint") {
                if is_external && !node.contains_key("externalConnection") {
                    node.insert(
                        "externalConnection".to_string(),
                        json!({ "type": "unspecified" }),
                    );
                }
            }
        }
    }
}

fn validate_scene_object_types(objects: &[SceneObject]) -> Result<(), String> {
    for object in objects {
        crate::SceneObjectKind::parse(&object.object_type)
            .map_err(|error| format!("{error} on object '{}'", object.id))?;
        validate_scene_object_types(&object.children)?;
    }
    Ok(())
}

fn validate_spectrum_objects(objects: &[SceneObject]) -> Result<(), String> {
    for object in objects {
        let spectrum = object.payload.spectrum.as_ref();
        if object.object_type == "spectrum" {
            let spectrum = spectrum.ok_or_else(|| {
                format!(
                    "spectrum object '{}' is missing payload.spectrum",
                    object.id
                )
            })?;
            spectrum
                .validate()
                .map_err(|error| format!("{error} on object '{}'", object.id))?;
            let Some([x, y, width, height]) = object.payload.bbox else {
                return Err(format!(
                    "spectrum object '{}' is missing payload.bbox",
                    object.id
                ));
            };
            if ![x, y, width, height].into_iter().all(f64::is_finite)
                || width <= EPSILON
                || height <= EPSILON
            {
                return Err(format!(
                    "spectrum object '{}' has an invalid payload.bbox",
                    object.id
                ));
            }
            if object.transform.rotate.abs() > EPSILON {
                return Err(format!("spectrum object '{}' cannot be rotated", object.id));
            }
            if (object.transform.scale[0] - 1.0).abs() > EPSILON
                || (object.transform.scale[1] - 1.0).abs() > EPSILON
            {
                return Err(format!(
                    "spectrum object '{}' must store its edited size in payload.bbox",
                    object.id
                ));
            }
        } else if spectrum.is_some() {
            return Err(format!(
                "non-spectrum object '{}' contains payload.spectrum",
                object.id
            ));
        }
        validate_spectrum_objects(&object.children)?;
    }
    Ok(())
}

fn validate_table_objects(
    objects: &[SceneObject],
    scene_ids: &BTreeSet<String>,
    owned_content_ids: &mut BTreeSet<String>,
) -> Result<(), String> {
    for object in objects {
        match (object.object_type.as_str(), object.payload.table.as_ref()) {
            ("table", Some(table)) => {
                if table.rows == 0
                    || table.columns == 0
                    || table.row_guides.len() != table.rows + 1
                    || table.column_guides.len() != table.columns + 1
                    || table.cells.len() != table.rows * table.columns
                {
                    return Err(format!(
                        "table object '{}' has inconsistent grid dimensions",
                        object.id
                    ));
                }
                let strictly_increasing = |guides: &[f64]| {
                    guides.iter().all(|value| value.is_finite())
                        && guides
                            .windows(2)
                            .all(|pair| pair[1] - pair[0] > crate::EPSILON)
                };
                if !strictly_increasing(&table.row_guides)
                    || !strictly_increasing(&table.column_guides)
                {
                    return Err(format!(
                        "table object '{}' has invalid row or column guides",
                        object.id
                    ));
                }
                let valid_border = |border: &TableBorder| {
                    border.width.is_finite()
                        && border.width >= 0.0
                        && border.color.len() == 7
                        && border.color.starts_with('#')
                        && border.color[1..]
                            .chars()
                            .all(|character| character.is_ascii_hexdigit())
                };
                if !valid_border(&table.default_border) {
                    return Err(format!(
                        "table object '{}' has an invalid default border",
                        object.id
                    ));
                }
                let mut positions = BTreeSet::new();
                let mut ids = BTreeSet::new();
                for cell in &table.cells {
                    if cell.row >= table.rows
                        || cell.column >= table.columns
                        || !positions.insert((cell.row, cell.column))
                        || cell.id.trim().is_empty()
                        || !ids.insert(cell.id.as_str())
                    {
                        return Err(format!(
                            "table object '{}' has an invalid or duplicated cell",
                            object.id
                        ));
                    }
                    if [
                        cell.borders.top.as_ref(),
                        cell.borders.left.as_ref(),
                        cell.borders.bottom.as_ref(),
                        cell.borders.right.as_ref(),
                    ]
                    .into_iter()
                    .flatten()
                    .any(|border| !valid_border(border))
                    {
                        return Err(format!(
                            "table object '{}' has an invalid cell border",
                            object.id
                        ));
                    }
                    let mut cell_content_ids = BTreeSet::new();
                    for content_id in &cell.content_object_ids {
                        if content_id == &object.id
                            || !scene_ids.contains(content_id)
                            || !cell_content_ids.insert(content_id.as_str())
                            || !owned_content_ids.insert(content_id.clone())
                        {
                            return Err(format!(
                                "table object '{}' cell '{}' has an invalid, missing, or multiply owned content object '{}'",
                                object.id, cell.id, content_id
                            ));
                        }
                    }
                }
            }
            ("table", None) => {
                return Err(format!(
                    "table object '{}' is missing payload.table",
                    object.id
                ));
            }
            (_, Some(_)) => {
                return Err(format!(
                    "non-table object '{}' contains payload.table",
                    object.id
                ));
            }
            _ => {}
        }
        validate_table_objects(&object.children, scene_ids, owned_content_ids)?;
    }
    Ok(())
}

fn validate_gel_electrophoresis_objects(objects: &[SceneObject]) -> Result<(), String> {
    for object in objects {
        if let Some(gel) = object.payload.gel_electrophoresis.as_ref() {
            if object.object_type != "shape"
                || object.payload.extra.get("kind").and_then(Value::as_str) != Some("gelPlate")
            {
                return Err(format!(
                    "non-gel shape '{}' contains payload.gelElectrophoresis",
                    object.id
                ));
            }
            gel.validate()
                .map_err(|error| format!("{error} on object '{}'", object.id))?;
            let Some([x, y, width, height]) = object.payload.bbox else {
                return Err(format!("gel plate '{}' is missing payload.bbox", object.id));
            };
            if ![x, y, width, height].into_iter().all(f64::is_finite)
                || width <= EPSILON
                || height <= EPSILON
            {
                return Err(format!(
                    "gel plate '{}' has invalid payload.bbox",
                    object.id
                ));
            }
        } else if object.object_type == "shape"
            && object.payload.extra.get("kind").and_then(Value::as_str) == Some("gelPlate")
        {
            return Err(format!(
                "gel plate '{}' is missing payload.gelElectrophoresis",
                object.id
            ));
        }
        validate_gel_electrophoresis_objects(&object.children)?;
    }
    Ok(())
}

fn validate_plasmid_map_objects(objects: &[SceneObject]) -> Result<(), String> {
    for object in objects {
        if let Some(plasmid) = object.payload.plasmid_map.as_ref() {
            if object.object_type != "shape"
                || object.payload.extra.get("kind").and_then(Value::as_str) != Some("plasmidMap")
            {
                return Err(format!(
                    "non-plasmid shape '{}' contains payload.plasmidMap",
                    object.id
                ));
            }
            plasmid
                .validate()
                .map_err(|error| format!("{error} on object '{}'", object.id))?;
            let Some([x, y, width, height]) = object.payload.bbox else {
                return Err(format!(
                    "plasmid map '{}' is missing payload.bbox",
                    object.id
                ));
            };
            if ![x, y, width, height].into_iter().all(f64::is_finite)
                || width <= EPSILON
                || height <= EPSILON
            {
                return Err(format!(
                    "plasmid map '{}' has invalid payload.bbox",
                    object.id
                ));
            }
        } else if object.object_type == "shape"
            && object.payload.extra.get("kind").and_then(Value::as_str) == Some("plasmidMap")
        {
            return Err(format!(
                "plasmid map '{}' is missing payload.plasmidMap",
                object.id
            ));
        }
        validate_plasmid_map_objects(&object.children)?;
    }
    Ok(())
}

fn validate_bio_shape_objects(objects: &[SceneObject]) -> Result<(), String> {
    for object in objects {
        match (
            object.object_type.as_str(),
            object.payload.bio_shape.as_ref(),
        ) {
            ("shape", Some(data)) => {
                if object.payload.extra.get("kind").and_then(Value::as_str) != Some("bioShape") {
                    return Err(format!(
                        "BioShape '{}' must use the explicit bioShape shape kind",
                        object.id
                    ));
                }
                data.validate()
                    .map_err(|error| format!("{error} on object '{}'", object.id))?;
                let Some([x, y, width, height]) = object.payload.bbox else {
                    return Err(format!("BioShape '{}' is missing payload.bbox", object.id));
                };
                if ![x, y, width, height].into_iter().all(f64::is_finite)
                    || width <= EPSILON
                    || height <= EPSILON
                {
                    return Err(format!("BioShape '{}' has invalid payload.bbox", object.id));
                }
            }
            ("shape", None)
                if object.payload.extra.get("kind").and_then(Value::as_str) == Some("bioShape") =>
            {
                return Err(format!(
                    "BioShape '{}' is missing payload.bioShape",
                    object.id
                ));
            }
            (_, Some(_)) => {
                return Err(format!(
                    "non-shape object '{}' contains payload.bioShape",
                    object.id
                ));
            }
            _ => {}
        }
        validate_bio_shape_objects(&object.children)?;
    }
    Ok(())
}

fn validate_stoichiometry_objects(
    document: &ChemSemaDocument,
    scene_ids: &BTreeSet<String>,
) -> Result<(), String> {
    let node_ids = document
        .editable_fragments()
        .into_iter()
        .flat_map(|entry| entry.fragment.nodes.iter().map(|node| node.id.as_str()))
        .collect::<BTreeSet<_>>();
    let mut scheme_ids = BTreeSet::new();
    let mut step_ids = BTreeSet::new();
    for scheme in &document.reaction_schemes {
        if scheme.id.trim().is_empty() || !scheme_ids.insert(scheme.id.as_str()) {
            return Err(format!(
                "reaction scheme id '{}' is empty or duplicated",
                scheme.id
            ));
        }
        for step in &scheme.steps {
            if step.id.trim().is_empty() || !step_ids.insert(step.id.as_str()) {
                return Err(format!(
                    "reaction step id '{}' is empty or duplicated",
                    step.id
                ));
            }
            for entity_id in step
                .reactant_entity_ids
                .iter()
                .chain(step.product_entity_ids.iter())
                .chain(step.arrow_object_ids.iter())
                .chain(step.plus_object_ids.iter())
                .chain(step.objects_above_arrow.iter())
                .chain(step.objects_below_arrow.iter())
            {
                if !scene_ids.contains(entity_id) {
                    return Err(format!(
                        "reaction step '{}' references missing scene object '{}'",
                        step.id, entity_id
                    ));
                }
            }
            for mapping in &step.atom_mappings {
                if !node_ids.contains(mapping.reactant_atom_id.as_str())
                    || !node_ids.contains(mapping.product_atom_id.as_str())
                {
                    return Err(format!(
                        "reaction step '{}' contains an atom mapping with a missing atom",
                        step.id
                    ));
                }
            }
        }
    }
    validate_stoichiometry_scene_objects(&document.objects, scene_ids, &step_ids)
}

fn validate_stoichiometry_scene_objects(
    objects: &[SceneObject],
    scene_ids: &BTreeSet<String>,
    step_ids: &BTreeSet<&str>,
) -> Result<(), String> {
    for object in objects {
        match (
            object.object_type.as_str(),
            object.payload.stoichiometry_grid.as_ref(),
        ) {
            ("stoichiometry-grid", Some(grid)) => {
                grid.validate()
                    .map_err(|error| format!("{error} on object '{}'", object.id))?;
                if grid
                    .source_reaction_step_id
                    .as_deref()
                    .is_some_and(|id| !step_ids.contains(id))
                {
                    return Err(format!(
                        "stoichiometry grid '{}' references missing reaction step",
                        object.id
                    ));
                }
                for component in &grid.components {
                    if component
                        .reference_entity_id
                        .as_ref()
                        .is_some_and(|id| !scene_ids.contains(id))
                    {
                        return Err(format!(
                            "stoichiometry grid '{}' component '{}' references missing scene object",
                            object.id, component.id
                        ));
                    }
                }
                let Some([x, y, width, height]) = object.payload.bbox else {
                    return Err(format!(
                        "stoichiometry grid '{}' is missing payload.bbox",
                        object.id
                    ));
                };
                if ![x, y, width, height].into_iter().all(f64::is_finite)
                    || width <= EPSILON
                    || height <= EPSILON
                {
                    return Err(format!(
                        "stoichiometry grid '{}' has an invalid payload.bbox",
                        object.id
                    ));
                }
            }
            ("stoichiometry-grid", None) => {
                return Err(format!(
                    "stoichiometry grid '{}' is missing payload.stoichiometryGrid",
                    object.id
                ));
            }
            (_, Some(_)) => {
                return Err(format!(
                    "non-stoichiometry-grid object '{}' contains payload.stoichiometryGrid",
                    object.id
                ));
            }
            _ => {}
        }
        validate_stoichiometry_scene_objects(&object.children, scene_ids, step_ids)?;
    }
    Ok(())
}

fn validate_geometry_constraint_objects(document: &ChemSemaDocument) -> Result<(), String> {
    let scene_ids = document
        .scene_objects()
        .into_iter()
        .map(|object| object.id.as_str())
        .collect::<BTreeSet<_>>();
    let node_ids = document
        .editable_fragments()
        .into_iter()
        .flat_map(|entry| entry.fragment.nodes.iter().map(|node| node.id.as_str()))
        .collect::<BTreeSet<_>>();
    let bond_ids = document
        .editable_fragments()
        .into_iter()
        .flat_map(|entry| entry.fragment.bonds.iter().map(|bond| bond.id.as_str()))
        .collect::<BTreeSet<_>>();
    for object in document.scene_objects() {
        let basis = match object.kind() {
            crate::SceneObjectKind::Geometry => {
                if object.payload.constraint.is_some() {
                    return Err(format!(
                        "geometry object '{}' cannot contain payload.constraint",
                        object.id
                    ));
                }
                let data = object.payload.geometry.as_ref().ok_or_else(|| {
                    format!(
                        "geometry object '{}' is missing payload.geometry",
                        object.id
                    )
                })?;
                data.validate()?;
                &data.basis_entity_ids
            }
            crate::SceneObjectKind::Constraint => {
                if object.payload.geometry.is_some() {
                    return Err(format!(
                        "constraint object '{}' cannot contain payload.geometry",
                        object.id
                    ));
                }
                let data = object.payload.constraint.as_ref().ok_or_else(|| {
                    format!(
                        "constraint object '{}' is missing payload.constraint",
                        object.id
                    )
                })?;
                data.validate()?;
                &data.basis_entity_ids
            }
            _ => continue,
        };
        for entity_id in basis {
            if entity_id == &object.id {
                return Err(format!(
                    "object '{}' cannot reference itself as basis",
                    object.id
                ));
            }
            if !scene_ids.contains(entity_id.as_str())
                && !node_ids.contains(entity_id.as_str())
                && !bond_ids.contains(entity_id.as_str())
            {
                return Err(format!(
                    "object '{}' references missing basis entity '{}'",
                    object.id, entity_id
                ));
            }
        }
    }
    crate::geometry_constraints::validate_annotation_graph(document)
}

fn validate_molecule_fragment_resources(document: &ChemSemaDocument) -> Result<(), String> {
    for (id, resource) in &document.resources {
        let declares_fragment = resource.resource_type == "molecule_fragment2d"
            || resource.encoding == "chemsema.molecule.fragment2d";
        if declares_fragment && !matches!(&resource.data, ResourceData::Fragment(_)) {
            let detail = match &resource.data {
                ResourceData::Json(value) => {
                    serde_json::from_value::<MoleculeFragment>(value.clone())
                        .err()
                        .map(|error| format!(" {error}"))
                        .unwrap_or_default()
                }
                ResourceData::Text(_) => " resource data is text, not an object".to_string(),
                ResourceData::Fragment(_) => String::new(),
            };
            return Err(format!(
                "Resource {id} is declared as molecule_fragment2d but data is not a valid chemsema.molecule.fragment2d fragment.{detail}"
            ));
        }
        if let ResourceData::Fragment(fragment) = &resource.data {
            for node in &fragment.nodes {
                if let Some(color) = &node.highlight_color {
                    validate_native_rgb_color(color, "node highlight", &node.id)?;
                }
            }
            for bond in &fragment.bonds {
                if let Some(color) = &bond.highlight_color {
                    validate_native_rgb_color(color, "bond highlight", &bond.id)?;
                }
            }
            let mut area_ids = BTreeSet::new();
            for area in &fragment.colored_areas {
                validate_native_rgb_color(&area.color, "colored molecular area", &area.id)?;
                if !area_ids.insert(area.id.as_str()) {
                    return Err(format!(
                        "Resource {id} has duplicate colored molecular area id '{}'.",
                        area.id
                    ));
                }
                if ordered_colored_area_node_ids(fragment, &area.basis_bonds).is_none() {
                    return Err(format!(
                        "Resource {id} colored molecular area '{}' must reference exactly one connected simple ring.",
                        area.id
                    ));
                }
            }
            for node in &fragment.nodes {
                for assignment in &node.nmr_assignments {
                    assignment
                        .validate()
                        .map_err(|error| format!("{error} on node '{}'", node.id))?;
                }
            }
        }
    }
    Ok(())
}

fn validate_native_rgb_color(color: &str, kind: &str, id: &str) -> Result<(), String> {
    if color.len() == 7
        && color.starts_with('#')
        && color[1..]
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        Ok(())
    } else {
        Err(format!(
            "{kind} '{id}' color must be an explicit #RRGGBB value."
        ))
    }
}

fn split_disconnected_molecule_objects(document: &mut ChemSemaDocument) {
    let mut next_index = next_available_molecule_index(document);
    let ChemSemaDocument {
        objects, resources, ..
    } = document;
    split_disconnected_molecule_objects_in(objects, resources, &mut next_index);
}

fn split_disconnected_molecule_objects_in(
    objects: &mut Vec<SceneObject>,
    resources: &mut BTreeMap<String, Resource>,
    next_index: &mut usize,
) {
    let mut index = 0;
    while index < objects.len() {
        split_disconnected_molecule_objects_in(&mut objects[index].children, resources, next_index);
        if !is_disconnected_molecule_split_candidate(&objects[index]) {
            index += 1;
            continue;
        }

        let object = objects[index].clone();
        let Some(resource_ref) = object.payload.resource_ref.as_ref() else {
            index += 1;
            continue;
        };
        let Some(resource) = resources.get(resource_ref).cloned() else {
            index += 1;
            continue;
        };
        let Some(fragment) = resource.data.as_fragment().cloned() else {
            index += 1;
            continue;
        };
        let components = split_legacy_fragment_components(&fragment);
        if components.len() <= 1 {
            index += 1;
            continue;
        }

        resources.remove(resource_ref);
        let replacements =
            legacy_component_scene_objects(&object, &resource, components, resources, next_index);
        objects.splice(index..=index, replacements);
        index += 1;
    }
}

fn is_disconnected_molecule_split_candidate(object: &SceneObject) -> bool {
    if object.object_type != "molecule" {
        return false;
    }
    let preserve_disconnected_components = object
        .meta
        .get("preserveDisconnectedComponents")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    object.visible
        && !preserve_disconnected_components
        && object.transform.rotate.abs() <= EPSILON
        && (object.transform.scale[0] - 1.0).abs() <= EPSILON
        && (object.transform.scale[1] - 1.0).abs() <= EPSILON
}

fn next_available_molecule_index(document: &ChemSemaDocument) -> usize {
    let mut max_index = 0usize;
    for id in document.resources.keys() {
        max_index = max_index.max(parse_indexed_id(id, "mol_").unwrap_or(0));
    }
    for object in document.scene_objects() {
        max_index = max_index.max(parse_indexed_id(&object.id, "obj_mol_").unwrap_or(0));
    }
    max_index + 1
}

fn parse_indexed_id(id: &str, prefix: &str) -> Option<usize> {
    id.strip_prefix(prefix)?.parse().ok()
}

pub(crate) fn subset_molecule_semantics(
    fragment: &MoleculeFragment,
    node_ids: &BTreeSet<String>,
    bond_ids: &BTreeSet<String>,
) -> (
    Vec<chemsema_chemical_graph::StereoElementV2>,
    Vec<chemsema_chemical_graph::MultiCenterInteractionV2>,
) {
    use chemsema_chemical_graph::{StereoCarrierV2, StereoElementV2, StereoReferenceV2};

    let carrier_is_inside = |carrier: &StereoCarrierV2| match carrier {
        StereoCarrierV2::Atom(atom)
        | StereoCarrierV2::LonePair(atom)
        | StereoCarrierV2::DuplicateAtom(atom) => node_ids.contains(atom),
        StereoCarrierV2::Bond(bond) => bond_ids.contains(bond),
        StereoCarrierV2::AtomSet(atoms) | StereoCarrierV2::Plane(atoms) => {
            atoms.iter().all(|atom| node_ids.contains(atom))
        }
        StereoCarrierV2::Axis(atoms) => atoms.iter().all(|atom| node_ids.contains(atom)),
        StereoCarrierV2::ConjugatedDoubleBondPair(bonds) => {
            bonds.iter().all(|bond| bond_ids.contains(bond))
        }
        StereoCarrierV2::Torsion(atoms) => atoms.iter().all(|atom| node_ids.contains(atom)),
    };
    let mut stereo = fragment
        .stereo
        .iter()
        .filter(|element| match element {
            StereoElementV2::Tetrahedral {
                center, references, ..
            } => {
                node_ids.contains(center)
                    && references.iter().all(|reference| match reference {
                        StereoReferenceV2::Atom(atom) => node_ids.contains(atom),
                        StereoReferenceV2::ImplicitHydrogen => true,
                    })
            }
            StereoElementV2::DoubleBond {
                bond,
                left_reference,
                right_reference,
                ..
            } => {
                bond_ids.contains(bond)
                    && node_ids.contains(left_reference)
                    && node_ids.contains(right_reference)
            }
            StereoElementV2::EnhancedGroup { .. } => false,
            StereoElementV2::Extended { carriers, .. }
            | StereoElementV2::Conformation { carriers, .. }
            | StereoElementV2::Unspecified { carriers, .. } => {
                carriers.iter().all(&carrier_is_inside)
            }
        })
        .cloned()
        .collect::<Vec<_>>();
    let retained_ids = stereo
        .iter()
        .map(|element| stereo_element_id(element).to_string())
        .collect::<BTreeSet<_>>();
    stereo.extend(fragment.stereo.iter().filter_map(|element| {
        let StereoElementV2::EnhancedGroup { members, .. } = element else {
            return None;
        };
        members
            .iter()
            .all(|member| {
                retained_ids.contains(member.as_str())
                    || member
                        .strip_prefix("tetrahedral-")
                        .is_some_and(|node| node_ids.contains(node))
            })
            .then(|| element.clone())
    }));
    let interactions = fragment
        .interactions
        .iter()
        .filter(|interaction| {
            interaction
                .centers
                .iter()
                .flat_map(|center| &center.atoms)
                .all(|atom| node_ids.contains(atom))
        })
        .cloned()
        .collect();
    (stereo, interactions)
}

pub(crate) fn retain_valid_molecule_semantics(fragment: &mut MoleculeFragment) {
    let node_ids = fragment
        .nodes
        .iter()
        .map(|node| node.id.clone())
        .collect::<BTreeSet<_>>();
    let bond_ids = fragment
        .bonds
        .iter()
        .map(|bond| bond.id.clone())
        .collect::<BTreeSet<_>>();
    let (stereo, interactions) = subset_molecule_semantics(fragment, &node_ids, &bond_ids);
    fragment.stereo = stereo;
    fragment.interactions = interactions;
    retain_valid_colored_molecular_areas(fragment);
}

/// Return the ring atoms in traversal order when `basis_bonds` describes one
/// connected simple cycle. This is the sole geometry rule used by import,
/// editing, rendering, deletion, and export.
pub(crate) fn ordered_colored_area_node_ids(
    fragment: &MoleculeFragment,
    basis_bonds: &[String],
) -> Option<Vec<String>> {
    if basis_bonds.len() < 3 {
        return None;
    }
    let requested = basis_bonds
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if requested.len() != basis_bonds.len() {
        return None;
    }
    let mut adjacency = BTreeMap::<&str, Vec<&str>>::new();
    for bond_id in requested {
        let bond = fragment.bonds.iter().find(|bond| bond.id == bond_id)?;
        if bond.begin == bond.end {
            return None;
        }
        adjacency
            .entry(bond.begin.as_str())
            .or_default()
            .push(bond.end.as_str());
        adjacency
            .entry(bond.end.as_str())
            .or_default()
            .push(bond.begin.as_str());
    }
    if adjacency.len() != basis_bonds.len()
        || adjacency.values().any(|neighbors| neighbors.len() != 2)
    {
        return None;
    }
    let start = *adjacency.keys().next()?;
    let mut ordered = Vec::with_capacity(adjacency.len());
    let mut previous: Option<&str> = None;
    let mut current = start;
    loop {
        if ordered.iter().any(|node_id| node_id == current) {
            return (current == start && ordered.len() == adjacency.len()).then_some(ordered);
        }
        ordered.push(current.to_string());
        let neighbors = adjacency.get(current)?;
        let next = if Some(neighbors[0]) != previous {
            neighbors[0]
        } else {
            neighbors[1]
        };
        previous = Some(current);
        current = next;
    }
}

pub(crate) fn retain_valid_colored_molecular_areas(fragment: &mut MoleculeFragment) {
    let valid = fragment
        .colored_areas
        .iter()
        .filter(|area| ordered_colored_area_node_ids(fragment, &area.basis_bonds).is_some())
        .cloned()
        .collect();
    fragment.colored_areas = valid;
}

fn stereo_element_id(element: &chemsema_chemical_graph::StereoElementV2) -> &str {
    use chemsema_chemical_graph::StereoElementV2;
    match element {
        StereoElementV2::Tetrahedral { id, .. }
        | StereoElementV2::DoubleBond { id, .. }
        | StereoElementV2::EnhancedGroup { id, .. }
        | StereoElementV2::Extended { id, .. }
        | StereoElementV2::Conformation { id, .. }
        | StereoElementV2::Unspecified { id, .. } => id,
    }
}

#[derive(Debug)]
struct LegacyMoleculeComponent {
    fragment: MoleculeFragment,
    local_bounds: [f64; 4],
    component_index: usize,
    component_count: usize,
}

fn split_legacy_fragment_components(fragment: &MoleculeFragment) -> Vec<LegacyMoleculeComponent> {
    let components = molecule_fragment_connected_components(fragment);
    if components.len() <= 1 {
        return Vec::new();
    }

    let component_count = components.len();
    components
        .into_iter()
        .enumerate()
        .filter_map(|(component_index, node_ids)| {
            let mut nodes: Vec<Node> = fragment
                .nodes
                .iter()
                .filter(|node| node_ids.contains(&node.id))
                .cloned()
                .collect();
            let bonds: Vec<Bond> = fragment
                .bonds
                .iter()
                .filter(|bond| node_ids.contains(&bond.begin) && node_ids.contains(&bond.end))
                .cloned()
                .collect();
            let bond_ids = bonds
                .iter()
                .map(|bond| bond.id.clone())
                .collect::<BTreeSet<_>>();
            let (stereo, interactions) = subset_molecule_semantics(fragment, &node_ids, &bond_ids);
            let colored_areas = fragment
                .colored_areas
                .iter()
                .filter(|area| area.basis_bonds.iter().all(|id| bond_ids.contains(id)))
                .cloned()
                .collect();
            if !component_has_visible_molecule_content(&nodes, &bonds) {
                return None;
            }

            let local_bounds = molecule_component_bounds(&nodes).unwrap_or([
                0.0,
                0.0,
                fragment.bbox[2].max(1.0),
                fragment.bbox[3].max(1.0),
            ]);
            for node in &mut nodes {
                node.position[0] = round2(node.position[0] - local_bounds[0]);
                node.position[1] = round2(node.position[1] - local_bounds[1]);
                if let Some(label) = &mut node.label {
                    translate_node_label_geometry(label, -local_bounds[0], -local_bounds[1]);
                }
            }

            let mut component_fragment = MoleculeFragment {
                schema: fragment.schema.clone(),
                bbox: [
                    0.0,
                    0.0,
                    round2((local_bounds[2] - local_bounds[0]).max(1.0)),
                    round2((local_bounds[3] - local_bounds[1]).max(1.0)),
                ],
                nodes,
                bonds,
                colored_areas,
                stereo,
                interactions,
                meta: fragment.meta.clone(),
            };
            annotate_legacy_component_fragment_meta(
                &mut component_fragment,
                component_index,
                component_count,
            );
            Some(LegacyMoleculeComponent {
                fragment: component_fragment,
                local_bounds,
                component_index,
                component_count,
            })
        })
        .collect()
}

pub(crate) fn molecule_fragment_connected_components(
    fragment: &MoleculeFragment,
) -> Vec<BTreeSet<String>> {
    let mut adjacency: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for node in &fragment.nodes {
        adjacency.entry(node.id.as_str()).or_default();
    }
    for bond in &fragment.bonds {
        adjacency
            .entry(bond.begin.as_str())
            .or_default()
            .push(bond.end.as_str());
        adjacency
            .entry(bond.end.as_str())
            .or_default()
            .push(bond.begin.as_str());
    }
    for interaction in &fragment.interactions {
        let participants = interaction
            .centers
            .iter()
            .flat_map(|center| center.atoms.iter().map(String::as_str))
            .collect::<Vec<_>>();
        for left in &participants {
            for right in &participants {
                if left != right {
                    adjacency.entry(left).or_default().push(right);
                }
            }
        }
    }

    let mut visited = BTreeSet::new();
    let mut components = Vec::new();
    for node in &fragment.nodes {
        if visited.contains(node.id.as_str()) {
            continue;
        }
        let mut queue = VecDeque::from([node.id.as_str()]);
        let mut component = BTreeSet::new();
        while let Some(id) = queue.pop_front() {
            if !visited.insert(id) {
                continue;
            }
            component.insert(id.to_string());
            if let Some(neighbors) = adjacency.get(id) {
                for neighbor in neighbors {
                    if !visited.contains(neighbor) {
                        queue.push_back(neighbor);
                    }
                }
            }
        }
        if !component.is_empty() {
            components.push(component);
        }
    }
    components
}

fn component_has_visible_molecule_content(nodes: &[Node], bonds: &[Bond]) -> bool {
    !bonds.is_empty()
        || nodes.iter().any(|node| {
            node.atomic_number != 6
                || node
                    .label
                    .as_ref()
                    .is_some_and(|label| label.has_visible_text())
        })
}

pub(crate) fn molecule_component_bounds(nodes: &[Node]) -> Option<[f64; 4]> {
    let mut bounds = None;
    for node in nodes {
        include_point_in_bounds(&mut bounds, node.position);
        if let Some(label) = &node.label {
            if let Some(label_bounds) = label.bbox() {
                include_box_in_bounds(&mut bounds, label_bounds);
            }
            for polygon in &label.glyph_polygons {
                for point in polygon {
                    include_point_in_bounds(&mut bounds, *point);
                }
            }
        }
    }
    bounds.map(|mut bounds| {
        if (bounds[2] - bounds[0]).abs() < 1.0 {
            let center = (bounds[0] + bounds[2]) * 0.5;
            bounds[0] = center - 0.5;
            bounds[2] = center + 0.5;
        }
        if (bounds[3] - bounds[1]).abs() < 1.0 {
            let center = (bounds[1] + bounds[3]) * 0.5;
            bounds[1] = center - 0.5;
            bounds[3] = center + 0.5;
        }
        [
            round2(bounds[0]),
            round2(bounds[1]),
            round2(bounds[2]),
            round2(bounds[3]),
        ]
    })
}

fn include_point_in_bounds(bounds: &mut Option<[f64; 4]>, point: [f64; 2]) {
    if let Some(bounds) = bounds {
        bounds[0] = bounds[0].min(point[0]);
        bounds[1] = bounds[1].min(point[1]);
        bounds[2] = bounds[2].max(point[0]);
        bounds[3] = bounds[3].max(point[1]);
    } else {
        *bounds = Some([point[0], point[1], point[0], point[1]]);
    }
}

fn include_box_in_bounds(bounds: &mut Option<[f64; 4]>, bbox: [f64; 4]) {
    include_point_in_bounds(bounds, [bbox[0], bbox[1]]);
    include_point_in_bounds(bounds, [bbox[2], bbox[3]]);
}

fn legacy_component_scene_objects(
    source_object: &SceneObject,
    source_resource: &Resource,
    components: Vec<LegacyMoleculeComponent>,
    resources: &mut BTreeMap<String, Resource>,
    next_index: &mut usize,
) -> Vec<SceneObject> {
    components
        .into_iter()
        .map(|component| {
            let resource_id = next_available_molecule_resource_id(resources, next_index);
            let mut resource = source_resource.clone();
            resource.data = ResourceData::Fragment(component.fragment.clone());
            annotate_legacy_resource_meta(
                &mut resource,
                component.component_index,
                component.component_count,
            );
            resources.insert(resource_id.clone(), resource);

            let mut object = source_object.clone();
            object.id = format!("obj_mol_{:03}", *next_index - 1);
            object.name = format!("molecule {}", *next_index - 1);
            object.transform.translate[0] =
                round2(source_object.transform.translate[0] + component.local_bounds[0]);
            object.transform.translate[1] =
                round2(source_object.transform.translate[1] + component.local_bounds[1]);
            object.payload.resource_ref = Some(resource_id);
            object.payload.bbox = Some([
                0.0,
                0.0,
                round2((component.local_bounds[2] - component.local_bounds[0]).max(1.0)),
                round2((component.local_bounds[3] - component.local_bounds[1]).max(1.0)),
            ]);
            object.children.clear();
            annotate_legacy_object_meta(
                &mut object,
                component.component_index,
                component.component_count,
            );
            object
        })
        .collect()
}

fn next_available_molecule_resource_id(
    resources: &BTreeMap<String, Resource>,
    next_index: &mut usize,
) -> String {
    loop {
        let id = format!("mol_{:03}", *next_index);
        *next_index += 1;
        if !resources.contains_key(&id) {
            return id;
        }
    }
}

fn annotate_legacy_component_fragment_meta(
    fragment: &mut MoleculeFragment,
    component_index: usize,
    component_count: usize,
) {
    ensure_object_meta(&mut fragment.meta).insert(
        "legacyCdxmlMergedComponent".to_string(),
        json!({
            "componentIndex": component_index,
            "componentCount": component_count,
        }),
    );
}

fn annotate_legacy_resource_meta(
    resource: &mut Resource,
    component_index: usize,
    component_count: usize,
) {
    ensure_object_meta(&mut resource.meta).insert(
        "legacyCdxmlMergedComponent".to_string(),
        json!({
            "componentIndex": component_index,
            "componentCount": component_count,
        }),
    );
}

fn annotate_legacy_object_meta(
    object: &mut SceneObject,
    component_index: usize,
    component_count: usize,
) {
    ensure_object_meta(&mut object.meta).insert(
        "legacyCdxmlMergedComponent".to_string(),
        json!({
            "componentIndex": component_index,
            "componentCount": component_count,
        }),
    );
}

fn ensure_object_meta(value: &mut Value) -> &mut Map<String, Value> {
    if !value.is_object() {
        *value = json!({});
    }
    value.as_object_mut().expect("meta should be an object")
}

pub(crate) fn translate_node_label_geometry(label: &mut NodeLabel, delta_x: f64, delta_y: f64) {
    if delta_x.abs() <= EPSILON && delta_y.abs() <= EPSILON {
        return;
    }
    if let Some(position) = &mut label.position {
        position[0] = round2(position[0] + delta_x);
        position[1] = round2(position[1] + delta_y);
    }
    if let Some(bbox) = &mut label.box_field {
        translate_bbox(bbox, delta_x, delta_y);
    }
    if let Some(bbox) = &mut label.box_value {
        translate_bbox(bbox, delta_x, delta_y);
    }
    for polygon in &mut label.glyph_polygons {
        for point in polygon {
            point[0] = round2(point[0] + delta_x);
            point[1] = round2(point[1] + delta_y);
        }
    }
    for polygon in &mut label.glyph_clip_polygons {
        for point in polygon {
            point[0] = round2(point[0] + delta_x);
            point[1] = round2(point[1] + delta_y);
        }
    }
}

fn translate_bbox(bbox: &mut [f64; 4], delta_x: f64, delta_y: f64) {
    bbox[0] = round2(bbox[0] + delta_x);
    bbox[1] = round2(bbox[1] + delta_y);
    bbox[2] = round2(bbox[2] + delta_x);
    bbox[3] = round2(bbox[3] + delta_y);
}

pub(crate) fn normalize_arrow_object_payloads(document: &mut ChemSemaDocument) {
    normalize_arrow_objects(&mut document.objects);
}

fn normalize_arrow_objects(objects: &mut [SceneObject]) {
    for object in objects {
        if object.object_type == "line" {
            normalize_arrow_payload_extra(&mut object.payload.extra);
        }
        normalize_arrow_objects(&mut object.children);
    }
}

pub(crate) fn normalize_arrow_payload_extra(extra: &mut BTreeMap<String, Value>) {
    normalize_arrow_head_payload(extra);
    let curve = arrow_payload_curve(extra);
    if curve.abs() <= EPSILON {
        return;
    }
    if arrow_payload_geometry_is_valid(extra) {
        return;
    }
    let Some((start, end)) = arrow_payload_line_endpoints(extra) else {
        return;
    };
    if let Some(geometry) = default_arrow_arc_geometry_payload(start, end, curve) {
        extra.insert("arrowGeometry".to_string(), geometry);
    }
}

pub(crate) fn default_arrow_arc_geometry_payload(
    start: Point,
    end: Point,
    curve: f64,
) -> Option<Value> {
    let chord = Point::new(end.x - start.x, end.y - start.y);
    let chord_length = start.distance(end);
    if chord_length <= EPSILON || curve.abs() <= EPSILON {
        return None;
    }
    let sweep = -curve.to_radians();
    let half = sweep.abs() * 0.5;
    let sin_half = half.sin().abs();
    if sin_half <= EPSILON {
        return None;
    }
    let ux = chord.x / chord_length;
    let uy = chord.y / chord_length;
    let radius = chord_length / (2.0 * sin_half);
    let offset = radius * half.cos() * sweep.signum();
    let center = Point::new(
        (start.x + end.x) * 0.5 - uy * offset,
        (start.y + end.y) * 0.5 + ux * offset,
    );
    Some(json!({
        "center": [round2(center.x), round2(center.y)],
        "majorAxisEnd": [round2(center.x + radius), round2(center.y)],
        "minorAxisEnd": [round2(center.x), round2(center.y + radius)]
    }))
}

fn normalize_arrow_head_payload(extra: &mut BTreeMap<String, Value>) {
    let legacy_head = extra
        .get("head")
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let legacy_tail = extra
        .get("tail")
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let Some(Value::Object(arrow_head)) = extra.get_mut("arrowHead") else {
        return;
    };

    let length = object_number(arrow_head, "length")
        .filter(|value| *value > EPSILON)
        .unwrap_or(crate::DEFAULT_ARROW_HEAD_LENGTH_RATIO);
    arrow_head.insert("length".to_string(), json!(round2(length)));

    let center_length = object_number(arrow_head, "centerLength")
        .or_else(|| object_number(arrow_head, "center_length"))
        .filter(|value| *value > 0.0)
        .unwrap_or(length * 0.875);
    arrow_head.insert("centerLength".to_string(), json!(round2(center_length)));

    let width = object_number(arrow_head, "width")
        .filter(|value| *value >= 0.0)
        .unwrap_or(length * 0.25);
    arrow_head.insert("width".to_string(), json!(round2(width)));

    let kind = arrow_head
        .get("kind")
        .and_then(Value::as_str)
        .map(canonical_arrow_head_kind)
        .unwrap_or("solid");
    arrow_head.insert("kind".to_string(), json!(kind));

    let curve = object_number(arrow_head, "curve").unwrap_or(0.0);
    arrow_head.insert("curve".to_string(), json!(round2(curve)));

    let head = arrow_head
        .get("head")
        .and_then(Value::as_str)
        .map(canonical_arrow_endpoint_payload)
        .unwrap_or_else(|| canonical_legacy_arrow_endpoint(legacy_head.as_deref(), "end"));
    arrow_head.insert("head".to_string(), json!(head));

    let tail = arrow_head
        .get("tail")
        .and_then(Value::as_str)
        .map(canonical_arrow_endpoint_payload)
        .unwrap_or_else(|| canonical_legacy_arrow_endpoint(legacy_tail.as_deref(), "start"));
    arrow_head.insert("tail".to_string(), json!(tail));

    let bold = arrow_head
        .get("bold")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    arrow_head.insert("bold".to_string(), json!(bold));

    let no_go = arrow_head
        .get("noGo")
        .or_else(|| arrow_head.get("no_go"))
        .and_then(Value::as_str)
        .map(canonical_arrow_no_go)
        .unwrap_or("none");
    arrow_head.insert("noGo".to_string(), json!(no_go));

    let curve_spacing = object_number(arrow_head, "curveSpacing")
        .or_else(|| object_number(arrow_head, "curve_spacing"))
        .filter(|value| *value >= 0.0);
    if let Some(curve_spacing) = curve_spacing {
        arrow_head.insert("curveSpacing".to_string(), json!(round2(curve_spacing)));
    } else {
        arrow_head.remove("curveSpacing");
    }
    arrow_head.remove("curve_spacing");

    for key in ["dipole", "closed"] {
        let enabled = arrow_head
            .get(key)
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if enabled {
            arrow_head.insert(key.to_string(), json!(true));
        } else {
            arrow_head.remove(key);
        }
    }

    let explicit_shaft_spacing = object_number(arrow_head, "shaftSpacing")
        .or_else(|| object_number(arrow_head, "shaft_spacing"))
        .filter(|value| *value >= 0.0);
    arrow_head.remove("shaft_spacing");
    if matches!(kind, "equilibrium" | "unequal-equilibrium") {
        let shaft_spacing = explicit_shaft_spacing
            .filter(|value| *value > 0.0)
            .unwrap_or(3.0);
        arrow_head.insert("shaftSpacing".to_string(), json!(round2(shaft_spacing)));
        if kind == "unequal-equilibrium" {
            let ratio = object_number(arrow_head, "equilibriumRatio")
                .or_else(|| object_number(arrow_head, "equilibrium_ratio"))
                .filter(|value| *value > 1.0)
                .unwrap_or(3.0);
            arrow_head.insert("equilibriumRatio".to_string(), json!(round2(ratio)));
        } else {
            arrow_head.remove("equilibriumRatio");
            arrow_head.remove("equilibrium_ratio");
        }
    } else if let Some(shaft_spacing) = explicit_shaft_spacing {
        arrow_head.insert("shaftSpacing".to_string(), json!(round2(shaft_spacing)));
    } else {
        arrow_head.remove("shaftSpacing");
    }
}

fn object_number(object: &Map<String, Value>, key: &str) -> Option<f64> {
    object.get(key)?.as_f64().filter(|value| value.is_finite())
}

fn canonical_arrow_head_kind(value: &str) -> &'static str {
    match value.to_ascii_lowercase().as_str() {
        "hollow" => "hollow",
        "angle" | "open" | "retrosynthetic" => "open",
        "equilibrium" => "equilibrium",
        "unequal-equilibrium" | "unequilibrium" | "unbalanced-equilibrium" => "unequal-equilibrium",
        _ => "solid",
    }
}

fn canonical_arrow_endpoint_payload(value: &str) -> &'static str {
    match value.to_ascii_lowercase().as_str() {
        "full" => "full",
        "half-left" | "halfleft" | "left" | "top" => "half-left",
        "half-right" | "halfright" | "right" | "bottom" => "half-right",
        _ => "none",
    }
}

fn canonical_legacy_arrow_endpoint(value: Option<&str>, enabled: &str) -> &'static str {
    if value.is_some_and(|value| {
        value.eq_ignore_ascii_case(enabled) || value.eq_ignore_ascii_case("both")
    }) {
        "full"
    } else {
        "none"
    }
}

fn canonical_arrow_no_go(value: &str) -> &'static str {
    match value.to_ascii_lowercase().as_str() {
        "cross" => "cross",
        "hash" => "hash",
        _ => "none",
    }
}

pub(crate) fn arrow_payload_line_endpoints(
    extra: &BTreeMap<String, Value>,
) -> Option<(Point, Point)> {
    let points = extra.get("points")?.as_array()?;
    let start = points.first()?.as_array()?;
    let end = points.get(1)?.as_array()?;
    Some((
        Point::new(start.first()?.as_f64()?, start.get(1)?.as_f64()?),
        Point::new(end.first()?.as_f64()?, end.get(1)?.as_f64()?),
    ))
}

fn arrow_payload_curve(extra: &BTreeMap<String, Value>) -> f64 {
    extra
        .get("arrowHead")
        .and_then(|value| value.get("curve"))
        .and_then(Value::as_f64)
        .unwrap_or(0.0)
}

fn arrow_payload_geometry_is_valid(extra: &BTreeMap<String, Value>) -> bool {
    ["center", "majorAxisEnd", "minorAxisEnd"]
        .into_iter()
        .all(|key| {
            extra
                .get("arrowGeometry")
                .and_then(|geometry| geometry.get(key))
                .and_then(Value::as_array)
                .is_some_and(|coords| {
                    coords.first().and_then(Value::as_f64).is_some()
                        && coords.get(1).and_then(Value::as_f64).is_some()
                })
        })
}

pub(crate) fn normalize_text_object_payloads(document: &mut ChemSemaDocument) {
    let styles = document.styles.clone();
    for object in &mut document.objects {
        if object.object_type != "text" {
            continue;
        }
        normalize_text_object_payload_defaults(object, &styles);
        let align = object
            .payload
            .extra
            .get("align")
            .and_then(Value::as_str)
            .unwrap_or("left");
        let anchor_x = match align {
            "center" => 0.5,
            "right" => 1.0,
            _ => continue,
        };
        let Some(mut box_value) = object
            .payload
            .extra
            .get("box")
            .cloned()
            .and_then(|value| serde_json::from_value::<[f64; 4]>(value).ok())
        else {
            continue;
        };
        if box_value[0].abs() > crate::EPSILON
            || box_value[2] <= crate::EPSILON
            || !box_value.iter().all(|value| value.is_finite())
        {
            continue;
        }
        box_value[0] = round2(-box_value[2] * anchor_x);
        object
            .payload
            .extra
            .insert("box".to_string(), json!(box_value));
        if let Some(bbox) = object.payload.bbox.as_mut() {
            if bbox[0].abs() <= crate::EPSILON
                && (bbox[2] - box_value[2]).abs() <= crate::EPSILON
                && bbox.iter().all(|value| value.is_finite())
            {
                bbox[0] = box_value[0];
            }
        }
    }
}

fn normalize_text_object_payload_defaults(
    object: &mut SceneObject,
    styles: &BTreeMap<String, Value>,
) {
    let style = object
        .style_ref
        .as_ref()
        .and_then(|style_ref| styles.get(style_ref));
    let font_size = object
        .payload
        .extra
        .get("fontSize")
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite() && *value > 0.0)
        .or_else(|| style.and_then(|style| style_number(style, "fontSize")))
        .or_else(|| style.and_then(|style| style_number(style, "font_size")))
        .unwrap_or(DEFAULT_TEXT_FONT_SIZE_PT);
    object
        .payload
        .extra
        .insert("fontSize".to_string(), json!(round2(font_size)));

    let line_height = object
        .payload
        .extra
        .get("lineHeight")
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite() && *value > 0.0)
        .unwrap_or(DEFAULT_TEXT_LINE_HEIGHT_PT);
    object
        .payload
        .extra
        .insert("lineHeight".to_string(), json!(round2(line_height)));
    let line_height_mode = object
        .payload
        .extra
        .get("lineHeightMode")
        .and_then(Value::as_str)
        .filter(|mode| matches!(*mode, "fixed" | "auto" | "variable"))
        .unwrap_or("auto");
    object
        .payload
        .extra
        .insert("lineHeightMode".to_string(), json!(line_height_mode));

    object
        .payload
        .extra
        .entry("align".to_string())
        .or_insert_with(|| json!("left"));
    object
        .payload
        .extra
        .entry("preserveLines".to_string())
        .or_insert_with(|| json!(false));

    if text_payload_box(&object.payload.extra).is_none() {
        let text = object
            .payload
            .extra
            .get("text")
            .and_then(Value::as_str)
            .unwrap_or("");
        let box_value = object
            .payload
            .bbox
            .filter(valid_bbox)
            .unwrap_or_else(|| default_text_object_box(text, font_size, line_height));
        object.payload.extra.insert(
            "box".to_string(),
            json!([
                round2(box_value[0]),
                round2(box_value[1]),
                round2(box_value[2]),
                round2(box_value[3])
            ]),
        );
    }
}

fn style_number(style: &Value, key: &str) -> Option<f64> {
    style
        .get(key)?
        .as_f64()
        .filter(|value| value.is_finite() && *value > 0.0)
}

fn text_payload_box(extra: &BTreeMap<String, Value>) -> Option<[f64; 4]> {
    let value = extra.get("box")?;
    serde_json::from_value::<[f64; 4]>(value.clone())
        .ok()
        .filter(valid_bbox)
}

fn default_text_object_box(text: &str, font_size: f64, line_height: f64) -> [f64; 4] {
    let max_chars = text
        .lines()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0) as f64;
    let line_count = text.lines().count().max(1) as f64;
    [
        0.0,
        0.0,
        (max_chars * font_size * 0.55).max(font_size),
        (line_count * line_height).max(font_size),
    ]
}

fn valid_bbox(bbox: &[f64; 4]) -> bool {
    bbox.iter().all(|value| value.is_finite())
}

pub(crate) fn normalize_shape_object_payloads(document: &mut ChemSemaDocument) {
    for object in &mut document.objects {
        if object.object_type != "shape" {
            continue;
        }
        let kind = object
            .payload
            .extra
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or("rect")
            .to_string();
        if !matches!(kind.as_str(), "circle" | "ellipse") {
            continue;
        }
        if shape_oval_geometry_is_valid(&object.payload.extra) {
            continue;
        }
        let Some([x, y, width, height]) = object.payload.bbox.filter(valid_bbox) else {
            continue;
        };
        let center = Point::new(
            object.transform.translate[0] + x + width * 0.5,
            object.transform.translate[1] + y + height * 0.5,
        );
        let major = Point::new(center.x + width.abs() * 0.5, center.y);
        let minor_radius = if height.abs() > EPSILON {
            height.abs() * 0.5
        } else {
            width.abs() * 0.5
        };
        let minor = Point::new(center.x, center.y + minor_radius);
        object.payload.extra.insert(
            "center".to_string(),
            json!([round2(center.x), round2(center.y)]),
        );
        object.payload.extra.insert(
            "majorAxisEnd".to_string(),
            json!([round2(major.x), round2(major.y)]),
        );
        object.payload.extra.insert(
            "minorAxisEnd".to_string(),
            json!([round2(minor.x), round2(minor.y)]),
        );
    }
}

fn shape_oval_geometry_is_valid(extra: &BTreeMap<String, Value>) -> bool {
    ["center", "majorAxisEnd", "minorAxisEnd"]
        .into_iter()
        .all(|key| {
            extra
                .get(key)
                .and_then(Value::as_array)
                .is_some_and(|coords| {
                    coords.first().and_then(Value::as_f64).is_some()
                        && coords.get(1).and_then(Value::as_f64).is_some()
                })
        })
}

pub(crate) fn normalize_fragment_label_payloads(document: &mut ChemSemaDocument) {
    let margin_width = document
        .style
        .defaults
        .get("marginWidth")
        .copied()
        .filter(|value| value.is_finite() && *value > EPSILON)
        .unwrap_or(crate::DEFAULT_BOND_MARGIN_WIDTH_PT.value());
    let line_width = document
        .style
        .defaults
        .get("lineWidth")
        .copied()
        .filter(|value| value.is_finite() && *value > EPSILON)
        .unwrap_or(crate::DEFAULT_BOND_STROKE);
    let glyph_clip_profile = crate::GlyphClipProfile::from_margin_width(margin_width);
    for resource in document.resources.values_mut() {
        let Some(fragment) = resource.data.as_fragment_mut() else {
            continue;
        };
        let node_ids: Vec<String> = fragment
            .nodes
            .iter()
            .filter(|node| node.label.is_some())
            .map(|node| node.id.clone())
            .collect();
        for node_id in node_ids {
            crate::engine::refresh_attached_node_label_geometry_for_node_without_implicit_hydrogen_refresh(
                fragment,
                [0.0, 0.0],
                &node_id,
                line_width,
                Some(glyph_clip_profile),
            );
        }
        for node in &mut fragment.nodes {
            if let Some(label) = &mut node.label {
                normalize_node_label_payload(label, node.position, glyph_clip_profile);
            }
        }
    }
}

fn normalize_node_label_payload(
    label: &mut NodeLabel,
    node_position: [f64; 2],
    glyph_clip_profile: crate::GlyphClipProfile,
) {
    if label.position.is_none() {
        label.position = Some(node_position);
    }
    if label.font_size.is_none() {
        label.font_size = Some(DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT);
    }
    let default_font_size = label
        .font_size
        .unwrap_or(DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT);
    if label
        .line_height
        .is_none_or(|value| !value.is_finite() || value <= 0.0)
    {
        label.line_height = Some(crate::molecule_label_line_advance(default_font_size));
    }
    if !matches!(
        label.line_height_mode.as_str(),
        "fixed" | "auto" | "variable"
    ) {
        label.line_height_mode = "variable".to_string();
    }
    label
        .line_advances
        .retain(|value| value.is_finite() && *value > 0.0);
    if label.align.is_none() {
        label.align = Some("left".to_string());
    }
    if label.font_family.is_none() {
        label.font_family = Some("Arial".to_string());
    }
    if label.fill.is_none() {
        label.fill = Some("#000000".to_string());
    }
    if label.runs.is_empty() && label.line_runs.is_empty() && label.has_visible_text() {
        label.runs.push(LabelRun {
            text: label.text.clone(),
            font_family: label.font_family.clone(),
            font_size: label.font_size,
            fill: label.fill.clone(),
            font_weight: Some(400),
            font_style: Some("normal".to_string()),
            underline: Some(false),
            outline: Some(false),
            shadow: Some(false),
            script: Some("normal".to_string()),
        });
    }
    if label.box_value.is_none() && label.box_field.is_none() {
        let font_size = label
            .font_size
            .unwrap_or(DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT);
        let position = label.position.unwrap_or(node_position);
        label.box_field = Some(default_node_label_box(position, &label.text, font_size));
    }
    rebuild_node_label_glyph_polygons(label, node_position, glyph_clip_profile);
}

fn rebuild_node_label_glyph_polygons(
    label: &mut NodeLabel,
    node_position: [f64; 2],
    glyph_clip_profile: crate::GlyphClipProfile,
) {
    if !label.has_visible_text() {
        label.glyph_polygons.clear();
        label.glyph_clip_polygons.clear();
        return;
    }

    let position = label.position.unwrap_or(node_position);
    let font_size = label
        .font_size
        .unwrap_or(DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT);
    let local_bbox = label.bbox();
    let align = label.align.as_deref().unwrap_or("left");
    let line_runs = if label.line_runs.is_empty() {
        &[][..]
    } else {
        label.line_runs.as_slice()
    };
    let single_line_runs = if line_runs.is_empty() {
        label.runs.as_slice()
    } else {
        &[][..]
    };

    let start_position = if align == "center" {
        let width = local_bbox
            .map(|bbox| (bbox[2] - bbox[0]).abs())
            .filter(|width| *width > EPSILON)
            .unwrap_or_else(|| {
                (label.text.chars().count() as f64 * font_size * 0.55).max(font_size)
            });
        [round2(position[0] - width * 0.5), position[1]]
    } else if align == "right" {
        let width = local_bbox
            .map(|bbox| (bbox[2] - bbox[0]).abs())
            .filter(|width| *width > EPSILON)
            .unwrap_or_else(|| {
                (label.text.chars().count() as f64 * font_size * 0.55).max(font_size)
            });
        [round2(position[0] - width), position[1]]
    } else {
        position
    };
    let geometry = crate::build_label_glyph_geometry_with_profile(
        single_line_runs,
        line_runs,
        start_position,
        local_bbox,
        font_size,
        label
            .line_height
            .unwrap_or_else(|| crate::molecule_label_line_advance(font_size)),
        &label.line_advances,
        node_position,
        glyph_clip_profile,
    );
    label.glyph_polygons = geometry.glyph_polygons;
    label.glyph_clip_polygons = geometry.clip_polygons;
}

fn default_node_label_box(position: [f64; 2], text: &str, font_size: f64) -> [f64; 4] {
    let line_count = text.lines().count().max(1) as f64;
    let max_chars = text
        .lines()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0) as f64;
    let width = (max_chars * font_size * 0.58).max(font_size);
    let height = (line_count * font_size * 1.25).max(font_size);
    [
        round2(position[0]),
        round2(position[1] - font_size),
        round2(position[0] + width),
        round2(position[1] - font_size + height),
    ]
}

fn ensure_document_json_pt_unit(value: &mut Value) -> Result<(), String> {
    if !value.is_object() {
        return Ok(());
    }
    let Some(format) = value.get_mut("format").and_then(Value::as_object_mut) else {
        return Ok(());
    };
    if let Some(unit) = format.get("unit").and_then(Value::as_str) {
        if unit.eq_ignore_ascii_case("pt") {
            return Ok(());
        }
        return Err(format!(
            "Unsupported chemsema document unit '{unit}'. Current development files must use pt."
        ));
    }
    format.insert("unit".to_string(), Value::String("pt".to_string()));
    Ok(())
}

fn default_format_unit() -> String {
    "pt".to_string()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormatInfo {
    pub name: String,
    pub version: String,
    #[serde(default = "default_format_unit")]
    pub unit: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentInfo {
    pub id: String,
    pub title: String,
    pub page: Page,
    /// Source-independent paper, pagination, header/footer, view, and
    /// in-place-embedding settings. `page` remains the infinite-canvas working
    /// extent; `layout.paper` is the physical sheet used by page preview,
    /// printing, and paged exports.
    #[serde(default)]
    pub layout: DocumentLayout,
    #[serde(default)]
    pub meta: Value,
}

const fn default_paper_width() -> f64 {
    595.275_590_551
}

const fn default_paper_height() -> f64 {
    841.889_763_78
}

const fn default_page_margin() -> f64 {
    36.0
}

const fn default_header_position() -> f64 {
    36.0
}

const fn default_footer_position() -> f64 {
    36.0
}

const fn default_magnification_percent() -> f64 {
    100.0
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DrawingSpace {
    #[default]
    Pages,
    Poster,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PageDefinition {
    #[default]
    Undefined,
    Center,
    Tl4,
    IdTerm,
    FlushLeft,
    FlushRight,
    Reaction1,
    Reaction2,
    MulticolumnTl4,
    MulticolumnNonTl4,
    UserDefined,
}

impl PageDefinition {
    pub(crate) fn from_cdxml(value: Option<&str>) -> Result<Self, String> {
        let value = value.unwrap_or("Undefined");
        let definition = match value.trim().to_ascii_lowercase().as_str() {
            "center" | "1" => Self::Center,
            "tl4" | "2" => Self::Tl4,
            "idterm" | "3" => Self::IdTerm,
            "flushleft" | "4" => Self::FlushLeft,
            "flushright" | "5" => Self::FlushRight,
            "reaction1" | "6" => Self::Reaction1,
            "reaction2" | "7" => Self::Reaction2,
            "multicolumntl4" | "8" => Self::MulticolumnTl4,
            "multicolumnnontl4" | "9" => Self::MulticolumnNonTl4,
            "userdefined" | "10" => Self::UserDefined,
            "undefined" | "0" => Self::Undefined,
            _ => return Err(format!("unsupported PageDefinition '{value}'")),
        };
        Ok(definition)
    }

    pub(crate) const fn as_cdxml(self) -> &'static str {
        match self {
            Self::Undefined => "Undefined",
            Self::Center => "Center",
            Self::Tl4 => "TL4",
            Self::IdTerm => "IDTerm",
            Self::FlushLeft => "FlushLeft",
            Self::FlushRight => "FlushRight",
            Self::Reaction1 => "Reaction1",
            Self::Reaction2 => "Reaction2",
            Self::MulticolumnTl4 => "MulticolumnTL4",
            Self::MulticolumnNonTl4 => "MulticolumnNonTL4",
            Self::UserDefined => "UserDefined",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageSplitter {
    pub id: String,
    #[serde(default)]
    pub position: Option<[f64; 2]>,
    #[serde(default)]
    pub page_definition: PageDefinition,
}

fn deserialize_legacy_splitter_position_ids<'de, D>(
    deserializer: D,
) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let values = Vec::<Value>::deserialize(deserializer)?;
    values
        .into_iter()
        .map(|value| match value {
            Value::String(value) if !value.trim().is_empty() => Ok(value),
            Value::Number(value) => Ok(value.to_string()),
            _ => Err(serde::de::Error::custom(
                "legacy splitter position IDs must be strings or numbers",
            )),
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaperSize {
    #[serde(default = "default_paper_width")]
    pub width: f64,
    #[serde(default = "default_paper_height")]
    pub height: f64,
}

impl Default for PaperSize {
    fn default() -> Self {
        Self {
            width: default_paper_width(),
            height: default_paper_height(),
        }
    }
}

/// Physical document layout. UI-only infinite/paper preview state deliberately
/// does not live here: changing the preview must not dirty the document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentLayout {
    #[serde(default)]
    pub drawing_space: DrawingSpace,
    #[serde(default)]
    pub paper: PaperSize,
    /// Page count across and down. In Pages mode each tile is a logical page;
    /// in Poster mode these values describe the physical sheets needed to tile
    /// one drawing space.
    #[serde(default = "default_one_u16")]
    pub width_pages: u16,
    #[serde(default = "default_one_u16")]
    pub height_pages: u16,
    /// When true the configured page counts are minimums. The resolved layout
    /// adds sheets until every visible document primitive is covered.
    #[serde(default = "default_true")]
    pub auto_paginate: bool,
    /// Document coordinates of the original page grid's top-left. It is
    /// established by centering on first layout/import, then retained. When
    /// edits extend above or left of it, resolved pages are prepended while
    /// this original page remains fixed in document space.
    #[serde(default)]
    pub page_origin: Option<[f64; 2]>,
    /// Top, right, bottom, left physical print margins in document points.
    #[serde(default = "default_page_margins")]
    pub margins: [f64; 4],
    #[serde(default)]
    pub page_overlap: f64,
    #[serde(default)]
    pub print_trim_marks: bool,
    #[serde(default)]
    pub header: String,
    #[serde(default = "default_header_position")]
    pub header_position: f64,
    #[serde(default)]
    pub footer: String,
    #[serde(default = "default_footer_position")]
    pub footer_position: f64,
    /// Human-facing percent. CDX stores ten times this number.
    #[serde(default = "default_magnification_percent")]
    pub magnification_percent: f64,
    /// The page-level formatting definition from the official CDX enum.
    #[serde(default)]
    pub page_definition: PageDefinition,
    /// Native horizontal page splitters. `position` uses document coordinates;
    /// the object is logical and does not create an editor drawing primitive.
    #[serde(default)]
    pub splitters: Vec<PageSplitter>,
    /// ChemDraw 6 defined `SplitterPositions` as an object-ID array, then
    /// obsoleted it in favor of Splitter objects. Preserve those IDs exactly.
    /// The alias is an explicit migration for early ChemSema files that
    /// incorrectly serialized the IDs as numeric `splitterPositions`.
    #[serde(
        default,
        alias = "splitterPositions",
        deserialize_with = "deserialize_legacy_splitter_position_ids"
    )]
    pub legacy_splitter_position_ids: Vec<String>,
    /// OLE/in-place editing extent and gap in document points.
    #[serde(default)]
    pub fix_in_place_extent: Option<[f64; 2]>,
    #[serde(default)]
    pub fix_in_place_gap: Option<[f64; 2]>,
}

const fn default_one_u16() -> u16 {
    1
}

const fn default_page_margins() -> [f64; 4] {
    [
        default_page_margin(),
        default_page_margin(),
        default_page_margin(),
        default_page_margin(),
    ]
}

impl Default for DocumentLayout {
    fn default() -> Self {
        Self {
            drawing_space: DrawingSpace::Pages,
            paper: PaperSize::default(),
            width_pages: 1,
            height_pages: 1,
            auto_paginate: true,
            page_origin: None,
            margins: default_page_margins(),
            page_overlap: 0.0,
            print_trim_marks: false,
            header: String::new(),
            header_position: default_header_position(),
            footer: String::new(),
            footer_position: default_footer_position(),
            magnification_percent: default_magnification_percent(),
            page_definition: PageDefinition::Undefined,
            splitters: Vec::new(),
            legacy_splitter_position_ids: Vec::new(),
            fix_in_place_extent: None,
            fix_in_place_gap: None,
        }
    }
}

impl DocumentLayout {
    pub fn validate(&self) -> Result<(), String> {
        if !self.paper.width.is_finite() || self.paper.width <= 0.0 {
            return Err("document paper width must be a positive finite value".to_string());
        }
        if !self.paper.height.is_finite() || self.paper.height <= 0.0 {
            return Err("document paper height must be a positive finite value".to_string());
        }
        if self.width_pages == 0 || self.height_pages == 0 {
            return Err("document page counts must be at least one".to_string());
        }
        if self
            .page_origin
            .is_some_and(|origin| origin.iter().any(|coordinate| !coordinate.is_finite()))
        {
            return Err("document page origin must contain finite coordinates".to_string());
        }
        if self.width_pages > 256 || self.height_pages > 256 {
            return Err("document page counts cannot exceed 256 in either direction".to_string());
        }
        if self
            .margins
            .iter()
            .any(|value| !value.is_finite() || *value < 0.0)
        {
            return Err("document margins must be finite and non-negative".to_string());
        }
        if self.margins[1] + self.margins[3] >= self.paper.width
            || self.margins[0] + self.margins[2] >= self.paper.height
        {
            return Err("document margins must leave a positive printable area".to_string());
        }
        if !self.page_overlap.is_finite()
            || self.page_overlap < 0.0
            || self.page_overlap >= self.paper.width.min(self.paper.height)
        {
            return Err(
                "poster page overlap must be finite, non-negative, and smaller than the paper"
                    .to_string(),
            );
        }
        for (name, value) in [
            ("header position", self.header_position),
            ("footer position", self.footer_position),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(format!("document {name} must be finite and non-negative"));
            }
        }
        if !self.magnification_percent.is_finite()
            || !(1.0..=999.0).contains(&self.magnification_percent)
        {
            return Err("document magnification must be between 1% and 999%".to_string());
        }
        let mut splitter_ids = BTreeSet::new();
        for splitter in &self.splitters {
            if splitter.id.trim().is_empty() || !splitter_ids.insert(splitter.id.as_str()) {
                return Err("document splitter IDs must be non-empty and unique".to_string());
            }
            if splitter
                .position
                .is_some_and(|point| point.into_iter().any(|value| !value.is_finite()))
            {
                return Err(format!(
                    "document splitter '{}' has a non-finite position",
                    splitter.id
                ));
            }
        }
        let mut legacy_splitter_ids = BTreeSet::new();
        if self
            .legacy_splitter_position_ids
            .iter()
            .any(|id| id.trim().is_empty() || !legacy_splitter_ids.insert(id.as_str()))
        {
            return Err("legacy splitter position IDs must be non-empty and unique".to_string());
        }
        for (name, value) in [
            ("in-place extent", self.fix_in_place_extent),
            ("in-place gap", self.fix_in_place_gap),
        ] {
            if value.is_some_and(|pair| {
                pair.iter()
                    .any(|coordinate| !coordinate.is_finite() || *coordinate < 0.0)
            }) {
                return Err(format!(
                    "document {name} coordinates must be finite and non-negative"
                ));
            }
        }
        Ok(())
    }

    pub fn total_width(&self) -> f64 {
        match self.drawing_space {
            DrawingSpace::Pages => self.paper.width * f64::from(self.width_pages),
            DrawingSpace::Poster => {
                self.paper.width * f64::from(self.width_pages)
                    - self.page_overlap * f64::from(self.width_pages.saturating_sub(1))
            }
        }
    }

    pub fn total_height(&self) -> f64 {
        match self.drawing_space {
            DrawingSpace::Pages => self.paper.height * f64::from(self.height_pages),
            DrawingSpace::Poster => {
                self.paper.height * f64::from(self.height_pages)
                    - self.page_overlap * f64::from(self.height_pages.saturating_sub(1))
            }
        }
    }

    pub fn resolve(&self, content_bounds: Option<[f64; 4]>) -> ResolvedDocumentLayout {
        let Some([min_x, min_y, max_x, max_y]) = content_bounds else {
            let anchor_origin = self.page_origin.unwrap_or([0.0, 0.0]);
            return ResolvedDocumentLayout {
                origin: anchor_origin,
                anchor_origin,
                width_pages: self.width_pages,
                height_pages: self.height_pages,
                prepended_pages: [0, 0],
                total_width: resolved_page_span(
                    self.paper.width,
                    self.page_overlap,
                    self.width_pages,
                    self.drawing_space,
                ),
                total_height: resolved_page_span(
                    self.paper.height,
                    self.page_overlap,
                    self.height_pages,
                    self.drawing_space,
                ),
            };
        };
        let content_width = (max_x - min_x).max(0.0);
        let content_height = (max_y - min_y).max(0.0);
        let centered_width_pages = if self.auto_paginate {
            page_count_for_span(
                content_width,
                self.paper.width,
                self.page_overlap,
                self.drawing_space,
            )
            .max(self.width_pages)
        } else {
            self.width_pages
        };
        let centered_height_pages = if self.auto_paginate {
            page_count_for_span(
                content_height,
                self.paper.height,
                self.page_overlap,
                self.drawing_space,
            )
            .max(self.height_pages)
        } else {
            self.height_pages
        };
        let centered_total_width = resolved_page_span(
            self.paper.width,
            self.page_overlap,
            centered_width_pages,
            self.drawing_space,
        );
        let centered_total_height = resolved_page_span(
            self.paper.height,
            self.page_overlap,
            centered_height_pages,
            self.drawing_space,
        );
        let centered_origin = [
            min_x - (centered_total_width - content_width) * 0.5,
            min_y - (centered_total_height - content_height) * 0.5,
        ];
        let anchor_origin = self.page_origin.unwrap_or(centered_origin);
        let (origin_x, width_pages, prepend_x) = resolve_pagination_axis(PaginationAxisRequest {
            content_min: min_x,
            content_max: max_x,
            anchor_origin: anchor_origin[0],
            minimum_pages: self.width_pages,
            paper_extent: self.paper.width,
            overlap: self.page_overlap,
            drawing_space: self.drawing_space,
            auto_paginate: self.auto_paginate,
        });
        let (origin_y, height_pages, prepend_y) = resolve_pagination_axis(PaginationAxisRequest {
            content_min: min_y,
            content_max: max_y,
            anchor_origin: anchor_origin[1],
            minimum_pages: self.height_pages,
            paper_extent: self.paper.height,
            overlap: self.page_overlap,
            drawing_space: self.drawing_space,
            auto_paginate: self.auto_paginate,
        });
        ResolvedDocumentLayout {
            origin: [origin_x, origin_y],
            anchor_origin,
            width_pages,
            height_pages,
            prepended_pages: [prepend_x, prepend_y],
            total_width: resolved_page_span(
                self.paper.width,
                self.page_overlap,
                width_pages,
                self.drawing_space,
            ),
            total_height: resolved_page_span(
                self.paper.height,
                self.page_overlap,
                height_pages,
                self.drawing_space,
            ),
        }
    }
}

struct PaginationAxisRequest {
    content_min: f64,
    content_max: f64,
    anchor_origin: f64,
    minimum_pages: u16,
    paper_extent: f64,
    overlap: f64,
    drawing_space: DrawingSpace,
    auto_paginate: bool,
}

fn resolve_pagination_axis(request: PaginationAxisRequest) -> (f64, u16, u16) {
    let PaginationAxisRequest {
        content_min,
        content_max,
        anchor_origin,
        minimum_pages,
        paper_extent,
        overlap,
        drawing_space,
        auto_paginate,
    } = request;
    if !auto_paginate {
        return (anchor_origin, minimum_pages, 0);
    }
    let step = match drawing_space {
        DrawingSpace::Pages => paper_extent,
        DrawingSpace::Poster => (paper_extent - overlap).max(EPSILON),
    };
    let prepend = if content_min < anchor_origin - EPSILON {
        ((anchor_origin - content_min) / step)
            .ceil()
            .clamp(0.0, 255.0) as u16
    } else {
        0
    };
    let base_end =
        anchor_origin + resolved_page_span(paper_extent, overlap, minimum_pages, drawing_space);
    let append = if content_max > base_end + EPSILON {
        ((content_max - base_end) / step).ceil().clamp(0.0, 255.0) as u16
    } else {
        0
    };
    let page_count = minimum_pages
        .saturating_add(prepend)
        .saturating_add(append)
        .clamp(1, 256);
    let retained_prepend = prepend.min(page_count.saturating_sub(minimum_pages));
    (
        anchor_origin - f64::from(retained_prepend) * step,
        page_count,
        retained_prepend,
    )
}

fn resolved_page_span(
    paper_extent: f64,
    overlap: f64,
    count: u16,
    drawing_space: DrawingSpace,
) -> f64 {
    match drawing_space {
        DrawingSpace::Pages => paper_extent * f64::from(count),
        DrawingSpace::Poster => {
            paper_extent * f64::from(count) - overlap * f64::from(count.saturating_sub(1))
        }
    }
}

fn page_count_for_span(
    span: f64,
    paper_extent: f64,
    overlap: f64,
    drawing_space: DrawingSpace,
) -> u16 {
    if span <= paper_extent {
        return 1;
    }
    let step = match drawing_space {
        DrawingSpace::Pages => paper_extent,
        DrawingSpace::Poster => (paper_extent - overlap).max(EPSILON),
    };
    let count = 1.0 + ((span - paper_extent) / step).ceil();
    count.clamp(1.0, 256.0) as u16
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedDocumentLayout {
    pub origin: [f64; 2],
    pub anchor_origin: [f64; 2],
    pub width_pages: u16,
    pub height_pages: u16,
    pub prepended_pages: [u16; 2],
    pub total_width: f64,
    pub total_height: f64,
}

fn default_document_style_preset() -> String {
    "default".to_string()
}

fn default_document_style_defaults() -> BTreeMap<String, f64> {
    BTreeMap::from([
        ("bondLength".to_string(), DEFAULT_BOND_LENGTH_PT),
        ("chainAngle".to_string(), 120.0),
        ("lineWidth".to_string(), DEFAULT_BOND_STROKE_PT),
        ("boldWidth".to_string(), crate::BOLD_BOND_WIDTH_PT.value()),
        (
            "wedgeWidth".to_string(),
            crate::SOLID_WEDGE_WIDTH_PT.value(),
        ),
        (
            "hashSpacing".to_string(),
            crate::DEFAULT_HASH_SPACING_PT.value(),
        ),
        (
            "bondSpacing".to_string(),
            crate::DEFAULT_BOND_SPACING_PERCENT,
        ),
        (
            "marginWidth".to_string(),
            crate::DEFAULT_BOND_MARGIN_WIDTH_PT.value(),
        ),
        ("graphicLineWidth".to_string(), DEFAULT_BOND_STROKE_PT),
    ])
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentTextStyle {
    pub font_family: String,
    pub font_size: f64,
    pub fill: String,
    pub font_weight: u32,
    pub font_style: String,
    pub underline: bool,
    #[serde(default)]
    pub outline: bool,
    #[serde(default)]
    pub shadow: bool,
    pub script: String,
    /// Resolved default baseline advance in document points.
    #[serde(default = "default_document_text_line_height")]
    pub line_height: f64,
    /// Source-independent behavior used when new multi-line text is created.
    #[serde(default = "default_document_text_line_height_mode")]
    pub line_height_mode: String,
}

fn default_document_text_line_height() -> f64 {
    DEFAULT_TEXT_FONT_SIZE_PT * 1.15
}

fn default_document_text_line_height_mode() -> String {
    "auto".to_string()
}

impl DocumentTextStyle {
    fn molecule_label_default() -> Self {
        Self {
            font_family: "Arial".to_string(),
            font_size: DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT,
            fill: "#000000".to_string(),
            font_weight: 400,
            font_style: "normal".to_string(),
            underline: false,
            outline: false,
            shadow: false,
            script: "chemical".to_string(),
            line_height: crate::molecule_label_line_advance(DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT),
            line_height_mode: "variable".to_string(),
        }
    }

    fn caption_default() -> Self {
        Self {
            font_family: "Arial".to_string(),
            font_size: DEFAULT_TEXT_FONT_SIZE_PT,
            fill: "#000000".to_string(),
            font_weight: 400,
            font_style: "normal".to_string(),
            underline: false,
            outline: false,
            shadow: false,
            script: "normal".to_string(),
            line_height: DEFAULT_TEXT_FONT_SIZE_PT * 1.15,
            line_height_mode: "auto".to_string(),
        }
    }
}

fn default_document_label_style() -> DocumentTextStyle {
    DocumentTextStyle::molecule_label_default()
}

fn default_document_caption_style() -> DocumentTextStyle {
    DocumentTextStyle::caption_default()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentStyleInfo {
    #[serde(default = "default_document_style_preset")]
    pub preset: String,
    #[serde(default)]
    pub defaults: BTreeMap<String, f64>,
    #[serde(default = "default_document_label_style")]
    pub label_style: DocumentTextStyle,
    #[serde(default = "default_document_caption_style")]
    pub caption_style: DocumentTextStyle,
}

impl Default for DocumentStyleInfo {
    fn default() -> Self {
        Self {
            preset: default_document_style_preset(),
            defaults: default_document_style_defaults(),
            label_style: default_document_label_style(),
            caption_style: default_document_caption_style(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Page {
    pub width: f64,
    pub height: f64,
    pub background: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneObject {
    pub id: String,
    #[serde(rename = "type")]
    pub object_type: String,
    #[serde(default)]
    pub name: String,
    #[serde(default = "default_true")]
    pub visible: bool,
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub z_index: i32,
    #[serde(default)]
    pub transform: Transform,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_ref: Option<String>,
    #[serde(default)]
    pub link_policy: LinkPolicy,
    #[serde(default)]
    pub meta: Value,
    #[serde(default)]
    pub payload: ObjectPayload,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<SceneObject>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LinkPolicy {
    #[default]
    Auto,
    Linked,
    Unlinked,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkRelation {
    pub id: String,
    pub kind: String,
    pub endpoints: Vec<LinkEndpoint>,
    #[serde(default)]
    pub data: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkEndpoint {
    pub entity_id: String,
    pub role: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChemicalProperty {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    pub property_type: ChemicalPropertyType,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub basis_entity_ids: Vec<String>,
    /// Source object ids that could not be mapped to a native scene, atom, or
    /// bond entity. They remain explicit so editing a partially-supported
    /// document never silently drops the official BasisObjects entries.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unresolved_basis_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_object_id: Option<String>,
    #[serde(default)]
    pub is_active: bool,
    #[serde(default)]
    pub value_origin: ChemicalPropertyValueOrigin,
    #[serde(default)]
    pub calculation_state: ChemicalPropertyCalculationState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_calculated_value: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChemicalPropertyType {
    /// `None` is the official "property absent/undefined" state, which is
    /// different from the explicit Unspecified value 0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl ChemicalPropertyType {
    pub fn undefined() -> Self {
        Self::default()
    }

    pub fn unspecified() -> Self {
        Self {
            code: Some(0),
            name: Some("Unspecified".to_string()),
        }
    }

    pub fn chemical_name() -> Self {
        Self {
            code: Some(1),
            name: Some("ChemicalName".to_string()),
        }
    }

    pub fn is_chemical_name(&self) -> bool {
        self.code == Some(1) || self.name.as_deref() == Some("ChemicalName")
    }

    pub fn cdxml_value(&self) -> Option<String> {
        self.code
            .map(|code| code.to_string())
            .or_else(|| self.name.clone())
    }

    fn validate(&self) -> Result<(), String> {
        if self.code == Some(0)
            && self
                .name
                .as_deref()
                .is_some_and(|name| name != "Unspecified")
        {
            return Err("chemical property type code 0 must be named Unspecified".to_string());
        }
        if self.code == Some(1)
            && self
                .name
                .as_deref()
                .is_some_and(|name| name != "ChemicalName")
        {
            return Err("chemical property type code 1 must be named ChemicalName".to_string());
        }
        if self.code.is_none() && self.name.as_deref().is_some_and(str::is_empty) {
            return Err("chemical property custom type name cannot be empty".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ChemicalPropertyValueOrigin {
    #[default]
    Imported,
    Authored,
    Calculated,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ChemicalPropertyCalculationState {
    #[default]
    Static,
    Current,
    Stale,
    Unsupported,
}

impl SceneObject {
    pub fn kind(&self) -> crate::SceneObjectKind {
        crate::SceneObjectKind::parse(&self.object_type)
            .unwrap_or_else(|error| panic!("{error} on validated object '{}'", self.id))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Transform {
    pub translate: [f64; 2],
    pub rotate: f64,
    pub scale: [f64; 2],
}

impl Transform {
    pub const fn identity() -> Self {
        Self {
            translate: [0.0, 0.0],
            rotate: 0.0,
            scale: [1.0, 1.0],
        }
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::identity()
    }
}

/// Native table geometry and formatting.
///
/// Coordinates are local to the owning scene object.  Row and column guides
/// include both outer edges, so their lengths are `rows + 1` and
/// `columns + 1`.  Borders are stored per cell because CDXML does the same:
/// the two cells adjacent to a shared edge may each carry an explicit border.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TableData {
    pub rows: usize,
    pub columns: usize,
    pub row_guides: Vec<f64>,
    pub column_guides: Vec<f64>,
    pub cells: Vec<TableCell>,
    pub default_border: TableBorder,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TableCell {
    pub id: String,
    pub row: usize,
    pub column: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub content_object_ids: Vec<String>,
    #[serde(default)]
    pub borders: TableCellBorders,
    #[serde(default)]
    pub horizontal_alignment: TableHorizontalAlignment,
    #[serde(default)]
    pub vertical_alignment: TableVerticalAlignment,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TableCellBorders {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top: Option<TableBorder>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub left: Option<TableBorder>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bottom: Option<TableBorder>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub right: Option<TableBorder>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TableBorder {
    pub visible: bool,
    pub line_style: TableLineStyle,
    pub width: f64,
    pub color: String,
}

impl Default for TableBorder {
    fn default() -> Self {
        Self {
            visible: true,
            line_style: TableLineStyle::Solid,
            width: 0.75,
            color: "#000000".to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum TableLineStyle {
    #[default]
    Solid,
    Dashed,
    Bold,
    Wavy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum TableHorizontalAlignment {
    #[default]
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum TableVerticalAlignment {
    Top,
    #[default]
    Middle,
    Bottom,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ObjectPayload {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bbox: Option<[f64; 4]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spectrum: Option<crate::SpectrumData>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geometry: Option<GeometryData>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub constraint: Option<ConstraintData>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table: Option<TableData>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stoichiometry_grid: Option<crate::StoichiometryGridData>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gel_electrophoresis: Option<crate::GelElectrophoresisData>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plasmid_map: Option<crate::PlasmidMapData>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bio_shape: Option<crate::BioShapeData>,
    #[serde(flatten, default)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageCropRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl ImageCropRect {
    pub fn validate(self, pixel_width: u32, pixel_height: u32) -> Result<(), String> {
        if ![self.x, self.y, self.width, self.height]
            .into_iter()
            .all(f64::is_finite)
            || [self.x, self.y, self.width, self.height]
                .into_iter()
                .any(|value| value.fract().abs() > crate::EPSILON)
            || self.x < 0.0
            || self.y < 0.0
            || self.width <= crate::EPSILON
            || self.height <= crate::EPSILON
            || self.x + self.width > f64::from(pixel_width) + crate::EPSILON
            || self.y + self.height > f64::from(pixel_height) + crate::EPSILON
        {
            return Err(format!(
                "imageCrop must be an integer positive resource-pixel rectangle inside {pixel_width}x{pixel_height}"
            ));
        }
        Ok(())
    }
}

impl ObjectPayload {
    pub fn image_crop(&self) -> Result<Option<ImageCropRect>, String> {
        self.extra
            .get("imageCrop")
            .map(|value| {
                serde_json::from_value(value.clone())
                    .map_err(|error| format!("invalid imageCrop: {error}"))
            })
            .transpose()
    }

    pub fn set_image_crop(&mut self, crop: Option<ImageCropRect>) {
        if let Some(crop) = crop {
            self.extra.insert(
                "imageCrop".to_string(),
                serde_json::to_value(crop).expect("serialize image crop"),
            );
        } else {
            self.extra.remove("imageCrop");
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Resource {
    #[serde(rename = "type")]
    pub resource_type: String,
    pub encoding: String,
    pub data: ResourceData,
    #[serde(default)]
    pub meta: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ResourceData {
    Fragment(MoleculeFragment),
    Text(String),
    Json(Value),
}

impl<'de> Deserialize<'de> for ResourceData {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        if let Value::String(text) = value {
            return Ok(Self::Text(text));
        }
        let is_fragment = value.as_object().is_some_and(|object| {
            object
                .get("schema")
                .and_then(Value::as_str)
                .is_some_and(|schema| schema.starts_with("chemsema.molecule.fragment"))
                || (object.get("nodes").is_some_and(Value::is_array)
                    && object.get("bonds").is_some_and(Value::is_array))
        });
        if is_fragment {
            if let Ok(fragment) = serde_json::from_value(value.clone()) {
                return Ok(Self::Fragment(fragment));
            }
        }
        Ok(Self::Json(value))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageResourceData {
    pub mime_type: String,
    pub data_base64: String,
    pub pixel_width: u32,
    pub pixel_height: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_name: Option<String>,
}

impl ResourceData {
    pub fn as_fragment(&self) -> Option<&MoleculeFragment> {
        match self {
            Self::Fragment(fragment) => Some(fragment),
            _ => None,
        }
    }

    pub fn as_fragment_mut(&mut self) -> Option<&mut MoleculeFragment> {
        match self {
            Self::Fragment(fragment) => Some(fragment),
            _ => None,
        }
    }

    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(text) => Some(text.as_str()),
            _ => None,
        }
    }

    pub fn as_image(&self) -> Option<ImageResourceData> {
        match self {
            Self::Json(value) => serde_json::from_value(value.clone()).ok(),
            _ => None,
        }
    }

    pub fn as_embedded_object(&self) -> Option<crate::EmbeddedObjectResourceData> {
        match self {
            Self::Json(value) => serde_json::from_value(value.clone()).ok(),
            _ => None,
        }
    }
}

impl Resource {
    pub fn display_image(&self) -> Option<ImageResourceData> {
        match self.resource_type.as_str() {
            "image" => self.data.as_image(),
            "embedded-object" => self
                .data
                .as_embedded_object()
                .and_then(|embedded| embedded.preview),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoleculeFragment {
    #[serde(default = "default_molecule_fragment_schema")]
    pub schema: String,
    #[serde(default = "default_molecule_fragment_bbox")]
    pub bbox: [f64; 4],
    #[serde(default)]
    pub nodes: Vec<Node>,
    #[serde(default)]
    pub bonds: Vec<Bond>,
    /// Native ChemDraw molecular-area objects. Every entry names the bonds
    /// forming one simple ring; geometry is always derived from current atom
    /// coordinates so edits cannot leave a stale polygon behind.
    #[serde(
        default,
        rename = "coloredAreas",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub colored_areas: Vec<ColoredMolecularArea>,
    /// Source-independent molecular stereochemistry that cannot be recovered
    /// solely from bond drawing glyphs. References use fragment node/bond IDs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stereo: Vec<chemsema_chemical_graph::StereoElementV2>,
    /// Native multicenter molecular relationships. CDXML MultiAttachment
    /// proxy nodes are a presentation/transport encoding of this field.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub interactions: Vec<chemsema_chemical_graph::MultiCenterInteractionV2>,
    #[serde(default)]
    pub meta: Value,
}

impl MoleculeFragment {
    pub fn blank() -> Self {
        Self {
            schema: "chemsema.molecule.fragment2d".to_string(),
            bbox: [0.0, 0.0, DEFAULT_PAGE_WIDTH, DEFAULT_PAGE_HEIGHT],
            nodes: Vec::new(),
            bonds: Vec::new(),
            colored_areas: Vec::new(),
            stereo: Vec::new(),
            interactions: Vec::new(),
            meta: Value::Null,
        }
    }
}

fn default_molecule_fragment_schema() -> String {
    "chemsema.molecule.fragment2d".to_string()
}

fn default_molecule_fragment_bbox() -> [f64; 4] {
    [0.0, 0.0, DEFAULT_PAGE_WIDTH, DEFAULT_PAGE_HEIGHT]
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    pub id: String,
    pub element: String,
    pub atomic_number: u8,
    pub position: [f64; 2],
    pub charge: i32,
    pub num_hydrogens: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub highlight_color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_connection: Option<ExternalConnection>,
    #[serde(default)]
    pub is_placeholder: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<NodeLabel>,
    #[serde(default, skip_serializing_if = "AtomProperties::is_default")]
    pub atom_properties: AtomProperties,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub nmr_assignments: Vec<crate::NmrAssignment>,
    #[serde(default)]
    pub meta: Value,
}

impl Node {
    pub fn carbon(id: String, point: Point) -> Self {
        Self {
            id,
            element: "C".to_string(),
            atomic_number: 6,
            position: [round2(point.x), round2(point.y)],
            charge: 0,
            num_hydrogens: 0,
            highlight_color: None,
            external_connection: None,
            is_placeholder: false,
            label: None,
            atom_properties: AtomProperties::default(),
            nmr_assignments: Vec::new(),
            meta: Value::Null,
        }
    }

    pub fn point(&self) -> Point {
        Point::new(self.position[0], self.position[1])
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalConnection {
    #[serde(rename = "type")]
    pub connection_type: ExternalConnectionType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number: Option<u16>,
}

impl Default for ExternalConnection {
    fn default() -> Self {
        Self {
            connection_type: ExternalConnectionType::Unspecified,
            number: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExternalConnectionType {
    #[default]
    Unspecified,
    Diamond,
    Star,
    PolymerBead,
    Wavy,
    Residue,
    Peptide,
    Dna,
    Rna,
    Terminus,
    Sulfide,
    Nucleotide,
    UnlinkedBranch,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AtomRadical {
    #[default]
    None,
    Singlet,
    Doublet,
    Triplet,
}

impl AtomRadical {
    pub fn electron_count(self) -> i32 {
        match self {
            Self::None => 0,
            Self::Doublet => 1,
            Self::Singlet | Self::Triplet => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IsotopicAbundance {
    #[default]
    Unspecified,
    Any,
    Natural,
    Enriched,
    Deficient,
    Nonnatural,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RingBondCount {
    #[default]
    Unspecified,
    NoRingBonds,
    AsDrawn,
    SimpleRing,
    Fusion,
    SpiroOrHigher,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UnsaturatedBonds {
    #[default]
    Unspecified,
    MustBeAbsent,
    MustBePresent,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum QueryTranslation {
    #[default]
    Equal,
    Broad,
    Narrow,
    Any,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AtomReactionStereo {
    #[default]
    Unspecified,
    Inversion,
    Retention,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BondQueryOrder {
    Single,
    Aromatic,
    Double,
    Triple,
}

impl BondQueryOrder {
    pub fn cdxml_value(self) -> &'static str {
        match self {
            Self::Single => "1",
            Self::Aromatic => "1.5",
            Self::Double => "2",
            Self::Triple => "3",
        }
    }

    pub fn mnemonic(self) -> &'static str {
        match self {
            Self::Single => "S",
            Self::Aromatic => "A",
            Self::Double => "D",
            Self::Triple => "T",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BondTopology {
    #[default]
    Unspecified,
    Ring,
    Chain,
    RingOrChain,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BondReactionParticipation {
    #[default]
    Unspecified,
    ReactionCenter,
    MakeOrBreak,
    ChangeType,
    MakeAndChange,
    NotReactionCenter,
    NoChange,
    Unmapped,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BondAbsoluteStereo {
    #[default]
    Unspecified,
    None,
    E,
    Z,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndicatorPosition {
    #[serde(default = "default_indicator_position_mode")]
    pub mode: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub angle: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<[f64; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub absolute: Option<[f64; 2]>,
}

impl Default for IndicatorPosition {
    fn default() -> Self {
        Self {
            mode: default_indicator_position_mode(),
            angle: None,
            offset: None,
            absolute: None,
        }
    }
}

fn default_indicator_position_mode() -> String {
    "auto".to_string()
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AtomProperties {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub isotope_mass: Option<i16>,
    #[serde(default, skip_serializing_if = "is_default_isotopic_abundance")]
    pub isotopic_abundance: IsotopicAbundance,
    #[serde(default, skip_serializing_if = "is_default_atom_radical")]
    pub radical: AtomRadical,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub atom_number: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_atom_number: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cip_stereo: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_atom_stereo: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub atom_number_position: Option<IndicatorPosition>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stereo_position: Option<IndicatorPosition>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub element_list: Vec<u8>,
    #[serde(default)]
    pub element_list_excluded: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub generic_list: Vec<String>,
    #[serde(default)]
    pub generic_list_excluded: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub free_sites: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_atom_query: Option<bool>,
    #[serde(default, skip_serializing_if = "is_default_ring_bond_count")]
    pub ring_bond_count: RingBondCount,
    #[serde(default, skip_serializing_if = "is_default_unsaturated_bonds")]
    pub unsaturated_bonds: UnsaturatedBonds,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub substituents_up_to: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub substituents_exactly: Option<u8>,
    #[serde(default, skip_serializing_if = "is_default_query_translation")]
    pub translation: QueryTranslation,
    #[serde(default)]
    pub abnormal_valence: bool,
    #[serde(default)]
    pub reaction_change: bool,
    #[serde(default, skip_serializing_if = "is_default_atom_reaction_stereo")]
    pub reaction_stereo: AtomReactionStereo,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_terminal_carbon_label: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_non_terminal_carbon_label: Option<bool>,
}

impl AtomProperties {
    pub fn is_default(&self) -> bool {
        self == &Self::default()
    }
}

fn is_default_isotopic_abundance(value: &IsotopicAbundance) -> bool {
    *value == IsotopicAbundance::Unspecified
}

fn is_default_atom_radical(value: &AtomRadical) -> bool {
    *value == AtomRadical::None
}

fn is_default_ring_bond_count(value: &RingBondCount) -> bool {
    *value == RingBondCount::Unspecified
}

fn is_default_unsaturated_bonds(value: &UnsaturatedBonds) -> bool {
    *value == UnsaturatedBonds::Unspecified
}

fn is_default_query_translation(value: &QueryTranslation) -> bool {
    *value == QueryTranslation::Equal
}

fn is_default_atom_reaction_stereo(value: &AtomReactionStereo) -> bool {
    *value == AtomReactionStereo::Unspecified
}

pub(crate) fn parse_query_string_list(value: Option<&str>) -> (Vec<String>, bool) {
    let mut tokens = value.unwrap_or("").split_whitespace();
    let first = tokens.next();
    let excluded = first.is_some_and(|value| value.eq_ignore_ascii_case("NOT"));
    let values = first
        .filter(|_| !excluded)
        .into_iter()
        .chain(tokens)
        .map(ToString::to_string)
        .collect();
    (values, excluded)
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BondProperties {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub query_orders: Vec<BondQueryOrder>,
    #[serde(default, skip_serializing_if = "is_default_bond_topology")]
    pub topology: BondTopology,
    #[serde(
        default,
        skip_serializing_if = "is_default_bond_reaction_participation"
    )]
    pub reaction_participation: BondReactionParticipation,
    #[serde(default, skip_serializing_if = "is_default_bond_absolute_stereo")]
    pub absolute_stereo: BondAbsoluteStereo,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_query: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_reaction: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_stereo: Option<bool>,
}

impl BondProperties {
    pub fn is_default(&self) -> bool {
        self == &Self::default()
    }
}

fn is_default_bond_topology(value: &BondTopology) -> bool {
    *value == BondTopology::Unspecified
}

fn is_default_bond_reaction_participation(value: &BondReactionParticipation) -> bool {
    *value == BondReactionParticipation::Unspecified
}

fn is_default_bond_absolute_stereo(value: &BondAbsoluteStereo) -> bool {
    *value == BondAbsoluteStereo::Unspecified
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeLabel {
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<[f64; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub box_field: Option<[f64; 4]>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub runs: Vec<LabelRun>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub line_runs: Vec<Vec<LabelRun>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lines: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub align: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attachment: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_size: Option<f64>,
    /// Default baseline-to-baseline advance in document points. It is present
    /// for single-line labels too, even though another baseline is required
    /// before the spacing becomes visible.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_height: Option<f64>,
    /// Resolved behavior, kept separately so equal numeric advances do not
    /// collapse fixed, automatic, and variable source semantics.
    #[serde(default = "default_node_label_line_height_mode")]
    pub line_height_mode: String,
    /// Optional per-transition baseline advances for source formats with
    /// variable line height. Entry 0 is the advance from line 0 to line 1.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub line_advances: Vec<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub glyph_polygons: Vec<Vec<[f64; 2]>>,
    /// Derived bond-retreat geometry. It is rebuilt from the styled runs and
    /// current MarginWidth whenever a document is loaded or a label edit is
    /// committed; it is deliberately not a CCJS persistence authority.
    #[serde(skip)]
    pub glyph_clip_polygons: Vec<Vec<[f64; 2]>>,
    #[serde(default, rename = "box", skip_serializing_if = "Option::is_none")]
    pub box_value: Option<[f64; 4]>,
    #[serde(default)]
    pub meta: Value,
}

fn default_node_label_line_height_mode() -> String {
    "variable".to_string()
}

impl NodeLabel {
    pub fn bbox(&self) -> Option<[f64; 4]> {
        self.box_value.or(self.box_field)
    }

    pub fn has_visible_text(&self) -> bool {
        !self.text.trim().is_empty()
    }

    pub fn glyph_polygons(&self) -> Vec<Vec<Point>> {
        self.glyph_polygons
            .iter()
            .map(|polygon| {
                polygon
                    .iter()
                    .map(|point| Point::new(point[0], point[1]))
                    .collect()
            })
            .collect()
    }

    pub fn glyph_clip_polygons(&self) -> Vec<Vec<Point>> {
        self.glyph_clip_polygons
            .iter()
            .map(|polygon| {
                polygon
                    .iter()
                    .map(|point| Point::new(point[0], point[1]))
                    .collect()
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LabelRun {
    #[serde(default)]
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_size: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_weight: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_style: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub underline: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outline: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadow: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bond {
    pub id: String,
    pub begin: String,
    pub end: String,
    pub order: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub highlight_color: Option<String>,
    #[serde(default, skip_serializing_if = "BondProperties::is_default")]
    pub properties: BondProperties,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub double: Option<DoubleBond>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stereo: Option<BondStereo>,
    #[serde(default)]
    pub stroke_width: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bold_width: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wedge_width: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_clip_margin: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hash_spacing: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bond_spacing: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_width: Option<f64>,
    #[serde(default)]
    pub line_styles: BondLineStyles,
    #[serde(default)]
    pub line_weights: BondLineWeights,
    #[serde(default)]
    pub meta: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColoredMolecularArea {
    pub id: String,
    pub color: String,
    /// Bond IDs must form exactly one connected simple cycle.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub basis_bonds: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DoubleBond {
    pub placement: DoubleBondPlacement,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub center_exit_side: Option<DoubleBondPlacement>,
    #[serde(default)]
    pub frozen: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BondLineStyles {
    #[serde(default)]
    pub main: BondLinePattern,
    #[serde(default)]
    pub left: BondLinePattern,
    #[serde(default)]
    pub right: BondLinePattern,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BondLineWeights {
    #[serde(default)]
    pub main: BondLineWeight,
    #[serde(default)]
    pub left: BondLineWeight,
    #[serde(default)]
    pub right: BondLineWeight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum BondLinePattern {
    #[default]
    Solid,
    Dashed,
    Wavy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum BondLineWeight {
    #[default]
    Normal,
    Bold,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BondStereo {
    pub kind: String,
    pub wide_end: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DoubleBondPlacement {
    Left,
    Right,
    Center,
}

pub struct EditableFragment<'a> {
    pub object: &'a SceneObject,
    pub fragment: &'a MoleculeFragment,
}

impl EditableFragment<'_> {
    pub fn world_point_for_node(&self, node: &Node) -> Point {
        Point::new(
            self.object.transform.translate[0] + node.position[0],
            self.object.transform.translate[1] + node.position[1],
        )
    }
}

pub struct EditableFragmentMut<'a> {
    pub object: &'a mut SceneObject,
    pub fragment: &'a mut MoleculeFragment,
}

impl EditableFragmentMut<'_> {
    pub fn world_point_for_node(&self, node: &Node) -> Point {
        Point::new(
            self.object.transform.translate[0] + node.position[0],
            self.object.transform.translate[1] + node.position[1],
        )
    }

    pub fn local_point(&self, point: Point) -> Point {
        Point::new(
            point.x - self.object.transform.translate[0],
            point.y - self.object.transform.translate[1],
        )
    }

    pub fn update_bounds(&mut self) {
        self.fragment.bbox = fragment_content_bbox(&self.fragment.nodes).unwrap_or([
            0.0,
            0.0,
            DEFAULT_PAGE_WIDTH,
            DEFAULT_PAGE_HEIGHT,
        ]);
        self.object.payload.bbox = Some(self.fragment.bbox);
    }
}

fn fragment_content_bbox(nodes: &[Node]) -> Option<[f64; 4]> {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut found = false;

    for node in nodes {
        let node_padding = if node.external_connection.is_some() {
            DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT * 0.75 + DEFAULT_BOND_STROKE_PT * 2.0
        } else {
            DEFAULT_TEXT_BLOCK_PADDING_PT
        };
        min_x = min_x.min(node.position[0] - node_padding);
        min_y = min_y.min(node.position[1] - node_padding);
        max_x = max_x.max(node.position[0] + node_padding);
        max_y = max_y.max(node.position[1] + node_padding);
        found = true;

        if let Some(label) = &node.label {
            if let Some([x1, y1, x2, y2]) = label.bbox() {
                min_x = min_x.min(x1);
                min_y = min_y.min(y1);
                max_x = max_x.max(x2);
                max_y = max_y.max(y2);
                found = true;
            }
        }
    }

    found.then_some([
        round2(min_x),
        round2(min_y),
        round2((max_x - min_x).max(1.0)),
        round2((max_y - min_y).max(1.0)),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MOLECULE_LABEL_ANCHOR_BASELINE_RATIO;

    fn polygon_bounds(polygon: &[[f64; 2]]) -> Option<[f64; 4]> {
        let mut iter = polygon.iter();
        let first = iter.next()?;
        let mut min_x = first[0];
        let mut min_y = first[1];
        let mut max_x = first[0];
        let mut max_y = first[1];
        for point in iter {
            min_x = min_x.min(point[0]);
            min_y = min_y.min(point[1]);
            max_x = max_x.max(point[0]);
            max_y = max_y.max(point[1]);
        }
        Some([min_x, min_y, max_x, max_y])
    }

    fn glyph_center(label: &NodeLabel, index: usize) -> Point {
        let bounds = polygon_bounds(
            label
                .glyph_polygons
                .get(index)
                .expect("glyph polygon should exist"),
        )
        .expect("glyph polygon should have bounds");
        Point::new((bounds[0] + bounds[2]) * 0.5, (bounds[1] + bounds[3]) * 0.5)
    }

    #[test]
    fn imported_label_rebuilds_active_box_from_current_glyph_metrics() {
        let stale_box = [16.0, 15.0, 31.0, 34.0];
        let document = parse_document_json(
            &json!({
                "format": { "name": "chemsema", "version": "0.1" },
                "document": {
                    "id": "doc_stale_label_box",
                    "title": "stale label box",
                    "page": { "width": 80.0, "height": 60.0, "background": "#ffffff" }
                },
                "objects": [{
                    "id": "obj_molecule_001",
                    "type": "molecule",
                    "visible": true,
                    "zIndex": 10,
                    "payload": { "resourceRef": "mol_001" }
                }],
                "resources": {
                    "mol_001": {
                        "type": "molecule_fragment2d",
                        "encoding": "chemsema.molecule.fragment2d",
                        "data": {
                            "schema": "chemsema.molecule.fragment2d",
                            "bbox": [0.0, 0.0, 80.0, 60.0],
                            "nodes": [{
                                "id": "label",
                                "element": "C",
                                "atomicNumber": 6,
                                "position": [20.0, 20.0],
                                "charge": 0,
                                "numHydrogens": 0,
                                "isPlaceholder": true,
                                "label": {
                                    "text": "OCF3",
                                    "sourceText": "OCF3",
                                    "position": [16.0, 24.0],
                                    "box": stale_box,
                                    "boxField": stale_box,
                                    "runs": [{
                                        "text": "OCF3",
                                        "fontFamily": "Arial",
                                        "fontSize": 10.0,
                                        "fontWeight": 700,
                                        "fontStyle": "normal",
                                        "script": "normal"
                                    }],
                                    "align": "left",
                                    "anchor": "start",
                                    "attachment": "node",
                                    "fontFamily": "Arial",
                                    "fontSize": 10.0,
                                    "meta": {
                                        "import": { "cdxml": { "boundingBox": stale_box } }
                                    }
                                }
                            }, {
                                "id": "neighbor",
                                "element": "C",
                                "atomicNumber": 6,
                                "position": [20.0, 36.0],
                                "charge": 0,
                                "numHydrogens": 0
                            }],
                            "bonds": [{
                                "id": "bond",
                                "begin": "label",
                                "end": "neighbor",
                                "order": 1
                            }]
                        }
                    }
                }
            })
            .to_string(),
        )
        .expect("document should parse");
        let label = document
            .editable_fragments()
            .iter()
            .find_map(|entry| entry.fragment.nodes.iter().find(|node| node.id == "label"))
            .and_then(|node| node.label.as_ref())
            .expect("imported label");
        let active_box = label.bbox().expect("active label box");

        assert_ne!(active_box, stale_box);
        assert!(
            active_box[2] - active_box[0] > 20.0,
            "the active box must cover the full OCF3 run: {active_box:?}"
        );
        assert!(
            active_box[3] - active_box[1] < 15.0,
            "the stale source height must not leak into the active box: {active_box:?}"
        );
        assert_eq!(
            label.meta.pointer("/import/cdxml/boundingBox"),
            Some(&json!(stale_box)),
            "raw import evidence must remain available for round-trip diagnostics"
        );
    }

    #[test]
    fn imported_label_anchor_follows_horizontal_bond_side() {
        let document = parse_document_json(
            &json!({
                "format": { "name": "chemsema", "version": "0.1" },
                "document": {
                    "id": "doc_anchor_side",
                    "title": "anchor side",
                    "page": { "width": 90.0, "height": 40.0, "background": "#ffffff" }
                },
                "objects": [{
                    "id": "obj_molecule_001",
                    "type": "molecule",
                    "visible": true,
                    "zIndex": 10,
                    "payload": { "resourceRef": "mol_001" }
                }],
                "resources": {
                    "mol_001": {
                        "type": "molecule_fragment2d",
                        "encoding": "chemsema.molecule.fragment2d",
                        "data": {
                            "schema": "chemsema.molecule.fragment2d",
                            "bbox": [0.0, 0.0, 90.0, 40.0],
                            "nodes": [{
                                "id": "left_label",
                                "element": "C",
                                "atomicNumber": 6,
                                "position": [10.0, 10.0],
                                "charge": 0,
                                "numHydrogens": 0,
                                "label": {
                                    "text": "Ph",
                                    "sourceText": "Ph",
                                    "position": [6.78, 13.63],
                                    "box": [6.78, 5.43, 19.08, 15.93],
                                    "runs": [{ "text": "Ph", "fontFamily": "Arial", "fontSize": 10.0 }],
                                    "align": "left",
                                    "anchor": "start",
                                    "attachment": "node",
                                    "fontFamily": "Arial",
                                    "fontSize": 10.0,
                                    "meta": { "import": { "cdxml": { "boundingBox": [6.78, 5.43, 19.08, 15.93] } } }
                                }
                            }, {
                                "id": "left_neighbor",
                                "element": "C",
                                "atomicNumber": 6,
                                "position": [24.0, 10.0],
                                "charge": 0,
                                "numHydrogens": 0
                            }, {
                                "id": "right_label",
                                "element": "C",
                                "atomicNumber": 6,
                                "position": [54.0, 10.0],
                                "charge": 0,
                                "numHydrogens": 0,
                                "label": {
                                    "text": "2-NP",
                                    "sourceText": "2-NP",
                                    "position": [50.78, 13.63],
                                    "box": [50.78, 5.43, 73.58, 15.93],
                                    "runs": [{ "text": "2-NP", "fontFamily": "Arial", "fontSize": 10.0 }],
                                    "align": "left",
                                    "anchor": "start",
                                    "attachment": "node",
                                    "fontFamily": "Arial",
                                    "fontSize": 10.0,
                                    "meta": { "import": { "cdxml": { "boundingBox": [50.78, 5.43, 73.58, 15.93] } } }
                                }
                            }, {
                                "id": "right_neighbor",
                                "element": "C",
                                "atomicNumber": 6,
                                "position": [40.0, 10.0],
                                "charge": 0,
                                "numHydrogens": 0
                            }],
                            "bonds": [{
                                "id": "b_right",
                                "begin": "left_label",
                                "end": "left_neighbor",
                                "order": 1
                            }, {
                                "id": "b_left",
                                "begin": "right_label",
                                "end": "right_neighbor",
                                "order": 1
                            }]
                        }
                    }
                }
            })
            .to_string(),
        )
        .expect("document should parse");
        let fragments = document.editable_fragments();
        let left_label_node = fragments
            .iter()
            .find_map(|entry| {
                entry
                    .fragment
                    .nodes
                    .iter()
                    .find(|node| node.id == "left_label")
            })
            .expect("left label node");
        let left_label = left_label_node.label.as_ref().expect("left label");
        let right_label_node = fragments
            .iter()
            .find_map(|entry| {
                entry
                    .fragment
                    .nodes
                    .iter()
                    .find(|node| node.id == "right_label")
            })
            .expect("right label node");
        let right_label = right_label_node.label.as_ref().expect("right label");

        let left_anchor = glyph_center(left_label, 1);
        let left_line_anchor_y = left_label.position.expect("left label baseline")[1]
            - left_label
                .font_size
                .unwrap_or(DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT)
                * MOLECULE_LABEL_ANCHOR_BASELINE_RATIO;
        assert!(
            (left_anchor.x - left_label_node.position[0]).abs() < 0.01
                && (left_line_anchor_y - left_label_node.position[1]).abs() < 0.01,
            "right-side bond should anchor Ph on h horizontally and the label line vertically: node={left_label_node:?}, label={left_label:?}"
        );
        let right_anchor = glyph_center(right_label, 0);
        let right_line_anchor_y = right_label.position.expect("right label baseline")[1]
            - right_label
                .font_size
                .unwrap_or(DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT)
                * MOLECULE_LABEL_ANCHOR_BASELINE_RATIO;
        assert!(
            (right_anchor.x - right_label_node.position[0]).abs() < 0.01
                && (right_line_anchor_y - right_label_node.position[1]).abs() < 0.01,
            "left-side bond should anchor 2-NP on 2 horizontally and the label line vertically: node={right_label_node:?}, label={right_label:?}"
        );
    }

    #[test]
    fn parse_document_json_rebuilds_fragment_label_glyph_polygons() {
        let mut document = ChemSemaDocument::blank();
        document.resources.insert(
            "frag_1".to_string(),
            Resource {
                resource_type: "molecule_fragment2d".to_string(),
                encoding: "chemsema.molecule.fragment2d".to_string(),
                data: ResourceData::Fragment(MoleculeFragment {
                    schema: "chemsema.molecule.fragment2d".to_string(),
                    bbox: [0.0, 0.0, 20.0, 20.0],
                    nodes: vec![Node {
                        id: "n1".to_string(),
                        element: "N".to_string(),
                        atomic_number: 7,
                        position: [10.0, 10.0],
                        charge: 0,
                        atom_properties: AtomProperties::default(),
                        nmr_assignments: Vec::new(),
                        num_hydrogens: 0,
                        highlight_color: None,
                        external_connection: None,
                        is_placeholder: false,
                        label: Some(NodeLabel {
                            text: "N".to_string(),
                            source_text: Some("N".to_string()),
                            position: Some([10.0, 10.0]),
                            box_field: None,
                            runs: vec![LabelRun {
                                text: "N".to_string(),
                                font_family: Some("Arial".to_string()),
                                font_size: Some(10.0),
                                fill: Some("#000000".to_string()),
                                font_weight: Some(400),
                                font_style: Some("normal".to_string()),
                                underline: None,
                                outline: None,
                                shadow: None,
                                script: Some("normal".to_string()),
                            }],
                            line_runs: Vec::new(),
                            lines: Vec::new(),
                            align: Some("left".to_string()),
                            layout: None,
                            attachment: Some("node".to_string()),
                            anchor: Some("start".to_string()),
                            font_family: Some("Arial".to_string()),
                            fill: Some("#000000".to_string()),
                            font_size: Some(10.0),
                            line_height: Some(crate::molecule_label_line_advance(10.0)),
                            line_height_mode: "variable".to_string(),
                            line_advances: Vec::new(),
                            glyph_polygons: vec![vec![
                                [0.0, 0.0],
                                [1.0, 0.0],
                                [1.0, 1.0],
                                [0.0, 1.0],
                            ]],
                            glyph_clip_polygons: Vec::new(),
                            box_value: Some([10.0, 2.0, 17.2, 10.0]),
                            meta: json!({
                                "import": {
                                    "cdxml": {
                                        "boundingBox": [26.4, 24.95, 33.62, 36.45]
                                    }
                                }
                            }),
                        }),
                        meta: Value::Null,
                    }],
                    bonds: Vec::new(),
                    colored_areas: Vec::new(),
                    stereo: Vec::new(),
                    interactions: Vec::new(),
                    meta: Value::Null,
                }),
                meta: Value::Null,
            },
        );

        normalize_fragment_label_payloads(&mut document);

        let resource = document.resources.get("frag_1").expect("resource");
        let fragment = resource.data.as_fragment().expect("fragment");
        let label = fragment.nodes[0].label.as_ref().expect("label");

        assert_eq!(label.text, "N");
        assert_eq!(label.glyph_polygons.len(), 1);
        assert_ne!(
            label.glyph_polygons[0],
            vec![[10.0, 2.0], [17.2, 2.0], [17.2, 10.0], [10.0, 10.0]],
            "stale glyph polygon must not remain authoritative"
        );
        assert!(
            !label.glyph_clip_polygons.is_empty(),
            "loading must rebuild the final retreat geometry"
        );
    }

    #[test]
    fn parse_document_json_accepts_legacy_fragment_without_schema_or_bbox() {
        let document = parse_document_json(
            &json!({
                "format": { "name": "chemsema", "version": "0.1" },
                "document": {
                    "id": "doc_legacy_fragment",
                    "title": "legacy fragment",
                    "page": { "width": 90.0, "height": 40.0, "background": "#ffffff" }
                },
                "objects": [{
                    "id": "obj_molecule_001",
                    "type": "molecule",
                    "visible": true,
                    "zIndex": 10,
                    "payload": { "resourceRef": "mol_001" }
                }],
                "resources": {
                    "mol_001": {
                        "type": "molecule_fragment2d",
                        "encoding": "chemsema.molecule.fragment2d",
                        "data": {
                            "nodes": [{
                                "id": "n1",
                                "element": "C",
                                "atomicNumber": 6,
                                "position": [10.0, 10.0],
                                "charge": 0,
                                "numHydrogens": 0
                            }],
                            "bonds": []
                        }
                    }
                }
            })
            .to_string(),
        )
        .expect("legacy fragment should parse with default schema and bbox");

        let fragment = document
            .resources
            .get("mol_001")
            .and_then(|resource| resource.data.as_fragment())
            .expect("fragment resource");
        assert_eq!(fragment.schema, "chemsema.molecule.fragment2d");
        assert_eq!(fragment.nodes.len(), 1);
    }

    #[test]
    fn parse_document_json_splits_legacy_cdxml_merged_molecule() {
        let document = parse_document_json(
            &json!({
                "format": { "name": "chemsema", "version": "0.1" },
                "document": {
                    "id": "doc_legacy_cdxml_merged",
                    "title": "legacy merged cdxml",
                    "page": { "width": 140.0, "height": 80.0, "background": "#ffffff" }
                },
                "objects": [{
                    "id": "obj_cdxml_merged_molecule",
                    "type": "molecule",
                    "visible": true,
                    "transform": {
                        "translate": [10.0, 10.0],
                        "rotate": 0.0,
                        "scale": [1.0, 1.0]
                    },
                    "meta": { "source": "cdxml", "mergedFragments": true },
                    "payload": {
                        "resourceRef": "mol_cdxml_merged",
                        "bbox": [0.0, 0.0, 102.0, 10.0]
                    }
                }],
                "resources": {
                    "mol_cdxml_merged": {
                        "type": "molecule_fragment2d",
                        "encoding": "chemsema.molecule.fragment2d",
                        "data": {
                            "schema": "chemsema.molecule.fragment2d",
                            "bbox": [0.0, 0.0, 102.0, 10.0],
                            "nodes": [
                                { "id": "n1", "element": "C", "atomicNumber": 6, "position": [0.0, 5.0], "charge": 0, "numHydrogens": 0 },
                                { "id": "n2", "element": "C", "atomicNumber": 6, "position": [30.0, 5.0], "charge": 0, "numHydrogens": 0 },
                                { "id": "n3", "element": "C", "atomicNumber": 6, "position": [72.0, 5.0], "charge": 0, "numHydrogens": 0 },
                                { "id": "n4", "element": "C", "atomicNumber": 6, "position": [102.0, 5.0], "charge": 0, "numHydrogens": 0 }
                            ],
                            "bonds": [
                                { "id": "b1", "begin": "n1", "end": "n2", "order": 1 },
                                { "id": "b2", "begin": "n3", "end": "n4", "order": 1 }
                            ]
                        },
                        "meta": { "import": { "cdxml": { "merged": true } } }
                    }
                }
            })
            .to_string(),
        )
        .expect("legacy merged molecule should parse");

        assert!(!document.resources.contains_key("mol_cdxml_merged"));
        assert_eq!(document.objects.len(), 2);
        assert_eq!(document.editable_fragments().len(), 2);
        assert_eq!(
            document
                .editable_fragments()
                .iter()
                .map(|entry| entry.fragment.bonds.len())
                .collect::<Vec<_>>(),
            vec![1, 1]
        );
        assert_eq!(document.objects[0].transform.translate, [10.0, 14.5]);
        assert_eq!(document.objects[1].transform.translate, [82.0, 14.5]);
    }

    #[test]
    fn parse_document_json_splits_imported_disconnected_visible_molecule() {
        let document = parse_document_json(
            &json!({
                "format": { "name": "chemsema", "version": "0.1" },
                "document": {
                    "id": "doc_disconnected_molecule",
                    "title": "disconnected molecule",
                    "page": { "width": 140.0, "height": 80.0, "background": "#ffffff" }
                },
                "objects": [{
                    "id": "obj_molecule_001",
                    "type": "molecule",
                    "visible": true,
                    "meta": { "source": "cdxml" },
                    "transform": {
                        "translate": [10.0, 10.0],
                        "rotate": 0.0,
                        "scale": [1.0, 1.0]
                    },
                    "payload": {
                        "resourceRef": "mol_001",
                        "bbox": [0.0, 0.0, 102.0, 10.0]
                    }
                }],
                "resources": {
                    "mol_001": {
                        "type": "molecule_fragment2d",
                        "encoding": "chemsema.molecule.fragment2d",
                        "data": {
                            "schema": "chemsema.molecule.fragment2d",
                            "bbox": [0.0, 0.0, 102.0, 10.0],
                            "nodes": [
                                { "id": "n1", "element": "C", "atomicNumber": 6, "position": [0.0, 5.0], "charge": 0, "numHydrogens": 0 },
                                { "id": "n2", "element": "C", "atomicNumber": 6, "position": [30.0, 5.0], "charge": 0, "numHydrogens": 0 },
                                { "id": "n3", "element": "C", "atomicNumber": 6, "position": [72.0, 5.0], "charge": 0, "numHydrogens": 0 },
                                { "id": "n4", "element": "C", "atomicNumber": 6, "position": [102.0, 5.0], "charge": 0, "numHydrogens": 0 }
                            ],
                            "bonds": [
                                { "id": "b1", "begin": "n1", "end": "n2", "order": 1 },
                                { "id": "b2", "begin": "n3", "end": "n4", "order": 1 }
                            ]
                        }
                    }
                }
            })
            .to_string(),
        )
        .expect("disconnected molecule should parse");

        assert!(!document.resources.contains_key("mol_001"));
        assert_eq!(document.objects.len(), 2);
        assert_eq!(document.editable_fragments().len(), 2);
        assert_eq!(
            document
                .editable_fragments()
                .iter()
                .map(|entry| entry.fragment.bonds.len())
                .collect::<Vec<_>>(),
            vec![1, 1]
        );
        assert_eq!(document.objects[0].transform.translate, [10.0, 14.5]);
        assert_eq!(document.objects[1].transform.translate, [82.0, 14.5]);
    }

    #[test]
    fn parse_document_json_splits_unmarked_disconnected_visible_molecule() {
        let document = parse_document_json(
            &json!({
                "format": { "name": "chemsema", "version": "0.1" },
                "document": {
                    "id": "doc_unmarked_disconnected_molecule",
                    "title": "unmarked disconnected molecule",
                    "page": { "width": 140.0, "height": 80.0, "background": "#ffffff" }
                },
                "objects": [{
                    "id": "obj_molecule_001",
                    "type": "molecule",
                    "visible": true,
                    "transform": {
                        "translate": [10.0, 10.0],
                        "rotate": 0.0,
                        "scale": [1.0, 1.0]
                    },
                    "payload": {
                        "resourceRef": "mol_001",
                        "bbox": [0.0, 0.0, 102.0, 10.0]
                    }
                }],
                "resources": {
                    "mol_001": {
                        "type": "molecule_fragment2d",
                        "encoding": "chemsema.molecule.fragment2d",
                        "data": {
                            "schema": "chemsema.molecule.fragment2d",
                            "bbox": [0.0, 0.0, 102.0, 10.0],
                            "nodes": [
                                { "id": "n1", "element": "C", "atomicNumber": 6, "position": [0.0, 5.0], "charge": 0, "numHydrogens": 0 },
                                { "id": "n2", "element": "C", "atomicNumber": 6, "position": [30.0, 5.0], "charge": 0, "numHydrogens": 0 },
                                { "id": "n3", "element": "C", "atomicNumber": 6, "position": [72.0, 5.0], "charge": 0, "numHydrogens": 0 },
                                { "id": "n4", "element": "C", "atomicNumber": 6, "position": [102.0, 5.0], "charge": 0, "numHydrogens": 0 }
                            ],
                            "bonds": [
                                { "id": "b1", "begin": "n1", "end": "n2", "order": 1 },
                                { "id": "b2", "begin": "n3", "end": "n4", "order": 1 }
                            ]
                        }
                    }
                }
            })
            .to_string(),
        )
        .expect("unmarked disconnected molecule should parse");

        assert!(!document.resources.contains_key("mol_001"));
        assert_eq!(document.objects.len(), 2);
        assert_eq!(document.editable_fragments().len(), 2);
        assert_eq!(
            document
                .editable_fragments()
                .iter()
                .map(|entry| entry.fragment.bonds.len())
                .collect::<Vec<_>>(),
            vec![1, 1]
        );
    }

    #[test]
    fn parse_document_json_preserves_explicit_disconnected_molecule_opt_out() {
        let document = parse_document_json(
            &json!({
                "format": { "name": "chemsema", "version": "0.1" },
                "document": {
                    "id": "doc_preserved_disconnected_molecule",
                    "title": "preserved disconnected molecule",
                    "page": { "width": 140.0, "height": 80.0, "background": "#ffffff" }
                },
                "objects": [{
                    "id": "obj_molecule_001",
                    "type": "molecule",
                    "visible": true,
                    "meta": { "preserveDisconnectedComponents": true },
                    "transform": {
                        "translate": [10.0, 10.0],
                        "rotate": 0.0,
                        "scale": [1.0, 1.0]
                    },
                    "payload": {
                        "resourceRef": "mol_001",
                        "bbox": [0.0, 0.0, 102.0, 10.0]
                    }
                }],
                "resources": {
                    "mol_001": {
                        "type": "molecule_fragment2d",
                        "encoding": "chemsema.molecule.fragment2d",
                        "data": {
                            "schema": "chemsema.molecule.fragment2d",
                            "bbox": [0.0, 0.0, 102.0, 10.0],
                            "nodes": [
                                { "id": "n1", "element": "C", "atomicNumber": 6, "position": [0.0, 5.0], "charge": 0, "numHydrogens": 0 },
                                { "id": "n2", "element": "C", "atomicNumber": 6, "position": [30.0, 5.0], "charge": 0, "numHydrogens": 0 },
                                { "id": "n3", "element": "C", "atomicNumber": 6, "position": [72.0, 5.0], "charge": 0, "numHydrogens": 0 },
                                { "id": "n4", "element": "C", "atomicNumber": 6, "position": [102.0, 5.0], "charge": 0, "numHydrogens": 0 }
                            ],
                            "bonds": [
                                { "id": "b1", "begin": "n1", "end": "n2", "order": 1 },
                                { "id": "b2", "begin": "n3", "end": "n4", "order": 1 }
                            ]
                        }
                    }
                }
            })
            .to_string(),
        )
        .expect("preserved disconnected molecule should parse");

        assert!(document.resources.contains_key("mol_001"));
        assert_eq!(document.objects.len(), 1);
        assert_eq!(document.editable_fragments().len(), 1);
        assert_eq!(document.editable_fragments()[0].fragment.bonds.len(), 2);
    }

    #[test]
    fn parse_document_json_rejects_invalid_declared_fragment_resources() {
        let error = parse_document_json(
            &json!({
                "format": { "name": "chemsema", "version": "0.1" },
                "document": {
                    "id": "doc_invalid_fragment",
                    "title": "invalid fragment",
                    "page": { "width": 90.0, "height": 40.0, "background": "#ffffff" }
                },
                "objects": [{
                    "id": "obj_molecule_001",
                    "type": "molecule",
                    "visible": true,
                    "zIndex": 10,
                    "payload": { "resourceRef": "mol_001" }
                }],
                "resources": {
                    "mol_001": {
                        "type": "molecule_fragment2d",
                        "encoding": "chemsema.molecule.fragment2d",
                        "data": {
                            "schema": "chemsema.molecule.fragment2d",
                            "bbox": [0.0, 0.0, 90.0, 40.0],
                            "nodes": [{
                                "id": "n1",
                                "element": "C",
                                "atomicNumber": 6,
                                "position": [10.0, 10.0],
                                "charge": 0,
                                "numHydrogens": 0
                            }, {
                                "id": "n2",
                                "element": "C",
                                "atomicNumber": 6,
                                "position": [30.0, 10.0],
                                "charge": 0,
                                "numHydrogens": 0
                            }],
                            "bonds": [{
                                "id": "b1",
                                "begin": "n1",
                                "end": "n2",
                                "order": 1,
                                "stereo": "wedge",
                                "strokeWidth": 1.0
                            }]
                        }
                    }
                }
            })
            .to_string(),
        )
        .unwrap_err();

        assert!(error.contains("mol_001"));
        assert!(error.contains("molecule_fragment2d"));
    }

    #[test]
    fn rebuild_left_aligned_label_glyph_polygons_uses_label_baseline() {
        let mut document = ChemSemaDocument::blank();
        document.resources.insert(
            "frag_1".to_string(),
            Resource {
                resource_type: "molecule_fragment2d".to_string(),
                encoding: "chemsema.molecule.fragment2d".to_string(),
                data: ResourceData::Fragment(MoleculeFragment {
                    schema: "chemsema.molecule.fragment2d".to_string(),
                    bbox: [0.0, 0.0, 60.0, 60.0],
                    nodes: vec![Node {
                        id: "n1".to_string(),
                        element: "N".to_string(),
                        atomic_number: 7,
                        position: [30.0, 30.0],
                        charge: 0,
                        atom_properties: AtomProperties::default(),
                        nmr_assignments: Vec::new(),
                        num_hydrogens: 0,
                        highlight_color: None,
                        external_connection: None,
                        is_placeholder: false,
                        label: Some(NodeLabel {
                            text: "N".to_string(),
                            source_text: Some("N".to_string()),
                            position: Some([26.4, 33.9]),
                            box_field: Some([26.4, 24.95, 33.62, 36.45]),
                            runs: vec![LabelRun {
                                text: "N".to_string(),
                                font_family: Some("Arial".to_string()),
                                font_size: Some(10.0),
                                fill: Some("#000000".to_string()),
                                font_weight: Some(400),
                                font_style: Some("normal".to_string()),
                                underline: None,
                                outline: None,
                                shadow: None,
                                script: Some("chemical".to_string()),
                            }],
                            line_runs: Vec::new(),
                            lines: Vec::new(),
                            align: Some("left".to_string()),
                            layout: None,
                            attachment: Some("node".to_string()),
                            anchor: Some("start".to_string()),
                            font_family: Some("Arial".to_string()),
                            fill: Some("#000000".to_string()),
                            font_size: Some(10.0),
                            line_height: Some(crate::molecule_label_line_advance(10.0)),
                            line_height_mode: "variable".to_string(),
                            line_advances: Vec::new(),
                            glyph_polygons: Vec::new(),
                            glyph_clip_polygons: Vec::new(),
                            box_value: None,
                            meta: json!({
                                "import": {
                                    "cdxml": {
                                        "boundingBox": [26.4, 24.95, 33.62, 36.45]
                                    }
                                }
                            }),
                        }),
                        meta: Value::Null,
                    }],
                    bonds: Vec::new(),
                    colored_areas: Vec::new(),
                    stereo: Vec::new(),
                    interactions: Vec::new(),
                    meta: Value::Null,
                }),
                meta: Value::Null,
            },
        );

        normalize_fragment_label_payloads(&mut document);

        let resource = document.resources.get("frag_1").expect("resource");
        let fragment = resource.data.as_fragment().expect("fragment");
        let label = fragment.nodes[0].label.as_ref().expect("label");
        let line_anchor_y = label.position.expect("label baseline")[1]
            - label
                .font_size
                .unwrap_or(DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT)
                * MOLECULE_LABEL_ANCHOR_BASELINE_RATIO;

        assert!(
            (line_anchor_y - 30.0).abs() < 0.01,
            "single-glyph imported node labels should align their ChemDraw-calibrated line anchor to the node position, got label={label:?}",
        );
    }
}
