mod plugin;
mod project_state;


mod engine {
    pub mod echo_detect;
    pub mod fx;
    pub mod kit;
    pub mod master_bus;
    pub mod sampler;
    pub mod sequencer;
    pub mod step_smoother;
}

mod analysis {
    pub mod cache;
    pub mod features;
    pub mod library;
    pub mod scanner;
}

mod ui {
    pub mod dialogs;
    pub mod editor;
    pub mod folder_browser;
    pub mod knob;
    pub mod pad_row;
    pub mod sample_map;
    pub mod sequencer_ui;
    pub mod state;
    pub mod theme;
    pub mod toolbar;
    pub mod waveform;
}

mod util {
    pub mod audio_file;
    pub mod config;
    pub mod default_kit;
    pub mod history;
    pub mod preset;
}

pub const NAME: &str = "ML Drums";
pub const ID: &str = "de.michaellane.ml-drums";
pub const VISIBILITY: &str = "visible_plugin-drums";
os_plugkit::linked_clap_entry!(plugin::Autokit);
