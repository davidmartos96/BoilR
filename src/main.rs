mod platforms;
mod settings;
mod steam;
mod steamgriddb;
mod sync;
mod ui;

fn main(){
    ensure_config_folder();
    //TODO
    // migration::migrate_config();

    let args: Vec<String> = std::env::args().collect();
    if args.contains(&"--no-ui".to_string()) {
        // ui::run_sync();
    } else {
        crate::ui::run_new_ui(args)
    }
}

fn ensure_config_folder() {
    let path = settings::get_config_folder();
    let _ = std::fs::create_dir_all(&path);
}
