//! 判断（`decide`）・場所・記憶の置き場（`app_profile_dir`）の決定論テスト（要件 10.1・10.2・10.5）。
//! 値だけで組み、プロセスの環境変数もファイルも触らない（要件 10.6）。

use super::*;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

const LAD: &str = r"C:\Users\u\AppData\Local";
/// 利用者向けのマニフェスト（手元・`__DefaultSource`）で入れたときの控え。
const MARKER_LOCAL: &str = "Areka.Areka.Portable__DefaultSource.db";
/// winget-pkgs から入れたときの控え。
const MARKER_PKGS: &str = "Areka.Areka.Portable_Microsoft.Winget.Source_8wekyb3d8bbwe.db";

fn signals(
    env: Option<&str>,
    lad: Option<&str>,
    marker: Option<&str>,
    writability: Writability,
) -> Signals {
    Signals {
        user_root_env: env.map(OsString::from),
        local_app_data: lad.map(OsString::from),
        winget_marker: marker.map(str::to_owned),
        writability,
    }
}

fn default_root() -> UserRoot {
    UserRoot {
        dir: Path::new(LAD).join(USER_ROOT_DIR_NAME),
        origin: UserRootOrigin::Default,
    }
}

#[test]
fn winget_marker_spellings_use_user_root_at_local_app_data() {
    for marker in [MARKER_LOCAL, MARKER_PKGS] {
        let d = decide(&signals(
            None,
            Some(LAD),
            Some(marker),
            Writability::Writable,
        ));
        assert_eq!(
            d,
            Decision::UserRoot {
                root: default_root(),
                reason: Reason::WingetInstall {
                    marker: marker.to_owned()
                },
            }
        );
    }
}

#[test]
fn denied_exe_dir_uses_user_root() {
    let d = decide(&signals(None, Some(LAD), None, Writability::Denied));
    assert_eq!(
        d,
        Decision::UserRoot {
            root: default_root(),
            reason: Reason::Unwritable,
        }
    );
}

#[test]
fn env_var_wins_even_when_marker_and_denied_overlap() {
    let env = r"C:\work\target\ur";
    let d = decide(&signals(
        Some(env),
        Some(LAD),
        Some(MARKER_LOCAL),
        Writability::Denied,
    ));
    assert_eq!(
        d,
        Decision::UserRoot {
            root: UserRoot {
                dir: PathBuf::from(env),
                origin: UserRootOrigin::EnvVar,
            },
            reason: Reason::EnvVar,
        }
    );
}

#[test]
fn neither_stays_beside() {
    let d = decide(&signals(None, Some(LAD), None, Writability::Writable));
    assert_eq!(
        d,
        Decision::Beside {
            reason: Reason::Neither
        }
    );
}

#[test]
fn unknown_writability_stays_beside() {
    let d = decide(&signals(
        None,
        Some(LAD),
        None,
        Writability::Unknown("sharing violation".to_owned()),
    ));
    assert_eq!(
        d,
        Decision::Beside {
            reason: Reason::Neither
        }
    );
}

#[test]
fn env_empty_or_relative_falls_back() {
    let d = decide(&signals(Some(""), Some(LAD), None, Writability::Writable));
    assert_eq!(
        d,
        Decision::Fallback {
            reason: Reason::EnvVar,
            error: Unlocatable::EnvEmpty,
        }
    );
    let d = decide(&signals(
        Some(r"rel\ur"),
        Some(LAD),
        None,
        Writability::Writable,
    ));
    assert_eq!(
        d,
        Decision::Fallback {
            reason: Reason::EnvVar,
            error: Unlocatable::EnvRelative {
                value: PathBuf::from(r"rel\ur")
            },
        }
    );
}

#[test]
fn local_app_data_missing_empty_or_relative_falls_back() {
    let d = decide(&signals(None, None, None, Writability::Denied));
    assert_eq!(
        d,
        Decision::Fallback {
            reason: Reason::Unwritable,
            error: Unlocatable::LocalAppDataMissing,
        }
    );
    let d = decide(&signals(
        None,
        Some(""),
        Some(MARKER_LOCAL),
        Writability::Writable,
    ));
    assert_eq!(
        d,
        Decision::Fallback {
            reason: Reason::WingetInstall {
                marker: MARKER_LOCAL.to_owned()
            },
            error: Unlocatable::LocalAppDataMissing,
        }
    );
    let d = decide(&signals(
        None,
        Some(r"AppData\Local"),
        None,
        Writability::Denied,
    ));
    assert_eq!(
        d,
        Decision::Fallback {
            reason: Reason::Unwritable,
            error: Unlocatable::LocalAppDataRelative {
                value: PathBuf::from(r"AppData\Local")
            },
        }
    );
}

fn user_root_layout() -> Layout {
    Layout {
        user_root: Some(default_root()),
        reason: Reason::Unwritable,
    }
}

fn beside_layout() -> Layout {
    Layout {
        user_root: None,
        reason: Reason::Neither,
    }
}

#[test]
fn profile_env_wins_even_with_user_root() {
    let env = OsString::from(r"C:\work\target\prof");
    let got = app_profile_dir(Some(&env), &user_root_layout(), Some(Path::new(r"C:\app")));
    assert_eq!(got, Some(PathBuf::from(r"C:\work\target\prof")));
}

#[test]
fn profile_under_user_root() {
    let got = app_profile_dir(None, &user_root_layout(), Some(Path::new(r"C:\app")));
    assert_eq!(
        got,
        Some(Path::new(LAD).join("areka").join("profile").join("areka"))
    );
}

#[test]
fn profile_beside_exe() {
    let got = app_profile_dir(None, &beside_layout(), Some(Path::new(r"C:\app")));
    assert_eq!(got, Some(PathBuf::from(r"C:\app\profile\areka")));
}

#[test]
fn profile_none_without_exe_dir() {
    assert_eq!(app_profile_dir(None, &beside_layout(), None), None);
}
