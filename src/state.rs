use std::collections::HashMap;
use std::str::FromStr;
use std::time::Instant;
use bigdecimal::BigDecimal;
use rosc::OscType;

/*
    Configurable, shared state variables.
*/

pub struct PadsConfiguration {
    pub pads: HashMap<u8, i32>, // <pad_id, sample_index>
    pub pack_name: String,
    pub args: Vec<OscType>,
}

impl PadsConfiguration {
    fn new() -> PadsConfiguration {

        // Generate default values as identical to id with a generous amount of indices
        let base_map: HashMap<_, _> = (0u8..128u8).into_iter()
            .map(|value| (value, value as i32))
            .collect();

        PadsConfiguration {
            pads: base_map,
            pack_name: "EMU_EDrum".to_string(),
            args: Vec::new()
        }
    }
}

pub struct State {
    pub bpm: i64,
    pub quantization: BigDecimal,
    pub message_args: Vec<OscType>,
    pub instrument_name: String,
    pub last_loop_start_time: Option<Instant>,
    pub pads_configuration: PadsConfiguration,
}

impl State {

    pub fn new() -> State {
        State {
            bpm: 120,
            quantization: BigDecimal::from_str("0.125").unwrap(),
            message_args: vec![
                OscType::String("amp".to_string()),
                OscType::Float(0.2),
                OscType::String("relT".to_string()),
                OscType::Float(0.2),
                OscType::String("ofs".to_string()),
                OscType::Float(0.0),
            ],
            instrument_name: "aPad".to_string(),
            last_loop_start_time: None,
            pads_configuration: PadsConfiguration::new(),
        }
    }

    pub fn set_args(&mut self, args: Vec<OscType>) {
        self.message_args = args;
    }

    pub fn set_bpm(&mut self, value: i64) {
        self.bpm = value;
    }

    pub fn set_quantization(&mut self, number_string: &str) {
        self.quantization = BigDecimal::from_str(number_string).unwrap();
    }
}