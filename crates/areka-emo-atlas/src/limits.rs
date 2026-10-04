//! 動く絵の 3 つの上限・環境変数・判定（spec: areka-P0-animated-image-decode 要件 6.1・6.2・6.9〜6.11）。
//!
//! 上限は外から渡せる値（[`AnimationLimits`]）で持ち、設定ファイルが無いあいだは `AREKA_` の
//! 環境変数で変える。名前と欄の割り当て・読めない値の扱いは純粋な [`AnimationLimits::from_lookup`]
//! に置き、本番の入口 [`AnimationLimits::from_env`] は本物の環境で 1 回だけそれを呼ぶ。

use std::env::VarError;
use std::sync::OnceLock;

use crate::decode::AnimationInfo;
use crate::pack::PackConfig;

/// ㋐ 動く絵 1 つのコマの枚数の上限。
pub const MAX_FRAMES_ENV: &str = "AREKA_ANIMATED_IMAGE_MAX_FRAMES";
/// ㋑ 動く絵 1 つの全コマの画素の量（枚数 × 幅 × 高さ）の上限。
pub const MAX_PIXELS_ENV: &str = "AREKA_ANIMATED_IMAGE_MAX_PIXELS";
/// ㋒ `bake` 1 回の、動く絵の全コマの画素の量の合計の上限。
pub const MAX_TOTAL_PIXELS_ENV: &str = "AREKA_ANIMATED_IMAGE_MAX_TOTAL_PIXELS";

/// 動く絵の 3 つの上限（要件 6.1・6.9）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnimationLimits {
    /// ㋐ 既定 1,024 枚。
    pub max_frames: u64,
    /// ㋑ 既定 67,108,864 画素（1 画素 4 バイトで 256 MiB）。
    pub max_pixels: u64,
    /// ㋒ 既定 268,435,456 画素（同 1 GiB）。
    pub max_total_pixels: u64,
}

impl Default for AnimationLimits {
    fn default() -> Self {
        Self {
            max_frames: 1_024,
            max_pixels: 67_108_864,
            max_total_pixels: 268_435_456,
        }
    }
}

/// 読めなかった設定値 1 件（`warn!` に書く中身）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LimitWarning {
    /// 環境変数の名前。
    pub name: &'static str,
    /// 与えられた値（UTF-8 でないときは、置き換え文字つきの写し）。
    pub given: String,
}

impl AnimationLimits {
    /// 純粋な組み立て口。`lookup` は「名前 → 値」を `std::env::var` と同じ形で引く関数。
    /// 読めなかった項目は既定にして、`LimitWarning` を 1 件ずつ積む。環境変数にも `warn!` にも触らない。
    pub(crate) fn from_lookup(
        lookup: impl Fn(&'static str) -> Result<String, VarError>,
    ) -> (Self, Vec<LimitWarning>) {
        let default = Self::default();
        let mut warnings = Vec::new();
        let mut one = |name, default| {
            let (value, warning) = resolve_limit(name, lookup(name), default);
            warnings.extend(warning);
            value
        };
        let limits = Self {
            max_frames: one(MAX_FRAMES_ENV, default.max_frames),
            max_pixels: one(MAX_PIXELS_ENV, default.max_pixels),
            max_total_pixels: one(MAX_TOTAL_PIXELS_ENV, default.max_total_pixels),
        };
        (limits, warnings)
    }

    /// 本番の入口。プロセスで 1 回だけ本物の環境を読み、読めない値を `warn!` で出し、以後は同じ値。
    pub fn from_env() -> Self {
        static LIMITS: OnceLock<AnimationLimits> = OnceLock::new();
        *LIMITS.get_or_init(|| {
            let (limits, warnings) = Self::from_lookup(std::env::var);
            for w in warnings {
                tracing::warn!(
                    target: "areka_emo_atlas",
                    env = w.name,
                    value = %w.given,
                    "[emo-atlas] 動く絵の上限として読めない値なので、この項目は既定を使う（1 以上の整数）"
                );
            }
            limits
        })
    }
}

/// 1 項目を決める純粋な関数（要件 6.11）。未設定なら既定（記録なし）。
/// 1 以上の整数でなければ（数でない・0・負・UTF-8 でない）既定と `LimitWarning` を返す。
pub(crate) fn resolve_limit(
    name: &'static str,
    raw: Result<String, VarError>,
    default: u64,
) -> (u64, Option<LimitWarning>) {
    let given = match raw {
        Err(VarError::NotPresent) => return (default, None),
        Err(VarError::NotUnicode(os)) => os.to_string_lossy().into_owned(),
        Ok(s) => match s.trim().parse::<u64>() {
            Ok(n) if n > 0 => return (n, None),
            _ => s,
        },
    };
    (default, Some(LimitWarning { name, given }))
}

/// 超えた上限（ページの一辺は利用者が変える上限ではないが、同じに縮める）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Exceeded {
    Frames,
    Pixels,
    TotalPixels,
    PageSide,
}

/// 絵 1 つを判定する（要件 6.2）。収まれば、この絵が使う画素の量を返す。
/// `used` はこの `bake` で既に載せた動く絵の合計。順は 枚数 → 画素 → ページの一辺 → 合計で、
/// 掛け算・足し算のあふれは上限超えとして扱う。
pub(crate) fn judge(
    info: AnimationInfo,
    limits: AnimationLimits,
    used: u64,
    cfg: PackConfig,
) -> Result<u64, Exceeded> {
    let frames = u64::from(info.frame_count);
    if frames > limits.max_frames {
        return Err(Exceeded::Frames);
    }
    let pixels = frames
        .checked_mul(u64::from(info.width))
        .and_then(|p| p.checked_mul(u64::from(info.height)))
        .filter(|&p| p <= limits.max_pixels)
        .ok_or(Exceeded::Pixels)?;
    let pad = 2 * u64::from(cfg.padding);
    let page = u64::from(cfg.page_size);
    if u64::from(info.width) + pad > page || u64::from(info.height) + pad > page {
        return Err(Exceeded::PageSide);
    }
    match used.checked_add(pixels) {
        Some(total) if total <= limits.max_total_pixels => Ok(pixels),
        _ => Err(Exceeded::TotalPixels),
    }
}

#[cfg(test)]
#[path = "limits_tests.rs"]
mod tests;
