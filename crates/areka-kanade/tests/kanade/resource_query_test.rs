//! 殻で答えるリソース照会（`KanadeMsg::ResourceQuery`）の統合檻（要件 9.2・タスク 2.3）。
//!
//! 偽 SHIORI（既存ハーネス）を相手に、項目名リソースの応答 4 通り——値を返す／空を返す／
//! 値なしを返す／失敗する——が結果語彙 [`ResourceOutcome`] の 4 通りへ 1 対 1 で写ることを
//! 確かめる（要件 3.2〜3.4）。加えて、会話できる状態になる前（`Phase::Idle`）に届いた照会が
//! SHIORI へ 1 通も送らず全件「値なし」で返ることを確かめる（要件 3.10）。
//!
//! # 観測の形
//! 照会は `ids` と同じ順・同じ長さの `Vec<(id, ResourceOutcome)>` が 1 度だけ返る契約なので、
//! どの檻も 3 件（`sakura.popupmenu.visible`・`readmebutton.caption`・`closebutton.caption`）を
//! まとめて問い、順序・長さも同時に見る。偽 SHIORI の記録列（`MockShiori::recorded`）から
//! 「その 3 名の GET が実際に出たか」を数え、応答語彙と送出の有無を突き合わせる。
//!
//! # 決定性（要件 9.2・記憶 deterministic-test-coverage-mandate）
//! 挨拶なし boot（`without_boot_greeting`）で `Steady{None}` へ直行させ、末尾の close talk を
//! quit:true にして終了系列（Unload→StopSelf）を駆動する。kanade の期限付き join が成功した
//! 時点で全 shiori 呼出と照会応答は確定しており、実時間 sleep を一切使わない。待ちはすべて
//! 期限付き（`join_bounded`／`ReplyReceiver::recv_timeout`）で、殻が答えなくなる後退は
//! ハングではなく赤として現れる。
//!
//! # 起動中の照会が踏む phase について
//! kanade アクターは 1 メッセージを同期完走させてから次を受ける。`Boot` は boot 系列を
//! 丸ごと同期完走して `Steady` まで進むため、`Boot` の後に投函した照会が boot 途中の phase で
//! 処理されることは構造上あり得ない（照会は inbox で待ち、`Steady` になってから処理される）。
//! ゆえに「会話できる状態でない」側を inbox から観測できる phase は `Boot` より前の `Idle`
//! であり、[`query_before_boot_returns_no_content_for_every_id`] はそこを踏む。

use areka_actor::reply_channel;
use areka_kanade::resources::ResourceOutcome;
use areka_kanade::{CloseReason, KanadeConfig, KanadeMsg};

use super::common::{
    CallMethod, DEFAULT_TIMEOUT, FailKind, FailOn, Fixture, Harness, MouseResponse, QuitPolicy,
    RecordedCall, join_bounded, spawn_harness, spawn_harness_failing,
};

/// 1 度に問う項目名（表示可否・説明書・終了）。順序・長さの突合に使う正本でもある。
const MENU_IDS: [&str; 3] = [
    "sakura.popupmenu.visible",
    "readmebutton.caption",
    "closebutton.caption",
];

/// 檻が注目する 1 名（里々検体の実機観察対象でもある・要件 9.9 ⑷）。
const README_CAPTION: &str = "readmebutton.caption";

/// 照会の投函位置（単一 inbox の FIFO で処理される phase が決まる）。
#[derive(Clone, Copy, PartialEq, Eq)]
enum QueryAt {
    /// `Boot` より前に投函する＝`Phase::Idle`（会話できない状態）で処理される。
    BeforeBoot,
    /// `Boot` の後に投函する＝boot 同期完走後の `Steady{None}` で処理される。
    Steady,
}

/// 照会を 1 件送って終了まで走らせ、`(応答, shiori 記録列)` を確定して返す。
///
/// `Boot`／照会／`CloseRequest` を `at` の指す順序で投函し、close talk（quit:true）で終了系列を
/// 駆動する。kanade の期限付き join 成功が「照会の処理も記録も確定した」ことの完了バリアであり、
/// 応答の受領も期限付きで行う（殻が答えない後退は待ち続けずに赤になる）。
fn run_query(
    harness: Harness,
    ids: Vec<&'static str>,
    at: QueryAt,
) -> (Vec<(&'static str, ResourceOutcome)>, Vec<RecordedCall>) {
    let (reply, receiver) = reply_channel::<Vec<(&'static str, ResourceOutcome)>>();
    let query = KanadeMsg::ResourceQuery { ids, reply };

    match at {
        QueryAt::BeforeBoot => {
            harness
                .sender
                .send(query)
                .expect("send ResourceQuery before Boot");
            harness.sender.send(KanadeMsg::Boot).expect("send Boot");
        }
        QueryAt::Steady => {
            harness.sender.send(KanadeMsg::Boot).expect("send Boot");
            harness
                .sender
                .send(query)
                .expect("send ResourceQuery after Boot");
        }
    }
    harness
        .sender
        .send(KanadeMsg::CloseRequest {
            reason: CloseReason::User { scope: 0 },
        })
        .expect("send CloseRequest");

    let Harness {
        sender,
        kanade,
        shiori,
        sakura,
    } = harness;

    join_bounded("kanade resource-query join", DEFAULT_TIMEOUT, kanade)
        .expect("kanade terminates after boot -> query -> close -> quit");
    drop(sender);
    sakura.join_bounded("mock-sakura resource-query join", DEFAULT_TIMEOUT);

    let answers = receiver
        .recv_timeout(DEFAULT_TIMEOUT)
        .expect("照会の応答は期限内にちょうど 1 度返るはず（殻が答えないと UI は待たされ続ける）");
    (answers, shiori.recorded())
}

/// 記録列から [`MENU_IDS`] の GET だけを到着順で取り出す（boot の username 照会等は除く）。
fn menu_gets(recorded: &[RecordedCall]) -> Vec<&str> {
    recorded
        .iter()
        .filter(|call| call.method == CallMethod::Get && MENU_IDS.contains(&call.id.as_str()))
        .map(|call| call.id.as_str())
        .collect()
}

/// 応答から 1 名分の結果語彙を取り出す（不在は説明付きで落とす）。
fn outcome_for<'a>(
    answers: &'a [(&'static str, ResourceOutcome)],
    id: &str,
) -> &'a ResourceOutcome {
    answers
        .iter()
        .find(|(answered, _)| *answered == id)
        .map(|(_, outcome)| outcome)
        .unwrap_or_else(|| panic!("応答に {id} が含まれていない: {answers:?}"))
}

/// 応答が入力の id と同じ順・同じ長さであることを確かめる（4 檻すべてが通る共通契約）。
fn assert_same_order_and_length(answers: &[(&'static str, ResourceOutcome)]) {
    let answered: Vec<&str> = answers.iter().map(|(id, _)| *id).collect();
    assert_eq!(
        answered,
        MENU_IDS.to_vec(),
        "応答は入力の id と同じ順・同じ長さで返るはず（設計 Event Contract）: {answers:?}"
    );
}

/// 挨拶なし・quit close の共通 fixture（`Steady{None}` 直行＋終了系列駆動）。
fn menu_fixture() -> Fixture {
    Fixture::quitting().without_boot_greeting()
}

/// 共通の運行構成（既存の統合檻と同一）。
fn config() -> KanadeConfig {
    KanadeConfig::new("master", "1.0.0")
}

// ============================================================================
// ⑴ 値を返す → `ResourceOutcome::Value(値)`
// ============================================================================

/// 項目名リソースが値を返したら、その値がそのまま `Value` として返る（要件 3.2）。
///
/// # 非空虚性
/// 注入した文言は fixture 側にしか無いため、殻が SHIORI へ問い合わせず既定へ倒していれば
/// `NoContent` になって落ちる。3 名すべての GET が記録に現れることも同時に見る。
#[test]
fn caption_value_is_returned_verbatim() {
    let harness = spawn_harness(
        config(),
        menu_fixture().with_resource_response(
            README_CAPTION,
            MouseResponse::Script("取扱説明書(&R)".to_string()),
        ),
        QuitPolicy::PerTalk(vec![true]),
    );

    let (answers, recorded) = run_query(harness, MENU_IDS.to_vec(), QueryAt::Steady);

    assert_same_order_and_length(&answers);
    assert_eq!(
        outcome_for(&answers, README_CAPTION),
        &ResourceOutcome::Value("取扱説明書(&R)".to_string()),
        "値を返す応答は値をそのまま載せた Value になるはず（要件 3.2）: {answers:?}"
    );
    assert_eq!(
        menu_gets(&recorded),
        MENU_IDS.to_vec(),
        "会話できる状態の照会は 3 名すべてを入力順に GET するはず: {recorded:?}"
    );
}

// ============================================================================
// ⑵ 空を返す → `ResourceOutcome::Value("")`（値なしとは別物）
// ============================================================================

/// 項目名リソースが空文字を返したら、`NoContent` ではなく空の `Value` として返る
/// （要件 3.3 の「空を返す」と要件 3.4 の「値なし」を取り違えない）。
///
/// # 非空虚性
/// 200 の空文字を 204 と同一視する後退は、ここだけが捕まえる（他の 4 檻は素通りする）。
/// 既定名へ倒すかどうかを決めるのは受け手であり、殻は空を空のまま運ぶ。
#[test]
fn empty_caption_is_returned_as_empty_value_not_no_content() {
    let harness = spawn_harness(
        config(),
        menu_fixture().with_resource_response(README_CAPTION, MouseResponse::Script(String::new())),
        QuitPolicy::PerTalk(vec![true]),
    );

    let (answers, recorded) = run_query(harness, MENU_IDS.to_vec(), QueryAt::Steady);

    assert_same_order_and_length(&answers);
    let outcome = outcome_for(&answers, README_CAPTION);
    assert_eq!(
        outcome,
        &ResourceOutcome::Value(String::new()),
        "空を返す応答は空文字を載せた Value になるはず（要件 3.3）: {answers:?}"
    );
    assert_ne!(
        outcome,
        &ResourceOutcome::NoContent,
        "空を返す応答と値なしの応答は別の語彙であるはず（要件 3.3／3.4）: {answers:?}"
    );
    assert!(
        menu_gets(&recorded).contains(&README_CAPTION),
        "空を返す場合も GET は出るはず: {recorded:?}"
    );
}

// ============================================================================
// ⑶ 値なしを返す → `ResourceOutcome::NoContent`
// ============================================================================

/// 項目名リソースが値なし（204）を返したら `NoContent` として返る（要件 3.4）。
///
/// # 非空虚性
/// 未注入の 2 名も同じ 204 経路を通るため、3 名すべてが `NoContent` で揃うことを見る。
/// GET が記録に現れることで「送らずに既定へ倒した」のではないことを区別する。
#[test]
fn absent_caption_is_returned_as_no_content() {
    let harness = spawn_harness(
        config(),
        menu_fixture().with_resource_response(README_CAPTION, MouseResponse::NoContent),
        QuitPolicy::PerTalk(vec![true]),
    );

    let (answers, recorded) = run_query(harness, MENU_IDS.to_vec(), QueryAt::Steady);

    assert_same_order_and_length(&answers);
    for (id, outcome) in &answers {
        assert_eq!(
            outcome,
            &ResourceOutcome::NoContent,
            "値なしの応答は NoContent になるはず（要件 3.4）: {id} → {outcome:?}"
        );
    }
    assert_eq!(
        menu_gets(&recorded),
        MENU_IDS.to_vec(),
        "値なしでも GET そのものは 3 名に出るはず（送らずに倒したのではない）: {recorded:?}"
    );
}

// ============================================================================
// ⑷ 失敗する → `ResourceOutcome::Failed(理由)`・残りの id は答える
// ============================================================================

/// 項目名リソースの照会が失敗したら、その id だけが理由付きの `Failed` になり、
/// 残りの id は通常どおり答える（要件 3.4・設計 `actor_resources` の「檻は 1 か所のまま」）。
///
/// # 非空虚性
/// 失敗を `NoContent` へ潰す後退（失敗と値なしの取り違え）はここで落ちる。1 件の失敗が
/// 照会全体を巻き込まないこと（残り 2 名が答える）も同時に見る。
#[test]
fn failing_caption_is_returned_as_failed_and_the_rest_still_answer() {
    let harness = spawn_harness_failing(
        config(),
        menu_fixture(),
        QuitPolicy::PerTalk(vec![true]),
        FailOn {
            id: README_CAPTION,
            kind: FailKind::Ipc,
        },
    );

    let (answers, recorded) = run_query(harness, MENU_IDS.to_vec(), QueryAt::Steady);

    assert_same_order_and_length(&answers);
    match outcome_for(&answers, README_CAPTION) {
        ResourceOutcome::Failed(reason) => assert!(
            reason.contains("ipc"),
            "Failed には失敗の理由が載るはず: {reason}"
        ),
        other => panic!("失敗した照会は Failed になるはず（要件 3.4）: {other:?}"),
    }
    for id in ["sakura.popupmenu.visible", "closebutton.caption"] {
        assert_eq!(
            outcome_for(&answers, id),
            &ResourceOutcome::NoContent,
            "1 件の失敗は残りの id を巻き込まないはず: {answers:?}"
        );
    }
    assert_eq!(
        menu_gets(&recorded),
        MENU_IDS.to_vec(),
        "失敗した id も GET そのものは出ている: {recorded:?}"
    );
}

// ============================================================================
// ⑸ 会話できる状態になる前の照会 → SHIORI へ送らず全件「値なし」
// ============================================================================

/// `Boot` より前（`Phase::Idle`）に届いた照会は、SHIORI へ 1 通も送らずに全件 `NoContent` で
/// 返る（要件 3.10・受け手は既定名で出す）。
///
/// # 決定性
/// 単一 inbox の FIFO により、`Boot` より前に投函した照会は必ず `Idle` で処理される。
/// その後 `Boot` が boot 系列を同期完走し、close talk（quit:true）で終了系列が回る。
///
/// # 非空虚性（discriminative）
/// fixture は `readmebutton.caption` に値を注入してある。もし殻がこの phase でも SHIORI へ
/// 問い合わせていれば、その id は `Value` になって `NoContent` の期待が落ちる。加えて、
/// 3 名の GET が記録列のどこにも現れないことを直接見る（起動系列の固定の呼出順へ照会が
/// 割り込んでいない）。
#[test]
fn query_before_boot_returns_no_content_for_every_id() {
    let harness = spawn_harness(
        config(),
        menu_fixture().with_resource_response(
            README_CAPTION,
            MouseResponse::Script("問い合わせた".to_string()),
        ),
        QuitPolicy::PerTalk(vec![true]),
    );

    let (answers, recorded) = run_query(harness, MENU_IDS.to_vec(), QueryAt::BeforeBoot);

    assert_same_order_and_length(&answers);
    for (id, outcome) in &answers {
        assert_eq!(
            outcome,
            &ResourceOutcome::NoContent,
            "会話できる状態でない照会は全件 NoContent で返るはず（要件 3.10）: {id} → {outcome:?}"
        );
    }
    assert!(
        menu_gets(&recorded).is_empty(),
        "会話できる状態でない照会は SHIORI へ 1 通も送らないはず（起動系列へ割り込まない）: {recorded:?}"
    );
}
