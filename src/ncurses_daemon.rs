use std::collections::HashMap;
use std::sync::{Arc};
use std::time::{Instant};

use notcurses::*;
use ringbuf::storage::Heap;
use ringbuf::traits::Producer;
use ringbuf::wrap::caching::Caching;
use ringbuf::SharedRb;

use crate::keyboard_model::{MIDIEvent};
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
    octave: u8,
    pub letter_numbers: HashMap<char, i32> 
}

impl KeyboardModeState {
    pub fn modify(&mut self, key: char, amount: i32) {
        let mut existing = self.letter_numbers.get(&key).unwrap().clone();
        existing += amount;
        self.letter_numbers.insert(key, existing);
    }
}

pub fn begin(
    mut state: KeyboardModeState,
    mut publisher: Caching<Arc<SharedRb<Heap<MIDIEvent>>>, true, false>
) -> NotcursesResult<()> {
    let mut nc = Notcurses::new()?;
    nc.mice_enable(MiceEvents::All)?;

    let mut plane = Plane::new(&mut nc)?;
    plane.set_scrolling(true);


    putstrln!(+render plane,
        "\n{0}\nStarting non-blocking event loop. Press `F01` to exit:\n{}\n",
        "-".repeat(50)
    )?;


    let mut last_key = None;

    let mut ctrl_pressed = false;

    loop {

        // TODO: Read any incoming osc state mods in this loop as well 


        let event = nc.poll_event()?;

        if event.received() {
            //putstrln![+render plane, "\n{event:?}"]?;

            let press_time = Instant::now();

            for char_key in KEYBOARD_KEYS {

                // Register key release
                if event.is_char(char_key) && event.is_release() && state.synth {

                    let midi_note_raw = KEYBOARD_KEYS.iter().position(|&e| e == char_key).unwrap() as u8;
                    // TODO: Messy hardcode for the second octave keys
                    let extra_octave = if "i9o0p".contains(char_key) { 1u8 } else { 0u8 };
                    let midi_note = ((state.octave + extra_octave) * 12u8) + midi_note_raw; 

                    let event = MIDIEvent::Key(KbKey {
                        pressed: false,
                        midi_note: midi_note,
                        force: 127,
                    });

                    publisher.try_push(event).unwrap();
                }

                // Register key press
                if event.is_char(char_key) && event.is_press() {

                    let midi_note_raw = KEYBOARD_KEYS.iter().position(|&e| e == char_key).unwrap() as u8;
                    // TODO: Messy hardcode for the second octave keys
                    let extra_octave = if "i9o0p".contains(char_key) { 1u8 } else { 0u8 };
                    let midi_note = ((state.octave + extra_octave) * 12u8) + midi_note_raw; 

                    // TODO: Sampler mode should fetch modded key index from state and publish abspad
                    let event = MIDIEvent::Key(KbKey {
                        pressed: false,
                        midi_note: midi_note,
                        force: 127,
                    });

                    publisher.try_push(event).unwrap();


                    last_key = Some(char_key);
                }

            }

            for mod_key in MOD_KEYS {
                if event.is_char(mod_key) && event.is_press() && last_key.is_some() {


                    if let Some(lk) = last_key {
                        let amount = if event.is_char('-') {-1} else {1};
                        state.modify(lk, amount);
                    }

                    // TODO: Call modify on last played letter in state and publish an abspad press if sample mode 

                }
            }

            // Clear history on enter
            if event.is_key(Key::Enter) {
                // TODO 
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