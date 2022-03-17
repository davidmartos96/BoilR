use super::{LegendaryGame, LegendarySettings};
use crate::platform::{Platform, SettingsValidity};
use serde_json::from_str;
use std::error::Error;
use std::process::Command;

pub struct LegendaryPlatform {
    settings: LegendarySettings,
}

impl LegendaryPlatform {
    pub fn new(settings: LegendarySettings) -> LegendaryPlatform {
        Self { settings }
    }
}

impl Platform<LegendaryGame, Box<dyn Error>> for LegendaryPlatform {
    fn enabled(&self) -> bool {
        self.settings.enabled
    }

    fn name(&self) -> &str {
        "Legendary"
    }

    fn get_shortcuts(&self) -> Result<Vec<LegendaryGame>, Box<dyn Error>> {
        let legendary_string = self
            .settings
            .executable
            .clone()
            .unwrap_or("legendary".to_string());
        let legendary = legendary_string.as_str();
        execute_legendary_command(legendary)
    }

    #[cfg(target_family = "unix")]
    fn create_symlinks(&self) -> bool {
        false
    }

    fn settings_valid(&self) -> crate::platform::SettingsValidity {
        let shortcuts_res = self.get_shortcuts();
        match shortcuts_res {
            Ok(_) => SettingsValidity::Valid,
            Err(err) => SettingsValidity::Invalid {
                reason: format!("{}", err),
            },
        }
    }
}

fn execute_legendary_command(program: &str) -> Result<Vec<LegendaryGame>, Box<dyn Error>> {
    let legendary_command = Command::new(program)
        .arg("list-installed")
        .arg("--json")
        .output()?;
    let json = String::from_utf8_lossy(&legendary_command.stdout);
    let legendary_ouput = from_str(&json)?;
    Ok(legendary_ouput)
}
