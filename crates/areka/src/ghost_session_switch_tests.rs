//! 偽の SHIORI を持つゴーストを同じ World で起こす統合テスト（areka-P0-ghost-shell-balloon-switch）。
//!
//! 土台は `emo2_boot::ghost_switch_test_support`。前半は土台そのものが使えることを確かめる:
//! 作り口がフォルダ名で偽の SHIORI を選ぶこと・先に書いた起動記録が効くこと・接続に失敗する
//! 版が kanade を `Fault` で止めること。後半は台本の `\![change,ghost,…]` から切替先の定常到達
//! までを 1 周させる（要件 10.1・10.6・10.13）。判定は集めてから 1 回・降ろすのは必ず有界に行う。

use std::sync::mpsc::TryRecvError;

use areka_kanade::{KanadeStopCause, ShioriFaultKind};
use bevy_ecs::schedule::Schedules;
use bevy_ecs::world::World;
use log_capture_kit::{CapturedEvent, capture};
use wintf::ecs::widget::bitmap_source::WintfTaskPool;
use wintf::ecs::{FrameFinalize, Input, Update};

use super::GhostBootInputsSource;
use crate::app_exit::{ExitOrigin, FirstExit};
use crate::emo2_boot::frame::KanadeNoticeRx;
use crate::emo2_boot::ghost_switch::SwitchInFlight;
use crate::emo2_boot::ghost_switch_test_support::{
    CONNECT_ERR, FakeShiori, SwitchRig, standard_script,
};
use crate::emo2_boot::spine::RecordedCall;
use crate::input_events::user_break::UserBreakWiring;
use crate::menu::{Frame, MenuWiring};
use crate::readme::ReadmeWiring;

/// 呼出列を「種類 名前」の列へ写す（Reference は見ない）。
fn kinds(calls: &[RecordedCall]) -> Vec<String> {
    calls
        .iter()
        .map(|c| match c {
            RecordedCall::Get { id, .. } => format!("GET {id}"),
            RecordedCall::Notify { id, .. } => format!("NOTIFY {id}"),
            RecordedCall::Unload => "UNLOAD".to_owned(),
            RecordedCall::Status => "STATUS".to_owned(),
        })
        .collect()
}

/// `folder` を 1 回起こした記録に `basewareversion` まで届いたか（起動系列の終わり）。
fn booted(rig: &SwitchRig, folder: &str) -> bool {
    rig.calls(folder).last().is_some_and(|calls| {
        calls
            .iter()
            .any(|c| matches!(c, RecordedCall::Notify { id, .. } if id == "basewareversion"))
    })
}

/// 土台だけで A（起動記録なし）を起こすと、A の偽の SHIORI に起動系列が
/// （`OnFirstBoot` 204 → `OnBoot` を含めて）記録され、B の偽の SHIORI は作られない。
/// 定常到達の通知を通知の相が受けても終了は指示されない。
#[test]
fn rig_boots_a_and_records_its_boot_sequence() {
    let mut rig = SwitchRig::new(vec![
        (
            "A",
            FakeShiori::Scripted(Box::new(|| standard_script("\\0A\\e"))),
        ),
        (
            "B",
            FakeShiori::Scripted(Box::new(|| standard_script("\\0B\\e"))),
        ),
    ]);
    rig.boot("A");
    let reached = rig.pump_until(|rig| booted(rig, "A"));
    let a_calls: Vec<Vec<String>> = rig.calls("A").iter().map(|c| kinds(c)).collect();
    let on_boot_ref_count = rig.calls("A").last().and_then(|calls| {
        calls.iter().find_map(|c| match c {
            RecordedCall::Get { id, references } if id == "OnBoot" => Some(references.len()),
            _ => None,
        })
    });
    let b_boots = rig.calls("B").len();
    let exit_requested = rig.exit_requested();
    let shutdown_ok = rig.shutdown();

    assert_eq!(
        (
            reached,
            a_calls,
            on_boot_ref_count,
            b_boots,
            exit_requested,
            shutdown_ok
        ),
        (
            true,
            vec![vec![
                "NOTIFY OnInitialize".to_owned(),
                "GET username".to_owned(),
                "GET OnFirstBoot".to_owned(),
                "GET OnBoot".to_owned(),
                "NOTIFY basewareversion".to_owned(),
            ]],
            Some(1),
            0,
            false,
            true,
        ),
        "A の起動系列が記録されない（届いた・A の呼出列・OnBoot の Reference の数＝由来がふつうなら \
         Ref0 だけ・B を起こした回数・終了の指示・降ろせた）"
    );
}

/// 先に起動記録を書いた B を起こすと `OnFirstBoot` を飛ばして `OnBoot` から起きる。
#[test]
fn rig_boot_record_skips_first_boot() {
    let mut rig = SwitchRig::new(vec![(
        "B",
        FakeShiori::Scripted(Box::new(|| standard_script("\\0B\\e"))),
    )]);
    rig.plant_boot_record("B");
    rig.boot("B");
    let reached = rig.pump_until(|rig| booted(rig, "B"));
    let b_calls: Vec<Vec<String>> = rig.calls("B").iter().map(|c| kinds(c)).collect();
    let shutdown_ok = rig.shutdown();

    assert_eq!(
        (reached, b_calls, shutdown_ok),
        (
            true,
            vec![vec![
                "NOTIFY OnInitialize".to_owned(),
                "GET username".to_owned(),
                "GET OnBoot".to_owned(),
                "NOTIFY basewareversion".to_owned(),
            ]],
            true,
        ),
        "起動記録が効かない（届いた・B の呼出列・降ろせた）"
    );
}

/// 接続に失敗する版を選ぶと kanade が `Fault`（接続できなかった）で止まり、切替の予約が無い
/// 今は通知の相が終了を指示する。台本は作られない。
#[test]
fn rig_connect_fail_stops_kanade_with_fault() {
    let mut rig = SwitchRig::new(vec![("B", FakeShiori::ConnectFail)]);
    rig.boot("B");
    let reached = rig.pump_until(SwitchRig::exit_requested);
    let fault = rig
        .world
        .get_resource::<FirstExit>()
        .and_then(|first| match &first.0 {
            ExitOrigin::KanadeStopped(KanadeStopCause::Fault(f)) => {
                Some((f.kind, f.reason.contains(CONNECT_ERR)))
            }
            _ => None,
        });
    let b_boots = rig.calls("B").len();
    let shutdown_ok = rig.shutdown();

    assert_eq!(
        (reached, fault, b_boots, shutdown_ok),
        (true, Some((ShioriFaultKind::ConnectFailed, true)), 0, true),
        "接続に失敗する版が効かない（終了の指示・最初の出所の Fault・台本の数・降ろせた）"
    );
}

// ── 台本の切替命令から切替先の定常到達までの 1 周（要件 1.8・3.3・4.1・4.2・4.4・4.8・10.1・10.6・10.13） ──

/// A の `OnBoot` の台本: 台詞の途中で B への切替（`OnGhostChanging` を送らせる）を命じる。
const A_TO_B: &str = "\\0A\\![change,ghost,B,--option=raise-event]\\e";
/// A の `OnGhostChanging` が返す送り出しの台本（B の `OnGhostChanged` の Ref1 に載る）。
const A_SEND_OFF: &str = "\\0Bへ交代します\\e";

/// A の偽の SHIORI: 標準の台本（`OnBoot`＝`on_boot`）に `OnGhostChanging`＝送り出しの台本と
/// `OnGhostChanged`＝`on_changed` を足したもの。
fn ghost_a(on_boot: &'static str, on_changed: Option<&'static str>) -> FakeShiori {
    FakeShiori::Scripted(Box::new(move || {
        standard_script(on_boot)
            .get("OnGhostChanging", Ok(Some(A_SEND_OFF.to_owned())))
            .get("OnGhostChanged", Ok(on_changed.map(str::to_owned)))
    }))
}

/// B の偽の SHIORI: 標準の台本（`OnBoot` あり）に `OnGhostChanged` 204 を足したもの。
fn ghost_b() -> FakeShiori {
    FakeShiori::Scripted(Box::new(|| {
        standard_script("\\0B\\e").get("OnGhostChanged", Ok(None))
    }))
}

/// A・B を据えた土台（窓の閉包の投函先つき・A は起動記録あり＝`OnBoot` の台本から始まる）。
fn lap_rig(a: FakeShiori, b: FakeShiori) -> SwitchRig {
    let mut rig = SwitchRig::new(vec![("A", a), ("B", b)]);
    // 切替先の窓の準備が閉包を投函する先（`Input` の段に作業プールの取り出しの系は無いので走らない）。
    rig.world.insert_resource(WintfTaskPool::new());
    rig.plant_boot_record("A");
    rig
}

/// `Input`／`Update`／`FrameFinalize` の各段に載っている系の数。
fn systems_lens(world: &World) -> [usize; 3] {
    let schedules = world.resource::<Schedules>();
    [
        schedules.get(Input).map_or(0, |s| s.systems_len()),
        schedules.get(Update).map_or(0, |s| s.systems_len()),
        schedules.get(FrameFinalize).map_or(0, |s| s.systems_len()),
    ]
}

/// 呼出列から `id` の GET の Reference を取る。
fn get_refs(calls: &[RecordedCall], id: &str) -> Option<Vec<String>> {
    calls.iter().find_map(|c| match c {
        RecordedCall::Get {
            id: got,
            references,
        } if got == id => Some(references.clone()),
        _ => None,
    })
}

/// `event` の記録の水準の列。
fn levels_of(events: &[CapturedEvent], event: &str) -> Vec<tracing::Level> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(event))
        .map(|e| e.level)
        .collect()
}

/// フォルダの絶対パスの文字列（Ref3 の形）。
fn abs(rig: &SwitchRig, folder: &str) -> String {
    std::path::absolute(rig.root.ghost_dir(folder))
        .expect("絶対パスにできる")
        .display()
        .to_string()
}

/// 停止通知の受け口が生きた送出端につながっているか。土台の作り口が持つ送出端の写しを
/// 先に落とすので、残る送出端は置き場のゴーストの kanade のものだけ（降ろしたゴーストの
/// 送出端は降ろすときに落ちる）。未読の通知は読み捨て、最後が `Empty` なら生きている。
fn notice_rx_live(world: &mut World) -> bool {
    world.remove_non_send::<GhostBootInputsSource>();
    let rx = &world.non_send::<KanadeNoticeRx>().0;
    loop {
        match rx.try_recv() {
            Ok(_) => continue,
            Err(err) => break err == TryRecvError::Empty,
        }
    }
}

/// 切替で起きたゴーストの定常到達まで回す（`folder` の起動が `boots` 回に達し、予約が消えた
/// 時点＝迎え入れの定常の通知を処理した時点。終了が指示されたら打ち切る）。
fn pump_to_welcomed(rig: &mut SwitchRig, folder: &str, boots: usize) -> bool {
    rig.pump_talking_until(|rig| {
        rig.exit_requested()
            || (rig.calls(folder).len() >= boots
                && rig.world.get_non_send::<SwitchInFlight>().is_none())
    })
}

/// 台本の `\![change,ghost,B,--option=raise-event]` から B の定常到達までの 1 周（要件 10.1）。
///
/// A を起こし、定常に入ってから台詞の時計を回す——台本の cue が本番の受け口（`ChangeCueSink`）に
/// 届き、`wire_change_rx` が据えた受信端を `register_systems` が登録した取り出しの系が `Input` の段で
/// 読んで入口へ渡す。A は `OnGhostChanging` の台本を再生し終えて降ろされ、起動記録のある B が
/// `OnGhostChanged`（204）→ `OnBoot` で起きて定常に入る。集めてから 1 回で判定する:
/// ⑴ A・B の呼出列と `OnGhostChanging`／`OnGhostChanged` の Reference（Ref0〜3・7、Ref4〜6 は空）
/// ⑵ 終了の指示 0 件 ⑶ 系の登録数が A の起動のときと同じ ⑷ 窓ごとの状態が B のもの（説明書の経路が
/// B の根の下・メニューの登記に「ゴースト」・中断の旗と停止通知の受け口が生きた送出端につながる）
/// ⑸ 予約が消えている。切替の要求の記録は 1 件だけ（台本の命令 1 つにつき 1 件）。
///
/// # 非空虚性
/// 取り出しの系の登録（`register_change_drain`）か受信端の据え付け（`wire_change_rx`）を外すと
/// 切替が起きず、B の起動 0 回のまま期限切れになる。降ろした A の停止通知で終了を指示すると ⑵ が赤。
#[test]
fn script_change_tag_switches_a_to_b_and_reaches_steady() {
    let mut rig = lap_rig(ghost_a(A_TO_B, None), ghost_b());
    rig.plant_boot_record("B");
    rig.boot("A");
    let lens_round1 = systems_lens(&rig.world);
    let steady = rig.wait_steady();
    let (welcomed, events) = capture(|| pump_to_welcomed(&mut rig, "B", 1));

    let a = rig.calls("A");
    let b = rig.calls("B");
    let a_kinds: Vec<Vec<String>> = a.iter().map(|c| kinds(c)).collect();
    let b_kinds: Vec<Vec<String>> = b.iter().map(|c| kinds(c)).collect();
    let changing = a.first().and_then(|c| get_refs(c, "OnGhostChanging"));
    let changed = b.first().and_then(|c| get_refs(c, "OnGhostChanged"));
    let exit_requested = rig.exit_requested();
    let lens_after = systems_lens(&rig.world);
    let b_root = rig.root.ghost_dir("B");
    let readme_under_b = rig
        .world
        .get_non_send::<ReadmeWiring>()
        .is_some_and(|w| w.path().starts_with(&b_root));
    let frames = rig
        .world
        .get_non_send::<MenuWiring>()
        .map(|w| w.registry.registered_frames());
    let user_break_live = rig
        .world
        .get_non_send_mut::<UserBreakWiring>()
        .is_some_and(|mut w| w.flag_source_connected());
    let notice_live = notice_rx_live(&mut rig.world);
    let reserved = rig.world.get_non_send::<SwitchInFlight>().is_some();
    // 切替先の新しい単位は LogSink へ倒れた旗を持たない（倒れた理由を引き継がない・要件 8.3）。
    let fallback = rig
        .world
        .get_non_send::<super::GhostSlot>()
        .and_then(|slot| slot.0.as_ref().map(super::GhostSession::logsink_fallback));
    let expected_changing = Some(vec![
        "Bのさくら".to_owned(),
        "automatic".to_owned(),
        "B".to_owned(),
        abs(&rig, "B"),
    ]);
    let expected_changed = Some(vec![
        "Aのさくら".to_owned(),
        A_SEND_OFF.to_owned(),
        "A".to_owned(),
        abs(&rig, "A"),
        String::new(),
        String::new(),
        String::new(),
        "master".to_owned(),
    ]);
    let shutdown_ok = rig.shutdown();

    assert_eq!(
        (
            (steady, welcomed, a_kinds, changing, b_kinds, changed),
            (
                exit_requested,
                lens_after,
                readme_under_b,
                frames,
                user_break_live,
                notice_live,
                reserved,
                fallback,
                levels_of(&events, "ghost_switch_requested"),
                shutdown_ok,
            ),
        ),
        (
            (
                true,
                true,
                vec![vec![
                    "NOTIFY OnInitialize".to_owned(),
                    "GET username".to_owned(),
                    "GET OnBoot".to_owned(),
                    "NOTIFY basewareversion".to_owned(),
                    "GET OnGhostChanging".to_owned(),
                    "UNLOAD".to_owned(),
                ]],
                expected_changing,
                vec![vec![
                    "NOTIFY OnInitialize".to_owned(),
                    "GET username".to_owned(),
                    "GET OnGhostChanged".to_owned(),
                    "GET OnBoot".to_owned(),
                    "NOTIFY basewareversion".to_owned(),
                ]],
                expected_changed,
            ),
            (
                false,
                lens_round1,
                true,
                Some(vec![
                    Frame::Ghost,
                    Frame::Shell,
                    Frame::Balloon,
                    Frame::Update,
                    Frame::Install,
                    Frame::Readme,
                    Frame::Close
                ]),
                true,
                true,
                false,
                Some(false),
                vec![tracing::Level::INFO],
                true,
            ),
        ),
        "1 周が通らない（A の定常・B の定常まで届いた・A の呼出列・OnGhostChanging の Reference・\
         B の呼出列・OnGhostChanged の Reference・終了の指示・系の数 [Input, Update, FrameFinalize]・\
         説明書が B の根の下・メニューの登記・中断の旗が生きている・停止通知の受け口が生きている・\
         予約が残っている・切替先が LogSink へ倒れた旗・切替の要求の記録・降ろせた): {events:?}"
    );
}

/// 起動記録の無い B へ切り替えると、起動の根は `OnFirstBoot`（204 → `OnBoot`）で
/// `OnGhostChanged` は 0 件（要件 4.4・10.13・起動の根は常に 1 つ）。
///
/// # 非空虚性
/// 起動の根が由来を先に見て `OnGhostChanged` を選ぶと、B の呼出列が `OnFirstBoot` を欠いて赤。
#[test]
fn switch_to_b_without_boot_record_sends_first_boot_not_ghost_changed() {
    let mut rig = lap_rig(ghost_a(A_TO_B, None), ghost_b());
    rig.boot("A");
    let steady = rig.wait_steady();
    let welcomed = pump_to_welcomed(&mut rig, "B", 1);

    let b_kinds: Vec<Vec<String>> = rig.calls("B").iter().map(|c| kinds(c)).collect();
    let exit_requested = rig.exit_requested();
    let reserved = rig.world.get_non_send::<SwitchInFlight>().is_some();
    let shutdown_ok = rig.shutdown();

    assert_eq!(
        (
            steady,
            welcomed,
            b_kinds,
            exit_requested,
            reserved,
            shutdown_ok
        ),
        (
            true,
            true,
            // `OnGhostChanged` は 0 件（列に無い）。
            vec![vec![
                "NOTIFY OnInitialize".to_owned(),
                "GET username".to_owned(),
                "GET OnFirstBoot".to_owned(),
                "GET OnBoot".to_owned(),
                "NOTIFY basewareversion".to_owned(),
            ]],
            false,
            false,
            true,
        ),
        "起動記録の無い B の起動の根が OnFirstBoot でない（A の定常・B の定常まで届いた・B の呼出列・\
         終了の指示・予約が残っている・降ろせた）"
    );
}

/// 自分自身（A → A）への切替は無視されず、A を降ろして起こし直す（要件 1.8・10.6）。
/// 2 度目の A は `OnGhostChanged`（Ref2＝A）で起き、その台本があるので `OnBoot` は送らない（要件 4.3）
/// ——2 度目の A が再び切替を命じて回り続けないための台本でもある。
///
/// # 非空虚性
/// 入口が今のゴーストを切替先から除外すると、A の起動は 1 回のままで期限切れになる。
#[test]
fn switch_to_self_takes_down_and_reboots_a() {
    const A_TO_A: &str = "\\0A\\![change,ghost,A,--option=raise-event]\\e";
    let mut rig = lap_rig(ghost_a(A_TO_A, Some("\\0A再び\\e")), ghost_b());
    rig.boot("A");
    let steady = rig.wait_steady();
    let welcomed = pump_to_welcomed(&mut rig, "A", 2);

    let a = rig.calls("A");
    let a_kinds: Vec<Vec<String>> = a.iter().map(|c| kinds(c)).collect();
    let changed_name = a
        .get(1)
        .and_then(|c| get_refs(c, "OnGhostChanged"))
        .and_then(|refs| refs.get(2).cloned());
    let b_boots = rig.calls("B").len();
    let exit_requested = rig.exit_requested();
    let reserved = rig.world.get_non_send::<SwitchInFlight>().is_some();
    let shutdown_ok = rig.shutdown();

    assert_eq!(
        (
            steady,
            welcomed,
            a_kinds,
            changed_name,
            b_boots,
            exit_requested,
            reserved,
            shutdown_ok
        ),
        (
            true,
            true,
            vec![
                vec![
                    "NOTIFY OnInitialize".to_owned(),
                    "GET username".to_owned(),
                    "GET OnBoot".to_owned(),
                    "NOTIFY basewareversion".to_owned(),
                    "GET OnGhostChanging".to_owned(),
                    "UNLOAD".to_owned(),
                ],
                vec![
                    "NOTIFY OnInitialize".to_owned(),
                    "GET username".to_owned(),
                    "GET OnGhostChanged".to_owned(),
                    "NOTIFY basewareversion".to_owned(),
                ],
            ],
            Some("A".to_owned()),
            0,
            false,
            false,
            true,
        ),
        "自分自身への切替で降ろして起こし直していない（A の定常・2 度目の A の定常まで届いた・\
         A の呼出列・2 度目の OnGhostChanged の Ref2・B を起こした回数・終了の指示・予約が残っている・\
         降ろせた）"
    );
}

// 失敗方向（既定へ戻す・致命・孤児の窓 0）は子のファイルへ分ける（1 ファイルの行数を抑える）。
#[path = "ghost_session_switch_fallback_tests.rs"]
mod fallback_tests;

// 切替の記憶の時系列（最後に使ったゴースト・起動中の印）は子のファイルへ分ける。
#[path = "ghost_session_switch_memory_tests.rs"]
mod memory_tests;
