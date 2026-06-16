//! BitmapSource モジュールのユニットテスト

use super::*;
use windows_core::Interface;

// ============================================================
// Task 1.1: WicCore Tests
// ============================================================

// COMを初期化するヘルパー
fn with_com_initialized<F: FnOnce()>(f: F) {
    use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize};
    // COINIT_MULTITHREADED for WIC free-threaded factory
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    f();
    unsafe {
        CoUninitialize();
    }
}

#[test]
fn test_wic_core_creation() {
    // WicCoreが正常に作成できることを確認
    with_com_initialized(|| {
        let result = WicCore::new();
        assert!(result.is_ok(), "WicCore creation should succeed");
    });
}

#[test]
fn test_wic_core_factory_access() {
    // factory()アクセサが有効な参照を返すことを確認
    with_com_initialized(|| {
        let wic_core = WicCore::new().expect("WicCore creation failed");
        let factory = wic_core.factory();
        // factory が存在することを確認（nullではない）
        assert!(!factory.as_raw().is_null(), "factory should not be null");
    });
}

#[test]
fn test_wic_core_clone() {
    // Cloneトレイトが正しく実装されていることを確認
    with_com_initialized(|| {
        let wic_core = WicCore::new().expect("WicCore creation failed");
        let cloned = wic_core.clone();
        // 両方のfactoryが有効であることを確認
        assert!(
            !wic_core.factory().as_raw().is_null(),
            "original factory should be valid"
        );
        assert!(
            !cloned.factory().as_raw().is_null(),
            "cloned factory should be valid"
        );
    });
}

#[test]
fn test_wic_core_send_sync() {
    // Send + Syncトレイトが実装されていることをコンパイル時に確認
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<WicCore>();
}

// ============================================================
// Task 1.2: WintfTaskPool Tests
// ============================================================

#[test]
fn test_wintf_task_pool_creation() {
    // WintfTaskPoolが正常に作成できることを確認
    let task_pool = WintfTaskPool::new();
    // 作成直後はコマンドキューが空
    assert!(
        task_pool.is_empty(),
        "new task pool should have empty queue"
    );
}

#[test]
fn test_wintf_task_pool_drain_empty() {
    // 空のプールでdrain_and_applyが安全に動作することを確認
    use bevy_ecs::prelude::*;
    let task_pool = WintfTaskPool::new();
    let mut world = World::new();
    // パニックしないことを確認
    task_pool.drain_and_apply(&mut world);
}

#[test]
fn test_wintf_task_pool_command_send_receive() {
    // spawnで送信したコマンドがdrain_and_applyで実行されることを確認
    use bevy_ecs::prelude::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    let task_pool = WintfTaskPool::new();
    let executed = Arc::new(AtomicBool::new(false));
    let executed_clone = executed.clone();

    // クロージャベースのコマンドを送信（BoxedCommandはFnOnce）
    task_pool.send_command(Box::new(move |_world: &mut World| {
        executed_clone.store(true, Ordering::SeqCst);
    }));

    // drain_and_apply実行
    let mut world = World::new();
    task_pool.drain_and_apply(&mut world);

    assert!(
        executed.load(Ordering::SeqCst),
        "command should be executed"
    );
}

// ============================================================
// Task 2.1: BitmapSource Component Tests
// ============================================================

#[test]
fn test_bitmap_source_creation() {
    // BitmapSourceが正しくパスを保持することを確認
    let bitmap_source = BitmapSource::new("test/path.png");
    assert_eq!(bitmap_source.path, "test/path.png");
}

#[test]
fn test_bitmap_source_from_string() {
    // String型からも作成できることを確認
    let path = String::from("assets/image.png");
    let bitmap_source = BitmapSource::new(path);
    assert_eq!(bitmap_source.path, "assets/image.png");
}

// ============================================================
// Task 2.2: BitmapSourceResource Tests
// ============================================================

#[test]
fn test_bitmap_source_resource_send_sync() {
    // Send + Syncトレイトが実装されていることをコンパイル時に確認
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<BitmapSourceResource>();
}

// ============================================================
// Task 2.3: BitmapSourceGraphics Tests
// ============================================================

#[test]
fn test_bitmap_source_graphics_new() {
    // 空のBitmapSourceGraphicsが作成できることを確認
    let graphics = BitmapSourceGraphics::new();
    assert!(!graphics.is_valid(), "new graphics should not be valid");
    assert!(graphics.bitmap().is_none(), "bitmap should be None");
}

#[test]
fn test_bitmap_source_graphics_invalidate() {
    // invalidate()でbitmap がNoneになることを確認
    let mut graphics = BitmapSourceGraphics::new();
    graphics.invalidate();
    assert!(
        !graphics.is_valid(),
        "invalidated graphics should not be valid"
    );
}

#[test]
fn test_bitmap_source_graphics_send_sync() {
    // Send + Syncトレイトが実装されていることをコンパイル時に確認
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<BitmapSourceGraphics>();
}

// ============================================================
// W5b-T 追加: resolve_path のデバイス非依存パス解決ロジック
//
// resolve_path は WIC/D2D に一切依存しない純粋なパス変換
// （絶対パスはそのまま / 相対パスは実行ファイルディレクトリ基準で join）。
// 既存テストはゼロだったため特性化する。
// ============================================================

#[test]
fn test_resolve_path_absolute_is_returned_unchanged() {
    use super::systems::resolve_path;
    use std::path::Path;

    // Windows 絶対パスはそのまま返る（current_exe を参照しない）
    let abs = r"C:\some\absolute\image.png";
    let resolved = resolve_path(abs).expect("absolute path resolves");
    assert!(resolved.is_absolute(), "result should stay absolute");
    assert_eq!(resolved, Path::new(abs));
}

#[test]
fn test_resolve_path_relative_is_joined_under_exe_dir() {
    use super::systems::resolve_path;

    // 相対パスは実行ファイルのディレクトリ配下へ join される
    let resolved = resolve_path("assets/logo.png").expect("relative path resolves");

    // 期待値: current_exe().parent() に "assets/logo.png" を結合したもの
    let exe = std::env::current_exe().expect("current_exe available in tests");
    let exe_dir = exe.parent().expect("exe has a parent dir");
    let expected = exe_dir.join("assets/logo.png");

    assert_eq!(resolved, expected);
    // 解決結果は実行ファイルディレクトリ配下（絶対パス）になる
    assert!(resolved.is_absolute());
    assert!(resolved.starts_with(exe_dir));
    assert!(resolved.ends_with("logo.png"));
}

#[test]
fn test_resolve_path_relative_preserves_subdirectories() {
    use super::systems::resolve_path;

    // ネストした相対パスのコンポーネントが保持される
    let resolved = resolve_path("a/b/c.png").expect("nested relative path resolves");
    let exe = std::env::current_exe().expect("current_exe available in tests");
    let exe_dir = exe.parent().expect("exe has a parent dir");
    assert_eq!(resolved, exe_dir.join("a/b/c.png"));
}
