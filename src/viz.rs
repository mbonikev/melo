//! Spectrum analyzer: turns recent PCM samples into smoothed frequency bands.

use std::f32::consts::PI;
use std::sync::Arc;

use rustfft::{num_complex::Complex, Fft, FftPlanner};

const FFT_SIZE: usize = 2048;

pub struct Visualizer {
    fft: Arc<dyn Fft<f32>>,
    scratch: Vec<Complex<f32>>,
    /// Smoothed, auto-gained band levels in 0.0..=1.0.
    pub bands: Vec<f32>,
    peak: f32,
}

impl Visualizer {
    pub fn new(n_bands: usize) -> Self {
        let mut planner = FftPlanner::new();
        Self {
            fft: planner.plan_fft_forward(FFT_SIZE),
            scratch: vec![Complex::new(0.0, 0.0); FFT_SIZE],
            bands: vec![0.0; n_bands],
            peak: 1e-3,
        }
    }

    /// Recompute bands from the latest samples. Bars rise instantly, fall slowly.
    pub fn update(&mut self, samples: &[f32]) {
        if samples.len() < 64 {
            self.decay();
            return;
        }

        let n = FFT_SIZE;
        let start = samples.len().saturating_sub(n);
        let slice = &samples[start..];

        for (i, c) in self.scratch.iter_mut().enumerate() {
            let s = slice.get(i).copied().unwrap_or(0.0);
            // Hann window to reduce spectral leakage.
            let w = 0.5 - 0.5 * ((2.0 * PI * i as f32) / (n as f32 - 1.0)).cos();
            *c = Complex::new(s * w, 0.0);
        }

        self.fft.process(&mut self.scratch);

        let half = n / 2;
        let n_bands = self.bands.len();
        let min_bin = 1.0_f32;
        let max_bin = half as f32;
        let ratio = max_bin / min_bin;

        let mut raw = vec![0.0_f32; n_bands];
        let mut frame_max = 0.0_f32;
        for (b, slot) in raw.iter_mut().enumerate() {
            let lo = (min_bin * ratio.powf(b as f32 / n_bands as f32)).floor() as usize;
            let hi = ((min_bin * ratio.powf((b + 1) as f32 / n_bands as f32)).ceil() as usize)
                .max(lo + 1)
                .min(half);

            let mut mag = 0.0;
            for k in lo..hi {
                mag += self.scratch[k].norm();
            }
            mag /= (hi - lo) as f32;

            // Pink/"tilt" compensation: musical energy rolls off ~1/f, which
            // makes the low bands dominate. Boosting by ~sqrt(frequency) flattens
            // the spectrum so all bars stay lively and balanced.
            let center = ((lo + hi) as f32 * 0.5).max(1.0);
            mag *= center.powf(0.5);

            // Log compression so quiet detail is still visible.
            let level = (1.0 + mag).ln();
            frame_max = frame_max.max(level);
            *slot = level;
        }

        // Auto-gain: track a decaying peak so the display always uses full height.
        self.peak = (self.peak * 0.92).max(frame_max).max(1e-3);

        for (smoothed, &level) in self.bands.iter_mut().zip(raw.iter()) {
            let target = (level / self.peak).clamp(0.0, 1.0);
            if target > *smoothed {
                *smoothed = target; // rise instantly
            } else {
                *smoothed = *smoothed * 0.78 + target * 0.22; // fall slowly
            }
        }
    }

    /// Let the bars sink toward zero when nothing is playing.
    pub fn decay(&mut self) {
        for b in self.bands.iter_mut() {
            *b *= 0.82;
        }
        self.peak = (self.peak * 0.96).max(1e-3);
    }
}
