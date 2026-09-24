// emulates input/output coupling capacitors (dc blocking)
#[derive(Clone, Copy, Default)]
pub struct DcBlocker {
    x_prev: f32,
    y_prev: f32,
}

impl DcBlocker {
    // applies one-pole high-pass filter
    pub fn process(&mut self, input: f32, coeff: f32) -> f32 {
        let output = input - self.x_prev + coeff * self.y_prev;
        self.x_prev = input;
        self.y_prev = output;
        output
    }
}

// emulates germanium transistor bandwidth limitation
#[derive(Clone, Copy, Default)]
pub struct OnePoleLpf {
    y_prev: f32,
}

impl OnePoleLpf {
    // applies one-pole low-pass filter
    pub fn process(&mut self, input: f32, coeff: f32) -> f32 {
        let output = (1.0 - coeff) * input + coeff * self.y_prev;
        self.y_prev = output;
        output
    }
}

// emulates the pnp germanium transistor stage (q1)
#[derive(Clone, Copy, Default)]
pub struct GermaniumStage {
    pub lpf: OnePoleLpf,
}

impl GermaniumStage {
    // applies soft saturation characteristic of germanium
    pub fn process(&mut self, input: f32, lpf_coeff: f32) -> f32 {
        let filtered = self.lpf.process(input, lpf_coeff);
        // soft clip with asymmetry typical of ge transistors
        let biased = filtered + 0.3;
        (biased / (1.0 + biased.abs())) - (0.3 / 1.3)
    }
}

// emulates the npn silicon transistor stage (q2)
#[derive(Clone, Copy, Default)]
pub struct SiliconStage {
    pub feedback_mem: f32,
}

impl SiliconStage {
    // applies cleaner gain with shunt feedback mixing
    pub fn process(&mut self, input: f32, feedback_mix: f32) -> f32 {
        let mixed = input + feedback_mix;
        // silicon clips harder and more symmetrically
        mixed.tanh()
    }
}

// emulates the asymmetric diode clipping stage
#[derive(Clone, Copy, Debug)]
pub struct DiodeClipper {
    pos_thres: f32,
    neg_thres: f32,
    sharpness: f32,
}

impl Default for DiodeClipper {
    fn default() -> Self {
        Self { 
            pos_thres: 0.26, 
            neg_thres: -0.27, 
            sharpness: 0.3 
        }
    }
}

impl DiodeClipper {
    // applies asymmetric soft clipping using a biased rational function
    pub fn process(&self, input: f32) -> f32 {
        let norm = if input >= 0.0 {
            input / self.pos_thres
        } else {
            input / self.neg_thres.abs()
        };

        let shaped = norm * self.sharpness / (1.0 + (norm * self.sharpness).abs());

        if input >= 0.0 {
            shaped * self.pos_thres
        } else {
            -(shaped * self.neg_thres.abs())
        }
    }
}
