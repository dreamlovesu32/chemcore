use super::*;
use crate::{BondAbsoluteStereo, BondQueryOrder, BondReactionParticipation, BondTopology};
use std::collections::BTreeSet;

fn replace_if_different<T: PartialEq>(target: &mut T, value: T) -> bool {
    if *target == value {
        false
    } else {
        *target = value;
        true
    }
}

fn explicit_visibility(value: Option<&str>) -> Result<Option<bool>, ()> {
    match value {
        None | Some("inherit") => Ok(None),
        Some("true" | "yes" | "on" | "1") => Ok(Some(true)),
        Some("false" | "no" | "off" | "0") => Ok(Some(false)),
        Some(_) => Err(()),
    }
}

enum BondPropertyUpdate {
    QueryOrders(Vec<BondQueryOrder>),
    Topology(BondTopology),
    ReactionParticipation(BondReactionParticipation),
    AbsoluteStereo(BondAbsoluteStereo),
    ShowQuery(Option<bool>),
    ShowReaction(Option<bool>),
    ShowStereo(Option<bool>),
}

impl Engine {
    pub fn set_bond_property_for_selection(&mut self, property: &str, value: Option<&str>) -> bool {
        self.with_command(
            EditorCommand::SetBondPropertyForSelection {
                property: property.to_string(),
                value: value.map(ToString::to_string),
            },
            |engine| engine.set_bond_property_for_selection_untracked(property, value),
        )
    }

    fn set_bond_property_for_selection_untracked(
        &mut self,
        property: &str,
        value: Option<&str>,
    ) -> bool {
        let selected: BTreeSet<String> = self.state.selection.bonds.iter().cloned().collect();
        if selected.is_empty() {
            return false;
        }
        let normalized = value.map(str::trim).filter(|value| !value.is_empty());
        let update = match property {
            "query-orders" => BondPropertyUpdate::QueryOrders(match normalized.unwrap_or("none") {
                "none" => Vec::new(),
                "single-double" => vec![BondQueryOrder::Single, BondQueryOrder::Double],
                "single-aromatic" => vec![BondQueryOrder::Single, BondQueryOrder::Aromatic],
                "double-aromatic" => vec![BondQueryOrder::Double, BondQueryOrder::Aromatic],
                _ => return false,
            }),
            "topology" => BondPropertyUpdate::Topology(match normalized.unwrap_or("unspecified") {
                "unspecified" => BondTopology::Unspecified,
                "ring" => BondTopology::Ring,
                "chain" => BondTopology::Chain,
                "ring-or-chain" => BondTopology::RingOrChain,
                _ => return false,
            }),
            "reaction-participation" => BondPropertyUpdate::ReactionParticipation(match normalized
                .unwrap_or("unspecified")
            {
                "unspecified" => BondReactionParticipation::Unspecified,
                "reaction-center" => BondReactionParticipation::ReactionCenter,
                "make-or-break" => BondReactionParticipation::MakeOrBreak,
                "change-type" => BondReactionParticipation::ChangeType,
                "make-and-change" => BondReactionParticipation::MakeAndChange,
                "not-reaction-center" => BondReactionParticipation::NotReactionCenter,
                "no-change" => BondReactionParticipation::NoChange,
                "unmapped" => BondReactionParticipation::Unmapped,
                _ => return false,
            }),
            "absolute-stereo" => {
                BondPropertyUpdate::AbsoluteStereo(match normalized.unwrap_or("unspecified") {
                    "unspecified" => BondAbsoluteStereo::Unspecified,
                    "none" => BondAbsoluteStereo::None,
                    "e" => BondAbsoluteStereo::E,
                    "z" => BondAbsoluteStereo::Z,
                    _ => return false,
                })
            }
            "show-query" => {
                let Ok(value) = explicit_visibility(normalized) else {
                    return false;
                };
                BondPropertyUpdate::ShowQuery(value)
            }
            "show-reaction" => {
                let Ok(value) = explicit_visibility(normalized) else {
                    return false;
                };
                BondPropertyUpdate::ShowReaction(value)
            }
            "show-stereo" => {
                let Ok(value) = explicit_visibility(normalized) else {
                    return false;
                };
                BondPropertyUpdate::ShowStereo(value)
            }
            _ => return false,
        };

        self.push_undo_snapshot();
        let Some(mut entry) = self.state.document.editable_fragment_mut() else {
            self.undo_stack.pop();
            return false;
        };
        let mut changed = false;
        for bond in &mut entry.fragment.bonds {
            if !selected.contains(&bond.id) {
                continue;
            }
            changed |= match &update {
                BondPropertyUpdate::QueryOrders(next) => {
                    replace_if_different(&mut bond.properties.query_orders, next.clone())
                }
                BondPropertyUpdate::Topology(next) => {
                    replace_if_different(&mut bond.properties.topology, *next)
                }
                BondPropertyUpdate::ReactionParticipation(next) => {
                    replace_if_different(&mut bond.properties.reaction_participation, *next)
                }
                BondPropertyUpdate::AbsoluteStereo(next) => {
                    replace_if_different(&mut bond.properties.absolute_stereo, *next)
                }
                BondPropertyUpdate::ShowQuery(next) => {
                    replace_if_different(&mut bond.properties.show_query, *next)
                }
                BondPropertyUpdate::ShowReaction(next) => {
                    replace_if_different(&mut bond.properties.show_reaction, *next)
                }
                BondPropertyUpdate::ShowStereo(next) => {
                    replace_if_different(&mut bond.properties.show_stereo, *next)
                }
            };
        }
        if !changed {
            self.undo_stack.pop();
            return false;
        }
        entry.update_bounds();
        self.state.overlay.hover_bond_center = None;
        self.pointer_bond_target = None;
        true
    }
}
