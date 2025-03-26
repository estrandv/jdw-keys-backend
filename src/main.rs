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
use std::time::{Duration, Instant, SystemTime};

use bigdecimal::ToPrimitive;
use jdw_osc_lib::osc_stack::OSCStack;
use midir::{Ignore, MidiInput};
use ncurses_daemon::{KeyboardModeState, NcursesDaemon};
use ringbuf::traits::{Consumer, Producer, Split};
use ringbuf::HeapRb;
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

mod midi_read_daemon;
mod ncurses_daemon;
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
    /*

        HEAPRB & STRUCTURING FOR REUSE WITH KEYBOARD
        - Sequencer has a working model for oscStack, where we combine an arc with the publish end of the heap
            -> So there can be a small lock delay for incoming, but incoming is just configuration anyway
        - Anything read and published by OSC should be identical, so the OSCStack can be moved into its own daemon
            that behaves the same in both applications (publishing a standard message)
        - Similarly, the MIDIEvent struct (as a result of midi_mapping functions done on midi read loop) should be
            a published part as the end product of another daemon
            -> It is up to the keyboard translator to treat it as the correct event (e.g. key or abspad)


        => Update
            - midi event pipe was a success. Easiest way forward is a separate keyboard daemon that makes simple midi events
                without any initial configuration


    */

    // NOTE: I have no idea what an appropriate capacity is
    let midi_pipe = HeapRb::<MIDIEvent>::new(100);
    let (mut midi_pub, mut midi_sub) = midi_pipe.split();

    let keycontrol_pipe = HeapRb::<KeyboardModeState>::new(100);
    let (mut keycontrol_pub, mut keycontrol_sub) = keycontrol_pipe.split();

    let oscd_keycontrol = Arc::new(Mutex::new(keycontrol_pub));

    let (mut history_event_out, mut history_event_in) = HeapRb::<Event>::new(100).split();

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
            while let Some(event) = history_event_in.try_pop() {
                hist_daemon_history.lock().unwrap().add(event);

                let bpm = hist_daemon_state.lock().unwrap().bpm.clone();
                let quantization = hist_daemon_state.lock().unwrap().quantization.clone();
                let args = hist_daemon_state.lock().unwrap().message_args.clone();

                let sequence = hist_daemon_history
                    .lock()
                    .unwrap()
                    .as_sequence(bpm, quantization.clone());

                let ends_on_sample = hist_daemon_history.lock().unwrap().ends_on_sample();

                let stringified = event_history::stringify_history(sequence, ends_on_sample);

                // Copy to clipboard
                let opts = Options::new();
                opts.copy(
                    Source::Bytes(stringified.clone().into_bytes().into()),
                    MimeType::Autodetect,
                )
                .unwrap();

                println!("Had bpm {}, Copied to clipboard! {}", bpm, stringified);
            }

            sleep(Duration::from_millis(200));
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
            .on_message(
                "/keyboard_octave",
                &|msg| match msg.args.get(0).unwrap().clone().int() {
                    Some(octave) => oscd_keycontrol
                        .lock()
                        .unwrap()
                        .try_push(KeyboardModeState {
                            octave: (octave - 1) as u8,
                        })
                        .unwrap_or_else(|_| {}),
                    None => println!("Could not find any octave number in octave message"),
                },
            )
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
                // UPDATE: Deprecatd in favour of jdw_sc_event, remove after testing with that
                let first_beat_plays_at = Instant::now() + Duration::from_millis(100);
                osc_read_history
                    .lock()
                    .unwrap()
                    .add(Event::Silence(Silence {
                        time: first_beat_plays_at,
                    }));
                println!("Loop start registered");
            })
            .on_message("/jdw_sc_event", &|msg| {
                let event_name = msg.args.get(0).unwrap().clone().string().unwrap();
                let timestamp = msg.args.get(1).unwrap().clone().time().unwrap();
                let time = SystemTime::try_from(timestamp).unwrap();
                // Yes, this is the only way to do it, because instant can only exist as a relative entity
                let ins = Instant::now() + time.duration_since(SystemTime::now()).unwrap();

                println!("EVENT: {}", event_name);

                if (event_name == "loop_started".to_string()) {
                    osc_read_history.lock().unwrap().register_beatbreak(ins);
                }
            })
            .begin();
    });

    // Start reading MIDI

    thread::spawn(move || {
        let mut last_played_pad: Option<u8> = None;

        loop {
            // TODO: Find a sweetspot between lag and cpu usage
            std::thread::sleep(Duration::from_nanos(500000));

            let read_time = Instant::now(); // - Duration::from_millis(15);

            while let Some(event) = midi_sub.try_pop() {
                // TODO: This and history locking is prob what slows things down
                /*

                    - INCOMING: state: instrument(str), args(vec<osctype>) (FROM: OSC READ)
                    - OUTGOING: History events (TO: History daemon)
                    - Also note: The thread.sleep call might be too high if things lag
                    - History clearing is a separate scenario that does not work with an event pusher

                */
                let state_lock = midi_read_state.lock().unwrap();
                let instrument = state_lock.instrument_name.clone();
                let args = state_lock.message_args.clone();
                drop(state_lock);

                match event {
                    MIDIEvent::Key(key) => {
                        // E.g. "a4"
                        let history_id = midi_translation::tone_to_oletter(key.midi_note);

                        if key.pressed {
                            let msg = osc_model::create_note_on(
                                key.midi_note as i32,
                                instrument.as_str(),
                                args,
                            );

                            println!("SENDING KEYPRESS {} {}", key.midi_note, instrument);

                            client.send(msg);

                            history_event_out
                                .try_push(Event::NoteOn(NoteOn {
                                    id: history_id,
                                    time: read_time,
                                    is_sample: false,
                                }))
                                .unwrap();
                        } else {
                            let msg = osc_model::create_note_off(key.midi_note as i32);

                            client.send(msg);

                            history_event_out
                                .try_push(Event::NoteOff(NoteOff {
                                    id: history_id,
                                    time: read_time,
                                }))
                                .unwrap();
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

                            history_event_out
                                .try_push(Event::NoteOn(NoteOn {
                                    id: sample_index.to_string(),
                                    time: read_time,
                                    is_sample: true,
                                }))
                                .unwrap();

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
                                // Note duplicate logic in ncurses daemon
                                let modifier = if button.id == 115u8 { -1 } else { 1 };

                                let mut state = midi_read_state.lock().unwrap();

                                let existing_value =
                                    state.pads_configuration.pads.get(&pad).unwrap().clone();

                                let new_value = (existing_value + modifier).max(0);

                                state.pads_configuration.pads.insert(pad, new_value);

                                // Play the new configuration for easy browsing
                                let sample_pack = state.pads_configuration.pack_name.clone();

                                let pad_args = state.pads_configuration.args.clone();

                                let msg = osc_model::create_play_sample(
                                    new_value,
                                    &sample_pack,
                                    pad_args,
                                );

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
        }
    });

    //midi_read_daemon::begin(midi_pub)
    NcursesDaemon::new(midi_pub, keycontrol_sub)
        .begin()
        .unwrap();
    // TODO: Effectively no error handling whatsoever - should be streamlined
    Ok(())
}
