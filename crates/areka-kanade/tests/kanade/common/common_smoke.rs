//! 疎通テスト（観測可能な完了条件）。
//!
//! `common/mod.rs`（1,657 行）の `#[cfg(test)] mod smoke` をそのまま 1 ファイルへ出した
//! もの（タスク 8.2・設計判断 #2「1 テストモジュール＝1 テストファイル」）。モジュール名
//! `smoke` は保存しているのでテスト完全修飾名は変わらない。

use areka_actor::reply_channel;
use areka_kanade::{
    ExecutionSnapshot, KanadeConfig, KanadeMsg, MouseButton, ShioriMsg, ShioriOutcome, StartTalk,
    TalkEndReason, TalkId, events,
};

use super::{
    CallMethod, DEFAULT_TIMEOUT, FIXED_BOOT_SCRIPT, Fixture, MouseResponse, QuitPolicy,
    expected_call, join_bounded, spawn_mock_sakura, spawn_mock_shiori,
};

/// mock shiori 単独駆動: `OnBoot` GET へ即時に固定 Value を返し、記録が
/// events 表から導出した期待値と一致する（fixture・assert・実装の三点一正本）。
#[test]
fn mock_shiori_replies_and_records_onboot() {
    let config = KanadeConfig::new("master", "1.0.0");
    let shiori = spawn_mock_shiori(Fixture::default());

    // events 表から GET OnBoot の呼出を導出して送る（ハードコードしない）。boot 系列は
    // talk 非アクティブ（INACTIVE スナップショット・DD-IT-4）ゆえ Status ヘッダは出ない。
    let call = events::on_boot(&config, &ExecutionSnapshot::INACTIVE);
    let expected = expected_call(events::on_boot(&config, &ExecutionSnapshot::INACTIVE));

    let (reply, receiver) = reply_channel::<ShioriOutcome>();
    shiori
        .sender
        .send(ShioriMsg::Request { call, reply })
        .expect("send request to mock shiori");

    // 応答は即時（期限付きで受ける・ハングしない）。
    let outcome = receiver
        .recv_timeout(DEFAULT_TIMEOUT)
        .expect("mock shiori should reply immediately");
    match outcome {
        ShioriOutcome::Value(script) => assert_eq!(script, FIXED_BOOT_SCRIPT),
        // ShioriOutcome は Debug 非実装ゆえ、非 Value は総称メッセージで報告する。
        _ => panic!("expected Value(boot script) from mock shiori OnBoot"),
    }

    // 記録列が events 導出の期待値と一致する。
    let recorded = shiori.recorded();
    assert_eq!(recorded, vec![expected.clone()]);
    assert_eq!(recorded[0].method, CallMethod::Get);
    assert_eq!(recorded[0].id, "OnBoot");
    assert_eq!(recorded[0].references, vec!["master".to_string()]);

    // 停止駆動（Close）→期限付き join（ハングしない）。
    shiori
        .sender
        .send(ShioriMsg::Close)
        .expect("send close to mock shiori");
    join_bounded("mock-shiori join", DEFAULT_TIMEOUT, shiori.handle)
        .expect("mock shiori body completes normally");
}

/// mock sakura sink 単独駆動: [`TalkCommand::Start`] を受領・記録し、シナリオ指示どおり
/// [`KanadeMsg::TalkDone`] を返す。
#[test]
fn mock_sakura_records_and_returns_talkdone() {
    use areka_kanade::talk::TalkCommand;

    let (talk_tx, talk_rx) = std::sync::mpsc::channel::<TalkCommand>();
    // sink → TalkDone を受け取るための疑似 kanade inbox。
    let (kanade_tx, kanade_rx) = std::sync::mpsc::channel::<KanadeMsg>();

    let sakura = spawn_mock_sakura(talk_rx, kanade_tx, QuitPolicy::Fixed(true));

    talk_tx
        .send(TalkCommand::Start(StartTalk {
            epilogue: Vec::new(),
            talk_id: TalkId(1),
            script: FIXED_BOOT_SCRIPT.to_string(),
        }))
        .expect("send TalkCommand::Start to mock sakura");

    // TalkDone が即時に返る（期限付き・ハングしない）。
    let msg = kanade_rx
        .recv_timeout(DEFAULT_TIMEOUT)
        .expect("mock sakura should emit TalkDone immediately");
    match msg {
        KanadeMsg::TalkDone(done) => {
            assert_eq!(done.talk_id, TalkId(1));
            assert_eq!(
                done.reason,
                TalkEndReason::Quit,
                "QuitPolicy::Fixed(true) should set reason=Quit"
            );
        }
        _ => panic!("expected TalkDone from mock sakura"),
    }

    // 記録に受領 talk が残る。
    let started = sakura.started();
    assert_eq!(started.len(), 1);
    assert_eq!(started[0].talk_id, TalkId(1));
    assert_eq!(started[0].script, FIXED_BOOT_SCRIPT);

    // TalkCommand 送信端を drop → sink スレッドは自然終了（期限付き join）。
    drop(talk_tx);
    sakura.join_bounded("mock-sakura join", DEFAULT_TIMEOUT);
}

/// task 1.4（design Testing Strategy 冒頭・C7 Ordering / delivery）: mock sakura sink が
/// [`TalkCommand`] 3 形の**到着順**を記録する。
///
/// `TalkCommand` は単一チャンネルを流れることで FIFO 順序保存が契約（DD-4 の前提）であり、
/// その観測面が本記録列である。ここでは Start→Resolve→Cancel→Start を投函し、記録列が
/// **投函順そのまま**であること（＝解決系と起動系が別扱いされず 1 本の順序に載ること）を固定する。
/// `started()` は `TalkCommand::Start` の射影であり、既存檻の意味は不変であることも併せて確認する。
#[test]
fn mock_sakura_records_talk_command_arrival_order() {
    use areka_kanade::talk::TalkCommand;

    let (talk_tx, talk_rx) = std::sync::mpsc::channel::<TalkCommand>();
    let (kanade_tx, kanade_rx) = std::sync::mpsc::channel::<KanadeMsg>();

    let sakura = spawn_mock_sakura(talk_rx, kanade_tx, QuitPolicy::Fixed(false));

    talk_tx
        .send(TalkCommand::Start(StartTalk::new(TalkId(1), "first")))
        .expect("send Start(1)");
    // Start の TalkDone を受けてから次を送る（記録スレッドの前進を確定させる barrier）。
    match kanade_rx
        .recv_timeout(DEFAULT_TIMEOUT)
        .expect("Start(1) の TalkDone が返るはず")
    {
        KanadeMsg::TalkDone(done) => assert_eq!(done.talk_id, TalkId(1)),
        _ => panic!("expected TalkDone from mock sakura"),
    }

    talk_tx
        .send(TalkCommand::ResolveChoice {
            talk_id: TalkId(1),
            id: "pick".to_string(),
        })
        .expect("send ResolveChoice");
    talk_tx
        .send(TalkCommand::CancelChoice {
            talk_id: TalkId(1),
        })
        .expect("send CancelChoice");
    talk_tx
        .send(TalkCommand::Start(StartTalk::new(TalkId(2), "second")))
        .expect("send Start(2)");
    match kanade_rx
        .recv_timeout(DEFAULT_TIMEOUT)
        .expect("Start(2) の TalkDone が返るはず")
    {
        KanadeMsg::TalkDone(done) => assert_eq!(done.talk_id, TalkId(2)),
        _ => panic!("expected TalkDone from mock sakura"),
    }

    // 到着順の記録（投函順そのまま・解決系と起動系が同一の順序列に載る）。
    let commands = sakura.commands();
    assert_eq!(commands.len(), 4, "4 件すべてが記録されるべき: {commands:?}");
    match &commands[0] {
        TalkCommand::Start(s) => {
            assert_eq!(s.talk_id, TalkId(1));
            assert_eq!(s.script, "first");
        }
        other => panic!("commands[0] は Start(1) のはず: {other:?}"),
    }
    match &commands[1] {
        TalkCommand::ResolveChoice { talk_id, id } => {
            assert_eq!(*talk_id, TalkId(1));
            assert_eq!(id, "pick");
        }
        other => panic!("commands[1] は ResolveChoice のはず: {other:?}"),
    }
    match &commands[2] {
        TalkCommand::CancelChoice { talk_id } => assert_eq!(*talk_id, TalkId(1)),
        other => panic!("commands[2] は CancelChoice のはず: {other:?}"),
    }
    match &commands[3] {
        TalkCommand::Start(s) => {
            assert_eq!(s.talk_id, TalkId(2));
            assert_eq!(s.script, "second");
        }
        other => panic!("commands[3] は Start(2) のはず: {other:?}"),
    }

    // `started()` は Start の射影（既存檻の意味不変・Resolve/Cancel は現れない）。
    let started = sakura.started();
    assert_eq!(started.len(), 2);
    assert_eq!(started[0].talk_id, TalkId(1));
    assert_eq!(started[1].talk_id, TalkId(2));

    drop(talk_tx);
    sakura.join_bounded("mock-sakura join", DEFAULT_TIMEOUT);
}

/// 4.1 配管の疎通: マウス GET へ script Value と 204 を任意注入でき、mock がそれを返す。
///
/// `Fixture::with_mouse_response` で `OnMouseMove` に script を注入・`OnMouseDoubleClick` は
/// 未注入（既定 204）とし、events 表から導出した両 GET を送って注入どおりの [`ShioriOutcome`]
/// が返ることを確認する（4.2／4.3 の (c)/(d) 檻の土台＝注入点が実在することの証明）。
#[test]
fn mock_shiori_injects_mouse_response() {
    const MOUSE_SCRIPT: &str = r"\0\s[0]なでなで\e";
    // OnMouseMove へ script を注入・OnMouseDoubleClick は未注入（既定 204）。
    let fixture = Fixture::default()
        .with_mouse_response("OnMouseMove", MouseResponse::Script(MOUSE_SCRIPT.to_string()));
    let shiori = spawn_mock_shiori(fixture);

    // (c) script 注入: OnMouseMove GET → Value（events 表から導出・ハードコードしない）。
    let move_call = events::on_mouse_move(10, 20, 0, Some("Head"), &ExecutionSnapshot::INACTIVE);
    let (reply, receiver) = reply_channel::<ShioriOutcome>();
    shiori
        .sender
        .send(ShioriMsg::Request {
            call: move_call,
            reply,
        })
        .expect("send OnMouseMove to mock shiori");
    let outcome = receiver
        .recv_timeout(DEFAULT_TIMEOUT)
        .expect("mock shiori should reply immediately");
    match outcome {
        ShioriOutcome::Value(script) => assert_eq!(script, MOUSE_SCRIPT),
        _ => panic!("expected injected Value(script) for OnMouseMove"),
    }

    // (d) 未注入: OnMouseDoubleClick GET → 204（NoContent・StartTalk 不発の源）。
    let dbl_call = events::on_mouse_double_click(
        10,
        20,
        0,
        Some("Head"),
        MouseButton::Left,
        &ExecutionSnapshot::INACTIVE,
    );
    let (reply, receiver) = reply_channel::<ShioriOutcome>();
    shiori
        .sender
        .send(ShioriMsg::Request {
            call: dbl_call,
            reply,
        })
        .expect("send OnMouseDoubleClick to mock shiori");
    let outcome = receiver
        .recv_timeout(DEFAULT_TIMEOUT)
        .expect("mock shiori should reply immediately");
    match outcome {
        ShioriOutcome::NoContent => {}
        _ => panic!("expected 204 (NoContent) for un-injected OnMouseDoubleClick"),
    }

    // 両 GET が events 導出の期待値どおり記録される（Ref layout 込みの突合は 4.2／4.3 が担う）。
    let recorded = shiori.recorded();
    assert_eq!(recorded.len(), 2);
    assert_eq!(recorded[0].id, "OnMouseMove");
    assert_eq!(recorded[1].id, "OnMouseDoubleClick");

    shiori
        .sender
        .send(ShioriMsg::Close)
        .expect("send close to mock shiori");
    join_bounded("mock-shiori join", DEFAULT_TIMEOUT, shiori.handle)
        .expect("mock shiori body completes normally");
}
