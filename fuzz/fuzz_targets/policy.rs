#![no_main]

use earkey_policy::identity::{Evidence, VerifiedEnrollment};
use earkey_policy::policy::Policy;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|bytes: &[u8]| {
    let enrollments = [VerifiedEnrollment::from_reviewed_measurement(
        Evidence::XiaomiPixelos20261107,
        "SYNTHETIC-ENROLLMENT",
    )
    .unwrap()];
    if let Ok(policy) = Policy::parse(bytes, &enrollments) {
        let canonical = policy.to_json();
        assert_eq!(
            Policy::parse(&canonical, &enrollments).unwrap().to_json(),
            canonical,
        );
    }
});
