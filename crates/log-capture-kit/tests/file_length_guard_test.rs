//! 1 ファイル 1,000 行の目安の機械的な番人（要件 10.1・10.2・10.3・10.4）。
//!
//! `.kiro/steering/structure.md:176` は「1 ファイル 1,000 行以下」を目安に掲げているが、
//! これまで誰も測っていなかった。本ファイルはその目安を実行テストにする。
//!
//! # 何を見張るか
//!
//! `crates/**/*.rs`（`src/`・`tests/`・`examples/`・本番の隣に置いた兄弟テストファイルを含む）
//! を列挙して行数を測り、上限を超え例外表 [`OVER_LIMIT_ALLOWED`] に無いファイルがあれば赤にする。
//! 列挙と計測と判定は走査器（`workspace_scan`）が持ち、本ファイルは**判定に使う表**だけを持つ。
//!
//! # 例外表は「今そこにある超過」だけを表す（要件 10.2・10.4）
//!
//! 本仕様が作るのは見張りだけで、現に超過している既存ファイルの分割・縮小は行わない
//! （要件 10.4。並走する仕様との衝突を避けるための裁定）。よって例外表は着手時点の
//! 超過ファイル 11 件で始まり、導入した時点では赤にならない。合流してくる仕事が新しく
//! 1,000 行を超えるファイルを作れば、その仕事の側で赤になる——それが番人の役割である。
//!
//! 例外表が**暗黙に増えない**ようにするため、`const` であることに加えて次を要求する
//! （要件 8.2 に対して `with_default_guard_test.rs` が置いたのと同じ規律）。
//!
//! - 件数を別の定数 [`OVER_LIMIT_ALLOWED_COUNT`] に逐語で持ち、表と食い違えば赤にする。
//!   項目の追加は**表と件数の 2 箇所**の編集になる。
//! - 各項目に理由（空でない文字列）を要求し、ファイル 1 件を逐語で指すことを要求する
//!   （`*` を含む総括的な指定は、書いた本人も気づかないうちに範囲が広がる）。
//! - 表に載っているのに**今はもう超過していない**項目を赤にする。例外は「今そこにある事情」
//!   だけを表すので、分割で事情が消えたら表からも消える＝**削除だけが自然な方向**になる。
//!
//! # 「0 件なら緑」への較正（要件 10.3）
//!
//! 主検査は「違反 0 件なら緑」の形で、**道具が壊れていても緑**になる——列挙が 1 ファイルも
//! 拾えていなくても、行数が全部 0 に測れても、結果は同じ空集合である。よって陽性側の相棒を
//! 3 本置く。
//!
//! - [`every_over_limit_exception_is_still_over_the_limit`] は、表の全項目に**今も**超過の実体が
//!   あること。列挙が空振りしていれば 11 件すべてが「実体無し」になって必ず赤になる。
//! - [`the_measurement_is_not_vacuous_and_matches_the_allow_table`] は、実測した超過ファイルの集合が
//!   表と**完全に一致**すること（要件 10.2 の「着手時の実測と一致」の固定）。
//! - [`dropping_a_known_exception_turns_the_guard_red`] は、既知の 1 件を表から外すと、実データで
//!   その 1 件が返ること（要件 10.3 が名指しで求める自己検査）。
//!
//! 上限そのもの（`LINE_LIMIT` が 1000 であること）と、行数の数え方・境界・列挙の被覆は
//! 走査器の較正（`workspace_scan_test.rs`）が縛る。ここでは表と実データの対応だけを見る。

mod workspace_scan;

use std::collections::BTreeSet;

use workspace_scan::{FileLines, LINE_LIMIT, measure_workspace_sources, over_limit};

// ---------------------------------------------------------------------------
// 例外表
// ---------------------------------------------------------------------------

/// 上限を超えていても赤にしないファイル（相対パス・理由）。
///
/// 初期値は着手時点の実測 11 件で、`verification/remeasure.md` §6（2026-08-23 採取）と
/// ファイル名・件数がともに一致する。**行数は理由欄に書かない**——書けば当該ファイルへの
/// 無関係な編集で表が陳腐化し、本仕様が触らないと決めたファイル（要件 10.4）を触らせる
/// 圧力になる。表が表すのは「上限を超えている」という事実だけで、超過の程度ではない。
const OVER_LIMIT_ALLOWED: &[(&str, &str)] = &[
    (
        "crates/areka-emo-compose/src/plan_ops_tests.rs",
        "着手前から超過している本番の隣の兄弟テストファイル。分割は本仕様の範囲外（要件 10.4）",
    ),
    (
        "crates/areka-emo-present/src/cache_tests.rs",
        "着手前から超過している本番の隣の兄弟テストファイル。分割は本仕様の範囲外（要件 10.4）",
    ),
    (
        "crates/areka-emo-present/src/presenter/budget_tests.rs",
        "着手前から超過している本番の隣の兄弟テストファイル。分割は本仕様の範囲外（要件 10.4）",
    ),
    (
        "crates/areka-ghost/tests/ghost/inproc_e2e_test.rs",
        "着手前から超過している統合テスト。分割は本仕様の範囲外（要件 10.4）",
    ),
    (
        "crates/areka-seriko/src/actor_bind_loop_tests.rs",
        "着手前から超過している本番の隣の兄弟テストファイル。分割は本仕様の範囲外（要件 10.4）",
    ),
    (
        "crates/areka-seriko/src/bind.rs",
        "着手前から超過している本番ソース。分割は本仕様の範囲外（要件 10.4）",
    ),
    (
        "crates/areka/src/emo2_boot/frame_transition_branch_tests.rs",
        "着手前から超過している本番の隣の兄弟テストファイル。分割は本仕様の範囲外（要件 10.4）",
    ),
    (
        "crates/areka/src/placement/follow/window_move.rs",
        "着手前から超過している本番ソース。分割は本仕様の範囲外（要件 10.4）",
    ),
    (
        "crates/areka/src/placement/transition_judge_tests.rs",
        "着手前から超過している本番の隣の兄弟テストファイル。分割は本仕様の範囲外（要件 10.4）",
    ),
    (
        "crates/areka/src/placement/transition_judge_verdict_tests.rs",
        "着手前から超過している本番の隣の兄弟テストファイル。分割は本仕様の範囲外（要件 10.4）",
    ),
    (
        "crates/pilot/examples/pilot-clickthrough-alpha-toggle/main.rs",
        "着手前から超過している実行例。分割は本仕様の範囲外（要件 10.4）",
    ),
];

/// [`OVER_LIMIT_ALLOWED`] の件数（逐語）。表を増やすときはここも編集する（要件 10.2）。
const OVER_LIMIT_ALLOWED_COUNT: usize = 11;

/// 要件 10.3 の自己検査で表から外す 1 件。表の項目でなければならない。
const CALIBRATION_DROPPED_ENTRY: &str = "crates/areka-emo-present/src/cache_tests.rs";

// ---------------------------------------------------------------------------
// 表と実測の取り回し
// ---------------------------------------------------------------------------

/// 例外表のパスだけを取り出す（判定に渡す形）。
fn allowed_paths() -> Vec<&'static str> {
    OVER_LIMIT_ALLOWED.iter().map(|(path, _)| *path).collect()
}

/// 例外表から 1 件だけ落としたパスの一覧を作る（要件 10.3 の自己検査で使う）。
fn allowed_paths_without(dropped: &str) -> Vec<&'static str> {
    allowed_paths()
        .into_iter()
        .filter(|path| *path != dropped)
        .collect()
}

/// 違反の一覧を人が読める形にする。
fn render(files: &[FileLines]) -> String {
    files
        .iter()
        .map(|f| format!("  {} ({} 行)", f.path, f.lines))
        .collect::<Vec<_>>()
        .join("\n")
}

// ---------------------------------------------------------------------------
// 主検査（要件 10.1）
// ---------------------------------------------------------------------------

#[test]
fn no_source_file_exceeds_the_line_limit_outside_the_allow_table() {
    let measured = measure_workspace_sources();
    let violations = over_limit(&measured, &allowed_paths());
    assert!(
        violations.is_empty(),
        "1 ファイル 1,000 行の目安（structure.md:176）を超えるファイルがある。\
         分割して収めるか、やむを得ない場合は理由付きで OVER_LIMIT_ALLOWED へ明示的に\
         追加すること（表と OVER_LIMIT_ALLOWED_COUNT の 2 箇所）:\n{}",
        render(&violations)
    );
}

// ---------------------------------------------------------------------------
// 例外表が暗黙に増えない形（要件 10.2）
// ---------------------------------------------------------------------------

#[test]
fn the_over_limit_allow_table_declares_its_own_size_and_reasons() {
    assert_eq!(
        OVER_LIMIT_ALLOWED.len(),
        OVER_LIMIT_ALLOWED_COUNT,
        "例外表の件数が宣言と食い違う。項目の追加は表と件数の 2 箇所を明示的に編集すること（要件 10.2）"
    );

    let measured = measure_workspace_sources();
    for (path, reason) in OVER_LIMIT_ALLOWED {
        assert!(
            !reason.trim().is_empty(),
            "例外には理由が要る（要件 10.2）: {path}"
        );
        assert!(
            !path.contains('*') && path.ends_with(".rs"),
            "例外はファイル 1 件を逐語で指すこと（総括的な指定は暗黙の増加を許す）: {path}"
        );
        assert!(
            measured.iter().any(|f| f.path == *path),
            "例外表が実在しないファイルを指している: {path}"
        );
    }
}

#[test]
fn the_over_limit_allow_table_has_no_duplicate_entries() {
    // 同じパスが 2 度載ると、件数の宣言を通ったまま実質の例外が 1 件減る
    //（＝表の見かけの大きさと効き目が食い違う）。
    let unique: BTreeSet<&str> = allowed_paths().into_iter().collect();
    assert_eq!(
        unique.len(),
        OVER_LIMIT_ALLOWED_COUNT,
        "例外表に重複した項目がある: {:?}",
        allowed_paths()
    );
}

#[test]
fn every_over_limit_exception_is_still_over_the_limit() {
    // 「違反 0 件」は道具が壊れていても成立する。表の全項目に**今も**超過の実体があることを
    // 要求すると、列挙や計測が空振りしていれば必ず赤になる（＝主検査の非空虚性の担保）。
    // あわせて、分割で 1,000 行以下になった項目を表に置き去りにできなくする
    //（例外が減る方向だけが自然になる＝要件 10.2）。
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
// 較正（要件 10.3）
// ---------------------------------------------------------------------------

#[test]
fn dropping_a_known_exception_turns_the_guard_red() {
    // 要件 10.3 の自己検査。見本ではなく**実データ**で行う——見本で作った両側は
    // 走査器の較正（workspace_scan_test.rs）が既に持っており、ここで確かめたいのは
    // 「この表とこのワークスペースの組み合わせで、番人が本当に鳴るか」である。
    assert!(
        allowed_paths().contains(&CALIBRATION_DROPPED_ENTRY),
        "較正で外す 1 件は例外表の項目でなければならない: {CALIBRATION_DROPPED_ENTRY}"
    );

    let measured = measure_workspace_sources();
    let violations = over_limit(&measured, &allowed_paths_without(CALIBRATION_DROPPED_ENTRY));
    let paths: Vec<&str> = violations.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(
        paths,
        vec![CALIBRATION_DROPPED_ENTRY],
        "例外表から外した 1 件だけが違反として返るはず（外して緑なら番人は鳴っていない）"
    );
    assert!(
        violations[0].lines > LINE_LIMIT,
        "返ってきた違反が上限を超えていない＝行数の計測が壊れている: {:?}",
        violations[0]
    );
}

#[test]
fn the_measurement_is_not_vacuous_and_matches_the_allow_table() {
    // 陽性側の相棒。⑴ 列挙が現実的な規模であること ⑵ 上限超過が確かに存在すること
    // ⑶ その集合が例外表と**完全に一致**すること（要件 10.2 の「着手時の実測と一致」）。
    let measured = measure_workspace_sources();
    assert!(
        measured.len() > 500,
        "列挙が極端に少ない＝走査が空振りしている疑い: {} 件",
        measured.len()
    );

    let all_over = over_limit(&measured, &[]);
    assert!(
        !all_over.is_empty(),
        "上限を超えるファイルが 1 件も無い。着手時の実測は 11 件だったので、\
         行数の計測か列挙が壊れている疑いが濃い"
    );

    let found: BTreeSet<&str> = all_over.iter().map(|f| f.path.as_str()).collect();
    let listed: BTreeSet<&str> = allowed_paths().into_iter().collect();
    assert_eq!(
        found, listed,
        "実測した超過ファイルの集合が例外表と一致しない。\
         新たな超過なら分割するか表へ明示的に追加し、解消したのなら表から削除すること"
    );
}
