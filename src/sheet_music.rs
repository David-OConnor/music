//! Generates sheet music from a composition. MusicXML to start. Perhaps PDF too.
//! Supports raw .musicxml and compressed .mxl formats.
//!
//! We may use either the `musicxml` crate, or do this directly using an XML library.
//! We will start with the library, but if it becomes easier/simpler to write and read XML directly,
//! we will do that.
//!
//! MusicXML standard: https://w3c-cg.github.io/musicxml/

use std::{io, path::Path};

use crate::composition::Composition;

pub fn write_sheet_music(comp: &Composition, path: &Path) -> io::Result<()> {
    Ok(())
}

pub fn read_sheet_music(path: &Path) -> io::Result<Composition> {
    unimplemented!()
}
