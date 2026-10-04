//! 入れ子（element定義でサーフェスを部品として置く・areka 独自の語）の静的な事実。
//!
//! element定義のファイル名の欄が半角の数字だけのとき、その欄を画像のファイル名でなく
//! サーフェスの番号として読む（要件 1.1・1.2）。読み分けの実装は本モジュールの
//! [`element_kind`] 1 関数だけで、畳み込み（[`crate::fold`]）と焼く前の除外（下流の
//! `shell_target`）が同じ関数を呼ぶ。
//!
//! 記録は出さない。事実を値で返し、記録は fs を触る入口が読み込み 1 回につき 1 度だけ出す。

use std::collections::BTreeMap;

use areka_parsers::shell::ElementPath;

use crate::bind::BindSet;
use crate::method::is_implemented_name;
use crate::pattern::PatternState;
use crate::plan::is_bind_interval;
use crate::world::EmoWorld;

/// element定義が置くもの。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementKind {
    /// 画像（今までどおり）。
    Image,
    /// サーフェスの番号（欄が半角の数字だけで、u32 に収まる）。
    Surface(u32),
    /// 欄は半角の数字だけだが u32 に収まらない。画像としては読まない（要件 1.9）。
    SurfaceOutOfRange,
}

/// 欄が空でなく、全部が半角の数字（0〜9）なら番号として読む。`0100` は 100。
/// 符号つき・全角の数字・拡張子つき・空は画像（要件 1.1・1.3・1.9）。
pub fn element_kind(path: &ElementPath) -> ElementKind {
    let s = path.as_str();
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return ElementKind::Image;
    }
    // 数字だけが確かなので、失敗は桁あふれだけ（先頭の 0 は値に効かない）。
    match s.parse::<u32>() {
        Ok(id) => ElementKind::Surface(id),
        Err(_) => ElementKind::SurfaceOutOfRange,
    }
}

/// サーフェスごとの静的な参照（要件 5.11・5.13）。面の表から 1 度作る不変の値（Send・seriko が写しを持つ）。
///
/// `children`・`bind_targets`・`bind_ids` のどれかが空でない番号だけを載せる（載せる条件はこの 1 か所）。
/// 入れ子も着せ替えの種類の animation も無いシェルでは空。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NestTable {
    surfaces: BTreeMap<u32, SurfaceParts>,
}

/// 1 つのサーフェスが指す先（[`NestTable`] の 1 行）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SurfaceParts {
    /// element定義で置いた子（面の表に在る番号だけ・element定義の番号の昇順）。
    pub children: Vec<u32>,
    /// 着せ替えの animation の pattern0 が指すサーフェス（animation の番号, 番号）。
    /// index が 0・番号が 0 以上・描画メソッドが動くものだけ。animation の番号の昇順。
    pub bind_targets: Vec<(u32, u32)>,
    /// 着せ替えの種類（`bind`・`bind+random`）の animation の番号（昇順・重複なし）。
    pub bind_ids: Vec<u32>,
}

impl NestTable {
    /// 面の表から作る（[`EmoWorld::nest_table`] の中身）。
    pub(crate) fn from_world(world: &EmoWorld) -> NestTable {
        let mut surfaces = BTreeMap::new();
        for id in world.surface_ids() {
            let Some(master) = world.surface(id) else {
                continue;
            };
            // elements は layer 昇順・同 layer は登場順（SurfaceMaster の不変条件）。
            let children: Vec<u32> = master
                .elements
                .iter()
                .filter_map(|e| match e.kind {
                    ElementKind::Surface(c) if world.surface(c).is_some() => Some(c),
                    _ => None,
                })
                .collect();
            let mut bind_ids: Vec<u32> = Vec::new();
            let mut bind_targets: Vec<(u32, u32)> = Vec::new();
            for anim in master
                .animations
                .iter()
                .filter(|a| is_bind_interval(&a.interval))
            {
                bind_ids.push(anim.id);
                // flatten_surface と同じ辺: index==0・番号が 0 以上・描画メソッドが動く。
                let target = anim
                    .patterns
                    .iter()
                    .find(|p| p.index == 0)
                    .filter(|p| is_implemented_name(p.method.as_str()))
                    .and_then(|p| u32::try_from(p.surface_id).ok());
                if let Some(target) = target {
                    bind_targets.push((anim.id, target));
                }
            }
            // 同じ番号の animation は畳み込みで 1 つ（後勝ち）。並びだけを番号の昇順へ揃える
            // （bind_ids は visible_parts が二分探索で引く）。
            bind_ids.sort_unstable();
            bind_targets.sort_unstable();
            if !(children.is_empty() && bind_targets.is_empty() && bind_ids.is_empty()) {
                surfaces.insert(
                    id,
                    SurfaceParts {
                        children,
                        bind_targets,
                        bind_ids,
                    },
                );
            }
        }
        NestTable { surfaces }
    }

    /// 1 行も載っていないか。
    pub fn is_empty(&self) -> bool {
        self.surfaces.is_empty()
    }

    /// `surface_id` の行（載っていなければ `None`）。
    pub fn parts(&self, surface_id: u32) -> Option<&SurfaceParts> {
        self.surfaces.get(&surface_id)
    }

    /// 今の絵に出ている部品の番号を、昇順・重複なしで `out` へ入れる（`out` は先に空にする）。
    ///
    /// 一番上から ① `children` ② 有効な着せ替え（`binds` に在る）の `bind_targets`（同じ animation の
    /// 番号にコマが在ればコマが置き換えるので数えない）③ そのサーフェスのコマ（一番上は今までの欄・
    /// 部品は部品の欄）のうち描画メソッドが動くもので、着せ替えの種類なら `binds` に在るもの、の先を
    /// 部品に数えてその先へ進む（`flatten_surface` と同じ辺）。先祖へ戻る辺は進まない。
    ///
    /// 辺は「一番上か部品か」と番号だけで決まるので、たどれる番号の集合は「一番上から届く番号」と
    /// 同じになる。そこで `out` 自身を訪れた印に使い（二分探索で昇順に差し込む）、一番上と既に入った
    /// 番号へは進まない。先祖は必ずそのどちらかなので、先祖へ戻る辺も進まない。確保は `out` の伸びだけ。
    pub fn visible_parts(
        &self,
        top: u32,
        binds: &BindSet,
        pattern: &PatternState,
        out: &mut Vec<u32>,
    ) {
        out.clear();
        self.walk(top, top, binds, pattern, out);
    }

    /// `surface` から出る辺をたどる（`surface == top` なら一番上の欄、それ以外は部品の欄）。
    fn walk(
        &self,
        surface: u32,
        top: u32,
        binds: &BindSet,
        pattern: &PatternState,
        out: &mut Vec<u32>,
    ) {
        let is_top = surface == top;
        let parts = self.parts(surface);
        let frame_of = |id: u32| {
            if is_top {
                pattern.get(id)
            } else {
                pattern.part_get(surface, id)
            }
        };
        if let Some(p) = parts {
            // ① element定義の子。
            for &child in &p.children {
                self.enter(child, top, binds, pattern, out);
            }
            // ② 有効な着せ替えの pattern0（コマが在ればコマが置き換える＝③で数える）。
            for &(id, target) in &p.bind_targets {
                if binds.contains(id) && frame_of(id).is_none() {
                    self.enter(target, top, binds, pattern, out);
                }
            }
        }
        // ③ コマ。着せ替えの種類なら有効なものだけ。
        let is_bind = |id: u32| parts.is_some_and(|p| p.bind_ids.binary_search(&id).is_ok());
        let top_frames = pattern.iter().filter(|_| is_top);
        let part_frames = pattern.part(surface).filter(|_| !is_top);
        for (id, frame) in top_frames.chain(part_frames) {
            if frame.method.is_implemented() && (!is_bind(id) || binds.contains(id)) {
                self.enter(frame.surface_id, top, binds, pattern, out);
            }
        }
    }

    /// `target` を部品に数えて進む（一番上・既に数えた番号へは進まない）。
    fn enter(
        &self,
        target: u32,
        top: u32,
        binds: &BindSet,
        pattern: &PatternState,
        out: &mut Vec<u32>,
    ) {
        if target == top {
            return;
        }
        let Err(at) = out.binary_search(&target) else {
            return;
        };
        out.insert(at, target);
        self.walk(target, top, binds, pattern, out);
    }
}

#[cfg(test)]
#[path = "nesting_kind_tests.rs"]
mod kind_tests;

#[cfg(test)]
#[path = "nesting_visible_tests.rs"]
mod visible_tests;
