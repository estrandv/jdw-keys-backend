use serde::Deserialize;
use std::path::Path;
use std::sync::OnceLock;
use toml::Value as TomlValue;

static CONFIG: OnceLock<Config> = OnceLock::new();
static APP_NAME: &str = "keys";

#[derive(Debug, Deserialize)]
pub struct Config {
    pub router_host: String,
    pub router_port: u16,
    pub osc_listen_port: u16,
    pub local_bind_port: u16,
    pub instrument_name: String,
    pub bpm: i64,
    pub quantization: String,
    pub default_pack: String,
    pub message_args: Vec<String>,
    pub initial_mode: String,
    pub record_history: bool,
    pub quantize_enabled: bool,
    pub multiline_output: bool,
    pub initial_octave: u8,
    #[serde(default)]
    pub available_instruments: Vec<String>,
    #[serde(default)]
    pub available_packs: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            router_host: "127.0.0.1".to_string(),
            router_port: 13339,
            osc_listen_port: 17777,
            local_bind_port: 15459,
            instrument_name: "aPad".to_string(),
            bpm: 120,
            quantization: "0.125".to_string(),
            default_pack: "EMU_EDrum".to_string(),
            message_args: vec![
                "amp".to_string(),
                "0.2".to_string(),
                "relT".to_string(),
                "0.2".to_string(),
                "ofs".to_string(),
                "0.0".to_string(),
            ],
            initial_mode: "keyboard".to_string(),
            record_history: true,
            quantize_enabled: true,
            multiline_output: false,
            initial_octave: 5,
            available_packs: vec![
                "CR-78".into(),
                "EMU_SP12".into(),
                "Roland707Demo".into(),
                "Roland808".into(),
            ],
            available_instruments: vec![
                "degrade".into(),
                "dangerBass".into(),
                "trumpet".into(),
                "cheapPiano".into(),
                "eighties".into(),
                "FMRhodes".into(),
                "subBass".into(),
                "wobble".into(),
                "hypersaw".into(),
            ],
        }
    }
}

impl Config {
    pub fn get() -> &'static Config {
        CONFIG.get().expect("Config not initialized")
    }
}

fn central_config_path() -> Option<String> {
    if let Ok(path) = std::env::var("JDW_CONFIG") {
        if Path::new(&path).exists() {
            return Some(path);
        }
    }
    let home = std::env::var("HOME").ok()?;
    let xdg = Path::new(&home).join(".config").join("jdw.toml");
    if xdg.exists() {
        Some(xdg.to_string_lossy().to_string())
    } else {
        None
    }
}

fn load_central_section() -> Option<TomlValue> {
    let path = central_config_path()?;
    let contents = std::fs::read_to_string(&path).ok()?;
    let root: TomlValue = contents.parse().ok()?;
    root.get(APP_NAME).cloned()
}

fn merge_str(base: &mut String, overlay: &TomlValue, key: &str) {
    if let Some(v) = overlay.get(key).and_then(|v| v.as_str()) {
        *base = v.to_string();
    }
}

fn merge_u16(base: &mut u16, overlay: &TomlValue, key: &str) {
    if let Some(v) = overlay.get(key).and_then(|v| v.as_integer()) {
        *base = v as u16;
    }
}

fn merge_i64(base: &mut i64, overlay: &TomlValue, key: &str) {
    if let Some(v) = overlay.get(key).and_then(|v| v.as_integer()) {
        *base = v;
    }
}

fn merge_i64_into_u8(base: &mut u8, overlay: &TomlValue, key: &str) {
    if let Some(v) = overlay.get(key).and_then(|v| v.as_integer()) {
        *base = v as u8;
    }
}

fn merge_bool(base: &mut bool, overlay: &TomlValue, key: &str) {
    if let Some(v) = overlay.get(key).and_then(|v| v.as_bool()) {
        *base = v;
    }
}

fn merge_string_vec(base: &mut Vec<String>, overlay: &TomlValue, key: &str) {
    if let Some(v) = overlay.get(key).and_then(|v| v.as_array()) {
        *base = v.iter().filter_map(|val| val.as_str().map(String::from)).collect();
    }
}

fn merge_config(base: &mut Config, overlay: &TomlValue) {
    merge_str(&mut base.router_host, overlay, "router_host");
    merge_u16(&mut base.router_port, overlay, "router_port");
    merge_u16(&mut base.osc_listen_port, overlay, "osc_listen_port");
    merge_u16(&mut base.local_bind_port, overlay, "local_bind_port");
    merge_str(&mut base.instrument_name, overlay, "instrument_name");
    merge_i64(&mut base.bpm, overlay, "bpm");
    merge_str(&mut base.quantization, overlay, "quantization");
    merge_str(&mut base.default_pack, overlay, "default_pack");
    merge_string_vec(&mut base.message_args, overlay, "message_args");
    merge_str(&mut base.initial_mode, overlay, "initial_mode");
    merge_bool(&mut base.record_history, overlay, "record_history");
    merge_bool(&mut base.quantize_enabled, overlay, "quantize_enabled");
    merge_bool(&mut base.multiline_output, overlay, "multiline_output");
    merge_i64_into_u8(&mut base.initial_octave, overlay, "initial_octave");
    merge_string_vec(&mut base.available_packs, overlay, "available_packs");
    merge_string_vec(&mut base.available_instruments, overlay, "available_instruments");
}

pub fn load(config_path: Option<&str>) -> Config {
    let mut cfg = Config::default();

    if let Some(central) = load_central_section() {
        merge_config(&mut cfg, &central);
    }

    if let Some(path) = config_path {
        if let Ok(contents) = std::fs::read_to_string(path) {
            if let Ok(local) = toml::from_str::<TomlValue>(&contents) {
                merge_config(&mut cfg, &local);
            }
        } else {
            eprintln!("Warning: Config file '{}' not found. Using defaults.", path);
        }
    }

    cfg
}

pub fn init(config_path: Option<&str>) {
    let config = load(config_path);
    CONFIG.set(config).ok();
}
