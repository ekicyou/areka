//! `EmoPresenter`（presenter.rs）: 提示段の統括ハブ（合成・キャッシュ・表示・マスクの一点結線）。
//!
//! 上流が組んだ部品（[`ComposeCache`]・[`SwapChainPresenter`]・[`VisualMount`]・emo-compose の
//! [`Composer`]）を target ごとに束ね、指令 [`PresentCommand`] を UI スレッド上で適用する。合成そのもの
//! は emo-compose、供給面バイト転送は `SwapChainPresenter`、当たり判定マスクは wintf hit-test が担い、
//! 本型は「指令を受けて、キャッシュ引き当て or 合成 → 供給面アップロード → AlphaMask 同期 → 可視制御」を
//! **一続きの UI スレッド呼び出し**として結線する（design §EmoPresenter・§System Flows 指令適用）。
//!
//! # UI スレッドアフィニティ（型で強制・R7.1）
//!
//! `EmoPresenter` は COM/GPU 資源（`SwapChainPresenter` 内の DXGI/D3D11・`VisualMount` の WUC visual）を
//! 内包するため **`!Send`**（NonSend）である。`unsafe impl Send` は置かず、`PhantomData<*const ()>` を
//! 併せ持つことで「他スレッドへ移動できない」ことを**構造（型）で**担保する。wintf World へ NonSend
//! 資源として登録するか example が直接所有し、`apply`/`attach_target`/`read_back` は必ず UI スレッド
//! （NonSend 到達可能スレッド）から呼ばれる（design §Responsibilities & Constraints）。
//!
//! # 原子入替（R2.4）
//!
//! 表示バッファ（`chain.upload`）と当たり判定マスク（`AlphaMaskResource::set`）の更新は**同一 `apply`
//! 呼び出し内**で連続して起き、hit-test も同一 UI スレッドで走るため中間状態は観測不能である。ゆえに
//! surface 切替に伴う「表示とマスクの対入替」は構造的に原子化される（別途ロック不要）。
//!
//! # 失敗経路のログ規律（silent failure 禁止）
//!
//! 全失敗分岐は返す前に `tracing::error!`/`warn!` を出す。`ComposeError::SurfaceNotFound`（解決不能 id）は
//! **error! ＋ 表示不変 ＋ reply `Err`**（R3.4）、`ComposeError::EmptyComposition`（全透明退化）は
//! **warn! ＋ Hide 縮退 ＋ reply `Ok`**（設計ディスカッション #1: 許容される正常退化・skip 解釈は採らない）、
//! デバイス層失敗は `PresentError::Device`（HRESULT ＋文脈）で `Err`。panic は用いない。
//!
//! # 表示スケール k の適用漏斗（emo-dpi-scaling・要件 1.1/1.2/1.5・2.1-2.4）
//!
//! DPI 追従表示の係数 k は **`ShowSurface` の適用ごと**に導出する（design Flow 1「k 導出は show 適用ごと
//! に行う」）——target へ焼き付けず、`attach` でも決めない。これにより「照会値＝実適用 k」の不変条件を
//! 維持する点が経路上の 1 箇所（表示成立点）に閉じる。
//!
//! 経路は `world.get::<DPI>(target.window)` → [`derive_scale`]（政策＝[`ScalePolicy`]・縮退は log-first）
//! → `cache.get(.., k)` → ミス時のみ `compose`（**native 原寸**）→ [`resample`]（native → k 適用）→
//! `cache.insert(.., k, ..)` である。以降の供給面アップロード・`AlphaMaskResource` 同期・`set_bounds`・
//! 可視制御は**既存コードのまま**で、流れる合成結果が k 適用済みになるだけで自動追従する
//! （design 「Strategy A2＝composed 外形従属の連鎖を k 追従へ転用」）。
//!
//! k=1/1（窓 DPI ＝ author_dpi）は [`resample`] を**呼ばずに** native をそのまま表示資源とする——
//! [`resample`] 自体も恒等をバイトコピーで保証するが、素通しなら「k 導入前と同一のオブジェクトが同一経路を
//! 流れる」ことが構造で言えるため、既存 golden の不変（要件 7.2）が最も強く担保される。

// 責務単位のサブモジュール。すべて私有 `mod` であり、新しい公開モジュールパスは生やさない
// （公開項目は下の `pub use` で従来と同一のパス `presenter::<Name>` に再輸出する）。
mod hit;
mod hub;
mod read;
mod refresh;
mod show;
mod target;
mod visibility;

// 分割前の `use` 一式はここに残す。テストファイル 8 本は `use super::*;` で本モジュールの束縛から
// 外部クレート型を拾っており、子モジュールへ分配してここから落とすと test ビルドが壊れるためである。
// 子モジュールも同じ束縛を `use super::{…};` 経由で引くので、非 test ビルドでも全 import が消費され、
// 未使用インポートの抑止指示は 1 つも要らない。

use std::collections::HashMap;
use std::marker::PhantomData;

use bevy_ecs::entity::Entity;
use bevy_ecs::world::World;

use areka_actor::{ReplySender, reply_channel};
use areka_emo_atlas::AtlasTable;
use areka_emo_compose::{
    BindSet, ComposeError, ComposedSurface, Composer, EmoWorld, PatternState, RegionPriority,
    ScaleRatio, hit_region_scaled, resample,
};

use wintf::ecs::{AlphaMaskResource, DPI, GraphicsCore, WucGraphicsResource};

use crate::cache::ComposeCache;
use crate::chain::SwapChainPresenter;
use crate::command::{PresentCommand, PresentError, PresentOutcome, TargetId};
use crate::mount::VisualMount;
use crate::scale::{ScalePolicy, derive_scale};

pub use self::hit::ClientHit;
pub use self::hub::EmoPresenter;
pub use self::read::TextSlotView;
pub use self::target::VisibilityOwnership;
use self::target::PresentTarget;

#[cfg(test)]
#[path = "presenter_test_support.rs"]
mod test_support;
#[cfg(test)]
#[path = "presenter_display_tests.rs"]
mod display_tests;
#[cfg(test)]
#[path = "presenter_compose_input_tests.rs"]
mod compose_input_tests;
#[cfg(test)]
#[path = "presenter_read_accessor_tests.rs"]
mod read_accessor_tests;
#[cfg(test)]
#[path = "presenter_dpi_scale_tests.rs"]
mod dpi_scale_tests;
#[cfg(test)]
#[path = "presenter_resize_report_tests.rs"]
mod resize_report_tests;
#[cfg(test)]
#[path = "presenter_refresh_and_log_tests.rs"]
mod refresh_and_log_tests;
#[cfg(test)]
#[path = "presenter_fractional_scale_tests.rs"]
mod fractional_scale_tests;
#[cfg(test)]
#[path = "presenter_visibility_tests.rs"]
mod visibility_tests;
