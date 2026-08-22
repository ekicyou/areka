//! 選択確定カスケードの統合檻（Req9.1／9.2(a)(b)(c)(d)・Req4.5・Req6.1・Req7.3〜7.5 —
//! 設計 Testing Strategy 「Integration Tests（kanade 檻 `choice_test.rs`）」の
//! (a)／(b)／(c)／(d)／DD-12 檻）。
//!
//! mock shiori＋mock sakura sink を kanade に結線し（`super::common` のハーネス）、注入
//! `ChoiceWaiting`／`Choice`／`Tick` のみで選択確定の全帰結を観測する。実時間 sleep も実時刻も
//! 用いず、全 join は期限付きである（Req9.1・steering `deterministic-test-coverage-mandate`）。
//! 期待 Reference 構成は必ず `events::on_choice_*` 構築子から `expected_call` で導出し、檻側に
//! ハードコードしない（fixture・assert・実装の三点一正本・Req7.1）。
//!
//! # 檻一覧（本ファイルが単一 pass/fail として固定する 3 群）
//! **群 1（Req9.2(a)・任意名形）**
//! 1. [`on_id_choice_fires_named_event_only_then_resolves_and_starts`]: `On` 始まり ID は任意名
//!    イベント **1 段のみ**を発行し（`OnChoiceSelectEx`／`OnChoiceSelect` を発行しない・裁定 1）、
//!    Value 応答が `ResolveChoice`→`Start`（この順・新 talk_id）を起こす。
//!
//! **群 2（Req9.2(b)・正典形）**
//! 2. [`canonical_choice_cascades_ex_then_select_and_resolves_without_start`]: `OnChoiceSelectEx`
//!    （Ref0=ラベル／Ref1=ID／Ref2 以降＝付随参照列）が先行し、204 で `OnChoiceSelect`（Ref0=ID）
//!    へ前進し、最終段 204 では `ResolveChoice` のみで **StartTalk が生じない**。
//! 3. [`canonical_choice_ex_value_short_circuits_select`]: 先行段が Value なら無印を**発行せず**
//!    `ResolveChoice`→`Start` へ短絡する（Req2.4・裁定 2）。
//!
//! **群 3（Req4.5・DD-12・選択起因の失敗）**
//! 4. [`choice_stage_failure_continues_as_204_without_fault_termination`]: 段 GET の `Failed` が
//!    `Unloading{Fault}` へ**倒れず** 204 相当で次段へ前進し、解決まで到達したうえで、終了は
//!    後から駆動した close によって起きる。
//! 5. [`non_choice_failure_during_choice_wait_still_faults`]: 同じ選択待ち窓でも **choice 起源で
//!    ない** GET の `Failed` は従来どおり `Unloading{Fault}` へ倒れる（DD-12 免除が非 choice 経路へ
//!    漏れていないこと＝既存 `failure_test.rs` の規律が不変であることの境界固定）。
//!
//! **群 4（Req9.2(c)・Req6.1／6.2／6.4・実行状態表示）**
//! 6. [`choosing_rides_pump_status_while_waiting_then_clears_after_resolution`]: 選択待ち確立後の
//!    周期リクエストが **NOTIFY**（Ref3="0"）で `Status: talking,choosing` を帯び、解決後の周期
//!    リクエストからは `choosing` が消えて `talking` 単独へ戻る。
//!
//! **群 5（Req9.2(d)・Req7.3／7.4／7.5・タイムアウト）**
//! 7. [`choice_timeout_fires_then_204_cancels_and_rejects_later_choice`]: 注入 Tick のみで期限へ
//!    到達し、`OnChoiceTimeout`（Ref0=起動スクリプト）GET が発行され、204 で `TalkCommand::Cancel`
//!    が届き、注入した `TalkDone{Interrupted}` で `Steady{None}` へ復帰し、以降の `Choice` 注入が
//!    棄却される。
//! 8. [`choice_timeout_value_replaces_talk_via_existing_start_path`]: タイムアウト応答が Value なら
//!    既存の起動経路で置換再生される（新 talk_id・解決／解除指示は発行しない）。
//!
//! **群 6（Req9.2(e)・Req1.1／1.3／1.4・一回性と棄却）**
//! 9. [`one_choice_injection_yields_a_single_cascade_and_later_injections_are_rejected`]: 1 回の
//!    選択確定が **高々 1 カスケード・高々 1 選択解決・高々 1 起動要求**しか起こさず、解決後に
//!    到着した同一 ID の遅延注入がイベントも指示も生まないこと。
//! 10. [`choice_outside_the_candidate_set_is_rejected_and_keeps_the_wait_open`]: 候補集合に無い ID の
//!    注入が何も起こさず、**選択待ちを閉じない**（直後の候補内 ID が正常に通る）こと。
//!
//! # 決定性（Req9.1）と同期イディオム
//! steady_test.rs／mouse_test.rs と同じ枠組み: 挨拶なし boot（`without_boot_greeting`）で
//! `Steady{None}` へ直行させ、Tick1 の `OnSecondChange` Value で steady talk（id=1）を起こし、
//! 保留ハーネス（`spawn_harness_gated`・hold_indices=[0]）でその `TalkDone` を park して active talk
//! 窓を維持する。選択待ち帳簿は「現行 talk と一致する `ChoiceWaiting`」でしか確立しないため、この
//! 窓が全群の前提である。カスケードは kanade drive ループの同期往復（execute-batch/reinject-last）で
//! 1 メッセージ処理内に完結するため、段の途中に Tick も別の選択確定も割り込まない。終了は末尾 talk を
//! quit:true にして駆動し、kanade の期限付き join 成功をもって全記録の確定点とする。
//!
//! # 期限（deadline）の作り方——実時刻を一切読まない
//! 注入する `ChoiceWaiting` は `timeout_directive_secs: None`（＝既定値
//! [`CHOICE_TIMEOUT_DEFAULT_MS`] へ委譲）で、起点は `display_end = `[`CHOICE_DISPLAY_END_MS`] ゆえ
//! 期限は [`CHOICE_DEADLINE_MS`] である。したがって**注入 Tick の `now` を選ぶだけ**で期限の手前／
//! 到達を作り分けられる（実時間待機も `Instant` 読み取りも一切ない・Req9.1／7.3）。
//! 群 1〜4 は期限より手前の Tick しか注入しないためタイムアウト経路へは構造上到達せず、群 5 が
//! 期限手前 → 期限到達の 2 点を注入してタイムアウトのみを踏む。

use std::sync::mpsc::Sender;

use areka_kanade::{
    ChoiceInput, CloseReason, ExecutionSnapshot, KanadeConfig, KanadeMsg, MonotonicMs, MouseButton,
    MouseEventKind, MouseInput, TalkCommand, TalkDone, TalkEndReason, TalkId, events,
};

use super::common::{
    CallMethod, ChoiceResponse, DEFAULT_TIMEOUT, FIXED_FAREWELL_SCRIPT, FIXED_STEADY_SCRIPT,
    FailKind, FailOn, Fixture, Harness, QuitPolicy, RecordedCall, expected_call, expected_unload,
    join_bounded, spawn_harness_gated, spawn_harness_gated_failing,
};

// テーマ単位のテストモジュール接続宣言（areka-P0-file-slimming タスク 8.3・要件 1.7 / 3.1 / 3.2）。
// 本ファイルは檻全体の module doc と共通 import のみを保持し、共有ヘルパは `choice_test_test_support.rs`
// へ、観測ケースは群ごとの兄弟ファイル `choice_test_<テーマ>_tests.rs` へ置く（子は `super::…` で本ファイルの
// import 束縛と共有ヘルパを引く）。
#[cfg(test)]
#[path = "choice_test_canonical_cascade_tests.rs"]
mod canonical_cascade_tests;
#[cfg(test)]
#[path = "choice_test_choosing_status_tests.rs"]
mod choosing_status_tests;
#[cfg(test)]
#[path = "choice_test_named_event_tests.rs"]
mod named_event_tests;
#[cfg(test)]
#[path = "choice_test_rejection_tests.rs"]
mod rejection_tests;
#[cfg(test)]
#[path = "choice_test_stage_failure_tests.rs"]
mod stage_failure_tests;
#[cfg(test)]
#[path = "choice_test_test_support.rs"]
mod test_support;
#[cfg(test)]
#[path = "choice_test_timeout_tests.rs"]
mod timeout_tests;
