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
/// このフラグを見て `error!`／`warn!` を出すだけにする。本クレートには `tracing` 捕捉ハーネスが
/// 無いため（Cargo.toml 不触＝`tracing-subscriber` の dev 依存を足さない）、**縮退分岐が実際に
/// 選択されたこと**はこの構造体を直接検査する in-crate テストで檻に入れる
/// （戻り値だけでは「正常経路が偶然 1/1 を返した」場合と区別できない）。
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
}
