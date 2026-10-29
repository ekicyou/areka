//! シェルのカーソルの資源（shell-mouse-cursor・design.md「CursorAssets」）。
//!
//! CursorTable の表のファイル名を探して作り、表の報告とファイルの失敗を読み込み 1 回につき 1 度だけ
//! 記録し、UI スレッドの NonSend の資源として渡す。失敗しない（シェルの読み込みを止めない）。
// 消費側（起動・切り替えの道とキャラクターの窓の答え）は tasks.md 4.2・4.3 で来る。
#![cfg_attr(not(test), allow(dead_code))]

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use areka_parsers::shell::CursorBrace;
use bevy_ecs::prelude::*;
use tracing::{debug, info, warn};
use windows::Win32::UI::WindowsAndMessaging::HCURSOR;
use wintf::ecs::cursor::answer::{clear_last_shown, push_cursor};
use wintf::ecs::cursor::{CursorTrigger, OwnedCursor, read_cur_directory};

use super::shell_cursor_table::{
    CursorIssue, CursorRows, GhostCursorKeys, Shape, candidate_paths, read_cursor_braces,
    read_ghost_cursor_keys,
};
use crate::placement::spawn::CharWindowMarker;

/// シェル 1 つぶんのカーソル（NonSend の資源そのもの）。既定値は「行も差し替えも無い」。
#[derive(Default)]
pub(crate) struct ShellCursorAssets {
    pub rows: CursorRows<usize>,
    pub ghost: GhostCursorKeys<usize>,
    /// 読んだファイル（番号＝`Shape::File` の値）。
    pub files: Vec<CursorFile>,
    /// .cur を窓の DPI の寸法ごとに作った控え（(番号, 寸法) → カーソル）。資源と一緒に捨てる。
    pub sized: RefCell<HashMap<(usize, u32), Option<OwnedCursor>>>,
}

pub(crate) enum CursorFile {
    /// 目録を確かめた .cur のバイト列（窓の DPI ごとに from_cur_bytes で作る）。
    Cur(Vec<u8>),
    /// LoadImageW で作った .ani（主のモニターの寸法・既知の限り）。
    Ani(OwnedCursor),
}

/// 表を作り、名前ごとに 1 度だけファイルを探して作る。作れなかった名前を使う行と差し替えは外す
/// （行が無いのと同じ既定になる・要件 6.5・6.6）。
pub(crate) fn build_shell_cursors(
    braces: &[CursorBrace],
    ghost_kv: &BTreeMap<String, String>,
    shell_dir: &Path,
    ghost_dir: &Path,
) -> ShellCursorAssets {
    let (rows, row_issues) = read_cursor_braces(braces);
    let (ghost, ghost_issues) = read_ghost_cursor_keys(ghost_kv);
    row_issues.iter().chain(&ghost_issues).for_each(log_issue);
    for key in &ghost.unused {
        info!(
            key,
            "読んだが、キャラクターの窓に出す場面が無いので使わない"
        );
    }

    let dirs = [shell_dir, ghost_dir];
    let mut files = Vec::new();
    let mut loaded: HashMap<String, Option<usize>> = HashMap::new();
    let mut resolve = |shape: Shape<String>| -> Option<Shape<usize>> {
        match shape {
            Shape::System(s) => Some(Shape::System(s)),
            Shape::File(name) => {
                let index = *loaded.entry(name).or_insert_with_key(|name| {
                    let file = load_file(name, &dirs)?;
                    files.push(file);
                    Some(files.len() - 1)
                });
                index.map(Shape::File)
            }
        }
    };
    let rows = CursorRows {
        rows: rows
            .rows
            .into_iter()
            .filter_map(|(key, shape)| Some((key, resolve(shape)?)))
            .collect(),
    };
    let mut map = |shape: Option<Shape<String>>| shape.and_then(&mut resolve);
    let ghost = GhostCursorKeys {
        finger: map(ghost.finger),
        hand: map(ghost.hand),
        grip: map(ghost.grip),
        arrow: map(ghost.arrow),
        unused: ghost.unused,
    };
    debug!(
        rows = rows.rows.len(),
        files = files.len(),
        "シェルのカーソルを読み終えた"
    );
    ShellCursorAssets {
        rows,
        ghost,
        files,
        sized: RefCell::default(),
    }
}

fn log_issue(issue: &CursorIssue) {
    match issue {
        CursorIssue::HeadingUnreadable { heading } => {
            warn!(
                heading,
                "cursorブレスの見出しが読めないので、ブレスごと使わない"
            )
        }
        CursorIssue::RowUnreadable {
            heading,
            line,
            reason,
        } => warn!(
            heading,
            line,
            ?reason,
            "cursorブレスの行が読めないので使わない"
        ),
        CursorIssue::RowReplaced { heading, line } => {
            warn!(heading, line, "同じ鍵の後の行を使うので、この行は使わない")
        }
        CursorIssue::GhostValueUnreadable { key, value, reason } => {
            warn!(
                key,
                value,
                ?reason,
                "descript のカーソルの値が読めないので使わない"
            )
        }
        CursorIssue::GhostBothSpellings { ignored } => {
            warn!(
                ignored,
                "descript に mousecursor と cursor の両方があるので cursor は使わない"
            )
        }
    }
}

/// シェルのフォルダ → `ghost\master` の順で最初に在るファイルを読む（要件 6.4）。失敗は名前ごとに
/// 1 度だけ記録する（呼び手が名前ごとに 1 度だけ呼ぶ・要件 6.7）。
fn load_file(name: &str, dirs: &[&Path]) -> Option<CursorFile> {
    let Ok(paths) = candidate_paths(name, dirs) else {
        warn!(
            file = name,
            "カーソルのファイル名が探す場所のフォルダの外を指すので読まない"
        );
        return None;
    };
    let Some(path) = paths.iter().find(|p| p.is_file()) else {
        warn!(file = name, searched = ?paths, "カーソルのファイルが見つからない");
        return None;
    };
    let is_cur = path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("cur"));
    let made = if is_cur {
        std::fs::read(path)
            .map_err(|e| e.to_string())
            .and_then(|bytes| match read_cur_directory(&bytes) {
                Ok(_) => Ok(CursorFile::Cur(bytes)),
                Err(e) => Err(e.to_string()),
            })
    } else {
        OwnedCursor::load_file(path)
            .map(CursorFile::Ani)
            .map_err(|e| match std::error::Error::source(&e) {
                Some(source) => format!("{e}: {source}"),
                None => e.to_string(),
            })
    };
    made.inspect_err(|reason| {
        warn!(file = name, searched = ?path, reason, "カーソルのファイルを読めない・壊れている");
    })
    .ok()
}

impl ShellCursorAssets {
    /// ファイルの番号と寸法のカーソル。`.cur` は寸法ごとに初めの 1 度だけ作って控える（作れなければ
    /// 控えに「無し」を置き、呼び手は既定へ落とす）。`.ani` は寸法に関わらず読んだもの。
    pub(crate) fn cursor_for(&self, file: usize, size: u32) -> Option<HCURSOR> {
        let bytes = match self.files.get(file)? {
            CursorFile::Ani(cursor) => return Some(cursor.handle()),
            CursorFile::Cur(bytes) => bytes,
        };
        self.sized
            .borrow_mut()
            .entry((file, size))
            .or_insert_with(|| {
                OwnedCursor::from_cur_bytes(bytes, size)
                    .inspect_err(
                        |e| warn!(file, size, error = %e, "シェルの .cur からカーソルを作れない"),
                    )
                    .ok()
            })
            .as_ref()
            .map(OwnedCursor::handle)
    }

    /// NonSend の資源を入れ替える: 新しい資源を入れ、全部のキャラクターの窓へ出し直し、最後に出した形の
    /// 控えを消してから前の資源を捨てる（捨てるカーソルを出さない・design.md「シェル・ゴーストの入れ替えの順」）。
    pub(crate) fn hand_to(self, world: &mut World) {
        let old = world.remove_non_send::<ShellCursorAssets>();
        world.insert_non_send(self);
        let windows: Vec<Entity> = world
            .query_filtered::<Entity, With<CharWindowMarker>>()
            .iter(world)
            .collect();
        for window in windows {
            push_cursor(world, window, CursorTrigger::Refresh);
        }
        clear_last_shown();
        if let Some(old) = old {
            debug!(files = old.files.len(), "前のシェルのカーソルを捨てる");
            drop(old);
        }
    }
}

#[cfg(test)]
#[path = "shell_cursor_assets_tests.rs"]
mod tests;
