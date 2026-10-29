//! cursorブレスとゴーストの descript の表（shell-mouse-cursor・design.md「CursorTable」）。
//!
//! 転記済みの cursorブレス（`parse_cursor_braces`）とゴーストの descript を厳しく読み、
//! 鍵（スコープ・当たり判定名・場面）→ 形の表と、ゴーストの差し替えにする純関数。記録はしない
//! （報告を返し、記録は CursorAssets が読み込み 1 回につき 1 度残す）。ファイルの実在も見ない。
// 消費側（CursorAssets・CursorPick・CursorAnswer）は tasks.md 3.1・3.2・4.2 で来る。
#![cfg_attr(not(test), allow(dead_code))]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use areka_parsers::shell::CursorBrace;

/// cursorブレスの場面（`mouseup*`・`mousedown*`・`mouserightdown*`・`mousehover*`・`mousewheel*`）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum CursorScene {
    Up,
    Down,
    RightDown,
    Hover,
    Wheel,
}

/// システムカーソル 10 種（`system:<名前>`）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SystemCursor {
    Arrow,
    Cross,
    Finger,
    Grip,
    Hand,
    Help,
    Move,
    No,
    Text,
    Wait,
}

/// F は読んだ段では String（ファイル名）、作った段では usize（作ったカーソルの番号）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Shape<F> {
    System(SystemCursor),
    File(F),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct CursorRows<F> {
    pub rows: BTreeMap<(u32, String, CursorScene), Shape<F>>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct GhostCursorKeys<F> {
    pub finger: Option<Shape<F>>,
    pub hand: Option<Shape<F>>,
    pub grip: Option<Shape<F>>,
    pub arrow: Option<Shape<F>>,
    /// 読んだが出さない鍵（`mousecursor.text`・`mousecursor.wait`）。
    pub unused: Vec<&'static str>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CursorIssue {
    HeadingUnreadable {
        heading: String,
    },
    RowUnreadable {
        heading: String,
        line: String,
        reason: RowReason,
    },
    RowReplaced {
        heading: String,
        line: String,
    },
    GhostValueUnreadable {
        key: &'static str,
        value: String,
        reason: ShapeReason,
    },
    GhostBothSpellings {
        ignored: String,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RowReason {
    TooFewFields,
    UnknownScene,
    NumberNotDigits,
    EmptyRegion,
    Shape(ShapeReason),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ShapeReason {
    UnknownSystemName,
    NotCurOrAni,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PathReason {
    OutsideFolder,
}

/// 場面の名前（番号の前の綴り）。`mousedown` は `mouserightdown` の頭ではないので順は問わない。
const SCENES: [(&str, CursorScene); 5] = [
    ("mouseup", CursorScene::Up),
    ("mousedown", CursorScene::Down),
    ("mouserightdown", CursorScene::RightDown),
    ("mousehover", CursorScene::Hover),
    ("mousewheel", CursorScene::Wheel),
];

const SYSTEM_NAMES: [(&str, SystemCursor); 10] = [
    ("arrow", SystemCursor::Arrow),
    ("cross", SystemCursor::Cross),
    ("finger", SystemCursor::Finger),
    ("grip", SystemCursor::Grip),
    ("hand", SystemCursor::Hand),
    ("help", SystemCursor::Help),
    ("move", SystemCursor::Move),
    ("no", SystemCursor::No),
    ("text", SystemCursor::Text),
    ("wait", SystemCursor::Wait),
];

/// cursorブレスの見出しをスコープへ（`sakura.cursor`→0・`kero.cursor`→1・`char<数>.cursor`→その数）。
fn heading_scope(heading: &str) -> Option<u32> {
    match heading.strip_suffix(".cursor")? {
        "sakura" => Some(0),
        "kero" => Some(1),
        name => {
            let digits = name.strip_prefix("char")?;
            if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            digits.parse().ok()
        }
    }
}

/// cursorブレスの 1 行を鍵の（当たり判定名・場面）と形に読む。
fn read_row(fields: &[String]) -> Result<(String, CursorScene, Shape<String>), RowReason> {
    let [scene_field, region, rest @ ..] = fields else {
        return Err(RowReason::TooFewFields);
    };
    if rest.is_empty() {
        return Err(RowReason::TooFewFields);
    }
    let (number, scene) = SCENES
        .iter()
        .find_map(|(name, scene)| scene_field.strip_prefix(name).map(|n| (n, *scene)))
        .ok_or(RowReason::UnknownScene)?;
    if number.is_empty() || !number.bytes().all(|b| b.is_ascii_digit()) {
        return Err(RowReason::NumberNotDigits);
    }
    if region.is_empty() {
        return Err(RowReason::EmptyRegion);
    }
    // 第 3 欄以降を継ぎ直す（ファイル名の `,` を失わない）。
    let shape = parse_shape(&rest.join(",")).map_err(RowReason::Shape)?;
    Ok((region.clone(), scene, shape))
}

/// 転記済みの cursorブレスを表にする。同じ鍵は後勝ちで、前の行を `RowReplaced` で報告する。
/// 当たり判定名は綴りどおりで、面の当たり判定と突き合わせない（要件 1.6）。
pub(crate) fn read_cursor_braces(braces: &[CursorBrace]) -> (CursorRows<String>, Vec<CursorIssue>) {
    let mut rows = BTreeMap::new();
    // 鍵ごとに、今使っている行の見出しと原文（後の行に替わったときの報告用）。
    let mut origin: BTreeMap<(u32, String, CursorScene), (String, String)> = BTreeMap::new();
    let mut issues = Vec::new();
    for brace in braces {
        let Some(scope) = heading_scope(&brace.heading) else {
            issues.push(CursorIssue::HeadingUnreadable {
                heading: brace.heading.clone(),
            });
            continue;
        };
        for fields in &brace.lines {
            let line = fields.join(",");
            match read_row(fields) {
                Ok((region, scene, shape)) => {
                    let key = (scope, region, scene);
                    rows.insert(key.clone(), shape);
                    if let Some((heading, line)) = origin.insert(key, (brace.heading.clone(), line))
                    {
                        issues.push(CursorIssue::RowReplaced { heading, line });
                    }
                }
                Err(reason) => issues.push(CursorIssue::RowUnreadable {
                    heading: brace.heading.clone(),
                    line,
                    reason,
                }),
            }
        }
    }
    (CursorRows { rows }, issues)
}

/// ゴーストの descript からカーソルの差し替えを読む（`mousecursor`、無ければ旧い綴りの `cursor`）。
/// `mousecursor.text`・`mousecursor.wait` は読んで `unused` に並べるだけ（出す場面が無い・要件 5.7）。
pub(crate) fn read_ghost_cursor_keys(
    kv: &BTreeMap<String, String>,
) -> (GhostCursorKeys<String>, Vec<CursorIssue>) {
    let mut keys = GhostCursorKeys::default();
    let mut issues = Vec::new();
    let mut read = |key: &'static str, value: &String| match parse_shape(value) {
        Ok(shape) => Some(shape),
        Err(reason) => {
            issues.push(CursorIssue::GhostValueUnreadable {
                key,
                value: value.clone(),
                reason,
            });
            None
        }
    };
    keys.finger = match (kv.get("mousecursor"), kv.get("cursor")) {
        (Some(value), _) => read("mousecursor", value),
        (None, Some(value)) => read("cursor", value),
        (None, None) => None,
    };
    keys.hand = kv
        .get("mousecursor.hand")
        .and_then(|v| read("mousecursor.hand", v));
    keys.grip = kv
        .get("mousecursor.grip")
        .and_then(|v| read("mousecursor.grip", v));
    keys.arrow = kv
        .get("mousecursor.arrow")
        .and_then(|v| read("mousecursor.arrow", v));
    for key in ["mousecursor.text", "mousecursor.wait"] {
        if kv.get(key).and_then(|v| read(key, v)).is_some() {
            keys.unused.push(key);
        }
    }
    if let (Some(_), Some(ignored)) = (kv.get("mousecursor"), kv.get("cursor")) {
        issues.push(CursorIssue::GhostBothSpellings {
            ignored: ignored.clone(),
        });
    }
    (keys, issues)
}

/// cursorブレスの形の欄・descript の値を読む。`system:` の名前と拡張子は ASCII の大小を区別しない。
pub(crate) fn parse_shape(value: &str) -> Result<Shape<String>, ShapeReason> {
    if let Some(prefix) = value.get(..7)
        && prefix.eq_ignore_ascii_case("system:")
    {
        let name = &value[7..];
        return SYSTEM_NAMES
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, cursor)| Shape::System(*cursor))
            .ok_or(ShapeReason::UnknownSystemName);
    }
    let ext = Path::new(value)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    if ext.eq_ignore_ascii_case("cur") || ext.eq_ignore_ascii_case("ani") {
        Ok(Shape::File(value.to_string()))
    } else {
        Err(ShapeReason::NotCurOrAni)
    }
}

/// ファイル名の探す候補（`dirs` の順＝シェルのフォルダ → `ghost\master`・要件 6.4）。
/// 絶対パス・ドライブ付き・根から始まる・上のフォルダへ出る（`..`）名前は断る（要件 6.6）。
/// 実在は確かめない（CursorAssets が見る）。
pub(crate) fn candidate_paths(name: &str, dirs: &[&Path]) -> Result<Vec<PathBuf>, PathReason> {
    let outside = Path::new(name).components().any(|c| {
        matches!(
            c,
            std::path::Component::Prefix(_)
                | std::path::Component::RootDir
                | std::path::Component::ParentDir
        )
    });
    if outside {
        return Err(PathReason::OutsideFolder);
    }
    Ok(dirs.iter().map(|dir| dir.join(name)).collect())
}

#[cfg(test)]
#[path = "shell_cursor_table_tests.rs"]
mod tests;
