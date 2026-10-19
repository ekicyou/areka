//! 起動時の根の並びの決め方（純粋）と、アプリの記憶の値の形（design.md「RootsResolve」）。
//!
//! 入力はすべて値で受ける（環境変数の値・記憶の値・実行ファイルのフォルダ）。プロセスの
//! 環境変数と記憶のファイルを読むのは `mod.rs` の薄い口だけ（要件 12.7）。
//! 控え（飛ばした項目）と「既定へ倒した」は結果に載せ、記録は `RootsReport` が残す。

use std::ffi::{OsStr, OsString};
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};

use areka_ghost::{BasewareRoot, BasewareRoots, same_root};
use areka_user_data::{UserRoot, UserRootOrigin};

/// 根の並びを渡す環境変数（要件 1.5）。
pub(crate) const ROOTS_ENV: &str = "AREKA_ROOT";
/// 記憶の中の「実行ファイルの隣」の綴り（要件 1.10）。
pub(crate) const EXE_TOKEN: &str = "<exe>";
/// 記憶の値の項目の区切り。
pub(crate) const MEMORY_SEPARATOR: char = '|';
/// `AREKA_ROOT` の項目の区切り（引用符は解かない＝要件 1.14）。
const ENV_SEPARATOR: u16 = b';' as u16;

/// 並び全体の出どころ（並びで 1 つ）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RootsOrigin {
    EnvVar,
    Memory,
    Default,
}

/// 並びに入れなかった項目（記録用）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RootSkip {
    /// `AREKA_ROOT` の空の項目（0 から数えて何番目か）。
    Empty { index: usize },
    /// 記憶の読めない項目（綴りのまま・理由 `empty`／`relative`／`invalid`／`exe_unavailable`）。
    Unreadable { item: String, reason: &'static str },
    /// 上に同じ根がある（下の綴り・残した上の根）。
    Duplicate { dir: PathBuf, kept: PathBuf },
}

/// 並びの 1 根の見え方（記録と告知に使う）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RootStatus {
    pub dir: PathBuf,
    pub found: bool,
}

/// 決めた並び。`status` は `roots` と同じ順・同じ数で、少なくとも 1 つが `found`。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ResolvedRoots {
    pub roots: BasewareRoots,
    pub origin: RootsOrigin,
    pub status: Vec<RootStatus>,
    pub skipped: Vec<RootSkip>,
    /// 記憶の読める項目が 0 だったので既定へ倒した（要件 1.13 の warn）。
    pub memory_fell_back: bool,
}

/// 並びが決まらない理由（告知へ・要件 9.4）。
///
/// どの腕も控え（`skipped`）と「既定へ倒した」（`memory_fell_back`）を持つ。起動が失敗しても
/// 控えの warn を残すため（要件 1.7・1.9・1.12・1.13）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RootError {
    /// 既定（記憶の `<exe>` だけで既定へ倒したときも）なのに実行ファイルの場所が取れない。
    ExeLocationUnavailable {
        tried: Vec<PathBuf>,
        origin: RootsOrigin,
        skipped: Vec<RootSkip>,
        memory_fell_back: bool,
    },
    /// `AREKA_ROOT` の項目がすべて空。
    EnvAllEmpty {
        skipped: Vec<RootSkip>,
        memory_fell_back: bool,
    },
    /// 並びの根がどれも見つからない。
    NoneFound {
        tried: Vec<PathBuf>,
        origin: RootsOrigin,
        skipped: Vec<RootSkip>,
        memory_fell_back: bool,
    },
}

/// 並びを決める純粋な判断（要件 1.3〜1.9・1.12・1.13・9.2・9.4）。
///
/// - `env`: `AREKA_ROOT` の値（`None`＝未設定）。在れば記憶は見ない。
/// - `memory`: アプリの記憶の `[baseware] roots`（`None`＝鍵が無い・空文字は 0 項目）。
/// - `exe_dir`: 実行ファイルのフォルダ（`None`＝取れない）。
/// - `user_root`: 並びに入れる書ける根（`None`＝今の形）。環境変数の腕は見ない（要件 3.4）。
///   既定の腕は「書ける根 → 実行ファイルの隣」、記憶の腕は無ければ先頭へ・在れば記憶の順のまま
///   （要件 3.1・3.2）。どちらでもその項目に印を付ける。既定の並びは記憶に書かない（要件 3.5）。
pub(crate) fn resolve_roots_from(
    env: Option<OsString>,
    memory: Option<String>,
    exe_dir: Option<PathBuf>,
    user_root: Option<&UserRoot>,
) -> Result<ResolvedRoots, RootError> {
    let mut skipped = Vec::new();
    // 印を付ける項目の位置（`items` の中・重複の除去の前）。
    let mut user_at = None;
    let mut memory_fell_back = false;
    let (items, origin) = match (env, memory) {
        (Some(env), _) => {
            let items = split_env(&env, &mut skipped);
            if items.is_empty() {
                return Err(RootError::EnvAllEmpty {
                    skipped,
                    memory_fell_back,
                });
            }
            (items, RootsOrigin::EnvVar)
        }
        (None, memory) => {
            let mut items = Vec::new();
            if let Some(memory) = memory {
                let (decoded, skips) = decode_memory(&memory, exe_dir.as_deref());
                skipped.extend(skips);
                memory_fell_back = decoded.is_empty();
                items = decoded;
            }
            if items.is_empty() {
                // 既定＝（書ける根 →）実行ファイルのフォルダ（要件 1.3・1.13・3.1）。
                let exe_dir = exe_dir.filter(|d| !d.as_os_str().is_empty());
                match (user_root, exe_dir) {
                    (Some(user), exe_dir) => {
                        user_at = Some(0);
                        let mut items = vec![user.dir.clone()];
                        items.extend(exe_dir);
                        (items, RootsOrigin::Default)
                    }
                    (None, Some(dir)) => (vec![dir], RootsOrigin::Default),
                    (None, None) => {
                        return Err(RootError::ExeLocationUnavailable {
                            tried: Vec::new(),
                            origin: RootsOrigin::Default,
                            skipped,
                            memory_fell_back,
                        });
                    }
                }
            } else {
                if let Some(user) = user_root {
                    // 記憶の並びに在れば記憶の順のまま、無ければ先頭へ（要件 3.2）。
                    user_at = items.iter().position(|d| same_root(d, &user.dir));
                    if user_at.is_none() {
                        items.insert(0, user.dir.clone());
                        user_at = Some(0);
                    }
                }
                (items, RootsOrigin::Memory)
            }
        }
    };

    // 重複は上を残して下を控える（要件 1.9）。印の項目は同じ根の最初なので必ず残る。
    let mut kept: Vec<PathBuf> = Vec::with_capacity(items.len());
    let mut kept_user = None;
    for (i, dir) in items.into_iter().enumerate() {
        match kept.iter().find(|k| same_root(k, &dir)) {
            Some(upper) => skipped.push(RootSkip::Duplicate {
                dir,
                kept: upper.clone(),
            }),
            None => {
                if user_at == Some(i) {
                    kept_user = Some(kept.len());
                }
                kept.push(dir);
            }
        }
    }

    // 見つからない根も並びに残す（要件 9.2）。どれも見つからなければ告知へ（要件 9.4）。
    let status: Vec<RootStatus> = kept
        .iter()
        .map(|dir| RootStatus {
            dir: dir.clone(),
            found: dir.is_dir(),
        })
        .collect();
    if !status.iter().any(|s| s.found) {
        return Err(RootError::NoneFound {
            tried: kept,
            origin,
            skipped,
            memory_fell_back,
        });
    }
    Ok(ResolvedRoots {
        roots: BasewareRoots::new(
            kept.into_iter()
                .enumerate()
                .map(|(i, dir)| match user_root {
                    Some(user) if kept_user == Some(i) => {
                        BasewareRoot::user_root(dir, user.origin == UserRootOrigin::EnvVar)
                    }
                    _ => BasewareRoot::new(dir),
                })
                .collect(),
        ),
        origin,
        status,
        skipped,
        memory_fell_back,
    })
}

/// `AREKA_ROOT` を `;` だけで区切る（引用符は解かない・`split_paths` は使わない）。
/// 空の項目は飛ばして控え、相対の項目はカレント基準で絶対にする（要件 1.6〜1.8・1.14）。
fn split_env(env: &OsStr, skipped: &mut Vec<RootSkip>) -> Vec<PathBuf> {
    let wide: Vec<u16> = env.encode_wide().collect();
    let mut items = Vec::new();
    for (index, part) in wide.split(|c| *c == ENV_SEPARATOR).enumerate() {
        if part.is_empty() {
            skipped.push(RootSkip::Empty { index });
            continue;
        }
        let dir = PathBuf::from(OsString::from_wide(part));
        let dir = std::path::absolute(&dir).unwrap_or_else(|e| {
            tracing::warn!(
                event = "root_absolute_failed",
                path = %dir.display(),
                error = %e,
                "AREKA_ROOT の項目を絶対化できないのでそのままの綴りで使う"
            );
            dir
        });
        items.push(dir);
    }
    items
}

/// 記憶の値を並びへ（`<exe>` の読み替え・空／相対／読めない項目の控え）。純粋（要件 1.10・1.12）。
/// 空文字は 0 項目（要件 1.13 の「0 項目」）。
pub(crate) fn decode_memory(value: &str, exe_dir: Option<&Path>) -> (Vec<PathBuf>, Vec<RootSkip>) {
    let mut items = Vec::new();
    let mut skipped = Vec::new();
    if value.is_empty() {
        return (items, skipped);
    }
    for item in value.split(MEMORY_SEPARATOR) {
        let unreadable = |reason| RootSkip::Unreadable {
            item: item.to_string(),
            reason,
        };
        if item == EXE_TOKEN {
            match exe_dir {
                Some(dir) => items.push(dir.to_path_buf()),
                None => skipped.push(unreadable("exe_unavailable")),
            }
        } else if item.is_empty() {
            skipped.push(unreadable("empty"));
        } else if has_invalid_chars(item) {
            skipped.push(unreadable("invalid"));
        } else if !Path::new(item).is_absolute() {
            skipped.push(unreadable("relative"));
        } else {
            items.push(PathBuf::from(item));
        }
    }
    (items, skipped)
}

/// Windows のフォルダのパスに使えない字（制御文字・`<>"?*`）を含むか。長いパスの接頭辞 `\\?\` は除いて見る。
fn has_invalid_chars(item: &str) -> bool {
    let body = item.strip_prefix(r"\\?\").unwrap_or(item);
    body.chars()
        .any(|c| c < ' ' || matches!(c, '<' | '>' | '"' | '?' | '*'))
}

/// 並びを記憶の値へ。実行ファイルのフォルダと同じ根は `<exe>`、ほかは絶対パスの綴り（要件 1.10・6.8）。
/// 一時の置き場は書かない（areka-P0-startup-options-core 要件 3.4）。純粋。
pub(crate) fn encode_memory(roots: &BasewareRoots, exe_dir: Option<&Path>) -> String {
    roots
        .roots()
        .iter()
        .filter(|r| !r.is_temporary())
        .map(|r| match exe_dir {
            Some(exe) if same_root(r.dir(), exe) => EXE_TOKEN.to_string(),
            _ => r.dir().to_string_lossy().into_owned(),
        })
        .collect::<Vec<_>>()
        .join(&MEMORY_SEPARATOR.to_string())
}

#[cfg(test)]
#[path = "resolve_tests.rs"]
mod tests;
