use std::error::Error;
use std::io::stdin;
use std::sync::Arc;
use crate::keyboard_model::MIDIEvent;
use crate::midi_mapping::map;
use midir::{Ignore, MidiInput};
use ringbuf::storage::Heap;
use ringbuf::traits::Producer;
use ringbuf::wrap::caching::Caching;
use ringbuf::{HeapRb, SharedRb};

// Read MIDI from a predefined source and translate the input to an internal struct, published to the provided ringbuf
pub fn begin(
    mut publisher: Caching<Arc<SharedRb<Heap<MIDIEvent>>>, true, false>
) -> Result<(), Box<dyn Error>> {
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
    
        // _conn_in needs to be a named parameter, because it needs to be kept alive until the end of the scope
        let _conn_in = midi_in.connect(
            &arturia_port,
            "midir-read-input",
            move |stamp, message, _| {
    
                if let Some(event) = map(message) {

                    publisher.try_push(event).unwrap();
    
                }
    
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