mod keyboard_model;
mod midi_mapping;
mod event_history;
mod event_model;
mod util;
mod midi_translation;
mod osc_model;
mod state;

use std::error::Error;
use std::io::{stdin, stdout, Write};
use std::sync::{Arc, Mutex};
use std::{clone, string, thread};
use std::time::{Duration, Instant};
use bigdecimal::ToPrimitive;
use jdw_osc_lib::osc_stack::OSCStack;

use midir::{Ignore, MidiInput};
use crate::event_history::EventHistory;
use crate::event_model::{Event, NoteOff, NoteOn, Silence};
use crate::keyboard_model::MIDIEvent;
use crate::state::State;

fn main() {
    match run() {
        Ok(_) => (),
        Err(err) => println!("Error: {}", err),
    }
}

/*


    TODO: STATUS
    - We have a full history struct, with an included stringify

    x* Make a key-to-octaved-letter translation function, for history ids and
        message sends
    * Implement message sending logic for notes and samples
        -> Functions done, need a client
        -> All in place, need to implement
    * Implement history writing
    * Implement osc settings reading from old application
    * Create a daemon that reacts to changes in history by performing stringify
        and writing it to clipboard (later osc)
    * ...

 */

fn run() -> Result<(), Box<dyn Error>> {

    /// State init

    let state = State::new();
    let midi_read_state = Arc::new(Mutex::new(state));
    let osc_read_state = midi_read_state.clone();

    // TODO: History should have an "is_updated" variable and a separate poller thread in order
    //  to perform stringify
    let history = EventHistory::new();
    let osc_read_history = Arc::new(Mutex::new(history));
    let midi_read_history = osc_read_history.clone();

    ///

    // OSC Read Thread
    thread::spawn(move || {
        // TODO: Same as regular keyboard address, atm
        OSCStack::init("127.0.0.1:17777".to_string())
            .on_message("/set_bpm", &|msg| {
                let bpm_arg = msg.args.get(0).unwrap().clone().int().unwrap().to_i64().unwrap();
                osc_read_state.lock().unwrap().set_bpm(bpm_arg)
            })
            .on_message("/keyboard_quantization", &|msg| {
                let quantization = msg.args.get(0).unwrap().clone().string().unwrap();
                osc_read_state.lock().unwrap().set_quantization(&quantization);
            })
            .on_message("/keyboard_args", &|msg| {
                osc_read_state.lock().unwrap().set_args(msg.args.clone());
            })
            .on_message("/keyboard_letter_index", &|msg| {
                // TODO: Pads
            })
            .on_message("/keyboard_instrument_name", &|msg| {
                osc_read_state.lock().unwrap().instrument_name = msg.args.get(0).unwrap().clone().string().unwrap();
            })
            .on_message("/loop_started", &|msg| {

                // TODO: Long story short, this is the delay to expect as opposed to human-played notes
                let first_beat_plays_at = Instant::now() + Duration::from_millis(200);
                osc_read_history.lock().unwrap().add(Event::Silence(Silence {
                    time: first_beat_plays_at,
                }));
            })
            .begin();
    });

    // Start reading MIDI

    let mut input = String::new();

    let mut midi_in = MidiInput::new("midir reading input")?;
    midi_in.ignore(Ignore::None);

    let arturia_id = "Arturia MiniLab mkII";

    let arturia_port = midi_in.ports().into_iter()
        .find(|port| midi_in.port_name(port).unwrap().contains(arturia_id))
        .expect("No Arturia MiniLab Keyboard found!");

    println!("\nOpening connection");
    let in_port_name = midi_in.port_name(&arturia_port)?;

    // _conn_in needs to be a named parameter, because it needs to be kept alive until the end of the scope
    let _conn_in = midi_in.connect(
        &arturia_port,
        "midir-read-input",
        move |stamp, message, _| {

            let decode = midi_mapping::map(message);

            match decode {
                None => {}
                Some(event) => {

                    let read_time = Instant::now();

                    println!("{:?}", event);

                    match event {
                        MIDIEvent::Key(key) => {

                            // E.g. "a4"
                            let history_id = midi_translation::tone_to_oletter(key.midi_note);

                            let instrument = midi_read_state.lock().unwrap().instrument_name.clone();

                            let args = midi_read_state.lock().unwrap().message_args.clone();

                            if key.pressed {

                                // TODO: WIP Message POC
                                let msg = osc_model::create_note_on(
                                    key.midi_note as i32,
                                    instrument.as_str(),
                                    args
                                );

                                midi_read_history.lock().unwrap().add(Event::NoteOn(NoteOn {
                                    id: history_id,
                                    time: read_time,
                                }));
                            } else {
                                midi_read_history.lock().unwrap().add(Event::NoteOff(NoteOff {
                                    id: history_id,
                                    time: read_time,
                                }));
                            }
                        }
                        MIDIEvent::AbsPad(pad) => {
                            println!("PAD!");
                        }
                        MIDIEvent::AbsKnob(knob) => {
                            println!("KNOB!");
                        }
                        MIDIEvent::KnobButton(button) => {
                            println!("KNOB PRESS!");
                        }
                        _ => {}
                    }
                }
            }

            //println!("{}: {:?} (len = {})", stamp, message, message.len());
        },
        (),
    )?;

    println!(
        "Connection open, reading input from '{}' (press enter to exit) ...",
        in_port_name
    );

    input.clear();
    stdin().read_line(&mut input)?; // wait for next enter key press

    println!("Closing connection");
    Ok(())
}
