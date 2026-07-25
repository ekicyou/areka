//! `ScaleRatio`: 表示スケールの数学（有理表現と丸めの単一権威）。
//!
//! DPI 追従表示の係数 k を**既約有理数** `num/den` で保持し、寸法の k 倍算（丸め）を
//! ここ 1 箇所へ集約する。`blit.rs` と同格の整数専用規約（決定性）に従い、画素・寸法演算に
//! 浮動小数（f32/f64）を一切持ち込まない。f32 が現れるのは照会契約の出口ビュー
//! [`ScaleRatio::as_f32`] のみである。
//!
//! - **既約正準化**（要件 1.1）: 構築時に gcd で約分し、`Eq`/`Hash` を正準形で厳密化する
//!   （下流 `emo-present` の合成キャッシュキーの一意性を担保する）。
//! - **丸め規約の単一権威**（要件 2.5）: [`ScaleRatio::scale_len`] ／
//!   [`ScaleRatio::scaled_extent`] は round half away from zero（wintf `DPI::to_physical_*`
//!   と同規約）で丸め、非ゼロ入力に最小 1px を保証する（拡大結果が消える欠けを作らない）。
//! - **乗算合成**（要件 1.6）: 最終拡大率＝アプリ管理拡大率 × DPI 由来 k を
//!   [`ScaleRatio::mul`] の有理数乗算として表現する（本仕様のアプリ管理拡大率は
//!   [`ScaleRatio::ONE`] 固定の縮退シーム）。
//! - **恒等**（要件 1.3）: 窓 DPI＝作者基準 DPI のとき k=1/1 となり、
//!   [`ScaleRatio::is_identity`] が真・`scale_len` は入力を素通しする（既存等倍表示と同一）。
//!
//! 公開 API はパニックしない（構築失敗は `Option`）。

/// 最大公約数（Euclid の互除法・u64 域）。
///
/// `gcd(a, 0) == a`。既約正準化と乗算合成の約分に用いる整数専用ヘルパ。
const fn gcd_u64(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

/// 既約正準の有理スケール（`num > 0`・`den > 0`・`gcd(num, den) == 1`）。
///
/// `Eq`/`Hash` は正準形で厳密——`120/96` と `5/4` は同一値として等価かつ同一ハッシュになる
/// （キャッシュキーの一意性・要件 1.1）。内部フィールドは非公開で、不変条件（非ゼロ・既約）は
/// [`ScaleRatio::new`]／[`ScaleRatio::mul`] のみが確立する。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScaleRatio {
    /// 分子（常に 1 以上・`den` と互いに素）。
    num: u32,
    /// 分母（常に 1 以上・`num` と互いに素）。
    den: u32,
}

impl Default for ScaleRatio {
    /// 既定は恒等 1/1（k 未確定時の縮退値・要件 1.3/1.4）。
    fn default() -> Self {
        ScaleRatio::ONE
    }
}

impl ScaleRatio {
    /// 恒等スケール 1/1（等倍・要件 1.3）。
    pub const ONE: ScaleRatio = ScaleRatio { num: 1, den: 1 };

    /// 0 を拒否して構築する（既約化して保持・要件 1.1）。
    ///
    /// `num == 0`（表示が消える）・`den == 0`（ゼロ除算）はいずれも `None` を返す。
    /// 成功時は gcd 約分済みの正準形を保持するため、`new(120, 96) == new(5, 4)` が成り立つ。
    /// パニックしない。
    pub fn new(num: u32, den: u32) -> Option<ScaleRatio> {
        if num == 0 || den == 0 {
            return None;
        }
        let g = gcd_u64(num as u64, den as u64) as u32;
        Some(ScaleRatio {
            num: num / g,
            den: den / g,
        })
    }

    /// 乗算合成（アプリ管理拡大率 × DPI 由来 k のシーム・要件 1.6）。
    ///
    /// 積は u64 中間で計算してから gcd で約分する（u32 域の桁溢れを起こさない）。
    ///
    /// # 縮退（近似・要件 1.4 のログ規律）
    ///
    /// 約分後に**分子・分母のいずれか一方でも** u32 を超えた場合（他方が 1 のような小さい値でも
    /// 起こりうる）、大きい側が `u32::MAX` ちょうどになる比率で**両者を線形縮小**して u32 域へ
    /// 収める。これは比の保存ではなく**近似**である——縮小後の値は整数へ切り捨てられるため、
    /// 誤差は各項 1 量子化ステップ以内に収まる（小さい側が 0 へ落ちる場合は 1 へ切り上げる
    /// ため、その項の相対誤差は大きくなり得る）。
    ///
    /// この縮退は**値の情報を失う**フォールバックゆえ、steering のログ規律（フォールバック発生＝
    /// `warn!`）に従い、縮退前後の分子・分母を載せた `warn!` を発する（ログ無し失敗経路の禁止）。
    /// 縮退経路を含め、結果の分子・分母は常に 1 以上・同一入力に対し決定論的であり、パニック・
    /// ラップアラウンドのいずれも起こさない。
    // design の Service Interface が固有メソッド `mul` を契約として定めるため、
    // `std::ops::Mul` との名前衝突警告は意図的に抑止する（演算子多重定義は行わない）。
    #[allow(clippy::should_implement_trait)]
    pub fn mul(self, rhs: ScaleRatio) -> ScaleRatio {
        let mut num = self.num as u64 * rhs.num as u64;
        let mut den = self.den as u64 * rhs.den as u64;
        let g = gcd_u64(num, den);
        num /= g;
        den /= g;
        // 約分しても u32 域に収まらない病的な比の決定論的縮退（大きい側を u32::MAX へ張り付け、
        // 小さい側を同一比率で線形縮小する）。切り捨てゆえ誤差は 1 量子化ステップ以内。
        let largest = num.max(den);
        if largest > u32::MAX as u64 {
            let (orig_num, orig_den) = (num, den);
            // u128 中間で num * u32::MAX / largest（桁溢れなし・largest > 0 ゆえゼロ除算なし）。
            let shrink = |v: u64| ((v as u128 * u32::MAX as u128) / largest as u128).max(1) as u64;
            num = shrink(num);
            den = shrink(den);
            tracing::warn!(
                target: "areka_emo_compose",
                orig_num,
                orig_den,
                num,
                den,
                "ScaleRatio::mul: 積が u32 域に収まらず近似縮退"
            );
        }
        let g = gcd_u64(num, den);
        ScaleRatio {
            num: (num / g) as u32,
            den: (den / g) as u32,
        }
    }

    /// 恒等（1/1）か（要件 1.3/7.2）。
    ///
    /// 正準形ゆえ `num == den` は 1/1 のときに限り成立する。恒等時は上位経路が
    /// 既存の等倍表示（バイト恒等）を選べる。
    pub fn is_identity(self) -> bool {
        self.num == self.den
    }

    /// 照会契約の出口ビュー（`num as f32 / den as f32`・要件 1.2/1.6）。
    ///
    /// 下流（`collision-dpi-hittest` の ÷k・`emo-text-layer` の行寸）が参照する
    /// 合成スケール照会値の表現。**寸法・画素演算にこの値を使ってはならない**——
    /// 寸法の k 倍は必ず [`scale_len`]／[`scaled_extent`] を通す（丸め規約の単一権威）。
    ///
    /// [`scale_len`]: ScaleRatio::scale_len
    /// [`scaled_extent`]: ScaleRatio::scaled_extent
    pub fn as_f32(self) -> f32 {
        self.num as f32 / self.den as f32
    }

    /// 長さの k 倍（丸め単一権威・round half away from zero・要件 2.5/3.1）。
    ///
    /// `round(len × num / den)` を整数のみで計算する（`(2·len·num + den) / (2·den)`）。
    /// 端数ちょうど 0.5 は 0 から遠い側（＝切り上げ）へ丸める——wintf `DPI::to_physical_*`
    /// と同一規約であり、窓寸・合成先寸・採寸の全消費点がこの 1 関数を通ることで
    /// 見切れ・隙間の原因となる不一致丸めを排除する。
    ///
    /// - `len == 0` は 0（存在しない寸法を作らない）。
    /// - `len > 0` の結果は最小 1（極小 k でも表示が消えない・要件 2.5）。
    /// - 恒等（1/1）は入力素通し（要件 1.3/7.2）。
    ///
    /// # 整数専用と桁溢れ（要件 2.5）
    ///
    /// 中間計算は u128 で行い、いかなる入力でもオーバーフローによるパニック・
    /// ラップアラウンドを起こさない（設計の u64 中間式を、極端な `num`／`len` の組でも
    /// 厳密であるよう幅だけ広げたもの・式そのものは同一）。u32 を超える結果は
    /// `u32::MAX` へ**飽和**する（ラップしない）。実寸として i32 域（Win32 の座標・寸法）
    /// へ渡す際の超過検査は呼び手の責務である。
    pub fn scale_len(self, len: u32) -> u32 {
        if len == 0 {
            return 0;
        }
        if self.is_identity() {
            return len;
        }
        let num = self.num as u128;
        let den = self.den as u128;
        // round half away from zero: (2·len·num + den) / (2·den)（全て非負ゆえ切り捨て除算で成立）。
        let scaled = (2 * len as u128 * num + den) / (2 * den);
        // 非ゼロ入力は最小 1px（縮小で消滅させない）。u32 超過は飽和（呼び手が i32 域を検査）。
        u32::try_from(scaled.max(1)).unwrap_or(u32::MAX)
    }

    /// 外形（幅・高さ）の k 倍（各軸へ [`scale_len`] を適用・要件 2.5/3.1）。
    ///
    /// k 倍後の合成寸・窓クライアント寸・採寸の**単一の丸め権威**。軸ごとに独立して
    /// 丸めるため、`scaled_extent` の結果は常に「各軸に `scale_len` を適用した値」と一致する。
    ///
    /// [`scale_len`]: ScaleRatio::scale_len
    pub fn scaled_extent(self, w: u32, h: u32) -> (u32, u32) {
        (self.scale_len(w), self.scale_len(h))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// 作者基準 DPI（ukadoc 正典既定）。DPI 対照表の分母。
    const AUTHOR_DPI: u32 = 96;

    /// 要件 1.1: 構築は gcd で既約正準化する（120/96 → 5/4）。
    #[test]
    fn new_reduces_to_canonical_form() {
        let k = ScaleRatio::new(120, AUTHOR_DPI).unwrap();
        assert_eq!((k.num, k.den), (5, 4));

        for (dpi, expect) in [
            (96u32, (1u32, 1u32)),
            (120, (5, 4)),
            (144, (3, 2)),
            (168, (7, 4)),
            (192, (2, 1)),
        ] {
            let k = ScaleRatio::new(dpi, AUTHOR_DPI).unwrap();
            assert_eq!((k.num, k.den), expect, "dpi={dpi}");
        }
    }

    /// 要件 1.1: 0 は分子・分母のいずれでも構築失敗（パニックしない）。
    #[test]
    fn new_rejects_zero() {
        assert!(ScaleRatio::new(0, 96).is_none());
        assert!(ScaleRatio::new(96, 0).is_none());
        assert!(ScaleRatio::new(0, 0).is_none());
        assert!(ScaleRatio::new(1, 1).is_some());
    }

    /// 要件 1.3: `ONE` は正準の恒等であり、既定値でもある。
    #[test]
    fn one_is_canonical_identity() {
        assert_eq!((ScaleRatio::ONE.num, ScaleRatio::ONE.den), (1, 1));
        assert!(ScaleRatio::ONE.is_identity());
        assert_eq!(ScaleRatio::ONE.as_f32(), 1.0);
        assert_eq!(ScaleRatio::default(), ScaleRatio::ONE);
        assert_eq!(ScaleRatio::new(96, 96).unwrap(), ScaleRatio::ONE);
    }

    /// 要件 1.1: `Eq`/`Hash` は正準形で厳密（キャッシュキーの一意性）。
    #[test]
    fn eq_and_hash_are_strict_on_canonical_form() {
        let a = ScaleRatio::new(120, 96).unwrap();
        let b = ScaleRatio::new(5, 4).unwrap();
        let c = ScaleRatio::new(4, 5).unwrap();
        assert_eq!(a, b);
        assert_ne!(a, c);

        let mut set = HashSet::new();
        set.insert(a);
        set.insert(b);
        assert_eq!(set.len(), 1, "同値は同一ハッシュキーへ畳まれる");
        set.insert(c);
        assert_eq!(set.len(), 2, "逆比は別キー");
    }

    /// 要件 1.3/7.2: 恒等判定は 1/1 のときに限り真。
    #[test]
    fn is_identity_holds_only_for_one() {
        assert!(ScaleRatio::new(96, 96).unwrap().is_identity());
        assert!(ScaleRatio::new(7, 7).unwrap().is_identity());
        assert!(!ScaleRatio::new(120, 96).unwrap().is_identity());
        assert!(!ScaleRatio::new(96, 120).unwrap().is_identity());
        assert!(!ScaleRatio::new(192, 96).unwrap().is_identity());
    }

    /// 要件 1.6: 乗算合成（アプリ管理拡大率 × DPI 由来 k）は約分済みの積を返す。
    #[test]
    fn mul_composes_and_reduces() {
        let k = ScaleRatio::new(120, 96).unwrap(); // 5/4
        // アプリ管理拡大率 1.0 固定シーム: ONE との積は恒等元。
        assert_eq!(ScaleRatio::ONE.mul(k), k);
        assert_eq!(k.mul(ScaleRatio::ONE), k);
        assert_eq!(ScaleRatio::ONE.mul(ScaleRatio::ONE), ScaleRatio::ONE);

        // アプリ 2.0 × DPI 5/4 = 5/2（最終拡大率 2.5）。
        let app = ScaleRatio::new(2, 1).unwrap();
        assert_eq!(app.mul(k), ScaleRatio::new(5, 2).unwrap());
        assert_eq!(k.mul(app), ScaleRatio::new(5, 2).unwrap(), "乗算は可換");

        // 逆数同士は恒等へ約分される。
        let a = ScaleRatio::new(1_000_000, 3).unwrap();
        let b = ScaleRatio::new(3, 1_000_000).unwrap();
        assert_eq!(a.mul(b), ScaleRatio::ONE);
    }

    /// 要件 1.6: 積は u64 中間で計算され、u32 域の桁溢れでラップしない。
    #[test]
    fn mul_uses_wide_intermediate_without_wrapping() {
        // u32 積が溢れる大きさでも約分で恒等へ戻る（中間が u32 なら破綻する）。
        let a = ScaleRatio::new(4_000_000_000, 1).unwrap();
        let b = ScaleRatio::new(1, 4_000_000_000).unwrap();
        assert_eq!(a.mul(b), ScaleRatio::ONE);
        // 縮退が起きない大きさでは近似も警告も起こらない（陰性確認）。
        assert_eq!(
            ScaleRatio::new(65_535, 1)
                .unwrap()
                .mul(ScaleRatio::new(65_535, 1).unwrap()),
            ScaleRatio::new(4_294_836_225, 1).unwrap()
        );
    }

    /// 要件 1.6: 約分後も u32 域へ収まらない病的比は、大きい側を `u32::MAX` へ張り付ける
    /// 線形縮小で近似縮退する（比の保存ではないが誤差は 1 量子化ステップ以内）。
    #[test]
    fn mul_degrades_proportionally_when_product_exceeds_u32() {
        // 65_537² = 4_295_098_369（u32::MAX = 4_294_967_295 を 131_074 だけ超える）。
        // 大きい側 num を u32::MAX へ張り付け、den は 1*u32::MAX/4_295_098_369 = 0 → 最小 1。
        let big = ScaleRatio::new(65_537, 1).unwrap();
        let sq = big.mul(big);
        assert_eq!(
            (sq.num, sq.den),
            (4_294_967_295, 1),
            "誤差 0.0031%（真値 4_295_098_369）"
        );
        assert_eq!(sq, big.mul(big), "縮退も決定論的");
        assert_eq!(sq.scale_len(1), 4_294_967_295);

        // 分母側が超過する対称ケース（den を u32::MAX へ張り付け・num は最小 1）。
        let tiny = ScaleRatio::new(1, 65_537).unwrap();
        let sq_inv = tiny.mul(tiny);
        assert_eq!((sq_inv.num, sq_inv.den), (1, 4_294_967_295));
        assert_eq!(sq_inv, tiny.mul(tiny), "縮退も決定論的");

        // 縮退後も分子・分母は常に非ゼロ（as_f32 が 0 / NaN / inf にならない）。
        assert!(sq.as_f32() > 0.0 && sq.as_f32().is_finite());
        assert!(sq_inv.as_f32() > 0.0 && sq_inv.as_f32().is_finite());
    }

    /// 要件 1.4（ログ規律・フォールバック発生＝`warn!`）: 近似縮退は縮退前後の値つきで
    /// `warn!` を発する（ログ無し失敗経路の禁止）。縮退しない通常経路は無音（非空虚性）。
    #[test]
    fn mul_degradation_emits_warn_log() {
        use crate::log_capture::capture_logs;

        let big = ScaleRatio::new(65_537, 1).unwrap();
        let out = capture_logs(|| {
            let _ = big.mul(big);
        });
        assert!(out.contains("level=WARN"), "縮退は warn 発火: {out}");
        assert!(out.contains("target=areka_emo_compose"), "target: {out}");
        assert!(out.contains("orig_num=4295098369"), "縮退前分子: {out}");
        assert!(out.contains("orig_den=1"), "縮退前分母: {out}");
        assert!(out.contains("num=4294967295"), "縮退後分子: {out}");
        assert!(out.contains("収まらず近似縮退"), "縮退の説明: {out}");

        // 通常経路（u32 域に収まる積）はログを一切出さない。
        let quiet = capture_logs(|| {
            let k = ScaleRatio::new(120, AUTHOR_DPI).unwrap();
            let _ = k.mul(ScaleRatio::new(2, 1).unwrap());
            let _ = ScaleRatio::ONE.mul(ScaleRatio::ONE);
        });
        assert!(quiet.is_empty(), "縮退しない積は無音: {quiet}");
    }

    /// 要件 1.2/1.6: `as_f32` は代表 DPI で厳密値を返す（照会契約の出口ビュー）。
    #[test]
    fn as_f32_yields_exact_dpi_values() {
        for (dpi, expect) in [
            (96u32, 1.0f32),
            (120, 1.25),
            (144, 1.5),
            (168, 1.75),
            (192, 2.0),
        ] {
            let k = ScaleRatio::new(dpi, AUTHOR_DPI).unwrap();
            assert_eq!(k.as_f32(), expect, "dpi={dpi}");
        }
    }

    /// 要件 1.1/1.3/2.2/2.5: DPI 対照表 × 代表原寸が決定論的に一致する。
    ///
    /// 96（k=1/1・等倍）／120（5/4）／144（3/2）／168（7/4）／192（2/1）の 5 水準で、
    /// 代表原寸の k 倍寸が期待表と厳密一致すること・96 と 192 が同一寸にならないこと
    /// （k=1.0 固定の途中状態を残さない・要件 2.2）を固定する。
    #[test]
    fn dpi_table_scaled_extent_is_deterministic() {
        const NATIVE: [u32; 10] = [1, 2, 3, 48, 100, 127, 200, 255, 300, 401];
        // (窓 DPI, 各 NATIVE に対する期待 k 倍寸)
        const TABLE: [(u32, [u32; 10]); 5] = [
            (96, [1, 2, 3, 48, 100, 127, 200, 255, 300, 401]),
            (120, [1, 3, 4, 60, 125, 159, 250, 319, 375, 501]),
            (144, [2, 3, 5, 72, 150, 191, 300, 383, 450, 602]),
            (168, [2, 4, 5, 84, 175, 222, 350, 446, 525, 702]),
            (192, [2, 4, 6, 96, 200, 254, 400, 510, 600, 802]),
        ];

        for (dpi, expect) in TABLE {
            let k = ScaleRatio::new(dpi, AUTHOR_DPI).unwrap();
            for (i, &len) in NATIVE.iter().enumerate() {
                assert_eq!(k.scale_len(len), expect[i], "dpi={dpi} len={len}");
                // 同一入力の反復が同一出力（決定論）。
                assert_eq!(k.scale_len(len), k.scale_len(len));
                // 外形は各軸への scale_len 適用と厳密一致。
                assert_eq!(
                    k.scaled_extent(len, len),
                    (expect[i], expect[i]),
                    "dpi={dpi} len={len}"
                );
            }
        }

        // 要件 2.2: 96 水準と 192 水準は同一物理寸にならない（k=1.0 固定の途中状態の排除）。
        let k96 = ScaleRatio::new(96, AUTHOR_DPI).unwrap();
        let k192 = ScaleRatio::new(192, AUTHOR_DPI).unwrap();
        assert_ne!(k96.scaled_extent(100, 200), k192.scaled_extent(100, 200));
        assert_eq!(k192.scaled_extent(100, 200), (200, 400));
    }

    /// 要件 2.5: 端数ちょうど 0.5 は 0 から遠い側（切り上げ）へ丸める。
    #[test]
    fn scale_len_rounds_half_away_from_zero() {
        let half = ScaleRatio::new(1, 2).unwrap();
        // 0.5 / 1.5 / 2.5 / 3.5 がすべて切り上がる。
        assert_eq!(half.scale_len(1), 1);
        assert_eq!(half.scale_len(3), 2);
        assert_eq!(half.scale_len(5), 3);
        assert_eq!(half.scale_len(7), 4);
        // 端数なしは素通し。
        assert_eq!(half.scale_len(2), 1);
        assert_eq!(half.scale_len(4), 2);

        let k54 = ScaleRatio::new(5, 4).unwrap();
        assert_eq!(k54.scale_len(2), 3); // 2.5
        assert_eq!(k54.scale_len(6), 8); // 7.5
        assert_eq!(k54.scale_len(10), 13); // 12.5

        let k32 = ScaleRatio::new(3, 2).unwrap();
        assert_eq!(k32.scale_len(1), 2); // 1.5
        assert_eq!(k32.scale_len(3), 5); // 4.5
        assert_eq!(k32.scale_len(5), 8); // 7.5

        let quarter = ScaleRatio::new(1, 4).unwrap();
        assert_eq!(quarter.scale_len(2), 1); // 0.5
        assert_eq!(quarter.scale_len(6), 2); // 1.5
        assert_eq!(quarter.scale_len(10), 3); // 2.5

        // 0.5 未満は切り捨て側（最小 1px 保証と区別される丸めそのものの挙動）。
        assert_eq!(ScaleRatio::new(1, 3).unwrap().scale_len(3), 1); // 1.0
        assert_eq!(ScaleRatio::new(2, 5).unwrap().scale_len(3), 1); // 1.2 → 1
        assert_eq!(ScaleRatio::new(3, 5).unwrap().scale_len(3), 2); // 1.8 → 2
    }

    /// 要件 2.5: 非ゼロ原寸は最小 1px（縮小で表示が消えない）。
    #[test]
    fn scale_len_clamps_nonzero_to_min_one_pixel() {
        let tiny = ScaleRatio::new(1, 100).unwrap();
        assert_eq!(tiny.scale_len(1), 1, "0.01 → 1（最小 1px）");
        assert_eq!(tiny.scale_len(49), 1, "0.49 → 1（最小 1px）");
        assert_eq!(tiny.scale_len(50), 1, "0.5 → 1（丸めが自然に 1）");
        assert_eq!(tiny.scale_len(200), 2);
        assert_eq!(ScaleRatio::new(1, 1000).unwrap().scale_len(1), 1);
        // 外形も両軸で最小 1px。
        assert_eq!(tiny.scaled_extent(1, 1), (1, 1));
    }

    /// 要件 2.5: 0 は 0 のまま（存在しない寸法を作らない）。
    #[test]
    fn scale_len_zero_stays_zero() {
        let k = ScaleRatio::new(192, AUTHOR_DPI).unwrap();
        assert_eq!(k.scale_len(0), 0);
        assert_eq!(k.scaled_extent(0, 0), (0, 0));
        assert_eq!(k.scaled_extent(0, 10), (0, 20), "軸ごとに独立して丸める");
        assert_eq!(ScaleRatio::ONE.scale_len(0), 0);
    }

    /// 要件 1.3/7.2: 恒等 k は入力を素通しする（既存等倍出力と等価）。
    #[test]
    fn identity_scale_is_passthrough() {
        for len in [0u32, 1, 2, 3, 127, 4096, u32::MAX] {
            assert_eq!(ScaleRatio::ONE.scale_len(len), len);
        }
        assert_eq!(ScaleRatio::ONE.scaled_extent(300, 401), (300, 401));
        assert_eq!(
            ScaleRatio::new(96, AUTHOR_DPI)
                .unwrap()
                .scaled_extent(300, 401),
            (300, 401)
        );
    }

    /// 要件 2.5: 大寸でも中間演算が溢れず、u32 超過は飽和（ラップしない）。
    #[test]
    fn scale_len_handles_large_extents_without_overflow() {
        let k54 = ScaleRatio::new(5, 4).unwrap();
        assert_eq!(k54.scale_len(2_000_000_000), 2_500_000_000);

        let k74 = ScaleRatio::new(7, 4).unwrap();
        assert_eq!(k74.scale_len(1_000_000_000), 1_750_000_000);

        // u32 を超える結果は飽和（ラップアラウンドなら 8_589_934_590 - 2^32 = 4_294_967_294）。
        let k2 = ScaleRatio::new(2, 1).unwrap();
        assert_eq!(k2.scale_len(u32::MAX), u32::MAX);
        assert_eq!(k2.scaled_extent(u32::MAX, u32::MAX), (u32::MAX, u32::MAX));

        // 巨大な分子・分母の組でもパニックしない（中間幅の証明）。
        let extreme = ScaleRatio::new(u32::MAX, u32::MAX - 1).unwrap();
        assert_eq!(extreme.scale_len(u32::MAX), u32::MAX);
        let shrink = ScaleRatio::new(1, u32::MAX).unwrap();
        assert_eq!(shrink.scale_len(u32::MAX), 1);
    }
}
