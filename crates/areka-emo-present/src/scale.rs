//! k の**政策**（`ScalePolicy`・[`derive_scale`]）＝ 作者基準 DPI・アプリ管理拡大率シーム・
//! DPI 取得不能時の縮退を、提示段（`presenter`）の外へ純関数として括り出した層。
//!
//! 責務分界（design「Architecture Integration」）: k の**数学**（既約有理表現・丸め・リサンプル）は
//! 上流 `areka-emo-compose` の `scale`（[`ScaleRatio`]）が単一権威として持ち、本モジュールは
//! 「どの数を掛けるか」という**政策**のみを決める。k の**適用点**は `presenter` の表示経路 1 箇所、
//! k の**時間軸**（DPI 変化・初期 k₀）は上位アプリ（`areka`）の領分である。
//!
//! - **導出規約**（要件 1.1・設計 D2）: k ＝ アプリ管理拡大率 × `窓 dpi_x ÷ author_dpi`。
//!   整数段階へ量子化せず**連続**（既約有理数）で保持する——125% 表示スケール（120/96＝5/4）が
//!   段階丸めで潰れないため（要件 2.2）。
//! - **2 因子モデル**（要件 1.6）: 最終拡大率はアプリ管理拡大率と DPI 由来 k の**乗算合成**である。
//!   本仕様のアプリ管理拡大率は [`ScaleRatio::ONE`] 固定の**縮退シーム**（実設定手段は将来 spec）
//!   だが、乗算そのものは実在の経路として実装・テストされる（席を潰さない）。
//! - **表示を失わない縮退**（要件 1.4）: DPI 取得不能・窓 DPI 不正・author_dpi 不正のいずれでも
//!   パニックせず、最悪でも `app_scale × 1/1`（＝等倍表示）へ落として表示を継続する。
//! - **log-first・無言縮退の禁止**（steering `logging.md`）: 上記の縮退分岐はすべて
//!   `error!`／`warn!` で観測可能にする。ログ無しで静かに 1.0 へ落ちる経路を作らない。
//!
//! 本モジュールは GPU・wintf いずれにも依存しない純関数であり、全分岐が in-crate の
//! `#[cfg(test)]` で実行テストされる（設計「Testing Strategy > Unit Tests」）。

use areka_emo_compose::ScaleRatio;

/// 作者基準 DPI の既定値（ukadoc 正典・設計 D1）。
///
/// shell は descript の `seriko.dpi`、balloon は descript の `dpi` で宣言する。無宣言・不正・0 は
/// この値を採る（読み取り側＝`areka` の placement source が担い、本モジュールは受け取った値の
/// **最終防衛**として 0 をここへ正規化する）。
pub const DEFAULT_AUTHOR_DPI: u16 = 96;

/// target（窓）ごとの拡大政策（attach 時に確定・以後不変）。
///
/// 窓ごとに保持されることで、DPI の異なる複数モニタに窓が同時に存在しても各窓が自窓の DPI 由来 k
/// で表示される（要件 1.5・保持は `presenter` の `PresentTarget` が行う）。
///
/// # 不変条件と構築（`author_dpi != 0`）
///
/// `author_dpi == 0` は k の分母ゼロ＝導出不能ゆえ、[`ScalePolicy::new`] が構築時に
/// [`DEFAULT_AUTHOR_DPI`] へ正規化して `warn!` を出す（設計「Error Handling」の
/// 「author_dpi 不正・0」行）。フィールドは design の Service Interface どおり公開のままとし、
/// 構造体リテラル直書きで正規化を迂回されても [`derive_scale`] が同じ正規化を再度行う
/// （**多層防御**——どちらの入口から入っても k=0 やゼロ除算にはならない）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScalePolicy {
    /// 作者基準 DPI（既定 [`DEFAULT_AUTHOR_DPI`]・k の分母）。
    pub author_dpi: u16,
    /// アプリ管理拡大率（本仕様は [`ScaleRatio::ONE`] 固定の縮退シーム・要件 1.6）。
    pub app_scale: ScaleRatio,
}

impl Default for ScalePolicy {
    /// 既定政策＝作者基準 DPI 96・アプリ管理拡大率 1/1（＝窓 DPI 96 で恒等 k）。
    fn default() -> Self {
        ScalePolicy {
            author_dpi: DEFAULT_AUTHOR_DPI,
            app_scale: ScaleRatio::ONE,
        }
    }
}

impl ScalePolicy {
    /// 政策を構築する（`author_dpi == 0` は [`DEFAULT_AUTHOR_DPI`] へ正規化＋`warn!`）。
    ///
    /// 本仕様の呼び手（`attach_target`）は `app_scale` に [`ScaleRatio::ONE`] を渡す
    /// （アプリ管理拡大率 1.0 固定シーム・要件 1.6）。将来 spec がアプリ管理拡大率を導入する際は
    /// ここへ非 ONE を渡すだけで 2 因子乗算が成立する。
    pub fn new(author_dpi: u16, app_scale: ScaleRatio) -> ScalePolicy {
        ScalePolicy {
            author_dpi: normalize_author_dpi(author_dpi),
            app_scale,
        }
    }
}

/// `author_dpi` の正規化（0 → [`DEFAULT_AUTHOR_DPI`]・`warn!`）。
///
/// 0 は k の分母として使えない（[`ScaleRatio::new`] が `None` を返す＝導出不能）ため、
/// 既定値へ寄せて表示を失わせない。無言では通さない（log-first）。
fn normalize_author_dpi(author_dpi: u16) -> u16 {
    if author_dpi == 0 {
        tracing::warn!(
            author_dpi,
            normalized = DEFAULT_AUTHOR_DPI,
            "ScalePolicy: author_dpi=0 は k の分母に使えない: 既定 96 へ正規化"
        );
        DEFAULT_AUTHOR_DPI
    } else {
        author_dpi
    }
}

/// [`derive_scale`] の判定結果＝実適用 k ＋**どの縮退分岐を通ったか**。
///
/// ログ発火の判断を [`classify`]（純粋・ログ無し）へ集約し、[`derive_scale`] は
/// このフラグを見て `error!`／`warn!` を出すだけにする。**縮退分岐が実際に選択されたこと**は
/// この構造体を直接検査する in-crate テストで檻に入れる（戻り値だけでは
/// 「正常経路が偶然 1/1 を返した」場合と区別できない）。
///
/// なお `error!`／`warn!` が**実際に発火するか**も、本ファイル `mod tests` の
/// ログ捕捉ハーネス（`tracing` 単体で手書き・`tracing-subscriber` の dev 依存は不要）で
/// 別途檻に入れてある（task 6.2）。分岐選択とログ発火は独立した 2 つの契約である。
///
/// 各フラグは独立に立ち得る（例: `author_dpi == 0` かつ `dpi_x != dpi_y`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ScaleDecision {
    /// 実適用 k（縮退経路でも常に有効値＝表示を失わない）。
    scale: ScaleRatio,
    /// 窓 DPI が取得できなかった（`DPI` component 不在・要件 1.4）。
    dpi_missing: bool,
    /// 窓 DPI の値が不正（`dpi_x == 0`＝比を構築できない）。
    window_dpi_invalid: bool,
    /// `dpi_x != dpi_y`（設計 D2: `dpi_x` を採用）。
    anisotropic: bool,
    /// `author_dpi == 0` を [`DEFAULT_AUTHOR_DPI`] へ正規化した。
    author_dpi_normalized: bool,
}

/// 政策と窓 DPI から k と縮退分岐を決める**ログ無しの純粋関数**。
///
/// 縮退の事後条件（要件 1.4）: どの分岐でも `scale` は有効な有理数であり、最悪でも
/// `policy.app_scale`（＝アプリ管理拡大率のみ・DPI 由来 k は 1/1）へ落ちる。表示は失われない。
fn classify(policy: ScalePolicy, dpi: Option<(u16, u16)>) -> ScaleDecision {
    let author_dpi_normalized = policy.author_dpi == 0;
    // 0 のときのみ既定へ寄せる（以降 author_dpi は必ず非ゼロ＝分母として有効）。
    let author_dpi = if author_dpi_normalized {
        DEFAULT_AUTHOR_DPI
    } else {
        policy.author_dpi
    };

    let Some((dpi_x, dpi_y)) = dpi else {
        // DPI component 不在＝取得不能: DPI 由来 k を 1/1 とみなす（要件 1.4 の k=1.0 縮退）。
        return ScaleDecision {
            scale: policy.app_scale,
            dpi_missing: true,
            window_dpi_invalid: false,
            anisotropic: false,
            author_dpi_normalized,
        };
    };

    // 単一スカラー規約（設計 D2）: 軸ごとに DPI が異なる環境では dpi_x を採用する。
    let anisotropic = dpi_x != dpi_y;

    match ScaleRatio::new(dpi_x as u32, author_dpi as u32) {
        // 正常経路: 最終拡大率＝アプリ管理拡大率 × DPI 由来 k（要件 1.6 の 2 因子乗算）。
        Some(k) => ScaleDecision {
            scale: policy.app_scale.mul(k),
            dpi_missing: false,
            window_dpi_invalid: false,
            anisotropic,
            author_dpi_normalized,
        },
        // 分母は正規化済みゆえ非ゼロ——ここへ来るのは dpi_x == 0（窓 DPI の値そのものが不正）。
        // 比を構築できないため DPI 不在と同じ縮退（app_scale × 1/1）を採る。
        None => ScaleDecision {
            scale: policy.app_scale,
            dpi_missing: false,
            window_dpi_invalid: true,
            anisotropic,
            author_dpi_normalized,
        },
    }
}

/// 実適用 k を導出する（表示 show 適用ごとに呼ぶ・数命令）。
///
/// design「emo-present / scale.rs」の Service Interface どおり:
///
/// - **正常**: `app_scale × ScaleRatio::new(dpi_x, author_dpi)`（要件 1.1/1.6）。
/// - **`dpi == None`**（`DPI` component 不在＝窓 DPI 取得不能）: `error!` ＋ `app_scale × 1/1`
///   （要件 1.4 の k=1.0 縮退・表示を失わない）。
/// - **`dpi_x != dpi_y`**: `warn!` ＋ `dpi_x` 採用（設計 D2 の単一スカラー規約）。
/// - **`author_dpi == 0`**: `warn!` ＋ 96 採用（[`ScalePolicy::new`] を迂回した場合の最終防衛）。
/// - **`dpi_x == 0`**（窓 DPI の値が不正）: `error!` ＋ `app_scale × 1/1`。
///
/// # 不変条件
///
/// - **純関数**: 同一入力→同一出力。内部に可変状態を持たない（ログ抑止のための静的カウンタ等も
///   持たない——design のコメントは `dpi_x != dpi_y` の `warn!` を「初回」と記すが、抑止状態は
///   純関数性と両立しないため**毎回警告する**方を採る。異軸 DPI は稀環境ゆえ実害は小さく、
///   「無言で dpi_y を捨てた」痕跡が必ず残る方が log-first に適う）。
/// - **恒等**（要件 1.3）: `author_dpi == dpi_x` かつ `app_scale == ONE` なら
///   [`ScaleRatio::is_identity`] が真（＝既存の等倍表示と同一寸・同一バイト）。
/// - **非パニック**: あらゆる入力（0・`u16::MAX`）でパニックしない。
pub fn derive_scale(policy: ScalePolicy, dpi: Option<(u16, u16)>) -> ScaleRatio {
    let decision = classify(policy, dpi);

    if decision.author_dpi_normalized {
        tracing::warn!(
            author_dpi = policy.author_dpi,
            normalized = DEFAULT_AUTHOR_DPI,
            "derive_scale: author_dpi=0 は k の分母に使えない: 既定 96 へ正規化"
        );
    }
    if decision.dpi_missing {
        tracing::error!(
            author_dpi = policy.author_dpi,
            app_scale = policy.app_scale.as_f32(),
            k = decision.scale.as_f32(),
            "derive_scale: 窓 DPI を取得できない（DPI component 不在）: k=アプリ管理拡大率×1/1 へ縮退して表示を継続"
        );
    }
    // 以降の 2 分岐は窓 DPI が存在するときにのみ立つ（`classify` の `Some` 経路）。
    if let Some((dpi_x, dpi_y)) = dpi {
        if decision.window_dpi_invalid {
            tracing::error!(
                dpi_x,
                dpi_y,
                author_dpi = policy.author_dpi,
                k = decision.scale.as_f32(),
                "derive_scale: 窓 DPI が不正（0）で比を構築できない: k=アプリ管理拡大率×1/1 へ縮退して表示を継続"
            );
        }
        if decision.anisotropic {
            tracing::warn!(
                dpi_x,
                dpi_y,
                "derive_scale: dpi_x != dpi_y（異軸 DPI）: 単一スカラー規約により dpi_x を採用"
            );
        }
    }

    decision.scale
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 本仕様の既定政策（作者基準 DPI 96・アプリ管理拡大率 1/1 固定シーム）。
    fn policy_96() -> ScalePolicy {
        ScalePolicy::new(DEFAULT_AUTHOR_DPI, ScaleRatio::ONE)
    }

    /// 縮退フラグが 1 つも立っていない（＝正常経路を通った）ことを主張する。
    fn assert_no_degradation(d: ScaleDecision) {
        assert!(
            !d.dpi_missing && !d.window_dpi_invalid && !d.anisotropic && !d.author_dpi_normalized,
            "正常経路では縮退分岐が立たない: {d:?}"
        );
    }

    /// 要件 1.1: 正常経路は `窓 DPI ÷ author_dpi` の既約有理数を返す（DPI 対照表・連続 k）。
    ///
    /// 125%（120/96＝5/4）が段階丸めで潰れないことが要件 2.2 の前提であるため、
    /// 非整数倍の水準を対照表へ含める。
    #[test]
    fn derive_scale_follows_dpi_table() {
        for (dpi, expect) in [
            (96u16, (1u32, 1u32)),
            (120, (5, 4)),
            (144, (3, 2)),
            (168, (7, 4)),
            (192, (2, 1)),
            (72, (3, 4)),
        ] {
            let k = derive_scale(policy_96(), Some((dpi, dpi)));
            assert_eq!(
                k,
                ScaleRatio::new(expect.0, expect.1).unwrap(),
                "dpi={dpi}: 窓 DPI ÷ author_dpi の既約有理数"
            );
            assert_no_degradation(classify(policy_96(), Some((dpi, dpi))));
        }
        // 隣接水準が同一 k へ潰れない（整数段階量子化の不在＝要件 2.2 の連続性）。
        assert_ne!(
            derive_scale(policy_96(), Some((120, 120))),
            derive_scale(policy_96(), Some((144, 144)))
        );
    }

    /// 要件 1.1: author_dpi は k の分母として実際に効く（96 ハードコードでない証拠）。
    #[test]
    fn derive_scale_uses_declared_author_dpi_as_denominator() {
        let p = ScalePolicy::new(120, ScaleRatio::ONE);
        assert_eq!(
            derive_scale(p, Some((240, 240))),
            ScaleRatio::new(2, 1).unwrap(),
            "author_dpi=120・窓 240 → 2/1"
        );
        assert_eq!(
            derive_scale(p, Some((96, 96))),
            ScaleRatio::new(4, 5).unwrap(),
            "author_dpi が窓 DPI より大きければ k<1（縮小）"
        );
    }

    /// 要件 1.3: 窓 DPI ＝ author_dpi かつ app_scale=ONE のとき恒等（k=1.0・等倍表示と同一）。
    #[test]
    fn derive_scale_is_identity_when_window_dpi_equals_author_dpi() {
        assert!(derive_scale(policy_96(), Some((96, 96))).is_identity());
        // 作者が 144 を宣言していれば恒等点も 144 へ移る（恒等は 96 固定ではない）。
        let p144 = ScalePolicy::new(144, ScaleRatio::ONE);
        assert!(derive_scale(p144, Some((144, 144))).is_identity());
        assert!(!derive_scale(p144, Some((96, 96))).is_identity());
        assert!(!derive_scale(policy_96(), Some((192, 192))).is_identity());
    }

    /// 要件 1.4: DPI 不在（`DPI` component 取得不能）は `app_scale × 1/1` へ縮退する。
    ///
    /// アプリ管理拡大率が非 ONE でも「DPI 由来 k のみ 1/1 になる」ことまで固定する
    /// （app_scale ごと捨てる実装との差を検出する）。
    #[test]
    fn derive_scale_without_dpi_degrades_to_app_scale() {
        assert_eq!(
            derive_scale(policy_96(), None),
            ScaleRatio::ONE,
            "app=ONE なら k=1.0（表示を失わない）"
        );
        let app2 = ScaleRatio::new(2, 1).unwrap();
        assert_eq!(
            derive_scale(ScalePolicy::new(96, app2), None),
            app2,
            "DPI 由来 k のみ 1/1 へ縮退し、アプリ管理拡大率は保たれる"
        );

        let d = classify(policy_96(), None);
        assert!(d.dpi_missing, "DPI 不在分岐が選択される: {d:?}");
        assert!(!d.window_dpi_invalid && !d.anisotropic && !d.author_dpi_normalized);
        // 正常経路では立たない（分岐の非空虚性）。
        assert!(!classify(policy_96(), Some((96, 96))).dpi_missing);
    }

    /// 設計 D2: `dpi_x != dpi_y` は dpi_x を採用する（単一スカラー規約）。
    ///
    /// 「パニックしない」ではなく**採用軸そのもの**を固定する——dpi_y 採用や平均採用なら落ちる。
    #[test]
    fn derive_scale_adopts_dpi_x_when_axes_differ() {
        assert_eq!(
            derive_scale(policy_96(), Some((192, 96))),
            ScaleRatio::new(2, 1).unwrap(),
            "dpi_x=192 を採用（dpi_y=96 採用なら 1/1 になる）"
        );
        assert_eq!(
            derive_scale(policy_96(), Some((96, 192))),
            ScaleRatio::ONE,
            "dpi_x=96 を採用（dpi_y=192 採用なら 2/1 になる）"
        );
        assert_eq!(
            derive_scale(policy_96(), Some((120, 192))),
            ScaleRatio::new(5, 4).unwrap(),
            "非整数 k でも dpi_x 採用（平均採用なら 13/8 相当になる）"
        );

        let d = classify(policy_96(), Some((192, 96)));
        assert!(d.anisotropic, "異軸分岐が選択される: {d:?}");
        assert!(!d.dpi_missing && !d.window_dpi_invalid);
        assert!(
            !classify(policy_96(), Some((96, 96))).anisotropic,
            "同軸では立たない（非空虚性）"
        );
    }

    /// 要件 1.1/1.4: `author_dpi == 0` は 96 へ正規化される（分母ゼロで表示を失わない）。
    ///
    /// 構造体リテラル直書き（[`ScalePolicy::new`] 迂回）でも [`derive_scale`] 側の最終防衛が
    /// 効くことを固定する。
    #[test]
    fn derive_scale_normalizes_zero_author_dpi() {
        let bare = ScalePolicy {
            author_dpi: 0,
            app_scale: ScaleRatio::ONE,
        };
        for dpi in [96u16, 120, 192] {
            assert_eq!(
                derive_scale(bare, Some((dpi, dpi))),
                derive_scale(policy_96(), Some((dpi, dpi))),
                "author_dpi=0 は author_dpi=96 と同一の k を与える（dpi={dpi}）"
            );
        }
        let d = classify(bare, Some((192, 192)));
        assert!(d.author_dpi_normalized, "正規化分岐が選択される: {d:?}");
        assert!(!d.dpi_missing && !d.window_dpi_invalid);
        assert!(
            !classify(policy_96(), Some((192, 192))).author_dpi_normalized,
            "非ゼロでは立たない（非空虚性）"
        );

        // 構築時（正規の入口）にも正規化される。
        assert_eq!(
            ScalePolicy::new(0, ScaleRatio::ONE).author_dpi,
            DEFAULT_AUTHOR_DPI
        );
        assert_eq!(ScalePolicy::new(120, ScaleRatio::ONE).author_dpi, 120);
        assert_eq!(ScalePolicy::default(), policy_96());
    }

    /// 要件 1.4: 窓 DPI が 0（不正値）でも比を構築せず `app_scale × 1/1` へ縮退する。
    #[test]
    fn derive_scale_degrades_on_zero_window_dpi() {
        assert_eq!(derive_scale(policy_96(), Some((0, 0))), ScaleRatio::ONE);
        let app2 = ScaleRatio::new(2, 1).unwrap();
        assert_eq!(
            derive_scale(ScalePolicy::new(96, app2), Some((0, 0))),
            app2,
            "アプリ管理拡大率は保たれる"
        );

        let d = classify(policy_96(), Some((0, 96)));
        assert!(d.window_dpi_invalid, "窓 DPI 不正分岐が選択される: {d:?}");
        assert!(!d.dpi_missing, "DPI 不在とは別分岐（別ログ）");
        assert!(
            d.anisotropic,
            "0 と 96 は異軸でもある（フラグは独立に立つ）"
        );
    }

    /// 要件 1.6: 最終拡大率＝アプリ管理拡大率 × DPI 由来 k（2 因子乗算が実在する証拠）。
    ///
    /// 本仕様の本番値は `app_scale == ONE` 固定だが、非 ONE が正しく乗るかを固定しておかないと
    /// 「シームがある」という主張は空虚になる（将来 spec が導入した瞬間に壊れる）。
    #[test]
    fn derive_scale_multiplies_app_scale_seam() {
        let app2 = ScaleRatio::new(2, 1).unwrap();
        assert_eq!(
            derive_scale(ScalePolicy::new(96, app2), Some((120, 120))),
            ScaleRatio::new(5, 2).unwrap(),
            "アプリ 2.0 × DPI 1.25 = 2.5"
        );
        let half = ScaleRatio::new(1, 2).unwrap();
        assert_eq!(
            derive_scale(ScalePolicy::new(96, half), Some((192, 192))),
            ScaleRatio::ONE,
            "アプリ 0.5 × DPI 2.0 = 1.0（相殺）"
        );
        // ONE 固定シームは DPI 由来 k をそのまま通す（恒等元）。
        assert_eq!(
            derive_scale(policy_96(), Some((120, 120))),
            ScaleRatio::new(5, 4).unwrap()
        );
    }

    /// Invariants: 同一入力→同一出力（純関数・隠れた可変状態を持たない）。
    ///
    /// 「初回のみ警告」のような抑止状態を内部へ持てば、同一入力の反復で分岐結果が変化し得る。
    /// 反復して `ScaleDecision` ごと一致することで、そのような状態が無いことを固定する。
    #[test]
    fn derive_scale_is_deterministic() {
        let p = ScalePolicy::new(96, ScaleRatio::new(3, 2).unwrap());
        for dpi in [
            None,
            Some((96u16, 96u16)),
            Some((120, 96)),
            Some((0, 0)),
            Some((192, 192)),
        ] {
            let first = classify(p, dpi);
            for _ in 0..3 {
                assert_eq!(classify(p, dpi), first, "dpi={dpi:?}: 分岐結果が反復不変");
                assert_eq!(
                    derive_scale(p, dpi),
                    first.scale,
                    "dpi={dpi:?}: k が反復不変"
                );
            }
        }
    }

    /// 非パニック: 極値（`u16::MAX`・0 の全組合せ）でも有効な k を返す（表示を失わない）。
    #[test]
    fn derive_scale_never_panics_and_always_yields_usable_scale() {
        for author in [0u16, 1, 96, u16::MAX] {
            for dpi in [
                None,
                Some((0u16, 0u16)),
                Some((1, 1)),
                Some((u16::MAX, 1)),
                Some((1, u16::MAX)),
                Some((u16::MAX, u16::MAX)),
            ] {
                let policy = ScalePolicy {
                    author_dpi: author,
                    app_scale: ScaleRatio::ONE,
                };
                let k = derive_scale(policy, dpi);
                assert!(
                    k.as_f32() > 0.0 && k.as_f32().is_finite(),
                    "author={author} dpi={dpi:?}: k は常に正の有限値（表示が消えない）"
                );
                // 有効な k は寸法へ適用でき、非ゼロ原寸は最小 1px を保つ（丸め権威側の契約）。
                assert!(k.scale_len(100) >= 1);
            }
        }
    }

    // ── 縮退ログ発火の檻（task 6.2・task 1.4 申し送りの回収）─────────────────────────
    //
    // task 1.4 時点で檻に入っていたのは私有 `ScaleDecision` の**分岐選択**だけであり、
    // `error!`／`warn!` が実際に発火するかは無検査だった（steering `logging.md` の
    // 「ログ無し失敗経路の禁止」＝縮退の唯一の観測点が空証明のまま）。ここで実行テストへ落とす。
    //
    // 捕捉は **`tracing` 単体**で組む——`tracing-subscriber` は本 crate の dev 依存に無く、
    // 要件 7.3（新規外部依存の禁止）ゆえ足さない（tasks.md Implementation Notes の
    // 「3.4: `areka-emo-present` でもログ発火を檻に入れられる」が 1.4 の申し送りを上書き済み）。
    // `presenter.rs` の同型ハーネスは同ファイルの `mod tests` に私有で import できないため、
    // 単一ファイル境界内へ最小複製する（`areka/src/emo2_boot/adapter.rs` が確立した流儀）。
    //
    // # `with_default` のスレッドローカル性だけでは足りない（callsite interest 毒化）
    //
    // `with_default` が差し替えるのはスレッドローカルの既定 dispatcher だが、
    // **callsite の interest キャッシュはプロセス大域**であり「その callsite をプロセス内で
    // 最初に踏んだスレッドが勝つ」。以下 `tracing-core-0.1.36` の実コード:
    //
    // - `DefaultCallsite::interest()` はキャッシュ未設定（`0xFF`）のとき `register()` を呼ぶ。
    // - `register()`（`callsite.rs:307-318`）は
    //   `rebuild_callsite_interest(self, &DISPATCHERS.rebuilder())` を実行する。
    // - `Dispatchers::rebuilder()`（同 :544-549）は `has_just_one` が真のとき
    //   `Rebuilder::JustOne` を返し、`for_each`（同 :562-567）はこれを
    //   **`dispatcher::get_default(f)`＝登録したスレッドの既定 dispatcher**で評価する。
    // - subscriber を持たないスレッドの既定は `NoSubscriber` で、その `register_callsite` は
    //   **`Interest::never()`**（`subscriber.rs:676-678`）を返す。
    // - こうして焼かれた `never` は `interest.is_never()` の早期 return でイベントを捨てる。
    //   本ファイルの縮退ログ callsite は捕捉しない他テスト（`derive_scale_*` の値検査群・
    //   `presenter.rs` の GPU テスト群）と共有されているため、**捕捉窓の内側でも取りこぼす**。
    //
    // 対策（構造的）: **プロセス寿命の probe dispatcher を 2 個常駐**させて
    // `has_just_one`（＝`dispatchers.len() <= 1`・`callsite.rs:551-558`）を恒久的に偽へ落とす。
    // 偽になれば `rebuilder()` は `Rebuilder::Read` を返し、interest は「生存する登録済み
    // dispatcher 全体の `Interest::and`」で決まる——`get_default`（毒の入口）は二度と参照されない。
    // probe の `register_callsite` は常に `Interest::sometimes()` を返し、`Interest::and` は
    // 「両者が異なれば必ず `sometimes`」（`subscriber.rs:652-658`）ゆえ**合成結果は決して
    // `never` にならない**。`sometimes` は「毎回 `enabled()` を訊く」＝ interest キャッシュが
    // 実質無効化された状態であり、判定は現スレッドの dispatcher（＝捕捉 subscriber）へ委ねられる。
    // probe 導入前に焼かれた `never` は捕捉窓の内側で `rebuild_interest_cache()` 1 回で解毒する。
    //
    // `set_global_default` はプロセス大域の既定を1度きりで奪うため使わない（probe は
    // `Dispatch::new` による**登録**のみで、既定 dispatcher は差し替えない）。

    /// 捕捉した 1 イベント（level ＋ フィールド名 → Debug 表現）。
    #[derive(Debug, Clone)]
    struct CapturedEvent {
        level: tracing::Level,
        fields: std::collections::HashMap<String, String>,
    }

    impl CapturedEvent {
        /// `message` フィールド（本文）。無ければ空文字（panic しない）。
        fn message(&self) -> &str {
            self.fields.get("message").map(String::as_str).unwrap_or("")
        }

        /// 構造化フィールドの Debug 表現。欠落は失敗（フィールド名も契約のうち）。
        fn field(&self, name: &str) -> &str {
            self.fields
                .get(name)
                .unwrap_or_else(|| panic!("ログフィールド `{name}` が無い: {:?}", self.fields))
        }
    }

    /// 全フィールドを Debug 表現で拾う visitor。
    ///
    /// [`tracing::field::Visit`] の `record_u64`/`record_f64`/`record_str` 等はすべて既定実装が
    /// `record_debug` へ転送するため、`record_debug` 1 本で型を問わず全フィールドを捕捉できる。
    struct FieldGrab(std::collections::HashMap<String, String>);

    impl tracing::field::Visit for FieldGrab {
        fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
            self.0
                .insert(field.name().to_string(), format!("{value:?}"));
        }
    }

    /// イベントを溜めるだけの最小 subscriber（span は使わないので new_span は固定 id を返す）。
    #[derive(Clone, Default)]
    struct CaptureSubscriber(std::sync::Arc<std::sync::Mutex<Vec<CapturedEvent>>>);

    impl tracing::Subscriber for CaptureSubscriber {
        fn enabled(&self, _meta: &tracing::Metadata<'_>) -> bool {
            true
        }
        fn new_span(&self, _attrs: &tracing::span::Attributes<'_>) -> tracing::span::Id {
            tracing::span::Id::from_u64(1)
        }
        fn record(&self, _span: &tracing::span::Id, _values: &tracing::span::Record<'_>) {}
        fn record_follows_from(&self, _span: &tracing::span::Id, _follows: &tracing::span::Id) {}
        fn event(&self, event: &tracing::Event<'_>) {
            let mut grab = FieldGrab(std::collections::HashMap::new());
            event.record(&mut grab);
            self.0
                .lock()
                .expect("捕捉バッファの毒化なし")
                .push(CapturedEvent {
                    level: *event.metadata().level(),
                    fields: grab.0,
                });
        }
        fn enter(&self, _span: &tracing::span::Id) {}
        fn exit(&self, _span: &tracing::span::Id) {}
    }

    /// interest キャッシュへ `never` を焼かせないための常駐 dispatcher。
    ///
    /// `register_callsite` が常に `Interest::sometimes()` を返すことだけが仕事で、
    /// `enabled()` は偽・`event()` は no-op（他テストの観測へ副作用を与えない）。
    struct InterestProbe;

    impl tracing::Subscriber for InterestProbe {
        fn register_callsite(
            &self,
            _meta: &'static tracing::Metadata<'static>,
        ) -> tracing::subscriber::Interest {
            // 既定実装は `enabled()` が偽なら `never` を返してしまう。ここを `sometimes` に
            // 固定することが本 probe の唯一の存在理由。
            tracing::subscriber::Interest::sometimes()
        }
        fn enabled(&self, _meta: &tracing::Metadata<'_>) -> bool {
            false
        }
        fn new_span(&self, _attrs: &tracing::span::Attributes<'_>) -> tracing::span::Id {
            tracing::span::Id::from_u64(1)
        }
        fn record(&self, _span: &tracing::span::Id, _values: &tracing::span::Record<'_>) {}
        fn record_follows_from(&self, _span: &tracing::span::Id, _follows: &tracing::span::Id) {}
        fn event(&self, _event: &tracing::Event<'_>) {}
        fn enter(&self, _span: &tracing::span::Id) {}
        fn exit(&self, _span: &tracing::span::Id) {}
    }

    /// probe dispatcher を**2 個**プロセス寿命で常駐させる（冪等）。
    ///
    /// 2 個必要なのは `has_just_one = (dispatchers.len() <= 1)` ゆえ——1 個では登録直後に
    /// `has_just_one` が真のままとなり、次の `register_dispatch` までの隙間で
    /// `Rebuilder::JustOne`（毒の経路）が生き残る。2 個目の登録で確定的に偽へ落とす。
    /// `OnceLock` が `Arc` をプロセス寿命で保持するので `retain(upgrade)` でも落ちない。
    fn ensure_interest_probes() {
        static PROBES: std::sync::OnceLock<(tracing::Dispatch, tracing::Dispatch)> =
            std::sync::OnceLock::new();
        PROBES.get_or_init(|| {
            // `Dispatch::new` が `callsite::register_dispatch` を呼ぶ（登録＋全走査再計算）。
            let first = tracing::Dispatch::new(InterestProbe);
            let second = tracing::Dispatch::new(InterestProbe);
            (first, second)
        });
    }

    /// クロージャ実行中に**現在のスレッド**で発火した tracing イベントを戻り値と共に返す。
    ///
    /// callsite interest 毒化への対策は本モジュール冒頭のコメントを参照。
    fn capture<R, F: FnOnce() -> R>(f: F) -> (R, Vec<CapturedEvent>) {
        ensure_interest_probes();

        let cap = CaptureSubscriber::default();
        // `with_default` は内部で `Dispatch::new`（＝register_dispatch＋全 callsite 再計算）を
        // 行うため、この時点で既存の `never` は解毒されている。
        let out = tracing::subscriber::with_default(cap.clone(), || {
            // probe 常駐前（プロセス起動〜初回捕捉）に焼かれた `never` の掃き残しを、
            // 窓が開いた**後**の時点でもう一度確定的に潰す。
            tracing::callsite::rebuild_interest_cache();
            f()
        });
        let events = cap.0.lock().expect("捕捉バッファの毒化なし").clone();
        (out, events)
    }

    /// メッセージに `needle` を含むイベントが**ちょうど 1 件**在ることを主張して返す。
    fn expect_one<'a>(events: &'a [CapturedEvent], needle: &str) -> &'a CapturedEvent {
        let hits: Vec<&CapturedEvent> = events
            .iter()
            .filter(|e| e.message().contains(needle))
            .collect();
        assert_eq!(
            hits.len(),
            1,
            "`{needle}` を含むログがちょうど 1 件ではない: {events:?}"
        );
        hits[0]
    }

    /// メッセージに `needle` を含むイベント数。
    fn count_msg(events: &[CapturedEvent], needle: &str) -> usize {
        events
            .iter()
            .filter(|e| e.message().contains(needle))
            .count()
    }

    /// 要件 1.4／設計「Error Handling」: DPI 不在縮退は **`error!`** を発火する。
    ///
    /// レベル（error）とメッセージ識別子・構造化フィールド（`author_dpi`/`app_scale`/`k`）を
    /// 契約として固定する。`k` フィールドには**実適用値**が載る（app_scale 非 ONE で `1.0` 固定
    /// でないことまで見るので、ログ値を定数直書きへ変異させると落ちる）。
    #[test]
    fn derive_scale_missing_dpi_emits_error_log() {
        let p = policy_96();
        let (k, events) = capture(|| derive_scale(p, None));
        assert_eq!(k, ScaleRatio::ONE);

        let ev = expect_one(&events, "窓 DPI を取得できない");
        assert_eq!(
            ev.level,
            tracing::Level::ERROR,
            "DPI 取得不能は error 格（warn/debug へ落とすと縮退が観測できない）: {ev:?}"
        );
        assert_eq!(ev.field("author_dpi"), "96");
        assert_eq!(ev.field("app_scale"), "1.0");
        assert_eq!(ev.field("k"), "1.0");
        assert_eq!(
            events.len(),
            1,
            "他分岐のログを巻き添えで出さない: {events:?}"
        );

        // app_scale 非 ONE では k フィールドもそれに追随する（定数 1.0 直書きでない証拠）。
        let app2 = ScalePolicy::new(96, ScaleRatio::new(2, 1).expect("2/1"));
        let (k2, events2) = capture(|| derive_scale(app2, None));
        assert_eq!(k2.as_f32(), 2.0);
        let ev2 = expect_one(&events2, "窓 DPI を取得できない");
        assert_eq!(ev2.field("k"), "2.0");
        assert_eq!(ev2.field("app_scale"), "2.0");
    }

    /// 要件 1.4／設計「Error Handling」: 窓 DPI 不正（0）縮退は **`error!`** を発火し、
    /// DPI 不在とは**別メッセージ**（別分岐であることがログから判別できる）。
    #[test]
    fn derive_scale_zero_window_dpi_emits_error_log() {
        let p = policy_96();
        let (k, events) = capture(|| derive_scale(p, Some((0, 0))));
        assert_eq!(k, ScaleRatio::ONE);

        let ev = expect_one(&events, "窓 DPI が不正");
        assert_eq!(
            ev.level,
            tracing::Level::ERROR,
            "窓 DPI 不正は error 格: {ev:?}"
        );
        assert_eq!(ev.field("dpi_x"), "0");
        assert_eq!(ev.field("dpi_y"), "0");
        assert_eq!(ev.field("author_dpi"), "96");
        assert_eq!(ev.field("k"), "1.0");
        assert_eq!(
            count_msg(&events, "取得できない"),
            0,
            "DPI 不在の文言と混ざらない（分岐の識別子が別）: {events:?}"
        );
        // dpi_x == dpi_y == 0 ゆえ異軸警告は立たない（0,0 は同軸）。
        assert_eq!(events.len(), 1, "1 分岐 1 ログ: {events:?}");
    }

    /// 設計 D2: 異軸 DPI（`dpi_x != dpi_y`）は **`warn!`**＋採用軸の実値を残す。
    ///
    /// 「無言で dpi_y を捨てた」痕跡が必ず残ることが D2 の観測条件であり、
    /// `dpi_x`/`dpi_y` 両方がフィールドに載ることまで契約に含める。
    #[test]
    fn derive_scale_anisotropic_dpi_emits_warn_log() {
        let p = policy_96();
        let (k, events) = capture(|| derive_scale(p, Some((192, 96))));
        assert_eq!(k, ScaleRatio::new(2, 1).expect("2/1"));

        let ev = expect_one(&events, "異軸 DPI");
        assert_eq!(
            ev.level,
            tracing::Level::WARN,
            "異軸 DPI は warn 格（表示は成立するので error ではない）: {ev:?}"
        );
        assert_eq!(ev.field("dpi_x"), "192");
        assert_eq!(ev.field("dpi_y"), "96", "捨てた軸の値も残す");
        assert_eq!(events.len(), 1, "1 分岐 1 ログ: {events:?}");
    }

    /// 要件 1.1/1.4: `author_dpi == 0` の最終防衛（[`derive_scale`] 側）は **`warn!`**＋
    /// 生の宣言値と正規化後の値を並べて残す。
    ///
    /// [`ScalePolicy::new`] 迂回（構造体リテラル直書き）で入っても無言では通らない。
    #[test]
    fn derive_scale_zero_author_dpi_emits_warn_log() {
        let bare = ScalePolicy {
            author_dpi: 0,
            app_scale: ScaleRatio::ONE,
        };
        let (k, events) = capture(|| derive_scale(bare, Some((192, 192))));
        assert_eq!(k, ScaleRatio::new(2, 1).expect("192/96 = 2/1"));

        let ev = expect_one(&events, "derive_scale: author_dpi=0");
        assert_eq!(ev.level, tracing::Level::WARN, "{ev:?}");
        assert_eq!(
            ev.field("author_dpi"),
            "0",
            "生の宣言値（正規化前）を載せる"
        );
        assert_eq!(ev.field("normalized"), "96");
        assert_eq!(events.len(), 1, "1 分岐 1 ログ: {events:?}");
    }

    /// 要件 1.1/1.4: 正規の入口 [`ScalePolicy::new`] の 0 正規化も **`warn!`**（無言正規化の禁止）。
    ///
    /// [`derive_scale`] 側の最終防衛とは**別メッセージ**（どちらの層で正規化されたか判別できる）。
    #[test]
    fn scale_policy_new_zero_author_dpi_emits_warn_log() {
        let (p, events) = capture(|| ScalePolicy::new(0, ScaleRatio::ONE));
        assert_eq!(p.author_dpi, DEFAULT_AUTHOR_DPI);

        let ev = expect_one(&events, "ScalePolicy: author_dpi=0");
        assert_eq!(ev.level, tracing::Level::WARN, "{ev:?}");
        assert_eq!(ev.field("author_dpi"), "0");
        assert_eq!(ev.field("normalized"), "96");
        assert_eq!(events.len(), 1, "1 分岐 1 ログ: {events:?}");

        // 非ゼロ構築は無言（「常に warn」変異の非空虚性）。
        let (p2, events2) = capture(|| ScalePolicy::new(120, ScaleRatio::ONE));
        assert_eq!(p2.author_dpi, 120);
        assert!(events2.is_empty(), "正常構築は無言: {events2:?}");
    }

    /// 正常経路は**完全に無言**（`debug!` すら出さない）。
    ///
    /// 上の 5 本が主張する「レベル・フィールド」の非空虚性はここで担保される——
    /// 「常にログを出す」実装なら、DPI 対照表の全水準でこのテストが落ちる。
    #[test]
    fn derive_scale_normal_path_is_silent() {
        let p = policy_96();
        for dpi in [96u16, 120, 144, 168, 192, 72] {
            let (_, events) = capture(|| derive_scale(p, Some((dpi, dpi))));
            assert!(events.is_empty(), "正常経路は無言（dpi={dpi}）: {events:?}");
        }
        // 非 ONE の app_scale でも同様（2 因子合成は縮退ではない）。
        let app = ScalePolicy::new(144, ScaleRatio::new(3, 2).expect("3/2"));
        let (_, events) = capture(|| derive_scale(app, Some((168, 168))));
        assert!(events.is_empty(), "2 因子合成も無言: {events:?}");
    }

    /// 縮退フラグは独立に立ち、**各々が自分のログを出す**（1 本にまとめて握り潰さない）。
    #[test]
    fn derive_scale_emits_each_degradation_log_independently() {
        // author_dpi=0 かつ DPI 不在 → warn（正規化）＋ error（DPI 不在）の 2 本。
        let bare = ScalePolicy {
            author_dpi: 0,
            app_scale: ScaleRatio::ONE,
        };
        let (k, events) = capture(|| derive_scale(bare, None));
        assert_eq!(k, ScaleRatio::ONE);
        assert_eq!(count_msg(&events, "derive_scale: author_dpi=0"), 1);
        assert_eq!(count_msg(&events, "窓 DPI を取得できない"), 1);
        assert_eq!(
            count_msg(&events, "異軸 DPI"),
            0,
            "DPI 不在時に存在しない dpi_x/dpi_y を騙らない: {events:?}"
        );
        assert_eq!(events.len(), 2, "{events:?}");

        // dpi_x=0 かつ dpi_y=96 → error（窓 DPI 不正）＋ warn（異軸）の 2 本。
        let (k, events) = capture(|| derive_scale(policy_96(), Some((0, 96))));
        assert_eq!(k, ScaleRatio::ONE);
        assert_eq!(count_msg(&events, "窓 DPI が不正"), 1);
        assert_eq!(count_msg(&events, "異軸 DPI"), 1);
        assert_eq!(events.len(), 2, "{events:?}");
    }

    /// Invariants（設計 D2 の実装時是正）: 縮退ログは**毎回**出る。
    ///
    /// 「初回のみ警告」の抑止状態を持てば純関数性が壊れる、というのが 1.4 レビューの裁定であり、
    /// その裁定は「同じ入力を反復したときログ件数が呼出回数に比例する」ことでしか観測できない
    /// （[`derive_scale_is_deterministic`] は戻り値の反復不変までしか見ていない）。
    #[test]
    fn derive_scale_repeats_degradation_log_on_every_call() {
        let p = policy_96();
        let (_, events) = capture(|| {
            for _ in 0..3 {
                derive_scale(p, Some((192, 96)));
            }
        });
        assert_eq!(
            count_msg(&events, "異軸 DPI"),
            3,
            "抑止状態（once / 初回のみ）を持たない: {events:?}"
        );

        let (_, events) = capture(|| {
            for _ in 0..3 {
                derive_scale(p, None);
            }
        });
        assert_eq!(count_msg(&events, "窓 DPI を取得できない"), 3, "{events:?}");
    }
}
