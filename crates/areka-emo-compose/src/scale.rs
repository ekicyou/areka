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
//! - **k× リサンプル**（要件 2.1/2.5/7.2）: [`resample`] が native 合成結果
//!   （[`ComposedSurface`]）を k 倍の表示用サーフェスへ整数固定小数点 bilinear で転写する。
//!   `plan.rs`／`blit.rs` を不触に保つため合成経路から隔離した新設モジュールである。
//!
//! 公開 API はパニックしない（構築失敗は `Option`）。

use crate::composed::ComposedSurface;

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

/// bilinear 混合重みの固定小数点分解能（16bit＝65536 段階）。
///
/// 重み対の和は常に厳密に `WEIGHT_ONE`（凸結合）ゆえ、premultiplied 不変条件
/// （B,G,R ≤ A）が補間後も保たれる。
const WEIGHT_SHIFT: u32 = 16;

/// 重み 1.0 に相当する固定小数点値（`1 << WEIGHT_SHIFT`）。
const WEIGHT_ONE: u32 = 1 << WEIGHT_SHIFT;

/// 2 軸ぶんの重み積のシフト量（`WEIGHT_SHIFT * 2`）。
const PRODUCT_SHIFT: u32 = WEIGHT_SHIFT * 2;

/// 2 軸重み積の丸め半量（round half up・`1 << (PRODUCT_SHIFT - 1)`）。
const PRODUCT_HALF: u64 = 1 << (PRODUCT_SHIFT - 1);

/// 出力 1 画素ぶんの入力サンプル指定（エッジクランプ済みの隣接 2 点と混合重み）。
#[derive(Debug, Clone, Copy)]
struct AxisSample {
    /// 手前側の入力座標（`0..len` へクランプ済み）。
    i0: u32,
    /// 奥側の入力座標（`0..len` へクランプ済み・端では `i0` と同値になる）。
    i1: u32,
    /// 奥側の重み（`0..WEIGHT_ONE`。手前側は `WEIGHT_ONE - w`）。
    w: u32,
}

/// 出力座標 → 入力座標の**有理逆写像**（`den/num`）を整数のみで前進させる走査子。
///
/// 画素中心を合わせた写像 `src = (d + 1/2)·den/num − 1/2`
/// ＝ `((2d+1)·den − num) / (2·num)` を、分子の整数部 `index` と剰余 `rem`（分母 `denom`）で
/// 厳密に保持する。前進は加算・剰余のみ（除算は重み量子化の 1 回だけ）で、浮動小数を
/// 一切用いないため累積誤差も丸め非決定も生じない。
#[derive(Debug, Clone, Copy)]
struct AxisWalk {
    /// 入力座標の整数部（出力先頭は入力 −1/2 側へ寄るため**負を取り得る**）。
    index: i64,
    /// 整数部の剰余（`0 <= rem < denom`）。
    rem: u64,
    /// 剰余の分母（`2·num`）。
    denom: u64,
    /// 出力 1 画素あたりの分子の前進量（`2·den`）。
    step: u64,
}

impl AxisWalk {
    /// 出力座標 0 の位置で走査子を初期化する。
    ///
    /// # 桁溢れ
    ///
    /// `num`／`den` は u32 ゆえ `2·num`・`2·den`・`den − num` はいずれも i64/u64 域に収まる。
    fn new(scale: ScaleRatio) -> AxisWalk {
        let denom = 2 * scale.num as i64;
        // d=0 の分子: (2·0+1)·den − num = den − num（拡大時は負＝先頭画素が入力の外側へ出る）。
        let n0 = scale.den as i64 - scale.num as i64;
        AxisWalk {
            index: n0.div_euclid(denom),
            rem: n0.rem_euclid(denom) as u64,
            denom: denom as u64,
            step: 2 * scale.den as u64,
        }
    }

    /// 現在位置のサンプル指定を返す（**エッジクランプ固定**・範囲外を外挿しない）。
    ///
    /// `len == 0` は呼び手が事前に弾く（[`resample`] の外形ゼロ早期復帰）。防御的に
    /// `saturating_sub` を用い、いかなる `len` でも算術パニックを起こさない。
    fn sample(&self, len: u32) -> AxisSample {
        let last = len.saturating_sub(1) as i64;
        AxisSample {
            i0: self.index.clamp(0, last) as u32,
            i1: self.index.saturating_add(1).clamp(0, last) as u32,
            // 剰余の 16bit 量子化（rem < denom <= 2·u32::MAX ゆえ rem<<16 は u64 域）。
            w: ((self.rem << WEIGHT_SHIFT) / self.denom) as u32,
        }
    }

    /// 出力座標を 1 進める（分子へ `2·den` を加算し、整数部へ繰り上げる）。
    ///
    /// # 桁溢れ
    ///
    /// `rem + step < 2·u32::MAX + 2·u32::MAX` は u64 域。`index` は最終的に入力座標
    /// （＝`src_w` 相当・u32 域）付近までしか進まない——縮小（`den > num`）では出力画素数
    /// 自体が入力より小さく、拡大（`num > den`）では 1 画素あたりの前進が 1 未満ゆえ
    /// `index <= 出力画素数`——ので i64 で厳密に足りる。
    fn advance(&mut self) {
        self.rem += self.step;
        self.index += (self.rem / self.denom) as i64;
        self.rem %= self.denom;
    }
}

/// native 合成結果を `scale` 倍の表示用サーフェスへ転写する
/// （premultiplied BGRA・**完全整数** bilinear・要件 2.1/2.5/7.2）。
///
/// design「emo-compose / scale.rs」の Service Interface に厳密に従う:
///
/// - **事前条件**: `src.width() > 0 && src.height() > 0`（0 寸は上流 `EmptyComposition` で先に落ちる）。
/// - **事後条件**: `out` の外形は `scale.scaled_extent(src 外形)` と厳密一致する
///   （欠け・意図しない切り捨てなし・要件 2.5）。同一 `(src, scale)` はバイト決定論。
/// - **恒等**（要件 7.2）: `scale.is_identity()` なら `src` のバイト恒等コピー。既存の等倍
///   golden が構造的に一切変化しないことを保証する。
///
/// # 整数専用（設計 D5・`blit.rs` と同格の決定性規約）
///
/// 座標写像は `den/num` の有理逆写像を分子・剰余で厳密保持（[`AxisWalk`]）し、混合重みは
/// 16bit 固定小数点へ量子化する。画素値の合成は u32/u64 のみで行い、**f32/f64 を経路に
/// 一切持ち込まない**。丸めは `(v + 2^31) >> 32`（round half up）で一意である。
///
/// # premultiplied ドメイン（設計 D5）
///
/// B・G・R・A の 4 チャンネルへ**同一式**を適用する（非乗算化しない・α を特別扱いしない）。
/// 重み対の和が厳密に `WEIGHT_ONE` の凸結合であり丸めが単調ゆえ、入力が満たす
/// premultiplied 不変条件（B,G,R ≤ A）は出力でも保たれる。
///
/// # エッジクランプ（設計 Risks）
///
/// 写像先が入力範囲の外へ出る端画素は、隣接 2 点を独立に `[0, len)` へクランプして混合する
/// （＝端では原画素そのもの）。外挿は行わない——決定論的で、テストで固定されている。
///
/// # 非パニック
///
/// 添字はクランプ済み座標から構成するため境界外に出ない。事前条件違反（外形ゼロ）は
/// `warn!`（log-first・無言縮退禁止）を出したうえで外形どおりの空バッファを返す。
///
/// なお `scaled_extent` が u32 を飽和するほどの巨大外形は、`ComposedSurface` の確保契約
/// （`stride = width * 4`）そのものが表現できない領域であり、[`ScaleRatio::scale_len`] と
/// 同じく**呼び手の寸法域検査の責務**である。
///
/// # 割り当て
///
/// 行間で不変な x 軸の写像表を 1 本だけ確保する（`O(out_w)`・画素あたりの除算を排するため）。
/// リサンプルは k 変化・合成入力変化時のみ発火する経路である（design「Performance」）。
pub fn resample(src: &ComposedSurface, scale: ScaleRatio, out: &mut ComposedSurface) {
    let (out_w, out_h) = scale.scaled_extent(src.width(), src.height());
    // 出力は常に事後条件どおりの外形へ再確保＋全透明クリア（容量再利用・残像なし）。
    out.resize_and_clear(out_w, out_h);

    // 事前条件違反（外形ゼロ）: 転写できる画素が存在しない。パニックせず空を返し、
    // 無言で通さない（steering ログ規律・要件 1.4 と同格の log-first）。
    if src.width() == 0 || src.height() == 0 {
        tracing::warn!(
            target: "areka_emo_compose",
            src_w = src.width(),
            src_h = src.height(),
            out_w,
            out_h,
            "resample: 外形ゼロの入力（事前条件違反・上流 EmptyComposition が先に落とすはず）: 空の出力を返す"
        );
        return;
    }

    // 恒等 k=1/1 はバイト恒等コピー（要件 7.2）。scale_len は素通しゆえ外形・stride は
    // src と厳密一致し、長さの一致が保証されるため copy_from_slice はパニックしない。
    if scale.is_identity() {
        out.bytes_mut().copy_from_slice(src.bytes());
        return;
    }

    // x 軸の写像は全行で共通ゆえ一度だけ表に落とす（内側ループから除算を除く）。
    let mut walk = AxisWalk::new(scale);
    let mut x_map: Vec<AxisSample> = Vec::with_capacity(out_w as usize);
    for _ in 0..out_w {
        x_map.push(walk.sample(src.width()));
        walk.advance();
    }

    let src_stride = src.stride() as usize;
    let src_bytes = src.bytes();
    let out_stride = out.stride() as usize;
    let dst = out.bytes_mut();

    let mut y_walk = AxisWalk::new(scale);
    for dy in 0..out_h as usize {
        let ys = y_walk.sample(src.height());
        y_walk.advance();

        let row0 = ys.i0 as usize * src_stride;
        let row1 = ys.i1 as usize * src_stride;
        let wy = ys.w as u64;
        let inv_wy = (WEIGHT_ONE - ys.w) as u64;
        let dst_row = dy * out_stride;

        for (dx, xs) in x_map.iter().enumerate() {
            let wx = xs.w;
            let inv_wx = WEIGHT_ONE - wx;
            // 4 近傍のバイト先頭（クランプ済み座標ゆえ全て範囲内）。
            let p00 = row0 + xs.i0 as usize * 4;
            let p01 = row0 + xs.i1 as usize * 4;
            let p10 = row1 + xs.i0 as usize * 4;
            let p11 = row1 + xs.i1 as usize * 4;
            let di = dst_row + dx * 4;

            // BGRA 4 チャンネルへ同式適用（premultiplied ドメイン・α も同じ）。
            for c in 0..4usize {
                // 横方向: 最大 255·65536 = 16_711_680（u32 域）。
                let top = src_bytes[p00 + c] as u32 * inv_wx + src_bytes[p01 + c] as u32 * wx;
                let bottom = src_bytes[p10 + c] as u32 * inv_wx + src_bytes[p11 + c] as u32 * wx;
                // 縦方向: 最大 16_711_680·65536 ≈ 1.1e12（u64 域）。丸めは round half up。
                let v = top as u64 * inv_wy + bottom as u64 * wy;
                dst[di + c] = ((v + PRODUCT_HALF) >> PRODUCT_SHIFT) as u8;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// 作者基準 DPI（ukadoc 正典既定）。DPI 対照表の分母。
    const AUTHOR_DPI: u32 = 96;

    /// premultiplied BGRA 画素列（行優先）から `ComposedSurface` を組む（テスト補助）。
    fn surface_of(w: u32, h: u32, px: &[[u8; 4]]) -> ComposedSurface {
        assert_eq!(px.len(), (w * h) as usize, "画素数と外形が一致すること");
        let mut s = ComposedSurface::new(w, h);
        let stride = s.stride() as usize;
        let bytes = s.bytes_mut();
        for (i, p) in px.iter().enumerate() {
            let (x, y) = (i % w as usize, i / w as usize);
            let o = y * stride + x * 4;
            bytes[o..o + 4].copy_from_slice(p);
        }
        s
    }

    /// 不透明グレー画素（B=G=R=v・A=255）。premultiplied 不変条件 B,G,R ≤ A を満たす。
    fn gray(v: u8) -> [u8; 4] {
        [v, v, v, 255]
    }

    /// グレー値表を BGRA バイト列へ展開する（golden 比較用）。
    fn gray_bytes(vals: &[u8]) -> Vec<u8> {
        vals.iter().flat_map(|&v| gray(v)).collect()
    }

    /// f64 参照 bilinear（**テスト専用オラクル**・本番経路には浮動小数を一切持ち込まない）。
    ///
    /// 本実装と同一の写像規約——画素中心を合わせた den/num の有理逆写像
    /// `src = (d + 0.5)·den/num − 0.5`・隣接 2 点の独立エッジクランプ——を浮動小数で解く。
    /// golden 値を「観測結果の追認」でなく**独立に導出した真値との一致**として検証するために用いる。
    /// 整数実装との差は重み量子化（1/65536）＋最終丸め（±0.5）に収まる。
    fn oracle(src: &ComposedSurface, scale: ScaleRatio, dx: u32, dy: u32, c: usize) -> f64 {
        let ratio = scale.den as f64 / scale.num as f64;
        let sx = (dx as f64 + 0.5) * ratio - 0.5;
        let sy = (dy as f64 + 0.5) * ratio - 0.5;
        let (x0, y0) = (sx.floor(), sy.floor());
        let (fx, fy) = (sx - x0, sy - y0);
        let at = |x: f64, y: f64| -> f64 {
            let cx = x.max(0.0).min((src.width() - 1) as f64) as usize;
            let cy = y.max(0.0).min((src.height() - 1) as f64) as usize;
            src.bytes()[cy * src.stride() as usize + cx * 4 + c] as f64
        };
        let top = at(x0, y0) * (1.0 - fx) + at(x0 + 1.0, y0) * fx;
        let bot = at(x0, y0 + 1.0) * (1.0 - fx) + at(x0 + 1.0, y0 + 1.0) * fx;
        top * (1.0 - fy) + bot * fy
    }

    /// 出力全画素がオラクル真値と量子化誤差内で一致することを確認する（golden 値の妥当性検証）。
    fn assert_matches_oracle(src: &ComposedSurface, scale: ScaleRatio, out: &ComposedSurface) {
        for dy in 0..out.height() {
            for dx in 0..out.width() {
                for c in 0..4usize {
                    let got =
                        out.bytes()[dy as usize * out.stride() as usize + dx as usize * 4 + c];
                    let want = oracle(src, scale, dx, dy, c);
                    assert!(
                        (got as f64 - want).abs() <= 0.6,
                        "({dx},{dy}) ch{c}: 整数実装 {got} と f64 オラクル {want} が乖離"
                    );
                }
            }
        }
    }

    /// 要件 7.2: 恒等 k（1/1）は入力バイトの厳密コピー（既存 golden が一切変化しない構造保証）。
    #[test]
    fn resample_identity_is_byte_copy() {
        let src = surface_of(
            3,
            2,
            &[
                [1, 2, 3, 250],
                [10, 20, 30, 255],
                [0, 0, 0, 0],
                [77, 66, 55, 200],
                [255, 255, 255, 255],
                [9, 8, 7, 128],
            ],
        );
        let mut out = ComposedSurface::new(0, 0);
        resample(&src, ScaleRatio::ONE, &mut out);
        assert_eq!((out.width(), out.height()), (3, 2));
        assert_eq!(out.stride(), src.stride());
        assert_eq!(out.bytes(), src.bytes(), "恒等はバイト恒等コピー");

        // 96/96 のように既約化で 1/1 になる比も同じ恒等経路を通る。
        let mut out2 = ComposedSurface::new(0, 0);
        resample(
            &src,
            ScaleRatio::new(AUTHOR_DPI, AUTHOR_DPI).unwrap(),
            &mut out2,
        );
        assert_eq!(out2.bytes(), src.bytes());
    }

    /// 要件 2.5: 出力外形は丸め単一権威 `scaled_extent` と厳密一致する（欠け・切り捨てなし）。
    #[test]
    fn resample_extent_matches_scaled_extent() {
        let src = surface_of(4, 3, &[gray(64); 12]);
        for (num, den) in [
            (2u32, 1u32),
            (5, 4),
            (3, 2),
            (7, 4),
            (1, 2),
            (1, 3),
            (1, 100),
        ] {
            let k = ScaleRatio::new(num, den).unwrap();
            let mut out = ComposedSurface::new(0, 0);
            resample(&src, k, &mut out);
            assert_eq!(
                (out.width(), out.height()),
                k.scaled_extent(src.width(), src.height()),
                "k={num}/{den}"
            );
            assert_eq!(out.stride(), out.width() * 4);
            assert_eq!(
                out.bytes().len(),
                (out.stride() * out.height()) as usize,
                "k={num}/{den}"
            );
        }
    }

    /// 要件 2.1: 整数 2 倍拡大の golden（bilinear の重みが 1/4 刻みで厳密＝丸め非依存）。
    ///
    /// 2×2 グレー（0,80 / 160,240）→ 4×4。端は clamp ゆえ原画素そのもの、内側は
    /// 0.75/0.25 の混合になる（手計算値とオラクルの双方で固定する）。
    #[test]
    fn resample_two_times_matches_golden() {
        let src = surface_of(2, 2, &[gray(0), gray(80), gray(160), gray(240)]);
        let k = ScaleRatio::new(2, 1).unwrap();
        let mut out = ComposedSurface::new(0, 0);
        resample(&src, k, &mut out);

        assert_eq!((out.width(), out.height()), (4, 4));
        assert_matches_oracle(&src, k, &out);
        #[rustfmt::skip]
        let expect = gray_bytes(&[
              0,  20,  60,  80,
             40,  60, 100, 120,
            120, 140, 180, 200,
            160, 180, 220, 240,
        ]);
        assert_eq!(out.bytes(), expect.as_slice());
    }

    /// 要件 2.1/2.5: 非整数 k（5/4）の決定論 golden＋反復不変（同一入力→同一バイト）。
    ///
    /// 4 チャンネルを別値にした 4×4 → 5×5。チャンネル取り違え（BGRA 順の崩れ）も検出する。
    #[test]
    fn resample_five_quarters_is_deterministic_golden() {
        const V: [u8; 16] = [
            10, 200, 30, 240, 250, 5, 128, 64, 33, 99, 210, 7, 180, 21, 66, 143,
        ];
        let px: Vec<[u8; 4]> = V.iter().map(|&v| [v, v / 2, v / 4, 255]).collect();
        let src = surface_of(4, 4, &px);
        let k = ScaleRatio::new(5, 4).unwrap();

        let mut out = ComposedSurface::new(0, 0);
        resample(&src, k, &mut out);
        assert_eq!((out.width(), out.height()), (5, 5));
        assert_matches_oracle(&src, k, &out);

        // golden はオラクル（上の `assert_matches_oracle`）で真値との一致を検証済みの値。
        // 四隅は入力四隅そのもの（エッジクランプ）・(0,2) は重み厳密 1/2 の中点 115=(200+30)/2。
        #[rustfmt::skip]
        let expect: [u8; 100] = [
             10,   5,   2, 255,  143,  71,  36, 255,  115,  58,  29, 255,   93,  46,  23, 255,  240, 120,  60, 255,
            178,  89,  44, 255,   98,  49,  24, 255,   81,  40,  20, 255,  104,  52,  26, 255,  117,  58,  29, 255,
            142,  71,  35, 255,   79,  39,  19, 255,  111,  55,  27, 255,  129,  64,  32, 255,   36,  18,   9, 255,
             77,  38,  19, 255,   76,  38,  19, 255,  121,  60,  30, 255,  131,  65,  32, 255,   48,  23,  11, 255,
            180,  90,  45, 255,   69,  34,  17, 255,   44,  22,  11, 255,   89,  44,  22, 255,  143,  71,  35, 255,
        ];
        assert_eq!(out.bytes(), &expect[..], "非整数 k の golden");
        // 四隅＝入力四隅の恒等（クランプの独立確認）。
        assert_eq!(&out.bytes()[..4], &[10, 5, 2, 255]);
        assert_eq!(&out.bytes()[96..], &[143, 71, 35, 255]);

        // 反復不変（決定論）: 同一入力の再実行が同一バイトを返す。
        let mut again = ComposedSurface::new(0, 0);
        resample(&src, k, &mut again);
        assert_eq!(again.bytes(), out.bytes(), "同一 (src, k) はバイト決定論");
    }

    /// 要件 2.5: 縮小（k<1）も外形・内容が決定論的（4×4 → 2×2・オラクル一致）。
    #[test]
    fn resample_downscale_matches_oracle_and_golden() {
        #[rustfmt::skip]
        let v = [
              0,  40,  80, 120,
             10,  50,  90, 130,
            200, 210, 220, 230,
            240, 250, 255,   5,
        ];
        let px: Vec<[u8; 4]> = v.iter().map(|&x| gray(x)).collect();
        let src = surface_of(4, 4, &px);
        let k = ScaleRatio::new(1, 2).unwrap();

        let mut out = ComposedSurface::new(0, 0);
        resample(&src, k, &mut out);
        assert_eq!((out.width(), out.height()), (2, 2));
        assert_matches_oracle(&src, k, &out);
        // 1/2 縮小は 2×2 ブロックの厳密平均（重みが全て 1/2）:
        // (0+40+10+50)/4=25・(80+120+90+130)/4=105・(200+210+240+250)/4=225・
        // (220+230+255+5)/4=177.5 → round half up → 178（丸め規約が効く画素）。
        assert_eq!(out.bytes(), gray_bytes(&[25, 105, 225, 178]).as_slice());
    }

    /// 設計 D5: premultiplied ドメインの不変条件（B,G,R ≤ A）が補間後も保たれる。
    ///
    /// bilinear は重み和が厳密に 1（65536）の凸結合であり、丸めも単調ゆえ
    /// premultiplied を崩さない（α を別扱いしない＝非乗算化しない証拠）。
    #[test]
    fn resample_preserves_premultiplied_invariant() {
        // 半透明・全透明・不透明の混在（各画素 B,G,R ≤ A）。
        let px = [
            [0, 0, 0, 0],
            [128, 64, 32, 128],
            [255, 255, 255, 255],
            [10, 5, 1, 10],
            [200, 100, 50, 200],
            [0, 0, 0, 0],
            [7, 7, 7, 7],
            [64, 32, 16, 64],
            [255, 0, 0, 255],
        ];
        let src = surface_of(3, 3, &px);
        for (num, den) in [(2u32, 1u32), (5, 4), (7, 3), (1, 2)] {
            let k = ScaleRatio::new(num, den).unwrap();
            let mut out = ComposedSurface::new(0, 0);
            resample(&src, k, &mut out);
            for (i, p) in out.bytes().chunks_exact(4).enumerate() {
                assert!(
                    p[0] <= p[3] && p[1] <= p[3] && p[2] <= p[3],
                    "k={num}/{den} 画素{i} が premultiplied を破っている: {p:?}"
                );
            }
        }
    }

    /// 設計「Risks」: 端画素の外挿はエッジクランプで固定（決定論・外挿しない）。
    #[test]
    fn resample_clamps_at_edges() {
        // 1×1 の拡大は全画素が原画素と同一（クランプのみ＝外挿値が混ざらない）。
        let one = surface_of(1, 1, &[[12, 34, 56, 200]]);
        let mut out = ComposedSurface::new(0, 0);
        resample(&one, ScaleRatio::new(3, 1).unwrap(), &mut out);
        assert_eq!((out.width(), out.height()), (3, 3));
        for p in out.bytes().chunks_exact(4) {
            assert_eq!(p, [12, 34, 56, 200], "1×1 の拡大は全画素がクランプ複製");
        }

        // 2×1 の 4 倍拡大 → 8×4。縦は 1 行しかないため全行が同一（縦クランプ）。
        let two = surface_of(2, 1, &[gray(0), gray(255)]);
        let mut wide = ComposedSurface::new(0, 0);
        let k4 = ScaleRatio::new(4, 1).unwrap();
        resample(&two, k4, &mut wide);
        assert_eq!((wide.width(), wide.height()), (8, 4));
        assert_matches_oracle(&two, k4, &wide);
        let stride = wide.stride() as usize;
        for dy in 1..4usize {
            assert_eq!(
                &wide.bytes()[dy * stride..(dy + 1) * stride],
                &wide.bytes()[..stride],
                "単一行入力の拡大は全行がクランプ複製（行 {dy}）"
            );
        }

        // 横方向: 両端は原画素そのもの（範囲外を混ぜない）・内側は 1/8 刻みの厳密混合。
        // src 座標 = (d+0.5)/4 − 0.5 → d=0,1 は負（→0 クランプ）・d=6,7 は 1 以上（→1 クランプ）。
        let row: Vec<u8> = wide.bytes()[..stride]
            .chunks_exact(4)
            .map(|p| p[0])
            .collect();
        assert_eq!(row.first(), Some(&0), "左端は原画素 0（負側外挿なし）");
        assert_eq!(row.last(), Some(&255), "右端は原画素 255（右側外挿なし）");
        assert!(
            row.windows(2).all(|w| w[0] <= w[1]),
            "単調（外挿の跳ねなし）: {row:?}"
        );
        // 0.125/0.375/0.625/0.875 × 255 = 31.875/95.625/159.375/223.125（round half up）。
        assert_eq!(row, vec![0, 0, 32, 96, 159, 223, 255, 255]);
    }

    /// 事前条件違反（外形ゼロ）でもパニックせず、空の出力と `warn!` を返す（log-first）。
    #[test]
    fn resample_zero_extent_is_empty_and_warns() {
        use crate::log_capture::capture_logs;

        let empty = ComposedSurface::new(0, 0);
        let mut out = ComposedSurface::new(4, 4);
        let logged = capture_logs(|| {
            resample(&empty, ScaleRatio::new(2, 1).unwrap(), &mut out);
        });
        assert_eq!((out.width(), out.height()), (0, 0));
        assert!(out.bytes().is_empty());
        assert!(
            logged.contains("level=WARN"),
            "外形ゼロは warn 発火: {logged}"
        );
        assert!(
            logged.contains("target=areka_emo_compose"),
            "target: {logged}"
        );

        // 片軸のみゼロ（高さ 0）も同様に非パニック・空出力。
        let flat = ComposedSurface::new(3, 0);
        let mut out2 = ComposedSurface::new(1, 1);
        resample(&flat, ScaleRatio::new(5, 4).unwrap(), &mut out2);
        assert_eq!((out2.width(), out2.height()), (4, 0));
        assert!(out2.bytes().is_empty());

        // 正常経路は無音（非空虚性）。
        let src = surface_of(2, 2, &[gray(1), gray(2), gray(3), gray(4)]);
        let quiet = capture_logs(|| {
            let mut o = ComposedSurface::new(0, 0);
            resample(&src, ScaleRatio::new(5, 4).unwrap(), &mut o);
            resample(&src, ScaleRatio::ONE, &mut o);
        });
        assert!(quiet.is_empty(), "正常な k× 転写は無音: {quiet}");
    }

    /// 出力バッファ再利用: 直前の内容・外形が残らず、毎回 `scaled_extent` どおりに上書きされる。
    #[test]
    fn resample_reuses_output_buffer_without_residue() {
        let src = surface_of(2, 2, &[gray(0), gray(80), gray(160), gray(240)]);
        let k2 = ScaleRatio::new(2, 1).unwrap();
        let mut out = ComposedSurface::new(0, 0);

        resample(&src, k2, &mut out);
        let big = out.bytes().to_vec();

        // 大→小→大: 縮小後に伸長しても残像が混ざらない。
        resample(&src, ScaleRatio::new(1, 2).unwrap(), &mut out);
        assert_eq!((out.width(), out.height()), (1, 1));
        resample(&src, k2, &mut out);
        assert_eq!(out.bytes(), big.as_slice(), "再利用バッファでも同一結果");

        // 恒等も同じバッファへ正しく畳み込まれる。
        resample(&src, ScaleRatio::ONE, &mut out);
        assert_eq!(out.bytes(), src.bytes());
    }

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

    // ------------------------------------------------------------------
    // task 6.1: 純関数の全網羅（既存の檻が届いていなかった領域を塞ぐ）
    // ------------------------------------------------------------------

    /// 設計 D5（premultiplied ドメイン）: **α 可変**入力の厳密 golden（縮小 1/2）。
    ///
    /// 既存の厳密値 golden は全て α=255 で、α 可変ケースは不変条件（B,G,R ≤ A）しか
    /// 見ていなかった。α=255 では非乗算化が恒等写像ゆえ、**「非乗算化 → 補間 → 再乗算」
    /// する実装でも既存テストは全緑になる**。本テストはその変異を殺す。
    ///
    /// # 殺す変異（変異注入の実測に基づく。「排他的」＝α 可変 golden 2 本のみ失敗・既存は全緑）
    ///
    /// - **排他的**: `resample` の内側ループを「straight α ドメインで補間（B,G,R を α で割り、
    ///   補間後に再び α で乗ずる）」へ差し替える変異。
    ///   例: 出力(0,0) は原画素 4 点 `[255,255,255,255] / [0,0,0,0] / [0,0,0,0] / [0,0,0,0]`
    ///   の premultiplied 平均 `255/4 = 63.75 → 64` になるが、非乗算化経路では
    ///   straight 色平均 `63.75→64` × α 平均 `64` / 255 ＝ **16** へ落ちる。
    /// - **排他的**: 4 チャンネルのうち α だけ別式にする変異（例 α を 4 近傍の max にする）。
    ///   既存 golden は α=255 一色ゆえ α 列の差を見分けられない。
    /// - **既存と共倒れ**: 最終丸め（round half up）を切り捨てへ変える変異——48/25/14/77/98/65
    ///   の各値が動くが、既存 `resample_clamps_at_edges`／
    ///   `resample_downscale_matches_oracle_and_golden`／
    ///   `resample_five_quarters_is_deterministic_golden` も同時に死ぬ（実測 5 失敗）。
    /// - **既存と共倒れ**: BGRA チャンネル順の取り違え（実測 4 失敗＝α 可変 golden 2 本＋
    ///   既存 `resample_clamps_at_edges`／`resample_five_quarters_is_deterministic_golden`）。
    ///
    /// # 殺せない変異（担当は別テスト）
    ///
    /// - 重み量子化の分解能を落とす変異（`WEIGHT_SHIFT` 16bit→8bit 等）。k=1/2 の重みは
    ///   厳密に 1/4 ずつで粗い量子化でも割り切れるため本テストは緑のまま。その檻は重みが
    ///   割り切れない k を使う既存 `resample_five_quarters_is_deterministic_golden` が持つ。
    #[test]
    fn resample_alpha_varying_downscale_is_premultiplied_golden() {
        // 各画素は premultiplied 不変条件 B,G,R ≤ A を満たす（α は 0〜255 に散らす）。
        #[rustfmt::skip]
        let px: [[u8; 4]; 16] = [
            [255, 255, 255, 255], [  0,   0,   0,   0], [128,  64,  32, 200], [ 10,  10,  10,  10],
            [  0,   0,   0,   0], [  0,   0,   0,   0], [ 50,  25,  12,  60], [  4,   2,   1,   8],
            [200, 150, 100, 255], [  8,   4,   2,  16], [  0,   0,   0,   0], [255,   0,   0, 255],
            [100,  80,  60, 120], [  0,   0,   0,   0], [  1,   1,   1,   1], [  3,   2,   1,   4],
        ];
        let src = surface_of(4, 4, &px);
        let k = ScaleRatio::new(1, 2).unwrap();

        let mut out = ComposedSurface::new(0, 0);
        resample(&src, k, &mut out);
        assert_eq!((out.width(), out.height()), (2, 2));
        // 独立導出（f64 オラクル＝premultiplied 各チャンネル独立 bilinear）との一致。
        assert_matches_oracle(&src, k, &out);

        // k=1/2 の重みは厳密に 1/4 ずつ（2×2 ブロックの round half up 平均）。
        // 各値は「和 S に対し floor((S+2)/4)」で手計算した真値:
        //   (0,0) B,G,R,A: S=255 → 64
        //   (1,0) B:192→48 G:101→25 R:55→14 A:278→70
        //   (0,1) B:308→77 G:234→59 R:162→41 A:391→98
        //   (1,1) B:259→65 G:3→1   R:2→1   A:260→65
        #[rustfmt::skip]
        let expect: [u8; 16] = [
             64,  64,  64,  64,   48,  25,  14,  70,
             77,  59,  41,  98,   65,   1,   1,  65,
        ];
        assert_eq!(
            out.bytes(),
            &expect[..],
            "α 可変の premultiplied 厳密 golden"
        );

        // 非乗算化変異の急所を単独でも固定する（straight 経路なら 16 前後へ落ちる画素）。
        assert_eq!(
            &out.bytes()[..4],
            &[64, 64, 64, 64],
            "premultiplied 平均 255/4=63.75→64（非乗算化経路なら 16 になる）"
        );
    }

    /// 設計 D5（premultiplied ドメイン）: **α 可変**入力の厳密 golden（拡大 2/1）。
    ///
    /// 縮小 golden と同じ変異を、重みが 1/4・3/4 になる拡大経路でも殺す。4 チャンネルを
    /// 別値にしてあるため BGRA 順の取り違えも検出する。
    ///
    /// # 殺す変異（変異注入の実測に基づく。「排他的」＝α 可変 golden 2 本のみ失敗・既存は全緑）
    ///
    /// - **排他的**: 非乗算化 → 再乗算する実装。例: 出力(1,0) は `0.75·p00 + 0.25·p01`
    ///   （p01 は α=0 の全透明）＝ `(191,150,75,191)` だが、straight 経路では
    ///   色 191.25 × α 191.25 / 255 ＝ **143** へ落ちる。
    /// - **排他的**: α だけ別式にする変異（縮小版 golden と同じ檻）。
    /// - **既存と共倒れ**: エッジクランプの改変（四隅が原画素と一致しなくなる）。実測 4 失敗＝
    ///   本テスト＋既存 `resample_clamps_at_edges`／`resample_two_times_matches_golden`／
    ///   `resample_five_quarters_is_deterministic_golden`（縮小版 golden はこの変異では死なない）。
    /// - **既存と共倒れ**: round half up の改変（124.5→125・57.5→58・73.5→74・72.5→73 が動く）。
    ///   実測 5 失敗＝α 可変 golden 2 本＋既存 3 本（縮小版 golden の doc に列挙）。
    /// - **既存と共倒れ**: BGRA チャンネル順の取り違え。4 チャンネルを別値にしてあるため本テストは
    ///   確実に落ちるが、既存 `resample_clamps_at_edges`／
    ///   `resample_five_quarters_is_deterministic_golden` も同時に落ちる。
    ///
    /// # 殺せない変異（担当は別テスト）
    ///
    /// - 重み量子化の分解能を落とす変異。k=2/1 の重みは厳密に 1/4・3/4（`AxisWalk` の 16bit
    ///   量子化が割り切れる）ゆえ粗い量子化でも値が動かない。縮小版 golden と同じ限界である。
    #[test]
    fn resample_alpha_varying_upscale_is_premultiplied_golden() {
        let src = surface_of(
            2,
            2,
            &[
                [255, 200, 100, 255], // 不透明
                [0, 0, 0, 0],         // 全透明
                [64, 32, 16, 64],     // 半透明
                [128, 96, 0, 200],    // 半透明・R=0
            ],
        );
        let k = ScaleRatio::new(2, 1).unwrap();

        let mut out = ComposedSurface::new(0, 0);
        resample(&src, k, &mut out);
        assert_eq!((out.width(), out.height()), (4, 4));
        assert_matches_oracle(&src, k, &out);

        // 重みは厳密に 1/4・3/4（AxisWalk の 16bit 量子化が割り切れる）。
        // 手計算の真値（round half up）。
        #[rustfmt::skip]
        let expect: [u8; 64] = [
            255, 200, 100, 255,   191, 150,  75, 191,    64,  50,  25,  64,     0,   0,   0,   0,
            207, 158,  79, 207,   163, 125,  59, 168,    76,  58,  20,  89,    32,  24,   0,  50,
            112,  74,  37, 112,   108,  74,  28, 121,   100,  73,   9, 140,    96,  72,   0, 150,
             64,  32,  16,  64,    80,  48,  12,  98,   112,  80,   4, 166,   128,  96,   0, 200,
        ];
        assert_eq!(
            out.bytes(),
            &expect[..],
            "α 可変拡大の premultiplied 厳密 golden"
        );

        // 四隅は原画素そのもの（エッジクランプ・外挿なし）。
        assert_eq!(&out.bytes()[..4], &[255, 200, 100, 255]);
        assert_eq!(&out.bytes()[60..], &[128, 96, 0, 200]);

        // 非乗算化変異の急所（全透明画素と混ざる位置）。
        assert_eq!(
            &out.bytes()[4..8],
            &[191, 150, 75, 191],
            "0.75·不透明 + 0.25·全透明（非乗算化経路なら B=143 になる）"
        );

        // premultiplied 不変条件は当然保たれる（厳密 golden の副次確認）。
        for (i, p) in out.bytes().chunks_exact(4).enumerate() {
            assert!(
                p[0] <= p[3] && p[1] <= p[3] && p[2] <= p[3],
                "画素{i} が premultiplied を破っている: {p:?}"
            );
        }
    }

    /// 要件 2.5: 丸め規約 round half away from zero を、**丁度 .5 とその両隣**の対で固定する。
    ///
    /// 片側だけの主張では「常に切り上げ」実装も緑になってしまう。`.5` 未満（切り捨て）・
    /// `.5` 丁度（切り上げ）・`.5` 超（切り上げ）の 3 点を隣接入力で対にする。
    ///
    /// # 殺す変異（変異注入の実測に基づく。**排他的キルは持たない**）
    ///
    /// 丸め変異の一次防衛線は既存 `scale_len_rounds_half_away_from_zero`／
    /// `dpi_table_scaled_extent_is_deterministic` が既に張っており、下の 3 変異はいずれも
    /// それらと共倒れになる。本テストの役割は「.5 未満／.5 丁度／.5 超」を隣接入力の対として
    /// 契約に明文化することであって、新しい変異を単独で捕まえることではない。
    ///
    /// - 常に切り上げ（`div_ceil`）: `…499/1000`（0.499）が 1 つ上へずれる。実測 5 失敗＝
    ///   本テスト・`as_f32_is_query_view_not_dimension_authority`・
    ///   `scale_len_u128_intermediate_beats_u64_overflow`＋既存 2 本
    ///   （`scale_len_rounds_half_away_from_zero`／`dpi_table_scaled_extent_is_deterministic`）。
    /// - 常に切り捨て（`len·num/den`）: `…500/1000`（0.5 丁度）が 1 つ下へずれる。実測 4 失敗＝
    ///   本テスト＋既存 3 本（上記 2 本と `resample_zero_extent_is_empty_and_warns`）。
    /// - round half to **even**: `n` が偶数の `n+0.5` が `n` へ落ちる（下の両ループが検出）。
    ///   実測 3 失敗＝本テスト＋既存 2 本（`scale_len_rounds_half_away_from_zero` も
    ///   同じ変異で死ぬ）。
    ///
    /// なお本テストの入力はすべて結果 ≥ 1 ゆえ、最小 1px クランプが丸めを覆い隠さない
    /// （min1px と丸めの檻を分離する）。
    #[test]
    fn scale_len_half_tie_pairs_pin_round_half_away_from_zero() {
        // 1/1000: 隣接入力で 0.499 / 0.500 / 0.501 の 3 点を対にする（ε＝1/1000）。
        let milli = ScaleRatio::new(1, 1000).unwrap();
        for m in 1u32..=6 {
            let base = 1000 * m;
            assert_eq!(milli.scale_len(base + 499), m, "{m}.499 は切り捨て");
            assert_eq!(milli.scale_len(base + 500), m + 1, "{m}.5 丁度は切り上げ");
            assert_eq!(milli.scale_len(base + 501), m + 1, "{m}.501 は切り上げ");
        }

        // 1/2: 端数なし（n）と丁度 .5（n+0.5）を全 n で対にする。
        // n が偶数の n+0.5 は round half to even なら n へ落ちるため、その変異も死ぬ。
        let half = ScaleRatio::new(1, 2).unwrap();
        for n in 1u32..=12 {
            assert_eq!(half.scale_len(2 * n), n, "端数なし n={n}");
            assert_eq!(half.scale_len(2 * n + 1), n + 1, "n+0.5 は上へ n={n}");
        }

        // DPI 対照表の k でも同様（3/2 と 7/4 の丁度 .5 と直下）。
        let k32 = ScaleRatio::new(3, 2).unwrap();
        assert_eq!(k32.scale_len(5), 8, "7.5 丁度 → 8");
        assert_eq!(k32.scale_len(4), 6, "6.0 は素通し");
        let k74 = ScaleRatio::new(7, 4).unwrap();
        assert_eq!(k74.scale_len(2), 4, "3.5 丁度 → 4");
        assert_eq!(
            k74.scale_len(6),
            11,
            "10.5 丁度 → 11（half-to-even なら 10）"
        );
        assert_eq!(k74.scale_len(5), 9, "8.75 → 9");
    }

    /// 要件 1.6: 乗算合成は可換かつ結合的で、真値（未約分の積）と厳密一致する。
    ///
    /// # 殺す変異（変異注入の実測に基づく。**排他的キルは持たない**）
    ///
    /// - `mul` の gcd 約分を**完全に**（積直後と縮退後の 2 箇所とも）落とす変異は、
    ///   `assert_eq!(a.mul(b), ScaleRatio::new(an·bn, ad·bd).unwrap())` の同値主張が殺す。
    ///   `new` は正準化する一方、約分を失った `mul` は未約分のまま返すため、`5/4 × 2/1` が
    ///   `{num:10, den:4}` vs `{num:5, den:2}` で食い違う（`Eq` はフィールド比較ゆえ、
    ///   両辺が正準形であるときに限り「同じ有理数」を意味する）。実測 3 失敗＝本テスト＋既存
    ///   `mul_composes_and_reduces`／`mul_uses_wide_intermediate_without_wrapping`。
    /// - **片側だけの gcd 削除は等価変異**（実測: 積直後のみ削除・縮退後のみ削除、どちらも全緑）。
    ///   縮退が起きない限り、どちらか一方が残っていれば結果は既約になるためである。片側削除を
    ///   殺すには「縮退経路で shrink が共通因子を作り直す」witness が要るが、本ファイルの
    ///   どのテストもそこへ到達していない。
    /// - 積の分子・分母を取り違える変異も同じ同値主張が殺す（既存 `mul_composes_and_reduces`
    ///   と共倒れ）。
    ///
    /// # 殺せない主張（契約の明文化であって檻ではない）
    ///
    /// 可換律・結合律のアサート自体は変異検出力を持たない。約分の有無に関わらず `(a·b)·c` と
    /// `a·(b·c)` はどちらも分子 `an·bn·cn`・分母 `ad·bd·cd` へ落ちるため、gcd を落としても
    /// 両辺が同時に動いて等式は保たれる。さらに 3 重ループは**同一 `TABLE` を独立に走査する**
    /// （`a=b=c` を許す）ため、既約後 `TABLE` の 7³=343 通りの上界は分子 2197（13³）・分母
    /// 4913（17³）——`mul` の飽和縮退（u32 超過）には**到達しない**。同節が触れる
    /// `ScaleRatio::new(an·bn, ad·bd)` は生値（96・120 を含む）を使うため別系統で上界
    /// 9216/14400 だが、これも到達しない。
    /// ゆえに「約分を落とすと中間値が u32 域を超えて縮退し `(a·b)·c != a·(b·c)` が破れる」という
    /// 機構は本テストでは発火し得ない。
    #[test]
    fn mul_is_commutative_and_associative() {
        const TABLE: [(u32, u32); 7] =
            [(5, 4), (3, 2), (7, 4), (2, 1), (1, 3), (96, 120), (13, 17)];
        for &(an, ad) in TABLE.iter() {
            let a = ScaleRatio::new(an, ad).unwrap();
            for &(bn, bd) in TABLE.iter() {
                let b = ScaleRatio::new(bn, bd).unwrap();
                assert_eq!(a.mul(b), b.mul(a), "可換: {an}/{ad} × {bn}/{bd}");
                assert_eq!(
                    a.mul(b),
                    ScaleRatio::new(an * bn, ad * bd).unwrap(),
                    "積は未約分の真値と同値: {an}/{ad} × {bn}/{bd}"
                );
                for &(cn, cd) in TABLE.iter() {
                    let c = ScaleRatio::new(cn, cd).unwrap();
                    assert_eq!(
                        a.mul(b).mul(c),
                        a.mul(b.mul(c)),
                        "結合律: {an}/{ad} × {bn}/{bd} × {cn}/{cd}"
                    );
                }
            }
        }
    }

    /// 要件 1.6: u32 域を超える積は「大きい側を `u32::MAX` へ張り付ける」飽和縮退になる。
    ///
    /// # 殺す変異（変異注入の実測に基づく。**排他的キルは持たない**）
    ///
    /// 下の 3 変異はいずれも既存 `mul_degrades_proportionally_when_product_exceeds_u32`
    /// （と、ログを見る `mul_degradation_emits_warn_log`）と共倒れになる。本テストの役割は
    /// 縮退の契約——「大きい側は `u32::MAX` ちょうど・小さい側は最小 1・決定論」——を
    /// 複数の base 族で明文化することである。
    ///
    /// - 縮退を「素朴な半減（`div_ceil(2)`）」へ差し替える実装（設計レビューで REJECT
    ///   された案）。`den == 1` の族では真値の **50%** へ落ちるため、下の厳密値と
    ///   相対誤差上限の双方が破れる。実測 3 失敗＝本テスト＋既存 2 本。
    /// - 大きい側の張り付け先を `u32::MAX` 以外（例 `u32::MAX/2`）にする変異
    ///   （`max(num, den) == u32::MAX` の主張が死ぬ）。実測 3 失敗＝本テスト＋既存 2 本。
    /// - 小さい側の `max(1)` を落とす変異。縮小後 `den == 0` となり、`gcd(u32::MAX, 0)` で
    ///   割った正準化の結果が `(1, 0)` へ落ちるため、まず `(sq.num, sq.den) == (u32::MAX, 1)`
    ///   の厳密値主張が発火する（`as_f32` も inf になるが、そこへ到達する前に死ぬ）。
    ///   実測 2 失敗＝本テスト＋既存 `mul_degrades_proportionally_when_product_exceeds_u32`。
    #[test]
    fn mul_saturating_degradation_pins_largest_to_u32_max() {
        for base in [65_536u32, 65_537, 100_000, 1_000_000, u32::MAX] {
            let up = ScaleRatio::new(base, 1).unwrap();
            let sq = up.mul(up);
            assert_eq!(
                (sq.num, sq.den),
                (u32::MAX, 1),
                "base={base}: 大きい側は u32::MAX へ張り付き、小さい側は最小 1"
            );
            assert_eq!(sq.num.max(sq.den), u32::MAX, "base={base}");
            assert!(sq.as_f32().is_finite() && sq.as_f32() > 0.0, "base={base}");

            // 逆数側（分母が溢れる対称ケース）。
            let down = ScaleRatio::new(1, base).unwrap();
            let sq_inv = down.mul(down);
            assert_eq!((sq_inv.num, sq_inv.den), (1, u32::MAX), "base={base}");
            assert_eq!(sq_inv.num.max(sq_inv.den), u32::MAX, "base={base}");

            // 決定論（同一入力は同一縮退）。
            assert_eq!(sq, up.mul(up), "base={base}");
            assert_eq!(sq_inv, down.mul(down), "base={base}");
        }

        // 真値が u32 域の直上にある族では、縮退後の比が真値へ十分近いこと。
        // 素朴な半減なら相対誤差 0.5 になり、この上限を破る。
        for base in [65_536u64, 65_537] {
            let truth = (base * base) as f64;
            let k = ScaleRatio::new(base as u32, 1)
                .unwrap()
                .mul(ScaleRatio::new(base as u32, 1).unwrap());
            let got = k.num as f64 / k.den as f64;
            let rel = (got - truth).abs() / truth;
            assert!(
                rel < 1.0e-3,
                "base={base}: 縮退後 {got} が真値 {truth} から乖離（相対誤差 {rel}）"
            );
        }
    }

    /// 要件 1.2/2.5: `as_f32` は**照会契約の出口ビュー**であり寸法権威ではない。
    ///
    /// doc 契約「寸法・画素演算にこの値を使ってはならない」を、f32 経路と
    /// [`ScaleRatio::scale_len`] が実際に食い違う具体例で固定する。誰かが `scale_len` を
    /// 「`as_f32` を使う実装」へ書き換えたら、下の `assert_ne!` と厳密値の双方が死ぬ。
    ///
    /// # アサーションの性格（契約の檻と、性質の記録の別）
    ///
    /// `assert_eq!(via_f32, 2_500_000_000)`（f32 経路の値）と
    /// `assert_ne!(as_f32(1/3) as f64, 1.0/3.0)` は、**本番コードの契約ではなく IEEE754
    /// binary32 の性質**を固定する主張である（24bit 仮数では `2_000_000_001` そのものが
    /// 丸められる／`1/3` は 2 冪分母でないため f32 で厳密表現できない）。本番実装を
    /// どう変えてもこの 2 行は動かない。ここに置く意図は「なぜ f32 を寸法権威にできないか」の
    /// 根拠を実行可能な形で残すことで、実装契約の檻は同じテスト内の `scale_len` 側の厳密値
    /// （`2_500_000_001`・最小 1px）が担う。
    ///
    /// # 殺す変異（変異注入の実測に基づく）
    ///
    /// - `scale_len` を `(len as f32 * self.as_f32()) as u32` へ差し替える（仮数欠落で
    ///   大寸が 1px ずれ、極小 k で 0 へ潰れる）。**既存と共倒れ**——実測 6 失敗＝本テスト・
    ///   `scale_len_half_tie_pairs_pin_round_half_away_from_zero`・
    ///   `scale_len_u128_intermediate_beats_u64_overflow`＋既存 3 本
    ///   （`scale_len_rounds_half_away_from_zero`／`dpi_table_scaled_extent_is_deterministic`／
    ///   `resample_zero_extent_is_empty_and_warns`）。
    ///   ただし丸めを保った穏当版 `(len as f32 * self.as_f32()).round() as u32` では既存が
    ///   全緑になり、本テストと `scale_len_u128_…` の**新 2 本だけが落ちる**——本テストの
    ///   固有価値はこの「f32 化が丸め規約を保っていても寸法権威にならない」域にある。
    /// - `as_f32` を `num as f32 / den as f32` 以外（例: 先に整数除算）へ変える。
    ///   **既存と共倒れ**——実測 3 失敗＝本テスト＋既存 `as_f32_yields_exact_dpi_values`／
    ///   `mul_degrades_proportionally_when_product_exceeds_u32`。
    /// - `scale_len` の最小 1px クランプを落とす。**既存と共倒れ**——実測 2 失敗＝本テスト＋
    ///   既存 `scale_len_clamps_nonzero_to_min_one_pixel`。
    #[test]
    fn as_f32_is_query_view_not_dimension_authority() {
        // 2 冪分母は f32 で厳密（照会値としての厳密性）。
        for (num, den, expect) in [
            (1u32, 2u32, 0.5f32),
            (1, 4, 0.25),
            (3, 8, 0.375),
            (7, 4, 1.75),
            (2, 1, 2.0),
            (9, 16, 0.5625),
        ] {
            assert_eq!(
                ScaleRatio::new(num, den).unwrap().as_f32(),
                expect,
                "{num}/{den}"
            );
        }
        // 非 2 冪は f32 で厳密表現できない（＝丸めの権威にできない）。
        assert_ne!(
            ScaleRatio::new(1, 3).unwrap().as_f32() as f64,
            1.0f64 / 3.0f64,
            "1/3 は f32 では厳密でない"
        );

        // 大寸: f32 の 24bit 仮数では原寸そのものが丸められ、結果が 1px ずれる。
        let k54 = ScaleRatio::new(5, 4).unwrap();
        assert_eq!(k54.as_f32(), 1.25);
        let len = 2_000_000_001u32;
        assert_eq!(k54.scale_len(len), 2_500_000_001, "整数権威の厳密値");
        let via_f32 = (len as f32 * k54.as_f32()) as u32;
        assert_eq!(via_f32, 2_500_000_000, "f32 経路は仮数欠落で 1px 少ない");
        assert_ne!(via_f32, k54.scale_len(len), "f32 は寸法権威になり得ない");

        // 極小: f32 の切り捨てキャストは表示を消すが、scale_len は最小 1px を守る。
        for (num, den) in [(1u32, 3u32), (2, 3), (1, 1000)] {
            let k = ScaleRatio::new(num, den).unwrap();
            assert_eq!(k.scale_len(1), 1, "{num}/{den}: 最小 1px");
            assert_eq!(
                (1.0f32 * k.as_f32()) as u32,
                0,
                "{num}/{den}: f32 キャストは 0 へ潰す"
            );
        }
    }

    /// 要件 2.5: `scale_len` の中間は **u128**——u64 では溢れる入力でも厳密値を返す。
    ///
    /// 既存の大寸テスト `scale_len_handles_large_extents_without_overflow` も
    /// `extreme = u32::MAX/(u32::MAX−1)` × `len = u32::MAX` で**既に u64 溢れ域を踏んでいる**
    /// ため、中間幅そのものの檻は既存にもある。既存に欠けていたのは、結果が `u32::MAX` へ
    /// 飽和しない witness——すなわち「**u64 なら溢れるのに真値は u32 域に収まる**」入力——で、
    /// これが無いと「溢れる域だけ `u32::MAX` へ逃げる」実装を見分けられない。本テストは
    /// その witness を構成し、飽和値ではない厳密値を主張する。
    ///
    /// - `k = (u32::MAX − 1)/u32::MAX`、`len = u32::MAX` のとき
    ///   `2·len·num ≈ 3.69e19 > u64::MAX ≈ 1.84e19`（u64 なら debug でパニック・
    ///   release ならラップ）。真値は `4_294_967_294`（＝飽和値 `u32::MAX` ではない）。
    ///
    /// # 殺す変異（変異注入の実測に基づく）
    ///
    /// - **既存と共倒れ**: `scale_len` の `u128` を `u64` へ落とす（オーバーフローで落ちる）。
    ///   実測 2 失敗＝本テスト＋既存 `scale_len_handles_large_extents_without_overflow`。
    /// - **排他的キル**: 「**u64 が溢れる域でのみ** `u32::MAX` へ逃げる」変異（溢れ判定を入れて
    ///   早期 `return u32::MAX`）。実測 1 失敗＝本テストのみ（既存は全て緑）。既存の大寸テストは
    ///   結果が `u32::MAX` へ飽和する族ばかりで飽和値と厳密値を区別できないため、この変異は
    ///   既存の檻を素通りする。本テストの固有価値はこの 1 変異に限定される。
    #[test]
    fn scale_len_u128_intermediate_beats_u64_overflow() {
        let k = ScaleRatio::new(u32::MAX - 1, u32::MAX).unwrap();
        assert!(!k.is_identity(), "既約のまま恒等短絡へ落ちない");

        // (2·len·num + den) / (2·den) を u128 で厳密に解いた値。
        assert_eq!(k.scale_len(u32::MAX), 4_294_967_294);
        assert_eq!(k.scale_len(4_000_000_000), 3_999_999_999);
        assert_eq!(
            k.scaled_extent(u32::MAX, 4_000_000_000),
            (4_294_967_294, 3_999_999_999),
            "外形も各軸へ同一権威を適用"
        );

        // 飽和値ではないこと（「無条件 u32::MAX」変異の直接の檻）。
        assert_ne!(k.scale_len(u32::MAX), u32::MAX);
        assert_ne!(k.scale_len(4_000_000_000), u32::MAX);

        // 決定論（同一入力は同一出力）。
        assert_eq!(k.scale_len(u32::MAX), k.scale_len(u32::MAX));
    }
}
