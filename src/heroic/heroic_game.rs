use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use steam_shortcuts_util::{shortcut::ShortcutOwned, Shortcut};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HeroicGame {
    pub app_name: String,
    pub title: String,
    pub is_dlc: bool,
    pub install_path: String,
    pub executable: String,
    pub launch_parameters: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HeroicGameExtended {
    #[serde(rename = "audioFix")]
    audio_fix: Option<bool>,

    #[serde(rename = "autoSyncSaves")]
    auto_sync_saves: Option<bool>,

    #[serde(rename = "savesPath")]
    saves_path: Option<String>,

    #[serde(rename = "enableEsync")]
    enable_esync: Option<bool>,

    #[serde(rename = "enableFsync")]
    enable_fsync: Option<bool>,

    #[serde(rename = "enableFSR")]
    enable_fsr: Option<bool>,

    #[serde(rename = "maxSharpness")]
    max_sharpness: Option<bool>,

    #[serde(rename = "enableResizableBar")]
    enable_resizable_bar: Option<bool>,

    #[serde(rename = "nvidiaPrime")]
    nvidia_prime: Option<bool>,

    #[serde(rename = "offlineMode")]
    offline_mode: Option<bool>,

    #[serde(rename = "showFps")]
    show_fps: Option<bool>,

    #[serde(rename = "showMangehud")]
    show_mangehud: Option<bool>,

    #[serde(rename = "useGameMode")]
    use_game_mode: Option<bool>,

    #[serde(rename = "launcherArgs")]
    launcher_args: Option<String>,

    #[serde(rename = "otherOptions")]
    other_options: Option<String>,

    #[serde(rename = "targetExe")]
    target_exe: Option<String>,

    #[serde(rename = "useSteamRuntime")]
    use_steam_runtime: Option<bool>,

    #[serde(rename = "winePrefix")]
    wine_prefix: Option<String>,

    #[serde(rename = "wineVersion")]
    wine_version: Option<WineVersion>,

    #[serde(rename = "altLegendaryBin")]
    alt_legendary_bin: Option<String>,

    #[serde(rename = "altGogdlBin")]
    alt_gogdl_bin: Option<String>,

    #[serde(rename = "egsLinkedPath")]
    egs_linked_path: Option<String>,

    #[serde(rename = "maxRecentGames")]
    max_recent_games: Option<usize>,

    #[serde(rename = "checkUpdateInterval")]
    check_update_interval: Option<usize>,

    #[serde(rename = "enable_updates")]
    enable_udates: Option<bool>,

    #[serde(rename = "appName")]
    app_name: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WineVersion {
    bin: Option<String>,
    name: Option<String>,

    #[serde(rename = "type")]
    wine_type: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
enum HeroicExtendedFields {
    Game(HeroicGameExtended),
    String(String),
    Bool(bool),
}

impl HeroicGame {
    pub fn is_installed(&self) -> bool {
        Path::new(&self.install_path)
            .join(&self.executable)
            .exists()
    }
}

impl From<HeroicGame> for ShortcutOwned {
    fn from(game: HeroicGame) -> Self {
        let target_path = Path::new(&game.install_path).join(game.executable);

        #[cfg(target_family = "unix")]
        let mut target = target_path.to_string_lossy().to_string();
        #[cfg(target_family = "unix")]
        {
            if !target.starts_with('\"') && !target.ends_with('\"') {
                target = format!("\"{}\"", target);
            }
        }

        #[cfg(target_family = "unix")]
        let mut install_path = game.install_path.to_string();
        #[cfg(target_family = "unix")]
        {
            if !install_path.starts_with('\"') && !install_path.ends_with('\"') {
                install_path = format!("\"{}\"", install_path);
            }
        }

        let shortcut = Shortcut::new(
            "0",
            game.title.as_str(),
            &target,
            &install_path,
            &target,
            "",
            game.launch_parameters.as_str(),
        );
        let mut owned_shortcut = shortcut.to_owned();
        owned_shortcut.tags.push("Heroic".to_owned());
        owned_shortcut.tags.push("Ready TO Play".to_owned());
        owned_shortcut.tags.push("Installed".to_owned());

        owned_shortcut
    }
}

pub fn parse_json(json: &str) -> Option<HeroicGameExtended> {
    let json_result = serde_json::from_str::<HashMap<String, HeroicExtendedFields>>(json);
    if !json_result.is_ok() {
        println!("{:?}", json_result);
        return None;
    }
    let game_map = json_result.unwrap();
    for (key, value) in game_map.iter() {
        match value {
            HeroicExtendedFields::Game(val) => {
                let mut res = val.clone();
                res.app_name = Some(key.clone());
                return Some(res);
            }
            HeroicExtendedFields::String(_) => {}
            HeroicExtendedFields::Bool(_) => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_parse_extended() {
        let json = include_str!("../testdata/heroic_extened_game.json");
        let game = parse_json(json);
        let game = game.unwrap();
        let app_name = "9b40e3ffb4074f22a856a521be5ce858";
        assert_eq!(app_name, game.app_name.unwrap().as_str());
        assert_eq!(false, game.nvidia_prime.unwrap());
    }
}
