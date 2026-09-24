//! 利用者の中断（バルーンの左ダブルクリック）の判定と送り出し（areka-P0-balloon-break）。
//!
//! 中断を禁じる旗・直前の押下の記憶・送出端を持ち、押下のたびに「中断にするか」を決めて、
//! 表示の側へ「隠せ」、kanade へ `KanadeMsg::UserBreak` を 1 件ずつ送る層がここへ入る。
//!
//! 判断は純関数 2 本（[`judge_press`]・[`fold_no_user_break`]）が担い、持ち物
//! [`UserBreakWiring`]・旗の取り出し [`drain_no_user_break_signals`]・押下の入口 [`on_left_press`] が
//! それを World の上で動かす。持ち物を World へ据えて取り出しを登録する結線が [`wire_user_break`]。

use std::sync::mpsc::{Receiver, Sender};

use areka_kanade::KanadeMsg;
use bevy_ecs::schedule::{IntoScheduleConfigs, Schedules};
use bevy_ecs::world::World;
use wintf::ecs::Input;
use wintf::ecs::pointer::{DoubleClick, dispatch_pointer_events};

use crate::emo2_boot::frame::Emo2Wiring;
use crate::emo2_boot::talk_lifecycle::TalkLifecycleSignal;
use crate::emo2_boot::target_map::balloon_target;
use crate::emo2_boot::user_break_cue::NoUserBreakSignal;

/// 中断の配線の持ち物（NonSend・UI スレッド所有）。
pub(crate) struct UserBreakWiring {
    /// 旗の線の受信端（talk スレッドの受け口 `NoUserBreakCueSink` と対）。
    flag_rx: Receiver<NoUserBreakSignal>,
    /// 中断を禁じる旗（要件 5）。
    no_user_break: bool,
    /// 表示の合図の線の送出端の複製（「中断された」を送る）。
    lifecycle_tx: Sender<TalkLifecycleSignal>,
    /// 運行（kanade）への送出端の複製。
    kanade: Sender<KanadeMsg>,
    /// 直前の左押下が選択の確定だったか（要件 1.5）。
    prev_press_selected: bool,
}

impl UserBreakWiring {
    /// 3 本の線の端から組む（旗は下りた状態・直前の押下の記憶は「確定でない」から始まる）。
    pub(crate) fn new(
        flag_rx: Receiver<NoUserBreakSignal>,
        lifecycle_tx: Sender<TalkLifecycleSignal>,
        kanade: Sender<KanadeMsg>,
    ) -> Self {
        Self {
            flag_rx,
            no_user_break: false,
            lifecycle_tx,
            kanade,
            prev_press_selected: false,
        }
    }

    /// 旗の読み口（要件 5.7）。読む者はまだ居ないので、運行の側への通知は作っていない。
    // 要件 5.7 が「開けておく」と定めた読み口で、本番にまだ読み手が無い。読むのは兄弟テストだけである。
    #[allow(dead_code)]
    pub(crate) fn no_user_break(&self) -> bool {
        self.no_user_break
    }

    /// 旗の線の受信端が今の送出端につながっているか（テスト専用の読み口）。
    ///
    /// 未読の値が残っていても 1 度の `try_recv` では生死が分からないので、`Err` が出るまで
    /// 回して最後の `Err` を見る: `Empty` なら生きている・`Disconnected` なら古い。
    /// 取り出した値は捨てる（読むのはテストだけ）。
    #[cfg(test)]
    pub(crate) fn flag_source_connected(&mut self) -> bool {
        loop {
            match self.flag_rx.try_recv() {
                Ok(_) => continue,
                Err(err) => return err == std::sync::mpsc::TryRecvError::Empty,
            }
        }
    }
}

/// 押下 1 回の結論（design「UserBreakWiring と judge_press」）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PressVerdict {
    /// 左ダブルクリックではない——何もしない（要件 1.2）。
    NotDoubleClick,
    /// この押下か直前の押下が選択の確定——合図を作らない（要件 1.5）。
    ConsumedBySelection,
    /// バルーンが出ていない・観測できない——作らない（要件 1.9）。
    BalloonHidden,
    /// 中断を禁じる区間——止めず隠さず記録だけ（要件 2.3）。
    Disabled,
    /// 受け入れ——隠して、止める要求を送る（要件 1.1・1.6）。
    Break,
}

/// 押下 1 回を結論へ写す（純関数）。上から順に、最初に当たった結論を返す。
///
/// 順は ⑴ 左ダブルクリックでない ⑵ 選択の確定に消費された ⑶ バルーンが出ていない
/// ⑷ 無効化の区間 ⑸ 受理（design「判定の順」）。`balloon_visible` の `None` は
/// 「表示層に相手が居ない＝観測できない」で、作らない側へ倒す（要件 1.9）。
///
/// SSTP に関する判定は 1 つも置かない（要件 5.8）。
pub(crate) fn judge_press(
    double_click: DoubleClick,
    selected_now: bool,
    prev_press_selected: bool,
    balloon_visible: Option<bool>,
    no_user_break: bool,
) -> PressVerdict {
    if double_click != DoubleClick::Left {
        PressVerdict::NotDoubleClick
    } else if selected_now || prev_press_selected {
        PressVerdict::ConsumedBySelection
    } else if balloon_visible != Some(true) {
        PressVerdict::BalloonHidden
    } else if no_user_break {
        PressVerdict::Disabled
    } else {
        PressVerdict::Break
    }
}

/// 中断を禁じる旗の畳み込み（純関数）。返り値は（次の旗, 区間外の「出る」だったか）。
///
/// 入れ子は数えない——「入る」が重なっても旗は 1 本で、「出る」1 回で区間から出る
/// （要件 5.5）。トークの始まりは閉じ忘れた区間を解く契機である（要件 5.4）。
/// 区間外の「出る」は旗を変えず、呼び手が記録できるように第 2 の返り値で伝える（要件 5.6）。
pub(crate) fn fold_no_user_break(flag: bool, signal: NoUserBreakSignal) -> (bool, bool) {
    match signal {
        NoUserBreakSignal::Enter => (true, false),
        NoUserBreakSignal::Leave => (false, !flag),
        NoUserBreakSignal::TalkStarted => (false, false),
    }
}

/// 旗の線に溜まった合図を全部取り出して旗へ畳み込む（入力の段で毎巡走る）。
///
/// 旗が変わったときと、区間外の「出る」を記録する（要件 5.6・6.3）。持ち物が無い（結線前）は
/// 取り出す線そのものが無いので、何もしない。
pub(crate) fn drain_no_user_break_signals(world: &mut World) {
    let Some(mut wiring) = world.get_non_send_mut::<UserBreakWiring>() else {
        return;
    };
    while let Ok(signal) = wiring.flag_rx.try_recv() {
        let (value, leave_outside) = fold_no_user_break(wiring.no_user_break, signal);
        if value != wiring.no_user_break {
            wiring.no_user_break = value;
            tracing::debug!(
                event = "no_user_break_changed",
                value,
                "中断を禁じる旗が変わった"
            );
        }
        if leave_outside {
            tracing::debug!(
                event = "no_user_break_leave_outside",
                "区間外の leave,nouserbreakmode: 旗は下りたまま"
            );
        }
    }
}

/// 中断の持ち物を World へ入れる（`fn wire_readme`・`crates/areka/src/readme.rs` と同型）。
/// 旗の取り出しの登録は [`register_user_break_drain`] が行う。
///
/// 呼び手は boot 成功後の `wire_emo2_boot`（1 回の実行につき 1 度だけ）。
pub(crate) fn wire_user_break(
    world: &mut World,
    flag_rx: Receiver<NoUserBreakSignal>,
    lifecycle_tx: Sender<TalkLifecycleSignal>,
    kanade: Sender<KanadeMsg>,
) {
    world.insert_non_send(UserBreakWiring::new(flag_rx, lifecycle_tx, kanade));
}

/// 旗の取り出しを毎巡の入力の段へ登録する（登録だけ・持ち物は置かない）。
///
/// 並びは `dispatch_pointer_events` の**前**——押下と同じ巡に届いた「入る」「出る」を、その
/// 押下の判定より先に旗へ畳み込むためである（要件 4.5・5.1）。後ろに置くと、旗が 1 巡遅れて
/// 効く。`dispatch_pointer_events` 自身は wintf の既定の登録（`EcsWorld::new`・
/// `crates/wintf/src/ecs/world/mod.rs`）で `drain_task_pool_commands` の後に置かれているだけ
/// なので、前に 1 本置いても順序は循環しない。隣の取り出し（`fn register_readme_drain`・
/// `fn register_choice_drain`）はどれも「後」で、「前」はここだけである。
///
/// 呼び手は `ghost_session::register_systems`（プロセスに 1 回）。
pub(crate) fn register_user_break_drain(world: &mut World) {
    world.resource_mut::<Schedules>().add_systems(
        Input,
        drain_no_user_break_signals.before(dispatch_pointer_events),
    );
}

/// バルーン窓の押下ハンドラ（`fn on_balloon_pointer_pressed`・`balloon.rs`）の末尾から呼ばれる入口。
/// 中断を受け入れたら `true`。
///
/// 「バルーンが出ているか」を表示層へ照会し、残りを [`press_with_visibility`] に任せる。
/// `Emo2Wiring` が無い・表示層に相手が居ないはどちらも「観測できない」で、作らない側へ倒れる
/// （要件 1.9）。
pub(crate) fn on_left_press(
    world: &mut World,
    scope: usize,
    double_click: DoubleClick,
    selected_now: bool,
) -> bool {
    // 型合わせのみ（スコープの実値は小さい。隣の `fn to_choice_input`・`choice_drain.rs` と同じ流儀）。
    let balloon_visible = world
        .get_non_send::<Emo2Wiring>()
        .and_then(|w| w.presenter().target_visible(balloon_target(scope as u32)));
    press_with_visibility(world, scope, double_click, selected_now, balloon_visible)
}

/// 押下 1 回の本体。照会の結果を引数に取るのは、GPU 無しでは可視のバルーンを作れず、
/// 受理の経路を照会ごとテストできないためである。
///
/// 送る順は「隠せ」→「止めろ」で、1 件ずつ。どちらの失敗も記録し、もう一方の送出はやめない
/// ——止める要求だけ落ちると「バルーンは消えたのに再生が続く」になるので、隣の
/// `fn send_selection` より重い水準で記録する（要件 1.7・2.8）。
fn press_with_visibility(
    world: &mut World,
    scope: usize,
    double_click: DoubleClick,
    selected_now: bool,
    balloon_visible: Option<bool>,
) -> bool {
    let Some(mut wiring) = world.get_non_send_mut::<UserBreakWiring>() else {
        tracing::trace!(
            event = "balloon_break_no_wiring",
            "中断の持ち物が無い（結線前）: 何もしない"
        );
        return false;
    };
    // 読んでから上書きする。単押しでも上書きするのは、選択を確定した 1 打目が単押しとして届くため。
    let prev_press_selected = wiring.prev_press_selected;
    wiring.prev_press_selected = selected_now;
    if double_click != DoubleClick::Left {
        return false;
    }
    tracing::trace!(
        event = "balloon_break_detected",
        scope,
        "バルーンの左ダブルクリックを検出"
    );

    match judge_press(
        double_click,
        selected_now,
        prev_press_selected,
        balloon_visible,
        wiring.no_user_break,
    ) {
        PressVerdict::NotDoubleClick => false,
        PressVerdict::ConsumedBySelection => {
            tracing::debug!(
                event = "balloon_break_ignored",
                scope,
                reason = "selection",
                "選択の確定の続きの押下: 中断にしない"
            );
            false
        }
        PressVerdict::BalloonHidden => {
            tracing::trace!(
                event = "balloon_break_ignored",
                scope,
                reason = "balloon_hidden",
                "バルーンが出ていない・観測できない: 中断にしない"
            );
            false
        }
        PressVerdict::Disabled => {
            tracing::debug!(
                event = "balloon_break_rejected",
                scope,
                reason = "no_user_break",
                "中断を禁じる区間: 止めず隠さない"
            );
            false
        }
        PressVerdict::Break => {
            if wiring
                .lifecycle_tx
                .send(TalkLifecycleSignal::UserBreak)
                .is_err()
            {
                tracing::error!(
                    event = "balloon_break_hide_send_failed",
                    scope,
                    "表示の側へ「中断された」を渡せない（受け手が消えている）"
                );
            }
            let msg = KanadeMsg::UserBreak {
                scope: scope as u32,
            };
            if wiring.kanade.send(msg).is_err() {
                tracing::error!(
                    event = "balloon_break_send_failed",
                    scope,
                    "運行の側へ中断の要求を渡せない（受け手が消えている）: 再生は止まらない"
                );
            }
            true
        }
    }
}

#[cfg(test)]
#[path = "user_break_tests.rs"]
mod tests;
