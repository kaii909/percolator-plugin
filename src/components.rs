use fundsp::prelude32::*;
use nalgebra::{SMatrix, SVector};

// integrated state-space system for linear circuitry simulation
#[derive(Clone)]
pub struct CircuitStateSpace {
    state: SVector<f32, 2>,
    ad: SMatrix<f32, 2, 2>,
    bd: SVector<f32, 2>,
    cd: SMatrix<f32, 1, 2>,
    dd: f32,
    current_sr: f32,
}

impl Default for CircuitStateSpace {
    fn default() -> Self {
        let mut s = Self {
            state: SVector::<f32, 2>::zeros(),
            ad: SMatrix::<f32, 2, 2>::zeros(),
            bd: SVector::<f32, 2>::zeros(),
            cd: SMatrix::<f32, 1, 2>::zeros(),
            dd: 0.0,
            current_sr: 44100.0,
        };
        s.recalculate_matrices(44100.0);
        s
    }
}

impl CircuitStateSpace {
    // computes analytical state-space matrices using bilinear transform
    pub fn recalculate_matrices(&mut self, sr: f32) {
        self.current_sr = sr;
        let t = 1.0 / sr;

        // equivalent physical cutoff frequencies (rad/s)
        let r1_c1 = 2.0 * std::f32::consts::PI * 15.0;   // dc input blocker cutoff
        let r2_c2 = 2.0 * std::f32::consts::PI * 3200.0; // germanium bandwidth cutoff

        // continuous-time state-space representation matrices
        let a_continuous = SMatrix::<f32, 2, 2>::new(
            -r1_c1,   0.0,
             0.0,   -r2_c2
        );
        let b_continuous = SVector::<f32, 2>::new(r1_c1, r2_c2);
        let c_continuous = SMatrix::<f32, 1, 2>::new(-1.0, 1.0);
        let d_continuous = 1.0;

        // discrete-time mapping using standard trapezoidal discretization
        let identity = SMatrix::<f32, 2, 2>::identity();
        let inv_term = (identity - a_continuous * (t / 2.0)).try_inverse()
            .unwrap_or_else(SMatrix::<f32, 2, 2>::identity);

        self.ad = inv_term * (identity + a_continuous * (t / 2.0));
        self.bd = inv_term * b_continuous * (t / 2.0);
        self.cd = c_continuous * (identity + self.ad) * 0.5;
        
        // extracts explicit scalar f32 value from 1x1 matrix multiplication product
        let cd_times_bd = c_continuous * self.bd;
        self.dd = d_continuous + cd_times_bd[0];
    }

    // applies the linear filter system in a single loop-free step
    #[inline]
    pub fn process(&mut self, input: f32) -> f32 {
        let u = SVector::<f32, 1>::new(input);
        
        // output equation: y[n] = C * x[n-1] + D * u[n]
        let y = self.cd * self.state + self.dd * u;
        
        // state update equation: x[n] = A * x[n-1] + B * u[n]
        self.state = self.ad * self.state + self.bd * u;

        y[0]
    }
}

// // emulates input/output coupling capacitors (dc blocking)
// #[derive(Clone, Copy, Default)]
// pub struct DcBlocker {
//     x_prev: f32,
//     y_prev: f32,
// }
//
// impl DcBlocker {
//     // applies one-pole high-pass filter
//     pub fn process(&mut self, input: f32, coeff: f32) -> f32 {
//         let output = input - self.x_prev + coeff * self.y_prev;
//         self.x_prev = input;
//         self.y_prev = output;
//         output
//     }
// }

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
#[derive(Clone)]
pub struct SiliconStage {
    pub feedback_mem: f32,
    pub feedback_mix: Shared,
    pub fb_hp_mem: f32,
}

impl Default for SiliconStage {
    fn default() -> Self {
        Self { feedback_mem: 0.0, feedback_mix: shared(0.15), fb_hp_mem: 0.0 }
    }
}

impl SiliconStage {
    // applies cleaner gain with shunt feedback mixing
    pub fn process(&mut self, input: f32) -> f32 {
        // squared mix to map the parameter exponentialy (0.2 turns 0.04, avoiding excessive
        // feedback at low values)
        let mix = self.feedback_mix.value() * self.feedback_mix.value();

        // AC coupling
        let fb_ac = self.feedback_mem - self.fb_hp_mem;

        // hp filter interacts with input, resulting in different tones
        let freq_factor = 0.03 + (input.abs() * 0.12);
        self.fb_hp_mem += freq_factor * fb_ac;

        // for unstopabble feedback, it needs to loop as a positive signal
        let input_loop = fb_ac * mix * 12.0;
        
        let mixed = input + input_loop;

        let output = (mixed * 1.9).tanh();
        
        self.feedback_mem = output;
        output
    }
}

// emulates the asymmetric diode clipping stage
#[derive(Clone)]
pub struct DiodeClipper {
    pub sharpness: Shared,
}

impl Default for DiodeClipper {
    fn default() -> Self {
        Self {
            sharpness: shared(0.2),
        }
    }
}

// high precision analytical model using lambert approximation without loops
impl DiodeClipper {
    #[inline]
    fn lambert_w_approx(&self, x: f32) -> f32 {
        if x < 0.0 { return 0.0; }
        if x < 1.0 {
            // approx for low values
            x * (1.0 - 0.3343 * x + 0.1145 * x * x)
        } else {
            // saturation approx
            let ln_x = x.ln();
            let ln_ln_x = ln_x.ln();
            ln_x - ln_ln_x + (ln_ln_x / ln_x)
        }
    }

    pub fn process(&self, input_v: f32) -> f32 {
        let r_impedance: f32 = 1000.0; // 1k Ohms
        
        let s_val = self.sharpness.value();
        let input_vec = SVector::<f32, 1>::new(input_v);

        if input_vec[0] >= 0.0 {
            // positive (germanium)
            let is_ge: f32 = 1.0e-6;
            let vt_ge: f32 = 0.026 * (1.0 + s_val * 4.0);

            let c_param = (r_impedance * is_ge) / vt_ge;
            
            let arg = c_param * ( (input_vec[0] + r_impedance * is_ge) / vt_ge ).exp();
            
            let w = self.lambert_w_approx(arg);
            let v_out = input_vec[0] - r_impedance * is_ge * ((w / c_param) - 1.0);
            
            v_out.clamp(0.0, input_vec[0])
        } else {
            // negative (silicon)
            let is_si: f32 = 1.0e-11;
            let vt_si: f32 = 0.052 * (1.0 + s_val * 2.0);
            let input_abs = input_vec[0].abs();

            let c_param = (r_impedance * is_si) / vt_si;
            let arg = c_param * ( (input_abs + r_impedance * is_si) / vt_si ).exp();
            
            let w = self.lambert_w_approx(arg);
            let v_out_abs = input_abs - r_impedance * is_si * ((w / c_param) - 1.0);
            
            -(v_out_abs.clamp(0.0, input_abs))
        }
    }
}

// FUN DSP AUDIO NODES =============================================================================
// circuit coefficients can be changed here

impl AudioNode for CircuitStateSpace {
    const ID: u64 = 1000;
    type Inputs = U1;
    type Outputs = U1;

    #[inline]
    fn tick(&mut self, input: &Frame<f32, Self::Inputs>) -> Frame<f32, Self::Outputs> {
        // reads current sample rate stored inside node instance state
        let out = self.process(input[0]);
        Frame::from([out])
    }

    // handles sample rate updates directly from host dsp graph pipeline activation
    fn set_sample_rate(&mut self, sample_rate: f64) {
        self.recalculate_matrices(sample_rate as f32);
    }
}

// impl AudioNode for DcBlocker {
//     const ID: u64 = 1001; 
//     type Inputs = U1;
//     type Outputs = U1;
//
//     #[inline]
//     fn tick(&mut self, input: &Frame<f32, Self::Inputs>) -> Frame<f32, Self::Outputs> {
//         let out = self.process(input[0], 0.995);
//         Frame::from([out])
//     }
// }

impl AudioNode for GermaniumStage {
    const ID: u64 = 1002;
    type Inputs = U1;
    type Outputs = U1;

    #[inline]
    fn tick(&mut self, input: &Frame<f32, Self::Inputs>) -> Frame<f32, Self::Outputs> {
        let out = self.process(input[0], 0.35);
        Frame::from([out])
    }
}

impl AudioNode for SiliconStage {
    const ID: u64 = 1003;
    type Inputs = U1;
    type Outputs = U1;

    #[inline]
    fn tick(&mut self, input: &Frame<f32, Self::Inputs>) -> Frame<f32, Self::Outputs> {
        let out = self.process(input[0]);
        Frame::from([out])
    }
}
