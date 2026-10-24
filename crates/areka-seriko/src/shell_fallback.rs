//! シェルの差し替えの戻り先（spec: descript-window-keys 要件 3.2〜3.5）。
//!
//! 今の面が新しいシェルに無いスコープを、descript の既定の面 → 組み込みの既定の面 → 出さない、の
//! 順に書き換える材料と 1 スコープ分の純粋な決め方。候補の並びは areka が組んで素の `u32` の列で
//! 渡す（seriko は areka を知らない）。書き換えは [`crate::ScopeStates::fall_back_shell_surfaces`]。

use std::collections::{BTreeMap, BTreeSet};

use areka_sakura::ActorKey;

/// シェルの差し替えで、今の面が新しいシェルに無いときの戻り先。
#[derive(Debug, Clone, Default)]
pub struct ShellSurfaceFallback {
    /// 新しいシェルの面の表に在る番号。
    pub surface_ids: BTreeSet<u32>,
    /// スコープごとの戻り先の候補の並び（前から試す）。載っていないスコープは候補なし。
    pub candidates: BTreeMap<ActorKey, Vec<u32>>,
}

/// 書き換えたスコープ 1 つぶんの結果（記録の材料）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellFallbackOutcome {
    /// 今の面が無く、候補の `to` へ替えた。
    FellBack {
        scope: ActorKey,
        missing: u32,
        to: u32,
    },
    /// 今の面も候補も無く、出さない（`Hidden`）。
    Hidden {
        scope: ActorKey,
        missing: u32,
        tried: Vec<u32>,
    },
}

/// 1 スコープ分の答え。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceKeep {
    /// 今の面が新しいシェルに在る（替えない）。
    Keep,
    /// 候補のこの面へ替える。
    FallBack(u32),
    /// 今の面も候補も無い（出さない）。
    Hide,
}

/// 1 スコープ分の決め方（純粋）: 今の面が在れば `Keep`、無ければ候補の最初に在る面、どれも無ければ `Hide`。
pub fn decide(current: u32, candidates: &[u32], surface_ids: &BTreeSet<u32>) -> SurfaceKeep {
    if surface_ids.contains(&current) {
        return SurfaceKeep::Keep;
    }
    candidates
        .iter()
        .find(|id| surface_ids.contains(id))
        .map_or(SurfaceKeep::Hide, |&id| SurfaceKeep::FallBack(id))
}

#[cfg(test)]
#[path = "shell_fallback_tests.rs"]
mod tests;
