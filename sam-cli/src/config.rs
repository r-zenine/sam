use crate::cli::CLISettings;
pub use sam_config::{AppSettings, ErrorsSettings};

pub type Result<T> = std::result::Result<T, ErrorsSettings>;

pub fn load_with_cli(cli_settings: Option<CLISettings>) -> Result<AppSettings> {
    let mut settings = AppSettings::load()?;
    if let Some(m) = cli_settings {
        settings.dry = m.dry;
        settings.silent = m.silent;
        settings.no_cache = m.no_cache;
        settings.defaults = m.default_choices.0;
    }
    Ok(settings)
}
