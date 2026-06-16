//! Task 5.1: WindowD3D11Compositor ライフサイクルテスト
//!
//! - `new()` が全4リソースを正しく作成することを検証
//! - `resize()` がリソースを再作成し generation をインクリメントすることを検証
//! - `invalidate()` が `is_valid() == false` にすることを検証

use wintf::ecs::GraphicsCore;
use wintf::ecs::compositor::WindowD3D11Compositor;

// ==========================================================================
// ヘルパー
// ==========================================================================

fn assert_send<T: Send>() {}
fn assert_sync<T: Sync>() {}

// ==========================================================================
// Send + Sync 保証
// ==========================================================================

#[test]
fn test_compositor_send_sync() {
    assert_send::<WindowD3D11Compositor>();
    assert_sync::<WindowD3D11Compositor>();
    eprintln!("✅ WindowD3D11Compositor は Send + Sync");
}

// ==========================================================================
// new() — 全4リソース作成
// ==========================================================================

#[test]
fn test_compositor_new_creates_all_resources() {
    let core = GraphicsCore::new().expect("GraphicsCore 作成失敗");
    let dc = core.device_context().expect("DeviceContext 取得失敗");

    let compositor =
        WindowD3D11Compositor::new(dc, 256, 128).expect("WindowD3D11Compositor 作成失敗");

    // 4リソースすべてが Some であること
    assert!(compositor.is_valid(), "is_valid() == true");
    assert!(
        compositor.composition_bitmap().is_some(),
        "composition_bitmap が存在する"
    );
    assert!(
        compositor.staging_bitmap().is_some(),
        "staging_bitmap が存在する"
    );
    assert!(compositor.hbitmap().is_some(), "hbitmap が存在する");
    assert!(compositor.memory_dc().is_some(), "memory_dc が存在する");
    assert!(compositor.dib_bits().is_some(), "dib_bits が存在する");

    // 初期状態
    assert_eq!(compositor.cached_size(), (256, 128), "cached_size 一致");
    assert_eq!(compositor.generation(), 0, "初期 generation == 0");
    assert!(!compositor.is_dirty(), "初期 dirty == false");

    eprintln!("✅ new() で全4リソースが正しく作成された");
}

// ==========================================================================
// resize() — リソース再作成 + generation インクリメント
// ==========================================================================

#[test]
fn test_compositor_resize_recreates_resources() {
    let core = GraphicsCore::new().expect("GraphicsCore 作成失敗");
    let dc = core.device_context().expect("DeviceContext 取得失敗");

    let mut compositor =
        WindowD3D11Compositor::new(dc, 100, 100).expect("WindowD3D11Compositor 作成失敗");
    assert_eq!(compositor.generation(), 0);
    assert_eq!(compositor.cached_size(), (100, 100));

    // リサイズ
    compositor.resize(dc, 200, 150).expect("resize() failed");

    assert!(compositor.is_valid(), "resize 後も is_valid() == true");
    assert_eq!(
        compositor.cached_size(),
        (200, 150),
        "cached_size が新サイズに更新"
    );
    assert_eq!(
        compositor.generation(),
        1,
        "generation が 1 にインクリメント"
    );
    assert!(!compositor.is_dirty(), "resize 後 dirty == false");

    // 全アクセサが Some
    assert!(compositor.composition_bitmap().is_some());
    assert!(compositor.staging_bitmap().is_some());
    assert!(compositor.hbitmap().is_some());
    assert!(compositor.memory_dc().is_some());

    eprintln!("✅ resize() でリソースが再作成され generation が増加した");
}

#[test]
fn test_compositor_resize_multiple_times() {
    let core = GraphicsCore::new().expect("GraphicsCore 作成失敗");
    let dc = core.device_context().expect("DeviceContext 取得失敗");

    let mut compositor =
        WindowD3D11Compositor::new(dc, 50, 50).expect("WindowD3D11Compositor 作成失敗");

    for i in 1..=5u32 {
        compositor
            .resize(dc, 50 + i * 10, 50 + i * 10)
            .expect("resize() failed");
        assert_eq!(compositor.generation(), i, "generation == {i}");
    }

    assert_eq!(compositor.cached_size(), (100, 100));
    eprintln!("✅ 連続 resize() で generation が正しく累積された");
}

// ==========================================================================
// invalidate() — is_valid() == false + アクセサ None
// ==========================================================================

#[test]
fn test_compositor_invalidate() {
    let core = GraphicsCore::new().expect("GraphicsCore 作成失敗");
    let dc = core.device_context().expect("DeviceContext 取得失敗");

    let mut compositor =
        WindowD3D11Compositor::new(dc, 100, 100).expect("WindowD3D11Compositor 作成失敗");
    assert!(compositor.is_valid());

    compositor.invalidate();

    assert!(!compositor.is_valid(), "invalidate 後 is_valid() == false");
    assert!(
        compositor.composition_bitmap().is_none(),
        "composition_bitmap が None"
    );
    assert!(
        compositor.staging_bitmap().is_none(),
        "staging_bitmap が None"
    );
    assert!(compositor.hbitmap().is_none(), "hbitmap が None");
    assert!(compositor.memory_dc().is_none(), "memory_dc が None");
    assert!(compositor.dib_bits().is_none(), "dib_bits が None");
    assert!(!compositor.is_dirty(), "invalidate 後 dirty == false");

    eprintln!("✅ invalidate() で全アクセサが None になった");
}

// ==========================================================================
// dirty フラグ
// ==========================================================================

#[test]
fn test_compositor_dirty_flag() {
    let core = GraphicsCore::new().expect("GraphicsCore 作成失敗");
    let dc = core.device_context().expect("DeviceContext 取得失敗");

    let mut compositor =
        WindowD3D11Compositor::new(dc, 100, 100).expect("WindowD3D11Compositor 作成失敗");

    assert!(!compositor.is_dirty(), "初期 dirty == false");
    compositor.set_dirty(true);
    assert!(compositor.is_dirty(), "set_dirty(true) で dirty == true");
    compositor.set_dirty(false);
    assert!(!compositor.is_dirty(), "set_dirty(false) で dirty == false");

    eprintln!("✅ dirty フラグの設定・取得が正しく動作する");
}

// ==========================================================================
// Debug 出力
// ==========================================================================

#[test]
fn test_compositor_debug_output() {
    let core = GraphicsCore::new().expect("GraphicsCore 作成失敗");
    let dc = core.device_context().expect("DeviceContext 取得失敗");

    let compositor =
        WindowD3D11Compositor::new(dc, 64, 64).expect("WindowD3D11Compositor 作成失敗");

    let debug_str = format!("{:?}", compositor);
    assert!(
        debug_str.contains("WindowD3D11Compositor"),
        "Debug 出力に型名が含まれる"
    );
    assert!(
        debug_str.contains("is_valid: true"),
        "Debug 出力に is_valid が含まれる"
    );

    eprintln!("✅ Debug 出力: {debug_str}");
}

// ==========================================================================
// increment_generation — 手動インクリメント
// ==========================================================================

#[test]
fn test_compositor_increment_generation() {
    let core = GraphicsCore::new().expect("GraphicsCore 作成失敗");
    let dc = core.device_context().expect("DeviceContext 取得失敗");

    let mut compositor =
        WindowD3D11Compositor::new(dc, 100, 100).expect("WindowD3D11Compositor 作成失敗");
    assert_eq!(compositor.generation(), 0);

    compositor.increment_generation();
    assert_eq!(compositor.generation(), 1);

    compositor.increment_generation();
    assert_eq!(compositor.generation(), 2);

    eprintln!("✅ increment_generation() が正しく動作する");
}

// ==========================================================================
// W3a-V: 境界値・失敗経路の特性化テスト
// ==========================================================================

/// W3a-V: D2D 最大ビットマップサイズ（FL11 で 16384）を超える巨大サイズは
/// CreateBitmap 段階で Err となり、panic / UB に至らないことを特性化する。
#[test]
fn new_with_size_exceeding_texture_limit_returns_err_without_panic() {
    let core = GraphicsCore::new().expect("GraphicsCore 作成失敗");
    let dc = core.device_context().expect("DeviceContext 取得失敗");

    let result = WindowD3D11Compositor::new(dc, 100_000, 64);
    assert!(result.is_err(), "最大ビットマップサイズ超過は Err で完結する");

    eprintln!("✅ 巨大サイズの new() は Err で完結（panic なし）");
}

/// W3a-V: 負の i32 サイズが u32 へラップした巨大値（compositor_init_system の
/// `size.width as u32` 経路の終端値）も Err で完結する（panic / UB なし）。
#[test]
fn new_with_negative_i32_wrapped_size_returns_err_without_panic() {
    let core = GraphicsCore::new().expect("GraphicsCore 作成失敗");
    let dc = core.device_context().expect("DeviceContext 取得失敗");

    let wrapped = (-1i32) as u32; // 4294967295
    let result = WindowD3D11Compositor::new(dc, wrapped, 64);
    assert!(result.is_err(), "ラップ後の巨大幅は Err で完結する");

    eprintln!("✅ 負値ラップ由来の巨大サイズも Err で完結（panic なし）");
}

/// W3a-V: resize() 失敗時は旧リソース・cached_size・generation を維持する
/// （新リソース作成成功後にのみ inner を置き換える失敗安全性の特性化）。
#[test]
fn resize_failure_keeps_previous_resources_and_state() {
    let core = GraphicsCore::new().expect("GraphicsCore 作成失敗");
    let dc = core.device_context().expect("DeviceContext 取得失敗");

    let mut compositor =
        WindowD3D11Compositor::new(dc, 64, 64).expect("WindowD3D11Compositor 作成失敗");

    let result = compositor.resize(dc, 100_000, 100_000);
    assert!(result.is_err(), "巨大サイズへの resize は Err");

    // 失敗時は旧状態が完全に保存される
    assert!(compositor.is_valid(), "失敗時は旧リソースを維持する");
    assert_eq!(compositor.cached_size(), (64, 64), "cached_size は旧値のまま");
    assert_eq!(compositor.generation(), 0, "失敗時は generation 不変");
    assert!(compositor.composition_bitmap().is_some());
    assert!(compositor.staging_bitmap().is_some());
    assert!(compositor.hbitmap().is_some());
    assert!(compositor.memory_dc().is_some());
    assert!(compositor.dib_bits().is_some());

    eprintln!("✅ resize 失敗時に旧リソース・状態が維持される");
}
