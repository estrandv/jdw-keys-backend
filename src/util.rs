use std::collections::HashMap;
use std::num::NonZeroU64;
use std::ops::Range;
use std::str::FromStr;
use std::time::Duration;
use bigdecimal::{BigDecimal, FromPrimitive, One, RoundingMode, ToPrimitive};
use rosc::OscType;

pub fn midi_to_float(range: Range<f32>, value: u8) -> f32 {
    const MIDI_RANGE: f32 = 127.0;
    let min = range.start;
    let max = range.end;
    let diff = max - min;
    let step = diff / MIDI_RANGE;
    ((value as f32) * step) - min.abs()
}

pub fn next_power_of_two(source: BigDecimal) -> BigDecimal {
    // Round(0) removes decimal digits
    let integer = source.with_scale_round(0, RoundingMode::Up).to_u64().unwrap();
    let nearest = integer.next_power_of_two();
    BigDecimal::from_str(format!("{}", nearest).as_str()).unwrap()
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify() {

        assert_eq!(round_to_nearest(
            BigDecimal::from_str("0.23").unwrap(),
            BigDecimal::from_str("0.25").unwrap(),
        ), BigDecimal::from_str("0.25").unwrap());

        assert_eq!(round_to_nearest(
            BigDecimal::from_str("0.73").unwrap(),
            BigDecimal::from_str("0.25").unwrap(),
        ), BigDecimal::from_str("0.75").unwrap());

        assert_eq!(round_to_nearest(
            BigDecimal::from_str("0.76").unwrap(),
            BigDecimal::from_str("0.25").unwrap(),
        ), BigDecimal::from_str("0.75").unwrap());

    }
}