//! Split long recordings into model-sized windows at quiet points so words
//! are not cut in half at chunk boundaries.

use crate::SAMPLE_RATE;
use std::ops::Range;

/// Analysis frame for finding a quiet cut point (50 ms).
const FRAME: usize = SAMPLE_RATE / 20;
/// How far back from the hard limit we search for a pause (max 5 s).
const MAX_SEARCH: usize = 5 * SAMPLE_RATE;
/// RMS below which a buffer is treated as silence (about -54 dBFS).
const SILENCE_RMS: f32 = 0.002;

fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt()
}

/// True when the whole buffer is effectively silent (or empty).
pub fn is_silent(samples: &[f32]) -> bool {
    rms(samples) < SILENCE_RMS
}

/// Split `pcm` into consecutive, non-overlapping ranges of at most
/// `max_len` samples, preferring to cut at the quietest frame in the final
/// quarter (capped at 5 s) of each window.
pub fn split(pcm: &[f32], max_len: usize) -> Vec<Range<usize>> {
    let max_len = max_len.max(FRAME * 2);
    let mut ranges = Vec::new();
    let mut start = 0;
    while pcm.len() - start > max_len {
        let hard_end = start + max_len;
        let search = (max_len / 4).min(MAX_SEARCH);
        let mut best_end = hard_end;
        let mut best_rms = f32::INFINITY;
        let mut frame_end = hard_end;
        while frame_end >= hard_end - search + FRAME {
            let level = rms(&pcm[frame_end - FRAME..frame_end]);
            if level < best_rms {
                best_rms = level;
                // Cut in the middle of the quiet frame.
                best_end = frame_end - FRAME / 2;
            }
            frame_end -= FRAME;
        }
        ranges.push(start..best_end);
        start = best_end;
    }
    if start < pcm.len() {
        ranges.push(start..pcm.len());
    }
    ranges
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(len: usize) -> Vec<f32> {
        (0..len).map(|i| (i as f32 * 0.05).sin() * 0.5).collect()
    }

    #[test]
    fn short_audio_is_a_single_chunk() {
        let pcm = tone(SAMPLE_RATE);
        assert_eq!(split(&pcm, 10 * SAMPLE_RATE), vec![0..SAMPLE_RATE]);
    }

    #[test]
    fn empty_audio_has_no_chunks() {
        assert!(split(&[], 10 * SAMPLE_RATE).is_empty());
    }

    #[test]
    fn chunks_cover_audio_without_gaps_and_respect_limit() {
        let pcm = tone(95 * SAMPLE_RATE);
        let max = 20 * SAMPLE_RATE;
        let chunks = split(&pcm, max);
        assert_eq!(chunks.first().unwrap().start, 0);
        assert_eq!(chunks.last().unwrap().end, pcm.len());
        for w in chunks.windows(2) {
            assert_eq!(w[0].end, w[1].start);
        }
        assert!(chunks.iter().all(|r| r.len() <= max && !r.is_empty()));
    }

    #[test]
    fn cuts_at_a_pause_near_the_limit() {
        // 20 s of speech-like tone with a 300 ms pause at 18 s.
        let mut pcm = tone(30 * SAMPLE_RATE);
        let pause = 18 * SAMPLE_RATE..18 * SAMPLE_RATE + 3 * SAMPLE_RATE / 10;
        pcm[pause.clone()].iter_mut().for_each(|s| *s = 0.0);
        let chunks = split(&pcm, 20 * SAMPLE_RATE);
        let cut = chunks[0].end;
        assert!(pause.contains(&cut), "cut at {cut}, pause {pause:?}");
    }

    #[test]
    fn silence_detection() {
        assert!(is_silent(&[]));
        assert!(is_silent(&vec![0.0; SAMPLE_RATE]));
        assert!(!is_silent(&tone(SAMPLE_RATE)));
    }
}
