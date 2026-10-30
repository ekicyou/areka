//! 書き込みが箱へ回されているかの見立て（spec: bridge-store-user-root 要件 3.5〜3.9）。
//!
//! Store 版のアプリから起こされた中継は、`%LOCALAPPDATA%` の下への書き込みを
//! アプリの箱（`…\Packages\<名>\LocalCache\Local\`）へ回されることがある。控えのフォルダの
//! 最終の場所を OS に尋ね、本当の `LOCALAPPDATA` の下に解けるかで見立てる。
//! OS に触るのは `observe` の `canonicalize` の 2 回だけで、何も作らず書かない。

use std::io;
use std::path::{Path, PathBuf};

use tracing::info;

/// `%LOCALAPPDATA%` の下への書き込みが箱へ回されているかの見立て。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum BoxSight {
    /// 控えのフォルダが本当の `LOCALAPPDATA` の外に解けた。`box_dir` は解けた場所から
    /// `LOCALAPPDATA` の下の相対の部分を除いた所（末尾が合わなければ `None`）。
    Inside {
        asked: PathBuf,
        resolved: PathBuf,
        box_dir: Option<PathBuf>,
    },
    /// 控えのフォルダが本当の `LOCALAPPDATA` の下に解けた。
    Outside,
    /// 尋ねる物が無い・解けない。`why` は `no_cache_dir`・`not_under_local_app_data`・
    /// `not_there_yet`・`cannot_resolve` のどれか。
    Unknown {
        asked: Option<PathBuf>,
        why: &'static str,
    },
}

/// 薄い OS の口: 控えのフォルダが `local_app_data` の下なら、両方を `canonicalize` して `classify` へ。
/// 尋ねるのは控えのフォルダだけで、親を辿らない（本当の場所に在る親は箱の中でも「外」に解ける）。
pub(crate) fn observe(local_app_data: Option<&Path>, cache_dir: Option<&Path>) -> BoxSight {
    let Some(asked) = cache_dir else {
        return BoxSight::Unknown {
            asked: None,
            why: "no_cache_dir",
        };
    };
    let under = local_app_data
        .filter(|local| local.is_absolute())
        .and_then(|local| Some((local, asked.strip_prefix(local).ok()?)))
        .filter(|(_, rel)| !rel.as_os_str().is_empty());
    let Some((local, rel)) = under else {
        return BoxSight::Unknown {
            asked: Some(asked.to_path_buf()),
            why: "not_under_local_app_data",
        };
    };
    classify(
        rel,
        asked,
        std::fs::canonicalize(local),
        std::fs::canonicalize(asked),
    )
}

/// 値だけの判断。`rel` は `asked` の `local_app_data` からの相対の部分。
pub(crate) fn classify(
    rel: &Path,
    asked: &Path,
    local_resolved: io::Result<PathBuf>,
    asked_resolved: io::Result<PathBuf>,
) -> BoxSight {
    let unknown = |why| BoxSight::Unknown {
        asked: Some(asked.to_path_buf()),
        why,
    };
    let resolved = match asked_resolved {
        Ok(path) => path,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return unknown("not_there_yet"),
        Err(_) => return unknown("cannot_resolve"),
    };
    let Ok(local) = local_resolved else {
        return unknown("cannot_resolve");
    };
    if resolved
        .as_os_str()
        .eq_ignore_ascii_case(local.join(rel).as_os_str())
    {
        // shortcut: 箱の外で作られた控えのフォルダを、箱の中の中継は控えを箱の中へ初めて書けた起動までは「外」と見立てる（areka が居なくて書けない間は何回でも続く）。困る場面が出たら名乗りの読み（D1）を重ねる
        return BoxSight::Outside;
    }
    BoxSight::Inside {
        box_dir: strip_tail(&resolved, rel),
        asked: asked.to_path_buf(),
        resolved,
    }
}

/// `resolved` の末尾の部品が `rel` と ASCII の大小を無視して合えば、その数だけ親へ上がった所。
fn strip_tail(resolved: &Path, rel: &Path) -> Option<PathBuf> {
    let mut dir = resolved.to_path_buf();
    for part in rel.components().rev() {
        if !dir
            .file_name()
            .is_some_and(|name| name.eq_ignore_ascii_case(part.as_os_str()))
        {
            return None;
        }
        dir.pop();
    }
    Some(dir)
}

/// 記録（`Inside`＝B1 を 1 件・`Unknown`＝B2 を 1 件・`Outside`＝0 件）。
pub(crate) fn report(sight: &BoxSight) {
    const B1: &str = "[bridge] writes under LOCALAPPDATA are redirected into the app's private folder (Microsoft Store app)";
    const B2: &str = "[bridge] cannot tell whether writes under LOCALAPPDATA are redirected";
    match sight {
        BoxSight::Inside {
            asked,
            resolved,
            box_dir: Some(box_dir),
        } => info!(
            asked = %asked.display(),
            resolved = %resolved.display(),
            box_dir = %box_dir.display(),
            "{B1}"
        ),
        BoxSight::Inside {
            asked,
            resolved,
            box_dir: None,
        } => info!(asked = %asked.display(), resolved = %resolved.display(), "{B1}"),
        BoxSight::Outside => {}
        BoxSight::Unknown {
            asked: Some(asked),
            why,
        } => info!(why = *why, asked = %asked.display(), "{B2}"),
        BoxSight::Unknown { asked: None, why } => info!(why = *why, "{B2}"),
    }
}

#[cfg(test)]
#[path = "redirect_tests.rs"]
mod tests;
