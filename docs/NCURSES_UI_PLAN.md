# NCurses UI Redesign Plan

## Current State

The ncurses UI is a barebones scrolling plane that prints a banner and separator. All state (octave, BPM, quantization, instrument, sample pack, etc.) is invisible — it lives in shared `State` and on the clipboard but has zero terminal presence.

## Goals

- Surface every toggle and value currently hidden in shared state
- Visual keyboard showing pressed keys in real time
- Live history preview (shuttle notation string as it builds)
- Mode system (keyboard vs sampler) with clear visual indication
- Keybindings discoverable from the UI itself
- No mouse support

## Layout

```
┌───────────────────────────────────────────────────────────────┐
│ jdw-keys-backend v0.1          Router: 127.0.0.1:13339       │
├───────────────────────────────────────────────────────────────┤
│                                                               │
│  STATUS                                                       │
│  Octave: 5      BPM: 120      Quant: 1/8                     │
│  Instrument: aPad   Pack: EMU_EDrum                           │
│  Mode: [KEYBOARD]   ● Recording                              │
│                                                               │
│  KEYBOARD                                                     │
│    q  2  w  3  e  r  5  t  6  y  7  u  i  9  o  0  p       │
│    ▼     ▼     ▼  ▼    ▼     ▼     ▼  ▼                     │
│                                                               │
│  PADS                                                         │
│    a s d f g h j k l                                          │
│    z x c v b n m                                              │
│                                                               │
│  HISTORY                                                      │
│  (60:1.0 62:1.0 64:0.5):len4,tot4                            │
│                                                               │
│  EVENTS   23    Duration: 4 beats                             │
│                                                               │
│  MIDI: ● Connected   OSC: ● Listening                         │
│                                                               │
│  F2:Mode  F3:Record  F4:Quantize  F5:Multi  F6:Bank  +/-:Oct  S+Enter:Clear  F10:Quit│
└───────────────────────────────────────────────────────────────┘
```

## Thread Model Changes

Add `Arc<Mutex<State>>` reference to `NcursesDaemon` so it can read live state during render cycles. No new threads needed.

## Command Channel

Add `MIDIEvent::Command(NcursesCommand)` variant so ncurses can request state changes (mode toggle, recording toggle, etc.) from the MIDI processing thread which owns the state mutex.

## Implementation Phases (completed)

### Phase 1: State extensions + command channel
- Add `KeyboardMode`, `record_history`, `quantize_enabled`, `multiline_output` to `State`
- Add `NcursesCommand` enum and `MIDIEvent::Command` variant
- Handle commands in the MIDI processing loop
- Pass `Arc<Mutex<State>>` and `Arc<Mutex<EventHistory>>` to ncurses
- Builds but no visible UI change

### Phase 2: Full ncurses rewrite
- Replace scrolling plane with fixed-position panels
- Header bar, status panel, mode panel
- Visual keyboard with pressed-key highlighting
- Pad rows
- History preview pane
- MIDI/OSC connection indicators
- Footer with keybindings
- Render throttled via dirty-flag + 30fps cap
- Sampler mode (keyboard keys produce `MIDIEvent::AbsPad` instead of `Key`)
- F2/F10 quit, F2 mode toggle, +/- octave, Enter clear, Shift modifier

### Phase 3: Event log panel
- Rolling log of recent events (NoteOn, NoteOff, PadHit) logged locally in ncurses
- Note names via `tone_to_oletter` for readability
- Capped at 100 entries, last 5 displayed
- MIDI/OSC connection dot indicators

### Phase 4: Fix keybinding conflicts
- Replace conflicting char-based toggles (r, q, l, p) with non-conflicting F-keys
- F3: Record toggle, F4: Quantize toggle, F5: Multiline toggle, F6: Pad bank cycle
- All toggle keybindings safe from keyboard/pad note conflicts
