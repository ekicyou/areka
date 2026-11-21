//! 母数の器（[`Floors`]）と、実データの写しの狙った綴り・1 つの欄だけを壊す道具
//! 4 つの較正。
//!
//! 道具そのものは兄弟の `documents.rs` にあり、そこにはテストの本体を 1 つも置かない。
//! 母数の下限そのものの主張は `documents_non_vacuity.rs` にある。2 つに割ってあるのは
//! 1 ファイル 1,000 行の目安を守るためである（`structure.md`）。
//!
//! # ここに置くもの
//!
//! - 母数の下限を集める器（[`Floors`]）の較正——下限を下回れば赤になり、下限 0 は
//!   主張として受け付けない。
//! - spec の一覧の絞り（[`spec_dir_names`]）の例——brief の無い名前は赤にせず落とす。
//! - 引用 id の取り出し（[`cited_ids`]）の較正。
//! - 写しの狙った綴り・1 つの欄だけを壊す道具 4 つの較正——狙ったところだけが変わり、
//!   狙いが無ければ止まる。道具は `&str` を受けて `String` を返すだけで、型の上で repo のファイルには
//!   触れない。

use std::collections::BTreeSet;

use ukadoc_survey::documents::Stage;

use super::documents::{
    COMPLETED_DIR, Documents, Floors, OWN_SPEC_DIR, cited_ids, drop_member, shift_count,
    shift_row_count, spec_dir_names, twist_id, twisted_id,
};

/// 母数の器が、下限を下回った行を**すべて**名指すこと。
#[test]
fn the_floor_container_reports_every_row_that_falls_short() {
    let mut floors = Floors::default();
    floors
        .at_least("足りている数", 5, 1)
        .at_least("足りない数", 0, 1)
        .at_least("もう 1 つ足りない数", 3, 4);

    let short = floors.short();
    assert_eq!(short.len(), 2, "下回った行を数え落としている: {short:?}");
    assert!(
        short[0].contains("足りない数") && short[0].contains('0'),
        "下回った行が何件だったかを名指していない: {short:?}"
    );
    assert!(
        short[1].contains("もう 1 つ足りない数"),
        "2 つ目の違反を名指していない: {short:?}"
    );
    assert!(
        short.iter().all(|line| !line.contains("足りている数")),
        "満たしている行まで挙げている: {short:?}"
    );
}

/// 母数の器が、下限 0（＝何も主張していない）を違反として扱うこと。
#[test]
fn the_floor_container_refuses_a_floor_of_zero() {
    let mut floors = Floors::default();
    floors.at_least("母数 0 でも通る下限", 1_000, 0);

    let short = floors.short();
    assert_eq!(short.len(), 1, "下限 0 を素通りさせている: {short:?}");
    assert!(
        short[0].contains("母数 0 でも通る下限"),
        "下限 0 の行を名指していない: {short:?}"
    );
}

/// 満たしている器は止まらないこと（較正の反対側）。実数が下限ちょうどでも緑である。
#[test]
fn the_floor_container_stays_quiet_when_every_row_is_met() {
    let mut floors = Floors::default();
    floors
        .at_least("足りている数", 2, 1)
        .at_least("ちょうど", 1, 1);
    assert!(
        floors.short().is_empty(),
        "満たしているのに違反を挙げている"
    );
    floors.assert_met();
}

/// spec の一覧の絞りが、brief の無い名前・`completed`・本 spec を赤にせず落とすこと。
///
/// `/kiro-spec-init` などで作ったばかりのフォルダには brief.md が無い。それで全体テストが
/// 赤になってはならず、一覧から黙って落ちればよい。
#[test]
fn the_spec_listing_drops_brief_less_names_without_turning_red() {
    let names = spec_dir_names([
        ("brief-on".to_owned(), true),
        ("brief-less".to_owned(), false),
        (COMPLETED_DIR.to_owned(), true),
        (OWN_SPEC_DIR.to_owned(), true),
    ]);
    assert_eq!(
        names,
        BTreeSet::from(["brief-on".to_owned()]),
        "brief 持ちの名前だけが残っていない"
    );
}

/// 引用 id の取り出しが引用符と逆引用符の両方を拾い、重複を畳むこと。
#[test]
fn cited_ids_takes_both_quoted_and_backticked_spellings() {
    let markdown = "\
本文で `ukadoc:list_shiori_event:OnBoot:1` を指す。

```toml
members = [\"ukadoc:list_shiori_event:OnBoot:1\", \"ukadoc:list_shiori_event:OnClose:1\"]
```
";
    let ids = cited_ids(markdown);
    assert_eq!(
        ids,
        BTreeSet::from([
            "ukadoc:list_shiori_event:OnBoot:1".to_owned(),
            "ukadoc:list_shiori_event:OnClose:1".to_owned(),
        ]),
        "引用符と逆引用符の両方を畳んで拾えていない"
    );
}

/// id を 1 文字変える摂動が、狙った綴りだけを変えること。
#[test]
fn twisting_an_id_changes_exactly_one_character() {
    let id = "ukadoc:list_shiori_event:OnBoot:1";
    let twisted = twisted_id(id);
    assert_eq!(twisted, "ukadoc:list_shiori_event:OnBoot:0");
    assert_eq!(
        twisted.chars().count(),
        id.chars().count(),
        "1 文字変えるつもりが長さが変わっている"
    );
    assert_eq!(
        id.chars()
            .zip(twisted.chars())
            .filter(|(a, b)| a != b)
            .count(),
        1,
        "変わった文字が 1 つでない"
    );
    assert!(
        twisted.starts_with("ukadoc:"),
        "1 文字変えた綴りが id として拾われなくなっている"
    );

    let text = format!("`{id}` と \"{id}\" を書いた本文");
    let broken = twist_id(&text, id);
    assert!(!broken.contains(id), "元の綴りが残っている: {broken}");
    assert_eq!(
        broken.matches(twisted.as_str()).count(),
        2,
        "本文の全出現を差し替えていない: {broken}"
    );
}

/// 構成 id を 1 つ抜く摂動が、狙った id と区切りだけを落とすこと。
#[test]
fn dropping_a_member_removes_one_id_and_its_comma() {
    let one_line = "members = [\"ukadoc:a:X:1\", \"ukadoc:b:Y:1\", \"ukadoc:c:Z:1\"]";
    assert_eq!(
        drop_member(one_line, "ukadoc:b:Y:1"),
        "members = [\"ukadoc:a:X:1\", \"ukadoc:c:Z:1\"]",
        "1 行の配列から真ん中の id と読点を落とせていない"
    );
    assert_eq!(
        drop_member(one_line, "ukadoc:c:Z:1"),
        "members = [\"ukadoc:a:X:1\", \"ukadoc:b:Y:1\"]",
        "末尾の id を落とすと前の読点が残る"
    );

    let many_lines = "members = [\n  \"ukadoc:a:X:1\",\n  \"ukadoc:b:Y:1\",\n]";
    assert_eq!(
        drop_member(many_lines, "ukadoc:b:Y:1"),
        "members = [\n  \"ukadoc:a:X:1\",\n]",
        "行ごとに書かれた配列から 1 行だけ落とせていない"
    );
}

/// 件数を 1 ずらす摂動が、狙った囲みの狙った鍵の数だけを動かすこと。
#[test]
fn shifting_a_count_moves_the_number_by_one() {
    let toml = "[tally]\ntarget = 1749\nsingles = 0\n";
    assert_eq!(
        shift_count(toml, "[tally]", "target"),
        "[tally]\ntarget = 1750\nsingles = 0\n",
        "狙った鍵の数を 1 ずらせていない"
    );
    assert_eq!(
        shift_count(toml, "[tally]", "singles"),
        "[tally]\ntarget = 1749\nsingles = 1\n",
        "0 の欄を 1 ずらせていない"
    );
}

/// 同じ鍵が別の表と、囲みの後ろの地の文にも現れるとき、錨が狙った表の中だけを見ること。
///
/// これがこの道具の要である。錨を本文全体に置くと、`briefing.md` の `[stage.A]`〜
/// `[stage.E]` が同じ鍵を 5 回持つだけで摂動が空振りして止まる。地の文は
/// 囲みの最後の表の後ろに逆引用符なしで `singles = 0` と書いてあり、表の範囲が閉じの
/// ` ``` ` を越えてはみ出すと、鍵が 2 度現れて止まる。
#[test]
fn shifting_a_count_looks_only_inside_the_named_table() {
    let markdown = "\
```toml
[stage.A]
bundles = 7
singles = 0
items = 320

[stage.B]
bundles = 4
singles = 0
items = 96
```

地の文に singles = 0 と書いても、表の欄ではない。
";
    let shifted = shift_count(markdown, "[stage.B]", "singles");
    assert!(
        shifted.contains("[stage.A]\nbundles = 7\nsingles = 0\nitems = 320"),
        "狙っていない段階の欄まで動かしている:\n{shifted}"
    );
    assert!(
        shifted.contains("[stage.B]\nbundles = 4\nsingles = 1\nitems = 96"),
        "狙った段階の欄を動かせていない:\n{shifted}"
    );
    assert!(
        shifted.contains("地の文に singles = 0 と書いても"),
        "地の文まで書き換えている:\n{shifted}"
    );
}

/// 実データの写しの上でも、5 つの段階の囲みを 1 つずつ狙えること。
///
/// `briefing.md` は `singles = ` を最低 5 回持つ。本文全体を錨に
/// する書き方はここで必ず止まる。
#[test]
fn shifting_a_count_reaches_every_stage_block_of_the_real_briefing() {
    let documents = Documents::load();
    for stage in Stage::ALL {
        let table = format!("[stage.{}]", stage.as_key());
        let shifted = shift_count(&documents.briefing_text, &table, "singles");
        assert_ne!(
            shifted, documents.briefing_text,
            "{table} の singles を 1 ずらせていない"
        );
    }
}

/// 表の行の数をずらす摂動が、狙った行の数だけを 1 動かすこと。
///
/// 錨を行頭に置くのがこの道具の要である。全体報告では「縮退」が状態の分布の**行**と
/// テーマ別の状態分布の**見出しの 4 桁目**の両方に現れるので、行頭で絞らないと
/// 「ちょうど 1 行ない」で空振りして止まる。
#[test]
fn shifting_a_row_count_moves_the_number_by_one() {
    let markdown = "\
| 状態 | 件数 |
| --- | ---: |
| 縮退 | 22 |
| 合計 | 1749 |

| テーマ | 実装済み | 語彙のみ | 縮退 | 合計 |
| --- | ---: | ---: | ---: | ---: |
| 装い | 30 | 186 | 2 | 420 |
";
    let shifted = shift_row_count(markdown, "縮退");
    assert_eq!(
        shifted,
        markdown.replace("| 縮退 | 22 |", "| 縮退 | 23 |"),
        "見出しに同じ語がある表で、狙った行の数だけを動かせていない"
    );
    assert_eq!(
        shift_row_count(markdown, "合計"),
        markdown.replace("| 合計 | 1749 |", "| 合計 | 1750 |"),
        "4 桁の数を 1 ずらせていない"
    );
}

/// 狙いが無いのに素通りしないこと（4 つの道具それぞれ）。
#[test]
#[should_panic(expected = "写しに id ukadoc:no:such:1 が無い")]
fn twisting_an_absent_id_stops() {
    twist_id("本文", "ukadoc:no:such:1");
}

#[test]
#[should_panic(expected = "ちょうど 1 度現れない")]
fn dropping_an_absent_member_stops() {
    drop_member("members = []", "ukadoc:no:such:1");
}

#[test]
#[should_panic(expected = "ちょうど 1 度現れない")]
fn shifting_an_absent_count_stops() {
    shift_count("[tally]\ntarget = 1\n", "[tally]", "singles");
}

#[test]
#[should_panic(expected = "ちょうど 1 度現れない")]
fn shifting_a_count_in_an_absent_table_stops() {
    shift_count("[tally]\ntarget = 1\n", "[stage.A]", "target");
}

#[test]
#[should_panic(expected = "ちょうど 1 行ない")]
fn shifting_an_absent_row_count_stops() {
    shift_row_count("| 縮退 | 22 |\n", "未対応");
}

#[test]
#[should_panic(expected = "ちょうど 1 行ない")]
fn shifting_a_row_count_of_a_repeated_label_stops() {
    shift_row_count("| 縮退 | 22 |\n| 縮退 | 23 |\n", "縮退");
}
