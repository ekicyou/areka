//! cue-system — 演出指令の構造化定義・配送・消費基盤
//!
//! # 設計メタファー: 舞台演出のキューシート
//! > 演劇シーンを与えたら、演者が演じてくれる
//!
//! # dola 思想の共有
//! CueSheet(相対時刻) → dispatch(compile) → CueQueue(絶対時刻) → pop_ready(consume)
//!
//! # さくらスクリプトからの設計継承
//! - 単一台本モデル（1つのスクリプトが全キャラクターの演技を記述）
//! - 掛け合い（演者間の切り替え）
//! - 直感的なタイミング制御
//!
//! # さくらスクリプトからの脱構築
//! - テキストストリーム → 型付き enum
//! - グローバルスコープ → エンティティごとの独立キュー
//! - FIFO 消費 → 時刻ベース消費
//!
//! # アーキテクチャ概要
//!
//! ```text
//! ┌──────────────┐
//! │  CueSheet    │  相対時刻の演出台本（pasta DSL / areka app が生成）
//! │  Vec<Cue>    │
//! └──────┬───────┘
//!        │ PendingCueSheet として spawn
//!        ▼
//! ┌──────────────────────────────┐
//! │  dispatch_pending_cue_sheets │  システム（Update スケジュール、消費者より前）
//! │  相対 → 絶対時刻変換         │
//! │  actor ごとに分配            │
//! └──────┬───────────────────────┘
//!        │ push_sorted(TimedCue)
//!        ▼
//! ┌──────────────┐     ┌──────────────────┐
//! │  CueQueue    │◄────│ CueSheetTracker  │  配送先全体の完了・バリア集約
//! │  (per entity)│     │ (per CueSheet)   │
//! └──────┬───────┘     └──────────────────┘
//!        │ pop_ready(current_time)
//!        ▼
//!   消費者システム（balloon, animation, ...）
//! ```
//!
//! # クイックスタート: CueSheet を作って配送する
//!
//! ```rust,ignore
//! use wintf::ecs::cue::*;
//!
//! // ── Step 1: アクターの登録（起動時に1回） ──
//! // EntityRegistry にアクター名 → Entity のマッピングを登録する。
//! // 各アクターは Shell（キャラ描画）と Balloon（テキスト表示）のスロットを持つ。
//! fn setup_actors(
//!     mut commands: Commands,
//!     mut registry: ResMut<EntityRegistry>,
//! ) {
//!     // Shell エンティティ（CueQueue を持たせる）
//!     let sakura_shell = commands.spawn(CueQueue::new()).id();
//!     // Balloon エンティティ
//!     let sakura_balloon = commands.spawn(CueQueue::new()).id();
//!
//!     // レジストリに登録
//!     registry.register_actor("sakura", CueTarget::Shell, sakura_shell);
//!     registry.register_actor("sakura", CueTarget::Balloon, sakura_balloon);
//! }
//!
//! // ── Step 2: CueSheet の生成と配送リクエスト ──
//! // CueSheet は相対時刻で記述する。PendingCueSheet として spawn すると
//! // 次フレームの dispatch_pending_cue_sheets で自動配送される。
//! fn start_dialogue(
//!     mut commands: Commands,
//!     frame_time: Res<FrameTime>,
//! ) {
//!     let sheet = CueSheet::new(vec![
//!         Cue {
//!             actor: ActorKey::from("sakura"),
//!             start_time: 0.0,  // 開始と同時
//!             command: CueCommand::Text("こんにちは！".into()),
//!         },
//!         Cue {
//!             actor: ActorKey::from("sakura"),
//!             start_time: 0.0,
//!             command: CueCommand::Emote { key: "smile".into() },
//!         },
//!         Cue {
//!             actor: ActorKey::from("sakura"),
//!             start_time: 2.0,  // 2秒後
//!             command: CueCommand::WaitForClick { timeout: Some(10.0) },
//!         },
//!     ]);
//!
//!     // PendingCueSheet として spawn → 次フレームで自動 dispatch
//!     commands.spawn(PendingCueSheet {
//!         sheet,
//!         start_time: frame_time.0,
//!     });
//! }
//! ```
//!
//! # 消費者パターン: バルーン（テキスト表示）
//!
//! 消費者システムは `CueQueue::pop_ready(current_time)` で時刻到達済みの
//! コマンドを取得し、自ドメインのコマンドのみを処理する。
//! バリア応答が必要な場合は `CueSheetTracker::receive_barrier()` で報告する。
//!
//! ```rust,ignore
//! /// バルーン消費者システム（Update スケジュール）
//! fn consume_balloon_cues(
//!     mut query: Query<(&mut CueQueue, &mut BalloonContent)>,
//!     mut tracker_query: Query<&mut CueSheetTracker>,
//!     frame_time: Res<FrameTime>,
//! ) {
//!     let now = frame_time.0;
//!
//!     for (mut queue, mut content) in query.iter_mut() {
//!         // ── 時刻到達コマンドを取得 ──
//!         let commands = queue.pop_ready(now);
//!
//!         for cmd in commands {
//!             match cmd {
//!                 CueCommand::Text(text) => {
//!                     content.append_text(&text);
//!                 }
//!                 CueCommand::Clear => {
//!                     content.clear();
//!                 }
//!                 CueCommand::Emote { key } => {
//!                     // バルーンでは Emote をフォントセット切替に使う
//!                     content.set_font_style(&key);
//!                 }
//!                 // 自ドメイン外のコマンドは無視（他の消費者が処理）
//!                 _ => {}
//!             }
//!         }
//!
//!         // ── バリア応答 ──
//!         // CueQueue がバリア状態に入ったら、消費者がハンドラーかどうかを判定し
//!         // CueSheetTracker に応答を報告する。
//!         if let Some(barrier_kind) = queue.pending_barrier_kind() {
//!             let tracker_entity = queue.cue_sheet_entity();
//!             let response = match barrier_kind {
//!                 BarrierKind::Click => {
//!                     // バルーンはクリックイベントを受け取れるのでハンドラー
//!                     if content.was_clicked() {
//!                         BarrierResponse::Click
//!                     } else {
//!                         BarrierResponse::Skipped // まだクリックされていない
//!                     }
//!                 }
//!                 BarrierKind::Choice => {
//!                     // バルーンは選択肢UIを表示してユーザー入力を待つ
//!                     let choices = queue.pending_choices();
//!                     content.show_choices(choices);
//!                     if let Some(selected_id) = content.poll_choice_selection() {
//!                         BarrierResponse::Choice { id: selected_id }
//!                     } else {
//!                         BarrierResponse::Skipped // まだ選択されていない
//!                     }
//!                 }
//!             };
//!
//!             // 応答を Tracker に報告（first valid wins）
//!             if let Some(entity) = tracker_entity {
//!                 if let Ok(mut tracker) = tracker_query.get_mut(entity) {
//!                     tracker.receive_barrier(response);
//!                 }
//!             }
//!         }
//!     }
//! }
//! ```
//!
//! # 消費者パターン: アニメーション（サーフェス制御）
//!
//! ```rust,ignore
//! /// アニメーション消費者システム
//! fn consume_animation_cues(
//!     mut query: Query<(&mut CueQueue, &mut SurfaceAnimator)>,
//!     frame_time: Res<FrameTime>,
//! ) {
//!     let now = frame_time.0;
//!
//!     for (mut queue, mut animator) in query.iter_mut() {
//!         let commands = queue.pop_ready(now);
//!
//!         for cmd in commands {
//!             match cmd {
//!                 CueCommand::Emote { key } => {
//!                     // Shell では Emote をサーフェスアニメーション選択に使う
//!                     animator.play_animation(&key);
//!                 }
//!                 CueCommand::Custom { command, params } => {
//!                     // Custom コマンドで消費者固有の処理を実行
//!                     match command.as_str() {
//!                         "fade_in" => {
//!                             let duration = params.get("duration")
//!                                 .and_then(|v| v.as_f64())
//!                                 .unwrap_or(0.5);
//!                             animator.fade_in(duration);
//!                         }
//!                         _ => { /* 未知の Custom は無視 */ }
//!                     }
//!                 }
//!                 _ => {} // Text, Clear 等はアニメーション層では無視
//!             }
//!         }
//!     }
//! }
//! ```
//!
//! # 選択肢（Choice）プロトコル
//!
//! Choice コマンドは WaitForChoice の**前に**連続投入する先積みプロトコルです。
//!
//! ```rust,ignore
//! // CueSheet での記述例
//! let sheet = CueSheet::new(vec![
//!     Cue { actor: ActorKey::from("sakura"), start_time: 0.0,
//!            command: CueCommand::Text("どちらにしますか？".into()) },
//!     // Choice を先に積む（WaitForChoice の前に）
//!     Cue { actor: ActorKey::from("sakura"), start_time: 0.5,
//!            command: CueCommand::Choice { id: "yes".into(), text: "はい".into() } },
//!     Cue { actor: ActorKey::from("sakura"), start_time: 0.5,
//!            command: CueCommand::Choice { id: "no".into(), text: "いいえ".into() } },
//!     // WaitForChoice で先行 Choice をブロック＆提示
//!     Cue { actor: ActorKey::from("sakura"), start_time: 0.5,
//!            command: CueCommand::WaitForChoice { timeout: Some(30.0) } },
//! ]);
//!
//! // pop_ready() の動作:
//! // 1. Text("どちらにしますか？") → 返却される
//! // 2. Choice { id: "yes", .. } → pending_choices に蓄積（返却されない）
//! // 3. Choice { id: "no", .. } → pending_choices に蓄積（返却されない）
//! // 4. WaitForChoice → キュー状態が WaitingForChoice に遷移（以降 pop_ready は空）
//! //
//! // バルーン消費者は queue.pending_choices() で蓄積された選択肢を取得し、
//! // UIに表示する。ユーザーが選択したら：
//! //   tracker.receive_barrier(BarrierResponse::Choice { id: "yes".into() });
//! //
//! // CueSheetTracker は CueSheetResult::Choice { id: "yes" } を発行して完了する。
//! ```
//!
//! # CueSheetResult のポーリング
//!
//! CueSheet の実行結果は `CueSheetTracker::result()` で毎フレーム poll します。
//! Modal Dialog の `DialogResult` に相当するパターンです。
//!
//! ```rust,ignore
//! fn poll_cue_results(
//!     query: Query<(Entity, &CueSheetTracker)>,
//!     mut commands: Commands,
//! ) {
//!     for (entity, tracker) in query.iter() {
//!         match tracker.result() {
//!             Some(CueSheetResult::Completed) => {
//!                 // 全演者の CueQueue が消費完了 → 次の台本を開始
//!                 commands.entity(entity).despawn();
//!                 start_next_dialogue(&mut commands);
//!             }
//!             Some(CueSheetResult::Choice { id }) => {
//!                 // 選択肢が選択された → 分岐処理
//!                 commands.entity(entity).despawn();
//!                 handle_choice(id, &mut commands);
//!             }
//!             Some(CueSheetResult::Timeout) => {
//!                 // バリアのタイムアウト → デフォルト動作
//!                 commands.entity(entity).despawn();
//!             }
//!             Some(CueSheetResult::Cancelled) => {
//!                 commands.entity(entity).despawn();
//!             }
//!             Some(CueSheetResult::Error(e)) => {
//!                 tracing::error!(error = %e, "CueSheet execution error");
//!                 commands.entity(entity).despawn();
//!             }
//!             None => {
//!                 // まだ実行中 — 何もしない
//!             }
//!         }
//!     }
//! }
//! ```
//!
//! # システムスケジュール順序
//!
//! ```text
//! Update スケジュール内の実行順序:
//!
//!   1. dispatch_pending_cue_sheets   ← PendingCueSheet を消費して CueQueue に配送
//!   2. 消費者システム群              ← pop_ready() + バリア応答
//!   3. update_cue_sheet_trackers     ← バリア集約・完了判定
//! ```

pub mod command;
pub mod component;
pub mod dispatch;
pub mod error;
pub mod queue;
pub mod registry;
pub mod systems;
pub mod tracker;

// ── re-exports ──
pub use command::{
    ActorKey, BarrierKind, CompiledCue, Cue, CueCommand, CuePayload, CueSheet, CueTarget,
    EntityKey, Entry, RoutingCommand, TimedSchedule, compile_sheet,
};
pub use component::PendingCueSheet;
pub use dispatch::{CueSheetHandle, dispatch_pending_cue_sheets};
pub use error::{CueSheetResult, CueSystemError};
pub use queue::{BarrierResponse, CueQueue, CueQueueState, PendingChoice};
pub use registry::EntityRegistry;
pub use systems::update_cue_sheet_trackers;
pub use tracker::CueSheetTracker;
pub use tracker::{QueueSnapshot, TrackerAction};
