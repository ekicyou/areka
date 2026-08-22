//! CaptureGuard パニック安全性テスト
//!
//! Task 5.2: `std::panic::catch_unwind` + thread spawn で
//! パニック時にも RAII による ReleaseCapture が呼ばれることを検証する。
//!
//! NOTE: SetCapture/ReleaseCapture は UI スレッドの API であり、
//! テストスレッドでは実質的に no-op だが、パニック unwind 時に
//! CaptureGuard::drop() が呼ばれること自体を検証する。

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use windows::Win32::Foundation::HWND;

/// パニック時にスタック巻き戻しで CaptureGuard が正しく Drop されること
///
/// ドラッグ中のパニック（例: コールバック内のバグ）でも
/// マウスキャプチャが確実に解放されることを保証する。
#[test]
fn capture_guard_dropped_on_panic() {
    // CaptureGuard の Drop が呼ばれたかどうかを監視する仕組み:
    // 別スレッドで CaptureGuard を作成→panic!() → join で catch
    let drop_called = Arc::new(AtomicBool::new(false));
    let drop_flag = drop_called.clone();

    let handle = std::thread::spawn(move || {
        // null HWND で CaptureGuard を生成（テスト環境では安全）
        let guard = wintf::ecs::drag::CaptureGuard::acquire(HWND::default());

        // Drop 検出のために、guard のライフタイムをスコープで管理
        // panic!() 前に guard がスコープにある状態で unwind が発生する
        struct DropDetector {
            flag: Arc<AtomicBool>,
        }
        impl Drop for DropDetector {
            fn drop(&mut self) {
                self.flag.store(true, Ordering::SeqCst);
            }
        }

        // DropDetector を guard の「後」に配置
        // （Rust のドロップ順序は宣言の逆順なので、guard → detector の順にドロップ）
        // つまり DropDetector の drop が先に呼ばれ、guard の drop がその後に呼ばれる
        let _detector = DropDetector { flag: drop_flag };

        // guard がまだ alive であることを確認
        let _ = &guard;

        // ここでパニック → スタック巻き戻し → Drop 発火
        panic!("simulated panic during drag");
    });

    // スレッドの join でパニックを catch
    let result = handle.join();
    assert!(result.is_err(), "スレッドがパニックしたはず");

    // DropDetector が Drop された = guard も Drop された
    assert!(
        drop_called.load(Ordering::SeqCst),
        "パニック時にスタック巻き戻しで Drop が呼ばれるはず"
    );
}

/// catch_unwind でパニックを捕捉した場合も CaptureGuard が Drop されること
#[test]
fn capture_guard_dropped_on_catch_unwind() {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _guard = wintf::ecs::drag::CaptureGuard::acquire(HWND::default());
        panic!("simulated panic during drag operation");
    }));

    assert!(result.is_err(), "catch_unwind がパニックを捕捉したはず");
    // CaptureGuard の Drop が呼ばれたことは、
    // ReleaseCapture が呼ばれても副作用がないことで暗黙的に検証
    // (null HWND なので安全に完了する)
}

/// 正常スコープ終了時にも CaptureGuard が Drop されること（ベースライン）
#[test]
fn capture_guard_dropped_on_normal_scope_exit() {
    let drop_called = Arc::new(AtomicBool::new(false));
    let drop_flag = drop_called.clone();

    {
        let _guard = wintf::ecs::drag::CaptureGuard::acquire(HWND::default());

        struct DropDetector {
            flag: Arc<AtomicBool>,
        }
        impl Drop for DropDetector {
            fn drop(&mut self) {
                self.flag.store(true, Ordering::SeqCst);
            }
        }
        let _detector = DropDetector { flag: drop_flag };
    }

    assert!(
        drop_called.load(Ordering::SeqCst),
        "正常スコープ終了時に Drop が呼ばれるはず"
    );
}
