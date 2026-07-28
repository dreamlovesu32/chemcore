use super::Engine;
use crate::{LinkPolicy, LogicalBindingOrigin, Point, ReactionInterpretationState};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

const REACTION_AUTO_SCHEME_ID: &str = "reaction_scheme_auto";

#[derive(Debug, Clone)]
struct ReactionArrowAxis {
    object_id: String,
    start: Point,
    end: Point,
    center: Point,
    unit: [f64; 2],
    normal: [f64; 2],
    length: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReactionSide {
    Reactant,
    Product,
}

#[derive(Debug, Clone)]
struct ReactionAutoCandidate {
    arrow_id: String,
    side: ReactionSide,
    score: f64,
    projection: f64,
}

#[derive(Debug, Clone)]
struct InferredReactionStep {
    arrow_id: String,
    reactants: Vec<(String, f64)>,
    products: Vec<(String, f64)>,
    pluses: Vec<(String, f64)>,
    above: Vec<(String, f64)>,
    below: Vec<(String, f64)>,
}

include!("logical_relations/commands.rs");
include!("logical_relations/reaction_auto.rs");
include!("logical_relations/helpers.rs");

#[cfg(test)]
mod tests;
