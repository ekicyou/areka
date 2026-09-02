//! areka 側で起床の旗を立てる側（生産者）が現に在るかの字面検査。
//!
//! 判断の中身は見ない——回すか省略するかの正しさは wintf の純関数の全組合せが見ており、
//! ここが守るのは**配線の抜け**だけである。旗を立て忘れると、門は正しく判断したまま
//! 反応しない画面更新を作ってしまう（発話中に表示が固まる形）。
//!
//! wintf 側の同種の検査（`tick_gate_tests.rs` の `WINTF_PRODUCERS`）は別クレートの
//! ファイルを読めないため、areka の生産者はこの表が受け持つ。両者は同じ書き方
//! （`include_str!` ＋註釈行の除去＋期待する旗の名前）で揃えてある。
//!
//! 「立っていないはず」は**主張しない**。旗はプロセスで 1 組しか無く、本番経路が同じ
//! 旗を立てるため、そちらの主張は他の検査の巻き添えで揺れる（tasks.md「(3.4)」）。

/// 旗を立てる側（areka 内）の一覧（見出し・中身・期待する旗の名前）。並びは見出しの辞書順。
///
/// パスは本ファイル（`crates/areka/src/`）からの相対である。
///
/// 1 つのファイルが 2 種類の旗を立てることがあるので、その場合は**旗ごとに 1 行**を
/// 置く——旗の欄は 1 つしか持てず、片方だけ載せると残りの旗が名簿から落ちて静かにずれる。
/// （現在は 1 ファイル 1 旗である。`balloon_visibility_phase.rs` が `REARM` と `ZORDER` の
/// 2 行を持っていたのは、退役したバルーン再表示の追随トリガのぶんである。）
const AREKA_PRODUCERS: [(&str, &str, &str); 7] = [
    (
        "emo2_boot/adapter.rs",
        include_str!("emo2_boot/adapter.rs"),
        "PRESENT",
    ),
    (
        "emo2_boot/balloon_visibility_phase.rs",
        include_str!("emo2_boot/balloon_visibility_phase.rs"),
        "REARM",
    ),
    (
        "emo2_boot/frame/scale_text.rs",
        include_str!("emo2_boot/frame/scale_text.rs"),
        "REARM",
    ),
    (
        "emo2_boot/hover_inject.rs",
        include_str!("emo2_boot/hover_inject.rs"),
        "REARM",
    ),
    (
        "emo2_boot/move_cue.rs",
        include_str!("emo2_boot/move_cue.rs"),
        "PRESENT",
    ),
    (
        "emo2_boot/talk_lifecycle.rs",
        include_str!("emo2_boot/talk_lifecycle.rs"),
        "PRESENT",
    ),
    (
        "emo2_boot/zorder_cue.rs",
        include_str!("emo2_boot/zorder_cue.rs"),
        "ZORDER",
    ),
];

/// 註釈の行を落とす——説明文に書いてあるだけの綴りを「在る」と数えないため。
fn code_only(src: &str) -> String {
    src.lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn every_areka_producer_marks_the_tick_wake() {
    for (label, src, flag) in AREKA_PRODUCERS {
        let code = code_only(src);
        assert!(
            code.contains("tick_wake::mark("),
            "{label}: 旗を立てる呼出（tick_wake::mark(）が無い"
        );
        assert!(
            code.contains(&format!("tick_wake::{flag}")),
            "{label}: 期待する旗 {flag} を立てていない"
        );
    }
}

/// バルーンの待ち時間は旗ではなく**期限**で預ける（設計 C16）。
///
/// 期限の枠は最も早い 1 つしか持てず到来で倒れるので、待っている限り相が走るたびに
/// 預け直す必要がある。ここでは預ける呼出が在ることだけを見る（預け直しの回数は
/// 相の側の決定論テストが `visibility_wake` の答えとして固定する）。
#[test]
fn balloon_visibility_phase_arms_the_deadline() {
    let code = code_only(include_str!("emo2_boot/balloon_visibility_phase.rs"));
    assert!(
        code.contains("tick_wake::arm_deadline("),
        "emo2_boot/balloon_visibility_phase.rs: 待ち時間を預ける呼出（tick_wake::arm_deadline(）が無い"
    );
}

// ==================================== 名簿の完全性（現物の木から逆向きに照合する）

// 上の 2 本は「表に載っている行が現に旗を立てるか」を見る。それだけでは**表から落ちた
// 生産者**を捕まえられない——行を回すだけの検査は、載っていないファイルについて何も
// 言わないからである。ここから下は逆向き、すなわち「木の中で旗を立てているファイルが
// 全部名簿に載っているか」を見る。wintf 側の `tick_gate_tests.rs` に同じ対の検査がある。
//
// 見るのは 2 つの書き方である——`tick_wake::mark` とパスで書いた参照と、
// `tick_wake::` を含む `use` 行が `mark` を名指している形（`use …tick_wake::{ZORDER, mark};`）である。
// 呼出の字面 `tick_wake::mark(` だけを見ていたときは、**表に載せていない新設の生産者が
// `use` 形だと両向とも緑で抜けた**（表の側の検査は表に在る行しか見ないので助けに
// ならない）。なおモジュールごと別名にする形（`use …tick_wake as tw;`）までは見ていない
// ——その形が要るときは [`uses_tick_wake_name`] を広げること。

use std::path::{Path, PathBuf};

/// `crates/areka/src/` 配下の本番 `.rs` を集める（テストのファイルは除く）。
fn production_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("木を歩けない（検査が空振りする）: {} — {e}", dir.display()));
    for entry in entries {
        let path = entry.expect("ディレクトリ項目が読めない").path();
        if path.is_dir() {
            production_rs_files(&path, out);
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !name.ends_with(".rs")
            || name.ends_with("_tests.rs")
            || name.ends_with("test_support.rs")
        {
            continue;
        }
        out.push(path);
    }
}

/// この本文が `tick_wake` の名前 `name` を使っているか。
///
/// 見るのは 2 つ——`tick_wake::name` と書いた参照と、`use …tick_wake::…name…` で
/// 引き込んでから素の `name` で書く形の `use` 行である。
fn uses_tick_wake_name(code: &str, name: &str) -> bool {
    if code.contains(&format!("tick_wake::{name}")) {
        return true;
    }
    code.lines().any(|line| {
        let line = line.trim_start();
        line.starts_with("use ") && line.contains("tick_wake::") && line.contains(name)
    })
}

/// 走査の判定子は 2 つの書き方をどちらも見る（`use` 形が抜けないことの自己検査）。
///
/// この 3 行が無いと、広げたはずの走査が実は広がっていなくても誰も気づかない
/// （現物の木に `use` 形の生産者が 1 つも無いため）。
#[test]
fn the_scan_sees_both_the_path_form_and_the_use_form() {
    assert!(
        uses_tick_wake_name("tick_wake::mark(tick_wake::ZORDER);", "mark"),
        "パス形の呼出を見落としている"
    );
    assert!(
        uses_tick_wake_name(
            "use crate::ecs::world::tick_wake::{ZORDER, mark};
mark(ZORDER);",
            "mark"
        ),
        "`use` 形で引き込んだ生産者を見落としている（この形は両向とも緑で抜ける）"
    );
    assert!(
        !uses_tick_wake_name("use crate::ecs::world::tick_wake;", "mark"),
        "モジュールを引き込んだだけの `use` 行を生産者と見なしている"
    );
}

/// `src/` からの相対パス（区切りは `/`）で、旗を立てている本番ファイルを列挙する。
///
/// `flag` が空文字なら「どれかの旗を立てている」、それ以外なら「その旗を立てている」。
fn files_marking(flag: &str) -> Vec<String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    production_rs_files(&root, &mut files);

    let mut found: Vec<String> = files
        .iter()
        .filter(|path| {
            let src = std::fs::read_to_string(path).expect("本番ファイルが読めない");
            let code = code_only(&src);
            uses_tick_wake_name(&code, "mark")
                && (flag.is_empty() || uses_tick_wake_name(&code, flag))
        })
        .map(|path| {
            path.strip_prefix(&root)
                .expect("走査の根の下に無い")
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect();
    found.sort();
    found.dedup();
    found
}

/// 木の中で旗を立てている areka の本番ファイルは、1 つ残らず `AREKA_PRODUCERS` に載っている。
///
/// 逆向きも同時に見る——表の行が現物のどのファイルにも当たらなければ、走査が壊れて
/// 0 件を返している（そのときは何を消しても緑になる）。両側から挟んでおく。
#[test]
fn the_areka_table_lists_every_file_that_marks_the_wake() {
    let found = files_marking("");
    assert!(
        !found.is_empty(),
        "走査が 1 件も見つけていない（検査そのものが壊れている）"
    );

    for rel in &found {
        assert!(
            AREKA_PRODUCERS
                .iter()
                .any(|(label, _, _)| rel.ends_with(label)),
            "旗を立てているのに AREKA_PRODUCERS に載っていない: {rel}"
        );
    }
    for (label, _, _) in AREKA_PRODUCERS {
        assert!(
            found.iter().any(|rel| rel.ends_with(label)),
            "AREKA_PRODUCERS の行が現物のどのファイルにも当たらない: {label}"
        );
    }
}

/// wintf の `tick_wake.rs` 冒頭の散文名簿は、areka 側の `ZORDER` 生産者も名指ししている。
///
/// あの名簿は**クロスクレートの生産者も対象**である（`PRESENT` の行が areka 側を名指し
/// している）。そして実際にずれた——areka の `zorder_cue.rs` が落ちたまま緑で通っていた。
/// 見つけたのは人の目であってテストではない。以後は機械が見る。
///
/// wintf からは areka のファイルが読めないので、この向きの照合は areka 側にしか置けない。
/// 名簿の現物はワークスペース相対で読む（読めなければその場で落ちる＝静かには壊れない）。
#[test]
fn the_wintf_module_roster_names_every_areka_zorder_producer() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../wintf/src/ecs/world/tick_wake.rs");
    let src = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "起床の旗の名簿が読めない（置き場所が変わったか）: {} — {e}",
            path.display()
        )
    });
    let normalized = src.replace("\r\n", "\n");

    let mut lines = normalized
        .lines()
        .skip_while(|l| !l.contains("- [`ZORDER`]"));
    let head = lines.next().expect("散文名簿に ZORDER の項が無い");
    let rest: Vec<&str> = lines
        .take_while(|l| l.starts_with("//!") && !l.contains("- ["))
        .collect();
    let bullet = format!("{head}\n{}", rest.join("\n"));

    let producers = files_marking("ZORDER");
    assert!(
        !producers.is_empty(),
        "areka 側の ZORDER 生産者が 1 件も見つからない（検査そのものが壊れている）"
    );
    for rel in &producers {
        let stem = rel
            .rsplit('/')
            .next()
            .and_then(|name| name.strip_suffix(".rs"))
            .expect("相対パスが .rs で終わっていない");
        assert!(
            bullet.contains(stem),
            "wintf の tick_wake.rs 冒頭の ZORDER の名簿が areka の {rel} を名指ししていない"
        );
    }
}
