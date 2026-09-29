//! 窓への投げ込み（areka-P0-file-drop）。
//!
//! ゴースト窓に落とされた物を「インストール対象でないファイル」「フォルダ」「インストール対象」の
//! 3 つに分ける。振り分け [`sort_drops`] は純粋で、fs にも記録にも触れない——フォルダかどうかと
//! 書庫の目次に `install.txt` が在るかは閉包で注入し、問い合わせの失敗は [`ProbeNote`] として
//! 戻り値に載せる（記録は呼び手が出す）。

use std::path::{Path, PathBuf};
use std::time::Instant;

use areka_kanade::{KanadeMsg, ShioriMethod};
use bevy_ecs::prelude::*;
use wintf::ecs::window::OnFilesDropped;

use crate::ghost_session::GhostSlot;
use crate::install::judge::SEPARATOR;
use crate::install::{self, InstallOrder, InstallOrigin};
use crate::placement::spawn::{BalloonWindowMarker, CharWindowMarker, GhostWindowMarker};

/// `GhostWindowMarker` を持つ全 entity へ受け手 [`on_ghost_files_dropped`] を差す（要件 1.1〜1.3）。
///
/// `ghost_session::prepare_ghost_windows` の窓を作る閉包の中で、spawn の直後に同期に呼ぶ
/// （`attach_balloon_pointer_handlers` と同じ契約）。起こし直しも同じ閉包を通るので、回数によらず
/// 新しい窓に差さる。印の無い窓には差さない。
pub(crate) fn attach_file_drop_receivers(world: &mut World) {
    let ghost_windows: Vec<Entity> = world
        .query_filtered::<Entity, With<GhostWindowMarker>>()
        .iter(world)
        .collect();
    for e in ghost_windows {
        world
            .entity_mut(e)
            .insert(OnFilesDropped(on_ghost_files_dropped));
    }
}

/// 窓に差す受け手（`OnFilesDropped` の署名）。1 回の投げ込みをこの 1 回の呼び出しで終える
/// （要件 6.5）: スコープ → 振り分け → 問い合わせの失敗の記録 → 受け取りの記録 →
/// `OnFileDrop2` を 1 回 → `OnDirectoryDrop` を落とされた順に 1 つずつ → 依頼（要件 6.1・6.2）。
/// 途中で戻るのは窓に印が無いときだけ。依頼はゴーストが定常かどうかを見ずに渡す（要件 3.8）。
pub(crate) fn on_ghost_files_dropped(world: &mut World, entity: Entity, paths: Vec<PathBuf>) {
    let count = paths.len();
    let scope = if let Some(m) = world.get::<CharWindowMarker>(entity) {
        m.scope
    } else if let Some(m) = world.get::<BalloonWindowMarker>(entity) {
        m.scope
    } else {
        tracing::warn!(
            event = "file_drop_unknown_window",
            entity = ?entity,
            count,
            "[file_drop] キャラ／バルーンの印が無い窓への投げ込み: 何もしない"
        );
        return;
    };
    let started = Instant::now();
    let sorted = sort_drops(
        paths,
        |p| std::fs::metadata(p).map(|m| m.is_dir()),
        areka_nar::peek_install_txt,
    );
    for note in &sorted.notes {
        match note {
            ProbeNote::IsDirFailed { path, error } => tracing::warn!(
                event = "file_drop_probe_failed",
                path = %path.display(),
                error = %error,
                "[file_drop] フォルダかどうかを問えない: フォルダでないものとして拡張子で決めた"
            ),
            ProbeNote::ArchiveUnreadable { path, error } => tracing::warn!(
                event = "file_drop_archive_unreadable",
                path = %path.display(),
                reason = %error,
                "[file_drop] 書庫の目次が読めない: インストール対象でないファイルとして扱う"
            ),
        }
    }
    tracing::info!(
        event = "file_drop_received",
        scope,
        count,
        files = sorted.files.len(),
        dirs = sorted.dirs.len(),
        installs = sorted.installs.len(),
        elapsed_ms = started.elapsed().as_millis() as u64,
        "[file_drop] 落とされた物を受け取って振り分けました"
    );
    if !sorted.files.is_empty() {
        send_event(
            world,
            ON_FILE_DROP2,
            file_drop2_references(&sorted.files, scope),
            scope,
        );
    }
    for dir in &sorted.dirs {
        send_event(
            world,
            ON_DIRECTORY_DROP,
            directory_drop_references(dir, scope),
            scope,
        );
    }
    if !sorted.installs.is_empty() {
        // 戻り値の記録は `submit` 自身が出す（`install_order_queued`／`install_order_refused`）。
        install::submit(
            world,
            InstallOrder {
                archives: sorted.installs,
                origin: InstallOrigin::WindowDrop,
            },
        );
    }
}

/// 今のゴーストへ GET・応え不要で 1 件送る（汎用の通知の入口・要件 7.4）。
///
/// 送り口は送る時点の置き場から取る。無ければ（切替の途中・結線なしの起動）`file_drop_no_kanade`、
/// kanade が止まっていれば `file_drop_send_failed` を warn に 1 件残して送らない（送り直さない）。
/// 定常かどうかは見ない（kanade が定常以外を捨てて記録する・要件 6.3）。
fn send_event(world: &World, id: &'static str, references: Vec<String>, scope: usize) {
    let Some(kanade) = world
        .get_non_send::<GhostSlot>()
        .and_then(|slot| slot.0.as_ref())
        .and_then(|s| s.kanade().cloned())
    else {
        tracing::warn!(
            event = "file_drop_no_kanade",
            id,
            scope,
            "[file_drop] 今のゴーストへの送り口が無いので、知らせを送りません"
        );
        return;
    };
    // 要約は Reference0 の要素数と先頭のパス（MIME の並びは載せない）。
    let reference0 = references.first().map_or("", String::as_str);
    let summary = (
        reference0.split(SEPARATOR).count(),
        reference0.split(SEPARATOR).next().unwrap_or("").to_owned(),
    );
    let msg = KanadeMsg::RaiseEvent {
        id: id.to_owned(),
        references,
        method: ShioriMethod::Get,
        reply: None,
    };
    match kanade.send(msg) {
        Ok(()) => tracing::info!(
            event = "file_drop_event_sent",
            id,
            scope,
            references = ?summary,
            "[file_drop] 知らせを今のゴーストへ送りました"
        ),
        Err(_) => tracing::warn!(
            event = "file_drop_send_failed",
            id,
            scope,
            "[file_drop] kanade が止まっているので、知らせを送れません"
        ),
    }
}

/// 振り分けの結果（1 回の投げ込みにつき 1 つ）。
///
/// # 不変条件
///
/// `files.len() + dirs.len() + installs.len()` は入力の数と等しく、3 つの `Vec` はどれも入力の順。
/// `notes` に載った物も `files` に入る（問い合わせに失敗しても一覧から落ちない）。
#[derive(Debug, Default)]
pub(crate) struct DropSort {
    /// インストール対象でないファイル。
    pub files: Vec<PathBuf>,
    /// フォルダ。
    pub dirs: Vec<PathBuf>,
    /// インストール対象（拡張子が `.nar`／`.zip` で目次に `install.txt` が在る書庫）。
    pub installs: Vec<PathBuf>,
    /// 問い合わせの失敗（記録は呼び手が出す）。
    pub notes: Vec<ProbeNote>,
}

/// 問い合わせの失敗。どちらも当該の物は「インストール対象でないファイル」側へ倒れる。
#[derive(Debug)]
pub(crate) enum ProbeNote {
    /// フォルダかどうかを問えなかった（フォルダでないものとして拡張子で決めた・要件 2.5）。
    IsDirFailed {
        path: PathBuf,
        error: std::io::Error,
    },
    /// 書庫の目次が読めなかった（`files` へ倒した・要件 2.4）。
    ArchiveUnreadable {
        path: PathBuf,
        error: areka_nar::NarError,
    },
}

/// 純粋な振り分け（要件 2.1〜2.6）。1 つの物につき上から順に最初に当たった種類に決める。
///
/// 1. `is_dir` が `Ok(true)` → `dirs`（`install.txt` を持つフォルダもフォルダ・要件 2.3）。
/// 2. `is_dir` が `Err` → [`ProbeNote::IsDirFailed`] を積み、フォルダでないものとして 3 へ。
/// 3. 拡張子が `.nar`／`.zip`（ASCII 大小無視）でなければ → `files`（目次は問わない）。
/// 4. `has_install_txt` が `Ok(true)` → `installs`／`Ok(false)` → `files`／
///    `Err` → [`ProbeNote::ArchiveUnreadable`] を積んで `files`。
///
/// 2 つの閉包と拡張子のほかは見ない（中身・大きさ・他のエントリを見ない・要件 2.2）。3 つの `Vec` は
/// 入力の順に押す（要件 2.6）。記録は出さず、問い合わせの失敗は `notes` に載せる。
///
/// 本番の注入は `fs::metadata(p).map(|m| m.is_dir())` と [`areka_nar::peek_install_txt`]。
pub(crate) fn sort_drops(
    paths: Vec<PathBuf>,
    mut is_dir: impl FnMut(&Path) -> std::io::Result<bool>,
    mut has_install_txt: impl FnMut(&Path) -> Result<bool, areka_nar::NarError>,
) -> DropSort {
    let mut sort = DropSort::default();
    for path in paths {
        match is_dir(&path) {
            Ok(true) => {
                sort.dirs.push(path);
                continue;
            }
            Ok(false) => {}
            Err(error) => sort.notes.push(ProbeNote::IsDirFailed {
                path: path.clone(),
                error,
            }),
        }
        if !is_archive_ext(&path) {
            sort.files.push(path);
            continue;
        }
        match has_install_txt(&path) {
            Ok(true) => sort.installs.push(path),
            Ok(false) => sort.files.push(path),
            Err(error) => {
                sort.notes.push(ProbeNote::ArchiveUnreadable {
                    path: path.clone(),
                    error,
                });
                sort.files.push(path);
            }
        }
    }
    sort
}

/// 拡張子が `nar`／`zip`（ASCII 大小無視）か。
fn is_archive_ext(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("nar") || e.eq_ignore_ascii_case("zip"))
}

// 送るイベント名はこの 2 定数だけ（要件 7.1・8.8）。
// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnFileDrop2:1
pub(crate) const ON_FILE_DROP2: &str = "OnFileDrop2";
// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnDirectoryDrop:1
pub(crate) const ON_DIRECTORY_DROP: &str = "OnDirectoryDrop";

/// `OnFileDrop2` の Reference 3 つ（要件 4.2〜4.4・4.6）:
/// [パスを byte 値 1 で連結, スコープ番号（十進）, 同じ並び・同じ数の MIME を byte 値 1 で連結]。
///
/// 決められない MIME は空文字のまま連結するので区切りは残り、Reference0 と Reference2 の要素数は
/// 常に等しい。パスは `to_string_lossy` で載せる（正規化も検査もしない・設計で決めたこと 10）。
pub(crate) fn file_drop2_references(files: &[PathBuf], scope: usize) -> Vec<String> {
    let paths: Vec<_> = files.iter().map(|p| p.to_string_lossy()).collect();
    let mimes: Vec<_> = files.iter().map(|p| mime_for(p)).collect();
    vec![
        paths.join(SEPARATOR),
        scope.to_string(),
        mimes.join(SEPARATOR),
    ]
}

/// `OnDirectoryDrop` の Reference 2 つ（要件 5.2）: [パス, スコープ番号（十進）]。
pub(crate) fn directory_drop_references(dir: &Path, scope: usize) -> Vec<String> {
    vec![dir.to_string_lossy().into_owned(), scope.to_string()]
}

/// 拡張子（ASCII 大小無視）から MIME を引く（要件 4.5・10.5）。表に無い・拡張子なしは `""`。
pub(crate) fn mime_for(path: &Path) -> &'static str {
    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
        return "";
    };
    MIME_TABLE
        .iter()
        .find(|(e, _)| e.eq_ignore_ascii_case(ext))
        .map_or("", |(_, m)| m)
}

/// 拡張子（小文字）→ MIME の定数表（拡張子 38・MIME 33 種・値は IANA の登録名）。
/// 広げるときは行を足し、`file_drop_mime_tests.rs` の直書きの数も直す（要件 9.8）。
const MIME_TABLE: &[(&str, &str)] = &[
    // 画像 8
    ("png", "image/png"),
    ("jpg", "image/jpeg"),
    ("jpeg", "image/jpeg"),
    ("gif", "image/gif"),
    ("bmp", "image/bmp"),
    ("webp", "image/webp"),
    ("ico", "image/vnd.microsoft.icon"),
    ("svg", "image/svg+xml"),
    // 音 7
    ("mp3", "audio/mpeg"),
    ("wav", "audio/wav"),
    ("ogg", "audio/ogg"),
    ("flac", "audio/flac"),
    ("mid", "audio/midi"),
    ("midi", "audio/midi"),
    ("m4a", "audio/mp4"),
    // 動画 5
    ("mp4", "video/mp4"),
    ("webm", "video/webm"),
    ("avi", "video/x-msvideo"),
    ("mkv", "video/x-matroska"),
    ("mov", "video/quicktime"),
    // 文字 7
    ("txt", "text/plain"),
    ("csv", "text/csv"),
    ("htm", "text/html"),
    ("html", "text/html"),
    ("css", "text/css"),
    ("js", "text/javascript"),
    ("md", "text/markdown"),
    // xml・json・pdf 3
    ("xml", "application/xml"),
    ("json", "application/json"),
    ("pdf", "application/pdf"),
    // 書庫 6（`.nar` は正典「実体はzip」）
    ("zip", "application/zip"),
    ("nar", "application/zip"),
    ("7z", "application/x-7z-compressed"),
    ("gz", "application/gzip"),
    ("tar", "application/x-tar"),
    ("rar", "application/vnd.rar"),
    // 実行体 2
    ("exe", "application/vnd.microsoft.portable-executable"),
    ("dll", "application/vnd.microsoft.portable-executable"),
];

#[cfg(test)]
#[path = "file_drop_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "file_drop_mime_tests.rs"]
mod mime_tests;

#[cfg(test)]
#[path = "file_drop_wiring_tests.rs"]
mod wiring_tests;
