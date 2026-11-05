//! 外形の 2 つの走査（面ごとの外形 `plan_extent.rs`・シェルの外形 `plan_bounds.rs`）が共有するたどり方
//! （指摘 84）。明示のスタックで深さ優先に回し、経路の集合で循環を避け、入る面を予算で数え、きれいに
//! 終わった面の答えを覚える。面ごとの寄与（自分の層の外形と、子へ入る積み荷）は呼び手が渡す。

use std::collections::{HashMap, HashSet};

use crate::budget::Budget;

/// 走査の積み荷: 面へ親の原点から `(x, y)` ずらして入る（`quiet` なら循環を黙って飛ばす）か、面から出る。
pub(crate) enum Step {
    Enter {
        surface: u32,
        x: i64,
        y: i64,
        quiet: bool,
    },
    Exit,
}

/// 走査の段: 面と、親の原点からの位置と、段の下で重ねた答え（その面の原点からの相対・何も数えて
/// いなければ `None`）と、きれいに終わるかと、段の下で予算に数えた数。
struct Level<R> {
    surface: u32,
    x: i64,
    y: i64,
    acc: Option<R>,
    clean: bool,
    spent: usize,
}

/// きれいに終わった面（その下で先祖へ戻る辺を捨てず、予算にも断られなかった面）の、下で予算に数えた数と
/// 答え（その面の原点からの相対）。きれいに終わった面の下に循環は無いので、どの経路から入っても答えは同じ。
pub(crate) type Memo<R> = HashMap<u32, (usize, Option<R>)>;

/// 走査の作業場（経路にある面の集合・積み荷の列・段の列）。どれも走査が終わると空に戻る。
pub(crate) struct Scratch<R> {
    on_path: HashSet<u32>,
    stack: Vec<Step>,
    levels: Vec<Level<R>>,
}

impl<R> Default for Scratch<R> {
    fn default() -> Self {
        Self {
            on_path: HashSet::new(),
            stack: Vec::new(),
            levels: Vec::new(),
        }
    }
}

/// 根の面 `root` からたどり、根の原点からの答えを返す（根の段が何も数えなければ `None`）。
///
/// `own(surface, children)` は面の自分の寄与（その面の原点からの相対）を返し、子を `children` へ
/// 取り出したい順に積む（逆にして上から取り出す＝深さ優先）。面の表に無ければ何も積まず `None` を返す。
/// `merge(acc, r, x, y)` は `r` を `(x, y)` ずらして `acc` に重ねる。根の面は予算に数えない。
///
/// `memo` が在れば、きれいに終わった面の答えを覚え、後で同じ面に入るとき予算の残りが「入る 1 ＋下で数えた
/// 数」以上ならそれだけ一度に引いて覚えた答えを使う（足りなければ今どおり入る＝答えと記録は覚えが無いときと
/// 同じ）。予算が尽きたら `name`（走査の名前）と根の面で `error!` を 1 行出す。
pub(crate) fn walk<R: Copy>(
    s: &mut Scratch<R>,
    mut memo: Option<&mut Memo<R>>,
    root: u32,
    limit: usize,
    name: &'static str,
    merge: fn(&mut Option<R>, Option<R>, i64, i64),
    mut own: impl FnMut(u32, &mut Vec<Step>) -> Option<R>,
) -> Option<R> {
    s.on_path.clear();
    s.stack.clear();
    s.levels.clear();
    let mut budget = Budget::new(limit);
    let mut answer = None;
    s.stack.push(Step::Enter {
        surface: root,
        x: 0,
        y: 0,
        quiet: false,
    });
    while let Some(step) = s.stack.pop() {
        let Step::Enter {
            surface,
            x,
            y,
            quiet,
        } = step
        else {
            // 段が尽きた: 経路から外し（非循環の重複参照は別経路で再走査可能・要件 7.1）、きれいなら覚え、
            // 答えを親の原点へずらして重ねる（根なら答え）。
            let Some(done) = s.levels.pop() else {
                break;
            };
            let removed = s.on_path.remove(&done.surface);
            debug_assert!(removed, "外形の走査: 経路の集合は段と揃う");
            if done.clean
                && let Some(m) = memo.as_deref_mut()
            {
                m.insert(done.surface, (done.spent, done.acc));
            }
            match s.levels.last_mut() {
                Some(up) => {
                    up.spent += done.spent + 1;
                    up.clean &= done.clean;
                    merge(&mut up.acc, done.acc, done.x, done.y);
                }
                None => answer = done.acc,
            }
            continue;
        };
        // 循環検出（ops 経路と同一規律・要件 7.2/7.3）: 現在の祖先経路に既出なら打ち切り。`quiet` でない
        // 積み荷（面ごとの外形の pattern の先）の知らせは ops 経路と同じく debug。
        if s.on_path.contains(&surface) {
            if !quiet {
                tracing::debug!(
                    target: "areka_emo_compose",
                    surface_id = surface,
                    "外形算出: 入れ子参照の循環を検出・枝を打ち切り"
                );
            }
            if let Some(up) = s.levels.last_mut() {
                up.clean = false;
            }
            continue;
        }
        // 根の面（段が無い）は予算に数えない。
        if let Some(up) = s.levels.last_mut() {
            if let Some(&(n, r)) = memo.as_deref().and_then(|m| m.get(&surface))
                && budget.take(n + 1)
            {
                up.spent += n + 1;
                merge(&mut up.acc, r, x, y);
                continue;
            }
            if !budget.enter(name, root) {
                up.clean = false;
                continue;
            }
        }
        s.on_path.insert(surface);
        s.stack.push(Step::Exit);
        // 子の積み荷はここから先へ今までの順に積み、最後に逆にする（先に取り出すものを上へ）。
        let children = s.stack.len();
        let acc = own(surface, &mut s.stack);
        s.levels.push(Level {
            surface,
            x,
            y,
            acc,
            clean: true,
            spent: 0,
        });
        s.stack[children..].reverse();
    }
    answer
}
