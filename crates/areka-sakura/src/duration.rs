//! テキスト再生時間の計算層（duration）— テキスト 1 チャンクの暗黙 per-char 再生時間を
//! 算出する唯一の純関数を持つ SakuraScript 固有モジュール。
//!
//! [`text_playback_duration`] は実時間・レイアウト・描画状態に依存せず **文字数だけ** から
//! 決定的に秒数を算出する（I/O・GPU・窓・COM 非依存）。compile 層はこの D を各テキスト cue の
//! envelope duration へ焼き込み、offset を D の整数 ms だけ進めて後続 cue の絶対時刻を整列する。
//! 本関数は明示 `\_w` を畳まない。
//!
//! [`CHAR_NOMINAL_MS`] は **本モジュールが唯一の定義箇所**。parser の `WAIT_UNIT_MS`（`\w` 単位）・
//! emo-text の旧 `char_wait` とは別概念であり、混同・統合しない。

use crate::cluster::cluster_count;

/// per-char ノミナルウェイト（ミリ秒・**唯一の定義箇所**）。
///
/// テキスト 1 文字（書記素クラスタ 1 つ）あたりの暗黙再生時間。dola（演出タイミング基盤）は
/// この SakuraScript 固有の値を内包せず、算出済みの秒数を不透明に受け取る。
pub const CHAR_NOMINAL_MS: u64 = 50;

/// テキスト 1 チャンクの暗黙 per-char 再生時間 D（秒）を算出する純関数。
///
/// `D = cluster_count(text) * CHAR_NOMINAL_MS` を秒へ変換した値。数える単位は拡張書記素
/// クラスタ（[`crate::cluster`] が唯一の定義点・emo-text の 1 段と同じ切り方・バイト数でも
/// UTF-16 単位でも `char` でもない）。ZWJ 列のような複数スカラー値の絵文字も 1 つ 50 ms。
///
/// 同一入力へ常に同一値を返す。FP 決定性のため整数 ms 算術＋単一変換（[`ms_to_secs`]）で、
/// compile の開始時刻の積みと同じ格子に乗せる（50 ms の倍では `Duration::as_secs_f64` と同じビット）。
///
/// 明示ウェイト（`\_w`/`\w`）は畳まない。それらは parser が `Instruction::Wait` へ分離済みで、
/// 暗黙 per-char との合成は compile のタイムライン累積が担う（二重計上回避）。
///
/// 戻り値は有限・非負。空文字列は 0.0。
pub fn text_playback_duration(text: &str) -> f64 {
    ms_to_secs(text_playback_ms(text))
}

/// テキスト 1 チャンクの暗黙再生時間（整数 ms）＝`cluster_count × CHAR_NOMINAL_MS`。compile が
/// 開始時刻を整数 ms で積むのに使う。
pub(crate) fn text_playback_ms(text: &str) -> u64 {
    (cluster_count(text) as u64).saturating_mul(CHAR_NOMINAL_MS)
}

/// 整数 ms を秒にする唯一の式。dispatcher の Tick の式 `(now-base) as f64 / 1000.0` と同じにして、
/// 開始時刻と Tick の時刻を同じ格子に乗せる（`Duration::as_secs_f64` は 1118 ms などで 1 ulp ずれる）。
pub(crate) fn ms_to_secs(ms: u64) -> f64 {
    ms as f64 / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// 空文字列は再生時間 0.0。
    #[test]
    fn empty_text_has_zero_duration() {
        assert_eq!(text_playback_duration(""), 0.0);
    }

    /// N 文字のテキストは N×CHAR_NOMINAL_MS 秒。期待値は同一の `Duration::from_millis`
    /// ＋`as_secs_f64()` で算出（10 進リテラル直書きの IEEE-754 表現誤差を排除）。
    #[test]
    fn char_count_times_nominal_ms_in_seconds() {
        let expected = Duration::from_millis(5 * CHAR_NOMINAL_MS).as_secs_f64();
        assert_eq!(text_playback_duration("こんにちは"), expected);
    }

    /// 数える単位は書記素クラスタ——バイト数でも UTF-16 単位でもない。
    /// "aあ🦆" は 3 クラスタ（1+3+4=8 バイト・🦆 は UTF-16 で 2 単位）で 3 単位で算出される。
    #[test]
    fn counts_clusters_not_bytes_or_utf16_units() {
        assert_eq!("aあ🦆".len(), 8, "UTF-8 バイト数");
        assert_eq!(cluster_count("aあ🦆"), 3, "クラスタ数");

        let expected = Duration::from_millis(3 * CHAR_NOMINAL_MS).as_secs_f64();
        assert_eq!(text_playback_duration("aあ🦆"), expected);
    }

    /// ZWJ 列 1 つ（5 スカラー値）はクラスタ 1 つ＝50 ms（部品の数で数えない）。
    #[test]
    fn zwj_sequence_counts_as_one_cluster() {
        let family = "👨\u{200D}👩\u{200D}👧";
        assert_eq!(family.chars().count(), 5, "スカラー値の数");

        let expected = Duration::from_millis(CHAR_NOMINAL_MS).as_secs_f64();
        assert_eq!(text_playback_duration(family), expected);
    }
}
