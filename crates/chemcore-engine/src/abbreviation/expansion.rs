use super::*;

#[derive(Default)]
struct ExpansionBuilder {
    atom_counts: BTreeMap<String, usize>,
    atoms: Vec<Value>,
    bonds: Vec<Value>,
}

struct FragmentExpansion {
    left_atom: String,
    right_atom: Option<String>,
    complete: bool,
}

impl ExpansionBuilder {
    fn add_atom(&mut self, element: &str, num_hydrogens: Option<u8>) -> String {
        self.add_labeled_atom(element, num_hydrogens, None)
    }

    fn add_labeled_atom(
        &mut self,
        element: &str,
        num_hydrogens: Option<u8>,
        label: Option<&str>,
    ) -> String {
        self.add_labeled_atom_with_charge(element, num_hydrogens, label, None)
    }

    fn add_labeled_atom_with_charge(
        &mut self,
        element: &str,
        num_hydrogens: Option<u8>,
        label: Option<&str>,
        formal_charge: Option<i8>,
    ) -> String {
        let key = element.to_ascii_lowercase();
        let key = if key.chars().all(|ch| ch.is_ascii_alphanumeric()) {
            key
        } else {
            "x".to_string()
        };
        let next = self.atom_counts.entry(key.clone()).or_insert(0);
        *next += 1;
        let id = format!("{key}{next}");
        let mut atom = json!({
            "id": id,
            "element": element,
        });
        if let Some(num_hydrogens) = num_hydrogens {
            atom["numHydrogens"] = json!(num_hydrogens);
        }
        if let Some(label) = label {
            atom["label"] = json!(label);
        }
        if let Some(formal_charge) = formal_charge {
            atom["formalCharge"] = json!(formal_charge);
        }
        self.atoms.push(atom);
        id
    }

    fn add_bond(&mut self, begin: &str, end: &str, order: u8) {
        self.bonds.push(json!({
            "begin": begin,
            "end": end,
            "order": order,
        }));
    }
}

pub(super) fn expansion_for_recognition(recognition: &AbbreviationRecognition) -> Value {
    let mut builder = ExpansionBuilder::default();
    let connection_kind = if recognition.kind == "bridge-fragment" {
        "bridge"
    } else {
        "terminal"
    };
    let mut complete = true;
    let attachments = if recognition.kind == "valence-fragment" {
        build_valence_expansion(&mut builder, &recognition.components, &mut complete)
    } else if connection_kind == "bridge" {
        build_bridge_expansion(&mut builder, &recognition.components, &mut complete)
    } else {
        build_terminal_expansion(&mut builder, &recognition.components, &mut complete)
    };
    json!({
        "schema": "chemcore.functionalGroupExpansion.v1",
        "connectionKind": connection_kind,
        "complete": complete,
        "atoms": builder.atoms,
        "bonds": builder.bonds,
        "attachments": attachments,
    })
}

fn build_valence_expansion(
    builder: &mut ExpansionBuilder,
    components: &[AbbreviationComponent],
    complete: &mut bool,
) -> Vec<Value> {
    let mut component_atoms: Vec<Option<String>> = vec![None; components.len()];
    for (index, component) in components.iter().enumerate() {
        let atom_id = if component.kind == "terminal" {
            let fragment = expand_component(builder, component);
            *complete &= fragment.complete;
            fragment.left_atom
        } else {
            builder.add_labeled_atom_with_charge(
                &component.left_anchor,
                None,
                None,
                component.formal_charge,
            )
        };
        component_atoms[index] = Some(atom_id);
    }
    for (index, component) in components.iter().enumerate() {
        let Some(parent_index) = component.parent_index else {
            continue;
        };
        let (Some(parent_atom), Some(child_atom)) = (
            component_atoms
                .get(parent_index)
                .and_then(|atom| atom.as_deref()),
            component_atoms.get(index).and_then(|atom| atom.as_deref()),
        ) else {
            continue;
        };
        builder.add_bond(
            parent_atom,
            child_atom,
            component.bond_order_to_parent.unwrap_or(1),
        );
    }
    components
        .iter()
        .position(|component| component.parent_index.is_none())
        .and_then(|index| component_atoms.get(index).and_then(|atom| atom.clone()))
        .map(|atom_id| vec![json!({ "role": "external", "atomId": atom_id })])
        .unwrap_or_default()
}

fn build_terminal_expansion(
    builder: &mut ExpansionBuilder,
    components: &[AbbreviationComponent],
    complete: &mut bool,
) -> Vec<Value> {
    let mut first_atom = None;
    let mut previous_right = None;
    for component in components {
        let fragment = expand_component(builder, component);
        *complete &= fragment.complete;
        if first_atom.is_none() {
            first_atom = Some(fragment.left_atom.clone());
        }
        if let Some(previous) = previous_right.as_deref() {
            builder.add_bond(previous, &fragment.left_atom, 1);
        }
        previous_right = fragment
            .right_atom
            .clone()
            .or_else(|| Some(fragment.left_atom.clone()));
    }
    first_atom
        .map(|atom_id| vec![json!({ "role": "external", "atomId": atom_id })])
        .unwrap_or_default()
}

fn build_bridge_expansion(
    builder: &mut ExpansionBuilder,
    components: &[AbbreviationComponent],
    complete: &mut bool,
) -> Vec<Value> {
    if components.len() == 2
        && components
            .first()
            .is_some_and(|component| component.label == "N")
    {
        let nitrogen = builder.add_atom("N", None);
        let substituent = expand_component(builder, &components[1]);
        *complete &= substituent.complete;
        builder.add_bond(&nitrogen, &substituent.left_atom, 1);
        return vec![
            json!({ "role": "left", "atomId": nitrogen }),
            json!({ "role": "right", "atomId": nitrogen }),
        ];
    }
    let Some(component) = components.first() else {
        return Vec::new();
    };
    let fragment = expand_component(builder, component);
    *complete &= fragment.complete;
    vec![
        json!({ "role": "left", "atomId": fragment.left_atom }),
        json!({ "role": "right", "atomId": fragment.right_atom.unwrap_or(fragment.left_atom) }),
    ]
}

fn expand_component(
    builder: &mut ExpansionBuilder,
    component: &AbbreviationComponent,
) -> FragmentExpansion {
    match component.label.as_str() {
        "CO2" => expand_co2(builder),
        "OCO" => expand_oco(builder),
        "SO2" => expand_sulfur_oxide_linker(builder, 2),
        "SO" => expand_sulfur_oxide_linker(builder, 1),
        "CH2" => single_atom_fragment(builder, "C", Some(2), Some("C"), true),
        "NH" => single_atom_fragment(builder, "N", Some(1), Some("N"), true),
        "CO" => expand_carbonyl_linker(builder),
        "O" => single_atom_fragment(builder, "O", None, Some("O"), true),
        "Me" => expand_alkyl_chain(builder, 1),
        "Et" => expand_alkyl_chain(builder, 2),
        "Pr" => expand_alkyl_chain(builder, 3),
        "nPr" => expand_alkyl_chain(builder, 3),
        "Bu" => expand_alkyl_chain(builder, 4),
        "nBu" => expand_alkyl_chain(builder, 4),
        "iPr" => expand_isopropyl(builder),
        "iBu" => expand_isobutyl(builder),
        "sBu" => expand_sec_butyl(builder),
        "tBu" => expand_tert_butyl(builder),
        "Ph" => expand_phenyl(builder),
        "PhCOOH" => expand_benzoic_acid_substituent(builder),
        "Bn" => expand_benzyl(builder),
        "Bz" => expand_benzoyl(builder),
        "Ac" => expand_acetyl(builder),
        "Piv" => expand_pivaloyl(builder),
        "CHO" => expand_formyl(builder),
        "CN" => expand_cyano(builder),
        "NCO" => expand_linear_three_atom(builder, "N", "C", "O", 2, 2),
        "NCS" => expand_linear_three_atom(builder, "N", "C", "S", 2, 2),
        "SCN" => expand_linear_three_atom(builder, "S", "C", "N", 1, 3),
        "NO2" => expand_nitro(builder),
        "N3" => expand_linear_three_atom(builder, "N", "N", "N", 1, 3),
        "H" | "F" | "Cl" | "Br" | "I" => {
            single_atom_fragment(builder, &component.label, None, None, true)
        }
        "OH" => single_atom_fragment(builder, "O", Some(1), None, true),
        "NH2" => single_atom_fragment(builder, "N", Some(2), None, true),
        "Ts" => expand_aryl_sulfonyl(builder, Some("Me"), None),
        "Bs" => expand_aryl_sulfonyl(builder, None, Some("Br")),
        "Ms" => expand_methanesulfonyl(builder),
        "Tf" => expand_triflyl(builder),
        "SO3H" => expand_sulfonic_acid(builder),
        "SO2H" => expand_sulfinic_acid(builder),
        "SO3" => expand_sulfur_oxo_terminal(builder, 3),
        "PO3H2" => expand_phosphonic_acid(builder),
        "Boc" => expand_boc(builder),
        "Cbz" => expand_cbz(builder),
        "Fmoc" => expand_fmoc(builder),
        "TMS" => expand_silyl(builder, 3, 0, 0),
        "TBDMS" => expand_silyl(builder, 2, 0, 1),
        "TBDPS" => expand_silyl(builder, 0, 2, 1),
        "CCl3" => expand_trihalomethyl(builder, "Cl"),
        "CF3" => expand_trihalomethyl(builder, "F"),
        "CPh3" => expand_triphenylmethyl(builder),
        "Cy" => expand_cyclohexyl(builder),
        "Mes" => expand_mesityl(builder),
        "NHPh" => expand_anilino(builder),
        _ => expand_opaque_component(builder, component),
    }
}

fn single_atom_fragment(
    builder: &mut ExpansionBuilder,
    element: &str,
    num_hydrogens: Option<u8>,
    right_element: Option<&str>,
    complete: bool,
) -> FragmentExpansion {
    let atom = builder.add_atom(element, num_hydrogens);
    FragmentExpansion {
        left_atom: atom.clone(),
        right_atom: right_element.map(|_| atom),
        complete,
    }
}

fn expand_opaque_component(
    builder: &mut ExpansionBuilder,
    component: &AbbreviationComponent,
) -> FragmentExpansion {
    let element = if component
        .left_anchor
        .chars()
        .all(|ch| ch.is_ascii_alphabetic())
    {
        component.left_anchor.as_str()
    } else {
        "*"
    };
    let atom = builder.add_labeled_atom(element, None, Some(&component.label));
    FragmentExpansion {
        left_atom: atom,
        right_atom: None,
        complete: false,
    }
}

fn expand_alkyl_chain(builder: &mut ExpansionBuilder, len: usize) -> FragmentExpansion {
    let mut atoms = Vec::new();
    for index in 0..len {
        let hydrogens = if len == 1 {
            3
        } else if index + 1 == len {
            3
        } else {
            2
        };
        atoms.push(builder.add_atom("C", Some(hydrogens)));
    }
    for pair in atoms.windows(2) {
        builder.add_bond(&pair[0], &pair[1], 1);
    }
    FragmentExpansion {
        left_atom: atoms[0].clone(),
        right_atom: None,
        complete: true,
    }
}

fn expand_isopropyl(builder: &mut ExpansionBuilder) -> FragmentExpansion {
    let center = builder.add_atom("C", Some(1));
    for _ in 0..2 {
        let methyl = builder.add_atom("C", Some(3));
        builder.add_bond(&center, &methyl, 1);
    }
    FragmentExpansion {
        left_atom: center,
        right_atom: None,
        complete: true,
    }
}

fn expand_isobutyl(builder: &mut ExpansionBuilder) -> FragmentExpansion {
    let left = builder.add_atom("C", Some(2));
    let center = builder.add_atom("C", Some(1));
    builder.add_bond(&left, &center, 1);
    for _ in 0..2 {
        let methyl = builder.add_atom("C", Some(3));
        builder.add_bond(&center, &methyl, 1);
    }
    FragmentExpansion {
        left_atom: left,
        right_atom: None,
        complete: true,
    }
}

fn expand_sec_butyl(builder: &mut ExpansionBuilder) -> FragmentExpansion {
    let left = builder.add_atom("C", Some(1));
    let methyl = builder.add_atom("C", Some(3));
    let methylene = builder.add_atom("C", Some(2));
    let terminal = builder.add_atom("C", Some(3));
    builder.add_bond(&left, &methyl, 1);
    builder.add_bond(&left, &methylene, 1);
    builder.add_bond(&methylene, &terminal, 1);
    FragmentExpansion {
        left_atom: left,
        right_atom: None,
        complete: true,
    }
}

fn expand_tert_butyl(builder: &mut ExpansionBuilder) -> FragmentExpansion {
    let center = builder.add_atom("C", Some(0));
    for _ in 0..3 {
        let methyl = builder.add_atom("C", Some(3));
        builder.add_bond(&center, &methyl, 1);
    }
    FragmentExpansion {
        left_atom: center,
        right_atom: None,
        complete: true,
    }
}

fn expand_phenyl(builder: &mut ExpansionBuilder) -> FragmentExpansion {
    expand_phenyl_ring(builder).0
}

fn expand_phenyl_ring(builder: &mut ExpansionBuilder) -> (FragmentExpansion, Vec<String>) {
    let mut ring = Vec::new();
    for index in 0..6 {
        ring.push(builder.add_atom("C", Some(if index == 0 { 0 } else { 1 })));
    }
    for index in 0..6 {
        let next = (index + 1) % 6;
        builder.add_bond(
            &ring[index],
            &ring[next],
            if index % 2 == 0 { 2 } else { 1 },
        );
    }
    (
        FragmentExpansion {
            left_atom: ring[0].clone(),
            right_atom: None,
            complete: true,
        },
        ring,
    )
}

fn expand_cyclohexyl(builder: &mut ExpansionBuilder) -> FragmentExpansion {
    let mut ring = Vec::new();
    for index in 0..6 {
        ring.push(builder.add_atom("C", Some(if index == 0 { 1 } else { 2 })));
    }
    for index in 0..6 {
        builder.add_bond(&ring[index], &ring[(index + 1) % 6], 1);
    }
    FragmentExpansion {
        left_atom: ring[0].clone(),
        right_atom: None,
        complete: true,
    }
}

fn expand_benzoic_acid_substituent(builder: &mut ExpansionBuilder) -> FragmentExpansion {
    let (phenyl, ring) = expand_phenyl_ring(builder);
    let carbon = builder.add_atom("C", Some(0));
    let oxo = builder.add_atom("O", Some(0));
    let hydroxyl = builder.add_atom("O", Some(1));
    builder.add_bond(&ring[0], &carbon, 1);
    builder.add_bond(&carbon, &oxo, 2);
    builder.add_bond(&carbon, &hydroxyl, 1);
    phenyl
}

fn expand_carbonyl_linker(builder: &mut ExpansionBuilder) -> FragmentExpansion {
    let carbon = builder.add_atom("C", Some(0));
    let oxygen = builder.add_atom("O", Some(0));
    builder.add_bond(&carbon, &oxygen, 2);
    FragmentExpansion {
        left_atom: carbon.clone(),
        right_atom: Some(carbon),
        complete: true,
    }
}

fn expand_co2(builder: &mut ExpansionBuilder) -> FragmentExpansion {
    let carbon = builder.add_atom("C", Some(0));
    let oxo = builder.add_atom("O", Some(0));
    let oxygen = builder.add_atom("O", Some(0));
    builder.add_bond(&carbon, &oxo, 2);
    builder.add_bond(&carbon, &oxygen, 1);
    FragmentExpansion {
        left_atom: carbon,
        right_atom: Some(oxygen),
        complete: true,
    }
}

fn expand_oco(builder: &mut ExpansionBuilder) -> FragmentExpansion {
    let oxygen = builder.add_atom("O", Some(0));
    let carbon = builder.add_atom("C", Some(0));
    let oxo = builder.add_atom("O", Some(0));
    builder.add_bond(&oxygen, &carbon, 1);
    builder.add_bond(&carbon, &oxo, 2);
    FragmentExpansion {
        left_atom: oxygen,
        right_atom: Some(carbon),
        complete: true,
    }
}

fn expand_sulfur_oxide_linker(
    builder: &mut ExpansionBuilder,
    oxo_count: usize,
) -> FragmentExpansion {
    let sulfur = builder.add_atom("S", Some(0));
    for _ in 0..oxo_count {
        let oxygen = builder.add_atom("O", Some(0));
        builder.add_bond(&sulfur, &oxygen, 2);
    }
    FragmentExpansion {
        left_atom: sulfur.clone(),
        right_atom: Some(sulfur),
        complete: true,
    }
}

fn expand_sulfur_oxo_terminal(
    builder: &mut ExpansionBuilder,
    oxo_count: usize,
) -> FragmentExpansion {
    let sulfur = builder.add_atom("S", Some(0));
    for _ in 0..oxo_count {
        let oxygen = builder.add_atom("O", Some(0));
        builder.add_bond(&sulfur, &oxygen, 2);
    }
    FragmentExpansion {
        left_atom: sulfur,
        right_atom: None,
        complete: true,
    }
}

fn expand_acetyl(builder: &mut ExpansionBuilder) -> FragmentExpansion {
    let carbon = builder.add_atom("C", Some(0));
    let oxygen = builder.add_atom("O", Some(0));
    let methyl = builder.add_atom("C", Some(3));
    builder.add_bond(&carbon, &oxygen, 2);
    builder.add_bond(&carbon, &methyl, 1);
    FragmentExpansion {
        left_atom: carbon,
        right_atom: None,
        complete: true,
    }
}

fn expand_benzoyl(builder: &mut ExpansionBuilder) -> FragmentExpansion {
    let carbon = builder.add_atom("C", Some(0));
    let oxygen = builder.add_atom("O", Some(0));
    let phenyl = expand_phenyl(builder);
    builder.add_bond(&carbon, &oxygen, 2);
    builder.add_bond(&carbon, &phenyl.left_atom, 1);
    FragmentExpansion {
        left_atom: carbon,
        right_atom: None,
        complete: true,
    }
}

fn expand_pivaloyl(builder: &mut ExpansionBuilder) -> FragmentExpansion {
    let carbon = builder.add_atom("C", Some(0));
    let oxygen = builder.add_atom("O", Some(0));
    let tert_butyl = expand_tert_butyl(builder);
    builder.add_bond(&carbon, &oxygen, 2);
    builder.add_bond(&carbon, &tert_butyl.left_atom, 1);
    FragmentExpansion {
        left_atom: carbon,
        right_atom: None,
        complete: true,
    }
}

fn expand_formyl(builder: &mut ExpansionBuilder) -> FragmentExpansion {
    let carbon = builder.add_atom("C", Some(1));
    let oxygen = builder.add_atom("O", Some(0));
    builder.add_bond(&carbon, &oxygen, 2);
    FragmentExpansion {
        left_atom: carbon,
        right_atom: None,
        complete: true,
    }
}

fn expand_cyano(builder: &mut ExpansionBuilder) -> FragmentExpansion {
    let carbon = builder.add_atom("C", Some(0));
    let nitrogen = builder.add_atom("N", Some(0));
    builder.add_bond(&carbon, &nitrogen, 3);
    FragmentExpansion {
        left_atom: carbon,
        right_atom: None,
        complete: true,
    }
}

fn expand_linear_three_atom(
    builder: &mut ExpansionBuilder,
    first_element: &str,
    second_element: &str,
    third_element: &str,
    first_order: u8,
    second_order: u8,
) -> FragmentExpansion {
    let first = builder.add_atom(first_element, Some(0));
    let second = builder.add_atom(second_element, Some(0));
    let third = builder.add_atom(third_element, Some(0));
    builder.add_bond(&first, &second, first_order);
    builder.add_bond(&second, &third, second_order);
    FragmentExpansion {
        left_atom: first,
        right_atom: None,
        complete: true,
    }
}

fn expand_nitro(builder: &mut ExpansionBuilder) -> FragmentExpansion {
    let nitrogen = builder.add_atom("N", Some(0));
    let oxo = builder.add_atom("O", Some(0));
    let oxygen = builder.add_atom("O", Some(0));
    builder.add_bond(&nitrogen, &oxo, 2);
    builder.add_bond(&nitrogen, &oxygen, 1);
    FragmentExpansion {
        left_atom: nitrogen,
        right_atom: None,
        complete: true,
    }
}

fn expand_benzyl(builder: &mut ExpansionBuilder) -> FragmentExpansion {
    let carbon = builder.add_atom("C", Some(2));
    let phenyl = expand_phenyl(builder);
    builder.add_bond(&carbon, &phenyl.left_atom, 1);
    FragmentExpansion {
        left_atom: carbon,
        right_atom: None,
        complete: true,
    }
}

fn expand_methanesulfonyl(builder: &mut ExpansionBuilder) -> FragmentExpansion {
    let sulfur = expand_sulfur_oxide_linker(builder, 2);
    let methyl = builder.add_atom("C", Some(3));
    builder.add_bond(&sulfur.left_atom, &methyl, 1);
    FragmentExpansion {
        left_atom: sulfur.left_atom,
        right_atom: None,
        complete: true,
    }
}

fn expand_triflyl(builder: &mut ExpansionBuilder) -> FragmentExpansion {
    let sulfur = expand_sulfur_oxide_linker(builder, 2);
    let cf3 = expand_trihalomethyl(builder, "F");
    builder.add_bond(&sulfur.left_atom, &cf3.left_atom, 1);
    FragmentExpansion {
        left_atom: sulfur.left_atom,
        right_atom: None,
        complete: true,
    }
}

fn expand_aryl_sulfonyl(
    builder: &mut ExpansionBuilder,
    para_methyl: Option<&str>,
    para_halogen: Option<&str>,
) -> FragmentExpansion {
    let sulfur = expand_sulfur_oxide_linker(builder, 2);
    let (aryl, ring) = expand_phenyl_ring(builder);
    builder.add_bond(&sulfur.left_atom, &aryl.left_atom, 1);
    if para_methyl.is_some() {
        let methyl = builder.add_atom("C", Some(3));
        builder.add_bond(&ring[3], &methyl, 1);
    }
    if let Some(halogen) = para_halogen {
        let atom = builder.add_atom(halogen, Some(0));
        builder.add_bond(&ring[3], &atom, 1);
    }
    FragmentExpansion {
        left_atom: sulfur.left_atom,
        right_atom: None,
        complete: true,
    }
}

fn expand_sulfonic_acid(builder: &mut ExpansionBuilder) -> FragmentExpansion {
    let sulfur = expand_sulfur_oxide_linker(builder, 2);
    let oxygen = builder.add_atom("O", Some(1));
    builder.add_bond(&sulfur.left_atom, &oxygen, 1);
    FragmentExpansion {
        left_atom: sulfur.left_atom,
        right_atom: None,
        complete: true,
    }
}

fn expand_sulfinic_acid(builder: &mut ExpansionBuilder) -> FragmentExpansion {
    let sulfur = expand_sulfur_oxide_linker(builder, 1);
    let oxygen = builder.add_atom("O", Some(1));
    builder.add_bond(&sulfur.left_atom, &oxygen, 1);
    FragmentExpansion {
        left_atom: sulfur.left_atom,
        right_atom: None,
        complete: true,
    }
}

fn expand_phosphonic_acid(builder: &mut ExpansionBuilder) -> FragmentExpansion {
    let phosphorus = builder.add_atom("P", Some(0));
    let oxo = builder.add_atom("O", Some(0));
    builder.add_bond(&phosphorus, &oxo, 2);
    for _ in 0..2 {
        let oxygen = builder.add_atom("O", Some(1));
        builder.add_bond(&phosphorus, &oxygen, 1);
    }
    FragmentExpansion {
        left_atom: phosphorus,
        right_atom: None,
        complete: true,
    }
}

fn expand_boc(builder: &mut ExpansionBuilder) -> FragmentExpansion {
    let carbon = builder.add_atom("C", Some(0));
    let oxo = builder.add_atom("O", Some(0));
    let oxygen = builder.add_atom("O", Some(0));
    let tert_butyl = expand_tert_butyl(builder);
    builder.add_bond(&carbon, &oxo, 2);
    builder.add_bond(&carbon, &oxygen, 1);
    builder.add_bond(&oxygen, &tert_butyl.left_atom, 1);
    FragmentExpansion {
        left_atom: carbon,
        right_atom: None,
        complete: true,
    }
}

fn expand_cbz(builder: &mut ExpansionBuilder) -> FragmentExpansion {
    let carbon = builder.add_atom("C", Some(0));
    let oxo = builder.add_atom("O", Some(0));
    let oxygen = builder.add_atom("O", Some(0));
    let methylene = builder.add_atom("C", Some(2));
    let phenyl = expand_phenyl(builder);
    builder.add_bond(&carbon, &oxo, 2);
    builder.add_bond(&carbon, &oxygen, 1);
    builder.add_bond(&oxygen, &methylene, 1);
    builder.add_bond(&methylene, &phenyl.left_atom, 1);
    FragmentExpansion {
        left_atom: carbon,
        right_atom: None,
        complete: true,
    }
}

fn expand_fmoc(builder: &mut ExpansionBuilder) -> FragmentExpansion {
    let carbon = builder.add_atom("C", Some(0));
    let oxo = builder.add_atom("O", Some(0));
    let oxygen = builder.add_atom("O", Some(0));
    let methylene = builder.add_atom("C", Some(2));
    let fluorenyl = builder.add_labeled_atom("C", None, Some("fluorenyl"));
    builder.add_bond(&carbon, &oxo, 2);
    builder.add_bond(&carbon, &oxygen, 1);
    builder.add_bond(&oxygen, &methylene, 1);
    builder.add_bond(&methylene, &fluorenyl, 1);
    FragmentExpansion {
        left_atom: carbon,
        right_atom: None,
        complete: false,
    }
}

fn expand_silyl(
    builder: &mut ExpansionBuilder,
    methyl_count: usize,
    phenyl_count: usize,
    tert_butyl_count: usize,
) -> FragmentExpansion {
    let silicon = builder.add_atom("Si", Some(0));
    for _ in 0..methyl_count {
        let methyl = builder.add_atom("C", Some(3));
        builder.add_bond(&silicon, &methyl, 1);
    }
    for _ in 0..phenyl_count {
        let phenyl = expand_phenyl(builder);
        builder.add_bond(&silicon, &phenyl.left_atom, 1);
    }
    for _ in 0..tert_butyl_count {
        let tert_butyl = expand_tert_butyl(builder);
        builder.add_bond(&silicon, &tert_butyl.left_atom, 1);
    }
    FragmentExpansion {
        left_atom: silicon,
        right_atom: None,
        complete: true,
    }
}

fn expand_trihalomethyl(builder: &mut ExpansionBuilder, halogen: &str) -> FragmentExpansion {
    let carbon = builder.add_atom("C", Some(0));
    for _ in 0..3 {
        let atom = builder.add_atom(halogen, Some(0));
        builder.add_bond(&carbon, &atom, 1);
    }
    FragmentExpansion {
        left_atom: carbon,
        right_atom: None,
        complete: true,
    }
}

fn expand_triphenylmethyl(builder: &mut ExpansionBuilder) -> FragmentExpansion {
    let carbon = builder.add_atom("C", Some(0));
    for _ in 0..3 {
        let phenyl = expand_phenyl(builder);
        builder.add_bond(&carbon, &phenyl.left_atom, 1);
    }
    FragmentExpansion {
        left_atom: carbon,
        right_atom: None,
        complete: true,
    }
}

fn expand_mesityl(builder: &mut ExpansionBuilder) -> FragmentExpansion {
    let (phenyl, ring) = expand_phenyl_ring(builder);
    for atom_id in [&ring[1], &ring[3], &ring[5]] {
        let methyl = builder.add_atom("C", Some(3));
        builder.add_bond(atom_id, &methyl, 1);
    }
    phenyl
}

fn expand_anilino(builder: &mut ExpansionBuilder) -> FragmentExpansion {
    let nitrogen = builder.add_atom("N", Some(1));
    let phenyl = expand_phenyl(builder);
    builder.add_bond(&nitrogen, &phenyl.left_atom, 1);
    FragmentExpansion {
        left_atom: nitrogen,
        right_atom: None,
        complete: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(recognition: &AbbreviationRecognition) -> Vec<&str> {
        recognition
            .components
            .iter()
            .map(|component| component.label.as_str())
            .collect()
    }

    #[test]
    fn parses_terminal_abbreviations() {
        let recognition = recognize_abbreviation_label("Boc").unwrap();
        assert_eq!(recognition.kind, "terminal-fragment");
        assert_eq!(recognition.canonical_label, "Boc");
        assert_eq!(labels(&recognition), vec!["Boc"]);

        let recognition = recognize_abbreviation_label("FMOC").unwrap();
        assert_eq!(recognition.canonical_label, "Fmoc");

        let benzoic_acid = recognize_abbreviation_label("PhCOOH").unwrap();
        assert_eq!(benzoic_acid.canonical_label, "PhCOOH");
        assert_eq!(benzoic_acid.components[0].name, "benzoic acid substituent");
        let benzoic_acid_meta = recognized_abbreviation_meta("PhCOOH").unwrap();
        let benzoic_acid_expansion = benzoic_acid_meta["expansion"].as_object().unwrap();
        assert_eq!(benzoic_acid_expansion["complete"], true);
        assert_eq!(benzoic_acid_expansion["attachments"][0]["atomId"], "c1");
        assert_eq!(benzoic_acid_expansion["atoms"].as_array().unwrap().len(), 9);
        assert!(benzoic_acid_expansion["bonds"]
            .as_array()
            .unwrap()
            .iter()
            .any(|bond| bond["begin"] == "c7" && bond["end"] == "o1" && bond["order"] == 2));

        let azide = recognize_abbreviation_label("N3").unwrap();
        assert_eq!(azide.canonical_label, "N3");
        assert_eq!(azide.components[0].name, "azido");

        let tert_butyl = recognize_abbreviation_label("t-Bu").unwrap();
        assert_eq!(tert_butyl.canonical_label, "tBu");
        assert_eq!(labels(&tert_butyl), vec!["tBu"]);
        assert!(recognized_abbreviation_uses_whole_label_layout("t-Bu", 1));

        let normal_butyl = recognize_abbreviation_label("n-Bu").unwrap();
        assert_eq!(normal_butyl.canonical_label, "nBu");
        assert!(recognized_abbreviation_uses_whole_label_layout("nBu", 1));
        assert!(recognized_abbreviation_uses_whole_label_layout("iPr", 1));
        assert!(!recognized_abbreviation_uses_whole_label_layout("CF3", 1));
    }

    #[test]
    fn parses_composite_abbreviations() {
        let co2et = recognize_abbreviation_label("CO2Et").unwrap();
        assert_eq!(co2et.kind, "valence-fragment");
        assert_eq!(labels(&co2et), vec!["C", "O", "O", "Et"]);
        assert_eq!(co2et.formula, "-C(=O)OEt");
        assert_eq!(
            labels(&recognize_abbreviation_label("COOCH2CH2CH3").unwrap()),
            vec!["C", "O", "O", "C", "H", "H", "C", "H", "H", "Me"]
        );
        assert_eq!(
            labels(&recognize_abbreviation_label("COOSO2Me").unwrap()),
            vec!["C", "O", "O", "S", "O", "O", "Me"]
        );
        assert_eq!(
            labels(&recognize_abbreviation_label("CO2Boc").unwrap()),
            vec!["C", "O", "O", "Boc"]
        );
        assert_eq!(
            labels(&recognize_abbreviation_label("NHTs").unwrap()),
            vec!["N", "H", "Ts"]
        );
    }

    #[test]
    fn rejects_open_fragments_without_terminal_fragment() {
        assert!(recognize_abbreviation_label("CO").is_none());
        assert!(recognize_abbreviation_label("CO2").is_none());
        assert!(recognize_abbreviation_label("COO").is_none());
        assert!(recognize_abbreviation_label("SO2").is_none());
        assert!(recognize_abbreviation_label("SO").is_none());
        assert!(recognize_abbreviation_label("CH2").is_none());
        assert!(recognize_abbreviation_label("NH").is_none());
    }

    #[test]
    fn recognizes_documented_first_stage_functional_groups() {
        let terminal_labels = [
            "Ac", "Bn", "Boc", "Bu", "Bz", "Cbz", "C2H5", "CCl3", "CF3", "CN", "CO2Et", "CO2H",
            "CO2Me", "CONH2", "CO2Pr", "CO2tBu", "Cp", "CPh3", "Cy", "Et", "FMOC", "iBu", "Indole",
            "iPr", "Me", "Mes", "Ms", "NCO", "NCS", "NHPh", "NO2", "OAc", "OCF3", "OCN", "OEt",
            "OMe", "Ph", "PhCOOH", "Piv", "PO2", "PO3", "PO3H2", "PO4", "PO4H2", "Pr", "sBu",
            "SCN", "SO2Cl", "SO2H", "SO3", "SO3H", "SO4", "SO4H", "ster", "TBDMS", "TBDPS", "tBu",
            "Tf", "TMS", "Tos", "Ts",
        ];
        for label in terminal_labels {
            assert!(
                recognize_abbreviation_label(label).is_some(),
                "{label} should be recognized in terminal context"
            );
        }

        assert!(recognize_abbreviation_label_for_connection_count("SO2", 2).is_some());

        let chemcanvas_aliases = [
            ("COOH", "CO2H"),
            ("COCH3", "COMe"),
            ("COBr", "COBr"),
            ("OCH3", "OMe"),
            ("OBs", "OBs"),
            ("OTs", "OTs"),
        ];
        for (label, canonical_label) in chemcanvas_aliases {
            let recognition = recognize_abbreviation_label(label).unwrap();
            assert_eq!(recognition.canonical_label, canonical_label);
        }
    }

    #[test]
    fn parses_valence_formula_like_labels() {
        let recognition = recognize_abbreviation_label("CH2COOCH2SO2NHCl").unwrap();
        assert_eq!(recognition.kind, "valence-fragment");
        assert_eq!(
            labels(&recognition),
            vec!["C", "H", "H", "C", "O", "O", "C", "H", "H", "S", "O", "O", "N", "H", "Cl"]
        );
        let meta = recognized_abbreviation_meta("CH2COOCH2SO2NHCl").unwrap();
        assert_eq!(meta["source"], "valence-parser");
        let expansion = meta["expansion"].as_object().unwrap();
        assert!(expansion["bonds"]
            .as_array()
            .unwrap()
            .iter()
            .any(|bond| bond["begin"] == "c2" && bond["end"] == "o1" && bond["order"] == 2));
        assert!(expansion["bonds"]
            .as_array()
            .unwrap()
            .iter()
            .any(|bond| bond["begin"] == "s1" && bond["end"] == "o3" && bond["order"] == 2));
        assert!(expansion["bonds"]
            .as_array()
            .unwrap()
            .iter()
            .any(|bond| bond["begin"] == "n1" && bond["end"] == "cl1" && bond["order"] == 1));
    }

    #[test]
    fn valence_parser_allows_one_open_valence_only_inside_parenthesized_groups() {
        assert!(recognize_abbreviation_label("B(OH)2").is_some());
        assert!(recognize_abbreviation_label("CH2(OH)").is_some());
        assert!(recognize_abbreviation_label("CH2(NH)").is_none());
        assert!(recognize_abbreviation_label("CH2N").is_none());
    }

    #[test]
    fn zero_connection_chemical_text_validates_without_functional_group_expansion() {
        for label in ["ArB(OH)2", "Cu(CH3CN)4PF6"] {
            let recognition = recognize_abbreviation_label_for_connection_count(label, 0).unwrap();
            assert_eq!(recognition.kind, "chemical-text");
            assert!(recognition.components.is_empty());

            let meta = recognized_abbreviation_meta_for_connection_count(label, 0).unwrap();
            assert_eq!(meta["status"], "recognized");
            assert_eq!(meta["groupKind"], "chemical-text");
            assert!(meta.get("expansion").is_none());
        }
    }

    #[test]
    fn valence_parser_treats_named_groups_as_monovalent_terminators() {
        let recognition = recognize_abbreviation_label("CH2Boc").unwrap();
        assert_eq!(recognition.kind, "valence-fragment");
        assert_eq!(labels(&recognition), vec!["C", "H", "H", "Boc"]);
        assert_eq!(recognition.components[3].kind, "terminal");
        assert_eq!(recognition.components[3].bond_order_to_parent, Some(1));
        let expansion = recognized_abbreviation_meta("CH2Boc").unwrap()["expansion"].clone();
        assert!(expansion["bonds"]
            .as_array()
            .unwrap()
            .iter()
            .any(|bond| bond["begin"] == "c1" && bond["end"] == "c2" && bond["order"] == 1));
    }

    #[test]
    fn valence_parser_applies_charged_boron_nitrogen_and_oxygen_exceptions() {
        let cases = [
            ("BH3", "b1", -1),
            ("NH3", "n1", 1),
            ("OH2", "o1", 1),
            ("OH3", "o1", 2),
        ];
        for (label, atom_id, formal_charge) in cases {
            let meta = recognized_abbreviation_meta(label).unwrap();
            let atoms = meta["expansion"]["atoms"].as_array().unwrap();
            let atom = atoms
                .iter()
                .find(|atom| atom["id"] == atom_id)
                .expect("charged atom should exist");
            assert_eq!(atom["formalCharge"], formal_charge);
        }

        for invalid in ["BCl3", "NMe4", "OCl3", "OCl4"] {
            assert!(
                recognize_abbreviation_label(invalid).is_none(),
                "{invalid} should not use charged-valence exceptions"
            );
        }
    }

    #[test]
    fn parses_two_connection_bridge_abbreviations() {
        let so2 = recognize_abbreviation_label_for_connection_count("SO2", 2).unwrap();
        assert_eq!(so2.kind, "bridge-fragment");
        assert_eq!(labels(&so2), vec!["SO2"]);

        let so = recognize_abbreviation_label_for_connection_count("SO", 2).unwrap();
        assert_eq!(so.kind, "bridge-fragment");
        assert_eq!(so.formula, "-S(=O)-");
        assert_eq!(labels(&so), vec!["SO"]);

        let nh = recognize_abbreviation_label_for_connection_count("NH", 2).unwrap();
        assert_eq!(nh.kind, "bridge-fragment");
        assert_eq!(labels(&nh), vec!["NH"]);

        let ntos = recognize_abbreviation_label_for_connection_count("NTos", 2).unwrap();
        assert_eq!(ntos.canonical_label, "NTs");
        assert_eq!(labels(&ntos), vec!["N", "Ts"]);

        let ncl = recognize_abbreviation_label_for_connection_count("NCl", 2).unwrap();
        assert_eq!(labels(&ncl), vec!["N", "Cl"]);
    }

    #[test]
    fn rejects_terminal_abbreviations_in_two_connection_context() {
        assert!(recognize_abbreviation_label_for_connection_count("Boc", 2).is_none());
        assert!(recognize_abbreviation_label_for_connection_count("CO2Et", 2).is_none());
    }
}
