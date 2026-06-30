//! WinApp facade の骨格特性化テスト（GUI 非依存）
//!
//! 新 runtime レイヤの公開 facade `WinApp` が COM 初期化・DPI awareness 設定・
//! `EcsWorld`（`Rc<RefCell<EcsWorld>>`）生成を統括することを固定する。可視ウィンドウの
//! 生成・メッセージループの実行（ブロッキングの run()）は行わない。
//!
//! COM/DPI はプロセスグローバルな副作用を持つが、ウィンドウ生成・メッセージループは
//! 伴わないためヘッドレスで実行可能。COM 初期化が別スレッドモデルで既に行われている
//! 場合（S_FALSE / RPC_E_CHANGED_MODE）はレガシー同様に成功扱いとする。

use wintf::WinApp;

#[test]
fn new_initializes_com_dpi_and_returns_usable_world() {
    let app = WinApp::new().expect("headless WinApp::new should succeed");

    // world は生成直後から可変借用可能（new() 内の初期化借用は解放済み）
    let world = app.world();
    assert!(world.try_borrow_mut().is_ok());
}

#[test]
fn world_returns_shared_single_world_handle() {
    let app = WinApp::new().expect("WinApp::new should succeed");

    // world() は同一 World への strong clone を返す（複数呼び出しで同一インスタンス）
    let a = app.world();
    let b = app.world();
    assert!(std::rc::Rc::ptr_eq(&a, &b));
}
