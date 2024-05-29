mod keyboard_model;
mod midi_mapping;
mod event_history;
mod event_model;

use std::error::Error;
use std::io::{stdin, stdout, Write};

use midir::{Ignore, MidiInput};
use crate::keyboard_model::MIDIEvent;

fn main() {
    match run() {
        Ok(_) => (),
        Err(err) => println!("Error: {}", err),
    }
}

/*
    TODO: This is a bit of a second "input lab"
    - Should be the keys-and-history backend for both
    - A separate repo should handle keypress reading for Arturia, but this
        right here is the lab right now

        ROAD AHEAD
        - Any input source can have a keyset, really
            - <port_name, mapping_function> and then just iterate
            - So we should build it a bit like this just to keep it clean
        - Begin reimplementation of keyboard structs
            - I think history will have to use its own structs
                - This because it should be fit for purpose; "knob up" isn't
                    really relevant to other front ends.
                - "note_on, note_off, control_change, button_press" etc.
                    -> Better!
                    -> This can then have different interpretations depending on mode,
                        so the original data is important (key index, etc)

 */

fn run() -> Result<(), Box<dyn Error>> {
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

                    println!("{:?}", event);

                    match event {
                        MIDIEvent::Key(key) => {
                            // ...
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
                    }
                }
            }


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
