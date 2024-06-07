use std::error::Error;

use midir::{Ignore, MidiInput};

use crate::keyboard_model::MIDIEvent;
use crate::midi_mapping;

pub fn scan<F: 'static + Fn(MIDIEvent) + Send>(
    callback: F
) -> Result<(), Box<dyn Error>> {

    let mut midi_in = MidiInput::new("midir reading input")?;
    midi_in.ignore(Ignore::None);

    let arturia_id = "Arturia MiniLab mkII";

    let arturia_port = midi_in.ports().into_iter()
        .find(|port| midi_in.port_name(port).unwrap().contains(arturia_id))
        .expect("No Arturia MiniLab Keyboard found!");

    println!("\nOpening connection");

    // _conn_in needs to be a named parameter, because it needs to be kept alive until the end of the scope
    let _conn_in = midi_in.connect(
        &arturia_port,
        "midir-read-input",
        move |stamp, message, _| {

            let decode = midi_mapping::map(message);

            match decode {
                None => {}
                Some(event) => {
                    callback(event);
                }
            }

            //println!("{}: {:?} (len = {})", stamp, message, message.len());
        },
        (),
    )?;

    Ok(())

}