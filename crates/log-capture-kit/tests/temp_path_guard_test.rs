//! 一時パスの共通窓口を迂回する新設を検知する見張り（要件 12.4・12.5・12.7）。
//!
//! # 何を見張るか
//!
//! 一時ディレクトリの**入口**（`std::env::temp_dir`）の呼出が、窓口 crate の定義
//! （`crates/temp-path-kit/src/`）と例外表 [`ALLOWED_ENTRY_POINT_USES`] の外に 1 件も
//! 無いこと。入口は 2 通りの綴りで書けるので両方を走査する（[`entry_point_tokens`]）。
//!
//! # なぜログ捕捉 crate に住んでいるのか
//!
//! 窓口は `temp-path-kit`、見張りはここ——**別 crate に分かれるのは意図的な設計**である
//! （設計 C8）。この `tests/` はワークスペース全体の見張りの置き場で（設計 C6）、走査器
//! `workspace_scan/mod.rs`（ファイル列挙・コメント除去・語の走査）を 4 本目の見張りとして
//! 共有する。見張りを窓口 crate へ置くと走査器が複製される。
//!
//! # 判定の規則は「固定名かどうか」を見ない（意図的）
//!
//! 要件 12.4 が名指しする欠陥は「入口から**固定名**を組み立てる箇所」だが、本見張りは
//! 固定名と一意名を式の形から見分けようとしない。見分けようとすれば、名前の材料が
//! 別の関数・別の定数・別のファイルにある形をすべて追う必要があり、**判定器そのものが
//! 壊れる**（本仕様は既に「自作の走査器が例外を吐きながら一致と報告した」事故を
//! 記録している）。
//!
//! 代わりに規則は 1 行で言える——**入口の呼出は、窓口の中か例外表の中にしか存在しては
//! ならない。** 固定名の新設はこの規則の**部分集合**なので、要件 12.4 が求める検知は
//! 含まれる。新しく入口を叩くコードは、一意名を組んでいても例外表への明示的な編集を
//! 求められる。**それが表の目的**である（要件 12.4「追加は明示的な編集としてのみ許す」）。
//!
//! # コメントの除去は必須（要件 12.7）
//!
//! 走査は [`scan_tokens`] を通す＝**コメントを除いてから**語を探す。要件 12.7 が記録する
//! とおり、本仕様の調査中に**コメント中の語で対象の絞り込みが反転する**事故が実際に
//! 起きている。除去が効いていることは [`a_fixed_name_in_a_comment_is_not_a_hit_but_the_same_line_of_code_is`]
//! が実行行とコメント行の**対**で固定する。
//!
//! # 「0 件なら緑」への較正（要件 12.5）
//!
//! 主検査は「違反 0 件なら緑」の形で、**何も検知できない壊れた道具でも、例外表さえ
//! 揃っていれば緑になる**。よって陽性側の相棒を置く。
//!
//! - [`dropping_a_known_exception_turns_the_guard_red`] — 実データで、例外表から 1 件
//!   外すとその 1 件だけが違反として返ること（要件 12.5 が名指しで求める自己テスト）。
//! - [`a_fixed_name_built_from_the_entry_point_is_detected`] ほか — 合成入力で、入口から
//!   固定名を組む式が実際に当たること（実ファイルを汚さずに済む）。
//! - [`every_exception_still_has_a_real_hit`] — 表の全項目に**今も**当たりがあること。
//!   列挙が空振りしていれば 16 件すべてが「実体無し」になって必ず赤になる。
//!   （この検査は飾りではない——タスク 10.7 が `areka-sylphya/src/persist/io.rs` を窓口へ
//!   寄せた直後、表に残っていた同ファイルの項目をこの検査が名指しで赤にした。）
//! - [`the_measurement_is_not_vacuous_and_matches_the_allow_table`] — 実測した集合が表と
//!   **完全に一致**すること（要件 12.4 の「移行後の実測値で開始」の固定）。
//!
//! # 走査語を逐語で書かない約束
//!
//! 本ファイルは `crates/` の下にあるので走査対象そのものでもある。走査語を開き括弧まで
//! 含めた形で書くと ⒜ 見張りが自分自身を違反として拾い ⒝ 要件 12.2 の実測に使った
//! `rg`／`grep` の母数が黙って動く。そこで走査語は `concat!` で 2 片から組み立て、
//! ファイルの字面には 1 度も現れないようにする（`with_default_guard_test.rs:17-28` と
//! 同じ約束）。守れていることは
//! [`the_guard_file_never_spells_the_tokens_out_not_even_inside_comments`] が縛る。

mod workspace_scan;

use std::collections::BTreeSet;

use workspace_scan::{read_source, scan_tokens, walk_workspace_sources};

// ---------------------------------------------------------------------------
// 走査語（逐語で置かないため 2 片に割る。module doc を参照）
// ---------------------------------------------------------------------------

/// 入口の修飾つきの綴り（`std::env::` を前置した形・`use std::env;` の形の両方に当たる）。
const TOKEN_QUALIFIED: &str = concat!("env::temp_", "dir(");

/// 入口を `use` で裸にした綴り。
///
/// 修飾つきの形の内側にも現れるが、[`scan_tokens`] は当たった語の内側から次の語を探さない
/// ので 1 呼出が 2 件に数えられることはない（規則は `workspace_scan/mod.rs:318-332`。
/// この規則が実際に効いていることは [`the_two_spellings_never_double_count_one_call`] が
/// 見本で固定する）。左端はアンカーされるので、名前の末尾がたまたま同じ綴りで終わる
/// 助走関数（実在: `unique_temp_dir`・`make_unique_temp_dir`）には当たらない。
const TOKEN_BARE: &str = concat!("temp_", "dir(");

/// 名前にプロセス識別子を織り込む呼出（例外の**分類が飾りでない**ことを裏取りするのに使う）。
const TOKEN_PROCESS_ID: &str = concat!("process::", "id(");

/// 主検査の走査語。
fn entry_point_tokens() -> Vec<&'static str> {
    vec![TOKEN_QUALIFIED, TOKEN_BARE]
}

// ---------------------------------------------------------------------------
// 例外表
// ---------------------------------------------------------------------------

/// 窓口の**定義**が置かれた領域。走査から外す唯一の領域。
///
/// 窓口が入口を叩くのは当然なので表の項目にはしない。外すのが飾りでない
/// （＝この領域に確かに入口の呼出がある）ことは
/// [`the_gateway_is_the_only_excluded_region_and_it_really_holds_the_entry_point`] が縛る。
/// 外すのは `src/` だけで、窓口 crate の `tests/` があれば走査する（姉妹の見張りと同じ規律）。
const GATEWAY_DEFINITION_PREFIX: &str = "crates/temp-path-kit/src/";

/// 例外を認める理由の**種別**。理由欄の散文だけでなく機械で裏の取れる分類を持たせる。
///
/// 散文の理由は書いた本人以外には検証できない。種別を付けておけば
/// [`each_exception_declares_whether_it_uniquifies_by_process_identifier`] が
/// ソースと突き合わせられる——「一意化している」と名乗る項目にプロセス識別子が無ければ
/// 赤になり、「していない」と名乗る項目に有れば分類の誤りとして赤になる。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Why {
    /// 名前にプロセス識別子を織り込んでいる＝プロセス間で衝突しない（要件 12.1 と同じ性質）。
    ProcessUnique,
    /// 入口から**読み出す**だけで、書込も削除もしない＝原理的に衝突し得ない。
    ReadOnly,
    /// **固定名であること自体が仕掛けの仕様**。一意名にすると仕掛けが成立しない。
    IntentionallyFixed,
}

/// 窓口を通さずに入口を叩いてよいファイル（相対パス・種別・理由）。
///
/// **初期値は移行（タスク 10.2・10.3・10.4）と、その取りこぼしの是正（タスク 10.7）が
/// 完了した後の実測 16 件**で、この見張り自身が空の表で走ったときに返した集合と逐語で
/// 一致する（`crates/**/*.rs` を列挙し、窓口の `src/` を除き、コメント除去後に入口の
/// 呼出を持つファイル＝**16 ファイル・20 箇所**。2026-08-27 実測）。
///
/// # 起草の「16 ファイル」との突合
///
/// タスク 10.5 の task 文は「既にプロセス識別子で一意化している **16 ファイル**を例外表に
/// 載せるか窓口へ寄せるかを決め、根拠を記録する」と書いている。この 16 は要件 12.1 が同じ
/// 日付で記録した「移植元と同じ型（プロセス識別子＋単調連番＋`Drop` の再帰削除）が**実在
/// する** 16 ファイル」と同じ母数で、**入口を叩くファイルの数ではない**。
///
/// 本表が数えるのは「窓口の `src/` の外で入口を叩くファイル」——実測 16 ファイル・20 箇所、
/// 内訳は一意化 13・読み出しのみ 2・固定名が仕様 1。**総数がたまたま同じ 16 なので取り違え
/// やすい**が別の集合であり、「一意化している」と名乗る項目は 13 件しかない。起草の 16 を
/// [`PROCESS_UNIQUE_COUNT`] へそのまま写してはならない。
///
/// なお 10.7 の**前**の実測は 17 ファイル・23 箇所だった。差の 1 ファイル・3 箇所は
/// `crates/areka-sylphya/src/persist/io.rs`（入口 3 箇所のうち固定名で書込・削除を行う 2 箇所を
/// 要件 12.2 の対象選定が取りこぼしていた）で、10.7 が 3 箇所すべてを窓口へ寄せたため表から
/// 落ちた。表に置き去りにすると [`every_exception_still_has_a_real_hit`] が赤にする。
///
/// **理由欄に行番号を書かない**——書けば当該ファイルへの無関係な編集で表が陳腐化し、
/// 本仕様が触らないと決めたファイルを触らせる圧力になる（`file_length_guard_test.rs:58-60`
/// が同じ理由で行数を書かないのと同型）。
const ALLOWED_ENTRY_POINT_USES: &[(&str, Why, &str)] = &[
    // ── ⒜ 既にプロセス識別子で一意化している 13 件 ───────────────────────────
    //
    // 設計 C8 が「例外表に載せるか窓口へ寄せるかを実装時に決め、根拠を記録する」と
    // 定めた当の項目。**載せる**を採った。根拠は 3 点。
    //
    // ⑴ 要件 12.2 の移行対象は「入口から組み立て、**かつ書込または削除を行う** 20 ファイル」
    //    で、これらは対象外（＝既に正しい）と要件そのものが定めている。窓口へ寄せる作業は
    //    要件が範囲外と宣言した変更であり、要件 12.3（既存の主張・期待値・本数を変えない）を
    //    負う移行を、直す必要のないファイルへ広げることになる。
    // ⑵ これらのうち 2 件（`shiori_proxy.rs`・`process_host.rs`）は
    //    **本番ファイル**である。入口の呼出はいずれも `#[cfg(test)]` の内側だが（実測で確認）、
    //    要件 12.3 は「本番コードの挙動は 1 行も変えない」を明示的に課しており、本番ファイルへ
    //    dev-dependency の型を持ち込む判断は本タスクの境界（走査）を超える。
    // ⑶ 表に載せても**素通りにはならない**——[`each_exception_declares_whether_it_uniquifies_by_process_identifier`]
    //    が「一意化している」という主張をソースと突き合わせるので、後から一意化を外せば赤になる。
    (
        "crates/areka-emo-present/src/balloon_test_support.rs",
        Why::ProcessUnique,
        "プロセス識別子＋単調連番＋Drop の再帰削除を自前で持つ（窓口の移植元と同型）。要件 12.2 の移行対象外",
    ),
    (
        "crates/areka-ghost/src/shiori_inproc_adapter_tests.rs",
        Why::ProcessUnique,
        "プロセス識別子で一意化済み。要件 12.2 の移行対象外",
    ),
    (
        "crates/areka-ghost/src/shiori_inproc_tests.rs",
        Why::ProcessUnique,
        "プロセス識別子で一意化済み。要件 12.2 の移行対象外",
    ),
    (
        "crates/areka-ghost/tests/ghost/inproc_e2e_test.rs",
        Why::ProcessUnique,
        "プロセス識別子で一意化済み。要件 12.2 の移行対象外",
    ),
    (
        "crates/areka-ghost/tests/ghost/spine_e2e_test_s7_second_boot_record_present.rs",
        Why::ProcessUnique,
        "プロセス識別子で一意化済み。要件 12.2 の移行対象外",
    ),
    // `crates/areka-sylphya/src/persist/io.rs` はかつてここに載っていたが、タスク 10.7 が
    // 入口 3 箇所すべてを窓口へ寄せたので**当たりが 0 件になり、表から落とした**（本表の
    // doc の「起草の 16 ファイルとの突合」を参照）。
    (
        "crates/areka/src/placement/measure_tests.rs",
        Why::ProcessUnique,
        "プロセス識別子＋単調連番＋Drop の再帰削除を持つ**同 crate 内の自前実装**。タスク 10.2 は \
         placement_shared_test_support.rs を窓口の薄い包みへ寄せた際に「crate 内に 2 系統を残さない」と \
         説明文に書いたが、本ファイルが残っているのでその説明は事実として成り立っていない \
         （レビュー所見）。**衝突の観点では既に正しい**ので移行対象（要件 12.2 の 20 ファイル）には \
         含まれず、ここへ載せることで「2 系統目が現に在る」ことを台帳として可視にする",
    ),
    (
        "crates/shiori-host32-helper/src/main_loopback_tests.rs",
        Why::ProcessUnique,
        "プロセス識別子で一意化済み。要件 12.2 の移行対象外",
    ),
    (
        "crates/shiori-host32-helper/src/shiori_proxy.rs",
        Why::ProcessUnique,
        "本番ファイルの #[cfg(test)] 内。プロセス識別子で一意化済み（入口を素で読むだけの箇所も同居する）",
    ),
    (
        "crates/shiori-host32-host/src/process_host.rs",
        Why::ProcessUnique,
        "本番ファイルの #[cfg(test)] 内。プロセス識別子で一意化済み（入口を素で読むだけの箇所も同居する）",
    ),
    (
        "crates/shiori-host32-host/tests/lifecycle_cyclic_e2e.rs",
        Why::ProcessUnique,
        "プロセス識別子＋単調連番で一意化済み。要件 12.2 の移行対象外",
    ),
    (
        "crates/shiori-host32-host/tests/lifecycle_kill_e2e.rs",
        Why::ProcessUnique,
        "プロセス識別子＋単調連番で一意化済み。要件 12.2 の移行対象外",
    ),
    (
        "crates/shiori-host32-host/tests/shiori_load_e2e.rs",
        Why::ProcessUnique,
        "プロセス識別子＋単調連番で一意化済み。要件 12.2 の移行対象外",
    ),
    (
        "crates/shiori-host32-host/tests/shiori_request_e2e.rs",
        Why::ProcessUnique,
        "プロセス識別子＋単調連番で一意化済み。要件 12.2 の移行対象外",
    ),
    // ── ⒝ 読み出しのみの 2 件（要件 12.2・設計 C8 が名指しで対象外と定めた）───────
    (
        "crates/areka/src/placement/placement_monitor_tests.rs",
        Why::ReadOnly,
        "不在パスを組み立てて「読めないこと」を確かめるだけで、書込も削除も行わない。実体を作らない \
         ので固定名でもプロセス間で奪い合いが起きない（要件 12.2 が名指しで対象外と定めた 2 件の 1 つ）",
    ),
    (
        "crates/shiori-host32-host/tests/error_paths.rs",
        Why::ReadOnly,
        "helper の作業ディレクトリとして一時ディレクトリの根をそのまま渡すだけで、その下に何も作らず \
         何も消さない（設計 C8 が名指しで対象外と定めた 2 件の 1 つ）",
    ),
    // ── ⒞ 固定名であること自体が仕様の 1 件（タスク 10.3 の申し送り）──────────────
    (
        "crates/areka-ghost/tests/ghost/inproc_fixture.rs",
        Why::IntentionallyFixed,
        "OnceLock の static が保持する**プロセスを跨いで共有される fixture**。前プロセスが残した実体が \
         健全ならそのまま再利用して再組立を丸ごと省くのが仕掛けの目的で、**固定であることがその仕様**。 \
         窓口が配る一意名にすると再利用の判定が原理的に成立せず、リーク（static は Drop されない）だけが \
         毎プロセス積み上がる。窓口を迂回する唯一の意図的な箇所（タスク 10.3 の申し送り）",
    ),
];

/// [`ALLOWED_ENTRY_POINT_USES`] の件数（逐語）。表を増やすときはここも編集する（要件 12.4）。
const ALLOWED_COUNT: usize = 16;

/// 種別ごとの件数（逐語）。表の**中身**が黙って入れ替わらないようにする。
///
/// 件数の合計だけを縛ると、一意化を止めた 1 件を「読み出しのみ」と言い換えて総数を保つ、
/// という書き換えが通ってしまう。3 つに割っておけば分類の移動も明示的な編集になる。
const PROCESS_UNIQUE_COUNT: usize = 13;
/// 同上（読み出しのみ）。
const READ_ONLY_COUNT: usize = 2;
/// 同上（固定名が仕様）。
const INTENTIONALLY_FIXED_COUNT: usize = 1;

/// 要件 12.5 の自己テストで表から外す 1 件。表の項目でなければならない。
///
/// 窓口を迂回する唯一の意図的な箇所＝この見張りが本来鳴るべき当の形を選ぶ。
const CALIBRATION_DROPPED_ENTRY: &str = "crates/areka-ghost/tests/ghost/inproc_fixture.rs";

/// 本ファイル自身の相対パス（自己検査で使う）。
const THIS_GUARD: &str = "crates/log-capture-kit/tests/temp_path_guard_test.rs";

// ---------------------------------------------------------------------------
// 走査の取り回し
// ---------------------------------------------------------------------------

/// 走査で当たった 1 件。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Hit {
    /// ワークスペース根からの相対パス（区切りは `/`）。
    path: String,
    /// 1 始まりの行番号。
    line: usize,
    /// 当たった語。
    token: String,
}

/// 見張りの対象（窓口の定義を除いた全ソース）。
fn watched_sources() -> Vec<String> {
    walk_workspace_sources()
        .into_iter()
        .filter(|p| !p.starts_with(GATEWAY_DEFINITION_PREFIX))
        .collect()
}

/// 窓口の定義（走査から外した領域）。
fn gateway_sources() -> Vec<String> {
    walk_workspace_sources()
        .into_iter()
        .filter(|p| p.starts_with(GATEWAY_DEFINITION_PREFIX))
        .collect()
}

/// 与えられたファイル群を走査する。
fn scan_sources(paths: &[String], tokens: &[&str]) -> Vec<Hit> {
    let mut hits = Vec::new();
    for path in paths {
        for (line, token) in scan_tokens(&read_source(path), tokens) {
            hits.push(Hit {
                path: path.clone(),
                line,
                token,
            });
        }
    }
    hits
}

/// 例外表のパスだけを取り出す（判定に渡す形）。
fn allowed_paths() -> Vec<&'static str> {
    ALLOWED_ENTRY_POINT_USES
        .iter()
        .map(|(path, _, _)| *path)
        .collect()
}

/// 例外表から 1 件だけ落としたパスの一覧を作る（要件 12.5 の自己テストで使う）。
fn allowed_paths_without(dropped: &str) -> Vec<&'static str> {
    allowed_paths()
        .into_iter()
        .filter(|path| *path != dropped)
        .collect()
}

/// 例外表に載っていない当たり（＝違反）を返す（純関数）。
fn unlisted(hits: &[Hit], allow: &[&str]) -> Vec<Hit> {
    hits.iter()
        .filter(|h| !allow.contains(&h.path.as_str()))
        .cloned()
        .collect()
}

/// 違反の一覧を人が読める形にする（どのファイルの何行目かが分かること）。
fn render(hits: &[Hit]) -> String {
    hits.iter()
        .map(|h| format!("  {}:{} （語: {}）", h.path, h.line, h.token))
        .collect::<Vec<_>>()
        .join("\n")
}

// ---------------------------------------------------------------------------
// 主検査（要件 12.4）
// ---------------------------------------------------------------------------

#[test]
fn no_temp_dir_entry_point_lives_outside_the_gateway_and_the_allow_table() {
    let hits = scan_sources(&watched_sources(), &entry_point_tokens());
    let violations = unlisted(&hits, &allowed_paths());
    assert!(
        violations.is_empty(),
        "テスト用の一時パスは共通窓口（temp-path-kit の TempPath）から取ること。\
         入口を直接叩くとプロセス間で名前が衝突し、同じテストを複数プロセスで同時に走らせたときに\
         互いの一時ファイルを奪い合って落ちる（要件 12 の裁定の背景）。\
         やむを得ない場合は種別と理由付きで ALLOWED_ENTRY_POINT_USES へ明示的に追加すること\
         （表・ALLOWED_COUNT・種別ごとの件数の 3 箇所の編集になる）:\n{}",
        render(&violations)
    );
}

// ---------------------------------------------------------------------------
// 例外表が暗黙に増えない形（要件 12.4）
// ---------------------------------------------------------------------------

#[test]
fn the_allow_table_declares_its_own_size_and_reasons() {
    assert_eq!(
        ALLOWED_ENTRY_POINT_USES.len(),
        ALLOWED_COUNT,
        "例外表の件数が宣言と食い違う。項目の追加は表と件数の 2 箇所を明示的に編集すること（要件 12.4）"
    );

    let sources = walk_workspace_sources();
    for (path, _, reason) in ALLOWED_ENTRY_POINT_USES {
        assert!(
            !reason.trim().is_empty(),
            "例外には理由が要る（要件 12.4）: {path}"
        );
        assert!(
            !path.contains('*') && path.ends_with(".rs"),
            "例外はファイル 1 件を逐語で指すこと（総括的な指定は暗黙の増加を許す）: {path}"
        );
        assert!(
            sources.contains(&(*path).to_string()),
            "例外表が実在しないファイルを指している: {path}"
        );
    }
}

#[test]
fn the_allow_table_has_no_duplicate_entries() {
    // 同じパスが 2 度載ると、件数の宣言を通ったまま実質の例外が 1 件減る
    //（＝表の見かけの大きさと効き目が食い違う）。
    let unique: BTreeSet<&str> = allowed_paths().into_iter().collect();
    assert_eq!(
        unique.len(),
        ALLOWED_COUNT,
        "例外表に重複した項目がある: {:?}",
        allowed_paths()
    );
}

#[test]
fn the_allow_table_declares_the_size_of_each_category() {
    let count = |want: Why| {
        ALLOWED_ENTRY_POINT_USES
            .iter()
            .filter(|(_, why, _)| *why == want)
            .count()
    };
    assert_eq!(
        (
            count(Why::ProcessUnique),
            count(Why::ReadOnly),
            count(Why::IntentionallyFixed)
        ),
        (
            PROCESS_UNIQUE_COUNT,
            READ_ONLY_COUNT,
            INTENTIONALLY_FIXED_COUNT
        ),
        "種別ごとの件数が宣言と食い違う。分類の移動も明示的な編集として行うこと（要件 12.4）"
    );
    assert_eq!(
        PROCESS_UNIQUE_COUNT + READ_ONLY_COUNT + INTENTIONALLY_FIXED_COUNT,
        ALLOWED_COUNT,
        "種別ごとの件数の合計が総数と合わない"
    );
}

/// 例外の種別（[`Why`]）をソースと突き合わせる。
///
/// # この突合は**ファイル単位**である（既知の限界・意図して残している）
///
/// 判定は「そのファイルのどこかにプロセス識別子の呼出が **1 箇所でも**あれば
/// [`Why::ProcessUnique`] を通す」形である。したがって**入口が複数あるファイルの中で、
/// 一部だけが固定名へ劣化した**場合はこの検査を素通りする。
///
/// **この穴は机上の話ではない。** 要件 12.2 の対象選定そのものが同じファイル単位の絞り込みで
/// 書かれていて（「ファイルのどこかに識別子があれば一意化済みとみなして除外」）、
/// `crates/areka-sylphya/src/persist/io.rs` を丸ごと取りこぼしていた——入口 3 箇所のうち識別子を
/// 使うのは 1 箇所だけで、残り 2 箇所は**固定名で書込と削除**を行っていた。取りこぼしは
/// タスク 10.5 のレビューで初めて見つかり、タスク 10.7 が事実のほうを是正した（要件 12.2 の
/// 2026-08-27 訂正・対象は 20 → 21 ファイル）。**つまり本検査は、自分を素通りした欠陥の存在を
/// 知っている。**
///
/// # 是正後に残る「入口が複数あるファイル」は 2 件（2026-08-27 実測）
///
/// - `crates/shiori-host32-helper/src/shiori_proxy.rs` — 入口 3・識別子 2
/// - `crates/shiori-host32-host/src/process_host.rs` — 入口 3・識別子 1
///
/// いずれも**安全であることを実測で確認済み**である。書込を伴う箇所はすべて識別子を織り込んだ
/// 名前を組み立てており、識別子を持たない箇所は一時ディレクトリの根を「実在するディレクトリ」
/// として読むだけ（そのまま作業ディレクトリとして渡す／`canonicalize` するだけ）で、その下に
/// 何も作らず何も消さない。**この 2 件が将来どう変わるかまでは本検査は見ていない。**
///
/// # なぜ行単位へ強めないのか
///
/// 行単位にすると理由欄が行番号を抱え、当該ファイルへの無関係な編集で表が黙って陳腐化し、
/// 本仕様が触らないと決めたファイルを触らせる圧力になる（1 ファイル 1,000 行の見張りが
/// まったく同じ理由で行数を書かないのと同型）。**限界をここへ逐語で書き残すことで折り合いを
/// 付けている**——「未達が spec の内側から見えない」を再演しないため。
#[test]
fn each_exception_declares_whether_it_uniquifies_by_process_identifier() {
    // 散文の理由は書いた本人以外に検証できない。種別だけはソースと突き合わせて裏を取る。
    // まず走査語が空振りしていないことを見本で確かめる（さもないと以下は恒真になる）。
    let planted = format!("    let n = std::{TOKEN_PROCESS_ID});\n");
    assert_eq!(
        scan_tokens(&planted, &[TOKEN_PROCESS_ID]).len(),
        1,
        "プロセス識別子の走査語が見本に当たらない＝以下の突合は無意味になる"
    );

    let mut wrong = Vec::new();
    for (path, why, _) in ALLOWED_ENTRY_POINT_USES {
        let found = !scan_tokens(&read_source(path), &[TOKEN_PROCESS_ID]).is_empty();
        match (why, found) {
            (Why::ProcessUnique, false) => wrong.push(format!(
                "  {path} は ProcessUnique を名乗るがプロセス識別子を使っていない"
            )),
            (Why::ReadOnly | Why::IntentionallyFixed, true) => wrong.push(format!(
                "  {path} は {why:?} を名乗るがプロセス識別子を使っている（分類が誤り）"
            )),
            _ => {}
        }
    }
    assert!(
        wrong.is_empty(),
        "例外の種別がソースと食い違う。種別は機械で裏の取れる分類なので、実体に合わせるか\
         窓口へ寄せること:\n{}",
        wrong.join("\n")
    );
}

#[test]
fn every_exception_still_has_a_real_hit() {
    // 「違反 0 件」は道具が壊れていても成立する。表の全項目に**今も**当たりがあることを
    // 要求すると、列挙や走査が空振りしていれば必ず赤になる（＝主検査の非空虚性の担保）。
    // あわせて、窓口へ寄って入口を叩かなくなった項目を表に置き去りにできなくする
    //（例外が減る方向だけが自然になる）。
    let hits = scan_sources(&watched_sources(), &entry_point_tokens());
    let stale: Vec<&str> = allowed_paths()
        .into_iter()
        .filter(|path| !hits.iter().any(|h| h.path == *path))
        .collect();
    assert!(
        stale.is_empty(),
        "例外表に載っているのに当たりが 1 件も無い（走査が空振りしているか、事情が消えた）。\
         事情が消えたのなら表から削除すること: {stale:?}"
    );
}

#[test]
fn the_measurement_is_not_vacuous_and_matches_the_allow_table() {
    // 陽性側の相棒。⑴ 列挙が現実的な規模であること ⑵ 入口の呼出が確かに存在すること
    // ⑶ その集合が例外表と**完全に一致**すること（要件 12.4 の「移行後の実測値で開始」）。
    let watched = watched_sources();
    assert!(
        watched.len() > 500,
        "列挙が極端に少ない＝走査が空振りしている疑い: {} 件",
        watched.len()
    );

    let hits = scan_sources(&watched, &entry_point_tokens());
    assert!(
        !hits.is_empty(),
        "入口の呼出が 1 件も無い。移行後の実測は 16 ファイル・20 箇所だったので、\
         走査か列挙が壊れている疑いが濃い"
    );

    let found: BTreeSet<&str> = hits.iter().map(|h| h.path.as_str()).collect();
    let listed: BTreeSet<&str> = allowed_paths().into_iter().collect();
    assert_eq!(
        found, listed,
        "実測した集合が例外表と一致しない。新たな迂回なら窓口へ寄せるか表へ明示的に追加し、\
         解消したのなら表から削除すること"
    );
}

#[test]
fn the_gateway_is_the_only_excluded_region_and_it_really_holds_the_entry_point() {
    // 除外が「効いている」ことの陽性側。除外領域には確かに入口の呼出があり、
    // 除外を外せば主検査は赤になる（＝除外が飾りでない）。
    let hits = scan_sources(&gateway_sources(), &entry_point_tokens());
    let files: BTreeSet<&str> = hits.iter().map(|h| h.path.as_str()).collect();
    assert_eq!(
        files,
        BTreeSet::from(["crates/temp-path-kit/src/lib.rs"]),
        "窓口が入口を叩く箇所が動いている。要件 12.1 は入口の組み立てを 1 箇所に保つことを求めている"
    );
}

// ---------------------------------------------------------------------------
// 較正（要件 12.5・12.7）
// ---------------------------------------------------------------------------

#[test]
fn dropping_a_known_exception_turns_the_guard_red() {
    // 要件 12.5 が名指しで求める自己テスト。見本ではなく**実データ**で行う——
    // 見本で作った両側は下の合成入力の検査が持っており、ここで確かめたいのは
    //「この表とこのワークスペースの組み合わせで、見張りが本当に鳴るか」である。
    assert!(
        allowed_paths().contains(&CALIBRATION_DROPPED_ENTRY),
        "較正で外す 1 件は例外表の項目でなければならない: {CALIBRATION_DROPPED_ENTRY}"
    );

    let hits = scan_sources(&watched_sources(), &entry_point_tokens());
    let violations = unlisted(&hits, &allowed_paths_without(CALIBRATION_DROPPED_ENTRY));
    assert!(
        !violations.is_empty(),
        "例外表から 1 件外しても違反が 0 件＝見張りは鳴っていない（何も検知できていない）"
    );
    let paths: BTreeSet<&str> = violations.iter().map(|h| h.path.as_str()).collect();
    assert_eq!(
        paths,
        BTreeSet::from([CALIBRATION_DROPPED_ENTRY]),
        "外した 1 件だけが違反として返るはず"
    );
}

#[test]
fn a_fixed_name_built_from_the_entry_point_is_detected() {
    // 陽性の合成入力（実ファイルを汚さずに済む）。要件 12.4 が名指しする欠陥の形そのもの。
    let source = format!(
        "fn plant() -> PathBuf {{\n    \
         let root = std::{TOKEN_QUALIFIED}).join(\"areka-fixed-name\");\n    \
         std::fs::create_dir_all(&root).unwrap();\n    root\n}}\n"
    );
    assert_eq!(
        scan_tokens(&source, &entry_point_tokens()),
        vec![(2usize, TOKEN_QUALIFIED.to_string())],
        "入口から固定名を組む式を検知できていない"
    );
}

#[test]
fn the_bare_spelling_of_the_entry_point_is_detected_too() {
    // `use std::env::temp_dir;` と書けば修飾は消える。裸の綴りも走査語に含めているので当たる。
    let source = format!("    let root = {TOKEN_BARE}).join(\"areka-fixed-name\");\n");
    assert_eq!(
        scan_tokens(&source, &entry_point_tokens()),
        vec![(1usize, TOKEN_BARE.to_string())],
        "裸の綴りを検知できていない（use で修飾を外すだけで見張りを抜けられてしまう）"
    );
}

#[test]
fn the_two_spellings_never_double_count_one_call() {
    // 裸の綴りは修飾つきの綴りの内側にも現れる。1 呼出が 2 件に数えられると、
    // 件数を語る主張（実測 20 箇所）が静かにずれる。
    let source = format!("    let root = std::{TOKEN_QUALIFIED});\n");
    assert_eq!(
        scan_tokens(&source, &entry_point_tokens()),
        vec![(1usize, TOKEN_QUALIFIED.to_string())],
        "1 呼出が 2 件に数えられている（語の重なりの規則が効いていない）"
    );
}

#[test]
fn a_helper_whose_name_merely_ends_with_the_same_spelling_is_not_a_hit() {
    // 実在の助走関数（`unique_temp_dir`・`make_unique_temp_dir`）に当たってはいけない。
    // 当たると例外表が偽陽性で膨らみ、表の意味が失われる。
    let source = format!(
        "fn unique_{TOKEN_BARE}tag: &str) -> TempPath {{\n    \
         TempPath::new(tag)\n}}\n\n    \
         let temp = make_unique_{TOKEN_BARE}\"tag\");\n"
    );
    assert_eq!(
        scan_tokens(&source, &entry_point_tokens()),
        Vec::new(),
        "名前の末尾がたまたま同じ綴りで終わる助走関数に当たっている（左端のアンカーが効いていない）"
    );
}

#[test]
fn a_fixed_name_in_a_comment_is_not_a_hit_but_the_same_line_of_code_is() {
    // 要件 12.7。**対で置く**——コメント側が 0 件でも走査そのものが空振りなら同じ結果になるので、
    // 実行行側で 1 件当たることを同じテストの中で示す。
    let body = format!("let root = std::{TOKEN_QUALIFIED}).join(\"areka-fixed-name\");");

    let commented = format!("    // かつては {body} と書いていた\n");
    assert_eq!(
        scan_tokens(&commented, &entry_point_tokens()),
        Vec::new(),
        "コメント中の語を拾っている。要件 12.7 が記録する事故（コメント中の語で判定が反転する）\
         と同じ形で、走査は必ずコメント除去を通すこと"
    );

    let block_commented = format!("    /* 旧実装:\n       {body}\n    */\n");
    assert_eq!(
        scan_tokens(&block_commented, &entry_point_tokens()),
        Vec::new(),
        "塊コメント中の語を拾っている"
    );

    let executable = format!("    {body}\n");
    assert_eq!(
        scan_tokens(&executable, &entry_point_tokens()),
        vec![(1usize, TOKEN_QUALIFIED.to_string())],
        "実行行で当たらない＝走査そのものが空振りしており、上の 0 件は何も意味しない"
    );

    // 行末コメントの手前は実行行なので当たる（除去で行が丸ごと消えていないことの確認）。
    let trailing = format!("    {body} // 一時ディレクトリの入口\n");
    assert_eq!(
        scan_tokens(&trailing, &entry_point_tokens()),
        vec![(1usize, TOKEN_QUALIFIED.to_string())],
        "行末コメントの除去が行の実行部分まで巻き込んでいる"
    );
}

// ---------------------------------------------------------------------------
// 見張り自身が走査語を逐語で持たないこと
// ---------------------------------------------------------------------------

#[test]
fn the_guard_file_itself_is_not_a_hit() {
    let mut tokens = entry_point_tokens();
    tokens.push(TOKEN_PROCESS_ID);
    assert_eq!(
        scan_sources(&[THIS_GUARD.to_string()], &tokens),
        Vec::new(),
        "見張り自身が走査語を逐語で持ってしまっている（concat! で割ること）"
    );
}

/// コメントを**除かずに**生テキストから走査語を探す（純関数）。
///
/// 要件 12.2 の実測に使った `rg`／`grep` と同じ見方（アンカーもコメント除去もしない素の
/// 部分一致）にそろえてある。**母数が動く条件をそのまま写すのが目的なので、ここを
/// 賢くしてはいけない。**
fn raw_occurrences(src: &str, tokens: &[&str]) -> Vec<(usize, String)> {
    let mut hits = Vec::new();
    for (index, line) in src.lines().enumerate() {
        for token in tokens {
            if line.contains(token) {
                hits.push((index + 1, (*token).to_string()));
            }
        }
    }
    hits
}

#[test]
fn the_guard_file_never_spells_the_tokens_out_not_even_inside_comments() {
    let mut tokens = entry_point_tokens();
    tokens.push(TOKEN_PROCESS_ID);

    // 較正: 生テキスト走査はコメントを除かない。除いてしまうと本検査は恒真になる
    //（コメント除去を通す上の検査は、コメントへ植えた走査語を 1 件も拾わない）。
    //
    // 1 行に**両方の綴りが当たる**のが正しい期待値である——`raw_occurrences` は
    // アンカーも語の重なりの規則も持たない素の部分一致で、修飾つきの綴りの内側にある
    // 裸の綴りを別に数える。要件 12.2 の実測に使った grep がまさにそう振る舞うので、
    // ここを賢くして 1 件へ丸めると母数の見方が実測とずれる。
    let planted = format!("// 説明: かつては std::{TOKEN_QUALIFIED}) と書いていた");
    assert_eq!(
        raw_occurrences(&planted, &tokens),
        vec![
            (1usize, TOKEN_QUALIFIED.to_string()),
            (1usize, TOKEN_BARE.to_string()),
        ],
        "コメントの中でも生テキスト走査は当たらねばならない（当たらないなら本検査は恒真）"
    );

    let violations: Vec<String> = raw_occurrences(&read_source(THIS_GUARD), &tokens)
        .into_iter()
        .map(|(line, token)| format!("  {THIS_GUARD}:{line} （語: {token}）"))
        .collect();
    assert!(
        violations.is_empty(),
        "走査語はコメントの中も含めて逐語で書いてはいけない。\
         ⒜ 見張りが自分自身を違反として拾い ⒝ 要件 12.2 の実測に使った grep の母数が黙って動く\
         （コメントは見張りが除去するが grep は除去しない）。concat! で 2 片に割ること:\n{}",
        violations.join("\n")
    );
}
