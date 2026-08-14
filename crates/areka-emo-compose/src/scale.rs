//! `ScaleRatio`: 表示スケールの数学（有理表現と丸めの単一権威）。
//!
//! DPI 追従表示の係数 k を**既約有理数** `num/den` で保持し、寸法の k 倍算（丸め）を
//! ここ 1 箇所へ集約する。`blit.rs` と同格の整数専用規約（決定性）に従い、画素・寸法演算に
//! 浮動小数（f32/f64）を一切持ち込まない。f32 が現れるのは照会契約の出口ビュー
//! [`ScaleRatio::as_f32`] のみである。
//!
//! - **既約正準化**（要件 1.1）: 構築時に gcd で約分し、`Eq`/`Hash` を正準形で厳密化する
//!   （下流 `emo-present` の合成キャッシュキーの一意性を担保する）。
//! - **丸め規約の単一権威（乗算方向・長さ）**（要件 2.5）: [`ScaleRatio::scale_len`] ／
//!   [`ScaleRatio::scaled_extent`] は round half away from zero（wintf `DPI::to_physical_*`
//!   と同規約）で丸め、非ゼロ入力に最小 1px を保証する（拡大結果が消える欠けを作らない）。
//! - **丸め規約の単一権威（除算方向・座標）**: [`ScaleRatio::unscale_coord`] は物理画素座標を
//!   native 画素座標へ縮約する唯一の写像で、[`resample`] の画素中心写像の最近傍逆をとる
//!   （当たり判定の点 ÷k がここを通る）。乗算方向権威と**対**を成すが互いの逆関数ではない。
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

    /// 物理画素**座標** → native 画素座標の縮約（**除算方向の丸め権威**・要件 2.1/2.2/2.5/1.5）。
    ///
    /// ```text
    /// s(v) = ⌊ ((2v + 1) · den) / (2 · num) ⌋      （⌊⌋ は Euclid 除算＝負値も床方向）
    /// ```
    ///
    /// [`resample`] が実際に用いた画素中心写像 `src = (v + 1/2)·den/num − 1/2` の**最近傍整数**
    /// ——すなわち「その表示画素に主として描かれている元画素」——を返す。当たり判定の
    /// 「見えているとおりの部位が当たる」は、この写像が描画写像と定義的に一致することに依拠する。
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

/// リサンプルの作業領域（x 軸写像表の席・`recompose-budget` 要件 3.1／design D2⑶）。
///
/// [`resample_with`] が行間で共有する x 軸写像表を**呼び手が所有**するための不透明な席である。
/// 中身は公開しない——保持しているのは私有型 [`AxisSample`] の表のみで、公開面に現れるのは
/// この型の名前だけである（`AxisSample` は私有のまま）。
///
/// [`Default`] で空から始まり、初回のリサンプルで出力幅ぶんの容量へ到達する。以後は
/// `clear`＋再充填で容量だけを持ち越すため、**同一の席を使い続ける限り容量は成長しない**
/// （出力幅が縮んでも解放しない＝縮小・再伸長の往復確保を作らない）。毎コマ経路で
/// この席を常設にすると、リサンプル内部の作業領域の確保が定常状態で 0 になる。
#[derive(Debug, Default)]
pub struct ResampleScratch {
    /// 行間で不変な x 軸の写像表（[`resample_with`] が毎回 `clear`＋再充填する）。
    x_map: Vec<AxisSample>,
}

impl ResampleScratch {
    /// 写像表がいま保持している容量（要素数）。
    ///
    /// **中身は返さない**——返すのは容量という 1 個の数だけで、写像表の要素型（私有）も
    /// その値も公開面には現れない（本型の不透明性は変わらない）。
    ///
    /// # 何のための観測口か（`recompose-budget` 要件 3.1）
    ///
    /// 席を持ち越す呼び手（`emo-present` の `FrameBudget`）が「この呼び出しで写像表を
    /// 確保し直したか」を**席そのものから**言い切るための唯一の口である。`Vec` の容量は
    /// 再確保でしか増えないため、[`resample_with`] の前後でこの値を読み比べれば、
    /// 増えた＝確保が起きた・変わらない＝確保は起きていない、が厳密に決まる。
    ///
    /// 呼び手が別に高水位を覚える形では、**席を毎回まっさらに起こす改変が計数に 1 件も
    /// 現れない**（高水位が前の実体の値を覚えたままになる）。容量を席から直接読むこの口が
    /// その抜け道を塞ぐ。
    ///
    /// 空の席は 0 を返す。初回のリサンプルで出力幅以上へ到達し、以後は縮まない。
    pub fn capacity(&self) -> usize {
        self.x_map.capacity()
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
/// 本関数は呼び出しごとに使い捨ての [`ResampleScratch`] を起こして [`resample_with`] へ委譲する
/// ——ゆえに毎回この 1 本を確保する。作業領域を呼び手が持ち越して確保をなくしたい場合は
/// [`resample_with`] を直接呼ぶこと（結果はバイト等価）。
/// リサンプルは k 変化・合成入力変化時のみ発火する経路である（design「Performance」）。
pub fn resample(src: &ComposedSurface, scale: ScaleRatio, out: &mut ComposedSurface) {
    // 使い捨ての作業領域で新形へ委譲する（挙動・出力バイトとも従来と同一）。
    let mut scratch = ResampleScratch::default();
    resample_with(src, scale, out, &mut scratch);
}

/// 作業領域受け取り形のリサンプル（`recompose-budget` 要件 3.1・additive）。
///
/// 転写の契約——事前条件・事後条件・恒等バイトコピー・整数専用・premultiplied ドメイン・
/// エッジクランプ・非パニック——は [`resample`] と**完全に同一**である（[`resample`] は本関数へ
/// 委譲するだけの薄い形であり、同一 `(src, scale)` に対する出力は 1 バイトも違わない）。
/// 唯一の差は、行間で共有する x 軸写像表を呼び手所有の `scratch` から借りる点にある。
///
/// # 作業領域の不変条件
///
/// `scratch` は入口で `clear` され、出力幅ぶんを再充填する。容量は出力幅へ到達した後は
/// 成長せず、より小さい出力幅の呼び出しでも縮まない（往復確保を作らない）。前回の内容は
/// `clear` で必ず捨てるため、使い回した席の残留が結果へ混ざることはない。恒等（k=1/1）と
/// 外形ゼロの早期復帰経路は `scratch` に一切触れない。
///
/// [`resample`]: crate::scale::resample
pub fn resample_with(
    src: &ComposedSurface,
    scale: ScaleRatio,
    out: &mut ComposedSurface,
    scratch: &mut ResampleScratch,
) {
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
    // 呼び手所有の席を clear＋再充填で使い回す（容量は到達後に成長しない・残留は残さない）。
    scratch.x_map.clear();
    scratch.x_map.reserve(out_w as usize);
    let mut walk = AxisWalk::new(scale);
    for _ in 0..out_w {
        scratch.x_map.push(walk.sample(src.width()));
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

        for (dx, xs) in scratch.x_map.iter().enumerate() {
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
