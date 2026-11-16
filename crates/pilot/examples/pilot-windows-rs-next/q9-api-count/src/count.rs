//! SCIP の索引から windows-rs の記号の参照を数える（問い 9）。

use std::collections::{BTreeMap, HashMap};

use scip::types::descriptor::Suffix;
use scip::types::{Index, SymbolRole};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Type,
    Method,
    Function,
    Constant,
    Field,
    Macro,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Part {
    pub krate: String,
    pub test_code: bool,
}

pub type Tally = BTreeMap<Part, HashMap<String, (Kind, u64)>>;

pub struct Sum {
    pub distinct: [u64; 6],
    pub occurrences: [u64; 6],
}

/// 数える包み（rust-analyzer の綴りの `-` は `_` にそろえて比べる）。
const PACKAGES: [&str; 5] = [
    "windows",
    "windows_core",
    "windows_numerics",
    "windows_strings",
    "windows_result",
];

/// 記号の綴りから種類を決める。数えない包み・局所の記号・名前空間などは `None`。
/// 最後の記述子が `#`＝型・`().`＝型の下ならメソッド／でなければ関数・
/// `.`＝型の下なら欄／でなければ定数ほか・`!`＝マクロ。
pub fn classify(symbol: &str) -> Option<Kind> {
    let sym = scip::symbol::parse_symbol(symbol).ok()?;
    let package = sym.package.as_ref()?.name.replace('-', "_");
    if !PACKAGES.contains(&package.as_str()) {
        return None;
    }
    let (last, before) = sym.descriptors.split_last()?;
    let under_type = before
        .iter()
        .any(|d| d.suffix.enum_value_or_default() == Suffix::Type);
    match last.suffix.enum_value_or_default() {
        Suffix::Type => Some(Kind::Type),
        Suffix::Method if under_type => Some(Kind::Method),
        Suffix::Method => Some(Kind::Function),
        Suffix::Term if under_type => Some(Kind::Field),
        Suffix::Term => Some(Kind::Constant),
        Suffix::Macro => Some(Kind::Macro),
        _ => None,
    }
}

/// 試験のコード: 途中のディレクトリが `tests`・`benches`・`examples`・`*_tests`・`tests_*` か、
/// ファイル名が `*_tests.rs`・`*_test.rs`・`test_*.rs`（`test_support.rs` を含む）・`*_test_support.rs`・
/// `tests.rs`・`testkit.rs`。ディレクトリ名の `*_test`（`hit_test/`）は本番。
// shortcut: 本番のファイルの中の `#[cfg(test)]` のモジュールは索引に属性が無いので本番に数える、本線の決まり（試験は隣の `*_tests.rs`）が崩れたら源を読んで分ける
pub fn is_test_path(path: &str) -> bool {
    let parts: Vec<&str> = path.split('/').collect();
    let file = parts.last().copied().unwrap_or("");
    let stem = file.strip_suffix(".rs").unwrap_or(file);
    parts[..parts.len().saturating_sub(1)].iter().any(|d| {
        matches!(*d, "tests" | "benches" | "examples")
            || d.ends_with("_tests")
            || d.starts_with("tests_")
    }) || stem.ends_with("_tests")
        || stem.ends_with("_test")
        || stem.starts_with("test_")
        || stem.ends_with("_test_support")
        || matches!(stem, "tests" | "testkit")
}

/// 試験の道具: 本線の manifest の `publish = false` の注が「テストだけが使う道具」（`*-kit`）・
/// 「試験用の DLL」（`*testdll*`）・「開発の道具」「調査の道具」の物。ほかは出荷する。
pub fn is_test_tool(krate: &str) -> bool {
    krate.ends_with("-kit")
        || krate.contains("testdll")
        || matches!(
            krate,
            "areka-impl-watch" | "surface-compare" | "ukadoc-survey"
        )
}

/// `crates/<名前>/…` の文書の参照を、クレートと本番／試験ごとに記号で集める（`crates/pilot` は除く）。
pub fn tally(index: &Index) -> Tally {
    let mut t = Tally::new();
    for doc in &index.documents {
        let mut parts = doc.relative_path.split('/');
        let (Some("crates"), Some(krate)) = (parts.next(), parts.next()) else {
            continue;
        };
        if krate == "pilot" {
            continue;
        }
        let part = Part {
            krate: krate.to_string(),
            test_code: is_test_path(&doc.relative_path),
        };
        for occ in &doc.occurrences {
            if occ.symbol_roles & SymbolRole::Definition as i32 != 0 {
                continue;
            }
            if let Some(kind) = classify(&occ.symbol) {
                let entry = t.entry(part.clone()).or_default();
                entry.entry(occ.symbol.clone()).or_insert((kind, 0)).1 += 1;
            }
        }
    }
    t
}

/// 選んだ部分の和。同じ記号は 1 つと数え、出現は足す。
pub fn sum(t: &Tally, pick: impl Fn(&Part) -> bool) -> Sum {
    let mut seen: HashMap<&str, Kind> = HashMap::new();
    let mut occurrences = [0; 6];
    for (_, symbols) in t.iter().filter(|(p, _)| pick(p)) {
        for (s, (kind, n)) in symbols {
            seen.insert(s, *kind);
            occurrences[*kind as usize] += n;
        }
    }
    let mut distinct = [0; 6];
    for kind in seen.values() {
        distinct[*kind as usize] += 1;
    }
    Sum {
        distinct,
        occurrences,
    }
}

fn row(out: &mut String, krate: &str, mark: &str, code: &str, s: &Sum) {
    let cell = |k: Kind| format!("{} ({})", s.distinct[k as usize], s.occurrences[k as usize]);
    let main = [Kind::Type, Kind::Method, Kind::Function, Kind::Constant];
    let total: u64 = main.iter().map(|k| s.distinct[*k as usize]).sum();
    let total_occ: u64 = main.iter().map(|k| s.occurrences[*k as usize]).sum();
    out.push_str(&format!(
        "| {krate} | {mark} | {code} | {} | {} | {} | {} | {} ({total_occ}) | {} | {} |\n",
        cell(Kind::Type),
        cell(Kind::Method),
        cell(Kind::Function),
        cell(Kind::Constant),
        total,
        cell(Kind::Field),
        cell(Kind::Macro),
    ));
}

/// クレート × 種類の表と、印ごと・本番／試験ごとの合計の行を Markdown で返す。
pub fn render(t: &Tally) -> String {
    let mark = |tool: bool| {
        if tool {
            "試験の道具"
        } else {
            "出荷する"
        }
    };
    let code = |test: bool| if test { "試験" } else { "本番" };
    let mut out = String::new();
    out.push_str("各欄は「記号の数（出現の回数）」。計は型・メソッド・関数・定数ほかの和（型の下の欄とマクロは別）。\n\n");
    out.push_str(
        "| クレート | 印 | コード | 型 | メソッド | 関数 | 定数ほか | 計 | 型の下の欄 | マクロ |\n",
    );
    out.push_str("|---|---|---|--:|--:|--:|--:|--:|--:|--:|\n");
    for p in t.keys() {
        let s = sum(t, |q| q == p);
        row(
            &mut out,
            &p.krate,
            mark(is_test_tool(&p.krate)),
            code(p.test_code),
            &s,
        );
    }
    for tool in [false, true] {
        for test in [false, true] {
            let s = sum(t, |p| is_test_tool(&p.krate) == tool && p.test_code == test);
            row(&mut out, "**合計**", mark(tool), code(test), &s);
        }
        let s = sum(t, |p| is_test_tool(&p.krate) == tool);
        row(&mut out, "**合計**", mark(tool), "本番＋試験", &s);
    }
    row(
        &mut out,
        "**合計**",
        "全部",
        "本番＋試験",
        &sum(t, |_| true),
    );
    out
}

#[cfg(test)]
#[path = "count_tests.rs"]
mod tests;
