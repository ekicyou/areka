use super::test_support::{
    FIXED_CHOICE_SCRIPT, NAMED_CHOICE_ID, choice_get_ids, choice_input, command_tags,
    establish_choice_wait,
};
use super::{
    ChoiceResponse, CloseReason, DEFAULT_TIMEOUT, FIXED_STEADY_SCRIPT, Fixture, Harness,
    KanadeConfig, KanadeMsg, QuitPolicy, TalkCommand, TalkId, expected_unload, join_bounded,
    spawn_harness_gated,
};

/// 選択待ちの候補集合に**含めない** ID（群 6 の候補照合の観測面・Req1.4）。
///
/// 正典形（`On` 始まりでない）を選ぶ——受理されてしまう退行では `OnChoiceSelectEx` が発行され、
/// [`choice_get_ids`] の突合で直ちに落ちる（任意名形だと GET の id が檻の抽出述語から外れ、
/// 退行が段列の比較をすり抜ける）。
const OUT_OF_CANDIDATE_CHOICE_ID: &str = "候補外選択肢";

// ============================================================================
// 群 6: Req9.2(e) — 一回性と棄却（統合経路でしか観測できない分）
// ============================================================================
//
// # 本群が担う範囲と、担わない範囲（設計 Testing Strategy 「(e) 補足（in-flight 分岐の檻方式）」）
// カスケードは kanade drive ループの同期往復で **1 メッセージ処理内に完結する**。したがって
// `ChoicePhase::Cascading`／`TimeoutInFlight` の最中に別の選択確定を**注入して**割り込ませることは
// 統合経路では構造上できない——段の進行中の二重確定棄却（`choice_rejected_busy`）と 1 世代 stale
// 防御（`talk_done_stale_choice`）は、状態を直接構成する `step()` 直呼びの純関数檻が担当する
// （`schedule::steady::tests::choice_during_cascade_or_timeout_is_rejected_as_busy`・
// `schedule::tests::stale_choice_talk_done_is_demoted_to_info_and_keeps_state`。ログ語彙は
// `schedule::log_firing_tests` が固定する）。
//
// 本群が担うのは、逆に**純関数檻では観測できない**面である: 実際の drive ループ・実際の
// `TalkCommand` チャンネル・実際の mock SHIORI 往復を通した end-to-end の件数（1 注入が発行した
// GET・`ResolveChoice`・`StartTalk` の実数）と、解決で帳簿が消えた**後**に到着した注入が
// 配送経路のどこからも副作用を出さないこと。
//
// # 決定性（Req9.1）——注入順序の確定と、記録読み出しの同期
// 決定性は**二つ**の面で要る。片方だけでは非決定が残る。
//
// **(i) 注入順序**: 群 1〜5 と違い、本群は `hold_indices=[0, 1]`（steady talk と選択由来 talk の
// **両方**の `TalkDone` を park）で駆動する。mock sakura が自発的に kanade inbox へ送るメッセージは
// `TalkDone` だけなので、両方を park すれば注入列の途中に非同期メッセージが一切割り込まない
// ——「遅延注入は必ずカスケード解決の後に処理される」ことが mpsc の FIFO だけで確定する
// （即応 sink だと選択由来 talk の `TalkDone` が遅延注入を追い越し得て、棄却が
// `choice_rejected_no_wait` ではなく終了系列の入力無視で起きたのか区別できなくなる）。
//
// **(ii) 記録の読み出し**: kanade の停止は `TalkCommand` を mock が**消費し終えた**ことを意味しない。
// 本群は `ForceQuit` で終了するため（下記）、`TalkDone` の往復という暗黙の同期が働かず、
// `MockSakura::commands()` の並行読みでは記録列が空のまま読まれ得る（実測: 群 6 の 2 檻を
// 100 回実行して 11 回、退行と無関係に `left: []` で FAILED した）。したがって本群は
// [`MockSakura::join_bounded_then_commands`]（recv ループの join 完了後に読む変種）を使う。
// 群 1〜5 は quit フラグ付き `TalkDone` の往復が mock の消費を強制するため `commands()` のままでよい。
//
// # 終了の駆動は `ForceQuit`——**檻の弁別力を退行時にも保つため**
// 群 1〜5 は「末尾 talk の quit:true」で終了を駆動するが、本群はそれを採らない。棄却が壊れる退行では
// **起動される talk の本数そのものが変わる**ため、`QuitPolicy::PerTalk` の index と quit フラグの
// 対応がずれて終了が駆動されず、檻が「期限付き join の失敗」（＝どの表明が破れたか分からない形）で
// しか落ちなくなる（実測で確認済み）。`ForceQuit` は quit ゲートを迂回して終了系列へ直行する
// （**close spec の DD-10**＝本 spec の DD-10（`ActiveTalk.script`→`OnChoiceTimeout` Ref0）とは
// 別物・`close_test.rs` の既存イディオム）ので、talk の本数に関わらず必ず join が成功し、
// **段列と `TalkCommand` の突合そのものが退行を名指しする**。ただしこの「名指しする」性質は
// 上記 (ii) の読み出し同期が前提である——同期を欠くと突合は退行と無関係な空列を報告し得る。
//
// 保留 talk は kanade 停止**後**に `release_all` で解放する。送り先の kanade は既に居らず送出は
// no-op で、観測対象の運行には影響しない——停止**前**に解放すると park した `TalkDone` が注入列へ
// 割り込み (i) が壊れる。`spawn_mock_sakura_gated` の「Sender drop より前に呼ぶ」契約は kanade 停止
// 後には timing 要件としての意味を失っており（実際の解放は `recv_closed` 安全弁が担う）、本檻は
// 決定性を優先して停止後に呼ぶ。

/// 1 回の選択確定が **高々 1 カスケード・高々 1 選択解決・高々 1 起動要求**しか起こさず、解決後に
/// 到着した遅延注入が何も起こさない（Req9.2(e)・1.1・1.3・4.6・5.4・C4 規則 1／7）。
///
/// # 駆動（決定的・sleep なし）
/// active talk 窓（id=1・保留）で `ChoiceWaiting{candidates:[NAMED]}` を確立し、同じ ID を **3 回**
/// 注入する。1 本目だけが受理されて任意名 1 段のカスケード（Value）→ `ResolveChoice`→`Start(2)` を
/// 起こし、2・3 本目は帳簿が消えた後に到着する遅延通知として棄却される。選択由来 talk（id=2）も
/// park するため、この 3 本は必ずこの順で連続処理される。終了は `ForceQuit` で駆動する。
///
/// # 非空虚性・discriminative 性
/// - 選択由来 GET が **ちょうど 1 件**であること。遅延注入を受理する退行（帳簿の消去漏れ・
///   `choice_rejected_no_wait` アームの欠落）では 2 件目・3 件目の任意名 GET が現れて落ちる。
///   実測: `on_cascade_reply` の Value 分岐で帳簿を消さず新 talk_id へ引き継ぐ変異を入れると、
///   段列が `[NAMED, NAMED, NAMED]` になって本表明が落ちる（もう一方の群 6 檻は緑のまま）。
/// - `TalkCommand` 到着順が `Start(1)`→`Resolve(1,NAMED)`→`Start(2)` であること。`ResolveChoice`
///   が 2 件以上になる（解決の重複発行・Req5.4 の破れ）／`CancelChoice` が混じる／選択由来
///   `StartTalk` が 2 本になる（Req4.6 の破れ）のいずれの退行でもタグ列が食い違って落ちる。
/// - 到達 `StartTalk` が steady と選択由来の 2 本ちょうどであること＝遅延注入が追加の talk を
///   起こしていないこと。
#[test]
fn one_choice_injection_yields_a_single_cascade_and_later_injections_are_rejected() {
    // 任意名 GET は Value（1 段で短絡）。終了は ForceQuit ゆえ close 応答の仕込みは要らない。
    let fixture = Fixture::default()
        .without_boot_greeting()
        .with_steady_value_indices([0])
        .with_choice_response(
            NAMED_CHOICE_ID,
            ChoiceResponse::Script(FIXED_CHOICE_SCRIPT.to_string()),
        );

    // hold_indices=[0, 1]: steady talk（id=1）と選択由来 talk（id=2）の TalkDone を両方 park。
    // quit フラグは終了駆動に使わない（ForceQuit が迂回する）ため一律 false。
    let (harness, gate) = spawn_harness_gated(
        KanadeConfig::new("master", "1.0.0"),
        fixture,
        QuitPolicy::Fixed(false),
        vec![0, 1],
    );

    establish_choice_wait(&harness.sender, &[NAMED_CHOICE_ID]);
    // 1 本目——受理される唯一の注入。
    harness
        .sender
        .send(KanadeMsg::Choice(choice_input(NAMED_CHOICE_ID)))
        .expect("send Choice (accepted)");
    // 2・3 本目——解決で帳簿が消えた後に届く遅延通知（いずれも棄却されるはず・Req1.3）。
    for nth in ["send Choice (late #1)", "send Choice (late #2)"] {
        harness
            .sender
            .send(KanadeMsg::Choice(choice_input(NAMED_CHOICE_ID)))
            .expect(nth);
    }
    harness
        .sender
        .send(KanadeMsg::ForceQuit {
            reason: CloseReason::User,
        })
        .expect("send ForceQuit");

    let Harness {
        sender,
        kanade,
        shiori,
        sakura,
    } = harness;

    join_bounded("kanade choice-once join", DEFAULT_TIMEOUT, kanade)
        .expect("kanade terminates directly on ForceQuit (close spec DD-10)");
    drop(sender);
    // 保留 talk の解放は kanade 停止**後**に行う（観測対象の運行には影響しない後始末——
    // 送り先が既に居ないため送出は no-op である）。解放を停止前に置くと park していた
    // `TalkDone` が注入列へ割り込み、群 6 の前提（非同期メッセージを走らせない）が壊れる。
    gate.release_all();
    // 記録は **join 完了後**に読む——`ForceQuit` 終了は mock sakura を経由しないため、
    // `commands()` の並行読みでは記録前のスナップショットを掴み得る（群 6 ヘッダ参照）。
    let commands =
        sakura.join_bounded_then_commands("mock-sakura choice-once join", DEFAULT_TIMEOUT);
    let recorded = shiori.recorded();

    // (1) カスケードは高々 1 回（3 注入でも段列は任意名 1 件のみ・Req1.1）。
    assert_eq!(
        choice_get_ids(&recorded),
        vec![NAMED_CHOICE_ID],
        "3 回の同一注入でカスケードは 1 回だけ走るはず（解決後の遅延注入は棄却）: {recorded:?}"
    );

    // (2) 解決も起動も高々 1 つ（順序込み・Req5.4／4.6）。
    assert_eq!(
        command_tags(&commands),
        vec![
            "Start(1)".to_string(),
            format!("Resolve(1,{NAMED_CHOICE_ID})"),
            "Start(2)".to_string(),
        ],
        "選択解決は 1 件・選択由来の起動は 1 本のみのはず: {commands:?}"
    );

    // (3) 到達 StartTalk の内訳——選択由来は 2 本目ただ 1 本（遅延注入は talk を起こさない）。
    let started: Vec<_> = commands
        .iter()
        .filter_map(|c| match c {
            TalkCommand::Start(s) => Some(s),
            _ => None,
        })
        .collect();
    assert_eq!(
        started.len(),
        2,
        "起動は steady と選択由来の 2 本ちょうどのはず: {started:?}"
    );
    assert_eq!(
        started[0].script, FIXED_STEADY_SCRIPT,
        "1 本目は steady talk"
    );
    assert_eq!(
        started[1].script, FIXED_CHOICE_SCRIPT,
        "2 本目は選択由来の応答スクリプト"
    );
    assert_eq!(
        started[1].talk_id,
        TalkId(2),
        "選択由来 talk は新 talk_id=2（旧 id=1 を再利用しない）"
    );

    // (4) 終了系列を完走した（末尾 Unload）＝遅延注入の棄却後も状態整合を保っている。
    assert_eq!(
        recorded.last().expect("記録列は空でない"),
        &expected_unload(),
        "末尾は Unload（ForceQuit→終了系列直行・close spec の DD-10）"
    );
}

/// 候補集合に無い ID の注入は何も起こさず、**選択待ちを閉じない**（Req9.2(e) 後段・1.4・C4 規則 1）。
///
/// # 駆動（決定的・sleep なし）
/// [`one_choice_injection_yields_a_single_cascade_and_later_injections_are_rejected`] と同じ保留構成で、
/// 候補集合 `[NAMED]` に対し **候補外 ID → 候補内 ID** の順に注入する。
///
/// # 非空虚性・discriminative 性
/// - 候補外 ID の注入が **段を 1 つも発行しない**こと。受理する退行では正典形カスケードの
///   `OnChoiceSelectEx` が段列の先頭に現れて落ちる（そのために候補外 ID は正典形を選んである）。
///   加えて記録列全体に当該 ID の呼び出しが無いことも直接表明する。実測: `on_choice` の候補照合を
///   無効化する変異を入れると段列が `[OnChoiceSelectEx, OnChoiceSelect]` になって落ちる
///   （もう一方の群 6 檻は緑のまま）。
/// - 続く**候補内 ID が正常に受理される**こと＝棄却が選択待ちを閉じていない（Req1.4「選択待ち状態を
///   変更しない」の観測面）。棄却のついでに帳簿を落とす退行では段列が空になり、`Resolve` も消えて落ちる。
///   件数を数えるだけの檻ではこの「閉じてしまう」退行を弁別できない——後続注入の成功が要る。
#[test]
fn choice_outside_the_candidate_set_is_rejected_and_keeps_the_wait_open() {
    let fixture = Fixture::default()
        .without_boot_greeting()
        .with_steady_value_indices([0])
        .with_choice_response(
            NAMED_CHOICE_ID,
            ChoiceResponse::Script(FIXED_CHOICE_SCRIPT.to_string()),
        );

    // 保留構成・終了駆動は群 6 共通（hold_indices=[0, 1]＋ForceQuit）。
    let (harness, gate) = spawn_harness_gated(
        KanadeConfig::new("master", "1.0.0"),
        fixture,
        QuitPolicy::Fixed(false),
        vec![0, 1],
    );

    establish_choice_wait(&harness.sender, &[NAMED_CHOICE_ID]);
    // (1) 候補外 ID——棄却されるはず（Req1.4）。
    harness
        .sender
        .send(KanadeMsg::Choice(choice_input(OUT_OF_CANDIDATE_CHOICE_ID)))
        .expect("send Choice (out of candidates)");
    // (2) 候補内 ID——(1) が選択待ちを閉じていなければ受理されるはず。
    harness
        .sender
        .send(KanadeMsg::Choice(choice_input(NAMED_CHOICE_ID)))
        .expect("send Choice (in candidates)");
    harness
        .sender
        .send(KanadeMsg::ForceQuit {
            reason: CloseReason::User,
        })
        .expect("send ForceQuit");

    let Harness {
        sender,
        kanade,
        shiori,
        sakura,
    } = harness;

    join_bounded("kanade unknown-id join", DEFAULT_TIMEOUT, kanade)
        .expect("kanade terminates directly on ForceQuit (close spec DD-10)");
    drop(sender);
    // 後始末・記録の読み出し同期は群 6 共通（もう一方の檻と同一の手順）。
    gate.release_all();
    let commands =
        sakura.join_bounded_then_commands("mock-sakura unknown-id join", DEFAULT_TIMEOUT);
    let recorded = shiori.recorded();

    // (1) 候補外 ID は段を 1 つも発行していない（段列は候補内 ID の 1 件のみ）。
    assert_eq!(
        choice_get_ids(&recorded),
        vec![NAMED_CHOICE_ID],
        "候補外 ID はカスケードを起こさず、候補内 ID のみが 1 段発行するはず: {recorded:?}"
    );
    assert!(
        !recorded.iter().any(|c| c.id == OUT_OF_CANDIDATE_CHOICE_ID),
        "候補外 ID を任意名イベントとして発火する退行はここで落ちる: {recorded:?}"
    );

    // (2) 棄却は選択待ちを閉じていない——後続の候補内 ID が解決・起動まで到達している（Req1.4）。
    assert_eq!(
        command_tags(&commands),
        vec![
            "Start(1)".to_string(),
            format!("Resolve(1,{NAMED_CHOICE_ID})"),
            "Start(2)".to_string(),
        ],
        "候補外 ID の棄却後も選択待ちは生きており、候補内 ID が解決と起動を起こすはず: {commands:?}"
    );

    // (3) 終了系列を完走した（末尾 Unload）。
    assert_eq!(
        recorded.last().expect("記録列は空でない"),
        &expected_unload(),
        "末尾は Unload（ForceQuit→終了系列直行・close spec の DD-10）"
    );
}
