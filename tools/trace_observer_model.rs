//! Host-testable event reduction for the proposed arm64 KernelSU observer.
//! No tracing, process creation, device access, or output is implemented here.
#![forbid(unsafe_code)]

const SYS_IOCTL: u64 = 29;
const SYS_REBOOT: u64 = 142;
const SYS_EXIT: u64 = 93;
const SYS_EXIT_GROUP: u64 = 94;
const INSTALL_MAGIC1: u64 = 0xdeadbeef;
const INSTALL_MAGIC2: u64 = 0xcafebabe;
const GET_INFO: u64 = 0x80104b02;
const GET_INFO_LEGACY: u64 = 0x80004b02;
const MAX_RELEVANT: u16 = 64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Kind {
    Other,
    Terminate,
    Acquire,
    Info,
    LegacyInfo,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct Counts {
    success: u16,
    failure: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Summary {
    acquire: Counts,
    info: Counts,
    legacy: Counts,
    complete: bool,
}

struct Observer {
    child: u32,
    pending: Option<Kind>,
    summary: Summary,
    finished: bool,
    observed: bool,
}

impl Observer {
    fn new(child: u32) -> Option<Self> {
        (child != 0).then_some(Self {
            child,
            pending: None,
            finished: false,
            observed: false,
            summary: Summary {
                acquire: Counts::default(),
                info: Counts::default(),
                legacy: Counts::default(),
                complete: true,
            },
        })
    }

    fn entry(&mut self, tid: u32, nr: u64, arg0: u64, arg1: u64) {
        if self.finished {
            return;
        }
        if tid != self.child || self.pending.is_some() {
            self.invalidate();
            return;
        }
        self.observed = true;
        self.pending = Some(match (nr, arg0, arg1) {
            (SYS_REBOOT, INSTALL_MAGIC1, INSTALL_MAGIC2) => Kind::Acquire,
            (SYS_IOCTL, _, GET_INFO) => Kind::Info,
            (SYS_IOCTL, _, GET_INFO_LEGACY) => Kind::LegacyInfo,
            (SYS_EXIT | SYS_EXIT_GROUP, _, _) => Kind::Terminate,
            _ => Kind::Other,
        });
    }

    fn exit(&mut self, tid: u32, result: i64) {
        if self.finished {
            return;
        }
        if tid != self.child {
            self.invalidate();
            return;
        }
        let Some(kind) = self.pending.take() else {
            self.invalidate();
            return;
        };
        let counts = match kind {
            Kind::Other => return,
            Kind::Terminate => {
                self.invalidate();
                return;
            }
            Kind::Acquire => &mut self.summary.acquire,
            Kind::Info => &mut self.summary.info,
            Kind::LegacyInfo => &mut self.summary.legacy,
        };
        let value = if result < 0 {
            &mut counts.failure
        } else {
            &mut counts.success
        };
        if *value == MAX_RELEVANT {
            self.invalidate();
        } else {
            *value += 1;
        }
    }

    fn finish(&mut self, exited_cleanly: bool) -> Summary {
        if !self.finished {
            if !exited_cleanly
                || !self.observed
                || (self.pending.is_some() && self.pending != Some(Kind::Terminate))
            {
                self.summary.complete = false;
            }
            self.finished = true;
        }
        self.summary
    }

    fn invalidate(&mut self) {
        self.summary.complete = false;
        self.finished = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_only_allowlisted_calls_as_success_or_failure() {
        let mut t = Observer::new(12).unwrap();
        t.entry(12, SYS_REBOOT, INSTALL_MAGIC1, INSTALL_MAGIC2);
        t.exit(12, 0);
        t.entry(12, SYS_IOCTL, 99, GET_INFO);
        t.exit(12, -1);
        t.entry(12, SYS_IOCTL, 99, GET_INFO_LEGACY);
        t.exit(12, 0);
        t.entry(12, SYS_IOCTL, 99, 0x1234);
        t.exit(12, 0);
        t.entry(12, SYS_REBOOT, 1, 2);
        t.exit(12, 0);
        assert_eq!(
            t.finish(true),
            Summary {
                acquire: Counts {
                    success: 1,
                    failure: 0
                },
                info: Counts {
                    success: 0,
                    failure: 1
                },
                legacy: Counts {
                    success: 1,
                    failure: 0
                },
                complete: true
            }
        );
    }

    #[test]
    fn mismatched_threads_unpaired_stops_and_timeout_are_incomplete() {
        assert!(Observer::new(0).is_none());
        let mut t = Observer::new(12).unwrap();
        t.entry(13, SYS_IOCTL, 3, GET_INFO);
        assert!(!t.finish(true).complete);
        let mut t = Observer::new(12).unwrap();
        t.exit(12, 0);
        assert!(!t.finish(true).complete);
        let mut t = Observer::new(12).unwrap();
        t.entry(12, SYS_IOCTL, 3, GET_INFO);
        assert!(!t.finish(true).complete);
        let mut t = Observer::new(12).unwrap();
        assert!(!t.finish(false).complete);
    }

    #[test]
    fn overflow_stops_classification_and_never_wraps() {
        let mut t = Observer::new(12).unwrap();
        for _ in 0..=MAX_RELEVANT {
            t.entry(12, SYS_IOCTL, 9, GET_INFO);
            t.exit(12, 0);
        }
        let s = t.finish(true);
        assert!(!s.complete);
        assert_eq!(s.info.success, MAX_RELEVANT);
        t.entry(12, SYS_IOCTL, 9, GET_INFO);
        t.exit(12, 0);
        assert_eq!(t.finish(true), s);
    }

    #[test]
    fn process_exit_needs_an_observation_and_allows_exit_group_without_return() {
        let mut empty = Observer::new(12).unwrap();
        assert!(!empty.finish(true).complete);
        let mut t = Observer::new(12).unwrap();
        t.entry(12, SYS_EXIT_GROUP, 0, 0);
        assert!(t.finish(true).complete);
        let mut returned = Observer::new(12).unwrap();
        returned.entry(12, SYS_EXIT, 0, 0);
        returned.exit(12, 0);
        assert!(!returned.finish(true).complete);
    }
}
