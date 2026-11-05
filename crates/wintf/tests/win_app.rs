//! WinApp facade の骨格特性化テスト（GUI 非依存）
//!
//! 新 runtime レイヤの公開 facade `WinApp` が COM 初期化・DPI awareness 設定・
//! `EcsWorld`（`Rc<RefCell<EcsWorld>>`）生成を統括することを固定する。可視ウィンドウの
//! 生成・メッセージループの実行（ブロッキングの run()）は行わない。
//!
//! COM/DPI はプロセスグローバルな副作用を持つが、ウィンドウ生成・メッセージループは
//! 伴わないためヘッドレスで実行可能。COM 初期化が別スレッドモデルで既に行われている
//! 場合（S_FALSE / RPC_E_CHANGED_MODE）はレガシー同様に成功扱いとする。

use windows::Win32::System::Com::{
    APTTYPE, APTTYPE_MTA, APTTYPEQUALIFIER, APTTYPEQUALIFIER_IMPLICIT_MTA, CoGetApartmentType,
};
use windows::Win32::UI::HiDpi::{
    AreDpiAwarenessContextsEqual, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
    GetThreadDpiAwarenessContext,
};
use wintf::WinApp;

#[test]
fn new_initializes_com_dpi_and_returns_usable_world() {
    let app = WinApp::new().expect("headless WinApp::new should succeed");

    // 呼んだスレッドの COM が MTA で初期化されている。テストのプロセスはプロセスの寿命の MTA を
    // 持ちうる（初期化しないスレッドでも MTA と答える）ので、暗黙の MTA でないことまで見る。
    let mut kind = APTTYPE::default();
    let mut qualifier = APTTYPEQUALIFIER::default();
    // SAFETY: 出力先は生きている局所変数。
    unsafe { CoGetApartmentType(&mut kind, &mut qualifier) }.expect("CoGetApartmentType");
    assert_eq!(
        kind, APTTYPE_MTA,
        "WinApp::new は呼んだスレッドを MTA にするはず"
    );
    assert_ne!(
        qualifier, APTTYPEQUALIFIER_IMPLICIT_MTA,
        "暗黙の MTA ではなく、WinApp::new 自身が CoInitializeEx しているはず"
    );

    // DPI の扱いがモニタごとの v2 になっている。
    // SAFETY: 引数なし・定数どうしの比べ。
    let per_monitor_v2 = unsafe {
        AreDpiAwarenessContextsEqual(
            GetThreadDpiAwarenessContext(),
            DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
        )
    };
    assert!(
        per_monitor_v2.as_bool(),
        "WinApp::new は DPI の扱いを per-monitor v2 にするはず"
    );

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
