use nice_plug::prelude::*;
use std::f32::consts::PI;
use std::sync::Arc;

// holds the dsp state for iir filters per channel
#[derive(Clone, Copy, Default)]
struct ChannelState {
    hpf_in_y: f32,
    hpf_in_x: f32,
    lpf_y: f32,
    hpf_out_y: f32,
    hpf_out_x: f32,
}

pub struct HarmonicPercolator {
    params: Arc<PercolatorParams>,
    // array to store dsp state for up to 2 channels (stereo)
    states: [ChannelState; 2],
    sample_rate: f32,
}

#[derive(Params)]
struct PercolatorParams {
    // harmonics controls the input gain/drive level
    #[id = "harmonics"]
    pub harmonics: FloatParam,

    // balance controls the output volume level
    #[id = "balance"]
    pub balance: FloatParam,
}

impl Default for HarmonicPercolator {
    fn default() -> Self {
        Self {
            params: Arc::new(PercolatorParams {
                harmonics: FloatParam::new(
                    "Harmonics",
                    0.5, // default value
                    FloatRange::Linear { min: 0.0, max: 1.0 },
                )
                .with_smoother(SmoothingStyle::Linear(30.0)),
                balance: FloatParam::new(
                    "Balance",
                    0.5, // default value
                    FloatRange::Linear { min: 0.0, max: 1.0 },
                )
                .with_smoother(SmoothingStyle::Linear(30.0)),
            }),
            states: [ChannelState::default(); 2],
            sample_rate: 44100.0,
        }
    }
}

impl Plugin for HarmonicPercolator {
    const NAME: &'static str = env!("CARGO_PKG_NAME");
    const VENDOR: &'static str = "ECHOSYSTEM";
    const URL: &'static str = "https://echosystem.wroof.net";
    const EMAIL: &'static str = "echosystem@wroof.net";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(2),
            main_output_channels: NonZeroU32::new(2),
            aux_input_ports: &[],
            aux_output_ports: &[],
            names: PortNames::const_default(),
        },
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(1),
            main_output_channels: NonZeroU32::new(1),
            ..AudioIOLayout::const_default()
        },
    ];
    type SysExMessage = ();
    type BackgroundTask = ();
    type Editor = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn activate(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl ActivateContext<Self>,
    ) -> bool {
        // updates sample rate when plugin is loaded or project rate changes
        self.sample_rate = buffer_config.sample_rate;
        true
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let lpf_coeff = calc_lpf_coeff(3000.0, self.sample_rate);
        let hpf_coeff = calc_hpf_coeff(20.0, self.sample_rate);

        for (ch, channel_samples) in buffer.iter_samples().enumerate() {
            // restricts index to 1 for safety in case of surround formats
            let state = &mut self.states[ch.min(1)];

            let harmonics_gain = self.params.harmonics.smoothed.next() * 15.0;
            let balance_gain = self.params.balance.smoothed.next();

            for sample in channel_samples {
                let mut sig = *sample;

                // input hpf to emulate input coupling capacitor
                let in_hpf = sig - state.hpf_in_x + hpf_coeff * state.hpf_in_y;
                state.hpf_in_x = sig;
                state.hpf_in_y = in_hpf;
                sig = in_hpf;

                // input gain stage driven by harmonics parameter
                sig *= harmonics_gain;

                // lpf to emulate germanium transistor bandwidth limitation
                sig = (1.0 - lpf_coeff) * sig + lpf_coeff * state.lpf_y;
                state.lpf_y = sig;

                // asymmetric soft clipping with bias to emulate transistor starvation
                let bias = 1.2;
                let biased = sig + bias;
                sig = biased / (1.0 + biased.abs()) - (bias / (1.0 + bias.abs()));

                // output hpf to remove dc offset introduced by asymmetric clipping
                let out_hpf = sig - state.hpf_out_x + hpf_coeff * state.hpf_out_y;
                state.hpf_out_x = sig;
                state.hpf_out_y = out_hpf;
                sig = out_hpf;

                // applies final output level control
                *sample = sig * balance_gain;
            }
        }
        ProcessStatus::Normal
    }

    fn deactivate(&mut self) {}
}

// calculates one-pole lpf coefficient based on cutoff and sample rate
fn calc_lpf_coeff(fc: f32, fs: f32) -> f32 {
    (-2.0 * PI * fc / fs).exp()
}

// calculates one-pole hpf coefficient for dc blocking
fn calc_hpf_coeff(fc: f32, fs: f32) -> f32 {
    (-2.0 * PI * fc / fs).exp()
}

impl Vst3Plugin for HarmonicPercolator {
    const VST3_CLASS_ID: [u8; 16] = *b"harmonpercolator";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Tools];
}
nice_export_vst3!(HarmonicPercolator);
