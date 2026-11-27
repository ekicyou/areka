//! 毎フレーム結線の排他 system `frame_system` と、その状態 `RuntimeWiring`。
//!
//! `frame_system` は `RuntimeWiring`（NonSend）を取り出し、下の 21 の相をこの順に 1 フレームで
//! 駆動して戻す。相の順の所有者は本ファイルで、順そのものが振る舞いの一部である（理由は
//! `frame_system` の中の各相の前に書く）。
//!
//! 1. 文字の層を進める（`scale_text::run_text_advance_phase`・フレームの時刻までに時刻の来た物を届いた順に当てる）
//! 2. 作業領域の同期（`work_area_sync`）
//! 3. 装着（`attach`・GPU 資源と `GhostWindows` が揃うまで何もしない。バルーンは不可視のまま確立し、
//!    シェルは最初の `\s` の台本まで非表示を保つ）
//! 4. 装置の作り直し（presenter の `refresh_after_device_change`）
//! 5. 拡大率の相（`dpi`・`Changed<DPI>` の窓の target を再スケールして窓寸を反映）
//! 6. 作業領域の変化の再スナップ（`work_area_sync`・変化があったフレームだけ）
//! 7. 取り出し（`drain_resnap`・表示の指令を FIFO で適用するだけ）
//! 8. 差し替え（`switch`・シェルとバルーンの置き換えの後始末）
//! 9. 可視性（`balloon_visibility`・指令適用後の実状態からバルーンの可視を決めて発行）
//! 10. 窓寸の突き合わせ（`scale_text::reconcile_reported_sizes`・表示成立点の報告を窓へ反映）
//! 11. 重なり（`zorder_drain`・台本の指令を台帳へ適用し、望む鎖 1 本を wintf の受け口へ置く）
//! 12. 常に手前の状態（`window_state`）
//! 13. 再スナップ（`drain_resnap`・表示寸が変わったキャラ窓のアンカー再適用）
//! 14. 移動の取り出し（`drain_resnap`・`\![move]` ほか位置の指令）
//! 15. 面の `move` のずれ（`move_shift`）
//! 16. 連鎖の確定（`drain_resnap`・初期配置を実表示寸で 1 度だけ）
//! 17. 連鎖の解き直し（`drain_resnap`・拡大率の遷移ごとに 1 度だけ）
//! 18. 文字の拡大率（`scale_text`・文字層の binding を presenter の現適用 k へ合わせる）
//! 19. 文字（`scale_text`・フレームの時刻が在るとき `present_frame`）
//! 20. 状態の報告（`status_report`・見えているバルーンの組を変わったときだけ kanade へ）
//! 21. 拡大率の知らせ（`scale_notice`）
//!
//! `plan_attachments`（`GhostWindows::scopes()` を正とする純関数）は装着の計画で、`attach` にある。

mod attach;
mod balloon_face;
mod balloon_offset_follow;
mod dpi;
// 下の `test-support` の組は、外へはテストのビルドでだけ開く。crate の中は今の道のまま。
#[cfg(feature = "test-support")]
pub mod drain_resnap;
#[cfg(not(feature = "test-support"))]
mod drain_resnap;
mod move_hold;
mod move_shift;
#[cfg(feature = "test-support")]
pub mod scale_notice;
#[cfg(not(feature = "test-support"))]
mod scale_notice;
mod scale_text;
pub mod status_report;
#[cfg(feature = "test-support")]
pub mod switch;
#[cfg(not(feature = "test-support"))]
mod switch;
#[cfg(feature = "test-support")]
pub mod window_state;
#[cfg(not(feature = "test-support"))]
mod window_state;
#[cfg(feature = "test-support")]
pub mod wiring;
#[cfg(not(feature = "test-support"))]
mod wiring;
use self::window_state::run_window_state_phase;
pub use self::window_state::{DiscardReason, log_stay_on_top_discard};
#[cfg(feature = "test-support")]
pub mod work_area_sync;
#[cfg(not(feature = "test-support"))]
mod work_area_sync;
mod zorder_descript;
mod zorder_drain;

// 子のテストモジュールの `use super::*;` が拾う束縛。
#[cfg(any(test, feature = "test-support"))]
use std::cell::RefCell;
#[cfg(any(test, feature = "test-support"))]
use std::rc::Rc;

#[cfg(test)]
use bevy_ecs::entity::Entity;
#[cfg(test)]
use bevy_ecs::system::SystemState;
use bevy_ecs::world::World;
use tracing::debug;

#[cfg(any(test, feature = "test-support"))]
use areka_emo_present::{EmoPresenter, PresentCommand, TargetId};
#[cfg(any(test, feature = "test-support"))]
use areka_emo_text::actor::{TextLayerRuntime, present_frame};
#[cfg(any(test, feature = "test-support"))]
use areka_sakura::ActorKey;
#[cfg(any(test, feature = "test-support"))]
use wintf::ecs::{GraphicsCore, WucGraphicsResource};

#[cfg(test)]
use areka_placement::diag::PlacementRoute;
#[cfg(test)]
use areka_placement::spawn::CharWindowMarker;
#[cfg(any(test, feature = "test-support"))]
use areka_placement::spawn::GhostWindows;

use super::assets::{BalloonScopeAssets, BootAssets, ScopeAssets};
// 可視性の相。相順の所有者は本モジュールなので、呼び出しはここから行う。
use super::balloon_visibility::run_balloon_visibility_phase;
// `scale_text::run_text_phase` が `super::hover_inject::drive` で辿る先。呼び出し本文を書き換えないための束縛。
use super::hover_inject;
use super::move_cue::{PositionCue, apply_move_directive};
use super::talk_clock::TalkClock;
use super::target_map::{balloon_target, shell_target};

pub use self::attach::{AttachPlan, PlannedAttach, plan_attachments, run_attach_phase};
pub use self::dpi::run_dpi_phase;
pub use self::drain_resnap::{run_drain_phase, run_move_drain_phase};
// 表示の拡大率を読み口で受ける move drain（GPU 無しの檻が窓を「表示した」前提で流すためのシーム）。
#[cfg(any(test, feature = "test-support"))]
pub use self::drain_resnap::run_move_drain_with;
pub use self::scale_text::{run_text_advance_phase, run_text_phase, run_text_scale_phase};
// 窓がどれも表示した前提の読み口（spine の檻も同じものを使う）。
#[cfg(any(test, feature = "test-support"))]
pub use self::test_support::all_shown;
pub use self::wiring::RuntimeWiring;
pub use self::zorder_drain::run_zorder_drain_phase;
#[cfg(test)]
pub(crate) use self::zorder_drain::test_support as zorder_test_support;

// 私有項目のファサード再束縛（クレート内可視性のみ・公開面は増やさない）。サブモジュール間の
// 相互参照とテストモジュールからの `use super::*;` は、いずれもここを経由する。
#[cfg(test)]
use self::attach::connect_balloon_text;
#[cfg(any(test, feature = "test-support"))]
use self::dpi::dpi_phase_with;
use self::dpi::{
    AuthorDpis, DpiChangedQuery, GhostWindowKind, ScaleReportSource, reconcile_window_size,
};
#[cfg(test)]
use self::dpi::{GhostWindowClass, classify_ghost_window, reproject_char_window_at_current_size};
#[cfg(test)]
use self::drain_resnap::resnap_from_sizes;
#[cfg(any(test, feature = "test-support"))]
use self::drain_resnap::{
    PhysicalSizeSource, finalize_chain_once_with, realign_chain_once_with_source, resnap_with,
};
use self::drain_resnap::{finalize_chain_once, realign_chain_once, resnap_shell_targets};
pub(crate) use self::scale_text::resolve_frame_now;
use self::scale_text::{TextLayerBusyLogged, reconcile_reported_sizes};
// 窓を全部閉じるとき（`app_exit::close_windows_for_restart`）に外す、窓の一式に属する控え。
pub use self::zorder_drain::ZOrderAbsentReports;

/// `Update` 登録の排他 system（donor パターン: remove→各フェーズ→insert）。
///
/// `RuntimeWiring`（NonSend）を [`World::remove_non_send`] で取り出し、モジュールの頭に並べた相を
/// 順に駆動して [`World::insert_non_send`] で戻す。remove→insert は `&mut World` を各相へ排他に
/// 渡すための借用衝突の回避である。本番の進める相と文字の相は override 無し（フレームの時刻は `FrameTime`）。
///
/// アプリの終了が指示済み（World の `wintf::AppExit` が指示済みを返す）の巡では、`RuntimeWiring` を
/// 取り出すより前に判定し、作業領域の同期も含めて相を 1 つも回さずに戻る。記録は
/// `debug!(event = "frame_phases_skipped_after_exit")` の 1 行。`wintf::AppExit` が World に無い構成は
/// 未指示とみなす。`RuntimeWiring` 未挿入（フォールバック boot 経路）なら何もしない。
///
/// schedule への登録（`add_systems(Update, frame_system.after(update_typewriters))`）は
/// `register_frame_system` が行う。載せ先が `Update`＝上流の `Update` 鎖の最後の系より後なのは、
/// 本 system が挿した描画命令と配置を同じ巡の伝播（`PostLayout`）・面の生成（`PreRenderSurface`）・
/// 描画（`RenderSurface`）に拾わせるためである（末尾の段に載せると絵の着地が次の巡へずれる）。
pub fn frame_system(world: &mut World) {
    // 終了が指示済みの巡は結線を取り出さず、作業領域の同期も含めて何もしない。窓は閉じる途中で、
    // 相を走らせても閉じる窓を描き直すだけである。
    if world
        .get_non_send::<wintf::AppExit>()
        .is_some_and(|e| e.is_requested())
    {
        debug!(
            event = "frame_phases_skipped_after_exit",
            "終了が指示済み——毎フレームの処理を読み飛ばす"
        );
        return;
    }
    let Some(mut wiring) = world.remove_non_send::<RuntimeWiring>() else {
        return;
    };
    // どの相よりも前に置く。合図はフレームの外で届いて列で待つので、文字の層に触る相（装着・取り出し・
    // 差し替え・可視性・文字の拡大率・文字）の全部が、このフレームの時刻まで進めた後の状態を見る。
    run_text_advance_phase(&mut wiring, world, None);
    // 各相より前に置く。相の後に同期すると旧下端へ書いてから源が変わり 2 段書込になる。
    // 戻り値は、変化を契機とする再スナップ（拡大率の相の後）が受ける。
    let work_area_change = work_area_sync::sync_monitor_snapshot(world);
    run_attach_phase(&mut wiring, world);
    // 装着の後（target が在る）・拡大率と取り出しの前に置く。この回の再スケールと表示の指令が、
    // 新しい装置の上で重ねて描くため。
    wiring.presenter.refresh_after_device_change(world);
    // 装着の後に置く。装着前は再スケール対象の target が無く、装着と同じフレームで窓が生えた直後の
    // `Changed<DPI>`（生成時の `GetDpiForWindow` 実値補正）を同じフレームで拾えるようにするため。
    // 取り出しの前は責任分界であって順序依存ではない（未消費要求は後段の状態照合が拾い、二重にも
    // 取りこぼしにもならない）。
    run_dpi_phase(&mut wiring, world);
    // 拡大率の相の直後に置く。作業領域だけが変わった窓は `Changed<DPI>` が立たず相を素通りするので、
    // ここで現寸のまま新しい下端へ射影し直す。相の前に置くと 2 段書込になる。
    // 作業領域が動かなかったフレームは `None` で、呼び出しごと起きない。
    if let Some(work_area_change) = &work_area_change {
        work_area_sync::resnap_for_work_area_change(world, work_area_change);
    }
    // 取り出しの所要は、差し替えの相が置き換えの返信のそろったフレームの `swap_ms` へ足す。
    let drain_started = std::time::Instant::now();
    run_drain_phase(&mut wiring, world);
    wiring.last_drain = drain_started.elapsed();
    // 取り出しの直後に置く。置き換えを適用した同じフレームで後始末し、続く可視性・窓寸・再スナップ・
    // 文字の層の相がその結果を拾う。
    switch::run_switch_phase(&mut wiring, world);
    // 本フレームの表示指令をすべて適用し終えた後に置く。取り出しの前だと 1 フレーム古い可視状態を見る。
    // 窓寸の突き合わせの前でもあるのは、ここで発行した表示が積む窓寸要求を同じフレームのうちに
    // 消化するため。
    run_balloon_visibility_phase(&mut wiring, world);
    // 全 apply が済んだ後に、表示成立点の状態照合が積んだ未消費の窓寸要求を窓 client へ反映する。
    // 装着の初回 `ShowSurface` が積む k₀ 補正もここで着地する。装着前は presenter に target が無く
    // 報告の取り出しが全 target で `None` を返すので、窓書込もログも出ない。
    reconcile_reported_sizes(&mut wiring.presenter, world);
    // 窓寸の突き合わせの直後に置く。鎖の適用系（確定段の末尾）より前である必要がある。後ろだと、
    // 望む鎖を適用系が読むのが 1 心拍ぶん遅れ、組み替えがそのイベントの巡で完了しない。
    // 順序は段の並びが担う（適用系が載る確定段 `FrameFinalize` は 1 巡の並びでこれより後ろ・
    // 段をまたぐ `.before` は書けない）。窓の正本が無い間は相の側が早期に戻り、チャネルが保留バッファを兼ねる。
    run_zorder_drain_phase(&wiring.zorder_rx, &mut wiring.zorder_ledger, world);
    // 重なりの取り出しの直後に置く。印と帯の指令の列を同じ巡の確定段（帯の系）が読む。
    run_window_state_phase(&mut wiring.window_state, world);
    // 取り出しの後に置く。適用後の実寸を読んで、変化した char 窓のアンカーだけを再適用する。
    resnap_shell_targets(&wiring.presenter, world);
    // `\![move]` の末端。talk スレッドから届いた `PositionCue` を取り出し、`Move` は実窓へ即時反映し、
    // `ResetWindowPos` は並べ直しの手続きへ渡す。表示の指令とは独立で、GPU の装着は待たない。
    // 再スナップの後に置く。前に置くと古い大きさで右下などをそろえ、続く再スナップが大きさだけ変えて
    // 位置がずれる。連鎖の確定の前でもある。面の指令と `\![move]` は別の線で届くので、別のフレームに
    // 割れて届いた場合の隙間はここでは扱わない。
    run_move_drain_phase(&wiring, world);
    // 移動の相の直後に置く。表示の指令（ずれは `Show` と同じ線で届く）・再スナップ・`\![move]` が
    // 窓を置いた後に、その位置を元にずれを当てる。連鎖の確定・解き直しより前に置き、それらが
    // ずらした後の位置を読む。
    move_shift::run_move_shift_phase(&wiring.presenter, world);
    // 再アンカーが着地して全スコープの実表示寸が確定したフレームで一度だけ連鎖を解き直し、
    // サーフェス差替で崩れた隣接を戻す。再スナップの後に置く（各窓の再アンカー結果を入力に取るため）。
    finalize_chain_once(&wiring.presenter, world);
    // 起動時確定の直後に置く。拡大率の遷移では全スコープの幅が変わり、下端中央保存の再射影だけでは
    // 隣接が幅変化の半分の和だけ開く。起動時の確定は一度きりでこれを直さないので、遷移ごとに一度だけ
    // 解き直す別機構が要る。再スナップ・確定の後である理由は確定と同じ。武装していないフレームは
    // 早期 return で、定常フレームは無操作である。
    realign_chain_once(&wiring.presenter, world);
    // 適用 k の更新点（拡大率の相の `refresh_scale`・取り出しの相の `apply_show`）の両方の下流、
    // かつ `present_frame` の上流に置く。どちらの経路で k が跳ねても同じフレーム内で binding が
    // 新 k へ組み直され、旧寸の文字が 1 フレーム残らない。戻り値（再構築 scope）は観測用で、
    // 本番は捨てる。
    let _ = run_text_scale_phase(&mut wiring, world);
    run_text_phase(&mut wiring, world, None);
    // 提示の直後に置く。窓の可視（可視性の相が発行済み）と箱の写し（直前の提示で作り直し済み）が
    // どちらもこのフレームの最終の姿になってから組を作るので、届けが画面から 1 フレーム遅れない。
    // 可視性の相の中で届けると、箱の同期と提示より前なので写しが 1 フレーム古い。
    status_report::run_status_report_phase(&mut wiring, world);
    // k を書く点（拡大率・取り出し・差し替え・可視性の相）はどれもこれより前なので、k が変わったのと
    // 同じフレームで表とイベントが出る。
    scale_notice::run_scale_notice_phase(&mut wiring, world);
    world.insert_non_send(wiring);
}

#[cfg(any(test, feature = "test-support"))]
#[path = "frame_test_support.rs"]
pub mod test_support;

#[cfg(any(test, feature = "test-support"))]
#[path = "frame_balloon_runtime_test_support.rs"]
pub mod balloon_runtime_test_support;

// areka-runtime のテストと本体のテストの両方が使う補助。
#[cfg(any(test, feature = "test-support"))]
#[path = "frame_shared_test_support.rs"]
pub mod shared_test_support;

#[cfg(test)]
#[path = "frame_transition_test_support.rs"]
pub mod transition_test_support;

#[cfg(any(test, feature = "test-support"))]
#[path = "frame_persist_test_support.rs"]
pub mod persist_test_support;

// 移動の取り出しの置き場の檻。
#[cfg(all(test, target_pointer_width = "64"))]
#[path = "frame_move_order_tests.rs"]
mod move_order_tests;

// 多フレーム駆動ハーネスそのものの檻。
#[cfg(all(test, target_pointer_width = "64"))]
#[path = "frame_harness_tests.rs"]
mod harness_tests;

// 作業領域源の同期の統合テスト。
#[cfg(all(test, target_pointer_width = "64"))]
#[path = "frame_work_area_sync_tests.rs"]
mod work_area_sync_tests;

// 作業領域変化を契機とする再スナップの統合テスト。
#[cfg(all(test, target_pointer_width = "64"))]
#[path = "frame_work_area_resnap_tests.rs"]
mod work_area_resnap_tests;

// 組の相手を連れ動かさない道（当て直し・大きさの変化・作業領域の変化）の檻。
#[cfg(test)]
#[path = "frame_sticky_no_follow_tests.rs"]
mod sticky_no_follow_tests;

// 拡大率と表の整合待ちの統合テスト。
#[cfg(all(test, target_pointer_width = "64"))]
#[path = "frame_dpi_sync_hold_tests.rs"]
mod dpi_sync_hold_tests;

// 既定位置の追跡の統合テスト。
#[cfg(all(test, target_pointer_width = "64"))]
#[path = "frame_default_pos_track_tests.rs"]
mod default_pos_track_tests;

// 遷移後の連鎖再解決の統合テスト。
#[cfg(all(test, target_pointer_width = "64"))]
#[path = "frame_chain_realign_tests.rs"]
mod chain_realign_tests;

// 遷移後の連鎖再解決の武装条件（2 連言の特異性）の檻。
#[cfg(all(test, target_pointer_width = "64"))]
#[path = "frame_chain_realign_arm_tests.rs"]
mod chain_realign_arm_tests;

// 拡大率の後の詰め直しで動かさない組（外す理由）と変わる直前の控えの檻。
#[cfg(all(test, target_pointer_width = "64"))]
#[path = "frame_chain_realign_exclusion_tests.rs"]
mod chain_realign_exclusion_tests;

// 拡大率の後の詰め直しが変わる直前の隣り合いで組を詰める檻。
#[cfg(all(test, target_pointer_width = "64"))]
#[path = "frame_chain_realign_adjacency_tests.rs"]
mod chain_realign_adjacency_tests;

// 遷移の原子性と随伴の統合テスト。
#[cfg(all(test, target_pointer_width = "64"))]
#[path = "frame_transition_atomicity_tests.rs"]
mod transition_atomicity_tests;

// 整合待ちと作業領域追随の判断分岐の統合テスト。
#[cfg(all(test, target_pointer_width = "64"))]
#[path = "frame_transition_branch_tests.rs"]
mod transition_branch_tests;

#[cfg(all(test, target_pointer_width = "64"))]
#[path = "frame_transition_branch_wait_tests.rs"]
mod transition_branch_wait_tests;

// 整合ゲートの 4 つ目の窓書込口の統合テスト。
#[cfg(all(test, target_pointer_width = "64"))]
#[path = "frame_work_area_resnap_hold_tests.rs"]
mod work_area_resnap_hold_tests;

// 位置の指令の預かりの檻。
#[cfg(all(test, target_pointer_width = "64"))]
#[path = "frame_move_hold_tests.rs"]
mod move_hold_tests;

#[cfg(test)]
#[path = "frame_reset_hold_test_support.rs"]
pub mod reset_hold_test_support;

// 並べ直しの預かりの檻。
#[cfg(all(test, target_pointer_width = "64"))]
#[path = "frame_reset_hold_tests.rs"]
mod reset_hold_tests;

// 窓の組・吸着・バルーン位置初期化の指令が位置の線で当たる檻。
#[cfg(all(test, target_pointer_width = "64"))]
#[path = "frame_window_link_drain_tests.rs"]
mod window_link_drain_tests;

#[cfg(test)]
#[path = "frame_attach_tests.rs"]
mod attach_tests;

#[cfg(test)]
#[path = "frame_drain_text_tests.rs"]
mod drain_text_tests;

// 終了指示の有無による毎フレームの処理の分岐。GPU もハーネスも使わない。
#[cfg(test)]
#[path = "frame_exit_gate_tests.rs"]
mod exit_gate_tests;

#[cfg(all(test, target_pointer_width = "64"))]
#[path = "frame_resnap_tests.rs"]
mod resnap_tests;

#[cfg(test)]
#[path = "frame_chain_finalize_tests.rs"]
mod chain_finalize_tests;

#[cfg(test)]
#[path = "frame_chain_finalize_align_tests.rs"]
mod chain_finalize_align_tests;

#[cfg(test)]
#[path = "frame_chain_rule_tests.rs"]
mod chain_rule_tests;

#[cfg(test)]
#[path = "frame_chain_finalize_remembered_tests.rs"]
mod chain_finalize_remembered_tests;

// 拡大率の違う画面をまたぐドラッグで掴んだ点を窓の大きさの比で置き直す檻。
#[cfg(all(test, target_pointer_width = "64"))]
#[path = "frame_dpi_drag_grab_tests.rs"]
mod dpi_drag_grab_tests;

#[cfg(test)]
#[path = "frame_dpi_tests.rs"]
mod dpi_tests;

#[cfg(test)]
#[path = "frame_dpi_reproject_tests.rs"]
mod dpi_reproject_tests;

#[cfg(test)]
#[path = "frame_dpi_reproject_none_tests.rs"]
mod dpi_reproject_none_tests;

// 収束の保証。
#[cfg(all(test, target_pointer_width = "64"))]
#[path = "frame_balloon_offset_converge_tests.rs"]
mod balloon_offset_converge_tests;

// キーワード再導出との排他の門。
#[cfg(all(test, target_pointer_width = "64"))]
#[path = "frame_balloon_offset_keyword_gate_tests.rs"]
mod balloon_offset_keyword_gate_tests;

// 遷移相の結合テスト行列。
#[cfg(all(test, target_pointer_width = "64"))]
#[path = "frame_balloon_offset_follow_tests.rs"]
mod balloon_offset_follow_tests;

// 拡大率の往復で基準へ bit 同一で戻ることの檻。
#[cfg(all(test, target_pointer_width = "64"))]
#[path = "frame_balloon_offset_roundtrip_tests.rs"]
mod balloon_offset_roundtrip_tests;

#[cfg(test)]
#[path = "frame_diag_route_tests.rs"]
mod diag_route_tests;

#[cfg(test)]
#[path = "frame_text_scale_tests.rs"]
mod text_scale_tests;

#[cfg(test)]
#[path = "frame_text_advance_tests.rs"]
mod text_advance_tests;

#[cfg(test)]
#[path = "frame_receiver_wake_tests.rs"]
mod receiver_wake_tests;
