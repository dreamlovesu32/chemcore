use crate::{round2, Point};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const DEFAULT_PAGE_WIDTH: f64 = 1200.0;
pub const DEFAULT_PAGE_HEIGHT: f64 = 800.0;
pub const DEFAULT_BOND_LENGTH: f64 = 36.0;
pub const DEFAULT_BOND_STROKE: f64 = 2.125;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChemcoreDocument {
    pub format: FormatInfo,
    pub document: DocumentInfo,
    pub styles: BTreeMap<String, Style>,
    pub objects: Vec<SceneObject>,
    pub resources: BTreeMap<String, Resource>,
}

impl ChemcoreDocument {
    pub fn blank() -> Self {
        let mut styles = BTreeMap::new();
        styles.insert(
            "style_molecule_default".to_string(),
            Style::Molecule {
                stroke: "#000000".to_string(),
                stroke_width: DEFAULT_BOND_STROKE,
                font_family: "Arial".to_string(),
                font_size: 11.0,
            },
        );

        let mut resources = BTreeMap::new();
        resources.insert(
            "mol_editor".to_string(),
            Resource {
                resource_type: "molecule_fragment2d".to_string(),
                encoding: "chemcore.molecule.fragment2d".to_string(),
                data: MoleculeFragment::blank(),
            },
        );

        Self {
            format: FormatInfo {
                name: "chemcore".to_string(),
                version: "0.1".to_string(),
            },
            document: DocumentInfo {
                id: "doc_editor_untitled".to_string(),
                title: "Untitled".to_string(),
                page: Page {
                    width: DEFAULT_PAGE_WIDTH,
                    height: DEFAULT_PAGE_HEIGHT,
                    background: "#ffffff".to_string(),
                },
            },
            styles,
            objects: vec![SceneObject {
                id: "obj_editor_molecule".to_string(),
                object_type: "molecule".to_string(),
                name: "molecule".to_string(),
                visible: true,
                locked: false,
                z_index: 10,
                transform: Transform::identity(),
                style_ref: "style_molecule_default".to_string(),
                payload: MoleculeObjectPayload {
                    resource_ref: "mol_editor".to_string(),
                    bbox: [0.0, 0.0, DEFAULT_PAGE_WIDTH, DEFAULT_PAGE_HEIGHT],
                },
            }],
            resources,
        }
    }

    pub fn editable_fragment_mut(&mut self) -> Option<EditableFragmentMut<'_>> {
        let object_index = self
            .objects
            .iter()
            .position(|object| object.object_type == "molecule")?;
        let resource_ref = self.objects[object_index].payload.resource_ref.clone();
        let resource = self.resources.get_mut(&resource_ref)?;
        Some(EditableFragmentMut {
            object: &mut self.objects[object_index],
            fragment: &mut resource.data,
        })
    }

    pub fn editable_fragment(&self) -> Option<EditableFragment<'_>> {
        let object = self
            .objects
            .iter()
            .find(|object| object.object_type == "molecule")?;
        let resource = self.resources.get(&object.payload.resource_ref)?;
        Some(EditableFragment {
            object,
            fragment: &resource.data,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentInfo {
    pub id: String,
    pub title: String,
    pub page: Page,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    pub width: f64,
    pub height: f64,
    pub background: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Style {
    #[serde(rename = "molecule", rename_all = "camelCase")]
    Molecule {
        stroke: String,
        stroke_width: f64,
        font_family: String,
        font_size: f64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneObject {
    pub id: String,
    #[serde(rename = "type")]
    pub object_type: String,
    pub name: String,
    pub visible: bool,
    pub locked: bool,
    pub z_index: i32,
    pub transform: Transform,
    pub style_ref: String,
    pub payload: MoleculeObjectPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoleculeObjectPayload {
    pub resource_ref: String,
    pub bbox: [f64; 4],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Resource {
    #[serde(rename = "type")]
    pub resource_type: String,
    pub encoding: String,
    pub data: MoleculeFragment,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoleculeFragment {
    pub schema: String,
    pub bbox: [f64; 4],
    pub nodes: Vec<Node>,
    pub bonds: Vec<Bond>,
}

impl MoleculeFragment {
    pub fn blank() -> Self {
        Self {
            schema: "chemcore.molecule.fragment2d".to_string(),
            bbox: [0.0, 0.0, DEFAULT_PAGE_WIDTH, DEFAULT_PAGE_HEIGHT],
            nodes: Vec::new(),
            bonds: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    pub id: String,
    pub element: String,
    pub atomic_number: u8,
    pub position: [f64; 2],
    pub charge: i32,
    pub num_hydrogens: u8,
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
        }
    }

    pub fn point(&self) -> Point {
        Point::new(self.position[0], self.position[1])
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bond {
    pub id: String,
    pub begin: String,
    pub end: String,
    pub order: u8,
    pub stroke_width: f64,
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
        let mut max_x = DEFAULT_PAGE_WIDTH;
        let mut max_y = DEFAULT_PAGE_HEIGHT;
        for node in &self.fragment.nodes {
            max_x = max_x.max(node.position[0] + 8.0);
            max_y = max_y.max(node.position[1] + 8.0);
        }
        self.fragment.bbox = [0.0, 0.0, round2(max_x), round2(max_y)];
        self.object.payload.bbox = self.fragment.bbox;
    }
}
