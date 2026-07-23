//! TOML 直列化形式（`format-version = 1`）と寛容読取（R6.3・R6.4）。
//!
//! 1 スコープの永続状態（`<scope root>/sylphya.toml`）の **format 層**——スキーマ定義・
//! 直列化・寛容パース——を担う。この層は語彙台帳（Major 2）にも鏡像（Major 3）にも依存
//! しない自己完結モジュールであり、std ＋ `toml` ＋ `tracing` のみに依る（design
//! 「永続層 → 自己完結モジュール」）。
//!
//! ## 物理スキーマ（design「Physical Data Model」の正本）
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
//! ```
//!
//! - `format-version` は最上位の **整数 key**（本バージョン = [`FORMAT_VERSION`] = 1）。
//! - `window` / `balloon-offset` の各エントリ key はスコープ ID（本 format 層では不透明文字列。
//!   typed な [`u32`] スコープへの写像は Task 4.3 の `PersistKey` の領分）。
//! - 値はすべて TOML 文字列（design「値はすべて TOML 文字列・R6.6 の往復自明性」）。
//!
//! ## 寛容読取 3 段（design「永続層 → Responsibilities」・R6.3/R6.4）
//!
//! [`read_toml_str`] は **content 段**（下記 2・3）を担う。ファイル不在段（debug＋全不在）は
//! IO 層（Task 4.2）の領分ゆえここには無い。
//!
//! 1. （IO 層）ファイル不在 → debug ＋ 全不在。
//! 2. **parse 失敗**（不正 TOML）／**未知・欠落 `format-version`**（≠ 1）→ `warn!` ＋ 全不在
//!    （[`FormatDoc::default`]）。呼び出し側はファイルを削除しない（削除は IO 関心）。未知
//!    バージョンの warn は「旧形式判別可能」（R6.4）の唯一の観測点。
//! 3. **有効バージョン・key 欠落** → 当該 key のみ不在（`None`）。欠落 key に warn は出さない
//!    （単なる不在）。型外れ（文字列でない値）も当該 key 不在として寛容に無視する。
//!
//! いかなる入力でも panic しない（R6.7・R8.1「無音失敗なし」——縮退は必ず warn 記録）。

use std::collections::BTreeMap;

/// 本モジュールが読み書きする TOML 形式のバージョン（design「Physical Data Model」）。
///
/// `format-version` 増分時は新旧判別のうえ読取側でマイグレーションする（旧形式は warn ＋不在
/// 縮退が最低保証・R6.4）。[`read_toml_str`] はこの値に一致するファイルのみ本文を採用する。
pub const FORMAT_VERSION: i64 = 1;

/// ログ target（steering: areka-log-first-no-silent-failure）。
const LOG_TARGET: &str = "areka_sylphya::persist::format";

/// 1 スコープの永続ドキュメントの中立 in-memory 表現（`sylphya.toml` 相当）。
///
/// 4 key 族（design「Physical Data Model」）を format 層の素の器として保持する:
/// 窓位置（`window`）・バルーン相対オフセット（`balloon_offset`）・起動記録（`boot_count`）・
/// vanish 回数（`vanish_count`）。値はすべて文字列（R6.6 の往復自明性）。スコープ ID は
/// 不透明文字列（typed 化は Task 4.3）。
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
}

impl FormatDoc {
    /// 全 key 族が不在か（寛容読取の「全不在」縮退＝空 doc の判定）。
    ///
    /// parse 失敗・未知/欠落バージョンの縮退結果（[`FormatDoc::default`]）はこれが `true`。
    pub fn is_all_absent(&self) -> bool {
        self.window.is_empty()
            && self.balloon_offset.is_empty()
            && self.boot_count.is_none()
            && self.vanish_count.is_none()
    }
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

/// [`FormatDoc`] を `format-version = 1` の TOML 文字列へ直列化する（design スキーマ）。
///
/// 値はすべて TOML 文字列で出力する。空のマップ・`None` の key はテーブル自体を省略する
/// （空テーブルを書かない）。[`BTreeMap`] の昇順走査ゆえ出力は決定論的。
pub fn to_toml_string(doc: &FormatDoc) -> String {
    let mut root = toml::Table::new();
    root.insert("format-version".into(), toml::Value::Integer(FORMAT_VERSION));

    if let Some(t) = axis_map_to_table(&doc.window) {
        root.insert("window".into(), toml::Value::Table(t));
    }
    if let Some(t) = axis_map_to_table(&doc.balloon_offset) {
        root.insert("balloon-offset".into(), toml::Value::Table(t));
    }
    if let Some(c) = &doc.boot_count {
        root.insert("boot".into(), count_table(c));
    }
    if let Some(c) = &doc.vanish_count {
        root.insert("vanish".into(), count_table(c));
    }

    // toml の直列化は本 Table 構造では失敗しない（全値が String/Integer/Table）。万一の
    // 失敗も panic させず、空 doc の最小形へ縮退する（R6.7・無音失敗なし）。
    toml::to_string(&root).unwrap_or_else(|e| {
        tracing::error!(
            target: LOG_TARGET,
            error = %e,
            "TOML serialize failed; emitting minimal format-version-only document"
        );
        format!("format-version = {FORMAT_VERSION}\n")
    })
}

/// スコープ ID → [`AxisPair`] のマップを TOML テーブルへ（全空なら `None`＝テーブル省略）。
fn axis_map_to_table(map: &BTreeMap<String, AxisPair>) -> Option<toml::Table> {
    let mut table = toml::Table::new();
    for (id, pair) in map {
        if pair.is_empty() {
            continue;
        }
        let mut axis = toml::Table::new();
        if let Some(x) = &pair.x {
            axis.insert("x".into(), toml::Value::String(x.clone()));
        }
        if let Some(y) = &pair.y {
            axis.insert("y".into(), toml::Value::String(y.clone()));
        }
        table.insert(id.clone(), toml::Value::Table(axis));
    }
    if table.is_empty() {
        None
    } else {
        Some(table)
    }
}

/// `count = "<v>"` の 1 key テーブル（`[boot]` / `[vanish]` 用）。
fn count_table(value: &str) -> toml::Value {
    let mut t = toml::Table::new();
    t.insert("count".into(), toml::Value::String(value.to_string()));
    toml::Value::Table(t)
}

/// TOML 文字列を寛容にパースして [`FormatDoc`] を得る（寛容読取の content 段・R6.3/R6.4）。
///
/// - parse 失敗 → `warn!` ＋ 全不在（空 doc）。panic しない。
/// - `format-version` 欠落・≠ [`FORMAT_VERSION`] → `warn!`（旧形式判別可能・R6.4）＋ 全不在。
/// - 有効バージョン・key 欠落 → 当該 key のみ `None`（warn なし）。
///
/// 同一 content からは常に同一 doc を返す（決定論・R2.5 の永続版）。
pub fn read_toml_str(content: &str) -> FormatDoc {
    let table: toml::Table = match content.parse::<toml::Table>() {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!(
                target: LOG_TARGET,
                error = %e,
                "persist TOML parse failed; degrading to all-absent (file preserved)"
            );
            return FormatDoc::default();
        }
    };

    // 未知/欠落バージョンは全不在（R6.4「未知バージョン→warn・旧形式判別可能」）。
    let version = table.get("format-version").and_then(toml::Value::as_integer);
    match version {
        Some(v) if v == FORMAT_VERSION => {}
        _ => {
            tracing::warn!(
                target: LOG_TARGET,
                found_version = ?version,
                expected_version = FORMAT_VERSION,
                "persist TOML has unknown/absent format-version; degrading to all-absent (file preserved)"
            );
            return FormatDoc::default();
        }
    }

    // 有効バージョン: 存在する key のみ採用（欠落・型外れは当該 key 不在・warn なし）。
    FormatDoc {
        window: read_axis_map(&table, "window"),
        balloon_offset: read_axis_map(&table, "balloon-offset"),
        boot_count: read_count(&table, "boot"),
        vanish_count: read_count(&table, "vanish"),
    }
}

/// `[<key>."ID"]` 群を [`AxisPair`] マップへ（テーブル不在・非テーブルは空マップ）。
fn read_axis_map(root: &toml::Table, key: &str) -> BTreeMap<String, AxisPair> {
    let mut out = BTreeMap::new();
    let Some(table) = root.get(key).and_then(toml::Value::as_table) else {
        return out;
    };
    for (id, entry) in table {
        let Some(axis) = entry.as_table() else {
            continue; // 型外れエントリは当該スコープ不在として無視。
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

    // --- 直列化・往復・寛容読取の決定論檻（R6.3/R6.4/R6.6・design「Validation」）---

    #[test]
    fn round_trip_preserves_all_string_values() {
        let doc = FormatDoc {
            window: BTreeMap::from([(
                "0".into(),
                AxisPair { x: Some("1024".into()), y: Some("512".into()) },
            )]),
            balloon_offset: BTreeMap::from([(
                "0".into(),
                AxisPair { x: Some("30".into()), y: Some("-10".into()) },
            )]),
            boot_count: Some("3".into()),
            vanish_count: Some("0".into()),
        };

        let toml = to_toml_string(&doc);
        let back = read_toml_str(&toml);
        assert_eq!(doc, back);
    }

    #[test]
    fn round_trip_multiple_scopes_and_negative_values() {
        let mut doc = FormatDoc::default();
        doc.window.insert("0".into(), AxisPair { x: Some("-5".into()), y: Some("0".into()) });
        doc.window.insert("1".into(), AxisPair { x: Some("100".into()), y: Some("200".into()) });
        doc.balloon_offset.insert("1".into(), AxisPair { x: None, y: Some("-10".into()) });

        let back = read_toml_str(&to_toml_string(&doc));
        assert_eq!(doc, back);
    }

    #[test]
    fn serialized_output_carries_format_version_1() {
        let toml = to_toml_string(&FormatDoc::default());
        assert!(toml.contains("format-version = 1"), "output=\n{toml}");
    }

    #[test]
    fn string_values_survive_as_strings() {
        // "1024" は往復後も文字列 "1024"（整数化しない）。
        let doc = FormatDoc { boot_count: Some("1024".into()), ..FormatDoc::default() };
        let back = read_toml_str(&to_toml_string(&doc));
        assert_eq!(back.boot_count.as_deref(), Some("1024"));
    }

    #[test]
    fn parse_failure_yields_all_absent_no_panic() {
        let doc = read_toml_str("this is = = not valid toml [[[");
        assert!(doc.is_all_absent());
    }

    #[test]
    fn unknown_version_yields_all_absent() {
        // 有効 TOML だが format-version = 2（未知）→ 本文があっても全不在へ縮退。
        let doc = read_toml_str("format-version = 2\n[boot]\ncount = \"7\"\n");
        assert!(doc.is_all_absent());
    }

    #[test]
    fn missing_version_yields_all_absent() {
        let doc = read_toml_str("[boot]\ncount = \"7\"\n");
        assert!(doc.is_all_absent());
    }

    #[test]
    fn empty_content_yields_all_absent() {
        assert!(read_toml_str("").is_all_absent());
    }

    #[test]
    fn missing_key_is_per_key_absent() {
        // 有効バージョン・boot テーブル欠落 → boot_count None、window は解決される。
        let doc = read_toml_str("format-version = 1\n[window.\"0\"]\nx = \"5\"\n");
        assert_eq!(doc.boot_count, None);
        assert_eq!(doc.vanish_count, None);
        assert_eq!(doc.window.get("0").and_then(|p| p.x.clone()), Some("5".into()));
        assert_eq!(doc.window.get("0").and_then(|p| p.y.clone()), None);
        assert!(doc.balloon_offset.is_empty());
    }

    #[test]
    fn type_mismatch_value_is_absent_not_panic() {
        // count が整数（型外れ）→ 当該 key 不在（panic しない・寛容）。
        let doc = read_toml_str("format-version = 1\n[boot]\ncount = 7\n");
        assert_eq!(doc.boot_count, None);
    }

    #[test]
    fn determinism_same_content_same_outcome() {
        let content = "format-version = 1\n[boot]\ncount = \"3\"\n[vanish]\ncount = \"1\"\n";
        assert_eq!(read_toml_str(content), read_toml_str(content));
    }

    #[test]
    fn determinism_same_doc_same_serialization() {
        let mut doc = FormatDoc::default();
        doc.window.insert("2".into(), AxisPair { x: Some("9".into()), y: None });
        doc.window.insert("0".into(), AxisPair { x: Some("1".into()), y: Some("2".into()) });
        assert_eq!(to_toml_string(&doc), to_toml_string(&doc));
    }

    // --- 未知バージョンの warn 発火檻（R6.4「未知バージョン→warn・旧形式判別可能」）---
    //
    // ログ捕捉は共有ヘルパ [`crate::test_log_capture`] を経由する。並列 `cargo test` 負荷下で
    // tracing callsite の Interest が `Never` に焼き付いて warn が偽消失する確率欠陥は、当該
    // ヘルパの interest-keeper（プロセスグローバル bare registry 常駐）で根絶される（Task 4.1
    // flaky 根治・R9.1「テスト可能領域は決定論的に檻へ」）。

    #[test]
    fn unknown_version_emits_warn() {
        let events = capture(|| {
            let _ = read_toml_str("format-version = 99\n[boot]\ncount = \"1\"\n");
        });
        // 未知バージョンの縮退アームが LOG_TARGET・WARN で「unknown/absent format-version」を
        // 発火することを表明（ログ削除・語彙変更・レベル降格で赤になる）。
        assert_logged(&events, Level::WARN, LOG_TARGET, "unknown/absent format-version");
    }

    #[test]
    fn parse_failure_emits_warn() {
        let events = capture(|| {
            let _ = read_toml_str("[[[ broken");
        });
        // parse 失敗の縮退アームが LOG_TARGET・WARN で「parse failed」を発火することを表明。
        assert_logged(&events, Level::WARN, LOG_TARGET, "parse failed");
    }

    #[test]
    fn valid_version_missing_key_emits_no_warn() {
        // key 欠落は単なる不在——warn を出さない（design「key 欠落→当該 key 不在」）。
        let events = capture(|| {
            let _ = read_toml_str("format-version = 1\n[window.\"0\"]\nx = \"5\"\n");
        });
        let warns = events.iter().filter(|e| e.level == Level::WARN).count();
        assert_eq!(warns, 0, "key 欠落で warn が出た。捕捉={:?}", events
            .iter()
            .map(|e| (e.level, e.message.clone()))
            .collect::<Vec<_>>());
    }
}
