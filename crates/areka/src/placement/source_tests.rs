use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::*;
use crate::placement::PlacementError;
use crate::placement::test_support::{ExpectField, capture_logs, expect_one};

/// emo2 実フィクスチャのルート（`crates/pilot/examples/shiori-host-32/fixtures/emo2/`）。
///
/// `CARGO_MANIFEST_DIR`（= `.../crates/areka`）相対で組み立てる
/// （areka-parsers validation_tests と同じ規約・絶対パス埋め込みやフィクスチャ複製をしない）。
fn emo2_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("pilot")
        .join("examples")
        .join("shiori-host-32")
        .join("fixtures")
        .join("emo2")
}

/// このテスト専用の一意な一時ディレクトリを返す（areka-parsers resolve_tests と
/// 同じ規約: 外部クレート tempfile に依存せず `std::env::temp_dir()` 直下へ
/// テスト名タグでユニーク化）。
fn unique_temp_dir(tag: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!("areka_placement_source_tests_{tag}"));
    dir
}

// ------------------------------------------------------------------
// T-I5: emo2 fixture からの end-to-end 読込（統合）
// ------------------------------------------------------------------

/// T-I5: emo2 fixture から `load_descript_source` を呼び、`shell_kv` に
/// 実測キー（`seriko.alignmenttodesktop=bottom` 等・正典表検証行）が含まれる。
#[test]
fn t_i5_emo2_fixture_loads_descript_source() {
    let src =
        load_descript_source(&emo2_root()).expect("emo2 fixture は Ok(DescriptSource) を返す");

    // shell descript 実測キー（正典表「emo2 実測値による検証行」）
    assert_eq!(
        src.shell_kv
            .get("seriko.alignmenttodesktop")
            .map(String::as_str),
        Some("bottom")
    );
    assert_eq!(
        src.shell_kv.get("sakura.defaultx").map(String::as_str),
        Some("0")
    );
    assert_eq!(
        src.shell_kv.get("kero.defaultx").map(String::as_str),
        Some("0")
    );
    assert_eq!(
        src.shell_kv
            .get("sakura.balloon.alignment")
            .map(String::as_str),
        Some("left")
    );
    assert_eq!(
        src.shell_kv
            .get("kero.balloon.alignment")
            .map(String::as_str),
        Some("right")
    );

    // ghost descript 実測キー（scope1 検出シグナル kero.name を含む）
    assert_eq!(
        src.ghost_kv.get("kero.name").map(String::as_str),
        Some("エモ")
    );
    assert_eq!(
        src.ghost_kv.get("sakura.name").map(String::as_str),
        Some("むらさき")
    );

    // shell_dir は resolve の解決値（<root>/shell/master）
    assert_eq!(src.shell_dir, emo2_root().join("shell").join("master"));
}

/// T-I5 補: emo2 の `GhostTitles` は `sakura.name`/`kero.name` を写像し、
/// 未定義スコープは既定 `"areka"` を返す（パニックしない）。
#[test]
fn t_i5_emo2_titles_from_names() {
    let src =
        load_descript_source(&emo2_root()).expect("emo2 fixture は Ok(DescriptSource) を返す");

    assert_eq!(src.titles.title(0), "むらさき");
    assert_eq!(src.titles.title(1), "エモ");
    // emo2 に char2 以降は無い → 既定
    assert_eq!(src.titles.title(2), "areka");
    assert_eq!(src.titles.title(99), "areka");
}

// ------------------------------------------------------------------
// GhostTitles 単体（既定値・char{n}.name 由来）
// ------------------------------------------------------------------

/// 名前情報が全欠落でも全スコープで既定 `"areka"` を返す（常に文字列・panic しない）。
#[test]
fn titles_all_missing_default_to_areka() {
    let titles = build_titles(
        &areka_parsers::package::GhostNames::default(),
        &BTreeMap::new(),
    );
    assert_eq!(titles.title(0), "areka");
    assert_eq!(titles.title(1), "areka");
    assert_eq!(titles.title(7), "areka");
}

/// scope n≥2 のタイトルは ghost descript KV の `char{n}.name` から拾う。
/// `charset` 等の非該当キー・`char0.name`/`char1.name`（scope0/1 の正本は
/// `sakura.name`/`kero.name`）は写像しない。
#[test]
fn titles_char_n_name_from_ghost_kv() {
    let names = areka_parsers::package::GhostNames::default();
    let ghost_kv: BTreeMap<String, String> = [
        ("char2.name", "三人目"),
        ("char10.name", "十人目"),
        ("char0.name", "偽さくら"),
        ("char1.name", "偽けろ"),
        ("charset", "UTF-8"),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect();

    let titles = build_titles(&names, &ghost_kv);
    assert_eq!(titles.title(2), "三人目");
    assert_eq!(titles.title(10), "十人目");
    // scope0/1 は names 由来のみ（欠落 → 既定）
    assert_eq!(titles.title(0), "areka");
    assert_eq!(titles.title(1), "areka");
}

// ------------------------------------------------------------------
// 失敗経路（決定論化可能なもの）
// ------------------------------------------------------------------

/// resolve 失敗（ghost_root 不在）は `Err(PlacementError::Mount)`。
#[test]
fn load_missing_root_returns_mount_err() {
    let root = unique_temp_dir("missing_root_returns_mount_err").join("no_such_ghost");
    let err = load_descript_source(&root).expect_err("不在 root は Err");
    assert!(
        matches!(err, PlacementError::Mount(_)),
        "Mount variant 以外が返った: {err:?}"
    );
}

/// shell descript 読取失敗（shell dir は実在・descript.txt 不在）は
/// `Err(PlacementError::DescriptRead)` で、path が shell 側 descript を指す。
#[test]
fn load_missing_shell_descript_returns_descript_read_err() {
    let root = unique_temp_dir("missing_shell_descript");
    let _ = fs::remove_dir_all(&root);
    let ghost_master = root.join("ghost").join("master");
    fs::create_dir_all(&ghost_master).expect("create ghost/master");
    fs::write(
        ghost_master.join("descript.txt"),
        "charset,UTF-8\nname,テスト\nsakura.name,さくら\n".as_bytes(),
    )
    .expect("write ghost descript");
    // shell/master は dir のみ実在（descript.txt なし）→ resolve は成功する
    let shell_dir = root.join("shell").join("master");
    fs::create_dir_all(&shell_dir).expect("create shell/master");

    let err = load_descript_source(&root).expect_err("shell descript 不在は Err");
    match err {
        PlacementError::DescriptRead { path, .. } => {
            assert_eq!(path, shell_dir.join("descript.txt"));
        }
        other => panic!("DescriptRead variant 以外が返った: {other:?}"),
    }

    let _ = fs::remove_dir_all(&root);
}

/// ghost descript の寛容読取ヘルパ: 読めなければ警告＋空 KV（継続契約の檻）。
#[test]
fn read_kv_lenient_missing_returns_empty() {
    let path = unique_temp_dir("read_kv_lenient_missing").join("descript.txt");
    let kv = read_kv_lenient(&path);
    assert!(kv.is_empty());
}

// ------------------------------------------------------------------
// author_dpi 読取（areka-P0-emo-dpi-scaling task 2.1・要件 1.1・design D1）
// 無宣言=96 / 宣言あり=その値 / 不正=warn+96 / 0=warn+96 の全パターン
// ------------------------------------------------------------------

/// shell_kv だけを差し替えた `DescriptSource`（author_dpi 読取の純関数檻用・I/O なし）。
fn shell_source_with(kv: &[(&str, &str)]) -> DescriptSource {
    DescriptSource {
        ghost_kv: BTreeMap::new(),
        shell_kv: kv
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect(),
        shell_dir: PathBuf::from("shell"),
        titles: GhostTitles::from_scope_titles([]),
    }
}

/// balloon descript.txt を持つ一時 balloon ルートを作る（内容は呼び手指定）。
fn balloon_root_with(tag: &str, descript: &str) -> PathBuf {
    let dir = unique_temp_dir(tag);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create balloon root");
    fs::write(dir.join("descript.txt"), descript.as_bytes()).expect("write balloon descript");
    dir
}

/// 無宣言（キー不在）は既定 96（ukadoc: 何も指定しなければ Windows 標準の 96 固定）。
#[test]
fn parse_author_dpi_absent_is_default_96() {
    assert_eq!(parse_author_dpi(None, "test"), 96);
}

/// 宣言ありは宣言値そのまま（ukadoc 対照表 96/120/144/168/192）。
#[test]
fn parse_author_dpi_declared_values_pass_through() {
    for raw in ["96", "120", "144", "168", "192"] {
        let expected: u16 = raw.parse().expect("テスト入力は u16");
        assert_eq!(parse_author_dpi(Some(raw), "test"), expected, "raw={raw}");
    }
    // 対照表外の任意値も切り捨てない（正典は「推奨 DPI 値」であり列挙ではない）
    assert_eq!(parse_author_dpi(Some("110"), "test"), 110);
}

/// 不正（数値化不能・負・u16 溢れ・空）は既定 96 へ縮退（warn・panic しない）。
#[test]
fn parse_author_dpi_invalid_is_default_96() {
    for raw in [
        "abc", "1x2", "", " ", "-96", "999999", "96.0", "0x60", "96 120",
    ] {
        assert_eq!(parse_author_dpi(Some(raw), "test"), 96, "raw={raw:?}");
    }
}

/// 0 は k の分母に使えない → 既定 96 へ縮退（warn）。
#[test]
fn parse_author_dpi_zero_is_default_96() {
    assert_eq!(parse_author_dpi(Some("0"), "test"), 96);
    assert_eq!(parse_author_dpi(Some("00"), "test"), 96);
}

/// `shell_author_dpi` は shell descript の `seriko.dpi` を読む（無宣言/宣言/不正/0）。
#[test]
fn shell_author_dpi_reads_seriko_dpi_all_patterns() {
    assert_eq!(shell_source_with(&[]).shell_author_dpi(), 96);
    assert_eq!(
        shell_source_with(&[("seriko.dpi", "120")]).shell_author_dpi(),
        120
    );
    assert_eq!(
        shell_source_with(&[("seriko.dpi", "abc")]).shell_author_dpi(),
        96
    );
    assert_eq!(
        shell_source_with(&[("seriko.dpi", "0")]).shell_author_dpi(),
        96
    );
    // balloon 側キー `dpi` は shell 側の正本ではない（キー取り違えの檻）
    assert_eq!(shell_source_with(&[("dpi", "192")]).shell_author_dpi(), 96);
}

/// emo2 実フィクスチャの shell descript は `seriko.dpi` 無宣言 → 96（既存期待値不変）。
#[test]
fn shell_author_dpi_emo2_fixture_is_default_96() {
    let src =
        load_descript_source(&emo2_root()).expect("emo2 fixture は Ok(DescriptSource) を返す");
    assert_eq!(src.shell_author_dpi(), 96);
}

/// balloon descript 不在（ファイル不在）は既定 96（lenient・panic しない）。
#[test]
fn load_balloon_author_dpi_missing_file_is_default_96() {
    let root = unique_temp_dir("balloon_dpi_missing_file").join("no_such_balloon");
    assert_eq!(load_balloon_author_dpi(&root), 96);
}

/// balloon descript の `dpi` を読む（無宣言/宣言/不正/0）。
#[test]
fn load_balloon_author_dpi_reads_dpi_all_patterns() {
    let absent = balloon_root_with("balloon_dpi_absent", "charset,UTF-8\nname,かくかく\n");
    assert_eq!(load_balloon_author_dpi(&absent), 96);
    let _ = fs::remove_dir_all(&absent);

    let declared = balloon_root_with("balloon_dpi_declared", "charset,UTF-8\ndpi,144\n");
    assert_eq!(load_balloon_author_dpi(&declared), 144);
    let _ = fs::remove_dir_all(&declared);

    let invalid = balloon_root_with("balloon_dpi_invalid", "charset,UTF-8\ndpi,abc\n");
    assert_eq!(load_balloon_author_dpi(&invalid), 96);
    let _ = fs::remove_dir_all(&invalid);

    let zero = balloon_root_with("balloon_dpi_zero", "charset,UTF-8\ndpi,0\n");
    assert_eq!(load_balloon_author_dpi(&zero), 96);
    let _ = fs::remove_dir_all(&zero);

    // shell 側キー `seriko.dpi` は balloon 側の正本ではない（キー取り違えの檻）
    let wrong_key = balloon_root_with("balloon_dpi_wrong_key", "charset,UTF-8\nseriko.dpi,192\n");
    assert_eq!(load_balloon_author_dpi(&wrong_key), 96);
    let _ = fs::remove_dir_all(&wrong_key);
}

/// emo2 実フィクスチャの balloon（emo2-kakukaku）は `dpi` 無宣言 → 96（既存期待値不変）。
#[test]
fn load_balloon_author_dpi_emo2_fixture_is_default_96() {
    let balloon_root = emo2_root().join("emo2-kakukaku");
    assert!(
        balloon_root.join("descript.txt").is_file(),
        "emo2 balloon fixture が見つからない: {}",
        balloon_root.display()
    );
    assert_eq!(load_balloon_author_dpi(&balloon_root), 96);
}

// ------------------------------------------------------------------
// author_dpi 縮退ログの発火（task 6.2・steering `logging.md` の
// 「ログ無し失敗経路の禁止」）
//
// task 2.1 の檻は**戻り値だけ**を見ており、無宣言=debug・不正/0=warn という
// 縮退梯子のレベル分離と `source` フィールドによる shell/balloon 帰属
// （[`load_balloon_author_dpi`] の doc が明示的に主張している契約）は無検査だった。
//
// 捕捉は共有ハーネス [`crate::placement::test_support`]（`#[cfg(test)]` 限定）を使う。
// **素朴な `with_default` 捕捉は非決定的に取りこぼす**——`tracing` の callsite interest
// キャッシュはプロセス大域かつ「最初に踏んだスレッドが勝つ」ため、subscriber を持たない
// 他テスト（`read_kv_lenient_missing_returns_empty` 等）が同じ callsite を先に踏むと
// `Interest::never()` が焼き付き、捕捉窓の内側でもイベントが捨てられる。
// 機構と対策（probe dispatcher 常駐による `has_just_one` 恒久偽化）は共有機構
// `log_capture_kit` の crate doc（および `test_support` のモジュール doc）を参照。
// ------------------------------------------------------------------

/// 無宣言は **`debug!`**（正典の既定＝異常ではない）で、`source`／`default_dpi` を残す。
///
/// 無宣言を `warn!` へ格上げする実装（emo2 を含む正典既定のゴーストが毎回警告を吐く）と、
/// 無言で 96 を返す実装（縮退が観測できない）の双方をここで落とす。
#[test]
fn parse_author_dpi_absent_logs_debug_with_source() {
    let (dpi, events) = capture_logs(|| parse_author_dpi(None, SHELL_DPI_KEY));
    assert_eq!(dpi, DEFAULT_AUTHOR_DPI);

    let ev = expect_one(&events, "宣言なし");
    assert_eq!(
        ev.level,
        tracing::Level::DEBUG,
        "無宣言は正典の既定＝異常ではない（warn 格上げ禁止）: {ev:?}"
    );
    assert_eq!(ev.expect_field("source"), SHELL_DPI_KEY);
    assert_eq!(ev.expect_field("default_dpi"), "96");
    assert_eq!(events.len(), 1, "1 分岐 1 ログ: {events:?}");
}

/// 正常宣言は**完全に無言**（無宣言/不正のレベル主張の非空虚性を担保する陰性対照）。
#[test]
fn parse_author_dpi_declared_is_silent() {
    for raw in ["96", "120", "144", "168", "192", "110", "65535"] {
        let expected: u16 = raw.parse().expect("テスト入力は u16");
        let (dpi, events) = capture_logs(|| parse_author_dpi(Some(raw), SHELL_DPI_KEY));
        assert_eq!(dpi, expected, "raw={raw}");
        assert!(events.is_empty(), "正常宣言は無言（raw={raw}）: {events:?}");
    }
}

/// 数値化不能は **`warn!`**＋`source`／`raw`／`error`（parse エラー）／`default_dpi`。
///
/// `raw` が載ることで「どの生値が捨てられたか」が実機ログから判る（無言縮退の禁止）。
#[test]
fn parse_author_dpi_invalid_logs_warn_with_source_and_raw() {
    for raw in ["abc", "", " 120 ", "-96", "65536", "96.0", "0x60"] {
        let (dpi, events) = capture_logs(|| parse_author_dpi(Some(raw), BALLOON_DPI_KEY));
        assert_eq!(dpi, DEFAULT_AUTHOR_DPI, "raw={raw:?}");

        let ev = expect_one(&events, "数値として解釈できない");
        assert_eq!(ev.level, tracing::Level::WARN, "raw={raw:?}: {ev:?}");
        assert_eq!(ev.expect_field("source"), BALLOON_DPI_KEY);
        assert_eq!(ev.expect_field("raw"), raw, "捨てた生値をそのまま残す");
        assert_eq!(ev.expect_field("default_dpi"), "96");
        assert!(
            ev.field("error").is_some(),
            "parse エラーを載せる（raw={raw:?}）: {ev:?}"
        );
        assert_eq!(events.len(), 1, "raw={raw:?}: {events:?}");
    }
}

/// `0` は **`warn!`** だが「数値化不能」とは**別メッセージ**（0 は解釈可能値であり、
/// 分母に使えないことが理由——実機ログで両者を取り違えないための識別子）。
#[test]
fn parse_author_dpi_zero_logs_warn_distinct_from_invalid() {
    for raw in ["0", "00"] {
        let (dpi, events) = capture_logs(|| parse_author_dpi(Some(raw), SHELL_DPI_KEY));
        assert_eq!(dpi, DEFAULT_AUTHOR_DPI, "raw={raw}");

        let ev = expect_one(&events, "表示スケールの分母に使えない");
        assert_eq!(ev.level, tracing::Level::WARN, "raw={raw}: {ev:?}");
        assert_eq!(ev.expect_field("source"), SHELL_DPI_KEY);
        assert_eq!(ev.expect_field("raw"), raw);
        assert_eq!(ev.expect_field("default_dpi"), "96");
        assert!(
            !ev.message().contains("数値として解釈できない"),
            "0 は解釈可能値ゆえ不正値と同じ文言にしない: {ev:?}"
        );
        assert_eq!(events.len(), 1, "raw={raw}: {events:?}");
    }
}

/// u16 境界の厳密確認（1 の差で受理／縮退が入れ替わる）。
///
/// 「極端に大きい値」を u16 上限で切るのか別閾値で切るのかは実装の契約であり、
/// 65535 受理・65536 縮退の対で固定する。
#[test]
fn parse_author_dpi_u16_boundary_is_exact() {
    assert_eq!(
        parse_author_dpi(Some("65535"), "test"),
        65535,
        "u16 上限ちょうどは素通し（正典は列挙ではなく推奨値）"
    );
    assert_eq!(
        parse_author_dpi(Some("65536"), "test"),
        96,
        "u16 溢れは既定へ縮退"
    );
    assert_eq!(parse_author_dpi(Some("1"), "test"), 1, "非ゼロ最小値は受理");
    // Rust の u16 パーサ準拠の受理形（現状挙動の固定・ukadoc は書式を規定しない）。
    assert_eq!(parse_author_dpi(Some("+120"), "test"), 120, "符号付き正値");
    assert_eq!(parse_author_dpi(Some("096"), "test"), 96, "前置ゼロ");
    // 前後空白は trim せず数値化不能扱い＝**縮退**（返り値 96 は正常宣言の 96 と
    // 数値的に区別できないため、縮退経路であることは
    // [`parse_author_dpi_invalid_logs_warn_with_source_and_raw`] の warn 検査が弁別する）。
    // なお本番経路ではここへ空白付きの値は届かない——`areka_parsers::kv::parse`
    // （`kv/parse.rs:31`/`:39`）がキー・値とも trim 済みで渡す。挙動の固定のみが目的。
    assert_eq!(parse_author_dpi(Some(" 120 "), "test"), 96);
}

/// [`load_balloon_author_dpi`] の doc が主張する契約:
/// **読取器は 1 本のまま**で、shell か balloon かの帰属は `source` フィールドで区別できる。
///
/// shell 経路は `seriko.dpi`・balloon 経路は `dpi` を `source` に載せる。
/// `source` を定数直書きにしたり、両アクセサのキーを取り違える変異はここで落ちる。
#[test]
fn author_dpi_log_source_field_attributes_shell_vs_balloon() {
    let (shell_dpi, shell_events) =
        capture_logs(|| shell_source_with(&[("seriko.dpi", "abc")]).shell_author_dpi());
    assert_eq!(shell_dpi, DEFAULT_AUTHOR_DPI);
    assert_eq!(
        expect_one(&shell_events, "数値として解釈できない").expect_field("source"),
        "seriko.dpi",
        "shell 起因のログは seriko.dpi に帰属する"
    );

    let zero = balloon_root_with("balloon_dpi_zero_log", "charset,UTF-8\ndpi,0\n");
    let (balloon_dpi, balloon_events) = capture_logs(|| load_balloon_author_dpi(&zero));
    assert_eq!(balloon_dpi, DEFAULT_AUTHOR_DPI);
    assert_eq!(
        expect_one(&balloon_events, "表示スケールの分母に使えない").expect_field("source"),
        "dpi",
        "balloon 起因のログは dpi に帰属する"
    );
    let _ = fs::remove_dir_all(&zero);
}

/// balloon descript 不在は **2 本**のログ（読取失敗 `warn!`＋パス、無宣言 `debug!`）を残し、
/// 読取失敗の文言は**帰属中立**である（[`read_kv_lenient`] は ghost/balloon 共有の読取器）。
///
/// ファイルが無かったのか宣言が無かったのかを実機ログで弁別できることが縮退の観測条件。
/// 加えて、共有読取器の文言を ghost 固定に戻す変異（バルーン起因を ghost 起因に見せる
/// 帰属誤り・R6.3 の `RUST_LOG` grep を誤らせる）を「ghost を含まない」主張で殺す。
#[test]
fn load_balloon_author_dpi_missing_file_logs_read_warn_and_absent_debug() {
    let root = unique_temp_dir("balloon_dpi_missing_file_log").join("no_such_balloon");
    let (dpi, events) = capture_logs(|| load_balloon_author_dpi(&root));
    assert_eq!(dpi, DEFAULT_AUTHOR_DPI);

    let read_fail = expect_one(&events, "読み取りに失敗");
    assert_eq!(read_fail.level, tracing::Level::WARN, "{read_fail:?}");
    assert!(
        read_fail.expect_field("path").contains("no_such_balloon"),
        "失敗したパスを残す: {read_fail:?}"
    );
    assert!(
        !read_fail.message().contains("ghost"),
        "共有読取器の失敗文言は帰属中立（balloon 経路で ghost 起因に見えてはならない）: {read_fail:?}"
    );

    let absent = expect_one(&events, "宣言なし");
    assert_eq!(absent.level, tracing::Level::DEBUG, "{absent:?}");
    assert_eq!(absent.expect_field("source"), BALLOON_DPI_KEY);

    assert_eq!(events.len(), 2, "読取失敗と無宣言の 2 本: {events:?}");
}

/// balloon descript が読めて宣言もあれば**無言**で宣言値を返す
/// （実ファイル経路でも正常系にログを撒かない）。
#[test]
fn load_balloon_author_dpi_declared_file_is_silent() {
    let declared = balloon_root_with("balloon_dpi_declared_log", "charset,UTF-8\ndpi,192\n");
    let (dpi, events) = capture_logs(|| load_balloon_author_dpi(&declared));
    assert_eq!(dpi, 192);
    assert!(events.is_empty(), "正常経路は無言: {events:?}");
    let _ = fs::remove_dir_all(&declared);
}

/// `shell_author_dpi` は**実ファイル**（descript.txt → decode → parse_kv）経由でも
/// 宣言値を運ぶ（`shell_source_with` の in-memory 檻が通す経路の end-to-end 確認）。
///
/// `seriko.dpi` が KV パーサで落ちる／キー名がずれる変異を、実 I/O 込みで落とす。
#[test]
fn shell_author_dpi_reads_declared_value_from_real_descript_file() {
    let root = unique_temp_dir("shell_author_dpi_declared_file");
    let _ = fs::remove_dir_all(&root);
    let ghost_master = root.join("ghost").join("master");
    fs::create_dir_all(&ghost_master).expect("create ghost/master");
    fs::write(
        ghost_master.join("descript.txt"),
        "charset,UTF-8\nname,テスト\nsakura.name,さくら\n".as_bytes(),
    )
    .expect("write ghost descript");
    let shell_dir = root.join("shell").join("master");
    fs::create_dir_all(&shell_dir).expect("create shell/master");
    fs::write(
        shell_dir.join("descript.txt"),
        "charset,UTF-8\nseriko.dpi,144\n".as_bytes(),
    )
    .expect("write shell descript");

    let src = load_descript_source(&root).expect("shell descript があれば Ok");
    assert_eq!(
        src.shell_author_dpi(),
        144,
        "descript の実宣言値（既定 96 の素通しではない）"
    );

    let _ = fs::remove_dir_all(&root);
}

/// 寛容読取ヘルパは読めれば通常どおり decode→parse_kv する（Ansi 既定・宣言優先）。
#[test]
fn read_kv_lenient_reads_utf8_declared_file() {
    let dir = unique_temp_dir("read_kv_lenient_reads");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create dir");
    let path = dir.join("descript.txt");
    fs::write(&path, "charset,UTF-8\nname,えも？？\n".as_bytes()).expect("write");

    let kv = read_kv_lenient(&path);
    assert_eq!(kv.get("name").map(String::as_str), Some("えも？？"));

    let _ = fs::remove_dir_all(&dir);
}
