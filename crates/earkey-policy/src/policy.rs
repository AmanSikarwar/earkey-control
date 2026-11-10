use crate::identity::{
    Candidate, Control, Environment, Evidence, Group, NodeInstance, VerifiedEnrollment,
};
use serde::{Deserialize, Serialize};

pub const MAX_POLICY_BYTES: usize = 16 * 1024;
const SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Preset {
    BlockAll,
    Custom,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Backend {
    EvdevUinput,
}

#[derive(Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum UnknownAccessory {
    Ignore,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryLatch {
    Clear,
    RestartLimit,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Recovery {
    pub paused: bool,
    pub boot_disabled: bool,
    pub latch: RecoveryLatch,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Diagnostics {
    pub enabled: bool,
    pub seconds: u16,
    pub max_bytes: u32,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Profile {
    evidence: Evidence,
    usb_serial: String,
    enabled: bool,
    blocked_groups: Vec<Group>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PolicyV1 {
    schema_version: u32,
    generation: u64,
    enabled: bool,
    preset: Preset,
    profiles: Vec<Profile>,
    requested_backend: Backend,
    unknown_accessory: UnknownAccessory,
    diagnostics: Diagnostics,
    recovery: Recovery,
}

#[derive(Debug, Eq, PartialEq)]
pub enum PolicyError {
    Oversized,
    InvalidJson,
    UnsupportedSchema(u64),
    UnverifiedProfile,
    DuplicateProfile,
    TooManyProfiles,
    InvalidGroups,
    InvalidDiagnostics,
    NoEnabledProfile,
}

// This type is created only after schema and evidence validation.
pub struct Policy(PolicyV1);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MatchError {
    Ambiguous,
    ChangedCapabilities,
}

#[derive(Eq, PartialEq)]
pub struct Target {
    generation: u64,
    instance: NodeInstance,
    profile_index: usize,
}

impl Target {
    pub fn instance(&self) -> NodeInstance {
        self.instance
    }

    pub fn profile_index(&self) -> usize {
        self.profile_index
    }
}

impl Policy {
    pub fn inactive() -> Self {
        Self(PolicyV1 {
            schema_version: SCHEMA_VERSION,
            generation: 0,
            enabled: false,
            preset: Preset::Custom,
            profiles: vec![],
            requested_backend: Backend::EvdevUinput,
            unknown_accessory: UnknownAccessory::Ignore,
            diagnostics: Diagnostics {
                enabled: false,
                seconds: 0,
                max_bytes: 0,
            },
            recovery: Recovery {
                paused: false,
                boot_disabled: false,
                latch: RecoveryLatch::Clear,
            },
        })
    }

    /// Validates policy bytes against the caller's independently verified enrollments.
    /// Successful decoding does not establish that an accessory is currently present.
    pub fn parse(
        bytes: &[u8],
        enrollments: &[VerifiedEnrollment<'_>],
    ) -> Result<Self, PolicyError> {
        Self::decode(bytes, Some(enrollments))
    }

    /// Applies every check except enrollment matching. The result may only be
    /// used for recovery state; device selection must come from [`Policy::parse`].
    pub(crate) fn parse_unverified(bytes: &[u8]) -> Result<Self, PolicyError> {
        Self::decode(bytes, None)
    }

    fn decode(
        bytes: &[u8],
        enrollments: Option<&[VerifiedEnrollment<'_>]>,
    ) -> Result<Self, PolicyError> {
        if bytes.len() > MAX_POLICY_BYTES {
            return Err(PolicyError::Oversized);
        }

        let value: serde_json::Value =
            serde_json::from_slice(bytes).map_err(|_| PolicyError::InvalidJson)?;
        let version = value
            .get("schema_version")
            .and_then(serde_json::Value::as_u64)
            .ok_or(PolicyError::InvalidJson)?;
        if version != u64::from(SCHEMA_VERSION) {
            return Err(PolicyError::UnsupportedSchema(version));
        }

        // Decode the original bytes to reject duplicates, then reject positional struct forms.
        let policy: PolicyV1 =
            serde_json::from_slice(bytes).map_err(|_| PolicyError::InvalidJson)?;
        if serde_json::to_value(&policy).map_err(|_| PolicyError::InvalidJson)? != value {
            return Err(PolicyError::InvalidJson);
        }

        if policy.profiles.len() > 8 {
            return Err(PolicyError::TooManyProfiles);
        }
        for (index, profile) in policy.profiles.iter().enumerate() {
            if let Some(enrollments) = enrollments
                && !enrollments
                    .iter()
                    .any(|entry| entry.matches(profile.evidence, &profile.usb_serial))
            {
                return Err(PolicyError::UnverifiedProfile);
            }
            if policy.profiles[..index]
                .iter()
                .any(|earlier| earlier.usb_serial == profile.usb_serial)
            {
                return Err(PolicyError::DuplicateProfile);
            }

            let groups = &profile.blocked_groups;
            if (policy.preset == Preset::BlockAll && !groups.is_empty())
                || groups.len() > profile.evidence.controls().len()
                || groups.iter().enumerate().any(|(index, group)| {
                    groups[..index].contains(group)
                        || !profile
                            .evidence
                            .controls()
                            .iter()
                            .any(|control| control.group == *group)
                })
            {
                return Err(PolicyError::InvalidGroups);
            }
        }
        if policy.enabled && !policy.profiles.iter().any(|profile| profile.enabled) {
            return Err(PolicyError::NoEnabledProfile);
        }
        let diagnostics = policy.diagnostics;
        if !(if diagnostics.enabled {
            (1..=120).contains(&diagnostics.seconds)
                && (1..=65_536).contains(&diagnostics.max_bytes)
        } else {
            diagnostics.seconds == 0 && diagnostics.max_bytes == 0
        }) {
            return Err(PolicyError::InvalidDiagnostics);
        }
        Ok(Self(policy))
    }

    pub fn generation(&self) -> u64 {
        self.0.generation
    }

    pub fn enabled(&self) -> bool {
        self.0.enabled
    }

    pub fn profile_count(&self) -> usize {
        self.0.profiles.len()
    }

    pub fn recovery(&self) -> Recovery {
        self.0.recovery
    }

    pub fn diagnostics(&self) -> Diagnostics {
        self.0.diagnostics
    }

    pub fn requested_backend(&self) -> Backend {
        self.0.requested_backend
    }

    pub(crate) fn set_generation(&mut self, generation: u64) {
        self.0.generation = generation;
    }

    pub(crate) fn set_recovery(&mut self, recovery: Recovery) {
        self.0.recovery = recovery;
    }

    /// Serializes private policy data, including enrolled accessory serials.
    pub fn to_json(&self) -> Vec<u8> {
        serde_json::to_vec(&self.0).expect("policy v1 only contains JSON data types")
    }

    /// Previews the configured selection, including when activation is inhibited.
    pub fn blocked_controls(&self, profile_index: usize) -> impl Iterator<Item = &'static Control> {
        self.0
            .profiles
            .get(profile_index)
            .into_iter()
            .flat_map(move |profile| {
                profile.evidence.controls().iter().filter(move |control| {
                    self.0.preset == Preset::BlockAll
                        || profile.blocked_groups.contains(&control.group)
                })
            })
    }

    /// Selects eligible nodes from a complete inventory before opening them.
    pub fn targets(
        &self,
        environment: &Environment<'_>,
        inventory: &[Candidate<'_>],
    ) -> Result<Vec<Target>, MatchError> {
        let recovery = self.0.recovery;
        if !self.0.enabled
            || recovery.paused
            || recovery.boot_disabled
            || recovery.latch != RecoveryLatch::Clear
        {
            return Ok(vec![]);
        }

        let mut targets = Vec::new();
        for (profile_index, profile) in self.0.profiles.iter().enumerate() {
            if !profile.enabled
                || !profile.evidence.matches_environment(environment)
                || self.blocked_controls(profile_index).next().is_none()
            {
                continue;
            }

            let mut matches = inventory.iter().filter(|device| {
                profile
                    .evidence
                    .matches_identity(&profile.usb_serial, device)
            });
            if let Some(device) = matches.next() {
                if matches.next().is_some()
                    || targets
                        .iter()
                        .any(|target: &Target| target.instance == device.instance)
                {
                    return Err(MatchError::Ambiguous);
                }
                if !profile.evidence.matches_capabilities(device) {
                    return Err(MatchError::ChangedCapabilities);
                }
                targets.push(Target {
                    generation: self.generation(),
                    instance: device.instance,
                    profile_index,
                });
            }
        }
        Ok(targets)
    }

    /// Checks observations from the opened descriptor and an independently refreshed inventory.
    /// A reconnect requires a new target from `targets`.
    pub fn revalidate(
        &self,
        target: &Target,
        opened: &Candidate<'_>,
        environment: &Environment<'_>,
        current_inventory: &[Candidate<'_>],
    ) -> Result<bool, MatchError> {
        let Some(profile) = self.0.profiles.get(target.profile_index) else {
            return Ok(false);
        };
        Ok(opened.instance == target.instance
            && profile
                .evidence
                .matches_identity(&profile.usb_serial, opened)
            && profile.evidence.matches_capabilities(opened)
            && self
                .targets(environment, current_inventory)?
                .contains(target))
    }
}
