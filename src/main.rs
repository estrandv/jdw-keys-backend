extern crate core;

use std::env::args;
use std::error::Error;
use std::io::{stdin, Write};
use std::net::{SocketAddrV4, UdpSocket};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::sleep;
use std::time::{Duration, Instant};

use bigdecimal::ToPrimitive;
use jdw_osc_lib::osc_stack::OSCStack;
use midir::{Ignore, MidiInput};
use wl_clipboard_rs::copy::{MimeType, Options, Source};

use crate::event_history::EventHistory;
use crate::event_model::{Event, NoteOff, NoteOn, Silence};
use crate::keyboard_model::MIDIEvent;
use crate::osc_client::OscClient;
use crate::state::State;

mod keyboard_model;
mod midi_mapping;
mod event_history;
mod event_model;
mod util;
mod midi_translation;
mod osc_model;

mod osc_client;
mod state;

fn main() {
    match run() {
        Ok(_) => (),
        Err(err) => println!("Error: {}", err),
    }
}

/*


    TODO: STATUS

    Planned features:
    - Sample shifting for pads
    - Shift-key history wipe
    - OSC-driven sample shifting
        - Requires that pads have a clear ID (ideally as written on board)
        - Requires new message (old configuration used letter-to-index)
    - ncurses notes display, like in old keyboard
        - A clear step towards front end
    - backend separation
        - Requires API definition
        - Do as late as possible
 */

fn run() -> Result<(), Box<dyn Error>> {

    /// State init

    let state = State::new();
    let midi_read_state = Arc::new(Mutex::new(state));
    let osc_read_state = midi_read_state.clone();
    let hist_daemon_state = midi_read_state.clone();

    let history = EventHistory::new();
    let osc_read_history = Arc::new(Mutex::new(history));
    let midi_read_history = osc_read_history.clone();
    let hist_daemon_history = osc_read_history.clone();


    // TODO: modular in/out ports
    let socket = UdpSocket::bind(
        SocketAddrV4::from_str("127.0.0.1:15459").unwrap()
    ).unwrap();

    let client = OscClient::new(
        socket,
        SocketAddrV4::from_str("127.0.0.1:13339").unwrap()
    );
    let midi_read_client = Arc::new(Mutex::new(client));

    ///

    // History stringify thread
    thread::spawn(move || {

        loop {
            let modified = hist_daemon_history.lock().unwrap().modified.clone();

            if modified {
                hist_daemon_history.lock().unwrap().modified = false;
                let stringified = event_history::stringify_history(
                    hist_daemon_history.clone(),
                    hist_daemon_state.clone()
                );

                // Copy to clipboard
                let opts = Options::new();
                opts.copy(Source::Bytes(stringified.clone().into_bytes().into()), MimeType::Autodetect).unwrap();

                println!("Copied to clipboard! {}", stringified);
            }

            sleep(Duration::from_millis(100));
        }
    });


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
                // TODO: Needs new message format designed for pads - letters are out
                //  This is front end code, too
                let letter = msg.args.get(0).unwrap().clone().string().unwrap().chars().nth(0).unwrap();
                let index = msg.args.get(1).unwrap().clone().int().unwrap();
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

                    let instrument = midi_read_state.lock().unwrap().instrument_name.clone();

                    let args = midi_read_state.lock().unwrap().message_args.clone();

                    match event {
                        MIDIEvent::Key(key) => {

                            // E.g. "a4"
                            let history_id = midi_translation::tone_to_oletter(key.midi_note);


                            if key.pressed {

                                let msg = osc_model::create_note_on(
                                    key.midi_note as i32,
                                    instrument.as_str(),
                                    args
                                );

                                midi_read_client.lock().unwrap().send(msg);

                                midi_read_history.lock().unwrap().add(Event::NoteOn(NoteOn {
                                    id: history_id,
                                    time: read_time,
                                }));
                            } else {

                                let msg = osc_model::create_note_off(
                                    key.midi_note as i32,
                                );

                                midi_read_client.lock().unwrap().send(msg);

                                midi_read_history.lock().unwrap().add(Event::NoteOff(NoteOff {
                                    id: history_id,
                                    time: read_time,
                                }));
                            }
                        }
                        MIDIEvent::AbsPad(pad) => {

                            if pad.pressed {

                                let sample_index = midi_read_state.lock().unwrap()
                                    .pads_configuration.pads.get(&pad.id).unwrap().clone();

                                let sample_pack = midi_read_state.lock().unwrap()
                                    .pads_configuration.pack_name.clone();

                                let msg = osc_model::create_play_sample(
                                    sample_index,
                                    &sample_pack,
                                    args
                                );

                                midi_read_client.lock().unwrap().send(msg);

                                midi_read_history.lock().unwrap().add(Event::NoteOn(NoteOn {
                                    id: sample_index.to_string(),
                                    time: read_time,
                                }));
                            }


                        }
                        MIDIEvent::AbsKnob(knob) => {
                            println!("KNOB!");
                        }
                        MIDIEvent::KnobButton(button) => {
                            println!("KNOB PRESS!");
                        }
                        MIDIEvent::ShiftButton(button) => {

                            // Wipe!
                            if button.pressed {
                                midi_read_history.lock().unwrap().clear();
                            }
                        }
                        _ => {}
                    }
                }
            }

            // Use for key detection when adding new keys
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
