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

pub struct ImageDownloadSelectState {
    pub steam_user: SteamUsersInfo,
    pub selected_shortcut: GameType,
    pub image_type_selected: ImageType,
    pub image_options: Receiver<FetcStatus<Vec<DownloadableImage>>>,
}

#[derive(Clone, Debug)]
pub struct DownloadableImage {
    pub thumbnail_path: PathBuf,
    pub thumbnail_url: String,
    pub mime: MimeTypes,
    pub full_url: String,
    pub id: u32,
}

impl ImageDownloadSelectState {
    pub fn render(
        &self,
        images_handles: &ImageHandlesMap,
        rt: &Runtime,
        ui: &mut egui::Ui,
    ) -> Option<UserAction> {
        let image_type = self.image_type_selected;
        ui.label(self.image_type_selected.name());

        if let Some(action) = ui
            .horizontal(|ui| {
                if ui
                    .small_button("Clear image?")
                    .on_hover_text("Click here to clear the image")
                    .clicked()
                {
                    return Some(UserAction::ImageTypeCleared(image_type, false));
                }

                if ui
                    .small_button("Stop downloading this image?")
                    .on_hover_text("Stop downloading this type of image for this shortcut at all")
                    .clicked()
                {
                    return Some(UserAction::ImageTypeCleared(image_type, true));
                }
                None
            })
            .inner
        {
            return Some(action);
        }
        let column_padding = 10.;
        let column_width = 200.;
        let width = ui.available_width();
        let columns = (width / (column_width + column_padding)).floor() as u32;
        let mut column = 0;
        match &*self.image_options.borrow() {
            FetcStatus::Fetched(images) => {
                let x = egui::Grid::new("ImageThumbnailSelectGrid")
                    .spacing([column_padding, column_padding])
                    .show(ui, |ui| {
                        for image in images {
                            let image_key =
                                image.thumbnail_path.as_path().to_string_lossy().to_string();

                            match images_handles.get_mut(&image_key) {
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
                                    let image_handles = images_handles;
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
                                            image_type: image_type,
                                        };
                                        let image_handles = image_handles.clone();
                                        let image_key = image_key.clone();
                                        rt.spawn_blocking(move || {
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

    pub fn handle_image_selected(
        &mut self,
        image: &DownloadableImage,
        rt: &Runtime,
        image_handles: &ImageHandlesMap,
    ) {
        //We must have a user here
        let user = &self.steam_user;
        let selected_image_type = &self.image_type_selected;
        let selected_shortcut = &self.selected_shortcut;
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
        let _ = image_handles.remove(&full_image_key);
        let thumbnail_key = image.thumbnail_path.to_string_lossy().to_string();
        let thumbnail = image_handles.remove(&thumbnail_key);
        if let Some((_key, thumbnail)) = thumbnail {
            image_handles.insert(full_image_key, thumbnail);
        }

        let app_name = selected_shortcut.name();
        let to_download = ToDownload {
            path: to_download_to_path,
            url: image.full_url.clone(),
            app_name: app_name.to_string(),
            image_type: *selected_image_type,
        };
        rt.spawn_blocking(move || {
            let _ = block_on(crate::steamgriddb::download_to_download(&to_download));
        });

        self.clear_loaded_images(image_handles);        
    }

    fn get_shortcut_image_path(&self, data_folder: &Path) -> String {
        self.selected_shortcut            
            .key(
                &self.image_type_selected,
                data_folder,
            )
            .1
    }

    fn clear_loaded_images(&mut self, image_handles: &ImageHandlesMap) {
        if let FetcStatus::Fetched(options) = &*self.image_options.borrow() {
            for option in options {
                let key = option.thumbnail_path.to_string_lossy().to_string();
                image_handles.remove(&key);
            }
        }
    }
}
