//! 描画面 1 枚の合成の計画（`wintf-visual-damage-redraw`）。この回に入れ物をどう描くかと、平行移動の写しを
//! 使ってよいかを決める。記録は target `wintf::damage` の debug の 1 行。

use super::visual::{DamageEntry, EntryKind, PxRect, diff};

/// 合成の記録の target（履歴の層の info 以上には入らない）。
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
    #[cfg_attr(
        test,
        expect(dead_code, reason = "平行移動の写しの判定（タスク 3.2）が S1 で読む")
    )]
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
    // 平行移動の写し（タスク 3.2）はここで写しと帯を決め、写しに取られた項を普通の範囲から除く
    let shifts: Vec<RegionShift> = Vec::new();
    let mut rects: Vec<PxRect> = Vec::new();

    // 普通の範囲は 1 つの外接矩形（決め D5）。項ごとに描画面で切ってから囲む（描画面の外の 2 つの
    // 変化の外接矩形が描画面をまたがないように）
    let mut damage = PxRect::default();
    let changed = d.removed.iter().map(|&i| &prev[i as usize]);
    let changed = changed.chain(d.added.iter().chain(&d.moved).map(|&j| &now[j as usize]));
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
        // 写しの帯の面積は平行移動の写し（タスク 3.2）が数える
        exposed_area: 0,
        rects,
        shifts,
        items_redrawn,
        present_rect,
    })
}
