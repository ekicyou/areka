//! 3 文書まわりの判定の母数が 0 件でないことの主張と、そこで使う道具の較正
//! （要件 11.3・11.4・設計「入口 / `tests/consistency`」）。
//!
//! # ここに置くもの
//!
//! - 読み込み（[`Documents::load`]）が 3 文書・全体報告・spec ディレクトリの一覧に
//!   届いていること。
//! - 母数の下限を集める器（[`Floors`]）の較正——下限を下回れば赤になり、下限 0 は
//!   主張として受け付けない。
//! - 写しを 1 か所だけ壊す道具 4 つの較正——狙った 1 か所だけが変わり、狙いが無ければ
//!   止まり、repo のファイルには 1 バイトも触れない。
//!
//! # ここにまだ置かないもの
//!
//! 判定 6 種それぞれの母数の下限（`linkage.md` の引用 id 数・名前付き束の数・
//! `[[rank]]` の行数・`[[spec]]` の行数など）は、その判定を置くタスク（3.7 以降）の
//! 持ち物である。3 文書は今のところ骨組みだけで配列表が 0 行なので、ここで下限を
//! 置くと**実データが追いつく前に赤くなる**。今ここが主張するのは「読める」ことと
//! 「道具が効く」ことだけである。
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
use ukadoc_survey::io::paths;
use ukadoc_survey::model::Domain;

use super::documents::{
    Documents, Floors, OWN_SPEC_DIR, cited_ids, drop_bundle_name, drop_member, shift_count,
    twist_id, twisted_id,
};

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

/// 3 文書と全体報告の本文の長さの下限（文字数）。
///
/// 数え方は復帰文字を落とした後の `chars().count()`（`io::files::read_normalized` が
/// 落とす。バイト長ではない）。2026-09-12 の実測は `linkage.md` 1,336・
/// `roadmap-draft.md` 1,416・`briefing.md` 4,618・`report/summary.md` 33,326 で、
/// **最小は `linkage.md` の 1,336 文字**である。下限 500 はその 4 割弱で、3 文書が
/// 骨組みから育っていく間も動かさずに済む余裕を採った。捕まえるのは読み込みが空を
/// 返す・別のファイルを見ている、といった**ほとんど何も残らない**壊れ方だけである。
const MIN_DOCUMENT_CHARS: usize = 500;

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

/// 件数を 1 ずらす摂動が、狙った鍵の数だけを動かすこと。
#[test]
fn shifting_a_count_moves_the_number_by_one() {
    let toml = "[tally]\ntarget = 1749\nsingles = 0\n";
    assert_eq!(
        shift_count(toml, "target"),
        "[tally]\ntarget = 1750\nsingles = 0\n",
        "狙った鍵の数を 1 ずらせていない"
    );
    assert_eq!(
        shift_count(toml, "singles"),
        "[tally]\ntarget = 1749\nsingles = 1\n",
        "0 の欄を 1 ずらせていない"
    );
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
    shift_count("[tally]\ntarget = 1\n", "singles");
}

#[test]
#[should_panic(expected = "ちょうど 1 行ない")]
fn dropping_an_absent_bundle_name_stops() {
    drop_bundle_name("[[rank]]\nrank = 1\n", "更新");
}

/// 壊す道具が repo のファイルに 1 バイトも触れないこと（要件 11.3 の前提）。
///
/// 4 つの道具を**合成の写し**の上で一通り働かせ、うち `shift_count` 1 つは
/// **実データの写し**（`linkage.md` の本文）の上でも働かせる。前後でファイルの中身と
/// 更新時刻が変わらないことを見る——写しの上だけで働く関数だという主張は、こうして
/// 実ファイルを見ないと「たまたま今は書いていない」と区別できない。
///
/// 残る 3 つを実データの写しに掛けないのは、3 文書がまだ骨組みで狙いの綴りを 1 つも
/// 持たないからである（2026-09-12 の実測: 3 文書の `ukadoc:` 0 件・`linkage.md` の
/// `members = ` 0 件・`briefing.md` の `bundle = ` 0 件）。掛ければ道具は空振りで
/// 止まり、確かめたい「触らない」ではなく「狙いが無い」で赤くなる。3 文書が育ったら
/// 実データの写しへ寄せてよい。
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
    let _ = shift_count(&sample, "target");
    let _ = drop_bundle_name(&sample, "起動");
    // 実データの写しの上でも 1 度働かせる（写しを取り違えて元を触る壊れ方を見る）。
    let _ = shift_count(&documents.linkage_text, "target");

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
