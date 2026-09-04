//! `content.rs` の在中テスト——**関連・別名・登場版・テーマ**（要件 6.7・6.8・6.13）。
//!
//! ソースの正典 URL と証拠の側は姉妹モジュール `content_tests` が持つ
//! （要件 6.5・6.6）。共用の道具は `content_test_support` にある。主張の作法
//! （入口を通す・壊した対と壊さない対・種類ごとの件数の等式）は `content_tests` の
//! 冒頭に書いたとおりで、このファイルも同じに従う。
//!
//! # ここが背負う 2 つの守り
//!
//! - **「全部を挙げる」**（タスク 4.2 からの申し送り）——`LinkEndpointMissing`・
//!   `IntroducedNotInCatalogVersions`・`UnknownTheme` は違反 2 件の**間に正しい項目を
//!   挟んだ**見本で件数まで主張する。
//! - **台帳を渡す並びに左右されない**（タスク 4.2 からの申し送り・要件 7.3）——
//!   `AliasChain` だけが相手の状態を別の台帳から引くので、並びが結果に漏れうる唯一の
//!   判定である。逆順で 1 度通すテストがそこを釘付けする。

use super::super::{CheckInput, FindingKind, run};
use super::test_support::{
    details, entry_mut, id, ids, kinds, ledger_place, of_kind, only_one, places,
};
use crate::ledger::Ledger;
use crate::lib_test_support::World;
use crate::model::{Domain, EntryId, Link, LinkKind, Status};

// ---------------------------------------------------------------------------
// 関連・別名・後継の相手がカタログに実在するか（要件 6.7）
// ---------------------------------------------------------------------------

/// `links` の相手 id をカタログに無いものへ変えると `LinkEndpointMissing` が出る。
#[test]
fn a_link_target_missing_from_the_catalog_is_a_finding() {
    let mut world = World::normal();
    entry_mut(
        &mut world,
        Domain::Shiori,
        "ukadoc:list_shiori_event:OnBoot:1",
    )
    .links = vec![Link {
        kind: LinkKind::SameFeature,
        to: id("ukadoc:list_shiori_event:OnGhostBooted:1"),
    }];
    world.refresh_reports();

    let outcome = run(&world.input());
    assert_eq!(
        kinds(&outcome.findings),
        vec![(FindingKind::LinkEndpointMissing, 1)]
    );
    let finding = only_one(&outcome.findings, FindingKind::LinkEndpointMissing);
    assert_eq!(
        finding.id.as_ref().map(EntryId::as_str),
        Some("ukadoc:list_shiori_event:OnBoot:1"),
        "主語は相手ではなく、その関連を書いた側の項目"
    );
    assert_eq!(finding.place, ledger_place("shiori"));
    assert_eq!(
        finding.detail,
        "関連 same-feature の相手がカタログに無い: ukadoc:list_shiori_event:OnGhostBooted:1"
    );
}

/// `alias_of` の参照先がカタログに無ければ `LinkEndpointMissing` が出る。
///
/// 指す先が台帳にも無いので `AliasChain` は出ない（状態を引けない相手を「別名だ」と
/// 決めつける実装はここで赤になる）。
#[test]
fn a_missing_alias_of_target_is_a_finding() {
    let mut world = World::normal();
    entry_mut(
        &mut world,
        Domain::Assets,
        "ukadoc:descript_ghost:charset:1",
    )
    .alias_of = Some(id("ukadoc:descript_ghost:charsets:1"));
    world.refresh_reports();

    let outcome = run(&world.input());
    assert_eq!(
        kinds(&outcome.findings),
        vec![(FindingKind::LinkEndpointMissing, 1)]
    );
    let finding = only_one(&outcome.findings, FindingKind::LinkEndpointMissing);
    assert_eq!(
        finding.id.as_ref().map(EntryId::as_str),
        Some("ukadoc:descript_ghost:charset:1")
    );
    assert_eq!(finding.place, ledger_place("assets"));
    assert_eq!(
        finding.detail,
        "alias_of の相手がカタログに無い: ukadoc:descript_ghost:charsets:1"
    );
}

/// `supersedes` の参照先がカタログに無ければ `LinkEndpointMissing` が出る。
#[test]
fn a_missing_supersedes_target_is_a_finding() {
    let mut world = World::normal();
    entry_mut(&mut world, Domain::Assets, "ukadoc:descript_ghost:name:1").supersedes =
        vec![id("ukadoc:descript_ghost:charsets:1")];
    world.refresh_reports();

    let outcome = run(&world.input());
    assert_eq!(
        kinds(&outcome.findings),
        vec![(FindingKind::LinkEndpointMissing, 1)]
    );
    let finding = only_one(&outcome.findings, FindingKind::LinkEndpointMissing);
    assert_eq!(
        finding.id.as_ref().map(EntryId::as_str),
        Some("ukadoc:descript_ghost:name:1")
    );
    assert_eq!(finding.place, ledger_place("assets"));
    assert_eq!(
        finding.detail,
        "supersedes の相手がカタログに無い: ukadoc:descript_ghost:charsets:1"
    );
}

/// 1 つの項目に無い相手が 2 つあれば、**2 件とも**所見になる。
///
/// 要件 6.7 は「関連の**両端**の id が実在すること」を求める。2 つの誤りの**間に
/// 実在する相手を 1 本挟んである**ので、「最初の誤りで打ち切る」も「最初の正しい
/// 相手で打ち切る」も、どちらもここで赤になる。
#[test]
fn every_missing_link_endpoint_is_reported_not_just_the_first() {
    let mut world = World::normal();
    entry_mut(
        &mut world,
        Domain::Shiori,
        "ukadoc:list_shiori_event:OnBoot:1",
    )
    .links = vec![
        Link {
            kind: LinkKind::Triggers,
            to: id("ukadoc:list_shiori_event:OnGhostBooted:1"),
        },
        Link {
            kind: LinkKind::SameFeature,
            to: id("ukadoc:list_shiori_event:OnClose:1"),
        },
        Link {
            kind: LinkKind::Queries,
            to: id("ukadoc:list_propertysystem:system.hour:1"),
        },
    ];
    world.refresh_reports();

    let outcome = run(&world.input());
    assert_eq!(
        kinds(&outcome.findings),
        vec![(FindingKind::LinkEndpointMissing, 2)]
    );
    assert_eq!(
        details(&outcome.findings, FindingKind::LinkEndpointMissing),
        vec![
            "関連 triggers の相手がカタログに無い: ukadoc:list_shiori_event:OnGhostBooted:1",
            "関連 queries の相手がカタログに無い: ukadoc:list_propertysystem:system.hour:1",
        ]
    );
    assert_eq!(
        ids(&outcome.findings, FindingKind::LinkEndpointMissing),
        vec![
            "ukadoc:list_shiori_event:OnBoot:1",
            "ukadoc:list_shiori_event:OnBoot:1",
        ]
    );
}

// ---------------------------------------------------------------------------
// 別名の連鎖の禁止（要件 6.7・2.4）
// ---------------------------------------------------------------------------

/// 別名の指す先を別名にすると `AliasChain` が出る。
///
/// `name` を別名に変えると、`charset` → `name` が別名から別名への写像になる。
///
/// **この見本は「1 段だけを見る」ことを 1 つも守らない。** 手前の `charset` の指す先が
/// すでに別名なので、何段でも辿る実装も 1 段目で同じ所見を出す。辿る段数を釘付けする
/// のは [`the_alias_chain_is_followed_only_one_hop`] の方で、そちらは**手前の指す先が
/// 別名でない**見本を組む（doc が守ると言った主張は、その主張を赤にできるテストと
/// 対でしか置かない）。
#[test]
fn an_alias_pointing_at_another_alias_is_a_finding() {
    let mut world = World::normal();
    let name = entry_mut(&mut world, Domain::Assets, "ukadoc:descript_ghost:name:1");
    name.status = Status::Alias;
    name.alias_of = Some(id("ukadoc:manual_shell"));
    world.refresh_reports();

    let outcome = run(&world.input());
    assert_eq!(kinds(&outcome.findings), vec![(FindingKind::AliasChain, 1)]);
    let finding = only_one(&outcome.findings, FindingKind::AliasChain);
    assert_eq!(
        finding.id.as_ref().map(EntryId::as_str),
        Some("ukadoc:descript_ghost:charset:1"),
        "主語は連鎖の手前側（指した方）"
    );
    assert_eq!(finding.place, ledger_place("assets"));
    assert_eq!(
        finding.detail,
        "alias_of の指す先 ukadoc:descript_ghost:name:1 も別名である"
    );
}

/// 連鎖は**1 段しか辿らない**（`content::check_alias_chain` の doc が言う守り）。
///
/// 見本は 3 つ並べる——`charset`（別名）→ `name`（**別名ではない**）→ `manual_shell`
/// （別名）。1 段だけを見る実装が申し立てるのは `name` の行だけである（`name` の指す先
/// `manual_shell` が別名だから）。**何段でも辿る実装は `charset` の行も申し立てて
/// 2 件になる**ので、ここでだけ赤になる。
///
/// # なぜ読み取りが弾く形をあえて組むのか
///
/// 付録 A.2 は `alias_of` を `status = "alias"` の行にだけ許し、`ledger::read` は
/// それ以外を落とす。だから**正しい台帳では 1 段と多段を原理的に区別できない**
/// ——手前の指す先が別名でなければ、その先には `alias_of` が無くて連鎖がそこで
/// 終わるからである。検査層は `Ledger` の値だけを受け取り、A.2 の整合は読み取りの段が
/// 守る（`check::structure` の `LedgerDomainMismatch` と同じ役割分担）。よって段数の
/// 守りを試せるのは、読み取りを通っていない値を組んだこの形だけである。
#[test]
fn the_alias_chain_is_followed_only_one_hop() {
    let mut world = World::normal();
    // 2 段目は別名ではないが、その先の 3 段目は別名。
    let name = entry_mut(&mut world, Domain::Assets, "ukadoc:descript_ghost:name:1");
    assert_eq!(
        name.status,
        Status::VocabularyOnly,
        "2 段目が別名では 1 段と多段を見分けられない"
    );
    name.alias_of = Some(id("ukadoc:manual_shell"));
    let tail = entry_mut(&mut world, Domain::Assets, "ukadoc:manual_shell");
    tail.status = Status::Alias;
    tail.alias_of = None;
    world.refresh_reports();

    let outcome = run(&world.input());
    assert_eq!(
        kinds(&outcome.findings),
        vec![(FindingKind::AliasChain, 1)],
        "2 件出るなら連鎖を 2 段以上辿っている"
    );
    let finding = only_one(&outcome.findings, FindingKind::AliasChain);
    assert_eq!(
        finding.id.as_ref().map(EntryId::as_str),
        Some("ukadoc:descript_ghost:name:1"),
        "申し立てるのは別名を直に指した行だけ"
    );
    assert_eq!(finding.place, ledger_place("assets"));
    assert_eq!(
        finding.detail,
        "alias_of の指す先 ukadoc:manual_shell も別名である"
    );
}

/// 別名が輪になっていても検査は止まり、行ごとに 1 件ずつ申し立てる。
///
/// この試験が守るのはそこまで——**辿る段数は守らない**。段数の檻は
/// `the_alias_chain_is_followed_only_one_hop` の方で、そちらは摂動で赤を確認してある。
///
/// 輪については、この見本では両方の行が別名なので「この先に別名があるか」を問う実装は
/// 1 段目で答えが出て輪に入らない。実際に段数を数えない実装を当てて走らせたところ、この
/// 試験は緑のまま通った（赤にしたのは段数の檻 1 本だけ）。したがってここは輪を辿る実装を
/// 捕まえる檻ではない。残る危うさは記録として置く: 別名の**終端**まで歩く形へ書き換えると
/// この見本から返ってこなくなる。返ってこない性質はどんな見本でも「きれいな赤」にはでき
/// ないので、`check_alias_chain` を書き換えるときは 1 段で答えを出す形を保つこと。
#[test]
fn a_cycle_of_aliases_terminates_and_is_reported_row_by_row() {
    let mut world = World::normal();
    let name = entry_mut(&mut world, Domain::Assets, "ukadoc:descript_ghost:name:1");
    name.status = Status::Alias;
    name.alias_of = Some(id("ukadoc:descript_ghost:charset:1"));
    world.refresh_reports();

    let outcome = run(&world.input());
    assert_eq!(kinds(&outcome.findings), vec![(FindingKind::AliasChain, 2)]);
    assert_eq!(
        ids(&outcome.findings, FindingKind::AliasChain),
        vec![
            "ukadoc:descript_ghost:charset:1",
            "ukadoc:descript_ghost:name:1",
        ]
    );
    assert_eq!(
        details(&outcome.findings, FindingKind::AliasChain),
        vec![
            "alias_of の指す先 ukadoc:descript_ghost:name:1 も別名である",
            "alias_of の指す先 ukadoc:descript_ghost:charset:1 も別名である",
        ]
    );
}

/// 別名の指す先が**別の台帳**にあっても `AliasChain` が出る。
///
/// 相手の状態を「同じ台帳の中だけ」で引く実装はここでだけ赤になる。見本の別名は
/// 同じ台帳に閉じているので、そちらのテストではこの取り違えが素通りする。
#[test]
fn an_alias_chain_across_two_ledgers_is_a_finding() {
    let mut world = World::normal();
    let spec = entry_mut(&mut world, Domain::Shiori, "ukadoc:spec_shiori3");
    spec.status = Status::Alias;
    spec.alias_of = Some(id("ukadoc:descript_ghost:charset:1"));
    world.refresh_reports();

    let outcome = run(&world.input());
    assert_eq!(kinds(&outcome.findings), vec![(FindingKind::AliasChain, 1)]);
    let finding = only_one(&outcome.findings, FindingKind::AliasChain);
    assert_eq!(
        finding.id.as_ref().map(EntryId::as_str),
        Some("ukadoc:spec_shiori3")
    );
    assert_eq!(
        finding.place,
        ledger_place("shiori"),
        "場所は連鎖を書いた側の台帳"
    );
    assert_eq!(
        finding.detail,
        "alias_of の指す先 ukadoc:descript_ghost:charset:1 も別名である"
    );
}

/// 台帳を渡す並びを変えても、所見は同じことを言う（要件 7.3）。
///
/// `World::normal()` は台帳を [`Domain::ALL`] の順で作り、`Domain` の導出 `Ord` は
/// 宣言順なので**同じ並び**になる。だから「渡された順に左右されない」という防御は、
/// 同じ入力を 2 回流すテストでは原理的に見えない（タスク 4.2 からの申し送り）。
/// 別名の相手を別の台帳へ置いた見本を**逆順**で 1 度通すことだけが釘付けする。
#[test]
fn the_order_of_the_ledgers_does_not_change_the_content_findings() {
    let mut world = World::normal();
    let spec = entry_mut(&mut world, Domain::Shiori, "ukadoc:spec_shiori3");
    spec.status = Status::Alias;
    spec.alias_of = Some(id("ukadoc:descript_ghost:charset:1"));
    world.refresh_reports();

    let reversed: Vec<Ledger> = world.ledgers.iter().rev().cloned().collect();
    assert_ne!(
        reversed[0].domain, world.ledgers[0].domain,
        "並びを変えていなければ試験にならない"
    );

    let forward = run(&world.input()).findings;
    let backward = run(&CheckInput {
        ledgers: &reversed,
        ..world.input()
    })
    .findings;

    let expected = vec!["alias_of の指す先 ukadoc:descript_ghost:charset:1 も別名である"];
    assert_eq!(details(&forward, FindingKind::AliasChain), expected);
    assert_eq!(
        details(&backward, FindingKind::AliasChain),
        expected,
        "台帳を渡す並びが判定に漏れている"
    );
    assert_eq!(kinds(&forward), kinds(&backward));
}

/// 別名が別名でない先を指しているのは所見にならない（対の片方）。
///
/// 見本の `charset` → `name` がまさにその形で、`name` の状態は `vocabulary-only`。
/// `alias_of` を持つだけで赤にする実装はここで赤になる。
#[test]
fn an_alias_pointing_at_a_non_alias_is_not_a_finding() {
    let world = World::normal();
    let charset = world.ledgers[1]
        .entries
        .get(&id("ukadoc:descript_ghost:charset:1"))
        .expect("見本の assets 台帳に charset があるはず");
    assert_eq!(charset.status, Status::Alias);
    assert_eq!(
        charset.alias_of.as_ref().map(EntryId::as_str),
        Some("ukadoc:descript_ghost:name:1"),
        "別名の行が無ければ試験にならない"
    );

    let outcome = run(&world.input());
    assert_eq!(kinds(&outcome.findings), vec![]);
}

// ---------------------------------------------------------------------------
// 登場版がカタログの版番号と矛盾しないか（要件 6.7）
// ---------------------------------------------------------------------------

/// 登場版をカタログの版番号の外へ動かすと `IntroducedNotInCatalogVersions` が出る。
#[test]
fn an_introduced_version_outside_the_catalog_versions_is_a_finding() {
    let mut world = World::normal();
    entry_mut(
        &mut world,
        Domain::Shiori,
        "ukadoc:list_shiori_event:OnBoot:1",
    )
    .introduced = "2.4.00".to_owned();
    world.refresh_reports();

    let outcome = run(&world.input());
    assert_eq!(
        kinds(&outcome.findings),
        vec![(FindingKind::IntroducedNotInCatalogVersions, 1)]
    );
    let finding = only_one(
        &outcome.findings,
        FindingKind::IntroducedNotInCatalogVersions,
    );
    assert_eq!(
        finding.id.as_ref().map(EntryId::as_str),
        Some("ukadoc:list_shiori_event:OnBoot:1")
    );
    assert_eq!(finding.place, ledger_place("shiori"));
    assert_eq!(
        finding.detail,
        "登場版 2.4.00 がカタログの版番号に無い: 2.3.53"
    );
}

/// 登場版の突き合わせは**完全一致**である。版番号の頭だけを書いた `2.3` は所見になる。
///
/// これは机上の話ではない。ドメイン別報告は `introduced` の**先頭 2 節**を SSP 世代と
/// して並べる（要件 7.1。`2.3.53` → `2.3`）ので、`2.3` は台帳を書く人が素直に打ち込む
/// 綴りである。前方一致で拾う実装はそれを黙って受け入れ、以後どの版で入ったのかが
/// 判らなくなる。`2.4.00` を使う前のテストは先頭の節から違うので、完全一致と前方一致を
/// 見分けられない。テーマ名について同じ穴を塞いだ
/// [`a_theme_name_that_merely_prefixes_a_real_theme_is_a_finding`] の対である。
#[test]
fn a_prefix_of_a_catalog_version_is_not_a_match() {
    let mut world = World::normal();
    let target = id("ukadoc:list_propertysystem:system.year:1");
    assert_eq!(
        world
            .catalog
            .entries
            .get(&target)
            .expect("見本のカタログに system.year があるはず")
            .versions,
        vec!["2.3.53"],
        "カタログ側の綴りが前方一致の相手でなければ試験にならない"
    );
    entry_mut(
        &mut world,
        Domain::Property,
        "ukadoc:list_propertysystem:system.year:1",
    )
    .introduced = "2.3".to_owned();
    world.refresh_reports();

    let outcome = run(&world.input());
    assert_eq!(
        kinds(&outcome.findings),
        vec![(FindingKind::IntroducedNotInCatalogVersions, 1)],
        "前方一致で拾う実装はここで所見を 1 件も出さない"
    );
    let finding = only_one(
        &outcome.findings,
        FindingKind::IntroducedNotInCatalogVersions,
    );
    assert_eq!(
        finding.id.as_ref().map(EntryId::as_str),
        Some("ukadoc:list_propertysystem:system.year:1")
    );
    assert_eq!(finding.place, ledger_place("property"));
    assert_eq!(
        finding.detail,
        "登場版 2.3 がカタログの版番号に無い: 2.3.53"
    );
}

/// カタログの版番号が 2 つあるとき、そのどちらかに一致していれば所見にならない。
///
/// 先頭の 1 つだけと比べる実装はここで赤になる。`OnClose` のカタログ側は
/// `2.3.53`・`2.5.60` の 2 つで、台帳は後ろの `2.5.60` を書いている。
#[test]
fn matching_any_of_the_catalog_versions_is_enough() {
    let world = World::normal();
    let catalog = world
        .catalog
        .entries
        .get(&id("ukadoc:list_shiori_event:OnClose:1"))
        .expect("見本のカタログに OnClose があるはず");
    assert_eq!(catalog.versions, vec!["2.3.53", "2.5.60"]);
    let ledger = world.ledgers[0]
        .entries
        .get(&id("ukadoc:list_shiori_event:OnClose:1"))
        .expect("見本の shiori 台帳に OnClose があるはず");
    assert_eq!(
        ledger.introduced, "2.5.60",
        "先頭でない側でなければ試験にならない"
    );

    let outcome = run(&world.input());
    assert_eq!(kinds(&outcome.findings), vec![]);
}

/// カタログの版番号が空なら、台帳の登場版は何であっても見ない（設計の判定表）。
///
/// この番人を落とすと、版番号を 1 つも持たないカタログ項目に登場版を書いた行が
/// すべて赤くなる。
#[test]
fn an_empty_catalog_version_list_is_not_checked() {
    let mut world = World::normal();
    let target = id("ukadoc:list_propertysystem:currentghost.name:1");
    assert!(
        world
            .catalog
            .entries
            .get(&target)
            .expect("見本のカタログにその id があるはず")
            .versions
            .is_empty(),
        "カタログ側が空でなければ番人を試したことにならない"
    );
    entry_mut(
        &mut world,
        Domain::Property,
        "ukadoc:list_propertysystem:currentghost.name:1",
    )
    .introduced = "2.9.99".to_owned();
    world.refresh_reports();

    let outcome = run(&world.input());
    assert_eq!(kinds(&outcome.findings), vec![]);
}

/// 台帳の登場版が空なら、カタログに版番号があっても見ない（設計の判定表）。
///
/// 空文字は「世代不明」であって最古ではない（要件 4.2）。この番人を落とすと、まだ
/// 版を調べていない行がすべて赤くなる。見本には既にその形（`system.month`）が 1 つ
/// あるので、もう 1 つ**書かれていた登場版を消す**向きの摂動を重ねて確かめる。
#[test]
fn an_empty_introduced_is_not_checked() {
    let mut world = World::normal();
    let month = world.ledgers[3]
        .entries
        .get(&id("ukadoc:list_propertysystem:system.month:1"))
        .expect("見本の property 台帳に system.month があるはず");
    assert!(
        month.introduced.is_empty(),
        "空の登場版が無ければ試験にならない"
    );
    assert!(
        !world
            .catalog
            .entries
            .get(&id("ukadoc:list_propertysystem:system.month:1"))
            .expect("見本のカタログにその id があるはず")
            .versions
            .is_empty(),
        "カタログ側が空では番人の違いが見えない"
    );

    entry_mut(
        &mut world,
        Domain::SakuraScript,
        "ukadoc:list_sakura_script:_5c_65:1",
    )
    .introduced = String::new();
    world.refresh_reports();

    let outcome = run(&world.input());
    assert_eq!(kinds(&outcome.findings), vec![]);
}

/// 矛盾した登場版が 2 つあれば、**2 件とも**所見になる。
///
/// 2 つの違反（shiori の `OnBoot`・sakura-script の `\e`）の**間に、版番号と一致した
/// 登場版**（shiori の `OnClose`）が挟まっている。
#[test]
fn every_conflicting_introduced_is_reported_not_just_the_first() {
    let mut world = World::normal();
    entry_mut(
        &mut world,
        Domain::Shiori,
        "ukadoc:list_shiori_event:OnBoot:1",
    )
    .introduced = "2.4.00".to_owned();
    entry_mut(
        &mut world,
        Domain::SakuraScript,
        "ukadoc:list_sakura_script:_5c_65:1",
    )
    .introduced = "2.9.99".to_owned();
    world.refresh_reports();

    let outcome = run(&world.input());
    assert_eq!(
        kinds(&outcome.findings),
        vec![(FindingKind::IntroducedNotInCatalogVersions, 2)]
    );
    assert_eq!(
        details(
            &outcome.findings,
            FindingKind::IntroducedNotInCatalogVersions
        ),
        vec![
            "登場版 2.4.00 がカタログの版番号に無い: 2.3.53",
            "登場版 2.9.99 がカタログの版番号に無い: 2.5.60",
        ]
    );
    assert_eq!(
        places(
            &outcome.findings,
            FindingKind::IntroducedNotInCatalogVersions
        ),
        vec![ledger_place("shiori"), ledger_place("sakura-script")]
    );
}

// ---------------------------------------------------------------------------
// テーマ名がテーマ定義に実在するか（要件 6.8）
// ---------------------------------------------------------------------------

/// テーマ名を 1 文字変えると `UnknownTheme` が 1 件だけ出る。
#[test]
fn a_one_character_change_in_a_theme_name_is_a_finding() {
    let mut world = World::normal();
    entry_mut(
        &mut world,
        Domain::Shiori,
        "ukadoc:list_shiori_event:OnBoot:1",
    )
    .values = vec!["気酉".to_owned()];
    world.refresh_reports();

    let outcome = run(&world.input());
    assert_eq!(
        kinds(&outcome.findings),
        vec![(FindingKind::UnknownTheme, 1)]
    );
    let finding = only_one(&outcome.findings, FindingKind::UnknownTheme);
    assert_eq!(
        finding.id.as_ref().map(EntryId::as_str),
        Some("ukadoc:list_shiori_event:OnBoot:1")
    );
    assert_eq!(finding.place, ledger_place("shiori"));
    assert_eq!(finding.detail, "テーマ定義に無いテーマ名: 気酉");
}

/// テーマ名の突き合わせは**完全一致**である。
///
/// 「触れ合い」の頭だけを書いた「触れ合」は所見になる。部分一致で拾う実装は
/// ここで赤になる（8 つのうち「気配」と「気配り」は片方が他方の接頭辞なので、
/// 緩めると別のテーマを取り違える）。
#[test]
fn a_theme_name_that_merely_prefixes_a_real_theme_is_a_finding() {
    let mut world = World::normal();
    entry_mut(
        &mut world,
        Domain::SakuraScript,
        "ukadoc:list_sakura_script:_5c_73_5bID_5d:1",
    )
    .values = vec!["触れ合".to_owned()];
    world.refresh_reports();

    let outcome = run(&world.input());
    assert_eq!(
        kinds(&outcome.findings),
        vec![(FindingKind::UnknownTheme, 1)]
    );
    assert_eq!(
        only_one(&outcome.findings, FindingKind::UnknownTheme).detail,
        "テーマ定義に無いテーマ名: 触れ合"
    );
}

/// 1 つの項目に知らないテーマ名が 2 つあれば、**2 件とも**所見になる。
///
/// 要件 6.8 は「台帳に書かれたテーマ名が」——つまり全部が——定義に実在することを
/// 求める。2 つの誤りの**間に正しいテーマ名を 1 つ挟んである**ので、項目ごとに
/// 1 件だけ報告する実装も、最初の正しい名前で打ち切る実装も、ここで赤になる。
#[test]
fn every_unknown_theme_in_one_entry_is_reported_not_just_the_first() {
    let mut world = World::normal();
    entry_mut(
        &mut world,
        Domain::Shiori,
        "ukadoc:list_shiori_event:OnClose:1",
    )
    .values = vec!["気酉".to_owned(), "更新".to_owned(), "更辛".to_owned()];
    world.refresh_reports();

    let outcome = run(&world.input());
    assert_eq!(
        kinds(&outcome.findings),
        vec![(FindingKind::UnknownTheme, 2)]
    );
    assert_eq!(
        details(&outcome.findings, FindingKind::UnknownTheme),
        vec![
            "テーマ定義に無いテーマ名: 気酉",
            "テーマ定義に無いテーマ名: 更辛",
        ]
    );
    assert_eq!(
        ids(&outcome.findings, FindingKind::UnknownTheme),
        vec![
            "ukadoc:list_shiori_event:OnClose:1",
            "ukadoc:list_shiori_event:OnClose:1",
        ]
    );
}

/// テーマ名は渡されたテーマ定義に照らす（凍結された 8 つを実装に埋め込まない）。
///
/// 定義の側から「気配」を外すと、それを書いている 2 つの項目が所見になる。
/// `THEMES` を直に見る実装はここで赤になる。
#[test]
fn the_theme_vocabulary_comes_from_the_input_not_from_a_constant() {
    let world = World::normal();
    let narrowed: [&str; 7] = [
        "触れ合い",
        "掛け合い",
        "装い",
        "記憶",
        "交わり",
        "気配り",
        "更新",
    ];
    let outcome = run(&CheckInput {
        themes: &narrowed,
        ..world.input()
    });

    assert_eq!(
        kinds(&outcome.findings),
        vec![(FindingKind::UnknownTheme, 2)]
    );
    assert_eq!(
        ids(&outcome.findings, FindingKind::UnknownTheme),
        vec![
            "ukadoc:list_shiori_event:OnBoot:1",
            "ukadoc:list_shiori_event:OnClose:1",
        ]
    );
    for finding in of_kind(&outcome.findings, FindingKind::UnknownTheme) {
        assert_eq!(finding.detail, "テーマ定義に無いテーマ名: 気配");
        assert_eq!(finding.place, ledger_place("shiori"));
    }
}

// ---------------------------------------------------------------------------
// 1 件目で止めない・同じ入力なら同じ結果（設計 Error Handling・要件 7.3）
// ---------------------------------------------------------------------------

/// 別々の種類を 2 か所同時に壊すと 2 件とも出る（1 件目で止めない）。
#[test]
fn two_breakages_of_different_kinds_produce_both_findings() {
    let mut world = World::normal();
    entry_mut(
        &mut world,
        Domain::Shiori,
        "ukadoc:list_shiori_event:OnBoot:1",
    )
    .introduced = "2.4.00".to_owned();
    entry_mut(
        &mut world,
        Domain::Property,
        "ukadoc:list_propertysystem:system.year:1",
    )
    .values = vec!["記憶り".to_owned()];
    world.refresh_reports();

    let outcome = run(&world.input());
    assert_eq!(
        kinds(&outcome.findings),
        vec![
            (FindingKind::IntroducedNotInCatalogVersions, 1),
            (FindingKind::UnknownTheme, 1),
        ]
    );
    assert_eq!(
        only_one(
            &outcome.findings,
            FindingKind::IntroducedNotInCatalogVersions
        )
        .place,
        ledger_place("shiori")
    );
    assert_eq!(
        only_one(&outcome.findings, FindingKind::UnknownTheme).place,
        ledger_place("property")
    );
}

/// 同じ入力を 2 度通すと所見は 1 件ずつ同じ並びで返る（要件 7.3 の決まり方）。
#[test]
fn the_same_input_yields_the_same_content_findings_in_the_same_order() {
    let mut world = World::normal();
    entry_mut(
        &mut world,
        Domain::Shiori,
        "ukadoc:list_shiori_event:OnBoot:1",
    )
    .introduced = "2.4.00".to_owned();
    entry_mut(
        &mut world,
        Domain::Shiori,
        "ukadoc:list_shiori_event:OnClose:1",
    )
    .values = vec!["気酉".to_owned(), "更辛".to_owned()];
    entry_mut(&mut world, Domain::Assets, "ukadoc:descript_ghost:name:1").supersedes =
        vec![id("ukadoc:descript_ghost:charsets:1")];
    world.refresh_reports();

    let first = run(&world.input()).findings;
    let second = run(&world.input()).findings;
    assert_eq!(first, second);
    assert!(first.len() >= 4, "所見が少なすぎて並びを見比べられない");
}
