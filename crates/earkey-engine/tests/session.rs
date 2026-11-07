use earkey_engine::{Activity, Engine, Event, Fault, PolicyError, Sink, Source};
use earkey_policy::identity::{Evidence, Group, LinuxKey};
use serde_json::Value;
use std::collections::VecDeque;
use std::io::{self, ErrorKind};
use std::time::{Duration, Instant};

const EVIDENCE: Evidence = Evidence::XiaomiPixelos20261107;
const STALL: Duration = Duration::from_millis(100);
const CENTER: u16 = 164;
const UP: u16 = 115;
const DOWN: u16 = 114;

#[derive(Default)]
struct Input {
    events: VecDeque<Event>,
    reads: VecDeque<Result<usize, ErrorKind>>,
    pressed: Vec<LinuxKey>,
    query_error: Option<ErrorKind>,
    queries: usize,
}

impl Input {
    fn with_events(events: impl IntoIterator<Item = Event>) -> Self {
        Self {
            events: events.into_iter().collect(),
            ..Self::default()
        }
    }
}

impl Source for Input {
    fn read_events(&mut self, buffer: &mut [Event]) -> io::Result<usize> {
        let count = self.reads.pop_front().unwrap_or_else(|| {
            if self.events.is_empty() {
                Err(ErrorKind::WouldBlock)
            } else {
                Ok(buffer.len().min(self.events.len()))
            }
        })?;
        for (slot, event) in buffer
            .iter_mut()
            .zip(self.events.drain(..count.min(self.events.len())))
        {
            *slot = event;
        }
        Ok(count)
    }

    fn query_pressed(&mut self, buffer: &mut [LinuxKey]) -> io::Result<usize> {
        self.queries += 1;
        if let Some(error) = self.query_error {
            return Err(error.into());
        }
        buffer[..self.pressed.len()].copy_from_slice(&self.pressed);
        Ok(self.pressed.len())
    }
}

#[derive(Default)]
struct Output {
    events: Vec<Event>,
    writes: VecDeque<Result<usize, ErrorKind>>,
}

impl Sink for Output {
    fn write_events(&mut self, events: &[Event]) -> io::Result<usize> {
        let count = self.writes.pop_front().unwrap_or(Ok(events.len()))?;
        if let Some(prefix) = events.get(..count) {
            self.events.extend_from_slice(prefix);
        }
        Ok(count)
    }
}

fn key(code: u16, value: i32) -> Event {
    Event {
        timestamp_us: i64::from(value),
        kind: 1,
        code,
        value,
    }
}

fn scan(value: i32) -> Event {
    Event {
        timestamp_us: 93,
        kind: 4,
        code: 4,
        value,
    }
}

fn sync(value: i32) -> Event {
    Event {
        timestamp_us: 94,
        kind: 0,
        code: 0,
        value,
    }
}

fn engine(groups: &[Group]) -> Engine {
    Engine::new(EVIDENCE, 1, groups, STALL).unwrap()
}

fn drain(
    engine: &mut Engine,
    source: &mut Input,
    sink: &mut Output,
    now: Instant,
) -> Result<Activity, Fault> {
    for _ in 0..16_384 {
        match engine.step(source, sink, now)? {
            Activity::Progress | Activity::Retry => {}
            waiting => return Ok(waiting),
        }
    }
    panic!("session failed to reach a wait or terminal state");
}

#[test]
fn measured_frames_replay_unchanged_in_passthrough() {
    let directory =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/discovery");
    let mut checked_records = 0;
    for entry in std::fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let fixture: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        for record in fixture["records"].as_array().unwrap() {
            let expected = fixture_events(&record["events"]);
            assert!(!expected.is_empty());
            let mut session = engine(&[]);
            let mut input = Input::with_events(expected.iter().copied());
            let mut output = Output::default();
            assert_eq!(
                drain(&mut session, &mut input, &mut output, Instant::now()),
                Ok(Activity::WaitInput),
                "{path:?}"
            );
            assert_eq!(output.events, expected, "{path:?}");
            assert!(
                session.possibly_forwarded_keys().next().is_none(),
                "{path:?}"
            );
            checked_records += 1;
        }
    }
    assert!(checked_records > 0);
}

#[test]
fn synthetic_frames_replay_through_the_drop_boundary() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/protocol/synthetic-event-summaries.v1.json"
    ))
    .unwrap();
    for case in fixture["cases"].as_array().unwrap() {
        let events = fixture_events(&case["events"]);
        let mut session = engine(&[]);
        let mut input = Input::with_events(events.iter().copied());
        let mut output = Output::default();
        let result = drain(&mut session, &mut input, &mut output, Instant::now());
        if let Some(drop) = events
            .iter()
            .position(|event| (event.kind, event.code) == (0, 3))
        {
            assert_eq!(
                result,
                Err(Fault::SyncLost {
                    physical_keys_down: 0
                })
            );
            let boundary = events[..drop]
                .iter()
                .rposition(|event| (event.kind, event.code) == (0, 0));
            let complete = boundary.map_or(0, |index| index + 1);
            assert_eq!(output.events, events[..complete]);
            assert_eq!(input.queries, 1);
        } else {
            assert_eq!(result, Ok(Activity::WaitInput), "{}", case["id"]);
            assert_eq!(output.events, events, "{}", case["id"]);
        }
    }
}

fn fixture_events(rows: &Value) -> Vec<Event> {
    rows.as_array()
        .unwrap()
        .iter()
        .map(|row| {
            let hex = |index: usize| u32::from_str_radix(row[index].as_str().unwrap(), 16).unwrap();
            Event {
                timestamp_us: row[0].as_i64().unwrap(),
                kind: hex(1).try_into().unwrap(),
                code: hex(2).try_into().unwrap(),
                value: hex(3) as i32,
            }
        })
        .collect()
}

#[test]
fn mixed_frames_remove_only_blocked_scans_and_key_sequences() {
    let mut session = engine(&[Group::Center]);
    let mut source = Input::with_events([
        scan(0xc00cd),
        key(CENTER, 1),
        scan(0xc00e9),
        key(UP, 1),
        sync(0),
        scan(0xc00cd),
        key(CENTER, 2),
        key(UP, 2),
        sync(0),
        key(UP, 0),
        key(CENTER, 0),
        sync(1),
        key(CENTER, 1),
        sync(0),
        key(CENTER, 0),
        sync(0),
        scan(0xc00e2),
        key(113, 1),
        sync(0),
        key(113, 0),
        sync(0),
    ]);
    let mut sink = Output::default();
    assert_eq!(
        drain(&mut session, &mut source, &mut sink, Instant::now()),
        Ok(Activity::WaitInput)
    );
    assert_eq!(
        sink.events,
        [
            scan(0xc00e9),
            key(UP, 1),
            sync(0),
            key(UP, 2),
            sync(0),
            key(UP, 0),
            sync(1),
            scan(0xc00e2),
            key(113, 1),
            sync(0),
            key(113, 0),
            sync(0),
        ]
    );
    assert!(session.possibly_forwarded_keys().next().is_none());
}

#[test]
fn policy_updates_preserve_frame_and_held_key_decisions() {
    let mut session = engine(&[Group::VolumeDown]);
    let mut input = Input::with_events([scan(0xc00cd), key(CENTER, 1), key(DOWN, 1), sync(0)]);
    let mut output = Output::default();
    let now = Instant::now();
    assert_eq!(
        session.step(&mut input, &mut output, now),
        Ok(Activity::Progress)
    );
    assert_eq!(session.select_policy(2, &[Group::Center]), Ok(()));
    assert_eq!(
        session.select_policy(1, &[]),
        Err(PolicyError::StaleGeneration)
    );
    assert_eq!(
        drain(&mut session, &mut input, &mut output, now),
        Ok(Activity::WaitInput)
    );
    assert_eq!(
        session.possibly_forwarded_keys().collect::<Vec<_>>(),
        [LinuxKey(CENTER)]
    );

    input.events.extend([
        key(DOWN, 2),
        key(CENTER, 2),
        sync(0),
        key(DOWN, 0),
        key(CENTER, 0),
        sync(1),
        key(CENTER, 1),
        key(DOWN, 1),
        sync(0),
        key(CENTER, 0),
        key(DOWN, 0),
        sync(0),
    ]);
    assert_eq!(
        drain(&mut session, &mut input, &mut output, now),
        Ok(Activity::WaitInput)
    );
    assert_eq!(
        output.events,
        [
            scan(0xc00cd),
            key(CENTER, 1),
            sync(0),
            key(CENTER, 2),
            sync(0),
            key(CENTER, 0),
            sync(1),
            key(DOWN, 1),
            sync(0),
            key(DOWN, 0),
            sync(0),
        ]
    );
}

#[test]
fn interrupted_short_io_and_backpressure_do_not_duplicate_output() {
    let original = [
        scan(0xc00cd),
        key(CENTER, 1),
        sync(0),
        key(CENTER, 0),
        sync(1),
    ];
    let mut session = engine(&[]);
    let mut input = Input::with_events(original);
    input.reads.extend([
        Err(ErrorKind::Interrupted),
        Ok(1),
        Err(ErrorKind::WouldBlock),
        Ok(1),
        Ok(3),
    ]);
    let mut output = Output {
        writes: [
            Ok(1),
            Err(ErrorKind::Interrupted),
            Err(ErrorKind::WouldBlock),
            Ok(1),
        ]
        .into(),
        ..Output::default()
    };
    let now = Instant::now();

    assert_eq!(
        drain(&mut session, &mut input, &mut output, now),
        Ok(Activity::WaitInput)
    );
    assert!(output.events.is_empty());
    assert_eq!(
        drain(&mut session, &mut input, &mut output, now),
        Ok(Activity::WaitOutput)
    );
    assert_eq!(output.events, [scan(0xc00cd)]);
    assert_eq!(
        drain(&mut session, &mut input, &mut output, now),
        Ok(Activity::WaitInput)
    );
    assert_eq!(output.events, original);
}

#[test]
fn output_timeout_retains_cleanup_state_and_stops_permanently() {
    let mut session = engine(&[]);
    let mut input = Input::with_events([key(CENTER, 1), sync(0), key(CENTER, 0), sync(0)]);
    let mut output = Output {
        writes: [Ok(1), Err(ErrorKind::WouldBlock)].into(),
        ..Output::default()
    };
    let now = Instant::now();
    assert_eq!(
        drain(&mut session, &mut input, &mut output, now),
        Ok(Activity::WaitOutput)
    );
    assert_eq!(output.events, [key(CENTER, 1)]);
    assert_eq!(session.deadline(), Some(now + STALL));
    assert_eq!(
        session.possibly_forwarded_keys().collect::<Vec<_>>(),
        [LinuxKey(CENTER)]
    );
    assert_eq!(
        drain(&mut session, &mut input, &mut output, now + STALL),
        Err(Fault::OutputDeadline)
    );
    assert_eq!(
        session.step(&mut input, &mut output, now + STALL),
        Err(Fault::OutputDeadline)
    );
    assert_eq!(output.events, [key(CENTER, 1)]);
}

#[test]
fn fatal_write_and_disconnect_stop_without_replaying() {
    let now = Instant::now();
    let mut session = engine(&[]);
    let mut input = Input::with_events([key(CENTER, 1), sync(0), key(CENTER, 0), sync(0)]);
    let mut output = Output {
        writes: [Ok(2), Ok(1), Err(ErrorKind::BrokenPipe)].into(),
        ..Output::default()
    };
    assert_eq!(
        drain(&mut session, &mut input, &mut output, now),
        Err(Fault::Write(ErrorKind::BrokenPipe))
    );
    assert_eq!(
        session.possibly_forwarded_keys().collect::<Vec<_>>(),
        [LinuxKey(CENTER)]
    );
    assert_eq!(output.events, [key(CENTER, 1), sync(0), key(CENTER, 0)]);

    let mut session = engine(&[]);
    let mut input = Input {
        reads: [Ok(2), Ok(0)].into(),
        ..Input::with_events([key(UP, 1), sync(0)])
    };
    let mut output = Output::default();
    assert_eq!(
        drain(&mut session, &mut input, &mut output, now),
        Err(Fault::Disconnected)
    );
    assert_eq!(
        session.possibly_forwarded_keys().collect::<Vec<_>>(),
        [LinuxKey(UP)]
    );
}

#[test]
fn malformed_frame_never_writes_a_valid_prefix() {
    for frame in [
        vec![key(CENTER, 1), key(CENTER, 1), sync(0)],
        vec![key(CENTER, 1), key(UP, 2), sync(0)],
        vec![key(CENTER, 1), key(UP, 0), sync(0)],
        vec![key(CENTER, 1), key(UP, 3), sync(0)],
        vec![key(CENTER, 1), key(116, 1), sync(0)],
        vec![key(CENTER, 1), scan(0xc00e9), sync(0)],
        vec![key(CENTER, 1), scan(0xc00e9), scan(0xc00ea), sync(0)],
        vec![key(CENTER, 1), scan(0xc00ea), key(UP, 1), sync(0)],
        vec![
            key(CENTER, 1),
            Event {
                kind: 5,
                ..Event::default()
            },
            sync(0),
        ],
    ] {
        let mut session = engine(&[]);
        let mut input = Input::with_events(frame);
        let mut output = Output::default();
        assert_eq!(
            drain(&mut session, &mut input, &mut output, Instant::now()),
            Err(Fault::MalformedFrame)
        );
        assert!(output.events.is_empty());
        assert!(session.possibly_forwarded_keys().next().is_none());
    }
}

#[test]
fn overlong_or_stalled_frames_and_interrupted_io_are_bounded() {
    let now = Instant::now();
    let mut session = engine(&[]);
    let mut input = Input::with_events(std::iter::repeat_n([key(UP, 1), key(UP, 0)], 32).flatten());
    let mut output = Output::default();
    assert_eq!(
        drain(&mut session, &mut input, &mut output, now),
        Err(Fault::FrameTooLong)
    );
    assert!(output.events.is_empty());

    let mut session = engine(&[]);
    let mut input = Input::with_events([key(UP, 1)]);
    assert_eq!(
        drain(&mut session, &mut input, &mut output, now),
        Ok(Activity::WaitInput)
    );
    assert_eq!(
        drain(&mut session, &mut input, &mut output, now + STALL),
        Err(Fault::FrameDeadline)
    );

    let mut session = engine(&[]);
    let mut input = Input {
        reads: [Err(ErrorKind::Interrupted); 8].into(),
        ..Input::default()
    };
    assert_eq!(
        drain(&mut session, &mut input, &mut output, now),
        Err(Fault::InterruptedTooOften)
    );
}

#[test]
fn dropped_events_are_discarded_until_report_then_snapshot_for_cleanup() {
    let now = Instant::now();
    let mut session = engine(&[]);
    let mut input = Input::with_events([
        key(CENTER, 1),
        sync(0),
        key(UP, 1),
        Event {
            kind: 0,
            code: 3,
            ..Event::default()
        },
        key(CENTER, 0),
        key(116, 1),
    ]);
    input.pressed = vec![LinuxKey(UP)];
    let mut output = Output::default();
    assert_eq!(
        drain(&mut session, &mut input, &mut output, now),
        Ok(Activity::WaitInput)
    );
    assert_eq!(input.queries, 0);
    input.events.extend([sync(1), key(UP, 0), sync(0)]);
    assert_eq!(
        drain(&mut session, &mut input, &mut output, now),
        Err(Fault::SyncLost {
            physical_keys_down: 1
        })
    );
    assert_eq!(input.queries, 1);
    assert_eq!(output.events, [key(CENTER, 1), sync(0)]);
    assert_eq!(
        session.possibly_forwarded_keys().collect::<Vec<_>>(),
        [LinuxKey(CENTER)]
    );
}

#[test]
fn driver_contract_violations_and_failed_state_queries_are_terminal() {
    let now = Instant::now();
    let mut session = engine(&[]);
    let mut input = Input {
        reads: [Ok(65)].into(),
        ..Input::default()
    };
    let mut output = Output::default();
    assert_eq!(
        session.step(&mut input, &mut output, now),
        Err(Fault::InvalidReadCount)
    );

    let mut session = engine(&[]);
    let mut input = Input::with_events([key(UP, 1), sync(0)]);
    output.writes.push_back(Ok(3));
    assert_eq!(
        drain(&mut session, &mut input, &mut output, now),
        Err(Fault::InvalidWriteCount)
    );

    let mut session = engine(&[]);
    let mut input = Input::with_events([
        Event {
            kind: 0,
            code: 3,
            ..Event::default()
        },
        sync(0),
    ]);
    input.query_error = Some(ErrorKind::PermissionDenied);
    assert_eq!(
        drain(&mut session, &mut input, &mut output, now),
        Err(Fault::StateQuery(ErrorKind::PermissionDenied))
    );
    assert!(output.events.is_empty());
}
