//! 3 文書の本文 → 骨組み（欄と語彙の検証つき）。
//!
//! ここは純粋層で、**ファイルには触らない**——本文の文字列だけを受け取る（設計
//! 「境界」）。読み取りは `toml` に任せ、表を手で辿る（既存の台帳の読み手と同じ流儀。
//! `serde` の派生は使わない・`Cargo.toml` の注記）。
//!
//! # 囲みを連結して 1 度だけ読む
//!
//! 1 文書の ```toml の囲みは**改行で連結して 1 度だけ** [`toml::Table`] にする。
//! そうすると ⑴ `[[rank]]` のような配列表を段階ごとに別の囲みへ分けて書けて、
//! ⑵ 同じ表の鍵が 2 度現れれば `toml` が重複として落とす（設計 `documents::parse`）。
//! 囲みの外の地の文は読まない——地の文には反例や書き方の説明が現れる。
//!
//! # 繕わない・行番号は添えない
//!
//! 3 文書は人が手で書くので、形が違えば黙って直さずに落とす。落とす本文には
//! **文書名と表の鍵**（束名・`[[rank]]` の段階と順位・spec 名）を添え、**行番号は
//! 添えない**（設計 Error Handling）——囲みを連結した本文の行番号は、書き手が見て
//! いる Markdown の行と一致しない。
//!
//! # 3 つの口
//!
//! 判定 ⑴ が拾う id の口はここに 3 つ置く——囲みの中の引用符付き（[`toml_blocks`]
//! ＋ [`quoted_ids`] の 2 関数）・地の文の逆引用符付き（[`backticked_ids`]）・どちらにも
//! 囲まれていない裸の語（[`bare_id_tokens`]・0 件であるべき）。
//! `tests/consistency/examples.rs` も囲みの中の口はここを呼ぶ。
//!
//! 「どこからどこまでが囲みか」の見分けは `segments` 1 本で、TOML の口と地の文の
//! 口が同じ見分け方をする。

use std::collections::{BTreeMap, BTreeSet};

use super::fields::{
    BRIEFING_FILE, LINKAGE_FILE, ROADMAP_FILE, array_of_tables, as_table, bad_vocabulary,
    bool_field, document_file, domain_array_field, id_array_field, malformed, optional_usize_field,
    read_blocks, reject_present, reject_unknown_keys, required_table, string_array_field,
    string_field, theme_array_field, usize_field,
};
use super::{
    After, Barrier, Breakage, Briefing, Briefs, BundleRef, Linkage, NamedBundle, OverrideKind,
    OwnerCompleted, PriorityBlank, RankOverride, RankRow, RankTarget, Reserved, RoadmapDraft,
    SpecRow, Stage, StageCount, Tally,
};
use crate::error::SurveyError;
use crate::model::{Domain, EntryId, ID_MARK, PageName};

/// 囲みの区切り。開く行はこれで始まり（後ろに言語名）、閉じる行はこれちょうど。
const FENCE: &str = "```";
/// TOML の囲みの言語名。
const TOML_INFO: &str = "toml";

/// 囲みを連結した本文の根。失敗に添える場所として使う。
const ROOT_PLACE: &str = "囲みの根";

// ---------------------------------------------------------------------------
// 3 つの口
// ---------------------------------------------------------------------------

/// 本文の 1 かたまり——囲みの外の 1 行か、囲み 1 つの中身。
enum Segment<'a> {
    /// 囲みの外の行。
    Prose(&'a str),
    /// 囲みの中の行（区切りの行は含まない）。`toml` は言語名が `toml` のとき真。
    Block { toml: bool, lines: Vec<&'a str> },
}

/// 本文を囲みの外の行と囲みに分ける。TOML の口と地の文の口はこれだけを使う。
///
/// 開くのは行頭の空白を削った行が ` ``` ` で始まるとき（字下げした囲みも囲み）、
/// 閉じるのは前後の空白を削った行がちょうど ` ``` ` のとき——囲みの中の
/// ` ```toml ` の行は囲みの中身で、開きも閉じもしない。閉じ忘れた囲みは本文の
/// 終わりまでを 1 つの囲みとする——取りこぼして黙って緑になるより、中身を検査に
/// 掛けたほうが安全側である。
fn segments(markdown: &str) -> Vec<Segment<'_>> {
    let mut out = Vec::new();
    let mut open: Option<(bool, Vec<&str>)> = None;
    for line in markdown.lines() {
        if let Some((toml, lines)) = open.as_mut() {
            if line.trim() == FENCE {
                out.push(Segment::Block {
                    toml: *toml,
                    lines: std::mem::take(lines),
                });
                open = None;
            } else {
                lines.push(line);
            }
            continue;
        }
        match line.trim_start().strip_prefix(FENCE) {
            Some(info) => open = Some((info.trim() == TOML_INFO, Vec::new())),
            None => out.push(Segment::Prose(line)),
        }
    }
    if let Some((toml, lines)) = open {
        out.push(Segment::Block { toml, lines });
    }
    out
}

/// TOML の囲みの中身だけを、現れた順に返す（見分け方は `segments`）。
pub fn toml_blocks(markdown: &str) -> Vec<String> {
    segments(markdown)
        .into_iter()
        .filter_map(|segment| match segment {
            Segment::Block { toml: true, lines } => Some(lines.join("\n")),
            Segment::Block { .. } | Segment::Prose(_) => None,
        })
        .collect()
}

/// 本文に現れる `"ukadoc:…"` の綴りを、現れた順に返す（重複は落とさない）。
///
/// 二重引用符で囲まれた `ukadoc:` 始まりの文字列だけを拾う。台帳でも 3 文書でも
/// id は必ず引用符の中に書かれるので、これで表の鍵も配列の要素も同じ規則で拾える。
pub fn quoted_ids(text: &str) -> Vec<String> {
    let mut ids = Vec::new();
    let needle = format!("\"{ID_MARK}");
    let mut rest = text;
    while let Some(start) = rest.find(&needle) {
        let after_quote = &rest[start + 1..];
        match after_quote.find('"') {
            Some(end) => {
                ids.push(after_quote[..end].to_owned());
                rest = &after_quote[end + 1..];
            }
            // 閉じ引用符が無い＝壊れた本文。ここで打ち切る。
            None => break,
        }
    }
    ids
}

/// 地の文で逆引用符に囲まれた `` `ukadoc:…` `` を、現れた順に返す。
///
/// 囲みの中は見ない——あちらは [`quoted_ids`] の持ち場である。
pub fn backticked_ids(markdown: &str) -> Vec<String> {
    let mut ids = Vec::new();
    for line in prose_lines(markdown) {
        for (index, segment) in line.split('`').enumerate() {
            if index % 2 == 1 && segment.starts_with(ID_MARK) {
                ids.push(segment.to_owned());
            }
        }
    }
    ids
}

/// 地の文で引用符にも逆引用符にも囲まれていない `ukadoc:` 始まりの語を返す。
///
/// **0 件であるべき**——3 文書の書き方の規律は「id は必ず逆引用符か引用符で囲む」で、
/// 裸で書かれた id は判定 ⑴ の網から漏れる（設計 D-1）。ここが 0 件でないことを
/// 判定が赤にする。
pub fn bare_id_tokens(markdown: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    for line in prose_lines(markdown) {
        for (backtick, outside_backtick) in line.split('`').enumerate() {
            if backtick % 2 == 1 {
                continue;
            }
            for (quote, outside_quote) in outside_backtick.split('"').enumerate() {
                if quote % 2 == 1 {
                    continue;
                }
                collect_bare_tokens(outside_quote, &mut tokens);
            }
        }
    }
    tokens
}

/// 囲み（どの言語でも）の外の行だけを返す（見分け方は `segments`）。
fn prose_lines(markdown: &str) -> Vec<&str> {
    segments(markdown)
        .into_iter()
        .filter_map(|segment| match segment {
            Segment::Prose(line) => Some(line),
            Segment::Block { .. } => None,
        })
        .collect()
}

/// id の綴りに現れてよい文字（`balloon.scope(ID).width` のような括弧付きも含む）。
fn is_id_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, ':' | '_' | '-' | '.' | '(' | ')')
}

/// 1 つの断片から裸の語を拾う。語の**途中**で始まる綴り（`xukadoc:…`）は拾わない
/// ——語境界を見ないと、別の語の一部を id として名指して判定が空振りする。
fn collect_bare_tokens(text: &str, out: &mut Vec<String>) {
    let mut consumed = 0usize;
    while let Some(offset) = text[consumed..].find(ID_MARK) {
        let start = consumed + offset;
        let tail = &text[start..];
        let end = tail.find(|c| !is_id_char(c)).unwrap_or(tail.len());
        let preceded_by_word = text[..start].chars().next_back().is_some_and(is_id_char);
        if !preceded_by_word {
            out.push(tail[..end].to_owned());
        }
        consumed = start + end;
    }
}

// ---------------------------------------------------------------------------
// linkage.md
// ---------------------------------------------------------------------------

/// `linkage.md` の根に置いてよい表（設計 D-2）。
const LINKAGE_TABLES: [&str; 2] = ["bundle", "tally"];

/// 束の表に置いてよい欄（設計 D-2 の 8 項目＋単独項目の 2 欄）。
const BUNDLE_FIELDS: [&str; 9] = [
    "single",
    "machine",
    "members",
    "hand",
    "domains",
    "foundation",
    "breakage",
    "themes",
    "reason",
];

/// `[tally]` に置いてよい欄。
const TALLY_FIELDS: [&str; 7] = [
    "target",
    "from_machine",
    "by_hand",
    "singles",
    "alias_excluded",
    "not_applicable_excluded",
    "singles_by_domain",
];

/// `linkage.md` の骨組みを読む（要件 4.1・4.2）。
pub fn read_linkage(markdown: &str) -> Result<Linkage, SurveyError> {
    let file = document_file(LINKAGE_FILE);
    let root = read_blocks(markdown, &file)?;
    reject_unknown_keys(&root, &LINKAGE_TABLES, ROOT_PLACE, &file)?;

    Ok(Linkage {
        bundles: read_bundles(&root, &file)?,
        tally: read_tally(&root, &file)?,
    })
}

/// 束をすべて読む。`[bundle]` が無い文書は束 0 件として扱う（書き始めの器）。
fn read_bundles(
    root: &toml::Table,
    file: &str,
) -> Result<BTreeMap<String, NamedBundle>, SurveyError> {
    let table = match root.get("bundle") {
        None => return Ok(BTreeMap::new()),
        Some(value) => as_table(value, "[bundle]", file)?,
    };

    let mut bundles = BTreeMap::new();
    for (name, value) in table {
        let place = format!("[bundle.\"{name}\"]");
        let item = as_table(value, &place, file)?;
        bundles.insert(name.clone(), read_bundle(name, item, &place, file)?);
    }
    Ok(bundles)
}

/// 束 1 つ分を読む。
///
/// 名前付き束と単独項目は書ける欄が違う（設計 D-2）。単独項目は `reason` を持ち
/// `machine`・`hand`・`foundation` を持たない——「単独項目に落とした」ことの印が
/// 名前付き束に紛れ込まないようにするためである。
fn read_bundle(
    name: &str,
    item: &toml::Table,
    place: &str,
    file: &str,
) -> Result<NamedBundle, SurveyError> {
    reject_unknown_keys(item, &BUNDLE_FIELDS, place, file)?;

    let single = bool_field(item, place, "single", file)?;
    let members = id_array_field(item, place, "members", file)?;
    if members.is_empty() {
        return Err(malformed(file, format!("{place}: 欄 members が空")));
    }
    reject_duplicate_ids(&members, place, "members", file)?;

    let (machine, hand, foundation, reason) = if single {
        reject_present(item, place, file, &["machine", "hand", "foundation"])?;
        if members.len() != 1 {
            return Err(malformed(
                file,
                format!(
                    "{place}: 単独項目の members は id 1 つ（{} 件書かれている）",
                    members.len()
                ),
            ));
        }
        // 順位は単独項目を名前で引くので、名前と中の id は同じ綴りでなければならない。
        if members[0].as_str() != name {
            return Err(malformed(
                file,
                format!(
                    "{place}: 単独項目の名前 {name} と members の {} が食い違う",
                    members[0].as_str()
                ),
            ));
        }
        let reason = string_field(item, place, "reason", file)?;
        if reason.trim().is_empty() {
            return Err(malformed(file, format!("{place}: 欄 reason が空")));
        }
        (Vec::new(), Vec::new(), String::new(), reason)
    } else {
        reject_present(item, place, file, &["reason"])?;
        let machine = id_array_field(item, place, "machine", file)?;
        reject_duplicate_ids(&machine, place, "machine", file)?;
        let hand = id_array_field(item, place, "hand", file)?;
        reject_duplicate_ids(&hand, place, "hand", file)?;
        for id in &hand {
            if !members.contains(id) {
                return Err(malformed(
                    file,
                    format!("{place}: 欄 hand の {} が members に無い", id.as_str()),
                ));
            }
        }
        let foundation = string_field(item, place, "foundation", file)?;
        if foundation.trim().is_empty() {
            return Err(malformed(file, format!("{place}: 欄 foundation が空")));
        }
        (machine, hand, foundation, String::new())
    };

    let raw_breakage = string_field(item, place, "breakage", file)?;
    let breakage = Breakage::parse(&raw_breakage)
        .ok_or_else(|| bad_vocabulary(file, place, "breakage", &raw_breakage))?;

    let domains = domain_array_field(item, place, "domains", file)?;
    reject_repeated(domains.iter().map(Domain::as_key), place, "domains", file)?;
    let themes = theme_array_field(item, place, "themes", file)?;
    reject_repeated(themes.iter().map(String::as_str), place, "themes", file)?;

    Ok(NamedBundle {
        single,
        machine,
        members,
        hand,
        domains,
        foundation,
        breakage,
        themes,
        reason,
    })
}

/// 同じ id を 2 度書いていないことを確かめる。畳むと導出と順位で扱いが割れる。
fn reject_duplicate_ids(
    ids: &[EntryId],
    place: &str,
    key: &str,
    file: &str,
) -> Result<(), SurveyError> {
    reject_repeated(ids.iter().map(EntryId::as_str), place, key, file)
}

/// 同じ綴りを 2 度書いていないことを確かめる（欄の値の重複・配列表の行の重複）。
///
/// 黙って通すと、数える側では 2 件に、引く側では 1 件に扱われて食い違う。
fn reject_repeated<'a>(
    spellings: impl IntoIterator<Item = &'a str>,
    place: &str,
    key: &str,
    file: &str,
) -> Result<(), SurveyError> {
    let mut seen = BTreeSet::new();
    for one in spellings {
        if !seen.insert(one) {
            return Err(malformed(
                file,
                format!("{place}: 欄 {key} に {one} が 2 度ある"),
            ));
        }
    }
    Ok(())
}

/// `[tally]` を読む。ドメイン別内訳は 4 つ揃わなければ落ちる（要件 4.2・11.5）。
fn read_tally(root: &toml::Table, file: &str) -> Result<Tally, SurveyError> {
    let place = "[tally]";
    let table = required_table(root, "tally", place, file)?;
    reject_unknown_keys(table, &TALLY_FIELDS, place, file)?;

    let by_domain_place = "[tally.singles_by_domain]";
    let by_domain = required_table(table, "singles_by_domain", by_domain_place, file)?;
    let keys: Vec<&str> = Domain::ALL.iter().map(Domain::as_key).collect();
    reject_unknown_keys(by_domain, &keys, by_domain_place, file)?;
    let mut singles_by_domain = BTreeMap::new();
    for domain in Domain::ALL {
        let count = usize_field(by_domain, by_domain_place, domain.as_key(), file)?;
        singles_by_domain.insert(domain, count);
    }

    Ok(Tally {
        target: usize_field(table, place, "target", file)?,
        from_machine: usize_field(table, place, "from_machine", file)?,
        by_hand: usize_field(table, place, "by_hand", file)?,
        singles: usize_field(table, place, "singles", file)?,
        alias_excluded: usize_field(table, place, "alias_excluded", file)?,
        not_applicable_excluded: usize_field(table, place, "not_applicable_excluded", file)?,
        singles_by_domain,
    })
}

// ---------------------------------------------------------------------------
// briefing.md
// ---------------------------------------------------------------------------

/// `briefing.md` の根に置いてよい表。
const BRIEFING_TABLES: [&str; 7] = [
    "rank",
    "stage",
    "barrier",
    "after",
    "priority_blank",
    "owner_completed",
    "template",
];

/// `[[rank]]` に置いてよい欄。
const RANK_FIELDS: [&str; 8] = [
    "stage",
    "rank",
    "bundle",
    "singles",
    "assets",
    "shared",
    "override",
    "insufficient",
];

/// `override` に置いてよい欄（`ref` は Rust の予約語なので型の側は `reference`）。
const OVERRIDE_FIELDS: [&str; 2] = ["kind", "ref"];

/// `[stage.X]` に置いてよい欄。
const STAGE_FIELDS: [&str; 3] = ["bundles", "singles", "items"];

/// `[[barrier]]` に置いてよい欄。
const BARRIER_FIELDS: [&str; 7] = [
    "page",
    "implemented",
    "vocabulary_only",
    "degraded",
    "absent",
    "alias",
    "not_applicable",
];

/// `[[owner_completed]]` に置いてよい欄。
const OWNER_COMPLETED_FIELDS: [&str; 2] = ["spec", "items"];

/// `[[template]]` に置いてよい欄。
const TEMPLATE_FIELDS: [&str; 8] = [
    "name",
    "shiori",
    "url",
    "fetched_on",
    "files",
    "mapping",
    "fallback",
    "ids",
];

/// `[priority_blank]` に置いてよい欄。
const PRIORITY_BLANK_FIELDS: [&str; 2] = ["alias", "not_applicable"];

/// `briefing.md` の骨組みを読む（要件 5.6・6.2）。
pub fn read_briefing(markdown: &str) -> Result<Briefing, SurveyError> {
    let file = document_file(BRIEFING_FILE);
    let root = read_blocks(markdown, &file)?;
    reject_unknown_keys(&root, &BRIEFING_TABLES, ROOT_PLACE, &file)?;

    let blank_place = "[priority_blank]";
    let blank = required_table(&root, "priority_blank", blank_place, &file)?;
    reject_unknown_keys(blank, &PRIORITY_BLANK_FIELDS, blank_place, &file)?;

    Ok(Briefing {
        ranks: read_ranks(&root, &file)?,
        stages: read_stages(&root, &file)?,
        barriers: read_barriers(&root, &file)?,
        afters: read_afters(&root, &file)?,
        priority_blank: PriorityBlank {
            alias: usize_field(blank, blank_place, "alias", &file)?,
            not_applicable: usize_field(blank, blank_place, "not_applicable", &file)?,
        },
        owner_completed: read_owner_completed(&root, &file)?,
        templates: read_templates(&root, &file)?,
    })
}

/// 順位の行を本文に現れた順に読む。
fn read_ranks(root: &toml::Table, file: &str) -> Result<Vec<RankRow>, SurveyError> {
    let mut rows = Vec::new();
    for item in array_of_tables(root, "rank", file)? {
        rows.push(read_rank(item, file)?);
    }
    Ok(rows)
}

/// 順位の行 1 つを読む。`bundle` と `singles` は**ちょうど一方**（設計 `documents::parse`）。
fn read_rank(item: &toml::Table, file: &str) -> Result<RankRow, SurveyError> {
    let place = "[[rank]]";
    reject_unknown_keys(item, &RANK_FIELDS, place, file)?;

    let raw_stage = string_field(item, place, "stage", file)?;
    let stage =
        Stage::parse(&raw_stage).ok_or_else(|| bad_vocabulary(file, place, "stage", &raw_stage))?;
    let rank = usize_field(item, place, "rank", file)?;
    // ここから先の失敗はどの行かが分かるように、段階と順位を添える。
    let place = &format!("[[rank]] {}{rank}", stage.as_key());

    let target = match (item.get("bundle"), item.get("singles")) {
        (Some(_), Some(_)) => {
            return Err(malformed(
                file,
                format!("{place}: 欄 bundle と欄 singles の両方がある（ちょうど一方だけ書く）"),
            ));
        }
        (None, None) => {
            return Err(malformed(
                file,
                format!("{place}: 欄 bundle も欄 singles も無い（ちょうど一方だけ書く）"),
            ));
        }
        (Some(_), None) => RankTarget::Bundle(string_field(item, place, "bundle", file)?),
        (None, Some(_)) => {
            let singles = id_array_field(item, place, "singles", file)?;
            if singles.is_empty() {
                return Err(malformed(file, format!("{place}: 欄 singles が空")));
            }
            RankTarget::Singles(singles)
        }
    };

    Ok(RankRow {
        stage,
        rank,
        target,
        // 数え直される値なので、書かなければ 0（設計「数の置き方の規則」）。
        assets: optional_usize_field(item, place, "assets", file)?,
        shared: optional_usize_field(item, place, "shared", file)?,
        exception: read_override(item, place, file)?,
        insufficient: bool_field(item, place, "insufficient", file)?,
    })
}

/// 4 つの根拠の順序から外す理由。書かれていなければ `None`。
fn read_override(
    item: &toml::Table,
    place: &str,
    file: &str,
) -> Result<Option<RankOverride>, SurveyError> {
    let Some(value) = item.get("override") else {
        return Ok(None);
    };
    let place = &format!("{place} の override");
    let table = as_table(value, place, file)?;
    reject_unknown_keys(table, &OVERRIDE_FIELDS, place, file)?;

    let raw_kind = string_field(table, place, "kind", file)?;
    let kind = OverrideKind::parse(&raw_kind)
        .ok_or_else(|| bad_vocabulary(file, place, "kind", &raw_kind))?;
    let reference = string_field(table, place, "ref", file)?;
    if reference.trim().is_empty() {
        return Err(malformed(file, format!("{place}: 欄 ref が空")));
    }
    Ok(Some(RankOverride { kind, reference }))
}

/// `[stage.A]`〜`[stage.E]` を読む。5 つ揃わなければ落ちる。
fn read_stages(root: &toml::Table, file: &str) -> Result<BTreeMap<Stage, StageCount>, SurveyError> {
    let table = required_table(root, "stage", "[stage]", file)?;
    let keys: Vec<&str> = Stage::ALL.iter().map(Stage::as_key).collect();
    reject_unknown_keys(table, &keys, "[stage]", file)?;

    let mut stages = BTreeMap::new();
    for stage in Stage::ALL {
        let place = &format!("[stage.{}]", stage.as_key());
        let item = required_table(table, stage.as_key(), place, file)?;
        reject_unknown_keys(item, &STAGE_FIELDS, place, file)?;
        stages.insert(
            stage,
            StageCount {
                bundles: usize_field(item, place, "bundles", file)?,
                singles: usize_field(item, place, "singles", file)?,
                items: usize_field(item, place, "items", file)?,
            },
        );
    }
    Ok(stages)
}

/// 段階 A の主障壁（ページ別の状態分布）。書かれていなければ 0 行。
fn read_barriers(root: &toml::Table, file: &str) -> Result<Vec<Barrier>, SurveyError> {
    let mut rows = Vec::new();
    for item in array_of_tables(root, "barrier", file)? {
        let page = string_field(item, "[[barrier]]", "page", file)?;
        let place = &format!("[[barrier]] {page}");
        reject_unknown_keys(item, &BARRIER_FIELDS, place, file)?;
        rows.push(Barrier {
            page: PageName::new(page.clone()),
            implemented: usize_field(item, place, "implemented", file)?,
            vocabulary_only: usize_field(item, place, "vocabulary_only", file)?,
            degraded: usize_field(item, place, "degraded", file)?,
            absent: usize_field(item, place, "absent", file)?,
            alias: usize_field(item, place, "alias", file)?,
            not_applicable: usize_field(item, place, "not_applicable", file)?,
        });
    }
    reject_repeated(
        rows.iter().map(|row| row.page.as_str()),
        "[[barrier]]",
        "page",
        file,
    )?;
    Ok(rows)
}

/// 書き戻した後のドメイン別段階分布。書かれていなければ 0 行。
fn read_afters(root: &toml::Table, file: &str) -> Result<Vec<After>, SurveyError> {
    let mut fields: Vec<&str> = Stage::ALL.iter().map(Stage::as_key).collect();
    fields.push("domain");
    fields.push("empty");

    let mut rows = Vec::new();
    for item in array_of_tables(root, "after", file)? {
        let raw_domain = string_field(item, "[[after]]", "domain", file)?;
        let place = &format!("[[after]] {raw_domain}");
        reject_unknown_keys(item, &fields, place, file)?;
        let domain = Domain::parse(&raw_domain).map_err(|err| err.at(file, place))?;

        let mut stages = BTreeMap::new();
        for stage in Stage::ALL {
            stages.insert(stage, usize_field(item, place, stage.as_key(), file)?);
        }
        rows.push(After {
            domain,
            stages,
            empty: usize_field(item, place, "empty", file)?,
        });
    }
    reject_repeated(
        rows.iter().map(|row| row.domain.as_key()),
        "[[after]]",
        "domain",
        file,
    )?;
    Ok(rows)
}

/// 完了済み spec を `owner` に残した宛先。書かれていなければ 0 行。
fn read_owner_completed(
    root: &toml::Table,
    file: &str,
) -> Result<Vec<OwnerCompleted>, SurveyError> {
    let mut rows = Vec::new();
    for item in array_of_tables(root, "owner_completed", file)? {
        let spec = string_field(item, "[[owner_completed]]", "spec", file)?;
        let place = &format!("[[owner_completed]] {spec}");
        reject_unknown_keys(item, &OWNER_COMPLETED_FIELDS, place, file)?;
        rows.push(OwnerCompleted {
            items: usize_field(item, place, "items", file)?,
            spec,
        });
    }
    reject_repeated(
        rows.iter().map(|row| row.spec.as_str()),
        "[[owner_completed]]",
        "spec",
        file,
    )?;
    Ok(rows)
}

/// 参照した標準テンプレート辞書。書かれていなければ 0 行。
fn read_templates(root: &toml::Table, file: &str) -> Result<Vec<super::Template>, SurveyError> {
    let mut rows = Vec::new();
    for item in array_of_tables(root, "template", file)? {
        let name = string_field(item, "[[template]]", "name", file)?;
        let place = &format!("[[template]] {name}");
        reject_unknown_keys(item, &TEMPLATE_FIELDS, place, file)?;
        // 人が読むための欄。形だけ確かめて値は持たない。
        string_field(item, place, "shiori", file)?;
        string_field(item, place, "url", file)?;
        string_field(item, place, "fetched_on", file)?;
        string_array_field(item, place, "files", file)?;
        string_field(item, place, "mapping", file)?;
        bool_field(item, place, "fallback", file)?;
        rows.push(super::Template {
            ids: id_array_field(item, place, "ids", file)?,
            name,
        });
    }
    reject_repeated(
        rows.iter().map(|row| row.name.as_str()),
        "[[template]]",
        "name",
        file,
    )?;
    Ok(rows)
}

// ---------------------------------------------------------------------------
// roadmap-draft.md
// ---------------------------------------------------------------------------

/// `roadmap-draft.md` の根に置いてよい表。
const ROADMAP_TABLES: [&str; 3] = ["briefs", "spec", "reserved"];

/// `[briefs]` に置いてよい欄。
const BRIEFS_FIELDS: [&str; 2] = ["count", "snapshot_on"];

/// `[[spec]]` に置いてよい欄。
const SPEC_FIELDS: [&str; 7] = [
    "name",
    "wave",
    "stage",
    "bundle",
    "none",
    "reason",
    "owner_count",
];

/// `[[reserved]]` に置いてよい欄。
const RESERVED_FIELDS: [&str; 4] = ["name", "bundle", "none", "reason"];

/// `roadmap-draft.md` の骨組みを読む（要件 10.2）。
pub fn read_roadmap_draft(markdown: &str) -> Result<RoadmapDraft, SurveyError> {
    let file = document_file(ROADMAP_FILE);
    let root = read_blocks(markdown, &file)?;
    reject_unknown_keys(&root, &ROADMAP_TABLES, ROOT_PLACE, &file)?;

    let place = "[briefs]";
    let briefs = required_table(&root, "briefs", place, &file)?;
    reject_unknown_keys(briefs, &BRIEFS_FIELDS, place, &file)?;

    let mut specs = Vec::new();
    for item in array_of_tables(&root, "spec", &file)? {
        let name = string_field(item, "[[spec]]", "name", &file)?;
        let place = &format!("[[spec]] {name}");
        reject_unknown_keys(item, &SPEC_FIELDS, place, &file)?;
        let bundle = read_bundle_ref(item, place, &file)?;
        // ウェーブは人が読むための欄。形だけ確かめて値は持たない。
        string_field(item, place, "wave", &file)?;
        specs.push(SpecRow {
            stage: read_stage(item, place, &file, &bundle)?,
            bundle,
            owner_count: usize_field(item, place, "owner_count", &file)?,
            name,
        });
    }
    reject_repeated(
        specs.iter().map(|row| row.name.as_str()),
        "[[spec]]",
        "name",
        &file,
    )?;

    let mut reserved = Vec::new();
    for item in array_of_tables(&root, "reserved", &file)? {
        let name = string_field(item, "[[reserved]]", "name", &file)?;
        let place = &format!("[[reserved]] {name}");
        reject_unknown_keys(item, &RESERVED_FIELDS, place, &file)?;
        reserved.push(Reserved {
            bundle: read_bundle_ref(item, place, &file)?,
            name,
        });
    }
    reject_repeated(
        reserved.iter().map(|row| row.name.as_str()),
        "[[reserved]]",
        "name",
        &file,
    )?;

    Ok(RoadmapDraft {
        briefs: Briefs {
            count: usize_field(briefs, place, "count", &file)?,
            snapshot_on: string_field(briefs, place, "snapshot_on", &file)?,
        },
        specs,
        reserved,
    })
}

/// 段階を読む。束を持つ行は必ず持ち、どの束にも属さない行は必ず持たない。
///
/// 段階は束が順位表で置かれている段階の写しなので、束の無い行に台帳から決まる段階は
/// 無い。そこを既定値で埋めると、値でない綴りが値のふりをして段階の分布を狂わせる
/// （置き字の `stage = "A"` が本物の `A` に混ざる）。だから省略を**強制**する。
fn read_stage(
    item: &toml::Table,
    place: &str,
    file: &str,
    bundle: &BundleRef,
) -> Result<Option<Stage>, SurveyError> {
    if matches!(bundle, BundleRef::None { .. }) {
        reject_present(item, place, file, &["stage"])?;
        return Ok(None);
    }
    let raw = string_field(item, place, "stage", file)?;
    Stage::parse(&raw)
        .map(Some)
        .ok_or_else(|| bad_vocabulary(file, place, "stage", &raw))
}

/// 属する束、または「どの束にも属さない＋理由」を読む。「属さない」を空欄や省略で
/// 表すことは許さない（要件 11.5）——黙って空にできると、束を書き忘れた行と、束が
/// 無いことを確かめた行が見分けられなくなる。
fn read_bundle_ref(item: &toml::Table, place: &str, file: &str) -> Result<BundleRef, SurveyError> {
    if bool_field(item, place, "none", file)? {
        reject_present(item, place, file, &["bundle"])?;
        let reason = string_field(item, place, "reason", file)?;
        if reason.trim().is_empty() {
            return Err(malformed(file, format!("{place}: 欄 reason が空")));
        }
        return Ok(BundleRef::None { reason });
    }

    reject_present(item, place, file, &["reason"])?;
    if !item.contains_key("bundle") {
        return Err(malformed(
            file,
            format!("{place}: 欄 bundle が無い（属さないなら none = true と reason を書く）"),
        ));
    }
    let bundle = string_field(item, place, "bundle", file)?;
    if bundle.trim().is_empty() {
        return Err(malformed(
            file,
            format!("{place}: 欄 bundle が空（属さないなら none = true と reason を書く）"),
        ));
    }
    Ok(BundleRef::Named(bundle))
}

#[cfg(test)]
#[path = "parse_tests.rs"]
mod tests;
