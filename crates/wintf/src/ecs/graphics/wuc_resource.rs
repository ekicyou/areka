//! WucGraphicsResource — Windows.UI.Composition (WUC) 合成デバイスの遅延初期化管理
//!
//! GraphicsCore とは独立した ECS Resource として WUC デバイス群
//! （`Compositor` / `CompositionGraphicsDevice` / `DispatcherQueueController`）を
//! 管理する。WUC モードのウィンドウが初めて必要になった時点で初期化（遅延初期化）。
//!
//! `dcomp_resource.rs` の `DCompGraphicsResource` を構造の 1:1 テンプレートとし、
//! `Option<Inner>` パターン・`unsafe impl Send/Sync`・手動 `Debug`・`#[derive(Resource)]`
//! を写像する。DComp と並存する新規リソースであり、ライブ登録の切替は後続タスクの領分。

use bevy_ecs::prelude::*;
use tracing::{debug, info};
use windows::UI::Composition::{CompositionGraphicsDevice, Compositor};
use windows::Win32::Graphics::Direct2D::ID2D1Device;
use windows::Win32::System::WinRT::DQTAT_COM_NONE;
use windows::System::DispatcherQueueController;
use windows::core::Result;

use crate::com::wuc::{CompositorInteropExt, create_dispatcher_queue_controller};

/// WUC 合成デバイス群の内部構造体。
///
/// フィールド宣言順は `compositor` → `graphics_device` → `dq_controller`
/// （**controller を最後**）で固定する。Rust の宣言順 drop により
/// DispatcherQueueController が Compositor より**後**に drop され、終了時ドレインが
/// 成立する（要件 3.3・design.md「Drop 順の不変条件」）。
struct WucGraphicsResourceInner {
    compositor: Compositor,
    graphics_device: CompositionGraphicsDevice,
    // dq_controller は読み出さず、Compositor より後に drop させるためだけに保持する
    // （終了時ドレインの前提・要件 3.3）。ゆえに dead_code を許可する。
    #[allow(dead_code)]
    dq_controller: DispatcherQueueController,
}

/// WUC 合成デバイス群（`Compositor` / `CompositionGraphicsDevice` /
/// `DispatcherQueueController`）を遅延初期化し、WUC パイプライン専用リソースとして
/// 管理する ECS Resource。
///
/// `Option<Inner>` パターンにより、`invalidate()` でリソースを無効化し、
/// 次フレームで再初期化可能。
#[derive(Resource)]
pub struct WucGraphicsResource {
    inner: Option<WucGraphicsResourceInner>,
}

// SAFETY 条件: 保持する COM/WinRT ポインタの跨スレッド移動（Send）・参照共有（Sync）は
// 「同一オブジェクトへの呼び出しが同時に実行されない」ことを前提とする。
// WUC の Compositor / CompositionGraphicsDevice / DispatcherQueueController は
// UI スレッドアフィニティを持ち、ワーカーからは channel marshal（schedule 上で
// 並行アクセスを排除）する構成が安全性条件を担う（DComp 系 dcomp_resource.rs と同一方針）。
unsafe impl Send for WucGraphicsResource {}
unsafe impl Sync for WucGraphicsResource {}

impl std::fmt::Debug for WucGraphicsResource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WucGraphicsResource")
            .field("is_valid", &self.is_valid())
            .finish()
    }
}

impl WucGraphicsResource {
    /// WUC 合成デバイスを初期化する。GraphicsCore の D2D デバイスに依存。
    ///
    /// # 初期化順序（design.md §State Management）
    /// 1. `create_dispatcher_queue_controller(DQTAT_COM_NONE)`（Compositor 生成前・要件 3.1）。
    ///    apartment は `DQTAT_COM_NONE`（R1 スパイク実測確定: 本番 UI スレッドは MTA であり、
    ///    MTA では apartment 不変の NONE が正解。ASTA は MTA で RPC_E_CHANGED_MODE になる）。
    /// 2. `Compositor::new()`。
    /// 3. `compositor.cast::<ICompositorInterop>().CreateGraphicsDevice(d2d_device)`。
    ///
    /// # Arguments
    /// * `d2d_device` - D2D デバイス参照（GraphicsCore から取得）
    ///
    /// # Errors
    /// COM/WinRT デバイス初期化に失敗した場合
    pub fn new(d2d_device: &ID2D1Device) -> Result<Self> {
        info!("[WucGraphicsResource] Initialization started");

        // (1) DispatcherQueueController（Compositor 生成前・apartment は NONE）。
        let dq_controller = create_dispatcher_queue_controller(DQTAT_COM_NONE)?;

        // (2) Compositor。
        let compositor = Compositor::new()?;

        // (3) 既存 D2D デバイスから CompositionGraphicsDevice。
        let interop: windows::Win32::System::WinRT::Composition::ICompositorInterop =
            windows::core::Interface::cast(&compositor)?;
        let graphics_device = interop.create_graphics_device(d2d_device)?;

        info!("[WucGraphicsResource] Initialization completed");

        // controller 最後の順で構築（宣言順 drop 保証）。
        Ok(Self {
            inner: Some(WucGraphicsResourceInner {
                compositor,
                graphics_device,
                dq_controller,
            }),
        })
    }

    /// 未初期化状態の WucGraphicsResource を作成する。
    ///
    /// テスト用。`is_valid()` は `false` を返す。
    pub fn new_empty() -> Self {
        Self { inner: None }
    }

    /// 全 WUC リソースを無効化する。
    ///
    /// デバイスロスト時に再初期化トリガから呼ばれる。次フレームの init 経路で再生成される。
    /// `inner = None` により宣言順 drop（controller を最後に解放）される。
    pub fn invalidate(&mut self) {
        if self.inner.is_some() {
            debug!("[WucGraphicsResource] Invalidated");
        }
        self.inner = None;
    }

    /// WUC デバイスが有効かどうかを返す。
    pub fn is_valid(&self) -> bool {
        self.inner.is_some()
    }

    /// Compositor への Option アクセサ
    pub fn compositor(&self) -> Option<&Compositor> {
        self.inner.as_ref().map(|i| &i.compositor)
    }

    /// CompositionGraphicsDevice への Option アクセサ
    pub fn graphics_device(&self) -> Option<&CompositionGraphicsDevice> {
        self.inner.as_ref().map(|i| &i.graphics_device)
    }
}

#[cfg(test)]
mod tests {
    use crate::ecs::graphics::wuc_resource::WucGraphicsResource;
    use crate::ecs::graphics::GraphicsCore;
    use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};

    /// WucGraphicsResource の遅延ライフサイクル統合テスト（要件 2.2 / 3.3）。
    ///
    /// 本番同様に COM を MTA 初期化してから `new` を呼ぶ（`new()` は
    /// `DQTAT_COM_NONE` を使うため COM 未初期化スレッドでは失敗する）。
    /// `DispatcherQueue` は 1 スレッド 1 つのため、全アサーションを 1 本に集約する。
    #[test]
    fn wuc_graphics_resource_lifecycle() {
        // 本番の UI スレッド（MTA）を再現。S_FALSE / RPC_E_CHANGED_MODE は無視。
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }

        // GraphicsCore から D2D デバイスを得て new。
        let core = GraphicsCore::new().expect("GraphicsCore::new 失敗（HARDWARE デバイス生成）");
        let d2d = core.d2d_device().expect("d2d_device が None");

        let resource = WucGraphicsResource::new(d2d).expect("WucGraphicsResource::new 失敗");

        // is_valid / アクセサが Some を返すこと。
        assert!(resource.is_valid(), "new 後は is_valid が true");
        assert!(resource.compositor().is_some(), "compositor() が Some");
        assert!(
            resource.graphics_device().is_some(),
            "graphics_device() が Some"
        );

        // invalidate → 無効化。
        let mut resource = resource;
        resource.invalidate();
        assert!(!resource.is_valid(), "invalidate 後は is_valid が false");
        assert!(
            resource.compositor().is_none(),
            "invalidate 後 compositor() が None"
        );
        assert!(
            resource.graphics_device().is_none(),
            "invalidate 後 graphics_device() が None"
        );

        // new_empty は無効。
        let empty = WucGraphicsResource::new_empty();
        assert!(!empty.is_valid(), "new_empty は is_valid が false");

        // ドレイン/drop 順の健全性: 生きた Resource を drop してもクラッシュしないこと
        // （controller 最後 drop の健全性・要件 3.3）。スコープ終了で drop される。
        let live = WucGraphicsResource::new(d2d).expect("再生成失敗");
        assert!(live.is_valid());
        drop(live); // controller が compositor より後に drop される（宣言順）— panic しないこと。

        println!("[TEST PASS] WucGraphicsResource lifecycle (new/invalidate/new_empty/drop)");
    }
}
