//! kanade prefetch 統合テスト群（mock shiori・username prefetch）。
//!
//! 実アクター（`spawn_kanade`・別スレッド）と実 `round_trip_request` 経路を通した
//! [`ResourceSink`] 境界での結果配送（204→sink `NoContent`・タイムアウト→sink `Failed`＋boot 続行）を
//! 見る。判断分岐そのものは純粋 step とアクターの檻が持つ。
//!
//! # 決定性
//! mock shiori は即応（sleep なし）。タイムアウトは実 wall-clock ではなく [`FailKind::Timeout`] で
//! 合成した [`ShioriFailure::Timeout`] を username GET へ注入して表す。終了は quit シナリオ
//! （`Fixture::quitting()`＋close talk quit:true）で駆動し、`join_bounded` の期限内 join 成功が
//! 「boot が prefetch を越えて完走し正規終了した」ことの完了バリアになる（ハングは panic 化）。
//!
//! # 判断分岐を持つ既存の檻
//! - prefetch 順序: `boot_reply_branch_tests.rs::prefetch_username_get_is_issued_once_between_initialize_and_firstboot`
//!   ／`actor_tests.rs::boot_prefetch_issues_username_between_initialize_and_firstboot_and_calls_sink`
//!   ／`full_run_test.rs::full_run_boot_pump_close_terminates_deterministically`。
//! - 結果写像＋boot 続行: `boot_reply_branch_tests.rs::prefetch_maps_outcomes_to_resource_outcome`
//!   ／`boot_reply_branch_tests.rs::prefetch_failure_continues_boot_not_fault`。
//! - 許可外リソース ID 拒否: `actor_resources_tests.rs::steady_round_trips_each_id_and_rejects_only_the_disallowed_one`
//!   ／`steady_test.rs::sporadic_value_starts_unique_talk`。
//! - 完了固定ログ 1 回: `boot_reply_branch_tests.rs::prefetch_emits_fixed_completion_log_exactly_once`。

use std::sync::{Arc, Mutex};

use areka_kanade::resources::{ResourceOutcome, ResourceSink};
use areka_kanade::{CloseReason, KanadeConfig, KanadeMsg, TalkCommand, spawn_kanade};

use super::common::{
    CallMethod, DEFAULT_TIMEOUT, FailKind, FailOn, Fixture, MockShiori, QuitPolicy, RecordedCall,
    join_bounded, spawn_mock_sakura, spawn_mock_shiori, spawn_mock_shiori_failing,
};

/// prefetch を実アクター経路で駆動し、`(shiori 記録列, sink 受領列)` を確定して返す。
///
/// 与えた `shiori`（良性 or username 失敗注入）へ `spawn_kanade` を結線し、記録するだけの
/// [`ResourceSink`] を注入する。`Boot`→`CloseRequest` を送って close talk の quit:true で終了系列を
/// 完走させ、`join_bounded` の期限内 join 成功で記録・sink 受領の双方が確定する。`quit_flags` は
/// mock sakura の per-talk quit 指示（boot 挨拶 talk＝false・close talk＝true の順）。
fn drive_prefetch(
    shiori: MockShiori,
    quit_flags: Vec<bool>,
) -> (Vec<RecordedCall>, Vec<(&'static str, ResourceOutcome)>) {
    // 別スレッドの kanade アクターから同期的に呼ばれる。
    let seen: Arc<Mutex<Vec<(&'static str, ResourceOutcome)>>> = Arc::new(Mutex::new(Vec::new()));
    let seen_body = Arc::clone(&seen);
    let sink: ResourceSink =
        Box::new(move |id, outcome| seen_body.lock().expect("sink mutex").push((id, outcome)));

    let (talk_tx, talk_rx) = std::sync::mpsc::channel::<TalkCommand>();
    let (kanade_tx, kanade_handle) = spawn_kanade(
        KanadeConfig::new("master", "1.0.0"),
        shiori.sender.clone(),
        talk_tx,
        sink,
    );
    let sakura = spawn_mock_sakura(talk_rx, kanade_tx.clone(), QuitPolicy::PerTalk(quit_flags));

    kanade_tx.send(KanadeMsg::Boot).expect("send Boot");
    kanade_tx
        .send(KanadeMsg::CloseRequest {
            reason: CloseReason::User { scope: 0 },
        })
        .expect("send CloseRequest");

    join_bounded("kanade prefetch join", DEFAULT_TIMEOUT, kanade_handle)
        .expect("kanade terminates after boot(prefetch)->close->quit (Unload->StopSelf)");

    drop(kanade_tx);
    let _ = sakura.join_bounded_then_commands("mock-sakura prefetch join", DEFAULT_TIMEOUT);

    let recorded = shiori.recorded();
    let outcomes = seen.lock().expect("sink mutex").clone();
    (recorded, outcomes)
}

/// 記録列から id 列（method 問わず）を取り出す。
fn ids(recorded: &[RecordedCall]) -> Vec<&str> {
    recorded.iter().map(|c| c.id.as_str()).collect()
}

/// prefetch 204 応答が sink へ `NoContent` として届き、かつ prefetch が OnInitialize の後・
/// OnFirstBoot の前に 1 回だけ挟まって boot が前進することを、実アクター経路で確認する。
///
/// `Fixture::quitting()` は username を fixture 表の未知 GET として 204 で返す（catch-all）。
/// `actor_tests.rs` の prefetch 檻は Value 応答のみを sink 到達として観測しており、204 の分岐
/// （`on_prefetch_reply` の `NoContent` アーム）が sink まで届く経路は本檻だけが覆う。
#[test]
fn prefetch_204_delivers_no_content_to_sink_and_boot_proceeds() {
    let shiori = spawn_mock_shiori(Fixture::quitting());
    // boot 挨拶 talk（quit:false）→ close talk（quit:true）で終了を駆動する。
    let (recorded, outcomes) = drive_prefetch(shiori, vec![false, true]);

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
    assert_eq!(
        recorded
            .iter()
            .filter(|c| c.method == CallMethod::Get && c.id == "username")
            .count(),
        1,
        "username prefetch GET はちょうど 1 回であるべき: {recorded:?}"
    );

    assert_eq!(
        outcomes,
        vec![("username", ResourceOutcome::NoContent)],
        "204 応答は sink へ ('username', NoContent) をちょうど 1 回届けるはず（R4.1/4.2）: {outcomes:?}"
    );
}

/// prefetch のタイムアウト失敗が sink へ `Failed` として届き、かつ boot を殺さず
/// OnFirstBoot→OnBoot→basewareversion（完了）へ続行する（Unloading{Fault} へ倒れない）ことを、
/// 実アクター経路で確認する。
///
/// 実 [`ResourceSink`] へ `Failed` が届く経路も、実スレッド上で prefetch 失敗後に boot が完走する経路も、
/// 既存檻（`actor_tests.rs`＝Value のみ／boot.rs＝純粋 step・sink 非経由）の外にある。
/// `FailKind::Timeout` を username GET へ注入し、`on_prefetch_reply` の Failed アーム→sink 配送→
/// OnFirstBoot 続行が実 round_trip＋実スレッドで成立することを固定する。
#[test]
fn prefetch_timeout_delivers_failed_to_sink_and_boot_continues_not_fault() {
    let shiori = spawn_mock_shiori_failing(
        Fixture::quitting(),
        FailOn::new("username", FailKind::Timeout),
    );
    let (recorded, outcomes) = drive_prefetch(shiori, vec![false, true]);

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
        other => {
            panic!("タイムアウトは sink へ ('username', Failed(..)) として届くべき: {other:?}")
        }
    }

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
        id_seq.contains(&"basewareversion"),
        "boot は basewareversion（起動系列完了）まで到達するはず（prefetch 失敗で殺されない・R4.1）: {recorded:?}"
    );
}
