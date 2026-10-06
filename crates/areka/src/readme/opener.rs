//! 開く処理の 1 か所（areka-P0-open-external-tags task 3.1・要件 1.1〜1.4・2.1〜2.4・3.1・
//! 4.1〜4.6・4.8・5.1・5.3・6.1・6.2・10.2・task 3.2・task 3.3＝要件 2.5・3.3・5.4・6.4・7.2〜7.4・
//! 7.6・7.8・10.5・task 3.4＝要件 7.1・7.5・10.1）。
//!
//! 行き先（[`Destination`]）と文脈（[`OpenContext`]）から OS への 1 回分の呼び出し（[`OsCall`]）を
//! 作る [`resolve`] を置く。fs は読むが OS は呼ばない（呼ぶのは開く専用のスレッドの実行だけ）。
//! [`execute`] は 1 件を記録して OS へ渡し、[`serve`] は受信端が閉じるまでそれを繰り返す。
//! [`Opener`] はそのスレッドへの送信端（World の持ち物）、[`submit`] は UI スレッドの唯一の入口。

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};

use bevy_ecs::world::World;

use super::destination::{Destination, Store, Target};
use super::os_port::{OsCall, OsPort, Verb};
use crate::boot_config::BootContext;
use crate::emo2_boot::ghost_switch::{GhostSpec, resolve_switch_target};
use crate::emo2_boot::shell_balloon_resolve::{
    SkinCandidate, balloon_candidates, shell_candidates,
};
use crate::ghost_session::GhostSlot;
use crate::log_history::TARGET_ERROR;

/// 開く文脈（UI スレッドで World から写す）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OpenContext {
    /// get_active_ghost_list と同じ名（記録の `ghost`）。
    pub ghost: String,
    /// ゴーストのフォルダ（`ghost/<フォルダ>`）の絶対パス。
    pub ghost_dir: PathBuf,
    /// ベースウェアの根（BootContext が無ければ None）。
    pub baseware: Option<areka_ghost::BasewareRoot>,
}

/// 解決の失敗。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum OpenFailure {
    /// 解いたパスに何も無い。
    NotFound(PathBuf),
    /// `\![open,explorer,種類,名前]` の名前に当たらない。
    NoMatch { store: Store, name: String },
    /// 目録を引く根（ベースウェアの根）が無い。
    NoBasewareRoot,
}

impl OpenFailure {
    /// 記録の欄 `reason` の値。
    fn reason(&self) -> &'static str {
        match self {
            OpenFailure::NotFound(_) => "not_found",
            OpenFailure::NoMatch { .. } => "no_match",
            OpenFailure::NoBasewareRoot => "no_baseware_root",
        }
    }
}

/// 開く専用のスレッドへ送る 1 件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OpenJob {
    pub destination: Destination,
    pub context: OpenContext,
}

/// 1 件を解決して記録し OS へ渡す。失敗の経路は必ず `error!` 1 行を残す（要件 7.2〜7.4）。
///
/// メッセージボックスは出さない（要件 7.6）。
pub(crate) fn execute(port: &mut dyn OsPort, job: OpenJob) {
    let OpenJob {
        destination: dest,
        context: ctx,
    } = job;
    let kind = dest.target.kind().as_str();
    let call = match resolve(&dest, &ctx, port) {
        Ok(call) => call,
        Err(failure) => {
            tracing::error!(
                target: TARGET_ERROR,
                ghost = %ctx.ghost,
                event = "open_external_failed",
                kind,
                destination = %dest.written,
                tag = %dest.tag,
                reason = failure.reason(),
                ?failure,
                "[readme] could not resolve the destination to open"
            );
            return;
        }
    };
    // 解決した行き先（explorer の /select は選ぶファイルまで見せる）。
    let mut resolved = call.file.clone();
    if let Some(params) = &call.params {
        resolved.push(" ");
        resolved.push(params);
    }
    let resolved = resolved.to_string_lossy().into_owned();
    let verb = match call.verb {
        Verb::Open => "open",
        Verb::Edit => "edit",
    };
    tracing::info!(
        event = "open_external",
        kind,
        destination = %resolved,
        ghost = %ctx.ghost,
        tag = %dest.tag,
        verb,
        "[readme] handed the destination to the OS"
    );
    if let Err(code) = port.shell_execute(&call) {
        tracing::error!(
            target: TARGET_ERROR,
            ghost = %ctx.ghost,
            event = "open_external_failed",
            kind,
            destination = %resolved,
            tag = %dest.tag,
            reason = "os",
            code,
            "[readme] the OS refused to open the destination"
        );
    }
}

/// 受信端が閉じるまで 1 件ずつ [`execute`] する（開く専用のスレッドの中身・要件 7.8）。
pub(crate) fn serve(rx: Receiver<OpenJob>, port: &mut dyn OsPort) {
    for job in rx {
        execute(port, job);
    }
}

/// 開く専用のスレッドへの送信端（World の持ち物・task 3.4・要件 7.1・7.5・10.1）。
///
/// World を落とすと送信端が落ちて受信が終わり、スレッドは自然に終わる。
#[derive(bevy_ecs::prelude::Resource)]
pub(crate) struct Opener {
    tx: Sender<OpenJob>,
}

impl Opener {
    /// `open-external` という名の 1 本のスレッドを起こす。
    ///
    /// 本番のビルドは COM の初期化と本物の [`WindowsShell`] を、テストのビルドは OS を呼ばずに
    /// 記録して断る口を持つ（COM の初期化も本番だけ・開発者の机で本物のアプリを起こさない）。
    pub(crate) fn spawn() -> std::io::Result<Opener> {
        let (tx, rx) = mpsc::channel();
        std::thread::Builder::new()
            .name("open-external".to_owned())
            .spawn(move || {
                #[cfg(not(test))]
                {
                    super::os_port::init_com_for_shell();
                    serve(rx, &mut super::os_port::WindowsShell);
                }
                #[cfg(test)]
                serve(rx, &mut super::opener_test_support::RefusingOs);
            })?;
        Ok(Opener { tx })
    }

    /// テストで送り先だけを差し込む（スレッドを起こさない）。
    #[cfg(test)]
    pub(crate) fn from_sender(tx: Sender<OpenJob>) -> Opener {
        Opener { tx }
    }
}

/// 唯一の入口（UI スレッド）。World から文脈を写して送るだけで、fs も OS も待たない。
///
/// ゴーストが居ない・[`Opener`] が無い・送れない（スレッドが落ちた）は `error!` 1 行で捨てる。
pub(crate) fn submit(world: &World, destination: Destination) {
    let Some(session) = world
        .get_non_send::<GhostSlot>()
        .and_then(|slot| slot.0.as_ref())
    else {
        dropped(None, &destination, "no_ghost");
        return;
    };
    let dir = session.ghost_dir();
    let ghost_dir = std::path::absolute(dir).unwrap_or_else(|_| dir.to_path_buf());
    let name = session.names().and_then(|n| n.name.as_deref());
    let ghost = listed_name(name, &ghost_dir);
    let Some(opener) = world.get_resource::<Opener>() else {
        dropped(Some(&ghost), &destination, "no_opener");
        return;
    };
    let context = OpenContext {
        ghost,
        ghost_dir,
        baseware: world.get_resource::<BootContext>().map(|c| c.root.clone()),
    };
    if let Err(mpsc::SendError(job)) = opener.tx.send(OpenJob {
        destination,
        context,
    }) {
        dropped(Some(&job.context.ghost), &job.destination, "disconnected");
    }
}

/// 入口で捨てた 1 件を記録する（ゴーストが居れば名も付ける）。
fn dropped(ghost: Option<&str>, dest: &Destination, reason: &'static str) {
    tracing::error!(
        target: TARGET_ERROR,
        ghost,
        event = "open_external_dropped",
        kind = dest.target.kind().as_str(),
        destination = %dest.written,
        tag = %dest.tag,
        reason,
        "[readme] dropped a request to open a destination"
    );
}

/// 記録の `ghost` の名（`get_active_ghost_list` と同じ決め方: descript の `name`、空・無しなら
/// ゴーストのフォルダの絶対パス・末尾の区切りなし）。
///
/// `mcp/resolve.rs` は非公開のモジュールなので同じ規則をここに置く。
fn listed_name(name: Option<&str>, ghost_dir: &Path) -> String {
    match name.filter(|n| !n.is_empty()) {
        Some(name) => name.to_owned(),
        None => ghost_dir
            .display()
            .to_string()
            .trim_end_matches(['\\', '/'])
            .to_owned(),
    }
}

/// `mailto:` の前置き（大文字小文字を区別しない）。
const MAILTO: &str = "mailto:";

/// 解決（fs を読む・OS は呼ばない）。
pub(crate) fn resolve(
    dest: &Destination,
    ctx: &OpenContext,
    port: &dyn OsPort,
) -> Result<OsCall, OpenFailure> {
    match &dest.target {
        Target::Url(s) => Ok(call(Verb::Open, s, None, None)),
        Target::Mail(s) => {
            let has_mailto = s
                .get(..MAILTO.len())
                .is_some_and(|h| h.eq_ignore_ascii_case(MAILTO));
            let to = if has_mailto {
                s.clone()
            } else {
                format!("{MAILTO}{s}")
            };
            Ok(call(Verb::Open, to, None, None))
        }
        Target::Path(s) => {
            let path = existing(ctx, s)?;
            Ok(open_file(path))
        }
        Target::Program(s) => {
            let expanded = expand_env(s, port);
            match existing(ctx, &expanded) {
                Ok(path) => Ok(open_file(path)),
                // 名前だけ（区切りを含まない）なら OS のパス探索に任せる（要件 2.4）。
                Err(_) if !expanded.contains(['\\', '/', ':']) => {
                    Ok(call(Verb::Open, expanded, None, None))
                }
                Err(e) => Err(e),
            }
        }
        Target::Folder(s) => {
            let path = existing(ctx, s)?;
            if path.is_dir() {
                Ok(call(Verb::Open, path, None, None))
            } else {
                // ファイルは選んだ状態でフォルダを開く（要件 4.2）。
                let mut params = OsString::from("/select,\"");
                params.push(&path);
                params.push("\"");
                Ok(call(Verb::Open, "explorer.exe", Some(params), None))
            }
        }
        Target::Edit(s) => {
            let path = existing(ctx, s)?;
            Ok(call(Verb::Edit, path, None, None))
        }
        Target::NamedFolder { store, name } => {
            let dir = named_folder(*store, name, ctx)?;
            if dir.is_dir() {
                Ok(call(Verb::Open, dir, None, None))
            } else {
                Err(OpenFailure::NotFound(dir))
            }
        }
    }
}

/// `\![open,explorer,種類,名前]` の名前を目録で引く（要件 4.4〜4.6）。
///
/// ゴーストは `\![change,ghost,名前]` と同じ引き方、シェル・バルーンは descript の `name` →
/// フォルダ名の順・大文字小文字を区別して引く。`random` などの特別な名前は解かない（名指しだけ）。
fn named_folder(store: Store, name: &str, ctx: &OpenContext) -> Result<PathBuf, OpenFailure> {
    let root = ctx.baseware.as_ref().ok_or(OpenFailure::NoBasewareRoot)?;
    let hit = match store {
        Store::Ghost => resolve_switch_target(
            &areka_ghost::catalog::list_ghosts(root),
            &GhostSpec::Name(name.to_owned()),
        )
        .map(|t| t.dir),
        Store::Balloon => skin_by_name(balloon_candidates(root), name),
        Store::Shell => skin_by_name(shell_candidates(&ctx.ghost_dir), name),
    };
    hit.ok_or_else(|| OpenFailure::NoMatch {
        store,
        name: name.to_owned(),
    })
}

/// 候補から descript の `name` → フォルダ名の順で 1 つ引く。
fn skin_by_name(candidates: Vec<SkinCandidate>, want: &str) -> Option<PathBuf> {
    candidates
        .iter()
        .find(|c| c.name.as_deref() == Some(want))
        .or_else(|| candidates.iter().find(|c| c.folder == want))
        .map(|c| c.dir.clone())
}

/// 1 回分の呼び出しを組む。
fn call(
    verb: Verb,
    file: impl Into<OsString>,
    params: Option<OsString>,
    dir: Option<PathBuf>,
) -> OsCall {
    OsCall {
        verb,
        file: file.into(),
        params,
        dir,
    }
}

/// ファイルを開く（作業フォルダはそのファイルのあるフォルダ＝エクスプローラーのダブルクリックと同じ）。
fn open_file(path: PathBuf) -> OsCall {
    let dir = path.parent().map(Path::to_path_buf);
    call(Verb::Open, path, None, dir)
}

/// 絶対ならそのまま、相対なら `ghost/master` に繋ぐ。無ければ [`OpenFailure::NotFound`]。
fn existing(ctx: &OpenContext, written: &str) -> Result<PathBuf, OpenFailure> {
    let p = Path::new(written);
    let path = if p.is_absolute() {
        p.to_path_buf()
    } else {
        ctx.ghost_dir.join("ghost").join("master").join(p)
    };
    if path.exists() {
        Ok(path)
    } else {
        Err(OpenFailure::NotFound(path))
    }
}

/// `%名前%` を境界の環境変数で置き換える。未定義はそのまま残す（OS の展開と同じ）。
fn expand_env(s: &str, port: &dyn OsPort) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(open) = rest.find('%') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let Some(close) = after.find('%') else {
            // 閉じの無い `%` は字面のまま。
            out.push_str(&rest[open..]);
            return out;
        };
        let name = &after[..close];
        match (!name.is_empty()).then(|| port.env_var(name)).flatten() {
            Some(value) => {
                out.push_str(&value);
                rest = &after[close + 1..];
            }
            None => {
                // 未定義は `%名前` を残し、閉じの `%` を次の開きとして読み直す。
                out.push('%');
                out.push_str(name);
                rest = &after[close..];
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
#[path = "opener_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "opener_execute_tests.rs"]
mod execute_tests;

#[cfg(test)]
#[path = "opener_submit_tests.rs"]
mod submit_tests;
