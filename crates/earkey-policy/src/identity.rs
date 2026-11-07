use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Evidence {
    XiaomiPixelos20261107,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Group {
    Center,
    VolumeUp,
    VolumeDown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LinuxKey(pub u16);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HidUsage {
    pub page: u16,
    pub usage: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AndroidKey {
    MediaPlayPause,
    VolumeUp,
    VolumeDown,
}

pub struct Control {
    pub group: Group,
    pub linux: LinuxKey,
    pub hid: HidUsage,
    pub android: AndroidKey,
    pub label: &'static str,
    pub observed_effects: &'static [&'static str],
    pub reported_effects: &'static [&'static str],
}

const CONTROLS: [Control; 3] = [
    Control {
        group: Group::Center,
        linux: LinuxKey(0x00a4),
        hid: HidUsage {
            page: 0x000c,
            usage: 0x00cd,
        },
        android: AndroidKey::MediaPlayPause,
        label: "Center (playback, tracks, assistant and calls)",
        observed_effects: &[
            "Play or pause",
            "Change track",
            "Open assistant and wake screen",
            "Answer or end cellular call",
        ],
        reported_effects: &["Mute or unmute an ongoing call", "Reject an incoming call"],
    },
    Control {
        group: Group::VolumeUp,
        linux: LinuxKey(0x0073),
        hid: HidUsage {
            page: 0x000c,
            usage: 0x00e9,
        },
        android: AndroidKey::VolumeUp,
        label: "Volume Up",
        observed_effects: &["Raise volume", "Show volume panel"],
        reported_effects: &["Silence an incoming ringtone"],
    },
    Control {
        group: Group::VolumeDown,
        linux: LinuxKey(0x0072),
        hid: HidUsage {
            page: 0x000c,
            usage: 0x00ea,
        },
        android: AndroidKey::VolumeDown,
        label: "Volume Down",
        observed_effects: &["Lower volume", "Show volume panel"],
        reported_effects: &["Silence an incoming ringtone"],
    },
];

#[derive(Eq, PartialEq)]
pub struct Environment<'a> {
    pub model: &'a str,
    pub device: &'a str,
    pub api: u32,
    pub rom: &'a str,
    pub build_id: &'a str,
    pub incremental: &'a str,
    pub kernel_release: &'a str,
    pub kernel_build: &'a str,
}

#[derive(Eq, PartialEq)]
pub struct UsbHid {
    pub vendor: u16,
    pub product: u16,
    pub revision: u16,
    pub interface: u8,
    pub interface_class: u8,
    pub hid_bus: u16,
    pub hid_vendor: u16,
    pub hid_product: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NodeInstance {
    pub filesystem_device: u64,
    pub inode: u64,
    pub character_device: u64,
}

// These are complete inventory observations. Missing observations cannot match.
pub struct Candidate<'a> {
    pub instance: NodeInstance,
    pub usb_hid: Option<UsbHid>,
    pub usb_serial: Option<&'a str>,
    pub usb_driver: Option<&'a str>,
    pub is_virtual: bool,
    pub input_id: [u16; 4],
    pub name: &'a str,
    pub layout_sha256: &'a str,
    pub event_types: &'a [u16],
    pub keys: &'a [LinuxKey],
    pub misc: &'a [u16],
    pub properties: &'a [u16],
}

const REFERENCE_ENV: Environment<'static> = Environment {
    model: "M2101K6P",
    device: "sweetin",
    api: 37,
    rom: "PixelOS_sweet-17.0-20261030-2116",
    build_id: "CP2A.260605.016",
    incremental: "1793394994",
    kernel_release: "4.14.357-openela-VantomKernel",
    kernel_build: "#1 SMP PREEMPT Sun Oct 11 07:45:56 UTC 2026",
};

const REFERENCE_USB: UsbHid = UsbHid {
    vendor: 0x0020,
    product: 0x0b21,
    revision: 0x0100,
    interface: 3,
    interface_class: 3,
    hid_bus: 3,
    hid_vendor: 0x0020,
    hid_product: 0x0b21,
};

const REFERENCE_KEYS: [LinuxKey; 8] = [
    LinuxKey(0x71),
    LinuxKey(0x72),
    LinuxKey(0x73),
    LinuxKey(0x8b),
    LinuxKey(0xa3),
    LinuxKey(0xa4),
    LinuxKey(0xa5),
    LinuxKey(0x246),
];

impl Evidence {
    pub fn controls(self) -> &'static [Control] {
        match self {
            Self::XiaomiPixelos20261107 => &CONTROLS,
        }
    }

    pub fn matches_environment(self, environment: &Environment<'_>) -> bool {
        match self {
            Self::XiaomiPixelos20261107 => *environment == REFERENCE_ENV,
        }
    }

    pub(crate) fn matches_identity(self, serial: &str, device: &Candidate<'_>) -> bool {
        match self {
            Self::XiaomiPixelos20261107 => {
                !device.is_virtual
                    && device.usb_serial == Some(serial)
                    && device.usb_driver == Some("usbhid")
                    && device.usb_hid.as_ref() == Some(&REFERENCE_USB)
                    && device.input_id == [3, 0x0020, 0x0b21, 0x0201]
                    && device.name == "Generic Xiaomi Type-C Earphones 1"
            }
        }
    }

    pub(crate) fn matches_capabilities(self, device: &Candidate<'_>) -> bool {
        match self {
            Self::XiaomiPixelos20261107 => {
                device.layout_sha256
                    == "4d28d904e72ea3beb4d32d73437cf9e7f1ed297a436d51d53b1a248a5cd5f773"
                    && device.event_types == [0, 1, 4]
                    && device.keys == REFERENCE_KEYS
                    && device.misc == [4]
                    && device.properties.is_empty()
            }
        }
    }
}

// Only the trusted enrollment path may supply these. Policy JSON cannot create one.
pub struct VerifiedEnrollment<'a> {
    evidence: Evidence,
    serial: &'a str,
}

impl<'a> VerifiedEnrollment<'a> {
    pub fn from_reviewed_measurement(evidence: Evidence, serial: &'a str) -> Option<Self> {
        valid_serial(serial).then_some(Self { evidence, serial })
    }

    pub(crate) fn matches(&self, evidence: Evidence, serial: &str) -> bool {
        self.evidence == evidence && self.serial == serial
    }
}

fn valid_serial(serial: &str) -> bool {
    !serial.trim().is_empty() && serial.len() <= 128 && !serial.chars().any(char::is_control)
}
