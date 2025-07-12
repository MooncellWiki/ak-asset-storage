use crate::{AppResult, HotUpdateList};

/// Version entity represents a game client version with resource updates
#[derive(Debug, Clone)]
pub struct Version {
    pub id: Option<i32>,
    pub res: String,
    pub client: String,
    pub is_ready: bool,
    pub hot_update_list: HotUpdateList,
}
impl Version {
    pub fn new(
        res: String,
        client: String,
        is_ready: bool,
        hot_update_list: &str,
    ) -> AppResult<Self> {
        Ok(Self {
            id: None,
            res,
            client,
            is_ready,
            hot_update_list: HotUpdateList::new(hot_update_list)?,
        })
    }
    pub fn with_id(
        id: i32,
        res: String,
        client: String,
        is_ready: bool,
        hot_update_list: &str,
    ) -> AppResult<Self> {
        Ok(Self {
            id: Some(id),
            res,
            client,
            is_ready,
            hot_update_list: HotUpdateList::new(hot_update_list)?,
        })
    }
}
