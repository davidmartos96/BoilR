use dashmap::DashMap;
use eframe::{egui, App, Frame};
use std::{error::Error, path::PathBuf, str::FromStr, sync::Arc};
use steam_shortcuts_util::shortcut::ShortcutOwned;
use tokio::{
    runtime::Runtime,
    sync::watch::{self, Receiver},
};

use crate::{
    settings::Settings,
    steam::{get_shortcuts_paths, SteamUsersInfo},
    steamgriddb::ImageType,
};

use super::{texture_state::TextureState, ui_images::get_logo_icon, FetchStatus, SyncActions};

type ImageMap = std::sync::Arc<DashMap<String, TextureState>>;

pub struct NewUiApp {
    pub(crate) sync_actions: Receiver<FetchStatus<SyncActions<ShortcutOwned>>>,
    pub(crate) settings: Settings,
    pub(crate) rt: Runtime,
    pub(crate) shortcut_thumbnails: ImageMap,
    pub(crate) steam_users: Option<Vec<SteamUsersInfo>>,
    pub(crate) settings_error_message: Option<String>,
    pub(crate) selected_steam_user: Option<SteamUsersInfo>,
}

impl App for NewUiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        self.ensure_steam_users_loaded();

        egui::CentralPanel::default().show(&ctx, |ui| {
            self.render_steam_users_select(ui);
            self.ensure_games_loaded();

            ui.add(egui::Label::new("Hello BoilR"));

            if let Some(steam_user) = self.selected_steam_user.as_ref() {
                let borrowed_actions = &*self.sync_actions.borrow();
                match borrowed_actions {
                    FetchStatus::NeedsFetched => {
                        ui.heading("Need to find games");
                    }
                    FetchStatus::Fetching => {
                        ui.heading("Finding games");
                    }
                    FetchStatus::Fetched(sync_actions) => {
                        render_sync_actions(
                            ui,
                            sync_actions,
                            &steam_user,
                            &mut self.shortcut_thumbnails,
                        );
                    }
                }
            }
        });
    }
}

impl NewUiApp {
    fn render_steam_users_select(&mut self, ui: &mut egui::Ui) {
        if let Some(steam_users) = &mut self.steam_users {
            if steam_users.len() == 1 && self.selected_steam_user.is_none() {
                self.selected_steam_user = Some(steam_users[0].clone());
            } else {
                if self.selected_steam_user.is_none() {
                    self.selected_steam_user = Some(steam_users[0].clone());
                }
                egui::ComboBox::from_label("Select a steam user")
                    .selected_text(format!(
                        "{}",
                        self.selected_steam_user
                            .as_ref()
                            .map(|s| s.user_id.clone())
                            .unwrap_or_default()
                    ))
                    .show_ui(ui, |ui| {
                        for steam_user in steam_users {
                            ui.selectable_value(
                                &mut self.selected_steam_user,
                                Some(steam_user.clone()),
                                steam_user.user_id.clone(),
                            );
                        }
                    });
            }
        }
    }
}

fn render_sync_actions(
    ui: &mut egui::Ui,
    sync_actions: &SyncActions<ShortcutOwned>,
    steam_user: &SteamUsersInfo,
    image_map: &mut ImageMap,
) -> egui::Response {
    ui.heading("To Add");
    ui.horizontal_wrapped(|ui| {
        for shortcut in &sync_actions.add {
            let app_id = shortcut.app_id;
            let extensions = ["png", "jpg", "jpeg"];
            let image_path_op = extensions
                .iter()
                .map(|ext| ImageType::Grid.file_name(app_id, ext))
                .map(|path_str| steam_user.get_images_folder().join(&path_str))
                .filter(|p| p.exists())
                .map(|path| path.to_string_lossy().to_string())
                .next();
            match image_path_op {
                Some(known_image )=> {
                    match image_map.get(&known_image){
                        Some(known_textute) => {
                            match known_textute.to_owned(){
                                TextureState::Downloading => {
                                    //already loading
                                },
                                TextureState::Downloaded => {                                    
                                    //It is loaded, time to put it in textue
                                },
                                TextureState::Loaded(textute) => {
                                    //Textute loaded, just show it

                                },
                            }
                        }                        
                        None => {
                            //need to start load
                            
                        },
                    }
                },
                None => {
                    //Make text grap                    
                    ui.label(shortcut.app_name.as_str());
                },
            }

            ui.label(&shortcut.app_name);
        }
    })
    .response
}

impl NewUiApp {
    pub fn new() -> Self {
        let runtime = Runtime::new().unwrap();
        NewUiApp {
            sync_actions: watch::channel(FetchStatus::NeedsFetched).1,
            rt: runtime,
            settings: Settings::new().expect("We must be able to load our settings"),
            shortcut_thumbnails: Arc::new(DashMap::new()),
            steam_users: None,
            settings_error_message: None,
            selected_steam_user: None,
        }
    }

    pub fn ensure_games_loaded(&mut self) {
        if self.sync_actions.borrow().needs_fetching() {
            let (tx, rx) = watch::channel(FetchStatus::NeedsFetched);
            self.sync_actions = rx;
            let settings = self.settings.clone();
            if let Some(selected_user) = self.selected_steam_user.as_ref() {
                let user = selected_user.clone();
                self.rt.spawn_blocking(move || {
                    let _ = tx.send(FetchStatus::Fetching);
                    let sync_actions = get_sync_actions(&settings, &user);
                    let _ = tx.send(FetchStatus::Fetched(sync_actions));
                });
            }
        }
    }

    fn ensure_steam_users_loaded(&mut self) {
        if self.settings_error_message.is_none() && self.steam_users.is_none() {
            let paths = get_shortcuts_paths(&self.settings.steam);
            match paths {
                Ok(paths) => self.steam_users = Some(paths),
                Err(err) => {
                    self.settings_error_message = Some(format!("Could not find user steam location, error message: {} , try to clear the steam location field in settings to let BoilR find it itself",err));
                }
            }
        }
    }
}

fn get_sync_actions(
    settings: &Settings,
    steam_user: &SteamUsersInfo,
) -> SyncActions<ShortcutOwned> {
    let platform_games = crate::sync::get_platform_shortcuts(settings);

    let mut sync_actions = SyncActions::new();
    for (_platform, games) in platform_games {
        for game in games {
            sync_actions.add.push(game);
        }
    }
    sync_actions
}

pub fn run_new_ui(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let app = NewUiApp::new();
    let no_v_sync = args.contains(&"--no-vsync".to_string());
    let native_options = eframe::NativeOptions {
        initial_window_size: Some(egui::Vec2 { x: 1280., y: 800. }),
        icon_data: Some(get_logo_icon()),
        vsync: !no_v_sync,
        ..Default::default()
    };
    eframe::run_native("BoilR", native_options, Box::new(|cc| Box::new(app)));
}
