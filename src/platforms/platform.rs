use steam_shortcuts_util::shortcut::ShortcutOwned;

use super::PlatformType;

pub trait Platform<T, E>
where
    T: Into<ShortcutOwned>,
{
    fn enabled(&self) -> bool;

    fn get_shortcuts(&self) -> Result<Vec<T>, E>;

    fn settings_valid(&self) -> SettingsValidity;

    #[cfg(target_family = "unix")]
    fn create_symlinks(&self) -> bool;

    // HOME/.local/share/Steam/config/config.vdf
    fn needs_proton(&self, input: &T) -> bool;

    fn platform_type(&self) -> PlatformType;
}

pub enum SettingsValidity {
    Valid,
    Invalid { reason: String },
}
