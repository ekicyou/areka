//! 文書の置き場の照合テスト（spec: doc-tree-tidy・設計 C4）。
//!
//! 案内 `doc/README.md` の表と、`doc/` の実物と、名前の決まりの食い違いを見つける。
//!
//! # 部品の分け方
//!
//! 案内の表の読み方（[`table_first_cells`]）と、表の行と名前の照らし方（[`row_matches`]）は、
//! 文字列だけを受ける純粋な関数で、ファイルシステムにも git にもネットにも時刻にも触らない
//! （要件 4.10）。純粋な側は見本の文字列で確かめられるので、「食い違い 0 件だから緑」の形の
//! 主検査が読み方の故障で緑になっていないことを別立てで示せる。
//!
//! # 案内の表の書式（設計「案内の表（照合テストが読む書式）」）
//!
//! - 表は見出しの文字（完全一致）で見分ける。見出しの下、次の見出しの手前までにある最初の
//!   `|` の行の塊を表とする。
//! - 表の 1 行目は列の名前、2 行目は区切り。残りの各行の 1 列目はバッククォート 1 組で
//!   囲んだ名前 1 つ。フォルダは名前の末尾に `/` を付ける。
//! - 直下の表の 1 列目には置き字 [`DATE_PLACEHOLDER`] を使ってよい。
//!
//! 依存は標準ライブラリだけ（要件 6.3）。置き場は見直しの受け持つクレートの外の
//! `crates/log-capture-kit/tests/`（要件 6.4）。

// ---------------------------------------------------------------------------
// 読む見出しと置き字
// ---------------------------------------------------------------------------

use std::collections::BTreeSet;
use std::fmt;

/// 直下の表の見出し（完全一致）。1 列目は `doc/` の直下の名前（`doc/` を付けない）。
const DIRECT_HEADING: &str = "## 直下のファイルとフォルダ";

/// 例外の表の見出し（完全一致）。1 列目はリポジトリの根からのパス（`doc/...`）。
const EXCEPTION_HEADING: &str = "## 名前の決まりの例外";

/// 直下の表の行に使える日付の置き字。4 桁・2 桁・2 桁の ASCII の数字をハイフンでつないだ形に当たる。
const DATE_PLACEHOLDER: &str = "{YYYY-MM-DD}";

// ---------------------------------------------------------------------------
// 食い違いの種類
// ---------------------------------------------------------------------------

/// 食い違い 1 件。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Finding {
    /// 直下の実物が表のどの行にも当たらない（4.2）。「表に行を足すか、置き場を直す」
    Unlisted(String),
    /// 表の行（置き字の無い行）が名指すものが無い（4.3）。「行を消すか直す」
    MissingRow(String),
    /// 根に docs フォルダがある（4.5）。「doc/ へまとめる」
    DocsDirExists,
    /// 名前の決まりに合わず、例外の表にも無い（4.6）。「名前を直すか、例外の表に理由と行方つきで足す」
    BadName(String),
    /// 例外の行が覆う破れが 1 つも無い（4.7）。「行を消す」
    StaleException(String),
    /// 表の行の 1 列目がバッククォートの名前 1 つの形でない。「表の書き方に合わせる」
    /// `line` は案内の 1 始まりの行番号。
    MalformedRow { heading: &'static str, line: usize },
    /// 読み取りが 1 件も拾わない（4.8）。「見出しの文字が変わったか、読み方が壊れた」
    EmptyReading(&'static str),
}

// ---------------------------------------------------------------------------
// 純粋な側: 案内の表の読み方
// ---------------------------------------------------------------------------

/// 案内 `guide` の見出し `heading` の下の表から、各行の 1 列目の名前を取る。
///
/// 改行は CRLF も LF も受ける。見出しが無いとき・見出しの下（次の見出しの手前まで）に
/// 表が無いときは空を返す（次の節の表を拾わない）。1 列目の形が崩れた行は名前に入れず、
/// [`Finding::MalformedRow`] として返す。
fn table_first_cells(guide: &str, heading: &'static str) -> (Vec<String>, Vec<Finding>) {
    let mut cells = Vec::new();
    let mut findings = Vec::new();

    // (1 始まりの行番号, 前後の空白を落とした行)。`lines` が LF と CRLF の両方を切り、trim が残りの \r も落とす。
    let mut lines = guide.lines().enumerate().map(|(i, l)| (i + 1, l.trim()));
    if !lines.any(|(_, l)| l == heading) {
        return (cells, findings);
    }
    let section = lines.take_while(|(_, l)| !l.starts_with('#'));
    let table = section
        .skip_while(|(_, l)| !l.starts_with('|'))
        .take_while(|(_, l)| l.starts_with('|'));

    for (index, (line_no, line)) in table.enumerate() {
        if index == 0 || (index == 1 && is_separator_row(line)) {
            continue; // 列の名前の行と区切りの行
        }
        match first_cell_name(line) {
            Some(name) => cells.push(name.to_string()),
            None => findings.push(Finding::MalformedRow {
                heading,
                line: line_no,
            }),
        }
    }
    (cells, findings)
}

/// 表の区切りの行（`|`・`-`・`:`・空白だけで、`-` を 1 つ以上含む）か。
fn is_separator_row(line: &str) -> bool {
    line.contains('-')
        && line
            .chars()
            .all(|c| matches!(c, '|' | '-' | ':') || c.is_whitespace())
}

/// 表の行の 1 列目が「バッククォート 1 組で囲んだ空でない名前 1 つ」なら、その名前を返す。
fn first_cell_name(line: &str) -> Option<&str> {
    let rest = line.strip_prefix('|')?;
    let cell = rest.split('|').next()?.trim();
    let name = cell.strip_prefix('`')?.strip_suffix('`')?;
    (!name.is_empty() && !name.contains('`')).then_some(name)
}

// ---------------------------------------------------------------------------
// 純粋な側: 表の行と名前の照らし方
// ---------------------------------------------------------------------------

/// 表の行 `row` が名前 `name` に当たるか。
///
/// 置き字の無い行は文字の完全一致。置き字の行は、置き字の前と後ろが名前の頭と尻に合い、
/// 間がちょうど `DDDD-DD-DD`（`D` は ASCII の数字）のときだけ当たる。フォルダの名前は末尾に
/// `/` を持つので、フォルダの行はフォルダにだけ、ファイルの行はファイルにだけ当たる。
fn row_matches(row: &str, name: &str) -> bool {
    let Some((prefix, suffix)) = row.split_once(DATE_PLACEHOLDER) else {
        return row == name;
    };
    name.strip_prefix(prefix)
        .and_then(|rest| rest.strip_suffix(suffix))
        .is_some_and(is_date_shape)
}

/// `DDDD-DD-DD`（`D` は ASCII の数字）ちょうどか。
fn is_date_shape(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10
        && b.iter().enumerate().all(|(i, c)| match i {
            4 | 7 => *c == b'-',
            _ => c.is_ascii_digit(),
        })
}

// ---------------------------------------------------------------------------
// 純粋な側: 名前の決まりと突き合わせ
// ---------------------------------------------------------------------------

/// 0 件を名指すときの読み取りの名前（[`Finding::EmptyReading`]）。
const DIRECT_ENTRIES: &str = "doc/ の直下の実物";
const DIRECT_TABLE: &str = "直下の表";
const EXCEPTION_TABLE: &str = "例外の表";

/// 実物はディスクで数えるので、手元にだけあるファイルで赤になりうる。そのとき表に要らない行を足させない一句。
const UNTRACKED_HINT: &str = "追跡していないファイルなら取り除く（`git status` で確かめられる）";

/// `doc/` の実物（ディスクの実際の字のまま）。集めるのはファイルシステムに触る側（タスク 3）。
struct DocTree {
    /// `doc/` の直下の名前（`doc/` を付けない。フォルダは `name/`）。
    direct: BTreeSet<String>,
    /// `doc/` の下の全部（`doc/archive/` の下を除く）。根からのパス（`doc/...`）。
    all: BTreeSet<String>,
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unlisted(n) => write!(
                f,
                "doc/{n}: 直下の表に無い。表に行を足すか、置き場を直す。{UNTRACKED_HINT}"
            ),
            Self::MissingRow(n) => {
                write!(f, "直下の表の行 `{n}`: 名指すものが無い。行を消すか直す")
            }
            Self::DocsDirExists => write!(f, "根の docs フォルダ: ある。中身を doc へまとめて消す"),
            Self::BadName(n) => write!(
                f,
                "{n}: 名前の決まりに合わず、例外の表にも無い。名前を直すか、例外の表に理由と行方つきで足す。{UNTRACKED_HINT}"
            ),
            Self::StaleException(n) => {
                write!(f, "例外の表の行 `{n}`: 覆う破れが 1 つも無い。行を消す")
            }
            Self::MalformedRow { heading, line } => write!(
                f,
                "doc/README.md の {line} 行目（{heading}）: 1 列目がバッククォートの名前 1 つでない。表の書き方に合わせる"
            ),
            Self::EmptyReading(what) => write!(
                f,
                "{what}: 1 件も拾わない。見出しの文字が変わったか、読み方が壊れた"
            ),
        }
    }
}

/// 名前（パスの最後の 1 区切り。フォルダは末尾の `/` を除く）が決まりに合うか。
/// 字の種類（`a-z`・`0-9`・`-`・`.`）だけを見て、続き方は見ない。`README.md` ちょうどは決まりの外。
fn is_conforming(path: &str) -> bool {
    let name = path.trim_end_matches('/').rsplit('/').next().unwrap_or("");
    name == "README.md"
        || (!name.is_empty()
            && name
                .bytes()
                .all(|b| matches!(b, b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.')))
}

/// 直下の実物と直下の表を突き合わせる。置き字の行は当たりが 0 でも何も出さない（4.4）。
fn check_direct(direct: &BTreeSet<String>, rows: &[String]) -> Vec<Finding> {
    let unlisted = direct
        .iter()
        .filter(|n| !rows.iter().any(|r| row_matches(r, n)))
        .map(|n| Finding::Unlisted(n.clone()));
    let missing = rows
        .iter()
        .filter(|r| !r.contains(DATE_PLACEHOLDER) && !direct.contains(*r))
        .map(|r| Finding::MissingRow(r.clone()));
    unlisted.chain(missing).collect()
}

/// 決まりの破れを例外の表と突き合わせる。フォルダの行（末尾 `/`）は下の全部を覆う。
fn check_names(all: &BTreeSet<String>, exceptions: &[String]) -> Vec<Finding> {
    let covers = |r: &str, v: &str| v == r || (r.ends_with('/') && v.starts_with(r));
    let violations: Vec<&String> = all.iter().filter(|p| !is_conforming(p)).collect();
    let bad = violations
        .iter()
        .filter(|v| !exceptions.iter().any(|r| covers(r, v)))
        .map(|v| Finding::BadName(v.to_string()));
    // 名指すものが無い・もう決まりに合う・フォルダの下に破れが無い、のどれも「覆う破れが無い」に入る。
    let stale = exceptions
        .iter()
        .filter(|r| !violations.iter().any(|v| covers(r, v)))
        .map(|r| Finding::StaleException(r.clone()));
    bad.chain(stale).collect()
}

/// 食い違いを全部集める。読み取りが 0 件の突き合わせは、見当違いの行を並べないよう
/// [`Finding::EmptyReading`] だけにする（4.8）。
fn check_all(tree: &DocTree, guide: &str, docs_exists: bool) -> BTreeSet<Finding> {
    let (rows, mut found) = table_first_cells(guide, DIRECT_HEADING);
    let (exceptions, malformed) = table_first_cells(guide, EXCEPTION_HEADING);
    found.extend(malformed);
    for (what, empty) in [
        (DIRECT_ENTRIES, tree.direct.is_empty()),
        (DIRECT_TABLE, rows.is_empty()),
        (EXCEPTION_TABLE, exceptions.is_empty()),
    ] {
        if empty {
            found.push(Finding::EmptyReading(what));
        }
    }
    if !tree.direct.is_empty() && !rows.is_empty() {
        found.extend(check_direct(&tree.direct, &rows));
    }
    if !exceptions.is_empty() {
        found.extend(check_names(&tree.all, &exceptions));
    }
    if docs_exists {
        found.push(Finding::DocsDirExists);
    }
    found.into_iter().collect()
}

// ---------------------------------------------------------------------------
// 見本の試験
// ---------------------------------------------------------------------------

/// 見本の案内（LF）。直下の表・例外の表・対応の表を持ち、見出しの下に表の無い節も置く。
const SAMPLE_GUIDE: &str = "\
# doc/

冒頭の 1 文。

## 決まり

- 表の書き方の説明。

## 直下のファイルとフォルダ

| 名前 | 役目 | 今か歴史か |
|---|---|---|
| `README.md` | 案内 | 今 |
| `archive/` | 歴史 | 歴史 |
| `roadmap-overview-{YYYY-MM-DD}.html` | 俯瞰 | 今 |

## 名前の決まりの例外

| 名前 | 理由 | 行方 |
|---|---|---|
| `doc/COMPAT_ARCHITECTURE.md` | 並んで走る見直しとの約束 | 後で改名する |
| `doc/shiori/fragments/` | 道具が読む目録 | 改名しない |

## 動かした名前の対応

| 古いパス | 新しいパス |
|---|---|
| `old/old_name.md` | `doc/new-name.md` |
| `doc/OLD_RULES.md` | `doc/old-rules.md` |

## 俯瞰の HTML

見出しの下に表の無い節。
";

#[test]
fn date_placeholder_reads_only_the_date_shape() {
    let row = "roadmap-overview-{YYYY-MM-DD}.html";

    assert!(row_matches(row, "roadmap-overview-2026-10-16.html"));
    for miss in [
        "roadmap-overview-2026-1-16.html",
        "roadmap-overview-latest.html",
        "roadmap-overview-2026-10-16.htm",
        "roadmap-overview-2026x10-16.html",
        "roadmap-overview-２０２６-10-16.html", // ASCII でない数字
        "roadmap-overview-2026-10-16.html/",    // 同じ形のフォルダ
        "roadmap-overview-{YYYY-MM-DD}.html",   // 置き字そのもの
    ] {
        assert!(
            !row_matches(row, miss),
            "当たってはならない名前に当たった: {miss}"
        );
    }

    // フォルダの行はフォルダにだけ、ファイルの行はファイルにだけ当たる。
    assert!(row_matches("notes-{YYYY-MM-DD}/", "notes-2026-11-05/"));
    assert!(!row_matches("notes-{YYYY-MM-DD}/", "notes-2026-11-05"));
    assert!(!row_matches("README.md/", "README.md"));
    assert!(row_matches("README.md", "README.md"));

    // 置き字の行は、当たる実物が 0 でも表の書き方の破れにならない（当たり 0 で赤にしない・4.4）。
    let (rows, findings) = table_first_cells(SAMPLE_GUIDE, DIRECT_HEADING);
    assert!(
        findings.is_empty(),
        "見本の直下の表に破れが出た: {findings:?}"
    );
    assert!(
        rows.iter().any(|r| r == row),
        "置き字の行が読めていない: {rows:?}"
    );
    let direct_without_dated = BTreeSet::from(["README.md", "archive/"].map(String::from));
    assert_eq!(check_direct(&direct_without_dated, &rows), vec![]);
}

#[test]
fn tables_are_told_apart_by_heading() {
    let direct = [
        "README.md",
        "archive/",
        "roadmap-overview-{YYYY-MM-DD}.html",
    ];
    let exceptions = ["doc/COMPAT_ARCHITECTURE.md", "doc/shiori/fragments/"];
    let crlf = SAMPLE_GUIDE.replace('\n', "\r\n");

    for guide in [SAMPLE_GUIDE, crlf.as_str()] {
        // 対応の表の 1 列目（`old/old_name.md` など）は、直下の表にも例外の表にも混ざらない。
        assert_eq!(
            table_first_cells(guide, DIRECT_HEADING),
            (direct.map(String::from).to_vec(), vec![])
        );
        assert_eq!(
            table_first_cells(guide, EXCEPTION_HEADING),
            (exceptions.map(String::from).to_vec(), vec![])
        );

        // 見出しが無いと 0 件。
        assert_eq!(table_first_cells(guide, "## 無い見出し"), (vec![], vec![]));

        // 見出しはあるが下に表が無いと 0 件（次の節の表を拾わない）。
        assert_eq!(table_first_cells(guide, "## 決まり"), (vec![], vec![]));
        assert_eq!(table_first_cells(guide, "## 俯瞰の HTML"), (vec![], vec![]));
    }

    // 直下の表が消えた案内は、次の節（例外の表）の行を拾わずに 0 件になる。
    let removed = "\
## 直下のファイルとフォルダ

## 名前の決まりの例外

| 名前 | 理由 | 行方 |
|---|---|---|
| `doc/COMPAT_ARCHITECTURE.md` | 理由 | 行方 |
";
    assert_eq!(table_first_cells(removed, DIRECT_HEADING), (vec![], vec![]));

    // 1 列目がバッククォートの名前 1 つでない行は、案内の行番号つきの破れになる。
    let malformed = "\
## 直下のファイルとフォルダ\r
\r
| 名前 | 役目 |\r
|:--|--:|\r
| `README.md` | 案内 |\r
| constitution.md | 囲みが無い |\r
| `a.md` `b.md` | 名前が 2 つ |\r
| `` | 空 |\r
| `ok/` | フォルダ |\r
";
    assert_eq!(
        table_first_cells(malformed, DIRECT_HEADING),
        (
            vec!["README.md".to_string(), "ok/".to_string()],
            [6, 7, 8]
                .map(|line| Finding::MalformedRow {
                    heading: DIRECT_HEADING,
                    line,
                })
                .to_vec(),
        )
    );
}

#[test]
fn an_exception_row_with_nothing_to_cover_turns_red() {
    let all = BTreeSet::from(
        [
            "doc/Kept_Name.md",
            "doc/Uncovered_Name.md",
            "doc/clean.md",
            "doc/data/",
            "doc/data/X_Y.toml",
            "doc/tidy/",
            "doc/tidy/ok.md",
        ]
        .map(String::from),
    );
    let exceptions = [
        "doc/Kept_Name.md", // 覆う破れがある
        "doc/data/",        // フォルダの下に破れがある
        "doc/Gone_Name.md", // 名指すものが無い
        "doc/clean.md",     // もう決まりに合う
        "doc/tidy/",        // フォルダの下に破れが無い
    ]
    .map(String::from);

    let mut found = check_names(&all, &exceptions);
    found.sort();
    assert_eq!(
        found,
        vec![
            Finding::BadName("doc/Uncovered_Name.md".into()),
            Finding::StaleException("doc/Gone_Name.md".into()),
            Finding::StaleException("doc/clean.md".into()),
            Finding::StaleException("doc/tidy/".into()),
        ]
    );

    // 名前の決まりは字の種類だけを見る。`README.md` ちょうどは決まりの外。
    for ok in ["doc/a-b.c.md", "doc/v1.2/", "doc/x/README.md", "doc/--..md"] {
        assert!(is_conforming(ok), "合うはずの名前: {ok}");
    }
    for bad in [
        "doc/Readme.md",
        "doc/readme.MD",
        "doc/a_b.md",
        "doc/a b.md",
        "doc/あ.md",
        "doc/Up/",
    ] {
        assert!(!is_conforming(bad), "合わないはずの名前: {bad}");
    }

    // 決まりの破れの直し方には、追跡していないファイルの取り除き方を添える。
    let line = Finding::BadName("doc/Uncovered_Name.md".into()).to_string();
    assert!(
        line.contains("doc/Uncovered_Name.md") && line.contains(UNTRACKED_HINT),
        "{line}"
    );
}

#[test]
fn a_docs_dir_turns_the_guard_red() {
    let tree = DocTree {
        direct: BTreeSet::from(["README.md", "archive/"].map(String::from)),
        all: BTreeSet::from(
            [
                "doc/README.md",
                "doc/COMPAT_ARCHITECTURE.md",
                "doc/shiori/",
                "doc/shiori/fragments/",
                "doc/shiori/fragments/A_B.md",
            ]
            .map(String::from),
        ),
    };

    assert_eq!(check_all(&tree, SAMPLE_GUIDE, false), BTreeSet::new());
    assert_eq!(
        check_all(&tree, SAMPLE_GUIDE, true),
        BTreeSet::from([Finding::DocsDirExists])
    );

    // 根の docs の文は最後の検索の語に当たらない形で書く。
    let line = Finding::DocsDirExists.to_string();
    assert!(
        line.contains("根の docs フォルダ") && !line.contains(concat!("docs", "/")), // 語を分けて、この行も検索に当てない
        "{line}"
    );

    // 3 つの読み取りのどれかが 0 件なら、どれかを名指して赤にする。
    let empty = DocTree {
        direct: BTreeSet::new(),
        all: tree.all.clone(),
    };
    assert_eq!(
        check_all(&empty, "", false),
        BTreeSet::from([DIRECT_ENTRIES, DIRECT_TABLE, EXCEPTION_TABLE].map(Finding::EmptyReading))
    );

    // 表に無い直下の実物と、名指すものの無い行（置き字の行を除く）。
    let direct = BTreeSet::from(
        ["README.md", "extra.md", "roadmap-overview-2026-10-16.html"].map(String::from),
    );
    let rows = [
        "README.md",
        "archive/",
        "roadmap-overview-{YYYY-MM-DD}.html",
    ]
    .map(String::from);
    assert_eq!(
        check_direct(&direct, &rows),
        vec![
            Finding::Unlisted("extra.md".into()),
            Finding::MissingRow("archive/".into())
        ]
    );
    assert!(
        Finding::Unlisted("extra.md".into())
            .to_string()
            .contains(UNTRACKED_HINT)
    );
}
