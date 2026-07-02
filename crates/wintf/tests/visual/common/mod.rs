//! Shared helpers for visual tests

use std::sync::OnceLock;

use windows::UI::Composition::{Compositor, Visual};
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use windows::core::{Interface, Result};
use wintf::ecs::{GraphicsCore, WucGraphicsResource};

/// テスト用の GraphicsCore を作成するヘルパー関数
pub fn setup_graphics() -> Result<GraphicsCore> {
    GraphicsCore::new()
}

/// COM を MTA 初期化する（WucGraphicsResource::new は DQTAT_COM_NONE を使うため
/// COM 初期化済みスレッドを要求する）。冪等で、S_FALSE / RPC_E_CHANGED_MODE は無視する。
pub fn init_com_mta() {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
}

/// プロセス全体で共有する単一の `WucGraphicsResource`。
///
/// **根本原因（実測 2026-07-02）**: WUC の `DispatcherQueueController` / `Compositor` を
/// **同一プロセスで繰り返し生成すると 2 個目以降でネイティブ `STATUS_ACCESS_VIOLATION`**
/// を起こす（cargo test の per-test スレッドで顕在化・単独テストでは通る）。これは破棄方法
/// （素の drop でも `World` teardown drop でも）に依らず**生成の反復そのもの**が原因で、
/// World に載せても解消しない（visual スイートで実測確認済み）。本番は UI スレッド上の
/// 単一インスタンスがプロセス生存中ずっと生きるため問題にならない。
///
/// 対策: **プロセスで一度だけ生成し drop しない**（`OnceLock` 保持）。実 WUC 描画等価の
/// 検証は `examples/wuc_spike` ＋ task 4.x のハーネスが担い、本ヘルパーは階層同期ロジック
/// 検証用の実 `Visual` 供給に限定する。
struct SharedWuc {
    // resource は drop させないために保持し続ける（読み出しは compositor のみ）。
    #[allow(dead_code)]
    resource: WucGraphicsResource,
    compositor: Compositor,
}
// SAFETY: 本番 `WucGraphicsResource` と同一条件（UI スレッドアフィニティ）。テストは共有
// リソース経由で Visual を生成するのみで、同一オブジェクトへ並行アクセスしない。
unsafe impl Send for SharedWuc {}
unsafe impl Sync for SharedWuc {}

static SHARED_WUC: OnceLock<SharedWuc> = OnceLock::new();

/// WUC ビジュアルファクトリ。プロセス共有の単一 `WucGraphicsResource`（`Compositor`）から
/// テストで必要な実 WUC `Visual`（`SpriteVisual` を基底 `Visual` へ cast）を供給する。
///
/// `visual_hierarchy_sync_system` は親を `ContainerVisual` へ cast して `.Children()` を
/// 呼ぶため、生成する Visual は Children を持つ `SpriteVisual` である必要がある。
pub struct WucVisualFactory;

impl WucVisualFactory {
    /// プロセス共有の `WucGraphicsResource` を（未生成なら）初期化する。
    ///
    /// 引数 `_graphics` は API 互換のため受け取るが、共有リソースは初回のみ内部生成の
    /// `GraphicsCore` から構築する（2 回目以降は既存を再利用）。
    pub fn new(_graphics: &GraphicsCore) -> Result<Self> {
        init_com_mta();
        SHARED_WUC.get_or_init(|| {
            let core = GraphicsCore::new().expect("GraphicsCore作成失敗");
            let d2d = core.d2d_device().expect("D2Dデバイスが無効");
            let resource = WucGraphicsResource::new(d2d).expect("WucGraphicsResource作成失敗");
            let compositor = resource
                .compositor()
                .expect("compositor should exist")
                .clone();
            // core は resource 生成後は不要（compositor が D2D デバイスを内部保持）。drop 可。
            drop(core);
            SharedWuc {
                resource,
                compositor,
            }
        });
        Ok(Self)
    }

    /// 新しい WUC Visual（`SpriteVisual` を基底 `Visual` へ cast）を生成する。
    pub fn create_visual(&self) -> Result<Visual> {
        let compositor = &SHARED_WUC
            .get()
            .expect("WucVisualFactory::new を先に呼ぶこと")
            .compositor;
        let v: Visual = compositor.CreateSpriteVisual()?.cast()?;
        Ok(v)
    }
}
