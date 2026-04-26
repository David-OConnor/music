#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InstrumentType {
    Plucked,
    Bowed,
    Brass,
    Woodwind,
    Percussion,
    Voice,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Instrument {
    Piano,
    Guitar,
    BassGuitar,
    Drums,
    Violin,
    Viola,
    Cello,
    DoubleBass,
    Trumpet,
    Saxophone,
    Flute,
    Oboe,
    Clarinet,
    Banjo,
}

impl Instrument {
    pub fn type_(self) -> InstrumentType {
        use Instrument::*;
        use InstrumentType::*;
        match self {
            Piano => Percussion,
            Guitar | BassGuitar | Banjo => Plucked,
            Violin | Viola | Cello | DoubleBass => Bowed,
            Trumpet => Brass,
            Saxophone | Flute | Oboe | Clarinet => Woodwind,
            Drums => Percussion,
        }
    }
}
