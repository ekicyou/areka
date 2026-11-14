//! 描く列の比べの項（`wintf-visual-damage-redraw`）。描く列と描画面の拡大から COM を持たない項の列を作り、
//! 前と今の列を比べる。項は描く列の Pop 以外の命令と 1 対 1（同じ順）。

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
#[expect(
    dead_code,
    reason = "項の列の作り entries（タスク 2.1）が使い始めたら外す"
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EntryKind {
    Draw { unbounded: bool },
    ClipRect,
    ClipGeometry,
    Opacity,
}

/// 比べの項。件 1 つ、または層を積む命令 1 つ。
#[expect(
    dead_code,
    reason = "項の列の作り entries（タスク 2.1）が使い始めたら外す"
)]
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DamageEntry {
    /// 同一性の鍵（親の `id` を含む）。
    pub id: u64,
    /// 平行移動を除いた鍵（位置と大きさを含まない・親の `shape` を含む）。
    pub shape: u64,
    pub kind: EntryKind,
    /// 囲む層の項の番号。
    pub parent: Option<u32>,
    /// 自分の部分木の終わり（含まない）。件は自分の番号 + 1。
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
