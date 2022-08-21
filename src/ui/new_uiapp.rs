use dashmap::DashMap;
use eframe::{egui, App, Frame};
use egui::{Image, ImageButton, Label, Pos2, Rect, ScrollArea};
use std::{collections::HashMap, error::Error, sync::Arc};
use steam_shortcuts_util::shortcut::ShortcutOwned;
use tokio::{
    runtime::Runtime,
    sync::watch::{self, Receiver},
};

use crate::platforms::{
    egs::ManifestItem,
    heroic::HeroicGameType,
    PlatformType,
};

use crate::{
    settings::Settings,
    steam::{get_shortcuts_for_user, get_shortcuts_paths, SteamUsersInfo},
    steamgriddb::ImageType,
};

use super::{shortcuts::SyncActions, image_handling::FetchStatus};


type ImageMap = std::sync::Arc<DashMap<String, Option<egui::TextureHandle>>>;

pub enum Menu {
    Shortcuts,
    Settings,
    Shortcut((PlatformType, ShortcutOwned)),
    // ImageDownload((PlatformInfo, ShortcutOwned,ImageType))
    // backup
}

pub struct NewUiApp {
    pub(crate) sync_actions: Receiver<FetchStatus<SyncActions<(PlatformType, ShortcutOwned)>>>,
    pub(crate) settings: Settings,
    pub(crate) rt: Runtime,
    pub(crate) image_map: ImageMap,
    pub(crate) steam_users: Option<Vec<SteamUsersInfo>>,
    pub(crate) settings_error_message: Option<String>,
    pub(crate) selected_steam_user: Option<SteamUsersInfo>,
    pub(crate) menu: Menu,
    pub(crate) epic_manifests: Option<Vec<ManifestItem>>,
    pub(crate) heroic_games: Option<Vec<HeroicGameType>>,
}

impl App for NewUiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        self.ensure_steam_users_loaded();

        egui::TopBottomPanel::top("top-panel").show(&ctx, |ui| {
            ui.horizontal(|ui| {
                match self.menu {
                    Menu::Shortcuts => {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Settings").clicked() {
                                self.menu = Menu::Settings;
                            }
                        });
                    }
                    Menu::Settings => {
                        if ui.button("<-").clicked() {
                            self.menu = Menu::Shortcuts;
                        }
                    }
                    Menu::Shortcut(_) => {
                        if ui.button("<-").clicked() {
                            self.menu = Menu::Shortcuts;
                        }
                    }
                };
            });
        });
        egui::CentralPanel::default().show(&ctx, |ui| match self.menu {
            Menu::Shortcuts => {
                self.render_shortcut_select(ui);
            }
            Menu::Settings => self.render_settings(ui),
            Menu::Shortcut(_) => self.render_shortcut(ui),
        });

        egui::TopBottomPanel::bottom("bottom-panel").show(&ctx, |ui| {
            ui.with_layout(
                egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                |ui| {
                    match self.menu {
                        Menu::Shortcuts => {
                            let sync_label = if self.settings.steamgrid_db.auth_key.is_some() {
                                "Import Shortcuts & Download Images"
                            } else {
                                "Import Shortcuts"
                            };
                            if ui.button(sync_label).clicked() {
                                //TODO
                                println!("should sync");
                            }
                        }
                        Menu::Settings => {
                            if ui.button("Save Settings").clicked() {
                                //TODO
                                println!("Should save");
                            }
                        }
                        Menu::Shortcut(_) => {}
                    };
                },
            );
        });
    }
}

impl NewUiApp {
    fn render_shortcut_select(&mut self, ui: &mut egui::Ui) {
        self.render_steam_users_select(ui);
        self.ensure_games_loaded();
        ScrollArea::vertical()
            .stick_to_right(true)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                if let Some(steam_user) = self.selected_steam_user.as_ref() {
                    if let Ok(true) = self.sync_actions.has_changed() {
                        self.image_map.clear();
                    }
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
                                &mut self.image_map,
                                &mut self.menu,
                            );
                        }
                    }
                }
            });
    }

    fn render_shortcut(&mut self, ui: &mut egui::Ui) {
        if let Menu::Shortcut((platform, shortcut)) = &self.menu {
            ui.heading(shortcut.app_name.as_str());
            ui.label(format!("Launcher: {}",platform.name()));
        }
    }
}

impl NewUiApp {
    fn render_steam_users_select(&mut self, ui: &mut egui::Ui) {
        if let Some(steam_users) = &mut self.steam_users {
            if steam_users.len() >= 1 && self.selected_steam_user.is_none() {
                self.selected_steam_user = Some(steam_users[0].clone());
            }
            if steam_users.len() > 1 {
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

const ICON_MAX_WIDTH: f32 = 35.;

const MAX_WIDTH: f32 = 125.;
const RATIO: f32 = 9.0 / 6.0;

fn render_sync_actions(
    ui: &mut egui::Ui,
    sync_actions: &SyncActions<(PlatformType, ShortcutOwned)>,
    steam_user: &SteamUsersInfo,
    image_map: &mut ImageMap,
    menu: &mut Menu,
) {
    render_shortcuts(
        "Shortcuts to add",
        ui,
        &sync_actions.add,
        steam_user,
        image_map,
        menu,
    );
    render_shortcuts(
        "Shortcuts to download images for",
        ui,
        &sync_actions.image_download,
        steam_user,
        image_map,
        menu,
    );
    render_shortcuts(
        "Shortcuts to remove",
        ui,
        &sync_actions.delete,
        steam_user,
        image_map,
        menu,
    );
    render_shortcuts(
        "Shortcuts to update",
        ui,
        &sync_actions.update,
        steam_user,
        image_map,
        menu,
    );
    render_shortcuts(
        "Shortcuts that will be untouched",
        ui,
        &sync_actions.none,
        steam_user,
        image_map,
        menu,
    );
}

fn render_shortcuts(
    header: &str,
    ui: &mut egui::Ui,
    to_render: &Vec<(PlatformType, ShortcutOwned)>,
    steam_user: &SteamUsersInfo,
    image_map: &mut ImageMap,
    menu: &mut Menu,
) {
    if !to_render.is_empty() {
        ui.heading(header);
        ui.horizontal_wrapped(|ui| {
            for (platform, shortcut) in to_render {
                let platform_name = platform.name();

                let app_id = shortcut.app_id;
                let image_key = &format!("{},{}", steam_user.user_id, app_id);

                if !image_map.contains_key(image_key) {
                    let extensions = ["png", "jpg", "jpeg"];
                    let handle = extensions
                        .iter()
                        .map(|ext| ImageType::Grid.file_name(app_id, ext))
                        .map(|path_str| steam_user.get_images_folder().join(&path_str))
                        .filter(|p| p.exists())
                        .map(|p| {
                            (
                                p.to_string_lossy().to_string(),
                                super::image_handling::load_image_from_path(&p),
                            )
                        })
                        .find_map(|(path, data)| match data {
                            Some(data) => Some((path, data)),
                            None => None,
                        })
                        .map(|(image_path, image_data)| {
                            ui.ctx().load_texture(image_path, image_data,egui::TextureFilter::Linear)
                        });
                    image_map.insert(image_key.clone(), handle);
                }

                //we just inserted this
                let cached = image_map.get(image_key).unwrap();
                let rect = if let Some(texture_handle) = cached.value() {
                    let mut size = texture_handle.size_vec2();
                    clamp_to_width(&mut size, MAX_WIDTH);
                    let image_button = ImageButton::new(texture_handle, size);
                    let response = ui.add(image_button);
                    if response.clicked() {
                        *menu = Menu::Shortcut((platform.clone(), shortcut.clone()));
                    }
                    response.rect
                } else {
                    egui::Frame::none()
                        .inner_margin(5.0)
                        .show(ui, |ui| {
                            ui.add_sized(
                                [MAX_WIDTH, RATIO * MAX_WIDTH],
                                egui::Button::new(shortcut.app_name.as_str()).wrap(true),
                            )
                        })
                        .response
                        .rect
                };
                if let Some(icon_data) = platform.logo() {
                    if !image_map.contains_key(platform_name) {
                        let image_data = crate::ui::image_handling::load_image_from_mem(icon_data);
                        let handle = ui.ctx().load_texture(platform_name, image_data,egui::TextureFilter::Linear);
                        image_map.insert(platform_name.to_owned(), Some(handle));
                    }
                    if let Some(textue_handle) = image_map.get(platform_name) {
                        if let Some(textue_handle) = textue_handle.value() {
                            let mut size = textue_handle.size_vec2();
                            clamp_to_width(&mut size, ICON_MAX_WIDTH);
                            let logo_image = Image::new(textue_handle, size);
                            let icon_max = size.y;
                            let icon_rect = Rect {
                                min: Pos2 {
                                    x: rect.min.x + 5.0,
                                    y: rect.max.y - icon_max - 5.0,
                                },
                                max: Pos2 {
                                    x: rect.min.x + ICON_MAX_WIDTH,
                                    y: rect.max.y - 5.0,
                                },
                            };
                            ui.put(icon_rect, logo_image);
                        }
                    }
                }
                let center = rect.center();
                let mut dummy_rect = rect.clone();
                dummy_rect.set_height(MAX_WIDTH * RATIO + 7.);
                dummy_rect.set_width(MAX_WIDTH);
                dummy_rect.set_center(center);
                ui.put(dummy_rect, Label::new(""));
            }
        });
    }
}

fn clamp_to_width(size: &mut egui::Vec2, max_width: f32) {
    let mut x = size.x;
    let mut y = size.y;
    if size.x > max_width {
        let ratio = size.y / size.x;
        x = max_width;
        y = x * ratio;
    }
    size.x = x;
    size.y = y;
}

impl NewUiApp {
    pub fn new() -> Self {
        let runtime = Runtime::new().unwrap();
        NewUiApp {
            sync_actions: watch::channel(FetchStatus::NeedsFetched).1,
            rt: runtime,
            settings: Settings::new().expect("We must be able to load our settings"),
            image_map: Arc::new(DashMap::new()),
            steam_users: None,
            settings_error_message: None,
            selected_steam_user: None,
            menu: Menu::Shortcuts,
            epic_manifests: None,
            heroic_games: None,
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
                    println!("Found shortcuts");
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
) -> SyncActions<(PlatformType, ShortcutOwned)> {
    let platform_shortcuts = crate::sync::get_platform_shortcuts(settings);
    let exsisting_shortcuts = get_shortcuts_for_user(steam_user);
    let mut sync_actions = SyncActions::new();
    let known_images = crate::steam::get_users_images(steam_user).unwrap_or_default();

    let types = vec![
        ImageType::Logo,
        ImageType::Hero,
        ImageType::Grid,
        ImageType::WideGrid,
        ImageType::Icon,
    ];
    let known_app_ids: Vec<u32> = exsisting_shortcuts
        .shortcuts
        .iter()
        .map(|s| s.app_id)
        .collect();

    let mut app_id_platform_map = HashMap::new();

    for (platform, games) in platform_shortcuts {
        for game in games {
            app_id_platform_map.insert(game.app_id, platform);
            if !known_app_ids.contains(&game.app_id) {
                sync_actions.add.push((platform, game));
            }
        }
    }

    for shortcut in exsisting_shortcuts.shortcuts.iter() {
        let platform = app_id_platform_map
            .get(&shortcut.app_id)
            .unwrap_or(&PlatformType::Unknown);
        if types
            .iter()
            .map(|t| t.file_name_no_extension(shortcut.app_id))
            .any(|image| !known_images.contains(&image))
        {
            sync_actions
                .image_download
                .push((*platform, shortcut.to_owned()));
        } else {
            sync_actions.none.push((*platform, shortcut.to_owned()));
        }
    }
    sync_actions
}

pub fn run_new_ui(args: Vec<String>){
    let app = NewUiApp::new();
    let no_v_sync = args.contains(&"--no-vsync".to_string());
    let native_options = eframe::NativeOptions {
        initial_window_size: Some(egui::Vec2 { x: 1280., y: 800. }),
        icon_data: Some(super::image_handling::get_logo_icon()),
        vsync: !no_v_sync,
        ..Default::default()
    };
    eframe::run_native(
        "BoilR",
        native_options,
        Box::new(|cc| {
            setup(&cc.egui_ctx);
            Box::new(app)
        }),
    );
}

fn setup(ctx: &egui::Context) {
    #[cfg(target_family = "unix")]
    ctx.set_pixels_per_point(1.0);

    let mut style: egui::Style = (*ctx.style()).clone();
    super::style::set_style(&mut style);
    ctx.set_style(style);
}
