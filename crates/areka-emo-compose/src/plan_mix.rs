//! 命令の列と入れ子の印から計画の項を作る（最初の項の読み替え・`replace` の範囲・`asis` の別の絵・まとめ）。
//!
//! 層は面の根か、まとめの中。層の描くものが在る最初の項は透明の上に置くので `SourceOver` にする
//! （`asis` は `asis` 用の絵のまま）。入れ子の面は、中に `SourceOver` でない項が在るか、指す語が
//! `SourceOver` でないときだけまとめにし、ほかは `SourceOver` の結合則で答えが同じなので平らに並べる。
//! まとめの中の位置と範囲はまとめの左上から。

use std::collections::HashMap;

use areka_emo_atlas::{AtlasTable, ElementId};

use crate::atlas_bind::AsisPictures;
use crate::draw_plan::{IRect, Mix, PlanItem, mix_of};
use crate::method::ComposeMethod;
use crate::normalized::Transform;
use crate::plan::BlitOp;
use crate::world::EmoWorld;

/// 入れ子の面を指す語の重ね方。`asis` は黒の塗りか `asis` 用の絵で描くので分けて持つ。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NestWord {
    Mix(Mix),
    Asis,
    /// 土台を置き換える pattern定義の `base`（置き換えた先は `SourceOver` と同じに描く）。
    Base,
}

impl NestWord {
    pub(crate) fn of(method: &ComposeMethod) -> NestWord {
        match method {
            ComposeMethod::Asis => NestWord::Asis,
            ComposeMethod::Base => NestWord::Base,
            // 描けない語の element定義の子の面は、今の CPU の合成と同じく平らに重ねる。
            other => NestWord::Mix(mix_of(other).unwrap_or(Mix::SourceOver)),
        }
    }
}

/// 入れ子の段の開き・閉じ。`at` は印より前に積んだ命令の数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mark {
    Open {
        at: usize,
        surface: u32,
        x: i64,
        y: i64,
        word: NestWord,
    },
    Close {
        at: usize,
    },
}

impl Mark {
    pub(crate) fn at(&self) -> usize {
        match *self {
            Mark::Open { at, .. } | Mark::Close { at } => at,
        }
    }
}

enum Event<'a> {
    Op(usize, &'a BlitOp),
    Mark(usize, Mark),
}

/// 命令と印を積んだ順に並べる。画素の上限で切った命令の後ろの印は読まない。
fn events<'a>(ops: &'a [BlitOp], marks: &'a [Mark]) -> impl Iterator<Item = Event<'a>> {
    let (mut i, mut k) = (0, 0);
    std::iter::from_fn(move || {
        if let Some(&mark) = marks.get(k)
            && mark.at() <= i
            && mark.at() <= ops.len()
        {
            k += 1;
            return Some(Event::Mark(k - 1, mark));
        }
        let op = ops.get(i)?;
        i += 1;
        Some(Event::Op(i - 1, op))
    })
}

/// 入れ子の段の中身（深さを問わない）。
#[derive(Debug, Clone, Copy, Default)]
struct Inner {
    /// 描くものが在る。
    drawn: bool,
    /// 平らに並べると層の答えが変わる項（`SourceOver` でない絵か、`SourceOver` でない語のまとめ）が在る。
    not_over: bool,
}

/// 命令が描く絵と重ね方（`asis` で描くなら `asis` 用の絵を `SourceOver` で）。描けない語と置き場の
/// 無い絵は `None`。
type Picture = Option<(ElementId, Mix)>;

/// 入れ子の段の描き方。
struct Nest {
    grouped: bool,
    /// まとめの外の層の左上。
    outer: (i64, i64),
}

/// 命令の列 `ops` と入れ子の印 `marks` から計画の項を `items` へ足す（描けない語の命令は入れない）。
pub(crate) fn push_items(
    items: &mut Vec<PlanItem>,
    ops: &[BlitOp],
    marks: &[Mark],
    world: &EmoWorld,
    atlas: &AtlasTable,
) {
    let has_rows = |surface| has_element_rows(world, surface);
    let empty = HashMap::new();
    let asis = world
        .world()
        .get_resource::<AsisPictures>()
        .map_or(&empty, |a| &a.0);
    let (inner, pictures) = walk(ops, marks, &has_rows, asis, atlas);

    let (mut origin, mut first) = ((0i64, 0i64), true);
    let mut stack: Vec<Nest> = Vec::new();
    for event in events(ops, marks) {
        match event {
            Event::Op(i, op) => {
                let Some((element, mut mix)) = pictures[i] else {
                    continue;
                };
                if std::mem::take(&mut first) {
                    mix = Mix::SourceOver;
                }
                let (x, y) = op.transform.offset();
                let (x, y) = (x.saturating_sub(origin.0), y.saturating_sub(origin.1));
                let size = atlas.entry(op.element).original;
                items.push(PlanItem::Image {
                    element,
                    transform: Transform::translate(x, y),
                    mix,
                    range: (mix == Mix::Copy).then_some(IRect {
                        x,
                        y,
                        w: size.w,
                        h: size.h,
                    }),
                });
            }
            Event::Mark(
                k,
                Mark::Open {
                    surface,
                    x,
                    y,
                    word,
                    ..
                },
            ) => {
                let (rx, ry) = (x.saturating_sub(origin.0), y.saturating_sub(origin.1));
                let extent = world.extent(surface);
                let outline = IRect {
                    x: rx,
                    y: ry,
                    w: extent.w,
                    h: extent.h,
                };
                let begin = match word {
                    NestWord::Asis if has_rows(surface) => {
                        items.push(PlanItem::Fill { rect: outline });
                        Some((Mix::SourceOver, None))
                    }
                    NestWord::Asis | NestWord::Base => {
                        inner[k].not_over.then_some((Mix::SourceOver, None))
                    }
                    NestWord::Mix(_) if !inner[k].drawn => None,
                    NestWord::Mix(mix) => {
                        let mix = if first { Mix::SourceOver } else { mix };
                        (mix != Mix::SourceOver || inner[k].not_over)
                            .then_some((mix, (mix == Mix::Copy).then_some(outline)))
                    }
                };
                let nest = Nest {
                    grouped: begin.is_some(),
                    outer: origin,
                };
                if let Some((mix, range)) = begin {
                    items.push(PlanItem::GroupBegin {
                        transform: Transform::translate(rx, ry),
                        mix,
                        range,
                    });
                    (origin, first) = ((x, y), true);
                }
                stack.push(nest);
            }
            Event::Mark(_, Mark::Close { .. }) => {
                if let Some(nest) = stack.pop() {
                    close(items, &nest, &mut origin, &mut first);
                }
            }
        }
    }
    while let Some(nest) = stack.pop() {
        close(items, &nest, &mut origin, &mut first);
    }
}

fn close(items: &mut Vec<PlanItem>, nest: &Nest, origin: &mut (i64, i64), first: &mut bool) {
    if nest.grouped {
        items.push(PlanItem::GroupEnd);
        (*origin, *first) = (nest.outer, false);
    }
}

/// 入れ子の段ごと（`marks` の添字）の中身と、命令ごとの描く絵。中身は開きの印の欄だけが意味を持つ。
/// element定義を持たない面を `asis` で置く段では、直に置く絵を `asis` 用の絵にする。
fn walk(
    ops: &[BlitOp],
    marks: &[Mark],
    has_rows: &impl Fn(u32) -> bool,
    asis: &HashMap<ElementId, ElementId>,
    atlas: &AtlasTable,
) -> (Vec<Inner>, Vec<Picture>) {
    let mut inner = vec![Inner::default(); marks.len()];
    let mut pictures: Vec<Picture> = vec![None; ops.len()];
    let mut stack: Vec<usize> = Vec::new();
    let asis_face = |k: usize| match marks[k] {
        Mark::Open { surface, word, .. } => word == NestWord::Asis && !has_rows(surface),
        Mark::Close { .. } => false,
    };
    let leave = |inner: &mut [Inner], stack: &mut Vec<usize>| {
        let Some(child) = stack.pop() else {
            return;
        };
        let Some(&parent) = stack.last() else {
            return;
        };
        let Mark::Open { surface, word, .. } = marks[child] else {
            return;
        };
        let c = inner[child];
        let fill = word == NestWord::Asis && has_rows(surface);
        inner[parent].drawn |= c.drawn || fill;
        inner[parent].not_over |=
            c.drawn && matches!(word, NestWord::Mix(m) if m != Mix::SourceOver);
    };
    for event in events(ops, marks) {
        match event {
            Event::Op(i, op) => {
                let Some(mut mix) = mix_of(&op.method) else {
                    continue;
                };
                let mut element = op.element;
                if op.method == ComposeMethod::Asis || stack.last().is_some_and(|&k| asis_face(k)) {
                    mix = Mix::SourceOver;
                    match asis.get(&op.element) {
                        Some(&picture) => element = picture,
                        None => tracing::debug!(
                            target: "areka_emo_compose",
                            element = op.element.0,
                            "asis 用の絵が無い: 通常の絵で描く"
                        ),
                    }
                }
                if atlas.entry(element).placement.is_none() {
                    continue;
                }
                pictures[i] = Some((element, mix));
                if let Some(&top) = stack.last() {
                    inner[top].drawn = true;
                    inner[top].not_over |= mix != Mix::SourceOver;
                }
            }
            Event::Mark(k, Mark::Open { .. }) => stack.push(k),
            Event::Mark(_, Mark::Close { .. }) => leave(&mut inner, &mut stack),
        }
    }
    while !stack.is_empty() {
        leave(&mut inner, &mut stack);
    }
    (inner, pictures)
}

/// element定義の行を 1 行以上持つ面か（ファイル名の慣習で置いた土台の絵は行に数えない）。
fn has_element_rows(world: &EmoWorld, surface: u32) -> bool {
    let base = world.base_images().used.contains_key(&surface);
    world
        .surface(surface)
        .is_some_and(|m| m.elements.len() > usize::from(base))
}

#[cfg(test)]
#[path = "plan_mix_tests.rs"]
mod tests;
