use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{
    config::get_thumbnails_folder,
    steam::{get_installed_games, SteamGameInfo},
    steam::{get_shortcuts_paths, SteamUsersInfo},
    steamgriddb::{get_image_extension, get_query_type, CachedSearch, ImageType, ToDownload},
    sync::{download_images, SyncProgress},
};
use dashmap::DashMap;
use egui::{Button, Grid, ImageButton, ScrollArea};
use futures::executor::block_on;
use steam_shortcuts_util::shortcut::ShortcutOwned;
use steamgriddb_api::images::MimeTypes;
use tokio::sync::watch::{self, Receiver};

use super::{ui_images::load_image_from_path, FetcStatus, MyEguiApp};

pub type ImageHandlesMap = std::sync::Arc<DashMap<String, TextureState>>;

pub struct ImageSelectState {
    pub selected_shortcut: Option<GameType>,
    pub grid_id: Option<usize>,
    pub steam_user: Option<SteamUsersInfo>,
    pub settings_error: Option<String>,
    pub steam_users: Option<Vec<SteamUsersInfo>>,
    pub user_shortcuts: Option<Vec<ShortcutOwned>>,
    pub game_mode: GameMode,
    pub image_type_selected: Option<ImageType>,
    pub image_options: Receiver<FetcStatus<Vec<PossibleImage>>>,
    pub steam_games: Option<Vec<crate::steam::SteamGameInfo>>,
    pub image_handles: ImageHandlesMap,
    pub possible_names: Option<Vec<steamgriddb_api::search::SearchResult>>,
}

struct ShortcutSelectState{
    user_shortcuts: Vec<ShortcutOwned>,
    steam_users: Vec<SteamUsersInfo>,
    steam_user: SteamUsersInfo,
}

struct SteamGameSelectState{
    steam_games: Vec<crate::steam::SteamGameInfo>,
    steam_users: Vec<SteamUsersInfo>,
    steam_user: SteamUsersInfo,
}


struct ImageTypeSelectState{
    selected_shortcut:GameType,
    steam_user: SteamUsersInfo,
}


struct NameChangeSelectState{
    steam_user: SteamUsersInfo,
    possible_names: Option<Vec<steamgriddb_api::search::SearchResult>>,
}


enum Screen{
    ShortcutSelect(ShortcutSelectState),
    SteamGameSelect(SteamGameSelectState),
    ImageTypeSelect(ImageTypeSelectState),
    ImageSelect(ImageSelectState),
    NameChangeSelect(NameChangeSelectState)
}


#[derive(Clone)]
pub enum TextureState {
    Downloading,
    Downloaded,
    Loaded(egui::TextureHandle),
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameMode {
    Shortcuts,
    SteamGames,
}

impl GameMode {
    pub fn is_shortcuts(&self) -> bool {
        match self {
            GameMode::Shortcuts => true,
            GameMode::SteamGames => false,
        }
    }

    pub fn label(&self) -> &str {
        match self {
            GameMode::Shortcuts => "Images for shortcuts",
            GameMode::SteamGames => "Images for steam games",
        }
    }
}

#[derive(Clone, Debug)]
pub struct PossibleImage {
    thumbnail_path: PathBuf,
    thumbnail_url: String,
    mime: MimeTypes,
    full_url: String,
    id: u32,
}

impl Default for ImageSelectState {
    fn default() -> Self {
        Self {
            selected_shortcut: Default::default(),
            grid_id: Default::default(),
            steam_user: Default::default(),
            steam_users: Default::default(),
            settings_error: Default::default(),
            user_shortcuts: Default::default(),
            game_mode: GameMode::Shortcuts,
            image_type_selected: Default::default(),
            possible_names: None,
            image_options: watch::channel(FetcStatus::NeedsFetched).1,
            image_handles: Arc::new(DashMap::new()),
            steam_games: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum GameType {
    Shortcut(ShortcutOwned),
    SteamGame(SteamGameInfo),
}

impl GameType {
    pub fn app_id(&self) -> u32 {
        match self {
            GameType::Shortcut(shortcut) => shortcut.app_id,
            GameType::SteamGame(game) => game.appid,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            GameType::Shortcut(s) => s.app_name.as_ref(),
            GameType::SteamGame(g) => g.name.as_ref(),
        }
    }
}

#[derive(Debug)]
pub enum UserAction {
    CorrectGridId,
    UserSelected(SteamUsersInfo),
    ShortcutSelected(GameType),
    ImageTypeSelected(ImageType),
    ImageTypeCleared(ImageType, bool),
    ImageSelected(PossibleImage),
    GridIdChanged(usize),
    SetGamesMode(GameMode),
    BackButton,
    NoAction,
    ClearImages,
    DownloadAllImages,
    RefreshImages,
}

impl MyEguiApp {
    fn render_ui_image_action(&self, ui: &mut egui::Ui) -> UserAction {
        let state = &self.image_selected_state;
        if let Some(action) = ui
            .horizontal(|ui| {
                let render_back =
                    state.selected_shortcut.is_some() || state.image_type_selected.is_some();
                if render_back {
                    if ui.button("Back").clicked() {
                        return Some(UserAction::BackButton);
                    }
                    None
                } else {
                    if let Some(value) = render_user_select(state, ui) {
                        return Some(value);
                    }
                    render_shortcut_mode_select(state, ui)
                }
            })
            .inner
        {
            return action;
        }

        if let Some(shortcut) = state.selected_shortcut.as_ref() {
            ui.heading(shortcut.name());

            if let Some(possible_names) = state.possible_names.as_ref() {
                if let Some(value) = render_possible_names(possible_names, ui, state) {
                    return value;
                }
            } else if let Some(image_type) = state.image_type_selected.as_ref() {
                if let Some(action) = self.render_possible_images(ui, image_type, state) {
                    return action;
                }
            } else if let Some(action) = render_shortcut_images(ui, state) {
                return action;
            }
        } else {
            let is_shortcut = state.game_mode.is_shortcuts();
            if is_shortcut {
                if let Some(action) = self.render_shortcut_select(ui) {
                    return action;
                }
            } else if let Some(action) = render_steam_game_select(ui, state) {
                return action;
            }
        }

        match *self.status_reciever.borrow() {
            crate::sync::SyncProgress::FindingImages => {
                ui.spinner();
                ui.label("Finding images to download");
                ui.ctx().request_repaint();
            }
            crate::sync::SyncProgress::DownloadingImages { to_download } => {
                ui.spinner();
                ui.label(format!("Downloading {to_download} images"));
                ui.ctx().request_repaint();
            }
            crate::sync::SyncProgress::Done => {
                ui.ctx().request_repaint();
                return UserAction::RefreshImages;
            }
            _ => {
                if ui.button("Download images for all games").clicked() {
                    return UserAction::DownloadAllImages;
                }
            }
        }

        UserAction::NoAction
    }

    fn render_shortcut_select(&self, ui: &mut egui::Ui) -> Option<UserAction> {
        let shortcuts = &self.image_selected_state.user_shortcuts;

        let width = ui.available_size().x;
        let column_width = 100.;
        let column_padding = 23.;
        let columns = (width / (column_width + column_padding)).floor() as u32;
        let mut cur_column = 0;
        match shortcuts {
            Some(shortcuts) => {
                let user_info = &self.image_selected_state.steam_user.as_ref().unwrap();
                if let Some(action) = egui::Grid::new("ui_images")
                    .show(ui, |ui| {
                        for shortcut in shortcuts {
                            let action = self.render_image(shortcut, user_info, column_width, ui);
                            if action.is_some() {
                                return action;
                            }
                            cur_column += 1;
                            if cur_column >= columns {
                                cur_column = 0;
                                ui.end_row();
                            }
                        }
                        ui.end_row();
                        None
                    })
                    .inner
                {
                    return action;
                }
            }
            None => {
                ui.label("Could not find any shortcuts");
            }
        }
        None
    }

    fn render_image(
        &self,
        shortcut: &ShortcutOwned,
        user_info: &&SteamUsersInfo,
        column_width: f32,
        ui: &mut egui::Ui,
    ) -> Option<Option<UserAction>> {
        let (_, key) = shortcut.key(
            &ImageType::Grid,
            Path::new(&user_info.steam_user_data_folder),
        );
        let mut clicked = false;

        let texture = self.image_selected_state.image_handles.get(&key);
        if let Some(texture) = texture {
            if let TextureState::Loaded(texture) = &texture.value() {
                let mut size = texture.size_vec2();
                clamp_to_width(&mut size, column_width);
                let image_button = ImageButton::new(texture, size);
                clicked = ui
                    .add(image_button)
                    .on_hover_text(&shortcut.app_name)
                    .clicked();
            }
        } else {
            let button = ui.add_sized(
                [column_width, column_width * 1.6],
                Button::new(&shortcut.app_name).wrap(true),
            );
            clicked = clicked || button.clicked();
        }

        if clicked {
            return Some(Some(UserAction::ShortcutSelected(GameType::Shortcut(
                shortcut.clone(),
            ))));
        }
        None
    }

    fn render_possible_images(
        &self,
        ui: &mut egui::Ui,
        image_type: &ImageType,
        state: &ImageSelectState,
    ) -> Option<UserAction> {
        ui.label(image_type.name());

        if let Some(action) = ui
            .horizontal(|ui| {
                if ui
                    .small_button("Clear image?")
                    .on_hover_text("Click here to clear the image")
                    .clicked()
                {
                    return Some(UserAction::ImageTypeCleared(*image_type, false));
                }

                if ui
                    .small_button("Stop downloading this image?")
                    .on_hover_text("Stop downloading this type of image for this shortcut at all")
                    .clicked()
                {
                    return Some(UserAction::ImageTypeCleared(*image_type, true));
                }
                None
            })
            .inner
        {
            return Some(action);
        }
        let column_padding = 10.;
        let column_width = MAX_WIDTH * 0.75;
        let width = ui.available_width();
        let columns = (width / (column_width + column_padding)).floor() as u32;
        let mut column = 0;
        match &*state.image_options.borrow() {
            FetcStatus::Fetched(images) => {
                let x = Grid::new("ImageThumbnailSelectGrid")
                    .spacing([column_padding, column_padding])
                    .show(ui, |ui| {
                        for image in images {
                            let image_key =
                                image.thumbnail_path.as_path().to_string_lossy().to_string();

                            match state.image_handles.get_mut(&image_key) {
                                Some(mut state) => {
                                    match state.value() {
                                        TextureState::Downloading => {
                                            ui.ctx().request_repaint();
                                            //nothing to do,just wait
                                            ui.spinner();
                                        }
                                        TextureState::Downloaded => {
                                            //Need to load
                                            let image_data =
                                                load_image_from_path(&image.thumbnail_path);
                                            match image_data {
                                                Ok(image_data) => {
                                                    let handle = ui.ctx().load_texture(
                                                        &image_key,
                                                        image_data,
                                                        egui::TextureFilter::Linear,
                                                    );
                                                    *state.value_mut() =
                                                        TextureState::Loaded(handle);
                                                    ui.spinner();
                                                }
                                                Err(_) => *state.value_mut() = TextureState::Failed,
                                            }
                                            ui.ctx().request_repaint();
                                        }
                                        TextureState::Loaded(texture_handle) => {
                                            //need to show
                                            let mut size = texture_handle.size_vec2();
                                            clamp_to_width(&mut size, column_width);
                                            let image_button =
                                                ImageButton::new(texture_handle, size);
                                            if ui.add_sized(size, image_button).clicked() {
                                                return Some(UserAction::ImageSelected(
                                                    image.clone(),
                                                ));
                                            }
                                        }
                                        TextureState::Failed => {
                                            ui.label("Failed to load image");
                                        }
                                    }
                                }
                                None => {
                                    //We need to start a download
                                    let image_handles = &self.image_selected_state.image_handles;
                                    let path = &image.thumbnail_path;
                                    //Redownload if file is too small
                                    if !path.exists()
                                        || std::fs::metadata(path)
                                            .map(|m| m.len())
                                            .unwrap_or_default()
                                            < 2
                                    {
                                        image_handles
                                            .insert(image_key.clone(), TextureState::Downloading);
                                        let to_download = ToDownload {
                                            path: path.clone(),
                                            url: image.thumbnail_url.clone(),
                                            app_name: "Thumbnail".to_string(),
                                            image_type: *image_type,
                                        };
                                        let image_handles = image_handles.clone();
                                        let image_key = image_key.clone();
                                        self.rt.spawn_blocking(move || {
                                            block_on(crate::steamgriddb::download_to_download(
                                                &to_download,
                                            ))
                                            .unwrap();
                                            image_handles
                                                .insert(image_key, TextureState::Downloaded);
                                        });
                                    } else {
                                        image_handles
                                            .insert(image_key.clone(), TextureState::Downloaded);
                                    }
                                }
                            }
                            column += 1;
                            if column >= columns {
                                column = 0;
                                ui.end_row();
                            }
                        }

                        None
                    })
                    .inner;
                if x.is_some() {
                    return x;
                }
            }
            _ => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Finding possible images");
                });
                ui.ctx().request_repaint();
            }
        }
        None
    }

    fn ensure_steam_users_loaded(&mut self) {
        if self.image_selected_state.settings_error.is_none()
            && self.image_selected_state.steam_users.is_none()
        {
            let paths = get_shortcuts_paths(&self.settings.steam);
            match paths {
                Ok(paths) => self.image_selected_state.steam_users = Some(paths),
                Err(err) => {
                    self.image_selected_state.settings_error = Some(format!("Could not find user steam location, error message: {} , try to clear the steam location field in settings to let BoilR find it itself",err));
                }
            }
        }
    }

    pub(crate) fn render_ui_images(&mut self, ui: &mut egui::Ui) {
        self.ensure_steam_users_loaded();

        if let Some(error_message) = &self.image_selected_state.settings_error {
            ui.label(error_message);
            return;
        }

        let mut action = UserAction::NoAction;
        ScrollArea::vertical()
            .stick_to_right(true)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.reset_style();
                action = self.render_ui_image_action(ui);
            });
        match action {
            UserAction::UserSelected(user) => {
                self.handle_user_selected(user, ui);
            }
            UserAction::ShortcutSelected(shortcut) => {
                self.handle_shortcut_selected(shortcut, ui);
            }
            UserAction::ImageTypeSelected(image_type) => {
                self.handle_image_type_selected(image_type);
            }
            UserAction::ImageSelected(image) => {
                self.handle_image_selected(image);
            }
            UserAction::BackButton => {
                self.handle_back_button_action();
            }
            UserAction::GridIdChanged(grid_id) => {
                self.handle_grid_change(grid_id);
            }
            UserAction::SetGamesMode(game_mode) => {
                self.handle_set_game_mode(game_mode);
            }
            UserAction::NoAction => {}
            UserAction::CorrectGridId => {
                self.handle_correct_grid_request();
            }
            UserAction::ImageTypeCleared(image_type, should_ban) => {
                let app_id = self
                    .image_selected_state
                    .selected_shortcut
                    .as_ref()
                    .unwrap()
                    .app_id();
                self.settings
                    .steamgrid_db
                    .set_image_banned(&image_type, app_id, should_ban);
                self.handle_image_type_clear(image_type);
            }
            UserAction::ClearImages => {
                for image_type in ImageType::all() {
                    self.handle_image_type_clear(*image_type);
                }
                self.handle_back_button_action();
            }
            UserAction::DownloadAllImages => {
                if let Some(users) = &self.image_selected_state.steam_users {
                    let (sender, reciever) = watch::channel(SyncProgress::FindingImages);
                    self.status_reciever = reciever;
                    let mut sender_op = Some(sender);
                    let settings = self.settings.clone();
                    let users = users.clone();                                                         
                    self.rt.spawn_blocking(move || {
                        let task = download_images(&settings, &users, &mut sender_op);
                        block_on(task);
                        let _ = sender_op.unwrap().send(SyncProgress::Done);
                    });
                        
                }
            }
            UserAction::RefreshImages => {
                let (_, reciever) = watch::channel(SyncProgress::NotStarted);            
                let user = self.image_selected_state.steam_user.clone();
                if let Some(user) = &user{
                    load_image_grids(user,&mut self.image_selected_state,ui);
                }
                self.status_reciever = reciever;                
            },
        };
    }

    fn handle_image_type_clear(&mut self, image_type: ImageType) {
        let data_folder = &self
            .image_selected_state
            .steam_user
            .as_ref()
            .unwrap()
            .steam_user_data_folder;
        for ext in POSSIBLE_EXTENSIONS {
            let file_name = image_type.file_name(
                self.image_selected_state
                    .selected_shortcut
                    .as_ref()
                    .unwrap()
                    .app_id(),
                ext,
            );
            let path = Path::new(data_folder)
                .join("config")
                .join("grid")
                .join(&file_name);
            if path.exists() {
                let _ = std::fs::remove_file(&path);
            }
            let key = path.to_string_lossy().to_string();
            self.image_selected_state.image_handles.remove(&key);
        }
        self.image_selected_state.image_type_selected = None;
    }

    fn handle_correct_grid_request(&mut self) {
        let app_name = self
            .image_selected_state
            .selected_shortcut
            .as_ref()
            .map(|s| s.name())
            .unwrap_or_default();
        let auth_key = self
            .settings
            .steamgrid_db
            .auth_key
            .clone()
            .unwrap_or_default();
        let client = steamgriddb_api::Client::new(&auth_key);
        let search_results = self.rt.block_on(client.search(app_name));
        self.image_selected_state.possible_names = search_results.ok();
    }

    fn handle_set_game_mode(&mut self, game_mode: GameMode) {
        self.image_selected_state.game_mode = game_mode;
        self.image_selected_state.steam_games = Some(get_installed_games(&self.settings.steam));
    }
    fn handle_grid_change(&mut self, grid_id: usize) {
        self.image_selected_state.grid_id = Some(grid_id);
        self.image_selected_state.possible_names = None;
        if let Some(auth_key) = &self.settings.steamgrid_db.auth_key {
            let client = steamgriddb_api::Client::new(auth_key);
            let mut cache = CachedSearch::new(&client);
            if let Some(shortcut) = &self.image_selected_state.selected_shortcut {
                cache.set_cache(shortcut.app_id(), shortcut.name(), grid_id);
                cache.save();
            }
        }
    }

    fn handle_user_selected(&mut self, user: SteamUsersInfo, ui: &mut egui::Ui) {
        let state = &mut self.image_selected_state;
        let shortcuts = load_image_grids(&user, state, ui);
        state.user_shortcuts = Some(shortcuts);
        state.steam_user = Some(user);
    }

    fn handle_image_type_selected(&mut self, image_type: ImageType) {
        let state = &mut self.image_selected_state;
        state.image_type_selected = Some(image_type);
        let (tx, rx) = watch::channel(FetcStatus::Fetching);
        self.image_selected_state.image_options = rx;
        let settings = self.settings.clone();
        if let Some(auth_key) = settings.steamgrid_db.auth_key {
            if let Some(grid_id) = self.image_selected_state.grid_id {
                let auth_key = auth_key;
                let image_type = image_type;
                self.rt.spawn_blocking(move || {
                    let thumbnails_folder = get_thumbnails_folder();
                    let client = steamgriddb_api::Client::new(auth_key);
                    let query = get_query_type(false, &image_type);
                    let search_res = block_on(client.get_images_for_id(grid_id, &query));
                    if let Ok(possible_images) = search_res {
                        let mut result = vec![];
                        for possible_image in &possible_images {
                            let ext = get_image_extension(&possible_image.mime);
                            let path =
                                thumbnails_folder.join(format!("{}.{}", possible_image.id, ext));
                            result.push(PossibleImage {
                                thumbnail_path: path,
                                mime: possible_image.mime.clone(),
                                thumbnail_url: possible_image.thumb.clone(),
                                full_url: possible_image.url.clone(),
                                id: possible_image.id,
                            });
                        }
                        let _ = tx.send(FetcStatus::Fetched(result));
                    }
                });
            }
        };
    }

    fn handle_image_selected(&mut self, image: PossibleImage) {
        //We must have a user here
        let user = self.image_selected_state.steam_user.as_ref().unwrap();
        let selected_image_type = self
            .image_selected_state
            .image_type_selected
            .as_ref()
            .unwrap();
        let selected_shortcut = self
            .image_selected_state
            .selected_shortcut
            .as_ref()
            .unwrap();

        let ext = get_image_extension(&image.mime);
        let to_download_to_path = Path::new(&user.steam_user_data_folder)
            .join("config")
            .join("grid")
            .join(selected_image_type.file_name(selected_shortcut.app_id(), ext));

        //Delete old possible images

        let data_folder = Path::new(&user.steam_user_data_folder);

        //Keep deleting images of this type untill we don't find any more
        let mut path = self.get_shortcut_image_path(data_folder);
        while Path::new(&path).exists() {
            let _ = std::fs::remove_file(&path);
            path = self.get_shortcut_image_path(data_folder);
        }

        //Put the loaded thumbnail into the image handler map, we can use that for preview
        let full_image_key = to_download_to_path.to_string_lossy().to_string();
        let _ = self
            .image_selected_state
            .image_handles
            .remove(&full_image_key);
        let thumbnail_key = image.thumbnail_path.to_string_lossy().to_string();
        let thumbnail = self
            .image_selected_state
            .image_handles
            .remove(&thumbnail_key);
        if let Some((_key, thumbnail)) = thumbnail {
            self.image_selected_state
                .image_handles
                .insert(full_image_key, thumbnail);
        }

        let app_name = selected_shortcut.name();
        let to_download = ToDownload {
            path: to_download_to_path,
            url: image.full_url.clone(),
            app_name: app_name.to_string(),
            image_type: *selected_image_type,
        };
        self.rt.spawn_blocking(move || {
            let _ = block_on(crate::steamgriddb::download_to_download(&to_download));
        });

        self.clear_loaded_images();
        {
            self.image_selected_state.image_type_selected = None;
            self.image_selected_state.image_options = watch::channel(FetcStatus::NeedsFetched).1;
        }
    }

    fn get_shortcut_image_path(&self, data_folder: &Path) -> String {
        self.image_selected_state
            .selected_shortcut
            .as_ref()
            .unwrap()
            .key(
                &self.image_selected_state.image_type_selected.unwrap(),
                data_folder,
            )
            .1
    }

    fn clear_loaded_images(&mut self) {
        if let FetcStatus::Fetched(options) = &*self.image_selected_state.image_options.borrow() {
            for option in options {
                let key = option.thumbnail_path.to_string_lossy().to_string();
                self.image_selected_state.image_handles.remove(&key);
            }
        }
    }

    fn handle_shortcut_selected(&mut self, shortcut: GameType, ui: &mut egui::Ui) {
        let state = &mut self.image_selected_state;
        //We must have a user to make see this action;
        let user = state.steam_user.as_ref().unwrap();
        if let Some(auth_key) = &self.settings.steamgrid_db.auth_key {
            let client = steamgriddb_api::Client::new(auth_key);
            let search = CachedSearch::new(&client);
            state.grid_id = self
                .rt
                .block_on(search.search(shortcut.app_id(), shortcut.name()))
                .ok()
                .flatten();
        }
        state.selected_shortcut = Some(shortcut.clone());

        for image_type in ImageType::all() {
            let (path, key) = shortcut.key(image_type, Path::new(&user.steam_user_data_folder));
            let image = load_image_from_path(&path);
            if let Ok(image) = image {
                let texture = ui
                    .ctx()
                    .load_texture(&key, image, egui::TextureFilter::Linear);
                state
                    .image_handles
                    .insert(key, TextureState::Loaded(texture));
            }
        }
        state.selected_shortcut = Some(shortcut);
    }

    fn handle_back_button_action(&mut self) {
        let state = &mut self.image_selected_state;
        if state.possible_names.is_some() {
            state.possible_names = None;
        } else if state.image_type_selected.is_some() {
            state.image_type_selected = None;
        } else if state.selected_shortcut.is_some() {
            state.selected_shortcut = None;
        } else {
            state.image_handles.clear();
            state.user_shortcuts = None;
            state.steam_user = None;
        }
    }
}

fn load_image_grids(user: &SteamUsersInfo, state: &mut ImageSelectState, ui: &mut egui::Ui) -> Vec<ShortcutOwned> {
    let user_info = crate::steam::get_shortcuts_for_user(user);
    let mut user_folder = user_info.path.clone();
    user_folder.pop();
    user_folder.pop();
    let mut shortcuts = user_info.shortcuts;
    shortcuts.sort_by_key(|s| s.app_name.clone());
    let image_type = &ImageType::Grid;
    for shortcut in &shortcuts {
        let (path, key) = shortcut.key(image_type, &user_folder);
        let loaded = state.image_handles.contains_key(&key);
        if !loaded && path.exists() {
            let image = load_image_from_path(&path);
            if let Ok(image) = image {
                let texture = ui
                    .ctx()
                    .load_texture(&key, image, egui::TextureFilter::Linear);
                state
                    .image_handles
                    .insert(key, TextureState::Loaded(texture));
            }
        }
    }
    shortcuts
}

fn render_shortcut_mode_select(state: &ImageSelectState, ui: &mut egui::Ui) -> Option<UserAction> {
    let mode_before = state.game_mode.clone();
    let combo_box = egui::ComboBox::new("ImageModeSelect", "").selected_text(mode_before.label());
    let mut mode_after = state.game_mode.clone();
    combo_box.show_ui(ui, |ui| {
        ui.selectable_value(
            &mut mode_after,
            GameMode::Shortcuts,
            GameMode::Shortcuts.label(),
        );
        ui.selectable_value(
            &mut mode_after,
            GameMode::SteamGames,
            GameMode::SteamGames.label(),
        );
    });
    if !mode_after.eq(&mode_before) {
        return Some(UserAction::SetGamesMode(mode_after));
    }
    None
}

fn render_possible_names(
    possible_names: &Vec<steamgriddb_api::search::SearchResult>,
    ui: &mut egui::Ui,
    state: &ImageSelectState,
) -> Option<UserAction> {
    let mut grid_id_text = state.grid_id.map(|id| id.to_string()).unwrap_or_default();
    ui.label("SteamGridDB ID")
        .on_hover_text("You can change this id to one you have found at the steamgriddb webpage");
    if ui.text_edit_singleline(&mut grid_id_text).changed() {
        if let Ok(grid_id) = grid_id_text.parse::<usize>() {
            return Some(UserAction::GridIdChanged(grid_id));
        }
    };

    for possible in possible_names {
        if ui.button(&possible.name).clicked() {
            return Some(UserAction::GridIdChanged(possible.id));
        }
    }

    ui.separator();
    if ui
        .button("Clear all images")
        .on_hover_text("Clicking this deletes all images for this shortcut")
        .clicked()
    {
        return Some(UserAction::ClearImages);
    }
    None
}

fn render_steam_game_select(ui: &mut egui::Ui, state: &ImageSelectState) -> Option<UserAction> {
    if let Some(steam_games) = state.steam_games.as_ref() {
        for game in steam_games {
            if ui.button(&game.name).clicked() {
                return Some(UserAction::ShortcutSelected(GameType::SteamGame(
                    game.clone(),
                )));
            }
        }
    }
    None
}

fn render_shortcut_images(ui: &mut egui::Ui, state: &ImageSelectState) -> Option<UserAction> {
    let shortcut = state.selected_shortcut.as_ref().unwrap();
    let user_path = &state.steam_user.as_ref().unwrap().steam_user_data_folder;
    let x = if ui.available_width() > MAX_WIDTH * 3. {
        ui.horizontal(|ui| {
            let x = ui
                .vertical(|ui| {
                    let texture =
                        texture_from_iamge_type(shortcut, &ImageType::Grid, user_path, state);
                    ui.label(ImageType::Grid.name());
                    if render_thumbnail(ui, texture).clicked() {
                        return Some(UserAction::ImageTypeSelected(ImageType::Grid));
                    }
                    None
                })
                .inner;
            if x.is_some() {
                return x;
            }
            let x = ui
                .vertical(|ui| {
                    let texture =
                        texture_from_iamge_type(shortcut, &ImageType::Hero, user_path, state);
                    ui.label(ImageType::Hero.name());
                    if render_thumbnail(ui, texture).clicked() {
                        return Some(UserAction::ImageTypeSelected(ImageType::Hero));
                    }
                    let texture =
                        texture_from_iamge_type(shortcut, &ImageType::WideGrid, user_path, state);
                    ui.label(ImageType::WideGrid.name());
                    if render_thumbnail(ui, texture).clicked() {
                        return Some(UserAction::ImageTypeSelected(ImageType::WideGrid));
                    }

                    let texture =
                        texture_from_iamge_type(shortcut, &ImageType::Logo, user_path, state);
                    ui.label(ImageType::Logo.name());
                    if render_thumbnail(ui, texture).clicked() {
                        return Some(UserAction::ImageTypeSelected(ImageType::Logo));
                    }
                    None
                })
                .inner;
            if x.is_some() {
                return x;
            }
            ui.vertical(|ui| {
                let texture = texture_from_iamge_type(shortcut, &ImageType::Icon, user_path, state);
                ui.label(ImageType::Icon.name());
                if render_thumbnail(ui, texture).clicked() {
                    return Some(UserAction::ImageTypeSelected(ImageType::Icon));
                }

                let texture =
                    texture_from_iamge_type(shortcut, &ImageType::BigPicture, user_path, state);
                ui.label(ImageType::BigPicture.name());
                if render_thumbnail(ui, texture).clicked() {
                    return Some(UserAction::ImageTypeSelected(ImageType::BigPicture));
                }
                None
            })
            .inner
        })
        .inner
    } else {
        render_image_types_as_list(shortcut, user_path, state, ui)
    };

    if ui
        .button("Click here if the images are for a wrong game")
        .clicked()
    {
        return Some(UserAction::CorrectGridId);
    }
    x
}

fn render_image_types_as_list(
    shortcut: &GameType,
    user_path: &String,
    state: &ImageSelectState,
    ui: &mut egui::Ui,
) -> Option<UserAction> {
    let types = ImageType::all();
    for image_type in types {
        let texture = texture_from_iamge_type(shortcut, image_type, user_path, state);
        let response = ui
            .vertical(|ui| {
                ui.label(image_type.name());
                render_thumbnail(ui, texture)
            })
            .inner;
        if response.clicked() {
            return Some(UserAction::ImageTypeSelected(*image_type));
        }
    }
    None
}

fn texture_from_iamge_type(
    shortcut: &GameType,
    image_type: &ImageType,
    user_path: &String,
    state: &ImageSelectState,
) -> Option<egui::TextureHandle> {
    let (_path, key) = shortcut.key(image_type, Path::new(&user_path));
    state.image_handles.get(&key).and_then(|k| match k.value() {
        TextureState::Loaded(texture) => Some(texture.clone()),
        _ => None,
    })
}

fn render_user_select(state: &ImageSelectState, ui: &mut egui::Ui) -> Option<UserAction> {
    if state.steam_user.is_none() {
        if let Some(users) = &state.steam_users {
            if users.len() > 0 {
                return Some(UserAction::UserSelected(users[0].clone()));
            }
        }
    } else {
        let mut selected_user = state.steam_user.as_ref().unwrap().clone();
        let id_before = selected_user.user_id.clone();
        if let Some(steam_users) = &state.steam_users {
            if steam_users.len() > 0 {
                let combo_box = egui::ComboBox::new("ImageUserSelect", "")
                    .selected_text(format!("Steam user id: {}", &selected_user.user_id));
                combo_box.show_ui(ui, |ui| {
                    for user in steam_users {
                        ui.selectable_value(&mut selected_user, user.clone(), &user.user_id);
                    }
                });
            }
        }
        let id_now = selected_user.user_id.clone();
        if !id_before.eq(&id_now) {
            return Some(UserAction::UserSelected(selected_user.clone()));
        }
    }

    None
}

const MAX_WIDTH: f32 = 300.;

fn render_thumbnail(ui: &mut egui::Ui, image: Option<egui::TextureHandle>) -> egui::Response {
    if let Some(texture) = image {
        let mut size = texture.size_vec2();
        clamp_to_width(&mut size, MAX_WIDTH);
        let image_button = ImageButton::new(&texture, size);
        ui.add(image_button)
    } else {
        ui.button("Pick an image")
    }
}

pub fn clamp_to_width(size: &mut egui::Vec2, max_width: f32) {
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

trait HasImageKey {
    fn key(&self, image_type: &ImageType, user_path: &Path) -> (PathBuf, String);
}
const POSSIBLE_EXTENSIONS: [&str; 4] = ["png", "jpg", "ico", "webp"];

impl HasImageKey for GameType {
    fn key(&self, image_type: &ImageType, user_path: &Path) -> (PathBuf, String) {
        match self {
            GameType::Shortcut(s) => s.key(image_type, user_path),
            GameType::SteamGame(g) => g.key(image_type, user_path),
        }
    }
}
impl HasImageKey for SteamGameInfo {
    fn key(&self, image_type: &ImageType, user_path: &Path) -> (PathBuf, String) {
        let mut keys = POSSIBLE_EXTENSIONS
            .iter()
            .map(|ext| key_from_extension(self.appid, image_type, user_path, ext));
        let first = keys.next().unwrap();
        let other = keys.find(|(exsists, _, _)| *exsists);
        let (_, path, key) = other.unwrap_or(first);
        (path, key)
    }
}

impl HasImageKey for ShortcutOwned {
    fn key(&self, image_type: &ImageType, user_path: &Path) -> (PathBuf, String) {
        let mut keys = POSSIBLE_EXTENSIONS
            .iter()
            .map(|ext| key_from_extension(self.app_id, image_type, user_path, ext));
        let first = keys.next().unwrap();
        let other = keys.find(|(exsists, _, _)| *exsists);
        let (_, path, key) = other.unwrap_or(first);
        (path, key)
    }
}

fn key_from_extension(
    app_id: u32,
    image_type: &ImageType,
    user_path: &Path,
    ext: &str,
) -> (bool, PathBuf, String) {
    let file_name = image_type.file_name(app_id, ext);
    let path = user_path.join("config").join("grid").join(&file_name);
    let key = path.to_string_lossy().to_string();
    (path.exists(), path, key)
}
