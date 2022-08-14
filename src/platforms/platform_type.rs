#[derive(Clone,Copy)]
pub enum PlatformType{
    Amazon,
    EpicGames,
    Flatpak,
    Gog,
    Heroic,
    Itch,
    Legendary,
    Lutris,
    Origin,
    UPlay
}

impl PlatformType{
    pub fn name(&self) -> &str{
        match self {
            PlatformType::Amazon => "Amazon",
            PlatformType::EpicGames => "EpicGames",
            PlatformType::Flatpak => "Flatpak",
            PlatformType::Gog => "Gog",
            PlatformType::Heroic => "Heroic",
            PlatformType::Itch => "Itch",
            PlatformType::Legendary => "Legendary",
            PlatformType::Lutris => "Lutris",
            PlatformType::Origin => "Origin",
            PlatformType::UPlay => "UPlay",
        }
    }
}