use std::path::PathBuf;

use crate::{
    steam::SteamUsersInfo,
    steamgriddb::{ImageType, ToDownload},
    ui::{FetcStatus,  ImageHandlesMap, TextureState, ui_images::load_image_from_path, clamp_to_width, UserAction},
};
use egui::ImageButton;
use futures::executor::block_on;
use steamgriddb_api::images::MimeTypes;
use tokio::{sync::watch::Receiver, runtime::Runtime};

struct ImageSelectState {
    steam_user: SteamUsersInfo,
    image_type_selected: ImageType,
    image_options: Receiver<FetcStatus<Vec<DownloadableImage>>>,
}

#[derive(Clone, Debug)]
struct DownloadableImage {
    thumbnail_path: PathBuf,
    thumbnail_url: String,
    mime: MimeTypes,
    full_url: String,
    id: u32,
}

impl ImageSelectState{

fn render(
    &self,
    images_handles: &ImageHandlesMap,
    rt: &mut Runtime,
    ui: &mut egui::Ui
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
}
