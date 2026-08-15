//! 路径工具：定位应用数据目录与缓存目录
//!
//! 配置文件位于 `app_data_dir/config.json`，
//! 每次测试的题目与音频缓存位于 `app_data_dir/cache/{session_id}/`。

use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// 应用数据根目录（如 Linux 下 `~/.local/share/com.peiyuan.desktop`）
pub fn app_data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map_err(|e| format!("无法解析应用数据目录: {e}"))
}

/// 配置文件路径
pub fn config_file(app: &AppHandle) -> Result<PathBuf, String> {
    let mut dir = app_data_dir(app)?;
    if !dir.exists() {
        std::fs::create_dir_all(&dir).map_err(|e| format!("创建应用数据目录失败: {e}"))?;
    }
    dir.push("config.json");
    Ok(dir)
}

/// 缓存根目录
pub fn cache_root(app: &AppHandle) -> Result<PathBuf, String> {
    let mut dir = app_data_dir(app)?;
    dir.push("cache");
    if !dir.exists() {
        std::fs::create_dir_all(&dir).map_err(|e| format!("创建缓存目录失败: {e}"))?;
    }
    Ok(dir)
}

/// 某个会话的缓存目录
pub fn session_cache_dir(app: &AppHandle, session_id: &str) -> Result<PathBuf, String> {
    let mut dir = cache_root(app)?;
    dir.push(session_id);
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建会话缓存目录失败: {e}"))?;
    Ok(dir)
}
