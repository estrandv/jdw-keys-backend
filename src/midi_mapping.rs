use std::collections::HashMap;
use std::ops::Range;
use std::slice::range;
use crate::keyboard_model::{AbsPad, Key, MIDIEvent};


#[derive(Clone)]
enum IntMatch {
    Abs(u8),
    Range(Range<u8>),
    Array([u8]),
    Any
}

fn matches(event: &[u8], structure: &[IntMatch]) -> bool {

    if event.len() != structure.len() {
        return false;
    }

    let mut result = true;
    for i in 0..(structure.len() - 1) {

        let content = event[i];

        let expected = structure[i].clone();

        match expected {
            IntMatch::Abs(value) => {
                if value != content {
                    result = false;
                }
            }
            IntMatch::Range(range) => {
                if !range.contains(content.into()) {
                    result = false;
                }
            },
            IntMatch::Array(range) => {
                if !range.contains(content.into()) {
                    result = false;
                }
            }
            IntMatch::Any => {}
        };
    }

    result
}

pub fn map(event: &[u8]) -> Option<MIDIEvent> {

    let mut result: Option<MIDIEvent> = None;

    if matches(event, &[
        IntMatch::Array(*[144u8, 128u8]),
        IntMatch::Any,
        IntMatch::Any
    ]) {
        result = Some(MIDIEvent::Key(Key {
            pressed: true,
            midi_note: 0u8,
            force: 0u8
        }));
    }
    else if matches(event, &[
        IntMatch::Abs(176u8),
        IntMatch::Range(22u8..29u8),
        IntMatch::Array(*[0u8, 127u8])
    ]) {
        result = Some(MIDIEvent::AbsPad(AbsPad {
            id: event[1],
            pressed: event[2] == 127u8,
        }));
    }
    else if matches(event, &[
        IntMatch::Abs(176u8),
        IntMatch::Array(*[
            74u8, 71u8, 76u8, 77u8, 93u8, 73u8, 75u8,
            18u8, 19u8, 16u8, 17u8, 91u8, 79u8, 72u8
        ]),
        IntMatch::Any
    ]) {
        // AbsKnob
    }
    else if matches(event, &[
        IntMatch::Abs(176u8),
        IntMatch::Array(*[
            74u8, 71u8, 76u8, 77u8, 93u8, 73u8, 75u8,
            18u8, 19u8, 16u8, 17u8, 91u8, 79u8, 72u8,
        ]),
        IntMatch::Any
    ]) {
        // AbsKnob
    }
    else if matches(event, &[
        IntMatch::Abs(176u8),
        IntMatch::Array(*[
            112u8,
            114u8,
        ]),
        IntMatch::Array(*[
            64u8, // "bonus", maybe ignore
            66u8, 67u8, // "upward"
            61u8, 62u8, // "downward"
        ])
    ]) {
        // RelKnob
    }
    else if matches(event, &[
        IntMatch::Abs(176u8),
        IntMatch::Array(*[
            112u8,
            114u8,
        ]),
        IntMatch::Array(*[
            127u8, 0u8 // Press, release
        ])
    ]) {
        // Knob press
    }
    else if matches(event, &[
        IntMatch::Abs(224u8),
        IntMatch::Array(*[
            0u8,
            127u8,
        ]),
        IntMatch::Any
    ]) {
        // Left slider
    }
    else if matches(event, &[
        IntMatch::Abs(176u8),
        IntMatch::Abs(1u8),
        IntMatch::Any
    ]) {
        // Right slider
    }

    result

}