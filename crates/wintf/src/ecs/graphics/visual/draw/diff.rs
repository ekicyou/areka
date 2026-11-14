//! 描く列の比べの項（`wintf-visual-damage-redraw`）。描く列と描画面の拡大から COM を持たない項の列を作り、
//! 前と今の列を比べる。項は描く列の Pop 以外の命令と 1 対 1（同じ順）。

use super::super::MixMode;
use super::command::*;
use super::emit::{DrawMethod, draw_method};
use crate::numerics::*;
use bevy_ecs::prelude::Entity;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use windows::Win32::Graphics::Direct2D::Common::D2D1_COMPOSITE_MODE_SOURCE_OVER;
use windows::Win32::Graphics::Direct2D::*;

/// 物理ピクセルの整数の矩形（右と下は含まない）。空は `right <= left` か `bottom <= top`。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct PxRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl PxRect {
    pub fn is_empty(self) -> bool {
        self.right <= self.left || self.bottom <= self.top
    }

    /// 交わり。交わらなければ `PxRect::default()`（空の形は 1 つにそろえる）。
    pub fn intersect(self, other: PxRect) -> PxRect {
        let r = PxRect {
            left: self.left.max(other.left),
            top: self.top.max(other.top),
            right: self.right.min(other.right),
            bottom: self.bottom.min(other.bottom),
        };
        if r.is_empty() { PxRect::default() } else { r }
    }

    /// 外接矩形。空の矩形は数えない（両方空なら `PxRect::default()`）。
    pub fn union(self, other: PxRect) -> PxRect {
        match (self.is_empty(), other.is_empty()) {
            (true, true) => PxRect::default(),
            (true, false) => other,
            (false, true) => self,
            (false, false) => PxRect {
                left: self.left.min(other.left),
                top: self.top.min(other.top),
                right: self.right.max(other.right),
                bottom: self.bottom.max(other.bottom),
            },
        }
    }

    pub fn area(self) -> u64 {
        if self.is_empty() {
            return 0;
        }
        (self.right as i64 - self.left as i64) as u64
            * (self.bottom as i64 - self.top as i64) as u64
    }

    /// 平行移動。i32 の端では飽和する（項の値は i32 に収まり、ずらしは描画面の大きさより小さい）。
    pub fn offset(self, dx: i32, dy: i32) -> PxRect {
        PxRect {
            left: self.left.saturating_add(dx),
            top: self.top.saturating_add(dy),
            right: self.right.saturating_add(dx),
            bottom: self.bottom.saturating_add(dy),
        }
    }
}

/// 項の種類。`Draw { unbounded }` は重ね方が `SOURCE_OVER` でない件（絵を切り抜きまで広げて合成する）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EntryKind {
    Draw { unbounded: bool },
    ClipRect,
    ClipGeometry,
    Opacity,
}

/// 比べの項。件 1 つ、または層を積む命令 1 つ。
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DamageEntry {
    /// 同一性の鍵（親の `id` を含む）。
    pub id: u64,
    /// 平行移動を除いた鍵（位置と大きさを含まない・親の `shape` を含む）。
    pub shape: u64,
    pub kind: EntryKind,
    /// 囲む層の項の番号。
    pub parent: Option<u32>,
    /// 自分の部分木の終わり（含まない）。件は自分の番号 + 1。`entries` が埋める（規則の関数は 0）。
    pub end: u32,
    /// 画素を変えうる範囲（物理・整数・外向き）。
    pub bounds: PxRect,
    /// left, top, right, bottom（物理・丸め前）。
    pub exact: [f32; 4],
    /// M11, M12, M21, M22（物理）。
    pub linear: [f32; 4],
    /// M31, M32（物理）。
    pub origin: [f32; 2],
}

/// 項を囲む層の鎖を外から内へ畳んだもの。根の直下は `Parent::default()`。
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Parent {
    /// 囲む層の項の番号。
    pub index: Option<u32>,
    /// 囲む層の項の `(id, shape)`。
    pub key: Option<(u64, u64)>,
    /// 鎖の矩形の切り抜きと図形の切り抜きの `bounds` の交わり。
    pub clip: Option<PxRect>,
    /// 鎖の矩形の切り抜きの `bounds` の交わり。
    pub rect_clip: Option<PxRect>,
}

impl Parent {
    /// 番号 `index` の層の項 `layer` の内側。不透明度の層の範囲では切らない。
    pub fn inner(&self, index: u32, layer: &DamageEntry) -> Parent {
        let cut = |c: Option<PxRect>| Some(c.map_or(layer.bounds, |c| c.intersect(layer.bounds)));
        let (clip, rect_clip) = match layer.kind {
            EntryKind::ClipRect => (cut(self.clip), cut(self.rect_clip)),
            EntryKind::ClipGeometry => (cut(self.clip), self.rect_clip),
            EntryKind::Opacity | EntryKind::Draw { .. } => (self.clip, self.rect_clip),
        };
        Parent {
            index: Some(index),
            key: Some((layer.id, layer.shape)),
            clip,
            rect_clip,
        }
    }
}

/// 層の種類と、種類ごとの材料。
#[derive(Clone, Copy, Debug)]
pub(crate) enum Layer {
    ClipRect,
    /// 図形の局所→世界（DIP）の変換。
    ClipGeometry(Matrix3x2),
    Opacity(f32),
}

const TAG_DRAW: u8 = 0;
const TAG_CLIP_RECT: u8 = 1;
const TAG_CLIP_GEOMETRY: u8 = 2;
const TAG_OPACITY: u8 = 3;

fn hasher(tag: u8) -> DefaultHasher {
    // `DefaultHasher::new()` は鍵が固定なので回をまたいで同じ値になる（1.6）。
    let mut h = DefaultHasher::new();
    tag.hash(&mut h);
    h
}

fn finish(mut h: DefaultHasher, parent: Option<u64>) -> u64 {
    parent.hash(&mut h);
    h.finish()
}

/// `i32` に収まる値だけ（有限でない値もここで落ちる）。
fn to_i32(v: f32) -> Option<i32> {
    (v >= i32::MIN as f32 && v < i32::MAX as f32).then_some(v as i32)
}

/// 外向きに丸めた整数の矩形。
fn outward(exact: [f32; 4]) -> Option<PxRect> {
    Some(PxRect {
        left: to_i32(exact[0].floor())?,
        top: to_i32(exact[1].floor())?,
        right: to_i32(exact[2].ceil())?,
        bottom: to_i32(exact[3].ceil())?,
    })
}

/// 世界の外接矩形と行列を拡大で物理へ写す（`emit` が D2D に渡すのと同じ f32 の掛け算）。
/// 返すのは `exact`・`linear`・`origin`。有限でない値が 1 つでもあれば `None`。
fn physical(
    world_aabb: Aabb,
    world_mat: Matrix3x2,
    scale: (f32, f32),
) -> Option<([f32; 4], [f32; 4], [f32; 2])> {
    let s = Matrix3x2::scale(scale.0, scale.1);
    let b = world_aabb.transform(s).0;
    let m = world_mat * s;
    let exact = [b.left, b.top, b.right, b.bottom];
    let linear = [m.M11, m.M12, m.M21, m.M22];
    let origin = [m.M31, m.M32];
    let finite = exact
        .iter()
        .chain(&linear)
        .chain(&origin)
        .all(|v| v.is_finite());
    finite.then_some((exact, linear, origin))
}

/// 重ね方が `SOURCE_OVER` でない `DrawImage` の件（D2D は絵を軸に沿う切り抜きまで広げて合成する）。
/// ブレンドと範囲の外の値は絵の外を変えないので含めない。
fn unbounded(mix: MixMode) -> bool {
    matches!(draw_method(mix, true), DrawMethod::Image(c) if c != D2D1_COMPOSITE_MODE_SOURCE_OVER)
}

/// 件が画素を変えうる範囲（物理・整数・外向き・2.1）。有限でない値・`i32` に収まらない値は `None`。
///
/// - `SOURCE_OVER` で描く件: `exact` を外向きに丸め、補間が最近傍でなければ各辺に `ceil(2 × s)`
///   （`s`＝`linear` の列の長さの大きい方・1 未満なら 1）を足し、親の鎖の切り抜きで切る。
/// - 重ね方が `SOURCE_OVER` でない件: 親の鎖の矩形の切り抜き。無ければ描画面の全面。
pub(crate) fn item_bounds(
    exact: [f32; 4],
    linear: [f32; 4],
    interpolation: D2D1_INTERPOLATION_MODE,
    mix: MixMode,
    parent: &Parent,
    surface: (u32, u32),
) -> Option<PxRect> {
    if !exact.iter().chain(&linear).all(|v| v.is_finite()) {
        return None;
    }
    if unbounded(mix) {
        let full = PxRect {
            left: 0,
            top: 0,
            right: i32::try_from(surface.0).ok()?,
            bottom: i32::try_from(surface.1).ok()?,
        };
        return Some(parent.rect_clip.unwrap_or(full));
    }
    let r = outward(exact)?;
    let margin = if interpolation == D2D1_INTERPOLATION_MODE_NEAREST_NEIGHBOR {
        0
    } else {
        let s = f32::hypot(linear[0], linear[2])
            .max(f32::hypot(linear[1], linear[3]))
            .max(1.0);
        to_i32((2.0 * s).ceil())?
    };
    let r = PxRect {
        left: r.left.checked_sub(margin)?,
        top: r.top.checked_sub(margin)?,
        right: r.right.checked_add(margin)?,
        bottom: r.bottom.checked_add(margin)?,
    };
    Some(parent.clip.map_or(r, |c| c.intersect(r)))
}

/// 件 1 つの項。`id` は `hash` と親の `id`、`shape` は実体・版・補間・混ぜ方と親の `shape`
/// （位置と大きさを含めない・5.3）。値が決められなければ `None`（3.3）。
#[allow(clippy::too_many_arguments)]
pub(crate) fn item_entry(
    hash: u64,
    entity: Entity,
    version: u64,
    world_mat: Matrix3x2,
    world_aabb: Aabb,
    interpolation: D2D1_INTERPOLATION_MODE,
    mix: MixMode,
    scale: (f32, f32),
    parent: &Parent,
    surface: (u32, u32),
) -> Option<DamageEntry> {
    let (exact, linear, origin) = physical(world_aabb, world_mat, scale)?;
    let bounds = item_bounds(exact, linear, interpolation, mix, parent, surface)?;
    let mut id = hasher(TAG_DRAW);
    hash.hash(&mut id);
    let mut shape = hasher(TAG_DRAW);
    entity.hash(&mut shape);
    version.hash(&mut shape);
    interpolation.0.hash(&mut shape);
    // 種類を先に混ぜる（重ね方とブレンドは同じ数値を持ち得る・`hash_draw` と同じ）。
    match mix {
        MixMode::Composite(c) => (0u8, c.0).hash(&mut shape),
        MixMode::Blend(b) => (1u8, b.0).hash(&mut shape),
    }
    Some(DamageEntry {
        id: finish(id, parent.key.map(|k| k.0)),
        shape: finish(shape, parent.key.map(|k| k.1)),
        kind: EntryKind::Draw {
            unbounded: unbounded(mix),
        },
        parent: parent.index,
        end: 0,
        bounds,
        exact,
        linear,
        origin,
    })
}

/// 層を積む命令 1 つの項。`bounds` は自分の矩形を外向きに丸めたもの（余白は足さない）。
/// `linear`・`origin` は図形の切り抜きが `world_mat × 拡大`、ほかは恒等と `exact` の左上。
pub(crate) fn layer_entry(
    layer: Layer,
    entity: Entity,
    version: u64,
    world_aabb: Aabb,
    scale: (f32, f32),
    parent: &Parent,
) -> Option<DamageEntry> {
    let (tag, kind) = match layer {
        Layer::ClipRect => (TAG_CLIP_RECT, EntryKind::ClipRect),
        Layer::ClipGeometry(_) => (TAG_CLIP_GEOMETRY, EntryKind::ClipGeometry),
        Layer::Opacity(_) => (TAG_OPACITY, EntryKind::Opacity),
    };
    let mat = match layer {
        Layer::ClipGeometry(m) => m,
        Layer::ClipRect | Layer::Opacity(_) => Matrix3x2::identity(),
    };
    let (exact, mut linear, mut origin) = physical(world_aabb, mat, scale)?;
    if !matches!(layer, Layer::ClipGeometry(_)) {
        linear = [1.0, 0.0, 0.0, 1.0];
        origin = [exact[0], exact[1]];
    }
    let bounds = outward(exact)?;
    let mut id = hasher(tag);
    let mut shape = hasher(tag);
    for h in [&mut id, &mut shape] {
        entity.hash(h);
        version.hash(h);
    }
    world_aabb.calc_hash(&mut id);
    match layer {
        Layer::ClipRect => {}
        Layer::ClipGeometry(m) => m.calc_hash(&mut id),
        Layer::Opacity(o) => {
            if !o.is_finite() {
                return None;
            }
            o.calc_hash(&mut id);
            o.calc_hash(&mut shape);
        }
    }
    Some(DamageEntry {
        id: finish(id, parent.key.map(|k| k.0)),
        shape: finish(shape, parent.key.map(|k| k.1)),
        kind,
        parent: parent.index,
        end: 0,
        bounds,
        exact,
        linear,
        origin,
    })
}

/// 描く列 `commands`（`build` の出した列）と描画面の拡大・大きさから比べの項の列を作る。
/// 項は Pop 以外の命令と 1 対 1（同じ順）。決められない値が 1 つでもあれば `None`（3.3）。
/// 同じ入力からはいつも同じ列（1.6）。COM を呼ばない。
pub(crate) fn entries(
    commands: &[DrawCommand],
    scale: (f32, f32),
    surface: (u32, u32),
) -> Option<Vec<DamageEntry>> {
    let mut out: Vec<DamageEntry> = Vec::with_capacity(commands.len());
    let mut stack: Vec<Parent> = Vec::new();
    for command in commands {
        let parent = stack.last().copied().unwrap_or_default();
        let index = u32::try_from(out.len()).ok()?;
        let layer =
            |l, entity, version, aabb| layer_entry(l, entity, version, aabb, scale, &parent);
        let entry = match command {
            DrawCommand::Draw(i) => {
                let mut e = item_entry(
                    i.hash,
                    i.entity,
                    i.version,
                    i.world_mat,
                    i.world_aabb,
                    i.interpolation_mode,
                    i.mix_mode,
                    scale,
                    &parent,
                    surface,
                )?;
                e.end = index + 1;
                out.push(e);
                continue;
            }
            DrawCommand::PushClipRect(c) => {
                layer(Layer::ClipRect, c.entity, c.version, c.world_aabb)?
            }
            DrawCommand::PushClipGeometry(c) => layer(
                Layer::ClipGeometry(c.world_mat),
                c.entity,
                c.version,
                c.world_aabb,
            )?,
            DrawCommand::PushOpacity(l) => {
                layer(Layer::Opacity(l.opacity), l.entity, l.version, l.world_aabb)?
            }
            DrawCommand::PopClipRect | DrawCommand::PopClipGeometry | DrawCommand::PopOpacity => {
                let open = stack.pop()?.index?;
                out[open as usize].end = index;
                continue;
            }
        };
        stack.push(parent.inner(index, &entry));
        out.push(entry);
    }
    // push と pop が対でない列は決められない
    stack.is_empty().then_some(out)
}
