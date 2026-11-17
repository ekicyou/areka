//! 3 文書まわりの判定の母数が 0 件でないことの主張
//! （要件 11.3・11.4・設計「入口 / `tests/consistency`」）。
//!
//! # ここに置くもの
//!
//! - 読み込み（[`Documents::load`]）が 3 文書・全体報告・spec ディレクトリの一覧に
//!   届いていること。
//!
//! - 判定 ⑴ ⑵ の母数の下限（`linkage.md` の引用 id 数・報告 5 本から読めた機械の
//!   束 id の数・`linkage.md` が引用した機械の束 id の数）。
//! - 判定 ⑶ の母数の下限（名前付き束の数・人手で足した id の数）。
//! - 判定 ⑷ の母数の下限（`[[rank]]` の行数・`[[barrier]]` の行数・段階 A の `items`）。
//! - 判定 ⑸ の母数の下限（`[[spec]]` の行数・`[[owner_completed]]` の行数・
//!   `completed/` の直下の数）。
//! - 判定 ⑹ の母数の下限（作り直した全体報告の本文の長さ・その束の表の行数）。
//!
//! 下限を下回れば赤になること（下限 0 は主張として受け付けない・実数と等しければ緑）は
//! 母数の器（[`Floors`]）の側が持つ。器と壊す道具 4 つの**較正**は兄弟の
//! `documents_tools.rs` にある（1 ファイル 1,000 行の目安・`structure.md`）。
//!
//! # 判定 ⑴ の下限は `linkage.md` と `briefing.md` の 2 本ぶん
//!
//! `roadmap-draft.md` は spec 名と束名だけを書き、項目 id を引用しないので、判定 ⑴ の
//! 下限はこの文書ぶんを置かない。
//!
//! # spec ディレクトリの数は「下限」であって「一致」ではない
//!
//! 生きたディレクトリの総数との一致を主張すると、他 spec の起票・完了のたびに赤に
//! なり、この一群が全 spec の共有ファイルになる（設計「⑸ の数え方」）。だから下限
//! だけを置く。

use ukadoc_survey::documents::Stage;
use ukadoc_survey::io::paths;
use ukadoc_survey::model::{Domain, THEMES};
use ukadoc_survey::report::summary::render_summary;

use super::RepoData;
use super::documents::{Documents, Floors, OWN_SPEC_DIR, cited_ids};
use super::documents_checks::{bundle_ids_in_report, linkage_bundle_ids, report_bundle_ids};

/// `.kiro/specs/` の直下にある brief 持ちディレクトリの数の下限。
///
/// 本 spec 自身は数えない。実数を釘付けすると他 spec の起票・
/// 完了で赤になるので下限にする。狙いは「一覧が空を返す・別のディレクトリを見ている」
/// といった**ほとんど何も残らない**壊れ方を捕まえることで、1 本 2 本の増減は見ない。
const MIN_SPEC_DIRS: usize = 10;

/// `.kiro/specs/completed/` の直下のディレクトリ数の下限（設計 判定 ⑸ の母数）。
///
/// 数えるのは `completed/` 直下の**ディレクトリ**だけで（`Documents` の読みの
/// `child_dirs` がディレクトリでない項目を落とす）、直下の単独の `.md` は数えない。
/// 完了した spec は減らないので下限として安定している。
const MIN_COMPLETED_SPECS: usize = 100;

/// 判定 ⑸ の母数——`roadmap-draft.md` の `[[spec]]` の行数の下限。
///
/// 行数は `[briefs].count` と一致する（腕 a が両者を突き合わせる）。下限を
/// 設計「判定の一覧」⑸ のとおり 20 に置くのは、表が着手時の写真であって生きた
/// 総数を追わないからである——他 spec の完了で表から行が減ることはあり、実数を
/// 釘付けするとそのたびに赤くなる。捕まえたいのは「表が空のまま `count = 0` と
/// 書いて全部の腕が恒真になる」壊れ方である。
const MIN_SPEC_ROWS: usize = 20;

/// 判定 ⑸ の母数——`briefing.md` の `[[owner_completed]]` の行数の下限。
///
/// 各行の `items` は腕 e が 1 行ずつ数え直す。設計「判定の一覧」⑸ のとおり
/// 「1 行以上」にする。完了済み spec を宛先に残す行は、宛先の整理（要件 7.4）が
/// 進むと増減するので実数は釘付けしない。0 行に落ちると腕 e は数える相手を失って
/// **恒真になる**（腕 f のほうは逆に、行き先を失った宛先を全部挙げて騒がしくなる）。
const MIN_OWNER_COMPLETED_ROWS: usize = 1;

/// 判定 ⑴ の母数——`linkage.md` の引用 id 数の下限。
///
/// 下限は `[tally].target`＝対象 4 状態の全数に合わせる（設計「判定の一覧」⑴）。
/// 束の `members` が対象を過不足なく並べる以上、`linkage.md` の引用 id はこの数を
/// 下回れない。地の文だけに現れる id を足して釘付けしないのは、それが本文の書き方
/// 次第で増減する余りだからである。項目を対象外へ移して `[tally].target` が減ったら、
/// この下限も同じ数へ下げる。
const MIN_LINKAGE_CITED_IDS: usize = 1_550;

/// 判定 ⑴ の母数——`briefing.md` の引用 id 数の下限。
///
/// **数え方は [`cited_ids`]（引用符付きと逆引用符付きの和集合・重複は畳む）**で、
/// 実数はこの注釈に写さない。`briefing.md` は段が進むたびに id を足す文書で、ここへ
/// 写した数はすぐに陳腐化する。いまの実数は、下の [`MIN_BRIEFING_CITED_IDS`] を使う
/// 腕が走るたびに [`cited_ids`] が数え直す。
///
/// 下限を 100 に置くのは、束の分け直しで単独項目の並びが増減しても動かさずに済む
/// 余裕を採ったためである。捕まえたいのは「読み込みが別のファイルを見ている・
/// 引用 id の取り出しが空を返す」といった**ほとんど何も残らない**壊れ方だけで、
/// 1 件 2 件の増減は見ない。
const MIN_BRIEFING_CITED_IDS: usize = 100;

/// 判定 ⑵ の母数——報告 5 本の束の一覧から読めた機械の束 id の数の下限。
///
/// 同じ束 id が summary とドメイン別報告の両方に現れることがあるので、数えるのは
/// 種類である。実数でなく下限（設計「判定の一覧」⑵ のとおり 100）にするのは、
/// `links` の補修で束が合流すれば種類が減りうるからである。
const MIN_REPORT_BUNDLE_IDS: usize = 100;

/// 判定 ⑵ の母数——`linkage.md` が引用した機械の束 id の数の下限。
///
/// 束を分けると増減するので、設計「判定の一覧」⑵ に合わせて「1 件以上」だけを
/// 主張する。
const MIN_CITED_BUNDLE_IDS: usize = 1;

/// 判定 ⑶ の母数——`linkage.md` の名前付き束の数の下限。
///
/// 設計「判定の一覧」⑶ のとおり「1 つ以上」にする。束は分けたり束ねたりするので
/// 実数を釘付けするとその作業のたびに赤くなり、捕まえたいのは「帰属が空の文書を
/// 緑と読む」壊れ方だけである。
const MIN_NAMED_BUNDLES: usize = 1;

/// 判定 ⑶ の母数——人手で足した構成 id の数の下限（`[tally].by_hand`）。
///
/// 関連を 1 本も持たない項目は人手で名前付き束へ入れる決まり（要件 4.2）なので、
/// この数が 0 に落ちたら腕 c は何も相手にしていない。設計「判定の一覧」⑶ のとおり
/// 「1 以上」にする。
const MIN_BY_HAND: usize = 1;

/// 判定 ⑷ の母数——`briefing.md` の `[[rank]]` の行数の下限（設計「判定の一覧」⑷）。
///
/// 行は束を指す行と単独項目を並べた行の 2 種。設計のとおり「1 行以上」にする。
/// 束は分けたり束ねたりするので実数を釘付けするとそのたびに赤くなり、捕まえたいのは
/// 「順位表が空の文書を緑と読む」壊れ方だけである。
const MIN_RANK_ROWS: usize = 1;

/// 判定 ⑷ の母数——段階 A の `items` の下限（設計「判定の一覧」⑷）。
///
/// 数えるのは台帳 4 本で `priority` が `A` で始まる項目である。設計のとおり
/// 「1 以上」にする。0 に落ちると、腕 b の 5 段階のうち最も大きい段階が
/// 「数え直しも 0・宣言も 0」でそろって緑になる。
const MIN_STAGE_A_ITEMS: usize = 1;

/// 判定 ⑷ の母数——`briefing.md` の `[[barrier]]` の行数の下限。
///
/// 腕 f は書いてある行を 1 行ずつ数え直すので、行が 0 行に落ちると比べる相手が無く
/// 緑になる。行はページの分け直しで増減するので「1 行以上」だけを主張する。
const MIN_BARRIER_ROWS: usize = 1;

/// 判定 ⑹ の母数——作り直した全体報告の束の表の行数の下限（設計「判定の一覧」⑹）。
///
/// 相手は `report/summary.md`「ドメインを跨いで繋がった束」の表。設計のとおり
/// 「1 行以上」にする。`links` の補修で束が合流すれば減りうるので実数を釘付けしない。
/// 捕まえたいのは「束の表が丸ごと落ちた本文を全文一致で緑と読む」壊れ方だけである
/// ——束の表は本文の 5 割強を占めるので、そこが両側で揃って落ちれば残りが一致する
/// かぎり判定 ⑹ は緑になる。
const MIN_SUMMARY_BUNDLE_ROWS: usize = 1;

/// 3 文書と全体報告の本文の長さの下限（文字数）。
///
/// 数え方は復帰文字を落とした後の `chars().count()`（`io::files::read_normalized` が
/// 落とす。バイト長ではない）。4 本とも下限よりずっと長い。下限 500 を動かさないのは、
/// 捕まえるのが読み込みが空を返す・別のファイルを見ている、といった**ほとんど何も
/// 残らない**壊れ方だけで、実数に寄せると本文が育つたびに書き換える羽目になるからで
/// ある。
const MIN_DOCUMENT_CHARS: usize = 500;

/// 読み込みが 3 文書と全体報告に届いていること。
#[test]
fn the_three_documents_and_the_summary_are_readable() {
    let repo = RepoData::load();
    let documents = Documents::load();

    Floors::default()
        .at_least(
            "linkage.md の本文",
            documents.linkage_text.chars().count(),
            MIN_DOCUMENT_CHARS,
        )
        .at_least(
            "briefing.md の本文",
            documents.briefing_text.chars().count(),
            MIN_DOCUMENT_CHARS,
        )
        .at_least(
            "roadmap-draft.md の本文",
            documents.roadmap_text.chars().count(),
            MIN_DOCUMENT_CHARS,
        )
        .at_least(
            "report/summary.md の本文",
            repo.summary_report.chars().count(),
            MIN_DOCUMENT_CHARS,
        )
        .assert_met();

    // 骨組みが骨組みとして読めていること。件数は 0 でもよいが、**欄そのもの**は
    // 揃っていなければならない（0 を省略で表さない・要件 11.5）。
    let domains: Vec<Domain> = documents
        .linkage
        .tally
        .singles_by_domain
        .keys()
        .copied()
        .collect();
    assert_eq!(
        domains,
        Domain::ALL.to_vec(),
        "linkage.md の [tally.singles_by_domain] に 4 ドメインが揃っていない"
    );

    let stages: Vec<Stage> = documents.briefing.stages.keys().copied().collect();
    assert_eq!(
        stages,
        Stage::ALL.to_vec(),
        "briefing.md の [stage.*] が A〜E の 5 つ揃っていない"
    );

    assert!(
        !documents.roadmap.briefs.snapshot_on.is_empty(),
        "roadmap-draft.md の [briefs].snapshot_on が空（表を撮った日が無い）"
    );
}

/// spec ディレクトリの一覧が本 spec 自身と `completed` を除いて集まること。
#[test]
fn the_spec_directory_listing_excludes_this_spec_and_completed() {
    let documents = Documents::load();

    Floors::default()
        .at_least(
            ".kiro/specs 直下の brief 持ち",
            documents.spec_dirs.len(),
            MIN_SPEC_DIRS,
        )
        .at_least(
            ".kiro/specs/completed 直下",
            documents.completed_specs.len(),
            MIN_COMPLETED_SPECS,
        )
        .assert_met();

    assert!(
        !documents.spec_dirs.contains(OWN_SPEC_DIR),
        "本 spec 自身 {OWN_SPEC_DIR} が一覧に混じっている"
    );
    assert!(
        !documents.spec_dirs.contains("completed"),
        "completed が spec の名前として一覧に混じっている"
    );

    // 除外の主張は、除外する相手が実在しなければ**無条件に成り立つ**（`non_vacuity.rs`
    // の `SELF_CRATE_PREFIX` と同じ弱さ）。だから本 spec の綴りが今どこかを指している
    // ことを先に言う——完了手続きの前は直下の brief 持ち、後は `completed/` の直下。
    let specs_root = paths::workspace_root().join(".kiro/specs");
    let live = specs_root.join(OWN_SPEC_DIR).join("brief.md").is_file();
    let completed = documents.completed_specs.contains(OWN_SPEC_DIR);
    assert!(
        live ^ completed,
        "本 spec {OWN_SPEC_DIR} が直下の brief 持ちと completed/ のちょうど一方に居ない\
         （直下={live}・completed={completed}）。除外が空回りしている"
    );
}

/// 判定 ⑴ ⑵ が数えている相手が 0 件でないこと（要件 11.4）。
#[test]
fn the_citation_and_bundle_checks_have_something_to_judge() {
    let repo = RepoData::load();
    let documents = Documents::load();

    let cited = cited_ids(&documents.linkage_text);
    let briefing_cited = cited_ids(&documents.briefing_text);
    let known = report_bundle_ids(repo);
    let machine = linkage_bundle_ids(&documents.linkage, &documents.linkage_text);

    Floors::default()
        .at_least(
            "判定 ⑴: linkage.md の引用 id",
            cited.len(),
            MIN_LINKAGE_CITED_IDS,
        )
        .at_least(
            "判定 ⑴: briefing.md の引用 id",
            briefing_cited.len(),
            MIN_BRIEFING_CITED_IDS,
        )
        .at_least(
            "判定 ⑵: 報告 5 本から読めた機械の束 id",
            known.len(),
            MIN_REPORT_BUNDLE_IDS,
        )
        .at_least(
            "判定 ⑵: linkage.md が引用した機械の束 id",
            machine.len(),
            MIN_CITED_BUNDLE_IDS,
        )
        .assert_met();
}

/// 判定 ⑶ が数えている相手が 0 件でないこと（要件 11.4）。
///
/// 見るのは 2 つ——名前付き束の数と、人手で足した構成 id の数である。帰属が空の
/// `linkage.md`（束 0・`[tally]` が全部 0）は、腕 a〜g（d は欠番）のうち a・c・e が
/// **母数 0 でそろって緑**になる。数えるものが無いことを「食い違いが無い」と読ませない。
#[test]
fn the_attribution_check_has_something_to_judge() {
    let documents = Documents::load();
    let named = documents
        .linkage
        .bundles
        .values()
        .filter(|bundle| !bundle.single)
        .count();

    Floors::default()
        .at_least("判定 ⑶: linkage.md の名前付き束", named, MIN_NAMED_BUNDLES)
        .at_least(
            "判定 ⑶: 人手で足した構成 id（[tally].by_hand）",
            documents.linkage.tally.by_hand,
            MIN_BY_HAND,
        )
        .assert_met();
}

/// 判定 ⑷ が数えている相手が 0 件でないこと（要件 11.4）。
///
/// 見るのは 3 つ——`[[rank]]` の行数・`[[barrier]]` の行数・段階 A の `items` である。
/// 順位表が空の `briefing.md` は、腕 a〜g のうち a・b・d・e・g が**母数 0 でそろって緑**
/// になり、`[[barrier]]` が 0 行なら腕 f のページの分布は比べる相手が無い。
#[test]
fn the_stage_and_rank_check_has_something_to_judge() {
    let documents = Documents::load();

    Floors::default()
        .at_least(
            "判定 ⑷: briefing.md の [[rank]] の行",
            documents.briefing.ranks.len(),
            MIN_RANK_ROWS,
        )
        .at_least(
            "判定 ⑷: briefing.md の [[barrier]] の行",
            documents.briefing.barriers.len(),
            MIN_BARRIER_ROWS,
        )
        .at_least(
            "判定 ⑷: 段階 A の items（[stage.A]）",
            documents
                .briefing
                .stages
                .get(&Stage::A)
                .map(|counts| counts.items)
                .unwrap_or_default(),
            MIN_STAGE_A_ITEMS,
        )
        .assert_met();
}

/// 判定 ⑸ が数えている相手が 0 件でないこと（要件 11.4）。
///
/// 見るのは 3 つ——`[[spec]]` の行数・`[[owner_completed]]` の行数・`completed/` の
/// 直下の数である。腕 a（`count` ＝ 行数）は**単独では母数 0 で恒真**で、
/// `count = 0` と 0 行なら緑になる。腕 b〜d も表が空なら数えるものが無く、
/// 腕 e は列挙が空なら同じく恒真になる。
#[test]
fn the_spec_table_check_has_something_to_judge() {
    let documents = Documents::load();

    Floors::default()
        .at_least(
            "判定 ⑸: roadmap-draft.md の [[spec]] の行",
            documents.roadmap.specs.len(),
            MIN_SPEC_ROWS,
        )
        .at_least(
            "判定 ⑸: briefing.md の [[owner_completed]] の行",
            documents.briefing.owner_completed.len(),
            MIN_OWNER_COMPLETED_ROWS,
        )
        .at_least(
            "判定 ⑸: .kiro/specs/completed 直下",
            documents.completed_specs.len(),
            MIN_COMPLETED_SPECS,
        )
        .assert_met();
}

/// 判定 ⑹ が数えている相手が 0 件でないこと（要件 11.4）。
///
/// 見るのは 2 つ——作り直した本文の長さと、その中の束の表の行数である。突き合わせ相手が
/// 空文字列に落ちれば全文一致は「ファイルも空なら緑」になり、束の表だけが落ちれば
/// 残りの節が一致するかぎり緑のまま残る。
#[test]
fn the_freshness_check_has_something_to_judge() {
    let repo = RepoData::load();
    let rendered = render_summary(&repo.catalog, &repo.ledgers, &THEMES);

    Floors::default()
        .at_least(
            "判定 ⑹: 作り直した全体報告の本文",
            rendered.chars().count(),
            MIN_DOCUMENT_CHARS,
        )
        .at_least(
            "判定 ⑹: 作り直した全体報告の束の表の行",
            bundle_ids_in_report(&rendered).len(),
            MIN_SUMMARY_BUNDLE_ROWS,
        )
        .assert_met();
}
