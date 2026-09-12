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
//! # 4 つの口
//!
//! 判定 ⑴ が拾う id の口はここに 4 つ置く——囲みの中の引用符付き（[`toml_blocks`]
//! ＋ [`quoted_ids`]）・地の文の逆引用符付き（[`backticked_ids`]）・どちらにも
//! 囲まれていない裸の語（[`bare_id_tokens`]・0 件であるべき）。
//! `tests/consistency/examples.rs` が自前で持っていた 2 つはここへ寄せた。

use std::collections::BTreeMap;

use super::fields::{
    array_of_tables, as_table, bad_vocabulary, bool_field, domain_array_field, id_array_field,
    malformed, optional_usize_field, read_blocks, reject_present, reject_unknown_keys,
    required_table, string_array_field, string_field, theme_array_field, usize_field,
};
use super::{
    After, Barrier, Breakage, Briefing, Briefs, BundleRef, Linkage, NamedBundle, OverrideKind,
    OwnerCompleted, PriorityBlank, RankOverride, RankRow, RankTarget, Reserved, RoadmapDraft,
    SpecRow, Stage, StageCount, Tally,
};
use crate::error::SurveyError;
use crate::model::{Domain, PageName};

/// 失敗の本文に添える文書の置き場を組む道具（`derive.rs` が引く公開名）。
///
/// 実体は共通の道具の側（`documents::fields`）にある。`parse.rs` から割ったときに
/// 名前の引き方を変えないよう、ここで再輸出して綴りを保つ。
pub(super) use super::fields::document_file;

/// 3 文書の置き場（`io::paths` と同じ場所・ワークスペース根から）。
///
/// `io::paths` を呼ばないのは、あれがワークスペース根から組み立てた絶対パスを返し、
/// 失敗の本文が計算機ごとに変わってしまうからである（`ledger::read` と同じ判断）。
pub(super) const DOCUMENT_DIR: &str = "doc/ukadoc-coverage";

/// 帰属の正本。
pub(super) const LINKAGE_FILE: &str = "linkage.md";
/// 段階と順位。
pub(super) const BRIEFING_FILE: &str = "briefing.md";
/// ロードマップ草案。
const ROADMAP_FILE: &str = "roadmap-draft.md";

/// 項目 id の先頭に必ず置かれる印（`model::EntryId` が受け付ける 2 形の共通の頭）。
const ID_MARK: &str = "ukadoc:";

/// 囲みの始まりの行。
const FENCE_OPEN: &str = "```toml";
/// 囲みの終わりの行（どの言語の囲みでも同じ）。
const FENCE: &str = "```";

/// 囲みを連結した本文の根。失敗に添える場所として使う。
const ROOT_PLACE: &str = "囲みの根";

// ---------------------------------------------------------------------------
// 4 つの口
// ---------------------------------------------------------------------------

/// ` ```toml ` で始まり ` ``` ` で閉じる囲みの中身だけを、現れた順に返す。
///
/// 入れ子は扱わない（Markdown の囲みは入れ子にならない）。閉じ忘れた囲みは本文の
/// 終わりまでを 1 つの囲みとして返す——取りこぼして黙って緑になるより、中身を
/// 検査に掛けたほうが安全側である。
pub fn toml_blocks(markdown: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut current: Option<Vec<&str>> = None;
    for line in markdown.lines() {
        match current.as_mut() {
            Some(lines) => {
                if line.trim_end() == FENCE {
                    blocks.push(lines.join("\n"));
                    current = None;
                } else {
                    lines.push(line);
                }
            }
            None => {
                if line.trim_end() == FENCE_OPEN {
                    current = Some(Vec::new());
                }
            }
        }
    }
    if let Some(lines) = current {
        blocks.push(lines.join("\n"));
    }
    blocks
}

/// 本文に現れる `"ukadoc:…"` の綴りを、現れた順に返す（重複は落とさない）。
///
/// 二重引用符で囲まれた `ukadoc:` 始まりの文字列だけを拾う。台帳でも 3 文書でも
/// id は必ず引用符の中に書かれるので、これで表の鍵も配列の要素も同じ規則で拾える。
pub fn quoted_ids(text: &str) -> Vec<String> {
    let mut ids = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("\"ukadoc:") {
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

/// 囲み（どの言語でも）の外の行だけを返す。
fn prose_lines(markdown: &str) -> Vec<&str> {
    let mut lines = Vec::new();
    let mut inside = false;
    for line in markdown.lines() {
        if line.trim_start().starts_with(FENCE) {
            inside = !inside;
            continue;
        }
        if !inside {
            lines.push(line);
        }
    }
    lines
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
        let reason = string_field(item, place, "reason", file)?;
        if reason.trim().is_empty() {
            return Err(malformed(file, format!("{place}: 欄 reason が空")));
        }
        (Vec::new(), Vec::new(), String::new(), reason)
    } else {
        reject_present(item, place, file, &["reason"])?;
        let machine = id_array_field(item, place, "machine", file)?;
        let hand = id_array_field(item, place, "hand", file)?;
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

    Ok(NamedBundle {
        name: name.to_owned(),
        single,
        machine,
        members,
        hand,
        domains: domain_array_field(item, place, "domains", file)?,
        foundation,
        breakage,
        themes: theme_array_field(item, place, "themes", file)?,
        reason,
    })
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
        (None, Some(_)) => RankTarget::Singles(id_array_field(item, place, "singles", file)?),
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
    Ok(rows)
}

/// 参照した標準テンプレート辞書。書かれていなければ 0 行。
fn read_templates(root: &toml::Table, file: &str) -> Result<Vec<super::Template>, SurveyError> {
    let mut rows = Vec::new();
    for item in array_of_tables(root, "template", file)? {
        let name = string_field(item, "[[template]]", "name", file)?;
        let place = &format!("[[template]] {name}");
        reject_unknown_keys(item, &TEMPLATE_FIELDS, place, file)?;
        rows.push(super::Template {
            shiori: string_field(item, place, "shiori", file)?,
            url: string_field(item, place, "url", file)?,
            fetched_on: string_field(item, place, "fetched_on", file)?,
            files: string_array_field(item, place, "files", file)?,
            mapping: string_field(item, place, "mapping", file)?,
            fallback: bool_field(item, place, "fallback", file)?,
            ids: id_array_field(item, place, "ids", file)?,
            name,
        });
    }
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
        specs.push(SpecRow {
            wave: string_field(item, place, "wave", &file)?,
            stage: read_stage(item, place, &file, &bundle)?,
            bundle,
            owner_count: usize_field(item, place, "owner_count", &file)?,
            name,
        });
    }

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
/// ——`stage = "A"` を置き字として 14 行に並べた版が実際にそうなっていた
/// （27 行中 23 行が `A`・本物は 9 行）。だから省略を**強制**する。
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
    let bundle = string_field(item, place, "bundle", file).map_err(|_| {
        malformed(
            file,
            format!("{place}: 欄 bundle が無い（属さないなら none = true と reason を書く）"),
        )
    })?;
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
