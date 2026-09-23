use nice_plug::prelude::*;

use dsp::PercolatorDSP;
use std::sync::Arc;

mod dsp;

pub struct HarmonicPercolator {
    params: Arc<PercolatorParams>,
    dsp: PercolatorDSP,
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
                .with_smoother(SmoothingStyle::Linear(10.0)),
                balance: FloatParam::new(
                    "Balance",
                    0.5, // default value
                    FloatRange::Linear { min: 0.0, max: 1.0 },
                )
                .with_smoother(SmoothingStyle::Linear(10.0)),
            }),
            dsp: PercolatorDSP::default(),
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
        self.dsp.set_sr(buffer_config.sample_rate);
        true
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        for (ch, channel_slice) in buffer.as_slice().iter_mut().enumerate() {
            let harmonics = &self.params.harmonics;
            let balance = &self.params.balance;
            self.dsp
            .process_sample(channel_slice, ch, harmonics, balance);
        }
        ProcessStatus::Normal
    }

    fn deactivate(&mut self) {}
}

impl Vst3Plugin for HarmonicPercolator {
    const VST3_CLASS_ID: [u8; 16] = *b"harmonpercolator";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Tools];
}
nice_export_vst3!(HarmonicPercolator);
