use std::fmt::format;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use bigdecimal::{BigDecimal, Zero};
use rosc::OscType;

use crate::event_model::{Event, NoteOff, NoteOn};
use crate::util;
use crate::util::duration_to_beats;

pub fn stringify_history(
    history: Arc<Mutex<EventHistory>>,
    bpm: i64,
    quantization: BigDecimal,
    args: Vec<OscType>,
) -> String {
    let sequence = history.lock().unwrap().as_sequence(bpm, quantization.clone());

    let total_beats = sequence.iter()
        .map(|event| event.reserved_beats.clone())
        .reduce(|a, b| a + b)
        .unwrap_or(BigDecimal::zero());

    let desired_total = util::round_up_to_nearest(
        total_beats.clone(), BigDecimal::from_str("4.0").unwrap(),
    );

    let difference = desired_total.clone() - total_beats;

    let arg_string = util::shuttlefiy_args(args);

    let notes = sequence.iter()
        .map(|seq| {

            let mut base = format!("{}:{}", seq.representation, seq.reserved_beats);

            if let Some(sus) = &seq.sustain_beats {
                base += format!(",sus{:.4}", sus).as_str();
            }

            base

        })
        .collect::<Vec<String>>().join(" ");

    // Add a silence at the end until we reach the next 4-beat
    let diff_note = format!("_:{:.4}", difference);

    format!("({} {}):{},len{}", notes, diff_note, arg_string, desired_total)

}

pub struct SequentialEvent {
    representation: String,
    reserved_beats: BigDecimal,
    sustain_beats: Option<BigDecimal>,
}

pub struct EventHistory {
    events: Vec<Event>,
}

impl EventHistory {
    pub fn new() -> EventHistory {
        EventHistory { events: Vec::new() }
    }

    pub fn add(&mut self, event: Event) {
        self.events.push(event);
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }

    /*
        Find the first following NoteOff, matching the given NoteOn,
            and infer the time passed between them.
    */
    pub fn get_sustain_dur(&self, event: &NoteOn) -> Option<Duration> {
        let mut off_match: Option<&NoteOff> = None;
        let mut self_found = false;

        // TODO: Could much more efficiently lookup a starting point of the loop...
        for iter_event in &self.events {
            if !self_found {
                if let Event::NoteOn(note_on) = iter_event {
                    if note_on == event {
                        self_found = true;
                    }
                }
            } else {
                if let Event::NoteOff(note_off) = iter_event {
                    if event.id == note_off.id {
                        off_match = Some(&note_off);
                        break;
                    }
                }
            }
        }

        off_match.map(|note_off| note_off.time.duration_since(event.time))
    }

    // Returns a sequential representation of the events in history as a shuttle-notation string.
    pub fn as_sequence(&self, bpm: i64, quantization: BigDecimal) -> Vec<SequentialEvent> {
        let mut next_note_time: Option<Instant> = None;

        let notes: Vec<SequentialEvent> = self
            .events
            .iter()
            .rev() // Iter backwards to always have the next event time available
            .filter_map(|event| {
                match event {
                    Event::NoteOn(note_on) => {
                        let time = next_note_time
                            .map(|next| next.duration_since(note_on.time))
                            .unwrap_or(Duration::ZERO);

                        next_note_time = Some(note_on.time.clone());

                        let sustain_beats: Option<BigDecimal> = self
                            .get_sustain_dur(note_on)
                            .map(|dur| util::round_to_nearest(
                                duration_to_beats(dur, bpm),
                                quantization.clone(),
                            ));

                        let time_beats = util::round_to_nearest(
                            duration_to_beats(time, bpm),
                            quantization.clone(),
                        );

                        Some(SequentialEvent {
                            representation: note_on.id.to_string(),
                            reserved_beats: time_beats,
                            sustain_beats,
                        })
                    }
                    Event::Silence(silence) => {
                        let time = next_note_time
                            .map(|next| next.duration_since(silence.time.clone()))
                            .unwrap_or(Duration::ZERO);

                        next_note_time = Some(silence.time);

                        let time_beats = util::round_to_nearest(
                            duration_to_beats(time, bpm),
                            quantization.clone(),
                        );

                        Some(SequentialEvent {
                            representation: "_".to_string(),
                            reserved_beats: time_beats,
                            sustain_beats: None,
                        })
                    }
                    _ => None
                }
            })
            .rev()
            .collect();

        notes
    }
}
