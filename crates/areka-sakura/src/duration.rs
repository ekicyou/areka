//! テキスト再生時間の計算層（duration）— テキスト 1 チャンクの暗黙 per-char 再生時間を
//! 算出する唯一の純関数を持つ SakuraScript 固有モジュール。
//!
//! [`text_playback_duration`] は「このテキストを喋り終えるのに何秒かかるか」を、実時間・
//! レイアウト・描画状態に一切依存せず **文字数だけ** から決定的に算出する純関数（no I/O・
//! GPU/窓/COM 非依存・R3.1/3.2/3.5）。compile 層はこの D を各テキスト cue の envelope
//! duration へ焼き込み、offset を D の整数 ms だけ進めて後続 cue の絶対時刻を整列する（合成は compile の
//! 責務・本関数は明示 `\_w` を畳まない・R3.4）。
//!
//! per-char ノミナルウェイト定数 [`CHAR_NOMINAL_MS`] は **本モジュールが唯一の定義箇所**
//! （R3.3）。parser の `WAIT_UNIT_MS`（`\w` 単位）・emo-text の旧 `char_wait`（task 6.1 で
//! 撤去）とは**別概念**であり、混同・統合・相互参照しない。

use crate::cluster::cluster_count;

/// per-char ノミナルウェイト（ミリ秒・**唯一の定義箇所**・R3.3）。
///
/// テキスト 1 文字（書記素クラスタ 1 つ）あたりの暗黙再生時間。parser `WAIT_UNIT_MS`（`\w` の単位）や emo-text の
/// 旧 `char_wait` とは別概念ゆえ、流用・統合しない。dola（演出タイミング基盤）はこの
/// SakuraScript 固有の値を内包せず、算出済みの秒数を不透明に受け取る（R9.2）。
pub const CHAR_NOMINAL_MS: u64 = 50;

/// テキスト 1 チャンクの暗黙 per-char 再生時間 D（秒）を算出する純関数（R3.1）。
///
/// `D = cluster_count(text) * CHAR_NOMINAL_MS` を秒へ変換した値。数える単位は拡張書記素
/// クラスタ（[`crate::cluster`] が唯一の定義点・emo-text の 1 段と同じ切り方・バイト数でも
/// UTF-16 単位でも `char` でもない）。ZWJ 列のような複数スカラー値の絵文字も 1 つ 50 ms。
///
/// 実時間・レイアウト・描画状態に依存せず、同一入力へ常に同一値を返す（決定的・R3.2）。
/// FP 決定性のため整数 ms 算術＋単一変換（[`ms_to_secs`]）で、compile の開始時刻の積みと同じ
/// 格子に乗せる（50 ms の倍では `Duration::as_secs_f64` と同じビット）。
///
/// 明示ウェイト（`\_w`/`\w`）は**畳まない**（R3.4）——それらは parser が `Instruction::Wait`
/// へ分離済みで、暗黙 per-char との合成は compile のタイムライン累積が担う（二重計上回避）。
///
/// 事前条件: なし（任意の `&str`）。
/// 事後条件: 戻り値は有限・非負。空文字列は 0.0。
pub fn text_playback_duration(text: &str) -> f64 {
    ms_to_secs(text_playback_ms(text))
}

/// テキスト 1 チャンクの暗黙再生時間（整数 ms）＝`cluster_count × CHAR_NOMINAL_MS`。compile が
/// 開始時刻を整数 ms で積むのに使う（指摘 8）。
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

    /// 空文字列は再生時間 0.0（事後条件・R3.2）。
    #[test]
    fn empty_text_has_zero_duration() {
        assert_eq!(text_playback_duration(""), 0.0);
    }

    /// N 文字のテキストは N×CHAR_NOMINAL_MS 秒。期待値は同一の `Duration::from_millis`
    /// ＋`as_secs_f64()` で算出（10 進リテラル直書きの IEEE-754 表現誤差を排除）。
    /// "こんにちは" は 5 文字（5 クラスタ）。
    #[test]
    fn char_count_times_nominal_ms_in_seconds() {
        let expected = Duration::from_millis(5 * CHAR_NOMINAL_MS).as_secs_f64();
        assert_eq!(text_playback_duration("こんにちは"), expected);
    }

    /// 数える単位は書記素クラスタ——バイト数でも UTF-16 単位でもない。
    /// "aあ🦆" は 3 クラスタ（1+3+4=8 バイト・🦆 は UTF-16 で 2 単位）で 3 単位で算出される。
    #[test]
    fn counts_clusters_not_bytes_or_utf16_units() {
        // 前提の可視化: バイト数・クラスタ数の乖離を固定。
        assert_eq!("aあ🦆".len(), 8, "UTF-8 バイト数");
        assert_eq!(cluster_count("aあ🦆"), 3, "クラスタ数");

        let expected = Duration::from_millis(3 * CHAR_NOMINAL_MS).as_secs_f64();
        assert_eq!(text_playback_duration("aあ🦆"), expected);
    }

    /// ZWJ 列 1 つ（5 スカラー値）はクラスタ 1 つ＝50 ms（部品の数で数えない・要件 2.6）。
    #[test]
    fn zwj_sequence_counts_as_one_cluster() {
        let family = "👨\u{200D}👩\u{200D}👧";
        assert_eq!(family.chars().count(), 5, "スカラー値の数");

        let expected = Duration::from_millis(CHAR_NOMINAL_MS).as_secs_f64();
        assert_eq!(text_playback_duration(family), expected);
    }
}
