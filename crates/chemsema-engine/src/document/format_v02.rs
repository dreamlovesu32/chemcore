use super::*;
use serde::{de::Error as DeError, ser::Error as SerError, Deserializer, Serializer};

const FORMAT_NAME: &str = "chemsema";
const CURRENT_VERSION: &str = "0.2";
const LEGACY_VERSION: &str = "0.1";
const FORMAT_UNIT: &str = "pt";
const FORMAT_PROFILE: &str = "snapshot";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DocumentV02<'a> {
    format: FormatV02<'a>,
    document: &'a DocumentInfo,
    style: &'a DocumentStyleInfo,
    styles: &'a BTreeMap<String, Value>,
    entities: EntitiesV02,
    hierarchy: HierarchyV02,
    relations: &'a [LinkRelation],
    orders: &'a DocumentOrders,
    #[serde(skip_serializing_if = "crate::LogicalObjectData::is_empty")]
    logical_objects: &'a crate::LogicalObjectData,
    reaction_schemes: &'a [crate::ReactionSchemeData],
    chemical_properties: &'a [ChemicalProperty],
    resources: &'a BTreeMap<String, Resource>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    interchange: &'a BTreeMap<String, InterchangeDocument>,
}

#[derive(Serialize)]
struct FormatV02<'a> {
    name: &'a str,
    version: &'a str,
    unit: &'a str,
    profile: &'a str,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct EntitiesV02 {
    scene: Vec<SceneObject>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct HierarchyV02 {
    roots: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    children: BTreeMap<String, Vec<String>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DocumentV02Owned {
    format: FormatV02Owned,
    document: DocumentInfo,
    #[serde(default)]
    style: DocumentStyleInfo,
    #[serde(default)]
    styles: BTreeMap<String, Value>,
    entities: EntitiesV02,
    hierarchy: HierarchyV02,
    #[serde(default)]
    relations: Vec<LinkRelation>,
    #[serde(default)]
    orders: DocumentOrders,
    #[serde(default)]
    logical_objects: crate::LogicalObjectData,
    #[serde(default)]
    reaction_schemes: Vec<crate::ReactionSchemeData>,
    #[serde(default)]
    chemical_properties: Vec<ChemicalProperty>,
    #[serde(default)]
    resources: BTreeMap<String, Resource>,
    #[serde(default)]
    interchange: BTreeMap<String, InterchangeDocument>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FormatV02Owned {
    name: String,
    version: String,
    unit: String,
    profile: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DocumentV01Owned {
    format: FormatInfo,
    document: DocumentInfo,
    #[serde(default)]
    style: DocumentStyleInfo,
    #[serde(default)]
    styles: BTreeMap<String, Value>,
    #[serde(default)]
    objects: Vec<SceneObject>,
    #[serde(default)]
    links: Vec<LinkRelation>,
    #[serde(default)]
    logical_objects: crate::LogicalObjectData,
    #[serde(default)]
    reaction_schemes: Vec<crate::ReactionSchemeData>,
    #[serde(default)]
    chemical_properties: Vec<ChemicalProperty>,
    #[serde(default)]
    resources: BTreeMap<String, Resource>,
    #[serde(default)]
    interchange: BTreeMap<String, InterchangeDocument>,
}

pub(super) fn validate_format_header(value: &Value) -> Result<(), String> {
    let root = value
        .as_object()
        .ok_or_else(|| "CCJS document root must be a JSON object".to_string())?;
    let format = root
        .get("format")
        .and_then(Value::as_object)
        .ok_or_else(|| "CCJS document requires a format object".to_string())?;
    let name = format
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| "CCJS format.name must be a string".to_string())?;
    if name != FORMAT_NAME {
        return Err(format!(
            "Unsupported document format name '{name}'; expected '{FORMAT_NAME}'"
        ));
    }
    let version = format
        .get("version")
        .and_then(Value::as_str)
        .ok_or_else(|| "CCJS format.version must be a string".to_string())?;
    if version != CURRENT_VERSION && version != LEGACY_VERSION {
        return Err(format!(
            "Unsupported chemsema document version '{version}'; supported versions are {LEGACY_VERSION} and {CURRENT_VERSION}"
        ));
    }
    if let Some(unit) = format.get("unit").and_then(Value::as_str) {
        if !unit.eq_ignore_ascii_case(FORMAT_UNIT) {
            return Err(format!(
                "Unsupported chemsema document unit '{unit}'; expected '{FORMAT_UNIT}'"
            ));
        }
    } else if version == CURRENT_VERSION {
        return Err("CCJS v0.2 format.unit is required".to_string());
    }
    if version == CURRENT_VERSION {
        let profile = format
            .get("profile")
            .and_then(Value::as_str)
            .ok_or_else(|| "CCJS v0.2 format.profile is required".to_string())?;
        if profile != FORMAT_PROFILE {
            return Err(format!(
                "Unsupported CCJS v0.2 profile '{profile}'; expected '{FORMAT_PROFILE}'"
            ));
        }
    }
    Ok(())
}

impl Serialize for ChemSemaDocument {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let (scene, hierarchy) = flatten_scene(&self.objects).map_err(S::Error::custom)?;
        DocumentV02 {
            format: FormatV02 {
                name: FORMAT_NAME,
                version: CURRENT_VERSION,
                unit: FORMAT_UNIT,
                profile: FORMAT_PROFILE,
            },
            document: &self.document,
            style: &self.style,
            styles: &self.styles,
            entities: EntitiesV02 { scene },
            hierarchy,
            relations: &self.links,
            orders: &self.orders,
            logical_objects: &self.logical_objects,
            reaction_schemes: &self.reaction_schemes,
            chemical_properties: &self.chemical_properties,
            resources: &self.resources,
            interchange: &self.interchange,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ChemSemaDocument {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        validate_format_header(&value).map_err(D::Error::custom)?;
        let version = value
            .pointer("/format/version")
            .and_then(Value::as_str)
            .ok_or_else(|| D::Error::custom("CCJS format.version must be a string"))?;
        match version {
            CURRENT_VERSION => {
                let wire: DocumentV02Owned =
                    serde_json::from_value(value).map_err(D::Error::custom)?;
                wire.into_document().map_err(D::Error::custom)
            }
            LEGACY_VERSION => {
                let wire: DocumentV01Owned =
                    serde_json::from_value(value).map_err(D::Error::custom)?;
                Ok(wire.into_document())
            }
            _ => Err(D::Error::custom("unsupported CCJS version")),
        }
    }
}

impl DocumentV01Owned {
    fn into_document(mut self) -> ChemSemaDocument {
        self.format.name = FORMAT_NAME.to_string();
        self.format.version = CURRENT_VERSION.to_string();
        self.format.unit = FORMAT_UNIT.to_string();
        ChemSemaDocument {
            format: self.format,
            document: self.document,
            style: self.style,
            styles: self.styles,
            objects: self.objects,
            links: self.links,
            orders: DocumentOrders::default(),
            logical_objects: self.logical_objects,
            reaction_schemes: self.reaction_schemes,
            chemical_properties: self.chemical_properties,
            resources: self.resources,
            interchange: self.interchange,
        }
    }
}

impl DocumentV02Owned {
    fn into_document(self) -> Result<ChemSemaDocument, String> {
        if self.format.name != FORMAT_NAME
            || self.format.version != CURRENT_VERSION
            || !self.format.unit.eq_ignore_ascii_case(FORMAT_UNIT)
            || self.format.profile != FORMAT_PROFILE
        {
            return Err("CCJS v0.2 format header is not canonical".to_string());
        }
        let objects = rebuild_scene(self.entities.scene, &self.hierarchy)?;
        validate_reading_order(&self.orders, &objects)?;
        Ok(ChemSemaDocument {
            format: FormatInfo {
                name: FORMAT_NAME.to_string(),
                version: CURRENT_VERSION.to_string(),
                unit: FORMAT_UNIT.to_string(),
            },
            document: self.document,
            style: self.style,
            styles: self.styles,
            objects,
            links: self.relations,
            orders: self.orders,
            logical_objects: self.logical_objects,
            reaction_schemes: self.reaction_schemes,
            chemical_properties: self.chemical_properties,
            resources: self.resources,
            interchange: self.interchange,
        })
    }
}

fn flatten_scene(objects: &[SceneObject]) -> Result<(Vec<SceneObject>, HierarchyV02), String> {
    fn visit(
        objects: &[SceneObject],
        parent: Option<&str>,
        scene: &mut Vec<SceneObject>,
        children: &mut BTreeMap<String, Vec<String>>,
        seen: &mut BTreeSet<String>,
    ) -> Result<(), String> {
        for object in objects {
            if object.id.is_empty() {
                return Err("scene object id must not be empty".to_string());
            }
            if !seen.insert(object.id.clone()) {
                return Err(format!("duplicate scene object id '{}'", object.id));
            }
            if let Some(parent_id) = parent {
                children
                    .entry(parent_id.to_string())
                    .or_default()
                    .push(object.id.clone());
            }
            let nested = object.children.clone();
            if !nested.is_empty() && object.object_type != "group" {
                return Err(format!(
                    "non-group scene object '{}' cannot own children",
                    object.id
                ));
            }
            let mut flat = object.clone();
            flat.children.clear();
            scene.push(flat);
            visit(&nested, Some(&object.id), scene, children, seen)?;
        }
        Ok(())
    }

    let roots = objects.iter().map(|object| object.id.clone()).collect();
    let mut scene = Vec::new();
    let mut children = BTreeMap::new();
    let mut seen = BTreeSet::new();
    visit(objects, None, &mut scene, &mut children, &mut seen)?;
    Ok((scene, HierarchyV02 { roots, children }))
}

fn rebuild_scene(
    scene: Vec<SceneObject>,
    hierarchy: &HierarchyV02,
) -> Result<Vec<SceneObject>, String> {
    let mut entities = BTreeMap::new();
    for object in scene {
        if object.id.is_empty() {
            return Err("scene object id must not be empty".to_string());
        }
        if !object.children.is_empty() {
            return Err(format!(
                "CCJS v0.2 scene entity '{}' must be flat; use hierarchy.children",
                object.id
            ));
        }
        let id = object.id.clone();
        if entities.insert(id.clone(), object).is_some() {
            return Err(format!("duplicate scene entity id '{id}'"));
        }
    }

    let mut membership = BTreeMap::<String, String>::new();
    for root in &hierarchy.roots {
        if !entities.contains_key(root) {
            return Err(format!("hierarchy root '{root}' does not exist"));
        }
        if membership
            .insert(root.clone(), "<root>".to_string())
            .is_some()
        {
            return Err(format!(
                "scene entity '{root}' appears more than once in hierarchy"
            ));
        }
    }
    for (parent, child_ids) in &hierarchy.children {
        let parent_object = entities
            .get(parent)
            .ok_or_else(|| format!("hierarchy parent '{parent}' does not exist"))?;
        if parent_object.object_type != "group" {
            return Err(format!("hierarchy parent '{parent}' is not a group"));
        }
        for child in child_ids {
            if !entities.contains_key(child) {
                return Err(format!(
                    "hierarchy child '{child}' of '{parent}' does not exist"
                ));
            }
            if let Some(previous) = membership.insert(child.clone(), parent.clone()) {
                return Err(format!(
                    "scene entity '{child}' has multiple containers ('{previous}' and '{parent}')"
                ));
            }
        }
    }
    for id in entities.keys() {
        if !membership.contains_key(id) {
            return Err(format!("scene entity '{id}' is not placed in hierarchy"));
        }
    }

    fn build(
        id: &str,
        entities: &BTreeMap<String, SceneObject>,
        hierarchy: &HierarchyV02,
        visiting: &mut BTreeSet<String>,
        built: &mut BTreeSet<String>,
    ) -> Result<SceneObject, String> {
        if !visiting.insert(id.to_string()) {
            return Err(format!("hierarchy cycle detected at '{id}'"));
        }
        let mut object = entities
            .get(id)
            .cloned()
            .ok_or_else(|| format!("scene entity '{id}' does not exist"))?;
        if let Some(child_ids) = hierarchy.children.get(id) {
            for child in child_ids {
                object
                    .children
                    .push(build(child, entities, hierarchy, visiting, built)?);
            }
        }
        visiting.remove(id);
        built.insert(id.to_string());
        Ok(object)
    }

    let mut roots = Vec::with_capacity(hierarchy.roots.len());
    let mut visiting = BTreeSet::new();
    let mut built = BTreeSet::new();
    for root in &hierarchy.roots {
        roots.push(build(
            root,
            &entities,
            hierarchy,
            &mut visiting,
            &mut built,
        )?);
    }
    if built.len() != entities.len() {
        return Err("hierarchy contains entities unreachable from roots".to_string());
    }
    Ok(roots)
}

fn validate_reading_order(orders: &DocumentOrders, objects: &[SceneObject]) -> Result<(), String> {
    let mut scene_ids = BTreeSet::new();
    fn collect(objects: &[SceneObject], ids: &mut BTreeSet<String>) {
        for object in objects {
            ids.insert(object.id.clone());
            collect(&object.children, ids);
        }
    }
    collect(objects, &mut scene_ids);
    let mut seen = BTreeSet::new();
    for id in &orders.reading {
        if !scene_ids.contains(id) {
            return Err(format!(
                "reading order references missing scene entity '{id}'"
            ));
        }
        if !seen.insert(id) {
            return Err(format!(
                "reading order contains duplicate scene entity '{id}'"
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_serialization_is_flat_v02() {
        let document = ChemSemaDocument::blank();
        let value = serde_json::to_value(&document).expect("document serializes");

        assert_eq!(
            value.pointer("/format/version"),
            Some(&Value::String("0.2".into()))
        );
        assert_eq!(
            value.pointer("/format/profile"),
            Some(&Value::String("snapshot".into()))
        );
        assert!(value.get("objects").is_none());
        assert!(value.get("links").is_none());
        assert_eq!(
            value
                .pointer("/entities/scene")
                .and_then(Value::as_array)
                .map(Vec::len),
            Some(1)
        );
        assert_eq!(
            value.pointer("/hierarchy/roots/0").and_then(Value::as_str),
            Some("obj_editor_molecule")
        );
        assert!(value.pointer("/entities/scene/0/children").is_none());
    }

    #[test]
    fn canonical_round_trip_preserves_document() {
        let mut expected = ChemSemaDocument::blank();
        expected.orders.reading = vec!["obj_editor_molecule".to_string()];
        let json = serde_json::to_string(&expected).expect("document serializes");
        let actual = crate::parse_document_json(&json).expect("v0.2 document reopens");
        assert_eq!(actual, expected);
    }

    #[test]
    fn v01_is_migrated_and_written_as_v02() {
        let document = ChemSemaDocument::blank();
        let mut value = serde_json::to_value(&document).expect("document serializes");
        value["format"]["version"] = Value::String("0.1".to_string());
        value["format"]
            .as_object_mut()
            .expect("format object")
            .remove("profile");
        let scene = value["entities"]["scene"].take();
        let relations = value["relations"].take();
        let root = value.as_object_mut().expect("document object");
        root.insert("objects".to_string(), scene);
        root.insert("links".to_string(), relations);
        root.remove("entities");
        root.remove("hierarchy");
        root.remove("orders");

        let migrated = crate::parse_document_json(&value.to_string()).expect("v0.1 migrates");
        let canonical = serde_json::to_value(&migrated).expect("migrated document serializes");
        assert_eq!(
            canonical.pointer("/format/version").and_then(Value::as_str),
            Some("0.2")
        );
        assert!(canonical.get("entities").is_some());
    }

    #[test]
    fn invalid_headers_are_rejected() {
        let document = ChemSemaDocument::blank();
        let mut value = serde_json::to_value(&document).expect("document serializes");
        value["format"]["name"] = Value::String("other".to_string());
        assert!(crate::parse_document_json(&value.to_string())
            .expect_err("wrong format name must fail")
            .contains("format name"));

        value["format"]["name"] = Value::String("chemsema".to_string());
        value["format"]["version"] = Value::String("9.9".to_string());
        assert!(crate::parse_document_json(&value.to_string())
            .expect_err("unknown version must fail")
            .contains("version"));
    }

    #[test]
    fn hierarchy_must_place_every_entity_exactly_once() {
        let document = ChemSemaDocument::blank();
        let mut value = serde_json::to_value(&document).expect("document serializes");
        value["hierarchy"]["roots"] = Value::Array(Vec::new());
        let error = crate::parse_document_json(&value.to_string())
            .expect_err("orphan scene entity must fail");
        assert!(error.contains("not placed in hierarchy"), "{error}");
    }

    #[test]
    fn reading_order_references_existing_unique_scene_entities() {
        let document = ChemSemaDocument::blank();
        let mut value = serde_json::to_value(&document).expect("document serializes");
        value["orders"]["reading"] = json!(["missing"]);
        let error = crate::parse_document_json(&value.to_string())
            .expect_err("missing reading-order target must fail");
        assert!(error.contains("reading order"), "{error}");
    }

    #[test]
    fn canonical_output_conforms_to_published_json_schema() {
        let schema: Value =
            serde_json::from_str(include_str!("../../../../schemas/ccjs-v0.2.schema.json"))
                .expect("published CCJS schema parses");
        let validator = jsonschema::validator_for(&schema).expect("published schema compiles");
        let instance =
            serde_json::to_value(ChemSemaDocument::blank()).expect("canonical document serializes");
        let errors = validator
            .iter_errors(&instance)
            .map(|error| error.to_string())
            .collect::<Vec<_>>();
        assert!(errors.is_empty(), "schema errors: {errors:#?}");
    }

    #[test]
    fn published_schema_accepts_every_engine_scene_object_kind() {
        let schema: Value =
            serde_json::from_str(include_str!("../../../../schemas/ccjs-v0.2.schema.json"))
                .expect("published CCJS schema parses");
        let validator = jsonschema::validator_for(&schema).expect("published schema compiles");
        let mut document = ChemSemaDocument::blank();
        let template = document.objects[0].clone();
        document.objects = crate::SceneObjectKind::ALL
            .iter()
            .enumerate()
            .map(|(index, kind)| {
                let mut object = template.clone();
                object.id = format!("kind_{index}");
                object.object_type = kind.as_str().to_string();
                object.children.clear();
                object
            })
            .collect();
        let instance = serde_json::to_value(document).expect("kind corpus serializes");
        let errors = validator
            .iter_errors(&instance)
            .map(|error| error.to_string())
            .collect::<Vec<_>>();
        assert!(errors.is_empty(), "schema errors: {errors:#?}");
    }
}
