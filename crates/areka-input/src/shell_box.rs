//! 箱の上のポインタの判断。
//!
//! シェルの窓に届いた操作を、箱の選択肢・箱での中断・シェルへの操作に振り分ける判断を
//! 純関数で置く。ハンドラ（crate の根の `on_char_pointer_moved`・`on_char_pointer_pressed`）は
//! 借用と送り出しだけを行い、判断はここへ書く。
//!
//! 座標はシェルの窓の物理 px のまま、`shown_boxes` の四角と `choice_hit_rows_at` の行
//! （どちらも拡大率を掛けた後の値）と突き合わせる（÷k はしない）。

use areka_emo_compose::BoxName;
use areka_emo_text::actor::{ChoiceHitRow, ShownBox};
use wintf::ecs::pointer::DoubleClick;

use super::balloon::{ChoiceSelection, click_selection, hit_choice_row};
use areka_runtime::user_break_cue::NoUserBreakSignal;
// 箱の滞在の型は runtime の持ち物（可視性の相が読む）。呼び手はここから引く。
pub use areka_runtime::shell_box_hover::ShellBoxHover;

/// 届いた座標の下にある、文字の出ている箱を 1 つ選ぶ。
///
/// `boxes` は `shown_boxes` の並び（手前＝element番号の大きい順）なので、最初に当たった箱が
/// 手前。四角は半開区間 `[left, right) × [top, bottom)`（選択肢の行の判定と同じ）。
pub(crate) fn box_under_point(boxes: &[ShownBox], x: f32, y: f32) -> Option<&ShownBox> {
    boxes.iter().find(|b| {
        let r = &b.rect;
        x >= r.left && x < r.right && y >= r.top && y < r.bottom
    })
}

/// 押下 1 回の結論。`ShellOp` 以外は「処理した」で、シェルへの操作としては通知しない。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BoxPressVerdict {
    /// シェルへの操作として既存の道へ。
    ShellOp,
    /// この押下か直前の押下が選択の確定。選択肢・アンカーのどちらでも同じ。
    ConsumedBySelection,
    /// 中断を禁じる区間（止めず、シェルへも送らない）。
    Disabled,
    /// 台詞を中断する。
    Break,
}

/// 箱の中（選択肢の行の上を含む）の押下 1 回を結論へ写す。上から順に、最初に当たった結論を返す。
///
/// ⑴ この押下が選択の確定 ⑵ 左ダブルクリックでない ⑶ 直前の押下が選択の確定 ⑷ 話していない
/// ⑸ 中断を禁じる区間 ⑹ それ以外は中断。
pub(crate) fn judge_box_press(
    double_click: DoubleClick,
    selected_now: bool,
    prev_press_selected: bool,
    talking: bool,
    no_user_break: bool,
) -> BoxPressVerdict {
    if selected_now {
        BoxPressVerdict::ConsumedBySelection
    } else if double_click != DoubleClick::Left {
        BoxPressVerdict::ShellOp
    } else if prev_press_selected {
        BoxPressVerdict::ConsumedBySelection
    } else if !talking {
        BoxPressVerdict::ShellOp
    } else if no_user_break {
        BoxPressVerdict::Disabled
    } else {
        BoxPressVerdict::Break
    }
}

/// 移動 1 回の結論。
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum BoxMove {
    /// 文字の出ている箱の四角の外 → 既存の道へ。
    Outside,
    /// 選択肢の行の上 → 強調して、シェルへは送らない。アンカーの範囲の上も同じ。
    OverChoice { name: BoxName, ordinal: usize },
    /// 箱の中で行の上でない → 滞在だけ記録して既存の道へ。
    OverBody { name: BoxName },
}

/// 移動 1 回を結論へ写す。`hit` は [`box_under_point`] の結果、`rows` は当たった箱の
/// `choice_hit_rows_at`（箱の位置を足した窓の物理 px）——重なる位置でも手前の箱の行だけを見る。
pub(crate) fn judge_box_move(
    hit: Option<&ShownBox>,
    rows: &[ChoiceHitRow],
    x: f32,
    y: f32,
) -> BoxMove {
    let Some(hit) = hit else {
        return BoxMove::Outside;
    };
    let name = hit.name.clone();
    match hit_choice_row(rows, x, y) {
        Some(i) => BoxMove::OverChoice {
            name,
            ordinal: rows[i].ordinal,
        },
        None => BoxMove::OverBody { name },
    }
}

/// 左押下が箱の選択肢の確定か。中身は普通のバルーンの確定
/// （[`click_selection`]）と同じで、箱の外は必ず `None`。
pub(crate) fn judge_box_click(
    hit: Option<&ShownBox>,
    active: bool,
    rows: &[ChoiceHitRow],
    x: f32,
    y: f32,
    scope: usize,
) -> Option<ChoiceSelection> {
    hit?;
    click_selection(active, rows, x, y, scope)
}

/// 滞在の記録の次の値と、強調を外す箱。
///
/// 外す箱は、前に居た箱から出た・別の箱へ移ったときはその箱、同じ箱で行の上から外れた
/// （行の上でない）ときもその箱。前に行の上に居なかった場合も返すので、外す側は
/// 強調していなければ何もしない（同値の注入をしない）こと。
pub(crate) fn next_box_hover(
    prev: Option<&BoxName>,
    mv: &BoxMove,
) -> (Option<BoxName>, Option<BoxName>) {
    let (next, on_row) = match mv {
        BoxMove::Outside => (None, false),
        BoxMove::OverChoice { name, .. } => (Some(name), true),
        BoxMove::OverBody { name } => (Some(name), false),
    };
    let clear = match prev {
        Some(p) if next != Some(p) || !on_row => Some(p.clone()),
        _ => None,
    };
    (next.cloned(), clear)
}

/// 台詞の始まりと終わりの合図から「話している最中か」を畳む。
///
/// 始まりで真・終わりで偽。中断を禁じる区間の出入りは変えない（選択肢を待つあいだも最中）。
pub(crate) fn fold_talking(talking: bool, signal: NoUserBreakSignal) -> bool {
    match signal {
        NoUserBreakSignal::TalkStarted => true,
        NoUserBreakSignal::TalkEnded => false,
        NoUserBreakSignal::Enter | NoUserBreakSignal::Leave => talking,
    }
}

#[cfg(test)]
#[path = "shell_box_tests.rs"]
mod tests;
