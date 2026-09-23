use nice_plug::prelude::*;
use std::sync::Arc;

pub struct HarmonicPercolator {
    params: Arc<PercolatorParams>,
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
        _buffer_config: &BufferConfig,
        _context: &mut impl ActivateContext<Self>,
    ) -> bool
    {
        true
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        for channel_samples in buffer.iter_samples() {
            let harmonics_gain = self.params.harmonics.smoothed.next() * 10.0;
            let balance_gain = self.params.balance.smoothed.next();

            for sample in channel_samples {
                // apply input gain stage (simulates transistor drive)
                let driven = *sample * harmonics_gain;

                // apply asymmetric soft clipping (simulates diode clipping)
                let clipped = asymmetric_soft_clip(driven);

                // apply output level control
                *sample = clipped * balance_gain;
            }
        }
        ProcessStatus::Normal
    }

    fn deactivate(&mut self) {}
}

// asymmetric soft clipping function to emulate germanium/silicon diode behavior
fn asymmetric_soft_clip(x: f32) -> f32 {
    if x >= 0.0 {
        // positive half uses softer germanium-like saturation
        x / (1.0 + x * 0.5)
    } else {
        // negative half uses harder silicon-like clipping
        x / (1.0 - x * 0.7)
    }
}

impl Vst3Plugin for HarmonicPercolator {
    const VST3_CLASS_ID: [u8; 16] = *b"harmonpercolator";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Tools];
}

nice_export_vst3!(HarmonicPercolator);
