//! 面の画像の欠け（Error）と無い単独の番号（Warning）を持つ検体の CSV の中身: 終了コード 2・BOM・
//! CRLF・4 列と `""`・行の数・レベル・同じ本文の行が無い・起動オプションの名前の記録（`[cli]`）が
//! 入らない（要件 7.1〜7.5・8.1）。写しの層（`CaptureLayer`）は exe の中にだけ在るので、ここが
//! CSV の中身の確かめの場所になる（tasks.md 6.2 の申し送り）。

use crate::common::{BASE, RED, build_ghost, run_areka};

/// 形の誤り 3 つ（知らない `animation-sort`・描かない method の pattern（面 0）・入れ子の循環
/// （面 10））と、絵の欠け（面 5・土台は描けるので合成は失敗しない）を持つ。
const SURFACES: &[&str] = &[
    "animation-sort,sideways",
    "surface0",
    "{",
    "element0,overlay,base.png,0,0",
    "animation0.interval,bind",
    "animation0.pattern0,replace,20,0,0,0",
    "}",
    "surface5",
    "{",
    "element0,overlay,base.png,0,0",
    "element1,overlay,nofile.png,8,0",
    "}",
    "surface10",
    "{",
    "element0,overlay,red.png,0,0",
    "animation0.interval,bind",
    "animation0.pattern0,overlay,10,0,0,0",
    "}",
    "surface20",
    "{",
    "element0,overlay,red.png,0,0",
    "}",
];

/// CSV の 1 行（囲みを外した 4 列）。
struct Row {
    time: String,
    ghost: String,
    level: String,
    body: String,
}

/// `"時刻","ゴースト名","レベル","内容"` の 1 行を読む。列の中の `""` は `"` に戻す。
fn parse_row(line: &str) -> Row {
    let inner = line
        .strip_prefix('"')
        .and_then(|l| l.strip_suffix('"'))
        .unwrap_or_else(|| panic!("各列を \" で囲む: {line}"));
    let cols: Vec<String> = inner
        .splitn(4, "\",\"")
        .map(|c| c.replace("\"\"", "\""))
        .collect();
    assert_eq!(cols.len(), 4, "4 列: {line}");
    Row {
        time: cols[0].clone(),
        ghost: cols[1].clone(),
        level: cols[2].clone(),
        body: cols[3].clone(),
    }
}

#[test]
fn warning_and_error_dump_writes_one_csv_line_per_record_and_exits_two() {
    let root = build_ghost(
        "offline-exe-errors",
        &["name,g \"err\""],
        SURFACES,
        &[BASE, RED],
    );
    let csv = root.child("errors.csv");
    let (code, stderr) = run_areka(
        root.path(),
        &[
            "--offline-dump",
            &root.path().display().to_string(),
            "--dump-surface-list",
            "0,5,10,20,999",
            "--dump-output-dir",
            &root.child("out").display().to_string(),
            "--dump-error-log",
            &csv.display().to_string(),
            // 起動オプションの名前の読み取りの記録は CSV に入らない（7.3）。
            "--no-such-option",
            "--dump-animation",
            "x",
        ],
    );
    assert_eq!(code, 2, "Error があれば 2: {stderr}");

    let bytes = std::fs::read(&csv).expect("記録があれば CSV を書く");
    let text = std::str::from_utf8(&bytes).expect("UTF-8");
    let text = text.strip_prefix('\u{FEFF}').expect("BOM 付き");
    assert!(text.ends_with("\r\n"), "行の終わりは CRLF");
    assert!(
        !text.replace("\r\n", "").contains(['\r', '\n']),
        "CRLF 以外の改行が無い: {text:?}"
    );
    let rows: Vec<Row> = text.split_terminator("\r\n").map(parse_row).collect();

    // 形の誤り 3・絵の欠け 1・無い単独の番号 1。
    assert_eq!(rows.len(), 5, "1 誤り 1 行: {text}");
    for row in &rows {
        let t = row.time.as_bytes();
        assert!(
            t.len() == 8 && t[2] == b':' && t[5] == b':',
            "時刻は HH:MM:SS: {}",
            row.time
        );
        assert_eq!(row.ghost, "g \"err\"", "ゴースト名は descript.txt の name");
        assert!(
            !row.body.contains("[cli]"),
            "[cli] の記録は入らない: {}",
            row.body
        );
    }
    let mut bodies: Vec<&str> = rows.iter().map(|r| r.body.as_str()).collect();
    bodies.sort();
    bodies.dedup();
    assert_eq!(bodies.len(), rows.len(), "同じ本文の行が無い: {text}");

    let errors: Vec<&Row> = rows.iter().filter(|r| r.level == "Error").collect();
    assert_eq!(errors.len(), 1, "Error は絵の欠けの 1 行: {text}");
    assert!(errors[0].body.contains("nofile.png"), "{}", errors[0].body);
    assert_eq!(
        rows.iter().filter(|r| r.level == "Warning").count(),
        4,
        "ほかは Warning: {text}"
    );
    for want in [
        "animation-sort",
        "描けない描画メソッド",
        "循環",
        "--dump-surface-list で指した面がシェルに無い",
    ] {
        assert_eq!(
            rows.iter()
                .filter(|r| r.level == "Warning" && r.body.contains(want))
                .count(),
            1,
            "「{want}」の Warning は 1 行: {text}"
        );
    }
}
