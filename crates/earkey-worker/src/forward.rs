//! Passthrough session: grab the verified node and forward through the proxy.
//!
//! The session grabs only after the proxy is ready, discards the descriptor's
//! pre-grab queue, and abandons if that queue held a key event or any key is
//! still down, because those events already reached Android. It runs the host
//! engine with `poll`, reports progress from the loop itself, honors an
//! immediate stop flag and a total session limit, and on every exit releases
//! keys that may be down on the proxy before releasing the grab.

use crate::device::Opened;
use crate::proxy::Proxy;
use earkey_engine::{Activity, Engine, Event, Fault, Sink, Source};
use earkey_policy::identity::{Evidence, Group, LinuxKey};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{self, Read, Write};
use std::os::fd::AsRawFd;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

const RECORD: usize = std::mem::size_of::<libc::input_event>();
const BATCH: usize = 64;
const EV_SYN: u16 = 0;
const EV_KEY: u16 = 1;

/// Bounds for one supervised session.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// Incomplete-frame and output deadline passed to the engine.
    pub stall: Duration,
    /// Total session length.
    pub session: Duration,
    /// Maximum wait between loop iterations, so stop and progress stay timely.
    pub tick: Duration,
}

/// Why a session ended. Every end has released the proxy keys and the grab.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum End {
    Stopped,
    SessionLimit,
    /// A key was down, or a key event was queued, at the grab transition.
    NotNeutral,
    Fault(Fault),
}

/// Native `struct input_event` records from a nonblocking descriptor.
pub struct NativeSource<'a> {
    file: &'a File,
    opened: Option<&'a Opened>,
    counts: Counts,
}

impl Source for NativeSource<'_> {
    fn read_events(&mut self, buffer: &mut [Event]) -> io::Result<usize> {
        let mut bytes = [0u8; RECORD * BATCH];
        let limit = buffer.len().min(BATCH) * RECORD;
        let read = (&*self.file).read(&mut bytes[..limit])?;
        if read % RECORD != 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "partial input_event",
            ));
        }
        for (slot, chunk) in buffer.iter_mut().zip(bytes[..read].as_chunks::<RECORD>().0) {
            // SAFETY: `chunk` holds exactly one `input_event`; read_unaligned allows any alignment.
            let native: libc::input_event =
                unsafe { std::ptr::read_unaligned(chunk.as_ptr().cast()) };
            *slot = Event {
                timestamp_us: (native.time.tv_sec as i64)
                    .saturating_mul(1_000_000)
                    .saturating_add(native.time.tv_usec as i64),
                kind: native.type_,
                code: native.code,
                value: native.value,
            };
            self.counts.add(slot);
        }
        Ok(read / RECORD)
    }

    fn query_pressed(&mut self, buffer: &mut [LinuxKey]) -> io::Result<usize> {
        let opened = self.opened.ok_or(io::ErrorKind::Unsupported)?;
        let keys = opened.pressed_keys()?;
        let count = keys.len().min(buffer.len());
        buffer[..count].copy_from_slice(&keys[..count]);
        Ok(count)
    }
}

impl<'a> NativeSource<'a> {
    /// Reads from an opened physical node, with key-state queries.
    pub fn physical(opened: &'a Opened) -> Self {
        Self {
            file: opened.file(),
            opened: Some(opened),
            counts: Counts::default(),
        }
    }
}

/// Native records written to the proxy; uinput accepts whole records only.
pub struct ProxySink<'a>(&'a File, Counts);

impl Sink for ProxySink<'_> {
    fn write_events(&mut self, events: &[Event]) -> io::Result<usize> {
        let bytes = encode(&events[..events.len().min(BATCH)]);
        let written = (&*self.0).write(&bytes)?;
        if written % RECORD != 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "partial uinput write",
            ));
        }
        for event in &events[..written / RECORD] {
            self.1.add(event);
        }
        Ok(written / RECORD)
    }
}

/// Encodes records with zero timestamps; uinput stamps them on write.
pub fn encode(events: &[Event]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(events.len() * RECORD);
    for event in events {
        // SAFETY: An all-zero input_event is valid.
        let mut native: libc::input_event = unsafe { std::mem::zeroed() };
        native.type_ = event.kind;
        native.code = event.code;
        native.value = event.value;
        // SAFETY: `native` is a plain-old-data struct of RECORD bytes.
        let raw = unsafe { std::slice::from_raw_parts((&raw const native).cast::<u8>(), RECORD) };
        bytes.extend_from_slice(raw);
    }
    bytes
}

/// Key records by code and value (0 up, 1 down, 2 repeat). Other records are not counted.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Counts(pub BTreeMap<(u16, i32), u64>);

impl Counts {
    fn add(&mut self, event: &Event) {
        if event.kind == EV_KEY {
            *self.0.entry((event.code, event.value)).or_default() += 1;
        }
    }

    /// `0xCODE:down=N,repeat=N,up=N` per key, `;`-separated; `none` when empty.
    pub fn summary(&self) -> String {
        let mut codes: Vec<u16> = self.0.keys().map(|(code, _)| *code).collect();
        codes.dedup();
        if codes.is_empty() {
            return "none".into();
        }
        let get = |code, value| self.0.get(&(code, value)).copied().unwrap_or(0);
        codes
            .iter()
            .map(|&code| {
                format!(
                    "{code:#04x}:down={},repeat={},up={}",
                    get(code, 1),
                    get(code, 2),
                    get(code, 0)
                )
            })
            .collect::<Vec<_>>()
            .join(";")
    }
}

/// Key records read from the physical node, forwarded to the proxy, and released at cleanup.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Tally {
    pub read: Counts,
    pub forwarded: Counts,
    pub cleanup_releases: usize,
}

/// Runs one passthrough or suppression session. `progress` receives a strictly
/// increasing counter from the loop itself, including idle waits.
#[allow(clippy::too_many_arguments)]
pub fn run(
    opened: &Opened,
    proxy: &Proxy,
    evidence: Evidence,
    groups: &[Group],
    limits: Limits,
    stop: &AtomicBool,
    tally: &mut Tally,
    mut progress: impl FnMut(u64),
) -> io::Result<End> {
    let mut engine = Engine::new(evidence, 1, groups, limits.stall)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "unsupported group"))?;
    opened.grab(true)?;
    let end = session(
        opened,
        proxy,
        &mut engine,
        limits,
        stop,
        tally,
        &mut progress,
    );
    tally.cleanup_releases = release(proxy, &engine);
    let ungrab = opened.grab(false);
    match (end?, ungrab) {
        (end, Ok(())) => Ok(end),
        // A faulted session may have lost its device; closing the descriptor releases the grab.
        (End::Fault(fault), Err(_)) => Ok(End::Fault(fault)),
        (_, Err(error)) => Err(error),
    }
}

fn session(
    opened: &Opened,
    proxy: &Proxy,
    engine: &mut Engine,
    limits: Limits,
    stop: &AtomicBool,
    tally: &mut Tally,
    progress: &mut impl FnMut(u64),
) -> io::Result<End> {
    if drain_had_keys(opened)? || !opened.pressed_keys()?.is_empty() {
        return Ok(End::NotNeutral);
    }
    let mut source = NativeSource::physical(opened);
    let mut sink = ProxySink(proxy.file(), Counts::default());
    let end = pump(
        opened,
        proxy,
        engine,
        limits,
        stop,
        &mut source,
        &mut sink,
        progress,
    );
    tally.read = source.counts;
    tally.forwarded = sink.1;
    end
}

#[allow(clippy::too_many_arguments)]
fn pump(
    opened: &Opened,
    proxy: &Proxy,
    engine: &mut Engine,
    limits: Limits,
    stop: &AtomicBool,
    source: &mut NativeSource<'_>,
    sink: &mut ProxySink<'_>,
    progress: &mut impl FnMut(u64),
) -> io::Result<End> {
    let end_at = Instant::now() + limits.session;
    let mut counter = 0u64;
    loop {
        if stop.load(Ordering::SeqCst) {
            return Ok(End::Stopped);
        }
        let now = Instant::now();
        if now >= end_at {
            return Ok(End::SessionLimit);
        }
        let activity = match engine.step(source, sink, now) {
            Ok(activity) => activity,
            Err(fault) => return Ok(End::Fault(fault)),
        };
        let wait = match activity {
            Activity::Progress | Activity::Retry => None,
            Activity::WaitInput => Some((opened.file().as_raw_fd(), libc::POLLIN)),
            Activity::WaitOutput => Some((proxy.file().as_raw_fd(), libc::POLLOUT)),
        };
        if let Some((fd, events)) = wait {
            let mut timeout = limits.tick.min(end_at - now);
            if let Some(deadline) = engine.deadline() {
                timeout = timeout.min(deadline.saturating_duration_since(now));
            }
            let mut poll = libc::pollfd {
                fd,
                events,
                revents: 0,
            };
            // SAFETY: One valid pollfd is passed for the duration of the call.
            let result = unsafe {
                libc::poll(
                    &mut poll,
                    1,
                    timeout.as_millis().min(i32::MAX as u128) as i32,
                )
            };
            if result < 0 && io::Error::last_os_error().kind() != io::ErrorKind::Interrupted {
                return Err(io::Error::last_os_error());
            }
        }
        counter += 1;
        progress(counter);
    }
}

/// Discards records queued before the grab. They already reached Android.
fn drain_had_keys(opened: &Opened) -> io::Result<bool> {
    let mut source = NativeSource::physical(opened);
    let mut buffer = [Event::default(); BATCH];
    let mut keys = false;
    loop {
        match source.read_events(&mut buffer) {
            Ok(0) => return Err(io::ErrorKind::UnexpectedEof.into()),
            Ok(count) => keys |= buffer[..count].iter().any(|event| event.kind == EV_KEY),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => return Ok(keys),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error),
        }
    }
}

/// Writes complete records to a proxy.
pub fn emit(proxy: &Proxy, events: &[Event]) -> io::Result<()> {
    proxy.file().write_all(&encode(events))
}

/// Best-effort release of every key that may be down on the proxy; returns how many.
fn release(proxy: &Proxy, engine: &Engine) -> usize {
    let mut events: Vec<Event> = engine
        .possibly_forwarded_keys()
        .map(|key| Event {
            timestamp_us: 0,
            kind: EV_KEY,
            code: key.0,
            value: 0,
        })
        .collect();
    let released = events.len();
    if released == 0 {
        return 0;
    }
    events.push(Event {
        timestamp_us: 0,
        kind: EV_SYN,
        code: 0,
        value: 0,
    });
    let _ = emit(proxy, &events);
    released
}
