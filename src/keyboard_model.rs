
/*
    CHARTING:

    message structure:

    keys: [event, midi_note, power]
        - event down: 144, event up: 128
    pad: behaves like keys, but with different events
        - increase: 169
        - quick press?: 153
        - decrease: 137
        - Normally maps as notes 36-43
        - IF THE PAD9-16 BUTTON IS PRESSED:
            [176, 22-29id, 0/127 release/press]
    regular knob: [176, id, absolute value: 0-127]
    relative knob: [176, id, event type]
        - top id: 112
        - bottom id: 114
        - event types:
            - bonus, always on any: 64
            - upward: 66, 67
            - downward: 61, 62
            - press: 127, release: 0
    left slider:
        [224, 0/127 slide/max, location: 0-127]
    right slider:
        [176, 1, 0-127]

Mapping plan:
- identify incoming messages
- one route is structs
   - e.g. absKnob, relKnob, absPad, relPad, key, bend, control, etc.
   - Similar to the osc conversion stuff
- tricky bit is how to treat history
- Ideally, each event should just be stored right away, regardless of type
    - Difficult for release:
        - Stringify would have to look through the events to find a matching release
            - This isn't super hard when iterating backwards, and perhaps very clean
- Needs of the trait:
- stringify: each event must be able to represent itself as a history note
    - For some structs, this will be superfluous
- release detection:
    - Completely useless for all but the keys and pads
- Conclusion: we need an interim struct

- History structs:
- Only concerned with events that should be in the history string
    -> Note that <a history> of <any event> isn't necessarily a bad start
- What if we just stringify at history manager level?
    - For each <key>
        - Match type and return Some(string) if relevant
            - In cases of keys, a scan happens on the full set to also find the release time
- The main problem is type casting
    - I think an enum is proper, like OscType
    - So that we retain everything that way
- Conclusion: KEY ENUM should contain all events so that we get full type
    access after match. Perfect.

*/

#[derive(Debug)]
pub enum MIDIEvent {
    Key(Key),
    AbsPad(AbsPad),
    AbsKnob(AbsKnob),
    KnobButton(KnobButton)
}

#[derive(Debug)]
pub struct Key {
    pub pressed: bool,
    pub midi_note: u8,
    pub force: u8
}

#[derive(Debug)]
pub struct AbsPad {
    pub id: u8,
    pub pressed: bool
}

#[derive(Debug)]
pub struct AbsKnob {
    pub id: u8,
    pub value: u8
}

#[derive(Debug)]
pub struct KnobButton {
    pub id: u8,
    pub pressed: bool
}
