use std::{collections::HashMap, path::PathBuf};

use eframe::{egui, App, Frame};
use egui::{ImageButton, Rounding, Stroke, TextureHandle, Ui};
use egui_extras::RetainedImage;
use steam_shortcuts_util::shortcut::ShortcutOwned;
use tokio::{
    runtime::Runtime,
    sync::watch::{self, Receiver},
};

use crate::{
    platforms::{get_platforms, Platforms},
    settings::Settings,
    sync::SyncProgress,
};

use super::{
    ui_colors::{
        BACKGROUND_COLOR, BG_STROKE_COLOR, EXTRA_BACKGROUND_COLOR, LIGHT_ORANGE, ORANGE, PURLPLE,
        TEXT_COLOR,
    },
    ui_images::{get_import_image, get_logo, get_logo_icon, get_save_image},
    ui_import_games::FetcStatus,
    BackupState, DiconnectState, ImageSelectState, ImportState, SettingsState,
};

const SECTION_SPACING: f32 = 25.0;

#[derive(Default)]
struct UiImages {
    import_button: Option<egui::TextureHandle>,
    save_button: Option<egui::TextureHandle>,
    logo_32: Option<egui::TextureHandle>,
}
type GamesToSync = Vec<(
    String,
    Receiver<FetcStatus<eyre::Result<Vec<ShortcutOwned>>>>,
)>;

pub(crate) fn all_ready(games: &GamesToSync) -> bool {
    games.iter().all(|(_name, rx)| rx.borrow().is_some())
}

pub(crate) fn get_all_games(games: &GamesToSync) -> Vec<(String, Vec<ShortcutOwned>)> {
    games
        .iter()
        .filter_map(|(name, rx)| match &*rx.borrow() {
            FetcStatus::NeedsFetched => None,
            FetcStatus::Fetching => None,
            FetcStatus::Fetched(data) => match data {
                Ok(ok_data) => Some((name.to_owned(), ok_data.to_owned())),
                Err(_) => None,
            },
        })
        .collect()
}

struct ImagesState {
    rt: Runtime,
    ui_images: UiImages,
    image_selected_state: ImageSelectState,
}

struct BackupsState {
    settings: Settings,
    available_backups: Option<Vec<PathBuf>>,
    last_restore: Option<PathBuf>,
}

enum MenuState {
    Import(ImportState),
    Images(ImagesState),
    Backup(BackupState),
    Settings(SettingsState),
}

struct MainAppState {
    menu: MenuState,
    settings: Settings,
    logo_image: RetainedImage,
}

type AppState = MainAppState;

impl MainAppState {
    pub fn new() -> Self {
        Self {
            menu: MenuState::Import(ImportState::new()),
            settings: (),
            logo_image: (),
        }
    }
}

#[derive(PartialEq, Clone)]
enum MenueSelection {
    Import,
    Settings,
    Images,
    Backup,
    Disconnect,
}

impl MenueSelection {
    fn name(&self) -> &str {
        match self {
            MenueSelection::Import => "Import",
            MenueSelection::Settings => "Settings",
            MenueSelection::Images => "Images",
            MenueSelection::Backup => "Backup",
            MenueSelection::Disconnect => "Disconnect",
        }
    }
}

#[derive(PartialEq, Clone)]
enum Menues {
    Import,
    Settings,
    Images,
    Backup,
    Disconnect,
}

impl Default for Menues {
    fn default() -> Menues {
        Menues::Import
    }
}

fn render_menues(ui: &mut Ui) -> Option<MenueSelection> {
    use MenueSelection::*;
    for menu in [Import, Settings, Images, Backup, Disconnect] {
        if ui.button(menu.name()).clicked() {
            return Some(menu);
        }
    }
    None
}

impl App for AppState {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        let frame = egui::Frame::default()
            .stroke(Stroke::new(0., BACKGROUND_COLOR))
            .fill(BACKGROUND_COLOR);
        egui::SidePanel::new(egui::panel::Side::Left, "Side Panel")
            .default_width(40.0)
            .frame(frame)
            .show(ctx, |ui| {
                self.logo_image.show(ui);
                ui.add_space(SECTION_SPACING);

                if let Some(selected_menu) = render_menues(ui) {
                    todo!("Need to handle menu change");
                }
            });

        if let MenuState::Settings(setting_state) = self.menu {
            egui::TopBottomPanel::new(egui::panel::TopBottomSide::Bottom, "Bottom Panel")
                .frame(frame)
                .show(ctx, |ui| {});
        }

        if let MenuState::Import(imort_state) = self.menu {
            egui::TopBottomPanel::new(egui::panel::TopBottomSide::Bottom, "Bottom Panel")
                .frame(frame)
                .show(ctx, |ui| imort_state.render_bottom(ui));
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            match self.menu {
                MenuState::Import(s) => s.render_import_games(ui),
                MenuState::Images(s) => todo!("Render images"),
                MenuState::Backup(s) => todo!("Render backup"),
                MenuState::Settings(s) => s.render_settings(ui),
            };
        });
    }
}

fn create_style(style: &mut egui::Style) {
    style.spacing.item_spacing = egui::vec2(15.0, 15.0);
    style.visuals.button_frame = false;
    style.visuals.dark_mode = true;
    style.visuals.override_text_color = Some(TEXT_COLOR);
    style.visuals.widgets.noninteractive.rounding = Rounding {
        ne: 0.0,
        nw: 0.0,
        se: 0.0,
        sw: 0.0,
    };
    style.visuals.faint_bg_color = PURLPLE;
    style.visuals.extreme_bg_color = EXTRA_BACKGROUND_COLOR;
    style.visuals.widgets.active.bg_fill = BACKGROUND_COLOR;
    style.visuals.widgets.active.bg_stroke = Stroke::new(2.0, BG_STROKE_COLOR);
    style.visuals.widgets.active.fg_stroke = Stroke::new(2.0, LIGHT_ORANGE);
    style.visuals.widgets.open.bg_fill = BACKGROUND_COLOR;
    style.visuals.widgets.open.bg_stroke = Stroke::new(2.0, BG_STROKE_COLOR);
    style.visuals.widgets.open.fg_stroke = Stroke::new(2.0, LIGHT_ORANGE);
    style.visuals.widgets.noninteractive.bg_fill = BACKGROUND_COLOR;
    style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(2.0, BG_STROKE_COLOR);
    style.visuals.widgets.noninteractive.fg_stroke = Stroke::new(2.0, ORANGE);
    style.visuals.widgets.inactive.bg_fill = BACKGROUND_COLOR;
    style.visuals.widgets.inactive.bg_stroke = Stroke::new(2.0, BG_STROKE_COLOR);
    style.visuals.widgets.inactive.fg_stroke = Stroke::new(2.0, ORANGE);
    style.visuals.widgets.hovered.bg_fill = BACKGROUND_COLOR;
    style.visuals.widgets.hovered.bg_stroke = Stroke::new(2.0, BG_STROKE_COLOR);
    style.visuals.widgets.hovered.fg_stroke = Stroke::new(2.0, LIGHT_ORANGE);
    style.visuals.selection.bg_fill = PURLPLE;
}

fn setup(ctx: &egui::Context) {
    #[cfg(target_family = "unix")]
    ctx.set_pixels_per_point(1.0);

    let mut style: egui::Style = (*ctx.style()).clone();
    create_style(&mut style);
    ctx.set_style(style);
}

pub fn run_sync() {
    let mut app = AppState::new();
    app.run_sync(true);
}

pub fn run_ui(args: Vec<String>) {
    let app = AppState::new();
    let no_v_sync = args.contains(&"--no-vsync".to_string());
    let native_options = eframe::NativeOptions {
        initial_window_size: Some(egui::Vec2 { x: 1280., y: 800. }),
        icon_data: Some(get_logo_icon()),
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
