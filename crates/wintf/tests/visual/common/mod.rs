//! Shared helpers for visual tests

use windows::Foundation::Size;
use windows::Graphics::DirectX::{DirectXAlphaMode, DirectXPixelFormat};
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

/// WUC ビジュアルファクトリ。テストごとに新規 `WucGraphicsResource` を保持し、
/// そこから実 WUC `Visual`（`SpriteVisual` を基底 `Visual` へ cast）を供給する。
///
/// **teardown レース対策（実測 2026-07-02）**: メッセージポンプの無いヘッドレステストで
/// `WucGraphicsResource` を同一プロセスで複数回生成すると、2 個目以降の生成/使用で
/// `STATUS_ACCESS_VIOLATION` を起こすことがある。実測では **`CompositionDrawingSurface` を
/// 1 枚生成・保持すると DispatcherQueue が安定し、後続テストの生成が落ちなくなる**
/// （graphics ドメインの surface 系テストが混ざる場合に落ちない現象と整合）。ゆえに
/// ファクトリ生成時に「ならし用」サーフェスを 1 枚作って保持する。
pub struct WucVisualFactory {
    resource: WucGraphicsResource,
    // DispatcherQueue 安定化のための「ならし」サーフェス（読み出さないが保持する）。
    #[allow(dead_code)]
    warmup_surface: windows::UI::Composition::CompositionDrawingSurface,
}

impl WucVisualFactory {
    /// 新規 `GraphicsCore` + `WucGraphicsResource` を構築する。
    pub fn new(_graphics: &GraphicsCore) -> Result<Self> {
        init_com_mta();
        let core = GraphicsCore::new()?;
        let d2d = core.d2d_device().expect("D2Dデバイスが無効");
        let resource = WucGraphicsResource::new(d2d).expect("WucGraphicsResource作成失敗");
        // ならしサーフェスを 1 枚生成（DispatcherQueue 安定化）。
        let gd = resource.graphics_device().expect("graphics_device");
        let warmup_surface = gd.CreateDrawingSurface(
            Size {
                Width: 1.0,
                Height: 1.0,
            },
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            DirectXAlphaMode::Premultiplied,
        )?;
        Ok(Self {
            resource,
            warmup_surface,
        })
    }

    /// 新しい WUC Visual（`SpriteVisual` を基底 `Visual` へ cast）を生成する。
    pub fn create_visual(&self) -> Result<Visual> {
        let compositor = self.resource.compositor().expect("compositor should exist");
        let v: Visual = compositor.CreateSpriteVisual()?.cast()?;
        Ok(v)
    }
}
