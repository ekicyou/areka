//! シェルの切り替え専用の口 [`ZOrderGroupLedger::rebase_descript`] と、据え損ねた基底の
//! 寿命の檻（`stay-on-top-directives` 要件 5.1／5.2／5.5／7.4・task 4.1）。
//!
//! 台帳の状態遷移の檻（`t_zgl*`）と分けて接頭辞を `t_zrb*` にする。実機・World を
//! 一切必要とせず、台帳は各テストが自前で作る値である。

use super::*;

/// バルーン窓の要素。
fn b(scope: u32) -> GroupElement {
    GroupElement {
        scope,
        kind: GroupWindowKind::Balloon,
    }
}

/// キャラ窓の要素。
fn s(scope: u32) -> GroupElement {
    GroupElement {
        scope,
        kind: GroupWindowKind::Char,
    }
}

/// 解釈を通った要素列（台帳の入口の呼び出し規約どおり）。
fn accept(tokens: &[&str]) -> Vec<GroupElement> {
    match parse_zorder_tokens(tokens) {
        Ok((elements, _)) => elements,
        Err(reject) => panic!("受理されるべき指定が解釈で拒否された: {tokens:?} reject={reject:?}"),
    }
}

fn add_tag(ledger: &mut ZOrderGroupLedger, tokens: &[&str]) -> u32 {
    match ledger.try_add_tag_group(accept(tokens)) {
        Ok(id) => id,
        Err(reject) => panic!("受理されるべきタグが拒否された: {tokens:?} reject={reject:?}"),
    }
}

/// 基底（ID）とタグの組（ID）を据えた台帳。基底はスコープ 0・1、タグはスコープ 2・3。
fn ledger_with_base_and_tag() -> (ZOrderGroupLedger, u32, u32) {
    let mut ledger = ZOrderGroupLedger::default();
    ledger.set_descript_base(accept(&["0", "1"]));
    let base_id = ledger.groups()[0].id;
    let tag_id = add_tag(&mut ledger, &["2", "3"]);
    (ledger, base_id, tag_id)
}

fn sources(ledger: &ZOrderGroupLedger) -> Vec<GroupSource> {
    ledger.groups().iter().map(|group| group.source).collect()
}

/// 台本の組を保ち、古い基底を外し、新しい基底を先頭に新しい ID で据える（要件 5.1）。
#[test]
fn t_zrb1_rebase_keeps_tag_groups_and_seats_the_new_base_first() {
    let (mut ledger, old_base, tag_id) = ledger_with_base_and_tag();
    let tag_members = ledger.groups()[1].members.clone();

    let outcome = ledger.rebase_descript(accept(&["5", "0"]));

    assert_eq!(outcome.kept_tag_ids, vec![tag_id]);
    assert_eq!(outcome.dropped_base_id, Some(old_base));
    let seated = outcome.seated_base_id.expect("新しい基底が据わる");
    assert!(
        seated != old_base && seated != tag_id,
        "据え直しには新しい ID"
    );
    assert!(outcome.refused_scopes.is_empty());

    assert_eq!(
        sources(&ledger),
        vec![GroupSource::Descript, GroupSource::Tag]
    );
    assert_eq!(ledger.groups()[0].id, seated);
    assert_eq!(ledger.groups()[0].members, vec![b(5), s(5), b(0), s(0)]);
    assert_eq!(ledger.groups()[1].id, tag_id);
    assert_eq!(
        ledger.groups()[1].members,
        tag_members,
        "台本の組は手つかず"
    );
    assert_eq!(ledger.pending_base, None);
}

/// 基底が無かった台帳でも新しい基底は先頭に入る（台本の組の前）。
#[test]
fn t_zrb2_rebase_without_an_old_base_inserts_the_base_ahead_of_tags() {
    let mut ledger = ZOrderGroupLedger::default();
    let first = add_tag(&mut ledger, &["2", "3"]);
    let second = add_tag(&mut ledger, &["4", "5"]);

    let outcome = ledger.rebase_descript(accept(&["0", "1"]));

    assert_eq!(outcome.kept_tag_ids, vec![first, second], "追加の順");
    assert_eq!(outcome.dropped_base_id, None);
    assert!(outcome.seated_base_id.is_some());
    assert_eq!(
        sources(&ledger),
        vec![GroupSource::Descript, GroupSource::Tag, GroupSource::Tag]
    );
}

/// 台本の組とスコープが重なる基底は据えず、重なったスコープを返し、据え損ねた基底として持つ（要件 5.2）。
#[test]
fn t_zrb3_a_base_overlapping_a_tag_group_is_not_seated() {
    let (mut ledger, old_base, tag_id) = ledger_with_base_and_tag();

    let outcome = ledger.rebase_descript(accept(&["3", "6", "2"]));

    assert_eq!(outcome.kept_tag_ids, vec![tag_id]);
    assert_eq!(outcome.dropped_base_id, Some(old_base), "古い基底は外れる");
    assert_eq!(outcome.seated_base_id, None);
    assert_eq!(
        outcome.refused_scopes,
        vec![3, 2],
        "要素列に現れた順で 1 度ずつ"
    );

    assert_eq!(sources(&ledger), vec![GroupSource::Tag], "基底は無い");
    assert_eq!(ledger.pending_base, Some(accept(&["3", "6", "2"])));
}

/// 古い基底のスコープは重なりに数えない（取り替えられる側だから）。
#[test]
fn t_zrb4_the_old_base_does_not_collide_with_its_replacement() {
    let (mut ledger, _, _) = ledger_with_base_and_tag();

    let outcome = ledger.rebase_descript(accept(&["1", "0"]));

    assert!(outcome.refused_scopes.is_empty());
    assert!(outcome.seated_base_id.is_some());
}

/// 空の列は基底なし。台本の組は残る。
#[test]
fn t_zrb5_an_empty_list_means_no_base() {
    let (mut ledger, old_base, tag_id) = ledger_with_base_and_tag();

    let outcome = ledger.rebase_descript(Vec::new());

    assert_eq!(
        outcome,
        RebaseOutcome {
            kept_tag_ids: vec![tag_id],
            dropped_base_id: Some(old_base),
            seated_base_id: None,
            refused_scopes: Vec::new(),
        }
    );
    assert_eq!(sources(&ledger), vec![GroupSource::Tag]);
    assert_eq!(ledger.pending_base, None);
}

/// 切り替えの後の `\![reset,zorder]` は台本の組を落とし、据え損ねた基底を先頭に据える（要件 5.5）。
#[test]
fn t_zrb6_reset_seats_the_pending_base() {
    let (mut ledger, _, tag_id) = ledger_with_base_and_tag();
    ledger.rebase_descript(accept(&["2", "6"]));
    let next_tag = add_tag(&mut ledger, &["7", "8"]);

    ledger.reset_to_descript();

    assert_eq!(sources(&ledger), vec![GroupSource::Descript]);
    let seated = &ledger.groups()[0];
    assert_eq!(seated.members, vec![b(2), s(2), b(6), s(6)]);
    assert!(
        seated.id != tag_id && seated.id != next_tag,
        "新しい ID で据わる"
    );
    assert_eq!(ledger.pending_base, None, "据わったら持たない");

    // 据わった基底は以後の reset でも残る（ふつうの基底と同じ）。
    let version = ledger.version();
    ledger.reset_to_descript();
    assert_eq!(sources(&ledger), vec![GroupSource::Descript]);
    assert_eq!(ledger.version(), version);
}

/// 起動の基底の口は据え損ねた基底を消してから始める。
#[test]
fn t_zrb7_the_startup_base_clears_the_pending_base() {
    let (mut ledger, _, _) = ledger_with_base_and_tag();
    ledger.rebase_descript(accept(&["2", "6"]));
    assert!(ledger.pending_base.is_some());

    ledger.set_descript_base(Vec::new());
    assert_eq!(ledger.pending_base, None);

    ledger.reset_to_descript();
    assert!(
        ledger.groups().is_empty(),
        "消えた基底は reset でも据わらない"
    );
}

/// 切り替えの口も据え損ねた基底を消してから始める（前の切り替えの残りを持ち越さない）。
#[test]
fn t_zrb8_a_later_rebase_clears_the_pending_base() {
    let (mut ledger, _, _) = ledger_with_base_and_tag();
    ledger.rebase_descript(accept(&["2", "6"]));

    let outcome = ledger.rebase_descript(Vec::new());
    assert_eq!(outcome.dropped_base_id, None);
    assert_eq!(ledger.pending_base, None);

    ledger.reset_to_descript();
    assert!(ledger.groups().is_empty());
}

/// 版は内容が動いたときだけ進む。進む経路と進まない経路を同じ 1 本で挟む。
#[test]
fn t_zrb9_version_moves_only_when_the_contents_move() {
    // 進む: 基底の取り替え・基底を外すだけ・基底を据えるだけ・reset で据え損ねた基底が据わる。
    let (mut ledger, _, _) = ledger_with_base_and_tag();
    let v = ledger.version();
    ledger.rebase_descript(accept(&["0", "1"]));
    assert_eq!(ledger.version(), v + 1, "同じ要素列でも据え直しは動き");

    let v = ledger.version();
    ledger.rebase_descript(Vec::new());
    assert_eq!(ledger.version(), v + 1, "基底を外した");

    let v = ledger.version();
    ledger.rebase_descript(accept(&["0", "1"]));
    assert_eq!(ledger.version(), v + 1, "基底を据えた");

    let v = ledger.version();
    ledger.rebase_descript(accept(&["2", "9"]));
    assert_eq!(
        ledger.version(),
        v + 1,
        "古い基底を外して新しいのは据え損ねた"
    );

    // 進まない: 基底が無く据えるものも無い・基底が無く重なって据え損ねた。
    let v = ledger.version();
    ledger.rebase_descript(Vec::new());
    assert_eq!(ledger.version(), v, "何も載らず何も外れない");
    ledger.rebase_descript(accept(&["3", "9"]));
    assert_eq!(ledger.version(), v, "据え損ねただけでは組は動かない");

    // reset: 台本の組を落として据え損ねた基底を据える＝進む。
    ledger.reset_to_descript();
    assert_eq!(ledger.version(), v + 1);
}
