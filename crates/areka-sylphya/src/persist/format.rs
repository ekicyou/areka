//! 取り込む前の記憶のファイル `sylphya.toml`（`format-version = 1`）の厳しい読み。
//!
//! `sylphya.toml` を読むのは取り込み（`crate::store` の `import`）だけで、書く所は無い。
//!
//! ```toml
//! format-version = 1
//!
//! [window."0"]         # areka.window.scope(0).x|y（値はすべて TOML 文字列）
//! x = "1024"
//! y = "512"
//!
//! [balloon-offset."0"] # areka.balloon.offset.scope(0).x|y
//! x = "30"
//! y = "-10"
//!
//! [boot]               # areka.boot.count
//! count = "3"
//!
//! [vanish]             # areka.vanish.count
//! count = "0"
//!
//! [last]               # areka.last.ghost|balloon|shell|running
//! ghost = "emo2"
//! balloon = "StayseeBalloon"
//! shell = "master"
//! running = "broken"   # 起動中の印（空文字は「無し」と読む）
//!
//! [baseware]           # areka.baseware.roots（空文字は空文字のまま）
//! roots = "<exe>|D:\\ssp"
//!
//! [window-adjacent]    # areka.window.scope(ID).adjacent
//! "1" = "left"
//! ```
//!
//! 壊れている・`format-version` が 1 でない内容は、warn を残して理由の文の `Err` を返す。
//! 欠けた鍵と文字列でない値は、その鍵だけ無しと読む。どの入力でも panic しない。

use std::collections::BTreeMap;

/// 取り込める `sylphya.toml` の形の版。
pub const FORMAT_VERSION: i64 = 1;

const LOG_TARGET: &str = "areka_sylphya::persist::format";

/// 1 スコープの永続ドキュメントの中立 in-memory 表現（`sylphya.toml` 相当）。
///
/// 4 key 族を format 層の素の器として保持する:
/// 窓位置（`window`）・バルーン相対オフセット（`balloon_offset`）・起動記録（`boot_count`）・
/// vanish 回数（`vanish_count`）。値はすべて文字列（往復が自明）。スコープ ID は
/// 不透明文字列。
///
/// マップは [`BTreeMap`] ゆえ直列化順序は ID 昇順で決定論的（同一 doc → 同一出力）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FormatDoc {
    /// 窓位置: スコープ ID → (x, y)。正準 key `areka.window.scope(ID).x|y`。
    pub window: BTreeMap<String, AxisPair>,
    /// バルーン相対オフセット: スコープ ID → (x, y)。正準 key `areka.balloon.offset.scope(ID).x|y`。
    pub balloon_offset: BTreeMap<String, AxisPair>,
    /// 起動記録。正準 key `areka.boot.count`。
    pub boot_count: Option<String>,
    /// vanish 回数。正準 key `areka.vanish.count`。
    pub vanish_count: Option<String>,
    /// 前回のゴーストのフォルダ名。正準 key `areka.last.ghost`・表 `[last]` ghost。
    pub last_ghost: Option<String>,
    /// 前回のバルーンのフォルダ名。正準 key `areka.last.balloon`・表 `[last]` balloon。
    pub last_balloon: Option<String>,
    /// 前回のシェルのフォルダ名。正準 key `areka.last.shell`・表 `[last]` shell。
    pub last_shell: Option<String>,
    /// 起動中の印（動いているゴーストの名前）。正準 key `areka.last.running`・表 `[last]` running。
    /// 空文字は「無し」と読む（[`parse_toml_str`] が `None` にする）。
    pub last_running: Option<String>,
    /// 根の並び（1 本の文字列・意味は呼び手が持つ）。正準 key `areka.baseware.roots`・表
    /// `[baseware]` roots。空文字は空文字のまま（`last_running` と違い「無し」に潰さない）。
    pub baseware_roots: Option<String>,
    /// 隣り合いの印: スコープ ID → 値の文字列。正準 key `areka.window.scope(ID).adjacent`・
    /// 表 `[window-adjacent]`。値の意味（`left`／`right`／`none`）は呼び手（areka）が持つ。
    pub window_adjacent: BTreeMap<String, String>,
}

/// 窓位置・バルーンオフセットの 1 スコープ分の (x, y)（各軸は文字列・欠落は `None`）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AxisPair {
    /// X 軸（文字列）。欠落時 `None`。
    pub x: Option<String>,
    /// Y 軸（文字列）。欠落時 `None`。
    pub y: Option<String>,
}

impl AxisPair {
    fn is_empty(&self) -> bool {
        self.x.is_none() && self.y.is_none()
    }
}

/// 寛容でない読み。使えれば `Ok((doc, 読んだ表))`、使えなければ `Err(利用者へ見せる理由の文)`。
///
/// 使えない内容の `warn!` はこの関数が出す。
pub(crate) fn parse_toml_str(content: &str) -> Result<(FormatDoc, toml::Table), String> {
    let table: toml::Table = match content.parse::<toml::Table>() {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!(
                target: LOG_TARGET,
                error = %e,
                "persist TOML parse failed; degrading to all-absent (file preserved)"
            );
            return Err("記憶のファイル sylphya.toml が TOML として読めないので、中身を消さないように書くのをやめました（手で直したときの書き誤りなど）".to_string());
        }
    };

    let version = table
        .get("format-version")
        .and_then(toml::Value::as_integer);
    match version {
        Some(v) if v == FORMAT_VERSION => {}
        _ => {
            tracing::warn!(
                target: LOG_TARGET,
                found_version = ?version,
                expected_version = FORMAT_VERSION,
                "persist TOML has unknown/absent format-version; degrading to all-absent (file preserved)"
            );
            let found = version.map_or_else(|| "書かれていない値".to_string(), |v| v.to_string());
            return Err(format!(
                "記憶のファイル sylphya.toml の format-version が {found}で、この areka が読める {FORMAT_VERSION} と違うので、中身を消さないように書くのをやめました（新しい areka で書いたファイルなど）"
            ));
        }
    }

    let last = table.get("last").and_then(toml::Value::as_table);
    let read_last = |field| last.and_then(|t| read_str(t, field));
    let doc = FormatDoc {
        last_ghost: read_last("ghost"),
        last_balloon: read_last("balloon"),
        last_shell: read_last("shell"),
        last_running: read_last("running").filter(|v| !v.is_empty()),
        baseware_roots: table
            .get("baseware")
            .and_then(toml::Value::as_table)
            .and_then(|t| read_str(t, "roots")),
        window: read_axis_map(&table, "window"),
        balloon_offset: read_axis_map(&table, "balloon-offset"),
        boot_count: read_count(&table, "boot"),
        vanish_count: read_count(&table, "vanish"),
        window_adjacent: read_string_map(&table, "window-adjacent"),
    };
    Ok((doc, table))
}

/// `[<key>."ID"]` 群を [`AxisPair`] マップへ（テーブル不在・非テーブルは空マップ）。
fn read_axis_map(root: &toml::Table, key: &str) -> BTreeMap<String, AxisPair> {
    let mut out = BTreeMap::new();
    let Some(table) = root.get(key).and_then(toml::Value::as_table) else {
        return out;
    };
    for (id, entry) in table {
        let Some(axis) = entry.as_table() else {
            continue;
        };
        let pair = AxisPair {
            x: read_str(axis, "x"),
            y: read_str(axis, "y"),
        };
        if !pair.is_empty() {
            out.insert(id.clone(), pair);
        }
    }
    out
}

/// `[<key>]` の `"ID" = "<文字列>"` 群をマップへ（表の不在・非テーブルは空・文字列でない値は無し）。
fn read_string_map(root: &toml::Table, key: &str) -> BTreeMap<String, String> {
    root.get(key)
        .and_then(toml::Value::as_table)
        .map(|t| {
            t.iter()
                .filter_map(|(id, v)| Some((id.clone(), v.as_str()?.to_string())))
                .collect()
        })
        .unwrap_or_default()
}

/// `[<key>]` テーブルの `count`（文字列）を取り出す（不在・型外れは `None`）。
fn read_count(root: &toml::Table, key: &str) -> Option<String> {
    root.get(key)
        .and_then(toml::Value::as_table)
        .and_then(|t| read_str(t, "count"))
}

/// テーブルから文字列 field を取り出す（不在・非文字列は `None`）。
fn read_str(table: &toml::Table, field: &str) -> Option<String> {
    table
        .get(field)
        .and_then(toml::Value::as_str)
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_log_capture::{assert_logged, capture};
    use tracing::Level;

    fn doc(content: &str) -> FormatDoc {
        parse_toml_str(content).unwrap().0
    }

    #[test]
    fn strict_parse_gives_the_reason_for_each_unusable_content() {
        let reason = |content| parse_toml_str(content).unwrap_err();
        assert!(reason("[[[ broken").contains("TOML として読めない"));
        assert!(reason("format-version = 2").contains("format-version が 2で"));
        assert!(reason("format-version = \"1\"").contains("書かれていない値で"));
        assert!(reason("").contains("書かれていない値で"));
        assert_eq!(doc("format-version = 1"), FormatDoc::default());
    }

    #[test]
    fn every_table_is_read() {
        let read = doc("format-version = 1
[window.\"0\"]
x = \"1024\"
y = \"512\"
[balloon-offset.\"1\"]
y = \"-10\"
[boot]
count = \"3\"
[vanish]
count = \"0\"
[last]
ghost = \"emo2\"
balloon = \"b\"
shell = \"master\"
running = \"broken\"
[baseware]
roots = \"\"
[window-adjacent]
\"1\" = \"left\"
");
        assert_eq!(
            read,
            FormatDoc {
                window: BTreeMap::from([(
                    "0".into(),
                    AxisPair {
                        x: Some("1024".into()),
                        y: Some("512".into()),
                    },
                )]),
                balloon_offset: BTreeMap::from([(
                    "1".into(),
                    AxisPair {
                        x: None,
                        y: Some("-10".into()),
                    },
                )]),
                boot_count: Some("3".into()),
                vanish_count: Some("0".into()),
                last_ghost: Some("emo2".into()),
                last_balloon: Some("b".into()),
                last_shell: Some("master".into()),
                last_running: Some("broken".into()),
                baseware_roots: Some(String::new()),
                window_adjacent: BTreeMap::from([("1".into(), "left".into())]),
            }
        );
    }

    #[test]
    fn missing_key_and_type_mismatch_are_per_key_absent() {
        let read = doc("format-version = 1
[window.\"0\"]
x = \"5\"
[boot]
count = 7
[last]
running = \"\"
");
        assert_eq!(read.boot_count, None);
        assert_eq!(read.last_running, None, "空文字の起動中の印は無し");
        assert_eq!(read.window["0"].x.as_deref(), Some("5"));
        assert_eq!(read.window["0"].y, None);
    }

    #[test]
    fn unusable_content_emits_warn() {
        for (content, needle) in [
            (
                "format-version = 99
",
                "unknown/absent format-version",
            ),
            (
                "[boot]
count = \"1\"
",
                "unknown/absent format-version",
            ),
            ("[[[ broken", "parse failed"),
        ] {
            let events = capture(|| {
                let _ = parse_toml_str(content);
            });
            assert_logged(&events, Level::WARN, LOG_TARGET, needle);
        }
    }

    #[test]
    fn valid_version_missing_key_emits_no_warn() {
        let events = capture(|| {
            let _ = parse_toml_str(
                "format-version = 1
[window.\"0\"]
x = \"5\"
",
            );
        });
        assert_eq!(events.iter().filter(|e| e.level == Level::WARN).count(), 0);
    }
}
