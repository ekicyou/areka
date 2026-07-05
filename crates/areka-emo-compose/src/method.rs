//! 合成メソッド写像表: `ComposeMethod` / `BlendMode` enum と式ステータス・dispatch シーム。
//!
//! ukadoc 由来の描画メソッド全量を `#[non_exhaustive]` enum として列挙し、各メソッドの
//! 実装状態（実装済み＝`overlay`・型シームのみ＝それ以外）を保持する。emo2 実測で使用される
//! `overlay`（および同義の `add`/`bind`）のみ実挙動を持ち、`overlay-fast`・`interpolate`・
//! `replace`・`asis`・`reduce`・`blend-*` 群などは未実装シームとして口だけを保つ。未実装メソッド
//! 参照時はパニックせず `warn` 以上のログで観測可能に扱う（実際のスキップ／ログは下流
//! `blit` 実行器の責務であり、本モジュールは登録簿＝型シームのみを提供する）。
//!
//! # 表現の選択（BlendMode）
//!
//! design の enum スケッチは `Blend(BlendMode)`＋「各 `fast: bool` を保持」とだけ規定する。
//! 本実装は `BlendMode { kind: BlendKind, fast: bool }` の構造体表現を採る。理由:
//! ブレンド種別（`BlendKind`・19 種の Photoshop 相当合成）と `-fast` 変調フラグは
//! 直交する二軸であり、`Multiply` と `Multiply-fast` を別 variant にすると列挙が 2 倍に
//! 膨らみ網羅 match が冗長化する。種別軸のみを `#[non_exhaustive] enum BlendKind` として
//! 全量列挙し、`fast` は直交フラグとして構造体に持たせることで、写像表（design
//! 「合成メソッド写像表」）の全ブレンドモードを最小の列挙で忠実に表現する。

use tracing::warn;

/// ukadoc 由来の合成（描画）メソッド全量列挙。
///
/// emo2 実測で使用されるのは `Overlay` のみ（`add`/`bind` は ukadoc 同義明文により
/// `Overlay` へ写像）。それ以外は将来拡張のための型シームであり `is_implemented() == false`。
///
/// `#[non_exhaustive]` により下流クレートからの網羅 match を禁じ、将来のメソッド追加に
/// 対する後方互換シームを保つ（本クレート内の網羅 match はテストで許容される）。
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComposeMethod {
    /// 単に重ねる（premultiplied SourceOver）。**実装対象（emo2 使用分）**。
    ///
    /// ukadoc 同義明文により `add`（着せ替え別名）・`bind` も本 variant へ写像する。
    Overlay,
    /// ベースの不透明度に応じて重ねる（旧称 `overlayfast`）。シーム。
    OverlayFast,
    /// ベースの透明度に応じて重ねる（`overlay-fast` の対・DestOver 相当）。シーム。
    Interpolate,
    /// src 範囲内をαごと完全上書き・範囲外は無操作。シーム。
    Replace,
    /// src の透過を無視して不透明扱いで重ねる。シーム。
    Asis,
    /// ベース完全置換・XY 無視（先頭層以外は overlay 読替）。シーム。
    Base,
    /// 不透明度の乗算（切り抜き・RGB 無視・範囲外は消去）。シーム。
    Reduce,
    /// Photoshop 相当のレイヤ合成群（`blend-*`）。`-fast` 変種は [`BlendMode`] 側で保持。シーム。
    Blend(BlendMode),
    /// element source オプション併用時のみレイヤ情報から自動推定（SSP 2.8.41）。シーム。
    Auto,
    /// 将来の転記層メソッド追加に備えた吸収口（未知メソッド名を原文保持）。シーム。
    Unknown(Box<str>),
}

/// `blend-*` 群のモード表現。ブレンド種別 [`BlendKind`] と `-fast`（ベース不透明度変調）
/// フラグの直交する二軸を保持する。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlendMode {
    /// ブレンド種別（Photoshop 相当）。
    pub kind: BlendKind,
    /// `-fast` 変種か（ベースの不透明度に応じて変調する）。旧称 `overlaymultiply` 等は
    /// `fast: true` に写像される。
    pub fast: bool,
}

impl BlendMode {
    /// 通常（非 fast）ブレンドモードを構築する。
    pub const fn new(kind: BlendKind) -> Self {
        Self { kind, fast: false }
    }

    /// `-fast` 変種ブレンドモードを構築する。
    pub const fn fast(kind: BlendKind) -> Self {
        Self { kind, fast: true }
    }
}

/// Photoshop 相当ブレンド種別の全量列挙（SSP 2.8.35〜2.8.40）。
///
/// `#[non_exhaustive]` により将来のモード追加に対する後方互換シームを保つ。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendKind {
    /// 加算合成（`blend-add`）。
    Add,
    /// 乗算合成（`blend-multiply`・旧 `overlaymultiply`＝fast）。
    Multiply,
    /// スクリーン合成（`blend-screen`・旧 `overlayscreen`＝fast）。
    Screen,
    /// オーバーレイ合成（`blend-overlay`・`overlay` メソッドとは無関係）。
    Overlay,
    /// 比較（暗）合成（`blend-darken`）。
    Darken,
    /// 比較（明）合成（`blend-lighten`）。
    Lighten,
    /// カラー比較（暗）合成（`blend-darker-color`）。
    DarkerColor,
    /// カラー比較（明）合成（`blend-lighter-color`）。
    LighterColor,
    /// 焼き込み（`blend-color-burn`）。
    ColorBurn,
    /// 覆い焼き（`blend-color-dodge`）。
    ColorDodge,
    /// ハードライト（`blend-hard-light`）。
    HardLight,
    /// ハードミックス（`blend-hard-mix`）。
    HardMix,
    /// 差の絶対値（`blend-difference`）。
    Difference,
    /// 除外（`blend-exclusion`）。
    Exclusion,
    /// 除算（`blend-divide`）。
    Divide,
    /// 色相（`blend-hue`）。
    Hue,
    /// 彩度（`blend-saturation`）。
    Saturation,
    /// カラー（`blend-color`）。
    Color,
    /// 輝度（`blend-luminosity`）。
    Luminosity,
}

impl ComposeMethod {
    /// このメソッドが実挙動を持つ（実装済みである）か。
    ///
    /// M1 の実装対象は `Overlay` のみ（emo2 使用分）。それ以外は型シームであり `false`。
    /// 実際の画素合成は下流 `blit` 実行器が担い、本メソッドは登録簿としての実装状況のみを返す。
    pub fn is_implemented(&self) -> bool {
        matches!(self, ComposeMethod::Overlay)
    }

    /// 転記層由来のメソッド名文字列を [`ComposeMethod`] へ写像する（写像表準拠）。
    ///
    /// 大文字小文字は無視する。ukadoc 同義明文により `add`/`bind` は [`Overlay`] へ、
    /// 旧書式別名（`overlaymultiply` = `blend-multiply-fast` 等）も写像表どおりに解決する。
    /// 未知の名前は原文を保持した [`Unknown`] へ吸収する。
    ///
    /// [`Overlay`]: ComposeMethod::Overlay
    /// [`Unknown`]: ComposeMethod::Unknown
    pub fn from_name(name: &str) -> ComposeMethod {
        let lower = name.trim().to_ascii_lowercase();
        // ハイフン／アンダースコアを吸収してから照合する（`overlay-fast`/`overlayfast` 等）。
        let canon = lower.replace(['-', '_'], "");
        match canon.as_str() {
            // overlay 同義群（ukadoc 同義明文）。
            "overlay" | "add" | "bind" => ComposeMethod::Overlay,
            "overlayfast" => ComposeMethod::OverlayFast,
            "interpolate" => ComposeMethod::Interpolate,
            "replace" => ComposeMethod::Replace,
            "asis" => ComposeMethod::Asis,
            "base" => ComposeMethod::Base,
            "reduce" => ComposeMethod::Reduce,
            "auto" => ComposeMethod::Auto,
            other => {
                if let Some(blend) = parse_blend(other) {
                    ComposeMethod::Blend(blend)
                } else {
                    warn!(method = %name, "未知の合成メソッド名: Unknown シームへ吸収");
                    ComposeMethod::Unknown(name.into())
                }
            }
        }
    }
}

/// 正規化済み（小文字・区切り除去）メソッド名を [`BlendMode`] へ写像する。
///
/// 新書式 `blend<kind>`／`blend<kind>fast` に加え、旧書式別名
/// `overlaymultiply`（= `blend-multiply-fast`）・`overlayscreen`（= `blend-screen-fast`）を
/// 写像表どおりに解決する。該当しなければ `None`。
fn parse_blend(canon: &str) -> Option<BlendMode> {
    // 旧書式別名（fast 固定）。
    match canon {
        "overlaymultiply" => return Some(BlendMode::fast(BlendKind::Multiply)),
        "overlayscreen" => return Some(BlendMode::fast(BlendKind::Screen)),
        _ => {}
    }
    let rest = canon.strip_prefix("blend")?;
    let (kind_str, fast) = match rest.strip_suffix("fast") {
        Some(head) => (head, true),
        None => (rest, false),
    };
    let kind = match kind_str {
        "add" => BlendKind::Add,
        "multiply" => BlendKind::Multiply,
        "screen" => BlendKind::Screen,
        "overlay" => BlendKind::Overlay,
        "darken" => BlendKind::Darken,
        "lighten" => BlendKind::Lighten,
        "darkercolor" => BlendKind::DarkerColor,
        "lightercolor" => BlendKind::LighterColor,
        "colorburn" => BlendKind::ColorBurn,
        "colordodge" => BlendKind::ColorDodge,
        "hardlight" => BlendKind::HardLight,
        "hardmix" => BlendKind::HardMix,
        "difference" => BlendKind::Difference,
        "exclusion" => BlendKind::Exclusion,
        "divide" => BlendKind::Divide,
        "hue" => BlendKind::Hue,
        "saturation" => BlendKind::Saturation,
        "color" => BlendKind::Color,
        "luminosity" => BlendKind::Luminosity,
        _ => return None,
    };
    Some(BlendMode { kind, fast })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 網羅 match ガード（Req 8.1）: `ComposeMethod` の全 variant を名指しする。
    ///
    /// `#[non_exhaustive]` はクレート外の網羅 match のみを禁じるため、クレート内の本テストは
    /// 全 variant を列挙できる。variant を追加／削除するとこの match が壊れ、写像表の
    /// 全量列挙契約の逸脱を静的に検出する。
    #[test]
    fn compose_method_is_fully_enumerated() {
        fn name(m: &ComposeMethod) -> &'static str {
            match m {
                ComposeMethod::Overlay => "overlay",
                ComposeMethod::OverlayFast => "overlay-fast",
                ComposeMethod::Interpolate => "interpolate",
                ComposeMethod::Replace => "replace",
                ComposeMethod::Asis => "asis",
                ComposeMethod::Base => "base",
                ComposeMethod::Reduce => "reduce",
                ComposeMethod::Blend(_) => "blend-*",
                ComposeMethod::Auto => "auto",
                ComposeMethod::Unknown(_) => "unknown",
            }
        }
        // 代表値で全 variant を踏む。
        let all = [
            ComposeMethod::Overlay,
            ComposeMethod::OverlayFast,
            ComposeMethod::Interpolate,
            ComposeMethod::Replace,
            ComposeMethod::Asis,
            ComposeMethod::Base,
            ComposeMethod::Reduce,
            ComposeMethod::Blend(BlendMode::new(BlendKind::Multiply)),
            ComposeMethod::Auto,
            ComposeMethod::Unknown("future".into()),
        ];
        assert_eq!(all.len(), 10);
        for m in &all {
            assert!(!name(m).is_empty());
        }
    }

    /// 網羅 match ガード（Req 8.1）: `BlendKind` の全 19 種を名指しする。
    #[test]
    fn blend_kind_is_fully_enumerated() {
        fn name(k: BlendKind) -> &'static str {
            match k {
                BlendKind::Add => "add",
                BlendKind::Multiply => "multiply",
                BlendKind::Screen => "screen",
                BlendKind::Overlay => "overlay",
                BlendKind::Darken => "darken",
                BlendKind::Lighten => "lighten",
                BlendKind::DarkerColor => "darker-color",
                BlendKind::LighterColor => "lighter-color",
                BlendKind::ColorBurn => "color-burn",
                BlendKind::ColorDodge => "color-dodge",
                BlendKind::HardLight => "hard-light",
                BlendKind::HardMix => "hard-mix",
                BlendKind::Difference => "difference",
                BlendKind::Exclusion => "exclusion",
                BlendKind::Divide => "divide",
                BlendKind::Hue => "hue",
                BlendKind::Saturation => "saturation",
                BlendKind::Color => "color",
                BlendKind::Luminosity => "luminosity",
            }
        }
        let all = [
            BlendKind::Add,
            BlendKind::Multiply,
            BlendKind::Screen,
            BlendKind::Overlay,
            BlendKind::Darken,
            BlendKind::Lighten,
            BlendKind::DarkerColor,
            BlendKind::LighterColor,
            BlendKind::ColorBurn,
            BlendKind::ColorDodge,
            BlendKind::HardLight,
            BlendKind::HardMix,
            BlendKind::Difference,
            BlendKind::Exclusion,
            BlendKind::Divide,
            BlendKind::Hue,
            BlendKind::Saturation,
            BlendKind::Color,
            BlendKind::Luminosity,
        ];
        assert_eq!(all.len(), 19);
        for k in all {
            assert!(!name(k).is_empty());
        }
    }

    /// Req 8.2/8.3: `Overlay` のみ実装済み・他はすべて型シーム（未実装）。
    #[test]
    fn only_overlay_is_implemented() {
        assert!(ComposeMethod::Overlay.is_implemented());

        assert!(!ComposeMethod::OverlayFast.is_implemented());
        assert!(!ComposeMethod::Interpolate.is_implemented());
        assert!(!ComposeMethod::Replace.is_implemented());
        assert!(!ComposeMethod::Asis.is_implemented());
        assert!(!ComposeMethod::Base.is_implemented());
        assert!(!ComposeMethod::Reduce.is_implemented());
        assert!(!ComposeMethod::Blend(BlendMode::new(BlendKind::Multiply)).is_implemented());
        assert!(!ComposeMethod::Blend(BlendMode::fast(BlendKind::Screen)).is_implemented());
        assert!(!ComposeMethod::Auto.is_implemented());
        assert!(!ComposeMethod::Unknown("future".into()).is_implemented());
    }

    /// 写像表準拠: `add`/`bind` は overlay 同義・大小文字無視。
    #[test]
    fn from_name_maps_overlay_synonyms() {
        assert_eq!(ComposeMethod::from_name("overlay"), ComposeMethod::Overlay);
        assert_eq!(ComposeMethod::from_name("add"), ComposeMethod::Overlay);
        assert_eq!(ComposeMethod::from_name("bind"), ComposeMethod::Overlay);
        assert_eq!(ComposeMethod::from_name("OVERLAY"), ComposeMethod::Overlay);
        assert_eq!(ComposeMethod::from_name("  Add  "), ComposeMethod::Overlay);
    }

    /// 写像表準拠: ハイフン／連結の両書式・単純メソッド群。
    #[test]
    fn from_name_maps_simple_methods() {
        assert_eq!(ComposeMethod::from_name("overlayfast"), ComposeMethod::OverlayFast);
        assert_eq!(ComposeMethod::from_name("overlay-fast"), ComposeMethod::OverlayFast);
        assert_eq!(ComposeMethod::from_name("interpolate"), ComposeMethod::Interpolate);
        assert_eq!(ComposeMethod::from_name("replace"), ComposeMethod::Replace);
        assert_eq!(ComposeMethod::from_name("asis"), ComposeMethod::Asis);
        assert_eq!(ComposeMethod::from_name("base"), ComposeMethod::Base);
        assert_eq!(ComposeMethod::from_name("reduce"), ComposeMethod::Reduce);
        assert_eq!(ComposeMethod::from_name("auto"), ComposeMethod::Auto);
    }

    /// 写像表準拠: blend-* 群（新書式＋fast＋旧書式別名）。
    #[test]
    fn from_name_maps_blend_family() {
        assert_eq!(
            ComposeMethod::from_name("blend-multiply"),
            ComposeMethod::Blend(BlendMode::new(BlendKind::Multiply))
        );
        assert_eq!(
            ComposeMethod::from_name("blend-multiply-fast"),
            ComposeMethod::Blend(BlendMode::fast(BlendKind::Multiply))
        );
        assert_eq!(
            ComposeMethod::from_name("blend-darker-color"),
            ComposeMethod::Blend(BlendMode::new(BlendKind::DarkerColor))
        );
        // 旧書式別名（写像表: overlaymultiply = blend-multiply-fast）。
        assert_eq!(
            ComposeMethod::from_name("overlaymultiply"),
            ComposeMethod::Blend(BlendMode::fast(BlendKind::Multiply))
        );
        assert_eq!(
            ComposeMethod::from_name("overlayscreen"),
            ComposeMethod::Blend(BlendMode::fast(BlendKind::Screen))
        );
    }

    /// 未知メソッドは原文保持の `Unknown` シームへ吸収する。
    #[test]
    fn from_name_absorbs_unknown() {
        assert_eq!(
            ComposeMethod::from_name("someFutureMethod"),
            ComposeMethod::Unknown("someFutureMethod".into())
        );
        // blend 接頭辞だが未知の種別も Unknown へ（誤った BlendMode 生成をしない）。
        assert_eq!(
            ComposeMethod::from_name("blend-nonsense"),
            ComposeMethod::Unknown("blend-nonsense".into())
        );
    }
}
