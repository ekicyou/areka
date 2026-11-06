//! com ドメインテスト共通ヘルパー

use std::path::PathBuf;
use windows::Win32::Foundation::RPC_E_CHANGED_MODE;
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
/// 初期化に失敗したら落とす。別の型の apartment に入っているスレッド（`RPC_E_CHANGED_MODE`）は
/// そのまま使い、入っていない apartment を `CoUninitialize` で壊さない。
pub fn with_com_initialized<T, F: FnOnce() -> T>(f: F) -> T {
    let hr = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    let entered = hr.is_ok();
    assert!(
        entered || hr == RPC_E_CHANGED_MODE,
        "CoInitializeEx が失敗した: {hr:?}"
    );
    let result = f();
    if entered {
        unsafe { CoUninitialize() };
    }
    result
}
