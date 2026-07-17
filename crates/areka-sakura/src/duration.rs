//! テキスト再生時間の計算層（duration）— テキスト 1 チャンクの暗黙 per-char 再生時間を
//! 算出する唯一の純関数を持つ SakuraScript 固有モジュール。
//!
//! [`text_playback_duration`] は「このテキストを喋り終えるのに何秒かかるか」を、実時間・
//! レイアウト・描画状態に一切依存せず **文字数だけ** から決定的に算出する純関数（no I/O・
//! GPU/窓/COM 非依存・R3.1/3.2/3.5）。compile 層はこの D を各テキスト cue の envelope
//! duration へ焼き込み、`offset += D` で後続 cue の絶対時刻を整列する（合成は compile の
//! 責務・本関数は明示 `\_w` を畳まない・R3.4）。
//!
//! per-char ノミナルウェイト定数 [`CHAR_NOMINAL_MS`] は **本モジュールが唯一の定義箇所**
//! （R3.3）。parser の `WAIT_UNIT_MS`（`\w` 単位）・emo-text の旧 `char_wait`（task 6.1 で
//! 撤去）とは**別概念**であり、混同・統合・相互参照しない。

use std::time::Duration;

/// per-char ノミナルウェイト（ミリ秒・**唯一の定義箇所**・R3.3）。
///
/// テキスト 1 文字あたりの暗黙再生時間。parser `WAIT_UNIT_MS`（`\w` の単位）や emo-text の
/// 旧 `char_wait` とは別概念ゆえ、流用・統合しない。dola（演出タイミング基盤）はこの
/// SakuraScript 固有の値を内包せず、算出済みの秒数を不透明に受け取る（R9.2）。
pub const CHAR_NOMINAL_MS: u64 = 50;

/// テキスト 1 チャンクの暗黙 per-char 再生時間 D（秒）を算出する純関数（R3.1）。
///
/// `D = char_count(text) * CHAR_NOMINAL_MS` を秒へ変換した値。グリフ単位は Rust `char`
/// （emo-text と同一の M1 正準・バイト数でも UTF-16 単位でもない）。
///
/// 実時間・レイアウト・描画状態に依存せず、同一入力へ常に同一値を返す（決定的・R3.2）。
/// FP 決定性のため整数 ms 算術＋単一変換（`Duration::from_millis(N*CHAR_NOMINAL_MS)
/// .as_secs_f64()`）で既存の `Duration→f64` 規律（compile.rs）に揃える。
///
/// 明示ウェイト（`\_w`/`\w`）は**畳まない**（R3.4）——それらは parser が `Instruction::Wait`
/// へ分離済みで、暗黙 per-char との合成は compile のタイムライン累積が担う（二重計上回避）。
///
/// 事前条件: なし（任意の `&str`）。
/// 事後条件: 戻り値は有限・非負。空文字列は 0.0。
pub fn text_playback_duration(text: &str) -> f64 {
    let char_count = text.chars().count() as u64;
    Duration::from_millis(char_count * CHAR_NOMINAL_MS).as_secs_f64()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 空文字列は再生時間 0.0（事後条件・R3.2）。
    #[test]
    fn empty_text_has_zero_duration() {
        assert_eq!(text_playback_duration(""), 0.0);
    }

    /// N 文字のテキストは N×CHAR_NOMINAL_MS 秒。期待値は同一の `Duration::from_millis`
    /// ＋`as_secs_f64()` で算出（10 進リテラル直書きの IEEE-754 表現誤差を排除）。
    /// "こんにちは" は 5 文字（char 単位）。
    #[test]
    fn char_count_times_nominal_ms_in_seconds() {
        let expected = Duration::from_millis(5 * CHAR_NOMINAL_MS).as_secs_f64();
        assert_eq!(text_playback_duration("こんにちは"), expected);
    }

    /// グリフ単位は Rust `char`——バイト数でも UTF-16 単位でもない（key cage）。
    /// "aあ🦆" は 3 char（1+3+4=8 バイト・🦆 は UTF-16 で 2 単位）だが 3 単位で算出される。
    #[test]
    fn counts_chars_not_bytes_or_utf16_units() {
        // 前提の可視化: バイト数・char 数の乖離を固定。
        assert_eq!("aあ🦆".len(), 8, "UTF-8 バイト数");
        assert_eq!("aあ🦆".chars().count(), 3, "char 数");

        let expected = Duration::from_millis(3 * CHAR_NOMINAL_MS).as_secs_f64();
        assert_eq!(text_playback_duration("aあ🦆"), expected);
    }

    /// 決定的: 同一入力を再計算しても常にビット同一の値を返す（R3.2・実時間非依存）。
    #[test]
    fn is_deterministic_for_identical_input() {
        let text = "同じ入力 aあ🦆";
        let first = text_playback_duration(text);
        let second = text_playback_duration(text);
        assert_eq!(first.to_bits(), second.to_bits(), "再計算で値が異なる");
    }

    /// 事後条件（NaN/∞/負の放電）: 代表的入力について戻り値は有限・非負。
    #[test]
    fn result_is_finite_and_non_negative() {
        for text in ["", "a", "こんにちは", "aあ🦆", "  ", "\n\t"] {
            let d = text_playback_duration(text);
            assert!(d.is_finite(), "{text:?} の D が非有限");
            assert!(d >= 0.0, "{text:?} の D が負");
        }
    }
}
