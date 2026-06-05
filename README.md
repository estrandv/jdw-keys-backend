# jdw-keys-backend

MIDI keyboard controller backend for the JackDAW system. Translates MIDI input (keys, pads, knobs, buttons) into OSC messages and sends them to the JDW OSC router.

## Features

- **Note-on/note-off** — keyboard keys trigger configurable synths
- **Pad/sample playback** — drum pads trigger samples with sample pack browsing
- **Control bus modulation** — knobs/sliders map to synth parameters in real-time
- **Event history → clipboard as Shuttle Notation** — play notes on the keyboard, then export the sequence directly to your clipboard as valid [Shuttle Notation](https://github.com/estrandv/shuttle-notation-python) text for pasting into a `.bbd` billboard file
- **Ncurses terminal UI** — octave display, state control, keyboard mode
- **OSC-based runtime config** — BPM, quantization, instrument, samples all configurable via OSC
- **MIDI mapping** — flexible mapping of MIDI CC to JDW control buses

## Composing with the Keyboard CLI

The killer feature: use the MIDI keyboard as a composition input device.

1. Play notes on the keyboard — the application records timing and note IDs
2. Press the capture key — the sequence is stringified to Shuttle Notation
3. The Shuttle Notation string is copied to your system clipboard automatically
4. Paste directly into your `.bbd` billboard file as track content

Example output:
```
(60:1.0 62:1.0 64:0.5 65:0.5):len4,tot4
```

This creates a tightly integrated loop: compose by playing, paste into your song spec, send to the sequencer, hear it play back through jdw-sc.

## Architecture

```
MIDI Keyboard  <--->  jdw-keys-backend  <--->  OSC Router (port 13339)
                              |
                    Ncurses TUI (state display)
                              |
                    Clipboard (Shuttle Notation)
```

## Dependencies

- `rosc` — OSC encoding/decoding
- `midir` — MIDI input
- `jdw-osc-lib` — shared OSC protocol library
- `ringbuf` — inter-thread communication
- `notcurses` — terminal UI
- `wl-clipboard-rs` — clipboard access for Shuttle Notation export
