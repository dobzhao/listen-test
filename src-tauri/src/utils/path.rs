//! 路径工具：定位应用数据目录与缓存目录
//!
//! 配置文件位于 `app_data_dir/config.json`，
//! 每次测试的题目与音频缓存位于 `app_data_dir/cache/{session_id}/`，
//! 日志位于 `app_data_dir/logs/peiyuan.log.YYYY-MM-DD`。

use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// 应用 identifier（与 tauri.conf.json:5 保持一致）
pub const APP_IDENTIFIER: &str = "com.peiyuan.desktop";

/// 不依赖 AppHandle 的应用数据目录（日志初始化等早期场景使用）
///
/// 解析规则与 Tauri `app.path().app_data_dir()` 在三平台上一致：
///   - Linux:   `$XDG_DATA_HOME/com.peiyuan.desktop` 或 `~/.local/share/com.peiyuan.desktop`
///   - macOS:   `~/Library/Application Support/com.peiyuan.desktop`
///   - Windows: `%APPDATA%\com.peiyuan.desktop`
pub fn app_data_dir_no_handle() -> Option<PathBuf> {
    let mut dir = dirs::data_dir()?;
    dir.push(APP_IDENTIFIER);
    Some(dir)
}

/// 日志目录：`<app_data_dir>/logs/`，不存在则创建
pub fn logs_dir() -> Result<PathBuf, String> {
    let dir = app_data_dir_no_handle()
        .ok_or_else(|| "无法解析应用数据目录".to_string())?
        .join("logs");
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建日志目录失败: {e}"))?;
    Ok(dir)
}

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
