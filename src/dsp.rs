use std::f32::consts::PI;

use nice_plug::params::FloatParam;

// holds the dsp state for iir filters per channel
#[derive(Clone, Copy, Default)]
pub struct ChannelState {
    hpf_in_y: f32,
    hpf_in_x: f32,
    lpf_y: f32,
    hpf_out_y: f32,
    hpf_out_x: f32,
}

pub struct PercolatorDSP {
    // array to store dsp state for up to 2 channels (stereo)
    states: [ChannelState; 2],
    sample_rate: f32,
}

impl Default for PercolatorDSP {
    fn default() -> Self {
        Self {
            states: [ChannelState::default(); 2],
            sample_rate: 44100.0,
        }
    }
}

impl PercolatorDSP {
    pub fn set_sr(&mut self, sr: f32) {
        self.sample_rate = sr;
    }

    pub fn process_sample(&mut self, samples: &mut [f32], ch: usize, harmonics: &FloatParam, balance: &FloatParam) {
        let state = &mut self.states[ch.min(1)];

        let lpf_coeff = calc_lpf_coeff(3000.0, self.sample_rate);
        let hpf_coeff = calc_hpf_coeff(20.0, self.sample_rate);

        for s in samples.iter_mut() {
            let h = harmonics.smoothed.next();
            let b = balance.smoothed.next();
            // input hpf to emulate input coupling capacitor
            let in_hpf: f32 = *s - state.hpf_in_x + hpf_coeff * state.hpf_in_y;
            state.hpf_in_x = *s;
            state.hpf_in_y = in_hpf;
            let mut sig = in_hpf;

            // input gain stage driven by harmonics parameter
            sig *= h * 15.0;

            // lpf to emulate germanium transistor bandwidth limitation
            sig = (1.0 - lpf_coeff) * sig + lpf_coeff * state.lpf_y;
            state.lpf_y = sig;

            // asymmetric soft clipping with bias to emulate transistor starvation
            let bias: f32 = 1.2;
            let biased = sig + bias;
            sig = biased / (1.0 + biased.abs()) - (bias / (1.0 + bias.abs()));

            // output hpf to remove dc offset introduced by asymmetric clipping
            let out_hpf: f32 = sig - state.hpf_out_x + hpf_coeff * state.hpf_out_y;
            state.hpf_out_x = sig;
            state.hpf_out_y = out_hpf;
            sig = out_hpf;

            // applies final output level control and writes back to the slice
            *s = sig * b;
        }
    }
}

// calculates one-pole lpf coefficient based on cutoff and sample rate
fn calc_lpf_coeff(fc: f32, fs: f32) -> f32 {
    (-2.0 * PI * fc / fs).exp()
}

// calculates one-pole hpf coefficient for dc blocking
fn calc_hpf_coeff(fc: f32, fs: f32) -> f32 {
    (-2.0 * PI * fc / fs).exp()
}
