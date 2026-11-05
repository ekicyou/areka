//! # Ancestors - 上限つきの祖先の走査
//!
//! `ChildOf` をたどる所（窓に属するかの判定・色の受け継ぎ・木の深さ）が共用する。
//! 親の輪（A→B→A）があっても、上限を越えたら `error!` を 1 行残して止まる。

use bevy_ecs::prelude::*;
use tracing::error;

/// 祖先をたどる段数の上限。数十段の木で 1,024 段に届くのは輪のときだけ。
pub(crate) const MAX_ANCESTOR_DEPTH: usize = 1024;

/// `start` から親を順にたどる（`start` は含まない。親・祖父母…の順）。
/// 根に着くか、上限を越えたら終わる。`parent_of` は `ChildOf` を読む閉包。
pub(crate) fn ancestors<F>(start: Entity, parent_of: F) -> Ancestors<F>
where
    F: FnMut(Entity) -> Option<Entity>,
{
    Ancestors {
        start,
        current: Some(start),
        steps: 0,
        parent_of,
    }
}

/// [`ancestors`] が返すイテレータ。
pub(crate) struct Ancestors<F> {
    start: Entity,
    current: Option<Entity>,
    steps: usize,
    parent_of: F,
}

impl<F: FnMut(Entity) -> Option<Entity>> Iterator for Ancestors<F> {
    type Item = Entity;

    fn next(&mut self) -> Option<Entity> {
        let current = self.current.take()?;
        let parent = (self.parent_of)(current)?;
        if self.steps >= MAX_ANCESTOR_DEPTH {
            error!(
                start = ?self.start,
                limit = MAX_ANCESTOR_DEPTH,
                "[ancestors] 祖先の走査が上限を越えたので止める（親の輪の疑い）"
            );
            return None;
        }
        self.steps += 1;
        self.current = Some(parent);
        Some(parent)
    }
}

#[cfg(test)]
#[path = "ancestors_tests.rs"]
mod ancestors_tests;
