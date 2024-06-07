use bigdecimal::ToPrimitive;
use rosc::{OscMessage, OscPacket, OscType};

pub fn create_play_sample(index: i32, pack_name: &str, args: Vec<OscType>) -> OscPacket {

    let mut base_args = vec![
        OscType::String("letter_sample".to_string()), // ext_id
        OscType::String(pack_name.to_string()),
        OscType::Int(index),
        OscType::String("".to_string()),
        OscType::Int(0),
    ];

    for arg in args {
        base_args.push(arg);
    }

    OscPacket::Message(OscMessage {
        addr: "/play_sample".to_string(),
        args: base_args,
    })
}

pub fn create_note_on(index: i32, synth_name: &str, args: Vec<OscType>) -> OscPacket {

    let external_id = "letter_note_".to_string() + index.to_string().as_str();

    let freq = psg::math::midi_pitch_to_frequency(index.to_f64().unwrap());

    let mut base_args = vec![
        OscType::String(synth_name.to_string()),
        OscType::String(external_id),
        OscType::Int(0),
        OscType::String("freq".to_string()), // NOTE: should be modular
        OscType::Float(freq as f32),
    ];

    for arg in args {
        base_args.push(arg);
    }

    OscPacket::Message(OscMessage {
        addr: "/note_on".to_string(),
        args: base_args,
    })

}

pub fn create_note_off(index: i32) -> OscPacket {
    OscPacket::Message(OscMessage {
        addr: "/note_modify".to_string(),
        args: vec![
            OscType::String("letter_note_".to_string() + index.to_string().as_str()),
            OscType::Int(0),
            OscType::String("gate".to_string()),
            OscType::Float(0.0)
        ],
    })
}