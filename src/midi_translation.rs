const NOTE_NAMES: [&str; 12] = ["c", "db", "d", "eb", "e", "f", "gb",
"g", "ab", "a", "bb", "b"];

pub fn tone_to_oletter(tone: u8) -> String {

    if tone == 0u8 {
        return "c1".to_string();
    }

    let letter_amount = NOTE_NAMES.len();
    let index = tone as usize;

    let letter_index = if index >= letter_amount {index % letter_amount} else {index};
    let letter = NOTE_NAMES[letter_index];
    let octave = (tone / 11u8) + 1u8;

    format!("{}{}", letter, octave)

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify() {
        assert_eq!(tone_to_oletter(0), "c1");
        assert_eq!(tone_to_oletter(1), "db1");
        assert_eq!(tone_to_oletter(23), "b3");
        assert_eq!(tone_to_oletter(12), "c2");
        assert_eq!(tone_to_oletter(16), "e2");
    }
}