//! 3 文書・全体報告・spec ディレクトリの一覧を読む道具、本文の写しを 1 か所だけ壊す
//! 道具、母数の下限を集める器（設計「入口 / `tests/consistency`」→「`documents.rs`」）。
//!
//! **テストの本体はここに 1 つも置かない**（`structure.md:129` の流儀・`perturb.rs` と同じ）。
//! 道具の較正と母数の下限は兄弟の `documents_non_vacuity.rs` にあり、判定 6 種は
//! `documents_checks.rs`（⑴ ⑵ ⑸ ⑹）と `linkage_checks.rs`（⑶ ⑷）が持つ。

use std::collections::BTreeSet;
use std::path::Path;

use ukadoc_survey::documents::parse::{
    backticked_ids, quoted_ids, read_briefing, read_linkage, read_roadmap_draft,
};
use ukadoc_survey::documents::{Briefing, Linkage, RoadmapDraft};
use ukadoc_survey::io::{files, paths};

/// 本 spec 自身の spec ディレクトリの**名前**（要件 12.8）。
pub(super) const OWN_SPEC_DIR: &str = "areka-P0-ukadoc-coverage-roadmap";

/// spec を置くディレクトリ（ワークスペース根からの相対）。
const SPECS_DIR: &str = ".kiro/specs";

/// 完了した spec を集めるディレクトリの名前（`.kiro/specs/` の直下）。
const COMPLETED_DIR: &str = "completed";

/// spec ディレクトリを spec だと見分ける目印（要件 10.2 の「brief 済み」）。
const BRIEF_FILE: &str = "brief.md";

/// repo から読み込んだ 3 文書・全体報告・spec ディレクトリの一覧。
pub(super) struct Documents {
    /// `doc/ukadoc-coverage/linkage.md` の本文（復帰文字を落としたもの）。
    pub(super) linkage_text: String,
    /// `doc/ukadoc-coverage/briefing.md` の本文。
    pub(super) briefing_text: String,
    /// `doc/ukadoc-coverage/roadmap-draft.md` の本文。
    pub(super) roadmap_text: String,
    /// `doc/ukadoc-coverage/report/summary.md` の本文（判定 ⑹ の突き合わせ相手）。
    pub(super) summary_text: String,
    /// `linkage.md` の骨組み。
    pub(super) linkage: Linkage,
    /// `briefing.md` の骨組み。
    pub(super) briefing: Briefing,
    /// `roadmap-draft.md` の骨組み。
    pub(super) roadmap: RoadmapDraft,
    /// `.kiro/specs/` の直下でディレクトリ名が [`COMPLETED_DIR`] でも [`OWN_SPEC_DIR`]
    /// でもなく [`BRIEF_FILE`] を持つディレクトリの名前。
    pub(super) spec_dirs: BTreeSet<String>,
    /// `.kiro/specs/completed/` の直下のディレクトリの名前。
    pub(super) completed_specs: BTreeSet<String>,
}

impl Documents {
    /// repo の実データを読む。
    pub(super) fn load() -> Self {
        let linkage_text = read_text(&paths::linkage_path());
        let briefing_text = read_text(&paths::briefing_path());
        let roadmap_text = read_text(&paths::roadmap_draft_path());
        let summary_text = read_text(&paths::summary_report_path());

        let linkage = read_linkage(&linkage_text)
            .unwrap_or_else(|err| panic!("linkage.md の骨組みを読めない: {err}"));
        let briefing = read_briefing(&briefing_text)
            .unwrap_or_else(|err| panic!("briefing.md の骨組みを読めない: {err}"));
        let roadmap = read_roadmap_draft(&roadmap_text)
            .unwrap_or_else(|err| panic!("roadmap-draft.md の骨組みを読めない: {err}"));

        let specs_root = paths::workspace_root().join(SPECS_DIR);
        let spec_dirs = child_dirs(&specs_root)
            .into_iter()
            .filter(|name| name != COMPLETED_DIR && name != OWN_SPEC_DIR)
            .filter(|name| specs_root.join(name).join(BRIEF_FILE).is_file())
            .collect();
        let completed_specs = child_dirs(&specs_root.join(COMPLETED_DIR))
            .into_iter()
            .collect();

        Self {
            linkage_text,
            briefing_text,
            roadmap_text,
            summary_text,
            linkage,
            briefing,
            roadmap,
            spec_dirs,
            completed_specs,
        }
    }
}

/// 1 本読む。読めなければ探した絶対パスと理由を添えて止まる（要件 6.12）。
fn read_text(path: &Path) -> String {
    files::read_normalized(path).unwrap_or_else(|err| panic!("{err}"))
}

/// そのディレクトリの直下にあるディレクトリの名前。
fn child_dirs(dir: &Path) -> Vec<String> {
    let entries =
        std::fs::read_dir(dir).unwrap_or_else(|err| panic!("{} を開けない: {err}", dir.display()));
    let mut names = Vec::new();
    for entry in entries {
        let entry = entry.unwrap_or_else(|err| panic!("{} の直下を読めない: {err}", dir.display()));
        if !entry.path().is_dir() {
            continue;
        }
        match entry.file_name().into_string() {
            Ok(name) => names.push(name),
            Err(raw) => panic!("{} の直下に UTF-8 でない名前がある: {raw:?}", dir.display()),
        }
    }
    names
}

/// 本文が引用した項目 id（引用符と逆引用符の両方・重複は畳む）。
pub(super) fn cited_ids(markdown: &str) -> BTreeSet<String> {
    quoted_ids(markdown)
        .into_iter()
        .chain(backticked_ids(markdown))
        .collect()
}

// ---------------------------------------------------------------------------
// 写しを 1 か所だけ壊す道具
// ---------------------------------------------------------------------------
//
// 手を入れるのは引数で渡された本文の**写し**（`String`）だけで、repo のファイルには
// 1 バイトも触れない（`perturb.rs` と同じ流儀）。狙った場所が見つからなければ黙って
// 素通りせず止まる——空振りした摂動は「壊しても赤くならない」を「壊れていない」と
// 読み違えさせる。

/// id の綴りを 1 文字だけ変えた綴り（末尾の 1 文字を差し替える）。
///
/// 末尾は出現番号の数字なので、数字にならない `X` へ差し替えればカタログに実在しない
/// 綴りになり、なお `ukadoc:` 始まりのままなので判定 ⑴ の網には掛かる。
pub(super) fn twisted_id(id: &str) -> String {
    let mut chars: Vec<char> = id.chars().collect();
    match chars.last_mut() {
        Some(last) => *last = if *last == 'X' { 'Y' } else { 'X' },
        None => panic!("空の id は 1 文字変えられない"),
    }
    chars.into_iter().collect()
}

/// 本文に現れる id の綴りを、すべて 1 文字変えた綴りへ差し替える。
pub(super) fn twist_id(text: &str, id: &str) -> String {
    let twisted = twisted_id(id);
    assert!(
        text.contains(id),
        "写しに id {id} が無いので 1 文字変えられない（摂動が空振りする）"
    );
    assert!(
        !text.contains(&twisted),
        "1 文字変えた綴り {twisted} が既に写しにある（摂動が見分けられない）"
    );
    text.replace(id, &twisted)
}

/// 配列から id を 1 つだけ抜く（前後の区切りの読点も一緒に落とす）。
pub(super) fn drop_member(text: &str, id: &str) -> String {
    let needle = format!("\"{id}\"");
    let at = only_occurrence(text, &needle, "抜く id");
    let mut end = at + needle.len();
    let mut start = at;
    // 後ろに読点があれば、それと続く空白（改行を含む）まで一緒に落とす。無ければ
    // 前の空白と読点を落とす——配列の最後の要素はこちらになる。
    let tail = &text[end..];
    match tail.strip_prefix(',') {
        Some(rest) => {
            end += 1 + (rest.len() - rest.trim_start().len());
            // 1 行に 1 つずつ書かれた配列では、後ろの改行まで食べた分だけ前の字下げが
            // 余る。id が行頭から始まっているならその字下げも一緒に落とす。
            let head = &text[..start];
            if let Some(newline) = head.rfind('\n')
                && head[newline + 1..].trim().is_empty()
            {
                start = newline + 1;
            }
        }
        None => {
            let head = text[..start].trim_end();
            match head.strip_suffix(',') {
                Some(before) => start = before.len(),
                None => start = head.len(),
            }
        }
    }
    format!("{}{}", &text[..start], &text[end..])
}

/// `鍵 = 数` の数を 1 つずらす（1 増やす）。
pub(super) fn shift_count(text: &str, key: &str) -> String {
    let needle = format!("{key} = ");
    let at = only_occurrence(text, &needle, "ずらす件数の鍵");
    let rest = &text[at + needle.len()..];
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    assert!(
        !digits.is_empty(),
        "{key} の右辺が数でない（摂動が空振りする）"
    );
    let shifted = digits
        .parse::<usize>()
        .unwrap_or_else(|err| panic!("{key} の右辺 {digits} を数として読めない: {err}"))
        + 1;
    format!("{}{needle}{shifted}{}", &text[..at], &rest[digits.len()..])
}

/// 束名を 1 つ消す（その名前が現れる行を丸ごと落とす）。
pub(super) fn drop_bundle_name(text: &str, name: &str) -> String {
    let needle = format!("\"{name}\"");
    let mut kept = Vec::new();
    let mut dropped = 0;
    for line in text.lines() {
        if line.contains(&needle) {
            dropped += 1;
            continue;
        }
        kept.push(line);
    }
    assert_eq!(
        dropped, 1,
        "束名 {name} を書いた行が写しにちょうど 1 行ない（{dropped} 行あった）"
    );
    kept.join("\n")
}

/// 本文にちょうど 1 度だけ現れる綴りの位置。
fn only_occurrence(text: &str, needle: &str, role: &str) -> usize {
    let hits: Vec<usize> = text.match_indices(needle).map(|(at, _)| at).collect();
    assert_eq!(
        hits.len(),
        1,
        "{role} {needle} が写しにちょうど 1 度現れない（{} 度）",
        hits.len()
    );
    hits[0]
}

// ---------------------------------------------------------------------------
// 母数の下限を集める器
// ---------------------------------------------------------------------------

/// 「この検査は何件を相手にしているか」の下限を集める器（要件 11.4）。
///
/// 母数 0 の緑は恒真である。下限を 1 か所へ集めておくと、下回った行を**まとめて**
/// 名指せる——1 つ目で止まると、残りが 0 件に落ちていることに気づけない。
///
/// 下限そのものが 0 の行も違反として扱う。`0 以上`は何も主張していないので、
/// 器に登録できてしまうと「下限を置いた」という記録だけが残る。
#[derive(Default)]
pub(super) struct Floors {
    rows: Vec<Floor>,
}

/// 下限 1 つ分。
struct Floor {
    /// 何を数えたか（失敗の本文に出る）。
    name: &'static str,
    /// 数え直した実数。
    actual: usize,
    /// 下限。1 以上でなければならない。
    floor: usize,
}

impl Floors {
    /// 空の器。
    pub(super) fn new() -> Self {
        Self::default()
    }

    /// 下限を 1 つ登録する。
    pub(super) fn at_least(
        &mut self,
        name: &'static str,
        actual: usize,
        floor: usize,
    ) -> &mut Self {
        self.rows.push(Floor {
            name,
            actual,
            floor,
        });
        self
    }

    /// 満たしていない行の本文（満たしていれば空）。
    pub(super) fn short(&self) -> Vec<String> {
        let mut short = Vec::new();
        for row in &self.rows {
            let Floor {
                name,
                actual,
                floor,
            } = row;
            if *floor == 0 {
                short.push(format!(
                    "{name}: 下限 0 は主張にならない（母数 0 の緑は恒真）"
                ));
                continue;
            }
            if actual < floor {
                short.push(format!("{name}: {actual} 件しかない（下限 {floor}）"));
            }
        }
        short
    }

    /// 満たしていない行が 1 つでもあれば、その全部を名指して止まる。
    pub(super) fn assert_met(&self) {
        let short = self.short();
        assert!(
            short.is_empty(),
            "母数が下限を下回っている:\n{}",
            short.join("\n")
        );
    }
}
