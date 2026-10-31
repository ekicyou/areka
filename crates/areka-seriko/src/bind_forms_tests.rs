//! [`apply_bind_directive`] の試験（spec: script-small-tags 要件 3.3・3.8〜3.10・8.1）。
//!
//! 期待値は SSP の測りの 7 行（research.md 2.11）をそのまま使う。カテゴリ「ボール」は定義の順に
//! おじさん・男の子・ギャル・妹（番号は定義と逆に振り、番号の順でなく定義の順に当てることも見る）・
//! 排他・既定なし。「目」は `mustselect`・既定はノーマル。「腕輪」は `multiple`（測っていない形）。

use super::tests::groups_of;
use super::*;
use crate::resolve::SurfaceTarget;
use crate::state::{BindApplyOutcome, ScopeStates};

const SHELL: &str = "charset,UTF-8\r\n\
sakura.bindgroup13.name,ボール,おじさん\r\n\
sakura.bindgroup12.name,ボール,男の子\r\n\
sakura.bindgroup11.name,ボール,ギャル\r\n\
sakura.bindgroup10.name,ボール,妹\r\n\
sakura.bindgroup20.name,目,ノーマル\r\n\
sakura.bindgroup21.name,目,ジト\r\n\
sakura.bindgroup20.default,1\r\n\
sakura.bindoption0.group,目,mustselect\r\n\
sakura.bindgroup30.name,腕輪,金\r\n\
sakura.bindgroup31.name,腕輪,銀\r\n\
sakura.bindoption1.group,腕輪,multiple\r\n";

const OJISAN: u32 = 13;
const OTOKONOKO: u32 = 12;
const IMOUTO: u32 = 10;
const NORMAL: u32 = 20;

/// 既定（目のノーマル）を着て面を出したスコープ 0 と、名前の表。
fn setup() -> (ScopeStates, BindResolver, ActorKey) {
    let groups = groups_of(SHELL);
    let mut states = ScopeStates::new(DefaultBinds::from_groups(&groups));
    let scope = ActorKey::from("0");
    states.apply(&scope, SurfaceTarget::Show(10));
    (states, BindResolver::from_groups(&groups), scope)
}

/// `\![bind,...]` の引数を読んで当てる。
fn run(
    states: &mut ScopeStates,
    resolver: &BindResolver,
    scope: &ActorKey,
    tokens: &[&str],
) -> Vec<AppliedBind> {
    apply_bind_directive(states, resolver, scope, &parse_bind_directive(tokens))
}

/// ボールのパーツを `part` だけ着た状態にする（空ならどれも着ない）。
fn wear_balls(states: &mut ScopeStates, resolver: &BindResolver, scope: &ActorKey, part: &str) {
    run(states, resolver, scope, &["ボール", "", "0"]);
    if !part.is_empty() {
        run(states, resolver, scope, &["ボール", part, "1"]);
    }
}

fn ids(states: &ScopeStates, scope: &ActorKey) -> Vec<u32> {
    states.current_binds(scope).ids().to_vec()
}

/// 当てたパーツの名前と着脱の列。
fn steps(applied: &[AppliedBind]) -> Vec<(&str, bool)> {
    applied.iter().map(|a| (a.part.as_str(), a.on)).collect()
}

/// 測り 1: `,,1`（全部脱いだ後）→ 妹（定義の最後）だけを着る。定義の順に 1 つずつ着せる。
#[test]
fn ssp_row1_category_on_after_bare_leaves_last_part() {
    let (mut states, resolver, scope) = setup();
    wear_balls(&mut states, &resolver, &scope, "");
    let applied = run(&mut states, &resolver, &scope, &["ボール", "", "1"]);
    assert_eq!(
        steps(&applied),
        vec![
            ("おじさん", true),
            ("男の子", true),
            ("ギャル", true),
            ("妹", true)
        ]
    );
    assert!(applied.iter().all(|a| a.category == "ボール"));
    assert_eq!(ids(&states, &scope), vec![IMOUTO, NORMAL]);
}

/// 測り 2: `,,`（全部脱いだ後）→ 妹だけを着る。
#[test]
fn ssp_row2_category_toggle_after_bare_leaves_last_part() {
    let (mut states, resolver, scope) = setup();
    wear_balls(&mut states, &resolver, &scope, "");
    let applied = run(&mut states, &resolver, &scope, &["ボール", "", ""]);
    assert_eq!(
        steps(&applied),
        vec![
            ("おじさん", true),
            ("男の子", true),
            ("ギャル", true),
            ("妹", true)
        ]
    );
    assert_eq!(ids(&states, &scope), vec![IMOUTO, NORMAL]);
}

/// 測り 3: `,,`（おじさんを着た後）→ 妹だけを着る（おじさんは反対で脱ぎ、残りを順に着る）。
#[test]
fn ssp_row3_category_toggle_after_first_part_leaves_last_part() {
    let (mut states, resolver, scope) = setup();
    wear_balls(&mut states, &resolver, &scope, "おじさん");
    assert_eq!(ids(&states, &scope), vec![OJISAN, NORMAL]);
    let applied = run(&mut states, &resolver, &scope, &["ボール", "", ""]);
    assert_eq!(
        steps(&applied),
        vec![
            ("おじさん", false),
            ("男の子", true),
            ("ギャル", true),
            ("妹", true)
        ]
    );
    assert_eq!(ids(&states, &scope), vec![IMOUTO, NORMAL]);
}

/// 測り 4: `,,`（妹を着た後）→ 妹のまま（おじさんが妹を脱がせ、最後に妹を着る）。省略形 `ボール,` も同じ。
#[test]
fn ssp_row4_category_toggle_after_last_part_keeps_last_part() {
    for tokens in [&["ボール", "", ""][..], &["ボール"][..]] {
        let (mut states, resolver, scope) = setup();
        wear_balls(&mut states, &resolver, &scope, "妹");
        let applied = run(&mut states, &resolver, &scope, tokens);
        assert_eq!(
            steps(&applied),
            vec![
                ("おじさん", true),
                ("男の子", true),
                ("ギャル", true),
                ("妹", true)
            ],
            "{tokens:?}"
        );
        assert_eq!(ids(&states, &scope), vec![IMOUTO, NORMAL], "{tokens:?}");
    }
}

/// 測り 5: `目,,0`（`mustselect`・ノーマルを着た後）→ 目はノーマルのまま。各パーツが今どおり外さない。
#[test]
fn ssp_row5_mustselect_category_off_keeps_worn_part() {
    let (mut states, resolver, scope) = setup();
    let applied = run(&mut states, &resolver, &scope, &["目", "", "0"]);
    assert_eq!(steps(&applied), vec![("ノーマル", false), ("ジト", false)]);
    assert_eq!(
        applied[0].result,
        NamedBindResult::MustSelectKept { id: NORMAL }
    );
    assert_eq!(
        applied[1].result,
        NamedBindResult::MustSelectKept { id: 21 }
    );
    assert_eq!(ids(&states, &scope), vec![NORMAL]);
}

/// 測り 6: `ボール,男の子`（妹を着た後）→ 男の子だけを着る（トグルで着せ、排他で妹が脱げる）。
#[test]
fn ssp_row6_part_toggle_wears_and_replaces() {
    let (mut states, resolver, scope) = setup();
    wear_balls(&mut states, &resolver, &scope, "妹");
    let applied = run(&mut states, &resolver, &scope, &["ボール", "男の子"]);
    assert_eq!(steps(&applied), vec![("男の子", true)]);
    assert!(
        matches!(
            applied[0].result,
            NamedBindResult::Applied {
                id: OTOKONOKO,
                outcome: BindApplyOutcome::Changed(_)
            }
        ),
        "{:?}",
        applied[0].result
    );
    assert_eq!(ids(&states, &scope), vec![OTOKONOKO, NORMAL]);
}

/// 測り 7: `ボール,男の子,`（男の子を着た後）→ 何も着ない（トグルで脱ぐ）。
#[test]
fn ssp_row7_part_toggle_takes_off_worn_part() {
    let (mut states, resolver, scope) = setup();
    wear_balls(&mut states, &resolver, &scope, "男の子");
    let applied = run(&mut states, &resolver, &scope, &["ボール", "男の子", ""]);
    assert_eq!(steps(&applied), vec![("男の子", false)]);
    assert_eq!(ids(&states, &scope), vec![NORMAL]);
}

/// パーツ名つきの `1`／`0` は今の当て方を 1 回（要件 3.3）。
#[test]
fn named_form_applies_once() {
    let (mut states, resolver, scope) = setup();
    let applied = run(&mut states, &resolver, &scope, &["ボール", "妹", "1"]);
    assert_eq!(steps(&applied), vec![("妹", true)]);
    assert_eq!(ids(&states, &scope), vec![IMOUTO, NORMAL]);
    let applied = run(&mut states, &resolver, &scope, &["ボール", "妹", "0"]);
    assert_eq!(steps(&applied), vec![("妹", false)]);
    assert_eq!(ids(&states, &scope), vec![NORMAL]);
}

/// `,,0` は排他のカテゴリのパーツを全部脱がせる（要件 3.8）。
#[test]
fn category_off_takes_off_all_parts() {
    let (mut states, resolver, scope) = setup();
    wear_balls(&mut states, &resolver, &scope, "ギャル");
    let applied = run(&mut states, &resolver, &scope, &["ボール", "", "0"]);
    assert_eq!(applied.len(), 4);
    assert!(applied.iter().all(|a| !a.on));
    assert_eq!(ids(&states, &scope), vec![NORMAL]);
}

/// `multiple` の `,,1` は全部着る（要件 3.10・同じ規則から）。
#[test]
fn multiple_category_on_wears_all() {
    let (mut states, resolver, scope) = setup();
    let applied = run(&mut states, &resolver, &scope, &["腕輪", "", "1"]);
    assert_eq!(steps(&applied), vec![("金", true), ("銀", true)]);
    assert_eq!(ids(&states, &scope), vec![NORMAL, 30, 31]);
}

/// `multiple` の `,,` はそれぞれ反対（金だけ着た後 → 銀だけ）。
#[test]
fn multiple_category_toggle_flips_each() {
    let (mut states, resolver, scope) = setup();
    run(&mut states, &resolver, &scope, &["腕輪", "金", "1"]);
    let applied = run(&mut states, &resolver, &scope, &["腕輪", "", ""]);
    assert_eq!(steps(&applied), vec![("金", false), ("銀", true)]);
    assert_eq!(ids(&states, &scope), vec![NORMAL, 31]);
}

/// 名前の無いカテゴリは「見つからない」を 1 つ（空の列を返さない・要件 3.10）。状態は変わらない。
#[test]
fn unknown_category_reports_not_found_once() {
    let (mut states, resolver, scope) = setup();
    for tokens in [
        &["無い", "", "0"][..],
        &["無い", "", "1"][..],
        &["無い"][..],
    ] {
        let applied = run(&mut states, &resolver, &scope, tokens);
        assert_eq!(applied.len(), 1, "{tokens:?}");
        assert_eq!(applied[0].category, "無い");
        assert_eq!(applied[0].result, NamedBindResult::NotFound, "{tokens:?}");
    }
    let applied = run(&mut states, &resolver, &scope, &["ボール", "無い"]);
    assert_eq!(applied.len(), 1);
    assert_eq!(applied[0].result, NamedBindResult::NotFound);
    assert_eq!(ids(&states, &scope), vec![NORMAL]);
}

/// 名前の表の無い（数でない）スコープは「名前の表なし」を 1 つ。どの形でも同じ。
#[test]
fn non_numeric_scope_reports_no_namespace_once() {
    let (mut states, resolver, _) = setup();
    let scope = ActorKey::from("x");
    for tokens in [
        &["ボール", "", "1"][..],
        &["ボール", "", ""][..],
        &["ボール", "妹"][..],
        &["ボール", "妹", "1"][..],
    ] {
        let applied = run(&mut states, &resolver, &scope, tokens);
        assert_eq!(applied.len(), 1, "{tokens:?}");
        assert_eq!(
            applied[0].result,
            NamedBindResult::NoNamespace,
            "{tokens:?}"
        );
    }
    assert_eq!(ids(&states, &ActorKey::from("0")), vec![NORMAL]);
}

/// 壊れた入力は空の列で、状態を変えない。
#[test]
fn malformed_returns_empty_and_keeps_state() {
    let (mut states, resolver, scope) = setup();
    assert!(run(&mut states, &resolver, &scope, &["", "妹", "1"]).is_empty());
    assert!(run(&mut states, &resolver, &scope, &["ボール", "妹", "2"]).is_empty());
    assert_eq!(ids(&states, &scope), vec![NORMAL]);
}
