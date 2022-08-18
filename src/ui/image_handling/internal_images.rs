use crate::platforms::PlatformType;

//UI Images
pub const IMPORT_GAMES_IMAGE: &[u8] = include_bytes!("../../../resources/import_games_button.png");
pub const SAVE_IMAGE: &[u8] = include_bytes!("../../../resources/save.png");
pub const LOGO_32: &[u8] = include_bytes!("../../../resources/logo32.png");
pub const LOGO_ICON: &[u8] = include_bytes!("../../../resources/logo_small.png");

//Platform Logos
const EPIC_LOGO: &[u8] = include_bytes!("../../../resources/platformlogos/epic.png");
const HEROIC_LOGO: &[u8] = include_bytes!("../../../resources/platformlogos/heroic.png");
const AMAZON_LOGO: &[u8] = include_bytes!("../../../resources/platformlogos/amazon.png");
const ORIGIN_LOGO: &[u8] = include_bytes!("../../../resources/platformlogos/origin.png");
const ITCH_LOGO: &[u8] = include_bytes!("../../../resources/platformlogos/itchio.png");
const GOG_LOGO: &[u8] = include_bytes!("../../../resources/platformlogos/gog.png");
const FLATPAK_LOGO: &[u8] = include_bytes!("../../../resources/platformlogos/Flatpak_logo.png");

impl PlatformType {
    pub fn logo(&self) -> Option<&[u8]> {
        match self {
            PlatformType::Amazon => Some(AMAZON_LOGO),
            PlatformType::EpicGames => Some(EPIC_LOGO),
            PlatformType::Flatpak => Some(FLATPAK_LOGO),
            PlatformType::Gog => Some(GOG_LOGO),
            PlatformType::Heroic => Some(HEROIC_LOGO),
            PlatformType::Itch => Some(ITCH_LOGO),
            PlatformType::Origin => Some(ORIGIN_LOGO),
            _ => None,
        }
    }
}
