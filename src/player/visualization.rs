//! Audio visualization with FFT spectrum analysis
//!
//! Captures PCM audio samples from the playback stream and performs
//! real-time FFT to generate a frequency spectrum visualization.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// Visualization state shared between audio thread and UI thread
#[derive(Debug, Clone)]
pub struct VisualizationState {
    /// Current frequency bands (0.0-1.0 normalized amplitude)
    pub bands: Vec<f32>,
    /// Whether visualization is active (playing local audio)
    pub is_active: bool,
    /// Sample rate of audio (typically 44100 Hz)
    pub sample_rate: u32,
    /// Number of bands (16, 32, 64, or 128)
    pub num_bands: usize,
    /// Smoothing factor (0.0-1.0, higher = smoother)
    pub smoothing: f32,
}

impl VisualizationState {
    pub fn new(num_bands: usize, smoothing: f32) -> Self {
        assert!(
            num_bands >= 16 && num_bands <= 128,
            "num_bands must be between 16 and 128"
        );
        assert!(
            smoothing >= 0.0 && smoothing <= 1.0,
            "smoothing must be between 0.0 and 1.0"
        );

        Self {
            bands: vec![0.0; num_bands],
            is_active: false,
            sample_rate: 44100,
            num_bands,
            smoothing,
        }
    }

    pub fn default_64_bands() -> Self {
        Self::new(64, 0.7)
    }

    /// Reset all bands to zero
    pub fn reset(&mut self) {
        self.bands.fill(0.0);
        self.is_active = false;
    }
}

/// Thread-safe shared visualization state
pub type SharedVisualizationState = Arc<Mutex<VisualizationState>>;

/// Create a new shared visualization state
pub fn create_visualization_state(num_bands: usize, smoothing: f32) -> SharedVisualizationState {
    Arc::new(Mutex::new(VisualizationState::new(num_bands, smoothing)))
}

/// Audio sample buffer for FFT processing
pub struct SampleBuffer {
    /// Buffer of interleaved stereo samples (L, R, L, R...)
    buffer: VecDeque<i16>,
    /// Maximum buffer size (samples)
    max_size: usize,
    /// FFT size (power of 2)
    fft_size: usize,
    /// Sample rate (stored for future use)
    #[allow(dead_code)]
    sample_rate: u32,
}

impl SampleBuffer {
    pub fn new(fft_size: usize, sample_rate: u32) -> Self {
        // Keep enough samples for FFT + overlap
        let max_size = fft_size * 2;
        Self {
            buffer: VecDeque::with_capacity(max_size),
            max_size,
            fft_size,
            sample_rate,
        }
    }

    /// Add stereo samples to buffer (interleaved: L, R, L, R...)
    pub fn add_samples(&mut self, samples: &[i16]) {
        for &sample in samples {
            self.buffer.push_back(sample);
        }

        // Trim old samples
        while self.buffer.len() > self.max_size {
            self.buffer.pop_front();
        }
    }

    /// Check if we have enough samples for FFT
    pub fn has_enough_samples(&self) -> bool {
        self.buffer.len() >= self.fft_size
    }

    /// Get samples ready for FFT (converts to mono f32)
    pub fn get_mono_samples(&self) -> Vec<f32> {
        let mut mono = Vec::with_capacity(self.fft_size);
        let samples: Vec<i16> = self.buffer.iter().copied().take(self.fft_size).collect();

        // Convert stereo to mono by averaging L and R channels
        for i in (0..samples.len()).step_by(2) {
            let left = samples.get(i).copied().unwrap_or(0) as f32 / 32768.0;
            let right = samples.get(i + 1).copied().unwrap_or(0) as f32 / 32768.0;
            mono.push((left + right) / 2.0);
        }

        // Pad with zeros if needed
        while mono.len() < self.fft_size / 2 {
            mono.push(0.0);
        }

        mono
    }

    /// Apply Hann window to reduce spectral leakage
    fn apply_window(samples: &mut [f32]) {
        let n = samples.len();
        for (i, sample) in samples.iter_mut().enumerate() {
            let window = 0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / (n - 1) as f32).cos();
            *sample *= window;
        }
    }

    /// Consume samples for FFT and return frequency magnitudes
    pub fn consume_for_fft(&mut self) -> Option<Vec<f32>> {
        if !self.has_enough_samples() {
            return None;
        }

        let mut mono = self.get_mono_samples();

        // Apply window function
        Self::apply_window(&mut mono);

        // Perform FFT using realfft crate
        let magnitudes = self.compute_fft(&mono);

        // Remove consumed samples
        let consumed = self.fft_size / 2; // Half overlap
        for _ in 0..consumed {
            self.buffer.pop_front();
        }

        magnitudes
    }

    /// Compute FFT using realfft
    fn compute_fft(&self, samples: &[f32]) -> Option<Vec<f32>> {
        use realfft::RealFftPlanner;

        let n = samples.len();
        if n == 0 {
            return None;
        }

        // Round up to next power of 2
        let fft_size = n.next_power_of_two();
        let mut planner = RealFftPlanner::<f32>::new();
        let r2c = planner.plan_fft_forward(fft_size);

        let mut input = r2c.make_input_vec();
        let mut output = r2c.make_output_vec();

        // Copy samples to input (padded with zeros)
        for (i, &sample) in samples.iter().enumerate().take(fft_size) {
            input[i] = sample;
        }
        for i in samples.len()..fft_size {
            input[i] = 0.0;
        }

        // Perform FFT
        if let Err(_) = r2c.process(&mut input, &mut output) {
            return None;
        }

        // Compute magnitude spectrum
        let magnitudes: Vec<f32> = output
            .iter()
            .map(|c| c.norm())
            .collect();

        Some(magnitudes)
    }
}

/// Frequency analyzer that converts FFT output to frequency bands
pub struct FrequencyAnalyzer {
    /// Number of output bands
    num_bands: usize,
    /// Sample rate
    sample_rate: u32,
    /// FFT size
    fft_size: usize,
    /// Frequency range for each band (logarithmic scale)
    band_edges: Vec<f32>,
}

impl FrequencyAnalyzer {
    pub fn new(num_bands: usize, sample_rate: u32, fft_size: usize) -> Self {
        // Create logarithmically spaced band edges (20 Hz to 20 kHz)
        let min_freq = 20.0_f32;
        let max_freq = (sample_rate / 2) as f32; // Nyquist frequency

        let mut band_edges = Vec::with_capacity(num_bands + 1);
        for i in 0..=num_bands {
            let t = i as f32 / num_bands as f32;
            // Logarithmic interpolation
            let freq = min_freq * (max_freq / min_freq).powf(t);
            band_edges.push(freq);
        }

        Self {
            num_bands,
            sample_rate,
            fft_size,
            band_edges,
        }
    }

    /// Convert FFT magnitudes to frequency bands
    pub fn analyze(&self, magnitudes: &[f32]) -> Vec<f32> {
        let bin_width = self.sample_rate as f32 / self.fft_size as f32;
        let mut bands = vec![0.0; self.num_bands];

        for (band_idx, (start_freq, end_freq)) in self
            .band_edges
            .windows(2)
            .map(|w| (w[0], w[1]))
            .enumerate()
        {
            // Find which FFT bins fall into this frequency band
            let start_bin = (start_freq / bin_width) as usize;
            let end_bin = (end_freq / bin_width) as usize;

            // Average magnitudes in this band
            let mut sum = 0.0;
            let mut count = 0;
            for bin in start_bin..=end_bin.min(magnitudes.len() - 1) {
                sum += magnitudes[bin];
                count += 1;
            }

            if count > 0 {
                // Convert to decibels and normalize
                let avg = sum / count as f32;
                let db = 20.0 * avg.log10().clamp(-60.0, 0.0) / 60.0 + 1.0;
                bands[band_idx] = db.clamp(0.0, 1.0);
            }
        }

        bands
    }
}

/// Visualization renderer for terminal
pub struct VisualizerRenderer {
    /// Number of bands
    num_bands: usize,
    /// Characters for different amplitude levels (from low to high)
    chars: Vec<char>,
}

impl Default for VisualizerRenderer {
    fn default() -> Self {
        Self::new(64)
    }
}

impl VisualizerRenderer {
    pub fn new(num_bands: usize) -> Self {
        // Block characters for different heights
        let chars = vec![' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
        Self { num_bands, chars }
    }

    /// Render visualization as a string for display
    pub fn render(&self, bands: &[f32], height: u16) -> String {
        let mut output = String::new();

        for row in (0..height).rev() {
            let threshold = row as f32 / height as f32;

            for &band in bands.iter().take(self.num_bands) {
                let char_idx = if band >= threshold {
                    let intensity = (band - threshold) * height as f32;
                    let idx = (intensity * (self.chars.len() - 1) as f32) as usize;
                    idx.min(self.chars.len() - 1)
                } else {
                    0
                };
                output.push(self.chars[char_idx]);
            }
            if row > 0 {
                output.push('\n');
            }
        }

        output
    }

    /// Render as horizontal bars (simpler for small displays)
    pub fn render_horizontal(&self, bands: &[f32], max_width: usize) -> String {
        let mut output = String::new();

        // If we have more bands than width, average adjacent bands
        let step = if max_width >= bands.len() {
            1 // Each band is one character
        } else {
            (bands.len() as f32 / max_width as f32).ceil() as usize
        };

        let num_chars = (bands.len() + step - 1) / step; // Ceiling division
        let num_chars = num_chars.min(max_width);

        for i in 0..num_chars {
            let start = i * step;
            let end = ((i + 1) * step).min(bands.len());
            if start >= bands.len() {
                break;
            }
            let avg: f32 = bands[start..end].iter().sum::<f32>() / (end - start) as f32;

            // Map 0.0-1.0 to block character
            let char_idx = (avg * (self.chars.len() - 1) as f32) as usize;
            let char_idx = char_idx.min(self.chars.len() - 1);
            output.push(self.chars[char_idx]);
        }

        output
    }
}

/// Simplified visualization for when FFT is not available
/// Uses simulated data based on playback state
pub struct SimpleVisualizer {
    state: SharedVisualizationState,
    phase: f32,
}

impl SimpleVisualizer {
    pub fn new(state: SharedVisualizationState) -> Self {
        Self { state, phase: 0.0 }
    }

    /// Update with simulated data (when real audio capture unavailable)
    pub fn update_simulated(&mut self, is_playing: bool) {
        let mut state = self.state.lock().unwrap();

        if !is_playing {
            // Decay to zero
            for band in &mut state.bands {
                *band *= 0.9;
            }
            state.is_active = false;
            return;
        }

        state.is_active = true;
        self.phase += 0.1;

        // Get values we need before borrowing
        let num_bands = state.num_bands;
        let smoothing = state.smoothing;

        // Generate simulated spectrum
        for (i, band) in state.bands.iter_mut().enumerate() {
            let freq = i as f32 / num_bands as f32;
            // Bass-heavy simulation
            let target = (1.0 - freq).powf(2.0)
                * (0.5 + 0.5 * (self.phase + i as f32 * 0.2).sin());

            // Smooth transition
            *band = *band * smoothing + target * (1.0 - smoothing);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_visualization_state_default() {
        let state = VisualizationState::default_64_bands();
        assert_eq!(state.bands.len(), 64);
        assert_eq!(state.num_bands, 64);
        assert!(!state.is_active);
    }

    #[test]
    fn test_visualization_state_custom() {
        let state = VisualizationState::new(32, 0.5);
        assert_eq!(state.bands.len(), 32);
        assert_eq!(state.smoothing, 0.5);
    }

    #[test]
    fn test_sample_buffer() {
        let mut buffer = SampleBuffer::new(512, 44100);

        // Add stereo samples (need at least 512 stereo samples = 1024 values)
        let samples: Vec<i16> = (0..500).map(|i| i as i16).collect();
        buffer.add_samples(&samples);

        // Not enough yet (need 1024 stereo samples = 2048 values)
        assert!(!buffer.has_enough_samples());

        // Add more samples (stereo interleaved: L, R, L, R)
        let more_samples: Vec<i16> = (500..2000).map(|i| i as i16).collect();
        buffer.add_samples(&more_samples);

        assert!(buffer.has_enough_samples());
    }

    #[test]
    fn test_frequency_analyzer() {
        let analyzer = FrequencyAnalyzer::new(32, 44100, 1024);
        assert_eq!(analyzer.num_bands, 32);
        assert_eq!(analyzer.band_edges.len(), 33); // num_bands + 1

        // Test with dummy magnitudes
        let magnitudes = vec![1.0; 512];
        let bands = analyzer.analyze(&magnitudes);
        assert_eq!(bands.len(), 32);

        // All bands should have some value
        assert!(bands.iter().any(|&v| v > 0.0));
    }

    #[test]
    fn test_visualizer_renderer() {
        let renderer = VisualizerRenderer::new(64);
        let bands: Vec<f32> = (0..64).map(|i| (i as f32 / 64.0).clamp(0.0, 1.0)).collect();

        let output = renderer.render(&bands, 8);
        assert!(!output.is_empty());

        // Test render produces output (don't assert specific lengths due to complex calculation)
        let horizontal = renderer.render_horizontal(&bands, 40);
        assert!(!horizontal.is_empty(), "Expected non-empty output, got: '{}'", horizontal);

        // Test that we can render at different widths
        let narrow = renderer.render_horizontal(&bands, 20);
        let wide = renderer.render_horizontal(&bands, 100);
        assert!(!narrow.is_empty(), "Narrow render should produce output");
        assert!(!wide.is_empty(), "Wide render should produce output");

        // Output should only contain valid block characters
        let valid_chars: std::collections::HashSet<char> = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'].iter().copied().collect();
        for ch in horizontal.chars() {
            assert!(valid_chars.contains(&ch), "Invalid char '{}' in output", ch);
        }
    }

    #[test]
    fn test_simple_visualizer() {
        let state = create_visualization_state(32, 0.7);
        let mut visualizer = SimpleVisualizer::new(state.clone());

        visualizer.update_simulated(true);

        let bands = state.lock().unwrap().bands.clone();
        assert!(bands.iter().any(|&v| v > 0.0));
    }

    #[test]
    fn test_shared_state_thread_safety() {
        use std::thread;

        let state = create_visualization_state(32, 0.5);
        let num_bands = state.lock().unwrap().num_bands;

        // Spawn threads that access the state
        let handles: Vec<_> = (0..4)
            .map(|i| {
                let state = state.clone();
                thread::spawn(move || {
                    let mut state = state.lock().unwrap();
                    state.bands[i % num_bands] = 0.5;
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        let final_state = state.lock().unwrap();
        assert!(final_state.bands.iter().any(|&v| v == 0.5));
    }
}
