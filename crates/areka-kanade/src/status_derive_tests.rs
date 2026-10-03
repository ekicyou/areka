//! 導出表の全組み合わせを決定論で固定する兄弟テスト（要件 8.1）。
//!
//! talk × choice × nouserbreak × online × balloon{なし・1 キャラクター・2 キャラクター} の
//! 48 通りについて、期待する `render()` の値（または行なし＝`None`）を**表に逐語で**持つ。
//! 期待値は導出のロジックで計算せず、正典の語彙順（要件 1.1）どおりの文字列を手で書く。

use super::*;

/// 表の balloon 軸。`Two` は 2 キャラクター（`\0` が 2 番・`\1` が 0 番）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Balloons {
    Nil,
    One,
    Two,
}
use Balloons::{Nil, One, Two};

fn binding(character_id: u32, balloon_id: u32) -> BalloonBinding {
    BalloonBinding {
        character_id,
        balloon_id,
    }
}

impl Balloons {
    fn bindings(self) -> Vec<BalloonBinding> {
        match self {
            Nil => Vec::new(),
            One => vec![binding(0, 0)],
            Two => vec![binding(0, 2), binding(1, 0)],
        }
    }
}

/// (talk, choice, nouserbreak, online, balloon, 期待する `render()`)。
type Row = (bool, bool, bool, bool, Balloons, Option<&'static str>);

#[rustfmt::skip]
const TABLE: [Row; 48] = [
    (false, false, false, false, Nil, None),
    (false, false, false, false, One, Some("balloon(0=0)")),
    (false, false, false, false, Two, Some("balloon(0=2/1=0)")),
    (false, false, false, true, Nil, Some("online")),
    (false, false, false, true, One, Some("online,balloon(0=0)")),
    (false, false, false, true, Two, Some("online,balloon(0=2/1=0)")),
    (false, false, true, false, Nil, Some("nouserbreak")),
    (false, false, true, false, One, Some("nouserbreak,balloon(0=0)")),
    (false, false, true, false, Two, Some("nouserbreak,balloon(0=2/1=0)")),
    (false, false, true, true, Nil, Some("nouserbreak,online")),
    (false, false, true, true, One, Some("nouserbreak,online,balloon(0=0)")),
    (false, false, true, true, Two, Some("nouserbreak,online,balloon(0=2/1=0)")),
    (false, true, false, false, Nil, Some("choosing")),
    (false, true, false, false, One, Some("choosing,balloon(0=0)")),
    (false, true, false, false, Two, Some("choosing,balloon(0=2/1=0)")),
    (false, true, false, true, Nil, Some("choosing,online")),
    (false, true, false, true, One, Some("choosing,online,balloon(0=0)")),
    (false, true, false, true, Two, Some("choosing,online,balloon(0=2/1=0)")),
    (false, true, true, false, Nil, Some("choosing,nouserbreak")),
    (false, true, true, false, One, Some("choosing,nouserbreak,balloon(0=0)")),
    (false, true, true, false, Two, Some("choosing,nouserbreak,balloon(0=2/1=0)")),
    (false, true, true, true, Nil, Some("choosing,nouserbreak,online")),
    (false, true, true, true, One, Some("choosing,nouserbreak,online,balloon(0=0)")),
    (false, true, true, true, Two, Some("choosing,nouserbreak,online,balloon(0=2/1=0)")),
    (true, false, false, false, Nil, Some("talking")),
    (true, false, false, false, One, Some("talking,balloon(0=0)")),
    (true, false, false, false, Two, Some("talking,balloon(0=2/1=0)")),
    (true, false, false, true, Nil, Some("talking,online")),
    (true, false, false, true, One, Some("talking,online,balloon(0=0)")),
    (true, false, false, true, Two, Some("talking,online,balloon(0=2/1=0)")),
    (true, false, true, false, Nil, Some("talking,nouserbreak")),
    (true, false, true, false, One, Some("talking,nouserbreak,balloon(0=0)")),
    (true, false, true, false, Two, Some("talking,nouserbreak,balloon(0=2/1=0)")),
    (true, false, true, true, Nil, Some("talking,nouserbreak,online")),
    (true, false, true, true, One, Some("talking,nouserbreak,online,balloon(0=0)")),
    (true, false, true, true, Two, Some("talking,nouserbreak,online,balloon(0=2/1=0)")),
    (true, true, false, false, Nil, Some("talking,choosing")),
    (true, true, false, false, One, Some("talking,choosing,balloon(0=0)")),
    (true, true, false, false, Two, Some("talking,choosing,balloon(0=2/1=0)")),
    (true, true, false, true, Nil, Some("talking,choosing,online")),
    (true, true, false, true, One, Some("talking,choosing,online,balloon(0=0)")),
    (true, true, false, true, Two, Some("talking,choosing,online,balloon(0=2/1=0)")),
    (true, true, true, false, Nil, Some("talking,choosing,nouserbreak")),
    (true, true, true, false, One, Some("talking,choosing,nouserbreak,balloon(0=0)")),
    (true, true, true, false, Two, Some("talking,choosing,nouserbreak,balloon(0=2/1=0)")),
    (true, true, true, true, Nil, Some("talking,choosing,nouserbreak,online")),
    (true, true, true, true, One, Some("talking,choosing,nouserbreak,online,balloon(0=0)")),
    (true, true, true, true, Two, Some("talking,choosing,nouserbreak,online,balloon(0=2/1=0)")),
];

fn render(snapshot: &ExecutionSnapshot) -> Option<String> {
    ExecutionStatus::derive(snapshot).render()
}

/// 要件 8.1/1.1/1.2/6.1: 48 通りすべてで `render()` が表の逐語の値と一致する
/// （正典順の連結・何も無ければ行なし・出どころの無い 5 状態は決して現れない）。
#[test]
fn derive_matches_the_verbatim_table_for_all_48_combinations() {
    for (talk, choice, no_user_break, online, balloons, expected) in TABLE {
        let snapshot = ExecutionSnapshot {
            talk_active: talk,
            choice_active: choice,
            no_user_break,
            online,
            balloons: balloons.bindings(),
        };
        assert_eq!(
            render(&snapshot).as_deref(),
            expected,
            "talk={talk} choice={choice} nouserbreak={no_user_break} online={online} \
             balloon={balloons:?} の導出が表と不一致"
        );
    }
}

/// 表そのものの健全性: 48 行が互いに異なる入力で、入力空間をちょうど覆う
/// （行の重複や欠けで網羅が崩れたら赤にする）。
#[test]
fn table_covers_each_input_exactly_once() {
    let mut inputs: Vec<_> = TABLE
        .iter()
        .map(|&(t, c, n, o, b, _)| (t, c, n, o, b as u8))
        .collect();
    inputs.sort_unstable();
    inputs.dedup();
    assert_eq!(inputs.len(), 48);
}

/// 要件 1.3: どの組み合わせでも同じ状態を 2 度含めない
/// （トップレベルの区切りは `,`・balloon の内部は `/` なので `,` で分けられる）。
#[test]
fn no_combination_repeats_a_state() {
    for (.., expected) in TABLE {
        let Some(value) = expected else { continue };
        let tokens: Vec<&str> = value.split(',').collect();
        let mut unique = tokens.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), tokens.len(), "{value} に重複した状態がある");
    }
}

/// 要件 4.3/1.3: 逆順に与えた組は昇順の `balloon(0=2/1=0)` になり、同じキャラクターの
/// 重複は先頭だけが残る（`balloon` は 1 つだけ）。
#[test]
fn reversed_and_duplicated_bindings_render_ascending_once() {
    let reversed = ExecutionSnapshot {
        balloons: vec![binding(1, 0), binding(0, 2)],
        ..ExecutionSnapshot::INACTIVE
    };
    assert_eq!(render(&reversed).as_deref(), Some("balloon(0=2/1=0)"));

    let duplicated = ExecutionSnapshot {
        talk_active: true,
        balloons: vec![binding(1, 0), binding(0, 2), binding(1, 5), binding(0, 7)],
        ..ExecutionSnapshot::INACTIVE
    };
    assert_eq!(
        render(&duplicated).as_deref(),
        Some("talking,balloon(0=2/1=0)")
    );
}

/// 要件 4.4: 空の組は `balloon` を出さず、他に何も無ければ行そのものを出さない。
#[test]
fn empty_bindings_emit_no_balloon() {
    let empty = ExecutionSnapshot {
        balloons: Vec::new(),
        ..ExecutionSnapshot::INACTIVE
    };
    assert_eq!(render(&empty), None);

    let online_empty = ExecutionSnapshot {
        online: true,
        balloons: Vec::new(),
        ..ExecutionSnapshot::INACTIVE
    };
    assert_eq!(render(&online_empty).as_deref(), Some("online"));
}
