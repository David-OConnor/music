//! For playing music.

use std::io;

use tinyaudio;
use crate::composition::Composition;

pub fn play(composition: &Composition) -> io::Result<()> {
    let mms = composition.make_micromeasures();


    Ok(())
}