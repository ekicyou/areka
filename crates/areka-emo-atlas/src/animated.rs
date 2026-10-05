//! 鍵 1 つの読み込みの段取りと、動く絵の 2 枚目以降のコマの扱い
//! （spec: areka-P0-animated-image-decode 要件 3.1〜3.5・4.1・4.7・5.1・6.2・6.4・6.9）。
//!
//! 見出しが無ければ今までの 1 枚読み（`decode` 1 回）をそのまま通す。見出しがあれば上限を
//! 判定してから全コマを読み、超えたか読めなければ 1 枚へ縮める（動きの 1 枚目 → 今までの
//! 1 枚読み → 今までの失敗の 3 段）。0 番のコマは呼び手（`bake_with_limits`）
//! が静止画と同じ道に通し、2 枚目以降のコマはここで透過・切り詰めをして、全部の鍵を回った後に
//! 鍵のエントリの後ろへ番号を振る。

use std::path::Path;

use crate::decode::{
    AnimatedImage, AnimationFrame, AnimationInfo, DecodeError, DecodedImage, ElementDecoder,
};
use crate::limits::{AnimationLimits, Exceeded, judge};
use crate::normalize::{NormalizedImage, clear_key_color};
use crate::pack::PackConfig;
use crate::table::{Animation, AtlasKey, ElementId, LoopCount, Size};
use crate::trim::{TrimResult, Trimmed, Trimmer};

/// 鍵 1 つの読み込みの結果。
pub(crate) enum Loaded {
    /// 1 枚の絵（静止画・1 枚へ縮めた動く絵）。今までの静止画の枝を通る。
    Still(DecodedImage),
    /// 全コマを読めた動く絵（コマは見出しどおりの枚数・寸法）と、その全コマの画素の数。
    /// 画素の数は、呼び手が 0 番のコマの正規化を通して表に載せると決めたときだけ合計に足す。
    Frames(AnimatedImage, u64),
    /// 1 枚も読めなかった（今までの静止画の失敗と同じ扱い）。
    Failed(DecodeError),
}

/// 鍵 1 つを読む。`used` はこの `bake` で既に全コマを載せた動く絵の画素の合計
/// （足すのは呼び手・要件 6.8）。
pub(crate) fn load(
    decoder: &impl ElementDecoder,
    path: &Path,
    key: &AtlasKey,
    limits: AnimationLimits,
    cfg: PackConfig,
    used: u64,
) -> Loaded {
    let Some(info) = decoder.probe_animation(path) else {
        return still(decoder, path);
    };
    let pixels = match judge(info, limits, used, cfg) {
        Ok(pixels) => pixels,
        Err(exceeded) => {
            tracing::warn!(
                target: "areka_emo_atlas",
                set = key.set.0,
                rel_path = key.rel_path.as_str(),
                frame_count = info.frame_count,
                width = info.width,
                height = info.height,
                exceeded = exceeded_name(exceeded),
                "bake: 動く絵が上限を超えたので 1 枚だけ読みます"
            );
            return shrink(decoder, path, key, info);
        }
    };
    let checked = decoder.decode_frames(path, info).and_then(|anim| {
        let fits = anim.frames.len() >= 2
            && anim.frames.len() == info.frame_count as usize
            && anim
                .frames
                .iter()
                .all(|f| f.image.width == info.width && f.image.height == info.height);
        if fits {
            Ok(anim)
        } else {
            Err(DecodeError::Decode {
                path: path.to_path_buf(),
                source: format!(
                    "frames do not match the header: header {} frames {}x{}, got {} frames ({})",
                    info.frame_count,
                    info.width,
                    info.height,
                    anim.frames.len(),
                    anim.frames
                        .iter()
                        .map(|f| format!("{}x{}", f.image.width, f.image.height))
                        .collect::<Vec<_>>()
                        .join(", "),
                ),
            })
        }
    });
    match checked {
        Ok(anim) => Loaded::Frames(anim, pixels),
        Err(reason) => {
            tracing::warn!(
                target: "areka_emo_atlas",
                set = key.set.0,
                rel_path = key.rel_path.as_str(),
                reason = %reason,
                "bake: 動く絵として読めなかったので 1 枚だけ読みます"
            );
            shrink(decoder, path, key, info)
        }
    }
}

/// 今までの 1 枚読み。
fn still(decoder: &impl ElementDecoder, path: &Path) -> Loaded {
    match decoder.decode(path) {
        Ok(img) => Loaded::Still(img),
        Err(e) => Loaded::Failed(e),
    }
}

/// 動く絵を 1 枚へ縮める（要件 6.2・6.4〜6.6）。段 1 は動きの 1 枚目、段 2 はそれが読めない
/// ときだけ今までの 1 枚読み、段 3 はそれも読めないときの今までの失敗。段 1・段 2 の絵は
/// 静止画の枝を通る（透明度が無ければ左上の色が抜ける）。
fn shrink(
    decoder: &impl ElementDecoder,
    path: &Path,
    key: &AtlasKey,
    info: AnimationInfo,
) -> Loaded {
    match decoder.decode_first_frame(path, info) {
        Ok(img) => Loaded::Still(img),
        Err(reason) => {
            tracing::warn!(
                target: "areka_emo_atlas",
                set = key.set.0,
                rel_path = key.rel_path.as_str(),
                reason = %reason,
                "bake: 動く絵の 1 枚目を読めなかったので、今までの読み方で 1 枚だけ読みます"
            );
            still(decoder, path)
        }
    }
}

fn exceeded_name(exceeded: Exceeded) -> &'static str {
    match exceeded {
        Exceeded::Frames => "frames",
        Exceeded::Pixels => "pixels",
        Exceeded::TotalPixels => "total_pixels",
        Exceeded::PageSide => "page_side",
    }
}

/// 2 枚目以降のコマを貯めたもの（親 1 つ分）。
pub(crate) struct PendingFrames {
    parent: ElementId,
    key: AtlasKey,
    /// 1 番のコマから順の切り詰めの結果。
    trims: Vec<TrimResult>,
    /// 0 番のコマから順の待ち時間（コマの枚数と同じ長さ）。
    delays_ms: Vec<u32>,
    loop_count: LoopCount,
}

impl PendingFrames {
    /// 0 番のコマを外した動く絵の残りを、0 番と同じ透過に通して切り詰める（要件 3.1〜3.3・3.5）。
    /// 0 番のコマは呼び手が静止画と同じ道で正規化を済ませているので、残りのコマは同じ `has_alpha`・
    /// 同じ設定で恒等か、0 番の左上の色（`key_color`）を消すだけになる。
    pub(crate) fn new(
        parent: ElementId,
        key: AtlasKey,
        first_delay_ms: u32,
        rest: Vec<AnimationFrame>,
        loop_count: LoopCount,
        key_color: Option<[u8; 4]>,
    ) -> Self {
        let mut delays_ms = vec![first_delay_ms];
        let trims = rest
            .into_iter()
            .map(|frame| {
                delays_ms.push(frame.delay_ms);
                let DecodedImage {
                    width,
                    height,
                    stride,
                    mut bgra,
                    ..
                } = frame.image;
                if let Some(color) = key_color {
                    clear_key_color(&mut bgra, width, height, stride, color);
                }
                Trimmer.trim(&NormalizedImage {
                    width,
                    height,
                    stride,
                    pbgra: bgra,
                })
            })
            .collect();
        Self {
            parent,
            key,
            trims,
            delays_ms,
            loop_count,
        }
    }

    /// 2 枚目以降のコマがどれも全透明か（全コマが透明かを決める材料）。
    pub(crate) fn all_transparent(&self) -> bool {
        self.trims.iter().all(|t| t.placement.is_none())
    }
}

/// 全部の鍵を回り終えた後、貯めたコマに鍵のエントリの後ろから番号を振る
/// （親の番号の昇順・コマの番号の昇順・要件 4.7）。
pub(crate) fn append_frames(
    pending: Vec<PendingFrames>,
    keys: &mut Vec<AtlasKey>,
    originals: &mut Vec<Size>,
    placed: &mut Vec<(ElementId, Trimmed)>,
) -> Vec<(ElementId, Animation)> {
    pending
        .into_iter()
        .map(|p| {
            let mut frames = vec![p.parent];
            for trim in p.trims {
                let id = ElementId(keys.len() as u32);
                keys.push(p.key.clone());
                originals.push(trim.original);
                if let Some(trimmed) = trim.placement {
                    placed.push((id, trimmed));
                }
                frames.push(id);
            }
            let animation = Animation {
                frames,
                delays_ms: p.delays_ms,
                loop_count: p.loop_count,
            };
            (p.parent, animation)
        })
        .collect()
}

#[cfg(test)]
#[path = "animated_tests.rs"]
mod tests;
