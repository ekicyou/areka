//! 毎フレーム結線（attach／dpi／drain／text）の排他 system と NonSend 配線状態。
//!
//! `Emo2Wiring`（NonSend resource・presenter／rx／runtime／clock／assets／attached を保持）と
//! 排他 system `emo2_frame_system(world: &mut World)`（donor パターン: remove→各フェーズ→insert）を
//! 所有する。各フェーズ:
//! - attach: GPU 資源＋`GhostWindows` 到達ゲート→`plan_attachments`（DD-12）→バルーン初回 `ShowSurface`
//!   （面0）→文字層スロット取得→`register_actor_view`（`Option::take` で高々 1 回消費）。**シェルは初回
//!   `ShowSurface` を発行せず**最初のさくらスクリプト `\s` cue まで非表示を保つ（defect #5・実機#5）。
//! - dpi: `Changed<DPI>` の窓を永続 `SystemState` で観測し（`anchor_changed_system` 先例）、当該窓の
//!   target を `refresh_scale` で再スケールして窓寸を reconcile する（emo-dpi-scaling task 4.2・D8）。
//! - drain: attach 完了後のみ `Receiver::try_iter` で `PresentCommand` を FIFO で `presenter.apply` へ適用し、
//!   続けて表示成立点の状態照合報告（`take_pending_resize`）で窓寸を reconcile する（第 2 経路）。
//! - text-scale: 装着済み balloon scope の文字層 binding を presenter の**現適用 k** へ毎フレーム
//!   合わせ直す（`refresh_actor_scale`・emo-dpi-scaling task 7.2・D11-4・Req8）。適用 k の更新点は
//!   1 フレームに 2 つ（dpi 相の `refresh_scale`／drain 相の `apply_show`）あるため、**両者の下流**・
//!   text 相の**上流**に置く。
//! - text: `TalkClock::talk_time` が `Some` のとき `present_frame` を呼ぶ（`Err` は `error!`＋継続）。
//!
//! `plan_attachments`（`GhostWindows::scopes()` を正とする純関数・DD-12）も本モジュールに属する。
//!
//! 本ファイルは task 3 の純関数 `plan_attachments`（＋`AttachPlan`／`PlannedAttach`）、task 4.1 の
//! NonSend 結線資源 `Emo2Wiring` と attach フェーズ（`run_attach_phase`＋補助 `connect_balloon_text`）、
//! そして task 4.2 の drain フェーズ（`run_drain_phase`）・text フェーズ（`run_text_phase`＋純判断
//! `resolve_talk_time`）・排他 system `emo2_frame_system`（remove→3 フェーズ→insert）を実装する。

mod attach;
mod dpi;
mod drain_resnap;
mod scale_text;
mod wiring;

// 移設前の本モジュール本体が使っていた import 一式をそのまま保持する。本番項目は各サブモジュール
// へ移り、そちらが同じ経路を直接 import するため、ここに残る束縛の役目は 2 つである——
// (1) `super::…`（emo2_boot 兄弟）の 4 本はサブモジュールが `super::` で辿る先、
// (2) それ以外は末尾のテストモジュール群の `use super::*;` が従来どおり拾うための束縛。
// (2) の消費者は `#[cfg(test)]` だけなので、`areka`（lib target を持たない bin クレート）では
// 非 test ビルド単位で未使用に見える。実際に未使用と判定された文にのみ抑止を付ける。
#[allow(unused_imports)]
use std::cell::RefCell;
#[allow(unused_imports)]
use std::collections::{BTreeSet, HashMap};
#[allow(unused_imports)]
use std::rc::Rc;
#[allow(unused_imports)]
use std::sync::mpsc::Receiver;

#[allow(unused_imports)]
use bevy_ecs::entity::Entity;
#[allow(unused_imports)]
use bevy_ecs::prelude::{Changed, Query};
#[allow(unused_imports)]
use bevy_ecs::system::SystemState;
use bevy_ecs::world::World;
#[allow(unused_imports)]
use tracing::{debug, error, info, warn};

#[allow(unused_imports)]
use areka_emo_present::{EmoPresenter, PresentCommand, TargetId, TextSlotView};
#[allow(unused_imports)]
use areka_emo_text::actor::{present_frame, TextLayerRuntime};
#[allow(unused_imports)]
use areka_parsers::balloon::BalloonModel;
#[allow(unused_imports)]
use areka_sakura::ActorKey;
#[allow(unused_imports)]
use wintf::ecs::{FrameTime, GraphicsCore, SizeI, WindowPos, WucGraphicsResource, DPI};

#[allow(unused_imports)]
use crate::placement::diag::{DESPAWNED_SKIP_TAG, PlacementRoute};
#[allow(unused_imports)]
use crate::placement::follow::{resize_window_keep_position, resize_window_to};
#[allow(unused_imports)]
use crate::placement::resolver::SizePx;
#[allow(unused_imports)]
use crate::placement::spawn::{BalloonWindowMarker, CharWindowMarker, GhostWindows};

use super::assets::{BalloonScopeAssets, BootAssets, ScopeAssets};
// `scale_text::run_text_phase` が `super::hover_inject::drive` で辿る先（移設前は本ファイルから
// `super::hover_inject` が emo2_boot を指していた）。呼び出し本文を書き換えないための束縛。
use super::hover_inject;
use super::move_cue::{apply_move_directive, MoveDirective};
use super::talk_clock::TalkClock;
use super::target_map::{balloon_target, shell_target};

// 公開面の維持（要件 2.5）だけを担う再輸出。このグループで非 test ビルドの未使用は
// `AttachPlan`（`plan_attachments` の戻り値型ゆえ公開パスの維持が要件・参照は定義元の
// サブモジュール内で完結する）と `plan_attachments`（`frame::` 経由で呼ぶのは
// `#[cfg(test)]` の `attach_tests` だけ）の 2 名。`PlannedAttach` は `dpi` サブモジュールが、
// `run_attach_phase` は下の `emo2_frame_system` が非 test ビルドでも使う。属性は use 文単位で
// 効くためグループ全体に付く（`areka` は lib target を持たない bin クレート）。
#[allow(unused_imports)]
pub use self::attach::{AttachPlan, PlannedAttach, plan_attachments, run_attach_phase};
pub use self::dpi::run_dpi_phase;
pub use self::drain_resnap::{run_drain_phase, run_move_drain_phase};
pub use self::scale_text::{run_text_phase, run_text_scale_phase};
pub use self::wiring::Emo2Wiring;

// 私有項目のファサード再束縛（クレート内可視性のみ・公開面は増やさない）。サブモジュール間の
// 相互参照とテストモジュールからの `use super::*;` は、いずれもここを経由する。
// `connect_balloon_text` の参照は定義元 `attach` 内で完結するため、`frame::` 経由の消費者は
// `#[cfg(test)]` の `attach_tests` だけである。
#[allow(unused_imports)]
use self::attach::connect_balloon_text;
// 同上。ただしこのグループで実際に未使用と判定されるのは `GhostWindowClass`・
// `classify_ghost_window`・`dpi_phase_with`・`reproject_char_window_at_current_size` の 4 名
// （いずれも参照が定義元 `dpi` 内で完結する）。残る `AuthorDpis`・`DpiChangedQuery`・
// `GhostWindowKind`・`ScaleReportSource`・`reconcile_window_size` は非 test ビルドでも
// `attach`／`wiring`／`scale_text` が消費している。
#[allow(unused_imports)]
use self::dpi::{
    AuthorDpis, DpiChangedQuery, GhostWindowClass, GhostWindowKind, ScaleReportSource,
    classify_ghost_window, dpi_phase_with, reconcile_window_size,
    reproject_char_window_at_current_size,
};
// 同上。未使用は `PhysicalSizeSource`・`resnap_from_sizes`・`resnap_with` の 3 名で、
// `resnap_shell_targets` は下の `emo2_frame_system` が呼ぶ。
#[allow(unused_imports)]
use self::drain_resnap::{PhysicalSizeSource, resnap_from_sizes, resnap_shell_targets, resnap_with};
// 同上。未使用は `resolve_talk_time` のみで、`reconcile_reported_sizes` は
// `drain_resnap::run_drain_phase` が非 test ビルドでも呼ぶ。
#[allow(unused_imports)]
use self::scale_text::{reconcile_reported_sizes, resolve_talk_time};

/// `FrameFinalize` 登録の排他 system（donor パターン: remove→3 フェーズ→insert・DD-1/DD-4）。
///
/// `Emo2Wiring`（NonSend）を [`World::remove_non_send_resource`] で取り出してから
/// attach→drain→text の 3 フェーズを順に駆動し、[`World::insert_non_send_resource`] で戻す。
/// remove→insert は `&mut World` を各フェーズへ排他に渡すための donor 慣行（借用衝突回避・
/// `examples/emo-present.rs::boot_present_system` と同型）。本番の text フェーズは override 無し
/// （`FrameTime`＋`TalkClock` で `talk_time` を解決）。
///
/// `Emo2Wiring` 未挿入（`wire_emo2_boot`＝task 5.1 前・フォールバック boot 経路）なら早期 return の
/// no-op（安全・panic しない）。schedule への登録（`add_systems(FrameFinalize, emo2_frame_system)`）は
/// `wire_emo2_boot`（task 5.1）が行い、本関数はここでは定義のみ（登録しない）。
pub fn emo2_frame_system(world: &mut World) {
    // Emo2Wiring 未挿入（wire_emo2_boot=task 5.1 前・LogSink フォールバック boot 経路）なら no-op。
    let Some(mut wiring) = world.remove_non_send_resource::<Emo2Wiring>() else {
        return;
    };
    // donor 慣行: remove して &mut World を各フェーズへ排他に渡し、全フェーズ駆動後に必ず戻す。
    run_attach_phase(&mut wiring, world);
    // DPI 追従（attach → dpi → drain …の順・design「run_dpi_phase（frame.rs）」）: attach の**後**に
    // 置く——装着前は再スケール対象の target が無く、attach と同一フレームで窓が生えた直後の
    // `Changed<DPI>`（生成時 GetDpiForWindow 実値補正）を同フレームで拾えるようにするため。drain の
    // **前**に置く理由は責任分界であり順序依存ではない: エッジ駆動の再表示を先に済ませ、残った
    // 未消費要求（初回表示の k₀ 補正）を drain 末尾の状態照合経路が拾う（両経路は presenter の
    // 消費規約により二重にも取りこぼしにもならない＝どちらの順でも整合する）。
    run_dpi_phase(&mut wiring, world);
    run_drain_phase(&mut wiring, world);
    // `\![move]` の末端結線: talk スレッドの MoveCueSink から届いた MoveDirective を drain し
    // apply_move_directive で実窓へ即時反映する（GhostWindows ゲート・R5・task 9.2）。present drain
    // とは独立で、GPU attach でなく GhostWindows 存在を待つ（move はキャラ窓 entity へ作用するため）。
    run_move_drain_phase(&wiring, world);
    // drain（全 PresentCommand 適用）後に shell サーフェス寸法の変化を検知し、変化した char 窓のみ
    // アンカー再適用を駆動する（適用後の実寸を読むため drain の**後**・同一 World・同一 tick 内の
    // 直接呼び・Req4.1/4.3/1.3）。text の前後とは機能的に無関係だが drain の後であることが必須。
    resnap_shell_targets(&wiring.presenter, world);
    // 文字層 k 追従（emo-dpi-scaling task 7.2・D11-4・Req8）: 適用 k の更新点（dpi 相の `refresh_scale`
    // ／drain 相の `apply_show`）の**両方の下流**、かつ `present_frame` の**上流**に置く。こうすると
    // どちらの経路で k が跳ねても同一フレーム内で binding が新 k へ組み直され、直後の描画が新しい物理寸
    // で走る（旧寸の文字が 1 フレーム残らない）。戻り値（再構築 scope）は観測用ゆえ本番は捨てる。
    let _ = run_text_scale_phase(&mut wiring);
    run_text_phase(&mut wiring, world, None); // 本番: override なし（FrameTime＋clock で解決）。
    world.insert_non_send_resource(wiring);
}

#[cfg(test)]
#[path = "frame_test_support.rs"]
mod test_support;

#[cfg(test)]
#[path = "frame_attach_tests.rs"]
mod attach_tests;

#[cfg(test)]
#[path = "frame_drain_text_tests.rs"]
mod drain_text_tests;

#[cfg(test)]
#[path = "frame_resnap_tests.rs"]
mod resnap_tests;

#[cfg(test)]
#[path = "frame_dpi_tests.rs"]
mod dpi_tests;

#[cfg(test)]
#[path = "frame_dpi_reproject_tests.rs"]
mod dpi_reproject_tests;

#[cfg(test)]
#[path = "frame_dpi_reproject_none_tests.rs"]
mod dpi_reproject_none_tests;

#[cfg(test)]
#[path = "frame_diag_route_tests.rs"]
mod diag_route_tests;

#[cfg(test)]
#[path = "frame_text_scale_tests.rs"]
mod text_scale_tests;
