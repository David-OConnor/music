//! For converting [`Composition`] values to Standard MIDI Files (SMF), and reading them back.

use std::{
    collections::{HashMap, VecDeque},
    fs, io,
    path::Path,
};

use crate::{
    composition::Composition,
    instrument::Instrument,
    key_scale::{Key, MajorMinor, SharpFlat},
    measure::{Measure, TimeSignature},
    note::{Note, NoteEngraving, NoteLetter, NotePlayed},
    overtones::Temperament,
};

#[derive(Clone, Debug)]
struct TimedEvent {
    abs_tick: u32,
    track: usize,
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
    track: usize,
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

#[derive(Clone, Debug)]
struct AssignedImportedNote {
    note: ImportedNote,
    voice: usize,
}

#[derive(Clone)]
struct MeasureSpan {
    start_tick: u32,
    end_tick: u32,
    measure: Measure,
}

#[derive(Clone)]
struct NoteSegment {
    start_in_measure: u32,
    duration: u32,
    note: Note,
    amplitude: f32,
    voice: usize,
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

fn write_smf_header(format: u16, track_count: u16, division: u16) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(14);
    bytes.extend_from_slice(b"MThd");
    bytes.extend_from_slice(&6_u32.to_be_bytes());
    bytes.extend_from_slice(&format.to_be_bytes());
    bytes.extend_from_slice(&track_count.to_be_bytes());
    bytes.extend_from_slice(&division.to_be_bytes());
    bytes
}

fn wrap_track(track_bytes: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(8 + track_bytes.len());
    bytes.extend_from_slice(b"MTrk");
    bytes.extend_from_slice(&(track_bytes.len() as u32).to_be_bytes());
    bytes.extend_from_slice(track_bytes);
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

fn ticks_per_measure(time_signature: TimeSignature, ppq: u32) -> u32 {
    ppq * 4 * u32::from(time_signature.numerator) / u32::from(time_signature.denominator)
}

fn bpm_to_micros_per_quarter(bpm: u16) -> io::Result<u32> {
    if bpm == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "tempo must be greater than zero",
        ));
    }
    Ok((60_000_000_u32 / u32::from(bpm)).max(1))
}

fn micros_per_quarter_to_bpm(micros_per_quarter: u32) -> u16 {
    if micros_per_quarter == 0 {
        120
    } else {
        (60_000_000_u32 / micros_per_quarter)
            .clamp(1, u32::from(u16::MAX))
            .try_into()
            .unwrap_or(120)
    }
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
            if *denominator == 0 || !denominator.is_power_of_two() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("unsupported time signature denominator {denominator} for MIDI"),
                ));
            }
            bytes.extend_from_slice(&[
                0xff,
                0x58,
                0x04,
                *numerator,
                denominator.trailing_zeros() as u8,
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
    track: usize,
    priority: u8,
    kind: TimedEventKind,
) {
    events.push(TimedEvent {
        abs_tick,
        track,
        priority,
        seq: *seq,
        kind,
    });
    *seq += 1;
}

fn serialize_track_events(events: &mut Vec<TimedEvent>) -> io::Result<Vec<u8>> {
    events.sort_by_key(|event| (event.abs_tick, event.priority, event.track, event.seq));

    let mut track_bytes = Vec::new();
    let mut current_tick = 0_u32;
    for event in events.iter() {
        let delta = event.abs_tick.saturating_sub(current_tick);
        write_vlq(&mut track_bytes, delta);
        emit_event_payload(&mut track_bytes, &event.kind)?;
        current_tick = event.abs_tick;
    }

    write_vlq(&mut track_bytes, 0);
    track_bytes.extend_from_slice(&[0xff, 0x2f, 0x00]);
    Ok(track_bytes)
}

fn all_measures(comp: &Composition) -> impl Iterator<Item = &Measure> {
    comp.measures_by_part
        .iter()
        .flat_map(|(_, measures)| measures.iter())
}

fn choose_ppq(comp: &Composition) -> io::Result<u16> {
    let ppq = all_measures(comp)
        .map(|measure| u32::from(measure.divisions))
        .fold(1_u32, lcm)
        .max(1);

    u16::try_from(ppq).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "combined measure divisions exceed MIDI header capacity",
        )
    })
}

fn scaled_ticks(raw_ticks: u32, measure_divisions: u16, ppq: u16) -> u32 {
    raw_ticks * u32::from(ppq) / u32::from(measure_divisions)
}

fn reference_measures(comp: &Composition) -> Option<&[Measure]> {
    comp.measures_by_part
        .iter()
        .find_map(|(_, measures)| (!measures.is_empty()).then_some(measures.as_slice()))
}

fn build_meta_track(comp: &Composition, ppq: u16) -> io::Result<Vec<TimedEvent>> {
    let mut events = Vec::new();
    let mut seq = 0_usize;

    if let Some(measures) = reference_measures(comp) {
        let mut tick = 0_u32;
        let mut last_key = None;
        let mut last_time_signature = None;
        let mut last_tempo = None;

        for measure in measures {
            if last_key != Some(measure.key) {
                push_timed_event(
                    &mut events,
                    &mut seq,
                    tick,
                    0,
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
                    &mut events,
                    &mut seq,
                    tick,
                    0,
                    0,
                    TimedEventKind::TimeSignature {
                        numerator: measure.time_signature.numerator,
                        denominator: measure.time_signature.denominator,
                    },
                );
                last_time_signature = Some(measure.time_signature);
            }

            if last_tempo != Some(measure.tempo) {
                push_timed_event(
                    &mut events,
                    &mut seq,
                    tick,
                    0,
                    0,
                    TimedEventKind::Tempo(bpm_to_micros_per_quarter(measure.tempo.max(1))?),
                );
                last_tempo = Some(measure.tempo);
            }

            tick = tick.saturating_add(scaled_ticks(
                measure.total_divisions(),
                measure.divisions,
                ppq,
            ));
        }
    } else {
        push_timed_event(
            &mut events,
            &mut seq,
            0,
            0,
            0,
            TimedEventKind::Tempo(bpm_to_micros_per_quarter(120)?),
        );
        push_timed_event(
            &mut events,
            &mut seq,
            0,
            0,
            0,
            TimedEventKind::TimeSignature {
                numerator: 4,
                denominator: 4,
            },
        );
        push_timed_event(
            &mut events,
            &mut seq,
            0,
            0,
            0,
            TimedEventKind::KeySignature {
                fifths: 0,
                minor: false,
            },
        );
    }

    Ok(events)
}

fn assign_channels(comp: &Composition) -> io::Result<Vec<u8>> {
    let mut channels = Vec::with_capacity(comp.measures_by_part.len());
    let mut next_channel = 0_u8;

    for (instrument, _) in &comp.measures_by_part {
        if *instrument == Instrument::Drums {
            channels.push(9);
            continue;
        }

        while next_channel == 9 || channels.contains(&next_channel) {
            next_channel = next_channel.saturating_add(1);
        }

        if next_channel > 15 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "too many non-drum parts for distinct MIDI channels",
            ));
        }

        channels.push(next_channel);
        next_channel = next_channel.saturating_add(1);
    }

    Ok(channels)
}

fn build_part_track(
    instr: Instrument,
    measures: &[Measure],
    channel: u8,
    ppq: u16,
) -> io::Result<Vec<TimedEvent>> {
    let mut events = Vec::new();
    let mut seq = 0_usize;

    if let Some(program) = instrument_to_program(instr) {
        push_timed_event(
            &mut events,
            &mut seq,
            0,
            0,
            0,
            TimedEventKind::ProgramChange { channel, program },
        );
    }

    let mut measure_start = 0_u32;
    for measure in measures {
        for voice_notes in &measure.notes {
            let mut voice_tick = measure_start;
            for note in voice_notes {
                let duration =
                    scaled_ticks(u32::from(note.duration).max(1), measure.divisions, ppq);
                if !note.is_rest() {
                    let pitch = note_to_midi_pitch(&note.note, measure.key)?;
                    let velocity = amplitude_to_velocity(note.amplitude);
                    push_timed_event(
                        &mut events,
                        &mut seq,
                        voice_tick,
                        0,
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
                        voice_tick.saturating_add(duration),
                        0,
                        1,
                        TimedEventKind::NoteOff {
                            channel,
                            pitch,
                            velocity: 64,
                        },
                    );
                }
                voice_tick = voice_tick.saturating_add(duration);
            }
        }

        measure_start = measure_start.saturating_add(scaled_ticks(
            measure.total_divisions(),
            measure.divisions,
            ppq,
        ));
    }

    Ok(events)
}

fn composition_to_smf_bytes(comp: &Composition) -> io::Result<Vec<u8>> {
    let ppq = choose_ppq(comp)?;
    let channels = assign_channels(comp)?;
    let mut track_payloads = Vec::new();

    let mut meta_events = build_meta_track(comp, ppq)?;
    track_payloads.push(serialize_track_events(&mut meta_events)?);

    for ((instrument, measures), channel) in comp.measures_by_part.iter().zip(channels) {
        let mut part_events = build_part_track(*instrument, measures, channel, ppq)?;
        track_payloads.push(serialize_track_events(&mut part_events)?);
    }

    let format = if track_payloads.len() > 1 { 1 } else { 0 };
    let mut bytes = write_smf_header(format, track_payloads.len() as u16, ppq);
    for track in &track_payloads {
        bytes.extend_from_slice(&wrap_track(track));
    }
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
    track_index: usize,
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
                    track_index,
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
                        track_index,
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
                        track_index,
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
                    track_index,
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
                        push_timed_event(
                            output,
                            seq,
                            abs_tick,
                            track_index,
                            0,
                            TimedEventKind::Tempo(micros),
                        );
                    }
                    0x58 if body.len() >= 2 => {
                        let numerator = body[0];
                        let denominator = 1_u8.checked_shl(u32::from(body[1])).unwrap_or(0);
                        if denominator != 0 {
                            push_timed_event(
                                output,
                                seq,
                                abs_tick,
                                track_index,
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
                            track_index,
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

    for track_index in 0..header.track_count as usize {
        if reader.read_exact(4)? != b"MTrk" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "missing MIDI track chunk",
            ));
        }
        let track_len = reader.read_u32_be()? as usize;
        let track_bytes = reader.read_exact(track_len)?;
        parse_track_events(track_index, track_bytes, &mut seq, &mut events)?;
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

fn build_measure_spans(
    total_ticks: u32,
    ppq: u16,
    key_changes: &[MetaState<Key>],
    time_signature_changes: &[MetaState<TimeSignature>],
    tempo_changes: &[MetaState<u16>],
    default_key: Key,
    default_time_signature: TimeSignature,
    default_tempo: u16,
) -> Vec<MeasureSpan> {
    if total_ticks == 0 {
        return Vec::new();
    }

    let mut spans = Vec::new();
    let mut measure_start = 0_u32;

    while measure_start < total_ticks {
        let key = current_meta_value(key_changes, measure_start, default_key);
        let time_signature = current_meta_value(
            time_signature_changes,
            measure_start,
            default_time_signature,
        );
        let tempo = current_meta_value(tempo_changes, measure_start, default_tempo);
        let measure_len = ticks_per_measure(time_signature, u32::from(ppq)).max(1);

        let mut measure = Measure::new(key, time_signature, None, tempo);
        measure.divisions = ppq;

        spans.push(MeasureSpan {
            start_tick: measure_start,
            end_tick: measure_start.saturating_add(measure_len),
            measure,
        });
        measure_start = measure_start.saturating_add(measure_len);
    }

    spans
}

fn assign_voices(notes: &mut [ImportedNote]) -> Vec<AssignedImportedNote> {
    notes.sort_by_key(|note| (note.start_tick, note.end_tick, note.pitch));

    let mut voice_end_ticks: Vec<u32> = Vec::new();
    let mut assigned = Vec::with_capacity(notes.len());

    for note in notes.iter().cloned() {
        let mut voice = None;
        for (idx, end_tick) in voice_end_ticks.iter_mut().enumerate() {
            if note.start_tick >= *end_tick {
                *end_tick = note.end_tick;
                voice = Some(idx);
                break;
            }
        }

        let voice = voice.unwrap_or_else(|| {
            voice_end_ticks.push(note.end_tick);
            voice_end_ticks.len() - 1
        });

        assigned.push(AssignedImportedNote { note, voice });
    }

    assigned
}

fn find_measure_index(spans: &[MeasureSpan], tick: u32) -> Option<usize> {
    spans.iter().position(|span| tick < span.end_tick)
}

fn split_note_segments(
    assigned_note: &AssignedImportedNote,
    spans: &[MeasureSpan],
    key_changes: &[MetaState<Key>],
    default_key: Key,
) -> Vec<(usize, NoteSegment)> {
    let mut segments = Vec::new();
    let mut segment_start = assigned_note.note.start_tick;
    let note_end = assigned_note.note.end_tick.max(segment_start + 1);

    while segment_start < note_end {
        let Some(measure_idx) = find_measure_index(spans, segment_start) else {
            break;
        };
        let span = &spans[measure_idx];
        let segment_end = note_end.min(span.end_tick);
        let duration = segment_end.saturating_sub(segment_start);
        let key = current_meta_value(key_changes, assigned_note.note.start_tick, default_key);

        segments.push((
            measure_idx,
            NoteSegment {
                start_in_measure: segment_start.saturating_sub(span.start_tick),
                duration,
                note: midi_pitch_to_note(assigned_note.note.pitch, key),
                amplitude: velocity_to_amplitude(assigned_note.note.velocity),
                voice: assigned_note.voice,
            },
        ));

        segment_start = segment_end;
    }

    segments
}

fn rest_note(duration: u32, divisions: u16, voice: usize) -> NotePlayed {
    let duration_u16 = duration.min(u32::from(u16::MAX)) as u16;
    NotePlayed {
        note: Note::new(NoteLetter::C, None, 4),
        engraving: NoteEngraving::from_duration_ticks(duration_u16, divisions),
        duration: duration_u16,
        amplitude: 0.0,
        staff: None,
        voice,
    }
}

fn sounded_note(segment: &NoteSegment, divisions: u16) -> NotePlayed {
    let duration_u16 = segment.duration.min(u32::from(u16::MAX)) as u16;
    NotePlayed {
        note: segment.note.clone(),
        engraving: NoteEngraving::from_duration_ticks(duration_u16, divisions),
        duration: duration_u16,
        amplitude: segment.amplitude,
        staff: None,
        voice: segment.voice,
    }
}

fn build_part_measures(
    assigned_notes: &[AssignedImportedNote],
    spans: &[MeasureSpan],
    key_changes: &[MetaState<Key>],
    default_key: Key,
) -> Vec<Measure> {
    if spans.is_empty() {
        return Vec::new();
    }

    let voice_count = assigned_notes
        .iter()
        .map(|note| note.voice + 1)
        .max()
        .unwrap_or(0);

    let mut segment_grid: Vec<Vec<Vec<NoteSegment>>> = spans
        .iter()
        .map(|_| vec![Vec::new(); voice_count])
        .collect();

    for assigned_note in assigned_notes {
        for (measure_idx, segment) in
            split_note_segments(assigned_note, spans, key_changes, default_key)
        {
            segment_grid[measure_idx][segment.voice].push(segment);
        }
    }

    spans
        .iter()
        .enumerate()
        .map(|(measure_idx, span)| {
            let mut measure = span.measure.clone();
            let mut voices = vec![Vec::new(); voice_count];

            for voice_idx in 0..voice_count {
                let mut segments = segment_grid[measure_idx][voice_idx].clone();
                segments.sort_by_key(|segment| (segment.start_in_measure, segment.duration));

                let mut cursor = 0_u32;
                for segment in segments {
                    if segment.start_in_measure > cursor {
                        voices[voice_idx].push(rest_note(
                            segment.start_in_measure - cursor,
                            measure.divisions,
                            voice_idx,
                        ));
                    }

                    voices[voice_idx].push(sounded_note(&segment, measure.divisions));
                    cursor = segment.start_in_measure.saturating_add(segment.duration);
                }
            }

            measure.notes = voices;
            measure
        })
        .collect()
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

    let ppq = header.division.max(1);
    let mut events = parse_track_chunks(bytes, header, &mut reader)?;
    events.sort_by_key(|event| (event.abs_tick, event.priority, event.track, event.seq));

    let default_key = Key::new(NoteLetter::C, SharpFlat::Natural, MajorMinor::Major);
    let default_time_signature = TimeSignature::new(4, 4);
    let default_tempo = 120_u16;

    let mut current_programs: HashMap<(usize, u8), Option<u8>> = HashMap::new();
    let mut active_notes: HashMap<(usize, u8, u8), VecDeque<ActiveNote>> = HashMap::new();
    let mut imported_notes = Vec::new();
    let mut tempo_changes = Vec::new();
    let mut time_signature_changes = Vec::new();
    let mut key_changes = Vec::new();
    let mut max_tick_seen = 0_u32;

    for event in &events {
        max_tick_seen = max_tick_seen.max(event.abs_tick);
        match event.kind {
            TimedEventKind::ProgramChange { channel, program } => {
                current_programs.insert((event.track, channel), Some(program));
            }
            TimedEventKind::Tempo(micros_per_quarter) => {
                tempo_changes.push(MetaState {
                    tick: event.abs_tick,
                    value: micros_per_quarter_to_bpm(micros_per_quarter),
                });
            }
            TimedEventKind::TimeSignature {
                numerator,
                denominator,
            } => {
                time_signature_changes.push(MetaState {
                    tick: event.abs_tick,
                    value: TimeSignature::new(numerator, denominator),
                });
            }
            TimedEventKind::KeySignature { fifths, minor } => {
                key_changes.push(MetaState {
                    tick: event.abs_tick,
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
                    .entry((event.track, channel, pitch))
                    .or_default()
                    .push_back(ActiveNote {
                        start_tick: event.abs_tick,
                        velocity,
                        program: current_programs
                            .get(&(event.track, channel))
                            .copied()
                            .unwrap_or(None),
                    });
            }
            TimedEventKind::NoteOff { channel, pitch, .. } => {
                if let Some(queue) = active_notes.get_mut(&(event.track, channel, pitch)) {
                    if let Some(active_note) = queue.pop_front() {
                        imported_notes.push(ImportedNote {
                            start_tick: active_note.start_tick,
                            end_tick: event.abs_tick.max(active_note.start_tick + 1),
                            track: event.track,
                            channel,
                            pitch,
                            velocity: active_note.velocity,
                            program: active_note.program,
                        });
                    }
                    if queue.is_empty() {
                        active_notes.remove(&(event.track, channel, pitch));
                    }
                }
            }
        }
    }

    for ((track, channel, pitch), mut queue) in active_notes {
        while let Some(active_note) = queue.pop_front() {
            imported_notes.push(ImportedNote {
                start_tick: active_note.start_tick,
                end_tick: max_tick_seen.max(active_note.start_tick + 1),
                track,
                channel,
                pitch,
                velocity: active_note.velocity,
                program: active_note.program,
            });
        }
    }

    let spans = build_measure_spans(
        imported_notes
            .iter()
            .map(|note| note.end_tick)
            .max()
            .unwrap_or(0),
        ppq,
        &key_changes,
        &time_signature_changes,
        &tempo_changes,
        default_key,
        default_time_signature,
        default_tempo,
    );

    let mut comp = Composition::new(Temperament::Even, vec![]);
    let mut part_order: Vec<(usize, u8, Option<u8>)> = Vec::new();
    let mut part_notes: HashMap<(usize, u8, Option<u8>), Vec<ImportedNote>> = HashMap::new();

    imported_notes.sort_by_key(|note| (note.track, note.start_tick, note.pitch, note.channel));
    for note in imported_notes {
        let part_key = (note.track, note.channel, note.program);
        if !part_notes.contains_key(&part_key) {
            part_order.push(part_key);
        }
        part_notes.entry(part_key).or_default().push(note);
    }

    for (track, channel, program) in part_order {
        let instrument = program_to_instrument(program.unwrap_or(0), channel);
        let mut notes = part_notes
            .remove(&(track, channel, program))
            .unwrap_or_default();
        let assigned_notes = assign_voices(&mut notes);
        let measures = build_part_measures(&assigned_notes, &spans, &key_changes, default_key);
        comp.measures_by_part.push((instrument, measures));
    }

    Ok(comp)
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
        composition::Composition,
        instrument::Instrument,
        key_scale::{Key, MajorMinor, SharpFlat},
        measure::{Measure, TimeSignature},
        note::{Note, NoteEngraving, NoteLetter, NotePlayed},
        overtones::Temperament,
    };

    fn note(
        letter: NoteLetter,
        sharp_flat: Option<SharpFlat>,
        octave: u8,
        engraving: NoteEngraving,
        divisions: u16,
        voice: usize,
    ) -> NotePlayed {
        NotePlayed {
            note: Note::new(letter, sharp_flat, octave),
            engraving,
            duration: engraving.to_duration_ticks(divisions),
            amplitude: 0.8,
            staff: None,
            voice,
        }
    }

    #[test]
    fn midi_round_trip_preserves_parts_and_note_durations() {
        let key = Key::new(NoteLetter::G, SharpFlat::Natural, MajorMinor::Major);
        let mut guitar_measure = Measure::new(key, TimeSignature::new(4, 4), None, 120);
        guitar_measure.divisions = 4;
        guitar_measure.notes = vec![vec![
            note(
                NoteLetter::G,
                Some(SharpFlat::Natural),
                4,
                NoteEngraving::Quarter,
                4,
                0,
            ),
            note(NoteLetter::F, None, 4, NoteEngraving::Quarter, 4, 0),
        ]];

        let mut bass_measure = Measure::new(key, TimeSignature::new(4, 4), None, 120);
        bass_measure.divisions = 4;
        bass_measure.notes = vec![vec![note(
            NoteLetter::D,
            Some(SharpFlat::Natural),
            3,
            NoteEngraving::Half,
            4,
            0,
        )]];

        let comp = Composition {
            metadata: Default::default(),
            measures_by_part: vec![
                (Instrument::Guitar, vec![guitar_measure]),
                (Instrument::BassGuitar, vec![bass_measure]),
            ],
            temperament: Temperament::Even,
        };

        let bytes = composition_to_smf_bytes(&comp).unwrap();
        let parsed = parse_smf(&bytes).unwrap();

        assert_eq!(parsed.measures_by_part.len(), 2);
        assert_eq!(parsed.measures_by_part[0].0, Instrument::Guitar);
        assert_eq!(parsed.measures_by_part[1].0, Instrument::BassGuitar);

        let guitar_notes = &parsed.measures_by_part[0].1[0].notes[0];
        assert_eq!(guitar_notes.len(), 2);
        assert_eq!(guitar_notes[0].note.letter, NoteLetter::G);
        assert_eq!(guitar_notes[0].duration, 4);
        assert_eq!(guitar_notes[1].note.sharp_flat, Some(SharpFlat::Sharp));

        let bass_notes = &parsed.measures_by_part[1].1[0].notes[0];
        assert_eq!(bass_notes.len(), 1);
        assert_eq!(bass_notes[0].note.letter, NoteLetter::D);
        assert_eq!(bass_notes[0].duration, 8);
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

        assert_eq!(comp.measures_by_part.len(), 1);
        let measure = &comp.measures_by_part[0].1[0];
        assert_eq!(measure.divisions, 4);
        assert_eq!(measure.tempo, 120);
        assert_eq!(measure.notes[0][0].note.letter, NoteLetter::C);
        assert_eq!(measure.notes[0][0].duration, 4);
    }
}
