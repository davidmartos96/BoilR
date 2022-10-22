use std::path::{Path, PathBuf};

use crate::{
    steam::SteamUsersInfo,
    steamgriddb::{get_image_extension, ImageType, ToDownload},
    ui::{
        clamp_to_width, ui_images::load_image_from_path, FetcStatus, GameType, ImageHandlesMap,
        TextureState, UserAction, HasImageKey,
    },
};
use egui::ImageButton;
use futures::executor::block_on;
use steamgriddb_api::images::MimeTypes;
use tokio::{runtime::Runtime, sync::watch::Receiver};

pub struct ImageTypeSelectState{
    pub selected_shortcut:GameType,
    pub steam_user: SteamUsersInfo,
}

const MAX_WIDTH: f32 = 300.;

impl ImageTypeSelectState{
pub fn render_shortcut_images(&self, image_handles: &ImageHandlesMap, ui: &mut egui::Ui) -> Option<UserAction> {
    let user_path = &self.steam_user.steam_user_data_folder;
    let x = if ui.available_width() > MAX_WIDTH * 3. {
        ui.horizontal(|ui| {
            let x = ui
                .vertical(|ui| {
                    let texture =
                        self.texture_from_iamge_type(&ImageType::Grid, image_handles);
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
                        self.texture_from_iamge_type(&ImageType::Hero, image_handles);
                    ui.label(ImageType::Hero.name());
                    if render_thumbnail(ui, texture).clicked() {
                        return Some(UserAction::ImageTypeSelected(ImageType::Hero));
                    }
                    let texture =
                        self.texture_from_iamge_type(&ImageType::WideGrid, image_handles);
                    ui.label(ImageType::WideGrid.name());
                    if render_thumbnail(ui, texture).clicked() {
                        return Some(UserAction::ImageTypeSelected(ImageType::WideGrid));
                    }

                    let texture =
                        self.texture_from_iamge_type(&ImageType::Logo, image_handles);
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
                let texture = self.texture_from_iamge_type(&ImageType::Icon, image_handles);
                ui.label(ImageType::Icon.name());
                if render_thumbnail(ui, texture).clicked() {
                    return Some(UserAction::ImageTypeSelected(ImageType::Icon));
                }

                let texture =
                    self.texture_from_iamge_type(&ImageType::BigPicture, image_handles);
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
        self.render_image_types_as_list(ui,image_handles)
    };

    if ui
        .button("Click here if the images are for a wrong game")
        .clicked()
    {
        return Some(UserAction::CorrectGridId);
    }
    x
}

fn texture_from_iamge_type(
    &self,    
    image_type: &ImageType,    
    image_handles: &ImageHandlesMap
) -> Option<egui::TextureHandle> {
    let user_path = &self.steam_user.steam_user_data_folder;
    let (_path, key) = self.selected_shortcut.key(image_type, Path::new(&user_path));
    image_handles.get(&key).and_then(|k| match k.value() {
        TextureState::Loaded(texture) => Some(texture.clone()),
        _ => None,
    })
}


fn render_image_types_as_list(
    &self,    
    ui: &mut egui::Ui,
    image_handles: &ImageHandlesMap
) -> Option<UserAction> {
    let types = ImageType::all();
    for image_type in types {
        let texture = self.texture_from_iamge_type(image_type,image_handles);
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


}


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
