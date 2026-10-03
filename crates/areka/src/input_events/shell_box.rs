//! 箱の上のポインタの判断（areka-P0-shell-balloon・design「箱のポインタの前段」）。
//!
//! シェルの窓に届いた操作を、箱の選択肢・箱での中断・シェルへの操作に振り分ける判断を
//! 純関数で置く。ハンドラ（`mod.rs` の `on_char_pointer_moved`・`on_char_pointer_pressed`）は
//! 借用と送り出しだけを行い、判断はここへ書く。
//!
//! 座標はシェルの窓の物理 px のまま、`shown_boxes` の四角と `choice_hit_rows_at` の行
//! （どちらも拡大率を掛けた後の値）と突き合わせる（÷k はしない）。

// ハンドラ（task 11.2）と中断の入口（task 11.1）が呼ぶまでは本番から到達しない。
#![allow(dead_code)]

use std::collections::HashMap;

use areka_emo_compose::BoxName;
use areka_emo_text::actor::{ChoiceHitRow, ShownBox};
use wintf::ecs::pointer::DoubleClick;

use super::balloon::{ChoiceSelection, click_selection, hit_choice_row};
use crate::emo2_boot::user_break_cue::NoUserBreakSignal;

/// 届いた座標の下にある、文字の出ている箱を 1 つ選ぶ（要件 9.5・3.9）。
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
    /// シェルへの操作として既存の道へ（要件 9.1・9.7）。
    ShellOp,
    /// この押下か直前の押下が選択の確定（要件 8.5）。
    ConsumedBySelection,
    /// 中断を禁じる区間（止めず、シェルへも送らない・要件 9.6）。
    Disabled,
    /// 台詞を中断する（要件 9.6）。
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
    /// 選択肢の行の上 → 強調して、シェルへは送らない（要件 8.2）。
    OverChoice { name: BoxName, ordinal: usize },
    /// 箱の中で行の上でない → 滞在だけ記録して既存の道へ（要件 9.1・9.7）。
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

/// 左押下が箱の選択肢の確定か（要件 8.3・8.5）。中身は普通のバルーンの確定
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

/// 滞在の記録の次の値と、強調を外す箱（要件 6.11・8.2）。
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

/// 毎フレームの整え: 滞在している箱が `shown_boxes` に無ければ `None` へ戻す（要件 6.11）。
pub(crate) fn settle_box_hover(prev: Option<&BoxName>, shown: &[ShownBox]) -> Option<BoxName> {
    prev.filter(|p| shown.iter().any(|b| &b.name == *p))
        .cloned()
}

/// 台詞の始まりと終わりの合図から「話している最中か」を畳む（要件 9.1・9.6〜9.8）。
///
/// 始まりで真・終わりで偽。中断を禁じる区間の出入りは変えない（選択肢を待つあいだも最中）。
pub(crate) fn fold_talking(talking: bool, signal: NoUserBreakSignal) -> bool {
    match signal {
        NoUserBreakSignal::TalkStarted => true,
        NoUserBreakSignal::TalkEnded => false,
        NoUserBreakSignal::Enter | NoUserBreakSignal::Leave => talking,
    }
}

/// スコープごとの「今ポインタが居る、文字の出ている箱の名前」（NonSend・UI スレッド所有）。
///
/// 書き込むのは [`next_box_hover`]（箱へ入る・移る・出る）・シェルの窓からの離脱（`None`）・
/// 毎フレームの [`settle_box_hover`] の結果だけ。時間切れの観測の滞在は `get(scope).is_some()`。
#[derive(Debug, Default)]
pub(crate) struct ShellBoxHover(HashMap<usize, BoxName>);

impl ShellBoxHover {
    /// スコープの滞在している箱（居なければ `None`）。
    pub(crate) fn get(&self, scope: usize) -> Option<&BoxName> {
        self.0.get(&scope)
    }

    /// スコープの滞在を書き換える（`None` で消す）。
    pub(crate) fn set(&mut self, scope: usize, name: Option<BoxName>) {
        match name {
            Some(name) => self.0.insert(scope, name),
            None => self.0.remove(&scope),
        };
    }
}

#[cfg(test)]
#[path = "shell_box_tests.rs"]
mod tests;
