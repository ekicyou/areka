//! バルーン追従コンポーネントと窓移動の公開 API。
//!
//! - [`BalloonFollow`]: キャラ窓に付与する追従 Component（物理 px。offset は
//!   `ScopePlacement.balloon_offset` の転写）
//! - [`on_char_drag`]: `OnDrag` ハンドラ。`BalloonFollow.balloon` の `WindowHandle` を直接引く
//! - [`on_balloon_drag`]: バルーン窓の `OnDrag` ハンドラ。バルーン単独ドラッグの相対位置を記憶し、
//!   `BalloonFollow.offset` を `balloon_pos − char_pos` へ更新する（キャラ窓は不動）
//! - [`move_window_to`]: UI スレッド関数・物理 px スクリーン座標直渡し
//! - [`project_anchor`]: bottom 吸着ドラッグの核。「生ドラッグ座標→実窓位置」の純粋写像
//!   （`Bottom` の腕は下端吸着）。吸着した窓と組に入った窓は `DragConfig{move_window:false}` で
//!   wndproc 移動を止め、[`on_char_drag`]／[`on_char_drag_end`] がドラッグの始めに控えた向きで
//!   [`project_anchor`] 適用済み座標を**単一ライター**として書く（組に入った Free は恒等。
//!   ドラッグ後に再釘付けする方式は wndproc と競合して振動した）
//! - [`MonitorSnapshot`]／[`work_area_for_window`]: 全モニタ work area 集合の
//!   Resource と窓中心→モニタ解決の純粋ヘルパ（ポリシーの入力）
//!
//! # 座標単位契約
//!
//! 本モジュールの座標はすべて**物理 px**。`WindowPos.position` は wndproc が
//! 実ウィンドウ位置から更新する物理 px であり、ここに DPI 再スケール
//! （`dpi/96` 乗除）を一切挟まない（二重スケールになる）。
//!
//! # UI スレッド契約
//!
//! 署名は `&mut World` のみで完結し channel／actor 型を持たない。`&mut World` は
//! wintf の UI スレッド tick 内でのみ到達可能なため、窓操作の UI スレッド専有を型で担保する。
//! UI 配送ブリッジ（`spawn_ui`／`UiSender`）との結線は後続の領分。

mod anchor;
mod balloon_drag;
mod drag_follow;
mod keyword_base;
// 面の `move` のずれを窓の位置へ当てる口。
mod move_shift;
// バルーン位置オフセットの単位空間契約の定義元。
mod offset_space;
// 並べ直しの後の判定し直し。
mod other_chars;
// 窓の組の連れ動き。
mod sticky_follow;
mod visibility;
mod window_move;
mod work_area;
// キャラ窓を作業領域へ寄せる判定と記録。
mod work_area_clamp;

// limit 補正の式とタグ語彙。runtime 関門（`window_move::enqueue_window_set_pos`）と
// 解放時補正（`balloon_drag::on_balloon_drag_end`）が消費する。
use super::balloon_limit::{
    BALLOON_LIMIT_CLAMP_TAG, BALLOON_LIMIT_PUSHED_HOW, BALLOON_LIMIT_RELEASE_CONTEXT,
    BALLOON_LIMIT_RUNTIME_CONTEXT, BALLOON_LIMIT_UNRESOLVED_TAG, limit_correction,
    limit_work_area_in,
};
use super::diag::{self, DESPAWNED_SKIP_TAG, PlacementRoute, WindowKind, WindowMoveRecord};
use super::persist::{
    balloon_offset_entries, char_pos_entries, char_pos_to_origin_x, persist_entries,
};
use super::resolver::{Anchor, PointPx, RectPx, SizePx, keyword_balloon_pos};
use super::spawn::{BalloonKeywordBase, BalloonWindowMarker, CharWindowMarker, GhostWindows};
// runtime 関門（`window_move::enqueue_window_set_pos`）と解放時補正
// （`balloon_drag::on_balloon_drag_end`）が読む limit 値の再束縛。
// `spawn::BalloonLimit` は既に公開ゆえここは私有再束縛に留め、公開面は増やさない
// （`BalloonWindowMarker`／`CharWindowMarker` と同じ扱い）。
use super::spawn::BalloonLimit;

pub use self::anchor::{Anchored, project_anchor};
// 外へはテストのビルド（`test-support`）でだけ開く。crate の中は使い手がある（道と名前は同じ）。
#[cfg(feature = "test-support")]
pub use self::anchor::AnchorSource;
#[cfg(not(feature = "test-support"))]
pub(crate) use self::anchor::AnchorSource;
// 外へはテストのビルド（`test-support`）でだけ開く。crate の中は使い手がある（道と名前は同じ）。
#[cfg(feature = "test-support")]
pub use self::balloon_drag::on_balloon_drag;
#[cfg(not(feature = "test-support"))]
pub(crate) use self::balloon_drag::on_balloon_drag;
pub(crate) use self::balloon_drag::on_balloon_drag_end;
pub(crate) use self::drag_follow::on_char_drag;
pub use self::drag_follow::on_char_drag_end;
// 追従 Component の定義元は単位空間契約のモジュール（`offset_space`）。外部からの参照は
// すべてこのファサードを経由する。
pub use self::offset_space::BalloonFollow;
/// 遷移時の唯一の変換規則と、その判定結果。
///
/// 呼び手は追随の適用相（`areka_runtime::…` の `balloon_offset_follow`）。
pub use self::offset_space::{OffsetRescale, rescale_follow_offset};
/// 作者空間のオフセットを合流欄の空間（物理 px）へ換算する純関数と、その 1 軸ぶんの結果。
///
/// 供給層（`placement::apply_author_balloon_offset_scale`）だけが呼ぶため crate 内に留める。
pub(crate) use self::offset_space::{ScaledAxis, scale_author_offset};
pub use self::work_area::{MonitorSnapshot, work_area_for_window};
// 外へはテストのビルド（`test-support`）でだけ開く。crate の中は使い手がある（道と名前は同じ）。
#[cfg(feature = "test-support")]
pub use self::work_area::{WorkAreaResolution, work_area_for_window_with_origin};
#[cfg(not(feature = "test-support"))]
pub(crate) use self::work_area::{WorkAreaResolution, work_area_for_window_with_origin};
/// 追従オフセットの基準対（型の定義は `resolver`）。
///
/// 配置解決の出力（`resolver::ScopePlacement`）と復元 merge（`persist`）が運ぶため、
/// ファサード再輸出に載せる。
pub use super::resolver::OffsetBase;
// 2 源の型と比較。呼び手は `areka_runtime::…` の作業領域の同期。
pub use self::work_area::{MonitorDpiTable, MonitorSources, same_monitors};
// モニタ別拡大率表の要素型。表の外から要素を組めないと `entries` が事実上読めない型になるので公開する。
pub use self::work_area::MonitorDpiEntry;
// 作業領域へ寄せる鎖の World の口と記録の語。呼び手は `areka_runtime::…` の
// 起動の並べ直しと詰め直し。記録の語の残り 2 つと純関数の核は、檻が定義元の道で引く。
pub use self::work_area_clamp::{
    ChainClampOutcome, ChainPlacement, ClampScene, place_chain_in_work_area,
};
// 記録の語。crate の中の使い手は定義元（`work_area_clamp`）が自分で引くので、ここは crate のテストと `test-support` のビルドでだけ開く。
#[cfg(any(test, feature = "test-support"))]
pub use self::work_area_clamp::WORK_AREA_CLAMP_TAG;
// 面の拡大の寄せ。`window_move` の `resize_window_to` が `super::` で辿る私有の再束縛。
use self::work_area_clamp::clamp_grown_free_default;
// 記憶から戻した組の寄せ。呼び手は記憶の戻し（`persist::restore_and_judge`）。
pub(crate) use self::work_area_clamp::{RestoredWindow, keep_restored_groups};
// 戻りの型を名前で呼ぶのは檻だけなので、テストの外では未使用に見える。
#[allow(unused_imports)]
pub(crate) use self::work_area_clamp::GroupShift;

// 窓の移動・寸の口。呼び手は `areka_runtime::…`（`anchor_changed_system` は schedule に登録しない見本）。
pub use self::window_move::{
    anchor_changed_system, move_window_to, resize_window_keep_position, resize_window_to,
};
// 外へはテストのビルド（`test-support`）でだけ開く。crate の中は使い手がある（道と名前は同じ）。
#[cfg(feature = "test-support")]
pub use self::window_move::move_window_with_route;
#[cfg(not(feature = "test-support"))]
pub(crate) use self::window_move::move_window_with_route;

// 私有・crate 内の再束縛と、`areka_runtime::…` などが呼ぶ公開の口（呼び手は下の各行の注）。
// サブモジュール間の相互参照とテストモジュールからの `super::` 参照は、いずれもここを経由する。
// 追随相（`areka_runtime::…`）が収束の保証で `follow_balloon` を呼ぶため、次の 2 項目は公開する。
pub use self::drag_follow::{BalloonFollowTrigger, follow_balloon};
// キーワード由来のバルーン基本位置の一度きりの再導出（`resize_window_to` が使う）。
use self::keyword_base::rederive_keyword_balloon_offset;
// 面の `move` のずれ。当てる口の呼び手は毎フレームのずれの相
// （`areka_runtime::…` の `move_shift`）。ドラッグの終わりと保存の側は当てた分を 0 にする・引く口、
// 位置の初期化は記録を外す口、絶対の位置への `\![move]` は軸ごとに当て直す口だけを使う。
pub use self::move_shift::{
    apply_balloon_shift, apply_char_shift, rebase_shift_axes, shift_is_current,
};
use self::move_shift::{balloon_shift, reset_applied};
pub(crate) use self::move_shift::{origin_position, rebase_shift};
// 組の指令・descript の組の適用の後に `DragConfig.move_window` を付け直す。
// 外へはテストのビルド（`test-support`）でだけ開く。crate の中は使い手がある（道と名前は同じ）。
#[cfg(feature = "test-support")]
pub use self::sticky_follow::sync_drag_writer;
#[cfg(not(feature = "test-support"))]
pub(crate) use self::sticky_follow::sync_drag_writer;
pub(crate) use self::sticky_follow::{is_char_drag_partner, is_char_dragging};
// ドラッグの始めの控え。呼び手は areka の `areka_input::drag`。
pub use self::sticky_follow::{CharDragSession, begin_char_drag};
// `\![move]` の連れ動き。呼び手は `areka_runtime::move_cue::place`。
pub use self::sticky_follow::{follow_sticky_move, log_sticky_ignored};
use self::visibility::{
    VISIBILITY_UNRESOLVED_TAG, apply_visibility_guard, evaluate_visibility_guard,
    route_applies_visibility_guard,
};
// 位置と大きさから窓の矩形を作る。crate 内の根の子（`balloon_limit`・`persist`・`reseed`）も使う。
pub(crate) use self::visibility::rect_at;
use self::window_move::enqueue_window_set_pos;
// 起動の並べ直しの後の判定し直し。登録の口の呼び手は `ghost_session::register_systems`、
// 判定し直しの関数の呼び手は `reset_pos`・`chain_realign`（書き込みの後）。
pub use self::other_chars::register_balloon_rejudge;
pub(crate) use self::other_chars::rejudge_all_balloons;
use self::window_move::window_rect_of;
// 遷移観測（`kind=ground` の発行口・書込指令の要求語彙タグ）の再束縛。
// `window_move` は `super::transition_diag` としてここを辿る（兄弟モジュールへの
// 直接参照はファサード分割では届かない）。
use super::transition_diag;
// 整合待ちの札の再束縛。単一の窓書込口（`window_move` の
// `enqueue_window_set_pos`）が待ち札の適用範囲の不変条件を見張るために読む。
//
// 見張る相手は**見送りが覆うべき経路**に限る——利用者のドラッグ（route 無し）や
// `\![move]`・ドラッグ解放時の補正といった**明示操作**は見送らないので、鳴らせば
// 偽の警報になる（debug ビルドではドラッグ 1 回で落ちていた）。どの経路語で鳴るかの分類は
// `window_move::deferral_covers_route` が 13 語＋route 無しの網羅 match で 1 本に持つ。
use super::dpi_sync::DpiSyncHold;

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
#[path = "follow_anchor_tests.rs"]
mod anchor_tests;
#[cfg(test)]
#[path = "follow_balloon_drag_tests.rs"]
mod balloon_drag_tests;
#[cfg(test)]
#[path = "follow_balloon_limit_tests.rs"]
mod balloon_limit_wiring_tests;
#[cfg(test)]
#[path = "follow_default_pos_track_tests.rs"]
mod default_pos_track_tests;
#[cfg(test)]
#[path = "follow_drag_end_gate_tests.rs"]
mod drag_end_gate_tests;
#[cfg(test)]
#[path = "follow_drag_end_limit_tests.rs"]
mod drag_end_limit_tests;
#[cfg(test)]
#[path = "follow_drag_end_persist_tests.rs"]
mod drag_end_persist_tests;
#[cfg(test)]
#[path = "follow_drag_tests.rs"]
mod drag_tests;
#[cfg(test)]
#[path = "follow_keyword_base_tests.rs"]
mod keyword_base_tests;
#[cfg(test)]
#[path = "follow_move_shift_tests.rs"]
mod move_shift_tests;
#[cfg(test)]
#[path = "follow_offset_component_tests.rs"]
mod offset_component_tests;
#[cfg(test)]
#[path = "follow_offset_residual_tests.rs"]
mod offset_residual_tests;
#[cfg(test)]
#[path = "follow_offset_space_tests.rs"]
mod offset_space_tests;
#[cfg(test)]
#[path = "follow_other_chars_tests.rs"]
mod other_chars_tests;
#[cfg(test)]
#[path = "follow_resize_tests.rs"]
mod resize_tests;
#[cfg(test)]
#[path = "follow_sticky_follow_drag_tests.rs"]
mod sticky_follow_drag_tests;
#[cfg(test)]
#[path = "follow_sticky_follow_tests.rs"]
mod sticky_follow_tests;
#[cfg(test)]
#[path = "follow_test_support.rs"]
pub(crate) mod test_support;
#[cfg(test)]
#[path = "follow_transition_diag_tests.rs"]
mod transition_diag_tests;
#[cfg(test)]
#[path = "follow_visibility_balloon_wiring_tests.rs"]
mod visibility_balloon_wiring_tests;
#[cfg(test)]
#[path = "follow_visibility_balloon_words_tests.rs"]
mod visibility_balloon_words_tests;
#[cfg(test)]
#[path = "follow_visibility_char_wiring_tests.rs"]
mod visibility_char_wiring_tests;
#[cfg(test)]
#[path = "follow_visibility_guard_tests.rs"]
mod visibility_guard_tests;
#[cfg(test)]
#[path = "follow_window_move_diag_tests.rs"]
mod window_move_diag_tests;
#[cfg(test)]
#[path = "follow_window_move_hold_watch_tests.rs"]
mod window_move_hold_watch_tests;
#[cfg(test)]
#[path = "follow_window_move_tests.rs"]
mod window_move_tests;
#[cfg(test)]
#[path = "follow_work_area_clamp_chain_tests.rs"]
mod work_area_clamp_chain_tests;
#[cfg(test)]
#[path = "follow_work_area_clamp_grow_tests.rs"]
mod work_area_clamp_grow_tests;
#[cfg(test)]
#[path = "follow_work_area_clamp_restore_tests.rs"]
mod work_area_clamp_restore_tests;
#[cfg(test)]
#[path = "follow_work_area_tests.rs"]
mod work_area_tests;
