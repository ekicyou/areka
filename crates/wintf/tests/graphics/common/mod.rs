//! 共有 GPU オーナースレッド fixture（C4 SharedGpuFixture・Path B）
//!
//! # 根本原因（research.md「根本原因記録」節が正本）
//! 本開発機（Intel Arc / Win 26200）では「**同一プロセス内で、別スレッド上に 2 個目の
//! MTA 束縛 `Compositor`（`DQTAT_COM_NONE`）を生成する**」と `STATUS_ACCESS_VIOLATION`。
//! 一方 **同一スレッド上の逐次再生成（生成→drop→再生成）は安全**（統制実験・
//! `wuc_graphics_resource_lifecycle` 緑で実証）。libtest は `#[test]` ごとに別スレッドを
//! spawn するため、各 WUC テストが自前スレッドで Compositor を作ると 2 個目で死ぬ。
//!
//! # 対処
//! WUC(Compositor) の**生成・操作・破棄をプロセス内で単一の専用オーナースレッドに集約**する。
//! テストはクロージャを marshal してオーナースレッド上で実行するため、「別スレッド上の 2 個目
//! Compositor」が構造的に発生しない（同一スレッド逐次再生成のみに帰着＝安全）。GPU テストは
//! 自然に直列化される（design.md C4）。
//!
//! # 適用範囲
//! **Compositor を生成するテストのみ**をラップする。`GraphicsCore` 単体（D3D/D2D・Compositor
//! 非生成）のテストは cross-thread でも安全（実測）ゆえラップ不要。

use std::sync::OnceLock;
use std::sync::mpsc::{Sender, channel};

use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};

/// オーナースレッドへ送るジョブ（クロージャは自前で結果チャネルへ返す）。
type Job = Box<dyn FnOnce() + Send>;

/// プロセス内唯一の GPU オーナースレッドへの送信端（遅延初期化）。
static OWNER: OnceLock<Sender<Job>> = OnceLock::new();

/// GPU オーナースレッドを遅延起動し、その送信端を返す。
///
/// オーナースレッドは MTA 初期化（本番 UI スレッドと同じ・`WucGraphicsResource::new` の
/// `DQTAT_COM_NONE` 経路が要求）し、受信したジョブを**逐次**実行する。プロセス終了まで常駐。
fn owner_sender() -> &'static Sender<Job> {
    OWNER.get_or_init(|| {
        let (tx, rx) = channel::<Job>();
        std::thread::Builder::new()
            .name("wuc-gpu-owner".into())
            .spawn(move || {
                // 本番 UI スレッドと同一の COM 初期化（MTA）。S_FALSE/RPC_E_CHANGED_MODE は無視。
                unsafe {
                    let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
                }
                // 全ジョブを逐次実行（GPU/WUC 操作は常にこの単一スレッド上）。
                while let Ok(job) = rx.recv() {
                    job();
                }
            })
            .expect("spawn wuc-gpu-owner thread 失敗");
        tx
    })
}

/// 専用 GPU オーナースレッド上でクロージャ `f` を実行し、結果を返す。
///
/// WUC(Compositor) を生成・操作・破棄するテスト本体をこの関数でラップすると、当該処理は
/// すべてプロセス内単一のオーナースレッド上で走り、「別スレッド上の 2 個目 Compositor」＝
/// `STATUS_ACCESS_VIOLATION` を構造的に排除できる。
///
/// クロージャ内の `panic!`（テストアサーション失敗を含む）は捕捉してテストスレッドへ**転送**し、
/// テスト失敗として観測させる（サイレント成功なし・Requirement 5.3）。
///
/// # Panics
/// `f` が panic した場合、その payload をテストスレッド上で `resume_unwind` する
/// （＝当該 `#[test]` が失敗する）。
pub fn on_gpu_owner_thread<T, F>(f: F) -> T
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    let (rtx, rrx) = channel::<std::thread::Result<T>>();
    let job: Job = Box::new(move || {
        // クロージャの panic を捕捉してテストスレッドへ転送する（オーナースレッドを殺さない）。
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        let _ = rtx.send(result);
    });
    owner_sender()
        .send(job)
        .expect("GPU オーナースレッドへのジョブ送信失敗（スレッド死亡？）");
    match rrx
        .recv()
        .expect("GPU オーナースレッドからの結果受信失敗（スレッド死亡？）")
    {
        Ok(value) => value,
        Err(panic_payload) => std::panic::resume_unwind(panic_payload),
    }
}

mod fixture_self_tests {
    use super::*;
    use windows::Win32::System::WinRT::DQTAT_COM_NONE;
    use windows::UI::Composition::Compositor;
    use wintf::com::wuc::create_dispatcher_queue_controller;
    use wintf::ecs::{GraphicsCore, WucGraphicsResource};

    /// フルスタック（GraphicsCore + WucGraphicsResource＝Compositor）を生成→破棄する最小操作。
    fn make_and_drop_full_stack() {
        let core = GraphicsCore::new().expect("GraphicsCore::new");
        let d2d = core.d2d_device().expect("d2d_device");
        let wuc = WucGraphicsResource::new(d2d).expect("WucGraphicsResource::new");
        assert!(wuc.is_valid());
        // 最小合成操作: Compositor から SpriteVisual を 1 個作る。
        let _visual = wuc
            .compositor()
            .expect("compositor")
            .CreateSpriteVisual()
            .expect("CreateSpriteVisual");
        // スコープ終端で core/wuc が同一（オーナー）スレッド上で drop される。
    }

    /// 単独実行: オーナースレッド上で 1 回フルスタック生成→破棄が完走する（要件 1.1）。
    #[test]
    fn owner_thread_single_full_cycle() {
        on_gpu_owner_thread(make_and_drop_full_stack);
    }

    /// 複数テスト連続実行の縮図: 同一オーナースレッド上で **2 回連続** フルスタック生成→破棄しても
    /// クラッシュしない（別スレッド 2 個目なら AV する構造を、オーナースレッド集約で回避・要件 1.1/1.3）。
    #[test]
    fn owner_thread_two_full_cycles_no_crash() {
        on_gpu_owner_thread(make_and_drop_full_stack);
        on_gpu_owner_thread(make_and_drop_full_stack);
    }

    /// 値の受け渡しと DispatcherQueue 単体生成もオーナースレッド上で成立すること。
    #[test]
    fn owner_thread_returns_value() {
        let n = on_gpu_owner_thread(|| {
            let _dq = create_dispatcher_queue_controller(DQTAT_COM_NONE)
                .expect("DispatcherQueueController");
            let _c = Compositor::new().expect("Compositor::new");
            42usize
        });
        assert_eq!(n, 42);
    }
}
