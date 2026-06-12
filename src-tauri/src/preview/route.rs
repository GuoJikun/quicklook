use crate::utils::File as UFile;

pub struct WebRoute {
    pub path: String,
    pub query: UFile,
}

impl WebRoute {
    pub fn to_url(&self) -> String {
        let query = [
            ("file_type", self.query.get_file_type()),
            ("path", self.query.get_path()),
            ("extension", self.query.get_extension()),
            ("size", self.query.get_size().to_string()),
            ("last_modified", self.query.get_last_modified().to_string()),
            ("name", self.query.get_name()),
        ];
        let encoded: Vec<String> = query
            .iter()
            .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
            .collect();
        format!("{}?{}", self.path, encoded.join("&"))
    }

    pub fn new(path: String, query: UFile) -> Self {
        Self { path, query }
    }

    pub fn get_route(type_str: &str, file_info: UFile) -> WebRoute {
        match type_str {
            "Markdown" => WebRoute::new("/preview/md".to_string(), file_info),
            "Image" => WebRoute::new("/preview/image".to_string(), file_info),
            "Audio" => WebRoute::new("/preview/audio".to_string(), file_info),
            "Video" => WebRoute::new("/preview/video".to_string(), file_info),
            "Font" => WebRoute::new("/preview/font".to_string(), file_info),
            "Code" => WebRoute::new("/preview/code".to_string(), file_info),
            "Book" => WebRoute::new("/preview/book".to_string(), file_info),
            "Archive" => WebRoute::new("/preview/archive".to_string(), file_info),
            "Doc" => WebRoute::new("/preview/document".to_string(), file_info),
            "Model3D" => WebRoute::new("/preview/model".to_string(), file_info),
            _ => WebRoute::new("/preview/not-support".to_string(), file_info),
        }
    }
}
