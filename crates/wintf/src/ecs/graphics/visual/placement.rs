//! 節の下の持ち主・自動で上げた持ち主の置き場の部品 [`SurfacePlacement`] と、
//! それを WUC へ写す純粋な補助（設計の PlacementMath）。ECS の係に依らない。

use bevy_ecs::prelude::*;
use tracing::warn;
use windows_numerics::{Matrix3x2, Matrix4x4, Vector3};

use super::promotion::Phase;
use crate::ecs::graphics::ClipShape;
use crate::numerics::{Aabb, Matrix3x2Ext};

/// 祖先をたどる深さの上限（木が壊れていても止まる）。
const MAX_HOST_DEPTH: usize = 256;

/// 節の下の持ち主・自動で上げた持ち主の置き場（1 回の判断だけが書く・Composition の係が読む）。
/// 無い持ち主は、直の親の持ち主の直下（今の写し方＋自分の VisualTransform）。
#[derive(Component, Clone, Debug, PartialEq)]
pub struct SurfacePlacement(pub(crate) PlacementKind);

impl SurfacePlacement {
    /// 自動で上げた持ち主（相が用意中か上げた）の描画面の物理の大きさ。手動の持ち主は `None`。
    pub(crate) fn auto_size_px(&self) -> Option<(u32, u32)> {
        match self.0 {
            PlacementKind::Auto { size_px, .. } => Some(size_px),
            PlacementKind::Embedded { .. } => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum PlacementKind {
    /// 手動の持ち主。行列は親の節の世界の行列（親の持ち主の DIP）。不透明度は祖先の節の積。
    Embedded {
        parent_world: Matrix3x2,
        opacity: f32,
        visible: bool,
        clip: PlacementClip,
    },
    /// 自動で上げた持ち主。原点は親の持ち主の WUC の Visual からの整数の物理ピクセル、
    /// 大きさは描画面の物理の大きさ。
    Auto {
        origin_px: (i32, i32),
        size_px: (u32, u32),
    },
}

/// 祖先の節の切り抜き（親の持ち主の DIP）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum PlacementClip {
    /// 祖先に矩形の切り抜きが無い。
    None,
    /// 祖先の矩形の切り抜きの交わり。
    Rect(Aabb),
    /// 矩形で表せない（祖先の角丸・図形・軸に沿わない切り抜き）。
    Unsupported,
}

/// 持ち主の WUC の親からの変形。平行移動だけなら Offset、ほかは行列（Offset 0）。
#[derive(Clone, Copy, Debug, PartialEq)]
// shortcut: 変形の写しを係から呼ぶのは 4.2。繋いだら WucTransform・wuc_transform の許しを外す。
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) enum WucTransform {
    Offset(Vector3),
    Matrix(Matrix4x4),
}

/// 持ち主の WUC の親からの変形（DIP の行列 `to_parent_dip` と拡大 `scale` から、物理ピクセルで）。
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn wuc_transform(to_parent_dip: Matrix3x2, scale: f32) -> WucTransform {
    const EPS: f32 = 1e-6;
    let m = to_parent_dip;
    // 局所の内容も親の Visual も物理ピクセルなので、一様な拡大では線形の部分はそのまま・平行移動だけ拡大する
    let (tx, ty) = (m.M31 * scale, m.M32 * scale);
    let translation_only = (m.M11 - 1.0).abs() < EPS
        && (m.M22 - 1.0).abs() < EPS
        && m.M12.abs() < EPS
        && m.M21.abs() < EPS;
    if translation_only {
        return WucTransform::Offset(Vector3 {
            X: tx,
            Y: ty,
            Z: 0.0,
        });
    }
    WucTransform::Matrix(Matrix4x4 {
        M11: m.M11,
        M12: m.M12,
        M13: 0.0,
        M14: 0.0,
        M21: m.M21,
        M22: m.M22,
        M23: 0.0,
        M24: 0.0,
        M31: 0.0,
        M32: 0.0,
        M33: 1.0,
        M34: 0.0,
        M41: tx,
        M42: ty,
        M43: 0.0,
        M44: 1.0,
    })
}

/// 自分の切り抜きと置き場の切り抜きを合わせた 1 つの WUC の切り抜き。
#[derive(Clone, Copy, Debug, PartialEq)]
// shortcut: 切り抜きの合わせを係から呼ぶのは 4.3。繋いだら ClipPlan・merged_clip・invert の許しを外す。
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) enum ClipPlan {
    /// 今の写し方（置き場の切り抜きが無いか、置き場の切り抜きが自分の矩形・角丸の矩形の切り抜きを全部含む）。
    Own,
    /// 1 つの軸に沿う矩形（局所の DIP）。
    Rect { local: Aabb },
    /// 親の DIP の矩形を逆行列（親の DIP → 局所の DIP）で局所へ。
    Transformed { host_rect: Aabb, inverse: Matrix3x2 },
    /// 合わせられない（自分の切り抜きだけを写し、warn! は 1 回の判断が残す）。
    Unsupported,
}

/// 自分の切り抜き（局所）と置き場の切り抜き（親の持ち主の DIP）を 1 つの WUC の切り抜きへ合わせる。
/// `to_host_dip` は局所の DIP → 親の持ち主の DIP、`size` は持ち主の局所の大きさ（DIP）。
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn merged_clip(
    own: Option<&ClipShape>,
    placement: &PlacementClip,
    to_host_dip: Matrix3x2,
    size: (f32, f32),
) -> ClipPlan {
    let host_rect = match placement {
        PlacementClip::None => return ClipPlan::Own,
        PlacementClip::Unsupported => return ClipPlan::Unsupported,
        PlacementClip::Rect(r) => *r,
    };
    let Some(inverse) = invert(to_host_dip) else {
        // 潰れた行列では持ち主も WUC の子も潰れて見えず、切り抜くものが無い
        return ClipPlan::Own;
    };
    let bounds = Aabb::new(0.0, 0.0, size.0, size.1);
    // WUC の切り抜きは子の持ち主にも効くので、省けるのは持ち主の範囲に収まる自分の切り抜きだけ
    // （切り抜きが無い・図形のときは、持ち主の外へ出る子の持ち主を置き場の切り抜きで切る）
    let own_inside_bounds = matches!(
        own,
        Some(
            ClipShape::Rectangle
                | ClipShape::RoundedRectangle { .. }
                | ClipShape::RoundedRectangleIndividual { .. }
        )
    );
    let b = bounds.transform(to_host_dip);
    if own_inside_bounds
        && host_rect.left() <= b.left()
        && host_rect.top() <= b.top()
        && host_rect.right() >= b.right()
        && host_rect.bottom() >= b.bottom()
    {
        return ClipPlan::Own;
    }
    if !to_host_dip.axis_aligned() {
        // WUC の切り抜きは 1 つなので、自分の切り抜きとは合わせられない
        return match own {
            None => ClipPlan::Transformed { host_rect, inverse },
            Some(_) => ClipPlan::Unsupported,
        };
    }
    let local = host_rect.transform(inverse);
    match own {
        None => ClipPlan::Rect { local },
        Some(ClipShape::Rectangle) => ClipPlan::Rect {
            local: local.intersect(bounds),
        },
        Some(_) => ClipPlan::Unsupported,
    }
}

/// 行が点の 3x2 行列の逆。潰れた行列は無し。
#[cfg_attr(not(test), allow(dead_code))]
fn invert(m: Matrix3x2) -> Option<Matrix3x2> {
    let det = m.M11 * m.M22 - m.M12 * m.M21;
    if !det.is_finite() || det.abs() < 1e-12 {
        return None;
    }
    Some(Matrix3x2 {
        M11: m.M22 / det,
        M12: -m.M12 / det,
        M21: -m.M21 / det,
        M22: m.M11 / det,
        M31: (m.M21 * m.M32 - m.M22 * m.M31) / det,
        M32: (m.M12 * m.M31 - m.M11 * m.M32) / det,
    })
}

/// 子の WUC の親になる持ち主。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HostAnswer {
    Host(Entity),
    /// 親に繋がない（今の「根の印」・用意中の自動の持ち主・窓の外）。
    Detached,
}

/// 子の種類（WUC の木での扱い）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ChildClass {
    /// 手動の持ち主（`VisualGraphics` を持ち節の種類でない）。
    Owner,
    /// 相 Active で `VisualGraphics` を持つ節。
    AttachedAuto,
    /// それ以外の `VisualGraphics` を持つ節（Preparing・戻した直後）。
    DetachedAuto,
    /// `VisualGraphics` を持たない節。
    Node,
    /// `Visual` を持たないか窓の外。
    Other,
}

/// 子の種類を部品の有無と相から決める。`manual_mark` は目印か命令の列を持つか。
/// 手動の持ち主は規則 `wants_own_surface` の相に依らない側（節でないか、目印か命令の列を持つ）。
pub(crate) fn child_class(
    has_graphics: bool,
    is_node: bool,
    manual_mark: bool,
    phase: Phase,
) -> ChildClass {
    match (has_graphics, is_node) {
        (true, false) => ChildClass::Owner,
        (true, true) if manual_mark => ChildClass::Owner,
        (true, true) if phase == Phase::Active => ChildClass::AttachedAuto,
        (true, true) => ChildClass::DetachedAuto,
        (false, true) => ChildClass::Node,
        (false, false) => ChildClass::Other,
    }
}

/// 子の WUC の親になる持ち主（節と用意中の自動の持ち主を通り抜けて祖先をたどる）。
pub(crate) fn wuc_host_of(
    e: Entity,
    parent_of: impl Fn(Entity) -> Option<Entity>,
    class_of: impl Fn(Entity) -> ChildClass,
) -> HostAnswer {
    if class_of(e) == ChildClass::DetachedAuto {
        return HostAnswer::Detached;
    }
    let mut cur = e;
    for _ in 0..MAX_HOST_DEPTH {
        let Some(p) = parent_of(cur) else {
            return HostAnswer::Detached;
        };
        match class_of(p) {
            // AttachedAuto の下に手動の持ち主は居ない（見分けの 2）が、居れば自分の Visual を持つ親として繋ぐ
            ChildClass::Owner | ChildClass::AttachedAuto => return HostAnswer::Host(p),
            ChildClass::Node | ChildClass::DetachedAuto => cur = p,
            ChildClass::Other => return HostAnswer::Detached,
        }
    }
    warn!(
        entity = ?e,
        depth = MAX_HOST_DEPTH,
        "[placement] 祖先の持ち主をたどる深さの上限に着いたので、親に繋がない"
    );
    HostAnswer::Detached
}

/// 親の持ち主の WUC の子の並び。この順に InsertAtBottom する（最初が最上）。
pub(crate) fn wuc_children_order(
    host: Entity,
    children_of: impl Fn(Entity) -> Vec<Entity>,
    class_of: impl Fn(Entity) -> ChildClass,
) -> Vec<Entity> {
    let mut owners = Vec::new();
    let mut autos = Vec::new();
    // 前順: 子を逆に積み、最初の子から取り出す
    let mut stack: Vec<Entity> = children_of(host).into_iter().rev().collect();
    while let Some(c) = stack.pop() {
        match class_of(c) {
            ChildClass::Owner => owners.push(c),
            ChildClass::AttachedAuto => autos.push(c),
            ChildClass::Node | ChildClass::DetachedAuto => {
                stack.extend(children_of(c).into_iter().rev());
            }
            ChildClass::Other => {}
        }
    }
    // 手動の持ち主を前順で先に（最初が最上）、自動は前順の逆で後に（前順で後が上）
    owners.extend(autos.into_iter().rev());
    owners
}
