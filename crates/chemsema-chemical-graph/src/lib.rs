//! ChemSema's strict, presentation-independent molecular semantic graph.
//!
//! `ChemicalGraphV2` is an internal interchange representation for naming,
//! prediction, identity comparison, and format adapters. Coordinates, drawing
//! styles, captions, query atoms, polymers, and reactions deliberately live in
//! other document/profile models.

use schemars::{schema_for, JsonSchema};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::collections::{BTreeMap, BTreeSet};

pub const CHEMICAL_GRAPH_V2_SCHEMA: &str = "chemsema-nomenclature/chemical-graph/2";
pub const NORMALIZATION_VERSION: &str = "chemsema-chemical-graph-normalization/1";
pub const NOMENCLATURE_REQUEST_V1_SCHEMA: &str = "chemsema.nomenclature-request.v1";
pub const MAPPING_REPORT_V1_SCHEMA: &str = "chemsema.chemical-graph-mapping-report.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum MoleculeFormatV1 {
    ChemicalGraphV2,
    Cdxml,
    Cdx,
    Smiles,
    SdfV2000,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GraphMappingReportV1 {
    pub schema: String,
    pub target: MoleculeFormatV1,
    pub lossless: bool,
    pub diagnostics: Vec<GraphMappingDiagnosticV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GraphMappingDiagnosticV1 {
    pub code: String,
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NomenclatureRequestV1 {
    pub schema: String,
    pub molecule_id: String,
    pub graph: ChemicalGraphV2,
    pub requested_names: Vec<NomenclatureNameKindV1>,
}

impl NomenclatureRequestV1 {
    pub fn new_preferred_iupac_name(
        molecule_id: impl Into<String>,
        graph: ChemicalGraphV2,
    ) -> Result<Self, String> {
        let request = Self {
            schema: NOMENCLATURE_REQUEST_V1_SCHEMA.to_string(),
            molecule_id: molecule_id.into(),
            graph,
            requested_names: vec![NomenclatureNameKindV1::PreferredIupacName],
        };
        request.validate()?;
        Ok(request)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema != NOMENCLATURE_REQUEST_V1_SCHEMA {
            return Err(format!(
                "unsupported nomenclature request schema '{}'",
                self.schema
            ));
        }
        if self.molecule_id.trim().is_empty() || self.requested_names.is_empty() {
            return Err(
                "nomenclature request requires a molecule id and at least one name kind"
                    .to_string(),
            );
        }
        if self
            .requested_names
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
            .len()
            != self.requested_names.len()
        {
            return Err("nomenclature request repeats a requested name kind".to_string());
        }
        self.graph.validate()
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum NomenclatureNameKindV1 {
    PreferredIupacName,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChemicalGraphV2 {
    pub schema: String,
    /// Informational declaration of the fixed V2 normalization contract.
    ///
    /// The original V2 wire contract predates this object. Its omission is
    /// therefore equivalent to the one supported V2 value; it is not a
    /// per-document switch between identity models.
    #[serde(default)]
    pub semantics: GraphSemanticsV2,
    pub atoms: Vec<AtomV2>,
    pub bonds: Vec<BondV2>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub free_valences: Vec<FreeValenceSiteV2>,
    pub stereo: Vec<StereoElementV2>,
    pub components: Vec<ComponentV2>,
    pub assumptions: Vec<GraphAssumptionV2>,
    pub interactions: Vec<MultiCenterInteractionV2>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GraphSemanticsV2 {
    pub profile: GraphProfileV2,
    pub aromaticity_model: AromaticityModelV2,
    pub hydrogen_model: HydrogenModelV2,
    pub valence_model: ValenceModelV2,
    pub normalization: String,
}

impl Default for GraphSemanticsV2 {
    fn default() -> Self {
        Self {
            profile: GraphProfileV2::MolecularEntity,
            aromaticity_model: AromaticityModelV2::ExplicitAromaticBonds,
            hydrogen_model: HydrogenModelV2::ResolvedCounts,
            valence_model: ValenceModelV2::ChemSema2026,
            normalization: NORMALIZATION_VERSION.to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum GraphProfileV2 {
    MolecularEntity,
    MolecularFragment,
    DiscreteComposition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum AromaticityModelV2 {
    /// Aromatic bonds are authoritative. Kekule and aromatic encodings are not
    /// silently considered identical until an adapter normalizes them.
    ExplicitAromaticBonds,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum HydrogenModelV2 {
    /// Every atom stores the resolved implicit-hydrogen count used by identity.
    ResolvedCounts,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ValenceModelV2 {
    ChemSema2026,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AtomV2 {
    pub id: String,
    pub atomic_number: u8,
    pub isotope: Option<u16>,
    pub formal_charge: i16,
    pub radical: RadicalStateV2,
    pub implicit_hydrogens: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum RadicalStateV2 {
    None,
    Singlet,
    Doublet,
    Triplet,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BondV2 {
    pub id: String,
    pub atoms: [String; 2],
    pub kind: BondKindV2,
    #[schemars(with = "Option<String>")]
    pub dative_direction: Option<DativeDirectionV2>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum BondKindV2 {
    Single,
    Double,
    Triple,
    Quadruple,
    Aromatic,
    Dative,
}

/// An unfilled valence that makes a connected graph a molecular fragment.
///
/// Repeated equal entries are significant: two single free valences on one
/// atom describe a different fragment from one double free valence.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FreeValenceSiteV2 {
    pub atom: String,
    pub order: FreeValenceOrderV2,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum FreeValenceOrderV2 {
    Single,
    Double,
    Triple,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DativeDirectionV2 {
    pub donor: String,
    pub acceptor: String,
}

impl Serialize for DativeDirectionV2 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{}->{}", self.donor, self.acceptor))
    }
}

impl<'de> Deserialize<'de> for DativeDirectionV2 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        let (donor, acceptor) = value
            .split_once("->")
            .filter(|(donor, acceptor)| {
                !donor.trim().is_empty() && !acceptor.trim().is_empty() && !acceptor.contains("->")
            })
            .ok_or_else(|| de::Error::custom("dative direction must be 'donor->acceptor'"))?;
        Ok(Self {
            donor: donor.to_string(),
            acceptor: acceptor.to_string(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum StereoElementV2 {
    Tetrahedral {
        id: String,
        center: String,
        references: [StereoReferenceV2; 4],
        parity: TetrahedralParityV2,
    },
    DoubleBond {
        id: String,
        bond: String,
        left_reference: String,
        right_reference: String,
        relation: DoubleBondRelationV2,
    },
    EnhancedGroup {
        id: String,
        group_kind: EnhancedStereoKindV2,
        members: Vec<String>,
    },
    Extended {
        id: String,
        class: ExtendedStereoClassV2,
        descriptor: ExtendedStereoDescriptorV2,
        carriers: Vec<StereoCarrierV2>,
    },
    Conformation {
        id: String,
        descriptor: ConformationDescriptorV2,
        carriers: Vec<StereoCarrierV2>,
    },
    Unspecified {
        id: String,
        descriptor: UnspecifiedStereoDescriptorV2,
        carriers: Vec<StereoCarrierV2>,
    },
}

impl StereoElementV2 {
    fn id(&self) -> &str {
        match self {
            Self::Tetrahedral { id, .. }
            | Self::DoubleBond { id, .. }
            | Self::EnhancedGroup { id, .. }
            | Self::Extended { id, .. }
            | Self::Conformation { id, .. }
            | Self::Unspecified { id, .. } => id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "camelCase",
    deny_unknown_fields
)]
pub enum StereoReferenceV2 {
    Atom(String),
    ImplicitHydrogen,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum TetrahedralParityV2 {
    Clockwise,
    Anticlockwise,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum DoubleBondRelationV2 {
    Together,
    Opposite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum EnhancedStereoKindV2 {
    Absolute,
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ExtendedStereoClassV2 {
    RelativeConfiguration,
    PseudoasymmetricCenter,
    NontetrahedralCenter,
    PolyhedralCenter,
    Axial,
    Planar,
    Helical,
    Spiro,
    Phane,
    Fullerene,
    RingAssembly,
}

/// Strongly typed descriptor values with a compact string wire form compatible
/// with the original V2 contract.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ExtendedStereoDescriptorV2 {
    R,
    S,
    LowerR,
    LowerS,
    Cis,
    Trans,
    LowerC,
    LowerT,
    Endo,
    Exo,
    Syn,
    Anti,
    SeqCis,
    SeqTrans,
    PolyhedralA,
    PolyhedralC,
    Ra,
    Sa,
    Rp,
    Sp,
    M,
    P,
    Cisoid,
    Transoid,
    AxR,
    AxS,
    AxM,
    AxP,
    PlR,
    PlS,
    SpiroR,
    SpiroS,
    PhaneR,
    PhaneS,
    FullereneR,
    FullereneS,
    FullereneA,
    FullereneC,
    AssemblyR,
    AssemblyS,
    AssemblyE,
    AssemblyZ,
    TrigonalPyramidal,
    TShaped,
    Seesaw,
    TrigonalBipyramidal,
    SquarePyramidal,
    Octahedral,
    Coordination {
        geometry: CoordinationGeometryV2,
        permutation_index: u16,
    },
    HelicalLocants {
        descriptor: HelicalDescriptorV2,
        locants: Vec<u32>,
    },
}

impl Serialize for ExtendedStereoDescriptorV2 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.wire_value())
    }
}

impl<'de> Deserialize<'de> for ExtendedStereoDescriptorV2 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::from_wire_value(&value).map_err(de::Error::custom)
    }
}

impl JsonSchema for ExtendedStereoDescriptorV2 {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "ExtendedStereoDescriptorV2".into()
    }

    fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        String::json_schema(generator)
    }
}

impl ExtendedStereoDescriptorV2 {
    fn wire_value(&self) -> String {
        match self {
            Self::R => "R".to_string(),
            Self::S => "S".to_string(),
            Self::LowerR => "r".to_string(),
            Self::LowerS => "s".to_string(),
            Self::Cis => "cis".to_string(),
            Self::Trans => "trans".to_string(),
            Self::LowerC => "c".to_string(),
            Self::LowerT => "t".to_string(),
            Self::Endo => "endo".to_string(),
            Self::Exo => "exo".to_string(),
            Self::Syn => "syn".to_string(),
            Self::Anti => "anti".to_string(),
            Self::SeqCis => "seqCis".to_string(),
            Self::SeqTrans => "seqTrans".to_string(),
            Self::PolyhedralA => "A".to_string(),
            Self::PolyhedralC => "C".to_string(),
            Self::Ra => "Ra".to_string(),
            Self::Sa => "Sa".to_string(),
            Self::Rp => "Rp".to_string(),
            Self::Sp => "Sp".to_string(),
            Self::M => "M".to_string(),
            Self::P => "P".to_string(),
            Self::Cisoid => "cisoid".to_string(),
            Self::Transoid => "transoid".to_string(),
            Self::AxR => "axR".to_string(),
            Self::AxS => "axS".to_string(),
            Self::AxM => "axM".to_string(),
            Self::AxP => "axP".to_string(),
            Self::PlR => "plR".to_string(),
            Self::PlS => "plS".to_string(),
            Self::SpiroR => "spiroR".to_string(),
            Self::SpiroS => "spiroS".to_string(),
            Self::PhaneR => "phaneR".to_string(),
            Self::PhaneS => "phaneS".to_string(),
            Self::FullereneR => "fullereneR".to_string(),
            Self::FullereneS => "fullereneS".to_string(),
            Self::FullereneA => "fullereneA".to_string(),
            Self::FullereneC => "fullereneC".to_string(),
            Self::AssemblyR => "assemblyR".to_string(),
            Self::AssemblyS => "assemblyS".to_string(),
            Self::AssemblyE => "assemblyE".to_string(),
            Self::AssemblyZ => "assemblyZ".to_string(),
            Self::TrigonalPyramidal => "tp".to_string(),
            Self::TShaped => "tshape".to_string(),
            Self::Seesaw => "seesaw".to_string(),
            Self::TrigonalBipyramidal => "tbpy".to_string(),
            Self::SquarePyramidal => "spy".to_string(),
            Self::Octahedral => "oc".to_string(),
            Self::Coordination {
                geometry,
                permutation_index,
            } => format!(
                "{}-{permutation_index}",
                serde_json::to_value(geometry)
                    .expect("coordination geometry serializes")
                    .as_str()
                    .expect("coordination geometry is a string")
            ),
            Self::HelicalLocants {
                descriptor,
                locants,
            } => format!(
                "{descriptor:?}-{}",
                locants
                    .iter()
                    .map(u32::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            ),
        }
    }

    fn from_wire_value(value: &str) -> Result<Self, String> {
        match value {
            "R" => return Ok(Self::R),
            "S" => return Ok(Self::S),
            "r" => return Ok(Self::LowerR),
            "s" => return Ok(Self::LowerS),
            "cis" => return Ok(Self::Cis),
            "trans" => return Ok(Self::Trans),
            "c" => return Ok(Self::LowerC),
            "t" => return Ok(Self::LowerT),
            "endo" => return Ok(Self::Endo),
            "exo" => return Ok(Self::Exo),
            "syn" => return Ok(Self::Syn),
            "anti" => return Ok(Self::Anti),
            "seqCis" => return Ok(Self::SeqCis),
            "seqTrans" => return Ok(Self::SeqTrans),
            "A" => return Ok(Self::PolyhedralA),
            "C" => return Ok(Self::PolyhedralC),
            "Ra" => return Ok(Self::Ra),
            "Sa" => return Ok(Self::Sa),
            "Rp" => return Ok(Self::Rp),
            "Sp" => return Ok(Self::Sp),
            "M" => return Ok(Self::M),
            "P" => return Ok(Self::P),
            "cisoid" => return Ok(Self::Cisoid),
            "transoid" => return Ok(Self::Transoid),
            "axR" => return Ok(Self::AxR),
            "axS" => return Ok(Self::AxS),
            "axM" => return Ok(Self::AxM),
            "axP" => return Ok(Self::AxP),
            "plR" => return Ok(Self::PlR),
            "plS" => return Ok(Self::PlS),
            "spiroR" => return Ok(Self::SpiroR),
            "spiroS" => return Ok(Self::SpiroS),
            "phaneR" => return Ok(Self::PhaneR),
            "phaneS" => return Ok(Self::PhaneS),
            "fullereneR" => return Ok(Self::FullereneR),
            "fullereneS" => return Ok(Self::FullereneS),
            "fullereneA" => return Ok(Self::FullereneA),
            "fullereneC" => return Ok(Self::FullereneC),
            "assemblyR" => return Ok(Self::AssemblyR),
            "assemblyS" => return Ok(Self::AssemblyS),
            "assemblyE" => return Ok(Self::AssemblyE),
            "assemblyZ" => return Ok(Self::AssemblyZ),
            "tp" => return Ok(Self::TrigonalPyramidal),
            "tshape" => return Ok(Self::TShaped),
            "seesaw" => return Ok(Self::Seesaw),
            "tbpy" => return Ok(Self::TrigonalBipyramidal),
            "spy" => return Ok(Self::SquarePyramidal),
            "oc" => return Ok(Self::Octahedral),
            _ => {}
        }
        if let Some((prefix, locants)) = value.split_once('-') {
            if matches!(prefix, "M" | "P") {
                let locants = locants
                    .split(',')
                    .map(|value| {
                        value
                            .parse::<u32>()
                            .map_err(|_| "extended stereo locants must be positive integers")
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                return Ok(Self::HelicalLocants {
                    descriptor: if prefix == "M" {
                        HelicalDescriptorV2::M
                    } else {
                        HelicalDescriptorV2::P
                    },
                    locants,
                });
            }
        }
        for geometry in [
            CoordinationGeometryV2::SquarePlanar,
            CoordinationGeometryV2::Tetrahedral,
            CoordinationGeometryV2::TrigonalBipyramidal,
            CoordinationGeometryV2::SquarePyramidal,
            CoordinationGeometryV2::Octahedral,
            CoordinationGeometryV2::PentagonalBipyramidal,
            CoordinationGeometryV2::SquareAntiprismatic,
        ] {
            let name = serde_json::to_value(geometry)
                .expect("coordination geometry serializes")
                .as_str()
                .expect("coordination geometry is a string")
                .to_string();
            if let Some(index) = value.strip_prefix(&format!("{name}-")) {
                return Ok(Self::Coordination {
                    geometry,
                    permutation_index: index
                        .parse()
                        .map_err(|_| "coordination permutation index must be an integer")?,
                });
            }
        }
        Err(format!("unknown extended stereo descriptor '{value}'"))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum HelicalDescriptorV2 {
    M,
    P,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum CoordinationGeometryV2 {
    SquarePlanar,
    Tetrahedral,
    TrigonalBipyramidal,
    SquarePyramidal,
    Octahedral,
    PentagonalBipyramidal,
    SquareAntiprismatic,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "camelCase",
    deny_unknown_fields
)]
pub enum StereoCarrierV2 {
    Atom(String),
    Bond(String),
    AtomSet(Vec<String>),
    Axis([String; 2]),
    Plane(Vec<String>),
    Torsion([String; 4]),
    LonePair(String),
    DuplicateAtom(String),
    ConjugatedDoubleBondPair([String; 2]),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ConformationDescriptorV2 {
    Synperiplanar,
    Antiperiplanar,
    Synclinal,
    Anticlinal,
    Eclipsed,
    Staggered,
    Gauche,
    Bisecting,
    Eclipsing,
    SCis,
    STrans,
    Chair,
    Boat,
    TwistBoat,
    Envelope,
    HalfChair,
    Crown,
    Tub,
    In,
    Out,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum UnspecifiedStereoDescriptorV2 {
    LowerXi,
    CapitalXi,
    Wavy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ComponentV2 {
    pub id: String,
    pub atoms: Vec<String>,
    /// V2 represents discrete molecular compositions only. Fractional and
    /// nonstoichiometric substances require a future substance profile.
    pub count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GraphAssumptionV2 {
    pub code: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MultiCenterInteractionV2 {
    pub id: String,
    pub kind: InteractionKindV2,
    pub centers: Vec<InteractionCenterV2>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum InteractionKindV2 {
    Coordination,
    DelocalizedBond,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InteractionCenterV2 {
    pub role: InteractionRoleV2,
    pub atoms: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum InteractionRoleV2 {
    Donor,
    Acceptor,
    Shared,
}

impl ChemicalGraphV2 {
    pub fn assess_mapping_to(
        &self,
        target: MoleculeFormatV1,
    ) -> Result<GraphMappingReportV1, String> {
        self.validate()?;
        let mut diagnostics = Vec::new();
        let mut reject = |code: &str, path: String, message: String| {
            diagnostics.push(GraphMappingDiagnosticV1 {
                code: code.to_string(),
                path,
                message,
            });
        };
        match target {
            MoleculeFormatV1::ChemicalGraphV2 => {}
            MoleculeFormatV1::Cdxml | MoleculeFormatV1::Cdx => {
                assess_free_valence_limits(self, &mut reject, "CDX/CDXML");
                for (index, interaction) in self.interactions.iter().enumerate() {
                    let (code, message) = match interaction.kind {
                        InteractionKindV2::Coordination => (
                            "requires-document-multicenter-encoding",
                            "A graph-only CDX/CDXML mapping cannot preserve this coordination interaction without constructing explicit MultiAttachment proxy geometry.",
                        ),
                        InteractionKindV2::DelocalizedBond => (
                            "unsupported-delocalized-interaction",
                            "CDX/CDXML MultiAttachment expresses directed coordination, not a standalone shared delocalized interaction.",
                        ),
                    };
                    reject(code, format!("/interactions/{index}"), message.to_string());
                }
                for (index, element) in self.stereo.iter().enumerate() {
                    if matches!(
                        element,
                        StereoElementV2::Extended { .. }
                            | StereoElementV2::Conformation { .. }
                            | StereoElementV2::Unspecified { .. }
                    ) {
                        reject(
                            "unsupported-stereo-class",
                            format!("/stereo/{index}"),
                            "The current CDX/CDXML adapter has no verified native mapping for this stereo class.".to_string(),
                        );
                    }
                }
            }
            MoleculeFormatV1::Smiles => {
                assess_linear_notation_limits(self, &mut reject, "SMILES");
            }
            MoleculeFormatV1::SdfV2000 => {
                assess_linear_notation_limits(self, &mut reject, "SDF V2000");
                for (index, bond) in self.bonds.iter().enumerate() {
                    if bond.kind == BondKindV2::Dative {
                        reject(
                            "unsupported-dative-bond",
                            format!("/bonds/{index}"),
                            "The ChemSema SDF V2000 adapter cannot preserve dative direction."
                                .to_string(),
                        );
                    }
                }
            }
        }
        Ok(GraphMappingReportV1 {
            schema: MAPPING_REPORT_V1_SCHEMA.to_string(),
            target,
            lossless: diagnostics.is_empty(),
            diagnostics,
        })
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema != CHEMICAL_GRAPH_V2_SCHEMA {
            return Err(format!("unsupported graph schema '{}'", self.schema));
        }
        if self.semantics.normalization != NORMALIZATION_VERSION {
            return Err(format!(
                "unsupported normalization contract '{}'",
                self.semantics.normalization
            ));
        }
        if self.atoms.is_empty() {
            return Err("graph has no atoms".to_string());
        }
        let atom_ids = unique_nonempty_ids(self.atoms.iter().map(|atom| atom.id.as_str()), "atom")?;
        for atom in &self.atoms {
            if atom.atomic_number == 0 {
                return Err(format!(
                    "atom '{}' is a query or pseudo atom; ChemicalGraphV2 requires a determined element",
                    atom.id
                ));
            }
            if atom.isotope == Some(0) {
                return Err(format!("atom '{}' has zero isotope mass", atom.id));
            }
        }

        let mut bond_ids = BTreeSet::new();
        let mut endpoint_pairs = BTreeSet::new();
        let mut adjacency = atom_ids
            .iter()
            .map(|id| ((*id).to_string(), BTreeSet::new()))
            .collect::<BTreeMap<_, _>>();
        for bond in &self.bonds {
            if bond.id.trim().is_empty() || !bond_ids.insert(bond.id.as_str()) {
                return Err(format!("empty or duplicate bond id '{}'", bond.id));
            }
            if bond.atoms[0] == bond.atoms[1]
                || bond
                    .atoms
                    .iter()
                    .any(|atom| !atom_ids.contains(atom.as_str()))
            {
                return Err(format!("bond '{}' has invalid endpoints", bond.id));
            }
            let mut pair = bond.atoms.clone();
            pair.sort();
            if !endpoint_pairs.insert(pair.clone()) {
                return Err(format!(
                    "duplicate bond between '{}' and '{}'",
                    pair[0], pair[1]
                ));
            }
            match (&bond.kind, &bond.dative_direction) {
                (BondKindV2::Dative, Some(direction))
                    if direction.donor != direction.acceptor
                        && bond.atoms.contains(&direction.donor)
                        && bond.atoms.contains(&direction.acceptor) => {}
                (BondKindV2::Dative, _) => {
                    return Err(format!(
                        "dative bond '{}' requires donor and acceptor endpoints",
                        bond.id
                    ))
                }
                (_, Some(_)) => {
                    return Err(format!(
                        "non-dative bond '{}' carries a dative direction",
                        bond.id
                    ))
                }
                _ => {}
            }
            adjacency
                .get_mut(&bond.atoms[0])
                .expect("validated atom")
                .insert(bond.atoms[1].clone());
            adjacency
                .get_mut(&bond.atoms[1])
                .expect("validated atom")
                .insert(bond.atoms[0].clone());
        }

        self.validate_stereo(&atom_ids, &bond_ids, &adjacency)?;
        self.validate_interactions(&atom_ids, &mut adjacency)?;
        self.validate_free_valences(&atom_ids)?;
        self.validate_components(&atom_ids, &adjacency)?;
        self.validate_assumptions()?;
        Ok(())
    }

    fn validate_stereo(
        &self,
        atom_ids: &BTreeSet<&str>,
        bond_ids: &BTreeSet<&str>,
        adjacency: &BTreeMap<String, BTreeSet<String>>,
    ) -> Result<(), String> {
        let stereo_ids =
            unique_nonempty_ids(self.stereo.iter().map(StereoElementV2::id), "stereo")?;
        let non_group_ids = self
            .stereo
            .iter()
            .filter(|value| !matches!(value, StereoElementV2::EnhancedGroup { .. }))
            .map(StereoElementV2::id)
            .collect::<BTreeSet<_>>();
        let bonds = self
            .bonds
            .iter()
            .map(|bond| (bond.id.as_str(), bond))
            .collect::<BTreeMap<_, _>>();
        let mut enhanced_members = BTreeSet::new();
        for element in &self.stereo {
            match element {
                StereoElementV2::Tetrahedral {
                    id,
                    center,
                    references,
                    ..
                } => {
                    if !atom_ids.contains(center.as_str()) {
                        return Err(format!("tetrahedral stereo '{id}' has no center atom"));
                    }
                    let mut referenced = BTreeSet::new();
                    let mut hydrogens = 0;
                    for reference in references {
                        match reference {
                            StereoReferenceV2::Atom(atom)
                                if atom != center
                                    && atom_ids.contains(atom.as_str())
                                    && referenced.insert(atom.as_str()) => {}
                            StereoReferenceV2::ImplicitHydrogen if hydrogens == 0 => hydrogens += 1,
                            _ => {
                                return Err(format!(
                                    "tetrahedral stereo '{id}' has an invalid or repeated reference"
                                ))
                            }
                        }
                    }
                }
                StereoElementV2::DoubleBond {
                    id,
                    bond,
                    left_reference,
                    right_reference,
                    ..
                } => {
                    let Some(double_bond) = bonds.get(bond.as_str()) else {
                        return Err(format!("double-bond stereo '{id}' references no bond"));
                    };
                    if double_bond.kind != BondKindV2::Double
                        || left_reference == right_reference
                        || !atom_ids.contains(left_reference.as_str())
                        || !atom_ids.contains(right_reference.as_str())
                    {
                        return Err(format!("double-bond stereo '{id}' is invalid"));
                    }
                    let direct = adjacency[&double_bond.atoms[0]].contains(left_reference)
                        && adjacency[&double_bond.atoms[1]].contains(right_reference);
                    let reverse = adjacency[&double_bond.atoms[1]].contains(left_reference)
                        && adjacency[&double_bond.atoms[0]].contains(right_reference);
                    if !direct && !reverse {
                        return Err(format!(
                            "double-bond stereo '{id}' references are not on opposite ends"
                        ));
                    }
                }
                StereoElementV2::EnhancedGroup { id, members, .. } => {
                    let unique = members.iter().map(String::as_str).collect::<BTreeSet<_>>();
                    if members.is_empty()
                        || unique.len() != members.len()
                        || unique.iter().any(|member| !non_group_ids.contains(member))
                        || unique
                            .iter()
                            .any(|member| !enhanced_members.insert(*member))
                    {
                        return Err(format!("enhanced stereo group '{id}' has invalid members"));
                    }
                }
                StereoElementV2::Extended {
                    id,
                    class,
                    descriptor,
                    carriers,
                } => {
                    validate_extended_descriptor(id, *class, descriptor)?;
                    validate_carriers(id, carriers, atom_ids, bond_ids)?;
                }
                StereoElementV2::Conformation { id, carriers, .. }
                | StereoElementV2::Unspecified { id, carriers, .. } => {
                    validate_carriers(id, carriers, atom_ids, bond_ids)?;
                }
            }
        }
        debug_assert_eq!(stereo_ids.len(), self.stereo.len());
        Ok(())
    }

    fn validate_interactions(
        &self,
        atom_ids: &BTreeSet<&str>,
        adjacency: &mut BTreeMap<String, BTreeSet<String>>,
    ) -> Result<(), String> {
        unique_nonempty_ids(
            self.interactions
                .iter()
                .map(|interaction| interaction.id.as_str()),
            "interaction",
        )?;
        for interaction in &self.interactions {
            if interaction.centers.len() < 2 {
                return Err(format!(
                    "interaction '{}' has fewer than two centers",
                    interaction.id
                ));
            }
            let mut participants = BTreeSet::new();
            for center in &interaction.centers {
                if center.atoms.is_empty()
                    || center.atoms.iter().any(|atom| {
                        !atom_ids.contains(atom.as_str()) || !participants.insert(atom.as_str())
                    })
                {
                    return Err(format!(
                        "interaction '{}' has an empty, missing, or repeated center atom",
                        interaction.id
                    ));
                }
            }
            match interaction.kind {
                InteractionKindV2::Coordination => {
                    let donors = interaction
                        .centers
                        .iter()
                        .filter(|center| center.role == InteractionRoleV2::Donor)
                        .count();
                    let acceptors = interaction
                        .centers
                        .iter()
                        .filter(|center| center.role == InteractionRoleV2::Acceptor)
                        .count();
                    if donors != 1
                        || acceptors == 0
                        || interaction
                            .centers
                            .iter()
                            .any(|center| center.role == InteractionRoleV2::Shared)
                    {
                        return Err(format!(
                            "coordination interaction '{}' requires one donor and one or more acceptors",
                            interaction.id
                        ));
                    }
                }
                InteractionKindV2::DelocalizedBond => {
                    if participants.len() < 3
                        || interaction
                            .centers
                            .iter()
                            .any(|center| center.role != InteractionRoleV2::Shared)
                    {
                        return Err(format!(
                            "delocalized interaction '{}' requires at least three shared atoms",
                            interaction.id
                        ));
                    }
                }
            }
            let participants = participants.into_iter().collect::<Vec<_>>();
            for left in &participants {
                for right in &participants {
                    if left != right {
                        adjacency
                            .get_mut(*left)
                            .expect("validated atom")
                            .insert((*right).to_string());
                    }
                }
            }
        }
        Ok(())
    }

    fn validate_components(
        &self,
        atom_ids: &BTreeSet<&str>,
        adjacency: &BTreeMap<String, BTreeSet<String>>,
    ) -> Result<(), String> {
        if self.components.is_empty() {
            return Err("graph has no components".to_string());
        }
        unique_nonempty_ids(
            self.components
                .iter()
                .map(|component| component.id.as_str()),
            "component",
        )?;
        let mut covered = BTreeSet::new();
        for component in &self.components {
            if component.count == 0 || component.atoms.is_empty() {
                return Err(format!(
                    "component '{}' has no atoms or zero count",
                    component.id
                ));
            }
            let allowed = component
                .atoms
                .iter()
                .map(String::as_str)
                .collect::<BTreeSet<_>>();
            if allowed.len() != component.atoms.len()
                || allowed
                    .iter()
                    .any(|atom| !atom_ids.contains(atom) || !covered.insert(*atom))
            {
                return Err(format!(
                    "component '{}' has missing or repeated atoms",
                    component.id
                ));
            }
            let start = *allowed.iter().next().expect("nonempty component");
            let mut seen = BTreeSet::from([start]);
            let mut pending = vec![start];
            while let Some(atom) = pending.pop() {
                for neighbor in &adjacency[atom] {
                    if allowed.contains(neighbor.as_str()) && seen.insert(neighbor.as_str()) {
                        pending.push(neighbor);
                    }
                }
            }
            if seen != allowed {
                return Err(format!("component '{}' is disconnected", component.id));
            }
        }
        if covered != *atom_ids {
            return Err("components do not cover every atom exactly once".to_string());
        }
        match self.semantics.profile {
            GraphProfileV2::MolecularEntity | GraphProfileV2::MolecularFragment
                if self.components.len() != 1 || self.components[0].count != 1 =>
            {
                return Err(format!(
                    "{} profile requires exactly one component with count 1",
                    match self.semantics.profile {
                        GraphProfileV2::MolecularEntity => "molecular-entity",
                        GraphProfileV2::MolecularFragment => "molecular-fragment",
                        GraphProfileV2::DiscreteComposition => unreachable!(),
                    }
                ));
            }
            _ => {}
        }
        Ok(())
    }

    fn validate_free_valences(&self, atom_ids: &BTreeSet<&str>) -> Result<(), String> {
        for (index, site) in self.free_valences.iter().enumerate() {
            if !atom_ids.contains(site.atom.as_str()) {
                return Err(format!(
                    "free valence {index} references missing atom '{}'",
                    site.atom
                ));
            }
        }
        match self.semantics.profile {
            GraphProfileV2::MolecularFragment if self.free_valences.is_empty() => {
                Err("molecular-fragment profile requires at least one free valence".to_string())
            }
            GraphProfileV2::MolecularFragment => Ok(()),
            _ if !self.free_valences.is_empty() => {
                Err("free valences require the molecular-fragment profile".to_string())
            }
            _ => Ok(()),
        }
    }

    fn validate_assumptions(&self) -> Result<(), String> {
        let mut codes = BTreeSet::new();
        for assumption in &self.assumptions {
            if assumption.code.trim().is_empty() || !codes.insert(assumption.code.as_str()) {
                return Err(format!(
                    "empty or duplicate assumption code '{}'",
                    assumption.code
                ));
            }
            if assumption
                .detail
                .as_deref()
                .is_some_and(|detail| detail.trim().is_empty())
            {
                return Err(format!(
                    "assumption '{}' has an empty detail",
                    assumption.code
                ));
            }
        }
        Ok(())
    }

    /// Returns a deterministic wire representation. This sorts ids and
    /// unordered sets; it does not relabel atom ids and is therefore not a
    /// graph-canonical identifier. Identity is defined by exact attributed
    /// graph isomorphism, not by serialized bytes.
    pub fn normalized(&self) -> Result<Self, String> {
        self.validate()?;
        let mut graph = self.clone();
        graph.atoms.sort_by(|left, right| left.id.cmp(&right.id));
        graph.bonds.sort_by(|left, right| left.id.cmp(&right.id));
        graph.free_valences.sort();
        graph
            .stereo
            .sort_by(|left, right| left.id().cmp(right.id()));
        graph
            .components
            .sort_by(|left, right| left.id.cmp(&right.id));
        graph
            .assumptions
            .sort_by(|left, right| left.code.cmp(&right.code));
        graph
            .interactions
            .sort_by(|left, right| left.id.cmp(&right.id));
        for component in &mut graph.components {
            component.atoms.sort();
        }
        for interaction in &mut graph.interactions {
            for center in &mut interaction.centers {
                center.atoms.sort();
            }
            interaction.centers.sort_by(|left, right| {
                format!("{:?}:{:?}", left.role, left.atoms)
                    .cmp(&format!("{:?}:{:?}", right.role, right.atoms))
            });
        }
        Ok(graph)
    }

    /// Exact attributed-graph identity. Source ids, array order, component ids,
    /// interaction ids, and audit assumptions do not affect chemical identity.
    pub fn is_isomorphic_to(&self, other: &Self) -> Result<bool, String> {
        self.validate()?;
        other.validate()?;
        if self.semantics != other.semantics
            || self.atoms.len() != other.atoms.len()
            || self.bonds.len() != other.bonds.len()
            || self.free_valences.len() != other.free_valences.len()
            || self.stereo.len() != other.stereo.len()
            || self.interactions.len() != other.interactions.len()
            || self.components.len() != other.components.len()
        {
            return Ok(false);
        }
        let left_components = component_membership(self);
        let right_components = component_membership(other);
        let mut candidates = Vec::with_capacity(self.atoms.len());
        for atom in &self.atoms {
            let component = left_components[atom.id.as_str()];
            let matching = other
                .atoms
                .iter()
                .enumerate()
                .filter_map(|(index, candidate)| {
                    (same_atom(atom, candidate)
                        && free_valence_orders(self, &atom.id)
                            == free_valence_orders(other, &candidate.id)
                        && component == right_components[candidate.id.as_str()])
                    .then_some(index)
                })
                .collect::<Vec<_>>();
            if matching.is_empty() {
                return Ok(false);
            }
            candidates.push(matching);
        }
        let mut order = (0..self.atoms.len()).collect::<Vec<_>>();
        order.sort_by_key(|index| candidates[*index].len());
        let mut mapping = vec![None; self.atoms.len()];
        let mut used = vec![false; other.atoms.len()];
        Ok(search_isomorphism(
            0,
            &order,
            &candidates,
            &mut mapping,
            &mut used,
            self,
            other,
        ))
    }
}

fn assess_linear_notation_limits(
    graph: &ChemicalGraphV2,
    reject: &mut impl FnMut(&str, String, String),
    format: &str,
) {
    assess_free_valence_limits(graph, reject, format);
    for index in 0..graph.interactions.len() {
        reject(
            "unsupported-multicenter-interaction",
            format!("/interactions/{index}"),
            format!("{format} cannot preserve this native multicenter interaction"),
        );
    }
    for (index, element) in graph.stereo.iter().enumerate() {
        if !matches!(
            element,
            StereoElementV2::Tetrahedral { .. } | StereoElementV2::DoubleBond { .. }
        ) {
            reject(
                "unsupported-stereo-class",
                format!("/stereo/{index}"),
                format!("{format} cannot preserve this native stereo element"),
            );
        }
    }
    for (index, component) in graph.components.iter().enumerate() {
        if component.count != 1 {
            reject(
                "unsupported-component-count",
                format!("/components/{index}/count"),
                format!("{format} cannot preserve an integer component multiplier"),
            );
        }
    }
}

fn assess_free_valence_limits(
    graph: &ChemicalGraphV2,
    reject: &mut impl FnMut(&str, String, String),
    format: &str,
) {
    if !graph.free_valences.is_empty() {
        reject(
            "requires-free-valence-encoding",
            "/freeValences".to_string(),
            format!(
                "The current {format} adapter has no verified lossless encoding for structured free valences"
            ),
        );
    }
}

fn free_valence_orders(graph: &ChemicalGraphV2, atom: &str) -> Vec<FreeValenceOrderV2> {
    let mut orders = graph
        .free_valences
        .iter()
        .filter_map(|site| (site.atom == atom).then_some(site.order))
        .collect::<Vec<_>>();
    orders.sort();
    orders
}

fn same_atom(left: &AtomV2, right: &AtomV2) -> bool {
    left.atomic_number == right.atomic_number
        && left.isotope == right.isotope
        && left.formal_charge == right.formal_charge
        && left.radical == right.radical
        && left.implicit_hydrogens == right.implicit_hydrogens
}

fn component_membership(graph: &ChemicalGraphV2) -> BTreeMap<&str, (u32, usize)> {
    graph
        .components
        .iter()
        .flat_map(|component| {
            component
                .atoms
                .iter()
                .map(move |atom| (atom.as_str(), (component.count, component.atoms.len())))
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn search_isomorphism(
    depth: usize,
    order: &[usize],
    candidates: &[Vec<usize>],
    mapping: &mut [Option<usize>],
    used: &mut [bool],
    left: &ChemicalGraphV2,
    right: &ChemicalGraphV2,
) -> bool {
    if depth == order.len() {
        return complete_mapping_matches(mapping, left, right);
    }
    let left_index = order[depth];
    for &right_index in &candidates[left_index] {
        if used[right_index] || !partial_bonds_match(left_index, right_index, mapping, left, right)
        {
            continue;
        }
        mapping[left_index] = Some(right_index);
        used[right_index] = true;
        if search_isomorphism(depth + 1, order, candidates, mapping, used, left, right) {
            return true;
        }
        used[right_index] = false;
        mapping[left_index] = None;
    }
    false
}

fn partial_bonds_match(
    left_atom: usize,
    right_atom: usize,
    mapping: &[Option<usize>],
    left: &ChemicalGraphV2,
    right: &ChemicalGraphV2,
) -> bool {
    let left_by_id = atom_indices(left);
    let right_by_id = atom_indices(right);
    mapping.iter().enumerate().all(|(other_left, other_right)| {
        let Some(other_right) = other_right else {
            return true;
        };
        let left_bond = bond_between(left, left_atom, other_left, &left_by_id);
        let right_bond = bond_between(right, right_atom, *other_right, &right_by_id);
        match (left_bond, right_bond) {
            (None, None) => true,
            (Some(left_bond), Some(right_bond)) => {
                left_bond.kind == right_bond.kind
                    && dative_role(left_bond, &left.atoms[left_atom].id)
                        == dative_role(right_bond, &right.atoms[right_atom].id)
            }
            _ => false,
        }
    })
}

fn atom_indices(graph: &ChemicalGraphV2) -> BTreeMap<&str, usize> {
    graph
        .atoms
        .iter()
        .enumerate()
        .map(|(index, atom)| (atom.id.as_str(), index))
        .collect()
}

fn bond_between<'a>(
    graph: &'a ChemicalGraphV2,
    left: usize,
    right: usize,
    by_id: &BTreeMap<&str, usize>,
) -> Option<&'a BondV2> {
    graph.bonds.iter().find(|bond| {
        let begin = by_id[bond.atoms[0].as_str()];
        let end = by_id[bond.atoms[1].as_str()];
        (begin == left && end == right) || (begin == right && end == left)
    })
}

fn dative_role(bond: &BondV2, atom: &str) -> Option<bool> {
    bond.dative_direction
        .as_ref()
        .map(|direction| direction.donor == atom)
}

fn complete_mapping_matches(
    mapping: &[Option<usize>],
    left: &ChemicalGraphV2,
    right: &ChemicalGraphV2,
) -> bool {
    let mapped = |id: &str| {
        let index = left
            .atoms
            .iter()
            .position(|atom| atom.id == id)
            .expect("validated atom reference");
        mapping[index].expect("complete mapping")
    };
    let identity = |id: &str| {
        right
            .atoms
            .iter()
            .position(|atom| atom.id == id)
            .expect("validated atom reference")
    };
    component_signatures(left, &mapped) == component_signatures(right, &identity)
        && free_valence_signatures(left, &mapped) == free_valence_signatures(right, &identity)
        && stereo_signatures(left, &mapped) == stereo_signatures(right, &identity)
        && interaction_signatures(left, &mapped) == interaction_signatures(right, &identity)
}

fn free_valence_signatures<F>(graph: &ChemicalGraphV2, atom: &F) -> Vec<(usize, FreeValenceOrderV2)>
where
    F: Fn(&str) -> usize,
{
    let mut result = graph
        .free_valences
        .iter()
        .map(|site| (atom(&site.atom), site.order))
        .collect::<Vec<_>>();
    result.sort();
    result
}

fn component_signatures<F>(graph: &ChemicalGraphV2, atom: &F) -> Vec<(u32, Vec<usize>)>
where
    F: Fn(&str) -> usize,
{
    let mut result = graph
        .components
        .iter()
        .map(|component| {
            let mut atoms = component
                .atoms
                .iter()
                .map(|id| atom(id))
                .collect::<Vec<_>>();
            atoms.sort_unstable();
            (component.count, atoms)
        })
        .collect::<Vec<_>>();
    result.sort();
    result
}

fn stereo_signatures<F>(graph: &ChemicalGraphV2, atom: &F) -> (Vec<String>, Vec<String>)
where
    F: Fn(&str) -> usize,
{
    let bonds = graph
        .bonds
        .iter()
        .map(|bond| (bond.id.as_str(), bond))
        .collect::<BTreeMap<_, _>>();
    let mut base_by_id = BTreeMap::new();
    for stereo in &graph.stereo {
        let signature = match stereo {
            StereoElementV2::Tetrahedral {
                center,
                references,
                parity,
                ..
            } => Some(format!(
                "T:{}:{:?}:{parity:?}",
                atom(center),
                references
                    .iter()
                    .map(|reference| match reference {
                        StereoReferenceV2::Atom(id) => format!("A{}", atom(id)),
                        StereoReferenceV2::ImplicitHydrogen => "H".to_string(),
                    })
                    .collect::<Vec<_>>()
            )),
            StereoElementV2::DoubleBond {
                bond,
                left_reference,
                right_reference,
                relation,
                ..
            } => {
                let bond = bonds[bond.as_str()];
                let mut ends = [
                    (atom(&bond.atoms[0]), atom(left_reference)),
                    (atom(&bond.atoms[1]), atom(right_reference)),
                ];
                if !graph.bonds.iter().any(|candidate| {
                    candidate.id != bond.id
                        && candidate.atoms.contains(&bond.atoms[0])
                        && candidate.atoms.contains(left_reference)
                }) {
                    ends = [
                        (atom(&bond.atoms[0]), atom(right_reference)),
                        (atom(&bond.atoms[1]), atom(left_reference)),
                    ];
                }
                ends.sort_unstable();
                Some(format!("D:{ends:?}:{relation:?}"))
            }
            StereoElementV2::EnhancedGroup { .. } => None,
            StereoElementV2::Extended {
                class,
                descriptor,
                carriers,
                ..
            } => Some(format!(
                "X:{class:?}:{descriptor:?}:{:?}",
                carrier_signatures(carriers, graph, atom)
            )),
            StereoElementV2::Conformation {
                descriptor,
                carriers,
                ..
            } => Some(format!(
                "C:{descriptor:?}:{:?}",
                carrier_signatures(carriers, graph, atom)
            )),
            StereoElementV2::Unspecified {
                descriptor,
                carriers,
                ..
            } => Some(format!(
                "U:{descriptor:?}:{:?}",
                carrier_signatures(carriers, graph, atom)
            )),
        };
        if let Some(signature) = signature {
            base_by_id.insert(stereo.id(), signature);
        }
    }
    let mut base = base_by_id.values().cloned().collect::<Vec<_>>();
    base.sort();
    let mut groups = graph
        .stereo
        .iter()
        .filter_map(|stereo| match stereo {
            StereoElementV2::EnhancedGroup {
                group_kind,
                members,
                ..
            } => {
                let mut signatures = members
                    .iter()
                    .map(|id| base_by_id[id.as_str()].clone())
                    .collect::<Vec<_>>();
                signatures.sort();
                Some(format!("G:{group_kind:?}:{signatures:?}"))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    groups.sort();
    (base, groups)
}

fn carrier_signatures<F>(
    carriers: &[StereoCarrierV2],
    graph: &ChemicalGraphV2,
    atom: &F,
) -> Vec<String>
where
    F: Fn(&str) -> usize,
{
    let mut result = carriers
        .iter()
        .map(|carrier| match carrier {
            StereoCarrierV2::Atom(id) => format!("A:{}", atom(id)),
            StereoCarrierV2::Bond(id) => bond_signature(graph, id, atom),
            StereoCarrierV2::AtomSet(ids) => mapped_atom_set("S", ids, atom),
            StereoCarrierV2::Axis(ids) => mapped_atom_set("AX", ids, atom),
            StereoCarrierV2::Plane(ids) => mapped_atom_set("PL", ids, atom),
            StereoCarrierV2::Torsion(ids) => format!(
                "TO:{},{},{},{}",
                atom(&ids[0]),
                atom(&ids[1]),
                atom(&ids[2]),
                atom(&ids[3])
            ),
            StereoCarrierV2::LonePair(id) => format!("LP:{}", atom(id)),
            StereoCarrierV2::DuplicateAtom(id) => format!("DU:{}", atom(id)),
            StereoCarrierV2::ConjugatedDoubleBondPair(ids) => {
                let mut bonds = ids
                    .iter()
                    .map(|id| bond_signature(graph, id, atom))
                    .collect::<Vec<_>>();
                bonds.sort();
                format!("DBP:{bonds:?}")
            }
        })
        .collect::<Vec<_>>();
    result.sort();
    result
}

fn mapped_atom_set<F>(prefix: &str, ids: &[String], atom: &F) -> String
where
    F: Fn(&str) -> usize,
{
    let mut mapped = ids.iter().map(|id| atom(id)).collect::<Vec<_>>();
    mapped.sort_unstable();
    format!("{prefix}:{mapped:?}")
}

fn bond_signature<F>(graph: &ChemicalGraphV2, id: &str, atom: &F) -> String
where
    F: Fn(&str) -> usize,
{
    let bond = graph
        .bonds
        .iter()
        .find(|bond| bond.id == id)
        .expect("validated bond carrier");
    let mut ends = [atom(&bond.atoms[0]), atom(&bond.atoms[1])];
    ends.sort_unstable();
    let direction = bond
        .dative_direction
        .as_ref()
        .map(|direction| (atom(&direction.donor), atom(&direction.acceptor)));
    format!("B:{ends:?}:{:?}:{direction:?}", bond.kind)
}

fn interaction_signatures<F>(graph: &ChemicalGraphV2, atom: &F) -> Vec<String>
where
    F: Fn(&str) -> usize,
{
    let mut result = graph
        .interactions
        .iter()
        .map(|interaction| {
            let mut centers = interaction
                .centers
                .iter()
                .map(|center| {
                    let mut atoms = center.atoms.iter().map(|id| atom(id)).collect::<Vec<_>>();
                    atoms.sort_unstable();
                    format!("{:?}:{atoms:?}", center.role)
                })
                .collect::<Vec<_>>();
            centers.sort();
            format!("{:?}:{centers:?}", interaction.kind)
        })
        .collect::<Vec<_>>();
    result.sort();
    result
}

fn unique_nonempty_ids<'a>(
    values: impl Iterator<Item = &'a str>,
    kind: &str,
) -> Result<BTreeSet<&'a str>, String> {
    let mut result = BTreeSet::new();
    for value in values {
        if value.trim().is_empty() || !result.insert(value) {
            return Err(format!("empty or duplicate {kind} id '{value}'"));
        }
    }
    Ok(result)
}

fn validate_extended_descriptor(
    id: &str,
    class: ExtendedStereoClassV2,
    descriptor: &ExtendedStereoDescriptorV2,
) -> Result<(), String> {
    let compatible = matches!(
        (class, descriptor),
        (
            ExtendedStereoClassV2::RelativeConfiguration,
            ExtendedStereoDescriptorV2::R
                | ExtendedStereoDescriptorV2::S
                | ExtendedStereoDescriptorV2::Cis
                | ExtendedStereoDescriptorV2::Trans
                | ExtendedStereoDescriptorV2::LowerC
                | ExtendedStereoDescriptorV2::LowerT
                | ExtendedStereoDescriptorV2::Endo
                | ExtendedStereoDescriptorV2::Exo
                | ExtendedStereoDescriptorV2::Syn
                | ExtendedStereoDescriptorV2::Anti
                | ExtendedStereoDescriptorV2::SeqCis
                | ExtendedStereoDescriptorV2::SeqTrans
        ) | (
            ExtendedStereoClassV2::PseudoasymmetricCenter,
            ExtendedStereoDescriptorV2::LowerR | ExtendedStereoDescriptorV2::LowerS
        ) | (
            ExtendedStereoClassV2::Axial,
            ExtendedStereoDescriptorV2::Ra
                | ExtendedStereoDescriptorV2::Sa
                | ExtendedStereoDescriptorV2::AxR
                | ExtendedStereoDescriptorV2::AxS
                | ExtendedStereoDescriptorV2::AxM
                | ExtendedStereoDescriptorV2::AxP
        ) | (
            ExtendedStereoClassV2::Planar,
            ExtendedStereoDescriptorV2::Rp
                | ExtendedStereoDescriptorV2::Sp
                | ExtendedStereoDescriptorV2::PlR
                | ExtendedStereoDescriptorV2::PlS
                | ExtendedStereoDescriptorV2::Cisoid
                | ExtendedStereoDescriptorV2::Transoid
        ) | (
            ExtendedStereoClassV2::Phane,
            ExtendedStereoDescriptorV2::Rp
                | ExtendedStereoDescriptorV2::Sp
                | ExtendedStereoDescriptorV2::PhaneR
                | ExtendedStereoDescriptorV2::PhaneS
        ) | (
            ExtendedStereoClassV2::Helical,
            ExtendedStereoDescriptorV2::M | ExtendedStereoDescriptorV2::P
        ) | (
            ExtendedStereoClassV2::Spiro,
            ExtendedStereoDescriptorV2::R
                | ExtendedStereoDescriptorV2::S
                | ExtendedStereoDescriptorV2::SpiroR
                | ExtendedStereoDescriptorV2::SpiroS
        ) | (
            ExtendedStereoClassV2::NontetrahedralCenter,
            ExtendedStereoDescriptorV2::TrigonalPyramidal
                | ExtendedStereoDescriptorV2::TShaped
                | ExtendedStereoDescriptorV2::Seesaw
                | ExtendedStereoDescriptorV2::TrigonalBipyramidal
                | ExtendedStereoDescriptorV2::SquarePyramidal
                | ExtendedStereoDescriptorV2::Octahedral
                | ExtendedStereoDescriptorV2::Coordination { .. }
        ) | (
            ExtendedStereoClassV2::PolyhedralCenter,
            ExtendedStereoDescriptorV2::PolyhedralA
                | ExtendedStereoDescriptorV2::PolyhedralC
                | ExtendedStereoDescriptorV2::Coordination { .. }
        ) | (
            ExtendedStereoClassV2::Fullerene,
            ExtendedStereoDescriptorV2::FullereneR
                | ExtendedStereoDescriptorV2::FullereneS
                | ExtendedStereoDescriptorV2::FullereneA
                | ExtendedStereoDescriptorV2::FullereneC
                | ExtendedStereoDescriptorV2::HelicalLocants { .. }
        ) | (
            ExtendedStereoClassV2::RingAssembly,
            ExtendedStereoDescriptorV2::AssemblyR
                | ExtendedStereoDescriptorV2::AssemblyS
                | ExtendedStereoDescriptorV2::AssemblyE
                | ExtendedStereoDescriptorV2::AssemblyZ
                | ExtendedStereoDescriptorV2::HelicalLocants { .. }
        )
    );
    if !compatible {
        return Err(format!(
            "extended stereo '{id}' descriptor is incompatible with class {class:?}"
        ));
    }
    match descriptor {
        ExtendedStereoDescriptorV2::Coordination {
            permutation_index, ..
        } if *permutation_index == 0 => {
            Err(format!("extended stereo '{id}' has zero permutation index"))
        }
        ExtendedStereoDescriptorV2::HelicalLocants { locants, .. }
            if locants.is_empty()
                || locants.contains(&0)
                || locants.iter().copied().collect::<BTreeSet<_>>().len() != locants.len() =>
        {
            Err(format!("extended stereo '{id}' has invalid locants"))
        }
        _ => Ok(()),
    }
}

fn validate_carriers(
    id: &str,
    carriers: &[StereoCarrierV2],
    atom_ids: &BTreeSet<&str>,
    bond_ids: &BTreeSet<&str>,
) -> Result<(), String> {
    if carriers.is_empty() {
        return Err(format!("stereo '{id}' has no carriers"));
    }
    for carrier in carriers {
        let valid = match carrier {
            StereoCarrierV2::Atom(atom)
            | StereoCarrierV2::LonePair(atom)
            | StereoCarrierV2::DuplicateAtom(atom) => atom_ids.contains(atom.as_str()),
            StereoCarrierV2::Bond(bond) => bond_ids.contains(bond.as_str()),
            StereoCarrierV2::AtomSet(atoms) | StereoCarrierV2::Plane(atoms) => {
                valid_atom_set(atoms, atom_ids, 1)
            }
            StereoCarrierV2::Axis(atoms) => {
                atoms[0] != atoms[1] && atoms.iter().all(|atom| atom_ids.contains(atom.as_str()))
            }
            StereoCarrierV2::Torsion(atoms) => {
                atoms.iter().collect::<BTreeSet<_>>().len() == 4
                    && atoms.iter().all(|atom| atom_ids.contains(atom.as_str()))
            }
            StereoCarrierV2::ConjugatedDoubleBondPair(bonds) => {
                bonds[0] != bonds[1] && bonds.iter().all(|bond| bond_ids.contains(bond.as_str()))
            }
        };
        if !valid {
            return Err(format!("stereo '{id}' has an invalid carrier"));
        }
    }
    Ok(())
}

fn valid_atom_set(atoms: &[String], atom_ids: &BTreeSet<&str>, minimum: usize) -> bool {
    atoms.len() >= minimum
        && atoms.iter().collect::<BTreeSet<_>>().len() == atoms.len()
        && atoms.iter().all(|atom| atom_ids.contains(atom.as_str()))
}

pub fn json_schema() -> serde_json::Value {
    serde_json::to_value(schema_for!(ChemicalGraphV2))
        .expect("ChemicalGraphV2 JSON Schema is serializable")
}

pub fn json_schema_pretty() -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(&json_schema())
}

pub fn nomenclature_request_json_schema() -> serde_json::Value {
    serde_json::to_value(schema_for!(NomenclatureRequestV1))
        .expect("NomenclatureRequestV1 JSON Schema is serializable")
}

pub fn nomenclature_request_json_schema_pretty() -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(&nomenclature_request_json_schema())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn methane() -> ChemicalGraphV2 {
        ChemicalGraphV2 {
            schema: CHEMICAL_GRAPH_V2_SCHEMA.to_string(),
            semantics: GraphSemanticsV2::default(),
            atoms: vec![AtomV2 {
                id: "c1".to_string(),
                atomic_number: 6,
                isotope: None,
                formal_charge: 0,
                radical: RadicalStateV2::None,
                implicit_hydrogens: 4,
            }],
            bonds: Vec::new(),
            free_valences: Vec::new(),
            stereo: Vec::new(),
            components: vec![ComponentV2 {
                id: "component-1".to_string(),
                atoms: vec!["c1".to_string()],
                count: 1,
            }],
            assumptions: Vec::new(),
            interactions: Vec::new(),
        }
    }

    fn propan_2_yl() -> ChemicalGraphV2 {
        ChemicalGraphV2 {
            schema: CHEMICAL_GRAPH_V2_SCHEMA.to_string(),
            semantics: GraphSemanticsV2 {
                profile: GraphProfileV2::MolecularFragment,
                ..GraphSemanticsV2::default()
            },
            atoms: vec![
                AtomV2 {
                    id: "c1".to_string(),
                    atomic_number: 6,
                    isotope: None,
                    formal_charge: 0,
                    radical: RadicalStateV2::None,
                    implicit_hydrogens: 3,
                },
                AtomV2 {
                    id: "c2".to_string(),
                    atomic_number: 6,
                    isotope: None,
                    formal_charge: 0,
                    radical: RadicalStateV2::None,
                    implicit_hydrogens: 1,
                },
                AtomV2 {
                    id: "c3".to_string(),
                    atomic_number: 6,
                    isotope: None,
                    formal_charge: 0,
                    radical: RadicalStateV2::None,
                    implicit_hydrogens: 3,
                },
            ],
            bonds: vec![
                BondV2 {
                    id: "b1".to_string(),
                    atoms: ["c1".to_string(), "c2".to_string()],
                    kind: BondKindV2::Single,
                    dative_direction: None,
                },
                BondV2 {
                    id: "b2".to_string(),
                    atoms: ["c2".to_string(), "c3".to_string()],
                    kind: BondKindV2::Single,
                    dative_direction: None,
                },
            ],
            free_valences: vec![FreeValenceSiteV2 {
                atom: "c2".to_string(),
                order: FreeValenceOrderV2::Single,
            }],
            stereo: Vec::new(),
            components: vec![ComponentV2 {
                id: "component-1".to_string(),
                atoms: vec!["c1".to_string(), "c2".to_string(), "c3".to_string()],
                count: 1,
            }],
            assumptions: Vec::new(),
            interactions: Vec::new(),
        }
    }

    #[test]
    fn validates_minimal_graph() {
        methane().validate().unwrap();
    }

    #[test]
    fn strict_json_rejects_unknown_identity_fields() {
        let mut value = serde_json::to_value(methane()).unwrap();
        value["mystery"] = serde_json::json!(true);
        assert!(serde_json::from_value::<ChemicalGraphV2>(value).is_err());
    }

    #[test]
    fn molecular_profile_rejects_mixture() {
        let mut graph = methane();
        graph.semantics.profile = GraphProfileV2::DiscreteComposition;
        graph.atoms.push(AtomV2 {
            id: "o1".to_string(),
            atomic_number: 8,
            isotope: None,
            formal_charge: 0,
            radical: RadicalStateV2::None,
            implicit_hydrogens: 2,
        });
        graph.components.push(ComponentV2 {
            id: "component-2".to_string(),
            atoms: vec!["o1".to_string()],
            count: 1,
        });
        graph.validate().unwrap();
        graph.semantics.profile = GraphProfileV2::MolecularEntity;
        assert!(graph.validate().is_err());
    }

    #[test]
    fn molecular_fragment_requires_structured_free_valence() {
        let graph = propan_2_yl();
        graph.validate().unwrap();

        let mut missing = graph.clone();
        missing.free_valences.clear();
        assert!(missing.validate().is_err());

        let mut wrong_profile = graph.clone();
        wrong_profile.semantics.profile = GraphProfileV2::MolecularEntity;
        assert!(wrong_profile.validate().is_err());

        let mut missing_atom = graph;
        missing_atom.free_valences[0].atom = "absent".to_string();
        assert!(missing_atom.validate().is_err());
    }

    #[test]
    fn free_valence_is_identity_bearing_but_ids_are_not() {
        let left = propan_2_yl();
        let mut renamed = propan_2_yl();
        renamed.atoms.reverse();
        renamed.bonds.reverse();
        renamed.atoms.iter_mut().for_each(|atom| {
            atom.id = match atom.id.as_str() {
                "c1" => "right".to_string(),
                "c2" => "center".to_string(),
                "c3" => "left".to_string(),
                _ => unreachable!(),
            };
        });
        for bond in &mut renamed.bonds {
            for atom in &mut bond.atoms {
                *atom = match atom.as_str() {
                    "c1" => "right".to_string(),
                    "c2" => "center".to_string(),
                    "c3" => "left".to_string(),
                    _ => unreachable!(),
                };
            }
        }
        renamed.components[0].atoms = vec!["left".into(), "center".into(), "right".into()];
        renamed.free_valences[0].atom = "center".to_string();
        assert!(left.is_isomorphic_to(&renamed).unwrap());

        renamed.free_valences[0].atom = "left".to_string();
        assert!(!left.is_isomorphic_to(&renamed).unwrap());
    }

    #[test]
    fn normalized_free_valences_are_sorted_without_collapsing_multiplicity() {
        let mut graph = propan_2_yl();
        graph.free_valences = vec![
            FreeValenceSiteV2 {
                atom: "c2".to_string(),
                order: FreeValenceOrderV2::Double,
            },
            FreeValenceSiteV2 {
                atom: "c2".to_string(),
                order: FreeValenceOrderV2::Single,
            },
            FreeValenceSiteV2 {
                atom: "c2".to_string(),
                order: FreeValenceOrderV2::Single,
            },
        ];
        let normalized = graph.normalized().unwrap();
        assert_eq!(normalized.free_valences.len(), 3);
        assert_eq!(
            normalized.free_valences[0].order,
            FreeValenceOrderV2::Single
        );
        assert_eq!(
            normalized.free_valences[1].order,
            FreeValenceOrderV2::Single
        );
        assert_eq!(
            normalized.free_valences[2].order,
            FreeValenceOrderV2::Double
        );
    }

    #[test]
    fn fragment_mapping_reports_unverified_external_encoding() {
        let graph = propan_2_yl();
        assert!(
            graph
                .assess_mapping_to(MoleculeFormatV1::ChemicalGraphV2)
                .unwrap()
                .lossless
        );
        for target in [
            MoleculeFormatV1::Cdxml,
            MoleculeFormatV1::Cdx,
            MoleculeFormatV1::Smiles,
            MoleculeFormatV1::SdfV2000,
        ] {
            let report = graph.assess_mapping_to(target).unwrap();
            assert!(!report.lossless);
            assert!(report
                .diagnostics
                .iter()
                .any(|item| item.code == "requires-free-valence-encoding"));
        }
    }

    #[test]
    fn delocalized_interaction_requires_shared_atoms() {
        let mut graph = methane();
        graph.interactions.push(MultiCenterInteractionV2 {
            id: "i1".to_string(),
            kind: InteractionKindV2::DelocalizedBond,
            centers: vec![
                InteractionCenterV2 {
                    role: InteractionRoleV2::Shared,
                    atoms: vec!["c1".to_string()],
                },
                InteractionCenterV2 {
                    role: InteractionRoleV2::Shared,
                    atoms: vec!["c1".to_string()],
                },
            ],
        });
        assert!(graph.validate().is_err());
    }

    #[test]
    fn accepts_the_original_v2_wire_shape_without_semantics() {
        let value = serde_json::json!({
            "schema": CHEMICAL_GRAPH_V2_SCHEMA,
            "atoms": [{
                "id": "c1",
                "atomicNumber": 6,
                "isotope": null,
                "formalCharge": 0,
                "radical": "none",
                "implicitHydrogens": 4
            }],
            "bonds": [],
            "stereo": [],
            "components": [{"id": "component-1", "atoms": ["c1"], "count": 1}],
            "assumptions": [],
            "interactions": []
        });
        let graph: ChemicalGraphV2 = serde_json::from_value(value).unwrap();
        assert_eq!(graph.semantics, GraphSemanticsV2::default());
        graph.validate().unwrap();
    }

    #[test]
    fn generated_schema_validates_emitted_json() {
        let schema = json_schema();
        let validator = jsonschema::validator_for(&schema).unwrap();
        assert!(validator.is_valid(&serde_json::to_value(methane()).unwrap()));
    }

    #[test]
    fn checked_in_schema_matches_the_rust_model() {
        let checked_in: serde_json::Value = serde_json::from_str(include_str!(
            "../../../schemas/chemical-graph-v2.schema.json"
        ))
        .unwrap();
        assert_eq!(checked_in, json_schema());
        let nomenclature: serde_json::Value = serde_json::from_str(include_str!(
            "../../../schemas/nomenclature-request-v1.schema.json"
        ))
        .unwrap();
        assert_eq!(nomenclature, nomenclature_request_json_schema());
    }

    #[test]
    fn identity_ignores_ids_order_and_assumption_text() {
        let left = methane();
        let mut right = methane();
        right.atoms[0].id = "renamed".to_string();
        right.components[0].id = "other-component".to_string();
        right.components[0].atoms = vec!["renamed".to_string()];
        right.assumptions.push(GraphAssumptionV2 {
            code: "source".to_string(),
            detail: Some("different provenance is not molecular identity".to_string()),
        });
        assert!(left.is_isomorphic_to(&right).unwrap());
        right.atoms[0].implicit_hydrogens = 3;
        assert!(!left.is_isomorphic_to(&right).unwrap());
    }

    #[test]
    fn mapping_report_never_hides_identity_loss() {
        let mut graph = methane();
        graph.atoms.extend([
            AtomV2 {
                id: "c2".to_string(),
                atomic_number: 6,
                isotope: None,
                formal_charge: 0,
                radical: RadicalStateV2::None,
                implicit_hydrogens: 0,
            },
            AtomV2 {
                id: "c3".to_string(),
                atomic_number: 6,
                isotope: None,
                formal_charge: 0,
                radical: RadicalStateV2::None,
                implicit_hydrogens: 0,
            },
        ]);
        graph.components[0].atoms.extend(["c2".into(), "c3".into()]);
        graph.interactions.push(MultiCenterInteractionV2 {
            id: "delocalized-1".to_string(),
            kind: InteractionKindV2::DelocalizedBond,
            centers: vec![
                InteractionCenterV2 {
                    role: InteractionRoleV2::Shared,
                    atoms: vec!["c1".to_string()],
                },
                InteractionCenterV2 {
                    role: InteractionRoleV2::Shared,
                    atoms: vec!["c2".to_string(), "c3".to_string()],
                },
            ],
        });
        graph.validate().unwrap();
        assert!(
            graph
                .assess_mapping_to(MoleculeFormatV1::ChemicalGraphV2)
                .unwrap()
                .lossless
        );
        for target in [
            MoleculeFormatV1::Cdxml,
            MoleculeFormatV1::Cdx,
            MoleculeFormatV1::Smiles,
            MoleculeFormatV1::SdfV2000,
        ] {
            let report = graph.assess_mapping_to(target).unwrap();
            assert!(!report.lossless);
            assert!(report.diagnostics.iter().all(|item| {
                !item.code.is_empty() && item.path.starts_with('/') && !item.message.is_empty()
            }));
        }

        graph.interactions[0].kind = InteractionKindV2::Coordination;
        graph.interactions[0].centers[0].role = InteractionRoleV2::Donor;
        graph.interactions[0].centers[1].role = InteractionRoleV2::Acceptor;
        graph.validate().unwrap();
        for target in [MoleculeFormatV1::Cdxml, MoleculeFormatV1::Cdx] {
            let report = graph.assess_mapping_to(target).unwrap();
            assert!(!report.lossless);
            assert!(report
                .diagnostics
                .iter()
                .any(|diagnostic| { diagnostic.code == "requires-document-multicenter-encoding" }));
        }
    }

    #[test]
    fn conformance_fixture_pack_has_expected_outcomes() {
        let valid = [
            include_str!("../../../fixtures/chemical-graph-v2/valid/methane.json"),
            include_str!("../../../fixtures/chemical-graph-v2/valid/propan-2-yl.json"),
        ];
        for fixture in valid {
            serde_json::from_str::<ChemicalGraphV2>(fixture)
                .unwrap()
                .validate()
                .unwrap();
        }
        let invalid = [
            include_str!("../../../fixtures/chemical-graph-v2/invalid/unknown-identity-field.json"),
            include_str!("../../../fixtures/chemical-graph-v2/invalid/disconnected-component.json"),
            include_str!(
                "../../../fixtures/chemical-graph-v2/invalid/repeated-interaction-atom.json"
            ),
            include_str!("../../../fixtures/chemical-graph-v2/invalid/pseudo-atom.json"),
        ];
        for fixture in invalid {
            if let Ok(graph) = serde_json::from_str::<ChemicalGraphV2>(fixture) {
                assert!(graph.validate().is_err());
            }
        }
    }

    #[test]
    fn dative_direction_is_typed_but_keeps_the_v2_wire_contract() {
        let direction = DativeDirectionV2 {
            donor: "n1".to_string(),
            acceptor: "cu1".to_string(),
        };
        let json = serde_json::to_string(&direction).unwrap();
        assert_eq!(json, "\"n1->cu1\"");
        assert_eq!(
            serde_json::from_str::<DativeDirectionV2>(&json).unwrap(),
            direction
        );
        assert!(serde_json::from_str::<DativeDirectionV2>("\"n1-cu1\"").is_err());
    }

    #[test]
    fn extended_descriptor_is_typed_and_class_checked() {
        let descriptor = ExtendedStereoDescriptorV2::Coordination {
            geometry: CoordinationGeometryV2::Octahedral,
            permutation_index: 3,
        };
        assert_eq!(
            serde_json::to_string(&descriptor).unwrap(),
            "\"octahedral-3\""
        );
        assert_eq!(
            serde_json::from_str::<ExtendedStereoDescriptorV2>("\"octahedral-3\"").unwrap(),
            descriptor
        );
        assert!(
            validate_extended_descriptor("x1", ExtendedStereoClassV2::Axial, &descriptor).is_err()
        );
    }

    #[test]
    fn accepts_all_descriptors_supported_by_the_existing_nomenclature_v2_contract() {
        let cases = [
            ("r", ExtendedStereoClassV2::PseudoasymmetricCenter),
            ("s", ExtendedStereoClassV2::PseudoasymmetricCenter),
            ("cis", ExtendedStereoClassV2::RelativeConfiguration),
            ("trans", ExtendedStereoClassV2::RelativeConfiguration),
            ("c", ExtendedStereoClassV2::RelativeConfiguration),
            ("t", ExtendedStereoClassV2::RelativeConfiguration),
            ("endo", ExtendedStereoClassV2::RelativeConfiguration),
            ("exo", ExtendedStereoClassV2::RelativeConfiguration),
            ("syn", ExtendedStereoClassV2::RelativeConfiguration),
            ("anti", ExtendedStereoClassV2::RelativeConfiguration),
            ("seqCis", ExtendedStereoClassV2::RelativeConfiguration),
            ("seqTrans", ExtendedStereoClassV2::RelativeConfiguration),
            ("A", ExtendedStereoClassV2::PolyhedralCenter),
            ("C", ExtendedStereoClassV2::PolyhedralCenter),
            ("M", ExtendedStereoClassV2::Helical),
            ("P", ExtendedStereoClassV2::Helical),
            ("cisoid", ExtendedStereoClassV2::Planar),
            ("transoid", ExtendedStereoClassV2::Planar),
            ("axR", ExtendedStereoClassV2::Axial),
            ("axS", ExtendedStereoClassV2::Axial),
            ("axM", ExtendedStereoClassV2::Axial),
            ("axP", ExtendedStereoClassV2::Axial),
            ("plR", ExtendedStereoClassV2::Planar),
            ("plS", ExtendedStereoClassV2::Planar),
            ("spiroR", ExtendedStereoClassV2::Spiro),
            ("spiroS", ExtendedStereoClassV2::Spiro),
            ("phaneR", ExtendedStereoClassV2::Phane),
            ("phaneS", ExtendedStereoClassV2::Phane),
            ("fullereneR", ExtendedStereoClassV2::Fullerene),
            ("fullereneS", ExtendedStereoClassV2::Fullerene),
            ("fullereneA", ExtendedStereoClassV2::Fullerene),
            ("fullereneC", ExtendedStereoClassV2::Fullerene),
            ("assemblyR", ExtendedStereoClassV2::RingAssembly),
            ("assemblyS", ExtendedStereoClassV2::RingAssembly),
            ("assemblyE", ExtendedStereoClassV2::RingAssembly),
            ("assemblyZ", ExtendedStereoClassV2::RingAssembly),
            ("tp", ExtendedStereoClassV2::NontetrahedralCenter),
            ("tshape", ExtendedStereoClassV2::NontetrahedralCenter),
            ("seesaw", ExtendedStereoClassV2::NontetrahedralCenter),
            ("tbpy", ExtendedStereoClassV2::NontetrahedralCenter),
            ("spy", ExtendedStereoClassV2::NontetrahedralCenter),
            ("oc", ExtendedStereoClassV2::NontetrahedralCenter),
        ];
        for (wire, class) in cases {
            let descriptor: ExtendedStereoDescriptorV2 =
                serde_json::from_value(serde_json::json!(wire)).unwrap();
            assert_eq!(descriptor.wire_value(), wire);
            validate_extended_descriptor(wire, class, &descriptor).unwrap();
        }
    }

    #[test]
    fn nomenclature_request_is_a_strict_validated_interface() {
        let request =
            NomenclatureRequestV1::new_preferred_iupac_name("molecule-1", methane()).unwrap();
        request.validate().unwrap();
        let mut value = serde_json::to_value(request).unwrap();
        value["requestedNames"] =
            serde_json::json!(["preferred-iupac-name", "preferred-iupac-name"]);
        let repeated: NomenclatureRequestV1 = serde_json::from_value(value).unwrap();
        assert!(repeated.validate().is_err());
    }
}
