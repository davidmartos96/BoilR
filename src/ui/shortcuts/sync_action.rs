#[derive(Default)]
pub struct SyncActions<T> {
    pub add: Vec<T>,
    pub image_download: Vec<T>,
    pub update: Vec<T>,
    pub delete: Vec<T>,
    pub none:Vec<T>,
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
