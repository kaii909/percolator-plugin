use dsp::PercolatorDSP;
use nice_plug::prelude::*;
use std::sync::Arc;

mod components;
mod dsp;

pub struct HarmonicPercolator {
    params: Arc<PercolatorParams>,
    dsp_left: PercolatorDSP,
    dsp_right: PercolatorDSP,
}

#[derive(Params)]
struct PercolatorParams {
    #[id = "harmonics"]
    pub harmonics: FloatParam,

    #[id = "balance"]
    pub balance: FloatParam,
}

impl Default for HarmonicPercolator {
    fn default() -> Self {
        Self {
            params: Arc::new(PercolatorParams {
                harmonics: FloatParam::new(
                    "Harmonics",
                    0.5,
                    FloatRange::Linear { min: 0.0, max: 1.0 },
                )
                .with_smoother(SmoothingStyle::Linear(10.0)),
                balance: FloatParam::new("Balance", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 })
                    .with_smoother(SmoothingStyle::Linear(10.0)),
            }),
            dsp_left: PercolatorDSP::default(),
            dsp_right: PercolatorDSP::default(),
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
        self.dsp_left.set_sr(buffer_config.sample_rate);
        self.dsp_right.set_sr(buffer_config.sample_rate);
        true
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        // sample-major loop so smoothers advance once per sample frame
        match buffer.as_slice() {
            [mono] => {
                for s in mono.iter_mut() {
                    let h = self.params.harmonics.smoothed.next();
                    let b = self.params.balance.smoothed.next();
                    *s = self.dsp_left.filter(h, b, *s);
                }
            }
            [left, right] => {
                for (l, r) in left.iter_mut().zip(right.iter_mut()) {
                    let h = self.params.harmonics.smoothed.next();
                    let b = self.params.balance.smoothed.next();
                    *l = self.dsp_left.filter(h, b, *l);
                    *r = self.dsp_right.filter(h, b, *r);
                }
            }
            _ => {}
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
