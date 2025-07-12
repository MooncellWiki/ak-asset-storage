/// Bundle entity represents a file within a specific version
#[derive(Debug, Clone)]
pub struct Bundle {
    pub id: Option<i32>,
    pub path: String,
    pub version_id: i32,
    pub file_id: i32,
}
