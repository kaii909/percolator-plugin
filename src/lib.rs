use crate::editor::{PercolatorEditor, INITIAL_ZOOM_FACTOR, MIN_WINDOW_SIZE, RESIZE_HINT};

use dsp::PercolatorDSP;
use nice_plug::prelude::*;
use nice_plug_egui::{
    EguiEditor, EguiEditorState, EguiNiceSettings, RepaintNotifier,
    create_egui_editor,
};
use std::sync::Arc;

mod editor;

// use nice_plug_egui::EguiEditor;

mod components;
mod dsp;

// parameters ====================================================================================

#[derive(Params)]
struct PercolatorParams {
    #[id = "harmonics"]
    pub harmonics: FloatParam,

    #[id = "balance"]
    pub balance: FloatParam,

    #[id = "character"]
    pub character: FloatParam,
    // #[id = "feedback"]
    // pub feedback: FloatParam,
}

impl Default for PercolatorParams {
    fn default() -> Self {
        Self {
            harmonics: FloatParam::new(
                "harmonics",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            balance: FloatParam::new(
                "balance",
                0.25,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            character: FloatParam::new(
                "character",
                0.2,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            // feedback: FloatParam::new("feedback", 0.15, FloatRange::Linear { min: 0.0, max: 1.0 })
            //     .with_smoother(SmoothingStyle::Linear(10.0)),
        }
    }
}

// ===============================================================================================

pub struct HarmonicPercolator {
    params: Arc<PercolatorParams>,

    // ui stuff =========
    editor_state: Arc<EguiEditorState>,

    /// A realtime-safe handle to request a repaint & update for an egui app.
    ///
    /// This can be used, for example, to notify the GUI that the value of a decibel
    /// meter has changed.
    repaint_notifier: RepaintNotifier,

    /// Temporarily hold on to the initial GUI state until the editor is first opened.
    initial_editor: Option<PercolatorEditor>,
    // ==================
    dsp_left: PercolatorDSP,
    dsp_right: PercolatorDSP,
}

impl Default for HarmonicPercolator {
    fn default() -> Self {
        let params = Arc::new(PercolatorParams::default());

        // If you wish to make the zoom factor user-configurable, then it should be loaded
        // from a config file to make it persistent. The window size however does not need
        // to be stored in a config file because hosts already keep track of that.
        let zoom_factor = INITIAL_ZOOM_FACTOR;

        let initial_editor = PercolatorEditor {
            open_state: None,
            track_info: None,
            params: params.clone(),
            zoom_factor,
        };

        Self {
            params,

            editor_state: EguiEditorState::from_size(MIN_WINDOW_SIZE, zoom_factor),

            repaint_notifier: RepaintNotifier::new(),

            initial_editor: Some(initial_editor),

            dsp_left: PercolatorDSP::default(),
            dsp_right: PercolatorDSP::default(),
        }
    }
}

impl Plugin for HarmonicPercolator {
    const NAME: &'static str = "ES Percolator";
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
    type Editor = EguiEditor<editor::PercolatorEditor>;

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Self::Editor> {
        create_egui_editor(
            self.editor_state.clone(),
            self.repaint_notifier.clone(),
            EguiNiceSettings::new().with_resize_hint(RESIZE_HINT),
            self.initial_editor.take().unwrap(),
        )
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
                for sample in mono.iter_mut() {
                    let h = self.params.harmonics.smoothed.next();
                    let b = self.params.balance.smoothed.next();
                    let s = self.params.character.smoothed.next();
                    // let f = self.params.feedback.smoothed.next();
                    *sample = self.dsp_left.filter(h, s, b, *sample);
                }
            }
            [left, right] => {
                for (l, r) in left.iter_mut().zip(right.iter_mut()) {
                    let h = self.params.harmonics.smoothed.next();
                    let b = self.params.balance.smoothed.next();
                    let s = self.params.character.smoothed.next();
                    // let f = self.params.feedback.smoothed.next();
                    *l = self.dsp_left.filter(h, s, b, *l);
                    *r = self.dsp_right.filter(h, s, b, *r);
                }
            }
            _ => {}
        }
        ProcessStatus::Normal
    }
    fn deactivate(&mut self) {}
}

impl Vst3Plugin for HarmonicPercolator {
    const VST3_CLASS_ID: [u8; 16] = *b"harmonpercolato2";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Tools];
}
nice_export_vst3!(HarmonicPercolator);
