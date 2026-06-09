use super::*;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

const EFFECTIVE_NUM_HYDROGENS_META: &str = "effectiveNumHydrogens";

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectionChemistrySummary {
    pub formula: String,
    pub formula_weight: f64,
    pub exact_mass: f64,
    pub atom_count: u32,
}

#[derive(Debug, Clone, Copy)]
struct ElementMass {
    average: f64,
    exact: f64,
}

impl Engine {
    pub fn selection_chemistry_summary_json(&self) -> String {
        serde_json::to_string(&self.selection_chemistry_summary())
            .unwrap_or_else(|_| "null".to_string())
    }

    pub fn selection_chemistry_summary(&self) -> Option<SelectionChemistrySummary> {
        let selected_node_ids = selected_atom_node_ids(&self.state.selection);
        if selected_node_ids.is_empty() {
            return None;
        }
        let entry = self.state.document.editable_fragment()?;
        let mut counts = BTreeMap::<String, u32>::new();
        let mut formula_weight = 0.0;
        let mut exact_mass = 0.0;
        let mut atom_count = 0_u32;

        for node in &entry.fragment.nodes {
            if !selected_node_ids.contains(node.id.as_str()) {
                continue;
            }
            if node.is_placeholder || node.atomic_number == 0 || node.element.trim().is_empty() {
                continue;
            }
            let Some(mass) = node_element_mass(node.element.as_str(), node.atomic_number) else {
                continue;
            };
            add_formula_count(&mut counts, node.element.as_str(), 1);
            formula_weight += mass.average;
            exact_mass += mass.exact;
            atom_count += 1;

            let hydrogens = u32::from(summary_implicit_hydrogen_count(entry.fragment, node));
            if hydrogens > 0 {
                let hydrogen = hydrogen_mass();
                add_formula_count(&mut counts, "H", hydrogens);
                formula_weight += hydrogen.average * hydrogens as f64;
                exact_mass += hydrogen.exact * hydrogens as f64;
                atom_count += hydrogens;
            }
        }

        if atom_count == 0 {
            return None;
        }

        Some(SelectionChemistrySummary {
            formula: render_formula(&counts),
            formula_weight,
            exact_mass,
            atom_count,
        })
    }
}

fn selected_atom_node_ids(selection: &SelectionState) -> BTreeSet<&str> {
    selection
        .nodes
        .iter()
        .chain(selection.label_nodes.iter())
        .map(String::as_str)
        .collect()
}

fn add_formula_count(counts: &mut BTreeMap<String, u32>, symbol: &str, count: u32) {
    if count == 0 {
        return;
    }
    *counts.entry(symbol.to_string()).or_insert(0) += count;
}

fn summary_implicit_hydrogen_count(fragment: &crate::MoleculeFragment, node: &crate::Node) -> u8 {
    if node.is_placeholder || node.atomic_number == 1 {
        return 0;
    }
    if let Some(value) = node
        .meta
        .get(EFFECTIVE_NUM_HYDROGENS_META)
        .and_then(serde_json::Value::as_u64)
    {
        return value.min(u64::from(u8::MAX)) as u8;
    }
    if node.atomic_number != 6 {
        return node.num_hydrogens;
    }
    let connection_order: i32 = fragment
        .bonds
        .iter()
        .filter(|bond| bond.begin == node.id || bond.end == node.id)
        .map(|bond| i32::from(bond.order.max(1)))
        .sum();
    (4 - connection_order - node.charge.abs()).clamp(0, 4) as u8
}

fn render_formula(counts: &BTreeMap<String, u32>) -> String {
    let mut symbols = Vec::new();
    if counts.contains_key("C") {
        symbols.push("C");
        if counts.contains_key("H") {
            symbols.push("H");
        }
        for symbol in counts.keys() {
            if symbol != "C" && symbol != "H" {
                symbols.push(symbol);
            }
        }
    } else {
        symbols.extend(counts.keys().map(String::as_str));
    }
    symbols
        .into_iter()
        .map(|symbol| {
            let count = counts.get(symbol).copied().unwrap_or(0);
            if count <= 1 {
                symbol.to_string()
            } else {
                format!("{symbol}{count}")
            }
        })
        .collect::<Vec<_>>()
        .join("")
}

fn hydrogen_mass() -> ElementMass {
    ElementMass {
        average: 1.008,
        exact: 1.007_825_032_23,
    }
}

fn node_element_mass(symbol: &str, atomic_number: u8) -> Option<ElementMass> {
    if symbol == "D" {
        return Some(ElementMass {
            average: 2.014_101_778_12,
            exact: 2.014_101_778_12,
        });
    }
    element_mass(atomic_number)
}

fn element_mass(atomic_number: u8) -> Option<ElementMass> {
    let (average, exact) = match atomic_number {
        1 => (1.008, 1.007_825_032_23),
        2 => (4.002_602, 4.002_603_254_13),
        3 => (6.94, 7.016_003_436_6),
        4 => (9.012_183_1, 9.012_183_065),
        5 => (10.81, 11.009_305_36),
        6 => (12.011, 12.0),
        7 => (14.007, 14.003_074_004_43),
        8 => (15.999, 15.994_914_619_57),
        9 => (18.998_403_163, 18.998_403_162_73),
        10 => (20.179_7, 19.992_440_176_2),
        11 => (22.989_769_28, 22.989_769_282),
        12 => (24.305, 23.985_041_697),
        13 => (26.981_538_5, 26.981_538_53),
        14 => (28.085, 27.976_926_534_65),
        15 => (30.973_761_998, 30.973_761_998_42),
        16 => (32.06, 31.972_071_174_4),
        17 => (35.45, 34.968_852_682),
        18 => (39.948, 39.962_383_123_7),
        19 => (39.098_3, 38.963_706_486_4),
        20 => (40.078, 39.962_590_863),
        21 => (44.955_908, 44.955_908_28),
        22 => (47.867, 47.947_941_98),
        23 => (50.941_5, 50.943_957_04),
        24 => (51.996_1, 51.940_506_23),
        25 => (54.938_044, 54.938_043_91),
        26 => (55.845, 55.934_936_33),
        27 => (58.933_194, 58.933_194_29),
        28 => (58.693_4, 57.935_342_41),
        29 => (63.546, 62.929_597_72),
        30 => (65.38, 63.929_142_01),
        31 => (69.723, 68.925_573_5),
        32 => (72.63, 73.921_177_761),
        33 => (74.921_595, 74.921_594_57),
        34 => (78.971, 79.916_521_8),
        35 => (79.904, 78.918_337_6),
        36 => (83.798, 83.911_497_728),
        37 => (85.467_8, 84.911_789_738),
        38 => (87.62, 87.905_612_5),
        39 => (88.905_84, 88.905_840_3),
        40 => (91.224, 89.904_697_7),
        41 => (92.906_37, 92.906_373),
        42 => (95.95, 97.905_404_82),
        43 => (98.0, 96.906_366_7),
        44 => (101.07, 101.904_344_1),
        45 => (102.905_5, 102.905_498),
        46 => (106.42, 105.903_480_4),
        47 => (107.868_2, 106.905_091_6),
        48 => (112.414, 113.903_365_09),
        49 => (114.818, 114.903_878_8),
        50 => (118.71, 119.902_201_63),
        51 => (121.76, 120.903_812),
        52 => (127.6, 129.906_222_748),
        53 => (126.904_47, 126.904_471_9),
        54 => (131.293, 131.904_155_086),
        55 => (132.905_451_96, 132.905_451_961),
        56 => (137.327, 137.905_247),
        57 => (138.905_47, 138.906_356_3),
        58 => (140.116, 139.905_448_4),
        59 => (140.907_66, 140.907_657_6),
        60 => (144.242, 141.907_729),
        61 => (145.0, 144.912_755_9),
        62 => (150.36, 151.919_739_7),
        63 => (151.964, 152.921_238),
        64 => (157.25, 157.924_112_3),
        65 => (158.925_35, 158.925_354_7),
        66 => (162.5, 163.929_181_9),
        67 => (164.930_33, 164.930_328_8),
        68 => (167.259, 165.930_299_5),
        69 => (168.934_22, 168.934_217_9),
        70 => (173.045, 173.938_866_4),
        71 => (174.966_8, 174.940_775_2),
        72 => (178.49, 179.946_557),
        73 => (180.947_88, 180.947_995_8),
        74 => (183.84, 183.950_930_92),
        75 => (186.207, 186.955_750_1),
        76 => (190.23, 191.961_477),
        77 => (192.217, 192.962_921_6),
        78 => (195.084, 194.964_791_7),
        79 => (196.966_569, 196.966_568_79),
        80 => (200.592, 201.970_643_4),
        81 => (204.38, 204.974_427_8),
        82 => (207.2, 207.976_652_5),
        83 => (208.980_4, 208.980_399_1),
        84 => (209.0, 208.982_430_8),
        85 => (210.0, 209.987_147_9),
        86 => (222.0, 221.970_29),
        87 => (223.0, 223.019_736),
        88 => (226.0, 226.025_41),
        89 => (227.0, 227.027_752),
        90 => (232.037_7, 232.038_055_8),
        91 => (231.035_88, 231.035_884),
        92 => (238.028_91, 238.050_788_4),
        93 => (237.0, 237.048_173_6),
        94 => (244.0, 244.064_205_3),
        95 => (243.0, 243.061_381_3),
        96 => (247.0, 247.070_354),
        97 => (247.0, 247.070_307),
        98 => (251.0, 251.079_588),
        99 => (252.0, 252.082_98),
        100 => (257.0, 257.095_106),
        101 => (258.0, 258.098_431),
        102 => (259.0, 259.101_03),
        103 => (262.0, 262.109_61),
        104 => (267.0, 267.121_79),
        105 => (270.0, 270.133_36),
        106 => (271.0, 271.133_47),
        107 => (270.0, 270.133_62),
        108 => (277.0, 277.151_9),
        109 => (278.0, 278.156_31),
        110 => (281.0, 281.164_51),
        111 => (282.0, 282.169_12),
        112 => (285.0, 285.177_12),
        113 => (286.0, 286.182_21),
        114 => (289.0, 289.190_42),
        115 => (290.0, 290.196_75),
        116 => (293.0, 293.204_49),
        117 => (294.0, 294.210_46),
        118 => (294.0, 294.213_92),
        _ => return None,
    };
    Some(ElementMass { average, exact })
}
