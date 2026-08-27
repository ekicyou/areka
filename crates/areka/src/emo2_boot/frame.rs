//! 毎フレーム結線（attach／dpi／drain／text）の排他 system と NonSend 配線状態。
//!
//! `Emo2Wiring`（NonSend resource・presenter／rx／runtime／clock／assets／attached を保持）と
//! 排他 system `emo2_frame_system(world: &mut World)`（donor パターン: remove→各フェーズ→insert）を
//! 所有する。各フェーズ:
//! - attach: GPU 資源＋`GhostWindows` 到達ゲート→`plan_attachments`（DD-12）→バルーンの可視性を
//!   外部所有へ→**不可視のままの確立**（`ShowSurface` 面0）→文字層スロット取得→`register_actor_view`
//!   （`Option::take` で高々 1 回消費）。バルーンは装着だけでは画面へ出ず、可視化はバルーン可視性制御が
//!   別途付与する（`areka-P0-balloon-visibility` Requirement 1.1/1.3）。**シェルは初回
//!   `ShowSurface` を発行せず**最初のさくらスクリプト `\s` cue まで非表示を保つ（defect #5・実機#5）。
//! - dpi: `Changed<DPI>` の窓を永続 `SystemState` で観測し（`anchor_changed_system` 先例）、当該窓の
//!   target を `refresh_scale` で再スケールして窓寸を reconcile する（emo-dpi-scaling task 4.2・D8）。
//! - drain: attach 完了後のみ `Receiver::try_iter` で `PresentCommand` を FIFO で `presenter.apply` へ
//!   適用する（**適用のみ**）。
//! - balloon-visibility: 本フレームの表示指令を適用し終えた実状態からバルーンの可視性を決めて発行する
//!   （`areka-P0-balloon-visibility` design 決定 D5・Requirement 3.5／6.6）。観測（可視状態・可視
//!   グリフ数・選択肢表示中・ポインタ滞在・ドラッグ中・表示ライフサイクル信号）を集めて判断中核へ渡し、
//!   返った遷移を発行して 1 行ずつ記録する。実装は `emo2_boot::balloon_visibility`（判断）と
//!   その配線層 `balloon_visibility_phase.rs`。
//! - 窓寸 reconcile: 表示成立点の状態照合報告（`take_pending_resize`）で窓寸を反映する（第 2 経路）。
//!   可視性の相が発行した表示が積む窓寸要求も、同一フレームのここで landing する。
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
//! `resolve_talk_time`）・排他 system `emo2_frame_system`（remove→各フェーズ→insert）を実装する。

mod attach;
// 拡大率遷移でのバルーン追従オフセットの追随（areka-P0-balloon-offset-dpi・design D6）。
mod balloon_offset_follow;
mod dpi;
mod drain_resnap;
mod scale_text;
mod wiring;
mod work_area_sync;

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
use areka_emo_text::actor::{TextLayerRuntime, present_frame};
#[allow(unused_imports)]
use areka_parsers::balloon::BalloonModel;
#[allow(unused_imports)]
use areka_sakura::ActorKey;
#[allow(unused_imports)]
use wintf::ecs::{DPI, FrameTime, GraphicsCore, SizeI, WindowPos, WucGraphicsResource};

#[allow(unused_imports)]
use crate::placement::diag::{DESPAWNED_SKIP_TAG, PlacementRoute};
#[allow(unused_imports)]
use crate::placement::follow::{resize_window_keep_position, resize_window_to};
#[allow(unused_imports)]
use crate::placement::resolver::SizePx;
#[allow(unused_imports)]
use crate::placement::spawn::{BalloonWindowMarker, CharWindowMarker, GhostWindows};

use super::assets::{BalloonScopeAssets, BootAssets, ScopeAssets};
// 可視性の相（design 決定 D5）。相順の所有者は本モジュールなので、呼び出しはここから行う。
use super::balloon_visibility::run_balloon_visibility_phase;
// `scale_text::run_text_phase` が `super::hover_inject::drive` で辿る先（移設前は本ファイルから
// `super::hover_inject` が emo2_boot を指していた）。呼び出し本文を書き換えないための束縛。
use super::hover_inject;
use super::move_cue::{MoveDirective, apply_move_directive};
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
// 同上。未使用は `PhysicalSizeSource`・`resnap_from_sizes`・`resnap_with`・
// `finalize_chain_once_with`・`realign_chain_once_with_source` の 5 名で、
// `resnap_shell_targets`・`finalize_chain_once`・`realign_chain_once` は
// 下の `emo2_frame_system` が呼ぶ。
#[allow(unused_imports)]
use self::drain_resnap::{
    PhysicalSizeSource, finalize_chain_once, finalize_chain_once_with, realign_chain_once,
    realign_chain_once_with_source, resnap_from_sizes, resnap_shell_targets, resnap_with,
};
// 同上。`reconcile_reported_sizes` は下の `emo2_frame_system` が非 test ビルドでも呼ぶ
// （呼び出しは task 4.3 で `run_drain_phase` から相順の所有者であるこちらへ移した・design 決定 D5）。
use self::scale_text::reconcile_reported_sizes;
// バルーン可視性の相（兄弟モジュール `emo2_boot::balloon_visibility`）が注入時刻の解決に用いる
// ための再輸出。可視性の相と text 相が**同一の**時刻契約を共有することが要る——可視性の相は
// `visible_glyphs(actor, t)` でリビール済みグリフ数を観測し、text 相は同じ `t` で描画するため、
// 別式で組むと「観測したグリフ数」と「実際に描かれる文字」が食い違う。
pub(super) use self::scale_text::resolve_talk_time;

/// `FrameFinalize` 登録の排他 system（donor パターン: remove→各フェーズ→insert・DD-1/DD-4）。
///
/// `Emo2Wiring`（NonSend）を [`World::remove_non_send`] で取り出してから
/// attach→dpi→drain→balloon-visibility→窓寸 reconcile→move-drain→resnap→連鎖確定→連鎖再解決→
/// text-scale→text
/// の順に各フェーズを駆動し、[`World::insert_non_send`] で戻す。
/// remove→insert は `&mut World` を各フェーズへ排他に渡すための donor 慣行（借用衝突回避・
/// `examples/emo-present.rs::boot_present_system` と同型）。本番の text フェーズは override 無し
/// （`FrameTime`＋`TalkClock` で `talk_time` を解決）。
///
/// `Emo2Wiring` 未挿入（`wire_emo2_boot`＝task 5.1 前・フォールバック boot 経路）なら早期 return の
/// no-op（安全・panic しない）。schedule への登録（`add_systems(FrameFinalize, emo2_frame_system)`）は
/// `wire_emo2_boot`（task 5.1）が行い、本関数はここでは定義のみ（登録しない）。
pub fn emo2_frame_system(world: &mut World) {
    // Emo2Wiring 未挿入（wire_emo2_boot=task 5.1 前・LogSink フォールバック boot 経路）なら no-op。
    let Some(mut wiring) = world.remove_non_send::<Emo2Wiring>() else {
        return;
    };
    // 作業領域源の実行時同期（atom 設計 C6・要件 5.1／5.4／5.5）: **各相より前**に置く。
    // 拡大率の相が読む作業領域源を同一フレームの先頭で新しくしておくと、相は新しい下端へ
    // 1 回で書ける（相の後に同期すると旧下端へ書いてから源が変わり 2 段書込になる）。
    // 表の内容が変わらないフレームでは作り直さない＝定常フレームは無操作である。
    // 戻り値（差し替えの有無）を受けるのは作業領域変化を契機とする再スナップ（task 5.2）で、
    // その置き場は拡大率の相の**後**である（下の `resnap_for_work_area_change`）。
    let work_area_change = work_area_sync::sync_monitor_snapshot(world);
    // donor 慣行: remove して &mut World を各フェーズへ排他に渡し、全フェーズ駆動後に必ず戻す。
    run_attach_phase(&mut wiring, world);
    // DPI 追従（attach → dpi → drain …の順・design「run_dpi_phase（frame.rs）」）: attach の**後**に
    // 置く——装着前は再スケール対象の target が無く、attach と同一フレームで窓が生えた直後の
    // `Changed<DPI>`（生成時 GetDpiForWindow 実値補正）を同フレームで拾えるようにするため。drain の
    // **前**に置く理由は責任分界であり順序依存ではない: エッジ駆動の再表示を先に済ませ、残った
    // 未消費要求（初回表示の k₀ 補正）を後段の状態照合経路が拾う（両経路は presenter の
    // 消費規約により二重にも取りこぼしにもならない＝どちらの順でも整合する）。
    run_dpi_phase(&mut wiring, world);
    // 作業領域変化を契機とする再スナップ（atom 設計 C6・要件 5.1／5.2／5.3／5.4／4.7）:
    // 拡大率の相の**直後**に置く。拡大率が変わらず作業領域だけが変わった窓は `Changed<DPI>`
    // が立たず相を素通りするので、ここで現寸のまま新しい下端へ射影し直す。相の**前**に置くと
    // 旧寸のまま先に動かしてから相が書き直す 2 段書込になり、後に置けば相が書き終えた窓は
    // 導出値が現在値と一致してべき等 skip で抜ける（＝同一フレームで 1 回に合流する）。
    // 作業領域が動かなかったフレームは同期段が `None` を返す＝**呼び出しごと起きない**。
    if let Some(work_area_change) = &work_area_change {
        work_area_sync::resnap_for_work_area_change(world, work_area_change);
    }
    run_drain_phase(&mut wiring, world);
    // バルーン可視性（areka-P0-balloon-visibility design 決定 D5・Requirement 3.5／6.6）: 本フレームの
    // 表示指令をすべて適用し終えた**後**に置く——判断の根拠は「指令適用後の実状態」でなければならず、
    // drain の前だと 1 フレーム古い可視状態を見る。同時に窓寸 reconcile の**前**でもある必要がある
    // ——ここで発行した表示が積む窓寸要求を同一フレームのうちに消化するため。
    run_balloon_visibility_phase(&mut wiring, world);
    // 窓寸 reconcile の第 2 経路（emo-dpi-scaling task 4.2・design Flow 2 キー決定 (d)／Flow 3 手順 5）:
    // 本フレームの全 apply が済んだ**後**に、表示成立点の状態照合が積んだ未消費の窓寸要求を取り出して
    // 窓 client へ反映する（同一フレーム内完結・エッジ消費順序に依存しない）。attach 相の初回
    // ShowSurface が積む k₀ 補正もここで landing する。task 4.3 で `run_drain_phase` 末尾から相順の
    // 所有者である本関数へ純移動した（drain の責務を「適用のみ」に保ち、可視性の相を drain と本反映の
    // 間へ挟めるようにするため。適用 → 反映という順序自体は移動前と変わらない）。
    // 移動により装着ゲート（`run_drain_phase` 冒頭の `attached` 早期 return）の内側から外側へ出るが、
    // 反映は自分でゲートされている——装着前は presenter に target が 1 つも無く、報告の取り出し
    // （`take_pending_resize`）が全 target で `None` を返すため、窓書込もログも発生しない。
    // 未装着 presenter で本 system を回して書込ゼロを主張する既存の検査
    // （`frame_text_scale_tests.rs::emo2_frame_system_runs_dpi_phase_without_writes_when_unattached`）が
    // この経路をそのまま覆っている。
    reconcile_reported_sizes(&mut wiring.presenter, world);
    // `\![move]` の末端結線: talk スレッドの MoveCueSink から届いた MoveDirective を drain し
    // apply_move_directive で実窓へ即時反映する（GhostWindows ゲート・R5・task 9.2）。present drain
    // とは独立で、GPU attach でなく GhostWindows 存在を待つ（move はキャラ窓 entity へ作用するため）。
    run_move_drain_phase(&wiring, world);
    // drain（全 PresentCommand 適用）後に shell サーフェス寸法の変化を検知し、変化した char 窓のみ
    // アンカー再適用を駆動する（適用後の実寸を読むため drain の**後**・同一 World・同一 tick 内の
    // 直接呼び・Req4.1/4.3/1.3）。text の前後とは機能的に無関係だが drain の後であることが必須。
    resnap_shell_targets(&wiring.presenter, world);
    // 初期配置の確定（scg 要件 7・C6）: 再アンカーが landing して全スコープの実表示寸が
    // 確定したフレームで**一度だけ**連鎖を解き直し、サーフェス差替で崩れた隣接を戻す。
    // resnap の**後**に置く（各窓の再アンカー結果を入力として受け取るため）。
    finalize_chain_once(&wiring.presenter, world);
    // 遷移後の連鎖再解決（atom 設計 C4・要件 6.1／6.2／6.3／6.6）: 起動時確定の**直後**に
    // 置く。拡大率の遷移では全スコープの幅が変わり、下端中央保存の再射影だけでは隣接が
    // 幅変化の半分の和だけ開く——起動時の確定は一度きりでこれを直さないので、遷移ごとに
    // 一度だけ解き直す別機構が要る。resnap・確定の**後**である必要は確定と同じ理由
    // （各窓の再射影結果と、全スコープの窓寸が遷移後の実表示寸へ揃ったことを入力に取る）。
    // 武装していないフレームは早期 return＝定常フレームは無操作である（要件 4.7）。
    realign_chain_once(&wiring.presenter, world);
    // 文字層 k 追従（emo-dpi-scaling task 7.2・D11-4・Req8）: 適用 k の更新点（dpi 相の `refresh_scale`
    // ／drain 相の `apply_show`）の**両方の下流**、かつ `present_frame` の**上流**に置く。こうすると
    // どちらの経路で k が跳ねても同一フレーム内で binding が新 k へ組み直され、直後の描画が新しい物理寸
    // で走る（旧寸の文字が 1 フレーム残らない）。戻り値（再構築 scope）は観測用ゆえ本番は捨てる。
    let _ = run_text_scale_phase(&mut wiring);
    run_text_phase(&mut wiring, world, None); // 本番: override なし（FrameTime＋clock で解決）。
    world.insert_non_send(wiring);
}

#[cfg(test)]
#[path = "frame_test_support.rs"]
mod test_support;

// 多フレーム駆動ハーネスそのものの檻（task 3.3）。**x64 のみ**で接続する——常時テストに
// x86 を用いない（要件 7.5）という実行形態を、属性 1 つで構造的に守る。
#[cfg(all(test, target_pointer_width = "64"))]
#[path = "frame_harness_tests.rs"]
mod harness_tests;

// 作業領域源の同期（task 5.1）の統合テスト。ハーネスに乗るので**x64 のみ**で接続する
// （常時テストに x86 を用いない・要件 7.5）。
#[cfg(all(test, target_pointer_width = "64"))]
#[path = "frame_work_area_sync_tests.rs"]
mod work_area_sync_tests;

// 作業領域変化を契機とする再スナップ（task 5.2）の統合テスト。同じくハーネスに乗るので
// **x64 のみ**で接続する（常時テストに x86 を用いない・要件 7.5）。
#[cfg(all(test, target_pointer_width = "64"))]
#[path = "frame_work_area_resnap_tests.rs"]
mod work_area_resnap_tests;

// 拡大率と表の整合待ち（task 5.4）の統合テスト。同じくハーネスに乗るので**x64 のみ**で
// 接続する（常時テストに x86 を用いない・要件 7.5）。
#[cfg(all(test, target_pointer_width = "64"))]
#[path = "frame_dpi_sync_hold_tests.rs"]
mod dpi_sync_hold_tests;

// 既定位置の追跡（task 5.5）の統合テスト。同じくハーネスに乗るので**x64 のみ**で接続する
// （常時テストに x86 を用いない・要件 7.5）。
#[cfg(all(test, target_pointer_width = "64"))]
#[path = "frame_default_pos_track_tests.rs"]
mod default_pos_track_tests;

// 遷移後の連鎖再解決（task 5.6）の統合テスト。同じくハーネスに乗るので**x64 のみ**で
// 接続する（常時テストに x86 を用いない・要件 7.5）。
#[cfg(all(test, target_pointer_width = "64"))]
#[path = "frame_chain_realign_tests.rs"]
mod chain_realign_tests;

// 遷移後の連鎖再解決の**武装条件**（task 5.6・3 連言の特異性）の檻。同じくハーネスに乗るので
// **x64 のみ**で接続する（常時テストに x86 を用いない・要件 7.5）。
#[cfg(all(test, target_pointer_width = "64"))]
#[path = "frame_chain_realign_arm_tests.rs"]
mod chain_realign_arm_tests;

// 遷移の原子性と随伴（task 6.1）の統合テスト。同じくハーネスに乗るので**x64 のみ**で
// 接続する（常時テストに x86 を用いない・要件 7.5）。
#[cfg(all(test, target_pointer_width = "64"))]
#[path = "frame_transition_atomicity_tests.rs"]
mod transition_atomicity_tests;

// 整合待ちと作業領域追随の判断分岐（task 6.2）の統合テスト。同じくハーネスに乗るので
// **x64 のみ**で接続する（常時テストに x86 を用いない・要件 7.5）。
#[cfg(all(test, target_pointer_width = "64"))]
#[path = "frame_transition_branch_tests.rs"]
mod transition_branch_tests;

// 整合ゲートの 4 つ目の窓書込口（task 6.5）の統合テスト。同じくハーネスに乗るので
// **x64 のみ**で接続する（常時テストに x86 を用いない・要件 7.5）。
#[cfg(all(test, target_pointer_width = "64"))]
#[path = "frame_work_area_resnap_hold_tests.rs"]
mod work_area_resnap_hold_tests;

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
#[path = "frame_chain_finalize_tests.rs"]
mod chain_finalize_tests;

#[cfg(test)]
#[path = "frame_dpi_tests.rs"]
mod dpi_tests;

#[cfg(test)]
#[path = "frame_dpi_reproject_tests.rs"]
mod dpi_reproject_tests;

#[cfg(test)]
#[path = "frame_dpi_reproject_none_tests.rs"]
mod dpi_reproject_none_tests;

// 収束の保証（areka-P0-balloon-offset-dpi task 6.2・design D16・要件 3.1／3.4）。
#[cfg(test)]
#[path = "frame_balloon_offset_converge_tests.rs"]
mod balloon_offset_converge_tests;

// キーワード再導出との排他の門（areka-P0-balloon-offset-dpi task 6.3・design D7・要件 4.3）。
#[cfg(test)]
#[path = "frame_balloon_offset_keyword_gate_tests.rs"]
mod balloon_offset_keyword_gate_tests;

#[cfg(test)]
#[path = "frame_diag_route_tests.rs"]
mod diag_route_tests;

#[cfg(test)]
#[path = "frame_text_scale_tests.rs"]
mod text_scale_tests;

// 相順そのものの検査（areka-P0-balloon-visibility task 6.6）。実 emo2 資産＋実 GPU で
// `emo2_frame_system` をそのまま駆動し、相を並べ替えたときにだけ壊れる性質を主張する。
#[cfg(test)]
#[path = "frame_visibility_integration_tests.rs"]
mod visibility_integration_tests;
