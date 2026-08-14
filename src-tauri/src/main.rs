// 防止 Windows 编译时弹出额外的 console 窗口
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    peiyuan_lib::run()
}
