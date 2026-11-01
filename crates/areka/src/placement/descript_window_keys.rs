//! descript の既定の面（`*.seriko.defaultsurface`）の読みと、戻り先の候補の並び
//! （descript-window-keys 要件 1・2.6）と、バルーンの動かせない指定（`*.balloon.dontmove`）の読み（要件 4）と、
//! バルーンの倍率をシェルに連動させる指定（`*.balloon.syncscale`）の読み（shell-scale-linkage 要件 6）。
//!
//! 組み込みの既定の面（本体側 0・それ以外 10）を持つのはこのモジュールだけ。
//! 既定の面はゴーストの descript だけから読む（シェルの KV を引数に持たない＝要件 1.6 を型で守る）。
//! I/O を持たず、`crate::` を参照しない（手本の `#[path]` 取り込みでも組める）。

use std::collections::{BTreeMap, BTreeSet};

use tracing::{debug, warn};

use super::config::{scope_prefixes, scoped_value};

// ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_ghost.html#sakura.seriko.defaultsurface_2c_6570_5024:1
// ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_ghost.html#kero.seriko.defaultsurface_2c_6570_5024:1
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
        if let Some((key, raw)) = scoped_entry(ghost_kv, scope, DEFAULT_SURFACE_SUFFIX) {
            insert_readable(&mut written, scope, &key, raw);
        }
    }
    // スコープ 2 以降は `char<n>.`（n は 10 進の 2 以上・先頭の 0 なし）だけ。
    for (key, raw) in ghost_kv {
        if let Some(scope) = numbered_scope(key, DEFAULT_SURFACE_SUFFIX) {
            insert_readable(&mut written, scope, key, raw);
        }
    }
    DefaultSurfaces { written }
}

// ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_ghost.html#balloon.dontmove_2ctrue:1
// ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_shell.html#sakura.balloon.dontmove_2c_6570_5024:1
// ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_shell.html#kero.balloon.dontmove_2c_6570_5024:1
// ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_shell.html#char_2a.balloon.dontmove_2c_6570_5024:1
/// バルーンの動かせない指定のキーのスコープの後ろの綴り（ゴーストの段はこの綴りそのもの）。
const BALLOON_DONTMOVE: &str = "balloon.dontmove";

/// スコープ別キーの値と、実際に書かれていた綴り（正典の綴りが在ればそれが読まれている）。
fn scoped_entry<'a>(
    kv: &'a BTreeMap<String, String>,
    scope: usize,
    suffix: &str,
) -> Option<(String, &'a str)> {
    let raw = scoped_value(kv, scope, &[suffix])?;
    let (canon, alias) = scope_prefixes(scope);
    let canon_key = format!("{canon}{suffix}");
    let key = match alias {
        Some(alias) if !kv.contains_key(&canon_key) => format!("{alias}{suffix}"),
        _ => canon_key,
    };
    Some((key, raw))
}

/// `scopes` のうち、descript の段が利用者のドラッグを禁止するスコープ。
/// 段の優先: シェルの `<スコープ>.balloon.dontmove`（`sakura.`＞`char0.`・`kero.`＞`char1.`・`char<n>.`）
/// ＞ ゴーストの `balloon.dontmove` ＞ 禁止しない。値は `0`・`1`・`true`・`false` だけを読む。
pub fn read_balloon_dontmove(
    ghost_kv: &BTreeMap<String, String>,
    shell_kv: &BTreeMap<String, String>,
    scopes: &[usize],
) -> BTreeSet<u32> {
    let mut locked = BTreeSet::new();
    for &scope in scopes {
        // 読めない段は書かれていないものとして次の段へ進む（要件 4.5）。
        let shell = scoped_entry(shell_kv, scope, BALLOON_DONTMOVE)
            .and_then(|(key, raw)| read_flag(scope, &key, raw));
        let ghost = || {
            ghost_kv
                .get(BALLOON_DONTMOVE)
                .and_then(|raw| read_flag(scope, BALLOON_DONTMOVE, raw))
        };
        if shell.or_else(ghost).unwrap_or(false) {
            // スコープ番号は u32 に収まる（窓のスコープは数個）。
            locked.insert(scope as u32);
        }
    }
    locked
}

/// 動かせない指定の値を読む（`0`・`false` → 許す・`1`・`true` → 禁止・それ以外は記録して読めない）。
fn read_flag(scope: usize, key: &str, raw: &str) -> Option<bool> {
    match raw.trim() {
        "0" | "false" => Some(false),
        "1" | "true" => Some(true),
        _ => {
            warn!(
                event = "descript_value_unreadable",
                scope,
                %key,
                value = %raw,
                "descript_value_unreadable: 動かせない指定が 0・1・true・false のどれでもない——次の段で決める"
            );
            None
        }
    }
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

// ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_ghost.html#balloon.syncscale_2ctrue:1
// ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_shell.html#sakura.balloon.syncscale_2ctrue:1
// ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_shell.html#kero.balloon.syncscale_2ctrue:1
// ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_shell.html#char_2a.balloon.syncscale_2ctrue:1
/// バルーンの倍率をシェルに連動させる指定のキーのスコープの後ろの綴り（ゴーストの段はこの綴りそのもの）。
const BALLOON_SYNCSCALE: &str = "balloon.syncscale";

/// `balloon.syncscale` の指定（shell-scale-linkage 要件 6）: ゴーストの段の真偽と、シェルの段で連動するスコープ。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BalloonSyncScale {
    ghost: bool,
    shell_scopes: BTreeSet<u32>,
}

impl BalloonSyncScale {
    /// スコープのバルーンの倍率をシェルに連動させるか（ゴーストの段かシェルの段のどちらかが `true`・要件 6.3）。
    // 本番で読む下流は shell-balloon-scale-setting（本 spec は持つだけ・要件 6.6）。
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn is_synced(&self, scope: u32) -> bool {
        self.ghost || self.shell_scopes.contains(&scope)
    }
}

/// ゴーストの `balloon.syncscale` と、シェルの `sakura.`＞`char0.`・`kero.`＞`char1.`・`char<n>.`（n≥2）の
/// `balloon.syncscale` を読む。値は前後の空白を除いて `true` と完全に一致するときだけ有効（要件 6.1〜6.4）。
pub fn read_balloon_syncscale(
    ghost_kv: &BTreeMap<String, String>,
    shell_kv: &BTreeMap<String, String>,
) -> BalloonSyncScale {
    let ghost = ghost_kv
        .get(BALLOON_SYNCSCALE)
        .is_some_and(|raw| read_syncscale("all", BALLOON_SYNCSCALE, raw));
    let mut shell_scopes = BTreeSet::new();
    let mut read = |scope: usize, key: &str, raw: &str| {
        // 収まらない番号の窓は在り得ないので、読んだ記録だけ残して集合には入れない。
        if read_syncscale(scope, key, raw)
            && let Ok(scope) = u32::try_from(scope)
        {
            shell_scopes.insert(scope);
        }
    };
    for scope in [0usize, 1] {
        if let Some((key, raw)) = scoped_entry(shell_kv, scope, BALLOON_SYNCSCALE) {
            read(scope, &key, raw);
        }
    }
    for (key, raw) in shell_kv {
        if let Some(scope) = numbered_scope(key, BALLOON_SYNCSCALE) {
            read(scope, key, raw);
        }
    }
    BalloonSyncScale {
        ghost,
        shell_scopes,
    }
}

/// 連動の指定の値を読み、鍵ごとに警告より下の段の記録を 1 件残す（`true` 以外は指定なし・要件 6.4・6.7）。
fn read_syncscale(scope: impl std::fmt::Display, key: &str, raw: &str) -> bool {
    let synced = raw.trim() == "true";
    debug!(
        event = "balloon_syncscale_read",
        %key,
        %scope,
        value = ?raw,
        synced,
        "balloon_syncscale_read: バルーンの倍率をシェルに連動させる指定を読んだ"
    );
    synced
}

/// `char<n>.<suffix>` のスコープ番号（n は 10 進の 2 以上・先頭の 0 なし）。
fn numbered_scope(key: &str, suffix: &str) -> Option<usize> {
    let n = key
        .strip_prefix("char")?
        .strip_suffix(suffix)?
        .strip_suffix('.')?;
    let scope = n.parse::<usize>().ok()?;
    (scope >= 2 && scope.to_string() == n).then_some(scope)
}

#[cfg(test)]
#[path = "descript_window_keys_tests.rs"]
mod tests;
