#![feature(int_roundings)]
#![allow(internal_features)]

extern crate core;

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
use crate::midi_mapping::map;
use crate::osc_client::OscClient;
use crate::state::State;

use itertools::Itertools;

mod event_history;
mod event_model;
mod keyboard_model;
mod midi_mapping;
mod midi_translation;
mod osc_model;
mod util;

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
    // State init

    let state = State::new();
    let midi_read_state = Arc::new(Mutex::new(state));
    let osc_read_state = midi_read_state.clone();
    let hist_daemon_state = midi_read_state.clone();

    let history = EventHistory::new();
    let osc_read_history = Arc::new(Mutex::new(history));
    let midi_read_history = osc_read_history.clone();
    let hist_daemon_history = osc_read_history.clone();

    // TODO: modular in/out ports
    let socket = UdpSocket::bind(SocketAddrV4::from_str("127.0.0.1:15459").unwrap()).unwrap();

    socket.set_nonblocking(true).unwrap();
    socket
        .set_write_timeout(Some(Duration::from_millis(1)))
        .unwrap();
    socket
        .set_read_timeout(Some(Duration::from_millis(1)))
        .unwrap();

    let mut client = OscClient::new(
        socket,
        // 13339 is router, 13331 is sc - testing if direct to sc is more efficient
        SocketAddrV4::from_str("127.0.0.1:13339").unwrap(),
    );

    // History stringify thread
    thread::spawn(move || {
        loop {
            let modified = hist_daemon_history.lock().unwrap().modified.clone();

            if modified {
                hist_daemon_history.lock().unwrap().modified = false;

                let bpm = hist_daemon_state.lock().unwrap().bpm.clone();
                let quantization = hist_daemon_state.lock().unwrap().quantization.clone();
                let args = hist_daemon_state.lock().unwrap().message_args.clone();

                let sequence = hist_daemon_history
                    .lock()
                    .unwrap()
                    .as_sequence(bpm, quantization.clone());

                let stringified = event_history::stringify_history(sequence, args);

                // Copy to clipboard
                let opts = Options::new();
                opts.copy(
                    Source::Bytes(stringified.clone().into_bytes().into()),
                    MimeType::Autodetect,
                )
                .unwrap();

                println!("Had bpm {}, Copied to clipboard! {}", bpm, stringified);
            }

            sleep(Duration::from_millis(50));
        }
    });

    // OSC Read Thread
    thread::spawn(move || {
        // TODO: Same as regular keyboard address, atm
        OSCStack::init("127.0.0.1:17777".to_string())
            .on_message("/set_bpm", &|msg| {
                let bpm_arg = msg
                    .args
                    .get(0)
                    .unwrap()
                    .clone()
                    .int()
                    .unwrap()
                    .to_i64()
                    .unwrap();
                osc_read_state.lock().unwrap().set_bpm(bpm_arg)
            })
            .on_message("/keyboard_quantization", &|msg| {
                let quantization = msg.args.get(0).unwrap().clone().string().unwrap();
                osc_read_state
                    .lock()
                    .unwrap()
                    .set_quantization(&quantization);
            })
            .on_message("/keyboard_args", &|msg| {
                osc_read_state.lock().unwrap().set_args(msg.args.clone());
            })
            .on_message("/keyboard_pad_samples", &|msg| {
                // Iterate osc args in pairs
                for w in msg.args.chunks(2) {
                    let pad_id = w[0].clone().int().unwrap() as u8;
                    let sample_index = w[1].clone().int().unwrap();

                    osc_read_state
                        .lock()
                        .unwrap()
                        .pads_configuration
                        .pads
                        .insert(pad_id, sample_index)
                        .unwrap();
                }
            })
            .on_message("/keyboard_pad_pack", &|msg| {
                let name = msg.args.get(0).cloned().unwrap().string().unwrap();
                println!("CHANGING SAMPLER TO {}", name);

                osc_read_state.lock().unwrap().pads_configuration.pack_name = name;
            })
            .on_message("/keyboard_pad_args", &|msg| {
                osc_read_state.lock().unwrap().pads_configuration.args = msg.args.clone();
            })
            .on_message("/keyboard_instrument_name", &|msg| {
                let name = msg.args.get(0).unwrap().clone().string().unwrap();
                println!("CHANGING KEYBOARD TO {}", name);
                osc_read_state.lock().unwrap().instrument_name = name;
            })
            .on_message("/loop_started", &|msg| {
                // TODO: Long story short, this is the delay to expect as opposed to human-played notes
                // UPDATE: Added delay compensation to human player, not sure how relevant this is now
                let first_beat_plays_at = Instant::now() + Duration::from_millis(70);
                osc_read_history
                    .lock()
                    .unwrap()
                    .add(Event::Silence(Silence {
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

    let arturia_port = midi_in
        .ports()
        .into_iter()
        .find(|port| midi_in.port_name(port).unwrap().contains(arturia_id))
        .expect("No Arturia MiniLab Keyboard found!");

    println!("\nOpening connection");
    let in_port_name = midi_in.port_name(&arturia_port)?;

    let mut last_played_pad: Option<u8> = None;

    // _conn_in needs to be a named parameter, because it needs to be kept alive until the end of the scope
    let _conn_in = midi_in.connect(
        &arturia_port,
        "midir-read-input",
        move |stamp, message, _| {
            // TODO: Clumsy latency compensation here to offset the fact that small delays
            //  can bias quantization towards rounding upwards.
            let read_time = Instant::now(); // - Duration::from_millis(15);

            let mut times: Vec<(Instant, &str)> = vec![(read_time, "read time")];

            if let Some(event) = map(message) {
                times.push((Instant::now(), "midi resolved and matched"));

                let state_lock = midi_read_state.lock().unwrap();
                let instrument = state_lock.instrument_name.clone();
                let args = state_lock.message_args.clone();
                drop(state_lock);

                times.push((Instant::now(), "locks acquired"));

                match event {
                    MIDIEvent::Key(key) => {
                        times.push((Instant::now(), "midi type matched"));

                        // E.g. "a4"
                        let history_id = midi_translation::tone_to_oletter(key.midi_note);

                        if key.pressed {
                            times.push((Instant::now(), "tone translated"));

                            let msg = osc_model::create_note_on(
                                key.midi_note as i32,
                                instrument.as_str(),
                                args,
                            );

                            times.push((Instant::now(), "note message created"));

                            client.send(msg);

                            times.push((Instant::now(), "send done"));

                            midi_read_history.lock().unwrap().add(Event::NoteOn(NoteOn {
                                id: history_id,
                                time: read_time,
                            }));
                        } else {
                            let msg = osc_model::create_note_off(key.midi_note as i32);

                            client.send(msg);

                            times.push((Instant::now(), "send done"));

                            midi_read_history
                                .lock()
                                .unwrap()
                                .add(Event::NoteOff(NoteOff {
                                    id: history_id,
                                    time: read_time,
                                }));
                        }
                    }
                    MIDIEvent::AbsPad(pad) => {
                        if pad.pressed {
                            let state_read = midi_read_state.lock().unwrap();

                            let sample_index = state_read
                                .pads_configuration
                                .pads
                                .get(&pad.id)
                                .unwrap()
                                .clone();

                            let sample_pack = state_read.pads_configuration.pack_name.clone();

                            let pad_args = state_read.pads_configuration.args.clone();

                            let msg =
                                osc_model::create_play_sample(sample_index, &sample_pack, pad_args);

                            client.send(msg);

                            times.push((Instant::now(), "send done"));

                            midi_read_history.lock().unwrap().add(Event::NoteOn(NoteOn {
                                id: sample_index.to_string(),
                                time: read_time,
                            }));

                            last_played_pad = Some(pad.id);

                            drop(state_read); // Should not be necessary but I've had issues
                        }
                    }
                    MIDIEvent::AbsKnob(knob) => {
                        // TODO: Range must be state-configurable
                        let value = util::midi_to_float(0.0..2.0, knob.value);
                        let msg = osc_model::create_control_bus_mod(knob.id as i32, value);
                        client.send(msg);
                        println!("{:?}, {}", knob, value);
                    }
                    MIDIEvent::KnobButton(button) => {
                        println!("Pressed a knob");

                        if let Some(pad) = last_played_pad {
                            if button.pressed {
                                // TODO: 113 is top, 115 is lower
                                let modifier = if button.id == 115u8 { -1 } else { 1 };

                                let mut state = midi_read_state.lock().unwrap();

                                let existing_value =
                                    state.pads_configuration.pads.get(&pad).unwrap().clone();

                                let new_value = (existing_value + modifier).max(0);

                                state.pads_configuration.pads.insert(pad, new_value);

                                // Play the new configuration for easy browsing
                                let sample_pack = state.pads_configuration.pack_name.clone();

                                let msg =
                                    osc_model::create_play_sample(new_value, &sample_pack, args);

                                client.send(msg);
                            }
                        }

                        //println!("KNOB PRESS! {:?}", button);
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

            // Benching
            /*            let mut previous = None;
            let mut total: u128 = 0;
            for tuple in times {
                match previous {
                    Some(time) => {
                        let ms = tuple.0.duration_since(time).as_micros();
                        total += ms;
                        println!("{}:{}ms", tuple.1, ms);
                    }
                    None => {
                        println!("{}:0ms", tuple.1);
                    }
                }
                previous = Some(tuple.0);
            }
            println!("Total: {}ms", total);*/

            // Use for key detection when adding new keys
            println!("{}: {:?} (len = {})", stamp, message, message.len());
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
