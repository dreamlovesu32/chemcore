use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

#[path = "abbreviation/expansion.rs"]
mod expansion;
#[path = "abbreviation/valence.rs"]
mod valence;

use self::expansion::expansion_for_recognition;
use self::valence::{parse_chemical_text_label, parse_valence_terminal_label};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AbbreviationComponent {
    pub label: String,
    pub kind: String,
    pub name: String,
    pub structure: String,
    pub left_anchor: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub right_attachment: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_index: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bond_order_to_parent: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formal_charge: Option<i8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AbbreviationRecognition {
    pub label: String,
    pub canonical_label: String,
    pub kind: String,
    pub formula: String,
    pub anchor_atom: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub components: Vec<AbbreviationComponent>,
}

#[derive(Debug, Clone, Copy)]
struct FragmentDef {
    label: &'static str,
    aliases: &'static [&'static str],
    kind: &'static str,
    name: &'static str,
    structure: &'static str,
    left_anchor: &'static str,
    right_attachment: Option<&'static str>,
}

impl FragmentDef {
    fn component(self, input_label: &str) -> AbbreviationComponent {
        AbbreviationComponent {
            label: canonical_label_for(input_label, self.label),
            kind: self.kind.to_string(),
            name: self.name.to_string(),
            structure: self.structure.to_string(),
            left_anchor: self.left_anchor.to_string(),
            right_attachment: self.right_attachment.map(ToString::to_string),
            parent_index: None,
            bond_order_to_parent: None,
            formal_charge: None,
        }
    }

    fn matches(self, text: &str) -> bool {
        self.label == text || self.aliases.contains(&text)
    }
}

const OPEN_FRAGMENTS: &[FragmentDef] = &[
    FragmentDef {
        label: "CO2",
        aliases: &["COO"],
        kind: "linker",
        name: "ester/carboxyl linker",
        structure: "-C(=O)O-",
        left_anchor: "C",
        right_attachment: Some("O"),
    },
    FragmentDef {
        label: "OCO",
        aliases: &[],
        kind: "linker",
        name: "reverse ester linker",
        structure: "-O-C(=O)-",
        left_anchor: "O",
        right_attachment: Some("C"),
    },
    FragmentDef {
        label: "SO2",
        aliases: &[],
        kind: "linker",
        name: "sulfonyl linker",
        structure: "-S(=O)2-",
        left_anchor: "S",
        right_attachment: Some("S"),
    },
    FragmentDef {
        label: "SO",
        aliases: &[],
        kind: "linker",
        name: "sulfinyl linker",
        structure: "-S(=O)-",
        left_anchor: "S",
        right_attachment: Some("S"),
    },
    FragmentDef {
        label: "CH2",
        aliases: &[],
        kind: "linker",
        name: "methylene linker",
        structure: "-CH2-",
        left_anchor: "C",
        right_attachment: Some("C"),
    },
    FragmentDef {
        label: "NH",
        aliases: &[],
        kind: "linker",
        name: "imino linker",
        structure: "-NH-",
        left_anchor: "N",
        right_attachment: Some("N"),
    },
    FragmentDef {
        label: "CO",
        aliases: &[],
        kind: "linker",
        name: "carbonyl linker",
        structure: "-C(=O)-",
        left_anchor: "C",
        right_attachment: Some("C"),
    },
    FragmentDef {
        label: "O",
        aliases: &[],
        kind: "linker",
        name: "oxy linker",
        structure: "-O-",
        left_anchor: "O",
        right_attachment: Some("O"),
    },
];

const N_BRIDGE_FRAGMENT: FragmentDef = FragmentDef {
    label: "N",
    aliases: &[],
    kind: "bridge",
    name: "substituted nitrogen bridge",
    structure: "-N(-)-",
    left_anchor: "N",
    right_attachment: Some("N"),
};

const TERMINAL_FRAGMENTS: &[FragmentDef] = &[
    terminal("R", &[], "R group / generic substituent", "R", "R"),
    terminal(
        "Ar",
        &[],
        "aryl group / generic aromatic substituent",
        "Ar",
        "Ar",
    ),
    terminal("Me", &["CH3"], "methyl", "-CH3", "C"),
    terminal("Et", &["C2H5"], "ethyl", "-CH2CH3", "C"),
    terminal("Pr", &[], "propyl", "-CH2CH2CH3", "C"),
    terminal("nPr", &["n-Pr"], "n-propyl", "-CH2CH2CH3", "C"),
    terminal("iPr", &["i-Pr"], "isopropyl", "-CH(CH3)2", "C"),
    terminal("Bu", &[], "butyl", "-CH2CH2CH2CH3", "C"),
    terminal("nBu", &["n-Bu"], "n-butyl", "-CH2CH2CH2CH3", "C"),
    terminal("iBu", &["i-Bu"], "isobutyl", "-CH2CH(CH3)2", "C"),
    terminal("sBu", &["s-Bu"], "sec-butyl", "-CH(CH3)CH2CH3", "C"),
    terminal("tBu", &["t-Bu"], "tert-butyl", "-C(CH3)3", "C"),
    terminal("Ph", &[], "phenyl", "-C6H5", "C"),
    terminal("PhCOOH", &[], "benzoic acid substituent", "PhCOOH", "C"),
    terminal("Bn", &[], "benzyl", "-CH2Ph", "C"),
    terminal("Bz", &[], "benzoyl", "-C(=O)Ph", "C"),
    terminal("Ac", &[], "acetyl", "-C(=O)CH3", "C"),
    terminal("Piv", &[], "pivaloyl", "-C(=O)tBu", "C"),
    terminal("CHO", &[], "formyl", "-C(=O)H", "C"),
    terminal("CN", &[], "cyano", "-C#N", "C"),
    terminal("NCO", &[], "isocyanato", "-N=C=O", "N"),
    terminal("NCS", &[], "isothiocyanato", "-N=C=S", "N"),
    terminal("SCN", &[], "thiocyanato", "-S-C#N", "S"),
    terminal("NO2", &[], "nitro", "-N(=O)O", "N"),
    terminal("N3", &[], "azido", "-N3", "N"),
    terminal("H", &[], "hydrogen terminator", "-H", "H"),
    terminal("F", &[], "fluoro", "-F", "F"),
    terminal("Cl", &[], "chloro", "-Cl", "Cl"),
    terminal("Br", &[], "bromo", "-Br", "Br"),
    terminal("I", &[], "iodo", "-I", "I"),
    terminal("OH", &[], "hydroxy", "-OH", "O"),
    terminal("NH2", &[], "amino", "-NH2", "N"),
    terminal("Ts", &["Tos"], "tosyl", "-S(=O)2-p-Tol", "S"),
    terminal("Bs", &[], "brosyl", "-S(=O)2-p-BrPh", "S"),
    terminal("Ms", &[], "mesyl", "-S(=O)2CH3", "S"),
    terminal("Tf", &[], "triflyl", "-S(=O)2CF3", "S"),
    terminal("SO3H", &[], "sulfonic acid", "-S(=O)2OH", "S"),
    terminal("SO2H", &[], "sulfinic acid style label", "-S(=O)OH", "S"),
    terminal("SO3", &[], "sulfonate fragment", "-S(=O)3-", "S"),
    terminal("SO4", &[], "sulfate fragment", "SO4", "S"),
    terminal("SO4H", &[], "sulfate monoacid", "SO4H", "O"),
    terminal("PO2", &[], "phosphoryl fragment", "PO2", "P"),
    terminal("PO3", &[], "phosphate fragment", "PO3", "P"),
    terminal("PO3H2", &[], "phosphonic acid", "-P(=O)(OH)2", "P"),
    terminal("PO4", &[], "phosphate", "PO4", "P"),
    terminal("PO4H2", &[], "phosphate acid form", "PO4H2", "O"),
    terminal("Boc", &[], "tert-butyloxycarbonyl", "-C(=O)O-tBu", "C"),
    terminal("Cbz", &[], "benzyloxycarbonyl", "-C(=O)OCH2Ph", "C"),
    terminal(
        "Fmoc",
        &["FMOC"],
        "fluorenylmethoxycarbonyl",
        "-C(=O)OCH2-fluorenyl",
        "C",
    ),
    terminal("TMS", &[], "trimethylsilyl", "-Si(CH3)3", "Si"),
    terminal(
        "TBDMS",
        &[],
        "tert-butyldimethylsilyl",
        "-Si(CH3)2tBu",
        "Si",
    ),
    terminal("TBDPS", &[], "tert-butyldiphenylsilyl", "-Si(Ph)2tBu", "Si"),
    terminal("CCl3", &[], "trichloromethyl", "-CCl3", "C"),
    terminal("CF3", &[], "trifluoromethyl", "-CF3", "C"),
    terminal("CPh3", &[], "trityl", "-CPh3", "C"),
    terminal("Cp", &[], "cyclopentadienyl", "Cp", "C"),
    terminal("Cy", &[], "cyclohexyl", "-C6H11", "C"),
    terminal("Mes", &[], "mesityl", "2,4,6-trimethylphenyl", "C"),
    terminal("NHPh", &[], "anilino", "-NHPh", "N"),
    terminal("Indole", &[], "indolyl / indole template", "Indole", "C"),
    terminal("ster", &[], "generic steric label", "ster", "C"),
];

const fn terminal(
    label: &'static str,
    aliases: &'static [&'static str],
    name: &'static str,
    structure: &'static str,
    left_anchor: &'static str,
) -> FragmentDef {
    FragmentDef {
        label,
        aliases,
        kind: "terminal",
        name,
        structure,
        left_anchor,
        right_attachment: None,
    }
}

pub fn recognize_abbreviation_label(label: &str) -> Option<AbbreviationRecognition> {
    recognize_abbreviation_label_for_connection_count(label, 1)
}

pub fn recognize_abbreviation_label_for_connection_count(
    label: &str,
    connection_count: usize,
) -> Option<AbbreviationRecognition> {
    let trimmed = label.trim();
    if trimmed.is_empty() {
        return None;
    }
    if connection_count == 0 {
        parse_chemical_text_label(trimmed)
    } else if connection_count == 1 {
        parse_valence_terminal_label(trimmed).or_else(|| recognize_terminal(trimmed))
    } else if connection_count == 2 {
        recognize_bridge(trimmed)
    } else {
        None
    }
}

pub fn recognized_abbreviation_meta(label: &str) -> Option<Value> {
    recognized_abbreviation_meta_for_connection_count(label, 1)
}

pub fn recognized_abbreviation_meta_for_connection_count(
    label: &str,
    connection_count: usize,
) -> Option<Value> {
    let recognition = recognize_abbreviation_label_for_connection_count(label, connection_count)?;
    let mut meta = json!({
        "kind": "functional-group",
        "status": "recognized",
        "label": recognition.label,
        "canonicalLabel": recognition.canonical_label,
        "groupKind": recognition.kind,
        "formula": recognition.formula,
        "anchorAtom": recognition.anchor_atom,
        "components": recognition.components,
    });
    if recognition.kind == "valence-fragment" {
        meta["source"] = json!("valence-parser");
    }
    if recognition.kind != "chemical-text" {
        meta["expansion"] = expansion_for_recognition(&recognition);
    }
    Some(meta)
}

pub fn recognized_abbreviation_uses_whole_label_layout(
    label: &str,
    connection_count: usize,
) -> bool {
    recognize_abbreviation_label_for_connection_count(label, connection_count).is_some_and(
        |recognition| {
            recognition.kind == "terminal-fragment"
                && recognition.components.len() == 1
                && canonical_abbreviation_uses_whole_label_layout(&recognition.canonical_label)
        },
    )
}

pub fn canonical_abbreviation_uses_whole_label_layout(canonical_label: &str) -> bool {
    let mut chars = canonical_label.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_lowercase() && chars.any(|character| character.is_ascii_uppercase())
}

pub fn invalid_abbreviation_meta(label: &str) -> Value {
    json!({
        "kind": "functional-label",
        "status": "invalid",
        "label": label.trim(),
    })
}

fn recognize_terminal(label: &str) -> Option<AbbreviationRecognition> {
    let terminal = find_terminal(label)?;
    let component = terminal.component(label);
    Some(AbbreviationRecognition {
        label: label.to_string(),
        canonical_label: component.label.clone(),
        kind: "terminal-fragment".to_string(),
        formula: component.structure.clone(),
        anchor_atom: component.left_anchor.clone(),
        components: vec![component],
    })
}

fn recognize_bridge(label: &str) -> Option<AbbreviationRecognition> {
    if let Some((fragment, matched)) = OPEN_FRAGMENTS
        .iter()
        .find_map(|fragment| fragment.matches(label).then_some((*fragment, label)))
    {
        let mut component = fragment.component(matched);
        component.kind = "bridge".to_string();
        return Some(AbbreviationRecognition {
            label: label.to_string(),
            canonical_label: component.label.clone(),
            kind: "bridge-fragment".to_string(),
            formula: component.structure.clone(),
            anchor_atom: component.left_anchor.clone(),
            components: vec![component],
        });
    }

    let suffix = label.strip_prefix('N')?;
    if suffix.is_empty() {
        return None;
    }
    let terminal = find_terminal(suffix)?;
    let n_component = N_BRIDGE_FRAGMENT.component("N");
    let terminal_component = terminal.component(suffix);
    let canonical_label = format!("N{}", terminal_component.label);
    let formula = format!("-N({})-", terminal_component.label);
    Some(AbbreviationRecognition {
        label: label.to_string(),
        canonical_label,
        kind: "bridge-fragment".to_string(),
        formula,
        anchor_atom: "N".to_string(),
        components: vec![n_component, terminal_component],
    })
}

fn find_terminal(label: &str) -> Option<FragmentDef> {
    TERMINAL_FRAGMENTS
        .iter()
        .copied()
        .find(|fragment| fragment.matches(label))
}

fn canonical_label_for(input_label: &str, canonical: &str) -> String {
    match input_label {
        "COO" => "CO2".to_string(),
        "Tos" => "Ts".to_string(),
        "FMOC" => "Fmoc".to_string(),
        "C2H5" => "Et".to_string(),
        "CH3" => "Me".to_string(),
        "n-Pr" => "nPr".to_string(),
        "i-Pr" => "iPr".to_string(),
        "n-Bu" => "nBu".to_string(),
        "i-Bu" => "iBu".to_string(),
        "s-Bu" => "sBu".to_string(),
        "t-Bu" => "tBu".to_string(),
        _ => canonical.to_string(),
    }
}
