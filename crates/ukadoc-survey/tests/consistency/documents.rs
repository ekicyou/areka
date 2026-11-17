//! 3 文書と spec ディレクトリの一覧を読む道具、判定が共有する所見の見方と
//! 綴りの定数、本文の写しの狙った綴り・1 つの欄だけを壊す道具、母数の下限を集める器（設計「入口 /
//! `tests/consistency`」→「`documents.rs`」）。
//!
//! **テストの本体はここに 1 つも置かない**（`structure.md` の流儀・`perturb.rs` と同じ）。
//! 母数の下限は兄弟の `documents_non_vacuity.rs`、器と壊す道具の較正は
//! `documents_tools.rs` にあり、判定 6 種は `documents_checks.rs`（⑴ ⑵ ⑹）と
//! `linkage_checks.rs`（⑶）と `briefing_checks.rs`（⑷）と `spec_checks.rs`（⑸）が持つ。

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::LazyLock;

use ukadoc_survey::documents::parse::{
    backticked_ids, quoted_ids, read_briefing, read_linkage, read_roadmap_draft,
};
use ukadoc_survey::documents::{Briefing, Linkage, RoadmapDraft};
use ukadoc_survey::io::paths;

use super::read_text;

/// 本 spec 自身の spec ディレクトリの**名前**。
pub(super) const OWN_SPEC_DIR: &str = "areka-P0-ukadoc-coverage-roadmap";

/// spec を置くディレクトリ（ワークスペース根からの相対）。
const SPECS_DIR: &str = ".kiro/specs";

/// 完了した spec を集めるディレクトリの名前（`.kiro/specs/` の直下）。
pub(super) const COMPLETED_DIR: &str = "completed";

/// spec ディレクトリを spec だと見分ける目印（要件 10.2 の「brief 済み」）。
const BRIEF_FILE: &str = "brief.md";

/// 3 文書のワークスペース根からの相対パス（失敗の本文はこの綴りで名指す）。
pub(super) const LINKAGE_MD: &str = "doc/ukadoc-coverage/linkage.md";
pub(super) const BRIEFING_MD: &str = "doc/ukadoc-coverage/briefing.md";
pub(super) const ROADMAP_MD: &str = "doc/ukadoc-coverage/roadmap-draft.md";

/// 既存の語彙台帳（要件 9.2 の「置き換えない」相手）。走査が届いていることの錨にも、
/// 語彙表経路の較正の相手にも使う。
pub(super) const SHIORI_RESOURCE_RS: &str = "crates/areka-sylphya/src/vocab/shiori_resource.rs";

/// repo から読み込んだ 3 文書と spec ディレクトリの一覧。
pub(super) struct Documents {
    /// `doc/ukadoc-coverage/linkage.md` の本文（復帰文字を落としたもの）。
    pub(super) linkage_text: String,
    /// `doc/ukadoc-coverage/briefing.md` の本文。
    pub(super) briefing_text: String,
    /// `doc/ukadoc-coverage/roadmap-draft.md` の本文。
    pub(super) roadmap_text: String,
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
    /// repo の 3 文書と spec の一覧（1 回の走行で 1 度だけ読み、全部のテストが
    /// 同じものを借りる）。読めなければ**どの呼び出しも**同じ文で止まる
    /// （[`super::RepoData::load`] と同じ形）。
    pub(super) fn load() -> &'static Self {
        static DOCUMENTS: LazyLock<Result<Documents, String>> = LazyLock::new(Documents::read);
        DOCUMENTS.as_ref().unwrap_or_else(|err| panic!("{err}"))
    }

    /// repo の実データを読む。呼ぶのは [`Documents::load`] の 1 か所だけである。
    fn read() -> Result<Self, String> {
        let linkage_text = read_text(&paths::linkage_path())?;
        let briefing_text = read_text(&paths::briefing_path())?;
        let roadmap_text = read_text(&paths::roadmap_draft_path())?;

        let linkage = read_linkage(&linkage_text)
            .map_err(|err| format!("linkage.md の骨組みを読めない: {err}"))?;
        let briefing = read_briefing(&briefing_text)
            .map_err(|err| format!("briefing.md の骨組みを読めない: {err}"))?;
        let roadmap = read_roadmap_draft(&roadmap_text)
            .map_err(|err| format!("roadmap-draft.md の骨組みを読めない: {err}"))?;

        // 直下を先・`completed/` を後に読む。移しは「直下→`completed/`」の 1 方向なので、
        // 2 つの読みのどの時点で移っても、その spec は少なくとも一方に見える。
        let specs_root = paths::workspace_root().join(SPECS_DIR);
        let spec_dirs = spec_dir_names(child_dirs(&specs_root)?.into_iter().map(|name| {
            let has_brief = specs_root.join(&name).join(BRIEF_FILE).is_file();
            (name, has_brief)
        }));
        let completed_specs = child_dirs(&specs_root.join(COMPLETED_DIR))?
            .into_iter()
            .collect();

        Ok(Self {
            linkage_text,
            briefing_text,
            roadmap_text,
            linkage,
            briefing,
            roadmap,
            spec_dirs,
            completed_specs,
        })
    }
}

/// 直下のフォルダの名前と「brief.md があるか」の組から、spec の名前だけを残す。
///
/// `completed`・本 spec・brief の無い名前は落とす。brief の無いフォルダは作りかけの
/// spec なので、赤にせず一覧から外すだけにする。
pub(super) fn spec_dir_names(
    entries: impl IntoIterator<Item = (String, bool)>,
) -> BTreeSet<String> {
    entries
        .into_iter()
        .filter(|(name, has_brief)| *has_brief && name != COMPLETED_DIR && name != OWN_SPEC_DIR)
        .map(|(name, _)| name)
        .collect()
}

/// そのディレクトリの直下にあるディレクトリの名前。
fn child_dirs(dir: &Path) -> Result<Vec<String>, String> {
    let entries =
        std::fs::read_dir(dir).map_err(|err| format!("{} を開けない: {err}", dir.display()))?;
    let mut names = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|err| format!("{} の直下を読めない: {err}", dir.display()))?;
        if !entry.path().is_dir() {
            continue;
        }
        match entry.file_name().into_string() {
            Ok(name) => names.push(name),
            Err(raw) => {
                return Err(format!(
                    "{} の直下に UTF-8 でない名前がある: {raw:?}",
                    dir.display()
                ));
            }
        }
    }
    Ok(names)
}

/// 本文が引用した項目 id（引用符と逆引用符の両方・重複は畳む）。
pub(super) fn cited_ids(markdown: &str) -> BTreeSet<String> {
    quoted_ids(markdown)
        .into_iter()
        .chain(backticked_ids(markdown))
        .collect()
}

// ---------------------------------------------------------------------------
// 判定の所見の見方（3 文書の判定がどれも同じ文で失敗を出す）
// ---------------------------------------------------------------------------

/// 所見が 1 つでもあれば、その全部を名指して止まる。
pub(super) fn assert_no_findings(judgement: &str, findings: &[String]) {
    assert!(
        findings.is_empty(),
        "{judgement}（{} 件）:\n{}",
        findings.len(),
        findings.join("\n")
    );
}

/// 摂動の後の所見がちょうど 1 件で、その本文が指定の綴りを全部含むこと。
pub(super) fn assert_single_finding(findings: &[String], what: &str, must_name: &[&str]) {
    assert_eq!(
        findings.len(),
        1,
        "{what} で赤が 1 件でない:\n{}",
        findings.join("\n")
    );
    for needle in must_name {
        assert!(
            findings[0].contains(needle),
            "失敗の本文が {needle} を名指していない: {}",
            findings[0]
        );
    }
}

// ---------------------------------------------------------------------------
// 写しの狙った綴り・1 つの欄だけを壊す道具
// ---------------------------------------------------------------------------
//
// 手を入れるのは引数で渡された本文の**写し**（`String`）だけで、repo のファイルには
// 1 バイトも触れない（`perturb.rs` と同じ流儀）。狙った場所が見つからなければ黙って
// 素通りせず止まる——空振りした摂動は「壊しても赤くならない」を「壊れていない」と
// 読み違えさせる。

/// id の綴りを 1 文字だけ変えた綴り（末尾の 1 文字を差し替える）。
///
/// 数字の末尾は 0 へ（もともと 0 なら 9 へ）、数字でない末尾（ページ単位の id）は
/// `X` へ（もともと `X` なら `Y` へ）差し替える。どちらも id の形のままなので判定 ⑴ の
/// 網に掛かる。差し替えた綴りが実在する id に当たったときは、呼び手の「赤がちょうど
/// 1 件」の assert が止める。
pub(super) fn twisted_id(id: &str) -> String {
    let mut chars: Vec<char> = id.chars().collect();
    match chars.last_mut() {
        Some(last) => {
            *last = match *last {
                '0' => '9',
                '1'..='9' => '0',
                'X' => 'Y',
                _ => 'X',
            }
        }
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

/// 囲みの中の `鍵 = 数` の数を 1 つずらす（1 増やす）。
///
/// # 錨は表の中だけに置く
///
/// 錨を本文全体に置くと、同じ綴りが**別の表**や**地の文**に現れただけで摂動が
/// 空振りして止まる。実データではどちらも起きる——`briefing.md` は設計 D-2 により
/// `[stage.A]`〜`[stage.E]` の 5 つが `bundles`・`singles`・`items` を 1 つずつ持ち、
/// `linkage.md` の「数の突き合わせ」は恒等式 `target = from_machine + by_hand +
/// singles` を地の文に書く。だから `table`（`[tally]`・`[stage.B]` のような表の見出しの
/// 行そのもの）で場所を絞り、その表の中でだけ鍵を探す。
///
/// 表の範囲は見出しの次の行から、次に行頭が `[` で始まる行の手前まで（見つからなければ
/// 本文の終わりまで）。TOML の表は行頭の `[` で始まるので、これで隣の表へはみ出さない。
/// 囲みを閉じる ` ``` ` の行はこの範囲に入るが、そこに `鍵 = ` は現れない。
pub(super) fn shift_count(text: &str, table: &str, key: &str) -> String {
    let table_line = format!("{table}\n");
    let table_at = only_occurrence(text, &table_line, "ずらす件数の表") + table_line.len();
    let section_len = text[table_at..]
        .match_indices("\n[")
        .map(|(at, _)| at + 1)
        .next()
        .unwrap_or(text.len() - table_at);
    let section = &text[table_at..table_at + section_len];

    let needle = format!("{key} = ");
    let at = table_at + only_occurrence(section, &needle, "ずらす件数の鍵");
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

/// 表の行の数を 1 つずらす（1 増やす）。
///
/// 狙うのは `| ラベル | 数 |` の形の行で、Markdown の表を持つ本文（全体報告）用である
/// ——[`shift_count`] は TOML の `鍵 = 数` を狙うので、こちらには使えない。
///
/// 錨は**行頭が `| ラベル |` で始まる行**にする。同じ語が別の表の見出しの途中の桁に
/// 現れても（全体報告では「縮退」が状態の分布の行と、テーマ別の状態分布の見出しの
/// 4 桁目の両方に現れる）、行頭で絞れば取り違えない。狙った行がちょうど 1 行でなければ
/// 止まる——空振りした摂動は「壊しても赤くならない」を「壊れていない」と読み違えさせる。
pub(super) fn shift_row_count(text: &str, label: &str) -> String {
    let head = format!("| {label} |");
    let hits: Vec<(usize, &str)> = line_offsets(text)
        .into_iter()
        .filter(|(_, line)| line.starts_with(&head))
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "行頭が {head} の行が写しにちょうど 1 行ない（{} 行あった。摂動が空振りする）",
        hits.len()
    );
    let (offset, row) = hits[0];

    // 右端の桁から数を 1 つ拾って 1 増やす（表の数は右詰めの最終桁にある）。
    let at = row
        .rfind(|c: char| c.is_ascii_digit())
        .unwrap_or_else(|| panic!("{row} に数が無い（摂動が空振りする）"));
    let start = row[..=at]
        .rfind(|c: char| !c.is_ascii_digit())
        .map_or(0, |before| before + 1);
    let digits = &row[start..=at];
    let shifted = digits
        .parse::<usize>()
        .unwrap_or_else(|err| panic!("{digits} を数として読めない: {err}"))
        + 1;
    format!(
        "{}{shifted}{}",
        &text[..offset + start],
        &text[offset + at + 1..]
    )
}

/// 各行の開始位置と本文（改行は含まない）。
fn line_offsets(text: &str) -> Vec<(usize, &str)> {
    let mut rows = Vec::new();
    let mut at = 0;
    for line in text.split('\n') {
        rows.push((at, line));
        at += line.len() + 1;
    }
    rows
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
