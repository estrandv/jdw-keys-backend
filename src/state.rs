use std::collections::HashMap;
use std::str::FromStr;
use std::time::Instant;
use bigdecimal::BigDecimal;
use rosc::OscType;

use crate::config::Config;

/*
    Configurable, shared state variables.
*/

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum KeyboardMode {
    Keyboard,
    Sampler,
}

pub struct PadsConfiguration {
    pub pads: HashMap<u8, i32>, // <pad_id, sample_index>
    pub pack_name: String,
    pub args: Vec<OscType>,
}

impl PadsConfiguration {
    fn new() -> PadsConfiguration {
        Self::new_with_pack("EMU_EDrum")
    }

    fn new_with_pack(pack_name: &str) -> PadsConfiguration {
        let base_map: HashMap<_, _> = (0u8..128u8).into_iter()
            .map(|value| (value, value as i32))
            .collect();

        PadsConfiguration {
            pads: base_map,
            pack_name: pack_name.to_string(),
            args: Vec::new()
        }
    }
}

pub struct State {
    pub bpm: i64,
    pub quantization: BigDecimal,
    pub available_instruments: Vec<String>,
    pub available_packs: Vec<String>,
    pub message_args: Vec<OscType>,
    pub instrument_name: String,
    pub last_loop_start_time: Option<Instant>,
    pub pads_configuration: PadsConfiguration,
    pub keyboard_mode: KeyboardMode,
    pub record_history: bool,
    pub quantize_enabled: bool,
    pub multiline_output: bool,
    pub history_preview: String,
}

impl State {

    pub fn new() -> State {
        let cfg = Config::get();
        let mode = match cfg.initial_mode.to_lowercase().as_str() {
            "sampler" => KeyboardMode::Sampler,
            _ => KeyboardMode::Keyboard,
        };
        let mut msg_args = Vec::new();
        for arg in &cfg.message_args {
            if let Ok(f) = f64::from_str(arg) {
                msg_args.push(OscType::Float(f as f32));
            } else {
                msg_args.push(OscType::String(arg.to_string()));
            }
        }

        State {
            bpm: cfg.bpm,
            quantization: BigDecimal::from_str(&cfg.quantization).unwrap(),
            message_args: msg_args,
            instrument_name: cfg.instrument_name.clone(),
            available_instruments: cfg.available_instruments.clone(),
            available_packs: cfg.available_packs.clone(),
            last_loop_start_time: None,
            pads_configuration: PadsConfiguration::new_with_pack(&cfg.default_pack),
            keyboard_mode: mode,
            record_history: cfg.record_history,
            quantize_enabled: cfg.quantize_enabled,
            multiline_output: cfg.multiline_output,
            history_preview: String::new(),
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