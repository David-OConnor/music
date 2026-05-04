//! Generation of music, perhaps based on NNs. (Burn?)

use std::io;

use rand::RngExt;

use crate::{
    chord::Chord,
    composition::Composition,
    composition_arch::CompositionComponent,
    instrument::Instrument,
    key_scale::{Key, SharpFlat},
    measure::{Measure, TimeSignature},
    note::{Note, NoteEngraving, NoteLetter, NotePlayed},
    overtones::Temperament,
    percussion::PercussionHit,
    rhythm::RhythmPattern,
};

const AMPLITUDE_PIANO: f32 = 0.3;
const AMPLITUDE_BASS: f32 = 0.4;
const AMPLITUDE_KICK: f32 = 0.7;
const AMPLITUDE_SNARE: f32 = 0.6;
const AMPLITUDE_HIHAT: f32 = 0.3;

/// We use this to create a simple composition from a structure. Example use: Specify a key, time
/// signature, chords and rhythm pattern. This will generate an initial composition which can be
/// exported as MIDI or MusicXML, then used as a baseline for a work in DAW or composition software.
pub struct CompositionGuide {
    // todo: Key and time sig can change; placeholder for now.
    // todo: Likely: Break this into sections; perhaps a Vec of this struct instead of
    // todo something new.
    pub key: Key,
    pub time_sig: TimeSignature,
    pub tempo: u16,
    /// By measure.
    pub chords: Vec<Chord>,
    /// By measure. Must match chord len.
    pub rhythm_pattern: Vec<RhythmPattern>,
    // todo: A/R. Not implemented yet.
    pub comps: Vec<CompositionComponent>,
}

impl CompositionGuide {
    /// Makes a composition with the following (Hard-coded for now) instruments:
    /// - Drums: Plays the most common drum hits, following all rhythm hits, prioritrizing primary,
    /// then secondary, then tertiary. Might use kick, snare, ride, toms as primaries, and accent
    /// with crash etc.
    /// - Piano: Plays the input chord structures exactly, roughly following the primary rhythm beats. This
    /// could also be used for guitar.
    /// - Bass guitar: plays arbitrary notes from the chords, following the primary and secondary
    /// rhythm beats. Perhaps a mix of eigth and sixteenth notes as a baseline, but see the time signature.
    pub fn make_comp(&self) -> io::Result<Composition> {
        if self.chords.len() != self.rhythm_pattern.len() {
            return Err(io::Error::other(
                "Error generating composition: Chords and rhythm pattern \
                must be the same length.",
            ));
        }

        let divisions = 32;

        let mut comp = Composition::new(
            Temperament::WellTempered(self.key),
            vec![Instrument::Piano, Instrument::BassGuitar, Instrument::Drums],
        );

        for measure in self.make_piano_part(divisions) {
            comp.add_measure(Instrument::Piano, measure);
        }
        for measure in self.make_bass_part(divisions) {
            comp.add_measure(Instrument::BassGuitar, measure);
        }
        for measure in self.make_drums_part(divisions) {
            comp.add_measure(Instrument::Drums, measure);
        }

        Ok(comp)
    }

    /// Piano: full chord struck on every primary rhythm hit, sustained until the next primary
    /// hit (or the end of the measure). Each chord tone goes in its own storage voice but shares
    /// the same logical voice index, so MusicXML export groups them as a single chord.
    fn make_piano_part(&self, divisions: u16) -> Vec<Measure> {
        let mut measures = Vec::new();

        for (chord, rhythm) in self.chords.iter().zip(&self.rhythm_pattern) {
            let mut measure =
                Measure::new(self.key, self.time_sig, Some(chord.clone()), self.tempo);
            measure.divisions = divisions;
            let total = measure.total_divisions();

            let primary = rhythm.primary_ticks(total);
            let starts: Vec<u32> = if primary.is_empty() { vec![0] } else { primary };

            for chord_note in chord.notes() {
                let voice = build_sustaining_voice(
                    &starts,
                    &chord_note,
                    total,
                    divisions,
                    0,
                    Some(1),
                    AMPLITUDE_PIANO,
                );
                measure.notes.push(voice);
            }

            measures.push(measure);
        }

        measures
    }

    /// Bass guitar: a single voice that plays one note on every primary AND secondary hit.
    /// Primary hits anchor on the chord root; secondary hits pick a non-root chord tone at random
    /// for movement. Each note sustains until the next bass hit (or end of measure).
    fn make_bass_part(&self, divisions: u16) -> Vec<Measure> {
        let mut rng = rand::rng();
        let mut measures = Vec::new();
        let bass_octave: u8 = 2;

        for (chord, rhythm) in self.chords.iter().zip(&self.rhythm_pattern) {
            let mut measure =
                Measure::new(self.key, self.time_sig, Some(chord.clone()), self.tempo);
            measure.divisions = divisions;
            let total = measure.total_divisions();

            let primary = rhythm.primary_ticks(total);
            let secondary = rhythm.secondary_ticks(total);

            let mut combined = primary.clone();
            for t in &secondary {
                if !combined.contains(t) {
                    combined.push(*t);
                }
            }
            combined.sort_unstable();
            combined.dedup();

            let chord_notes = chord.notes();
            let root_letter = chord_notes[0].letter;
            let root_sf = chord_notes[0].sharp_flat;

            let mut voice_notes: Vec<NotePlayed> = Vec::new();
            let mut cursor = 0u32;

            for (i, &start) in combined.iter().enumerate() {
                if start > cursor {
                    voice_notes.push(rest_note(start - cursor, divisions, 0, None));
                }
                let end = combined.get(i + 1).copied().unwrap_or(total);
                if end <= start {
                    continue;
                }

                let pitch = if primary.contains(&start) || chord_notes.len() < 2 {
                    Note::new(root_letter, root_sf, bass_octave)
                } else {
                    let pick = &chord_notes[rng.random_range(1..chord_notes.len())];
                    Note::new(pick.letter, pick.sharp_flat, bass_octave)
                };

                voice_notes.push(played_note(
                    pitch,
                    end - start,
                    divisions,
                    0,
                    None,
                    AMPLITUDE_BASS,
                ));
                cursor = end;
            }

            if cursor < total {
                voice_notes.push(rest_note(total - cursor, divisions, 0, None));
            }
            if voice_notes.is_empty() {
                voice_notes.push(rest_note(total, divisions, 0, None));
            }

            measure.notes.push(voice_notes);
            measures.push(measure);
        }

        measures
    }

    /// Drums: kick on primary hits, snare on secondary hits (skipping any that coincide with a
    /// primary), closed hi-hat on tertiary hits. Each strike is a short sixteenth-note hit, with
    /// rests filling the rest of the voice. Each percussion goes in its own logical voice.
    fn make_drums_part(&self, divisions: u16) -> Vec<Measure> {
        let mut measures = Vec::new();
        let hit_dur = NoteEngraving::Sixteenth.to_duration_ticks(divisions) as u32;

        for rhythm in &self.rhythm_pattern {
            let mut measure = Measure::new(self.key, self.time_sig, None, self.tempo);
            measure.divisions = divisions;
            let total = measure.total_divisions();

            let primary = rhythm.primary_ticks(total);
            let snare_hits: Vec<u32> = rhythm
                .secondary_ticks(total)
                .into_iter()
                .filter(|t| !primary.contains(t))
                .collect();
            let hat_hits = rhythm.tertiary_ticks(total);

            measure.notes.push(drum_voice(
                &primary,
                PercussionHit::Kick,
                total,
                divisions,
                0,
                hit_dur,
                AMPLITUDE_KICK,
            ));
            measure.notes.push(drum_voice(
                &snare_hits,
                PercussionHit::Snare,
                total,
                divisions,
                1,
                hit_dur,
                AMPLITUDE_SNARE,
            ));
            measure.notes.push(drum_voice(
                &hat_hits,
                PercussionHit::HighhatClosed,
                total,
                divisions,
                2,
                hit_dur,
                AMPLITUDE_HIHAT,
            ));

            measures.push(measure);
        }

        measures
    }
}

/// Builds a single voice that plays `note` from each entry in `starts` until the next entry
/// (or `total_ticks` for the final note). Leading/trailing gaps become rests.
fn build_sustaining_voice(
    starts: &[u32],
    note: &Note,
    total_ticks: u32,
    divisions: u16,
    voice: usize,
    staff: Option<usize>,
    amplitude: f32,
) -> Vec<NotePlayed> {
    let mut out: Vec<NotePlayed> = Vec::new();
    let mut cursor = 0u32;

    for (i, &start) in starts.iter().enumerate() {
        if start > cursor {
            out.push(rest_note(start - cursor, divisions, voice, staff));
        }
        let end = starts.get(i + 1).copied().unwrap_or(total_ticks);
        if end > start {
            out.push(played_note(
                note.clone(),
                end - start,
                divisions,
                voice,
                staff,
                amplitude,
            ));
            cursor = end;
        }
    }

    if cursor < total_ticks {
        out.push(rest_note(total_ticks - cursor, divisions, voice, staff));
    }
    if out.is_empty() {
        out.push(rest_note(total_ticks, divisions, voice, staff));
    }
    out
}

/// Drums-style voice: each hit plays for `hit_dur` ticks (clipped to the measure end), and the
/// remainder of the voice is filled with rests.
fn drum_voice(
    hits: &[u32],
    hit: PercussionHit,
    total_ticks: u32,
    divisions: u16,
    voice: usize,
    hit_dur: u32,
    amplitude: f32,
) -> Vec<NotePlayed> {
    let mut out: Vec<NotePlayed> = Vec::new();
    let mut cursor = 0u32;
    let pitch = hit.midi_note();

    for &t in hits {
        if t < cursor {
            continue;
        }
        if t > cursor {
            out.push(rest_note(t - cursor, divisions, voice, None));
            cursor = t;
        }
        let dur = hit_dur.min(total_ticks.saturating_sub(t));
        if dur == 0 {
            continue;
        }
        out.push(played_note(
            pitch.clone(),
            dur,
            divisions,
            voice,
            None,
            amplitude,
        ));
        cursor = t + dur;
    }

    if cursor < total_ticks {
        out.push(rest_note(total_ticks - cursor, divisions, voice, None));
    }
    if out.is_empty() {
        out.push(rest_note(total_ticks, divisions, voice, None));
    }
    out
}

fn played_note(
    note: Note,
    duration: u32,
    divisions: u16,
    voice: usize,
    staff: Option<usize>,
    amplitude: f32,
) -> NotePlayed {
    let dur = duration.min(u32::from(u16::MAX)) as u16;
    NotePlayed {
        note,
        engraving: NoteEngraving::from_duration_ticks(dur, divisions),
        duration: dur,
        amplitude,
        staff,
        voice,
    }
}

fn rest_note(duration: u32, divisions: u16, voice: usize, staff: Option<usize>) -> NotePlayed {
    let dur = duration.min(u32::from(u16::MAX)) as u16;
    NotePlayed {
        note: Note::new(NoteLetter::C, Some(SharpFlat::Natural), 4),
        engraving: NoteEngraving::from_duration_ticks(dur, divisions),
        duration: dur,
        amplitude: 0.0,
        staff,
        voice,
    }
}
