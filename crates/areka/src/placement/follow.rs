//! バルーン追従コンポーネントと窓移動の公開 API。
//!
//! - [`BalloonFollow`]: キャラ窓に付与する追従 Component（配置時確定の暫定 offset・
//!   物理 px・4.4。offset は `ScopePlacement.balloon_offset` の転写）
//! - [`on_char_drag`]: `OnDrag` ハンドラ（mock-shell donor `on_shell_drag` の一般化。
//!   マーカー全走査ではなく `BalloonFollow.balloon` の `WindowHandle` を直接引く）
//! - [`on_balloon_drag`]: バルーン窓の `OnDrag` ハンドラ（4.8・DD16・task 8.3）——
//!   バルーン単独ドラッグの相対位置記憶。`BalloonFollow.offset` を
//!   `balloon_pos − char_pos` へ更新する（キャラ窓は不動）
//! - [`move_window_to`]: R7 公開 API（UI スレッド関数・物理 px スクリーン座標直渡し）
//! - [`DragPositionPolicy`]／[`BottomSnapPolicy`]: bottom 吸着ドラッグ（4.7・
//!   DD15 v2・task 8.2R）の核——「生ドラッグ座標→実窓位置」の純粋写像トレイトと
//!   その bottom 吸着実装（[`project_anchor`] の `Bottom` 腕が委譲）。非 Free
//!   アンカーのキャラ窓は `DragConfig{move_window:false}` で wndproc 移動を止め、
//!   [`on_char_drag`]／[`on_char_drag_end`] が [`Anchored`] を読んで [`project_anchor`]
//!   適用済み座標を**単一ライター**として書く（v1 の事後再釘付けは wndproc と
//!   競合し振動→撤去）
//! - [`MonitorSnapshot`]／[`work_area_for_window`]: 全モニタ work area 集合の
//!   Resource と窓中心→モニタ解決の純粋ヘルパ（task 8.1・ポリシーの入力）
//!
//! # 座標単位契約（design U1/U4）
//!
//! 本モジュールの座標はすべて**物理 px**。`WindowPos.position` は wndproc が
//! 実ウィンドウ位置から更新する物理 px であり、ここに DPI 再スケール
//! （`dpi/96` 乗除）を一切挟まない（2026-07-05 の二重スケール欠陥の檻）。
//!
//! # UI スレッド契約（7.1/7.2/7.3）
//!
//! 署名は `&mut World` のみで完結し channel／actor 型を持たない。`&mut World` は
//! wintf の UI スレッド tick 内でのみ到達可能なため、窓操作の UI スレッド専有
//! （7.2）を型で担保する。UI 配送ブリッジ（`spawn_ui`／`UiSender`）との結線は
//! 後続の領分（7.3）。

mod anchor;
mod drag_follow;
mod keyword_base;
// バルーン位置オフセットの単位空間契約の定義元（areka-P0-balloon-offset-dpi・design C1）。
mod offset_space;
mod visibility;
mod window_move;
mod work_area;

// limit 補正の式とタグ語彙（windowposition-limit C5/C10）。runtime 関門
// （`window_move::enqueue_window_set_pos`・task 3.3）と解放時補正
// （`drag_follow::on_balloon_drag_end`・task 3.4）が消費する。
use super::balloon_limit::{
    BALLOON_LIMIT_CLAMP_TAG, BALLOON_LIMIT_RELEASE_CONTEXT, BALLOON_LIMIT_RUNTIME_CONTEXT,
    BALLOON_LIMIT_UNRESOLVED_TAG, limit_correction,
};
use super::diag::{self, DESPAWNED_SKIP_TAG, PlacementRoute, WindowKind, WindowMoveRecord};
use super::persist::{
    balloon_offset_entries, char_pos_entries, char_pos_to_origin_x, persist_entries,
};
use super::resolver::{Anchor, PointPx, RectPx, SizePx, keyword_balloon_pos};
use super::spawn::{BalloonKeywordBase, BalloonWindowMarker, CharWindowMarker, GhostWindows};
// runtime 関門（`window_move::enqueue_window_set_pos`・task 3.3）と解放時補正
// （`drag_follow::on_balloon_drag_end`・task 3.4）が読む limit 値の
// ファサード再束縛（windowposition-limit C9「follow facade から再輸出」）。
// `spawn::BalloonLimit` は既に公開ゆえここは私有再束縛に留め、公開面は増やさない
// （`BalloonWindowMarker`／`CharWindowMarker` と同じ扱い）。
use super::spawn::BalloonLimit;

pub use self::anchor::{Anchored, project_anchor};
pub(crate) use self::drag_follow::{
    on_balloon_drag, on_balloon_drag_end, on_char_drag, on_char_drag_end,
};
// 追従 Component の定義元は単位空間契約のモジュール（`offset_space`）へ移した
// （areka-P0-balloon-offset-dpi・design D14／task 3.1）。外部からの参照はすべて
// このファサードを経由するため、移設の波及はこの 1 行に閉じる。
pub use self::offset_space::BalloonFollow;
/// 単位空間契約の定義元が持つ基準対（areka-P0-balloon-offset-dpi・要件 1.1／3.1）。
///
/// 配置解決の出力（`resolver::ScopePlacement`）と復元 merge（`persist`）が運ぶため、
/// 移設の波及をここで吸収するファサード再輸出に載せる（design「Modified Files」）。
pub use self::offset_space::OffsetBase;
/// 遷移時の唯一の変換規則と、その判定結果
/// （areka-P0-balloon-offset-dpi・要件 3.1／3.3／3.6・task 6.1）。
///
/// 追随の適用相（`emo2_boot::frame::balloon_offset_follow`）だけが呼ぶため crate 内に留める。
/// examples は `#[path]` で src を取り込むが本相を持たないため、そちらのビルドでは未使用に
/// 見える——下の `MonitorDpiTable` と同じ事情なので同じ扱いにする。
#[allow(unused_imports)]
pub(crate) use self::offset_space::{OffsetRescale, rescale_follow_offset};
/// 作者空間のオフセットを合流欄の空間（物理 px）へ換算する純関数と、その 1 軸ぶんの結果
/// （areka-P0-balloon-offset-dpi・要件 1.2／2.1／2.5・task 4.1）。
///
/// 供給層（`placement::apply_author_balloon_offset_scale`）だけが呼ぶため crate 内に留める。
pub(crate) use self::offset_space::{ScaledAxis, scale_author_offset};
pub use self::work_area::{
    MonitorSnapshot, WorkAreaResolution, work_area_for_window, work_area_for_window_with_origin,
};
// 2 源の型と比較（task 5.1 で新設）。本番の消費者は `emo2_boot::frame::work_area_sync` が
// 定義元から直接 import するため、この再輸出を**名前で呼ぶ**のはテストと起動シームだけである。
// examples は `#[path]` で src を取り込むが本番項目しか使わないので、そちらのビルドでは
// 未使用に見える——下の `MonitorDpiEntry` と同じ事情なので同じ扱いにする。公開面の維持
// （要件 2.5）のため再輸出そのものは残す。
#[allow(unused_imports)]
pub use self::work_area::{MonitorDpiTable, MonitorSources, same_monitors};
// モニタ別拡大率表の要素型。`MonitorDpiTable::entries` の要素として本番でも運ばれるが、
// **名前で呼ぶ**のはテストだけ（本番の消費者は表ごと受け渡す）ゆえ非 test ビルドでは未使用に
// 見える。表の外から要素を組めないと `entries` が事実上読めない型になるので再輸出は残す。
#[allow(unused_imports)]
pub use self::work_area::MonitorDpiEntry;

// 公開面の維持（要件 2.5）だけを担う再輸出。本体からは定義元のサブモジュール内で直接
// 呼ばれるため、`follow::` 経由で参照するのは `#[cfg(test)]` のテストモジュールと
// examples だけであり、areka は bin クレートゆえ非 test ビルドでは未使用に見える
// （移設前は同一モジュール内の項目定義だったのでこの見え方が生じなかった）。
#[allow(unused_imports)]
pub use self::anchor::{BottomSnapPolicy, DragPositionPolicy};
#[allow(unused_imports)]
pub use self::visibility::{VisibilityVerdict, guard_visibility};
// 同上。ただしこのグループで実際に未使用と判定されるのは `anchor_changed_system` のみ
// （schedule 登録＝結線が main.rs／runtime 側の領分のため）。残る 3 項目は非 test ビルドでも
// 消費されている（`move_cue.rs`／`emo2_boot/frame.rs`）が、属性は use 文単位で効くため
// このグループ全体に付く。
#[allow(unused_imports)]
pub use self::window_move::{
    anchor_changed_system, move_window_to, move_window_with_route, resize_window_keep_position,
    resize_window_to,
};

// 私有項目のファサード再束縛（クレート内可視性のみ・公開面は増やさない）。
// サブモジュール間の相互参照とテストモジュールからの `super::` 参照は、いずれも
// ここを経由する。
// 追随相（`emo2_boot::frame`）が収束の保証（design D16）で `follow_balloon` を呼ぶため、
// この 2 項目だけは私有再束縛から crate 内公開へ格上げする。areka は bin crate であり
// `pub(crate)` はクレート内に留まるので、外部 API 面は増えない。
pub(crate) use self::drag_follow::{BalloonFollowTrigger, follow_balloon};
use self::keyword_base::rederive_keyword_balloon_offset;
use self::visibility::{
    VISIBILITY_UNRESOLVED_TAG, apply_visibility_guard, evaluate_visibility_guard, rect_at,
    route_applies_visibility_guard,
};
use self::window_move::enqueue_window_set_pos;
// 遷移観測（`kind=ground` の発行口・書込指令の要求語彙タグ）のファサード再束縛。
// `window_move` は `super::transition_diag` としてここを辿る（兄弟モジュールへの
// 直接参照はファサード分割では届かない・structure.md「ファサード形式」の注意点）。
use super::transition_diag;
// 整合待ちの札（atom 設計 C5）のファサード再束縛。単一の窓書込口（`window_move` の
// `enqueue_window_set_pos`）が待ち札の適用範囲の不変条件を見張るために読む。
//
// 見張る相手は**見送りが覆うべき経路**に限る（task 7.5）——利用者のドラッグ（route 無し）や
// `\![move]`・ドラッグ解放時の補正といった**明示操作**は設計上そもそも見送らないので、鳴らせば
// 偽の警報になる（debug ビルドではドラッグ 1 回で落ちていた）。どの経路語で鳴るかの分類は
// `window_move::deferral_covers_route` が 12 語＋route 無しの網羅 match で 1 本に持つ。
use super::dpi_sync::DpiSyncHold;

// =============================================================================
// Tests（TDD RED: 実装前に振る舞いを固定する）
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
#[path = "follow_offset_component_tests.rs"]
mod offset_component_tests;
#[cfg(test)]
#[path = "follow_offset_residual_tests.rs"]
mod offset_residual_tests;
#[cfg(test)]
#[path = "follow_offset_space_tests.rs"]
mod offset_space_tests;
#[cfg(test)]
#[path = "follow_resize_tests.rs"]
mod resize_tests;
#[cfg(test)]
#[path = "follow_test_support.rs"]
mod test_support;
#[cfg(test)]
#[path = "follow_transition_diag_tests.rs"]
mod transition_diag_tests;
#[cfg(test)]
#[path = "follow_visibility_balloon_wiring_tests.rs"]
mod visibility_balloon_wiring_tests;
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
#[path = "follow_window_move_tests.rs"]
mod window_move_tests;
#[cfg(test)]
#[path = "follow_work_area_tests.rs"]
mod work_area_tests;
// 既定位置の追跡規則（task 5.5・design D9／D16・要件 6.2）。
#[cfg(test)]
#[path = "follow_default_pos_track_tests.rs"]
mod default_pos_track_tests;
// 整合待ちの札の監視が鳴る対象の分類（task 7.5・design C5・要件 5.8）。
#[cfg(test)]
#[path = "follow_window_move_hold_watch_tests.rs"]
mod window_move_hold_watch_tests;
