// =============================================================================
// 利用者の中断の判定と送り出しの決定論テスト（areka-P0-balloon-break）
// =============================================================================
//
// 押下を中断にするかの判定・中断を禁じる旗の出入り・送出が 1 件ずつであることをここで固定する。
//
// 本ファイルの現在の範囲は純関数 2 本（`judge_press`・`fold_no_user_break`）である。
// 確かめること: 押下 1 回を 5 つの結論へ写す順序（左ダブルクリックでない・選択の確定に
// 消費された・バルーンが出ていない・無効化の区間・受理）と、その順序が上から順であること
// （要件 1.2・1.5・1.6・1.9・2.3・7.1・7.3）。旗は入る・出る・トークの始まりで解く・
// 区間外の「出る」の 4 通りと、入れ子を数えないことを固定する（要件 5.1・5.2・5.4・5.5・5.6）。

use super::*;

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
