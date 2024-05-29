use std::time::Duration;

use crate::event_model::{Event, NoteOff, NoteOn};

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
                match iter_event {
                    Event::NoteOff(note_off) if note_off.id == event.id => {
                        off_match = Some(&note_off);
                        break;
                    }
                    _ => {}
                }
            }
        }

        off_match.map(|note_off| note_off.time.duration_since(event.time))
    }
}
