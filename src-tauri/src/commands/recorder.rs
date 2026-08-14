//! 录音相关 Tauri commands
//!
//! - `start_recording`：启动 cpal 录音，输出路径由前端传入
//! - `stop_recording`：停止并写入 wav，返回最终路径
//! - `get_audio_level`：拉取当前音量 RMS（用于前端实时音量条）

use crate::services::recorder::RecorderState;
use std::sync::Arc;

pub struct RecorderGlobal {
    pub state: Arc<RecorderState>,
}

impl Default for RecorderGlobal {
    fn default() -> Self {
        Self {
            state: Arc::new(RecorderState::default()),
        }
    }
}

#[tauri::command]
pub fn start_recording(recorder: tauri::State<'_, RecorderGlobal>) -> Result<(), String> {
    recorder
        .state
        .start_recording()
        .map_err(|e| format!("启动录音失败: {e}"))
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StopRecordingArgs {
    pub output_path: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StopRecordingResponse {
    pub output_path: String,
}

#[tauri::command]
pub fn stop_recording(
    recorder: tauri::State<'_, RecorderGlobal>,
    args: StopRecordingArgs,
) -> Result<StopRecordingResponse, String> {
    recorder
        .state
        .stop_recording(std::path::Path::new(&args.output_path))
        .map_err(|e| format!("停止录音失败: {e}"))?;
    Ok(StopRecordingResponse {
        output_path: args.output_path,
    })
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioLevelResponse {
    /// 0.0 ~ 1.0
    pub level: f32,
    pub is_recording: bool,
}

#[tauri::command]
pub fn get_audio_level(recorder: tauri::State<'_, RecorderGlobal>) -> AudioLevelResponse {
    AudioLevelResponse {
        level: recorder.state.current_level(),
        is_recording: recorder.state.is_recording(),
    }
}
