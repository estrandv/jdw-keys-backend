use std::time::Instant;

#[derive(PartialEq, Debug)]
pub enum Event {
    NoteOn(NoteOn),
    NoteOff(NoteOff),
    Silence(Silence),
    BeatBreak(BeatBreak),
}

#[derive(PartialEq, Debug)]
pub struct BeatBreak {
    pub time: Instant,
}

#[derive(PartialEq, Debug)]
pub struct NoteOn {
    pub id: String,
    pub time: Instant,
    pub is_sample: bool,
}

#[derive(PartialEq, Debug)]
pub struct NoteOff {
    pub id: String,
    pub time: Instant,
}

#[derive(PartialEq, Debug)]
pub struct Silence {
    pub time: Instant,
}
