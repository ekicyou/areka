//! 実物を読む口（[`probe`]）・書けるかの確かめの唯一の部品（[`open_dir_for_add`]）と、
//! プロセスのはじめに 1 回だけ走る準備（[`establish`]）（spec: areka-P0-user-data-root
//! 要件 1.1・1.2・1.5・1.6・2.2・2.4・5.4・5.5）。利用者には何も尋ねない。記録の文は ASCII。

use crate::{
    CopyDone, Decision, Layout, Reason, Signals, USER_ROOT_ENV, Unlocatable, UserRoot,
    UserRootOrigin, WINGET_MARKER_PREFIX, Writability, copy_tree_atomic, decide,
};
use std::ffi::OsStr;
use std::os::windows::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

/// `FILE_ADD_FILE`（フォルダに物を足す権限）。
const FILE_ADD_FILE: u32 = 0x2;
/// `FILE_ADD_SUBDIRECTORY`（フォルダにフォルダを足す権限）。
const FILE_ADD_SUBDIRECTORY: u32 = 0x4;
/// `FILE_FLAG_BACKUP_SEMANTICS`（フォルダのハンドルを開くのに要る）。
const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;

/// フォルダに物を足せるかを、何も作らずに OS へ尋ねる（書けるかの確かめの唯一の部品）。
/// 「物を足す」権限でフォルダを開いてすぐ閉じ、OS の結果をそのまま返す。
/// `write-failure-notice` もこの部品を呼ぶ。
pub fn open_dir_for_add(dir: &Path) -> std::io::Result<()> {
    std::fs::OpenOptions::new()
        .access_mode(FILE_ADD_FILE | FILE_ADD_SUBDIRECTORY)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
        .open(dir)
        .map(drop)
}

/// 実行ファイルのフォルダの確かめ。[`open_dir_for_add`] の結果を写すだけ。
pub(crate) fn exe_dir_writability(dir: &Path) -> Writability {
    match open_dir_for_add(dir) {
        Ok(()) => Writability::Writable,
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => Writability::Denied,
        Err(error) => Writability::Unknown(error.to_string()),
    }
}

/// 実行ファイルのフォルダの直下の winget の控えのファイル（`Areka.Areka.Portable_*.db`・大小無視）。
/// 読めなければ無い扱いで warn。
pub(crate) fn find_winget_marker(exe_dir: &Path) -> Option<String> {
    let entries = match std::fs::read_dir(exe_dir) {
        Ok(entries) => entries,
        Err(error) => {
            tracing::warn!(
                event = "user_data_marker_unreadable",
                exe_dir = %exe_dir.display(),
                error = %error,
                "exe folder could not be read for the winget marker; treated as absent"
            );
            return None;
        }
    };
    let prefix = WINGET_MARKER_PREFIX.to_ascii_lowercase();
    entries.flatten().find_map(|entry| {
        let name = entry.file_name().into_string().ok()?;
        let lower = name.to_ascii_lowercase();
        let is_file = entry.file_type().is_ok_and(|kind| kind.is_file());
        (is_file && lower.starts_with(&prefix) && lower.ends_with(".db")).then_some(name)
    })
}

/// 本物の入力を読む薄い口（環境変数 2 つ・控えのファイル・書けるかの確かめ）。
/// 控えが在れば書けるかは確かめない（判断は控えで決まる）。
pub fn probe(exe_dir: Option<&Path>) -> Signals {
    let (winget_marker, writability) = match exe_dir {
        None => (
            None,
            Writability::Unknown("exe folder is unknown".to_owned()),
        ),
        Some(dir) => match find_winget_marker(dir) {
            // 確かめていない。判断は控えで決まるので値は使われない。
            Some(marker) => (Some(marker), Writability::Writable),
            None => (None, exe_dir_writability(dir)),
        },
    };
    Signals {
        user_root_env: std::env::var_os(USER_ROOT_ENV),
        local_app_data: std::env::var_os("LOCALAPPDATA"),
        winget_marker,
        writability,
    }
}

/// プロセスのはじめに 1 回: probe → decide → 書ける根を作る → 記憶の引き継ぎ → 記録。
/// `profile_env` は [`crate::PROFILE_DIR_ENV`] の値（在れば引き継がない＝記憶はそちら）。
pub fn establish(exe_dir: Option<&Path>, profile_env: Option<&OsStr>) -> Layout {
    establish_from(&probe(exe_dir), exe_dir, profile_env)
}

/// [`establish`] の内側（本物の環境を読まない）。
pub(crate) fn establish_from(
    signals: &Signals,
    exe_dir: Option<&Path>,
    profile_env: Option<&OsStr>,
) -> Layout {
    let (root, reason) = decide_logged(signals, exe_dir);
    let user_root = root.and_then(|root| create(root, &reason));
    if let (Some(root), None, Some(exe_dir)) = (&user_root, profile_env, exe_dir) {
        migrate(&root.dir, exe_dir);
    }
    let layout = Layout { user_root, reason };
    log_layout(&layout);
    layout
}

/// 読み取りだけの走りの準備（spec: readonly-mode 要件 4.1・4.2・4.5）: probe → decide → 記録。
/// 書ける根は作らず（無ければ隣の形）、記憶も写さない。
pub fn establish_readonly(exe_dir: Option<&Path>, profile_env: Option<&OsStr>) -> Layout {
    establish_readonly_from(&probe(exe_dir), exe_dir, profile_env)
}

/// [`establish_readonly`] の内側（本物の環境を読まない）。判断と記録は [`establish_from`] と分け合う。
/// `profile_env` は形を変えない（記憶の置き場は [`readonly_app_profile_dir`] が決める）。
pub fn establish_readonly_from(
    signals: &Signals,
    exe_dir: Option<&Path>,
    _profile_env: Option<&OsStr>,
) -> Layout {
    let (root, reason) = decide_logged(signals, exe_dir);
    let user_root = root.filter(|root| {
        let present = root.dir.is_dir();
        if !present {
            tracing::info!(
                event = "user_root_withheld",
                dir = %root.dir.display(),
                "read-only run: the user root is not created; staying beside the exe"
            );
        }
        present
    });
    let layout = Layout { user_root, reason };
    log_layout(&layout);
    layout
}

/// 読み取りだけの走りの記憶の置き場（spec: readonly-mode 要件 4.3・11.2）。
/// `profile_env` が在ればそれ。写すはずの場面（[`migrate_source`]）なら写す元をそのまま返す。
/// ほかは [`app_profile_dir`](crate::app_profile_dir) と同じ値。
pub fn readonly_app_profile_dir(
    profile_env: Option<&OsStr>,
    layout: &Layout,
    exe_dir: Option<&Path>,
) -> Option<PathBuf> {
    if profile_env.is_none()
        && let (Some(root), Some(exe_dir)) = (&layout.user_root, exe_dir)
        && let Some((from, to)) = migrate_source(&root.dir, exe_dir)
    {
        tracing::info!(
            event = "app_memory_read_beside",
            from = %from.display(),
            to = %to.display(),
            "read-only run: reading the bundled app memory without copying it"
        );
        return Some(from);
    }
    crate::app_profile_dir(profile_env, layout, exe_dir)
}

/// 判断と、隣に居る・決められないときの記録（ふつうの道と読み取りだけの道が分け合う）。
/// 書ける根に当たれば、まだ作っていない根を返す。
fn decide_logged(signals: &Signals, exe_dir: Option<&Path>) -> (Option<UserRoot>, Reason) {
    if let (Writability::Unknown(error), None) = (&signals.writability, &signals.user_root_env) {
        tracing::warn!(
            event = "user_data_writability_unknown",
            exe_dir = %exe_dir.map(Path::display).map(|d| d.to_string()).unwrap_or_default(),
            error = %error,
            "exe folder writability could not be checked; staying beside the exe"
        );
    }
    match decide(signals) {
        Decision::Beside { reason } => (None, reason),
        Decision::Fallback { reason, error } => {
            let (kind, tried) = unlocatable(&error);
            tracing::error!(
                event = "user_root_unavailable",
                reason = reason_str(&reason),
                tried = %tried,
                error = kind,
                "user root could not be located; staying beside the exe"
            );
            (None, reason)
        }
        Decision::UserRoot { root, reason } => (Some(root), reason),
    }
}

/// 書ける根を作る（無いときだけ）。作れなければ `None`（今の形）。
fn create(root: UserRoot, reason: &Reason) -> Option<UserRoot> {
    if root.dir.is_dir() {
        return Some(root);
    }
    match std::fs::create_dir_all(&root.dir) {
        Ok(()) => {
            tracing::info!(
                event = "user_root_created",
                dir = %root.dir.display(),
                "user root created"
            );
            Some(root)
        }
        Err(error) => {
            tracing::error!(
                event = "user_root_unavailable",
                reason = "create_failed",
                layout_reason = reason_str(reason),
                dir = %root.dir.display(),
                error = %error,
                "user root could not be created; staying beside the exe"
            );
            None
        }
    }
}

/// 実行ファイルの隣の記憶を書ける根へ一度だけ写す。隣は読むだけ。
/// 写せなくても置き場は書ける根の `profile\areka` のまま（要件 5.5）。
fn migrate(root: &Path, exe_dir: &Path) {
    let Some((from, to)) = migrate_source(root, exe_dir) else {
        return;
    };
    match copy_tree_atomic(&from, &to, root) {
        Ok(CopyDone::Copied(stats)) => tracing::info!(
            event = "app_memory_migrated",
            from = %from.display(),
            to = %to.display(),
            files = stats.files,
            "app memory migrated to the user root"
        ),
        Ok(CopyDone::Raced) => tracing::info!(
            event = "app_memory_migrated",
            from = %from.display(),
            to = %to.display(),
            raced = true,
            "app memory already migrated by another process"
        ),
        Err(error) => tracing::error!(
            event = "app_memory_migrate_failed",
            from = %from.display(),
            to = %to.display(),
            error = %error,
            "app memory could not be migrated; using the user root profile"
        ),
    }
}

/// 写すはずか（写しの条件の持ち主は 1 つ）: `root\profile\areka` が無く `exe_dir\profile\areka` が
/// フォルダのときだけ `Some((写す元, 写し先))`。[`migrate`] と [`readonly_app_profile_dir`] が分け合う。
pub(crate) fn migrate_source(root: &Path, exe_dir: &Path) -> Option<(PathBuf, PathBuf)> {
    let to = root.join("profile").join("areka");
    let from = exe_dir.join("profile").join("areka");
    (!to.exists() && from.is_dir()).then_some((from, to))
}

fn log_layout(layout: &Layout) {
    let marker = match &layout.reason {
        Reason::WingetInstall { marker } => marker.as_str(),
        _ => "",
    };
    let (mode, dir, origin) = match &layout.user_root {
        Some(root) => (
            "user_root",
            root.dir.display().to_string(),
            match root.origin {
                UserRootOrigin::Default => "default",
                UserRootOrigin::EnvVar => "env_var",
            },
        ),
        None => ("beside", String::new(), ""),
    };
    tracing::info!(
        event = "user_data_layout",
        mode,
        reason = reason_str(&layout.reason),
        user_root = %dir,
        origin,
        marker,
        "user data layout decided"
    );
}

fn reason_str(reason: &Reason) -> &'static str {
    match reason {
        Reason::WingetInstall { .. } => "winget_install",
        Reason::Unwritable => "unwritable",
        Reason::EnvVar => "env_var",
        Reason::Neither => "none",
    }
}

/// 決められない理由の種類と、試した場所（値か環境変数の名前）。
fn unlocatable(error: &Unlocatable) -> (&'static str, String) {
    match error {
        Unlocatable::EnvEmpty => ("env_empty", USER_ROOT_ENV.to_owned()),
        Unlocatable::EnvRelative { value } => ("env_relative", value.display().to_string()),
        Unlocatable::LocalAppDataMissing => ("local_app_data_missing", "LOCALAPPDATA".to_owned()),
        Unlocatable::LocalAppDataRelative { value } => {
            ("local_app_data_relative", value.display().to_string())
        }
    }
}

#[cfg(test)]
#[path = "establish_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "establish_readonly_tests.rs"]
mod readonly_tests;
