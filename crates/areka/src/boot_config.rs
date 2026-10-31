//! 起動時の構成入力解決と ghost 結線ヘルパ（`main.rs` から切り出し）。
//!
//! `main.rs` が 1,000 行規約（`.kiro/steering/structure.md`）を超えたため、
//! 相互に凝集した「構成入力（根 → ゴースト → バルーン）の起動前の解決」と「`GhostBootOptions` の組み立て」を
//! 本モジュールに置く。根の並びの決め方は `crate::roots`（areka-P0-baseware-root-list）。
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
// 起動前の解決（areka-P0-baseware-root-layout task 5.1・`main` が WinApp 構築の前に呼ぶ）
// ---------------------------------------------------------------------------

/// 起動前に決まったもの: 構成入力と、ゴースト・バルーンの決定（経路とフォルダ）と、
/// 前回落ちたゴーストの名前（起動中の印の値・`read_session_mark`＝読むだけで消さない・要件 12.4）と、
/// 根の並び（起動の文脈へ渡す）。
pub(crate) type BootResolved = (
    ConfigInputs,
    crate::boot_resolve::GhostDecision,
    crate::boot_resolve::BalloonDecision,
    Option<String>,
    crate::roots::ResolvedRoots,
);

/// 起動の文脈（プロセスに 1 つ）: 根の並びとその出どころ・記憶の置き場・helper のパス・argv で始まったプロセスか・今のゴースト。
/// 目録とバルーンの解決に要る根を切替の経路へ渡す（切替が成功したら `current` を更新する）。
/// 据え付けは `fn main`（系の登録の直後）。
#[derive(bevy_ecs::prelude::Resource)]
pub(crate) struct BootContext {
    pub roots: areka_ghost::BasewareRoots,
    /// 並びの出どころ。読み手はメニューの置き場の項目と並びの変更（`roots::add_root`・`remove_root`）。
    pub roots_origin: crate::roots::RootsOrigin,
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

/// env（`AREKA_ROOT`・`AREKA_PROFILE_DIR`）・アプリの記憶・起動した exe の本当の場所を読んで [`resolve_boot_with`] へ渡す薄い口。
pub(crate) fn resolve_boot(
    request: &crate::cli::BootRequest,
) -> Result<BootResolved, crate::alert::AlertScene> {
    let app_profile_dir = default_app_profile_dir();
    resolve_boot_with(
        crate::roots::resolve_roots(&app_profile_dir),
        request,
        None,
        &app_profile_dir,
        crate::boot_resolve::pick_index,
        crate::roots::user_root::writable_ghost,
    )
}

/// テストの口: 引数の列を読み方（`cli::parse`）に通し、起動の求めで [`resolve_boot_with`] を呼ぶ。
#[cfg(test)]
pub(crate) fn resolve_boot_from(
    resolved: Result<crate::roots::ResolvedRoots, crate::roots::RootError>,
    args: &[String],
    app_profile_dir: &std::path::Path,
    pick: fn(usize) -> usize,
    prepare_ghost: fn(&areka_ghost::BasewareRoots, &std::path::Path) -> std::path::PathBuf,
) -> Result<BootResolved, crate::alert::AlertScene> {
    resolve_boot_with(
        resolved,
        &crate::cli::parse::parse(args).options.boot,
        None,
        app_profile_dir,
        pick,
        prepare_ghost,
    )
}

/// 根の並び → ゴースト → バルーン → `ConfigInputs`（design「起動解決」）。決まらなければ告知の場面を返す。
/// 並びは決まっても決まらなくても先に記録を残す（要件 1.15・2.5）。
///
/// 位置の引数のゴーストでも絶対パスの指し先でもないゴースト（前回きれいに終わらなかったときの既定も）は
/// `prepare_ghost`（本番は `user_root::writable_ghost`）の答えのフォルダへ替え、バルーンと
/// `ConfigInputs` もそのフォルダで決める（user-data-root 要件 4.3・7.4）。表示するだけのバルーンは写さない（要件 4.5）。
///
/// 位置の引数のゴーストがある側は列挙も記憶も読まない（要件 4.1・5.1）。位置の引数のゴーストは
/// 「ゴーストか」の 1 検査だけ（要件 4.8）。起動中の印は位置の引数のゴーストが無く `nobootcheck` でない
/// ときに読む（startup-options 要件 6.2・6.8）。決まるたびに経路と場所を info に残す（要件 4.10・5.11）。
///
/// `call_ghost` は起こすゴーストのフォルダ名（起動の前に入れた道が決めたときだけ `Some`）。
/// `GhostInputs` へ渡すだけで、記憶より先・起動中の印に関わらず目録から引かれる（install-before-boot 要件 4.3・4.7）。
pub(crate) fn resolve_boot_with(
    resolved: Result<crate::roots::ResolvedRoots, crate::roots::RootError>,
    request: &crate::cli::BootRequest,
    call_ghost: Option<&str>,
    app_profile_dir: &std::path::Path,
    pick: fn(usize) -> usize,
    prepare_ghost: fn(&areka_ghost::BasewareRoots, &std::path::Path) -> std::path::PathBuf,
) -> Result<BootResolved, crate::alert::AlertScene> {
    use crate::alert::AlertScene;
    use crate::boot_resolve::{self, BalloonInputs, GhostInputs, NoBalloon, NoGhost};
    use crate::roots::report::{self, ReportOccasion};
    use areka_ghost::catalog;

    let mut resolved = resolved.map_err(|err| {
        report::log_root_error(&err, ReportOccasion::Boot);
        AlertScene::RootMissing(err)
    })?;
    // `--ghost` の指し先（3.1〜3.3・4.1〜4.4）。一時の置き場は並びと状態の先頭に同じ順で足し、
    // 以後の目録はこの並びで読む。
    let pointed = request
        .ghost
        .as_deref()
        .map_or(Pointed::Miss, |v| point_ghost(v, &resolved.roots));
    if let Pointed::Path {
        temporary: Some(dir),
        ..
    } = &pointed
    {
        let roots = std::iter::once(areka_ghost::BasewareRoot::temporary(dir.clone()))
            .chain(resolved.roots.roots().iter().cloned())
            .collect();
        resolved.roots = areka_ghost::BasewareRoots::new(roots);
        resolved.status.insert(
            0,
            crate::roots::RootStatus {
                dir: dir.clone(),
                found: true,
            },
        );
        tracing::info!(event = "temporary_root_added", dir = %dir.display(), "--ghost の親のフォルダを、この起動のあいだだけの置き場として並びの先頭に足しました");
    }
    let pointed_at = match &pointed {
        Pointed::Path { dir, folder, .. } | Pointed::Name { dir, folder } => {
            Some((dir.as_path(), folder.as_str()))
        }
        Pointed::Miss => None,
    };
    report::log_roots(&resolved, ReportOccasion::Boot);
    let ghost_stores = resolved.roots.ghost_stores();
    let (argv_ghost, argv_balloon) = (
        request.positional_ghost.as_deref(),
        request.positional_balloon.as_deref(),
    );

    // 隠れの記録は目録を読む。argv でゴーストを渡した起動は目録の外なので読まない（要件 11.2）。
    if argv_ghost.is_none() {
        report::log_hidden(
            &areka_ghost::catalog_roots::survey(&resolved.roots).hidden,
            ReportOccasion::Boot,
        );
    }
    if let Some(argv) = argv_ghost.filter(|argv| !catalog::is_ghost_dir(argv)) {
        return Err(AlertScene::GhostMissing {
            ghost_stores,
            argv: Some(argv.to_path_buf()),
        });
    }
    // 起動中の印（要件 12.4・12.5）: argv でゴーストを指定した起動（開発者の上書き）と `nobootcheck`
    // （startup-options 要件 6.2）は読まない。`--ghost`・`--random` でも読み、値は返す（要件 6.8）。在れば前回はきれいに終わらなかったので、最後のゴーストの記憶を読まずに残りの段
    // （唯一 → 既定 → 無作為）で解く。印は読むだけで消さない（起こす前の書き込みが上書きする）。
    let mark = (argv_ghost.is_none() && !request.no_boot_check)
        .then(|| boot_resolve::read_session_mark(app_profile_dir))
        .flatten();
    if let Some(ghost) = &mark {
        tracing::info!(
            event = "session_mark_found",
            ghost = %ghost,
            "前回はきれいに終わらなかったので、最後のゴーストの記憶を読まずに起動するゴーストを決めます"
        );
    }
    let (memory, listed) = match argv_ghost {
        Some(_) => (None, Vec::new()),
        None => (
            // `--random` と指し先は記憶を読まない（startup-options 要件 3.1・4.3・5.1）。
            (mark.is_none() && pointed_at.is_none() && !request.random)
                .then(|| boot_resolve::read_last_ghost(app_profile_dir))
                .flatten(),
            areka_ghost::catalog_roots::list_ghosts(&resolved.roots),
        ),
    };
    let mut ghost = boot_resolve::resolve_ghost(
        &GhostInputs {
            argv: argv_ghost,
            pointed: pointed_at,
            random: request.random,
            call_ghost,
            memory: memory.as_deref(),
            listed: &listed,
            stores: &ghost_stores,
        },
        pick,
    )
    .map_err(|NoGhost { ghost_stores }| AlertScene::GhostMissing {
        ghost_stores,
        argv: None,
    })?;
    // 書く前に写す（user-data-root 要件 4.3）: 位置の引数で渡したフォルダ（要件 10.3）と絶対パスの
    // 指し先（startup-options 要件 3.8）は写さない。名前の指し先は写す（要件 4.3）。
    let pointed_by_path = matches!(pointed, Pointed::Path { .. });
    if ghost.route != boot_resolve::GhostRoute::Argv && !pointed_by_path {
        ghost.dir = prepare_ghost(&resolved.roots, &ghost.dir);
    }
    // 指されたゴーストを起こすので `--random` は使わない（startup-options 要件 5.3）。
    if request.random
        && matches!(
            ghost.route,
            boot_resolve::GhostRoute::Argv | boot_resolve::GhostRoute::Pointed
        )
    {
        tracing::info!(event = "random_unused", route = ?ghost.route, "指されたゴーストを起こすので --random は使いません");
    }
    tracing::info!(event = "ghost_resolved", route = ?ghost.route, dir = %ghost.dir.display(), "起動するゴーストを決めました");

    let balloon = match argv_balloon {
        Some(argv) => boot_resolve::resolve_balloon(
            &BalloonInputs {
                argv: Some(argv),
                memory: None,
                default_balloon_path: None,
                balloon_name: None,
                companion: None,
                listed: &[],
                stores: &[],
            },
            pick,
        ),
        None => resolve_balloon_for_ghost(&resolved.roots, &ghost.dir, pick),
    }
    .map_err(|NoBalloon { balloon_stores }| AlertScene::BalloonMissing { balloon_stores })?;
    tracing::info!(event = "balloon_resolved", route = ?balloon.route, dir = %balloon.dir.display(), "バルーンを決めました");

    let cfg = ConfigInputs {
        ghost_root: ghost.dir.clone(),
        balloon_root: balloon.dir.clone(),
    };
    Ok((cfg, ghost, balloon, mark, resolved))
}

/// `--ghost` の値の指し先（design「BootResolve」）。
enum Pointed {
    /// 絶対パス（3.1）。`temporary` は先頭に足す一時の置き場（3.2）・None は 3.3。
    Path {
        dir: std::path::PathBuf,
        folder: String,
        temporary: Option<std::path::PathBuf>,
    },
    /// 名前（4.2）。
    Name {
        dir: std::path::PathBuf,
        folder: String,
    },
    /// 指せなかった・値が無い（3.10・4.4 の warn は `point_ghost` が出す）。
    Miss,
}

/// `--ghost` の値を指し先へ解く（fs を読む）。絶対パスは `ghost\master\descript.txt` を持つかを見て、
/// 親がどの置き場の `ghost\` とも同じでなければ一時の置き場にする（3.1〜3.3・3.10）。
/// 絶対パスでなければ切り替えと同じ名前の引き当て（`name` が先・フォルダ名が後）で並びのゴーストから探す（4.1〜4.4）。
fn point_ghost(value: &str, roots: &areka_ghost::BasewareRoots) -> Pointed {
    use crate::emo2_boot::ghost_switch::{GhostSpec, resolve_switch_target};
    let path = std::path::Path::new(value);
    if !path.is_absolute() {
        let listed = areka_ghost::catalog_roots::list_ghosts(roots);
        return match resolve_switch_target(&listed, &GhostSpec::Name(value.to_owned())) {
            Some(hit) => Pointed::Name {
                dir: hit.dir,
                folder: hit.folder,
            },
            None => {
                tracing::warn!(
                    event = "ghost_option_not_found",
                    value,
                    "--ghost の名前のゴーストが置き場に見つからないので、いつもの選び方で起こします"
                );
                Pointed::Miss
            }
        };
    }
    if !areka_ghost::catalog::is_ghost_dir(path) {
        tracing::warn!(event = "ghost_option_not_ghost_dir", dir = %path.display(), "--ghost のフォルダは ghost\\master\\descript.txt を持たないので、いつもの選び方で起こします");
        return Pointed::Miss;
    }
    let parent = path.parent().unwrap_or(path);
    let in_store = roots
        .ghost_stores()
        .iter()
        .any(|store| areka_ghost::catalog_roots::same_root(store, parent));
    Pointed::Path {
        dir: path.to_path_buf(),
        folder: path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        temporary: (!in_store).then(|| parent.to_path_buf()),
    }
}

/// argv 無しの分岐（そのゴーストの最後のバルーンの記憶 → ゴーストの descript.txt の
/// `default.balloon.path` → `balloon` → 同梱の最初の 1 個 → 唯一 → 既定 → 無作為）で
/// 1 ゴースト分のバルーンを解く（起動前の解決の後半・切替先のバルーンの解決＝要件 4.7）。
/// 初回起動の argv の第 2 引数はここへ届かない。
///
/// 記憶・descript の 2 鍵・同梱・並びの和の一覧は、段が当たるかどうかに関わらず先にまとめて読み、
/// 判断と「当たらなかった」の記録は鎖（`boot_resolve::resolve_balloon`）に任せる。候補は
/// `balloons_for`（ゴーストのフォルダの中の `balloon\` が先・根の和が後・同じフォルダ名の根は外す）。
/// 外した根の項目は 1 件ごとに info `ghost_balloon_hides_root` を残す（起動と切替でだけ・
/// areka-P0-ghost-inner-balloon 要件 3.2）。「当たらなかった」の記録の置き場は今どおり根の列だけ。
pub(crate) fn resolve_balloon_for_ghost(
    roots: &areka_ghost::BasewareRoots,
    ghost_dir: &std::path::Path,
    pick: fn(usize) -> usize,
) -> Result<crate::boot_resolve::BalloonDecision, crate::boot_resolve::NoBalloon> {
    use crate::boot_resolve::{self, BalloonInputs};
    use areka_ghost::catalog;

    let memory = boot_resolve::read_last_balloon(ghost_dir);
    let standard = catalog::standard_balloon_keys(ghost_dir);
    let companion = catalog::companion_balloon(ghost_dir);
    let areka_ghost::catalog_roots::GhostBalloons { listed, hidden } =
        areka_ghost::catalog_roots::balloons_for(roots, ghost_dir);
    for h in &hidden {
        tracing::info!(
            event = "ghost_balloon_hides_root",
            folder = h.folder.as_str(),
            hidden = %h.hidden.display(),
            by = %h.by.display(),
            "[boot_config] ゴーストの中のバルーンが根の同じフォルダ名を隠した"
        );
    }
    boot_resolve::resolve_balloon(
        &BalloonInputs {
            stores: &roots.balloon_stores(),
            argv: None,
            memory: memory.as_deref(),
            default_balloon_path: standard.default_balloon_path.as_deref(),
            balloon_name: standard.balloon.as_deref(),
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
/// - 未設定でプロセスの形に書ける根が在れば `<書ける根>\profile\areka`（`areka_user_data::app_profile_dir`）。
/// - 無ければ実行ファイル隣接の `profile/areka/`（起動した exe の本当の場所（`exe_location`）の親ディレクトリ／`current_exe()`
///   失敗時は `"."` へ寛容フォールバック——boot 呼び出し自体が非致命ゆえ panic/Err 伝播は不要）。
///
/// App スコープはマウント解決に現れない（ghost/shell スコープは `<shiori.dir>`／`<shell.dir>` から
/// ghost が導く）ため、bin が本関数で供給して `GhostBootOptions.app_profile_dir` へ渡す。
///
/// 読み取りだけの走り（readonly-mode 要件 4.3・4.5）では 1 回の走りに 1 度だけ決めて覚える
/// （呼び手は 1 回の走りで 3 か所以上・別の areka が途中で写しを終えても置き場が変わらないため）。
pub(crate) fn default_app_profile_dir() -> std::path::PathBuf {
    static READONLY_DIR: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
    let readonly = crate::readonly::is_on();
    let decide = || {
        app_profile_dir_for(
            readonly,
            std::env::var_os(areka_user_data::PROFILE_DIR_ENV).as_deref(),
            crate::roots::user_root::layout(),
            exe_location().and_then(std::path::Path::parent),
        )
    };
    if readonly {
        remembered(&READONLY_DIR, decide)
    } else {
        decide()
    }
}

/// [`default_app_profile_dir`] の選び分け（旗を引数で受ける）。共有の決め方（user-data-root
/// 要件 5.1〜5.3）: env → 書ける根 → 実行ファイルの隣。読み取りだけなら写す元をそのまま読む
/// `readonly_app_profile_dir`（readonly-mode 要件 4.3）。
pub(crate) fn app_profile_dir_for(
    readonly: bool,
    profile_env: Option<&std::ffi::OsStr>,
    layout: &areka_user_data::Layout,
    exe_dir: Option<&std::path::Path>,
) -> std::path::PathBuf {
    let dir = if readonly {
        areka_user_data::readonly_app_profile_dir(profile_env, layout, exe_dir)
    } else {
        areka_user_data::app_profile_dir(profile_env, layout, exe_dir)
    };
    // 決まらない（実行ファイルの場所が取れない）ときは今どおり `"."` の下。
    dir.unwrap_or_else(|| std::path::Path::new(".").join("profile").join("areka"))
}

/// 覚える所: 初めの 1 回だけ `decide` を呼び、後は覚えた値を返す（試験は自分の `OnceLock` を渡す）。
pub(crate) fn remembered(
    cell: &std::sync::OnceLock<std::path::PathBuf>,
    decide: impl FnOnce() -> std::path::PathBuf,
) -> std::path::PathBuf {
    cell.get_or_init(decide).clone()
}

/// `ghost_root`／helper パスから `GhostBootOptions` を組み立てる純粋ヘルパ
/// （design.md「main の ghost boot／shutdown 結線」）。
///
/// - `shiori`: `shiori_wiring_for(modules, helper_exe)`（「読む」は実行ファイル隣接の 32bit helper・本番結線、「読まない」は差し替え口）。
/// - `default_encoding`: `DefaultEncoding::Ansi`（charset 未宣言時の SSP 既定・記憶
///   areka-descript-encoding-ishiori-utf8）。
/// - `sinks`: 可変長 sink 列（S-3）を `vec![LogSink, DiscardSink]` で埋める。broadcast（D4）で
///   全 cue は登録された全 sink へ配られるため、両スロットを `LogSink` にすると 1 cue が 2 回ログ
///   される（二重ログ）。記録 sink を **1 本（`LogSink`）だけ**にし、もう一方を破棄専用の
///   `DiscardSink` で埋めることで cue ごと 1 回ログへ正す（設計 D4 Topic 2）。
/// - `system_vars`: 本番 provider（`SystemVarWiring::FromSylphya`＝boot が据えた sylphya reader
///   由来のスナップショット・R7.1）。
/// - `app_profile_dir`: App スコープの sylphya profile root。呼び手が渡す（本番の fallback の腕は結線の
///   入力の置き場＝`default_app_profile_dir()`）。ここで本物の環境から決めないので、テストが書ける根の
///   形を作らない（user-data-root 要件 10.6）。
/// - `ticker`: `TickerMode::Real` を既定 `TickerConfig`（`base_interval=50ms`／
///   `kanade_interval=1000ms`／実クロック `GetTickCount64`）で駆動する。
///
pub(crate) fn ghost_boot_options(
    ghost_root: std::path::PathBuf,
    helper_exe: std::path::PathBuf,
    app_profile_dir: Option<std::path::PathBuf>,
    modules: crate::cli::ModulePolicy,
) -> areka_ghost::GhostBootOptions {
    areka_ghost::GhostBootOptions {
        strict_judges: crate::emo2_boot::strict_judges::production(),
        ghost_root,
        default_encoding: areka_parsers::charset::DefaultEncoding::Ansi,
        shiori: shiori_wiring_for(modules, helper_exe),
        sinks: vec![
            Box::new(areka_ghost::sink::LogSink::new()),
            Box::new(areka_ghost::sink::DiscardSink::new()),
        ],
        system_vars: areka_ghost::SystemVarWiring::FromSylphya,
        app_profile_dir,
        ticker: areka_ghost::TickerMode::Real(Default::default()),
    }
}

/// SHIORI の読み方の指定から結線を決める純粋な口（要件 7.1）。「読む」は今の補助のプロセス
/// （`Helper { helper_exe }`）、「読まない」は何を聞かれても中身なしで答える差し替え口
/// （`Custom(empty_connect())`）。倒れ先の [`ghost_boot_options`] と結線ありの起動の両方が使う。
pub(crate) fn shiori_wiring_for(
    modules: crate::cli::ModulePolicy,
    helper_exe: std::path::PathBuf,
) -> areka_ghost::ShioriWiring {
    match modules {
        crate::cli::ModulePolicy::Load => areka_ghost::ShioriWiring::Helper { helper_exe },
        crate::cli::ModulePolicy::Skip => {
            areka_ghost::ShioriWiring::Custom(Box::new(areka_ghost::shiori_wiring::empty_connect()))
        }
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

#[cfg(test)]
#[path = "boot_config_balloon_tests.rs"]
mod balloon_tests;

#[cfg(test)]
#[path = "boot_config_roots_tests.rs"]
mod roots_tests;

#[cfg(test)]
#[path = "boot_config_copy_tests.rs"]
mod copy_tests;

#[cfg(test)]
#[path = "boot_config_option_tests.rs"]
mod option_tests;

#[cfg(test)]
#[path = "boot_config_readonly_tests.rs"]
mod readonly_tests;

#[cfg(test)]
#[path = "boot_config_callghost_tests.rs"]
mod callghost_tests;
