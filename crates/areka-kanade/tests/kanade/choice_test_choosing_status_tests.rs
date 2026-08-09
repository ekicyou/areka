use super::test_support::{
    CANONICAL_CHOICE_ID, cascading_snapshot, choice_get_ids, choice_input, establish_choice_wait,
    position_of, pumps,
};
use super::{
    CallMethod, CloseReason, DEFAULT_TIMEOUT, ExecutionSnapshot, Fixture, Harness, KanadeConfig,
    KanadeMsg, MonotonicMs, QuitPolicy, RecordedCall, events, expected_call, expected_unload,
    join_bounded, spawn_harness_gated,
};

/// active talk のみ（選択待ちは終了済み）の運行状態＝`talking` 単独。
fn talking_only_snapshot() -> ExecutionSnapshot {
    ExecutionSnapshot {
        talk_active: true,
        choice_active: false,
    }
}

// ============================================================================
// 群 4: Req9.2(c) — 選択待ち中の周期リクエストに `choosing` が載り、解決後に消える
// ============================================================================

/// 選択待ち確立後の周期リクエストが **NOTIFY**（Ref3="0"）で `Status: talking,choosing` を帯び、
/// 選択解決後の周期リクエストからは `choosing` が消えて `talking` 単独へ戻る
/// （Req9.2(c)・6.1／6.2／6.4・C5・裁定 6）。
///
/// # 駆動（決定的・sleep なし）
/// 挨拶なし boot →`Steady{None}` 直行 → Tick1（`OnSecondChange` GET Value）で steady talk（id=1）を
/// 起こし、保留ハーネス（`hold_indices=[0]`）でその `TalkDone` を park して active talk 窓を保つ。
/// `ChoiceWaiting` を確立してから **Tick2** を注入し（選択待ち中の pump）、`Choice` を注入して
/// 正典形カスケードを 204／204 で走らせ（＝解決のみ・帳簿消去）、続けて **Tick3** を注入する
/// （解決後の pump）。終了は `CloseRequest`＋保留解放（`TalkDone{Ended}`→close 握手→別れ talk
/// quit:true）で駆動する。注入 Tick はいずれも期限 [`CHOICE_DEADLINE_MS`] より手前ゆえ、
/// タイムアウト経路は踏まない。
///
/// # 非空虚性・discriminative 性
/// - 周期リクエストのうち **NOTIFY はちょうど 2 件**（Tick2／Tick3）であり、それぞれの `status` を
///   **wire 実値**（`"talking,choosing"` ／ `"talking"`）で突合する。`choosing` の導出が入って
///   いなければ 1 件目が `"talking"` になり落ち、解決時の帳簿消去が漏れていれば 2 件目が
///   `"talking,choosing"` のまま残って落ちる（両方向の退行を弁別する）。
/// - 連結順序（正典順 `talking,choosing`）も実値突合に含まれる——順序が逆（`choosing,talking`）に
///   なる退行はここで落ちる（Req6.3 の既存規律が複合値でも保たれること）。
/// - 両件が **NOTIFY・Ref3="0"** であること（GET でないこと）を明示確認する＝選択待ち中も slot
///   占有継続として扱われ、応答スクリプトを運べない型で送出される（Req6.4／6.5 の構造充足）。
/// - 記録位置の前後（Tick2 の NOTIFY < カスケード段 GET < Tick3 の NOTIFY）を突合し、2 件目が
///   確かに**解決後**の pump であることを固定する（空虚な合格を防ぐ）。
#[test]
fn choosing_rides_pump_status_while_waiting_then_clears_after_resolution() {
    // quitting: close 握手が別れ talk（Value）を返す。選択由来 GET は未注入＝両段とも 204
    // （＝解決のみ・StartTalk なし）で、解決後の pump を同一 talk 窓のまま観測できる。
    let fixture = Fixture::quitting()
        .without_boot_greeting()
        .with_steady_value_indices([0]);

    // quit_flags: index0（steady talk）=false（Ended→pending_close 消化）・index1（close talk）=true。
    let (harness, gate) = spawn_harness_gated(
        KanadeConfig::new("master", "1.0.0"),
        fixture,
        QuitPolicy::PerTalk(vec![false, true]),
        vec![0],
    );

    establish_choice_wait(&harness.sender, &[CANONICAL_CHOICE_ID]);
    // Tick2: 選択待ち中の周期リクエスト（期限 CHOICE_DEADLINE_MS より手前）。
    harness
        .sender
        .send(KanadeMsg::Tick {
            now: MonotonicMs(2_000),
        })
        .expect("send Tick 2 (choosing)");
    harness
        .sender
        .send(KanadeMsg::Choice(choice_input(CANONICAL_CHOICE_ID)))
        .expect("send Choice");
    // Tick3: 解決後の周期リクエスト（同じ active talk 窓・choosing だけが消えているはず）。
    harness
        .sender
        .send(KanadeMsg::Tick {
            now: MonotonicMs(3_000),
        })
        .expect("send Tick 3 (resolved)");
    harness
        .sender
        .send(KanadeMsg::CloseRequest {
            reason: CloseReason::User,
        })
        .expect("send CloseRequest");
    // 保留解放は全注入の後——保留 TalkDone は CloseRequest の後ろに並ぶ（FIFO・決定的）。
    gate.release_all();

    let Harness {
        sender,
        kanade,
        shiori,
        sakura,
    } = harness;

    join_bounded("kanade choosing-status join", DEFAULT_TIMEOUT, kanade)
        .expect("kanade terminates via the driven close (farewell talk quit:true)");
    drop(sender);
    sakura.join_bounded("mock-sakura choosing-status join", DEFAULT_TIMEOUT);
    let recorded = shiori.recorded();

    // (1) 周期リクエストのうち NOTIFY はちょうど 2 件（Tick2／Tick3）——active talk 窓が両 Tick で
    //     維持されている（GET へ落ちていない）ことの直接確認。
    let notifies: Vec<&RecordedCall> = pumps(&recorded)
        .into_iter()
        .filter(|c| c.method == CallMethod::Notify)
        .collect();
    assert_eq!(
        notifies.len(),
        2,
        "active talk 窓で注入した 2 本の Tick は NOTIFY pump を 1 件ずつ発行するはず: {recorded:?}"
    );

    // (2) 選択待ち中の pump: NOTIFY・Ref3="0"・Status は複合値 `talking,choosing`（Req6.1／6.4・裁定 6）。
    let expected_choosing = expected_call(events::on_second_change(
        MonotonicMs(2_000),
        &cascading_snapshot(),
    ));
    assert_eq!(
        *notifies[0], expected_choosing,
        "選択待ち中の pump は events 表導出（NOTIFY・Ref3=0・複合 Status）と一致するはず: {recorded:?}"
    );
    assert_eq!(
        notifies[0].status,
        Some("talking,choosing".to_string()),
        "選択待ち中の Status wire は正典順の複合値 `talking,choosing`（Req6.1／6.3・裁定 6）"
    );
    assert_eq!(
        notifies[0].references[3], "0",
        "選択待ち中も slot 占有継続＝Ref3 は \"0\"（応答スクリプトを再生しない・Req6.4）"
    );

    // (3) 解決後の pump: 同じ active talk 窓のまま `choosing` だけが消える（Req6.2）。
    let expected_resolved = expected_call(events::on_second_change(
        MonotonicMs(3_000),
        &talking_only_snapshot(),
    ));
    assert_eq!(
        *notifies[1], expected_resolved,
        "解決後の pump は events 表導出（NOTIFY・Ref3=0・talking 単独）と一致するはず: {recorded:?}"
    );
    assert_eq!(
        notifies[1].status,
        Some("talking".to_string()),
        "解決後の Status wire から choosing が消え talking 単独へ戻るはず（Req6.2）"
    );
    assert_eq!(
        notifies[1].references[3], "0",
        "解決後も同じ active talk 窓＝Ref3 は \"0\" のまま（talk 軸は不変）"
    );

    // (4) 非空虚性: 2 件目が確かに「解決後」の pump である（カスケード段が両 NOTIFY の間にある）。
    let first_notify_pos = recorded
        .iter()
        .position(|c| *c == expected_choosing)
        .expect("選択待ち中の NOTIFY が記録されているはず");
    let last_notify_pos = recorded
        .iter()
        .position(|c| *c == expected_resolved)
        .expect("解決後の NOTIFY が記録されているはず");
    let ex_pos = position_of(&recorded, CallMethod::Get, "OnChoiceSelectEx");
    let select_pos = position_of(&recorded, CallMethod::Get, "OnChoiceSelect");
    assert!(
        first_notify_pos < ex_pos && ex_pos < select_pos && select_pos < last_notify_pos,
        "記録順は [choosing pump] → Ex → 無印 → [解決後 pump] のはず（2 件目が解決後である証拠）: {recorded:?}"
    );

    // (5) 段列は正典形 2 段のみ（タイムアウト経路を踏んでいないことの裏取り）。
    assert_eq!(
        choice_get_ids(&recorded),
        vec!["OnChoiceSelectEx", "OnChoiceSelect"],
        "本檻の Tick は期限手前のみ＝OnChoiceTimeout は発行されないはず: {recorded:?}"
    );

    // (6) 終了系列を完走した（末尾 Unload）。
    assert_eq!(
        recorded.last().expect("記録列は空でない"),
        &expected_unload(),
        "末尾は Unload（driven close→別れ talk quit:true 由来）"
    );
}
