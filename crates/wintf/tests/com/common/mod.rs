//! com ドメインテスト共通ヘルパー

use std::path::PathBuf;
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize};

/// テストアセットディレクトリのパスを取得
pub fn test_asset_path(filename: &str) -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir)
        .join("tests")
        .join("assets")
        .join(filename)
}

/// COM をスレッド単位で初期化して `f` を実行するヘルパー
/// （`tests/com.rs` と `tests/widget.rs` が同じファイルを読む）
pub fn with_com_initialized<T, F: FnOnce() -> T>(f: F) -> T {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let result = f();
    unsafe {
        CoUninitialize();
    }
    result
}
