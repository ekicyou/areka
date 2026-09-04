//! `bundle.rs` の在中テスト。
//!
//! ここは純粋層のテストなので、ファイルも一時ディレクトリも 1 つも作らない
//! （要件 6.2・設計 File Structure Plan）。
//!
//! 見本の関連の対は**わざと乱した順**で書く。整列済みの見本は「整列する」摂動を
//! 素通りさせるので、並び順を主張するテストの見本にはならない（タスク 2.5 の教訓）。

use super::{Bundle, bundles};
use crate::model::EntryId;

fn id(raw: &str) -> EntryId {
    EntryId::parse(raw).expect("見本の id は 2 形のいずれかのはず")
}

fn edge(from: &str, to: &str) -> (EntryId, EntryId) {
    (id(from), id(to))
}

// 3 つの連結成分。どの成分も「最初に現れた id」が「最小の id」ではない形にしてある
// （束 id を先着で決める実装と区別するため・要件 7.9）。
//
// - 成分 A: alpha — bravo — charlie（3 構成 id・2 対の鎖）
// - 成分 B: mike — november
// - 成分 C: xray — yankee
const ALPHA: &str = "ukadoc:page_a:alpha:1";
const BRAVO: &str = "ukadoc:page_a:bravo:1";
const CHARLIE: &str = "ukadoc:page_a:charlie:1";
const DELTA: &str = "ukadoc:page_a:delta:1";
const ECHO: &str = "ukadoc:page_a:echo:1";
const MIKE: &str = "ukadoc:page_b:mike:1";
const NOVEMBER: &str = "ukadoc:page_b:november:1";
const XRAY: &str = "ukadoc:page_c:xray:1";
const YANKEE: &str = "ukadoc:page_c:yankee:1";

/// 対の並びも各対の向きも乱した見本。
///
/// 成分が最初に現れる順は C → A → B で、束 id の昇順（A → B → C）と食い違う。
/// 各成分で最初に現れる id は yankee・bravo・november で、いずれも成分の最小値
/// ではない。成分 A は 2 つ目の対が 1 つ目の対の**前**に接ぐ向きにしてあり、
/// 繋がりを 1 歩しか辿らない実装ではここで割れる。
fn scrambled() -> Vec<(EntryId, EntryId)> {
    vec![
        edge(YANKEE, XRAY),
        edge(BRAVO, CHARLIE),
        edge(NOVEMBER, MIKE),
        edge(ALPHA, BRAVO),
    ]
}

/// [`scrambled`] と同じ関連を、別の順・一部は別の向きで並べた見本。
///
/// 対の並びは 4・1・3・2 の順に入れ替えてある。向きを裏返したのは 4 対のうち
/// 2 対（`XRAY`—`YANKEE` と `MIKE`—`NOVEMBER`）で、残る 2 対は [`scrambled`] と
/// 同じ向きのまま——成分 A の鎖の接ぎ方（後の対が前に接ぐ）を保つためである。
/// 向きを見ないことそのものは [`two_links_converging_on_the_same_id_merge`] と
/// [`two_links_leaving_the_same_id_merge`] が受け持つ。
fn shuffled() -> Vec<(EntryId, EntryId)> {
    vec![
        edge(ALPHA, BRAVO),
        edge(XRAY, YANKEE),
        edge(MIKE, NOVEMBER),
        edge(BRAVO, CHARLIE),
    ]
}

/// 見本 2 つが期待する答え。束 id は構成 id の最小値、構成 id は byte 昇順、
/// 束の並びは束 id の昇順（要件 7.9・設計 report 節の事後条件）。
fn expected() -> Vec<Bundle> {
    vec![
        Bundle {
            id: id(ALPHA),
            members: vec![id(ALPHA), id(BRAVO), id(CHARLIE)],
        },
        Bundle {
            id: id(MIKE),
            members: vec![id(MIKE), id(NOVEMBER)],
        },
        Bundle {
            id: id(XRAY),
            members: vec![id(XRAY), id(YANKEE)],
        },
    ]
}

// ---- 連結成分ごとに 1 つの束（要件 7.1 の 5 つ目・要件 7.9） ----

#[test]
fn each_connected_component_becomes_exactly_one_bundle() {
    assert_eq!(bundles(&scrambled()), expected());
}

#[test]
fn shuffling_the_input_changes_neither_the_bundle_ids_nor_the_list_order() {
    // 2 つの見本が本当に別物であることを先に主張する。同じ列を 2 回渡しただけの
    // テストは並び替えを 1 つも試していない（タスク 2.5 の教訓）。
    assert_ne!(scrambled(), shuffled());

    let from_scrambled = bundles(&scrambled());
    let from_shuffled = bundles(&shuffled());

    // 互いに等しいだけでは「同じ誤りを 2 回した」と区別が付かないので、逐語の
    // 期待値とも突き合わせる（タスク 1.3・1.4 の教訓）。
    assert_eq!(from_scrambled, expected());
    assert_eq!(from_shuffled, expected());
    assert_eq!(from_scrambled, from_shuffled);
}

#[test]
fn a_chain_through_a_shared_middle_id_merges_into_one_bundle() {
    // alpha—bravo と bravo—charlie は bravo でしか繋がっていない。
    let got = bundles(&[edge(CHARLIE, BRAVO), edge(BRAVO, ALPHA)]);
    assert_eq!(
        got,
        vec![Bundle {
            id: id(ALPHA),
            members: vec![id(ALPHA), id(BRAVO), id(CHARLIE)],
        }]
    );
}

#[test]
fn a_chain_merges_no_matter_which_end_the_later_edge_hangs_from() {
    // 上のテストと同じ 3 つの id を、**後の対が先の対に前から接ぐ**向きで渡す。
    // 繋がりを 1 歩しか辿らない実装はこの向きで割れる（上の向きでは割れない）。
    let got = bundles(&[edge(BRAVO, CHARLIE), edge(ALPHA, BRAVO)]);
    assert_eq!(
        got,
        vec![Bundle {
            id: id(ALPHA),
            members: vec![id(ALPHA), id(BRAVO), id(CHARLIE)],
        }]
    );
}

#[test]
fn two_links_converging_on_the_same_id_merge() {
    // 同じ id を**指す**対が 2 つ。台帳の日常の形である——旧い綴り 2 つが同じ
    // 正典 id を `alias_of` で指すと、対は (旧1, 新)・(旧2, 新) になる。
    //
    // 書かれた向きにしか繋がりを認めない実装はここで 2 束に割れ、`linkage.md` が
    // 引用している束 id が黙って付け替わる（要件 7.9 の安定した束 id が防ごうと
    // しているのはまさにこれ）。`bundle.rs` の「関連の向きも見ない」という主張は
    // この 1 本が背負う。
    let got = bundles(&[edge(ALPHA, BRAVO), edge(CHARLIE, BRAVO)]);
    assert_eq!(
        got,
        vec![Bundle {
            id: id(ALPHA),
            members: vec![id(ALPHA), id(BRAVO), id(CHARLIE)],
        }]
    );
}

#[test]
fn two_links_leaving_the_same_id_merge() {
    // 同じ id から**出る**対が 2 つ（1 つの正典 id が 2 つの旧 id を `supersedes`
    // する形）。上の収束形と対になる発散形で、向きの契約を両側から言う。
    let got = bundles(&[edge(BRAVO, ALPHA), edge(BRAVO, CHARLIE)]);
    assert_eq!(
        got,
        vec![Bundle {
            id: id(ALPHA),
            members: vec![id(ALPHA), id(BRAVO), id(CHARLIE)],
        }]
    );
}

#[test]
fn a_five_id_chain_fed_outward_merges_into_one_bundle() {
    // delta—echo から順に前へ前へと接いでいく 5 構成 id の鎖。
    //
    // **長さを 5 にしたのは、繋がりを決まった歩数しか辿らない実装を落とすため**で
    // ある。この向きで接ぐと繋がりの木は 1 対につき 1 段深くなるので、n 構成 id の
    // 鎖は n−1 段になる。3 構成 id（2 段）は 1 歩の実装を、4 構成 id（3 段）でも
    // 2 歩の実装は素通りした。5 構成 id なら 4 段になり、1 歩・2 歩・3 歩の
    // いずれも割れる。**短くしないこと**——縮めた分だけ通り抜ける実装が増える。
    let got = bundles(&[
        edge(DELTA, ECHO),
        edge(CHARLIE, DELTA),
        edge(BRAVO, CHARLIE),
        edge(ALPHA, BRAVO),
    ]);
    assert_eq!(
        got,
        vec![Bundle {
            id: id(ALPHA),
            members: vec![id(ALPHA), id(BRAVO), id(CHARLIE), id(DELTA), id(ECHO)],
        }]
    );
}

#[test]
fn a_four_id_chain_merges_into_one_bundle() {
    // alpha—bravo—charlie—delta を、真ん中から両端へ伸びる順で渡す。
    let got = bundles(&[
        edge(BRAVO, CHARLIE),
        edge(ALPHA, BRAVO),
        edge(CHARLIE, DELTA),
    ]);
    assert_eq!(
        got,
        vec![Bundle {
            id: id(ALPHA),
            members: vec![id(ALPHA), id(BRAVO), id(CHARLIE), id(DELTA)],
        }]
    );
}

#[test]
fn components_without_a_connecting_edge_do_not_merge() {
    let got = bundles(&[edge(CHARLIE, ALPHA), edge(YANKEE, XRAY)]);
    assert_eq!(
        got,
        vec![
            Bundle {
                id: id(ALPHA),
                members: vec![id(ALPHA), id(CHARLIE)],
            },
            Bundle {
                id: id(XRAY),
                members: vec![id(XRAY), id(YANKEE)],
            },
        ]
    );
}

// ---- 束 id は構成 id の最小値（要件 7.9） ----

#[test]
fn the_bundle_id_is_the_smallest_member_not_the_first_one_seen() {
    // 最初に現れる id は yankee で、最小の id は xray。
    let got = bundles(&[edge(YANKEE, XRAY)]);
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].id.as_str(), "ukadoc:page_c:xray:1");
    assert_ne!(got[0].id.as_str(), "ukadoc:page_c:yankee:1");
}

#[test]
fn members_are_listed_in_byte_ascending_order() {
    let got = bundles(&[edge(CHARLIE, ALPHA), edge(ALPHA, BRAVO)]);
    assert_eq!(got.len(), 1);
    let members: Vec<&str> = got[0].members.iter().map(EntryId::as_str).collect();
    assert_eq!(
        members,
        vec![
            "ukadoc:page_a:alpha:1",
            "ukadoc:page_a:bravo:1",
            "ukadoc:page_a:charlie:1",
        ]
    );
}

// ---- 端の場合を釘付けする ----

#[test]
fn no_links_yields_no_bundles() {
    assert_eq!(bundles(&[]), Vec::<Bundle>::new());
}

#[test]
fn a_link_to_itself_yields_a_bundle_of_that_id_alone() {
    // 自分を指す関連は連結成分を 1 つ（構成 id 1 つ）作る。黙って落とさない
    // ——落とすと「関連を書いたのに束に出てこない」が説明の付かない形で起きる。
    let got = bundles(&[edge(ALPHA, ALPHA)]);
    assert_eq!(
        got,
        vec![Bundle {
            id: id(ALPHA),
            members: vec![id(ALPHA)],
        }]
    );
}

#[test]
fn the_same_link_written_twice_does_not_duplicate_a_member() {
    let got = bundles(&[edge(BRAVO, ALPHA), edge(ALPHA, BRAVO), edge(BRAVO, ALPHA)]);
    assert_eq!(
        got,
        vec![Bundle {
            id: id(ALPHA),
            members: vec![id(ALPHA), id(BRAVO)],
        }]
    );
}

#[test]
fn an_id_that_appears_in_no_link_never_appears_in_a_bundle() {
    // 頂点は対に現れた id だけ。関連を持たない項目は束に出ない（要件 7.1 の
    // 「関連で繋がった束」）。ここでは mike が対に一度も現れない。
    let got = bundles(&[edge(BRAVO, ALPHA)]);
    let seen: Vec<&str> = got
        .iter()
        .flat_map(|bundle| bundle.members.iter())
        .map(EntryId::as_str)
        .collect();
    // 否定の主張だけだと対象が空でも真になるので、非空の主張と対で置く
    // （タスク 1.6 の教訓）。
    assert_eq!(seen, vec!["ukadoc:page_a:alpha:1", "ukadoc:page_a:bravo:1"]);
    assert!(!seen.contains(&"ukadoc:page_b:mike:1"));
}

// ---- 2 回続けて同じ答え（要件 7.3） ----

#[test]
fn running_twice_yields_the_same_bundles() {
    let first = bundles(&scrambled());
    let second = bundles(&scrambled());
    assert_eq!(first, second);
    assert_eq!(first, expected());
}
