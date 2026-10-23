//! descript の既定の面（`*.seriko.defaultsurface`）の読みと、戻り先の候補の並び
//! （descript-window-keys 要件 1・2.6）。
//!
//! 組み込みの既定の面（本体側 0・それ以外 10）を持つのはこのモジュールだけ。
//! 既定の面はゴーストの descript だけから読む（シェルの KV を引数に持たない＝要件 1.6 を型で守る）。
//! I/O を持たず、`crate::` を参照しない（手本の `#[path]` 取り込みでも組める）。

use std::collections::BTreeMap;

use tracing::warn;

use super::config::{scope_prefixes, scoped_value};

/// 既定の面のキーのスコープの後ろの綴り。
const DEFAULT_SURFACE_SUFFIX: &str = "seriko.defaultsurface";

/// 組み込みの既定の面（ukadoc の既定の値・本体側 0・それ以外 10）。
pub const fn builtin_default_surface(scope: usize) -> u32 {
    if scope == 0 { 0 } else { 10 }
}

/// スコープごとの descript の既定の面（書かれていなければ組み込みの既定の面）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DefaultSurfaces {
    /// descript に読める値が書かれていたスコープだけ（スコープ 2 以降を含む）。
    written: BTreeMap<usize, u32>,
}

impl DefaultSurfaces {
    /// どのスコープも書かれていない値（`Default` と同じ・テストと手本の入口）。
    pub fn builtin() -> Self {
        Self::default()
    }

    /// スコープの descript の既定の面。
    pub fn surface(&self, scope: usize) -> u32 {
        self.written
            .get(&scope)
            .copied()
            .unwrap_or(builtin_default_surface(scope))
    }

    /// 戻り先の候補の並び（`fallback_candidates(scope, self.surface(scope))`）。
    pub fn candidates(&self, scope: usize) -> Vec<u32> {
        fallback_candidates(scope, self.surface(scope))
    }
}

/// 戻り先の候補の並び: descript の既定の面、それが組み込みの既定の面と違えば続けて組み込みの既定の面。
pub fn fallback_candidates(scope: usize, descript_default: u32) -> Vec<u32> {
    let builtin = builtin_default_surface(scope);
    if descript_default == builtin {
        vec![builtin]
    } else {
        vec![descript_default, builtin]
    }
}

/// 候補の並びのうち、前から最初に在る面。
pub fn first_existing(candidates: &[u32], exists: impl Fn(u32) -> bool) -> Option<u32> {
    candidates.iter().copied().find(|&id| exists(id))
}

/// ゴーストの descript から既定の面を読む（`sakura.`＞`char0.`、`kero.`＞`char1.`、`char<n>.`（n≥2）の
/// `seriko.defaultsurface`）。値は前後の空白を除いて `u32` として読む。
pub fn read_default_surfaces(ghost_kv: &BTreeMap<String, String>) -> DefaultSurfaces {
    let mut written = BTreeMap::new();
    // 本体側・相方側は別名つき（配置の設定のほかのキーと同じ読み）。
    for scope in [0usize, 1] {
        if let Some(raw) = scoped_value(ghost_kv, scope, &[DEFAULT_SURFACE_SUFFIX]) {
            let (canon, alias) = scope_prefixes(scope);
            // 記録には実際に書かれていた綴りを載せる（正典の綴りが在ればそれが読まれている）。
            let canon_key = format!("{canon}{DEFAULT_SURFACE_SUFFIX}");
            let key = match alias {
                Some(alias) if !ghost_kv.contains_key(&canon_key) => {
                    format!("{alias}{DEFAULT_SURFACE_SUFFIX}")
                }
                _ => canon_key,
            };
            insert_readable(&mut written, scope, &key, raw);
        }
    }
    // スコープ 2 以降は `char<n>.`（n は 10 進の 2 以上・先頭の 0 なし）だけ。
    for (key, raw) in ghost_kv {
        let Some(n) = key
            .strip_prefix("char")
            .and_then(|rest| rest.strip_suffix(DEFAULT_SURFACE_SUFFIX))
            .and_then(|rest| rest.strip_suffix('.'))
        else {
            continue;
        };
        if let Ok(scope) = n.parse::<usize>()
            && scope >= 2
            && scope.to_string() == n
        {
            insert_readable(&mut written, scope, key, raw);
        }
    }
    DefaultSurfaces { written }
}

/// 値を `u32` として読めれば入れ、読めなければ組み込みの面のまま記録を 1 件残す（要件 1.5）。
fn insert_readable(written: &mut BTreeMap<usize, u32>, scope: usize, key: &str, raw: &str) {
    match raw.trim().parse::<u32>() {
        Ok(id) => {
            written.insert(scope, id);
        }
        Err(error) => warn!(
            event = "descript_value_unreadable",
            scope,
            %key,
            value = %raw,
            %error,
            to = builtin_default_surface(scope),
            "descript_value_unreadable: 既定の面が 0 以上の整数として読めない——組み込みの既定の面を使う"
        ),
    }
}

#[cfg(test)]
#[path = "descript_window_keys_tests.rs"]
mod tests;
