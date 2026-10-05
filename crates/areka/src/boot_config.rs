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
// 起動した exe の本当の場所（areka-P0-release-package-versioned task 4.1）
// ---------------------------------------------------------------------------
// 純粋な判断 `follow_exe_links` と、実 I/O の口 `probe_link`・1 回だけ解いて覚える口 `exe_location`
// （task 4.2）。根・補助 exe・記憶の置き場の 3 関数が `exe_location` を使う。

/// リンクを 1 段読んだ結果。I/O は呼び手（`probe_link`）か、テストの偽の口が返す。
pub(crate) enum LinkProbe {
    /// リンクではない（普通のファイル）。
    NotALink,
    /// リンクで、先はこのパス（`read_link` の綴りのまま・絶対でも相対でもよい）。
    Target(std::path::PathBuf),
    /// リンクかどうか、または先を読めなかった（理由の文）。
    Unreadable(String),
}

/// 解けなかった理由（警告の行に載せる）。どの形でも起動した exe のパスをそのまま使う（要件 6.6）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ExeLinkWarning {
    /// `link` がリンクかどうか、または先を読めなかった（`reason` は読めなかった理由の文）。
    Unreadable {
        link: std::path::PathBuf,
        reason: String,
    },
    /// 辿った回数が `limit` に達した（輪になったリンク）。`last` は最後に読んだ先。
    TooManyHops {
        limit: usize,
        last: std::path::PathBuf,
    },
    /// たどり着いたパスに長いパスの接頭辞（`\\?\`）が付いている（要件 6.5）。
    VerbatimPrefix { path: std::path::PathBuf },
}

/// 辿る回数の上限（輪になったリンクで止まるため・Windows の再解析の上限 63 より小さい値）。
pub(crate) const EXE_LINK_MAX_HOPS: usize = 32;

/// 起動した exe のパス `exe` からリンクを辿って、根・補助 exe・記憶の置き場に使うパスを決める
/// 純粋な判断（要件 6.1〜6.3・6.5・6.7）。
///
/// - `probe(path)` が `NotALink` → そのパスを使う（リンクを経ていなければ `exe` の綴りそのまま・
///   ドライブ文字や `subst` の綴りを書き換えない＝要件 6.3）。ただし `\\?\` の接頭辞が付いていれば
///   `VerbatimPrefix`。
/// - `Target(t)` → `t` が絶対ならそれ、相対ならリンクの親と結合して `std::path::absolute` で
///   `.`・`..` を畳み、次を読む（要件 6.2）。`absolute` が失敗すれば `Unreadable`。
/// - `Target` を `EXE_LINK_MAX_HOPS` 回受けたら次を読まずに `TooManyHops`。
/// - `Unreadable(r)` → `Unreadable { link, reason: r }`。
///
/// I/O は `probe` だけ（`absolute` は文字列の操作）。`canonicalize` は使わない（要件 6.5）。
/// 戻りの第 2 要素が `Some` のときは第 1 要素は `exe` そのもの（今のふるまいへ戻る・要件 6.6）。
pub(crate) fn follow_exe_links(
    exe: &std::path::Path,
    probe: &mut dyn FnMut(&std::path::Path) -> LinkProbe,
) -> (std::path::PathBuf, Option<ExeLinkWarning>) {
    let fallback = |warning| (exe.to_path_buf(), Some(warning));
    let mut path = exe.to_path_buf();
    let mut hops = 0;
    loop {
        match probe(&path) {
            LinkProbe::NotALink => {
                let verbatim = matches!(
                    path.components().next(),
                    Some(std::path::Component::Prefix(p)) if p.kind().is_verbatim()
                );
                return if verbatim {
                    fallback(ExeLinkWarning::VerbatimPrefix { path })
                } else {
                    (path, None)
                };
            }
            LinkProbe::Unreadable(reason) => {
                return fallback(ExeLinkWarning::Unreadable { link: path, reason });
            }
            LinkProbe::Target(target) => {
                hops += 1;
                let next = if target.is_absolute() {
                    target
                } else {
                    let joined = path
                        .parent()
                        .unwrap_or(std::path::Path::new(""))
                        .join(&target);
                    match std::path::absolute(&joined) {
                        Ok(next) => next,
                        Err(e) => {
                            return fallback(ExeLinkWarning::Unreadable {
                                link: path,
                                reason: e.to_string(),
                            });
                        }
                    }
                };
                if hops >= EXE_LINK_MAX_HOPS {
                    return fallback(ExeLinkWarning::TooManyHops {
                        limit: EXE_LINK_MAX_HOPS,
                        last: next,
                    });
                }
                path = next;
            }
        }
    }
}

/// `std::fs::symlink_metadata` と `std::fs::read_link` で 1 段読む本物の口（配線・テストは踏まない）。
fn probe_link(path: &std::path::Path) -> LinkProbe {
    match std::fs::symlink_metadata(path) {
        Err(e) => LinkProbe::Unreadable(e.to_string()),
        Ok(meta) if !meta.file_type().is_symlink() => LinkProbe::NotALink,
        Ok(_) => match std::fs::read_link(path) {
            Ok(target) => LinkProbe::Target(target),
            Err(e) => LinkProbe::Unreadable(e.to_string()),
        },
    }
}

/// 起動した exe の本当の場所（プロセスで 1 回だけ解いて覚える）。`current_exe()` が失敗したときは `None`。
///
/// 解けなければ（`follow_exe_links` の警告が `Some`）初回に 1 度だけ `exe_link_unresolved` を
/// `warn!` に出し、起動した exe のパスをそのまま使う（要件 6.6）。覚えるのは、何度も呼ばれる
/// `default_app_profile_dir` で警告が繰り返されず、途中でリンクが張り替えられても置き場が変わらないため。
pub(crate) fn exe_location() -> Option<&'static std::path::Path> {
    static LOC: std::sync::OnceLock<Option<std::path::PathBuf>> = std::sync::OnceLock::new();
    LOC.get_or_init(|| {
        let exe = std::env::current_exe().ok()?;
        let (path, warning) = follow_exe_links(&exe, &mut probe_link);
        if let Some(warning) = warning {
            tracing::warn!(event = "exe_link_unresolved", exe = %exe.display(), reason = ?warning, "起動した exe のリンクの先を解けなかったので、起動した exe のパスをそのまま使います");
        }
        Some(path)
    })
    .as_deref()
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
/// - `exe`: 起動した exe の本当の場所（`exe_location` の結果・実行ファイルそのもののパス・`None`＝失敗）。根はその親。
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

/// env（`AREKA_ROOT`）と起動した exe の本当の場所（`exe_location`）を読んで [`resolve_root_from`] へ渡す薄い口。
pub(crate) fn resolve_root() -> Result<(std::path::PathBuf, RootSource), RootError> {
    resolve_root_from(
        std::env::var_os("AREKA_ROOT").map(std::path::PathBuf::from),
        exe_location().map(std::path::Path::to_path_buf),
    )
}

// ---------------------------------------------------------------------------
// 起動前の解決（areka-P0-baseware-root-layout task 5.1・`main` が WinApp 構築の前に呼ぶ）
// ---------------------------------------------------------------------------

/// 起動前に決まったもの: 構成入力と、ゴースト・バルーンの決定（経路とフォルダ）と、
/// 前回落ちたゴーストの名前（起動中の印の値・`read_session_mark`＝読むだけで消さない・要件 12.4）と、
/// 根（起動の文脈へ渡す）。
pub(crate) type BootResolved = (
    ConfigInputs,
    crate::boot_resolve::GhostDecision,
    crate::boot_resolve::BalloonDecision,
    Option<String>,
    areka_ghost::BasewareRoot,
);

/// 起動の文脈（プロセスに 1 つ）: 根・記憶の置き場・helper のパス・argv で始まったプロセスか・今のゴースト。
/// 目録とバルーンの解決に要る根を切替の経路へ渡す（切替が成功したら `current` を更新する）。
/// 据え付けは `fn main`（系の登録の直後）。
#[derive(bevy_ecs::prelude::Resource)]
pub(crate) struct BootContext {
    pub root: areka_ghost::BasewareRoot,
    pub app_profile_dir: std::path::PathBuf,
    pub helper_exe: std::path::PathBuf,
    /// argv でゴーストを指定して始まったプロセスか（要件 12.5）。`fn main` が据え付けで 1 度だけ詰め、
    /// 以後変えない（切替の後の `current.ghost.route` では判断しない）。真なら起動中の印を読まず
    /// 書かず消さない。
    pub argv_session: bool,
    pub current: CurrentGhost,
}

/// 今のゴースト（構成入力・ゴーストの決定・バルーンの決定）。
pub(crate) struct CurrentGhost {
    pub cfg: ConfigInputs,
    pub ghost: crate::boot_resolve::GhostDecision,
    /// 読み手は切替の定常到達の記憶（`ghost_switch::record_steady_memory`）と、後続の
    /// `shell-balloon-switch`（今のバルーンを入れ替える口）。
    pub balloon: crate::boot_resolve::BalloonDecision,
}

/// env（`AREKA_ROOT`・`AREKA_PROFILE_DIR`）と起動した exe の本当の場所（`exe_location`）を読んで [`resolve_boot_from`] へ渡す薄い口。
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
    // 起動中の印（要件 12.4・12.5）: argv でゴーストを指定した起動（開発者の上書き）は読まない。
    // 在れば前回はきれいに終わらなかったので、最後のゴーストの記憶を読まずに残りの段
    // （唯一 → 既定 → 無作為）で解く。印は読むだけで消さない（起こす前の書き込みが上書きする）。
    let mark = match argv_ghost {
        Some(_) => None,
        None => boot_resolve::read_session_mark(app_profile_dir),
    };
    if let Some(ghost) = &mark {
        tracing::info!(
            event = "session_mark_found",
            ghost = %ghost,
            "前回はきれいに終わらなかったので、最後のゴーストの記憶を読まずに起動するゴーストを決めます"
        );
    }
    let (memory, listed): (_, Vec<String>) = match argv_ghost {
        Some(_) => (None, Vec::new()),
        None => (
            mark.is_none()
                .then(|| boot_resolve::read_last_ghost(app_profile_dir))
                .flatten(),
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

    let balloon = match argv_balloon {
        Some(argv) => boot_resolve::resolve_balloon(
            &BalloonInputs {
                root: &root,
                argv: Some(argv),
                memory: None,
                default_balloon_path: None,
                balloon_name: None,
                companion: None,
                listed: &[],
            },
            pick,
        ),
        None => resolve_balloon_for_ghost(&root, &ghost.dir, pick),
    }
    .map_err(|NoBalloon { balloon_store }| AlertScene::BalloonMissing { balloon_store })?;
    tracing::info!(event = "balloon_resolved", route = ?balloon.route, dir = %balloon.dir.display(), "バルーンを決めました");

    let cfg = ConfigInputs {
        ghost_root: ghost.dir.clone(),
        balloon_root: balloon.dir.clone(),
    };
    Ok((cfg, ghost, balloon, mark, root))
}

/// argv 無しの分岐（そのゴーストの最後のバルーンの記憶 → 同梱 → 唯一 → 既定 → 無作為）で
/// 1 ゴースト分のバルーンを解く（起動前の解決の後半・切替先のバルーンの解決＝要件 4.7）。
/// 初回起動の argv の第 2 引数はここへ届かない。
pub(crate) fn resolve_balloon_for_ghost(
    root: &areka_ghost::BasewareRoot,
    ghost_dir: &std::path::Path,
    pick: fn(usize) -> usize,
) -> Result<crate::boot_resolve::BalloonDecision, crate::boot_resolve::NoBalloon> {
    use crate::boot_resolve::{self, BalloonInputs};
    use areka_ghost::catalog;

    let memory = boot_resolve::read_last_balloon(ghost_dir);
    let companion = catalog::companion_balloon(ghost_dir);
    let listed = catalog::list_balloons(root);
    boot_resolve::resolve_balloon(
        &BalloonInputs {
            root,
            argv: None,
            memory: memory.as_deref(),
            default_balloon_path: None,
            balloon_name: None,
            companion: companion.as_deref(),
            listed: &listed,
        },
        pick,
    )
}

// ---------------------------------------------------------------------------
// Ghost Wiring (task 3.3)
// ---------------------------------------------------------------------------

/// 実行ファイル隣接の 32bit SHIORI helper 実行ファイルパスを解決する（純粋・DD 準拠）。
///
/// 起動した exe の本当の場所（`exe_location`）の親ディレクトリへ `shiori-host32-helper.exe` を結合する。
/// `current_exe()` が失敗した場合（環境依存の稀な事象）は、この骨格の既存の寛容な
/// （panic しない）流儀に倣い `"."` を親ディレクトリ扱いにフォールバックする——`boot` 呼び出し
/// 自体はどのみち非致命として扱われるため、ここで panic/Err 伝播する必要はない。
pub(crate) fn default_helper_exe_path() -> std::path::PathBuf {
    let dir = exe_location()
        .and_then(std::path::Path::parent)
        .unwrap_or(std::path::Path::new("."));
    dir.join("shiori-host32-helper.exe")
}

/// App スコープの sylphya profile root を解決する（task 8.2・R8.2 の `AREKA_` 冠準拠）。
///
/// - 環境変数 `AREKA_PROFILE_DIR` が設定されていればそのパスを採用する（本番 env は `AREKA_`
///   名前空間・記憶 areka-runtime-env-naming）。
/// - 未設定なら実行ファイル隣接の `profile/areka/`（起動した exe の本当の場所（`exe_location`）の親ディレクトリ／`current_exe()`
///   失敗時は `"."` へ寛容フォールバック——boot 呼び出し自体が非致命ゆえ panic/Err 伝播は不要）。
///
/// App スコープはマウント解決に現れない（ghost/shell スコープは `<shiori.dir>`／`<shell.dir>` から
/// ghost が導く）ため、bin が本関数で供給して `GhostBootOptions.app_profile_dir` へ渡す。
pub(crate) fn default_app_profile_dir() -> std::path::PathBuf {
    if let Some(dir) = std::env::var_os("AREKA_PROFILE_DIR") {
        return std::path::PathBuf::from(dir);
    }
    let base = exe_location()
        .and_then(std::path::Path::parent)
        .unwrap_or(std::path::Path::new("."));
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
/// `app_profile_dir` の解決は env（`AREKA_PROFILE_DIR`）・起動した exe の本当の場所（`exe_location`）を読むため厳密には純粋
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

#[cfg(test)]
#[path = "boot_config_exe_link_tests.rs"]
mod exe_link_tests;
