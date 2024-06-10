use std::ops::Range;
use crate::keyboard_model::{AbsKnob, AbsPad, Key, KnobButton, MIDIEvent, ShiftButton};


#[derive(Clone)]
enum IntMatch {
    Abs(u8),
    Range(Range<u8>),
    Array(&'static [u8]),
    Any
}

/*
    Allows for easy matching of u8 arrays by specifying
        a fixed-size array of dynamic matching conditions.
 */
fn matches(event: &[u8], structure: &[IntMatch]) -> bool {

    if event.len() != structure.len() {
        return false;
    }

    let fail = structure.iter().enumerate()
        .any(|tuple| {

            let expected = tuple.1;

            match expected {
                IntMatch::Abs(value) => {
                    let content = event[tuple.0];
                    *value != content
                }
                IntMatch::Range(range) => {
                    let content = event[tuple.0];
                    !range.contains(&content)
                },
                IntMatch::Array(range) => {
                    let content = event[tuple.0];
                    !range.contains(&content)
                }
                _ => {
                    false
                }
            }
        });

    !fail
}

// Literal matches for "Arturia MINILAB MK2"
pub fn map(event: &[u8]) -> Option<MIDIEvent> {

    let mut result: Option<MIDIEvent> = None;

    if matches(event, &[
        IntMatch::Array(&[144u8, 128u8]),
        IntMatch::Any,
        IntMatch::Any
    ]) {
        result = Some(MIDIEvent::Key(Key {
            pressed: event[0] == 144u8,
            midi_note: event[1],
            force: event[2]
        }));
    }
    else if matches(event, &[
        IntMatch::Abs(176u8),
        IntMatch::Range(22u8..29u8),
        IntMatch::Array(&[0u8, 127u8])
    ]) {
        // TODO: Id as actual id on board?
        result = Some(MIDIEvent::AbsPad(AbsPad {
            id: event[1],
            pressed: event[2] == 127u8,
        }));
    }
    else if matches(event, &[
        IntMatch::Abs(176u8),
        IntMatch::Array(&[
            74u8, 71u8, 76u8, 77u8, 93u8, 73u8, 75u8,
            18u8, 19u8, 16u8, 17u8, 91u8, 79u8, 72u8
        ]),
        IntMatch::Any
    ]) {
        // TODO: Id as actual id on board?
        result = Some(MIDIEvent::AbsKnob(AbsKnob { id: event[1], value: event[2] }))
    }
    else if matches(event, &[
        IntMatch::Abs(176u8),
        IntMatch::Array(&[
            112u8,
            114u8,
        ]),
        IntMatch::Array(&[
            64u8, // "bonus", maybe ignore
            66u8, 67u8, // "upward"
            61u8, 62u8, // "downward"
        ])
    ]) {
        // RelKnob
    }
    else if matches(event, &[
        IntMatch::Abs(176u8),
        IntMatch::Array(&[
            113u8,
            115u8,
        ]),
        IntMatch::Array(&[
            127u8, 0u8 // Press, release
        ])
    ]) {
        result = Some(MIDIEvent::KnobButton(KnobButton {
            id: event[1],
            pressed: event[2] == 127u8,
        }));
    }
    else if matches(event, &[
        IntMatch::Abs(224u8),
        IntMatch::Array(&[
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
    } else if matches(event, &[

        // [240, 0, 32, 107, 127, 66, 2, 0, 0, 46, 127, 247]
        IntMatch::Abs(240u8),
        IntMatch::Abs(0u8),
        IntMatch::Abs(32u8),
        IntMatch::Abs(107u8),
        IntMatch::Abs(127u8),
        IntMatch::Abs(66u8),
        IntMatch::Abs(2u8),
        IntMatch::Abs(0u8),
        IntMatch::Abs(0u8),
        IntMatch::Abs(46u8),
        IntMatch::Array(&[127u8, 0u8]),
        IntMatch::Abs(247u8),
    ]) {
        // SHIFT button
        result = Some(MIDIEvent::ShiftButton(ShiftButton {
            pressed: event[10] == 127u8,
        }));
    }

    result

}