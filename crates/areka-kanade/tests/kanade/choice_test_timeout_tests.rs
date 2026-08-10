use super::test_support::{
    CANONICAL_CHOICE_ID, CHOICE_DEADLINE_MS, FIXED_CHOICE_SCRIPT, cascading_snapshot,
    choice_get_ids, choice_gets, choice_input, command_tags, establish_choice_wait, position_of,
    pumps,
};
use super::{
    CallMethod, ChoiceResponse, CloseReason, DEFAULT_TIMEOUT, ExecutionSnapshot,
    FIXED_STEADY_SCRIPT, Fixture, Harness, KanadeConfig, KanadeMsg, MonotonicMs, QuitPolicy,
    TalkCommand, TalkDone, TalkEndReason, TalkId, events, expected_call, expected_unload,
    join_bounded, spawn_harness_gated,
};

// ============================================================================
// 群 5: Req9.2(d) — タイムアウト（期限到達・204 解除・以降棄却／Value 置換再生）
// ============================================================================

/// 注入 Tick のみで期限へ到達し、`OnChoiceTimeout`（Ref0=起動スクリプト）GET が発行され、204 で
/// `TalkCommand::CancelChoice` が届き、`TalkDone{Interrupted}` の注入で `Steady{None}` へ復帰し、
/// **解除後に到着する `Choice` が棄却される**（Req9.2(d)・7.3／7.5・C4 規則 5／6・DD-11・F3）。
///
/// # 駆動（決定的・sleep なし・実時刻を読まない）
/// active talk 窓（id=1・保留）で `ChoiceWaiting` を確立すると期限は [`CHOICE_DEADLINE_MS`] になる。
/// 注入する Tick は次の 4 本だけで、いずれも `now` は檻が与える論理値である:
///
/// 1. `Tick(CHOICE_DISPLAY_END_MS)`（[`establish_choice_wait`] 内）: steady talk を起こす。
/// 2. `Tick(CHOICE_DEADLINE_MS - 1_000)`: **期限手前**——タイムアウトは発火せず通常 pump（NOTIFY）。
/// 3. `Tick(CHOICE_DEADLINE_MS)`: **期限到達**——`OnChoiceTimeout` GET を発行し、この周期は pump を
///    発行しない（C4 規則 5）。mock は当該 id へ未注入ゆえ 204 を返し、`CancelChoice` が発行される。
/// 4. `Tick(CHOICE_DEADLINE_MS + 1_000)`: `TalkDone{Interrupted}` 注入後の pump——`Steady{None}` へ
///    復帰していれば **GET**（Ref3="1"・Status ヘッダなし）になる。
///
/// mock sakura は再生層を持たず `CancelChoice` を記録するだけなので、Close funnel の帰結である
/// `TalkDone{Interrupted}` は檻が直接注入する（設計 Testing Strategy (d) の明文どおり）。終了は
/// `Steady{None}` 復帰後の `CloseRequest`（→別れ talk quit:true）で駆動する。保留は解放しない。
///
/// # 非空虚性・discriminative 性
/// - **期限の両側**を注入して既定値 [`CHOICE_TIMEOUT_DEFAULT_MS`] そのものを固定する:
///   手前（2）では発火せず `talking,choosing` の NOTIFY pump が出る・到達（3）で初めて発火する。
///   既定値が大きくなれば (3) でも発火せず `OnChoiceTimeout` が現れない。小さくなれば (2) で
///   発火し、**記録順が `OnChoiceTimeout` → NOTIFY pump へ逆転**したうえ (2) の pump から
///   `choosing` が消えるため、順序突合と Status 突合の両方で落ちる（**両方向**を弁別する。
///   単に「段列と pump 件数」を数えるだけでは早期発火を見逃す——実測で確認済み）。
/// - `OnChoiceTimeout` GET を events 表導出と完全一致で突合し、Ref0 が **当該トークの起動
///   スクリプト**（`FIXED_STEADY_SCRIPT`＝ActiveTalk.script・DD-10）であることを実値で確認する。
///   Ref0 に選択肢 ID やラベルを載せる退行はここで落ちる。
/// - `OnSecondChange` の総数が 3 件（GET 2・NOTIFY 1）であること＝**発火した周期は pump を出して
///   いない**こと（C4 規則 5）。pump を止め損ねる退行は 4 件になって落ちる。
/// - `TalkCommand` 到着順が `Start(1)`→`Cancel(1)`→`Start(2)` であること。`ResolveChoice` が混じる
///   （タイムアウトを解決扱いする退行）／`Cancel` が落ちる（Req7.5 の破れ）のいずれでも落ちる。
/// - 復帰後の pump が `Steady{None}` の GET（Ref3="1"・Status ヘッダなし）であること＝
///   `TalkDone{Interrupted}` が非 quit 扱いで定常復帰する正規到達点（DD-11）を通ったこと。
/// - 選択由来 GET が `OnChoiceTimeout` **ただ 1 件**であること＝解除後に注入した `Choice` 2 本が
///   いずれもカスケードを起こしていない（棄却された）こと。棄却が漏れれば Ex／無印が現れて落ちる。
///   2 本の注入点は意図的に分けてある——1 本目は**解除直後（現行トークはまだ active）**であり
///   解除側の帳簿消去だけで棄却されねばならず、2 本目は `Steady{None}` 復帰後の遅延通知である。
///   1 本目が無いと、解除で帳簿を消し損ねる退行を `TalkDone{Interrupted}` の掃除（C4 規則 7）が
///   覆い隠して素通りする（実測で確認済み）。
#[test]
fn choice_timeout_fires_then_204_cancels_and_rejects_later_choice() {
    // quitting: 復帰後の driven close が別れ talk（Value）を返す。OnChoiceTimeout は未注入＝204。
    let fixture = Fixture::quitting()
        .without_boot_greeting()
        .with_steady_value_indices([0]);

    // hold_indices=[0]: steady talk（id=1）の TalkDone を park（解放しない——終了は driven close）。
    // quit_flags: index0（steady talk）=false・index1（close talk）=true（終了駆動）。
    let (harness, _gate) = spawn_harness_gated(
        KanadeConfig::new("master", "1.0.0"),
        fixture,
        QuitPolicy::PerTalk(vec![false, true]),
        vec![0],
    );

    establish_choice_wait(&harness.sender, &[CANONICAL_CHOICE_ID]);

    // (2) 期限手前の Tick——発火しない（既定値の下限を固定する）。
    harness
        .sender
        .send(KanadeMsg::Tick {
            now: MonotonicMs(CHOICE_DEADLINE_MS - 1_000),
        })
        .expect("send Tick just before the deadline");
    // (3) 期限到達の Tick——OnChoiceTimeout GET を発行し pump は出さない。
    harness
        .sender
        .send(KanadeMsg::Tick {
            now: MonotonicMs(CHOICE_DEADLINE_MS),
        })
        .expect("send Tick at the deadline");
    // 解除直後（現行トークはまだ active）の選択確定——解除側の掃除点だけで棄却されるはず（Req7.5）。
    // この 1 本目は `TalkDone{Interrupted}` の掃除（C4 規則 7）に**先行する**ため、解除で帳簿を
    // 消し損ねる退行を単独で弁別する（後述の 2 本目だけでは 2 つの掃除点が互いを覆い隠す）。
    harness
        .sender
        .send(KanadeMsg::Choice(choice_input(CANONICAL_CHOICE_ID)))
        .expect("send Choice right after cancellation (talk still active)");
    // Close funnel の帰結（dispatcher が Close を転送し talk が返す通知）を檻が直接注入する。
    harness
        .sender
        .send(KanadeMsg::TalkDone(TalkDone {
            talk_id: TalkId(1),
            reason: TalkEndReason::Interrupted,
        }))
        .expect("send TalkDone{Interrupted} (Close funnel の完了通知)");
    // (4) 復帰後の pump——Steady{None} なら GET（Ref3=1）になる。
    harness
        .sender
        .send(KanadeMsg::Tick {
            now: MonotonicMs(CHOICE_DEADLINE_MS + 1_000),
        })
        .expect("send Tick after the interrupted recovery");
    // 2 本目の選択確定——`Steady{None}` 復帰後に到着する遅延通知（同じく棄却されるはず・Req7.5）。
    harness
        .sender
        .send(KanadeMsg::Choice(choice_input(CANONICAL_CHOICE_ID)))
        .expect("send Choice after the interrupted recovery");
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

    join_bounded("kanade choice-timeout join", DEFAULT_TIMEOUT, kanade)
        .expect("kanade terminates via the driven close after the interrupted recovery");
    drop(sender);
    let commands = sakura.commands();
    sakura.join_bounded("mock-sakura choice-timeout join", DEFAULT_TIMEOUT);
    let recorded = shiori.recorded();

    // (1) 選択由来 GET は OnChoiceTimeout ただ 1 件（期限手前で非発火・解除後の Choice は棄却）。
    assert_eq!(
        choice_get_ids(&recorded),
        vec!["OnChoiceTimeout"],
        "期限到達でのみタイムアウトが発火し、解除後の Choice はカスケードを起こさないはず: {recorded:?}"
    );

    // (2) OnChoiceTimeout の layout（Ref0=当該トークの起動スクリプト・Status は複合値・Req3.4／DD-10）。
    let expected_timeout = expected_call(events::on_choice_timeout(
        FIXED_STEADY_SCRIPT,
        &cascading_snapshot(),
    ));
    let timeout_get = choice_gets(&recorded);
    assert_eq!(
        *timeout_get[0], expected_timeout,
        "OnChoiceTimeout は events 表導出（Ref0=起動スクリプト・Status）と一致するはず: {recorded:?}"
    );
    assert_eq!(
        timeout_get[0].references,
        vec![FIXED_STEADY_SCRIPT.to_string()],
        "Ref0 は当該トークの起動スクリプトそのもの（ActiveTalk.script・DD-10）"
    );
    assert_eq!(
        timeout_get[0].status,
        Some("talking,choosing".to_string()),
        "タイムアウト GET も選択待ち継続中（TimeoutInFlight）＝複合 Status を帯びる（裁定 6）"
    );

    // (3) 発火した周期は pump を出していない（C4 規則 5）——OnSecondChange は GET 2・NOTIFY 1 の計 3 件。
    let pump_calls = pumps(&recorded);
    let pump_methods: Vec<&CallMethod> = pump_calls.iter().map(|c| &c.method).collect();
    assert_eq!(
        pump_methods,
        vec![
            &CallMethod::Get,    // Tick(display_end): Steady{None} の pump 問い合わせ
            &CallMethod::Notify, // 期限手前の Tick: active talk 窓の NOTIFY
            &CallMethod::Get,    // 復帰後の Tick: Steady{None} の pump 問い合わせ
        ],
        "期限到達の Tick は pump を発行しない（規則 5）＝周期リクエストは 3 件のみのはず: {recorded:?}"
    );

    // (3') 既定値そのものの固定（Req7.8）: 期限**手前**の Tick では発火していない。
    //      観測面は 2 点——(i) その pump がまだ `talking,choosing` を帯びる（帳簿が Waiting のまま）、
    //      (ii) 記録順が [手前の NOTIFY pump] → [OnChoiceTimeout] である。既定値が
    //      CHOICE_TIMEOUT_DEFAULT_MS より小さければ手前の Tick で発火し、この 2 点が両方とも破れる。
    assert_eq!(
        pump_calls[1].status,
        Some("talking,choosing".to_string()),
        "期限手前の pump はまだ選択待ち継続中＝複合 Status を帯びるはず（既定値が短ければ落ちる）: {recorded:?}"
    );
    let before_deadline_pump_pos = recorded
        .iter()
        .position(|c| *c == *pump_calls[1])
        .expect("期限手前の NOTIFY pump が記録されているはず");
    let timeout_get_pos = position_of(&recorded, CallMethod::Get, "OnChoiceTimeout");
    assert!(
        before_deadline_pump_pos < timeout_get_pos,
        "タイムアウトは期限手前の Tick より後（期限到達の Tick）で初めて発火するはず: {recorded:?}"
    );

    // (4) 204 で選択待ちが解除された（Cancel が届き、Resolve は混じらない・Req7.5・F3）。
    assert_eq!(
        command_tags(&commands),
        vec![
            "Start(1)".to_string(),
            "Cancel(1)".to_string(),
            "Start(2)".to_string(),
        ],
        "タイムアウト 204 は解決ではなく解除指示を出すはず（その後の起動は close talk のみ）: {commands:?}"
    );

    // (5) `TalkDone{Interrupted}` で `Steady{None}` へ復帰した——復帰後の pump が GET（Ref3=1・
    //     Status ヘッダなし）であることが観測面（DD-11 の正規到達点）。
    let expected_resumed = expected_call(events::on_second_change(
        MonotonicMs(CHOICE_DEADLINE_MS + 1_000),
        &ExecutionSnapshot::INACTIVE,
    ));
    assert_eq!(
        *pump_calls[2], expected_resumed,
        "解除後の pump は Steady{{None}} の GET（Ref3=1・Status ヘッダなし）のはず: {recorded:?}"
    );
    assert_eq!(
        pump_calls[2].references[3], "1",
        "Steady{{None}} 復帰の Ref3 は \"1\"（再生可能＝応答スクリプトを再生できる）"
    );
    assert_eq!(
        pump_calls[2].status, None,
        "復帰後は実行状態が空＝Status ヘッダ行そのものが省略される（Req6.3 の既存規律）"
    );

    // (6) 終了は driven close 由来（末尾 Unload・タイムアウトはゴーストを終了させない）。
    let close_pos = position_of(&recorded, CallMethod::Get, "OnClose");
    assert!(
        timeout_get_pos < close_pos,
        "close 握手はタイムアウト解除の後に走るはず（順序の退行検出）: {recorded:?}"
    );
    assert_eq!(
        recorded.last().expect("記録列は空でない"),
        &expected_unload(),
        "末尾は Unload（driven close→別れ talk quit:true 由来）"
    );
}

/// タイムアウト応答が Value なら **既存の起動経路**で置換再生される（新 talk_id・Req9.2(d) 後段・
/// 7.4・C4 規則 6・F3）。
///
/// # 非空虚性・discriminative 性
/// [`choice_timeout_fires_then_204_cancels_and_rejects_later_choice`] と**同一の期限・同一の注入
/// Tick** で応答だけを Value に替えた対の檻である。
/// - `TalkCommand` 到着順が `Start(1)`→`Start(2)` であること＝置換起動は解決指示（`Resolve`）も
///   解除指示（`Cancel`）も伴わない（F3 の Value 分岐）。どちらかが混じれば落ちる。
/// - 起動された talk が **新 talk_id=2**・**タイムアウト応答のスクリプト**であること＝旧 id の
///   再利用や応答の取り違えを検出する。
/// - 選択由来 GET が `OnChoiceTimeout` 1 件のみであること＝Value 応答が更なる段を撃たないこと。
#[test]
fn choice_timeout_value_replaces_talk_via_existing_start_path() {
    let fixture = Fixture::default()
        .without_boot_greeting()
        .with_steady_value_indices([0])
        .with_choice_response(
            "OnChoiceTimeout",
            ChoiceResponse::Script(FIXED_CHOICE_SCRIPT.to_string()),
        );

    // quit_flags: index0（steady talk・保留）=false・index1（タイムアウト由来 talk）=true（終了駆動）。
    let (harness, _gate) = spawn_harness_gated(
        KanadeConfig::new("master", "1.0.0"),
        fixture,
        QuitPolicy::PerTalk(vec![false, true]),
        vec![0],
    );

    establish_choice_wait(&harness.sender, &[CANONICAL_CHOICE_ID]);
    harness
        .sender
        .send(KanadeMsg::Tick {
            now: MonotonicMs(CHOICE_DEADLINE_MS),
        })
        .expect("send Tick at the deadline");

    let Harness {
        sender,
        kanade,
        shiori,
        sakura,
    } = harness;

    join_bounded("kanade timeout-value join", DEFAULT_TIMEOUT, kanade)
        .expect("kanade terminates via the timeout-derived talk (quit:true)");
    drop(sender);
    let commands = sakura.commands();
    sakura.join_bounded("mock-sakura timeout-value join", DEFAULT_TIMEOUT);
    let recorded = shiori.recorded();

    // (1) 段列はタイムアウト 1 件のみ（Value が更なる段を撃たない）。
    assert_eq!(
        choice_get_ids(&recorded),
        vec!["OnChoiceTimeout"],
        "タイムアウト GET は 1 件のみのはず: {recorded:?}"
    );

    // (2) 置換起動は解決も解除も伴わない（F3 の Value 分岐・Req7.4）。
    assert_eq!(
        command_tags(&commands),
        vec!["Start(1)".to_string(), "Start(2)".to_string()],
        "タイムアウト Value は置換起動のみを起こすはず（Resolve／Cancel を伴わない）: {commands:?}"
    );

    // (3) 起動は新 talk_id・タイムアウト応答のスクリプト（既存の起動棚に載った）。
    let started: Vec<_> = commands
        .iter()
        .filter_map(|c| match c {
            TalkCommand::Start(s) => Some(s),
            _ => None,
        })
        .collect();
    assert_eq!(
        started[0].script, FIXED_STEADY_SCRIPT,
        "1 本目は steady talk"
    );
    assert_eq!(
        started[1].script, FIXED_CHOICE_SCRIPT,
        "2 本目はタイムアウト応答のスクリプト由来の talk"
    );
    assert_eq!(
        started[1].talk_id,
        TalkId(2),
        "置換再生は新 talk_id=2（旧 id=1 を再利用しない・Req4.1）"
    );

    // (4) 終了系列を完走した（末尾 Unload）。
    assert_eq!(
        recorded.last().expect("記録列は空でない"),
        &expected_unload(),
        "末尾は Unload（タイムアウト由来 talk quit:true→Unloading{{Quit}}→Unload）"
    );
}
