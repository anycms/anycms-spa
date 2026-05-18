use std::path::{Component, Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum PathError {
    #[error("Invalid path: {0}")]
    InvalidPath(String),
    #[error("Path traversal attempt detected")]
    PathTraversal,
}

/// 路径规范化：解析 `.` 和 `..`，去除连续斜杠，防止路径遍历
pub fn normalize_path(path: &str) -> Result<String, PathError> {
    let mut buf = PathBuf::new();
    for comp in Path::new(path).components() {
        match comp {
            Component::Normal(name) => {
                buf.push(name);
            }
            Component::ParentDir if !buf.pop() => {
                return Err(PathError::PathTraversal);
            }
            Component::ParentDir => {}
            _ => {}
        }
    }

    Ok(buf.to_string_lossy().to_string())
}

/// 合并连续斜杠为单个 `/`
pub fn collapse_slashes(path: &str) -> String {
    let mut result = String::with_capacity(path.len());
    let mut prev_slash = false;
    for ch in path.chars() {
        if ch == '/' {
            if !prev_slash {
                result.push(ch);
            }
            prev_slash = true;
        } else {
            result.push(ch);
            prev_slash = false;
        }
    }
    result
}

/// 提取相对于基路径的资源路径
pub fn relative_to_base(path: &str, base: &str) -> String {
    let base = base.trim_matches('/');
    if base.is_empty() {
        return path.trim_start_matches('/').to_string();
    }
    let base_with_slash = format!("{}/", base);
    let path_no_lead = path.trim_start_matches('/');
    if let Some(stripped) = path_no_lead.strip_prefix(&base_with_slash) {
        if stripped.is_empty() {
            "index.html".to_string()
        } else {
            stripped.to_string()
        }
    } else if path_no_lead == base {
        "index.html".to_string()
    } else {
        path_no_lead.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_path() {
        assert_eq!(normalize_path("a/b/../c").unwrap(), "a/c");
        assert_eq!(normalize_path("/a//b/").unwrap(), "a/b");
        assert_eq!(normalize_path("").unwrap(), "");
        assert_eq!(normalize_path("css/style.css").unwrap(), "css/style.css");
        assert_eq!(normalize_path("a/../../etc/passwd"), Err(PathError::PathTraversal));
    }

    #[test]
    fn test_collapse_slashes() {
        assert_eq!(collapse_slashes("/a//b///c/"), "/a/b/c/");
        assert_eq!(collapse_slashes("/"), "/");
        assert_eq!(collapse_slashes("///"), "/");
        assert_eq!(collapse_slashes("css//style.css"), "css/style.css");
    }

    #[test]
    fn test_relative_to_base() {
        assert_eq!(relative_to_base("/app/index.html", "app"), "index.html");
        assert_eq!(relative_to_base("/app/", "app"), "index.html");
        assert_eq!(relative_to_base("/app/css/style.css", "app"), "css/style.css");
        assert_eq!(relative_to_base("/other", "app"), "other");
        assert_eq!(relative_to_base("/css/style.css", "/"), "css/style.css");
    }
}
