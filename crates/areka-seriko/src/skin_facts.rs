//! 今のシェルとバルーンの事実と、`\s`・数の `\b` の判定（spec: mcp-strict-errors）。
//!
//! `check_script`（`areka` の `mcp/check_script_judge.rs`）から移した判定で、strict の記録と
//! `check_script` が同じ関数を使う。判定は自分で持たず、再生が使う解決器（`SurfaceResolver::resolve`・
//! `resolve_balloon_key`）の答えを検出点へ写すだけにする。

use std::collections::{BTreeMap, BTreeSet};

use areka_sakura::strict::DetectionPoint;

use crate::{BalloonResolve, SurfaceResolver, SurfaceTarget, resolve_balloon_key};

/// 判定に要る事実。本番は表示の層から写した [`SkinFacts`] が答え、テストは手書きの表で答える。
pub trait SkinLookup {
    /// `\s` の引数を、再生と同じ解決器で解いた結果。
    fn resolve_surface(&self, key: &str) -> SurfaceTarget;
    /// そのスコープのシェルに生の surface ID が在るか。スコープに絵が無ければ None。
    fn shell_has(&self, scope: u32, surface_id: u32) -> Option<bool>;
    /// そのスコープのバルーンに面の ID が在るか。バルーンが無ければ None。
    fn balloon_has(&self, scope: u32, balloon_id: u32) -> Option<bool>;
}

/// 表示の層から写した事実（別のスレッドへ渡せる値）。
#[derive(Clone)]
pub struct SkinFacts {
    /// scope 0 のシェルの別名の表から作った解決器（再生の解決器と同じ作り方）。
    resolver: SurfaceResolver,
    /// スコープ → シェルの生の surface ID の集合（表示の層に登録の在るスコープだけ）。
    shells: BTreeMap<u32, BTreeSet<u32>>,
    /// スコープ → バルーンの面の ID の集合（同上）。
    balloons: BTreeMap<u32, BTreeSet<u32>>,
}

impl SkinFacts {
    /// 写した値から組む（テストは手で組んだ値をここへ渡す）。
    pub fn new(
        aliases: BTreeMap<String, Vec<u32>>,
        shells: BTreeMap<u32, BTreeSet<u32>>,
        balloons: BTreeMap<u32, BTreeSet<u32>>,
    ) -> Self {
        Self {
            resolver: SurfaceResolver::new(aliases),
            shells,
            balloons,
        }
    }

    /// シェルの差し替えの後の事実（別名の表とシェルを替え、バルーンは今のまま）。
    pub fn with_shells(
        &self,
        aliases: BTreeMap<String, Vec<u32>>,
        shells: BTreeMap<u32, BTreeSet<u32>>,
    ) -> Self {
        Self::new(aliases, shells, self.balloons.clone())
    }

    /// バルーンの差し替えの後の事実（シェルと別名の表は今のまま）。
    pub fn with_balloons(&self, balloons: BTreeMap<u32, BTreeSet<u32>>) -> Self {
        Self {
            balloons,
            ..self.clone()
        }
    }
}

impl SkinLookup for SkinFacts {
    fn resolve_surface(&self, key: &str) -> SurfaceTarget {
        self.resolver.resolve(key)
    }

    fn shell_has(&self, scope: u32, surface_id: u32) -> Option<bool> {
        self.shells.get(&scope).map(|ids| ids.contains(&surface_id))
    }

    fn balloon_has(&self, scope: u32, balloon_id: u32) -> Option<bool> {
        self.balloons
            .get(&scope)
            .map(|ids| ids.contains(&balloon_id))
    }
}

/// 解けない名前・範囲外の数は `SurfaceUnresolved`、シェルに無い数は `SurfaceAbsent`、在れば None。
/// スコープに絵が無ければ診ない（解けない名前・範囲外の数も、絵の在るスコープでだけ知らせる）。
pub fn surface_missing(facts: &dyn SkinLookup, scope: u32, key: &str) -> Option<DetectionPoint> {
    match facts.resolve_surface(key) {
        SurfaceTarget::Show(id) => {
            (facts.shell_has(scope, id) == Some(false)).then_some(DetectionPoint::SurfaceAbsent)
        }
        SurfaceTarget::Unresolved => facts
            .shell_has(scope, 0)
            .is_some()
            .then_some(DetectionPoint::SurfaceUnresolved),
        SurfaceTarget::Hide => None,
    }
}

/// 範囲外は `BalloonInvalid`、バルーンに無い ID は `BalloonAbsent`、在れば None。
/// 名前の形の `\b[名前]` は、そのとき表示しているサーフェスの箱に依るので診ない（文字の層の持ち分）。
pub fn balloon_missing(facts: &dyn SkinLookup, scope: u32, key: &str) -> Option<DetectionPoint> {
    match resolve_balloon_key(key) {
        BalloonResolve::Show(id) => {
            (facts.balloon_has(scope, id) == Some(false)).then_some(DetectionPoint::BalloonAbsent)
        }
        BalloonResolve::Invalid => facts
            .balloon_has(scope, 0)
            .is_some()
            .then_some(DetectionPoint::BalloonInvalid),
        BalloonResolve::Hide | BalloonResolve::NameForm => None,
    }
}

#[cfg(test)]
#[path = "skin_facts_tests.rs"]
mod tests;
