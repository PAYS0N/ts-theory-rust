//! Drive the [`Sim`] editor over a keystroke stream and report the result.

use std::collections::BTreeMap;

use super::sim::Sim;
use super::{Behaviors, EditorState, Event};

/// Fold one event into the simulation, recording landing marks.
fn apply_event(sim: &mut Sim, ev: &Event, landing: &mut BTreeMap<u32, usize>) {
    match ev {
        Event::Mark(n) => {
            landing.insert(*n, sim.cursor);
        },
        Event::Text(s) => {
            for ch in s.chars() {
                sim.type_char(ch);
            }
        },
        Event::Key { key, n } => {
            for _ in 0..*n {
                sim.apply_key(*key);
            }
        },
    }
}

/// Run a keystroke stream through an editor with the given behaviors.
#[must_use]
pub fn interpret(events: &[Event], b: Behaviors) -> EditorState {
    let mut sim = Sim::new(b);
    let mut landing: BTreeMap<u32, usize> = BTreeMap::new();
    for ev in events {
        apply_event(&mut sim, ev, &mut landing);
    }
    EditorState {
        buffer: sim.buffer.iter().collect(),
        rest: sim.cursor,
        target: landing.get(&0).copied(),
    }
}
