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
│  Octave: 6      BPM: 120      Quant: 0.125                   │
│  Instrument: aPad    Pack: EMU_EDrum                          │
│  Mode: [KEYBOARD]   ● Recording                              │
│                                                               │
│   q  w  e  r  t  y  u  i  o  p                               │
│    2  3  5  6  7  9  0                                       │
│                                                               │
│  PADS                                                         │
│    a  s  d  f  g  h  j  k  l                                 │
│    z  x  c  v  b  n  m                                       │
│                                                               │
│  HISTORY                                                      │
│  (60:1.0 62:1.0 64:0.5):len4,tot4                            │
│                                                               │
│  EVENTS                                                       │
│    NoteOn  C4  vel:127                                       │
│    NoteOff C4                                                 │
│    PadHit  pad:1                                              │
│                                                               │
│  MIDI: ● Connected   OSC: ● Listening                         │
├───────────────────────────────────────────────────────────────┤
│  F2:Mode  F3:Record  F4:Quantize  F5:Multi  F6:Bank  F7:Inst │
│  +/-:Oct  S+Enter:Clear  F10:Quit                            │
└───────────────────────────────────────────────────────────────┘

--- Instrument editor (F7) ---

┌───────────────────────────────────────────────────────────────┐
│ ...                                                           │
│  Instrument→ aPad_    Pack: EMU_EDrum                        │
│ ...                                                           │
│  ESC:Cancel  Enter:Confirm                                   │
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
- `F2`/`F10` quit, `F2` mode toggle, `+`/`-` octave, `Enter` clear, `Shift` modifier

### Phase 3: Event log panel
- Rolling log of recent events (NoteOn, NoteOff, PadHit) logged locally in ncurses
- Note names via `tone_to_oletter` for readability
- Capped at 100 entries, last 5 displayed
- MIDI/OSC connection dot indicators

### Phase 4: Fix keybinding conflicts
- Replace conflicting char-based toggles (r, q, l, p) with non-conflicting F-keys
- `F3`: Record toggle, `F4`: Quantize toggle, `F5`: Multiline toggle, `F6`: Pad bank cycle
- All toggle keybindings safe from keyboard/pad note conflicts

### Phase 5: Instrument editor (inline text prompt)
- New `NcursesCommand::SetInstrument(String)` variant
- `F7` key enters edit mode, pre-filling buffer with current `instrument_name` from `State`
- In edit mode, printable ASCII keys append to buffer, `Backspace` deletes, `Enter` sends `SetInstrument(buffer)` via command channel, `Esc`/`F1` cancels
- UI line changes from `Instrument: aPad` to `Instrument→ aPad_` with cursor indicator
- Footer changes to `ESC:Cancel  Enter:Confirm` while editing
- All keyboard/pad note input suppressed during edit session

### Phase 6: Latency & CPU optimization
- Remove `println!("SENDING KEYPRESS...")` from MIDI hot path (main.rs:311)
- Replace fixed 500µs polling sleeps with adaptive idle detection:
  - Active (events flowing): zero sleep between batches
  - Idle 1–5 cycles: 100µs sleep
  - Idle 6–10 cycles: 500µs sleep
  - Idle 11+: 2ms sleep
  - On any event, `idle_count` resets to 0 for immediate re-check
- History ringbuf `try_push().unwrap()` → `let _ = try_push()` so full buffer silently drops events instead of panicking
- Typical keypress→OSC latency: ~100–200µs sustained, at most ~2ms from deep idle
