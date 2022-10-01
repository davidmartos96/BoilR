use std::collections::HashMap;

use eframe::egui;
use egui::{ScrollArea};
use egui_extras::RetainedImage;
use futures::executor::block_on;

use steam_shortcuts_util::shortcut::ShortcutOwned;
use tokio::sync::watch;
use tokio::{
    runtime::Runtime,
    sync::watch::Receiver,
};
use crate::config::get_renames_file;
use crate::platforms::{get_platforms, GamesPlatform};
use crate::settings::Settings;
use crate::sync;

use crate::sync::{download_images, SyncProgress};

use super::ui_images::get_import_image;
use super::{backup_shortcuts, all_ready, get_all_games};
use super::{
    ui_colors::{BACKGROUND_COLOR, EXTRA_BACKGROUND_COLOR},
};

const SECTION_SPACING: f32 = 25.0;

pub enum FetcStatus<T> {
    NeedsFetched,
    Fetching,
    Fetched(T),
}

impl<T> FetcStatus<T> {
    pub fn is_some(&self) -> bool {
        match self {
            FetcStatus::NeedsFetched => false,
            FetcStatus::Fetching => false,
            FetcStatus::Fetched(_) => true,
        }
    }
}
type GamesToSync = Vec<(
    String,
    Receiver<FetcStatus<eyre::Result<Vec<ShortcutOwned>>>>,
)>;
pub(crate) struct ImportState {
    games_to_sync: GamesToSync,
    status_reciever: Receiver<SyncProgress>,
    rename_map: HashMap<u32, String>,
    current_edit: Option<u32>,
    rt: Runtime,
    import_image: RetainedImage,
    blacklisted_games: Vec<u32>
}

impl ImportState{
    pub fn new() -> Self{
        let mut runtime = Runtime::new().unwrap();
        let settings = Settings::new().expect("We must be able to load our settings");
        let platforms = get_platforms();
        let games_to_sync = create_games_to_sync(&mut runtime, &platforms);

        Self{
            games_to_sync,
            status_reciever: watch::channel(SyncProgress::NotStarted).1,
            rename_map: get_rename_map(),
            current_edit: None,
            blacklisted_games: vec![],
            rt: runtime,
            import_image: RetainedImage::from_color_image("import_image",get_import_image())
        }
    }

    pub(crate) fn render_import_games(&mut self, ui: &mut egui::Ui) {
        ui.heading("Import Games");

        let mut scroll_style = ui.style_mut();
        scroll_style.visuals.extreme_bg_color = BACKGROUND_COLOR;
        scroll_style.visuals.widgets.inactive.bg_fill = EXTRA_BACKGROUND_COLOR;
        scroll_style.visuals.widgets.active.bg_fill = EXTRA_BACKGROUND_COLOR;
        scroll_style.visuals.selection.bg_fill = EXTRA_BACKGROUND_COLOR;
        scroll_style.visuals.widgets.hovered.bg_fill = EXTRA_BACKGROUND_COLOR;

        ScrollArea::vertical()
        .stick_to_right(true)
        .auto_shrink([false,true])
        .show(ui,|ui| {
            ui.reset_style();
            ui.label("Select the games you want to import into steam");
            for (name,status) in &self.games_to_sync{
                ui.heading(name);
                match &*status.borrow(){
                    FetcStatus::NeedsFetched => {ui.label("Need to find games");},
                    FetcStatus::Fetching => {
                        ui.horizontal(|ui|{
                            ui.spinner();                            
                            ui.label("Finding installed games");
                        });
                    },
                    FetcStatus::Fetched(shortcuts) => {
                        match shortcuts{
                            Ok(shortcuts) => {
                                if shortcuts.is_empty(){
                                    ui.label("Did not find any games");
                                }
                                for shortcut in shortcuts {
                                    let mut import_game = !self.blacklisted_games.contains(&shortcut.app_id);
                                    ui.horizontal(|ui|{
                                        if self.current_edit == Option::Some(shortcut.app_id){
                                            if let Some(new_name) = self.rename_map.get_mut(&shortcut.app_id){
                                                ui.text_edit_singleline(new_name).request_focus();
                                                if ui.button("Rename").clicked() {
                                                    if new_name.is_empty(){
                                                        *new_name = shortcut.app_name.to_string();
                                                    }
                                                    self.current_edit = Option::None;
                                                    let rename_file_path = get_renames_file();
                                                    let contents = serde_json::to_string(&self.rename_map);
                                                    if let Ok(contents) = contents{
                                                        let res = std::fs::write(&rename_file_path, contents);
                                                        println!("Write rename file at {:?} with result: {:?}",rename_file_path, res);
                                                    }
                                                }
                                            }
                                        }  else {                                                          
                                        let name = self.rename_map.get(&shortcut.app_id).unwrap_or(&shortcut.app_name);
                                        let checkbox = egui::Checkbox::new(&mut import_game,name);
                                        let response = ui.add(checkbox);                                
                                        if response.double_clicked(){
                                            self.rename_map.entry(shortcut.app_id).or_insert_with(|| shortcut.app_name.to_owned());                                    
                                            self.current_edit = Option::Some(shortcut.app_id);
                                        }
                                        if response.clicked(){
                                            if !self.blacklisted_games.contains(&shortcut.app_id){
                                                self.blacklisted_games.push(shortcut.app_id);
                                            }else{
                                                self.blacklisted_games.retain(|id| *id != shortcut.app_id);
                                            }
                                        }
                                    }   
                                        
                                    });                                                 
                                }
                            },
                            Err(err) => {
                                ui.label("Failed finding games").on_hover_text(format!("Error message: {err}"));
                            },
                        };
                        
                    },
                }

            };
            ui.add_space(SECTION_SPACING);

            ui.label("Check the settings if BoilR didn't find the game you where looking for");
        });
    }

   pub(crate) fn render_bottom(&mut self, ui : &mut egui::Ui,settings: &Settings){
 
    let (status_string, syncing) = match &*self.status_reciever.borrow() {
        SyncProgress::NotStarted => ("".to_string(), false),
        SyncProgress::Starting => ("Starting Import".to_string(), true),
        SyncProgress::FoundGames { games_found } => {
            (format!("Found {} games to  import", games_found), true)
        }
        SyncProgress::FindingImages => ("Searching for images".to_string(), true),
        SyncProgress::DownloadingImages { to_download } => {
            (format!("Downloading {} images ", to_download), true)
        }
        SyncProgress::Done => ("Done importing games".to_string(), false),
    };
    if syncing {
        ui.ctx().request_repaint();
    }
    if !status_string.is_empty() {
        if syncing {
            ui.horizontal(|c| {
                c.spinner();
                c.label(&status_string);
            });
        } else {
            ui.label(&status_string);
        }
    }
    let all_ready = all_ready(&self.games_to_sync);

    
    if all_ready
        && 
        self.import_image.show(ui)
            .on_hover_text("Import your games into steam")
            .clicked()
        && !syncing
    {
        self.run_sync(false,settings);
    }
}

    pub fn run_sync(&mut self, wait: bool, settings: &Settings) {
        let (sender, reciever) = watch::channel(SyncProgress::NotStarted);
        let settings = settings.clone();
        if settings.steam.stop_steam {
            crate::steam::ensure_steam_stopped();
        }

        //TODO This might break cli sync, test it

        self.status_reciever = reciever;
        let renames = self.rename_map.clone();
        let all_ready= all_ready(&self.games_to_sync);
        let _ = sender.send(SyncProgress::Starting);
        if all_ready{
            let games = get_all_games(&self.games_to_sync);
            let handle = self.rt.spawn_blocking(move || {
                settings.save();
                let mut some_sender = Some(sender);
                backup_shortcuts(&settings.steam);
                let usersinfo = sync::sync_shortcuts(&settings, &games, &mut some_sender,&renames).unwrap();
                let task = download_images(&settings, &usersinfo, &mut some_sender);
                block_on(task);
                //Run a second time to fix up shortcuts after images are downloaded
                sync::sync_shortcuts(&settings, &games, &mut some_sender,&renames).unwrap();
    
                if let Some(sender) = some_sender {
                    let _ = sender.send(SyncProgress::Done);
                }
                if settings.steam.start_steam {
                    crate::steam::ensure_steam_started(&settings.steam);
                }
            });
            if wait {
                self.rt.block_on(handle).unwrap();
            }
        }
}
}

fn create_games_to_sync(rt: &mut Runtime, platforms: &[Box<dyn GamesPlatform>]) -> GamesToSync {
    let mut to_sync = vec![];
    for platform in platforms {
        if platform.enabled() {
            let (tx, rx) = watch::channel(FetcStatus::NeedsFetched);
            to_sync.push((platform.name().to_string(), rx));
            let platform = platform.clone();
            rt.spawn_blocking(move || {
                let _ = tx.send(FetcStatus::Fetching);
                let games_to_sync = sync::get_platform_shortcuts(platform);
                let _ = tx.send(FetcStatus::Fetched(games_to_sync));
            });
        }
    }
    to_sync
}

fn get_rename_map() -> HashMap<u32, String> {
    try_get_rename_map().unwrap_or_default()
}

fn try_get_rename_map() -> eyre::Result<HashMap<u32, String>> {
    let rename_map = get_renames_file();
    let file_content = std::fs::read_to_string(rename_map)?;
    let deserialized = serde_json::from_str(&file_content)?;
    Ok(deserialized)
}
