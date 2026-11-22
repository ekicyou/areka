//! `shell_box_hover.rs` の兄弟テスト。
//!
//! 箱の名前はテストの中に持つ surfaces.txt の文面を畳んで取る（`box_layout`）。

use areka_emo_compose::BoxName;
use areka_emo_text::actor::{HitRectPx, ShownBox};

use super::{ShellBoxHover, settle_box_hover};
use crate::balloon_visibility::test_support::box_layout;

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

fn box_name(name: &str) -> BoxName {
    box_layout(SHELL)
        .placements(0)
        .iter()
        .find(|p| p.name.as_str() == name)
        .expect("文面に在る箱")
        .name
        .clone()
}

fn rect(left: f32, top: f32, right: f32, bottom: f32) -> HitRectPx {
    HitRectPx {
        left,
        top,
        right,
        bottom,
    }
}

fn shown() -> Vec<ShownBox> {
    vec![
        ShownBox {
            name: box_name("b"),
            element: 2,
            rect: rect(50.0, 20.0, 130.0, 60.0),
        },
        ShownBox {
            name: box_name("a"),
            element: 1,
            rect: rect(0.0, 0.0, 100.0, 50.0),
        },
    ]
}

/// 滞在している箱が `shown_boxes` から消えたら無し・残っていればそのまま・もともと無しなら無し。
#[test]
fn settle_drops_vanished_box() {
    let boxes = shown();
    let a = box_name("a");
    assert_eq!(settle_box_hover(Some(&a), &boxes), Some(a.clone()));
    assert_eq!(
        settle_box_hover(Some(&a), &boxes[..1]),
        None,
        "a だけ消えた"
    );
    assert_eq!(settle_box_hover(Some(&a), &[]), None);
    assert_eq!(settle_box_hover(None, &boxes), None);
}

/// 印が残らないことの通し: 箱に滞在している → 箱の文字が消える → 整える の後、観測の滞在は偽。
/// 他のスコープの記録には触れない。
#[test]
fn hover_mark_does_not_survive_vanished_text() {
    let mut hover = ShellBoxHover::default();
    hover.set(0, Some(box_name("a")));
    hover.set(1, Some(box_name("b")));

    let settled = settle_box_hover(hover.get(0), &[]);
    hover.set(0, settled);
    assert!(hover.get(0).is_none(), "時間切れの観測の hover は偽");
    assert_eq!(hover.get(1), Some(&box_name("b")), "他のスコープはそのまま");
}

/// 強調の記録は同じ箱の滞在の書き直しでは残り、別の箱へ替わると落ちる。滞在の無いスコープへは
/// 書かない。
#[test]
fn injected_survives_the_same_box_and_drops_on_another() {
    let mut hover = ShellBoxHover::default();
    hover.set(0, Some(box_name("a")));
    hover.set_injected(0, Some(2));

    hover.set(0, Some(box_name("a")));
    assert_eq!(hover.injected(0), Some(2), "同じ箱のままなら残る");

    hover.set(0, Some(box_name("b")));
    assert_eq!(hover.injected(0), None, "別の箱へ替わると落ちる");

    hover.set_injected(1, Some(3));
    assert_eq!((hover.get(1), hover.injected(1)), (None, None));
}
