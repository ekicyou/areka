//! WinThreadMgr のスレッドライフサイクル特性化テスト（GUI 非依存）
//!
//! メッセージ専用ウィンドウ（HWND_MESSAGE 親）・COM 初期化・VSync 監視スレッドのみを
//! 使用し、可視ウィンドウの生成・メッセージループの実行（ブロッキングの run()）は
//! 行わない。Drop の「stop_flag 設定 → VSync スレッド join → DestroyWindow」順序が
//! ハングせず完了することを固定する（W1-V: スレッドライフサイクル・HWND 生存期間の検証）。

use wintf::WinThreadMgr;

#[test]
fn new_and_drop_join_vsync_thread_without_hang() {
    let mgr = WinThreadMgr::new().expect("headless WinThreadMgr::new should succeed");

    // モジュールハンドル（プロセスシングルトン経由）は常に有効
    assert!(!mgr.instance().is_invalid());

    // world は生成直後から可変借用可能（new() 内の初期化借用は解放済み）
    {
        let world = mgr.world();
        assert!(world.try_borrow_mut().is_ok());
    }

    // Drop: VSync スレッドは DwmFlush（最大1フレーム）または 15ms リトライ待機の後に
    // stop_flag を観測して終了し、join が返る。ここでハングしないことが特性化対象。
    drop(mgr);
}

#[test]
fn multiple_managers_can_coexist_and_drop_independently() {
    // 多重生成は panic しない（COM 初期化はカウント加算、ウィンドウクラス登録は
    // OnceLock で1回限り）。ECS_WORLD 束縛が初回 set のまま固定される点は P32 を参照。
    let first = WinThreadMgr::new().expect("first manager");
    let second = WinThreadMgr::new().expect("second manager");

    // 各インスタンスは独立した world を所有する
    assert!(!std::rc::Rc::ptr_eq(&first.world(), &second.world()));

    // 生成順と逆でない順序の drop でもハングしない
    drop(first);
    drop(second);
}
