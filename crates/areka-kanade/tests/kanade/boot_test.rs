//! boot 系列の順序・Method・Reference 検証（Req 1.5）。
//!
//! mock shiori＋mock sakura sink を kanade に結線し（`super::common` のハーネス）、
//! 起動指示から起動系列完了までの shiori 記録列が、正典（ukadoc Reference 表）と
//! 完全一致することを単一の `#[test]` で検証する。full_run_test.rs（boot→pump→close の
//! 全体観測）に対し、本ファイルは起動系列そのものへ焦点を絞った適合検証（Req 1.5）——
//! 記録列の先頭 4 件が events 表から導出した期待列と、順序・NOTIFY／GET の別・Reference
//! 構成のすべてで完全一致することを厳密に検証する。
//!
//! 決定性（Req 7.3）は full_run_test.rs と同じ同期イディオムで担保する: quit シナリオ
//! （`Fixture::quitting()`＋`QuitPolicy::PerTalk(vec![false, true])`）で駆動し、close talk
//! 完了で終了系列が完走してから kanade を期限付き join する。join 成功時点で起動系列の
//! 全 shiori 呼出は記録済みであり、実時間 sleep なしで記録列を確定できる。

use areka_kanade::{CloseReason, ExecutionSnapshot, KanadeConfig, KanadeMsg, events, resources};

use super::common::{
    DEFAULT_TIMEOUT, Fixture, Harness, QuitPolicy, RecordedCall, expected_call, join_bounded,
    spawn_harness,
};

/// 起動系列を駆動し、記録が確定した shiori 記録列を返す（同期は kanade 終了 join で担保）。
///
/// full_run_test.rs と同じ手順で決定的に同期する: quit シナリオで Boot→CloseRequest を送り、
/// close talk（PerTalk index 1＝quit:true）の完了が終了系列（Unload→StopSelf）を駆動する。
/// kanade の期限付き join が成功すれば、それまでの全 shiori 呼出（起動系列を含む）は記録済み。
/// 実時間 sleep を一切用いず（mock 即応・時刻は Boot/Close のみ）記録列を確定する。
fn drive_boot_and_collect() -> Vec<RecordedCall> {
    // quit シナリオ: OnClose が別れの Value を返し、close talk（id 2）の quit:true で
    // 終了系列が完走する。boot talk（id 1）は quit:false（起動系列の観測を妨げない）。
    let harness = spawn_harness(
        KanadeConfig::new("master", "1.0.0"),
        Fixture::quitting(),
        QuitPolicy::PerTalk(vec![false, true]),
    );

    // 起動指示。
    harness.sender.send(KanadeMsg::Boot).expect("send Boot");

    // close 指示。active talk なしゆえ即握手開始（OnClose GET→別れの Value→close talk）。
    // close talk の TalkDone（quit:true）が終了系列を駆動する。
    harness
        .sender
        .send(KanadeMsg::CloseRequest {
            reason: CloseReason::User,
        })
        .expect("send CloseRequest");

    let Harness {
        sender,
        kanade,
        shiori,
        sakura,
    } = harness;

    // 終了系列完走（StopSelf 到達）まで期限付き join。成功時点で起動系列は記録済み。
    join_bounded("kanade boot join", DEFAULT_TIMEOUT, kanade)
        .expect("kanade terminates after close→quit sequence (Unload→StopSelf)");

    // kanade 停止後に送信端 drop → sakura sink スレッドも自然終了（期限付き join）。
    drop(sender);
    sakura.join_bounded("mock-sakura boot join", DEFAULT_TIMEOUT);

    shiori.recorded()
}

/// 起動系列の記録列が正典と完全一致する（順序・NOTIFY／GET の別・Reference 構成・Req 1.5）。
///
/// 期待列は `areka_kanade::events::*` から `expected_call` で導出する（Reference をテスト側に
/// ハードコードしない・単一正本＝events 表を共有・Req 7.1）。記録列の先頭 4 件が期待起動系列と
/// 完全一致することを厳密に検証する（起動系列そのものへ焦点を絞った適合検証）。
///
/// 期待起動系列（ukadoc Reference 表・design「boot 系列」＋prefetch 増分・R4.1）:
/// 1. `OnInitialize` — NOTIFY（References なし・talk 非アクティブ＝Status 行なし）
/// 2. `username` — GET（References なし・prefetch リソース照会）→ fixture が 204 を返す（R4.1）
/// 3. `OnFirstBoot` — GET（Ref0="0"）→ fixture が 204 を返す（talk 非アクティブ）
/// 4. `OnBoot` — GET（Ref0=shell_name）→ fixture が固定 Value を返す（talk 非アクティブ）
/// 5. `basewareversion` — NOTIFY（Ref0=version・Ref1=name・**Status: talking**）
///
/// prefetch（`username` 照会）は OnInitialize の後・OnFirstBoot の前に 1 回だけ挟まる（R4.1・design
/// boot 図）。DD-IT-12: 挨拶（`OnBoot` Value）を正規追跡するため、`basewareversion` はフェーズ更新後
/// （`BootVersion{talk: Some}`＝talk アクティブ）のスナップショットで送出され、Status ヘッダに
/// `talking` を運ぶ。前 4 件は talk 非アクティブ（[`ExecutionSnapshot::INACTIVE`]）で Status 行なし。
#[test]
fn boot_sequence_matches_canonical_exactly() {
    // 期待値導出用の config（events 表への入力）。`KanadeConfig` は Clone 非実装のため、
    // ハーネスへ渡す実体は同一パラメタで別途構築する（`new` は決定的ゆえ同値になる）。
    let config = KanadeConfig::new("master", "1.0.0");

    // 期待起動系列を events 表から導出する（順序・Method・Reference・Status の単一正本・Req 7.1）。
    // boot 系列前段は INACTIVE（Status 行なし）・basewareversion は挨拶追跡後の talk_active=true
    // （Status: talking・DD-IT-12）。
    let expected_boot = vec![
        expected_call(events::on_initialize(&ExecutionSnapshot::INACTIVE)), // NOTIFY（References なし）
        expected_call(resources::resource_username(&ExecutionSnapshot::INACTIVE)), // GET（prefetch・R4.1）→204
        expected_call(events::on_first_boot(&ExecutionSnapshot::INACTIVE)), // GET（Ref0="0"）→204
        expected_call(events::on_boot(&config, &ExecutionSnapshot::INACTIVE)), // GET（Ref0=shell_name）→Value
        expected_call(events::baseware_version(
            &config,
            &ExecutionSnapshot { talk_active: true },
        )), // NOTIFY（Ref0=version・Ref1=name・Status: talking）
    ];

    let recorded = drive_boot_and_collect();

    // 記録列は起動系列（4 件）以上でなければならない（起動系列が発火した最低条件）。
    assert!(
        recorded.len() >= expected_boot.len(),
        "記録列が起動系列より短い（起動系列が完走していない）: {recorded:?}"
    );

    // 起動系列そのものの完全一致（順序・NOTIFY／GET の別・Reference 構成のすべて・Req 1.5）。
    // 先頭 4 件が期待起動系列と厳密一致する——順序が入れ替われば、GET/NOTIFY を取り違えれば、
    // Reference 構成が正典とずれれば、この assert が落ちる。
    assert_eq!(
        &recorded[..expected_boot.len()],
        expected_boot.as_slice(),
        "起動系列が正典（順序・NOTIFY／GET の別・Reference 構成）と完全一致しない"
    );
}
