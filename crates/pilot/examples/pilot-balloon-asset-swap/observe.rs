//! 観測の規則: 見分け方（標本点・絵と当たり判定の判別）と数え方（1 フレームの判定）。
//!
//! 規則の唯一の定義は design.md §Observer。ここはそれを写しただけで、閾値を変えるなら先に design を直す。
//! World を読む部分（tick の記録・当たり判定の採取）は 2.2、観測の窓と集計ログは 2.4 が足す。
#![allow(dead_code)] // 2.2〜2.4 で使う

use areka_emo_compose::ComposedSurface;
use tracing::{debug, error};
use windows::Win32::Foundation::RECT;

/// 共通範囲を切る格子の升数（縦横とも）。
const GRID: u32 = 8;
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

// ---------------------------------------------------------------------------
// 型
// ---------------------------------------------------------------------------

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

/// tick の終わりの記録（当たり判定側・UI スレッドが書く）。
#[derive(Clone, Copy, Debug)]
pub struct TickRecord {
    pub tick: u32,
    pub ended_qpc: i64,
    pub rect: RECT,
    pub hit: HitClass,
    pub h_p: f32,
    pub h_q: f32,
    pub h_both: f32,
    pub covered: bool,
    pub k_ok: bool,
    pub slot_hit: bool,
}

/// 取り込んだ 1 フレームの記録（絵側・取り込みスレッドが書く）。
#[derive(Clone, Copy, Debug)]
pub struct FrameRecord {
    pub present_qpc: i64,
    pub accumulated: u32,
    pub tick: Option<u32>,
    pub picture: PictureClass,
    pub a: f32,
    pub b: f32,
    pub ab_p: f32,
    pub ab_q: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub frames: u32,
    pub missed: u32,
    pub unmeasurable: u32,
    pub mixed: u32,
    /// 混在の内訳: 反映待ち（揃う前の 絵＝from ∧ 当たり判定＝to）。
    pub pending: u32,
    pub empty: u32,
    pub stale: u32,
    pub size: u32,
}

/// 1 本の観測。窓の開け閉め（30／180 tick）と `complete` は 2.4 が足す。
#[derive(Debug)]
pub struct Observation {
    pub name: String,
    /// 出発と到達（`Class::P` か `Class::Q`）。
    pub from: Class,
    pub to: Class,
    /// P と Q の原寸（k=1.0 なので物理寸）。
    pub size_p: (u32, u32),
    pub size_q: (u32, u32),
    pub request_tick: u32,
    /// 揃った最初のフレーム（観測の中の通し番号, tick）。
    pub settled: Option<(u32, u32)>,
    pub counts: Counts,
}

impl Observation {
    pub fn new(name: &str, sig: &Signature, from: Class, to: Class, request_tick: u32) -> Self {
        Self {
            name: name.to_string(),
            from,
            to,
            size_p: sig.size_p,
            size_q: sig.size_q,
            request_tick,
            settled: None,
            counts: Counts::default(),
        }
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
    let a = ratio(
        sig.only_p
            .iter()
            .map(|&(x, y, c)| at(x, y).map(|px| near(px, c))),
    );
    let b = ratio(
        sig.only_q
            .iter()
            .map(|&(x, y, c)| at(x, y).map(|px| near(px, c))),
    );
    let ab_p = ratio(
        sig.both
            .iter()
            .map(|&(x, y, c, _)| at(x, y).map(|px| near(px, c))),
    );
    let ab_q = ratio(
        sig.both
            .iter()
            .map(|&(x, y, _, c)| at(x, y).map(|px| near(px, c))),
    );
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

fn rect_size(r: &RECT) -> (u32, u32) {
    (
        (r.right - r.left).max(0) as u32,
        (r.bottom - r.top).max(0) as u32,
    )
}

// ---------------------------------------------------------------------------
// 1 フレームの判定
// ---------------------------------------------------------------------------

/// 1 フレームを観測に数える（4 種は重複計上）。`tick` は `frame` と組になった tick の記録。
pub fn judge_frame(obs: &mut Observation, frame: &FrameRecord, tick: &TickRecord) {
    let idx = obs.counts.frames;
    obs.counts.frames += 1;
    obs.counts.missed += frame.accumulated.saturating_sub(1);

    let why = match (frame.picture, tick.hit) {
        (Class::Unmeasurable(w), _) | (_, Class::Unmeasurable(w)) => Some(w),
        _ if tick.covered => Some(Why::Covered),
        _ if !tick.k_ok => Some(Why::ScaleNotOne),
        _ if tick.slot_hit => Some(Why::SlotHit),
        _ if frame.accumulated > 1 => Some(Why::Dropped),
        _ => None,
    };
    if let Some(why) = why {
        obs.counts.unmeasurable += 1;
        debug!(name = %obs.name, idx, ?why, "測れないフレーム");
        return;
    }

    let (pic, hit) = (frame.picture, tick.hit);
    let (from, to) = (obs.from, obs.to);
    let moving = from != to;
    let before = obs.settled.is_none();

    if before {
        // 揃った: 要求より後のフレームで 絵＝to ∧ 当たり判定＝to（大きさは条件に入れない）。
        if tick.tick >= obs.request_tick && pic == to && hit == to {
            obs.settled = Some((idx, tick.tick));
        }
    } else if moving && (pic == from || pic == Class::Both) {
        obs.counts.stale += 1;
    }

    let dt = frame.present_qpc - tick.ended_qpc;
    if pic != hit || pic == Class::Both {
        obs.counts.mixed += 1;
        debug!(name = %obs.name, idx, ?pic, ?hit, dt, "混在");
    }
    if moving && before && pic == from && hit == to {
        obs.counts.pending += 1;
    }
    if moving && pic == to && hit == from {
        error!(name = %obs.name, idx, ?pic, ?hit, dt, "逆向きの組（絵＝到達先・当たり判定＝出発）— 突き合わせの前提が外れた");
    }
    if pic == Class::Neither {
        obs.counts.empty += 1;
    }
    let native = match pic {
        Class::P => Some(obs.size_p),
        Class::Q => Some(obs.size_q),
        _ => None,
    };
    if native.is_some_and(|s| s != rect_size(&tick.rect)) {
        obs.counts.size += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Class::*;

    const SP: (u32, u32) = (335, 205);
    const SQ: (u32, u32) = (400, 224);

    fn sig() -> Signature {
        Signature {
            only_p: vec![],
            only_q: vec![],
            both: vec![],
            size_p: SP,
            size_q: SQ,
        }
    }

    /// 合成した (絵, 当たり判定, 矩形) の 1 フレームを数える。
    fn feed(obs: &mut Observation, pic: Class, hit: Class, tick: u32, size: (u32, u32)) {
        feed_with(obs, pic, hit, tick, size, 1, false);
    }

    fn feed_with(
        obs: &mut Observation,
        pic: Class,
        hit: Class,
        tick: u32,
        (w, h): (u32, u32),
        accumulated: u32,
        covered: bool,
    ) {
        let t = TickRecord {
            tick,
            ended_qpc: tick as i64 * 100,
            rect: RECT {
                left: 10,
                top: 20,
                right: 10 + w as i32,
                bottom: 20 + h as i32,
            },
            hit,
            h_p: 0.0,
            h_q: 0.0,
            h_both: 0.0,
            covered,
            k_ok: true,
            slot_hit: false,
        };
        let f = FrameRecord {
            present_qpc: tick as i64 * 100 + 50,
            accumulated,
            tick: Some(tick),
            picture: pic,
            a: 0.0,
            b: 0.0,
            ab_p: 0.0,
            ab_q: 0.0,
        };
        judge_frame(obs, &f, &t);
    }

    #[test]
    fn judge_frame_counts_by_the_rules() {
        let mut o = Observation::new("t P→Q", &sig(), P, Q, 10);
        feed(&mut o, Q, Q, 9, SQ); // 0: 要求より前 → 揃ったにならない
        feed(&mut o, P, Q, 10, SP); // 1: 反映待ち（混在の内訳）
        feed(&mut o, Unmeasurable(Why::Ambiguous), Q, 10, SQ); // 2: 測れない
        feed(&mut o, Q, P, 10, SQ); // 3: 逆向き → 混在・反映待ちではない
        feed(&mut o, Neither, Neither, 10, SP); // 4: 空（混在ではない）
        feed(&mut o, Both, Q, 10, SQ); // 5: 混在・揃う前なので残りではない
        feed(&mut o, Q, Q, 11, SQ); // 6: 揃った
        feed(&mut o, P, Q, 12, SP); // 7: 混在・残り・揃った後なので反映待ちではない
        feed(&mut o, Both, Both, 13, SQ); // 8: 「両方」は混在・残り
        feed(&mut o, Q, Q, 14, SP); // 9: 大きさの食い違い
        feed_with(&mut o, P, P, 15, SP, 1, true); // 10: 覆い → 測れない
        feed_with(&mut o, P, Q, 16, SP, 3, false); // 11: 取りこぼし 2 → 測れない
        assert_eq!(o.settled, Some((6, 11)));
        assert_eq!(
            o.counts,
            Counts {
                frames: 12,
                missed: 2,
                unmeasurable: 3,
                mixed: 5,
                pending: 1,
                empty: 1,
                stale: 2,
                size: 1,
            }
        );

        // from == to（静止）では古い絵の残りを数えない。
        let mut s = Observation::new("t P→P", &sig(), P, P, 1);
        feed(&mut s, P, P, 1, SP);
        feed(&mut s, P, P, 2, SP);
        assert_eq!(s.settled, Some((0, 1)));
        assert_eq!(
            s.counts,
            Counts {
                frames: 2,
                ..Counts::default()
            }
        );
    }

    /// w×h の premultiplied BGRA（x が `xs` の範囲なら `color`・α=255、他は透明）。
    fn image(w: u32, h: u32, xs: std::ops::Range<u32>, color: Bgr) -> Vec<u8> {
        let mut v = vec![0u8; (w * h * 4) as usize];
        for y in 0..h {
            for x in xs.clone() {
                let i = ((y * w + x) * 4) as usize;
                v[i..i + 4].copy_from_slice(&[color[0], color[1], color[2], 255]);
            }
        }
        v
    }

    #[test]
    fn signature_and_picture_rule_on_tiny_images() {
        let (red, blue) = ([0, 0, 255], [255, 0, 0]);
        fn img(bytes: &[u8]) -> Img<'_> {
            Img {
                w: 64,
                h: 64,
                stride: 256,
                bytes,
            }
        }
        let (p, q) = (image(64, 64, 0..48, red), image(64, 64, 16..64, blue));
        // 升 8 列: 0–1 列は P だけ・2–5 列は両方・6–7 列は Q だけ。
        let sig = derive(&img(&p), &img(&q)).unwrap();
        assert_eq!(
            (sig.only_p.len(), sig.only_q.len(), sig.both.len()),
            (16, 16, 32)
        );

        let rect = RECT {
            left: 0,
            top: 0,
            right: 64,
            bottom: 64,
        };
        assert_eq!(classify_picture(&sig, &rect, &p, 256).0, P);
        assert_eq!(classify_picture(&sig, &rect, &q, 256).0, Q);
        assert_eq!(
            classify_picture(&sig, &rect, &vec![0u8; 64 * 64 * 4], 256).0,
            Neither
        );
        let mut over = q.clone(); // P を Q の上に描いた絵
        over[..].chunks_mut(4).zip(p.chunks(4)).for_each(|(o, s)| {
            if s[3] == 255 {
                o.copy_from_slice(s)
            }
        });
        assert_eq!(classify_picture(&sig, &rect, &over, 256).0, Both);

        // 同じ絵どうしは見分けられない。
        assert!(derive(&img(&p), &img(&p)).is_err());

        // 当たり判定の規則（「P だけ」が無い対では Both が出ない）。
        assert_eq!(hit_rule(true, 1.0, 1.0, 1.0), Both);
        assert_eq!(hit_rule(false, f32::NAN, 1.0, 1.0), Q);
        assert_eq!(hit_rule(false, f32::NAN, 0.0, 1.0), P);
    }
}
