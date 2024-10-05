use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use notcurses::*;
use ringbuf::storage::Heap;
use ringbuf::traits::{Consumer, Producer};
use ringbuf::wrap::caching::Caching;
use ringbuf::SharedRb;

use crate::keyboard_model::Key as KbKey;
use crate::keyboard_model::{AbsPad, KnobButton, MIDIEvent, ShiftButton};

const KEYBOARD_KEYS: [char; 17] = [
    'q', '2', 'w', '3', 'e', 'r', '5', 't', '6', 'y', '7', 'u', 'i', '9', 'o', '0', 'p',
];

const PAD_KEYS: [char; 8] = ['a', 's', 'd', 'f', 'g', 'h', 'j', 'k'];

const MOD_KEYS: [char; 2] = ['+', '-'];

#[derive(Clone)]
pub struct KeyboardModeState {
    pub octave: u8,
}

pub struct NcursesDaemon {
    publisher: Caching<Arc<SharedRb<Heap<MIDIEvent>>>, true, false>,
    state_sub: Caching<Arc<SharedRb<Heap<KeyboardModeState>>>, false, true>,
}

impl NcursesDaemon {
    pub fn new(
        publisher: Caching<Arc<SharedRb<Heap<MIDIEvent>>>, true, false>,
        state_sub: Caching<Arc<SharedRb<Heap<KeyboardModeState>>>, false, true>,
    ) -> NcursesDaemon {
        NcursesDaemon {
            publisher,
            state_sub,
        }
    }

    pub fn begin(&mut self) -> NotcursesResult<()> {
        // Init sensible default configuration

        let mut curr_state = KeyboardModeState { octave: 5 };

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

        let mut shift_pressed = false;

        loop {
            // TODO: Find a sweetspot between lag and cpu usage
            std::thread::sleep(Duration::from_nanos(500000));
            let state = match self.state_sub.try_pop() {
                Some(val) => {
                    curr_state.octave = val.octave;
                    val
                }
                None => curr_state.clone(),
            };

            let event = nc.poll_event()?;

            if event.received() {
                //putstrln![+render plane, "\n{event:?}"]?;

                for pad_key in PAD_KEYS {
                    // Register key press
                    if event.is_char(pad_key) && event.is_press() {
                        // Sampler

                        let pad_id = PAD_KEYS.iter().position(|&e| e == pad_key).unwrap() as u8 + 1;

                        let event = MIDIEvent::AbsPad(AbsPad {
                            id: pad_id,
                            pressed: true,
                        });

                        self.publisher.try_push(event).unwrap();
                    }
                }

                for char_key in KEYBOARD_KEYS {
                    // Register key release
                    if event.is_char(char_key) && event.is_release() {
                        let midi_note_raw =
                            KEYBOARD_KEYS.iter().position(|&e| e == char_key).unwrap() as u8;
                        let midi_note = (state.octave * 12u8) + midi_note_raw;

                        let event = MIDIEvent::Key(KbKey {
                            pressed: false,
                            midi_note,
                            force: 127,
                        });

                        self.publisher.try_push(event).unwrap();
                    }

                    // Register key press
                    if event.is_char(char_key) && event.is_press() {
                        let midi_note_raw =
                            KEYBOARD_KEYS.iter().position(|&e| e == char_key).unwrap() as u8;
                        let midi_note = (state.octave * 12u8) + midi_note_raw;

                        // TODO: Sampler mode should fetch modded key index from state and publish abspad
                        let event = MIDIEvent::Key(KbKey {
                            pressed: true,
                            midi_note: midi_note,
                            force: 127,
                        });

                        self.publisher.try_push(event).unwrap();
                    }
                }

                for mod_key in MOD_KEYS {
                    if event.is_char(mod_key) && event.is_press() {
                        if shift_pressed {
                            if event.is_char('+') {
                                curr_state.octave += 1;
                            } else {
                                curr_state.octave -= 1;
                                if curr_state.octave <= 0 {
                                    curr_state.octave = 0;
                                }
                            }

                            println!("Keyboard octave changed to {}", curr_state.octave + 1);
                        } else {
                            // TODO: 113 is top, 115 is lower
                            // Fix together with the midi todo
                            let emulated_knob_id = if event.is_char('+') { 113 } else { 115 };

                            let event = MIDIEvent::KnobButton(KnobButton {
                                id: emulated_knob_id,
                                pressed: true,
                            });

                            self.publisher.try_push(event).unwrap();
                        }
                    }
                }

                // Clear history on enter
                if event.is_key(Key::Enter) {
                    let event = MIDIEvent::ShiftButton(ShiftButton { pressed: true });

                    self.publisher.try_push(event).unwrap();
                }

                if event.is_key(Key::LShift) {
                    if event.is_press() {
                        shift_pressed = true;
                    } else if event.is_release() {
                        shift_pressed = false;
                    }
                }

                if event.is_key(Key::F01) {
                    break;
                }
            }
        }

        println!("Exiting ncurses read...");

        Ok(())
    }
}
