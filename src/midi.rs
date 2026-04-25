//! For converting Compositions to Midi, and reading from Midi.
//!

use std::{io, path::Path};

use crate::composition::Composition;

// todo: Structs for Midi here, but we may not need them. Could just read and write files.
pub fn make_midi(comp: &Composition) -> Midi {}

pub fn save_midi(midi: &Midi, path: &Path) -> io::Result<()> {}
