//! 3 文書まわりの判定の母数が 0 件でないことの主張と、そこで使う道具の較正
//! （要件 11.3・11.4・設計「入口 / `tests/consistency`」）。
//!
//! # ここに置くもの
//!
//! - 読み込み（[`Documents::load`]）が 3 文書・全体報告・spec ディレクトリの一覧に
//!   届いていること。
//! - 母数の下限を集める器（[`Floors`]）の較正——下限を下回れば赤になり、下限 0 は
//!   主張として受け付けない。
//! - 写しを 1 か所だけ壊す道具 5 つの較正——狙った 1 か所だけが変わり、狙いが無ければ
//!   止まり、repo のファイルには 1 バイトも触れない。
//!
//! - 判定 ⑴ ⑵ の母数の下限（`linkage.md` の引用 id 数・報告 5 本から読めた機械の
//!   束 id の数・3 文書が引用した機械の束 id の数）と、その下限が**下回ると赤になる**
//!   ことの較正。
//! - 判定 ⑶ の母数の下限（名前付き束の数・人手で足した id の数）と、同じ較正。
//! - 判定 ⑷ の母数の下限（`[[rank]]` の行数・段階 A の `items`）と、同じ較正。
//! - 判定 ⑹ の母数の下限（作り直した全体報告の本文の長さ・その束の表の行数）と、同じ較正。
//!
//! # ここにまだ置かないもの
//!
//! 残る判定の母数の下限（`[[spec]]` の行数・`[[owner_completed]]` の行数など）は、
//! その判定を置くタスク（判定 ⑸ は 6.6）の持ち物である。
//! `roadmap-draft.md` は今のところ骨組みだけで配列表が 0 行なので、ここで下限を置くと
//! **実データが追いつく前に赤くなる**。同じ理由で `roadmap-draft.md` の引用 id の
//! 下限も置かず、段 5 の持ち物にする。
//!
//! `briefing.md` の引用 id は**もう 0 件ではない**（段 4 が順位表を書いた）ので、
//! 「まだ何も書いていないから」という先送りの理由はこの文書には効かない。だから
//! 判定 ⑴ の下限は `linkage.md` と `briefing.md` の 2 本ぶん置く（タスク 4.7 で追加）。
//!
//! # spec ディレクトリの数は「下限」であって「一致」ではない
//!
//! 生きたディレクトリの総数との一致を主張すると、他 spec の起票・完了のたびに赤に
//! なり、この一群が全 spec の共有ファイルになる（設計「⑸ の数え方」）。だから下限
//! だけを置く。

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::SystemTime;

use ukadoc_survey::documents::Stage;
use ukadoc_survey::documents::parse::{read_briefing, read_linkage};
use ukadoc_survey::io::paths;
use ukadoc_survey::model::{Domain, THEMES};
use ukadoc_survey::report::summary::render_summary;

use super::RepoData;
use super::documents::{
    Documents, Floors, OWN_SPEC_DIR, cited_ids, drop_bundle_name, drop_member, shift_count,
    shift_row_count, twist_id, twisted_id,
};
use super::documents_checks::{bundle_ids_in_report, linkage_bundle_ids, report_bundle_ids};

/// `.kiro/specs/` の直下にある brief 持ちディレクトリの数の下限。
///
/// 2026-09-12 の実測は 27（本 spec 自身を除く）。実数を釘付けすると他 spec の起票・
/// 完了で赤になるので下限にする。狙いは「一覧が空を返す・別のディレクトリを見ている」
/// といった**ほとんど何も残らない**壊れ方を捕まえることで、1 本 2 本の増減は見ない。
const MIN_SPEC_DIRS: usize = 10;

/// `.kiro/specs/completed/` の直下のディレクトリ数の下限（設計 判定 ⑸ の母数）。
///
/// 2026-09-12 の実測は 174。完了した spec は減らないので下限として安定している。
const MIN_COMPLETED_SPECS: usize = 100;

/// 判定 ⑴ の母数——`linkage.md` の引用 id 数の下限。
///
/// 2026-09-12 に段 1 の後で数え直した実測は **1,553 件**で、内訳は束の `members` の
/// 和集合 1,552 件と、地の文だけに現れる別名 1 件
/// （`ukadoc:list_sakura_script:_5c7:1`・「過剰だった関連」の節）である。
///
/// 下限を 1,552 に置くのは設計「判定の一覧」⑴ の行のとおりで、この値は
/// `[tally].target`＝対象 4 状態の全数でもある。束の `members` が対象を過不足なく
/// 並べる以上、`linkage.md` の引用 id はこの数を下回れない。地の文の 1 件を足して
/// 1,553 に釘付けしないのは、それが本文の書き方次第で増減する余りだからである。
const MIN_LINKAGE_CITED_IDS: usize = 1_552;

/// 判定 ⑴ の母数——`briefing.md` の引用 id 数の下限。
///
/// **数え方は [`cited_ids`]（引用符付きと逆引用符付きの和集合・重複は畳む）**で、
/// 2026-09-13 に段 4 の後で数え直した実測は **237 件**（引用符付き 延べ 314・異なり
/// 220／逆引用符付き 延べ 23・異なり 22）。段 1 の時点では 0 件で、順位表と
/// `[[template]]` を段 4 が書いたことで増えた。
///
/// 下限を 100 に置くのは、束の分け直しで単独項目の並びが増減しても動かさずに済む
/// 余裕を採ったためである。捕まえたいのは「読み込みが別のファイルを見ている・
/// 引用 id の取り出しが空を返す」といった**ほとんど何も残らない**壊れ方だけで、
/// 1 件 2 件の増減は見ない。
const MIN_BRIEFING_CITED_IDS: usize = 100;

/// 判定 ⑵ の母数——報告 5 本の束の一覧から読めた機械の束 id の数の下限。
///
/// 2026-09-12 の実測は **123 行・122 種**（`report/summary.md` 75 行・
/// `report/sakura-script.md` 23 行・`report/shiori.md` 25 行・`report/assets.md`
/// **0 行**・`report/property.md` **0 行**）。行数と種類数が 1 だけ食い違うのは、
/// 1 種が summary とドメイン別報告の両方に現れるためである。
///
/// 実数でなく下限（設計「判定の一覧」⑵ のとおり 100）にするのは、`links` の補修で
/// 束が合流すれば種類が減りうるからである。
const MIN_REPORT_BUNDLE_IDS: usize = 100;

/// 判定 ⑵ の母数——3 文書が引用した機械の束 id の数の下限。
///
/// 2026-09-12 の実測は **47 種**で、すべて `linkage.md` である（`machine` 欄 47 種と、
/// 見出しが「束 id」で終わる表の列 3 種。列の 3 種はいずれも `machine` 欄にも在る）。
/// `briefing.md`・`roadmap-draft.md` はどちらも **0 種**。段 2 以降で束を分けると
/// 増減するので、設計「判定の一覧」⑵ に合わせて「1 件以上」だけを主張する。
const MIN_CITED_BUNDLE_IDS: usize = 1;

/// 判定 ⑶ の母数——`linkage.md` の名前付き束の数の下限。
///
/// 2026-09-12 の実測は **63**（ほかに単独項目 4）。設計「判定の一覧」⑶ のとおり
/// 「1 つ以上」にする。束は段 3 で分けたり束ねたりするので実数を釘付けすると
/// その作業のたびに赤くなり、捕まえたいのは「帰属が空の文書を緑と読む」壊れ方だけ
/// である。
const MIN_NAMED_BUNDLES: usize = 1;

/// 判定 ⑶ の母数——人手で足した構成 id の数の下限（`[tally].by_hand`）。
///
/// 2026-09-12 の実測は **1,353**（機械由来 195・単独項目 4 と合わせて 1,552）。
/// 開発者裁定（要件 4.2）で「関連を 1 本も持たない項目は人手で名前付き束へ入れる」
/// と決まっているので、この数が 0 に落ちたら腕 c は何も相手にしていない。設計
/// 「判定の一覧」⑶ のとおり「1 以上」にする。
const MIN_BY_HAND: usize = 1;

/// 判定 ⑷ の母数——`briefing.md` の `[[rank]]` の行数の下限（設計「判定の一覧」⑷）。
///
/// 2026-09-12 の実測は **66 行**（束を指す行 63・単独項目を並べた行 3）。設計のとおり
/// 「1 行以上」にする。束は段 4 以降で分けたり束ねたりするので実数を釘付けするとその
/// たびに赤くなり、捕まえたいのは「順位表が空の文書を緑と読む」壊れ方だけである。
const MIN_RANK_ROWS: usize = 1;

/// 判定 ⑷ の母数——段階 A の `items` の下限（設計「判定の一覧」⑷）。
///
/// 2026-09-12 の実測は **874**（台帳 4 本で `priority` が `A` で始まる項目数）。設計の
/// とおり「1 以上」にする。0 に落ちると、腕 b の 5 段階のうち最も大きい段階が
/// 「数え直しも 0・宣言も 0」でそろって緑になる。
const MIN_STAGE_A_ITEMS: usize = 1;

/// 判定 ⑹ の母数——作り直した全体報告の束の表の行数の下限（設計「判定の一覧」⑹）。
///
/// 2026-09-13 の実測は **75 行**（`report/summary.md`「ドメインを跨いで繋がった束」の
/// 表）。設計のとおり「1 行以上」にする。`links` の補修で束が合流すれば減りうるので
/// 実数を釘付けしない。捕まえたいのは「束の表が丸ごと落ちた本文を全文一致で緑と読む」
/// 壊れ方だけである——束の表は本文の 5 割強を占めるので、そこが両側で揃って落ちれば
/// 残りが一致するかぎり判定 ⑹ は緑になる。
const MIN_SUMMARY_BUNDLE_ROWS: usize = 1;

/// 順位表が 1 行も無い `briefing.md`（母数が空に落ちた壊れ方の写し）。
///
/// 読み手が受け付ける最小の形で、`[stage.A]`〜`[stage.E]` の 3 つの鍵と
/// `[priority_blank]` の 2 つの鍵をすべて 0 にしてある。
const EMPTY_BRIEFING: &str = "```toml
[stage.A]
bundles = 0
singles = 0
items = 0

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
singles = 0
items = 0

[priority_blank]
alias = 0
not_applicable = 0
```
";

/// 3 文書と全体報告の本文の長さの下限（文字数）。
///
/// 数え方は復帰文字を落とした後の `chars().count()`（`io::files::read_normalized` が
/// 落とす。バイト長ではない）。2026-09-12 の実測は `linkage.md` 1,336・
/// `roadmap-draft.md` 1,416・`briefing.md` 4,618・`report/summary.md` 33,326 で、
/// **最小は `linkage.md` の 1,336 文字**である。下限 500 はその 4 割弱で、3 文書が
/// 骨組みから育っていく間も動かさずに済む余裕を採った。捕まえるのは読み込みが空を
/// 返す・別のファイルを見ている、といった**ほとんど何も残らない**壊れ方だけである。
const MIN_DOCUMENT_CHARS: usize = 500;

/// 束が 1 つも無い `linkage.md`（母数が空に落ちた壊れ方の写し）。
///
/// 読み手が受け付ける最小の形で、`[tally]` の 6 つの鍵とドメイン別の 4 欄をすべて 0
/// にしてある。これを判定へ渡すと、数えるものが無いまま緑になる腕がどれなのかが
/// 分かる。
const EMPTY_LINKAGE: &str = "```toml\n[tally]\ntarget = 0\nfrom_machine = 0\nby_hand = 0\nsingles = 0\nalias_excluded = 0\nnot_applicable_excluded = 0\n\n[tally.singles_by_domain]\nassets = 0\nproperty = 0\nsakura-script = 0\nshiori = 0\n```\n";

/// 読み込みが 3 文書と全体報告に届いていること。
#[test]
fn the_three_documents_and_the_summary_are_readable() {
    let documents = Documents::load();

    Floors::new()
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
            documents.summary_text.chars().count(),
            MIN_DOCUMENT_CHARS,
        )
        .assert_met();

    // 骨組みが骨組みとして読めていること。件数はまだ 0 でよいが、**欄そのもの**は
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

    Floors::new()
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

    // 一覧をここでもう 1 度、独立に組み直して突き合わせる。同じ関数の答えを同じ関数で
    // 言い換えても「読み込みが読み込みに同意する」だけになる（`non_vacuity.rs` の
    // 割り当ての事例と同じ理由）。
    let mut again = BTreeSet::new();
    let mut brief_less = BTreeSet::new();
    for entry in std::fs::read_dir(&specs_root).expect(".kiro/specs を開けない") {
        let entry = entry.expect(".kiro/specs の直下を読めない");
        if !entry.path().is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == "completed" || name == OWN_SPEC_DIR {
            continue;
        }
        if entry.path().join("brief.md").is_file() {
            again.insert(name);
        } else {
            brief_less.insert(name);
        }
    }
    assert_eq!(
        documents.spec_dirs, again,
        "一覧が .kiro/specs の直下を独立に数え直した結果と食い違う"
    );

    // brief を持たない直下のディレクトリは 2026-09-12 時点で **0 件**である（0 も書く・
    // 要件 11.5）。つまり brief の絞りは今日の実データでは 1 件も落としておらず、
    // 「brief 持ちだけを拾う」の側は生きた反例で較正できていない。反例が現れたら
    // ここが名前付きで赤くなるので、そのときに数え直すこと。絞りが働くこと自体は
    // すぐ下の `completed` で見る。
    assert_eq!(
        brief_less,
        BTreeSet::new(),
        "brief.md を持たない spec ディレクトリが直下に現れた（0 件の写真が古い）"
    );
    assert!(
        !specs_root.join("completed").join("brief.md").is_file(),
        "completed が brief.md を持っている（名前で除かなくても落ちる、の前提が崩れた）"
    );
}

/// 判定 ⑴ ⑵ が数えている相手が 0 件でないこと（要件 11.4）。
#[test]
fn the_citation_and_bundle_checks_have_something_to_judge() {
    let repo = RepoData::load();
    let documents = Documents::load();

    let cited = cited_ids(&documents.linkage_text);
    let briefing_cited = cited_ids(&documents.briefing_text);
    let known = report_bundle_ids(&repo, &documents.summary_text);
    let machine = linkage_bundle_ids(&documents.linkage, &documents.linkage_text);

    Floors::new()
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
            "判定 ⑵: 3 文書が引用した機械の束 id",
            machine.len(),
            MIN_CITED_BUNDLE_IDS,
        )
        .assert_met();
}

/// 母数が空に落ちたとき、上の下限が**実際に**赤になること。
///
/// 下限を置いただけでは「置いた」という記録が残るだけである。取り出しが空を返す
/// 壊れ方（読み込みが別のファイルを見ている・表の見出しが変わって拾えなくなった）を
/// 同じ取り出しの上で作り、3 行とも名指しで違反に挙がることを見る。
#[test]
fn the_floors_of_the_citation_and_bundle_checks_turn_red_when_the_source_goes_empty() {
    let empty_linkage = read_linkage(EMPTY_LINKAGE).expect("空の linkage.md を組み立てられない");

    let mut floors = Floors::new();
    floors
        .at_least(
            "判定 ⑴: linkage.md の引用 id",
            cited_ids("").len(),
            MIN_LINKAGE_CITED_IDS,
        )
        .at_least(
            "判定 ⑴: briefing.md の引用 id",
            cited_ids("").len(),
            MIN_BRIEFING_CITED_IDS,
        )
        .at_least(
            "判定 ⑵: 報告 5 本から読めた機械の束 id",
            bundle_ids_in_report("").len(),
            MIN_REPORT_BUNDLE_IDS,
        )
        .at_least(
            "判定 ⑵: 3 文書が引用した機械の束 id",
            linkage_bundle_ids(&empty_linkage, "").len(),
            MIN_CITED_BUNDLE_IDS,
        );

    let short = floors.short();
    assert_eq!(
        short.len(),
        4,
        "母数が空に落ちたのに違反が 4 行そろわない: {short:?}"
    );
    assert!(
        short.iter().all(|line| line.contains("0 件しかない")),
        "空に落ちたことを本文が言っていない: {short:?}"
    );
    assert!(
        short[0].contains("linkage.md の引用 id")
            && short[1].contains("briefing.md の引用 id")
            && short[2].contains("報告 5 本")
            && short[3].contains("3 文書が引用した"),
        "どの母数が空なのかを名指していない: {short:?}"
    );
}

/// 判定 ⑶ が数えている相手が 0 件でないこと（要件 11.4）。
///
/// 見るのは 2 つ——名前付き束の数と、人手で足した構成 id の数である。帰属が空の
/// `linkage.md`（束 0・`[tally]` が全部 0）は、腕 a〜g のうち a・c・d・e が
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

    Floors::new()
        .at_least("判定 ⑶: linkage.md の名前付き束", named, MIN_NAMED_BUNDLES)
        .at_least(
            "判定 ⑶: 人手で足した構成 id（[tally].by_hand）",
            documents.linkage.tally.by_hand,
            MIN_BY_HAND,
        )
        .assert_met();
}

/// 帰属が空に落ちたとき、上の下限が**実際に**赤になること。
#[test]
fn the_floors_of_the_attribution_check_turn_red_when_the_bundles_go_empty() {
    let empty = read_linkage(EMPTY_LINKAGE).expect("空の linkage.md を組み立てられない");

    let mut floors = Floors::new();
    floors
        .at_least(
            "判定 ⑶: linkage.md の名前付き束",
            empty.bundles.values().filter(|b| !b.single).count(),
            MIN_NAMED_BUNDLES,
        )
        .at_least(
            "判定 ⑶: 人手で足した構成 id（[tally].by_hand）",
            empty.tally.by_hand,
            MIN_BY_HAND,
        );

    let short = floors.short();
    assert_eq!(
        short.len(),
        2,
        "帰属が空に落ちたのに違反が 2 行そろわない: {short:?}"
    );
    assert!(
        short.iter().all(|line| line.contains("0 件しかない")),
        "空に落ちたことを本文が言っていない: {short:?}"
    );
    assert!(
        short[0].contains("名前付き束") && short[1].contains("by_hand"),
        "どの母数が空なのかを名指していない: {short:?}"
    );
}

/// 判定 ⑷ が数えている相手が 0 件でないこと（要件 11.4）。
///
/// 見るのは 2 つ——`[[rank]]` の行数と、段階 A の `items` である。順位表が空の
/// `briefing.md` は、腕 a〜g のうち a・b・d・e・g が**母数 0 でそろって緑**になる。
#[test]
fn the_stage_and_rank_check_has_something_to_judge() {
    let documents = Documents::load();

    Floors::new()
        .at_least(
            "判定 ⑷: briefing.md の [[rank]] の行",
            documents.briefing.ranks.len(),
            MIN_RANK_ROWS,
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

/// 順位表が空に落ちたとき、上の下限が**実際に**赤になること。
#[test]
fn the_floors_of_the_stage_and_rank_check_turn_red_when_the_ranks_go_empty() {
    let empty = read_briefing(EMPTY_BRIEFING).expect("空の briefing.md を組み立てられない");

    let mut floors = Floors::new();
    floors
        .at_least(
            "判定 ⑷: briefing.md の [[rank]] の行",
            empty.ranks.len(),
            MIN_RANK_ROWS,
        )
        .at_least(
            "判定 ⑷: 段階 A の items（[stage.A]）",
            empty
                .stages
                .get(&Stage::A)
                .map(|counts| counts.items)
                .unwrap_or_default(),
            MIN_STAGE_A_ITEMS,
        );

    let short = floors.short();
    assert_eq!(
        short.len(),
        2,
        "順位表が空に落ちたのに違反が 2 行そろわない: {short:?}"
    );
    assert!(
        short.iter().all(|line| line.contains("0 件しかない")),
        "空に落ちたことを本文が言っていない: {short:?}"
    );
    assert!(
        short[0].contains("[[rank]]") && short[1].contains("[stage.A]"),
        "どの母数が空なのかを名指していない: {short:?}"
    );
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

    Floors::new()
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

/// 突き合わせ相手が空に落ちたとき、上の下限が**実際に**赤になること。
#[test]
fn the_floors_of_the_freshness_check_turn_red_when_the_summary_goes_empty() {
    let mut floors = Floors::new();
    floors
        .at_least(
            "判定 ⑹: 作り直した全体報告の本文",
            "".chars().count(),
            MIN_DOCUMENT_CHARS,
        )
        .at_least(
            "判定 ⑹: 作り直した全体報告の束の表の行",
            bundle_ids_in_report("").len(),
            MIN_SUMMARY_BUNDLE_ROWS,
        );

    let short = floors.short();
    assert_eq!(
        short.len(),
        2,
        "突き合わせ相手が空に落ちたのに違反が 2 行そろわない: {short:?}"
    );
    assert!(
        short.iter().all(|line| line.contains("0 件しかない")),
        "空に落ちたことを本文が言っていない: {short:?}"
    );
    assert!(
        short[0].contains("本文") && short[1].contains("束の表の行"),
        "どの母数が空なのかを名指していない: {short:?}"
    );
}

/// 母数の器が、下限を下回った行を**すべて**名指すこと。
#[test]
fn the_floor_container_reports_every_row_that_falls_short() {
    let mut floors = Floors::new();
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
    let mut floors = Floors::new();
    floors.at_least("母数 0 でも通る下限", 1_000, 0);

    let short = floors.short();
    assert_eq!(short.len(), 1, "下限 0 を素通りさせている: {short:?}");
    assert!(
        short[0].contains("母数 0 でも通る下限"),
        "下限 0 の行を名指していない: {short:?}"
    );
}

/// 満たしている器は止まらないこと（較正の反対側）。
#[test]
fn the_floor_container_stays_quiet_when_every_row_is_met() {
    let mut floors = Floors::new();
    floors.at_least("足りている数", 2, 1);
    assert!(
        floors.short().is_empty(),
        "満たしているのに違反を挙げている"
    );
    floors.assert_met();
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
    assert_eq!(twisted, "ukadoc:list_shiori_event:OnBoot:X");
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

/// 同じ鍵が別の表と地の文にも現れるとき、錨が狙った表の中だけを見ること。
///
/// これがこの道具の要である。錨を本文全体に置くと、`briefing.md` の `[stage.A]`〜
/// `[stage.E]` が同じ鍵を 5 回持つだけで摂動が空振りして止まり、`linkage.md` の
/// 地の文に恒等式を半角の `=` で書くだけで同じことが起きる（設計 D-2）。
#[test]
fn shifting_a_count_looks_only_inside_the_named_table() {
    let markdown = "\
恒等式は target = from_machine + by_hand + singles である。

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
        shifted.contains("恒等式は target = from_machine"),
        "地の文まで書き換えている:\n{shifted}"
    );
}

/// 実データの写しの上でも、5 つの段階の囲みを 1 つずつ狙えること。
///
/// `briefing.md` は設計 D-2 により `singles = ` を最低 5 回持つ。本文全体を錨に
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

/// 束名を 1 つ消す摂動が、その名前を書いた行だけを落とすこと。
#[test]
fn dropping_a_bundle_name_removes_its_line() {
    let toml = "[[rank]]\nstage = \"B\"\nbundle = \"更新\"\nrank = 1\n";
    assert_eq!(
        drop_bundle_name(toml, "更新"),
        "[[rank]]\nstage = \"B\"\nrank = 1",
        "束名を書いた行だけを落とせていない"
    );
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

/// 狙いが無いのに素通りしないこと（5 つの道具それぞれ）。
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
fn dropping_an_absent_bundle_name_stops() {
    drop_bundle_name("[[rank]]\nrank = 1\n", "更新");
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

/// 壊す道具が repo のファイルに 1 バイトも触れないこと（要件 11.3 の前提）。
///
/// 5 つの道具を**合成の写し**の上で一通り働かせ、うち `twist_id`・`shift_count`・
/// `shift_row_count` の 3 つは**実データの写し**（前 2 つは `linkage.md`、最後は
/// `report/summary.md` の本文）の上でも働かせる。前後でファイルの
/// 中身と更新時刻が変わらないことを見る——写しの上だけで働く関数だという主張は、
/// こうして実ファイルを見ないと「たまたま今は書いていない」と区別できない。
///
/// 残る 2 つ（`drop_member`・`drop_bundle_name`）は合成の写しだけに掛ける。実データで
/// 掛けられないからではなく、**狙いを 1 つに決める理屈がここには無い**からである。
/// どちらの道具も「写しにちょうど 1 度だけ現れる綴り」を要求するので、実データでは
/// 狙いを選ぶ側の理屈（どの id・どの束名なら 1 度きりか）が要る。それは帰属の判定
/// （⑶ ⑷）の持ち物なので、選び方はそちらへ置く。
///
/// 2026-09-13 に段 4 の後で数え直した実測（**この段落は 2 度古びている**——初版は
/// 3 文書が骨組みだったころの写真のまま、2026-09-12 の版は段 1 の写真のままで、
/// どちらも `briefing.md` を 0 件と書いていた。**数を doc に書く限りこの壊れ方は
/// 繰り返す**ので、増減したら日付ごと採り直すこと）:
///
/// - 3 文書の `ukadoc:` の出現: `linkage.md` **3,135 件**・`briefing.md` **337 件**・
///   `roadmap-draft.md` **0 件**。
/// - `linkage.md` の `members = `: **67 件**（束 63・単独項目 4）。
/// - `briefing.md` の `bundle = `: **63 件**（段 4 が書いた。`[[rank]]` は **66 行**で、
///   差の 3 行は単独項目を並べた `singles` の行である）。
/// - `linkage.md` の引用符付き id で**ちょうど 1 度**現れるもの: **151 件**
///   （`drop_member` の狙いになりうる綴り）。束名 67 のうち、その綴りを引用符付きで
///   書いた行が 1 行きりのもの: **62 件**（`drop_bundle_name` の狙いになりうる名前）。
#[test]
fn the_breaking_tools_do_not_touch_the_repository_files() {
    let watched: Vec<PathBuf> = vec![
        paths::linkage_path(),
        paths::briefing_path(),
        paths::roadmap_draft_path(),
        paths::summary_report_path(),
    ];
    let before: Vec<(Vec<u8>, SystemTime)> = watched.iter().map(fingerprint).collect();

    let documents = Documents::load();
    let id = "ukadoc:list_shiori_event:OnBoot:1";
    let sample = format!(
        "[bundle.\"起動\"]\nmembers = [\"{id}\", \"ukadoc:list_shiori_event:OnClose:1\"]\n\n[tally]\ntarget = 2\n"
    );
    let _ = twist_id(&sample, id);
    let _ = drop_member(&sample, id);
    let _ = shift_count(&sample, "[tally]", "target");
    let _ = drop_bundle_name(&sample, "起動");
    let _ = shift_row_count("| 縮退 | 22 |\n", "縮退");
    // 実データの写しの上でも働かせる（写しを取り違えて元を触る壊れ方を見る）。
    let _ = shift_count(&documents.linkage_text, "[tally]", "target");
    let _ = shift_row_count(&documents.summary_text, "縮退");
    let cited = cited_ids(&documents.linkage_text);
    let first = cited
        .iter()
        .next()
        .expect("linkage.md が id を 1 つも引用していない");
    let _ = twist_id(&documents.linkage_text, first);

    let after: Vec<(Vec<u8>, SystemTime)> = watched.iter().map(fingerprint).collect();
    for (path, (before, after)) in watched.iter().zip(before.iter().zip(after.iter())) {
        assert_eq!(
            before.0.len(),
            after.0.len(),
            "{} の長さが変わった（道具が repo のファイルを書き換えた）",
            path.display()
        );
        assert_eq!(
            before.0,
            after.0,
            "{} の中身が変わった（道具が repo のファイルを書き換えた）",
            path.display()
        );
        assert_eq!(
            before.1,
            after.1,
            "{} の更新時刻が変わった（道具が repo のファイルへ書き戻した）",
            path.display()
        );
    }
}

/// ファイルの中身と更新時刻。
fn fingerprint(path: &PathBuf) -> (Vec<u8>, SystemTime) {
    let bytes =
        std::fs::read(path).unwrap_or_else(|err| panic!("{} を読めない: {err}", path.display()));
    let modified = std::fs::metadata(path)
        .and_then(|meta| meta.modified())
        .unwrap_or_else(|err| panic!("{} の更新時刻を読めない: {err}", path.display()));
    (bytes, modified)
}
