use std::collections::{HashSet, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::fmt::Write;

use notcurses::*;
use ringbuf::storage::Heap;
use ringbuf::traits::{Consumer, Producer};
use ringbuf::wrap::caching::Caching;
use ringbuf::SharedRb;

use crate::config::Config;
use crate::event_history::EventHistory;
use crate::keyboard_model::Key as KbKey;
use crate::keyboard_model::{AbsPad, KnobButton, MIDIEvent, NcursesCommand, ShiftButton};
use crate::midi_translation::tone_to_oletter;
use crate::state::{KeyboardMode, State};

const KEYBOARD_KEYS: [char; 17] = [
    'q', '2', 'w', '3', 'e', 'r', '5', 't', '6', 'y', '7', 'u', 'i', '9', 'o', '0', 'p',
];

    const PAD_KEYS: [char; 16] = [
        'a', 's', 'd', 'f', 'g', 'h', 'j', 'k', 'l', 'z', 'x', 'c', 'v', 'b', 'n', 'm',
    ];

    const MOD_KEYS: [char; 2] = ['+', '-'];

    const MAX_LOG_ENTRIES: usize = 100;

enum EditTarget {
    Instrument,
    Pack,
}

enum EditState {
    TextEdit(String),
    Dropdown {
        target: EditTarget,
        items: Vec<String>,
        selected: usize,
    },
}

#[derive(Clone)]
pub struct KeyboardModeState {
    pub octave: u8,
}

pub struct NcursesDaemon {
    publisher: Caching<Arc<SharedRb<Heap<MIDIEvent>>>, true, false>,
    state_sub: Caching<Arc<SharedRb<Heap<KeyboardModeState>>>, false, true>,
    state: Arc<Mutex<State>>,
    _history: Arc<Mutex<EventHistory>>,
}

impl NcursesDaemon {
    pub fn new(
        publisher: Caching<Arc<SharedRb<Heap<MIDIEvent>>>, true, false>,
        state_sub: Caching<Arc<SharedRb<Heap<KeyboardModeState>>>, false, true>,
        state: Arc<Mutex<State>>,
        history: Arc<Mutex<EventHistory>>,
    ) -> NcursesDaemon {
        NcursesDaemon {
            publisher,
            state_sub,
            state,
            _history: history,
        }
    }

    fn build_ui(&self, curr_octave: u8, pressed_keys: &HashSet<char>, pressed_pads: &HashSet<char>, event_log: &VecDeque<String>, edit_state: &Option<EditState>) -> String {
        let shared = self.state.lock().unwrap();
        let bpm = shared.bpm;
        let quant = shared.quantization.to_string();
        let instrument = shared.instrument_name.clone();
        let mode = shared.keyboard_mode;
        let recording = shared.record_history;
        let pack = shared.pads_configuration.pack_name.clone();
        let history_preview = shared.history_preview.clone();
        drop(shared);

        let mode_label = match mode {
            KeyboardMode::Keyboard => "KEYBOARD",
            KeyboardMode::Sampler => "SAMPLER",
        };
        let rec_dot = if recording { "●" } else { "○" };

        let mut ui = String::new();

        let _ = writeln!(ui, "jdw-keys-backend v0.1          Router: 127.0.0.1:13339");
        let _ = writeln!(ui, "{}", "-".repeat(78));
        let _ = writeln!(ui);
        let base_note = tone_to_oletter(curr_octave * 12);
        let _ = writeln!(ui, "  Octave: {} ({})    BPM: {}    Quant: {}", curr_octave, base_note, bpm, quant);
        if let Some(EditState::TextEdit(buf)) = edit_state {
            let _ = writeln!(ui, "  Instrument\u{2192} {}_    Pack: {}", buf, pack);
        } else if let Some(EditState::Dropdown { target, items, selected, .. }) = edit_state {
            let label = match target {
                EditTarget::Instrument => "Instrument",
                EditTarget::Pack => "Pack",
            };
            let _ = writeln!(ui, "  {}: {}    {}", label, items[*selected], if matches!(target, EditTarget::Instrument) { format!("Pack: {}", pack) } else { String::new() });
            for (i, item) in items.iter().enumerate() {
                if i == *selected {
                    let _ = writeln!(ui, "    \u{25b6} {}", item);
                } else {
                    let _ = writeln!(ui, "    {}", item);
                }
            }
        } else {
            let _ = writeln!(ui, "  Instrument: {}    Pack: {}", instrument, pack);
        }
        let _ = writeln!(ui, "  Mode: [{}]   {} Recording", mode_label, rec_dot);
        let _ = writeln!(ui);

        // Keyboard row 1 (white keys)
        let _ = write!(ui, "  ");
        for &ch in &['q', 'w', 'e', 'r', 't', 'y', 'u', 'i', 'o', 'p'] {
            if pressed_keys.contains(&ch) {
                let _ = write!(ui, "[{}]", ch);
            } else {
                let _ = write!(ui, " {} ", ch);
            }
        }
        let _ = writeln!(ui);

        // Keyboard row 2 (black keys)
        let _ = write!(ui, "   ");
        for &ch in &['2', '3', '5', '6', '7', '9', '0'] {
            if pressed_keys.contains(&ch) {
                let _ = write!(ui, " [{}]", ch);
            } else {
                let _ = write!(ui, " {} ", ch);
            }
        }
        let _ = writeln!(ui);
        let _ = writeln!(ui);

        // Pad rows
        let _ = writeln!(ui, "  PADS:");
        let _ = write!(ui, "    ");
        for &ch in &PAD_KEYS[..9] {
            if pressed_pads.contains(&ch) {
                let _ = write!(ui, "[{}]", ch);
            } else {
                let _ = write!(ui, " {} ", ch);
            }
        }
        let _ = writeln!(ui);
        let _ = write!(ui, "    ");
        for &ch in &PAD_KEYS[9..] {
            if pressed_pads.contains(&ch) {
                let _ = write!(ui, "[{}]", ch);
            } else {
                let _ = write!(ui, " {} ", ch);
            }
        }
        let _ = writeln!(ui);
        let _ = writeln!(ui);

        // History
        let _ = writeln!(ui, "  HISTORY:");
        let _ = writeln!(ui, "  {}", history_preview);
        let _ = writeln!(ui);

        // Event log
        let _ = writeln!(ui, "  EVENTS:");
        for entry in event_log.iter().rev().take(5) {
            let _ = writeln!(ui, "    {}", entry);
        }
        let _ = writeln!(ui);

        // Connection
        let _ = writeln!(ui, "  MIDI: ● Connected   OSC: ● Listening");
        let _ = writeln!(ui, "{}", "-".repeat(78));
        if matches!(edit_state, Some(EditState::TextEdit(_))) {
            let _ = writeln!(ui, "  ESC:Cancel  Enter:Confirm");
        } else if matches!(edit_state, Some(EditState::Dropdown { .. })) {
            let _ = writeln!(ui, "  \u{2191}\u{2193}:Navigate  Enter:Select  Esc:Cancel");
        } else {
            let _ = writeln!(ui, "  F2:Mode  F3:Record  F4:Quantize  F5:Multi  F6:Pack  F7:Inst  F8:-Oct  F9:+Oct  S+Enter:Clear  F10:Quit");
        }

        ui
    }

    pub fn begin(&mut self) -> NotcursesResult<()> {
        let mut nc = Notcurses::new()?;
        let mut plane = Plane::new(&mut nc)?;
        plane.set_scrolling(true);

        let mut curr_octave: u8 = Config::get().initial_octave;
        let mut shift_pressed = false;
        let mut pressed_keys: HashSet<char> = HashSet::new();
        let mut pressed_pads: HashSet<char> = HashSet::new();
        let mut event_log: VecDeque<String> = VecDeque::with_capacity(MAX_LOG_ENTRIES);
        let mut last_render = Instant::now();
        let render_interval = Duration::from_millis(33);
        let mut edit_state: Option<EditState> = None;
        let mut idle_count: u32 = 0;

        loop {
            let now = Instant::now();
            if now - last_render >= render_interval {
                last_render = now;

                if let Some(val) = self.state_sub.try_pop() {
                    curr_octave = val.octave;
                }

                let ui = self.build_ui(curr_octave, &pressed_keys, &pressed_pads, &event_log, &edit_state);
                plane.erase();
                plane.putstr(&ui)?;
                plane.render()?;
                plane.rasterize()?;
            }

            let had_event = match nc.poll_event()? {
                ref e if e.received() => {
                    let event = e;

                    if let Some(ref mut state) = edit_state {
                        match state {
                            EditState::TextEdit(buffer) => {
                                if let Some(c) = event.received.char() {
                                    if c.is_ascii_graphic() || c == ' ' {
                                        buffer.push(c);
                                    }
                                } else if event.is_key(Key::Backspace) && event.is_press() {
                                    buffer.pop();
                                } else if event.is_key(Key::Enter) {
                                    let name = buffer.clone();
                                    let _ = self.publisher.try_push(MIDIEvent::Command(NcursesCommand::SetInstrument(name)));
                                    edit_state = None;
                                } else if event.is_key(Key::Esc) || event.is_key(Key::F01) {
                                    edit_state = None;
                                }
                            }
                            EditState::Dropdown { target, items, selected } => {
                                if event.is_key(Key::Enter) {
                                    let name = items[*selected].clone();
                                    let cmd = match target {
                                        EditTarget::Instrument => NcursesCommand::SetInstrument(name),
                                        EditTarget::Pack => NcursesCommand::SetPack(name),
                                    };
                                    let _ = self.publisher.try_push(MIDIEvent::Command(cmd));
                                    edit_state = None;
                                } else if event.is_key(Key::Esc) || event.is_key(Key::F01) {
                                    edit_state = None;
                                } else if event.is_key(Key::Up) && *selected > 0 {
                                    *selected -= 1;
                                } else if event.is_key(Key::Down) && *selected + 1 < items.len() {
                                    *selected += 1;
                                }
                            }
                        }
                    } else {
                        for &pad_key in &PAD_KEYS {
                            if event.is_char(pad_key) {
                                if event.is_press() {
                                    pressed_pads.insert(pad_key);
                                    let pad_id = PAD_KEYS.iter().position(|&e| e == pad_key).unwrap() as u8 + 1;
                                    event_log.push_back(format!("PadHit  pad:{}", pad_id));
                                    let _ = self.publisher.try_push(MIDIEvent::AbsPad(AbsPad {
                                        id: pad_id,
                                        pressed: true,
                                    }));
                                } else if event.is_release() {
                                    pressed_pads.remove(&pad_key);
                                }
                            }
                        }

                        for &char_key in &KEYBOARD_KEYS {
                            if event.is_char(char_key) && event.is_release() {
                                let midi_note_raw = KEYBOARD_KEYS.iter().position(|&e| e == char_key).unwrap() as u8;
                                let midi_note = (curr_octave * 12u8) + midi_note_raw;
                                pressed_keys.remove(&char_key);
                                event_log.push_back(format!("NoteOff {}", tone_to_oletter(midi_note)));
                                let _ = self.publisher.try_push(MIDIEvent::Key(KbKey {
                                    pressed: false,
                                    midi_note,
                                    force: 127,
                                }));
                            }

                            if event.is_char(char_key) && event.is_press() {
                                let midi_note_raw = KEYBOARD_KEYS.iter().position(|&e| e == char_key).unwrap() as u8;
                                let midi_note = (curr_octave * 12u8) + midi_note_raw;
                                pressed_keys.insert(char_key);
                                let shared = self.state.lock().unwrap();
                                let is_sampler = shared.keyboard_mode == KeyboardMode::Sampler;
                                drop(shared);

                                if is_sampler {
                                    let pad_id = midi_note_raw + 1;
                                    event_log.push_back(format!("PadHit  pad:{}", pad_id));
                                    let _ = self.publisher.try_push(MIDIEvent::AbsPad(AbsPad {
                                        id: pad_id,
                                        pressed: true,
                                    }));
                                } else {
                                    event_log.push_back(format!("NoteOn  {}  vel:127", tone_to_oletter(midi_note)));
                                    let _ = self.publisher.try_push(MIDIEvent::Key(KbKey {
                                        pressed: true,
                                        midi_note,
                                        force: 127,
                                    }));
                                }
                            }
                        }

                        for &mod_key in &MOD_KEYS {
                            if event.is_char(mod_key) && event.is_press() {
                                if shift_pressed {
                                    if event.is_char('+') {
                                        curr_octave += 1;
                                    } else {
                                        curr_octave = curr_octave.saturating_sub(1);
                                    }
                                } else {
                                    let emulated_knob_id = if event.is_char('+') { 113 } else { 115 };
                                    let _ = self.publisher.try_push(MIDIEvent::KnobButton(KnobButton {
                                        id: emulated_knob_id,
                                        pressed: true,
                                    }));
                                }
                            }
                        }

                        if event.is_key(Key::Enter) {
                            let _ = self.publisher.try_push(MIDIEvent::ShiftButton(ShiftButton {
                                pressed: true,
                            }));
                        }

                        if event.is_key(Key::LShift) {
                            if event.is_press() {
                                shift_pressed = true;
                            } else if event.is_release() {
                                shift_pressed = false;
                            }
                        }

                        if event.is_key(Key::F01) || event.is_key(Key::F10) {
                            break;
                        }

                        if event.is_key(Key::F02) {
                            let _ = self.publisher.try_push(MIDIEvent::Command(NcursesCommand::ToggleMode));
                        }

                        if event.is_key(Key::F03) {
                            let _ = self.publisher.try_push(MIDIEvent::Command(NcursesCommand::ToggleRecording));
                        }

                        if event.is_key(Key::F04) {
                            let _ = self.publisher.try_push(MIDIEvent::Command(NcursesCommand::ToggleQuantize));
                        }

                        if event.is_key(Key::F05) {
                            let _ = self.publisher.try_push(MIDIEvent::Command(NcursesCommand::ToggleMultiline));
                        }

                        if event.is_key(Key::F07) && edit_state.is_none() {
                            let shared = self.state.lock().unwrap();
                            let instrument = shared.instrument_name.clone();
                            let avail = shared.available_instruments.clone();
                            drop(shared);
                            if avail.is_empty() {
                                edit_state = Some(EditState::TextEdit(instrument));
                            } else {
                                let selected = avail.iter().position(|a| a == &instrument).unwrap_or(0);
                                edit_state = Some(EditState::Dropdown { target: EditTarget::Instrument, items: avail, selected });
                            }
                        }

                        if event.is_key(Key::F06) && edit_state.is_none() {
                            let shared = self.state.lock().unwrap();
                            let current_pack = shared.pads_configuration.pack_name.clone();
                            let avail = shared.available_packs.clone();
                            drop(shared);
                            if !avail.is_empty() {
                                let selected = avail.iter().position(|a| a == &current_pack).unwrap_or(0);
                                edit_state = Some(EditState::Dropdown { target: EditTarget::Pack, items: avail, selected });
                            }
                        }

                        if event.is_key(Key::F08) {
                            curr_octave = curr_octave.saturating_sub(1);
                        }
                        if event.is_key(Key::F09) {
                            curr_octave = curr_octave.saturating_add(1);
                        }

                        while event_log.len() > MAX_LOG_ENTRIES {
                            event_log.pop_front();
                        }
                    }
                    true
                }
                _ => false
            };

            if had_event {
                idle_count = 0;
            } else {
                idle_count = idle_count.saturating_add(1);
                let delay = if idle_count > 10 { 2000 }
                    else if idle_count > 5 { 500 }
                    else { 100 };
                std::thread::sleep(Duration::from_micros(delay));
            }
        }

        Ok(())
    }
}
