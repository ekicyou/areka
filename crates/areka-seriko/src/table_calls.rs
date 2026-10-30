//! 再生表のコマを描くコマと働きのコマに分ける（spec: seriko-pattern-calls 要件 1.1・1.3・1.5・2.4・
//! 7.4）。
//!
//! 8 語の見分けと相手の欄の読みは `areka_emo_compose::pattern_word` が唯一の正本で、ここはその答えを
//! 再生表のコマ（[`LoopFrame`]）の働き（[`FrameAct`]）へ写すだけ。働きのコマの合成メソッドは記録なしの
//! `ComposeMethod::Unknown`（使われない）で、絵の番号は持たない。

use areka_emo_compose::{ComposeMethod, PatternWord, call_targets, pattern_word};
use areka_parsers::shell::Pattern;

use crate::table::LoopFrame;

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
        // 記録は表を作った後の検査が出す（task 3.2）。
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

#[cfg(test)]
#[path = "table_calls_tests.rs"]
mod tests;
