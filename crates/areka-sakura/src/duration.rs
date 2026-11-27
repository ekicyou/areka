//! テキスト再生時間の計算層（duration）— テキスト 1 チャンクの暗黙 per-char 再生時間を
//! 算出する唯一の純関数と、作者の書くミリ秒を長さへ変える縁を持つ SakuraScript 固有モジュール。
//!
//! [`text_playback_duration`] は実時間・レイアウト・描画状態に依存せず **文字数だけ** から
//! 決定的に長さを算出する（I/O・GPU・窓・COM 非依存）。compile 層はこの D を各テキスト cue の
//! envelope duration へ焼き込み、offset を D だけ進めて後続 cue の時刻を整列する。
//! 本関数は明示 `\_w` を畳まない。
//!
//! [`CHAR_NOMINAL_MS`] は **本モジュールが唯一の定義箇所**。parser の `WAIT_UNIT_MS`（`\w` 単位）・
//! emo-text の旧 `char_wait` とは別概念であり、混同・統合しない。

use crate::cluster::cluster_count;
use crate::contract::TimeSpan;

/// per-char ノミナルウェイト（ミリ秒・**唯一の定義箇所**）。
///
/// テキスト 1 文字（書記素クラスタ 1 つ）あたりの暗黙再生時間。dola（演出タイミング基盤）は
/// この SakuraScript 固有の値を内包せず、算出済みの長さを不透明に受け取る。
pub const CHAR_NOMINAL_MS: u64 = 50;

/// テキスト 1 チャンクの暗黙 per-char 再生時間 D を算出する純関数。
///
/// `D = cluster_count(text) * CHAR_NOMINAL_MS`。数える単位は拡張書記素
/// クラスタ（[`crate::cluster`] が唯一の定義点・emo-text の 1 段と同じ切り方・バイト数でも
/// UTF-16 単位でも `char` でもない）。ZWJ 列のような複数スカラー値の絵文字も 1 つ 50 ms。
///
/// 明示ウェイト（`\_w`/`\w`）は畳まない。それらは parser が `Instruction::Wait` へ分離済みで、
/// 暗黙 per-char との合成は compile のタイムライン累積が担う（二重計上回避）。
///
/// 空文字列は 0。
pub fn text_playback_duration(text: &str) -> TimeSpan {
    ms_to_span((cluster_count(text) as u64).saturating_mul(CHAR_NOMINAL_MS))
}

/// 作者の書くミリ秒を長さにする唯一の式。表せない長さは上限に張り付く。
pub(crate) fn ms_to_span(ms: u64) -> TimeSpan {
    TimeSpan::try_from_millis(ms).unwrap_or(TimeSpan::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use dola::time::testing::ms;

    #[test]
    fn empty_text_has_zero_duration() {
        assert_eq!(text_playback_duration(""), TimeSpan::ZERO);
    }

    #[test]
    fn char_count_times_nominal_ms() {
        assert_eq!(text_playback_duration("こんにちは"), ms(250));
    }

    /// "aあ🦆" は 3 クラスタ（1+3+4=8 バイト・🦆 は UTF-16 で 2 単位）で 3 単位で算出される。
    #[test]
    fn counts_clusters_not_bytes_or_utf16_units() {
        assert_eq!("aあ🦆".len(), 8, "UTF-8 バイト数");
        assert_eq!(cluster_count("aあ🦆"), 3, "クラスタ数");
        assert_eq!(text_playback_duration("aあ🦆"), ms(150));
    }

    /// ZWJ 列 1 つ（5 スカラー値）はクラスタ 1 つ＝50 ms（部品の数で数えない）。
    #[test]
    fn zwj_sequence_counts_as_one_cluster() {
        let family = "👨\u{200D}👩\u{200D}👧";
        assert_eq!(family.chars().count(), 5, "スカラー値の数");
        assert_eq!(text_playback_duration(family), ms(50));
    }

    #[test]
    fn unrepresentable_millis_stick_to_the_max() {
        assert_eq!(ms_to_span(u64::MAX), TimeSpan::MAX);
    }
}
