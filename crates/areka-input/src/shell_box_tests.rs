//! 箱の上のポインタの判断（純関数）の檻。
//!
//! 箱の名前は公開の構築口を持たないので、テストの中に持つ surfaces.txt の文面を
//! `parse_boxes` → `fold_boxes` で畳んで名前を取る（検体は読まない）。座標はシェルの窓の物理 px。

use areka_emo_text::actor::{ChoiceHitRow, HitRectPx, ShownBox};
use areka_emo_text::state::SpanKind;
use wintf::ecs::pointer::DoubleClick;

use super::*;
use crate::balloon::test_support::{self, box_name};
use crate::balloon::{ChoiceSelection, click_selection};
use areka_runtime::user_break_cue::NoUserBreakSignal;

/// サーフェス 0 に箱 a（element1）と箱 b（element2・手前）。
const SHELL: &str = "\
balloon.a
{
size,100,50
}
balloon.b
{
size,80,40
}
surface0
{
element1,balloon,a,0,0
element2,balloon,b,50,20
}
";

fn rect(left: f32, top: f32, right: f32, bottom: f32) -> HitRectPx {
    HitRectPx {
        left,
        top,
        right,
        bottom,
    }
}

/// `shown_boxes` と同じ並び（手前＝element番号の大きい順）: b(50,20)-(130,60)・a(0,0)-(100,50)。
fn shown() -> Vec<ShownBox> {
    vec![
        ShownBox {
            name: box_name(SHELL, "b"),
            element: 2,
            rect: rect(50.0, 20.0, 130.0, 60.0),
        },
        ShownBox {
            name: box_name(SHELL, "a"),
            element: 1,
            rect: rect(0.0, 0.0, 100.0, 50.0),
        },
    ]
}

fn row(ordinal: usize, left: f32, top: f32, right: f32, bottom: f32) -> ChoiceHitRow {
    ChoiceHitRow {
        references: vec![format!("ref{ordinal}")],
        ..test_support::row(ordinal, left, top, right, bottom)
    }
}

// ---------------------------------------------------------------------------
// box_under_point
// ---------------------------------------------------------------------------

/// 重なる位置では手前（element番号の大きい）の箱を 1 つだけ選ぶ。
#[test]
fn overlapping_boxes_pick_the_front_one() {
    let boxes = shown();
    let hit = box_under_point(&boxes, 70.0, 30.0).expect("重なりの中");
    assert_eq!(hit.name, box_name(SHELL, "b"));
    assert_eq!(hit.element, 2);
}

/// 奥の箱だけが覆う位置ではその箱を選ぶ。
#[test]
fn back_box_alone_is_picked() {
    let boxes = shown();
    assert_eq!(
        box_under_point(&boxes, 10.0, 10.0).map(|b| b.name.clone()),
        Some(box_name(SHELL, "a"))
    );
}

/// 四角の外・文字の出ている箱が無いときは無し。右辺と下辺は含まない（半開区間）。
#[test]
fn outside_every_rect_is_none() {
    let boxes = shown();
    assert_eq!(box_under_point(&boxes, 200.0, 200.0), None);
    assert_eq!(box_under_point(&boxes, 130.0, 30.0), None, "右辺は外");
    assert_eq!(box_under_point(&boxes, 120.0, 60.0), None, "下辺は外");
    assert_eq!(
        box_under_point(&[], 10.0, 10.0),
        None,
        "文字の出ている箱が無い"
    );
}

// ---------------------------------------------------------------------------
// judge_box_press
// ---------------------------------------------------------------------------

/// この押下が選択の確定 → 処理した（シェルへは送らない）。単押しでも同じ。
#[test]
fn press_selected_now_is_consumed() {
    for dc in [DoubleClick::None, DoubleClick::Left] {
        assert_eq!(
            judge_box_press(dc, true, false, false, false),
            BoxPressVerdict::ConsumedBySelection
        );
    }
}

/// 左ダブルクリックでない → シェルへの操作。話している最中でも同じ。
#[test]
fn press_not_left_double_is_shell_op() {
    for dc in [DoubleClick::None, DoubleClick::Right, DoubleClick::Middle] {
        assert_eq!(
            judge_box_press(dc, false, true, true, false),
            BoxPressVerdict::ShellOp
        );
    }
}

/// 直前の押下が選択の確定 → 処理した（続きのダブルクリックを中断にもシェルにもしない）。
#[test]
fn press_after_selection_is_consumed() {
    assert_eq!(
        judge_box_press(DoubleClick::Left, false, true, true, false),
        BoxPressVerdict::ConsumedBySelection
    );
}

/// 話していない → シェルへのダブルクリック。
#[test]
fn press_not_talking_is_shell_op() {
    assert_eq!(
        judge_box_press(DoubleClick::Left, false, false, false, true),
        BoxPressVerdict::ShellOp
    );
}

/// 中断を禁じる区間 → 止めず、シェルへも送らない。
#[test]
fn press_in_no_user_break_is_disabled() {
    assert_eq!(
        judge_box_press(DoubleClick::Left, false, false, true, true),
        BoxPressVerdict::Disabled
    );
}

/// それ以外 → 台詞を中断する。
#[test]
fn press_while_talking_breaks() {
    assert_eq!(
        judge_box_press(DoubleClick::Left, false, false, true, false),
        BoxPressVerdict::Break
    );
}

// ---------------------------------------------------------------------------
// judge_box_move（3 つの結論）
// ---------------------------------------------------------------------------

/// 四角の外 → 既存の道へ。
#[test]
fn move_outside_box() {
    let boxes = shown();
    let hit = box_under_point(&boxes, 300.0, 300.0);
    assert_eq!(judge_box_move(hit, &[], 300.0, 300.0), BoxMove::Outside);
}

/// 選択肢の行の上 → その箱の名前と行の序数。
#[test]
fn move_over_choice_row() {
    let boxes = shown();
    let rows = [row(0, 0.0, 0.0, 40.0, 10.0), row(3, 0.0, 10.0, 40.0, 20.0)];
    let hit = box_under_point(&boxes, 5.0, 15.0);
    assert_eq!(
        judge_box_move(hit, &rows, 5.0, 15.0),
        BoxMove::OverChoice {
            name: box_name(SHELL, "a"),
            ordinal: 3
        }
    );
}

/// 箱の中で行の上でない → 滞在だけ（行が無いときも同じ）。
#[test]
fn move_over_body() {
    let boxes = shown();
    let rows = [row(0, 0.0, 0.0, 40.0, 10.0)];
    let hit = box_under_point(&boxes, 30.0, 40.0);
    assert_eq!(
        judge_box_move(hit, &rows, 30.0, 40.0),
        BoxMove::OverBody {
            name: box_name(SHELL, "a")
        }
    );
    assert_eq!(
        judge_box_move(hit, &[], 30.0, 40.0),
        BoxMove::OverBody {
            name: box_name(SHELL, "a")
        }
    );
}

/// 重なる位置では手前の箱の行だけを見る: 奥の箱 a の行がその位置にあっても、手前の箱 b の
/// 行（呼び手が `choice_hit_rows_at` で手前の箱について取った行）に当たらなければ行の上でない。
#[test]
fn overlap_looks_only_at_front_box_rows() {
    let boxes = shown();
    // 奥の箱 a の行は (70,30) を覆う: 奥の箱だけを見れば行の上になる位置。
    let back_rows = [row(0, 60.0, 25.0, 90.0, 35.0)];
    let back = box_under_point(&boxes[1..], 70.0, 30.0);
    assert!(matches!(
        judge_box_move(back, &back_rows, 70.0, 30.0),
        BoxMove::OverChoice { .. }
    ));
    // 実際の当たりは手前の箱 b で、b の行には当たらない → 行の上でない。
    let front_rows = [row(1, 50.0, 40.0, 130.0, 50.0)];
    let hit = box_under_point(&boxes, 70.0, 30.0);
    assert_eq!(hit.map(|b| b.name.clone()), Some(box_name(SHELL, "b")));
    assert_eq!(
        judge_box_move(hit, &front_rows, 70.0, 30.0),
        BoxMove::OverBody {
            name: box_name(SHELL, "b")
        }
    );
    assert_eq!(
        judge_box_move(hit, &front_rows, 70.0, 45.0),
        BoxMove::OverChoice {
            name: box_name(SHELL, "b"),
            ordinal: 1
        }
    );
}

// ---------------------------------------------------------------------------
// judge_box_click
// ---------------------------------------------------------------------------

/// 行の上の左押下: 普通のバルーンの確定と同じ中身（スコープ・ID・表示・参照）が 1 つ返る。
#[test]
fn click_on_row_returns_same_payload_as_balloon() {
    let boxes = shown();
    let rows = [row(0, 0.0, 0.0, 40.0, 10.0), row(2, 0.0, 10.0, 40.0, 20.0)];
    let hit = box_under_point(&boxes, 5.0, 15.0);
    let got = judge_box_click(hit, true, &rows, 5.0, 15.0, 1);
    assert_eq!(
        got,
        Some(ChoiceSelection {
            kind: SpanKind::Choice,
            id: "q2".into(),
            label: "label2".into(),
            scope: 1,
            references: vec!["ref2".into()],
        })
    );
    assert_eq!(
        got,
        click_selection(true, &rows, 5.0, 15.0, 1),
        "普通のバルーンと同じ"
    );
}

/// 行の外・箱の外・選択肢が出ていない（`active` が偽・行が空）ときは無し。
#[test]
fn click_without_row_box_or_choices_is_none() {
    let boxes = shown();
    let rows = [row(0, 0.0, 0.0, 40.0, 10.0)];
    let inside = box_under_point(&boxes, 30.0, 40.0);
    assert_eq!(
        judge_box_click(inside, true, &rows, 30.0, 40.0, 0),
        None,
        "行の外"
    );
    // 箱の外: 行の四角に入る座標でも、箱に当たっていなければ無し。
    assert_eq!(
        judge_box_click(None, true, &rows, 5.0, 5.0, 0),
        None,
        "箱の外"
    );
    let on_row = box_under_point(&boxes, 5.0, 5.0);
    assert_eq!(
        judge_box_click(on_row, false, &rows, 5.0, 5.0, 0),
        None,
        "選択肢が出ていない"
    );
    assert_eq!(
        judge_box_click(on_row, true, &[], 5.0, 5.0, 0),
        None,
        "行が無い"
    );
}

// ---------------------------------------------------------------------------
// 箱のアンカー
// ---------------------------------------------------------------------------

fn anchor_row(ordinal: usize, left: f32, top: f32, right: f32, bottom: f32) -> ChoiceHitRow {
    ChoiceHitRow {
        references: vec![format!("ref{ordinal}")],
        ..test_support::anchor_row(ordinal, left, top, right, bottom)
    }
}

/// 箱のアンカーの上の左押下はアンカーの知らせになり、その押下は「選択で使った」になる——
/// 単押しでも、話している最中でも、シェルの操作にも中断にもならない。続く 2 打目も同じ。
#[test]
fn click_on_anchor_in_box_is_consumed_by_selection() {
    let boxes = shown();
    let rows = [anchor_row(0, 0.0, 0.0, 40.0, 10.0)];
    let hit = box_under_point(&boxes, 5.0, 5.0);
    let got = judge_box_click(hit, true, &rows, 5.0, 5.0, 1);
    assert_eq!(
        got.as_ref().map(|s| (s.kind, s.id.as_str())),
        Some((SpanKind::Anchor, "q0"))
    );
    let selected_now = got.is_some();
    for talking in [false, true] {
        assert_eq!(
            judge_box_press(DoubleClick::None, selected_now, false, talking, false),
            BoxPressVerdict::ConsumedBySelection
        );
        // 2 打目: アンカーの上のまま（範囲が残っている）でも、範囲の外（台詞が替わった）でも。
        for second_on_anchor in [true, false] {
            assert_eq!(
                judge_box_press(
                    DoubleClick::Left,
                    second_on_anchor,
                    selected_now,
                    talking,
                    false
                ),
                BoxPressVerdict::ConsumedBySelection
            );
        }
    }
}

/// 箱の中でも、アンカーの上の移動は強調の対象になり、選択肢と重なる所は選択肢が先。
/// 押下の知らせも同じ順で決まる。
#[test]
fn anchor_in_box_is_hovered_and_choice_comes_first() {
    let boxes = shown();
    let rows = [
        row(0, 0.0, 0.0, 40.0, 10.0),
        anchor_row(1, 0.0, 0.0, 40.0, 20.0),
    ];
    let hit = box_under_point(&boxes, 5.0, 15.0);
    assert_eq!(
        judge_box_move(hit, &rows, 5.0, 15.0),
        BoxMove::OverChoice {
            name: box_name(SHELL, "a"),
            ordinal: 1
        },
        "アンカーだけの所"
    );
    assert_eq!(
        judge_box_move(hit, &rows, 5.0, 5.0),
        BoxMove::OverChoice {
            name: box_name(SHELL, "a"),
            ordinal: 0
        },
        "重なる所は選択肢"
    );
    let kind = |y: f32| judge_box_click(hit, true, &rows, 5.0, y, 0).map(|s| s.kind);
    assert_eq!(kind(15.0), Some(SpanKind::Anchor));
    assert_eq!(kind(5.0), Some(SpanKind::Choice));
    assert_eq!(kind(30.0), None, "範囲の外は無し");
}

// ---------------------------------------------------------------------------
// next_box_hover
// ---------------------------------------------------------------------------

/// 外から箱へ入る: 記録は入った箱・外す強調は無し。
#[test]
fn hover_enter() {
    let a = box_name(SHELL, "a");
    assert_eq!(
        next_box_hover(None, &BoxMove::OverBody { name: a.clone() }),
        (Some(a.clone()), None)
    );
    assert_eq!(
        next_box_hover(
            None,
            &BoxMove::OverChoice {
                name: a.clone(),
                ordinal: 0
            }
        ),
        (Some(a), None)
    );
}

/// 同じ箱の中で行から行へ動く: 記録はそのまま・外す強調は無し（新しい行は強調の側が付け替える）。
#[test]
fn hover_move_within_same_box() {
    let a = box_name(SHELL, "a");
    assert_eq!(
        next_box_hover(
            Some(&a),
            &BoxMove::OverChoice {
                name: a.clone(),
                ordinal: 1
            }
        ),
        (Some(a), None)
    );
}

/// 同じ箱の中で行から行の外へ: 記録はそのまま・その箱の強調を外す。
#[test]
fn hover_leave_row_within_box_clears_highlight() {
    let a = box_name(SHELL, "a");
    assert_eq!(
        next_box_hover(Some(&a), &BoxMove::OverBody { name: a.clone() }),
        (Some(a.clone()), Some(a))
    );
}

/// 別の箱へ移る: 記録は移った先・前の箱の強調を外す。
#[test]
fn hover_move_to_another_box() {
    let (a, b) = (box_name(SHELL, "a"), box_name(SHELL, "b"));
    assert_eq!(
        next_box_hover(Some(&a), &BoxMove::OverBody { name: b.clone() }),
        (Some(b.clone()), Some(a.clone()))
    );
    assert_eq!(
        next_box_hover(
            Some(&a),
            &BoxMove::OverChoice {
                name: b.clone(),
                ordinal: 0
            }
        ),
        (Some(b), Some(a))
    );
}

/// 箱の外へ出る: 記録は無し・前の箱の強調を外す。もともと外なら何もしない。
#[test]
fn hover_leave_box() {
    let a = box_name(SHELL, "a");
    assert_eq!(next_box_hover(Some(&a), &BoxMove::Outside), (None, Some(a)));
    assert_eq!(next_box_hover(None, &BoxMove::Outside), (None, None));
}

// ---------------------------------------------------------------------------
// fold_talking
// ---------------------------------------------------------------------------

/// 台詞の始まりで真・終わりで偽。中断を禁じる区間の出入りは「話している最中か」を変えない。
#[test]
fn fold_talking_follows_talk_start_and_end() {
    assert!(fold_talking(false, NoUserBreakSignal::TalkStarted));
    assert!(!fold_talking(true, NoUserBreakSignal::TalkEnded));
    for talking in [false, true] {
        assert_eq!(fold_talking(talking, NoUserBreakSignal::Enter), talking);
        assert_eq!(fold_talking(talking, NoUserBreakSignal::Leave), talking);
    }
}
