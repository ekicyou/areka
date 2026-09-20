// =============================================================================
// 利用者の中断の判定と送り出しの決定論テスト（areka-P0-balloon-break）
// =============================================================================
//
// 押下を中断にするかの判定・中断を禁じる旗の出入り・送出が 1 件ずつであることをここで固定する。
//
// 純関数 2 本（`judge_press`・`fold_no_user_break`）について確かめること: 押下 1 回を 5 つの
// 結論へ写す順序（左ダブルクリックでない・選択の確定に消費された・バルーンが出ていない・
// 無効化の区間・受理）と、その順序が上から順であること（要件 1.2・1.5・1.6・1.9・2.3・7.1・
// 7.3）。旗は入る・出る・トークの始まりで解く・区間外の「出る」の 4 通りと、入れ子を数えない
// ことを固定する（要件 5.1・5.2・5.4・5.5・5.6）。
//
// 押下の入口と旗の取り出しについて確かめること: 受理のとき 2 本の線へちょうど 1 件ずつ届く・
// 片方の線の受け手が消えていても記録してもう片方へは届ける・結線前は何もしない・無効化の
// 区間ではどちらの線へも送らない・直前の押下の記憶は読んでから上書きする・旗の出入りが
// 記録される（要件 1.1・1.7・1.8・2.3・5.7・6.3・6.4・6.6）。
//
// 「バルーンが出ているか」は表示層への照会で、GPU 無しのテストでは可視のバルーンを作れない
// （表示が一度も確立していない target は可視化できない）。そこで送り出しのテストは照会の結果を
// 引数に取る内側の関数 `press_with_visibility` を直接呼ぶ。外側の `on_left_press` は照会だけを
// 足した薄い包みで、照会が効いていること（可視を決め打ちしていないこと）は 1 本で確かめる。

use std::sync::mpsc::{self, Receiver};

use tracing::Level;

use super::*;
use crate::placement::test_support::{LogEvent, capture_logs};

// ---------------------------------------------------------------- 道具立て

/// 受理になる入力の組（どの欄を崩すと結論が変わるかを 1 か所で見せるため）。
fn all_clear() -> (DoubleClick, bool, bool, Option<bool>, bool) {
    (DoubleClick::Left, false, false, Some(true), false)
}

/// 組をそのまま `judge_press` へ渡す。
fn judge(args: (DoubleClick, bool, bool, Option<bool>, bool)) -> PressVerdict {
    judge_press(args.0, args.1, args.2, args.3, args.4)
}

// ---------------------------------------------------------------- 押下の判定

/// 何も妨げが無ければ受け入れる（要件 1.1・1.6）。
#[test]
fn all_clear_is_break() {
    assert_eq!(judge(all_clear()), PressVerdict::Break);
}

/// 左以外のダブルクリック・単押しからは中断の合図を作らない（要件 1.2）。
#[test]
fn non_left_double_click_is_not_double_click() {
    for kind in [
        DoubleClick::None,
        DoubleClick::Right,
        DoubleClick::Middle,
        DoubleClick::XButton1,
        DoubleClick::XButton2,
    ] {
        let mut args = all_clear();
        args.0 = kind;
        assert_eq!(
            judge(args),
            PressVerdict::NotDoubleClick,
            "{kind:?} は中断の合図にしない"
        );
    }
}

/// 今回の押下が選択の確定なら作らない（要件 1.5）。
#[test]
fn selected_now_is_consumed_by_selection() {
    let mut args = all_clear();
    args.1 = true;
    assert_eq!(judge(args), PressVerdict::ConsumedBySelection);
}

/// 1 打目が選択の確定だったときの 2 打目も作らない（要件 1.5・判断分岐 ⑸）。
#[test]
fn prev_press_selected_is_consumed_by_selection() {
    let mut args = all_clear();
    args.2 = true;
    assert_eq!(judge(args), PressVerdict::ConsumedBySelection);
}

/// バルーンが隠れている・表示層に相手が居ないのどちらも作らない（要件 1.9）。
#[test]
fn balloon_not_visible_is_balloon_hidden() {
    for visible in [Some(false), None] {
        let mut args = all_clear();
        args.3 = visible;
        assert_eq!(
            judge(args),
            PressVerdict::BalloonHidden,
            "{visible:?} では中断の合図にしない"
        );
    }
}

/// 無効化の区間では退ける（要件 2.3・判断分岐 ⑷）。
#[test]
fn no_user_break_is_disabled() {
    let mut args = all_clear();
    args.4 = true;
    assert_eq!(judge(args), PressVerdict::Disabled);
}

/// 判定は上から順で、先に当たった結論が後ろの結論より優先する（design「判定の順」）。
#[test]
fn verdicts_are_ordered_top_down() {
    // 左でない × 他 3 つすべて → 左でないが勝つ
    assert_eq!(
        judge_press(DoubleClick::Right, true, true, Some(false), true),
        PressVerdict::NotDoubleClick
    );
    // 選択の確定 × 隠れている × 無効化 → 選択の確定が勝つ
    assert_eq!(
        judge_press(DoubleClick::Left, true, false, Some(false), true),
        PressVerdict::ConsumedBySelection
    );
    // 隠れている × 無効化 → 隠れているが勝つ
    assert_eq!(
        judge_press(DoubleClick::Left, false, false, None, true),
        PressVerdict::BalloonHidden
    );
}

// ---------------------------------------------------------------- 旗の畳み込み

/// `\![enter,nouserbreakmode]` で区間に入る（要件 5.1）。
#[test]
fn enter_raises_flag() {
    assert_eq!(
        fold_no_user_break(false, NoUserBreakSignal::Enter),
        (true, false)
    );
}

/// `\![leave,nouserbreakmode]` で区間から出る（要件 5.2）。
#[test]
fn leave_inside_section_lowers_flag() {
    assert_eq!(
        fold_no_user_break(true, NoUserBreakSignal::Leave),
        (false, false)
    );
}

/// 区間外の「出る」は旗を下ろしたままにし、その旨を返す（要件 5.6）。
#[test]
fn leave_outside_section_is_reported() {
    assert_eq!(
        fold_no_user_break(false, NoUserBreakSignal::Leave),
        (false, true)
    );
}

/// 閉じ忘れたままトークが始まったら解く（要件 5.4）。
#[test]
fn talk_started_clears_flag() {
    assert_eq!(
        fold_no_user_break(true, NoUserBreakSignal::TalkStarted),
        (false, false)
    );
    assert_eq!(
        fold_no_user_break(false, NoUserBreakSignal::TalkStarted),
        (false, false)
    );
}

/// 入れ子は数えない——「入る」2 回でも「出る」1 回で区間から出る（要件 5.5）。
#[test]
fn nesting_is_not_counted() {
    let (flag, _) = fold_no_user_break(false, NoUserBreakSignal::Enter);
    let (flag, _) = fold_no_user_break(flag, NoUserBreakSignal::Enter);
    assert!(flag, "2 回目の「入る」でも区間に入ったまま");
    let (flag, outside) = fold_no_user_break(flag, NoUserBreakSignal::Leave);
    assert!(!flag, "「出る」1 回で区間から出る");
    assert!(!outside, "区間内の「出る」は区間外の扱いにしない");
}

// ---------------------------------------------------------------- 押下の入口と旗の取り出し

/// 結線済みの World と、2 本の線の受信端・旗の線の送出端。
///
/// 受信端を `Option` で持つのは、受け手が消えた線を作るテストが片方だけ落とすためである。
struct Lines {
    world: World,
    flag_tx: Sender<NoUserBreakSignal>,
    lifecycle_rx: Option<Receiver<TalkLifecycleSignal>>,
    kanade_rx: Option<Receiver<KanadeMsg>>,
}

impl Lines {
    fn wired() -> Self {
        let (flag_tx, flag_rx) = mpsc::channel();
        let (lifecycle_tx, lifecycle_rx) = mpsc::channel();
        let (kanade_tx, kanade_rx) = mpsc::channel();
        let mut world = World::new();
        world.insert_non_send(UserBreakWiring::new(flag_rx, lifecycle_tx, kanade_tx));
        Lines {
            world,
            flag_tx,
            lifecycle_rx: Some(lifecycle_rx),
            kanade_rx: Some(kanade_rx),
        }
    }

    /// バルーンが出ている状態での押下 1 回。
    fn press(&mut self, scope: usize, double_click: DoubleClick, selected_now: bool) -> bool {
        press_with_visibility(
            &mut self.world,
            scope,
            double_click,
            selected_now,
            Some(true),
        )
    }

    /// 旗の合図を 1 件流して取り出す。
    fn signal(&mut self, signal: NoUserBreakSignal) {
        self.flag_tx.send(signal).expect("受信端は持ち物が持つ");
        drain_no_user_break_signals(&mut self.world);
    }

    fn flag(&self) -> bool {
        self.world
            .get_non_send::<UserBreakWiring>()
            .expect("結線済み")
            .no_user_break()
    }

    /// 表示の合図の線へ届いた分を全部取り出す。
    fn lifecycle(&self) -> Vec<TalkLifecycleSignal> {
        let rx = self.lifecycle_rx.as_ref().expect("受信端は生きている");
        rx.try_iter().collect()
    }

    /// 運行への線へ届いた分を全部取り出す。
    fn kanade(&self) -> Vec<KanadeMsg> {
        let rx = self.kanade_rx.as_ref().expect("受信端は生きている");
        rx.try_iter().collect()
    }
}

/// `event` フィールドが `name` の記録だけを取り出す。
fn named<'a>(events: &'a [LogEvent], name: &str) -> Vec<&'a LogEvent> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(name))
        .collect()
}

/// 受理のとき、表示の合図の線と運行への線へちょうど 1 件ずつ届く（要件 1.1・2.8・6.6）。
#[test]
fn accepted_break_sends_exactly_one_to_each_line() {
    let mut lines = Lines::wired();
    let (accepted, events) = capture_logs(|| lines.press(1, DoubleClick::Left, false));

    assert!(accepted, "妨げが無ければ受け入れる");
    assert_eq!(lines.lifecycle(), vec![TalkLifecycleSignal::UserBreak]);
    let to_kanade = lines.kanade();
    assert_eq!(to_kanade.len(), 1, "運行への要求はちょうど 1 件");
    assert!(
        matches!(to_kanade[0], KanadeMsg::UserBreak { scope: 1 }),
        "押されたバルーンのスコープ番号を運ぶ"
    );

    let detected = named(&events, "balloon_break_detected");
    assert_eq!(detected.len(), 1, "検出の記録は 1 行: {events:?}");
    assert_eq!(detected[0].level, Level::TRACE);
    assert_eq!(detected[0].field("scope"), Some("1"));
}

/// 単押しは中断にせず、検出としても記録しない。同じ窓で左ダブルクリックの検出が見えている
/// ことで、「記録が無い」が捕捉の空振りでないことを示す（要件 1.2・6.6）。
#[test]
fn single_press_sends_nothing_and_is_not_recorded_as_detected() {
    let mut lines = Lines::wired();
    let (_, events) = capture_logs(|| {
        assert!(!lines.press(0, DoubleClick::None, false));
        assert!(lines.lifecycle().is_empty() && lines.kanade().is_empty());
        lines.press(0, DoubleClick::Left, false)
    });
    assert_eq!(named(&events, "balloon_break_detected").len(), 1);
}

/// 表示の側の受け手が消えていても記録し、運行への要求は送る（要件 1.7・6.4）。
#[test]
fn dropped_lifecycle_receiver_is_recorded_and_kanade_still_receives() {
    let mut lines = Lines::wired();
    lines.lifecycle_rx = None;
    let (_, events) = capture_logs(|| lines.press(0, DoubleClick::Left, false));

    let failed = named(&events, "balloon_break_hide_send_failed");
    assert_eq!(failed.len(), 1, "隠す指示を渡せない旨が 1 行: {events:?}");
    assert_eq!(failed[0].level, Level::ERROR);
    assert_eq!(failed[0].field("scope"), Some("0"));
    assert_eq!(lines.kanade().len(), 1, "もう一方の送出はやめない");
}

/// 運行の側の受け手が消えていても記録し、隠す合図は送る（要件 1.7・6.4）。
#[test]
fn dropped_kanade_receiver_is_recorded_and_lifecycle_still_receives() {
    let mut lines = Lines::wired();
    lines.kanade_rx = None;
    let (_, events) = capture_logs(|| lines.press(0, DoubleClick::Left, false));

    let failed = named(&events, "balloon_break_send_failed");
    assert_eq!(failed.len(), 1, "運行へ渡せない旨が 1 行: {events:?}");
    assert_eq!(failed[0].level, Level::ERROR);
    assert_eq!(failed[0].field("scope"), Some("0"));
    assert_eq!(lines.lifecycle(), vec![TalkLifecycleSignal::UserBreak]);
}

/// 結線前の押下は記録だけして何もしない（要件 1.8）。
#[test]
fn unwired_press_is_recorded_and_does_nothing() {
    let mut world = World::new();
    let (accepted, events) =
        capture_logs(|| press_with_visibility(&mut world, 0, DoubleClick::Left, false, Some(true)));

    assert!(!accepted);
    let no_wiring = named(&events, "balloon_break_no_wiring");
    assert_eq!(no_wiring.len(), 1, "結線前の旨が 1 行: {events:?}");
    assert_eq!(no_wiring[0].level, Level::TRACE);
    assert!(
        world.get_non_send::<UserBreakWiring>().is_none(),
        "持ち物を勝手に作らない"
    );
}

/// 無効化の区間ではどちらの線へも送らず、退けた旨を記録する。区間を出れば同じ押下が受理に
/// なる（要件 2.3・6.3・判断分岐 ⑷）。
#[test]
fn disabled_section_sends_nothing_until_left() {
    let mut lines = Lines::wired();
    lines.signal(NoUserBreakSignal::Enter);
    assert!(lines.flag(), "旗の読み口から区間に入ったことが読める");

    let (accepted, events) = capture_logs(|| lines.press(0, DoubleClick::Left, false));
    assert!(!accepted);
    assert!(lines.lifecycle().is_empty(), "隠さない");
    assert!(lines.kanade().is_empty(), "止めない");
    let rejected = named(&events, "balloon_break_rejected");
    assert_eq!(rejected.len(), 1, "退けた旨が 1 行: {events:?}");
    assert_eq!(rejected[0].level, Level::DEBUG);
    assert_eq!(rejected[0].field_str("reason"), Some("no_user_break"));
    assert_eq!(rejected[0].field("scope"), Some("0"));

    lines.signal(NoUserBreakSignal::Leave);
    assert!(lines.press(0, DoubleClick::Left, false), "区間を出れば受理");
    assert_eq!(lines.lifecycle().len(), 1);
    assert_eq!(lines.kanade().len(), 1);
}

/// 直前の押下の記憶は読んでから今回で上書きする: 選択を確定した単押しに続く左ダブルクリックは
/// 何も送らず、その次の左ダブルクリックは受理になる（要件 1.5・判断分岐 ⑸）。
#[test]
fn previous_press_memory_is_read_then_overwritten() {
    let mut lines = Lines::wired();
    assert!(
        !lines.press(0, DoubleClick::None, true),
        "1 打目＝選択の確定"
    );

    let (accepted, events) = capture_logs(|| lines.press(0, DoubleClick::Left, false));
    assert!(!accepted, "選択の確定の続きの 2 打目は中断にしない");
    assert!(lines.lifecycle().is_empty() && lines.kanade().is_empty());
    let ignored = named(&events, "balloon_break_ignored");
    assert_eq!(ignored.len(), 1, "見送った旨が 1 行: {events:?}");
    assert_eq!(ignored[0].level, Level::DEBUG);
    assert_eq!(ignored[0].field_str("reason"), Some("selection"));

    assert!(
        lines.press(0, DoubleClick::Left, false),
        "記憶は 2 打目で上書きされているので、次の左ダブルクリックは受理"
    );
    assert_eq!(lines.lifecycle().len(), 1);
    assert_eq!(lines.kanade().len(), 1);
}

/// 外側の入口は表示層へ照会した結果で判定する: 表示層に相手が居なければ、結線済みでも
/// どちらの線へも送らない（要件 1.9）。
#[test]
fn on_left_press_asks_the_presenter_and_ignores_when_no_balloon_is_shown() {
    let mut lines = Lines::wired();
    let (accepted, events) =
        capture_logs(|| on_left_press(&mut lines.world, 0, DoubleClick::Left, false));

    assert!(!accepted);
    assert!(lines.lifecycle().is_empty() && lines.kanade().is_empty());
    let ignored = named(&events, "balloon_break_ignored");
    assert_eq!(ignored.len(), 1, "見送った旨が 1 行: {events:?}");
    assert_eq!(ignored[0].level, Level::TRACE);
    assert_eq!(ignored[0].field_str("reason"), Some("balloon_hidden"));
}

/// 旗の取り出しは、旗が変わったときと区間外の「出る」を記録する（要件 5.6・5.7・6.3）。
#[test]
fn drain_records_flag_changes_and_leave_outside() {
    let mut lines = Lines::wired();

    // 「入る」2 回: 旗が変わるのは 1 回目だけなので記録は 1 行。
    let (_, events) = capture_logs(|| {
        lines.flag_tx.send(NoUserBreakSignal::Enter).unwrap();
        lines.flag_tx.send(NoUserBreakSignal::Enter).unwrap();
        drain_no_user_break_signals(&mut lines.world);
    });
    let changed = named(&events, "no_user_break_changed");
    assert_eq!(changed.len(), 1, "変わったときだけ記録する: {events:?}");
    assert_eq!(changed[0].level, Level::DEBUG);
    assert_eq!(changed[0].field("value"), Some("true"));
    assert!(lines.flag());

    // 閉じ忘れたままトークが始まる → 解ける。続く「出る」は区間外。
    let (_, events) = capture_logs(|| {
        lines.flag_tx.send(NoUserBreakSignal::TalkStarted).unwrap();
        lines.flag_tx.send(NoUserBreakSignal::Leave).unwrap();
        drain_no_user_break_signals(&mut lines.world);
    });
    let changed = named(&events, "no_user_break_changed");
    assert_eq!(changed.len(), 1, "{events:?}");
    assert_eq!(changed[0].field("value"), Some("false"));
    let outside = named(&events, "no_user_break_leave_outside");
    assert_eq!(outside.len(), 1, "{events:?}");
    assert_eq!(outside[0].level, Level::DEBUG);
    assert!(!lines.flag());
}

/// 結線前の取り出しは何もしない（落ちない）。
#[test]
fn drain_without_wiring_does_nothing() {
    let mut world = World::new();
    drain_no_user_break_signals(&mut world);
    assert!(world.get_non_send::<UserBreakWiring>().is_none());
}
