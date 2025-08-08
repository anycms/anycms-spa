pub mod path;


use rust_embed::RustEmbed;
use thiserror::Error;
use std::borrow::Cow;

#[derive(Debug, Error)]
pub enum SpaError {
    #[error("Resource not found: {0}")]
    NotFound(String),
    #[error("MIME type detection failed")]
    MimeDetection,
    #[error("Path error: {0}")]
    PathError(#[from] crate::core::path::PathError),
    #[error("Index file not found")]
    IndexFileNotFound,
}

/// SPA 配置
#[derive(Clone)]
pub struct SpaConfig {
    pub base_path: String,
    pub index_files: Vec<String>,
}

impl Default for SpaConfig {
    fn default() -> Self {
        SpaConfig {
            base_path: "/".to_string(),
            index_files: vec!["index.html".to_string()],
        }
    }
}

impl SpaConfig {
    /// 设置基础路径
    pub fn with_base_path(mut self, base_path: &str) -> Self {
        self.base_path = base_path.to_string();
        self
    }
    
    /// 设置索引文件（可多个）
    pub fn with_index_files(mut self, files: &[&str]) -> Self {
        self.index_files = files.iter().map(|s| s.to_string()).collect();
        self
    }
    
    /// 添加索引文件
    pub fn add_index_file(mut self, file: &str) -> Self {
        self.index_files.push(file.to_string());
        self
    }
}

/// SPA 处理器
pub struct SpaHandler<E: RustEmbed> {
    config: SpaConfig,
    _marker: std::marker::PhantomData<E>,
}

impl<E: RustEmbed> SpaHandler<E> {
    pub fn new(config: SpaConfig) -> Self {
        Self {
            config,
            _marker: std::marker::PhantomData,
        }
    }
    
    /// 获取嵌入的文件（考虑基础路径）
    pub fn get_file(&self, request_path: &str) -> Result<(Cow<'static, [u8]>, &'static str), SpaError> {
        // 规范化请求路径
        let normalized_path = crate::core::path::normalize_path(request_path, &self.config.base_path)?;
        
        // 获取相对于基础路径的资源路径
        let resource_path = crate::core::path::relative_to_base(&normalized_path, &self.config.base_path);
        
        // 尝试获取资源
        if let Some(content) = E::get(&resource_path) {
            let mime = mime_guess::from_path(&resource_path)
                .first_raw()
                .ok_or(SpaError::MimeDetection)?;
            return Ok((content.data, mime));
        }
        
        // 尝试获取索引文件
        for index_file in &self.config.index_files {
            if let Some(content) = E::get(index_file) {
                return Ok((content.data, "text/html"));
            }
        }
        
        // 尝试默认索引文件
        if let Some(content) = E::get("index.html") {
            return Ok((content.data, "text/html"));
        }
        
        Err(SpaError::IndexFileNotFound)
    }
}