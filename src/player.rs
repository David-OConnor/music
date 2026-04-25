//! For playing music.

use std::{f32::consts::PI, io, num::NonZero};

use rodio::{DeviceSinkBuilder, Player, buffer::SamplesBuffer, nz};

use crate::composition::Composition;

const SAMPLE_RATE: u32 = 44_100;

/// Fraction of each note's duration used for linear fade-in and fade-out.
const FADE_FRACTION: f32 = 0.08;

pub fn play(composition: &Composition) -> io::Result<()> {
    let tick_s = composition.ms_per_tick as f32 / 1_000.0;

    // Determine total buffer length from the last sample any note will occupy.
    let total_ticks = {
        let mut v: usize = 0;

        for (tick_idx, slot) in composition.notes_by_tick.iter().enumerate() {
            for note in &slot.notes {
                let dur_ticks = note
                    .duration
                    .get_ticks(composition.ticks_per_sixteenth_note)
                    .unwrap_or(1) as usize;

                v = v.max(tick_idx + dur_ticks);
            }
        }
        v
    };

    if total_ticks == 0 {
        return Ok(());
    }

    let total_samples = (total_ticks as f32 * tick_s * SAMPLE_RATE as f32).ceil() as usize;
    let mut buf = vec![0.; total_samples];

    for (tick_idx, slot) in composition.notes_by_tick.iter().enumerate() {
        let start = (tick_idx as f32 * tick_s * SAMPLE_RATE as f32) as usize;

        for note in &slot.notes {
            let freq = note.frequency(composition.key, composition.temperament);

            let dur_ticks = note
                .duration
                .get_ticks(composition.ticks_per_sixteenth_note)?;

            let n = ((dur_ticks as f32 * tick_s) * SAMPLE_RATE as f32) as usize;
            let fade = ((n as f32 * FADE_FRACTION) as usize).max(1);

            for i in 0..n {
                let idx = start + i;
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
        }
    }

    // Normalize to [-1, 1] if any mixing pushed samples above that.
    let peak = buf.iter().map(|s| s.abs()).fold(0., f32::max);
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
