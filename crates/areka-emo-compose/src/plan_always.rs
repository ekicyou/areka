//! 合成で、動く絵の子と `always` の経過 0 を描く（要件 1.4・3.1・4.1・4.5・4.6・7.1）。
//!
//! - 動く絵の子（[`ElementKind::Film`](crate::nesting::ElementKind::Film)）は element を持たず、今の
//!   コマの絵 1 枚だけを、置いた element の位置・描画メソッドで命令にする（下に何も敷かない）。
//! - `always` の animation は、欄が「載っていない」なら経過 0 の pattern を描き、「消えている」なら
//!   何も描かない（[`always_rest_target`]）。経過 0 の求め方は [`rest_pattern`] の 1 本だけ。

use areka_emo_atlas::{AtlasTable, ElementId};
use areka_parsers::shell::{Animation, Pattern};

use crate::method::{ComposeMethod, is_implemented_name};
use crate::nesting::{FilmId, PartKey, rest_pattern};
use crate::normalized::Transform;
use crate::pattern::{Cell, PatternState};
use crate::plan::BlitOp;
use crate::world::EmoWorld;

/// 動く絵の子 `film` の今のコマを 1 枚の命令にして積む。欄が「載っていない」なら経過 0 のコマ、
/// 絵の番号が子のコマに無ければ経過 0 のコマ（`debug!`）、「消えている」なら何も。全透明のコマは
/// 命令にしない。子の定義が無ければ（起きないはず）描かずに `error!`。
pub(crate) fn push_film_op(
    out_ops: &mut Vec<BlitOp>,
    world: &EmoWorld,
    atlas: &AtlasTable,
    film: FilmId,
    pattern: &PatternState,
    method: &ComposeMethod,
    transform: Transform,
) {
    let Some(sheet) = world.film_sheet(film) else {
        tracing::error!(
            target: "areka_emo_compose",
            film = film.0,
            "動く絵の子の定義が無い: 描かない"
        );
        return;
    };
    let rest = sheet.frames[sheet.rest];
    let picture = match pattern.cell(Some(PartKey::Film(film)), 0) {
        Cell::Picture(p) if sheet.frames.contains(&p) => p,
        Cell::Picture(p) => {
            tracing::debug!(
                target: "areka_emo_compose",
                film = film.0,
                picture = p,
                "欄の絵の番号が子のコマに無い: 経過 0 のコマを描く"
            );
            rest
        }
        Cell::Blank => return,
        Cell::Rest | Cell::Frame(_) => rest,
    };
    let element = ElementId(picture);
    if atlas.entry(element).placement.is_none() {
        // 全透明のコマ（通常系）。
        return;
    }
    out_ops.push(BlitOp {
        element,
        transform,
        method: method.clone(),
    });
}

/// `always` の animation `anim` の欄が `cell` のとき、経過 0 として描く pattern。「載っていない」で、
/// 経過 0 の pattern が在り、番号が u32 に収まり・描画メソッドが動くときだけ `Some`（見える部品の辺と同じ）。
pub(crate) fn always_rest_target<'a>(
    anim: &'a Animation,
    cell: Cell<'_>,
    surface_id: u32,
) -> Option<&'a Pattern> {
    if cell != Cell::Rest {
        return None;
    }
    let p = rest_pattern(anim)?;
    // 負の番号（何も描かない）と u32 に収まらない番号は、見える部品の辺と同じく先が無い。
    if u32::try_from(p.surface_id).is_err() {
        return None;
    }
    if !is_implemented_name(p.method.as_str()) {
        tracing::debug!(
            target: "areka_emo_compose",
            surface_id,
            animation_id = anim.id,
            method = p.method.as_str(),
            "always の経過 0 の pattern の描画メソッドが動かない: 描かない"
        );
        return None;
    }
    Some(p)
}

#[cfg(test)]
#[path = "plan_always_tests.rs"]
mod tests;
