//! For playing music.

use std::{f32::consts::PI, io, num::NonZero};

use rodio::{DeviceSinkBuilder, Player, buffer::SamplesBuffer, nz};

use crate::composition::Composition;

const SAMPLE_RATE: u32 = 44_100;
const FADE_FRACTION: f32 = 0.08;

pub fn play(composition: &Composition) -> io::Result<()> {
    // First pass: compute total buffer size
    let mut total_samples = 0usize;
    for (_, measures) in &composition.measures_by_part {
        let mut measure_start_sample = 0usize;

        for measure in measures {
            let bpm = if measure.tempo > 0 {
                measure.tempo
            } else {
                120
            } as f32;
            let div = measure.divisions as f32;
            let seconds_per_div = 60.0 / (bpm * div);

            for voice in &measure.notes {
                let mut pos = 0u32;
                for note in voice {
                    let start_s = pos as f32 * seconds_per_div;
                    let dur_s = note.duration as f32 * seconds_per_div;
                    let end = measure_start_sample
                        + ((start_s + dur_s) * SAMPLE_RATE as f32).ceil() as usize;
                    total_samples = total_samples.max(end);
                    pos += u32::from(note.duration);
                }
            }

            let measure_divs = measure.total_divisions();
            measure_start_sample +=
                (measure_divs as f32 * seconds_per_div * SAMPLE_RATE as f32).ceil() as usize;
        }
    }

    if total_samples == 0 {
        return Ok(());
    }

    let mut buf = vec![0f32; total_samples];

    // Second pass: fill buffer
    for (_, measures) in &composition.measures_by_part {
        let mut measure_start_sample = 0usize;

        for measure in measures {
            let bpm = if measure.tempo > 0 {
                measure.tempo
            } else {
                120
            } as f32;
            let div = measure.divisions as f32;
            let seconds_per_div = 60.0 / (bpm * div);

            for voice in &measure.notes {
                let mut pos = 0u32;
                for note in voice {
                    if note.is_rest() {
                        pos += u32::from(note.duration);
                        continue;
                    }

                    let start_s = pos as f32 * seconds_per_div;
                    let dur_s = note.duration as f32 * seconds_per_div;
                    let start_sample =
                        measure_start_sample + (start_s * SAMPLE_RATE as f32) as usize;
                    let n = (dur_s * SAMPLE_RATE as f32) as usize;

                    let freq = note.frequency(measure.key, composition.temperament);
                    let fade = ((n as f32 * FADE_FRACTION) as usize).max(1);

                    for i in 0..n {
                        let idx = start_sample + i;
                        if idx >= buf.len() {
                            break;
                        }
                        let envelope = if i < fade {
                            i as f32 / fade as f32
                        } else if i >= n - fade {
                            (n - i) as f32 / fade as f32
                        } else {
                            1.0
                        };
                        let t = i as f32 / SAMPLE_RATE as f32;
                        buf[idx] += note.amplitude * envelope * (2.0 * PI * freq * t).sin();
                    }

                    pos += u32::from(note.duration);
                }
            }

            let measure_divs = measure.total_divisions();
            measure_start_sample +=
                (measure_divs as f32 * seconds_per_div * SAMPLE_RATE as f32).ceil() as usize;
        }
    }

    let peak = buf.iter().map(|s| s.abs()).fold(0f32, f32::max);
    if peak > 1.0 {
        buf.iter_mut().for_each(|s| *s /= peak);
    }

    let mut device_sink =
        DeviceSinkBuilder::open_default_sink().map_err(|e| io::Error::other(e.to_string()))?;
    device_sink.log_on_drop(false);

    let player = Player::connect_new(device_sink.mixer());

    let sample_rate = NonZero::new(SAMPLE_RATE).ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "sample rate must be non-zero")
    })?;
    player.append(SamplesBuffer::new(nz!(1), sample_rate, buf));
    player.sleep_until_end();

    Ok(())
}
