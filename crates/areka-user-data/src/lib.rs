//! 書ける根（利用者ごとの置き場）を使う形の判断と記憶の置き場の決め方（spec: areka-P0-user-data-root）。
//!
//! areka と中継（`areka-mcp-bridge`）が共有する葉のクレート。判断（[`decide`]）と
//! 記憶の置き場（[`app_profile_dir`]）は値だけで決まる純粋な関数で、本物の入力を読む口
//! （`probe`）とプロセスのはじめに 1 回だけ走る準備（`establish`）、フォルダの木を半端なく
//! 写す手続き（`copy` モジュール）は、この判断の上に載る。

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

/// 書ける根の場所を指す環境変数（要件 2.3）。在れば winget の控え・書けるかに関わらず書ける根を使う（要件 1.3）。
pub const USER_ROOT_ENV: &str = "AREKA_USER_ROOT";
/// areka の記憶の置き場を指す環境変数（今のまま・要件 5.1）。
pub const PROFILE_DIR_ENV: &str = "AREKA_PROFILE_DIR";
/// winget の控えのファイルの名前の頭（`<名乗り>_<出どころ>.db`・要件 1.1・7.5）。
pub const WINGET_MARKER_PREFIX: &str = "Areka.Areka.Portable_";
/// 既定の書ける根の、`LOCALAPPDATA` の下のフォルダ名（要件 2.1）。
pub const USER_ROOT_DIR_NAME: &str = "areka";
/// 写しの作業フォルダの棚（書ける根の直下）。
pub const COPY_WORK: &str = ".copy-work";

/// 実行ファイルのフォルダに書けるか（何も作らずに確かめた結果）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Writability {
    Writable,
    /// 権限が無い（`PermissionDenied`）。
    Denied,
    /// 確かめられなかった（理由の文）。今の形のまま（書ける扱い）にして warn を残す。
    Unknown(String),
}

/// 判断の入力（すべて値）。`probe` が本物を詰め、テストは直接組む（要件 10.6）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Signals {
    pub user_root_env: Option<OsString>,
    pub local_app_data: Option<OsString>,
    /// 実行ファイルのフォルダの直下に在った winget の控えのファイルの名前（`None`＝無い）。
    pub winget_marker: Option<String>,
    pub writability: Writability,
}

/// 書ける根がどこから決まったか。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UserRootOrigin {
    /// `LOCALAPPDATA\areka`。
    Default,
    /// [`USER_ROOT_ENV`] の値。
    EnvVar,
}

/// 書ける根。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserRoot {
    /// 絶対パス。
    pub dir: PathBuf,
    pub origin: UserRootOrigin,
}

/// 書ける根を使う形にした理由（要件 1.5）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reason {
    WingetInstall {
        marker: String,
    },
    Unwritable,
    EnvVar,
    /// どれでもない（zip を展開して書ける形）。
    Neither,
}

/// 書ける根を決められない・作れない理由（要件 2.4）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Unlocatable {
    EnvEmpty,
    EnvRelative {
        value: PathBuf,
    },
    /// `LOCALAPPDATA` が無い・空。
    LocalAppDataMissing,
    LocalAppDataRelative {
        value: PathBuf,
    },
    CreateFailed {
        dir: PathBuf,
        error: String,
    },
}

/// 判断の結果。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Decision {
    /// 今の形（実行ファイルの隣）。`reason` は [`Reason::Neither`]。
    Beside {
        reason: Reason,
    },
    UserRoot {
        root: UserRoot,
        reason: Reason,
    },
    /// 使う形に当たったが場所を決められない → 今の形で続ける（error）。
    Fallback {
        reason: Reason,
        error: Unlocatable,
    },
}

/// プロセスに 1 つの形（`establish` の答え）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layout {
    /// `None`＝今の形（実行ファイルの隣）。
    pub user_root: Option<UserRoot>,
    pub reason: Reason,
}

/// 純粋な判断（要件 1.1〜1.4・2.1・2.3・2.4 の「決められない」）。フォルダを作るのは `establish`。
///
/// 順: 環境変数 → winget の控え → 書けない → どれでもない（書けるか分からないも「どれでもない」）。
/// 場所: 環境変数の値（空・相対は [`Unlocatable`]）→ `LOCALAPPDATA\areka`（無い・空・相対は [`Unlocatable`]）。
pub fn decide(signals: &Signals) -> Decision {
    if let Some(env) = &signals.user_root_env {
        return match absolute(env) {
            Ok(dir) => Decision::UserRoot {
                root: UserRoot {
                    dir,
                    origin: UserRootOrigin::EnvVar,
                },
                reason: Reason::EnvVar,
            },
            Err(value) => Decision::Fallback {
                reason: Reason::EnvVar,
                error: match value {
                    None => Unlocatable::EnvEmpty,
                    Some(value) => Unlocatable::EnvRelative { value },
                },
            },
        };
    }
    let reason = match (&signals.winget_marker, &signals.writability) {
        (Some(marker), _) => Reason::WingetInstall {
            marker: marker.clone(),
        },
        (None, Writability::Denied) => Reason::Unwritable,
        (None, Writability::Writable | Writability::Unknown(_)) => {
            return Decision::Beside {
                reason: Reason::Neither,
            };
        }
    };
    match signals.local_app_data.as_deref().map(absolute) {
        Some(Ok(lad)) => Decision::UserRoot {
            root: UserRoot {
                dir: lad.join(USER_ROOT_DIR_NAME),
                origin: UserRootOrigin::Default,
            },
            reason,
        },
        None | Some(Err(None)) => Decision::Fallback {
            reason,
            error: Unlocatable::LocalAppDataMissing,
        },
        Some(Err(Some(value))) => Decision::Fallback {
            reason,
            error: Unlocatable::LocalAppDataRelative { value },
        },
    }
}

/// 環境変数の値を絶対パスとして読む。空は `Err(None)`、相対は `Err(Some(値))`。
fn absolute(value: &OsStr) -> Result<PathBuf, Option<PathBuf>> {
    let path = PathBuf::from(value);
    if value.is_empty() {
        Err(None)
    } else if path.is_absolute() {
        Ok(path)
    } else {
        Err(Some(path))
    }
}

/// areka の記憶の置き場（要件 5.1〜5.3・6.1）。純粋。
///
/// `profile_env` → `<書ける根>\profile\areka` → `<exe_dir>\profile\areka` → `None`（`exe_dir` も無い）。
pub fn app_profile_dir(
    profile_env: Option<&OsStr>,
    layout: &Layout,
    exe_dir: Option<&Path>,
) -> Option<PathBuf> {
    if let Some(env) = profile_env {
        return Some(PathBuf::from(env));
    }
    let base = match &layout.user_root {
        Some(root) => root.dir.as_path(),
        None => exe_dir?,
    };
    Some(base.join("profile").join("areka"))
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
