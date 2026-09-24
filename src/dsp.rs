use crate::components::*;
use fundsp::prelude32::*;

// holds the fundsp graph and the shared knobs that feed it
pub struct PercolatorDSP {
    circuit: Box<dyn AudioUnit>,
    drive: Shared,
    sharpness: Shared,
    pub feedback_mix: Shared,
    volume: Shared,
}

impl Default for PercolatorDSP {
    fn default() -> Self {
        let drive = shared(0.5);
        let sharpness = shared(0.5);
        let feedback_mix = shared(0.15);
        let volume = shared(0.5);
        let circuit = Self::build_circuit(drive.clone(), sharpness.clone(), feedback_mix.clone(), volume.clone());
        Self {
            circuit,
            drive,
            sharpness,
            feedback_mix,
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
    fn build_circuit(drive: Shared, sharp: Shared, fb_mix: Shared, volume: Shared) -> Box<dyn AudioUnit> {
        let dc_in = DcBlocker::default();
        let q1_stage = GermaniumStage::default();
        let q2_stage = SiliconStage {
            feedback_mix: fb_mix.clone(),
            ..Default::default()
        };
        let diodes = DiodeClipper {
            sharpness: sharp.clone(),
            ..Default::default()
        };
        let dc_out = DcBlocker::default();

        let chain = 
            // 1. dc_in stage
            An(dc_in)
            // 2. circuit gain
            >> shape_fn(move |x|  x * (drive.clone().value() * 25.0))
            // 3. q1_stage (germanium)
            >> An(q1_stage)
            // 4. q2_stage (silicon)
            >> An(q2_stage)
            // 5. diode hard clipping
            >> shape_fn(move |x| diodes.process(x))
            // 6. dc_out stage
            >> An(dc_out)
            // 7. volume stage
            >> shape_fn(move |x| x * (volume.clone().value() * 2.0));

            // >> lowpass_hz(3000.0, 1.0)
            // >> (shape_fn(move |x: f32| diodes.process(x)) * 2.8)
            // >> (highpass_hz(20.0, 1.0) * var(volume));

        // 2x oversampling around the non-linear stages kills aliasing
        Box::new(oversample(chain))
    }

    // pushes smoothed knob values into the graph and filters one sample
    pub fn filter(&mut self, harmonics: f32, sharp: f32, fb: f32, balance: f32, input: f32) -> f32 {
        self.drive.set(harmonics);
        self.sharpness.set(sharp);
        self.feedback_mix.set(fb);
        self.volume.set(balance);
        self.circuit.filter_mono(input)
    }
}
