//! 3 文書まわりの判定の母数が 0 件でないことの主張と、そこで使う道具の較正
//! （要件 11.3・11.4・設計「入口 / `tests/consistency`」）。
//!
//! # ここに置くもの
//!
//! - 読み込み（[`Documents::load`]）が 3 文書・全体報告・spec ディレクトリの一覧に
//!   届いていること。
//!
//! - 判定 ⑴ ⑵ の母数の下限（`linkage.md` の引用 id 数・報告 5 本から読めた機械の
//!   束 id の数・3 文書が引用した機械の束 id の数）と、その下限が**下回ると赤になる**
//!   ことの較正。
//! - 判定 ⑶ の母数の下限（名前付き束の数・人手で足した id の数）と、同じ較正。
//! - 判定 ⑷ の母数の下限（`[[rank]]` の行数・段階 A の `items`）と、同じ較正。
//! - 判定 ⑸ の母数の下限（`[[spec]]` の行数・`[[owner_completed]]` の行数・
//!   `completed/` の直下の数）と、同じ較正。
//! - 判定 ⑹ の母数の下限（作り直した全体報告の本文の長さ・その束の表の行数）と、同じ較正。
//!
//! 母数の器（[`Floors`]）と壊す道具 5 つの**較正**は兄弟の `documents_tools.rs` にある
//! （1 ファイル 1,000 行の目安・`structure.md:176`）。
//!
//! # 判定 ⑸ の下限は 6.6 で置いた
//!
//! 段 5 が `roadmap-draft.md` の配列表を書くまでは、ここに下限を置くと**実データが
//! 追いつく前に赤くなる**ので先送りしていた。段 5 が済んだので下限を置く。
//! `roadmap-draft.md` の引用 id は今も **0 件**（この文書は spec 名と束名だけを書き、
//! 項目 id を引用しない）なので、判定 ⑴ の下限はこの文書ぶんを置かないままにする。
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

use ukadoc_survey::documents::Stage;
use ukadoc_survey::documents::parse::{read_briefing, read_linkage, read_roadmap_draft};
use ukadoc_survey::io::paths;
use ukadoc_survey::model::{Domain, THEMES};
use ukadoc_survey::report::summary::render_summary;

use super::RepoData;
use super::documents::{Documents, Floors, OWN_SPEC_DIR, cited_ids};
use super::documents_checks::{bundle_ids_in_report, linkage_bundle_ids, report_bundle_ids};

/// `.kiro/specs/` の直下にある brief 持ちディレクトリの数の下限。
///
/// 2026-09-13 の実測は 27（本 spec 自身を除く）。実数を釘付けすると他 spec の起票・
/// 完了で赤になるので下限にする。狙いは「一覧が空を返す・別のディレクトリを見ている」
/// といった**ほとんど何も残らない**壊れ方を捕まえることで、1 本 2 本の増減は見ない。
const MIN_SPEC_DIRS: usize = 10;

/// `.kiro/specs/completed/` の直下のディレクトリ数の下限（設計 判定 ⑸ の母数）。
///
/// 2026-09-13 の実測は **176**（数え方: `completed/` 直下の**ディレクトリ**を数えた。
/// 直下には単独の `.md` が 1 本混じっているので、`ls` の行数 177 とは 1 ずれる。数える
/// のは [`Documents::load`] の `child_dirs` で、ディレクトリでない項目は落とす）。
/// 完了した spec は減らないので下限として安定している。
const MIN_COMPLETED_SPECS: usize = 100;

/// 判定 ⑸ の母数——`roadmap-draft.md` の `[[spec]]` の行数の下限。
///
/// 2026-09-13 の実測は **27 行**（`[briefs].count` と一致する。腕 a が両者を突き合わせる）。
/// 下限を設計「判定の一覧」⑸ のとおり 20 に置くのは、表が着手時の写真であって生きた
/// 総数を追わないからである——他 spec の完了で表から行が減ることはあり、実数を釘付け
/// するとそのたびに赤くなる。捕まえたいのは「表が空のまま `count = 0` と書いて全部の
/// 腕が恒真になる」壊れ方で、段 5 の着手前の実データがまさにそれだった。
const MIN_SPEC_ROWS: usize = 20;

/// 判定 ⑸ の母数——`briefing.md` の `[[owner_completed]]` の行数の下限。
///
/// 2026-09-13 の実測は **14 行**・`items` の合計 **41 件**（数え方: `[[owner_completed]]`
/// の直後の `items` を 14 行ぶん足した。腕 e が 1 行ずつ数え直す）。設計「判定の一覧」⑸ のとおり
/// 「1 行以上」にする。完了済み spec を宛先に残す行は、宛先の整理（要件 7.4）が
/// 進むと増減するので実数は釘付けしない。0 行に落ちると腕 e は数える相手を失って
/// **恒真になる**（腕 f のほうは逆に、行き先を失った宛先を全部挙げて騒がしくなる）。
const MIN_OWNER_COMPLETED_ROWS: usize = 1;

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
/// 実数はこの注釈に写さない。`briefing.md` は段が進むたびに id を足す文書で、ここへ
/// 写した数は 2 度陳腐化した（段 1 の 0 件・段 4 の 237 件）。いまの実数は下の
/// [`MIN_BRIEFING_CITED_IDS`] を使う腕が走るたびに [`cited_ids`] が数え直す。
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
/// 落とす。バイト長ではない）。2026-09-13 の実測は `linkage.md` 268,349・
/// `briefing.md` 95,695・`roadmap-draft.md` 52,007・`report/summary.md` 33,420 で、
/// **最小は `report/summary.md` の 33,420 文字**である（2026-09-12 に骨組みだった
/// ころの最小は `linkage.md` の 1,336 文字だった。4 本とも段が進むたびに増えるので、
/// 下限に近づく向きには動かない）。下限 500 を動かさないのは、捕まえるのが読み込みが
/// 空を返す・別のファイルを見ている、といった**ほとんど何も残らない**壊れ方だけで、
/// 実数に寄せると本文が育つたびに書き換える羽目になるからである。
const MIN_DOCUMENT_CHARS: usize = 500;

/// 束が 1 つも無い `linkage.md`（母数が空に落ちた壊れ方の写し）。
///
/// 読み手が受け付ける最小の形で、`[tally]` の 6 つの鍵とドメイン別の 4 欄をすべて 0
/// にしてある。これを判定へ渡すと、数えるものが無いまま緑になる腕がどれなのかが
/// 分かる。
const EMPTY_LINKAGE: &str = "```toml\n[tally]\ntarget = 0\nfrom_machine = 0\nby_hand = 0\nsingles = 0\nalias_excluded = 0\nnot_applicable_excluded = 0\n\n[tally.singles_by_domain]\nassets = 0\nproperty = 0\nsakura-script = 0\nshiori = 0\n```\n";

/// spec 表も予約群も 1 行も無い `roadmap-draft.md`（母数が空に落ちた壊れ方の写し）。
///
/// 段 5 の着手前の実データそのものである——`count = 0` と 0 行で腕 a は緑だった。
const EMPTY_ROADMAP: &str = "```toml
[briefs]
count = 0
snapshot_on = \"2026-09-11\"
```
";

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

/// 判定 ⑸ が数えている相手が 0 件でないこと（要件 11.4）。
///
/// 見るのは 3 つ——`[[spec]]` の行数・`[[owner_completed]]` の行数・`completed/` の
/// 直下の数である。腕 a（`count` ＝ 行数）は**単独では母数 0 で恒真**で、段 5 の
/// 着手前は `count = 0` と 0 行で緑だった。腕 b〜d も表が空なら数えるものが無く、
/// 腕 e は列挙が空なら同じく恒真になる。
#[test]
fn the_spec_table_check_has_something_to_judge() {
    let documents = Documents::load();

    Floors::new()
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

/// spec 表と完了済みの列挙が空に落ちたとき、上の下限が**実際に**赤になること。
#[test]
fn the_floors_of_the_spec_table_check_turn_red_when_the_tables_go_empty() {
    let empty_roadmap =
        read_roadmap_draft(EMPTY_ROADMAP).expect("空の roadmap-draft.md を組み立てられない");
    let empty_briefing =
        read_briefing(EMPTY_BRIEFING).expect("空の briefing.md を組み立てられない");

    let mut floors = Floors::new();
    floors
        .at_least(
            "判定 ⑸: roadmap-draft.md の [[spec]] の行",
            empty_roadmap.specs.len(),
            MIN_SPEC_ROWS,
        )
        .at_least(
            "判定 ⑸: briefing.md の [[owner_completed]] の行",
            empty_briefing.owner_completed.len(),
            MIN_OWNER_COMPLETED_ROWS,
        )
        .at_least(
            "判定 ⑸: .kiro/specs/completed 直下",
            BTreeSet::<String>::new().len(),
            MIN_COMPLETED_SPECS,
        );

    let short = floors.short();
    assert_eq!(
        short.len(),
        3,
        "表が空に落ちたのに違反が 3 行そろわない: {short:?}"
    );
    assert!(
        short.iter().all(|line| line.contains("0 件しかない")),
        "空に落ちたことを本文が言っていない: {short:?}"
    );
    assert!(
        short[0].contains("[[spec]]")
            && short[1].contains("[[owner_completed]]")
            && short[2].contains("completed"),
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
