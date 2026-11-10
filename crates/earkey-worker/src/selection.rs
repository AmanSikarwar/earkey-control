//! Blocked-group selections and their disclosed effects.

use earkey_policy::identity::{Evidence, Group};

/// Which measured control groups a session blocks.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Selection {
    Passthrough,
    BlockAll,
    Groups(Vec<Group>),
}

impl Selection {
    /// Parses `passthrough`, `block-all`, or a comma list of distinct
    /// `center`, `volume-up` and `volume-down`.
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "passthrough" => return Some(Self::Passthrough),
            "block-all" => return Some(Self::BlockAll),
            _ => {}
        }
        let mut groups = Vec::new();
        for name in text.split(',') {
            let group = match name {
                "center" => Group::Center,
                "volume-up" => Group::VolumeUp,
                "volume-down" => Group::VolumeDown,
                _ => return None,
            };
            if groups.contains(&group) {
                return None;
            }
            groups.push(group);
        }
        Some(Self::Groups(groups))
    }

    /// The blocked groups for `evidence`; `BlockAll` means every measured control.
    pub fn groups(&self, evidence: Evidence) -> Vec<Group> {
        match self {
            Self::Passthrough => vec![],
            Self::BlockAll => evidence.controls().iter().map(|c| c.group).collect(),
            Self::Groups(groups) => groups.clone(),
        }
    }
}

/// Disclosed effects of blocking `groups`, including shared center behavior.
pub fn blocked_effects(evidence: Evidence, groups: &[Group]) -> String {
    let effects: Vec<&str> = evidence
        .controls()
        .iter()
        .filter(|control| groups.contains(&control.group))
        .flat_map(|control| {
            control
                .observed_effects
                .iter()
                .chain(control.reported_effects)
        })
        .copied()
        .collect();
    if effects.is_empty() {
        "none".into()
    } else {
        effects.join("|")
    }
}
