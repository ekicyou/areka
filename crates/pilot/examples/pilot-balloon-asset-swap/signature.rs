//! 見分け方: 判別対の標本点の導出と、絵・当たり判定の判別の規則（`observe.rs` から分けた）。
//!
//! 規則の唯一の定義は design.md §Observer。ここはそれを写しただけで、閾値を変えるなら先に design を直す。

use areka_emo_compose::ComposedSurface;
use bevy_ecs::name::Name;
use bevy_ecs::prelude::*;
use tracing::trace;
use windows::Win32::Foundation::RECT;
use wintf::ecs::{PointF, hit_test_in_window};

/// 共通範囲を切る格子の升数（縦横とも・実装 2.2 で 8 から 16 へ改訂＝design §Observer）。
const GRID: u32 = 16;
/// 「不透明」の α の下限。
const OPAQUE: u8 = 128;
/// 「両方」の点の色差（各チャネルの差の最大）の下限。
const COLOR_DIFF: u8 = 48;
/// 取り込み画素と合成色の一致の許容（各チャネル）。
const TOLERANCE: u8 = 12;
/// 集合が使える点の数の下限。
const MIN_POINTS: usize = 16;
const HI: f32 = 0.9;
const MID: f32 = 0.5;
const LO: f32 = 0.1;

type Bgr = [u8; 3];

/// 絵・当たり判定の判別結果（design の `PictureClass`／`HitClass` は同じ値の集合なので 1 つにした）。
///
/// 観測の `from`／`to` も `P`／`Q` で持つので、`絵 == to` のような比較がそのまま書ける。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    P,
    Q,
    Both,
    Neither,
    Unmeasurable(Why),
}
pub type PictureClass = Class;
pub type HitClass = Class;

/// 測れない理由。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Why {
    Ambiguous,
    Covered,
    ScaleNotOne,
    SlotHit,
    Dropped,
    CaptureLost,
    OutsideOutput,
    /// 観測と違う判別対で判別したフレーム（対を切り替える前の tick の直前のフレーム）。
    OtherPair,
}

/// 判別対 (P, Q) の標本点。座標は窓（＝面）の左上からの物理 px。色は α=255 の点だけなので色そのもの。
#[derive(Debug)]
pub struct Signature {
    pub only_p: Vec<(u32, u32, Bgr)>,
    pub only_q: Vec<(u32, u32, Bgr)>,
    /// (x, y, P の色, Q の色)
    pub both: Vec<(u32, u32, Bgr, Bgr)>,
    pub size_p: (u32, u32),
    pub size_q: (u32, u32),
}

impl Signature {
    /// 「P だけ」を使うか（16 点未満なら無しとして扱う。点そのものはログ用に残す）。
    pub fn has_only_p(&self) -> bool {
        self.only_p.len() >= MIN_POINTS
    }
}

// ---------------------------------------------------------------------------
// 標本点の導出
// ---------------------------------------------------------------------------

/// premultiplied BGRA の画像の見え方（`ComposedSurface` は画素を書く口が crate 内限りなので、検査はこれで作る）。
struct Img<'a> {
    w: u32,
    h: u32,
    stride: usize,
    bytes: &'a [u8],
}

impl<'a> Img<'a> {
    fn of(s: &'a ComposedSurface) -> Self {
        Self {
            w: s.width(),
            h: s.height(),
            stride: s.stride() as usize,
            bytes: s.bytes(),
        }
    }

    fn px(&self, x: u32, y: u32) -> [u8; 4] {
        let i = y as usize * self.stride + x as usize * 4;
        [
            self.bytes[i],
            self.bytes[i + 1],
            self.bytes[i + 2],
            self.bytes[i + 3],
        ]
    }
}

fn bgr(px: [u8; 4]) -> Bgr {
    [px[0], px[1], px[2]]
}

fn color_diff(a: Bgr, b: Bgr) -> u8 {
    (0..3).map(|i| a[i].abs_diff(b[i])).max().unwrap_or(0)
}

/// 判別対 (P, Q) の標本点を導出する。見分けられない対は `Err`。
pub fn derive_signature(p: &ComposedSurface, q: &ComposedSurface) -> Result<Signature, String> {
    derive(&Img::of(p), &Img::of(q))
}

fn derive(p: &Img, q: &Img) -> Result<Signature, String> {
    let (cw, ch) = (p.w.min(q.w), p.h.min(q.h));
    let mut sig = Signature {
        only_p: Vec::new(),
        only_q: Vec::new(),
        both: Vec::new(),
        size_p: (p.w, p.h),
        size_q: (q.w, q.h),
    };
    for gy in 0..GRID {
        for gx in 0..GRID {
            let (mut op, mut oq, mut bo) = (None, None, None);
            for y in gy * ch / GRID..(gy + 1) * ch / GRID {
                for x in gx * cw / GRID..(gx + 1) * cw / GRID {
                    let (a, b) = (p.px(x, y), q.px(x, y));
                    if op.is_none() && a[3] == 255 && b[3] < OPAQUE {
                        op = Some((x, y, bgr(a)));
                    }
                    if oq.is_none() && b[3] == 255 && a[3] < OPAQUE {
                        oq = Some((x, y, bgr(b)));
                    }
                    if bo.is_none()
                        && a[3] == 255
                        && b[3] == 255
                        && color_diff(bgr(a), bgr(b)) >= COLOR_DIFF
                    {
                        bo = Some((x, y, bgr(a), bgr(b)));
                    }
                }
            }
            sig.only_p.extend(op);
            sig.only_q.extend(oq);
            sig.both.extend(bo);
        }
    }
    if sig.only_q.len() < MIN_POINTS || sig.both.len() < MIN_POINTS {
        return Err(format!(
            "見分けられない対（P だけ {}・Q だけ {}・両方 {}。Q だけと両方が各 {MIN_POINTS} 点以上要る）",
            sig.only_p.len(),
            sig.only_q.len(),
            sig.both.len()
        ));
    }
    Ok(sig)
}

// ---------------------------------------------------------------------------
// 絵と当たり判定の判別
// ---------------------------------------------------------------------------

/// 一致した割合。分母（範囲内の点）が 0 なら NaN（どの閾値も満たさず「測れない」へ落ちる）。
fn ratio(results: impl Iterator<Item = Option<bool>>) -> f32 {
    let (mut n, mut hit) = (0u32, 0u32);
    for r in results.flatten() {
        n += 1;
        hit += r as u32;
    }
    if n == 0 {
        f32::NAN
    } else {
        hit as f32 / n as f32
    }
}

fn near(px: Bgr, c: Bgr) -> bool {
    color_diff(px, c) <= TOLERANCE
}

/// 窓の矩形を切り出した画素（BGRA・`stride` バイト／行）から絵を判別する。→ (class, a, b, ab_p, ab_q)
///
/// 矩形の外に出た点は割合の分母から外す。
pub fn classify_picture(
    sig: &Signature,
    rect: &RECT,
    pixels: &[u8],
    stride: usize,
) -> (PictureClass, f32, f32, f32, f32) {
    let (w, h) = rect_size(rect);
    let at = |x: u32, y: u32| -> Option<Bgr> {
        if x >= w || y >= h {
            return None;
        }
        let i = y as usize * stride + x as usize * 4;
        pixels.get(i..i + 3).map(|s| [s[0], s[1], s[2]])
    };
    let check = |x: u32, y: u32, c: Bgr| {
        at(x, y).map(|px| {
            let m = near(px, c);
            trace!(x, y, ?px, ?c, m, "標本点の色");
            m
        })
    };
    let a = ratio(sig.only_p.iter().map(|&(x, y, c)| check(x, y, c)));
    let b = ratio(sig.only_q.iter().map(|&(x, y, c)| check(x, y, c)));
    let ab_p = ratio(sig.both.iter().map(|&(x, y, c, _)| check(x, y, c)));
    let ab_q = ratio(sig.both.iter().map(|&(x, y, _, c)| check(x, y, c)));
    (
        picture_rule(sig.has_only_p(), a, b, ab_p, ab_q),
        a,
        b,
        ab_p,
        ab_q,
    )
}

/// 割合 → 絵の判別（design §Observer 絵の判別）。
fn picture_rule(has_only_p: bool, a: f32, b: f32, ab_p: f32, ab_q: f32) -> PictureClass {
    let neither = b <= LO && ab_p <= LO && ab_q <= LO;
    if has_only_p {
        if a >= HI && ab_p >= HI && b <= LO {
            Class::P
        } else if b >= HI && ab_q >= HI && a <= LO {
            Class::Q
        } else if a >= MID && b >= MID {
            Class::Both
        } else if a <= LO && neither {
            Class::Neither
        } else {
            Class::Unmeasurable(Why::Ambiguous)
        }
    } else if ab_p >= HI && b <= LO {
        Class::P
    } else if b >= HI && ab_q >= HI {
        Class::Q
    } else if b >= MID && ab_p >= MID {
        Class::Both
    } else if neither {
        Class::Neither
    } else {
        Class::Unmeasurable(Why::Ambiguous)
    }
}

/// 当たりの割合 → 当たり判定の判別（design §Observer 当たり判定の判別）。
///
/// h_p＝「P だけ」、h_q＝「Q だけ」、h_both＝「両方」の各集合で当たった割合。
/// 「P だけ」が無い対では `Both` は Q と区別できないので出ない。
pub fn hit_rule(has_only_p: bool, h_p: f32, h_q: f32, h_both: f32) -> HitClass {
    if has_only_p {
        if h_p >= HI && h_q <= LO {
            Class::P
        } else if h_q >= HI && h_p <= LO {
            Class::Q
        } else if h_p >= HI && h_q >= HI {
            Class::Both
        } else if h_p <= LO && h_q <= LO {
            Class::Neither
        } else {
            Class::Unmeasurable(Why::Ambiguous)
        }
    } else if h_q <= LO && h_both >= HI {
        Class::P
    } else if h_q >= HI && h_both >= HI {
        Class::Q
    } else if h_q <= LO && h_both <= LO {
        Class::Neither
    } else {
        Class::Unmeasurable(Why::Ambiguous)
    }
}

/// 文字層の子の名前（`areka-emo-present` の mount が付ける）。
const TEXT_SLOT: &str = "emo-text-layer-slot";

/// 3 集合の各点で実際の当たり判定を引く。→ (class, h_p, h_q, h_both, 文字層の子が当たったか)
///
/// 点は面の左上からの物理 px。k=1.0 では窓のクライアント座標と 1:1（窓の原点＝面の原点）なので、
/// 画素の中心をそのまま `hit_test_in_window` に渡す。
pub fn classify_hit(
    world: &World,
    window: Entity,
    sig: &Signature,
) -> (HitClass, f32, f32, f32, bool) {
    let mut slot_hit = false;
    let mut probe = |x: u32, y: u32| {
        let e = hit_test_in_window(world, window, PointF::new(x as f32 + 0.5, y as f32 + 0.5));
        trace!(x, y, ?e, "標本点の当たり");
        slot_hit |= e
            .and_then(|e| world.get::<Name>(e))
            .is_some_and(|n| n.as_str() == TEXT_SLOT);
        Some(e.is_some())
    };
    let h_p = ratio(sig.only_p.iter().map(|&(x, y, _)| probe(x, y)));
    let h_q = ratio(sig.only_q.iter().map(|&(x, y, _)| probe(x, y)));
    let h_both = ratio(sig.both.iter().map(|&(x, y, _, _)| probe(x, y)));
    (
        hit_rule(sig.has_only_p(), h_p, h_q, h_both),
        h_p,
        h_q,
        h_both,
        slot_hit,
    )
}

pub fn rect_size(r: &RECT) -> (u32, u32) {
    (
        (r.right - r.left).max(0) as u32,
        (r.bottom - r.top).max(0) as u32,
    )
}

#[cfg(test)]
#[path = "signature_tests.rs"]
mod tests;
