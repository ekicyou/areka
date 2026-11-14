//! 描画面 1 枚の合成の計画（`wintf-visual-damage-redraw`）。この回に入れ物をどう描くかと、平行移動の写しを
//! 使ってよいかを決める。記録は target `wintf::damage` の debug の 1 行。

use super::visual::{DamageEntry, EntryKind, PxRect, diff};
use std::collections::{HashMap, VecDeque};
use std::ops::Range;

/// 合成の記録の target（履歴の層の info 以上には入らない）。
#[cfg_attr(not(test), expect(dead_code, reason = "5.3 の記録の行が使い始める"))]
pub(crate) const DAMAGE_TARGET: &str = "wintf::damage";

/// 全部の合成の理由（記録の `reason` の綴り・3.4）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FullReason {
    First,
    Recreated,
    RootList,
    Scale,
    NoPrev,
    Undecidable,
    CoversSurface,
}

impl FullReason {
    #[cfg_attr(not(test), expect(dead_code, reason = "5.3 の記録の行が使い始める"))]
    pub fn as_str(self) -> &'static str {
        match self {
            FullReason::First => "first",
            FullReason::Recreated => "recreated",
            FullReason::RootList => "root_list",
            FullReason::Scale => "scale",
            FullReason::NoPrev => "no_prev",
            FullReason::Undecidable => "undecidable",
            FullReason::CoversSurface => "covers_surface",
        }
    }
}

/// 入れ物の中の写し（`src` を左上 `dst` へ）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RegionShift {
    pub src: PxRect,
    pub dst: (u32, u32),
}

/// 範囲だけの合成の計画。
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PartialPlan {
    /// 描き直す矩形（描画面の中・空でない）。
    pub rects: Vec<PxRect>,
    pub shifts: Vec<RegionShift>,
    pub items_redrawn: u32,
    /// `rects` の面積の和（重なりは重ねて数える）。
    pub area: u64,
    /// 写した面積。
    pub shift_area: u64,
    /// 写しの切り抜きの中で描き直した面積。
    pub exposed_area: u64,
    /// 描画面へ写す矩形＝`rects` と `shifts` の先の外接矩形（2.6）。
    pub present_rect: PxRect,
}

/// 描画面 1 枚の、この回の合成の計画。
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ComposePlan {
    Skip,
    PresentOnly,
    Full(FullReason),
    Partial(PartialPlan),
}

/// 計画の入力。控えの値は入れ物が持ち、ここへは値だけが渡る。
#[derive(Clone, Copy, Debug)]
pub(crate) struct PlanInput<'a> {
    /// 入れ物を今作った回の理由（`First` か `Recreated`）。
    pub fresh: Option<FullReason>,
    /// 根の命令の列が前の合成と同じ参照（両方 `None` も同じ）。
    pub same_root: bool,
    /// 拡大が前の合成とビットで同じ。
    pub same_scale: bool,
    /// 根の命令の列を持つ（写しの条件 S1）。
    pub root_present: bool,
    pub prev: Option<&'a [DamageEntry]>,
    pub now: Option<&'a [DamageEntry]>,
    /// 前の合成の入れ物を描画面へ写し終えた。
    pub presented: bool,
    pub surface: (u32, u32),
}

/// 描画面 1 枚の、この回の合成の計画（判定の順は design の「描画面 1 枚の計画」の図）。
pub(crate) fn plan(input: &PlanInput<'_>) -> ComposePlan {
    if let Some(reason) = input.fresh {
        return ComposePlan::Full(reason);
    }
    if !input.same_root {
        return ComposePlan::Full(FullReason::RootList);
    }
    if !input.same_scale {
        return ComposePlan::Full(FullReason::Scale);
    }
    let Some(now) = input.now else {
        return ComposePlan::Full(FullReason::Undecidable);
    };
    let Some(prev) = input.prev else {
        return ComposePlan::Full(FullReason::NoPrev);
    };
    let surface = PxRect {
        left: 0,
        top: 0,
        right: input.surface.0.min(i32::MAX as u32) as i32,
        bottom: input.surface.1.min(i32::MAX as u32) as i32,
    };
    let d = diff(prev, now);
    let mut shifts: Vec<RegionShift> = Vec::new();
    let mut rects: Vec<PxRect> = Vec::new();
    let mut exposed_area = 0;
    // 写しに取られた部分木の項（普通の範囲に数えない・5.3）
    let mut taken_prev = vec![false; prev.len()];
    let mut taken_now = vec![false; now.len()];
    // S1: 根の命令の列を持つ描画面では写さない（下に何が描かれているか決められない）
    if !input.root_present {
        let mut take = |c: &Candidate| {
            let Some((dx, dy, dst)) = scroll_check(prev, now, c, surface) else {
                return false;
            };
            shifts.push(RegionShift {
                src: dst.offset(-dx, -dy),
                // `dst` は描画面の中なので負でない
                dst: (dst.left as u32, dst.top as u32),
            });
            for band in bands(c.r_out, dst).into_iter().filter(|b| !b.is_empty()) {
                exposed_area += band.area();
                rects.push(band);
            }
            taken_prev[c.prev.clone()].fill(true);
            taken_now[c.now.clone()].fill(true);
            true
        };
        // 描画面の全面の候補 E0（受け入れたら他の候補は見ない）
        let whole = Candidate {
            prev: 0..prev.len(),
            now: 0..now.len(),
            prev_chain: Vec::new(),
            now_chain: Vec::new(),
            r_out: surface,
            r_in: surface,
        };
        if !take(&whole) {
            // 動かなかった矩形の切り抜きを画家の順に。受け入れた候補の部分木の中は見ない
            let mut skip_end = 0;
            for &(i, j) in &d.kept {
                let (i, j) = (i as usize, j as usize);
                if j < skip_end || now[j].kind != EntryKind::ClipRect {
                    continue;
                }
                let c = Candidate::clip(prev, now, i, j, surface);
                if take(&c) {
                    skip_end = c.now.end;
                }
            }
        }
    }

    // 普通の範囲は 1 つの外接矩形（決め D5）。項ごとに描画面で切ってから囲む（描画面の外の 2 つの
    // 変化の外接矩形が描画面をまたがないように）
    let mut damage = PxRect::default();
    let changed = d.removed.iter().filter(|&&i| !taken_prev[i as usize]);
    let changed = changed.map(|&i| &prev[i as usize]);
    let now_changed = d.added.iter().chain(&d.moved);
    let now_changed = now_changed.filter(|&&j| !taken_now[j as usize]);
    let changed = changed.chain(now_changed.map(|&j| &now[j as usize]));
    for e in changed {
        damage = damage.union(e.bounds.intersect(surface));
    }
    if !damage.is_empty() {
        if damage == surface {
            return ComposePlan::Full(FullReason::CoversSurface);
        }
        rects.push(damage);
    }

    if rects.is_empty() && shifts.is_empty() {
        // 変わった項が全部描画面の外（または変わらない）なら画素は変わらない
        return if input.presented {
            ComposePlan::Skip
        } else {
            ComposePlan::PresentOnly
        };
    }

    // `emit_within` が描く件と同じ数え方
    let items_redrawn = now
        .iter()
        .filter(|e| matches!(e.kind, EntryKind::Draw { .. }))
        .filter(|e| rects.iter().any(|r| !r.intersect(e.bounds).is_empty()))
        .count() as u32;
    let present_rect = shifts.iter().fold(
        rects.iter().fold(PxRect::default(), |acc, r| acc.union(*r)),
        |acc, s| {
            let (w, h) = (s.src.right - s.src.left, s.src.bottom - s.src.top);
            let (x, y) = (s.dst.0 as i32, s.dst.1 as i32);
            acc.union(PxRect {
                left: x,
                top: y,
                right: x + w,
                bottom: y + h,
            })
        },
    );
    ComposePlan::Partial(PartialPlan {
        area: rects.iter().map(|r| r.area()).sum(),
        shift_area: shifts.iter().map(|s| s.src.area()).sum(),
        exposed_area,
        rects,
        shifts,
        items_redrawn,
        present_rect,
    })
}

/// 平行移動の写しの候補（矩形の切り抜き E か、描画面の全面 E0）。
struct Candidate {
    /// 前と今の部分木の項の番号（E 自身を含まない）。
    prev: Range<usize>,
    now: Range<usize>,
    /// E と、E を囲む層の項の番号（前と今）。
    prev_chain: Vec<usize>,
    now_chain: Vec<usize>,
    /// 鎖の矩形の切り抜きの `bounds` の交わりを描画面で切ったもの。
    r_out: PxRect,
    /// 鎖の矩形の切り抜きの `exact` を内向きに丸めた交わり（描画面では切らない）。
    r_in: PxRect,
}

impl Candidate {
    /// 前の `i` 番・今の `j` 番の、動かなかった矩形の切り抜き。
    fn clip(
        prev: &[DamageEntry],
        now: &[DamageEntry],
        i: usize,
        j: usize,
        surface: PxRect,
    ) -> Self {
        let now_chain = chain(now, j);
        let clips = || {
            now_chain
                .iter()
                .map(|&k| &now[k])
                .filter(|e| e.kind == EntryKind::ClipRect)
        };
        let r_out = clips().fold(surface, |acc, e| acc.intersect(e.bounds));
        let r_in = clips()
            .map(inward)
            .reduce(PxRect::intersect)
            .unwrap_or_default();
        Candidate {
            prev: subtree(prev, i),
            now: subtree(now, j),
            prev_chain: chain(prev, i),
            now_chain,
            r_out,
            r_in,
        }
    }
}

/// `k` 番の項と、それを囲む層の項の番号。
fn chain(list: &[DamageEntry], k: usize) -> Vec<usize> {
    let mut out = vec![k];
    let mut p = list[k].parent;
    while let Some(q) = p {
        out.push(q as usize);
        p = list[q as usize].parent;
    }
    out
}

/// `k` 番の項の部分木（自分を含まない）。
fn subtree(list: &[DamageEntry], k: usize) -> Range<usize> {
    k + 1..(list[k].end as usize).clamp(k + 1, list.len())
}

/// `exact` を内向きに丸めた矩形（縁の部分の覆いの画素を除く）。
fn inward(e: &DamageEntry) -> PxRect {
    PxRect {
        left: e.exact[0].ceil() as i32,
        top: e.exact[1].ceil() as i32,
        right: e.exact[2].floor() as i32,
        bottom: e.exact[3].floor() as i32,
    }
}

/// `out` から `inner`（`out` の中）を除いた上・下・左・右の帯（空のものも含む）。
fn bands(out: PxRect, inner: PxRect) -> [PxRect; 4] {
    [
        PxRect {
            bottom: inner.top,
            ..out
        },
        PxRect {
            top: inner.bottom,
            ..out
        },
        PxRect {
            top: inner.top,
            right: inner.left,
            bottom: inner.bottom,
            ..out
        },
        PxRect {
            left: inner.right,
            top: inner.top,
            bottom: inner.bottom,
            ..out
        },
    ]
}

/// 候補を入れ物の中の写しで済ませてよいか（条件 S2〜S9・S1 は呼び手）。よければ `(dx, dy)` と写しの先 `D`。
fn scroll_check(
    prev: &[DamageEntry],
    now: &[DamageEntry],
    c: &Candidate,
    surface: PxRect,
) -> Option<(i32, i32, PxRect)> {
    // S2: 親の鎖に図形の切り抜きが無い（E の id が親の鎖を含むので、前の鎖も同じ）
    if c.now_chain
        .iter()
        .any(|&k| now[k].kind == EntryKind::ClipGeometry)
    {
        return None;
    }
    let (sub_prev, sub_now) = (&prev[c.prev.clone()], &now[c.now.clone()]);
    // S6: `SOURCE_OVER` でない重ね方の件が無い
    let unbounded = EntryKind::Draw { unbounded: true };
    if sub_prev.iter().chain(sub_now).any(|e| e.kind == unbounded) {
        return None;
    }
    // S3: `shape` で k 番目どうしを対にし、対が在って前後の順が同じ
    let mut by_shape: HashMap<u64, VecDeque<usize>> = HashMap::new();
    for (i, e) in sub_prev.iter().enumerate() {
        by_shape.entry(e.shape).or_default().push_back(i);
    }
    let mut paired_prev = vec![false; sub_prev.len()];
    let mut pairs: Vec<(usize, usize)> = Vec::new();
    let mut lone_now: Vec<usize> = Vec::new();
    for (j, e) in sub_now.iter().enumerate() {
        match by_shape.get_mut(&e.shape).and_then(VecDeque::pop_front) {
            Some(i) => {
                paired_prev[i] = true;
                pairs.push((i, j));
            }
            None => lone_now.push(j),
        }
    }
    if pairs.is_empty() || pairs.windows(2).any(|w| w[0].0 > w[1].0) {
        return None;
    }
    // S4・S5: f32 の物理の行列の平行移動の差を f64 で取り（R5）、どの対も同じ整数の差で線形の部分が同じ
    let (p0, n0) = (&sub_prev[pairs[0].0], &sub_now[pairs[0].1]);
    let dx = n0.origin[0] as f64 - p0.origin[0] as f64;
    let dy = n0.origin[1] as f64 - p0.origin[1] as f64;
    let step = [dx, dy, dx, dy];
    for &(i, j) in &pairs {
        let (p, n) = (&sub_prev[i], &sub_now[j]);
        let same_origin = (0..2).all(|k| n.origin[k] as f64 - p.origin[k] as f64 == step[k]);
        let same_linear = (0..4).all(|k| n.linear[k].to_bits() == p.linear[k].to_bits());
        let same_exact = (0..4).all(|k| n.exact[k] as f64 - p.exact[k] as f64 == step[k]);
        if !(same_origin && same_linear && same_exact) {
            return None;
        }
    }
    let width = c.r_in.right as f64 - c.r_in.left as f64;
    let height = c.r_in.bottom as f64 - c.r_in.top as f64;
    if dx.fract() != 0.0
        || dy.fract() != 0.0
        || (dx == 0.0 && dy == 0.0)
        || dx.abs() >= width
        || dy.abs() >= height
    {
        return None;
    }
    // |dx| < 幅なので i32 に収まる
    let (dx, dy) = (dx as i32, dy as i32);
    // S7: 対の取れない前の項は切り抜きの外へ出て、今の項は露出した帯に入っただけ
    let hits = |b: PxRect| !b.intersect(c.r_out).is_empty();
    let mut lone_prev = sub_prev.iter().zip(&paired_prev).filter(|(_, p)| !**p);
    if lone_prev.any(|(e, _)| hits(e.bounds.offset(dx, dy)))
        || lone_now
            .iter()
            .any(|&j| hits(sub_now[j].bounds.offset(-dx, -dy)))
    {
        return None;
    }
    // S8: 部分木の外で親の鎖でもない項は、下にも上にも重ならない
    let outside = |list: &[DamageEntry], sub: &Range<usize>, chain: &[usize]| {
        (0..list.len())
            .filter(|k| !sub.contains(k) && !chain.contains(k))
            .any(|k| hits(list[k].bounds))
    };
    if outside(prev, &c.prev, &c.prev_chain) || outside(now, &c.now, &c.now_chain) {
        return None;
    }
    // S9: 写しの先が空でない（元も先も描画面の中）
    let r_in = c.r_in.intersect(surface);
    let dst = r_in.intersect(r_in.offset(dx, dy));
    (!dst.is_empty()).then_some((dx, dy, dst))
}
