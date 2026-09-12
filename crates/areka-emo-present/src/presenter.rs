//! `EmoPresenter`（presenter.rs）: 提示段の統括ハブ（合成・キャッシュ・表示・マスクの一点結線）。
//!
//! 上流が組んだ部品（[`ComposeCache`]・`display::record_display`・[`VisualMount`]・emo-compose の
//! [`Composer`]）を target ごとに束ね、指令 [`PresentCommand`] を UI スレッド上で適用する。合成そのもの
//! は emo-compose、表示の記録（原寸 D2D bitmap → 描画命令）は `display.rs`、面の生成・変換・描画は
//! wintf のコマンドリスト経路、当たり判定マスクは wintf hit-test が担い、本型は「指令を受けて、
//! キャッシュ引き当て or 合成＋記録 → 表示記録・配置の書き込み → AlphaMask 同期 → 可視制御」を
//! **一続きの UI スレッド呼び出し**として結線する（design §EmoPresenter・§System Flows 指令適用）。
//!
//! # UI スレッドアフィニティ（型で強制・R7.1）
//!
//! `EmoPresenter` は COM/GPU 資源（合成メモが保持する `GraphicsCommandList`＝D2D コマンドリスト）を
//! 内包するため **`!Send`**（NonSend）である。`unsafe impl Send` は置かず、`PhantomData<*const ()>` を
//! 併せ持つことで「他スレッドへ移動できない」ことを**構造（型）で**担保する。wintf World へ NonSend
//! 資源として登録するか example が直接所有し、`apply`/`attach_target`/`read_back` は必ず UI スレッド
//! （NonSend 到達可能スレッド）から呼ばれる（design §Responsibilities & Constraints）。
//!
//! # 原子入替（R2.4）
//!
//! 表示記録（`VisualMount::set_display`）・配置（`set_layout`）と当たり判定マスク
//! （`AlphaMaskResource::set_shared`）の更新は**同一 `apply` 呼び出し内**で連続して起き、hit-test も
//! 同一 UI スレッドで走るため中間状態は観測不能である。ゆえに surface 切替に伴う「表示とマスクの
//! 対入替」は構造的に原子化される（別途ロック不要）。
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
//! → `cache.touch(surface, binds, pattern)`（**k はキーでない**）→ ミス時のみ `compose`（**native
//! 原寸**）＋ `record_display`（原寸の表示記録）→ `cache.insert(..)` である。k は
//! `VisualMount::set_layout` が surface entity の `Arrangement.scale` へ書く**変換の係数**としてだけ
//! 現れ、拡大は wintf の `render_surface`（`SetTransform`）が行う（`areka-P0-present-gpu-transform-scale`
//! 裁定 D）。ゆえに k=1 と k≠1 で通る手順は同じで、k だけが変わった再適用は再合成なしのヒットになる。

// 責務単位のサブモジュール。すべて私有 `mod` であり、新しい公開モジュールパスは生やさない
// （公開項目は下の `pub use` で従来と同一のパス `presenter::<Name>` に再輸出する）。
mod budget;
mod hit;
mod hub;
mod read;
mod refresh;
mod show;
mod target;
mod timing;
mod transition_record;
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
    ScaleRatio, hit_region_scaled,
};
// `resample`（使い捨て作業領域を毎回起こす形）の消費者は**テストだけ**になった——本番のミス経路は
// `FrameBudget` の常設席を使う `resample_with` 側（`budget.rs`）へ移ったためである
// （`areka-P0-recompose-budget` task 5.3）。テスト 4 本が `use super::*;` でここから拾うので束縛は
// 残すが、非 test ビルドでは消費者が 1 人も居ないので `#[cfg(test)]` で畳む（抑止指示を足さない）。
#[cfg(test)]
use areka_emo_compose::resample;

use wintf::ecs::{AlphaMaskResource, DPI, GraphicsCore};
// `WucGraphicsResource` の本番消費者は無い（装着が純 ECS になり `Compositor` を要さない）。テストが
// `use super::*;` でここから拾うので、`resample` と同じく `#[cfg(test)]` で畳む。
#[cfg(test)]
use wintf::ecs::WucGraphicsResource;

use crate::cache::ComposeCache;
use crate::command::{PresentCommand, PresentError, PresentOutcome, TargetId};
use crate::mount::VisualMount;
use crate::scale::{ScalePolicy, derive_scale};

pub use self::hit::ClientHit;
pub use self::hub::EmoPresenter;
pub use self::read::TextSlotView;
use self::target::PresentTarget;
pub use self::target::VisibilityOwnership;
// 遷移観測のサーフェス記録の**語彙**（design C3・Requirement 2.7）。判定側（areka の
// `transition_judge`）は文字列リテラルを二重定義せずここを参照する。レコード型と純関数は
// 発行点の内部事情ゆえ再輸出しない（語だけを公開面へ出す）。
pub use self::transition_record::{
    KIND_SURFACE, SURFACE_FIELD_H, SURFACE_FIELD_REASON, SURFACE_FIELD_RESIZED,
    SURFACE_FIELD_TARGET_ID, SURFACE_FIELD_W, SURFACE_FIELDS, SURFACE_REASON_ALL,
    SURFACE_REASON_INVISIBLE, SURFACE_REASON_K_UNCHANGED, SURFACE_STAGE_ALL, SURFACE_STAGE_SKIPPED,
    SURFACE_STAGE_UPLOAD, SURFACE_STAGE_VISUALIZE,
};

#[cfg(test)]
#[path = "presenter_budget_steady_state_tests.rs"]
mod budget_steady_state_tests;
#[cfg(test)]
#[path = "presenter_compose_input_tests.rs"]
mod compose_input_tests;
#[cfg(test)]
#[path = "presenter_display_tests.rs"]
mod display_tests;
#[cfg(test)]
#[path = "presenter_dpi_scale_tests.rs"]
mod dpi_scale_tests;
#[cfg(test)]
#[path = "presenter_fractional_scale_tests.rs"]
mod fractional_scale_tests;
#[cfg(test)]
#[path = "presenter_hide_contract_tests.rs"]
mod hide_contract_tests;
#[cfg(test)]
#[path = "presenter_perf_log_tests.rs"]
mod perf_log_tests;
#[cfg(test)]
#[path = "presenter_read_accessor_tests.rs"]
mod read_accessor_tests;
#[cfg(test)]
#[path = "presenter_refresh_and_log_tests.rs"]
mod refresh_and_log_tests;
#[cfg(test)]
#[path = "presenter_resize_report_tests.rs"]
mod resize_report_tests;
#[cfg(test)]
#[path = "presenter_test_support.rs"]
mod test_support;
#[cfg(test)]
#[path = "presenter_upload_failure_tests.rs"]
mod upload_failure_tests;
#[cfg(test)]
#[path = "presenter_visibility_tests.rs"]
mod visibility_tests;

#[cfg(test)]
#[path = "presenter_budget_equivalence_tests.rs"]
mod budget_equivalence_tests;

#[cfg(test)]
#[path = "presenter_cache_capacity_tests.rs"]
mod cache_capacity_tests;
