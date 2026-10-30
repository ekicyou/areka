//! 再生表のコマを描くコマと働きのコマに分ける（spec: seriko-pattern-calls 要件 1.1・1.3・1.5・2.4・
//! 7.4）。
//!
//! 8 語の見分けと相手の欄の読みは `areka_emo_compose::pattern_word` が唯一の正本で、ここはその答えを
//! 再生表のコマ（[`LoopFrame`]）の働き（[`FrameAct`]）へ写すだけ。働きのコマの合成メソッドは記録なしの
//! `ComposeMethod::Unknown`（使われない）で、絵の番号は持たない。
//!
//! 面ごとの表を作った後に 8 語の誤りを検査し、1 件ごとに 1 度だけ `warn!` を出す（[`check_calls`]・
//! 要件 2.5・5.1・5.3・5.4・6.4〜6.6・8.3）。表を作るのは読み込みにつき 1 度なので、刻みや合成の
//! たびには出ない。

use std::collections::{BTreeMap, BTreeSet};

use areka_emo_compose::{ComposeMethod, EmoWorld, PatternWord, call_targets, pattern_word};
use areka_parsers::shell::{Animation, Interval, Pattern};

use crate::table::{LoopAnimation, LoopFrame};

/// 再生表のコマの働き。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameAct {
    /// 描くコマ（今までのコマ）。
    Draw,
    /// start・parallelstart（`pick_one` は偽）／alternativestart（`pick_one` は真）。
    Start {
        /// 相手の animation の番号（書いた順・空でない）。
        targets: Box<[u32]>,
        /// 相手から 1 つを選ぶか。
        pick_one: bool,
    },
    /// stop・parallelstop（`pick_one` は偽）／alternativestop（`pick_one` は真）。
    Stop {
        /// 相手の animation の番号（書いた順・空でない）。
        targets: Box<[u32]>,
        /// 相手から 1 つを選ぶか。
        pick_one: bool,
    },
    /// move: このコマの x・y が元の表示位置からのずれ（面の画素）。
    Move,
    /// 何もしない（insert・読めない相手）。
    Inert,
}

/// pattern定義 1 行を再生表のコマにする。8 語以外は今と同じ（合成メソッドの解決と未知の語の記録）。
/// 呼び出しの 6 語・`insert`・読めない相手の待ちは 0、`move` は待ちと X・Y をそのまま持ち、欄 2 を
/// 読まない。
pub(crate) fn loop_frame(p: &Pattern) -> LoopFrame {
    let word = pattern_word(p.method.as_str());
    let call = |start: bool, pick_one: bool| match call_targets(word, &p.target) {
        Ok(targets) if start => FrameAct::Start { targets, pick_one },
        Ok(targets) => FrameAct::Stop { targets, pick_one },
        // 記録は表を作った後の検査（`check_calls`）が出す。
        Err(_) => FrameAct::Inert,
    };
    let act = match word {
        PatternWord::Draw => {
            return LoopFrame {
                surface_id: p.surface_id,
                picture: None,
                method: ComposeMethod::from_name(p.method.as_str()),
                act: FrameAct::Draw,
                wait_ms: p.wait,
                x: p.x,
                y: p.y,
            };
        }
        PatternWord::Start | PatternWord::ParallelStart => call(true, false),
        PatternWord::AlternativeStart => call(true, true),
        PatternWord::Stop | PatternWord::ParallelStop => call(false, false),
        PatternWord::AlternativeStop => call(false, true),
        PatternWord::Insert => FrameAct::Inert,
        PatternWord::Move => FrameAct::Move,
    };
    let (wait_ms, x, y) = match act {
        FrameAct::Move => (p.wait, p.x, p.y),
        _ => (0, 0, 0),
    };
    LoopFrame {
        surface_id: p.surface_id,
        picture: None,
        method: ComposeMethod::Unknown(p.method.as_str().into()),
        act,
        wait_ms,
        x,
        y,
    }
}

/// 8 語の誤り 1 件の記録（面・animation・pattern・語・原文の欄）。
fn warn_call(surface_id: u32, a: &Animation, p: &Pattern, why: &str) {
    tracing::warn!(
        surface_id,
        animation_id = a.id,
        pattern = p.index,
        word = p.method.as_str(),
        target = p.target.as_str(),
        "seriko table: {why}"
    );
}

/// 面ごとの表を作った後に、8 語の誤りを 6 種に分けて 1 件ごとに 1 度だけ記録する。純 `bind` の
/// 呼び出しの 6 語と `move` は、表のコマを何もしないコマ（`Inert`）に変える（要件 5.3）。
pub(crate) fn check_calls(world: &EmoWorld, table: &mut BTreeMap<u32, Vec<LoopAnimation>>) {
    for surface_id in world.surface_ids() {
        let Some(master) = world.surface(surface_id) else {
            continue;
        };
        let listed: BTreeSet<u32> = table
            .get(&surface_id)
            .map(|v| v.iter().map(|a| a.id).collect())
            .unwrap_or_default();
        let is_bind = |a: &Animation| matches!(a.interval, Interval::Bind);
        // 純 `bind` の `insert` の辺（巡回の検査に使う）。
        let edges: BTreeMap<u32, Vec<u32>> = master
            .animations
            .iter()
            .filter(|a| is_bind(a))
            .map(|a| {
                let to = a
                    .patterns
                    .iter()
                    .filter(|p| pattern_word(p.method.as_str()) == PatternWord::Insert)
                    .filter_map(|p| call_targets(PatternWord::Insert, &p.target).ok())
                    .map(|t| t[0])
                    .collect();
                (a.id, to)
            })
            .collect();
        for a in &master.animations {
            // 表のコマは pattern の番号の昇順に並ぶ（table.rs と同じ並べ方）。
            let mut patterns: Vec<&Pattern> = a.patterns.iter().collect();
            patterns.sort_by_key(|p| p.index);
            for (i, p) in patterns.into_iter().enumerate() {
                let word = pattern_word(p.method.as_str());
                match word {
                    PatternWord::Draw => {}
                    PatternWord::Insert if !is_bind(a) => warn_call(
                        surface_id,
                        a,
                        p,
                        "着せ替え（純 bind）でない animation の insert は何もしない（要件 6.6）",
                    ),
                    PatternWord::Insert => match call_targets(word, &p.target) {
                        Err(_) => warn_call(
                            surface_id,
                            a,
                            p,
                            "相手の欄が読めないコマは何もしない（要件 2.5）",
                        ),
                        Ok(t) if !edges.contains_key(&t[0]) => warn_call(
                            surface_id,
                            a,
                            p,
                            "insert の相手が面に無いか純 bind でないので何も描かない（要件 6.5）",
                        ),
                        Ok(t) if reaches(&edges, t[0], a.id) => warn_call(
                            surface_id,
                            a,
                            p,
                            "insert が巡回するので打ち切る（要件 6.4）",
                        ),
                        Ok(_) => {}
                    },
                    _ if is_bind(a) => {
                        warn_call(
                            surface_id,
                            a,
                            p,
                            "着せ替え（純 bind）の呼び出しと move は何もしない（要件 5.3）",
                        );
                        if let Some(f) = table
                            .get_mut(&surface_id)
                            .and_then(|v| v.iter_mut().find(|l| l.id == a.id))
                            .and_then(|l| l.frames.get_mut(i))
                        {
                            f.act = FrameAct::Inert;
                            (f.wait_ms, f.x, f.y) = (0, 0, 0);
                        }
                    }
                    PatternWord::Move => {}
                    _ => match call_targets(word, &p.target) {
                        Err(_) => warn_call(
                            surface_id,
                            a,
                            p,
                            "相手の欄が読めないコマは何もしない（要件 2.5）",
                        ),
                        Ok(targets) => {
                            for missing in targets.iter().filter(|t| !listed.contains(t)) {
                                tracing::warn!(
                                    surface_id,
                                    animation_id = a.id,
                                    pattern = p.index,
                                    word = p.method.as_str(),
                                    target = p.target.as_str(),
                                    missing,
                                    "seriko table: 呼び出しの相手が面の再生表に無いのでその相手には何もしない（要件 5.1）"
                                );
                            }
                        }
                    },
                }
            }
        }
    }
}

/// 純 `bind` の `insert` の辺をたどって、`from` から `to` へ着くか。
fn reaches(edges: &BTreeMap<u32, Vec<u32>>, from: u32, to: u32) -> bool {
    let mut seen = BTreeSet::new();
    let mut stack = vec![from];
    while let Some(id) = stack.pop() {
        if id == to {
            return true;
        }
        if seen.insert(id) {
            stack.extend(edges.get(&id).into_iter().flatten().copied());
        }
    }
    false
}

#[cfg(test)]
#[path = "table_calls_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "table_calls_check_tests.rs"]
mod check_tests;
