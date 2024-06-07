use std::collections::HashMap;
use std::num::NonZeroU64;
use std::str::FromStr;
use std::time::Duration;
use bigdecimal::{BigDecimal, FromPrimitive, One, RoundingMode};
use rosc::OscType;

// Round up to nearest multiple of fraction
pub fn round_up_to_nearest(source: BigDecimal, fraction: BigDecimal) -> BigDecimal {
    let multiplier = BigDecimal::one() / fraction.clone();

    let prec = NonZeroU64::new(1).unwrap();
    let full_times = (source * multiplier).with_precision_round(prec, RoundingMode::Ceiling);
    fraction * full_times
}

// Round <source> to the nearest multiple of <fraction>, e.g. <0.73, 0.25> => <0.75>
pub fn round_to_nearest(source: BigDecimal, fraction: BigDecimal) -> BigDecimal {
    let multiplier = BigDecimal::one() / fraction.clone();
    let full_times = (source * multiplier).round(0);
    fraction * full_times
}

pub fn duration_to_beats(duration: Duration, bpm: i64) -> BigDecimal {
    // E.g. 60 / 120 = 2 beats per second
    let beats_per_second = BigDecimal::from_i64(60).unwrap() / BigDecimal::from_i64(bpm).unwrap();
    let seconds_elapsed = BigDecimal::from_u128(duration.as_nanos()).unwrap()
        / BigDecimal::from_str("1000000000.000000000").unwrap();
    seconds_elapsed / beats_per_second
}

pub fn shuttlefiy_args(args: Vec<OscType>) -> String {
    let mut map: HashMap<String, OscType> = HashMap::new();

    let mut last_key_lol: Option<String> = None;
    let mut expect_key = true;
    for arg in args {
        match arg {
            OscType::String(value) => {
                if expect_key {
                    expect_key = false;
                    last_key_lol = Some(value);
                }
            }
            value => {
                if !expect_key && last_key_lol.clone().is_some() {
                    expect_key = true;
                    map.insert(last_key_lol.clone().unwrap(), value);
                }
            }
        }
    }

    map.iter()
        .map(|entry| {
            let val: String = match entry.1 {
                OscType::Int(int) => int.to_string(),
                OscType::Float(float) => float.to_string(),
                OscType::String(str) => str.to_string(),
                _ => "err".to_string(),
            };

            format!("{}{}", entry.0, val)
        })
        .collect::<Vec<String>>().join(",")
}

