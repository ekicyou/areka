//! pattern定義の `base` の置き換え（段の静的な層を、描く順の後の `base` のコマが指す面に替える）。

use std::sync::OnceLock;

use areka_emo_atlas::AtlasTable;

use crate::bind::BindSet;
use crate::pattern::PatternState;
use crate::plan::{Level, PlanScratch, Step, push_bind_layer, surface_and_binding};
use crate::plan_mix::NestWord;
use crate::world::EmoWorld;

/// 層 (ii)（`s.steps[mid..]`）の `base` のうち、描く順の最も後で、指す面が面の表に在り今の経路に無いものの面。
fn winner(s: &PlanScratch, world: &EmoWorld, mid: usize) -> Option<u32> {
    s.steps[mid..].iter().rev().find_map(|step| match *step {
        Step::Enter {
            surface,
            word: NestWord::Base,
            ..
        } if world.surface(surface).is_some() && !s.on_path.contains(&surface) => Some(surface),
        _ => None,
    })
}

/// 段の層 (i)（`s.steps[start..mid]`）を、置き換える `base` の指す面へ段の位置で入る積み荷 1 つに替え、
/// 層 (ii) のほかの `base` を外す（負けたものも描かない）。置き換えた面を返す。置き換えないなら何も触らない。
pub(crate) fn replace_root(
    s: &mut PlanScratch,
    level: &Level<'_>,
    start: usize,
    mid: usize,
) -> Option<u32> {
    let surface = winner(s, level.world, mid)?;
    let mut kept = mid;
    for read in mid..s.steps.len() {
        if !matches!(
            s.steps[read],
            Step::Enter {
                word: NestWord::Base,
                ..
            }
        ) {
            s.steps.swap(kept, read);
            kept += 1;
        }
    }
    s.steps.truncate(kept);
    let base = Step::Enter {
        surface,
        x: level.offset_x,
        y: level.offset_y,
        word: NestWord::Base,
    };
    s.steps.splice(start..mid, std::iter::once(base));
    Some(surface)
}

/// 根の面 `surface_id` の層 (ii) だけを組み、[`replace_root`] と同じ判定で置き換える面を答える。
pub(crate) fn root_base(
    world: &EmoWorld,
    surface_id: u32,
    binds: &BindSet,
    pattern: &PatternState,
) -> Option<u32> {
    // 層 (ii) は絵を引かないので、段の欄を満たすだけの空の表で足りる。
    static NO_PICTURES: OnceLock<AtlasTable> = OnceLock::new();
    let (master, binding) = surface_and_binding(world, surface_id)?;
    let level = Level {
        world,
        atlas: NO_PICTURES.get_or_init(|| AtlasTable::new(Vec::new(), Vec::new(), Vec::new())),
        master,
        binding,
        surface_id,
        binds,
        pattern,
        sort: world.animation_sort(),
        offset_x: 0,
        offset_y: 0,
        is_top_level: true,
    };
    let mut s = PlanScratch::default();
    s.on_path.insert(surface_id);
    push_bind_layer(&mut s, &level);
    winner(&s, world, 0)
}

#[cfg(test)]
#[path = "plan_base_tests.rs"]
pub(crate) mod tests;
