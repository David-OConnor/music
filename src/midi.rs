//! For converting [`Composition`] values to Standard MIDI Files (SMF), and reading them back.

use std::{
    collections::{HashMap, VecDeque},
    fs, io,
    path::Path,
};

use crate::{
    composition::{Composition, NotesStartingThisTick},
    instrument::Instrument,
    key_scale::{Key, MajorMinor, SharpFlat},
    measure::{Measure, TimeSignature},
    note::{Note, NoteDurationGeneral, NoteLetter, NotePlayed},
    overtones::Temperament,
};

#[derive(Clone, Debug)]
struct TimedEvent {
    abs_tick: u32,
    priority: u8,
    seq: usize,
    kind: TimedEventKind,
}

#[derive(Clone, Debug)]
enum TimedEventKind {
    Tempo(u32),
    TimeSignature {
        numerator: u8,
        denominator: u8,
    },
    KeySignature {
        fifths: i8,
        minor: bool,
    },
    ProgramChange {
        channel: u8,
        program: u8,
    },
    NoteOn {
        channel: u8,
        pitch: u8,
        velocity: u8,
    },
    NoteOff {
        channel: u8,
        pitch: u8,
        velocity: u8,
    },
}

#[derive(Clone, Debug)]
struct ImportedNote {
    start_tick: u32,
    end_tick: u32,
    channel: u8,
    pitch: u8,
    velocity: u8,
    program: Option<u8>,
}

#[derive(Clone, Debug)]
struct ActiveNote {
    start_tick: u32,
    velocity: u8,
    program: Option<u8>,
}

#[derive(Clone, Copy)]
struct Header {
    format: u16,
    track_count: u16,
    division: u16,
}

#[derive(Clone, Copy)]
struct MetaState<T: Copy> {
    tick: u32,
    value: T,
}

struct ByteReader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> ByteReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.pos)
    }

    fn read_u8(&mut self) -> io::Result<u8> {
        if self.pos >= self.bytes.len() {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "unexpected end of MIDI data",
            ));
        }
        let byte = self.bytes[self.pos];
        self.pos += 1;
        Ok(byte)
    }

    fn read_exact(&mut self, len: usize) -> io::Result<&'a [u8]> {
        let end = self.pos.checked_add(len).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "MIDI chunk length overflow")
        })?;
        if end > self.bytes.len() {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "unexpected end of MIDI data",
            ));
        }
        let slice = &self.bytes[self.pos..end];
        self.pos = end;
        Ok(slice)
    }

    fn read_u16_be(&mut self) -> io::Result<u16> {
        let bytes = self.read_exact(2)?;
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    fn read_u32_be(&mut self) -> io::Result<u32> {
        let bytes = self.read_exact(4)?;
        Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }
}

fn write_midi_header(ppq: u16, track_len: u32) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(14 + track_len as usize);
    bytes.extend_from_slice(b"MThd");
    bytes.extend_from_slice(&6_u32.to_be_bytes());
    bytes.extend_from_slice(&0_u16.to_be_bytes());
    bytes.extend_from_slice(&1_u16.to_be_bytes());
    bytes.extend_from_slice(&ppq.to_be_bytes());
    bytes.extend_from_slice(b"MTrk");
    bytes.extend_from_slice(&track_len.to_be_bytes());
    bytes
}

fn write_vlq(bytes: &mut Vec<u8>, mut value: u32) {
    let mut stack = [0_u8; 5];
    let mut idx = stack.len() - 1;
    stack[idx] = (value & 0x7f) as u8;
    value >>= 7;

    while value > 0 {
        idx -= 1;
        stack[idx] = ((value & 0x7f) as u8) | 0x80;
        value >>= 7;
    }

    bytes.extend_from_slice(&stack[idx..]);
}

fn read_vlq(reader: &mut ByteReader<'_>) -> io::Result<u32> {
    let mut value = 0_u32;
    for _ in 0..4 {
        let byte = reader.read_u8()?;
        value = (value << 7) | u32::from(byte & 0x7f);
        if byte & 0x80 == 0 {
            return Ok(value);
        }
    }
    Err(io::Error::new(
        io::ErrorKind::InvalidData,
        "invalid variable-length quantity in MIDI file",
    ))
}

fn natural_semitone(letter: NoteLetter) -> i32 {
    use NoteLetter::*;

    match letter {
        C => 0,
        D => 2,
        E => 4,
        F => 5,
        G => 7,
        A => 9,
        B => 11,
    }
}

fn key_accidental(letter: NoteLetter, key: Key) -> SharpFlat {
    use NoteLetter::*;

    let sig = key.get_sharps_flats();
    match letter {
        A => sig.a,
        B => sig.b,
        C => sig.c,
        D => sig.d,
        E => sig.e,
        F => sig.f,
        G => sig.g,
    }
}

fn note_to_midi_pitch(note: &Note, key: Key) -> io::Result<u8> {
    use SharpFlat::*;

    let accidental = note
        .sharp_flat
        .unwrap_or_else(|| key_accidental(note.letter, key));
    let semitone = natural_semitone(note.letter)
        + match accidental {
            Sharp => 1,
            Flat => -1,
            Natural => 0,
        };
    let midi_pitch = (i32::from(note.octave) + 1) * 12 + semitone;
    u8::try_from(midi_pitch).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("note {note} is outside the MIDI pitch range"),
        )
    })
}

fn key_to_fifths(key: Key) -> i8 {
    let sharps_flats = key.get_sharps_flats();
    let accidentals = [
        sharps_flats.f,
        sharps_flats.c,
        sharps_flats.g,
        sharps_flats.d,
        sharps_flats.a,
        sharps_flats.e,
        sharps_flats.b,
    ];
    let sharp_count = accidentals
        .iter()
        .filter(|&&sf| sf == SharpFlat::Sharp)
        .count() as i8;
    let flat_count = accidentals
        .iter()
        .filter(|&&sf| sf == SharpFlat::Flat)
        .count() as i8;
    sharp_count - flat_count
}

fn fifths_to_key(fifths: i8, mode: MajorMinor) -> Key {
    use MajorMinor::*;
    use NoteLetter::*;
    use SharpFlat::*;

    let (base_note, sharp_flat) = match (mode, fifths) {
        (Major, 0) => (C, Natural),
        (Major, 1) => (G, Natural),
        (Major, 2) => (D, Natural),
        (Major, 3) => (A, Natural),
        (Major, 4) => (E, Natural),
        (Major, 5) => (B, Natural),
        (Major, 6) => (F, Sharp),
        (Major, 7) => (C, Sharp),
        (Major, -1) => (F, Natural),
        (Major, -2) => (B, Flat),
        (Major, -3) => (E, Flat),
        (Major, -4) => (A, Flat),
        (Major, -5) => (D, Flat),
        (Major, -6) => (G, Flat),
        (Major, -7) => (C, Flat),
        (Minor, 0) => (A, Natural),
        (Minor, 1) => (E, Natural),
        (Minor, 2) => (B, Natural),
        (Minor, 3) => (F, Sharp),
        (Minor, 4) => (C, Sharp),
        (Minor, 5) => (G, Sharp),
        (Minor, 6) => (D, Sharp),
        (Minor, 7) => (A, Sharp),
        (Minor, -1) => (D, Natural),
        (Minor, -2) => (G, Natural),
        (Minor, -3) => (C, Natural),
        (Minor, -4) => (F, Natural),
        (Minor, -5) => (B, Flat),
        (Minor, -6) => (E, Flat),
        (Minor, -7) => (A, Flat),
        _ => (C, Natural),
    };

    Key::new(base_note, sharp_flat, mode)
}

fn midi_pitch_to_note(pitch: u8, key: Key) -> Note {
    use NoteLetter::*;
    use SharpFlat::*;

    let octave = pitch / 12 - 1;
    let pitch_class = pitch % 12;
    let prefer_flats = key_to_fifths(key) < 0;

    let (letter, accidental) = match (pitch_class, prefer_flats) {
        (0, _) => (C, Natural),
        (1, true) => (D, Flat),
        (1, false) => (C, Sharp),
        (2, _) => (D, Natural),
        (3, true) => (E, Flat),
        (3, false) => (D, Sharp),
        (4, _) => (E, Natural),
        (5, _) => (F, Natural),
        (6, true) => (G, Flat),
        (6, false) => (F, Sharp),
        (7, _) => (G, Natural),
        (8, true) => (A, Flat),
        (8, false) => (G, Sharp),
        (9, _) => (A, Natural),
        (10, true) => (B, Flat),
        (10, false) => (A, Sharp),
        (11, _) => (B, Natural),
        _ => unreachable!(),
    };

    Note::new(letter, Some(accidental), octave)
}

fn instrument_channel(instr: Instrument) -> u8 {
    if instr == Instrument::Drums { 9 } else { 0 }
}

fn instrument_to_program(instr: Instrument) -> Option<u8> {
    Some(match instr {
        Instrument::Piano => 0,
        Instrument::Guitar => 24,
        Instrument::BassGuitar => 33,
        Instrument::Drums => return None,
        Instrument::Violin => 40,
        Instrument::Viola => 41,
        Instrument::Cello => 42,
        Instrument::DoubleBass => 43,
        Instrument::Trumpet => 56,
        Instrument::Saxophone => 65,
        Instrument::Flute => 73,
        Instrument::Oboe => 68,
        Instrument::Clarinet => 71,
        Instrument::Banjo => 105,
    })
}

fn program_to_instrument(program: u8, channel: u8) -> Instrument {
    if channel == 9 {
        return Instrument::Drums;
    }

    match program {
        0..=7 => Instrument::Piano,
        24..=31 => Instrument::Guitar,
        32..=39 => Instrument::BassGuitar,
        40 => Instrument::Violin,
        41 => Instrument::Viola,
        42 => Instrument::Cello,
        43..=47 => Instrument::DoubleBass,
        56..=63 => Instrument::Trumpet,
        64..=67 => Instrument::Saxophone,
        68..=69 => Instrument::Oboe,
        71 => Instrument::Clarinet,
        73..=79 => Instrument::Flute,
        104..=105 => Instrument::Banjo,
        _ => Instrument::Piano,
    }
}

fn amplitude_to_velocity(amplitude: f32) -> u8 {
    if !amplitude.is_finite() {
        return 100;
    }
    let clamped = amplitude.clamp(0.0, 1.0);
    let scaled = (clamped * 126.0).round() as u8;
    scaled.saturating_add(1)
}

fn velocity_to_amplitude(velocity: u8) -> f32 {
    f32::from(velocity) / 127.0
}

fn gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        let next = a % b;
        a = b;
        b = next;
    }
    a
}

fn lcm(a: u32, b: u32) -> u32 {
    if a == 0 || b == 0 {
        0
    } else {
        a / gcd(a, b) * b
    }
}

fn ticks_per_measure(time_signature: TimeSignature, ticks_per_sixteenth: u32) -> u32 {
    ticks_per_sixteenth * 16 * u32::from(time_signature.numerator)
        / u32::from(time_signature.denominator)
}

fn denom_to_power(denominator: u8) -> io::Result<u8> {
    if denominator == 0 || !denominator.is_power_of_two() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unsupported time signature denominator {denominator} for MIDI"),
        ));
    }
    Ok(denominator.trailing_zeros() as u8)
}

fn tempo_to_micros_per_quarter(ms_per_tick: u32, ticks_per_quarter: u16) -> io::Result<u32> {
    let micros = u64::from(ms_per_tick) * u64::from(ticks_per_quarter) * 1000;
    if micros == 0 || micros > 0x00ff_ffff {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "tempo is outside the representable MIDI range",
        ));
    }
    Ok(micros as u32)
}

fn micros_per_quarter_to_ms_per_tick(micros_per_quarter: u32, quarter_ticks: u32) -> u32 {
    let denom = u64::from(quarter_ticks) * 1000;
    let rounded = (u64::from(micros_per_quarter) + denom / 2) / denom;
    rounded.max(1) as u32
}

fn emit_event_payload(bytes: &mut Vec<u8>, event: &TimedEventKind) -> io::Result<()> {
    match event {
        TimedEventKind::Tempo(micros) => {
            if *micros > 0x00ff_ffff {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "tempo exceeds MIDI meta-event capacity",
                ));
            }
            bytes.extend_from_slice(&[0xff, 0x51, 0x03]);
            bytes.push(((micros >> 16) & 0xff) as u8);
            bytes.push(((micros >> 8) & 0xff) as u8);
            bytes.push((micros & 0xff) as u8);
        }
        TimedEventKind::TimeSignature {
            numerator,
            denominator,
        } => {
            bytes.extend_from_slice(&[
                0xff,
                0x58,
                0x04,
                *numerator,
                denom_to_power(*denominator)?,
                24,
                8,
            ]);
        }
        TimedEventKind::KeySignature { fifths, minor } => {
            bytes.extend_from_slice(&[0xff, 0x59, 0x02, *fifths as u8, u8::from(*minor)]);
        }
        TimedEventKind::ProgramChange { channel, program } => {
            bytes.push(0xc0 | (channel & 0x0f));
            bytes.push(*program);
        }
        TimedEventKind::NoteOn {
            channel,
            pitch,
            velocity,
        } => {
            bytes.push(0x90 | (channel & 0x0f));
            bytes.push(*pitch);
            bytes.push(*velocity);
        }
        TimedEventKind::NoteOff {
            channel,
            pitch,
            velocity,
        } => {
            bytes.push(0x80 | (channel & 0x0f));
            bytes.push(*pitch);
            bytes.push(*velocity);
        }
    }
    Ok(())
}

fn push_timed_event(
    events: &mut Vec<TimedEvent>,
    seq: &mut usize,
    abs_tick: u32,
    priority: u8,
    kind: TimedEventKind,
) {
    events.push(TimedEvent {
        abs_tick,
        priority,
        seq: *seq,
        kind,
    });
    *seq += 1;
}

fn push_default_meta_events(
    comp: &Composition,
    events: &mut Vec<TimedEvent>,
    seq: &mut usize,
    ticks_per_quarter: u16,
) -> io::Result<()> {
    push_timed_event(
        events,
        seq,
        0,
        0,
        TimedEventKind::Tempo(tempo_to_micros_per_quarter(
            comp.ms_per_tick,
            ticks_per_quarter,
        )?),
    );
    push_timed_event(
        events,
        seq,
        0,
        0,
        TimedEventKind::KeySignature {
            fifths: key_to_fifths(comp.key),
            minor: comp.key.major_minor == MajorMinor::Minor,
        },
    );
    push_timed_event(
        events,
        seq,
        0,
        0,
        TimedEventKind::TimeSignature {
            numerator: 4,
            denominator: 4,
        },
    );
    Ok(())
}

fn push_measure_meta_events(
    comp: &Composition,
    events: &mut Vec<TimedEvent>,
    seq: &mut usize,
    ticks_per_quarter: u16,
) -> io::Result<()> {
    if comp.measures.is_empty() {
        return push_default_meta_events(comp, events, seq, ticks_per_quarter);
    }

    let mut tick = 0_u32;
    let mut last_key = None;
    let mut last_time_signature = None;
    let mut last_ms_per_tick = None;

    for measure in &comp.measures {
        if last_key != Some(measure.key) {
            push_timed_event(
                events,
                seq,
                tick,
                0,
                TimedEventKind::KeySignature {
                    fifths: key_to_fifths(measure.key),
                    minor: measure.key.major_minor == MajorMinor::Minor,
                },
            );
            last_key = Some(measure.key);
        }

        if last_time_signature != Some(measure.time_signature) {
            push_timed_event(
                events,
                seq,
                tick,
                0,
                TimedEventKind::TimeSignature {
                    numerator: measure.time_signature.numerator,
                    denominator: measure.time_signature.denominator,
                },
            );
            last_time_signature = Some(measure.time_signature);
        }

        let measure_ms_per_tick = if measure.tempo == 0 {
            comp.ms_per_tick
        } else {
            measure.tempo
        };
        if last_ms_per_tick != Some(measure_ms_per_tick) {
            push_timed_event(
                events,
                seq,
                tick,
                0,
                TimedEventKind::Tempo(tempo_to_micros_per_quarter(
                    measure_ms_per_tick,
                    ticks_per_quarter,
                )?),
            );
            last_ms_per_tick = Some(measure_ms_per_tick);
        }

        tick = tick.saturating_add(ticks_per_measure(
            measure.time_signature,
            comp.ticks_per_sixteenth_note,
        ));
    }

    Ok(())
}

fn composition_to_smf_bytes(comp: &Composition) -> io::Result<Vec<u8>> {
    let ticks_per_quarter_u32 = comp
        .ticks_per_sixteenth_note
        .checked_mul(4)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "ticks-per-quarter overflow"))?;
    let ticks_per_quarter = u16::try_from(ticks_per_quarter_u32).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "ticks-per-quarter exceeds MIDI header capacity",
        )
    })?;

    let primary_instrument = comp
        .instruments
        .first()
        .copied()
        .unwrap_or(Instrument::Piano);
    let channel = instrument_channel(primary_instrument);

    let mut events = Vec::new();
    let mut seq = 0_usize;

    push_measure_meta_events(comp, &mut events, &mut seq, ticks_per_quarter)?;

    if let Some(program) = instrument_to_program(primary_instrument) {
        push_timed_event(
            &mut events,
            &mut seq,
            0,
            0,
            TimedEventKind::ProgramChange { channel, program },
        );
    }

    for (tick_idx, group) in comp.notes_by_tick.iter().enumerate() {
        let start_tick = tick_idx as u32;
        for note in &group.notes {
            let duration = note.duration.get_ticks(comp.ticks_per_sixteenth_note)?;
            let pitch = note_to_midi_pitch(&note.note, comp.key)?;
            let velocity = amplitude_to_velocity(note.amplitude);
            let end_tick = start_tick.saturating_add(duration);

            push_timed_event(
                &mut events,
                &mut seq,
                start_tick,
                2,
                TimedEventKind::NoteOn {
                    channel,
                    pitch,
                    velocity,
                },
            );
            push_timed_event(
                &mut events,
                &mut seq,
                end_tick,
                1,
                TimedEventKind::NoteOff {
                    channel,
                    pitch,
                    velocity: 64,
                },
            );
        }
    }

    events.sort_by_key(|event| (event.abs_tick, event.priority, event.seq));

    let mut track_bytes = Vec::new();
    let mut current_tick = 0_u32;
    for event in &events {
        let delta = event.abs_tick.saturating_sub(current_tick);
        write_vlq(&mut track_bytes, delta);
        emit_event_payload(&mut track_bytes, &event.kind)?;
        current_tick = event.abs_tick;
    }

    write_vlq(&mut track_bytes, 0);
    track_bytes.extend_from_slice(&[0xff, 0x2f, 0x00]);

    let mut bytes = write_midi_header(ticks_per_quarter, track_bytes.len() as u32);
    bytes.extend_from_slice(&track_bytes);
    Ok(bytes)
}

fn parse_header(bytes: &[u8]) -> io::Result<(Header, ByteReader<'_>)> {
    let mut reader = ByteReader::new(bytes);
    if reader.read_exact(4)? != b"MThd" {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "missing MIDI header chunk",
        ));
    }
    let header_len = reader.read_u32_be()?;
    if header_len < 6 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid MIDI header length",
        ));
    }

    let format = reader.read_u16_be()?;
    let track_count = reader.read_u16_be()?;
    let division = reader.read_u16_be()?;

    if header_len > 6 {
        let extra_len = usize::try_from(header_len - 6).map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidData, "invalid MIDI header padding")
        })?;
        let _ = reader.read_exact(extra_len)?;
    }

    Ok((
        Header {
            format,
            track_count,
            division,
        },
        reader,
    ))
}

fn parse_track_events(
    track_bytes: &[u8],
    seq: &mut usize,
    output: &mut Vec<TimedEvent>,
) -> io::Result<()> {
    let mut reader = ByteReader::new(track_bytes);
    let mut abs_tick = 0_u32;
    let mut running_status: Option<u8> = None;

    while reader.remaining() > 0 {
        let delta = read_vlq(&mut reader)?;
        abs_tick = abs_tick.checked_add(delta).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "MIDI tick count overflow")
        })?;

        let first = reader.read_u8()?;
        let status = if first & 0x80 != 0 {
            first
        } else {
            running_status.ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "missing MIDI running status")
            })?
        };

        let read_data_byte = |first_data: u8, reader: &mut ByteReader<'_>| -> io::Result<u8> {
            if first & 0x80 != 0 {
                reader.read_u8()
            } else {
                Ok(first_data)
            }
        };

        match status {
            0x80..=0x8f => {
                let channel = status & 0x0f;
                let pitch = read_data_byte(first, &mut reader)?;
                let velocity = reader.read_u8()?;
                running_status = Some(status);
                push_timed_event(
                    output,
                    seq,
                    abs_tick,
                    1,
                    TimedEventKind::NoteOff {
                        channel,
                        pitch,
                        velocity,
                    },
                );
            }
            0x90..=0x9f => {
                let channel = status & 0x0f;
                let pitch = read_data_byte(first, &mut reader)?;
                let velocity = reader.read_u8()?;
                running_status = Some(status);
                if velocity == 0 {
                    push_timed_event(
                        output,
                        seq,
                        abs_tick,
                        1,
                        TimedEventKind::NoteOff {
                            channel,
                            pitch,
                            velocity: 64,
                        },
                    );
                } else {
                    push_timed_event(
                        output,
                        seq,
                        abs_tick,
                        2,
                        TimedEventKind::NoteOn {
                            channel,
                            pitch,
                            velocity,
                        },
                    );
                }
            }
            0xa0..=0xaf | 0xb0..=0xbf | 0xe0..=0xef => {
                let _ = read_data_byte(first, &mut reader)?;
                let _ = reader.read_u8()?;
                running_status = Some(status);
            }
            0xc0..=0xcf => {
                let channel = status & 0x0f;
                let program = read_data_byte(first, &mut reader)?;
                running_status = Some(status);
                push_timed_event(
                    output,
                    seq,
                    abs_tick,
                    0,
                    TimedEventKind::ProgramChange { channel, program },
                );
            }
            0xd0..=0xdf => {
                let _ = read_data_byte(first, &mut reader)?;
                running_status = Some(status);
            }
            0xf0 | 0xf7 => {
                let len = read_vlq(&mut reader)? as usize;
                let _ = reader.read_exact(len)?;
            }
            0xff => {
                let meta_type = reader.read_u8()?;
                let len = read_vlq(&mut reader)? as usize;
                let body = reader.read_exact(len)?;
                match meta_type {
                    0x2f => break,
                    0x51 if body.len() == 3 => {
                        let micros = (u32::from(body[0]) << 16)
                            | (u32::from(body[1]) << 8)
                            | u32::from(body[2]);
                        push_timed_event(output, seq, abs_tick, 0, TimedEventKind::Tempo(micros));
                    }
                    0x58 if body.len() >= 2 => {
                        let numerator = body[0];
                        let denominator = 1_u8.checked_shl(u32::from(body[1])).unwrap_or(0);
                        if denominator != 0 {
                            push_timed_event(
                                output,
                                seq,
                                abs_tick,
                                0,
                                TimedEventKind::TimeSignature {
                                    numerator,
                                    denominator,
                                },
                            );
                        }
                    }
                    0x59 if body.len() >= 2 => {
                        let fifths = body[0] as i8;
                        let minor = body[1] != 0;
                        push_timed_event(
                            output,
                            seq,
                            abs_tick,
                            0,
                            TimedEventKind::KeySignature { fifths, minor },
                        );
                    }
                    _ => {}
                }
            }
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("unsupported MIDI status byte 0x{status:02x}"),
                ));
            }
        }
    }

    Ok(())
}

fn parse_track_chunks(
    bytes: &[u8],
    header: Header,
    reader: &mut ByteReader<'_>,
) -> io::Result<Vec<TimedEvent>> {
    let mut events = Vec::new();
    let mut seq = 0_usize;

    for _ in 0..header.track_count {
        if reader.read_exact(4)? != b"MTrk" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "missing MIDI track chunk",
            ));
        }
        let track_len = reader.read_u32_be()? as usize;
        let track_bytes = reader.read_exact(track_len)?;
        parse_track_events(track_bytes, &mut seq, &mut events)?;
    }

    if reader.pos != bytes.len() {
        let _ = reader.read_exact(bytes.len() - reader.pos)?;
    }

    Ok(events)
}

fn current_meta_value<T: Copy>(states: &[MetaState<T>], tick: u32, default: T) -> T {
    let mut current = default;
    for state in states {
        if state.tick > tick {
            break;
        }
        current = state.value;
    }
    current
}

fn build_measures(
    total_ticks: u32,
    ticks_per_sixteenth: u32,
    key_changes: &[MetaState<Key>],
    time_signature_changes: &[MetaState<TimeSignature>],
    tempo_changes: &[MetaState<u32>],
    default_key: Key,
    default_time_signature: TimeSignature,
    default_ms_per_tick: u32,
) -> Vec<Measure> {
    if total_ticks == 0 {
        return Vec::new();
    }

    let mut measures = Vec::new();
    let mut measure_start = 0_u32;

    while measure_start < total_ticks {
        let key = current_meta_value(key_changes, measure_start, default_key);
        let time_signature = current_meta_value(
            time_signature_changes,
            measure_start,
            default_time_signature,
        );
        let ms_per_tick = current_meta_value(tempo_changes, measure_start, default_ms_per_tick);
        let measure_len = ticks_per_measure(time_signature, ticks_per_sixteenth).max(1);

        measures.push(Measure::new(key, time_signature, None, ms_per_tick));
        measure_start = measure_start.saturating_add(measure_len);
    }

    measures
}

fn parse_smf(bytes: &[u8]) -> io::Result<Composition> {
    let (header, mut reader) = parse_header(bytes)?;
    if header.format > 1 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("unsupported MIDI format {}", header.format),
        ));
    }
    if header.division & 0x8000 != 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "SMPTE MIDI timing is not supported",
        ));
    }

    let division = u32::from(header.division);
    let quarter_ticks = lcm(division, 4);
    let ticks_per_sixteenth = (quarter_ticks / 4).max(1);
    let tick_scale = (quarter_ticks / division).max(1);

    let mut events = parse_track_chunks(bytes, header, &mut reader)?;
    events.sort_by_key(|event| (event.abs_tick, event.priority, event.seq));

    let default_key = Key::new(NoteLetter::C, SharpFlat::Natural, MajorMinor::Major);
    let default_time_signature = TimeSignature::new(4, 4);
    let default_micros_per_quarter = 500_000_u32;

    let mut current_programs = [None; 16];
    let mut active_notes: HashMap<(u8, u8), VecDeque<ActiveNote>> = HashMap::new();
    let mut imported_notes = Vec::new();
    let mut tempo_changes = Vec::new();
    let mut time_signature_changes = Vec::new();
    let mut key_changes = Vec::new();
    let mut max_tick_seen = 0_u32;

    for event in &events {
        max_tick_seen = max_tick_seen.max(event.abs_tick);
        match event.kind {
            TimedEventKind::ProgramChange { channel, program } => {
                current_programs[channel as usize] = Some(program);
            }
            TimedEventKind::Tempo(micros_per_quarter) => {
                tempo_changes.push(MetaState {
                    tick: event.abs_tick.saturating_mul(tick_scale),
                    value: micros_per_quarter_to_ms_per_tick(micros_per_quarter, quarter_ticks),
                });
            }
            TimedEventKind::TimeSignature {
                numerator,
                denominator,
            } => {
                time_signature_changes.push(MetaState {
                    tick: event.abs_tick.saturating_mul(tick_scale),
                    value: TimeSignature::new(numerator, denominator),
                });
            }
            TimedEventKind::KeySignature { fifths, minor } => {
                key_changes.push(MetaState {
                    tick: event.abs_tick.saturating_mul(tick_scale),
                    value: fifths_to_key(
                        fifths,
                        if minor {
                            MajorMinor::Minor
                        } else {
                            MajorMinor::Major
                        },
                    ),
                });
            }
            TimedEventKind::NoteOn {
                channel,
                pitch,
                velocity,
            } => {
                active_notes
                    .entry((channel, pitch))
                    .or_default()
                    .push_back(ActiveNote {
                        start_tick: event.abs_tick,
                        velocity,
                        program: current_programs[channel as usize],
                    });
            }
            TimedEventKind::NoteOff { channel, pitch, .. } => {
                if let Some(queue) = active_notes.get_mut(&(channel, pitch)) {
                    if let Some(active_note) = queue.pop_front() {
                        imported_notes.push(ImportedNote {
                            start_tick: active_note.start_tick.saturating_mul(tick_scale),
                            end_tick: event.abs_tick.saturating_mul(tick_scale),
                            channel,
                            pitch,
                            velocity: active_note.velocity,
                            program: active_note.program,
                        });
                    }
                    if queue.is_empty() {
                        active_notes.remove(&(channel, pitch));
                    }
                }
            }
        }
    }

    for ((channel, pitch), mut queue) in active_notes {
        while let Some(active_note) = queue.pop_front() {
            imported_notes.push(ImportedNote {
                start_tick: active_note.start_tick.saturating_mul(tick_scale),
                end_tick: max_tick_seen
                    .max(active_note.start_tick + 1)
                    .saturating_mul(tick_scale),
                channel,
                pitch,
                velocity: active_note.velocity,
                program: active_note.program,
            });
        }
    }

    imported_notes.sort_by_key(|note| (note.start_tick, note.pitch, note.channel));

    let key = key_changes
        .first()
        .map(|state| state.value)
        .unwrap_or(default_key);
    let ms_per_tick = tempo_changes
        .first()
        .map(|state| state.value)
        .unwrap_or_else(|| {
            micros_per_quarter_to_ms_per_tick(default_micros_per_quarter, quarter_ticks)
        });

    let mut instruments = Vec::new();
    for note in &imported_notes {
        let instrument = program_to_instrument(note.program.unwrap_or(0), note.channel);
        if !instruments.contains(&instrument) {
            instruments.push(instrument);
        }
    }
    if instruments.is_empty() {
        instruments.push(Instrument::Piano);
    }

    let mut composition = Composition::new(
        ticks_per_sixteenth,
        ms_per_tick,
        key,
        Temperament::Even,
        instruments,
    );

    let mut last_note_end = 0_u32;
    for note in imported_notes {
        let active_key = current_meta_value(&key_changes, note.start_tick, key);
        let duration = note.end_tick.saturating_sub(note.start_tick).max(1);
        let start_tick = note.start_tick as usize;
        while composition.notes_by_tick.len() <= start_tick {
            composition
                .notes_by_tick
                .push(NotesStartingThisTick::empty());
        }

        composition.notes_by_tick[start_tick]
            .notes
            .push(NotePlayed {
                note: midi_pitch_to_note(note.pitch, active_key),
                engraving: NoteDurationGeneral::Ticks(duration),
                amplitude: velocity_to_amplitude(note.velocity),
                staff: None,
                voice: None,
            });
        last_note_end = last_note_end.max(note.end_tick);
    }

    composition.measures = build_measures(
        last_note_end,
        ticks_per_sixteenth,
        &key_changes,
        &time_signature_changes,
        &tempo_changes,
        key,
        default_time_signature,
        ms_per_tick,
    );

    Ok(composition)
}

pub fn write_midi(comp: &Composition, path: &Path) -> io::Result<()> {
    let bytes = composition_to_smf_bytes(comp)?;
    fs::write(path, bytes)
}

pub fn read_midi(path: &Path) -> io::Result<Composition> {
    let bytes = fs::read(path)?;
    parse_smf(&bytes)
}

#[cfg(test)]
mod tests {
    use super::{composition_to_smf_bytes, parse_smf};
    use crate::{
        composition::{Composition, NotesStartingThisTick},
        instrument::Instrument,
        key_scale::{Key, MajorMinor, SharpFlat},
        measure::{Measure, TimeSignature},
        note::{Note, NoteDurationGeneral, NoteLetter, NotePlayed},
        overtones::Temperament,
    };

    fn push_note(
        comp: &mut Composition,
        tick: usize,
        note: Note,
        duration: NoteDurationGeneral,
        amplitude: f32,
    ) {
        while comp.notes_by_tick.len() <= tick {
            comp.notes_by_tick.push(NotesStartingThisTick::empty());
        }
        comp.notes_by_tick[tick].notes.push(NotePlayed {
            note,
            engraving: duration,
            amplitude,
            staff: None,
            voice: None,
        });
    }

    #[test]
    fn midi_round_trip_preserves_basic_composition_data() {
        let key = Key::new(NoteLetter::G, SharpFlat::Natural, MajorMinor::Major);
        let mut comp = Composition::new(1, 125, key, Temperament::Even, vec![Instrument::Guitar]);
        comp.measures = vec![
            Measure::new(key, TimeSignature::new(4, 4), None, 125),
            Measure::new(key, TimeSignature::new(4, 4), None, 125),
        ];

        push_note(
            &mut comp,
            0,
            Note::new(NoteLetter::G, Some(SharpFlat::Natural), 4),
            NoteDurationGeneral::Traditional(crate::note::NoteEngraving::Quarter),
            0.75,
        );
        push_note(
            &mut comp,
            4,
            Note::new(NoteLetter::F, None, 4),
            NoteDurationGeneral::Traditional(crate::note::NoteEngraving::Quarter),
            0.5,
        );
        push_note(
            &mut comp,
            8,
            Note::new(NoteLetter::D, Some(SharpFlat::Natural), 5),
            NoteDurationGeneral::Ticks(2),
            0.9,
        );

        let bytes = composition_to_smf_bytes(&comp).unwrap();
        let parsed = parse_smf(&bytes).unwrap();

        assert_eq!(parsed.ticks_per_sixteenth_note, 1);
        assert_eq!(parsed.ms_per_tick, 125);
        assert_eq!(parsed.key, key);
        assert_eq!(parsed.instruments.len(), 1);
        assert!(parsed.instruments[0] == Instrument::Guitar);
        assert_eq!(parsed.measures.len(), 1);

        assert_eq!(parsed.notes_by_tick[0].notes.len(), 1);
        assert_eq!(parsed.notes_by_tick[0].notes[0].note.letter, NoteLetter::G);
        assert_eq!(
            parsed.notes_by_tick[0].notes[0].duration,
            NoteDurationGeneral::Ticks(4)
        );

        assert_eq!(parsed.notes_by_tick[4].notes[0].note.letter, NoteLetter::F);
        assert_eq!(
            parsed.notes_by_tick[4].notes[0].note.sharp_flat,
            Some(SharpFlat::Sharp)
        );
        assert_eq!(
            parsed.notes_by_tick[8].notes[0].duration,
            NoteDurationGeneral::Ticks(2)
        );
    }

    #[test]
    fn midi_parser_supports_running_status_and_note_on_zero_for_note_off() {
        let track = vec![
            0x00, 0xff, 0x51, 0x03, 0x07, 0xa1, 0x20, // tempo
            0x00, 0x90, 60, 100, // note on
            0x04, 60, 0, // running-status note on velocity 0 => note off
            0x00, 0xff, 0x2f, 0x00, // end of track
        ];

        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"MThd");
        bytes.extend_from_slice(&6_u32.to_be_bytes());
        bytes.extend_from_slice(&0_u16.to_be_bytes());
        bytes.extend_from_slice(&1_u16.to_be_bytes());
        bytes.extend_from_slice(&4_u16.to_be_bytes());
        bytes.extend_from_slice(b"MTrk");
        bytes.extend_from_slice(&(track.len() as u32).to_be_bytes());
        bytes.extend_from_slice(&track);

        let comp = parse_smf(&bytes).unwrap();

        assert_eq!(comp.ticks_per_sixteenth_note, 1);
        assert_eq!(comp.ms_per_tick, 125);
        assert_eq!(comp.notes_by_tick.len(), 1);
        assert_eq!(comp.notes_by_tick[0].notes.len(), 1);
        assert_eq!(comp.notes_by_tick[0].notes[0].note.letter, NoteLetter::C);
        assert_eq!(
            comp.notes_by_tick[0].notes[0].duration,
            NoteDurationGeneral::Ticks(4)
        );
    }
}
