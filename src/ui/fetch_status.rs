pub enum FetchStatus<T> {
    NeedsFetched,
    Fetching,
    Fetched(T),
}

impl<T> FetchStatus<T> {
    pub fn is_fetched(&self) -> bool {
        match self {
            FetchStatus::NeedsFetched => false,
            FetchStatus::Fetching => false,
            FetchStatus::Fetched(_) => true,
        }
    }

    pub fn needs_fetching(&self) -> bool {
        match self {
            FetchStatus::NeedsFetched => true,
            FetchStatus::Fetching => false,
            FetchStatus::Fetched(_) => false,
        }
    }
}