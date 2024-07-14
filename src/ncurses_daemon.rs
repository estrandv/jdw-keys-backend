use std::collections::HashMap;
use std::sync::{Arc};
use std::time::{Instant};

use notcurses::*;
use ringbuf::storage::Heap;
use ringbuf::traits::Producer;
use ringbuf::wrap::caching::Caching;
use ringbuf::SharedRb;

use crate::keyboard_model::{AbsPad, KnobButton, MIDIEvent, ShiftButton};
use crate::keyboard_model::Key as KbKey; 

const KEYBOARD_KEYS: [char; 17] = [
    'q',
    '2',
    'w',
    '3',
    'e',
    'r',
    '5',
    't',
    '6',
    'y',
    '7',
    'u',
    'i',
    '9',
    'o',
    '0',
    'p',
];

const MOD_KEYS: [char; 2] = [
    '+', '-'
];

pub struct KeyboardModeState {
    synth: bool,
    octave: u8
}

pub fn begin(
    mut publisher: Caching<Arc<SharedRb<Heap<MIDIEvent>>>, true, false>
) -> NotcursesResult<()> {

    // Init sensible default configuration

    let mut tone_map: HashMap<char, i32> = HashMap::new();

    for i in 0..KEYBOARD_KEYS.len() {
        tone_map.insert(KEYBOARD_KEYS[i], i as i32 + 1);
    }

    let mut state = KeyboardModeState {
        synth: true,
        octave: 5,
    };

    // Init ncurses

    let mut nc = Notcurses::new()?;
    nc.mice_enable(MiceEvents::All)?;

    let mut plane = Plane::new(&mut nc)?;
    plane.set_scrolling(true);


    putstrln!(+render plane,
        "\n{0}\nStarting non-blocking event loop. Press `F01` to exit:\n{}\n",
        "-".repeat(50)
    )?;

    // Begin loop 

    let mut ctrl_pressed = false;

    loop {

        // TODO: Read any incoming osc state mods in this loop as well 

        let event = nc.poll_event()?;

        if event.received() {
            //putstrln![+render plane, "\n{event:?}"]?;

            for char_key in KEYBOARD_KEYS {

                // Register key release
                if event.is_char(char_key) && event.is_release() && state.synth {

                    let midi_note_raw = KEYBOARD_KEYS.iter().position(|&e| e == char_key).unwrap() as u8;
                    let midi_note = (state.octave * 12u8) + midi_note_raw; 

                    let event = MIDIEvent::Key(KbKey {
                        pressed: false,
                        midi_note: midi_note,
                        force: 127,
                    });

                    publisher.try_push(event).unwrap();
                }

                // Register key press
                if event.is_char(char_key) && event.is_press() {

                    if state.synth {
                        let midi_note_raw = KEYBOARD_KEYS.iter().position(|&e| e == char_key).unwrap() as u8;
                        let midi_note = (state.octave * 12u8) + midi_note_raw; 
    
                        // TODO: Sampler mode should fetch modded key index from state and publish abspad
                        let event = MIDIEvent::Key(KbKey {
                            pressed: true,
                            midi_note: midi_note,
                            force: 127,
                        });
    
                        publisher.try_push(event).unwrap();
                    } else {
                        // Sampler

                        let pad_index =  KEYBOARD_KEYS.iter().position(|&e| e == char_key).unwrap() as u8 + 1;

                        let event = MIDIEvent::AbsPad(AbsPad {
                            id: pad_index,
                            pressed: true,
                        });

                        publisher.try_push(event).unwrap();


                    }

                }

            }

            for mod_key in MOD_KEYS {
                if event.is_char(mod_key) && event.is_press() {

                    // TODO: 113 is top, 115 is lower
                    // Fix together with the midi todo
                    let emulated_knob_id = if event.is_char('+') { 113 } else { 115 };

                    let event = MIDIEvent::KnobButton(KnobButton {
                        id: emulated_knob_id,
                        pressed: true,
                    });

                    publisher.try_push(event).unwrap();

                }
            }

            // Clear history on enter
            if event.is_key(Key::Enter) {

                let event = MIDIEvent::ShiftButton(ShiftButton {
                    pressed: true,
                });

                publisher.try_push(event).unwrap();

            }

            if event.is_key(Key::LCtrl) {
                if event.is_press() {
                    ctrl_pressed = true;
                } else if event.is_release() {
                    ctrl_pressed = false;
                }
            }

            if event.is_key(Key::F01) {
                break;
            }
        }
    }



    Ok(())

}