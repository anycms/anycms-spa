use regex::Regex;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PathError {
    #[error("Invalid path: {0}")]
    InvalidPath(String),
    #[error("Path traversal attempt detected")]
    PathTraversal,
}

/// 路径规范化工具
pub fn normalize_path(path: &str) -> Result<String, PathError> {
    let full_path = path.to_string();
    
    let normalized = Path::new(&full_path)
        .components()
        .fold(PathBuf::new(), |mut acc, comp| {
            match comp {
                std::path::Component::Normal(name) => {
                    acc.push(name);
                }
                std::path::Component::ParentDir => {
                    acc.pop();
                }
                _ => {} // 忽略根目录和前缀
            }
            acc
        });
    
    // 防止路径遍历攻击
    if normalized.to_string_lossy().contains("..") {
        return Err(PathError::PathTraversal);
    }
    
    let normalized_str = normalized.to_string_lossy().to_string();
    Ok(normalized_str)
}

/// 提取相对于基路径的路径
pub fn relative_to_base(path: &str, base: &str) -> String {
    let base = base.trim_matches('/');
    if base.is_empty() {
        return path.to_string();
    }
    let re = Regex::new(&format!("^{}/?", base)).unwrap();
    let relative = re.replace(path, "");
    
    if relative.is_empty() {
        "index.html".to_string()
    } else {
        relative.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_path() {
        assert_eq!(normalize_path("a/b/../c").unwrap(), "/a/c");
        assert_eq!(normalize_path("/a//b/").unwrap(), "/a/b");
        assert_eq!(normalize_path("").unwrap(), "/base");
    }

    #[test]
    fn test_relative_to_base() {
        assert_eq!(relative_to_base("/app/index.html", "app"), "index.html");
        assert_eq!(relative_to_base("/app/", "app"), "index.html");
        assert_eq!(relative_to_base("/app/css/style.css", "app"), "css/style.css");
        assert_eq!(relative_to_base("/other", "app"), "other");
    }
}