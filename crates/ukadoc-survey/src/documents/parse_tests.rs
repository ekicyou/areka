//! [`super`]（3 文書の骨組みの読み手）の在中テスト。
//!
//! 相手にするのは**テスト本文の中の文字列だけ**である。`doc/ukadoc-coverage/` の
//! 実ファイルは 1 行も読まない——実データを相手にする判定は統合テストの担当で
//! （設計 D-3）、ここで読むと文書の書きかけがそのまま純粋層のテストを赤くする。
//!
//! 守るのは次の 4 つと、形の検査の主な分岐である。
//!
//! - 囲みを**連結して 1 度だけ**読むこと（配列表を複数の囲みに分けて書ける）
//! - 同じ鍵が 2 度現れたら落ちること（囲みをまたいでも）
//! - 欄が欠けたら**鍵を名指す**こと（行番号は添えない）
//! - 3 つの id の口（引用符・逆引用符・裸）の振り分け

use super::*;

// ---------------------------------------------------------------------------
// 見本の本文
// ---------------------------------------------------------------------------

/// 名前付き束 1 つ分の囲み（設計 D-2 の例そのままの欄）。
const BUNDLE_BLOCK: &str = r#"```toml
[bundle."時刻の刻み"]
machine = ["ukadoc:descript_plugin:secondchangeinterval:1"]
members = [
  "ukadoc:descript_plugin:secondchangeinterval:1",
  "ukadoc:list_shiori_event:OnMinuteChange:1",
]
hand = ["ukadoc:list_shiori_event:OnMinuteChange:1"]
domains = ["assets", "shiori"]
foundation = "時刻イベントの発火路"
breakage = "黙って壊れる"
themes = ["気配"]
```
"#;

/// 単独項目 1 つ分の囲み（名前は id そのもの）。
const SINGLE_BLOCK: &str = r#"```toml
[bundle."ukadoc:list_shiori_event:OnMinuteChange:1"]
single = true
members = ["ukadoc:list_shiori_event:OnMinuteChange:1"]
domains = ["shiori"]
breakage = "見た目の差"
themes = []
reason = "壊れる振る舞いを書けない"
```
"#;

/// `[tally]` の囲み（4 ドメインを省略しない）。
const TALLY_BLOCK: &str = r#"```toml
[tally]
target = 3
from_machine = 1
by_hand = 1
singles = 1
alias_excluded = 27
not_applicable_excluded = 170

[tally.singles_by_domain]
assets = 0
property = 0
sakura-script = 0
shiori = 1
```
"#;

/// `[stage.A]`〜`[stage.E]` の囲み。
const STAGE_BLOCK: &str = r#"```toml
[stage.A]
bundles = 1
singles = 0
items = 2

[stage.B]
bundles = 0
singles = 0
items = 0

[stage.C]
bundles = 0
singles = 0
items = 0

[stage.D]
bundles = 0
singles = 0
items = 0

[stage.E]
bundles = 0
singles = 1
items = 1
```
"#;

/// `[priority_blank]` の囲み。
const BLANK_BLOCK: &str = r#"```toml
[priority_blank]
alias = 27
not_applicable = 170
```
"#;

/// `[briefs]` の囲み。
const BRIEFS_BLOCK: &str = r#"```toml
[briefs]
count = 1
snapshot_on = "2026-09-11"
```
"#;

/// 失敗の本文を取り出す（緑になったら名指しで落とす）。
fn err_body<T: std::fmt::Debug>(result: Result<T, crate::error::SurveyError>) -> String {
    match result {
        Ok(value) => panic!("落ちるはずが読めてしまった: {value:?}"),
        Err(err) => err.to_string(),
    }
}

// ---------------------------------------------------------------------------
// 囲みの取り出しと連結
// ---------------------------------------------------------------------------

/// 1 文書の囲みを連結して 1 度だけ読む——配列表を複数の囲みに分けて書ける。
#[test]
fn blocks_are_joined_into_one_document() {
    let markdown = format!(
        "# 段階と順位\n\n地の文。\n\n{STAGE_BLOCK}\n本文が挟まる。\n\n\
         ```toml\n[[rank]]\nstage = \"A\"\nrank = 1\nbundle = \"時刻の刻み\"\n\
         assets = 3\nshared = 2\n```\n\n\
         ```toml\n[[rank]]\nstage = \"E\"\nrank = 9\n\
         singles = [\"ukadoc:list_shiori_event:OnMinuteChange:1\"]\n```\n\n{BLANK_BLOCK}"
    );

    let briefing = read_briefing(&markdown).expect("3 つの囲みが 1 つの文書として読めるはず");

    assert_eq!(
        briefing.ranks.len(),
        2,
        "別々の囲みに書いた [[rank]] が連結されていない"
    );
    assert_eq!(
        briefing.ranks[0].target,
        RankTarget::Bundle("時刻の刻み".to_owned()),
        "1 行目は束の行"
    );
    assert_eq!(briefing.ranks[0].assets, 3);
    assert_eq!(briefing.ranks[0].shared, 2);
    assert_eq!(briefing.ranks[0].stage, Stage::A);
    assert_eq!(
        briefing.ranks[1].target,
        RankTarget::Singles(vec![
            EntryId::parse("ukadoc:list_shiori_event:OnMinuteChange:1").unwrap()
        ]),
        "2 行目は単独項目の行"
    );
    assert_eq!(
        briefing.stages.len(),
        Stage::ALL.len(),
        "[stage.*] が 5 つ揃うはず"
    );
    assert_eq!(briefing.priority_blank.alias, 27);
}

/// 囲みの外の本文は読まない（地の文に TOML を書いても効かない）。
#[test]
fn prose_outside_the_blocks_is_not_read() {
    let markdown = format!(
        "count = 999\nsnapshot_on = \"だまし\"\n\n{BRIEFS_BLOCK}\n\
         [[spec]] と地の文に書いても行にならない。\n"
    );

    let draft = read_roadmap_draft(&markdown).expect("囲みだけを読むはず");
    assert_eq!(draft.briefs.count, 1, "囲みの外の count を拾っている");
    assert_eq!(draft.briefs.snapshot_on, "2026-09-11");
    assert!(draft.specs.is_empty(), "地の文が [[spec]] の行になっている");
}

/// 取り出しそのものの較正——囲みが 2 つあれば 2 つ返り、囲みの外は入らない。
#[test]
fn toml_blocks_returns_each_fence_body_in_order() {
    let markdown = "地の文\n\n```toml\na = 1\n```\n\n```text\nb = 2\n```\n\n```toml\nc = 3\n```\n";
    assert_eq!(toml_blocks(markdown), vec!["a = 1", "c = 3"]);
}

/// 閉じ忘れた囲みは本文の終わりまでを 1 つの囲みとする——中身は TOML の口で検査に
/// 掛かり、地の文の口には混ざらない（取りこぼして黙って緑にしない）。
#[test]
fn an_unclosed_fence_runs_to_the_end_of_the_text() {
    let id = "ukadoc:list_shiori_event:OnBoot:1";
    let markdown = format!("地の文\n\n```toml\na = 1\nb = \"`{id}`\"\n");
    assert_eq!(
        toml_blocks(&markdown),
        vec![format!("a = 1\nb = \"`{id}`\"")],
        "閉じ忘れた囲みの中身を取りこぼしている"
    );
    assert!(
        backticked_ids(&markdown).is_empty(),
        "閉じ忘れた囲みの中を地の文として読んでいる"
    );
}

/// 箇条書きの中で字下げした囲みも囲みである。TOML の口と地の文の口が同じ見分け方を
/// するので、表は読まれ、中の id は地の文の口に混ざらない。
#[test]
fn an_indented_toml_fence_is_read_and_kept_out_of_the_prose() {
    let markdown = "- 写真\n\n  ```toml\n  [briefs]\n  count = 4\n  snapshot_on = \"`ukadoc:list_shiori_event:OnBoot:1`\"\n  ```\n\n地の文。\n";

    let draft = read_roadmap_draft(markdown).expect("字下げした囲みも読めるはず");
    assert_eq!(draft.briefs.count, 4, "字下げした囲みの表が読まれていない");
    assert!(
        backticked_ids(markdown).is_empty(),
        "囲みの中の綴りを地の文の口が拾っている: {:?}",
        backticked_ids(markdown)
    );
}

/// 別の言語の囲みの中に書いた「```toml」の行は、囲みを開かず、地の文の中と外も
/// 逆さにしない（閉じるのは「```」ちょうどの行だけ）。
#[test]
fn a_toml_fence_line_inside_another_fence_is_just_a_line_of_that_fence() {
    let id = "ukadoc:list_shiori_event:OnBoot:1";
    let markdown =
        format!("```text\n```toml\n[briefs]\ncount = 9\n```\n\n地の文で `{id}` を引く。\n");

    assert!(
        toml_blocks(&markdown).is_empty(),
        "例の中の行を TOML の囲みとして読んでいる: {:?}",
        toml_blocks(&markdown)
    );
    assert_eq!(
        backticked_ids(&markdown),
        vec![id],
        "例の囲みの後ろの地の文を囲みの中として扱っている"
    );
}

/// 見出しで始まらない囲みは、前の囲みの最後の表へ黙って混ざらずに落ちる。
/// 空行と `#` の行の後に見出しが来る囲みは読める（較正）。
#[test]
fn a_block_that_does_not_open_with_a_header_is_rejected() {
    let rank = "```toml\n[[rank]]\nstage = \"A\"\nrank = 1\nbundle = \"時刻の刻み\"\n```\n";
    let headless = format!(
        "{STAGE_BLOCK}\n{BLANK_BLOCK}\n{rank}\n```toml\n# 見出しを書き忘れた\n\ninsufficient = true\n```\n"
    );
    let body = err_body(read_briefing(&headless));
    assert!(body.contains("briefing.md"), "文書名が本文に無い: {body}");
    assert!(
        body.contains("insufficient"),
        "見出しの無い行が本文に無い: {body}"
    );

    let commented = format!(
        "{STAGE_BLOCK}\n{BLANK_BLOCK}\n```toml\n# 段階 A\n\n[[rank]]\nstage = \"A\"\nrank = 1\nbundle = \"時刻の刻み\"\n```\n"
    );
    let briefing = read_briefing(&commented).expect("注記の後に見出しが来る囲みは読めるはず");
    assert!(!briefing.ranks[0].insufficient);
}

// ---------------------------------------------------------------------------
// 鍵の重複
// ---------------------------------------------------------------------------

/// 同じ鍵が 2 度現れたら落ちる。囲みをまたいでも同じ（連結して 1 度読むから）。
#[test]
fn a_key_written_twice_across_blocks_is_rejected() {
    let markdown = format!("{TALLY_BLOCK}\n本文\n\n{TALLY_BLOCK}");
    let body = err_body(read_linkage(&markdown));

    assert!(
        body.contains("linkage.md"),
        "文書名が本文に無い（設計 Error Handling）: {body}"
    );
    assert!(body.contains("tally"), "重複した表の鍵が本文に無い: {body}");
    assert!(
        !body.contains("line "),
        "行番号を添えている（設計 Error Handling）: {body}"
    );

    // 母数の較正——重複を取り除けば同じ本文が読める。
    read_linkage(TALLY_BLOCK).expect("重複を除けば読めるはず");
}

// ---------------------------------------------------------------------------
// 欄の欠落と語彙
// ---------------------------------------------------------------------------

/// 欄が欠けたら、欠けた鍵と表の鍵を名指す。
#[test]
fn a_missing_field_names_the_key_and_the_table() {
    let broken = TALLY_BLOCK.replace("singles = 1\n", "");
    let body = err_body(read_linkage(&broken));

    assert!(body.contains("linkage.md"), "文書名が本文に無い: {body}");
    assert!(
        body.contains("singles"),
        "欠けた欄の名前が本文に無い: {body}"
    );
    assert!(body.contains("tally"), "表の鍵が本文に無い: {body}");
}

/// 単独項目のドメイン別内訳は 0 のドメインも省略できない（要件 4.2・11.5）。
#[test]
fn singles_by_domain_must_list_every_domain() {
    let broken = TALLY_BLOCK.replace("property = 0\n", "");
    let body = err_body(read_linkage(&broken));

    assert!(
        body.contains("property"),
        "欠けたドメインを名指していない: {body}"
    );
    assert!(
        body.contains("singles_by_domain"),
        "表の鍵が本文に無い: {body}"
    );
}

/// 語彙外の壊れ方は落ちる（値と束名を添える）。
#[test]
fn a_breakage_outside_the_vocabulary_is_rejected() {
    let markdown = format!(
        "{}\n{TALLY_BLOCK}",
        BUNDLE_BLOCK.replace("黙って壊れる", "たぶん壊れる")
    );
    let body = err_body(read_linkage(&markdown));

    assert!(
        body.contains("たぶん壊れる"),
        "書かれていた値が無い: {body}"
    );
    assert!(body.contains("時刻の刻み"), "束名が本文に無い: {body}");

    // 較正——語彙の綴りに戻せば読める。
    let good = format!("{BUNDLE_BLOCK}\n{TALLY_BLOCK}");
    let linkage = read_linkage(&good).expect("語彙どおりなら読めるはず");
    assert_eq!(
        linkage.bundles["時刻の刻み"].breakage,
        Breakage::Silent,
        "壊れ方の綴りが型に写っていない"
    );
}

/// 語彙外のテーマは落ちる（台帳と同じ 8 テーマ・要件 4.4）。
#[test]
fn a_theme_outside_the_vocabulary_is_rejected() {
    let markdown = format!(
        "{}\n{TALLY_BLOCK}",
        BUNDLE_BLOCK.replace("\"気配\"", "\"きはい\"")
    );
    let body = err_body(read_linkage(&markdown));
    assert!(body.contains("きはい"), "書かれていた値が無い: {body}");
}

/// 知らない欄は黙って読み飛ばさない（台帳の読み手と同じ流儀）。
#[test]
fn an_unknown_field_is_rejected() {
    let markdown = format!(
        "{}\n{TALLY_BLOCK}",
        BUNDLE_BLOCK.replace("themes = [\"気配\"]", "themes = [\"気配\"]\nweight = 3")
    );
    let body = err_body(read_linkage(&markdown));
    assert!(
        body.contains("weight"),
        "知らない欄の名前が本文に無い: {body}"
    );
}

/// 名前付き束と単独項目は書ける欄が違う（設計 D-2）。
#[test]
fn a_single_item_bundle_has_its_own_shape() {
    let single = SINGLE_BLOCK;
    let good = format!("{single}\n{TALLY_BLOCK}");
    let linkage = read_linkage(&good).expect("単独項目の形は読めるはず");
    let bundle = &linkage.bundles["ukadoc:list_shiori_event:OnMinuteChange:1"];
    assert!(bundle.single);
    assert_eq!(bundle.members.len(), 1);
    assert_eq!(bundle.reason, "壊れる振る舞いを書けない");

    // 理由の無い単独項目は落ちる（要件 4.4）。
    let no_reason = single.replace("reason = \"壊れる振る舞いを書けない\"\n", "");
    let body = err_body(read_linkage(&format!("{no_reason}\n{TALLY_BLOCK}")));
    assert!(
        body.contains("reason"),
        "欠けた欄の名前が本文に無い: {body}"
    );

    // 名前付き束に理由は書けない（単独項目に落としたことの印だから）。
    let both = format!(
        "{}\n{TALLY_BLOCK}",
        BUNDLE_BLOCK.replace(
            "themes = [\"気配\"]",
            "themes = [\"気配\"]\nreason = \"？\""
        )
    );
    let body = err_body(read_linkage(&both));
    assert!(body.contains("reason"), "欄の名前が本文に無い: {body}");
}

/// 人手で足した id は構成 id の部分集合でなければならない（設計 D-2）。
#[test]
fn hand_ids_must_be_members() {
    let markdown = format!(
        "{}\n{TALLY_BLOCK}",
        BUNDLE_BLOCK.replace(
            "hand = [\"ukadoc:list_shiori_event:OnMinuteChange:1\"]",
            "hand = [\"ukadoc:list_shiori_event:OnBoot:1\"]",
        )
    );
    let body = err_body(read_linkage(&markdown));
    assert!(
        body.contains("ukadoc:list_shiori_event:OnBoot:1"),
        "はみ出した id が本文に無い: {body}"
    );
    assert!(body.contains("時刻の刻み"), "束名が本文に無い: {body}");
}

/// 単独項目の名前と中の id が食い違えば、両方の綴りを挙げて落ちる——順位は名前で
/// 引かれるので、通すと違う項目に優先度が付く。
#[test]
fn a_single_item_whose_member_differs_from_its_name_is_rejected() {
    let other = "ukadoc:list_shiori_event:OnBoot:1";
    let broken = SINGLE_BLOCK.replace(
        "members = [\"ukadoc:list_shiori_event:OnMinuteChange:1\"]",
        &format!("members = [\"{other}\"]"),
    );
    let body = err_body(read_linkage(&format!("{broken}\n{TALLY_BLOCK}")));
    assert!(
        body.contains("ukadoc:list_shiori_event:OnMinuteChange:1"),
        "名前の綴りが本文に無い: {body}"
    );
    assert!(body.contains(other), "中の id の綴りが本文に無い: {body}");
}

/// 1 つの束の `members` に同じ id を 2 度書けば、その id と束名を挙げて落ちる。
#[test]
fn an_id_written_twice_in_one_bundle_is_rejected() {
    let twice = "ukadoc:descript_plugin:secondchangeinterval:1";
    let broken = BUNDLE_BLOCK.replace("members = [\n", &format!("members = [\n  \"{twice}\",\n"));
    let body = err_body(read_linkage(&format!("{broken}\n{TALLY_BLOCK}")));
    assert!(body.contains(twice), "重複した id が本文に無い: {body}");
    assert!(body.contains("members"), "欄の名前が本文に無い: {body}");
    assert!(body.contains("時刻の刻み"), "束名が本文に無い: {body}");
}

// ---------------------------------------------------------------------------
// 順位の行と段階
// ---------------------------------------------------------------------------

/// `[[rank]]` は `bundle` と `singles` のちょうど一方を持つ。
#[test]
fn a_rank_row_names_either_a_bundle_or_singles_but_not_both() {
    let head = format!("{STAGE_BLOCK}\n{BLANK_BLOCK}\n");

    let both = format!(
        "{head}```toml\n[[rank]]\nstage = \"A\"\nrank = 1\nbundle = \"時刻の刻み\"\n\
         singles = [\"ukadoc:list_shiori_event:OnBoot:1\"]\n```\n"
    );
    let body = err_body(read_briefing(&both));
    assert!(body.contains("bundle"), "欄の名前が本文に無い: {body}");
    assert!(body.contains("singles"), "欄の名前が本文に無い: {body}");

    let neither = format!("{head}```toml\n[[rank]]\nstage = \"A\"\nrank = 1\n```\n");
    let body = err_body(read_briefing(&neither));
    assert!(body.contains("bundle"), "欄の名前が本文に無い: {body}");
    assert!(body.contains("singles"), "欄の名前が本文に無い: {body}");

    // 較正——ちょうど一方なら読める。
    let one =
        format!("{head}```toml\n[[rank]]\nstage = \"A\"\nrank = 1\nbundle = \"時刻の刻み\"\n```\n");
    let briefing = read_briefing(&one).expect("ちょうど一方なら読めるはず");
    assert_eq!(briefing.ranks.len(), 1);
}

/// 中身の無い `singles = []` の順位の行は落ちる（指す相手の無い順位を残さない）。
#[test]
fn a_rank_row_with_empty_singles_is_rejected() {
    let markdown = format!(
        "{STAGE_BLOCK}\n{BLANK_BLOCK}\n```toml\n[[rank]]\nstage = \"B\"\nrank = 4\nsingles = []\n```\n"
    );
    let body = err_body(read_briefing(&markdown));
    assert!(body.contains("singles"), "欄の名前が本文に無い: {body}");
    assert!(body.contains("B4"), "どの行かが本文に無い: {body}");
}

/// `[stage.*]` は A〜E の 5 つが揃わなければ落ちる。
#[test]
fn every_stage_from_a_to_e_must_be_present() {
    let broken = STAGE_BLOCK.replace("[stage.E]\nbundles = 0\nsingles = 1\nitems = 1\n", "");
    let body = err_body(read_briefing(&format!("{broken}\n{BLANK_BLOCK}")));
    assert!(
        body.contains("stage.E"),
        "欠けた段階を名指していない: {body}"
    );

    // 較正——5 つ揃えば読める。
    read_briefing(&format!("{STAGE_BLOCK}\n{BLANK_BLOCK}")).expect("5 つ揃えば読めるはず");
}

/// 段階の文字が A〜E の外なら落ちる。
#[test]
fn a_stage_letter_outside_a_to_e_is_rejected() {
    let markdown = format!(
        "{STAGE_BLOCK}\n{BLANK_BLOCK}\n```toml\n[[rank]]\nstage = \"F\"\nrank = 1\n\
         bundle = \"時刻の刻み\"\n```\n"
    );
    let body = err_body(read_briefing(&markdown));
    assert!(body.contains('F'), "書かれていた値が本文に無い: {body}");
}

/// `[[barrier]]`・`[[after]]`・`[[template]]`・`[[owner_completed]]` は
/// 書かれていなければ 0 行（TOML は配列表の見出しだけを空で置けない）。
#[test]
fn the_array_tables_are_zero_rows_when_absent() {
    let briefing = read_briefing(&format!("{STAGE_BLOCK}\n{BLANK_BLOCK}"))
        .expect("配列表が 1 つも無くても読めるはず");
    assert!(briefing.ranks.is_empty());
    assert!(briefing.barriers.is_empty());
    assert!(briefing.afters.is_empty());
    assert!(briefing.owner_completed.is_empty());
    assert!(briefing.templates.is_empty());

    let draft = read_roadmap_draft(BRIEFS_BLOCK).expect("[[spec]] が無くても読めるはず");
    assert!(draft.specs.is_empty());
    assert!(draft.reserved.is_empty());

    // 較正——1 行足せばその 1 行が読める（0 件の緑が恒真でないことの確認）。
    let with_row = format!(
        "{STAGE_BLOCK}\n{BLANK_BLOCK}\n```toml\n[[after]]\ndomain = \"shiori\"\n\
         A = 1\nB = 0\nC = 0\nD = 0\nE = 0\nempty = 2\n```\n"
    );
    let briefing = read_briefing(&with_row).expect("[[after]] 1 行を読めるはず");
    assert_eq!(briefing.afters.len(), 1);
    assert_eq!(briefing.afters[0].domain, Domain::Shiori);
    assert_eq!(briefing.afters[0].stages[&Stage::A], 1);
    assert_eq!(briefing.afters[0].empty, 2);
}

// ---------------------------------------------------------------------------
// roadmap-draft.md
// ---------------------------------------------------------------------------

/// spec 表の行は束名か「どの束にも属さない＋理由」のどちらかを持つ（要件 11.5）。
///
/// 段階の欄は束と一蓮托生である——束を持つ行は必ず持ち、持たない行は必ず持たない。
#[test]
fn a_spec_row_carries_a_bundle_or_a_stated_none() {
    let markdown = format!(
        "{BRIEFS_BLOCK}\n```toml\n[[spec]]\nname = \"areka-P0-present-gpu-transform-scale\"\n\
         wave = \"W13\"\nstage = \"A\"\nbundle = \"時刻の刻み\"\nowner_count = 3\n\n\
         [[spec]]\nname = \"tick-gate-adoption\"\nwave = \"保留\"\n\
         none = true\nreason = \"どの束にも写らない\"\nowner_count = 0\n```\n"
    );
    let draft = read_roadmap_draft(&markdown).expect("spec 表が読めるはず");
    assert_eq!(draft.specs.len(), 2);
    assert_eq!(
        draft.specs[0].bundle,
        BundleRef::Named("時刻の刻み".to_owned())
    );
    assert_eq!(draft.specs[0].stage, Some(Stage::A));
    assert_eq!(draft.specs[0].owner_count, 3);
    assert_eq!(
        draft.specs[1].bundle,
        BundleRef::None {
            reason: "どの束にも写らない".to_owned()
        }
    );
    assert_eq!(
        draft.specs[1].stage, None,
        "束の無い行に段階が付いている（置き字が値のふりをする）"
    );

    // 空欄で「属さない」を表すことは許さない。
    let silent = format!(
        "{BRIEFS_BLOCK}\n```toml\n[[spec]]\nname = \"tick-gate-adoption\"\n\
         wave = \"保留\"\nstage = \"E\"\nowner_count = 0\n```\n"
    );
    let body = err_body(read_roadmap_draft(&silent));
    assert!(body.contains("bundle"), "欄の名前が本文に無い: {body}");
    assert!(
        body.contains("tick-gate-adoption"),
        "どの行かが本文に無い: {body}"
    );
}

/// 段階の欄は束と一蓮托生で、片方だけを書いた行は読めない。
///
/// 置き字を締め出すのはこの 2 本である——束の無い行に段階を書けば落ち、束のある行が
/// 段階を書かなくても落ちる。どちらかが抜けると、束の無い行に置き字の `stage = "A"`
/// を並べた版がまた通ってしまう。
#[test]
fn the_stage_field_follows_the_bundle() {
    let with_stage = format!(
        "{BRIEFS_BLOCK}\n```toml\n[[spec]]\nname = \"tick-gate-adoption\"\n\
         wave = \"保留\"\nstage = \"A\"\nnone = true\nreason = \"どの束にも写らない\"\n\
         owner_count = 0\n```\n"
    );
    let body = err_body(read_roadmap_draft(&with_stage));
    assert!(body.contains("stage"), "欄の名前が本文に無い: {body}");
    assert!(
        body.contains("tick-gate-adoption"),
        "どの行かが本文に無い: {body}"
    );

    let without_stage = format!(
        "{BRIEFS_BLOCK}\n```toml\n[[spec]]\nname = \"areka-P0-present-gpu-transform-scale\"\n\
         wave = \"W13\"\nbundle = \"時刻の刻み\"\nowner_count = 3\n```\n"
    );
    let body = err_body(read_roadmap_draft(&without_stage));
    assert!(body.contains("stage"), "欄の名前が本文に無い: {body}");

    let bad = format!(
        "{BRIEFS_BLOCK}\n```toml\n[[spec]]\nname = \"areka-P0-present-gpu-transform-scale\"\n\
         wave = \"W13\"\nstage = \"Z\"\nbundle = \"時刻の刻み\"\nowner_count = 3\n```\n"
    );
    let body = err_body(read_roadmap_draft(&bad));
    assert!(body.contains('Z'), "語彙外の綴りが本文に無い: {body}");
}

/// `bundle` の型の誤りは「欄が無い」でなく型の誤りとして告げる。
#[test]
fn a_bundle_of_the_wrong_type_is_reported_as_the_wrong_type() {
    let markdown = format!(
        "{BRIEFS_BLOCK}\n```toml\n[[spec]]\nname = \"tick-gate-adoption\"\nwave = \"W13\"\n\
         stage = \"A\"\nbundle = [\"時刻の刻み\"]\nowner_count = 0\n```\n"
    );
    let body = err_body(read_roadmap_draft(&markdown));
    assert!(
        body.contains("文字列でない"),
        "型の誤りと告げていない: {body}"
    );
    assert!(
        !body.contains("が無い"),
        "書いてある欄を「無い」と告げている: {body}"
    );
}

// ---------------------------------------------------------------------------
// 形の検査を 1 か所ずつ
// ---------------------------------------------------------------------------

/// 順位の行に `override` を持たせた囲み（見本を壊す元）。
const OVERRIDE_RANK: &str = "```toml\n[[rank]]\nstage = \"A\"\nrank = 1\nbundle = \"時刻の刻み\"\n\
     override = { kind = \"second-stage\", ref = \"M1 の持ち越し\" }\n```\n";

/// 束のある行と束の無い行の spec 表（見本を壊す元）。
const SPEC_BLOCK: &str = "```toml\n[[spec]]\nname = \"areka-P0-present-gpu-transform-scale\"\n\
     wave = \"W13\"\nstage = \"A\"\nbundle = \"時刻の刻み\"\nowner_count = 3\n\n\
     [[spec]]\nname = \"tick-gate-adoption\"\nwave = \"保留\"\n\
     none = true\nreason = \"どの束にも写らない\"\nowner_count = 0\n```\n";

/// 配列表の 1 行ずつの見本（2 度並べて重複を作る元）。
const AFTER_ROW: &str = "```toml\n\
     [[after]]\n\
     domain = \"shiori\"\n\
     A = 1\n\
     B = 0\n\
     C = 0\n\
     D = 0\n\
     E = 0\n\
     empty = 2\n\
     ```\n";
const BARRIER_ROW: &str = "```toml\n\
     [[barrier]]\n\
     page = \"list_shiori_event\"\n\
     implemented = 1\n\
     vocabulary_only = 0\n\
     degraded = 0\n\
     absent = 0\n\
     alias = 0\n\
     not_applicable = 0\n\
     ```\n";
const OWNER_ROW: &str = "```toml\n\
     [[owner_completed]]\n\
     spec = \"areka-P0-done\"\n\
     items = 1\n\
     ```\n";
const RESERVED_ROW: &str = "```toml\n\
     [[reserved]]\n\
     name = \"areka-P0-later\"\n\
     none = true\n\
     reason = \"まだ束が無い\"\n\
     ```\n";
const TEMPLATE_ROW: &str = "```toml\n\
     [[template]]\n\
     name = \"yaya\"\n\
     shiori = \"YAYA\"\n\
     url = \"https://example.test/\"\n\
     fetched_on = \"2026-09-11\"\n\
     files = []\n\
     mapping = \"写像\"\n\
     fallback = false\n\
     ids = []\n\
     ```\n";

/// 見本を 1 か所ずつ書き換えた写しが、どれも落ち、本文に壊した欄の名前が載る。
///
/// 見本そのものは読める（較正）。`override` の 2 つの綴りが型へ写ることもここで見る
/// ——実データに無い綴りの対応表が壊れても、ほかに赤にする所が無い。
#[test]
fn each_single_break_of_the_samples_is_rejected_naming_its_field() {
    let linkage = |bundle: &str, single: &str| {
        read_linkage(&format!("{bundle}\n{single}\n{TALLY_BLOCK}")).map(|_| ())
    };
    let briefing =
        |rank: &str| read_briefing(&format!("{STAGE_BLOCK}\n{BLANK_BLOCK}\n{rank}")).map(|_| ());
    let roadmap = |spec: &str| read_roadmap_draft(&format!("{BRIEFS_BLOCK}\n{spec}")).map(|_| ());
    let bundle = |from: &str, to: &str| linkage(&BUNDLE_BLOCK.replace(from, to), SINGLE_BLOCK);
    let single = |from: &str, to: &str| linkage(BUNDLE_BLOCK, &SINGLE_BLOCK.replace(from, to));
    let rank = |from: &str, to: &str| briefing(&OVERRIDE_RANK.replace(from, to));
    let spec = |from: &str, to: &str| roadmap(&SPEC_BLOCK.replace(from, to));

    linkage(BUNDLE_BLOCK, SINGLE_BLOCK).expect("見本の帰属は読めるはず");
    roadmap(SPEC_BLOCK).expect("見本の spec 表は読めるはず");
    roadmap(RESERVED_ROW).expect("見本の [[reserved]] の 1 行は読めるはず");
    for row in [AFTER_ROW, BARRIER_ROW, OWNER_ROW, TEMPLATE_ROW] {
        briefing(row).expect("見本の配列表の 1 行は読めるはず");
    }
    let twice = |row: &str| briefing(&format!("{row}\n{row}"));
    for (kind, want) in [
        ("second-stage", OverrideKind::SecondStage),
        ("stage-rule", OverrideKind::StageRule),
    ] {
        let read = read_briefing(&format!(
            "{STAGE_BLOCK}\n{BLANK_BLOCK}\n{}",
            OVERRIDE_RANK.replace("second-stage", kind)
        ))
        .expect("見本の順位の行は読めるはず");
        let exception = read.ranks[0]
            .exception
            .as_ref()
            .expect("override を読むはず");
        assert_eq!(
            exception.kind, want,
            "override の綴り {kind} が型へ写らない"
        );
        assert_eq!(exception.reference, "M1 の持ち越し");
    }

    let hand = "hand = [\"ukadoc:list_shiori_event:OnMinuteChange:1\"]";
    let machine = "machine = [\"ukadoc:descript_plugin:secondchangeinterval:1\"]";
    let one = "members = [\"ukadoc:list_shiori_event:OnMinuteChange:1\"]";
    let members = "members = [\n  \"ukadoc:descript_plugin:secondchangeinterval:1\",\n  \
                   \"ukadoc:list_shiori_event:OnMinuteChange:1\",\n]";
    let cases = [
        ("空の members", bundle(members, "members = []"), "members"),
        (
            "hand の重複",
            bundle(
                hand,
                &hand.replace("\"]", "\", \"ukadoc:list_shiori_event:OnMinuteChange:1\"]"),
            ),
            "hand",
        ),
        (
            "machine の重複",
            bundle(
                machine,
                &machine.replace(
                    "\"]",
                    "\", \"ukadoc:descript_plugin:secondchangeinterval:1\"]",
                ),
            ),
            "machine",
        ),
        (
            "空白だけの foundation",
            bundle("\"時刻イベントの発火路\"", "\"  \""),
            "foundation",
        ),
        (
            "単独項目の machine",
            single(one, &format!("{one}\nmachine = []")),
            "machine",
        ),
        (
            "単独項目の hand",
            single(one, &format!("{one}\nhand = []")),
            "hand",
        ),
        (
            "単独項目の foundation",
            single(one, &format!("{one}\nfoundation = \"基盤\"")),
            "foundation",
        ),
        (
            "単独項目の 2 つの members",
            single(
                one,
                &one.replace("\"]", "\", \"ukadoc:list_shiori_event:OnBoot:1\"]"),
            ),
            "members",
        ),
        (
            "空白だけの reason",
            single("\"壊れる振る舞いを書けない\"", "\" \""),
            "reason",
        ),
        (
            "表でない override",
            rank(
                "{ kind = \"second-stage\", ref = \"M1 の持ち越し\" }",
                "\"second-stage\"",
            ),
            "override",
        ),
        (
            "override の知らない欄",
            rank("ref = ", "weight = 1, ref = "),
            "weight",
        ),
        (
            "override の語彙外の kind",
            rank("second-stage", "third-stage"),
            "third-stage",
        ),
        (
            "override の ref が無い",
            rank(", ref = \"M1 の持ち越し\"", ""),
            "ref",
        ),
        (
            "override の空の ref",
            rank("\"M1 の持ち越し\"", "\" \""),
            "ref",
        ),
        (
            "none と bundle の両方",
            spec("none = true\n", "none = true\nbundle = \"時刻の刻み\"\n"),
            "bundle",
        ),
        (
            "none の空白だけの reason",
            spec("\"どの束にも写らない\"", "\" \""),
            "reason",
        ),
        (
            "束のある行の reason",
            spec("owner_count = 3\n", "owner_count = 3\nreason = \"？\"\n"),
            "reason",
        ),
        (
            "空の bundle",
            spec("bundle = \"時刻の刻み\"", "bundle = \"\""),
            "bundle",
        ),
        (
            "themes の重複",
            bundle("themes = [\"気配\"]", "themes = [\"気配\", \"気配\"]"),
            "themes",
        ),
        (
            "domains の重複",
            bundle(
                "domains = [\"assets\", \"shiori\"]",
                "domains = [\"shiori\", \"shiori\"]",
            ),
            "domains",
        ),
        (
            "根の知らない表",
            linkage(
                BUNDLE_BLOCK,
                &format!("{SINGLE_BLOCK}\n```toml\n[extra]\na = 1\n```\n"),
            ),
            "extra",
        ),
        ("同じ domain の [[after]]", twice(AFTER_ROW), "domain"),
        ("同じ page の [[barrier]]", twice(BARRIER_ROW), "page"),
        ("同じ spec の [[owner_completed]]", twice(OWNER_ROW), "spec"),
        ("同じ name の [[template]]", twice(TEMPLATE_ROW), "name"),
        (
            "同じ name の [[spec]]",
            roadmap(&format!("{SPEC_BLOCK}\n{SPEC_BLOCK}")),
            "name",
        ),
        (
            "同じ name の [[reserved]]",
            roadmap(&format!("{RESERVED_ROW}\n{RESERVED_ROW}")),
            "name",
        ),
    ];
    for (label, result, field) in cases {
        let body = match result {
            Ok(()) => panic!("{label}: 落ちるはずが読めてしまった"),
            Err(err) => err.to_string(),
        };
        assert!(
            body.contains(field),
            "{label}: 欄の名前 {field} が本文に無い: {body}"
        );
    }
}

// ---------------------------------------------------------------------------
// 3 つの id の口
// ---------------------------------------------------------------------------

/// 引用符・逆引用符・裸の 3 つの口に、それぞれの綴りだけが振り分けられる。
///
/// これが無いと、口のどれかが壊れて 0 件を返すだけで判定 ⑴ が緑になる
/// （`examples.rs` の較正と同じ形）。
#[test]
fn the_three_id_scanners_sort_the_spellings_into_their_own_mouths() {
    let quoted = "ukadoc:list_shiori_event:OnBoot:1";
    let backticked = "ukadoc:list_shiori_event:OnClose:1";
    let markdown = format!(
        "地の文で `{backticked}` を引く。\n\n```toml\n[bundle.\"束\"]\n\
         members = [\"{quoted}\"]\n```\n\n表の中でも `{backticked}` は同じ扱い。\n"
    );

    assert_eq!(
        toml_blocks(&markdown)
            .iter()
            .flat_map(|block| quoted_ids(block))
            .collect::<Vec<_>>(),
        vec![quoted],
        "囲みの中の引用符付きだけを拾うはず"
    );
    assert_eq!(
        backticked_ids(&markdown),
        vec![backticked, backticked],
        "地の文の逆引用符付きを現れた順に拾うはず（囲みの中は拾わない）"
    );
    assert!(
        bare_id_tokens(&markdown).is_empty(),
        "囲まれた綴りを裸の語として拾っている: {:?}",
        bare_id_tokens(&markdown)
    );

    // 母数 0 の緑を作らない——裸で書けば、その 1 件だけが挙がる。
    let bare =
        format!("{markdown}\n地の文に ukadoc:list_shiori_event:OnFirstBoot:1 と裸で書く。\n");
    assert_eq!(
        bare_id_tokens(&bare),
        vec!["ukadoc:list_shiori_event:OnFirstBoot:1"],
        "裸の語をちょうど 1 件挙げるはず"
    );
    assert_eq!(
        backticked_ids(&bare),
        vec![backticked, backticked],
        "裸の語が逆引用符の口に混じっている"
    );
}

/// 裸の口は語の途中で始まる綴りを拾わない（語境界）。
#[test]
fn the_bare_scanner_respects_word_boundaries() {
    assert!(
        bare_id_tokens("xukadoc:list_shiori_event:OnBoot:1 は別の語\n").is_empty(),
        "語の途中の綴りを拾っている"
    );
    assert_eq!(
        bare_id_tokens("（ukadoc:list_shiori_event:OnBoot:1）は拾う\n"),
        vec!["ukadoc:list_shiori_event:OnBoot:1"],
        "括弧で挟まれた裸の語を取り落としている"
    );
}
