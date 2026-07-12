//! 毎フレーム三相結線（attach／drain／text）の排他 system と NonSend 配線状態。
//!
//! `Emo2Wiring`（NonSend resource・presenter／rx／runtime／clock／assets／attached を保持）と
//! 排他 system `emo2_frame_system(world: &mut World)`（donor パターン: remove→3 フェーズ→insert）を
//! 所有する。三フェーズ:
//! - attach: GPU 資源＋`GhostWindows` 到達ゲート→`plan_attachments`（DD-12）→初回 `ShowSurface`→
//!   文字層スロット取得→`register_actor_view`（`Option::take` で高々 1 回消費）。
//! - drain: attach 完了後のみ `Receiver::try_iter` で `PresentCommand` を FIFO で `presenter.apply` へ適用。
//! - text: `TalkClock::talk_time` が `Some` のとき `present_frame` を呼ぶ（`Err` は `error!`＋継続）。
//!
//! `plan_attachments`（`GhostWindows::scopes()` を正とする純関数・DD-12）も本モジュールに属する。
//!
//! 骨格のみ。実装は tasks.md task 3（`plan_attachments`）／task 4.1（attach）／task 4.2（drain・text・
//! `emo2_frame_system`）が担う。
