//! 1 ファイル 1,000 行の目安の機械的な番人。
//!
//! `.kiro/steering/structure.md` の Test Naming Conventions の項が掲げる「1 ファイル 1,000 行以下」の
//! 目安を実行テストにする。
//!
//! # 何を見張るか
//!
//! `crates/**/*.rs`（`src/`・`tests/`・`examples/`・本番の隣に置いた兄弟テストファイルを含む）
//! を列挙して行数を測り、上限を超え例外表 [`OVER_LIMIT_ALLOWED`] に無いファイルがあれば赤にする。
//! 列挙と計測と判定は走査器（`workspace_scan`）が持ち、本ファイルは**判定に使う表**だけを持つ。
//!
//! # 例外表は「今そこにある超過」だけを表す
//!
//! 現に超過している既存ファイルの分割・縮小は番人の仕事ではなく、例外表がそれらの超過を表す。
//! 合流してくる仕事が新しく 1,000 行を超えるファイルを作れば、その仕事の側で赤になる——
//! それが番人の役割である。
//!
//! 例外表が**暗黙に増えない**ようにするため、`const` であることに加えて次を要求する
//! （`with_default_guard_test.rs` と同じ規律）。
//!
//! - 件数を別の定数 [`OVER_LIMIT_ALLOWED_COUNT`] に逐語で持ち、表と食い違えば赤にする。
//!   項目の追加は**表と件数の 2 箇所**の編集になる。
//! - 各項目に理由（空でない文字列）を要求し、ファイル 1 件を逐語で指すことを要求する
//!   （`*` を含む総括的な指定は、書いた本人も気づかないうちに範囲が広がる）。
//! - 表に載っているのに**今はもう超過していない**項目を赤にする。例外は「今そこにある事情」
//!   だけを表すので、分割で事情が消えたら表からも消える＝**削除だけが自然な方向**になる。
//!
//! # 「0 件なら緑」への較正
//!
//! 主検査は「違反 0 件なら緑」の形で、**道具が壊れていても緑**になる——列挙が 1 ファイルも
//! 拾えていなくても、行数が全部 0 に測れても、結果は同じ空集合である。よって次の陽性側の相棒を
//! 置く。
//!
//! - [`every_over_limit_exception_is_still_over_the_limit`] は、表の全項目に**今も**超過の実体が
//!   あること。列挙が空振りしていれば表の全項目が「実体無し」になって必ず赤になる。
//! - [`the_measurement_is_not_vacuous`] は、列挙が現実的な規模で、いちばん長いファイルが上限の
//!   半分を越えること（[`measurement_is_plausible`]）。上限越えのファイルが 1 件も無くなっても
//!   （表が空になっても）赤にならない。
//! - [`dropping_a_known_exception_turns_the_guard_red`] は、表の先頭の項を外すと、実データで
//!   その 1 件が返ること（自己検査）。表が空なら何もしない。
//!
//! 上限そのもの（`LINE_LIMIT` が 1000 であること）と、行数の数え方・境界・列挙の被覆は
//! 走査器の較正（`workspace_scan_test.rs`）が縛る。ここでは表と実データの対応だけを見る。

mod workspace_scan;

use std::collections::BTreeSet;

use workspace_scan::{
    FileLines, LINE_LIMIT, allowed_paths, allowed_paths_without, measure_workspace_sources,
    over_limit, render,
};

// ---------------------------------------------------------------------------
// 例外表
// ---------------------------------------------------------------------------

/// 上限を超えていても赤にしないファイル（相対パス・理由）。
///
/// **行数は理由欄に書かない**——書けば当該ファイルへの
/// 無関係な編集で表が陳腐化し、そのファイルを触らせる圧力になる。表が表すのは「上限を超えている」という事実だけで、超過の程度ではない。
const OVER_LIMIT_ALLOWED: &[(&str, &str)] = &[
    (
        "crates/areka-placement/src/follow/window_move.rs",
        "着手前から超過している本番ソース。分割は本仕様の範囲外（要件 10.4）",
    ),
    (
        "crates/areka-runtime/src/placement_tests/transition_judge_verdict_tests.rs",
        "着手前から超過している本番の隣の兄弟テストファイル。分割は本仕様の範囲外（要件 10.4）",
    ),
];

/// [`OVER_LIMIT_ALLOWED`] の件数（逐語）。表を増やすときはここも編集する。
const OVER_LIMIT_ALLOWED_COUNT: usize = 2;

/// 列挙と計測が空振りしていないことの目安（純関数）。いちばん長いファイルが上限の半分を越えるか。
///
/// 上限越えのファイルが 1 件も無くなっても（表が空になっても）真になる形にしてある。
fn measurement_is_plausible(files: &[FileLines]) -> bool {
    files.iter().any(|f| f.lines > LINE_LIMIT / 2)
}

// ---------------------------------------------------------------------------
// 主検査
// ---------------------------------------------------------------------------

#[test]
fn no_source_file_exceeds_the_line_limit_outside_the_allow_table() {
    let measured = measure_workspace_sources();
    let violations = over_limit(&measured, &allowed_paths(OVER_LIMIT_ALLOWED));
    assert!(
        violations.is_empty(),
        "1 ファイル 1,000 行の目安（structure.md の Test Naming Conventions）を超えるファイルがある。\
         分割して収めるか、やむを得ない場合は理由付きで OVER_LIMIT_ALLOWED へ明示的に\
         追加すること（表と OVER_LIMIT_ALLOWED_COUNT の 2 箇所）:\n{}",
        render(&violations)
    );
}

// ---------------------------------------------------------------------------
// 例外表が暗黙に増えない形
// ---------------------------------------------------------------------------

#[test]
fn the_over_limit_allow_table_declares_its_own_size_and_reasons() {
    assert_eq!(
        OVER_LIMIT_ALLOWED.len(),
        OVER_LIMIT_ALLOWED_COUNT,
        "例外表の件数が宣言と食い違う。項目の追加は表と件数の 2 箇所を明示的に編集すること（要件 10.2）"
    );

    for (path, reason) in OVER_LIMIT_ALLOWED {
        assert!(
            !reason.trim().is_empty(),
            "例外には理由が要る（要件 10.2）: {path}"
        );
        assert!(
            !path.contains('*') && path.ends_with(".rs"),
            "例外はファイル 1 件を逐語で指すこと（総括的な指定は暗黙の増加を許す）: {path}"
        );
    }
}

#[test]
fn the_over_limit_allow_table_has_no_duplicate_entries() {
    // 同じパスが 2 度載ると、件数の宣言を通ったまま実質の例外が 1 件減る
    //（＝表の見かけの大きさと効き目が食い違う）。
    let unique: BTreeSet<&str> = allowed_paths(OVER_LIMIT_ALLOWED).into_iter().collect();
    assert_eq!(
        unique.len(),
        OVER_LIMIT_ALLOWED_COUNT,
        "例外表に重複した項目がある: {:?}",
        allowed_paths(OVER_LIMIT_ALLOWED)
    );
}

#[test]
fn every_over_limit_exception_is_still_over_the_limit() {
    // 「違反 0 件」は道具が壊れていても成立する。表の全項目に**今も**超過の実体があることを
    // 要求すると、列挙や計測が空振りしていれば必ず赤になる（＝主検査の非空虚性の担保）。
    // あわせて、分割で 1,000 行以下になった項目を表に置き去りにできなくする
    //（例外が減る方向だけが自然になる）。
    let measured = measure_workspace_sources();
    let mut stale = Vec::new();
    for (path, _) in OVER_LIMIT_ALLOWED {
        match measured.iter().find(|f| f.path == *path) {
            Some(found) if found.lines > LINE_LIMIT => {}
            Some(found) => stale.push(format!("  {} は {} 行で上限内", path, found.lines)),
            None => stale.push(format!("  {path} は列挙に現れない")),
        }
    }
    assert!(
        stale.is_empty(),
        "例外表に載っているのに超過の実体が無い（走査が空振りしているか、事情が消えた）。\
         事情が消えたのなら表から削除すること:\n{}",
        stale.join("\n")
    );
}

// ---------------------------------------------------------------------------
// 較正
// ---------------------------------------------------------------------------

#[test]
fn dropping_a_known_exception_turns_the_guard_red() {
    // 見本ではなく**実データ**で行う——見本で作った両側は
    // 走査器の較正（workspace_scan_test.rs）が既に持っており、ここで確かめたいのは
    // 「この表とこのワークスペースの組み合わせで、番人が本当に鳴るか」である。
    // 表が空なら外す項が無いので何もしない（合成の較正が残る）。
    let Some((dropped, _)) = OVER_LIMIT_ALLOWED.first() else {
        return;
    };

    let measured = measure_workspace_sources();
    let violations = over_limit(
        &measured,
        &allowed_paths_without(OVER_LIMIT_ALLOWED, dropped),
    );
    let paths: Vec<&str> = violations.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(
        paths,
        vec![*dropped],
        "例外表から外した 1 件だけが違反として返るはず（外して緑なら番人は鳴っていない）"
    );
    assert!(
        violations[0].lines > LINE_LIMIT,
        "返ってきた違反が上限を超えていない＝行数の計測が壊れている: {:?}",
        violations[0]
    );
}

#[test]
fn the_measurement_is_not_vacuous() {
    // 陽性側の相棒。列挙が現実的な規模で、いちばん長いファイルが上限の半分を越えること。
    // 上限を越えるファイルがあることは要求しない（要求すると最後の 1 件を割った側が赤になる）。
    let measured = measure_workspace_sources();
    assert!(
        measured.len() > 500,
        "列挙が極端に少ない＝走査が空振りしている疑い: {} 件",
        measured.len()
    );
    assert!(
        measurement_is_plausible(&measured),
        "上限の半分を越えるファイルが 1 件も無い。行数の計測か列挙が壊れている疑いが濃い"
    );
}

#[test]
fn the_vacuity_check_needs_no_over_limit_file() {
    // 上限越えが 1 件も無い列でも、長いファイルがあれば真。短いものだけ・空なら偽。
    let under_limit = [
        FileLines::new("a.rs", LINE_LIMIT),
        FileLines::new("b.rs", LINE_LIMIT / 2 + 1),
    ];
    assert!(over_limit(&under_limit, &[]).is_empty());
    assert!(measurement_is_plausible(&under_limit));

    assert!(!measurement_is_plausible(&[FileLines::new(
        "c.rs",
        LINE_LIMIT / 2
    )]));
    assert!(!measurement_is_plausible(&[]));
}
