//! [`super`]（3 文書の骨組みの読み手）の在中テスト。
//!
//! 相手にするのは**テスト本文の中の文字列だけ**である。`doc/ukadoc-coverage/` の
//! 実ファイルは 1 行も読まない——実データを相手にする判定は統合テストの担当で
//! （設計 D-3）、ここで読むと文書の書きかけがそのまま純粋層のテストを赤くする。
//!
//! 守るのは 4 つ（タスク 1.2 の完了状態）。
//!
//! - 囲みを**連結して 1 度だけ**読むこと（配列表を複数の囲みに分けて書ける）
//! - 同じ鍵が 2 度現れたら落ちること（囲みをまたいでも）
//! - 欄が欠けたら**鍵を名指す**こと（行番号は添えない）
//! - 3 種の id の口（引用符・逆引用符・裸）の振り分け

use super::*;
// 共通の道具を `fields.rs` へ割ったとき、親が引かなくなった型はここで名指す
// （`use super::*` は親の私有の輸入までは連れてこない）。
use crate::model::EntryId;

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
        "行番号を添えている（タスク 1.2・要件 12.6 の向き）: {body}"
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
    let single = r#"```toml
[bundle."ukadoc:list_shiori_event:OnMinuteChange:1"]
single = true
members = ["ukadoc:list_shiori_event:OnMinuteChange:1"]
domains = ["shiori"]
breakage = "見た目の差"
themes = []
reason = "壊れる振る舞いを書けない"
```
"#;
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
         [[spec]]\nname = \"areka-P0-tick-gate-adoption\"\nwave = \"保留\"\n\
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
        "{BRIEFS_BLOCK}\n```toml\n[[spec]]\nname = \"areka-P0-tick-gate-adoption\"\n\
         wave = \"保留\"\nstage = \"E\"\nowner_count = 0\n```\n"
    );
    let body = err_body(read_roadmap_draft(&silent));
    assert!(body.contains("bundle"), "欄の名前が本文に無い: {body}");
    assert!(
        body.contains("areka-P0-tick-gate-adoption"),
        "どの行かが本文に無い: {body}"
    );
}

/// 段階の欄は束と一蓮托生で、片方だけを書いた行は読めない。
///
/// 置き字を締め出すのはこの 2 本である——束の無い行に段階を書けば落ち、束のある行が
/// 段階を書かなくても落ちる。どちらかが抜けると `stage = "A"` を 14 行に並べた版が
/// また通ってしまう（27 行中 23 行が `A`・本物は 9 行）。
#[test]
fn the_stage_field_follows_the_bundle() {
    let with_stage = format!(
        "{BRIEFS_BLOCK}\n```toml\n[[spec]]\nname = \"areka-P0-tick-gate-adoption\"\n\
         wave = \"保留\"\nstage = \"A\"\nnone = true\nreason = \"どの束にも写らない\"\n\
         owner_count = 0\n```\n"
    );
    let body = err_body(read_roadmap_draft(&with_stage));
    assert!(body.contains("stage"), "欄の名前が本文に無い: {body}");
    assert!(
        body.contains("areka-P0-tick-gate-adoption"),
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

// ---------------------------------------------------------------------------
// 3 種の id の口
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

/// 同じ本文から 2 回読めば同じ値（設計「不変条件」）。
#[test]
fn reading_the_same_text_twice_gives_the_same_value() {
    let markdown = format!("{BUNDLE_BLOCK}\n{TALLY_BLOCK}");
    assert_eq!(
        read_linkage(&markdown).expect("読めるはず"),
        read_linkage(&markdown).expect("読めるはず")
    );
}
