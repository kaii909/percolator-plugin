use crate::components::DiodeClipper;
use fundsp::prelude32::*;

// holds the fundsp graph and the shared knobs that feed it
pub struct PercolatorDSP {
    circuit: Box<dyn AudioUnit>,
    drive: Shared,
    volume: Shared,
}

impl Default for PercolatorDSP {
    fn default() -> Self {
        let drive = shared(0.5);
        let volume = shared(0.5);
        let circuit = Self::build_circuit(&drive, &volume);
        Self {
            circuit,
            drive,
            volume,
        }
    }
}

impl PercolatorDSP {
    pub fn set_sr(&mut self, sr: f32) {
        // retunes filters in place without resetting their internal state
        self.circuit.set_sample_rate(sr as f64);
    }

    // graph is built once; knobs enter through var nodes read at audio rate
    fn build_circuit(drive: &Shared, volume: &Shared) -> Box<dyn AudioUnit> {
        let diodes = DiodeClipper::default();
        let chain = (highpass_hz(20.0, 1.0) * (var(drive) * 15.0))
            >> lowpass_hz(3000.0, 1.0)
            >> shape_fn(move |x: f32| diodes.process(x))
            >> (highpass_hz(20.0, 1.0) * var(volume));
        // 2x oversampling around the non-linear stages kills aliasing
        Box::new(oversample(chain))
    }

    // pushes smoothed knob values into the graph and filters one sample
    pub fn filter(&mut self, harmonics: f32, balance: f32, input: f32) -> f32 {
        self.drive.set(harmonics);
        self.volume.set(balance);
        self.circuit.filter_mono(input)
    }
}
