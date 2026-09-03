//! 適合一周走行の支援層（areka-P0-emo2-conformance-e2e・design D3／D6）。
//!
//! 本ファイルは**期待値を持たない**。段の駆動（注入と有界待ちの組）と、記録の突合に使う
//! 投影関数、そして進行状態の記録（第 2 系統）の型と取り出し口の本体を置く。台本と期待列は
//! `spine_conformance_script.rs`、判定は `spine_conformance_lap_tests.rs` が持つ。
//!
//! # 進行状態の記録が要る理由（R3.8・design D3「進行状態の台帳が要る理由」）
//!
//! 会話中であることは毎秒の変化通知の別（照会か片道か）と Ref3 で既に読める。**しかし選択待ちは
//! Ref3 では会話中と区別できない**——選択待ちの間も会話の枠は占有されたままで、`talk_active` と
//! `choice_active` が同時に真になり複合値 `talking,choosing` を成す
//! （`crates/areka-kanade/src/status.rs:211-216`）。Ref3 の源は `talk_active` だけ
//! （`crates/areka-kanade/src/schedule/events.rs:171-180`）ゆえ両者で同一値になる。
//! よって進行状態そのもの（組み立て済みのヘッダ値）を記録しなければ選択待ちは観測できない。

use std::sync::{Arc, Mutex};

use super::{RecordedCall, ScriptedShioriBackend, ShioriBackend};
use areka_kanade::{ExecutionSnapshot, MonotonicMs, ShioriCall};

/// 進行状態の記録 1 件（呼出 id と、その呼出に載った**組み立て済み**の進行状態の対）。
///
/// `status` は kanade が `ExecutionStatus::render()` 済みの wire 値をそのまま持つ
/// （`crates/areka-kanade/src/shiori/real.rs:136`／`:151`）。`None` は「`Status` ヘッダ行を
/// 出さない」ことを表す値であって、記録の欠落ではない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RecordedStatus {
    /// 呼出のイベント id（wire 形の逐語）。
    pub(super) id: String,
    /// 組み立て済みの進行状態。`None`＝ヘッダ行を出さない。
    pub(super) status: Option<String>,
}

/// 進行状態の記録の台帳（受け口本体と観測ハンドルが `Arc` で共有する）。
pub(super) type StatusLedger = Arc<Mutex<Vec<RecordedStatus>>>;

/// 台本受け口の書き込み点の本体（`spine.rs` の照会・片道の 2 か所から呼ばれる）。
///
/// **書き込み専用**である。既存の呼出記録とは別の台帳へ積むだけで、既存の読み手を 1 つも
/// 増やさない（design D3「追補の形（挙動を変えない）」）。
pub(super) fn record_status(ledger: &StatusLedger, id: &str, status: Option<&str>) {
    ledger
        .lock()
        .expect("status ledger mutex poisoned")
        .push(RecordedStatus {
            id: id.to_string(),
            status: status.map(str::to_string),
        });
}

/// 観測ハンドルの取り出し口の本体（進行状態の記録のスナップショットを呼出順で返す）。
pub(super) fn snapshot_status_calls(ledger: &StatusLedger) -> Vec<RecordedStatus> {
    ledger.lock().expect("status ledger mutex poisoned").clone()
}

// ===========================================================================
// 進行状態の記録（第 2 系統）の受け入れ確認（task 2.1・R3.8）
// ===========================================================================

/// 進行状態の記録が**選択待ちを会話中と区別して**読めることを固定する（R3.8）。
///
/// 檻に入れる判断分岐:
/// - **付随参照だけでは足りないこと**: 会話中のみと会話中かつ選択待ちの 2 つのスナップショットから
///   本番の構築関数が組む毎秒の変化通知は、**参照列が 1 バイトも違わない**（Ref3 の源は
///   `talk_active` だけ）。既存の呼出記録は参照列までしか持たないので、ここで区別が消える。
/// - **新しい取り出し口が区別を回復すること**: 同じ 3 呼出を台本受け口へ通すと、進行状態の記録には
///   組み立て済みのヘッダ値がそのまま残り、選択待ちの回だけが `talking,choosing` として読める。
/// - **ヘッダ行を出さない場合の表し方**: 全状態が非アクティブなら記録は `None`（＝ヘッダ行なし）。
/// - **既存の記録が変わらないこと**: 同じ走行で `non_status_calls()` が従来どおり 3 件を返す。
#[test]
fn status_ledger_reads_choosing_where_references_cannot() {
    // ── 実状態の 3 通り（いずれも本番で起こりうる組み合わせのみ） ──
    // 選択待ちの間も会話の枠は占有されたままゆえ、選択待ちは常に talk_active と同時に真になる。
    let idle = ExecutionSnapshot::INACTIVE;
    let talking = ExecutionSnapshot {
        talk_active: true,
        choice_active: false,
    };
    let choosing = ExecutionSnapshot {
        talk_active: true,
        choice_active: true,
    };

    // ── (1) 付随参照は会話中と選択待ちを区別しない（＝記録の第 2 系統が要る理由・R3.8） ──
    let talking_call = areka_kanade::events::on_second_change(MonotonicMs(0), &talking);
    let choosing_call = areka_kanade::events::on_second_change(MonotonicMs(0), &choosing);
    assert_eq!(
        call_references(&talking_call),
        call_references(&choosing_call),
        "会話中と選択待ちで参照列が違うなら Ref3 で区別できてしまい、進行状態の記録は要らない"
    );

    // ── (2) 同じ 3 呼出を台本受け口へ通す ──
    let (mut backend, handle) = ScriptedShioriBackend::builder()
        .get("OnSecondChange", Ok(None))
        .notify("OnSecondChange", Ok(()))
        .notify("OnSecondChange", Ok(()))
        .build();
    let idle_call = areka_kanade::events::on_second_change(MonotonicMs(0), &idle);
    drive_call(&mut backend, &idle_call);
    drive_call(&mut backend, &talking_call);
    drive_call(&mut backend, &choosing_call);

    // ── (3) 新しい取り出し口から進行状態が読める（選択待ちが会話中と別物として現れる） ──
    assert_eq!(
        handle.status_calls(),
        vec![
            RecordedStatus {
                id: "OnSecondChange".to_string(),
                status: None,
            },
            RecordedStatus {
                id: "OnSecondChange".to_string(),
                status: Some("talking".to_string()),
            },
            RecordedStatus {
                id: "OnSecondChange".to_string(),
                status: Some("talking,choosing".to_string()),
            },
        ],
        "進行状態の記録が組み立て済みのヘッダ値（無いときは None）を呼出順に保持していない"
    );

    // ── (4) 既存の呼出記録は素通し（追補は書き込みのみで既存の読み手を増やさない） ──
    let existing = handle.non_status_calls();
    assert_eq!(
        existing.len(),
        3,
        "既存の呼出記録が追補で変質している: {existing:?}"
    );
    assert!(
        matches!(existing[0], RecordedCall::Get { .. })
            && matches!(existing[1], RecordedCall::Notify { .. })
            && matches!(existing[2], RecordedCall::Notify { .. }),
        "既存の呼出記録の別（照会・片道）が従来どおりに残っていない: {existing:?}"
    );
}

/// [`ShioriCall`] の参照列を借りる（照会・片道のどちらでも同じ位置にある）。
fn call_references(call: &ShioriCall) -> &[String] {
    match call {
        ShioriCall::Get { references, .. } | ShioriCall::Notify { references, .. } => references,
    }
}

/// 本番の構築関数が組んだ [`ShioriCall`] を、別と id と参照列と**組み立て済み進行状態**の
/// まま台本受け口へ通す（`crates/areka-kanade/src/shiori/real.rs:136-151` と同じ渡し方）。
fn drive_call(backend: &mut ScriptedShioriBackend, call: &ShioriCall) {
    match call {
        ShioriCall::Get {
            id,
            references,
            status,
        } => {
            let wire = status.render();
            backend
                .get(id.as_str(), references, wire.as_deref())
                .expect("台本の照会応答は Ok");
        }
        ShioriCall::Notify {
            id,
            references,
            status,
        } => {
            let wire = status.render();
            backend
                .notify(id.as_str(), references, wire.as_deref())
                .expect("台本の片道応答は Ok");
        }
    }
}
