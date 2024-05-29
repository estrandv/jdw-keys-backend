use std::time::Instant;

/*
   Rethinking History

   - When a press is registered, two things happen:
       1. The relevant OSC message is constructed and sent
           - This resolves from current settings for the detected key
       2. An entry is created in history and history converted to a shuttle string
           - Again, current settings are needed
   - What does history need to know?
       a. Time played
       b. String representation of the note played
           - Either separate or combined with an id that knows:
       c. How to pair events by key, in case of e.g. release
   - Since some cases (e.g. samples) don't really pair the button itself
       to the representation (e.g. multiple buttons playing "0"), a button
       id is probably fair.

   - Example data:
       On(id: "key_1", rep: "c4", args: {"sus": 0.2}, time: 120412...)
       On(id: "pad_1", rep: "0", ...)

   - id, rep and args can of course also exist in a similar setup in some
       configurable object, so that they can be taken straight from there
       when a note is pressed. That place would also produce the OSC message.

*/

#[derive(PartialEq)]
pub enum Event {
    NoteOn(NoteOn),
    NoteOff(NoteOff),
}

#[derive(PartialEq)]
pub struct NoteOn {
    pub id: String,
    pub time: Instant,
}

#[derive(PartialEq)]
pub struct NoteOff {
    pub id: String,
    pub time: Instant,
}
