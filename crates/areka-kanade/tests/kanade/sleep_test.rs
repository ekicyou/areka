//! 時計が止まっている間（眠り）の締め切りと、`OnSecondChange` の時の値の結合テスト。
//!
//! kanade は時計を読まず、刻みが運ぶ時刻だけで動く。眠りの間は時計が進まないので、その間の刻みは
//! 同じ時刻を運ぶ。ここでは、その形の刻みの列を入れる。

use areka_kanade::{CloseReason, KanadeConfig, KanadeMsg, TalkDone, TalkEndReason, TalkId};
use dola::TimePoint;
use dola::time::testing::at_ms;

use super::common::{
    CallMethod, Driven, Fixture, QuitPolicy, position_of, pumps, send, spawn_harness,
    spawn_harness_gated, tick,
};

const CHOICE_ID: &str = "頻度変更";

/// 選択待ちの期限は表示の終わり（1 秒）＋既定の 30 秒＝31 秒。21 秒で時計が止まり、その値の刻みが
/// 15 回来ても（止まらない時計なら期限を越える回数）時間切れは出ない。動き出した後は、31 秒ちょうどの
/// 刻みで初めて `OnChoiceTimeout` が出る。
#[test]
fn choice_deadline_holds_while_clock_is_stopped() {
    let fixture = Fixture::quitting()
        .without_boot_greeting()
        .with_steady_value_indices([0]);
    let (harness, _gate) = spawn_harness_gated(
        KanadeConfig::new("master", "1.0.0"),
        fixture,
        QuitPolicy::PerTalk(vec![false, true]),
        vec![0],
    );

    send(&harness, KanadeMsg::Boot);
    tick(&harness, at_ms(1_000));
    send(
        &harness,
        KanadeMsg::ChoiceWaiting {
            talk_id: TalkId(1),
            choice_ids: vec![CHOICE_ID.to_string()],
            display_end: at_ms(1_000),
            timeout_directive: None,
        },
    );
    for _ in 0..15 {
        tick(&harness, at_ms(21_000));
    }
    tick(&harness, at_ms(30_999));
    tick(&harness, at_ms(31_000));
    send(
        &harness,
        KanadeMsg::TalkDone(TalkDone {
            talk_id: TalkId(1),
            reason: TalkEndReason::Interrupted,
            quit_reserved: false,
        }),
    );
    send(
        &harness,
        KanadeMsg::CloseRequest {
            reason: CloseReason::User { scope: 0 },
        },
    );
    let Driven { recorded, .. } = harness.finish("kanade sleep choice join");

    let pump_calls = pumps(&recorded);
    assert_eq!(
        pump_calls.len(),
        17,
        "刻み 1 秒の GET と、期限の前の NOTIFY 16 件。期限の刻みは pump を出さない: {recorded:?}"
    );
    for pump in &pump_calls[1..] {
        assert_eq!(pump.method, CallMethod::Notify);
        assert_eq!(
            pump.status,
            Some("talking,choosing".to_string()),
            "期限の前の刻みでは選択待ちが続いている: {recorded:?}"
        );
    }
    let timeouts: Vec<_> = recorded
        .iter()
        .filter(|c| c.id == "OnChoiceTimeout")
        .collect();
    assert_eq!(timeouts.len(), 1, "時間切れは 1 度だけ: {recorded:?}");
    let last_pump = recorded
        .iter()
        .rposition(|c| c.id == "OnSecondChange")
        .expect("最後の pump が記録されている");
    let timeout_pos = position_of(&recorded, CallMethod::Get, "OnChoiceTimeout")
        .expect("OnChoiceTimeout が記録されている");
    assert!(
        last_pump < timeout_pos,
        "時間切れは 31 秒の刻みで初めて出る: {recorded:?}"
    );
}

/// Reference0 は、刻みが運ぶ時刻の 100ns の数を 1 時間の数で割った整数。止まった時計の刻みは同じ
/// 値を、動き出した後の刻みは、その時刻から求めた値を運ぶ（止まっていた間の分は足されない）。
#[test]
fn second_change_hour_comes_from_tick_time() {
    let hour = 36_000_000_000;
    let ticks = [
        (at_ms(3_599_999), "0"),
        (at_ms(3_600_000), "1"),
        (at_ms(3_600_000), "1"),
        (at_ms(3_600_000), "1"),
        (at_ms(7_199_999), "1"),
        (TimePoint::from_ticks(5 * hour - 1), "4"),
        (TimePoint::from_ticks(5 * hour), "5"),
    ];
    let harness = spawn_harness(
        KanadeConfig::new("master", "1.0.0"),
        Fixture::quitting().without_boot_greeting(),
        QuitPolicy::PerTalk(vec![true]),
    );
    send(&harness, KanadeMsg::Boot);
    for (now, _) in ticks {
        tick(&harness, now);
    }
    send(
        &harness,
        KanadeMsg::CloseRequest {
            reason: CloseReason::User { scope: 0 },
        },
    );
    let Driven { recorded, .. } = harness.finish("kanade sleep second-change join");

    let hours: Vec<&str> = pumps(&recorded)
        .iter()
        .map(|c| c.references[0].as_str())
        .collect();
    let expected: Vec<&str> = ticks.iter().map(|(_, h)| *h).collect();
    assert_eq!(hours, expected, "{recorded:?}");
}
