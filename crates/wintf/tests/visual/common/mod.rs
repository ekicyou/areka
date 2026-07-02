//! Shared helpers for visual tests

use std::sync::OnceLock;

use windows::UI::Composition::Visual;
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
/// WUC の `Compositor` / `DispatcherQueueController` は「1 スレッド 1 DispatcherQueue」で、
/// 同一プロセス内で **DispatcherQueue を繰り返し生成・破棄すると 2 個目以降でネイティブ
/// `STATUS_ACCESS_VIOLATION`** を起こす（本番は UI スレッド上の単一インスタンスがプロセス
/// 生存中ずっと生きるため問題にならない）。テストでは**プロセスで一度だけ生成し drop しない**
/// （`OnceLock` に保持）ことで、この反復生成クラッシュを回避する。実 WUC 描画等価の検証は
/// `examples/wuc_spike` ＋ task 4.x のハーネスが担い、本ヘルパーは階層同期ロジック検証用の
/// 実 `Visual` 供給に限定する。
struct SharedWuc(WucGraphicsResource);
// SAFETY: 本番 `WucGraphicsResource` と同一条件（UI スレッドアフィニティ）。テストは共有
// リソース経由で Visual を生成するのみで、同一オブジェクトへ並行アクセスしない。
unsafe impl Send for SharedWuc {}
unsafe impl Sync for SharedWuc {}

static SHARED_WUC: OnceLock<SharedWuc> = OnceLock::new();

/// WUC ビジュアルファクトリ。プロセス共有の単一 `WucGraphicsResource`（`Compositor`）から
/// テストで必要な実 WUC Visual（`SpriteVisual` を基底 `Visual` へ cast）を生成する。
///
/// `visual_hierarchy_sync_system` は親を `ContainerVisual` へ cast して `.Children()` を
/// 呼ぶため、生成する Visual は Children を持つ `SpriteVisual` である必要がある。
pub struct WucVisualFactory;

impl WucVisualFactory {
    /// プロセス共有の `WucGraphicsResource` を（未生成なら）初期化する。
    ///
    /// `graphics` は初回のみ生成に使われる（生成済みなら無視）。呼び出し側スレッドの COM を
    /// MTA 初期化してから行う（`init_com_mta`）。
    pub fn new(graphics: &GraphicsCore) -> Result<Self> {
        init_com_mta();
        SHARED_WUC.get_or_init(|| {
            let d2d = graphics.d2d_device().expect("D2Dデバイスが無効");
            SharedWuc(WucGraphicsResource::new(d2d).expect("WucGraphicsResource作成失敗"))
        });
        Ok(Self)
    }

    /// 新しい WUC Visual（`SpriteVisual` を基底 `Visual` へ cast）を生成する。
    pub fn create_visual(&self) -> Result<Visual> {
        let compositor = SHARED_WUC
            .get()
            .expect("WucVisualFactory::new を先に呼ぶこと")
            .0
            .compositor()
            .expect("compositor should exist");
        let v: Visual = compositor.CreateSpriteVisual()?.cast()?;
        Ok(v)
    }
}
