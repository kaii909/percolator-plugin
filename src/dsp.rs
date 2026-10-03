use crate::components::*;
use fundsp::prelude32::*;

// holds the fundsp graph and the shared knobs that feed it
pub struct PercolatorDSP {
    circuit: Box<dyn AudioUnit>,
    drive: Shared,
    sharpness: Shared,
    // pub feedback_mix: Shared,
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
            // feedback_mix,
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
        // state-space matrices for linear filter unifications
        let state_space_filters = CircuitStateSpace::default();
        
        // explicitly pass the shared atomic pointer using struct initialization style to appease clippy
        let q1_stage = GermaniumStage {
            feedback_mix: fb_mix.clone(),
            ..GermaniumStage::default()
        };
        
        let q2_stage = SiliconStage::default();
        let diodes = DiodeClipper {
            sharpness: sharp.clone(),
        };

        // clone references here so each closure gets its own ownership safely before the chain starts
        let drive_for_gain = drive.clone();
        let volume_for_gain = volume.clone();

        let chain = 
            // 1. integrated linear state-space stage
            An(state_space_filters)
            // 2. circuit gain applied symmetrically to the single audio channel before splitting parameters
            >> shape_fn(move |x| x * (drive_for_gain.value() * 18.0))
            // 3. clone signal path to pass both balanced audio x and harmonics configuration down into GermaniumStage
            >> (pass() | var(&drive))
            // 4. q1_stage (germanium) processing audio input and consuming raw harmonics configuration
            >> An(q1_stage)
            // 5. q2_stage (silicon)
            >> An(q2_stage)
            // 6. diode hard clipping
            >> shape_fn(move |x| diodes.process(x))
            // 7. volume stage
            >> shape_fn(move |x| x * (volume_for_gain.value() * 2.0));

        // 2x oversampling around the non-linear stages kills aliasing
        Box::new(oversample(chain))
    }

    // pushes smoothed knob values into the graph and filters one sample
    pub fn filter(&mut self, harmonics: f32, sharp: f32, balance: f32, input: f32) -> f32 {
        self.drive.set(harmonics);
        self.sharpness.set(sharp);
        // self.feedback_mix.set(fb);
        self.volume.set(balance);
        self.circuit.filter_mono(input)
    }
}
