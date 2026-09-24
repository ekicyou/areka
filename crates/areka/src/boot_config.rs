//! 起動時の構成入力解決と ghost 結線ヘルパ（`main.rs` から切り出し）。
//!
//! `main.rs` が 1,000 行規約（`.kiro/steering/structure.md`）を超えたため、
//! 相互に凝集した「構成入力（根 → ゴースト → バルーン）の起動前の解決」と「`GhostBootOptions` の組み立て」を
//! 本モジュールに置く。根の決め方（`resolve_root_from`）は areka-P0-baseware-root-layout が
//! `CARGO_MANIFEST_DIR` 相対の既定パスと置き換えた（要件 1.5）。
//!
//! 消費者は `main.rs`（`pub(crate) use` で crate 直下へ再輸出）と `emo2_boot`
//! （`crate::default_app_profile_dir` / `crate::is_benign_boot_error`）、および
//! 檻 `main_config_input_tests.rs` / `main_ghost_wiring_tests.rs`。

// ---------------------------------------------------------------------------
// Config Inputs (task 2.1)
// ---------------------------------------------------------------------------

/// 構成入力（解決済みルートパス）。
///
/// ゴースト／バルーンのルートパスを保持する。値は `main` の起動前の解決（根 → ゴースト →
/// バルーン）が決めたもので、この型自身はマウントも読取もしない（R6.1）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConfigInputs {
    pub(crate) ghost_root: std::path::PathBuf,
    pub(crate) balloon_root: std::path::PathBuf,
}

// ---------------------------------------------------------------------------
// ベースウェアの根（areka-P0-baseware-root-layout task 2.3）
// ---------------------------------------------------------------------------
// 消費者は下の起動前の解決（`resolve_boot`）。

/// 根が決まらない理由（利用者向けの告知と `error!` の両方に載せる・要件 1.4）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RootError {
    /// `AREKA_ROOT` が無く、`current_exe()` の場所が取れない。
    ExeLocationUnavailable,
    /// 決まった根が実在しない（フォルダでない）。`source` は `AREKA_ROOT` か exe の隣か。
    NotADirectory {
        dir: std::path::PathBuf,
        source: RootSource,
    },
}

/// 根の出所。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RootSource {
    /// 環境変数 `AREKA_ROOT`（要件 1.3）。
    EnvVar,
    /// 実行ファイルのあるフォルダ（要件 1.2・裁定 1＝要件 9.1）。
    ExeDir,
}

/// 根を決めて実在を検査する純粋な判断（env もプロセス引数も読まない・要件 8.2）。
///
/// - `env`: `AREKA_ROOT` の値（`None`＝未設定）。あれば `exe` は見ない（要件 1.3）。
///   空の値は `AREKA_PROFILE_DIR` と同じく「設定あり」として扱い、exe の隣へは倒さない
///   （空は絶対化できないので `NotADirectory { dir: "" }` になる＝黙らず告知へ届く）。
/// - `exe`: `current_exe()` の結果（実行ファイルそのもののパス・`None`＝失敗）。根はその親。
///   env も exe も無ければ `ExeLocationUnavailable`（`"."` へ倒さない・要件 1.4）。
///
/// `Ok` のパスは絶対で `is_dir()` が真。相対の値はカレント基準で `std::path::absolute` により
/// 絶対化してから検査する（`canonicalize` は使わない＝長いパスの接頭辞と失敗の口を持ち込まない）。
/// `NotADirectory` の `dir` も絶対（絶対化できない空の値だけは元の綴り）。
pub(crate) fn resolve_root_from(
    env: Option<std::path::PathBuf>,
    exe: Option<std::path::PathBuf>,
) -> Result<(std::path::PathBuf, RootSource), RootError> {
    let (dir, source) = match env {
        Some(dir) => (dir, RootSource::EnvVar),
        None => {
            let dir = exe
                .as_deref()
                .and_then(std::path::Path::parent)
                .filter(|dir| !dir.as_os_str().is_empty())
                .ok_or(RootError::ExeLocationUnavailable)?;
            (dir.to_path_buf(), RootSource::ExeDir)
        }
    };
    let dir = std::path::absolute(&dir).unwrap_or(dir);
    if dir.is_dir() {
        Ok((dir, source))
    } else {
        Err(RootError::NotADirectory { dir, source })
    }
}

/// env（`AREKA_ROOT`）と `current_exe()` を読んで [`resolve_root_from`] へ渡す薄い口。
pub(crate) fn resolve_root() -> Result<(std::path::PathBuf, RootSource), RootError> {
    resolve_root_from(
        std::env::var_os("AREKA_ROOT").map(std::path::PathBuf::from),
        std::env::current_exe().ok(),
    )
}

// ---------------------------------------------------------------------------
// 起動前の解決（areka-P0-baseware-root-layout task 5.1・`main` が WinApp 構築の前に呼ぶ）
// ---------------------------------------------------------------------------

/// 起動前に決まったもの: 構成入力と、ゴースト・バルーンの決定（経路とフォルダ）。
pub(crate) type BootResolved = (
    ConfigInputs,
    crate::boot_resolve::GhostDecision,
    crate::boot_resolve::BalloonDecision,
);

/// env（`AREKA_ROOT`・`AREKA_PROFILE_DIR`）と `current_exe()` を読んで [`resolve_boot_from`] へ渡す薄い口。
pub(crate) fn resolve_boot(args: &[String]) -> Result<BootResolved, crate::alert::AlertScene> {
    resolve_boot_from(
        resolve_root(),
        args,
        &default_app_profile_dir(),
        crate::boot_resolve::pick_index,
    )
}

/// 根 → ゴースト → バルーン → `ConfigInputs`（design「起動解決」）。決まらなければ告知の場面を返す。
///
/// argv がある側は列挙も記憶も読まない（要件 4.1・5.1）。argv のゴーストは「ゴーストか」の 1 検査
/// だけ（要件 4.8）。決まるたびに経路と場所を info に残す（要件 4.10・5.11）。
pub(crate) fn resolve_boot_from(
    root: Result<(std::path::PathBuf, RootSource), RootError>,
    args: &[String],
    app_profile_dir: &std::path::Path,
    pick: fn(usize) -> usize,
) -> Result<BootResolved, crate::alert::AlertScene> {
    use crate::alert::AlertScene;
    use crate::boot_resolve::{self, BalloonInputs, GhostInputs, NoBalloon, NoGhost};
    use areka_ghost::catalog;

    let (dir, source) = root.map_err(AlertScene::RootMissing)?;
    tracing::info!(event = "root_resolved", root = %dir.display(), source = ?source, "ベースウェアの根を決めました");
    let root = areka_ghost::BasewareRoot::new(dir);
    let (argv_ghost, argv_balloon) = (
        args.get(1).map(std::path::Path::new),
        args.get(2).map(std::path::Path::new),
    );

    if let Some(argv) = argv_ghost.filter(|argv| !catalog::is_ghost_dir(argv)) {
        return Err(AlertScene::GhostMissing {
            ghost_store: root.ghost_store(),
            argv: Some(argv.to_path_buf()),
        });
    }
    let (memory, listed): (_, Vec<String>) = match argv_ghost {
        Some(_) => (None, Vec::new()),
        None => (
            boot_resolve::read_last_ghost(app_profile_dir),
            catalog::list_ghosts(&root)
                .into_iter()
                .map(|e| e.identity.folder)
                .collect(),
        ),
    };
    let ghost = boot_resolve::resolve_ghost(
        &GhostInputs {
            root: &root,
            argv: argv_ghost,
            memory: memory.as_deref(),
            listed: &listed,
        },
        pick,
    )
    .map_err(|NoGhost { ghost_store }| AlertScene::GhostMissing {
        ghost_store,
        argv: None,
    })?;
    tracing::info!(event = "ghost_resolved", route = ?ghost.route, dir = %ghost.dir.display(), "起動するゴーストを決めました");

    let (memory, companion, listed): (_, _, Vec<String>) = match argv_balloon {
        Some(_) => (None, None, Vec::new()),
        None => (
            boot_resolve::read_last_balloon(&ghost.dir),
            catalog::companion_balloon(&ghost.dir),
            catalog::list_balloons(&root)
                .into_iter()
                .map(|e| e.identity.folder)
                .collect(),
        ),
    };
    let balloon = boot_resolve::resolve_balloon(
        &BalloonInputs {
            root: &root,
            argv: argv_balloon,
            memory: memory.as_deref(),
            companion: companion.as_deref(),
            listed: &listed,
        },
        pick,
    )
    .map_err(|NoBalloon { balloon_store }| AlertScene::BalloonMissing { balloon_store })?;
    tracing::info!(event = "balloon_resolved", route = ?balloon.route, dir = %balloon.dir.display(), "バルーンを決めました");

    let cfg = ConfigInputs {
        ghost_root: ghost.dir.clone(),
        balloon_root: balloon.dir.clone(),
    };
    Ok((cfg, ghost, balloon))
}

// ---------------------------------------------------------------------------
// Ghost Wiring (task 3.3)
// ---------------------------------------------------------------------------

/// 実行ファイル隣接の 32bit SHIORI helper 実行ファイルパスを解決する（純粋・DD 準拠）。
///
/// `std::env::current_exe()` の親ディレクトリへ `shiori-host32-helper.exe` を結合する。
/// `current_exe()` が失敗した場合（環境依存の稀な事象）は、この骨格の既存の寛容な
/// （panic しない）流儀に倣い `"."` を親ディレクトリ扱いにフォールバックする——`boot` 呼び出し
/// 自体はどのみち非致命として扱われるため、ここで panic/Err 伝播する必要はない。
pub(crate) fn default_helper_exe_path() -> std::path::PathBuf {
    let dir = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(std::path::Path::to_path_buf))
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    dir.join("shiori-host32-helper.exe")
}

/// App スコープの sylphya profile root を解決する（task 8.2・R8.2 の `AREKA_` 冠準拠）。
///
/// - 環境変数 `AREKA_PROFILE_DIR` が設定されていればそのパスを採用する（本番 env は `AREKA_`
///   名前空間・記憶 areka-runtime-env-naming）。
/// - 未設定なら実行ファイル隣接の `profile/areka/`（`current_exe()` の親ディレクトリ／`current_exe()`
///   失敗時は `"."` へ寛容フォールバック——boot 呼び出し自体が非致命ゆえ panic/Err 伝播は不要）。
///
/// App スコープはマウント解決に現れない（ghost/shell スコープは `<shiori.dir>`／`<shell.dir>` から
/// ghost が導く）ため、bin が本関数で供給して `GhostBootOptions.app_profile_dir` へ渡す。
pub(crate) fn default_app_profile_dir() -> std::path::PathBuf {
    if let Some(dir) = std::env::var_os("AREKA_PROFILE_DIR") {
        return std::path::PathBuf::from(dir);
    }
    let base = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(std::path::Path::to_path_buf))
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    base.join("profile").join("areka")
}

/// `ghost_root`／helper パスから `GhostBootOptions` を組み立てる純粋ヘルパ
/// （design.md「main の ghost boot／shutdown 結線」）。
///
/// - `shiori`: `ShioriWiring::Helper { helper_exe }`（実行ファイル隣接の 32bit helper・本番結線）。
/// - `default_encoding`: `DefaultEncoding::Ansi`（charset 未宣言時の SSP 既定・記憶
///   areka-descript-encoding-ishiori-utf8）。
/// - `sinks`: 可変長 sink 列（S-3）を `vec![LogSink, DiscardSink]` で埋める。broadcast（D4）で
///   全 cue は登録された全 sink へ配られるため、両スロットを `LogSink` にすると 1 cue が 2 回ログ
///   される（二重ログ）。記録 sink を **1 本（`LogSink`）だけ**にし、もう一方を破棄専用の
///   `DiscardSink` で埋めることで cue ごと 1 回ログへ正す（設計 D4 Topic 2）。
/// - `system_vars`: 本番 provider（`SystemVarWiring::FromSylphya`＝boot が据えた sylphya reader
///   由来のスナップショット・R7.1）。
/// - `app_profile_dir`: App スコープの sylphya profile root（`default_app_profile_dir()`＝env
///   `AREKA_PROFILE_DIR` 優先・既定は実行ファイル隣接 `profile/areka/`・R8.2）。
/// - `ticker`: `TickerMode::Real` を既定 `TickerConfig`（`base_interval=50ms`／
///   `kanade_interval=1000ms`／実クロック `GetTickCount64`）で駆動する。
///
/// `app_profile_dir` の解決は env（`AREKA_PROFILE_DIR`）・`current_exe()` を読むため厳密には純粋
/// ではない（副作用のない read のみ）。他フィールドの決定は従来どおり引数からの写しに留まる。
pub(crate) fn ghost_boot_options(
    ghost_root: std::path::PathBuf,
    helper_exe: std::path::PathBuf,
) -> areka_ghost::GhostBootOptions {
    areka_ghost::GhostBootOptions {
        ghost_root,
        default_encoding: areka_parsers::charset::DefaultEncoding::Ansi,
        shiori: areka_ghost::ShioriWiring::Helper { helper_exe },
        sinks: vec![
            Box::new(areka_ghost::sink::LogSink::new()),
            Box::new(areka_ghost::sink::DiscardSink::new()),
        ],
        system_vars: areka_ghost::SystemVarWiring::FromSylphya,
        app_profile_dir: Some(default_app_profile_dir()),
        ticker: areka_ghost::TickerMode::Real(Default::default()),
    }
}

/// `GhostBootError` を「起点不在（良性・`warn!` どまり）」と「それ以外（予期しない・`error!`）」
/// へ分類する純粋関数（design.md「main の ghost boot／shutdown 結線」・要件 8.2）。
///
/// 起動前の解決（`boot_config::resolve_boot`）が `ghost/master/descript.txt` の実在を確かめてから
/// boot するので、ここでの `MountError::StartPointMissing` は解決後の消失（起動中の削除等）に
/// 限られる。分類は `wire_emo2_boot` のフォールバックが使い続けるため残す。読取不能
/// （`StartPointUnreadable`）・shell 不在（`ShellDirMissing`）・将来追加される
/// `#[non_exhaustive]` variant は、真に予期しない I/O 問題として区別する。
///
/// `pub(crate)`: `emo2_boot::wire_emo2_boot`（task 5.1）が boot 失敗（`GhostBootError`）を
/// 同一方針（起点不在＝良性 `warn!`・他＝`error!`・R7.4）で分類するため再利用する。
pub(crate) fn is_benign_boot_error(err: &areka_ghost::GhostBootError) -> bool {
    match err {
        areka_ghost::GhostBootError::Mount(
            areka_parsers::package::MountError::StartPointMissing { .. },
        ) => true,
        _ => false,
    }
}
