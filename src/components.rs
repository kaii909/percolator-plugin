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
        let r1_c1 = 2.0 * std::f32::consts::PI * 22.0; // dc input blocker cutoff - tuned for tighter low end
        let r2_c2 = 2.0 * std::f32::consts::PI * 2800.0; // germanium bandwidth cutoff - matched to real leakage

        // continuous-time state-space representation matrices
        let a_continuous = SMatrix::<f32, 2, 2>::new(-r1_c1, 0.0, 0.0, -r2_c2);
        let b_continuous = SVector::<f32, 2>::new(r1_c1, r2_c2);
        let c_continuous = SMatrix::<f32, 1, 2>::new(-1.0, 1.0);
        let d_continuous = 1.0;

        // discrete-time mapping using standard trapezoidal discretization
        let identity = SMatrix::<f32, 2, 2>::identity();
        let inv_term = (identity - a_continuous * (t / 2.0))
            .try_inverse()
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

// emulates the npn silicon transistor stage (q1 in original hp1)
#[derive(Clone, Copy, Default)]
pub struct SiliconStage {}

impl SiliconStage {
    // applies hard and aggressive npn silicon saturation first in the chain
    pub fn process(&mut self, input: f32) -> f32 {
        (input * 2.2).tanh()
    }
}

// emulates the pnp germanium transistor stage (q2 in original hp1)
#[derive(Clone)]
pub struct GermaniumStage {
    pub lpf: OnePoleLpf,
    pub bias: Shared,
    pub feedback_mix: Shared,
    pub feedback_mem: f32,
    pub fb_hp_mem: f32,
    pub acoustic_mem: f32,
    pub fb_lpf_mem: f32,
    pub dc_block_mem: f32, // introduced memory tracker for realtime output waveform centering
}

impl Default for GermaniumStage {
    fn default() -> Self {
        Self {
            lpf: OnePoleLpf::default(),
            bias: shared(0.3),
            feedback_mix: shared(0.15),
            feedback_mem: 0.0,
            fb_hp_mem: 0.0,
            acoustic_mem: 0.0,
            fb_lpf_mem: 0.0,
            dc_block_mem: 0.0,
        }
    }
}

impl GermaniumStage {
    // applies soft pnp germanium saturation with interstage 220k shunt feedback loop
    pub fn process(&mut self, input: f32, lpf_coeff: f32, harmonics: f32) -> f32 {
        let mix = self.feedback_mix.value();

        // AC coupling to prevent DC offset accumulation inside feedback path
        let fb_ac = self.feedback_mem - self.fb_hp_mem;

        // dynamic link: high harmonics content alters the time-constant dissipation directly (Caso 1 vs Caso 3)
        let lock_protection = 1.0 + (fb_ac.abs() * 4.0);
        let base_factor = 0.03 + (1.0 - harmonics) * 0.05;
        let freq_factor =
            (base_factor + (input.abs() * 0.12 * (1.0 + harmonics))) * lock_protection;
        self.fb_hp_mem += freq_factor.min(0.95) * fb_ac;

        // fast acoustic coupling phase emulation using physical dissipation
        let acoustic_coupling = fb_ac - self.acoustic_mem;
        self.acoustic_mem += 0.08 * acoustic_coupling;

        // emulates original 220k ohm resistor network scale factor feeding back into silicon node
        let r_220k_scale = 1.0 / 220000.0;

        // gate threshold scale: prevent low level noise from infinitely re-triggering feedback loop
        let input_envelope = input.abs().min(1.0);
        let soft_gate = (input_envelope * 12.0).tanh();

        // scaled gain down substantially and tied directly to harmonics presence
        // this completely tames the linear loop gain, stopping auto-oscillation at low drive settings
        let feedback_gain_mod = 4500.0 * (harmonics + 0.05) * soft_gate * (mix * 3.0);
        let raw_input_loop = acoustic_coupling * r_220k_scale * feedback_gain_mod;

        // loop lowpass dampening: smoothed factor slightly to ensure unity phase margin
        self.fb_lpf_mem = 0.4 * raw_input_loop + 0.6 * self.fb_lpf_mem;
        let input_loop = self.fb_lpf_mem;

        let mixed = input + input_loop;

        // electrical rail bounding: prevent numerical infinite explosions on feedback memory accumulation
        let bounded_mixed = mixed.clamp(-10.0, 10.0);
        let filtered = self.lpf.process(bounded_mixed, lpf_coeff);

        // reads the atomic parameter to shift the clipping wave center
        // dynamic link: smoothly scale bias impact back as drive grows to preserve symmetry headroom
        let current_bias = self.bias.value() * (1.0 - harmonics * 0.45);
        let biased = filtered + current_bias;
        let saturated_out =
            (biased / (1.0 + biased.abs())) - (current_bias / (1.0 + current_bias.abs()));

        self.feedback_mem = saturated_out;

        // dynamic waveform centering: high pass filter to prevent disproportionate electrical sag
        let dc_leak = 0.993; // tight time constant to preserve structural asymmetric harmonic transients
        self.dc_block_mem = dc_leak * self.dc_block_mem + (1.0 - dc_leak) * saturated_out;

        saturated_out - self.dc_block_mem
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
        if x < 0.0 || x.is_nan() {
            return 0.0;
        }
        if x < 1.0 {
            // approx for low values
            x * (1.0 - 0.3343 * x + 0.1145 * x * x)
        } else {
            // saturation approx
            let ln_x = if x.is_infinite() { 88.0 } else { x.ln() }; // safe fallback boundary for extreme exponents
            let ln_ln_x = ln_x.ln();
            ln_x - ln_ln_x + (ln_ln_x / ln_x)
        }
    }

    pub fn process(&self, input_v: f32) -> f32 {
        // fail-safe check: if incoming signal exploded elsewhere in the chain, catch it instantly
        if input_v.is_nan() {
            return 0.0;
        }

        let r_impedance = 1000.0; // 1k Ohms

        let s_val = self.sharpness.value();

        if input_v >= 0.0 {
            // positive (germanium)
            let is_ge = 1.0e-6;
            let vt_ge = 0.026 * (1.0 + s_val * 4.0);

            let c_param = (r_impedance * is_ge) / vt_ge;

            // bound exponents to safe mathematical thresholds to completely avoid infinity explosion
            let exp_arg = ((input_v + r_impedance * is_ge) / vt_ge).min(80.0);
            let arg = c_param * exp_arg.exp();

            let w = self.lambert_w_approx(arg);
            let v_out = input_v - r_impedance * is_ge * ((w / c_param) - 1.0);

            if v_out.is_nan() {
                input_v
            } else {
                v_out.clamp(0.0, input_v)
            }
        } else {
            // negative (silicon)
            let is_si = 1.0e-11;
            let vt_si = 0.052 * (1.0 + s_val * 2.0);
            let input_abs = input_v.abs();

            let c_param = (r_impedance * is_si) / vt_si;

            // bound exponents to safe mathematical thresholds to completely avoid infinity explosion
            let exp_arg = ((input_abs + r_impedance * is_si) / vt_si).min(80.0);
            let arg = c_param * exp_arg.exp();

            let w = self.lambert_w_approx(arg);
            let v_out_abs = input_abs - r_impedance * is_si * ((w / c_param) - 1.0);

            if v_out_abs.is_nan() {
                input_v
            } else {
                -(v_out_abs.clamp(0.0, input_abs))
            }
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
impl AudioNode for SiliconStage {
    const ID: u64 = 1001;
    type Inputs = U1;
    type Outputs = U1;
    #[inline]
    fn tick(&mut self, input: &Frame<f32, Self::Inputs>) -> Frame<f32, Self::Outputs> {
        let out = self.process(input[0]);
        Frame::from([out])
    }
}
impl AudioNode for GermaniumStage {
    const ID: u64 = 1002;
    type Inputs = U2; // Modified to accept both Signal and Harmonics parameter dynamically
    type Outputs = U1;
    #[inline]
    fn tick(&mut self, input: &Frame<f32, Self::Inputs>) -> Frame<f32, Self::Outputs> {
        let out = self.process(input[0], 0.35, input[1]);
        Frame::from([out])
    }
}
