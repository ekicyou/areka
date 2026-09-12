//! `ScaleRatio`: 表示スケールの数学（有理表現と丸めの単一権威）。
//!
//! DPI 追従表示の係数 k を**既約有理数** `num/den` で保持し、寸法の k 倍算（丸め）を
//! ここ 1 箇所へ集約する。`blit.rs` と同格の整数専用規約（決定性）に従い、画素・寸法演算に
//! 浮動小数（f32/f64）を一切持ち込まない。f32 が現れるのは照会契約の出口ビュー
//! [`ScaleRatio::as_f32`] のみである。
//!
//! **唯一の例外（裁定済み）**: 唯一の既知の例外は emo-text
//! `ScaleContract::physical_extent`（文字供給面の確保寸）であり、2026-08-14 の裁定
//! （spec `areka-P0-scale-exact-rational`）に基づく。誤差は +1 側のみで不可視。
//! **この例外を他の用途へ拡大してはならない**——例外は供給面寸のこの 1 点に限られ、
//! 新たな f32 寸法演算を許す一般則ではない。
//!
//! - **既約正準化**（要件 1.1）: 構築時に gcd で約分し、`Eq`/`Hash` を正準形で厳密化する
//!   （下流 `emo-present` の合成キャッシュキーの一意性を担保する）。
//! - **丸め規約の単一権威（乗算方向・長さ）**（要件 2.5）: [`ScaleRatio::scale_len`] ／
//!   [`ScaleRatio::scaled_extent`] は round half away from zero（wintf `DPI::to_physical_*`
//!   と同規約）で丸め、非ゼロ入力に最小 1px を保証する（拡大結果が消える欠けを作らない）。
//! - **丸め規約の単一権威（除算方向・座標）**: [`ScaleRatio::unscale_coord`] は物理画素座標を
//!   native 画素座標へ縮約する唯一の写像で、GPU の線形サンプリングのテクセル中心規約の
//!   最近傍逆をとる（当たり判定の点 ÷k がここを通る）。乗算方向権威と**対**を成すが
//!   互いの逆関数ではない。
//! - **乗算合成**（要件 1.6）: 最終拡大率＝アプリ管理拡大率 × DPI 由来 k を
//!   [`ScaleRatio::mul`] の有理数乗算として表現する（本仕様のアプリ管理拡大率は
//!   [`ScaleRatio::ONE`] 固定の縮退シーム）。
//! - **恒等**（要件 1.3）: 窓 DPI＝作者基準 DPI のとき k=1/1 となり、
//!   [`ScaleRatio::is_identity`] が真・`scale_len` は入力を素通しする（既存等倍表示と同一）。
//!
//! # 本 module は**寸法権威だけ**である（画素を 1 バイトも触らない）
//!
//! 拡大縮小そのものは wintf の描画経路（`Arrangement.scale` が生む D2D の変換行列）が行う。
//! かつてここに在った CPU の k 倍リサンプラ（整数固定小数点 bilinear・重み表・軸走査・
//! 作業席）は消費者が 0 になったため撤去した（spec `areka-P0-present-gpu-transform-scale`・
//! 要件 1.4）。`ScaleRatio` の数学（伸長・寸法の丸め・座標の縮約）はそのとき 1 文字も
//! 変えていない（要件 1.5）。
//!
//! CPU リサンプルの**高速化・別スレッド化・合成メモの容量増は採らない**（要件 1.6）——
//! 提示段は原寸のバイト列しか持たず拡大は GPU が行うので、速くする対象そのものが無い。
//! 画素を書き換える拡大経路をここへ戻してはならない。
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
    /// # 裁定済みの消費者は 2 つだけである
    ///
    /// 1. **供給面寸の導出**（emo-text `ScaleContract::physical_extent`＝文字供給面の確保寸）。
    ///    2026-08-14 の裁定（spec `areka-P0-scale-exact-rational`）に基づく。誤差は +1 側のみで
    ///    不可視。
    /// 2. **表示の拡大係数**（emo-present が surface entity の `Arrangement.scale` へ書く値）。
    ///    2026-09-11 の裁定（spec `areka-P0-present-gpu-transform-scale`）に基づく。これは寸法を
    ///    整数で刻む用途ではなく、描画側（D2D の変換行列）へ渡す**係数そのもの**であり、
    ///    画素の格子は GPU が持つ。窓寸・物理寸は従来どおり [`scaled_extent`] が決める。
    ///
    /// **この 2 例を他の用途へ拡大してはならない**——例外は上の 2 点に限られ、
    /// 上の禁止は他の全ての用途に対してそのまま効く。
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

    /// 物理画素**座標** → native 画素座標の縮約（**除算方向の丸め権威**・要件 2.1/2.2/2.5/1.5）。
    ///
    /// ```text
    /// s(v) = ⌊ ((2v + 1) · den) / (2 · num) ⌋      （⌊⌋ は Euclid 除算＝負値も床方向）
    /// ```
    ///
    /// GPU の線形サンプリングのテクセル中心規約（`dst 中心 (d+½) → src (d+½)·den/num − ½`）の
    /// **最近傍整数**——すなわち「その表示画素に主として描かれている元画素」——を返す。
    /// 当たり判定の「見えているとおりの部位が当たる」は、この写像が描画写像と定義的に
    /// 一致することに依拠する。
    ///
    /// # 乗算方向権威との対（責務の相互参照）
    ///
    /// [`scaled_extent`]／[`scale_len`] が**乗算方向（native → 物理）の「長さ」の丸め権威**であるのに対し、
    /// 本メソッドは**除算方向（物理 → native）の「座標」の丸め権威**である。両者は対を成すが
    /// **互いの逆関数ではない**——長さの丸めは round half away from zero、座標の丸めは画素中心逆写像で
    /// 規約そのものが異なる（長さの丸めを鏡写しにすると整数倍 k で半画素ずれる: k=2 の表示画素 101 は
    /// 元画素 50 を映すのに 51 を返してしまう）。
    ///
    /// # 座標専用（長さの縮約には使わない）
    ///
    /// 引数は**点の座標**であり、寸法・長さを渡してはならない。物理寸から native 寸を得たい場合に
    /// 本メソッドを使うのは誤り（`+1/2` の中心補正が入るため長さの丸めにならない）。
    ///
    /// # 端の注意
    ///
    /// [`scale_len`] が切り上げた最終物理画素では `s(v)` が native 寸を 1 だけ超え得る
    /// （例: native 27・k=7/6 → 物理 32px の最終列 31 は 27 を返し、有効添字 `0..=26` の外側になる）。
    /// 当たり判定矩形は native 寸の内側にあるため、この値は照合で自然に「該当なし」となる
    /// ——定義された結果であり、異常でも panic 事象でもない。
    ///
    /// # 規約変更の権威
    ///
    /// ÷k の丸め規約を変える改修は**本メソッド 1 箇所**で行う（経路ごとの丸め持ち込みを禁ずる）。
    /// 変更時は下流の期待値檻と実機受け入れ記録の再実施が必要である。
    ///
    /// # 桁溢れと飽和（panic なし）
    ///
    /// 中間は i128——`v: i64` ゆえ `2v+1` は 2^64 域、`den ≤ u32::MAX` を掛けても 2^96 域で溢れない。
    /// `num ≥ 1`（`ScaleRatio` 不変条件）ゆえゼロ除算もない。k<1（`num < den`）では極値近傍の `v` で
    /// 結果が i64 域を超え得るため、戻り値は **i64 へ飽和縮小**する（`as` のラップは単調性を破り、
    /// `try_into().unwrap()` は非パニック宣言を破るため、飽和が唯一の整合解）。単調非減少は
    /// 非飽和域で成立し、飽和域では定値になる。Win32 の実座標は i32 域に束縛されるため、
    /// 実経路で飽和は発生しない（防御規約）。
    ///
    /// # 性質（in-source 檻で固定）
    ///
    /// - k=1 で厳密恒等 `s(v) = v`（負値・i64 極値を含む全域）。
    /// - `v` について単調非減少（非飽和域）——サーフェス px の閉区間矩形の逆像が物理空間でも
    ///   連続区間になり、境界画素の内外一貫が k によらず保存される。
    /// - 決定論・整数のみ（f32 を一切経由しない）。
    ///
    /// # 公開面の申し送り（W6.5 `scale-exact-rational`）
    ///
    /// 本 spec が `scale.rs` へ追加する公開面は本メソッドのみである（`num`／`den` アクセサは
    /// **新設しない**——W6.5 が計画する `ratio()` 等との名前二重化を避けるため）。W6.5 は設計前に
    /// 本メソッド着地後の `scale.rs` へ rebase すること。
    ///
    /// [`scale_len`]: ScaleRatio::scale_len
    /// [`scaled_extent`]: ScaleRatio::scaled_extent
    pub fn unscale_coord(self, v: i64) -> i64 {
        let num = self.num as i128;
        let den = self.den as i128;
        // 画素中心逆写像の最近傍整数（Euclid 除算ゆえ負値も床方向・num ≥ 1 でゼロ除算なし）。
        let s = ((2 * v as i128 + 1) * den).div_euclid(2 * num);
        // i64 への飽和縮小（k<1 の極値近傍でのみ到達し得る）。
        s.clamp(i64::MIN as i128, i64::MAX as i128) as i64
    }
}

#[cfg(test)]
#[path = "scale_test_support.rs"]
mod test_support;

#[cfg(test)]
#[path = "scale_resample_tests.rs"]
mod resample_tests;

#[cfg(test)]
#[path = "scale_ratio_tests.rs"]
mod ratio_tests;

#[cfg(test)]
#[path = "scale_prior_path_tests.rs"]
mod prior_path_tests;
