//! pattern定義の描画メソッドの 8 語と、相手の欄（欄 2）の読み（seriko-pattern-calls
//! 要件 1.1・1.2・2.1〜2.3）。
//!
//! 8 語の表と綴りの比べ方の正本は `areka_parsers::shell::pattern_word`（emo-atlas のマニフェストと
//! 共有する）。ここはその型と関数を再公開し、相手の欄の読みを持つ。照会だけで、記録は出さない。

pub use areka_parsers::shell::{PatternWord, pattern_word};

/// 相手の欄が読めなかった（空・数でない・名前を含む・負の数・並びが空）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnreadableTarget;

/// 相手の欄（`Pattern.target`）を animation の番号の列として読む。
///
/// `start`・`stop`・`insert` は番号 1 つ。並びの 4 語は `(…)`／`[…]` の中を `,` か `.` で
/// 割った番号の列で、括弧の無い数だけの欄も 1 つの並びとして受ける。書いた順と重複を保つ。
/// `Draw` と `Move` は相手の欄を持たないので `Err` を返す。`Ok` の列は空でない。
pub fn call_targets(word: PatternWord, target: &str) -> Result<Box<[u32]>, UnreadableTarget> {
    let target = target.trim();
    match word {
        PatternWord::Start | PatternWord::Stop | PatternWord::Insert => {
            Ok(Box::new([number(target)?]))
        }
        PatternWord::AlternativeStart
        | PatternWord::AlternativeStop
        | PatternWord::ParallelStart
        | PatternWord::ParallelStop => {
            let inner = target
                .strip_prefix('(')
                .and_then(|t| t.strip_suffix(')'))
                .or_else(|| target.strip_prefix('[').and_then(|t| t.strip_suffix(']')))
                .unwrap_or(target);
            inner.split([',', '.']).map(number).collect()
        }
        PatternWord::Draw | PatternWord::Move => Err(UnreadableTarget),
    }
}

/// 欄の 1 つを animation の番号（0 以上の 10 進）として読む。
fn number(text: &str) -> Result<u32, UnreadableTarget> {
    let text = text.trim();
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return Err(UnreadableTarget);
    }
    text.parse().map_err(|_| UnreadableTarget)
}

#[cfg(test)]
#[path = "pattern_word_tests.rs"]
mod tests;
