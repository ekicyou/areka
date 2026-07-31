//! kanade prefetch 統合テスト群（mock shiori・R4.1/4.2/9.1/9.2/9.3・タスク 6.3）。
//!
//! 本ファイルは「kanade リソース照会増分（username prefetch）」の**統合検証を第一級の群として
//! 一箇所へ集約する**（test-consolidation）。個々の判断分岐は 6.1／6.2 が既に檻へ入れており
//! （下記「基準→既存檻の対応表」）、本群が新たに埋めるのは、実アクター（`spawn_kanade`・別スレッド）
//! ＋実 `round_trip_request` 経路を通した **[`ResourceSink`] 境界での結果配送**——とりわけ
//! 204→sink `NoContent` と タイムアウト→sink `Failed`＋boot 続行——である。これは
//! `actor.rs` §9（Value 応答のみ・sink 経由）と `schedule/boot.rs`（純粋 step・sink 非経由の
//! `Action::ResourceOutcome` 検査）が覆う範囲の**外**にある唯一の統合シームであり、design
//! Testing Strategy → Integration Tests 1 が明記する「200→sink に `Value`・204→`NoContent`・
//! タイムアウト→`Failed`＋boot 続行」の後二者を sink 到達として直接観測する。
//!
//! # 決定性（Req 9.1/9.2/7.3）
//! mock shiori は即応（sleep なし）。タイムアウトは実 wall-clock ではなく
//! [`FailKind::Timeout`] で合成した [`ShioriFailure::Timeout`] を username GET へ注入して表す
//! （Instant/sleep 非依存の決定的注入）。全経路 x64 純粋・非決定 I/O なし。終了は quit シナリオ
//! （`Fixture::quitting()`＋close talk quit:true）で駆動し、`join_bounded` の期限内 join 成功が
//! 「boot が prefetch を越えて完走し正規終了した」ことの完了バリアになる（ハングは panic 化・Req 7.3）。
//! 統合スレッド構成ゆえ既知の並列 Defender-rescan flakiness を避けるべく `--test-threads=1` で回す。
//!
//! # 基準→既存檻の対応表（6.1／6.2 で既に caged・本群は統合シームのみ追加）
//! - **prefetch 順序（R9.3）**: `boot.rs::prefetch_username_get_is_issued_once_between_initialize_and_firstboot`
//!   （純粋 step）／`actor.rs §9::boot_prefetch_issues_username_between_initialize_and_firstboot_and_calls_sink`
//!   （アクター経路）／`boot_test.rs::boot_sequence_matches_canonical_exactly`（統合・正典記録列）。
//!   本群の [`prefetch_204_delivers_no_content_to_sink_and_boot_proceeds`] も記録列先頭で再確認する。
//! - **結果写像＋boot 続行（R4.1/4.2）**: `boot.rs::prefetch_maps_outcomes_to_resource_outcome`
//!   （200→Value／204→NoContent／timeout→Failed）／`boot.rs::prefetch_failure_continues_boot_not_fault`
//!   （Failed は Unloading{Fault} へ倒れず OnFirstBoot へ続行）。本群は sink 到達＋実スレッド続行を追加する。
//! - **許可外リソース ID 拒否＋egress 許可語彙（R4.1/3.2）**: `actor.rs §8::disallowed_id_still_rejected_as_internal_after_resource_extension`
//!   （Failed(Internal)）／`steady_test.rs::spontaneous_talk_egress_sweep_only_allowed_ids_no_ontalk_onhour`
//!   （egress スイープが「イベント許可 ∨ リソース許可」へ更新・username を許可語彙へ追加）。
//! - **完了固定ログ 1 回（R9.3）**: `boot.rs::prefetch_emits_fixed_completion_log_exactly_once`
//!   （3 経路とも `log_capture::assert_resource_prefetch_logged_once` で target/level/outcome/message 一致・1 回）。

use std::sync::{Arc, Mutex};

use areka_kanade::resources::{ResourceOutcome, ResourceSink};
use areka_kanade::{CloseReason, KanadeConfig, KanadeMsg, TalkCommand, spawn_kanade};

use super::common::{
    CallMethod, DEFAULT_TIMEOUT, FailKind, FailOn, Fixture, MockShiori, QuitPolicy, RecordedCall,
    join_bounded, spawn_mock_sakura, spawn_mock_shiori, spawn_mock_shiori_failing,
};

/// prefetch を実アクター経路で駆動し、`(shiori 記録列, sink 受領列)` を確定して返す。
///
/// 与えた `shiori`（良性 or username 失敗注入）へ `spawn_kanade` を結線し、**記録するだけの
/// [`ResourceSink`]** を注入する。`Boot`→`CloseRequest`（quit シナリオ）を送って close talk の
/// quit:true で終了系列を完走させ、`join_bounded` の期限内 join 成功で記録・sink 受領の双方が
/// 確定する（実時間 sleep なし・mock 即応）。`quit_flags` は mock sakura の per-talk quit 指示
/// （boot 挨拶 talk＝false・close talk＝true の順）。
fn drive_prefetch(
    shiori: MockShiori,
    quit_flags: Vec<bool>,
) -> (Vec<RecordedCall>, Vec<(&'static str, ResourceOutcome)>) {
    // 記録専用 sink（別スレッドの kanade アクターから同期的に呼ばれる・R4.1）。
    let seen: Arc<Mutex<Vec<(&'static str, ResourceOutcome)>>> = Arc::new(Mutex::new(Vec::new()));
    let seen_body = Arc::clone(&seen);
    let sink: ResourceSink =
        Box::new(move |id, outcome| seen_body.lock().expect("sink mutex").push((id, outcome)));

    // kanade→sakura の TalkCommand チャンネルを 1 本張り、kanade を記録 sink 付きで起動する。
    let (talk_tx, talk_rx) = std::sync::mpsc::channel::<TalkCommand>();
    let (kanade_tx, kanade_handle) = spawn_kanade(
        KanadeConfig::new("master", "1.0.0"),
        shiori.sender.clone(),
        talk_tx,
        sink,
    );
    // mock sakura sink には TalkDone 返送用に kanade inbox 送信端のクローンを渡す。
    let sakura = spawn_mock_sakura(talk_rx, kanade_tx.clone(), QuitPolicy::PerTalk(quit_flags));

    // 起動指示 → close 指示（active talk なしなら即握手・boot 挨拶完了後に握手）。
    kanade_tx.send(KanadeMsg::Boot).expect("send Boot");
    kanade_tx
        .send(KanadeMsg::CloseRequest {
            reason: CloseReason::User,
        })
        .expect("send CloseRequest");

    // 終了系列完走（StopSelf 到達）まで期限付き join。成功時点で prefetch 記録・sink 受領は確定済み。
    join_bounded("kanade prefetch join", DEFAULT_TIMEOUT, kanade_handle)
        .expect("kanade terminates after boot(prefetch)->close->quit (Unload->StopSelf)");

    // kanade 停止後に送信端 drop → sakura sink スレッドも自然終了（期限付き join）。
    drop(kanade_tx);
    sakura.join_bounded("mock-sakura prefetch join", DEFAULT_TIMEOUT);

    let recorded = shiori.recorded();
    let outcomes = seen.lock().expect("sink mutex").clone();
    (recorded, outcomes)
}

/// 記録列から id 列（method 問わず）を取り出す。
fn ids(recorded: &[RecordedCall]) -> Vec<&str> {
    recorded.iter().map(|c| c.id.as_str()).collect()
}

/// prefetch 204 応答が **sink へ `NoContent` として届き**、かつ prefetch が OnInitialize の後・
/// OnFirstBoot の前に 1 回だけ挟まって boot が前進することを、実アクター経路で確認する
/// （criterion 1＋criterion 2 の sink 到達・R4.1/4.2・design Integration Tests 1「204→NoContent」）。
///
/// `Fixture::quitting()` は username を fixture 表の未知 GET として 204 で返す（catch-all）。
/// §9（actor.rs）は Value 応答のみを sink 到達として観測しており、204 の分岐（`on_prefetch_reply`
/// の `NoContent` アーム）が sink まで届く経路は本統合檻が初めて覆う。
#[test]
fn prefetch_204_delivers_no_content_to_sink_and_boot_proceeds() {
    let shiori = spawn_mock_shiori(Fixture::quitting());
    // boot 挨拶 talk（id1・quit:false）→ close talk（id2・quit:true）で終了駆動（boot_test と同型）。
    let (recorded, outcomes) = drive_prefetch(shiori, vec![false, true]);

    // criterion 1（順序）: 記録列先頭 3 件が OnInitialize NOTIFY → username GET → OnFirstBoot GET。
    let head: Vec<(&CallMethod, &str)> = recorded
        .iter()
        .take(3)
        .map(|c| (&c.method, c.id.as_str()))
        .collect();
    assert_eq!(
        head,
        vec![
            (&CallMethod::Notify, "OnInitialize"),
            (&CallMethod::Get, "username"),
            (&CallMethod::Get, "OnFirstBoot"),
        ],
        "prefetch は OnInitialize の後・OnFirstBoot の前に username GET を挟むはず: {recorded:?}"
    );
    // username GET は run 全域でちょうど 1 回（prefetch は 1 回・後段で再照会しない・R9.3）。
    assert_eq!(
        recorded
            .iter()
            .filter(|c| c.method == CallMethod::Get && c.id == "username")
            .count(),
        1,
        "username prefetch GET はちょうど 1 回であるべき: {recorded:?}"
    );

    // criterion 2（sink 到達）: 204 は sink へ ("username", NoContent) をちょうど 1 回配送する。
    assert_eq!(
        outcomes,
        vec![("username", ResourceOutcome::NoContent)],
        "204 応答は sink へ ('username', NoContent) をちょうど 1 回届けるはず（R4.1/4.2）: {outcomes:?}"
    );
}

/// prefetch のタイムアウト失敗が **sink へ `Failed` として届き**、かつ boot を殺さず
/// OnFirstBoot→OnBoot→basewareversion（完了）へ続行する（Unloading{Fault} へ倒れない）ことを、
/// 実アクター経路で確認する（criterion 2「timeout→Failed＋boot 続行」・R4.1・design Integration Tests 1）。
///
/// これは本群が埋める中核ギャップ: 実 [`ResourceSink`] へ `Failed` が届く経路も、実スレッド上で
/// prefetch 失敗後に boot が完走する経路も、既存檻（§9＝Value のみ／boot.rs＝純粋 step・sink 非経由）の
/// **外**にある。`FailKind::Timeout` を username GET へ注入し、`on_prefetch_reply` の Failed アーム
/// →sink 配送→OnFirstBoot 続行が実 round_trip＋実スレッドで成立することを固定する。
#[test]
fn prefetch_timeout_delivers_failed_to_sink_and_boot_continues_not_fault() {
    // username GET の最初の呼出へタイムアウトを注入（他呼出は良性表・quit シナリオで終了駆動）。
    let shiori = spawn_mock_shiori_failing(
        Fixture::quitting(),
        FailOn {
            id: "username",
            kind: FailKind::Timeout,
        },
    );
    let (recorded, outcomes) = drive_prefetch(shiori, vec![false, true]);

    // criterion 2（sink 到達・失敗語彙）: sink へ ("username", Failed(reason)) がちょうど 1 回届く。
    assert_eq!(
        outcomes.len(),
        1,
        "prefetch 失敗でも sink はちょうど 1 回呼ばれるはず（照会は 1 回・R4.1）: {outcomes:?}"
    );
    match &outcomes[0] {
        ("username", ResourceOutcome::Failed(reason)) => assert!(
            reason.contains("timeout"),
            "Failed の理由に失敗語彙（timeout）が載るはず: {reason}"
        ),
        other => panic!("タイムアウトは sink へ ('username', Failed(..)) として届くべき: {other:?}"),
    }

    // criterion 2（boot 続行）: 失敗した username prefetch の後に boot が前進する（終了系列へ倒れない）。
    let id_seq = ids(&recorded);
    let uname_pos = id_seq
        .iter()
        .position(|&id| id == "username")
        .expect("username prefetch GET が記録されているはず");
    let firstboot_pos = id_seq
        .iter()
        .position(|&id| id == "OnFirstBoot")
        .expect("prefetch 失敗後も OnFirstBoot へ続行するはず（boot を殺さない・R4.1）");
    assert!(
        firstboot_pos > uname_pos,
        "OnFirstBoot は失敗した username prefetch の後に続くはず（boot 続行）: {recorded:?}"
    );
    assert!(
        id_seq.contains(&"OnBoot"),
        "boot は OnBoot まで前進するはず（prefetch 失敗で殺されない・R4.1）: {recorded:?}"
    );
    assert!(
        id_seq.contains(&"basewareversion"),
        "boot は basewareversion（起動系列完了）まで到達するはず: {recorded:?}"
    );
    // Unload が prefetch 直後（OnFirstBoot より前）に現れない＝Unloading{Fault} へ即倒れしていない。
    let unload_before_firstboot = recorded
        .iter()
        .take(firstboot_pos)
        .any(|c| c.method == CallMethod::Unload);
    assert!(
        !unload_before_firstboot,
        "prefetch 失敗で Unloading{{Fault}}（即 Unload）へ倒れてはならない・OnFirstBoot へ続行する: {recorded:?}"
    );
    // 期限内 join 成功（drive_prefetch 内）自体が「boot 完走→正規終了」を保証する（ハングなし・Req 7.3）。
}
