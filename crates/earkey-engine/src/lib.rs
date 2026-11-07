use earkey_policy::identity::{Evidence, Group, LinuxKey};
use std::io::{self, ErrorKind};
use std::ops::Range;
use std::time::{Duration, Instant};

const EV_SYN: u16 = 0;
const EV_KEY: u16 = 1;
const EV_MSC: u16 = 4;
const SYN_REPORT: u16 = 0;
const SYN_DROPPED: u16 = 3;
const MSC_SCAN: u16 = 4;
const MAX_EVENTS: usize = 64;
const MAX_INTERRUPTS: u8 = 8;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Event {
    pub timestamp_us: i64,
    pub kind: u16,
    pub code: u16,
    pub value: i32,
}

pub trait Source {
    /// Reads complete records into the supplied buffer. Zero means disconnection.
    /// The transport must be nonblocking and reject a partial native record.
    fn read_events(&mut self, buffer: &mut [Event]) -> io::Result<usize>;

    /// Fills the buffer from a fresh key-state query after the recovery report.
    fn query_pressed(&mut self, buffer: &mut [LinuxKey]) -> io::Result<usize>;
}

pub trait Sink {
    /// Writes a prefix of complete records without blocking. A retryable error
    /// must mean that the sink accepted none of the supplied records.
    fn write_events(&mut self, events: &[Event]) -> io::Result<usize>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Activity {
    Progress,
    WaitInput,
    WaitOutput,
    Retry,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Fault {
    Disconnected,
    Read(ErrorKind),
    Write(ErrorKind),
    InvalidReadCount,
    InvalidWriteCount,
    MalformedFrame,
    FrameTooLong,
    FrameDeadline,
    OutputDeadline,
    InvalidDeadline,
    InterruptedTooOften,
    StateQuery(ErrorKind),
    InvalidState,
    SyncLost { physical_keys_down: u32 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PolicyError {
    StaleGeneration,
    UnsupportedGroup,
}

pub struct Engine {
    evidence: Evidence,
    generation: u64,
    selected: u64,
    frame_selection: u64,
    pressed: u64,
    forwarding: u64,
    published: u64,
    input: [Event; MAX_EVENTS],
    unread: Range<usize>,
    frame: Vec<Event>,
    raw_count: usize,
    scan: Option<Event>,
    written: Option<usize>,
    attempted_write: bool,
    dropped: bool,
    interrupts: u8,
    deadline: Option<Instant>,
    stall_limit: Duration,
    fault: Option<Fault>,
}

impl Engine {
    /// Starts a session after the caller has independently verified neutral
    /// physical and proxy states. An empty selection requests passthrough.
    pub fn new(
        evidence: Evidence,
        generation: u64,
        groups: &[Group],
        stall_limit: Duration,
    ) -> Result<Self, PolicyError> {
        Ok(Self {
            evidence,
            generation,
            selected: selection(evidence, groups)?,
            frame_selection: 0,
            pressed: 0,
            forwarding: 0,
            published: 0,
            input: [Event::default(); MAX_EVENTS],
            unread: 0..0,
            frame: Vec::with_capacity(MAX_EVENTS),
            raw_count: 0,
            scan: None,
            written: None,
            attempted_write: false,
            dropped: false,
            interrupts: 0,
            deadline: None,
            stall_limit,
            fault: None,
        })
    }

    /// Selects a newer generation for new frames. Held keys keep the decision
    /// assigned at press time. This is local engine state, before worker acknowledgment.
    pub fn select_policy(&mut self, generation: u64, groups: &[Group]) -> Result<(), PolicyError> {
        if generation <= self.generation {
            return Err(PolicyError::StaleGeneration);
        }
        self.selected = selection(self.evidence, groups)?;
        self.generation = generation;
        Ok(())
    }

    /// The caller must wake the engine by this deadline even without I/O readiness.
    pub fn deadline(&self) -> Option<Instant> {
        self.deadline
    }

    /// Includes presses from an attempted partial frame because a faulty sink
    /// may have accepted a prefix. Use the set only for cleanup.
    pub fn possibly_forwarded_keys(&self) -> impl Iterator<Item = LinuxKey> {
        let mut keys = self.published;
        if self.attempted_write {
            for event in &self.frame {
                if event.kind == EV_KEY && event.value == 1 {
                    keys |= key_bit(self.evidence, LinuxKey(event.code)).unwrap_or(0);
                }
            }
        }
        self.evidence
            .advertised_keys()
            .iter()
            .enumerate()
            .filter_map(move |(index, key)| (keys & (1 << index) != 0).then_some(*key))
    }

    /// Advances one input record or output write. A fault is terminal for this
    /// session. The caller must stop interception and clean up its descriptors.
    pub fn step(
        &mut self,
        source: &mut impl Source,
        sink: &mut impl Sink,
        now: Instant,
    ) -> Result<Activity, Fault> {
        if let Some(fault) = self.fault {
            return Err(fault);
        }
        let result = self.advance(source, sink, now);
        if let Err(fault) = result {
            self.fault = Some(fault);
        }
        result
    }

    fn advance(
        &mut self,
        source: &mut impl Source,
        sink: &mut impl Sink,
        now: Instant,
    ) -> Result<Activity, Fault> {
        if self.deadline.is_some_and(|deadline| now >= deadline) {
            return Err(if self.written.is_some() {
                Fault::OutputDeadline
            } else {
                Fault::FrameDeadline
            });
        }
        if let Some(written) = self.written {
            return self.flush(sink, written);
        }
        if self.unread.is_empty() {
            let count = match source.read_events(&mut self.input) {
                Ok(0) => return Err(Fault::Disconnected),
                Ok(count) if count <= MAX_EVENTS => count,
                Ok(_) => return Err(Fault::InvalidReadCount),
                Err(error) => return self.io_failure(error.kind(), Activity::WaitInput),
            };
            self.unread = 0..count;
        }

        self.interrupts = 0;
        let event = self.input[self.unread.next().expect("buffered input")];
        self.accept(event, source, now)?;
        Ok(Activity::Progress)
    }

    fn accept(
        &mut self,
        event: Event,
        source: &mut impl Source,
        now: Instant,
    ) -> Result<(), Fault> {
        if !self.dropped && (event.kind, event.code) == (EV_SYN, SYN_DROPPED) {
            self.frame.clear();
            self.scan = None;
            self.raw_count = 0;
            self.dropped = true;
            self.arm_deadline(now)?;
            return Ok(());
        }
        if (event.kind, event.code) == (EV_SYN, SYN_REPORT) {
            if self.dropped {
                return self.query_after_drop(source);
            }
            if self.scan.is_some() {
                return Err(Fault::MalformedFrame);
            }
            if !self.frame.is_empty() || self.raw_count == 0 {
                self.frame.push(event);
                self.written = Some(0);
                self.arm_deadline(now)?;
            } else {
                self.deadline = None;
            }
            self.raw_count = 0;
            return Ok(());
        }

        self.raw_count += 1;
        if self.raw_count >= MAX_EVENTS {
            return Err(Fault::FrameTooLong);
        }
        if self.dropped {
            return Ok(());
        }
        if self.raw_count == 1 {
            self.frame_selection = self.selected;
            self.arm_deadline(now)?;
        }
        match (event.kind, event.code) {
            (EV_MSC, MSC_SCAN) if self.scan.is_none() => self.scan = Some(event),
            (EV_KEY, _) => self.accept_key(event)?,
            _ => return Err(Fault::MalformedFrame),
        }
        Ok(())
    }

    fn arm_deadline(&mut self, now: Instant) -> Result<(), Fault> {
        self.deadline = Some(
            now.checked_add(self.stall_limit)
                .ok_or(Fault::InvalidDeadline)?,
        );
        Ok(())
    }

    fn accept_key(&mut self, event: Event) -> Result<(), Fault> {
        let key = LinuxKey(event.code);
        let bit = key_bit(self.evidence, key).ok_or(Fault::MalformedFrame)?;
        let scan = self.scan.take();
        if let Some(scan) = scan
            && let Some(control) = self
                .evidence
                .controls()
                .iter()
                .find(|control| control.linux == key)
        {
            let expected = (u32::from(control.hid.page) << 16) | u32::from(control.hid.usage);
            if scan.value as u32 != expected {
                return Err(Fault::MalformedFrame);
            }
        }

        let down = self.pressed & bit != 0;
        if event.value == 1 && !down {
            self.pressed |= bit;
            if self.frame_selection & bit == 0 {
                self.forwarding |= bit;
            }
        } else if !down || !matches!(event.value, 0 | 2) {
            return Err(Fault::MalformedFrame);
        }
        if self.forwarding & bit != 0 {
            self.frame.extend(scan);
            self.frame.push(event);
        }
        if event.value == 0 {
            self.pressed &= !bit;
            self.forwarding &= !bit;
        }
        Ok(())
    }

    fn flush(&mut self, sink: &mut impl Sink, written: usize) -> Result<Activity, Fault> {
        self.attempted_write = true;
        let remaining = &self.frame[written..];
        let count = match sink.write_events(remaining) {
            Ok(0) => return Err(Fault::Write(ErrorKind::WriteZero)),
            Ok(count) if count <= remaining.len() => count,
            Ok(_) => return Err(Fault::InvalidWriteCount),
            Err(error) => return self.io_failure(error.kind(), Activity::WaitOutput),
        };

        self.interrupts = 0;
        let end = written + count;
        if end == self.frame.len() {
            self.published = self.forwarding;
            self.frame.clear();
            self.written = None;
            self.attempted_write = false;
            self.deadline = None;
        } else {
            self.written = Some(end);
        }
        Ok(Activity::Progress)
    }

    fn io_failure(&mut self, error: ErrorKind, waiting: Activity) -> Result<Activity, Fault> {
        match error {
            ErrorKind::WouldBlock => {
                self.interrupts = 0;
                Ok(waiting)
            }
            ErrorKind::Interrupted => {
                self.interrupts += 1;
                if self.interrupts == MAX_INTERRUPTS {
                    return Err(Fault::InterruptedTooOften);
                }
                Ok(Activity::Retry)
            }
            _ if waiting == Activity::WaitInput => Err(Fault::Read(error)),
            _ => Err(Fault::Write(error)),
        }
    }

    fn query_after_drop(&self, source: &mut impl Source) -> Result<(), Fault> {
        let mut keys = [LinuxKey(0); 64];
        let count = source
            .query_pressed(&mut keys)
            .map_err(|error| Fault::StateQuery(error.kind()))?;
        let mut down = 0u64;
        for &key in keys.get(..count).ok_or(Fault::InvalidState)? {
            let bit = key_bit(self.evidence, key).ok_or(Fault::InvalidState)?;
            if down & bit != 0 {
                return Err(Fault::InvalidState);
            }
            down |= bit;
        }
        // A key snapshot cannot recover lost transitions, scans or repeat ownership.
        Err(Fault::SyncLost {
            physical_keys_down: down.count_ones(),
        })
    }
}

fn key_bit(evidence: Evidence, key: LinuxKey) -> Option<u64> {
    let index = evidence
        .advertised_keys()
        .iter()
        .position(|&candidate| candidate == key)?;
    1u64.checked_shl(index as u32)
}

fn selection(evidence: Evidence, groups: &[Group]) -> Result<u64, PolicyError> {
    let mut selected = 0;
    for &group in groups {
        let control = evidence
            .controls()
            .iter()
            .find(|control| control.group == group);
        let bit = control
            .and_then(|control| key_bit(evidence, control.linux))
            .ok_or(PolicyError::UnsupportedGroup)?;
        selected |= bit;
    }
    Ok(selected)
}
