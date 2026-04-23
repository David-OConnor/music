//! Architecture, or structure, for songs. We will start by setting up the basics
//! which can replicate the most common works. E.g. perhaps popular contemporary to start.
//!
//! This is used in *songwriting*.

//! This is somewhat provicial, but may be a good starting point, as it's applicable to many
//! works.
#[derive(Clone, Copy, PartialEq)]
pub enum CompositionComponent {
    Intro,
    Verse,
    PreChorus,
    Chorus,
    Bridge,
    Solo,
    Outro,
    // todo: Consider a name here
    Other,
    // Other(String),
}
