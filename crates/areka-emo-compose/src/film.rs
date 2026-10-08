//! 動く絵を「子」へ分解する（element を持たず `always` を 1 本だけ持つ部品・要件 1.1〜1.4・1.11・2.5・8.2）。
//!
//! [`crate::EmoWorld::bind_atlas`] の束縛の直後に 1 度だけ呼ぶ。束縛先が動く絵の親
//! （[`AtlasTable::animation`] が `Some`）である画像の element を見つけ、絵ごとに 1 度だけ検査し、
//! 通った絵は子の定義 [`FilmSheet`] を 1 つ作って、その絵を置いていた element の種類だけを
//! [`ElementKind::Film`] に替える（番号・X,Y・描画メソッドはそのまま・束縛は空）。落ちた絵は
//! 分解せず理由つきで [`FilmSheets`] の `skipped` に載せる（画像の element のまま＝ 1 枚目の静止画）。
//! 動く GIF・縮んだ絵は `animation` が `None` なので対象にならない（要件 1.11）。
//!
//! 記録は出さない（面の表はスコープの数だけ組まれる）。事実を値で返し、記録は seriko の表が出す。

use std::collections::BTreeMap;
use std::num::NonZeroU32;

use areka_emo_atlas::{Animation, AtlasTable, ElementId, LoopCount};
use bevy_ecs::entity::Entity;
use bevy_ecs::prelude::Resource;
use bevy_ecs::world::World;

use crate::nesting::{ElementKind, FilmId, rest_index};
use crate::normalized::SurfaceMaster;
use crate::world::AtlasBinding;

/// 動く絵から作った子の定義（element 0 個・`always` のアニメーション 1 本）。
///
/// pattern i はコマ i の絵を指し、出す前の待ちは pattern 0 が 0、pattern i（i ≥ 1）がコマ i−1 の
/// 待ち時間。周期は `delays_ms` の合計（最後のコマを出しておく時間を含む）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilmSheet {
    pub id: FilmId,
    /// 記録に出す相対パス。
    pub path: String,
    /// コマの絵の番号（`ElementId` の値）。2 枚以上。`frames[0]` は親。
    pub frames: Vec<u32>,
    /// コマごとの「出しておく時間」（ミリ秒・0 は 0 のまま）。合計は 1 以上。
    pub delays_ms: Vec<u32>,
    /// 合計の回数。`None` は終わりなし。
    pub laps: Option<NonZeroU32>,
    /// 全部のコマに共通の原寸（幅, 高さ）。
    pub original: (u32, u32),
    /// 経過 0 のコマの番号（1 枚目の待ち時間が 0 なら次のコマ）。
    pub rest: usize,
}

/// 分解しなかった絵 1 つ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilmSkip {
    /// 相対パス。
    pub path: String,
    pub reason: FilmSkipReason,
}

/// 分解しなかった理由。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilmSkipReason {
    /// 待ち時間の合計が 0。
    ZeroTotalDelay,
    /// コマの並びが約束と違う（0 番が親でない・コマと待ち時間の数が違う・原寸が揃わない）。
    BrokenFrames,
}

/// 面の表 1 つぶんの子の定義（動く絵も分解しなかった絵も無ければ面の表に載らない）。
#[derive(Debug, Default, Resource)]
pub struct FilmSheets {
    /// 番号の昇順。
    pub(crate) sheets: BTreeMap<FilmId, FilmSheet>,
    /// 相対パスの昇順。
    pub(crate) skipped: Vec<FilmSkip>,
}

/// 束縛の済んだ面の表で、動く絵の element を子を置く element へ替える（モジュール冒頭）。
pub(crate) fn decompose(world: &mut World, atlas: &AtlasTable) {
    // 先に (entity, element の添字, 親) を不変クエリで集めてから書き替える（借用の衝突を避ける）。
    let mut placed: Vec<(Entity, usize, ElementId, &Animation)> = Vec::new();
    let mut query = world.query::<(Entity, &SurfaceMaster, &AtlasBinding)>();
    for (entity, master, binding) in query.iter(world) {
        for (i, (element, bound)) in master.elements.iter().zip(&binding.0).enumerate() {
            if let (ElementKind::Image, Some(id)) = (element.kind, *bound)
                && let Some(anim) = atlas.animation(id)
            {
                placed.push((entity, i, id, anim));
            }
        }
    }
    if placed.is_empty() {
        return;
    }

    // 絵ごとに 1 度だけ検査する（番号の昇順・同じ絵は同じ答え）。
    let mut verdicts: BTreeMap<u32, Result<FilmSheet, FilmSkipReason>> = BTreeMap::new();
    for &(_, _, id, anim) in &placed {
        verdicts
            .entry(id.0)
            .or_insert_with(|| sheet_of(id, anim, atlas));
    }

    for (entity, i, id, _) in placed {
        if !matches!(verdicts.get(&id.0), Some(Ok(_))) {
            continue;
        }
        if let Some(mut master) = world.get_mut::<SurfaceMaster>(entity) {
            master.elements[i].kind = ElementKind::Film(FilmId(id.0));
        }
        if let Some(mut binding) = world.get_mut::<AtlasBinding>(entity) {
            binding.0[i] = None;
        }
    }

    let mut films = FilmSheets::default();
    for (parent, verdict) in verdicts {
        match verdict {
            Ok(sheet) => {
                films.sheets.insert(sheet.id, sheet);
            }
            Err(reason) => films.skipped.push(FilmSkip {
                path: atlas.key(ElementId(parent)).rel_path.clone(),
                reason,
            }),
        }
    }
    films.skipped.sort_by(|a, b| a.path.cmp(&b.path));
    world.insert_resource(films);
}

/// 親 `parent` の動く絵 `anim` を検査して子の定義を作る。
fn sheet_of(
    parent: ElementId,
    anim: &Animation,
    atlas: &AtlasTable,
) -> Result<FilmSheet, FilmSkipReason> {
    check(parent, anim, atlas)?;
    let original = atlas.entry(parent).original;
    let n = anim.frames.len();
    // 出す前の待ち: pattern 0 は 0、pattern i はコマ i−1 の待ち時間。先頭が 0 なので必ず Some。
    let waits = std::iter::once(0).chain(anim.delays_ms[..n - 1].iter().copied());
    Ok(FilmSheet {
        id: FilmId(parent.0),
        path: atlas.key(parent).rel_path.clone(),
        frames: anim.frames.iter().map(|f| f.0).collect(),
        delays_ms: anim.delays_ms.clone(),
        laps: match anim.loop_count {
            LoopCount::Infinite => None,
            LoopCount::Finite(n) => Some(n),
        },
        original: (original.w, original.h),
        rest: rest_index(waits).unwrap_or(0),
    })
}

/// 動く絵 1 つの検査: ①0 番が親自身 ②コマ（2 枚以上）と待ち時間の数が同じ ③全部のコマの原寸が
/// 親と同じ（①〜③は `BrokenFrames`）④待ち時間の合計が 1 以上（`ZeroTotalDelay`）。
pub(crate) fn check(
    parent: ElementId,
    anim: &Animation,
    atlas: &AtlasTable,
) -> Result<(), FilmSkipReason> {
    let original = atlas.entry(parent).original;
    let in_table = |f: &ElementId| (f.0 as usize) < atlas.len();
    let broken = anim.frames.first() != Some(&parent)
        || anim.frames.len() < 2
        || anim.frames.len() != anim.delays_ms.len()
        || !anim
            .frames
            .iter()
            .all(|f| in_table(f) && atlas.entry(*f).original == original);
    if broken {
        return Err(FilmSkipReason::BrokenFrames);
    }
    if anim.delays_ms.iter().all(|&d| d == 0) {
        return Err(FilmSkipReason::ZeroTotalDelay);
    }
    Ok(())
}

#[cfg(test)]
#[path = "film_tests.rs"]
mod tests;
