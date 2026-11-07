#![no_main]

use earkey_engine::{Engine, Event, Sink, Source};
use earkey_policy::identity::{Evidence, Group, LinuxKey};
use libfuzzer_sys::fuzz_target;
use std::collections::HashSet;
use std::io;
use std::time::{Duration, Instant};

struct Input<'a>(std::slice::Iter<'a, Event>);

impl Source for Input<'_> {
    fn read_events(&mut self, buffer: &mut [Event]) -> io::Result<usize> {
        let count = buffer.len().min(self.0.len());
        for (slot, event) in buffer.iter_mut().zip(self.0.by_ref().take(count)) {
            *slot = *event;
        }
        Ok(count)
    }

    fn query_pressed(&mut self, _: &mut [LinuxKey]) -> io::Result<usize> {
        Ok(0)
    }
}

struct Output<'a> {
    input: std::slice::Iter<'a, Event>,
    pressed: HashSet<u16>,
    blocked: u8,
}

impl Sink for Output<'_> {
    fn write_events(&mut self, events: &[Event]) -> io::Result<usize> {
        let Some(report) = events.last() else {
            panic!("empty output frame");
        };
        assert_eq!((report.kind, report.code), (0, 0));
        for event in events {
            assert!(self.input.by_ref().any(|input| input == event));
            if event.kind != 1 {
                continue;
            }
            if let Some(index) = [164, 115, 114].iter().position(|&code| event.code == code) {
                assert_eq!(self.blocked & (1 << index), 0);
            }
            match event.value {
                0 => assert!(self.pressed.remove(&event.code)),
                1 => assert!(self.pressed.insert(event.code)),
                2 => assert!(self.pressed.contains(&event.code)),
                _ => panic!("invalid key value"),
            }
        }
        Ok(events.len())
    }
}

fuzz_target!(|bytes: &[u8]| {
    let Some((&mask, bytes)) = bytes.split_first() else {
        return;
    };
    let groups: Vec<_> = [Group::Center, Group::VolumeUp, Group::VolumeDown]
        .into_iter()
        .enumerate()
        .filter_map(|(index, group)| (mask & (1 << index) != 0).then_some(group))
        .collect();
    let events: Vec<_> = bytes
        .as_chunks::<8>()
        .0
        .iter()
        .enumerate()
        .map(|(index, bytes)| Event {
            timestamp_us: index as i64,
            kind: u16::from_le_bytes(bytes[..2].try_into().unwrap()),
            code: u16::from_le_bytes(bytes[2..4].try_into().unwrap()),
            value: i32::from_le_bytes(bytes[4..].try_into().unwrap()),
        })
        .collect();

    let mut input = Input(events.iter());
    let mut output = Output {
        input: events.iter(),
        pressed: HashSet::new(),
        blocked: mask,
    };
    let mut engine = Engine::new(
        Evidence::XiaomiPixelos20261107,
        1,
        &groups,
        Duration::from_millis(100),
    )
    .unwrap();
    let now = Instant::now();
    for _ in 0..=(events.len() * 2) {
        if engine.step(&mut input, &mut output, now).is_err() {
            return;
        }
    }
    panic!("finite input did not reach a terminal state");
});
