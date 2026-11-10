//! Proxy check: the read-only check, then a proxy that is created, observed and destroyed.
//!
//! It never grabs the physical node and never writes an event to the proxy.
//! The proxy is destroyed on every return path.

use crate::check::{self, Checked, Report};
use crate::environment::Observed;
use crate::inventory::{self, Observation};
use crate::layout;
use crate::proxy::{PROXY_NAME, Proxy, node_removed, wait_ready};
use earkey_policy::identity::{Environment, Evidence, LinuxKey, VerifiedEnrollment};
use std::io;
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

const GENERIC_LAYOUT: &str = "/system/usr/keylayout/Generic.kl";
const TEARDOWN: Duration = Duration::from_secs(2);

/// Android's view of one input device, from `dumpsys input`.
#[derive(Debug, Default, Eq, PartialEq)]
pub struct Classification {
    pub classes: String,
    pub path: String,
    pub key_layout: String,
    pub key_character_map: String,
    pub configuration: String,
}

/// Finds the Event Hub block for `name` in `dumpsys input` output.
pub fn classify(text: &str, name: &str) -> Option<Classification> {
    let mut lines = text.lines();
    lines.find(|line| header(line) == Some(name))?;
    let mut found = Classification::default();
    for line in lines {
        if header(line).is_some() || line.trim().is_empty() {
            break;
        }
        let Some((key, value)) = line.trim().split_once(": ") else {
            continue;
        };
        let slot = match key {
            "Classes" => &mut found.classes,
            "Path" => &mut found.path,
            "KeyLayoutFile" => &mut found.key_layout,
            "KeyCharacterMapFile" => &mut found.key_character_map,
            "ConfigurationFile" => &mut found.configuration,
            _ => continue,
        };
        *slot = value.to_owned();
    }
    (!found.classes.is_empty()).then_some(found)
}

/// The device name of an Event Hub header line such as `    12: Name`.
fn header(line: &str) -> Option<&str> {
    let (number, rest) = line.trim_start().split_once(": ")?;
    let number = number.strip_prefix('-').unwrap_or(number);
    (!number.is_empty() && number.bytes().all(|b| b.is_ascii_digit())).then_some(rest)
}

/// The evidence's control keys in ascending order, matching how the inventory reports bitmaps.
pub fn proxy_keys(evidence: Evidence) -> Vec<LinuxKey> {
    let mut keys: Vec<LinuxKey> = evidence.controls().iter().map(|c| c.linux).collect();
    keys.sort_by_key(|key| key.0);
    keys
}

/// True if an inventoried node is the expected proxy and cannot be enrolled.
pub fn excluded(
    observed: &Observation,
    checked: Checked,
    environment: &Environment<'_>,
    layout: &str,
) -> bool {
    observed.is_virtual
        && observed.usb_hid.is_none()
        && observed.name == PROXY_NAME
        && observed.input_id == checked.input_id
        && observed.event_types == [0, 1, 4]
        && observed.keys == proxy_keys(checked.evidence)
        && observed.misc == [4]
        && VerifiedEnrollment::from_observation(
            checked.evidence,
            environment,
            &observed.candidate(layout),
        )
        .is_none()
}

/// Runs the check, then creates, observes and destroys the proxy.
pub fn run(
    root: &Path,
    environment: &Observed,
    deadline: Duration,
    dumpsys: impl FnOnce() -> io::Result<String>,
) -> Report {
    let mut report = check::run(root, environment);
    let Some(checked) = report.checked else {
        report.note("proxy", "skipped");
        return report;
    };
    report.passed = false;
    let fail = |mut report: Report, key: &'static str, value: String| {
        report.note(key, value);
        report.note("proxy_check", "fail");
        report
    };
    let keys = proxy_keys(checked.evidence);
    let (sys, dev) = (root.join("sys"), root.join("dev"));
    let proxy = match Proxy::create(&dev, checked.input_id, &keys) {
        Ok(proxy) => proxy,
        Err(error) => return fail(report, "proxy", format!("error:{:?}", error.kind())),
    };
    report.note("proxy", "created");
    let ready = match wait_ready(&sys, &dev, &root.join("proc"), &proxy.sysname, deadline) {
        Ok(ready) => ready,
        Err(error) => return fail(report, "proxy_ready", format!("error:{:?}", error.kind())),
    };
    report.note("proxy_ready_ms", ready.elapsed.as_millis());
    report.note("proxy_event", &ready.event);

    let layout = match layout::generic_digest(root) {
        Ok(layout) => layout,
        Err(error) => return fail(report, "proxy_layout", format!("error:{:?}", error.kind())),
    };
    let inventory = match inventory::scan(&sys, &dev) {
        Ok(inventory) => inventory,
        Err(error) => {
            return fail(
                report,
                "proxy_inventory",
                format!("error:{:?}", error.kind()),
            );
        }
    };
    let env = environment.as_environment();
    let excluded = inventory
        .iter()
        .find(|o| o.event == ready.event)
        .is_some_and(|o| excluded(o, checked, &env, &layout));
    if !excluded {
        return fail(report, "proxy_excluded", "no".into());
    }
    report.note("proxy_excluded", "yes");

    let text = match dumpsys() {
        Ok(text) => text,
        Err(error) => return fail(report, "proxy_dumpsys", format!("error:{:?}", error.kind())),
    };
    let Some(class) = classify(&text, PROXY_NAME) else {
        return fail(report, "proxy_android", "absent".into());
    };
    report.note("proxy_classes", &class.classes);
    report.note("proxy_layout", &class.key_layout);
    report.note("proxy_kcm", &class.key_character_map);
    report.note("proxy_configuration", &class.configuration);
    let classes: Vec<&str> = class.classes.split(" | ").collect();
    if !classes.contains(&"KEYBOARD")
        || !classes.contains(&"EXTERNAL")
        || class.key_layout != GENERIC_LAYOUT
        || class.configuration != "<none>"
        || class.path != format!("/dev/input/{}", ready.event)
    {
        return fail(report, "proxy_android", "mismatch".into());
    }
    report.note("proxy_android", "ok");

    drop(proxy);
    let start = Instant::now();
    while !node_removed(&dev, &ready.event) && start.elapsed() < TEARDOWN {
        thread::sleep(Duration::from_millis(10));
    }
    if !node_removed(&dev, &ready.event) {
        return fail(report, "proxy_teardown", "node_remains".into());
    }
    report.note("proxy_teardown", "ok");
    report.note("proxy_check", "pass");
    report.passed = true;
    report
}
