#[derive(Default)]
pub struct SyncActions<T> {
    pub add: Vec<T>,
    update: Vec<T>,
    image_download: Vec<T>,
    delete: Vec<T>,
    none:Vec<T>,
}

impl<T> SyncActions<T>{
    pub fn new() -> Self{
        SyncActions{
            add: vec![],
            update: vec![],
            image_download: vec![],
            delete: vec![],
            none:vec![],
        }
    }
}
