//! Recovery gate backed by the private policy store.

use crate::Gate;
use earkey_policy::policy::{Recovery, RecoveryLatch};
use earkey_policy::store::{Store, StoreError};
use std::io;

/// Maps durable recovery state and the pre-open marker to a supervisor gate.
pub fn gate(recovery: Recovery, stay_inactive: bool) -> Gate {
    Gate {
        paused: recovery.paused,
        boot_disabled: recovery.boot_disabled || stay_inactive,
        latched: recovery.latch == RecoveryLatch::RestartLimit,
    }
}

/// Reads the gate without enrollments. Any error must keep the worker stopped.
pub fn read_gate(store: &Store) -> io::Result<Gate> {
    let recovery = store.recovery().map_err(into_io)?;
    let stay_inactive = store.has_inactive_marker().map_err(into_io)?;
    Ok(gate(recovery, stay_inactive))
}

/// Persists the restart latch.
pub fn persist_latch(store: &Store) -> io::Result<()> {
    store.latch_restart_limit().map_err(into_io)
}

fn into_io(error: StoreError) -> io::Error {
    match error {
        StoreError::Io(error) => error,
        error => io::Error::other(format!("{error:?}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marker_and_recovery_fields_close_the_gate() {
        let clear = Recovery {
            paused: false,
            boot_disabled: false,
            latch: RecoveryLatch::Clear,
        };
        assert_eq!(gate(clear, false), Gate::OPEN);
        assert!(gate(clear, true).boot_disabled);
        assert!(
            gate(
                Recovery {
                    paused: true,
                    ..clear
                },
                false
            )
            .paused
        );
        assert!(
            gate(
                Recovery {
                    boot_disabled: true,
                    ..clear
                },
                false
            )
            .boot_disabled
        );
        assert!(
            gate(
                Recovery {
                    latch: RecoveryLatch::RestartLimit,
                    ..clear
                },
                false
            )
            .latched
        );
    }
}
