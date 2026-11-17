//! `diff.rs` の在中テスト（要件 8.1・8.2・8.3・8.4）。
//!
//! # 見本は 2 つのカタログを手で組む
//!
//! スナップショットには 1 バイトも触らない（要件 8.4・6.2）。「新しいスナップショット
//! から作ったカタログ」に当たるものは、ここで値として組み立てた 2 つ目のカタログで
//! ある。だからこのファイルのテストは常時走るテストの中に居られる——新しいカタログを
//! 建てる段がスナップショットを要るのであって、差分の算出は値と値の比較でしかない。
//!
//! # 4 つの一覧はすべて逐語で主張する
//!
//! 件数だけの主張は中身が全部誤っていても緑になる。そこで 4 つの一覧を id の綴りで
//! 並べて等式にする [`two_sample_catalogs_split_into_four_separate_lists`] を通す。
//! 見本そのものの形は [`sample_shape_premises_hold`] が固定する。
//!
//! # 「全部を挙げる」を守るために 1 一覧 2 件・間に非該当を挟む
//!
//! 違反を 1 件だけ置いた見本では「全部挙げる」と「最初の 1 件だけ挙げる」を区別
//! できない。見本の 4 つの一覧はいずれも 2 件以上で、しかも
//! **その 2 件の間に一覧へ入らない id を挟んである**（下の id 一覧の表を見よ）。

use super::{CatalogDiff, diff};
use crate::catalog::{CATALOG_FORMAT, Catalog, CatalogEntry, SnapshotMeta};
use crate::hash::HASH_ALGORITHM;
use crate::ledger::{Ledger, LedgerEntry};
use crate::model::{Domain, EntryId, PageName, Status};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// 見本の id
//
// byte 昇順は次のとおり（`ukadoc:list_p...` < `ukadoc:list_s...`）。
//
//   1. currentghost.name  削除・台帳にある（property）
//   2. system.day         削除・どの台帳にも無い
//   3. system.month       据え置き
//   4. system.year        本文が変わった
//   5. OnBoot             削除・台帳にある（shiori）
//   6. OnChoiceSelect     追加
//   7. OnClose            据え置き
//   8. OnFirstBoot        追加
//   9. OnGhostChanged     本文が変わった（ほかの列は 1 文字も動かない）
//  10. OnMouseMove       本文が変わった（見出しも分類も版番号も URL も一緒に動く・
//                        しかも 2 つのハッシュは先頭 15 桁が同じ）
//  11. OnSecondChange     本文は同じ・見出しと分類と版番号だけ変わった
// ---------------------------------------------------------------------------

const CURRENT_GHOST_NAME: &str = "ukadoc:list_propertysystem:currentghost.name:1";
const SYSTEM_DAY: &str = "ukadoc:list_propertysystem:system.day:1";
const SYSTEM_MONTH: &str = "ukadoc:list_propertysystem:system.month:1";
const SYSTEM_YEAR: &str = "ukadoc:list_propertysystem:system.year:1";
const ON_BOOT: &str = "ukadoc:list_shiori_event:OnBoot:1";
const ON_CHOICE_SELECT: &str = "ukadoc:list_shiori_event:OnChoiceSelect:1";
const ON_CLOSE: &str = "ukadoc:list_shiori_event:OnClose:1";
const ON_FIRST_BOOT: &str = "ukadoc:list_shiori_event:OnFirstBoot:1";
const ON_GHOST_CHANGED: &str = "ukadoc:list_shiori_event:OnGhostChanged:1";
const ON_MOUSE_MOVE: &str = "ukadoc:list_shiori_event:OnMouseMove:1";
const ON_SECOND_CHANGE: &str = "ukadoc:list_shiori_event:OnSecondChange:1";

/// 正典 URL の根。見出しから機械で導く（手で書くと id と食い違った見本が作れる）。
const URL_BASE: &str = "https://ssp.shillest.net/ukadoc/manual/";

/// 見本のカタログ 1 行。
struct Row {
    id: &'static str,
    title: &'static str,
    category: &'static str,
    versions: &'static [&'static str],
    hash: &'static str,
}

/// 現行のカタログ 9 件。
///
/// 表の並びは id 順にしていない。ただし**この並びは何も守らない**——
/// [`Catalog::entries`] は `BTreeMap` なので、ここに書いた順は組み立てた時点で
/// 消える。カタログ側の並びは作りからして昇順で、取り違えようがない。
///
/// 並びの取り決めを本当に守っているのは、①期待値を綴りで並べた各テスト（逆順で
/// 返す実装が赤になる）と、②台帳を **id の順と逆**に渡す [`ledgers`] の 2 つで
/// ある。id 順に並べた見本は並びを 1 つも守らないので、
/// 守りは見本の並びではなくこの 2 つに載せてある。
const CURRENT_ROWS: [Row; 9] = [
    Row {
        id: ON_SECOND_CHANGE,
        title: "OnSecondChange",
        category: "shiori_event",
        versions: &["2.3.53"],
        hash: "1111111111111111",
    },
    Row {
        id: SYSTEM_YEAR,
        title: "system.year",
        category: "propertysystem",
        versions: &["2.3.53"],
        hash: "2222222222222222",
    },
    Row {
        id: ON_BOOT,
        title: "OnBoot",
        category: "shiori_event",
        versions: &["2.3.53"],
        hash: "3333333333333333",
    },
    Row {
        id: CURRENT_GHOST_NAME,
        title: "currentghost.name",
        category: "propertysystem",
        versions: &[],
        hash: "4444444444444444",
    },
    Row {
        id: ON_GHOST_CHANGED,
        title: "OnGhostChanged",
        category: "shiori_event",
        versions: &["2.5.60"],
        hash: "5555555555555555",
    },
    Row {
        id: SYSTEM_DAY,
        title: "system.day",
        category: "propertysystem",
        versions: &[],
        hash: "6666666666666666",
    },
    Row {
        id: ON_CLOSE,
        title: "OnClose",
        category: "shiori_event",
        versions: &["2.3.53", "2.5.60"],
        hash: "7777777777777777",
    },
    Row {
        id: SYSTEM_MONTH,
        title: "system.month",
        category: "propertysystem",
        versions: &["2.3.53"],
        hash: "8888888888888888",
    },
    Row {
        id: ON_MOUSE_MOVE,
        title: "OnMouseMove",
        category: "shiori_event",
        versions: &["2.3.53"],
        hash: "eeeeeeee00000000",
    },
];

/// 新しいスナップショットから作ったことにするカタログ 8 件。
///
/// 現行との違いは 5 通りある。最後の 2 つは**ハッシュだけを見る**という取り決めの
/// 両向きで、片方だけでは判定を 1 方向にしか縛れない。
///
/// - `OnChoiceSelect`・`OnFirstBoot` が増えた（追加）
/// - `currentghost.name`・`system.day`・`OnBoot` が消えた（削除）
/// - `system.year`・`OnGhostChanged` の本文のハッシュが変わった。ほかの列は 1 文字も
///   動かない（本文の変更）
/// - `OnSecondChange` は**ハッシュだけが同じ**で見出し・分類・版番号（したがって
///   URL も）が変わった。本文は変わっていないので、どの一覧にも入らない（要件 8.2）
/// - `OnMouseMove` は**ハッシュも見出しも分類も版番号も URL も**変わった。正典の
///   改訂で節の中身と見出しが一緒に書き換わる形で、本文が変わった以上は挙げる。
///   しかも 2 つのハッシュ（`eeeeeeee00000000` と `eeeeeeee00000001`）は**先頭
///   15 桁が同じ**で、最後の 1 桁だけが違う。前方一致や桁の切り詰めに緩めた比較は
///   この 1 行でだけ赤くなる（過去に 2 度素通りした緩め方）
const NEXT_ROWS: [Row; 8] = [
    Row {
        id: ON_FIRST_BOOT,
        title: "OnFirstBoot",
        category: "shiori_event",
        versions: &["2.3.53"],
        hash: "aaaaaaaaaaaaaaaa",
    },
    Row {
        id: ON_SECOND_CHANGE,
        title: "OnSecondChange（改称）",
        category: "protocol",
        versions: &["2.3.53", "2.5.60"],
        hash: "1111111111111111",
    },
    Row {
        id: SYSTEM_YEAR,
        title: "system.year",
        category: "propertysystem",
        versions: &["2.3.53"],
        hash: "cccccccccccccccc",
    },
    Row {
        id: ON_CHOICE_SELECT,
        title: "OnChoiceSelect",
        category: "shiori_event",
        versions: &["2.5.60"],
        hash: "bbbbbbbbbbbbbbbb",
    },
    Row {
        id: ON_GHOST_CHANGED,
        title: "OnGhostChanged",
        category: "shiori_event",
        versions: &["2.5.60"],
        hash: "dddddddddddddddd",
    },
    Row {
        id: ON_CLOSE,
        title: "OnClose",
        category: "shiori_event",
        versions: &["2.3.53", "2.5.60"],
        hash: "7777777777777777",
    },
    Row {
        id: SYSTEM_MONTH,
        title: "system.month",
        category: "propertysystem",
        versions: &["2.3.53"],
        hash: "8888888888888888",
    },
    Row {
        id: ON_MOUSE_MOVE,
        title: "OnMouseMove（改称）",
        category: "protocol",
        versions: &["2.3.53", "2.5.60"],
        hash: "eeeeeeee00000001",
    },
];

/// 見本の id を作る。
fn id(raw: &str) -> EntryId {
    EntryId::parse(raw).expect("見本の id は要件 1.9 の 2 形のいずれかのはず")
}

/// id の一覧を作る（期待値の組み立て用）。
fn ids(raws: &[&str]) -> Vec<EntryId> {
    raws.iter().map(|raw| id(raw)).collect()
}

/// カタログ冒頭のスナップショット情報。差分は冒頭を見ないので、2 つのカタログで
/// 違うのは生成日時だけにしてある。
fn meta(generated_at: &str) -> SnapshotMeta {
    SnapshotMeta {
        package: "ukagaka-doc-mcp".to_owned(),
        package_version: "0.2.7".to_owned(),
        snapshot_version: 1,
        generated_at: generated_at.to_owned(),
        total_entries: 2_983,
        ukadoc_entries: 1_749,
        catalog_format: CATALOG_FORMAT,
        hash_algorithm: HASH_ALGORITHM.to_owned(),
    }
}

/// 1 行からカタログの項目を組む。ページと URL は id と見出しから導く。
fn entry(row: &Row) -> CatalogEntry {
    let entry_id = id(row.id);
    let page = entry_id.page();
    CatalogEntry {
        url: format!("{URL_BASE}{}.html#{}", page.as_str(), row.title),
        page,
        title: row.title.to_owned(),
        category: row.category.to_owned(),
        versions: row.versions.iter().map(|v| (*v).to_owned()).collect(),
        hash: row.hash.to_owned(),
        id: entry_id,
    }
}

/// 行の表からその id の項目を組む。
///
/// **添字では引かない**。見本に 1 行足しただけで別の項目を指し、テストが黙って
/// 別のことを主張してしまう。
fn entry_of(rows: &[Row], target: &str) -> CatalogEntry {
    let row = rows
        .iter()
        .find(|row| row.id == target)
        .expect("見本の表にその id が無い");
    entry(row)
}

/// 行の並びからカタログを組む。
fn catalog(rows: &[Row], generated_at: &str) -> Catalog {
    let mut entries = BTreeMap::new();
    for row in rows {
        let built = entry(row);
        entries.insert(built.id.clone(), built);
    }
    Catalog {
        snapshot: meta(generated_at),
        entries,
    }
}

/// 現行のカタログ。
fn current() -> Catalog {
    catalog(&CURRENT_ROWS, "2026-08-24T04:08:57.881Z")
}

/// 新しいカタログ。
fn next() -> Catalog {
    catalog(&NEXT_ROWS, "2026-09-04T00:00:00.000Z")
}

/// 台帳の 1 項目（既定の欄だけ）。
fn ledger_entry(raw: &str) -> LedgerEntry {
    LedgerEntry {
        id: id(raw),
        status: Status::Unclassified,
        introduced: String::new(),
        alias_of: None,
        supersedes: Vec::new(),
        owner: String::new(),
        priority: String::new(),
        values: Vec::new(),
        links: Vec::new(),
        note: String::new(),
    }
}

/// 台帳を組む。
fn ledger(domain: Domain, page: &str, raws: &[&str]) -> Ledger {
    let mut entries = BTreeMap::new();
    for raw in raws {
        let built = ledger_entry(raw);
        entries.insert(built.id.clone(), built);
    }
    let file_order = entries.keys().cloned().collect();
    Ledger {
        domain,
        pages: vec![PageName::new(page)],
        entries,
        file_order,
    }
}

/// 見本の台帳 2 本。
///
/// **並びは shiori・property の順**で、id の順（property が先）と逆にしてある。
/// 「台帳ごとに拾って並べる」実装はこの並びで初めて赤になる——台帳の順が id の順と
/// 同じ見本では、間違った並べ方が正しい答えを出してしまう。
///
/// `system.day` はどちらの台帳にも置かない。台帳は人が手で書く側なので、カタログに
/// あってまだ記入されていない id は現実に起こる（要件 2.1）。要件 8.3 の「かつその
/// id が台帳に現れる」という条件は、この行が無ければ試せない。
fn ledgers() -> Vec<Ledger> {
    vec![
        ledger(
            Domain::Shiori,
            "list_shiori_event",
            &[
                ON_BOOT,
                ON_CLOSE,
                ON_GHOST_CHANGED,
                ON_MOUSE_MOVE,
                ON_SECOND_CHANGE,
            ],
        ),
        ledger(
            Domain::Property,
            "list_propertysystem",
            &[CURRENT_GHOST_NAME, SYSTEM_MONTH, SYSTEM_YEAR],
        ),
    ]
}

/// 見本に対する期待の答え。
fn expected() -> CatalogDiff {
    CatalogDiff {
        added: ids(&[ON_CHOICE_SELECT, ON_FIRST_BOOT]),
        removed: ids(&[CURRENT_GHOST_NAME, SYSTEM_DAY, ON_BOOT]),
        changed: ids(&[SYSTEM_YEAR, ON_GHOST_CHANGED, ON_MOUSE_MOVE]),
        removed_in_ledger: ids(&[CURRENT_GHOST_NAME, ON_BOOT]),
    }
}

// ---------------------------------------------------------------------------
// 要件 8.1: 追加・削除・本文の変更を id 付きで列挙し、互いに混ぜない
// ---------------------------------------------------------------------------

/// 4 つの一覧が正しく分かれる（差分の核となる主張）。
///
/// 4 つを 1 つの等式で主張するので、ある一覧の id が別の一覧へ紛れ込めば必ず赤に
/// なる。据え置きの 3 件（`system.month`・`OnClose`・`OnSecondChange`）はどこにも
/// 現れない。
#[test]
fn two_sample_catalogs_split_into_four_separate_lists() {
    let got = diff(&current(), &next(), &ledgers());
    assert_eq!(got, expected());
}

/// 何も変わっていなければ 4 つの一覧はすべて空。
///
/// 「空である」だけの主張は、常に空を返す実装で無条件に真になる。上の非空のテストと対で置いてある。
#[test]
fn two_identical_catalogs_yield_four_empty_lists() {
    let got = diff(&current(), &current(), &ledgers());
    assert_eq!(got, CatalogDiff::default());
}

/// 見本の形の前提。上の等式が「何も守らない見本」で緑にならないよう、見本そのものの
/// 形を 1 か所で固定する。
///
/// - `OnMouseMove` の 2 つのハッシュは 16 桁のうち先頭 15 桁が同じで、末尾の 1 桁だけが
///   違う
/// - `OnGhostChanged` はハッシュだけが違い、見出し・分類・版番号・URL は同じ（要件 8.1）
/// - `OnSecondChange` はハッシュだけが同じで、見出し・分類・版番号・URL が動いた
///   （要件 8.2）
/// - `OnMouseMove` はハッシュも見出しも分類も版番号も URL も動いた
/// - 4 つの一覧の期待値はいずれも 2 件以上で、id の byte 昇順（要件 7.3）
///
/// 比べ方を取り違えた実装を当てて確かめてある。項目をまるごと比べる実装も、ハッシュの
/// 先頭 15 桁だけを比べる実装も、赤になるのは
/// [`two_sample_catalogs_split_into_four_separate_lists`] の 1 本だけである。大小を
/// 無視する比較を赤にするのは `hashes_differing_only_in_letter_case_are_not_the_same_hash`
/// の 1 本だけ。
#[test]
fn sample_shape_premises_hold() {
    let hash_only = (
        entry_of(&CURRENT_ROWS, ON_GHOST_CHANGED),
        entry_of(&NEXT_ROWS, ON_GHOST_CHANGED),
    );
    assert_eq!(hash_only.0.title, hash_only.1.title);
    assert_eq!(hash_only.0.category, hash_only.1.category);
    assert_eq!(hash_only.0.versions, hash_only.1.versions);
    assert_eq!(hash_only.0.url, hash_only.1.url);
    assert_ne!(hash_only.0.hash, hash_only.1.hash);

    let same_hash = (
        entry_of(&CURRENT_ROWS, ON_SECOND_CHANGE),
        entry_of(&NEXT_ROWS, ON_SECOND_CHANGE),
    );
    assert_eq!(same_hash.0.hash, same_hash.1.hash);
    assert_ne!(same_hash.0.title, same_hash.1.title);
    assert_ne!(same_hash.0.category, same_hash.1.category);
    assert_ne!(same_hash.0.versions, same_hash.1.versions);
    assert_ne!(same_hash.0.url, same_hash.1.url);

    let all_moved = (
        entry_of(&CURRENT_ROWS, ON_MOUSE_MOVE),
        entry_of(&NEXT_ROWS, ON_MOUSE_MOVE),
    );
    assert_ne!(all_moved.0.title, all_moved.1.title);
    assert_ne!(all_moved.0.category, all_moved.1.category);
    assert_ne!(all_moved.0.versions, all_moved.1.versions);
    assert_ne!(all_moved.0.url, all_moved.1.url);
    assert_eq!(all_moved.0.hash.len(), 16, "見本のハッシュは 16 桁のはず");
    assert_eq!(all_moved.1.hash.len(), 16, "見本のハッシュは 16 桁のはず");
    assert_eq!(all_moved.0.hash[..15], all_moved.1.hash[..15]);
    assert_ne!(all_moved.0.hash, all_moved.1.hash);

    let want = expected();
    for (name, list) in [
        ("added", &want.added),
        ("removed", &want.removed),
        ("changed", &want.changed),
        ("removed_in_ledger", &want.removed_in_ledger),
    ] {
        let mut sorted = list.clone();
        sorted.sort();
        assert_eq!(*list, sorted, "{name} の期待値が昇順でない: {list:?}");
        assert!(list.len() >= 2, "{name} は 2 件以上の見本のはず");
    }
}

// ---------------------------------------------------------------------------
// 要件 8.2: 本文の変更はハッシュの比較だけで判じる
// ---------------------------------------------------------------------------

/// 16 進の大小だけが違うハッシュも「違う」と見る。
///
/// `hash.rs` が作る印は必ず 16 桁の 16 進**小文字**なので、カタログを機械で作って
/// いる限りこの形は現れない。それでも檻を置くのは、現行のカタログが**ファイルから
/// 読んだ値**だからである——`catalog::read` は `hash` の綴りを大小そのままに持つ。
/// 大小を無視する比較は前方一致と同じ「緩める向き」の取り違えで、当てても何も
/// 赤くならない状態にはしておかない。
///
/// 見本は主の 2 つのカタログとは別に、この 1 件だけで組む（主の見本に大小違いの
/// 行を足すと、ほかのテストの期待値がすべて動く）。
///
/// **整理でこのテストを畳まないこと。** 大小を無視する比較を赤にするのはこの 1 本だけで
/// （`eq_ignore_ascii_case` を当てて実測した）、隣のテストへ吸収すると守りが消える。
#[test]
fn hashes_differing_only_in_letter_case_are_not_the_same_hash() {
    const LOWER: [Row; 1] = [Row {
        id: ON_CLOSE,
        title: "OnClose",
        category: "shiori_event",
        versions: &["2.3.53"],
        hash: "abcdefabcdef1234",
    }];
    const UPPER: [Row; 1] = [Row {
        id: ON_CLOSE,
        title: "OnClose",
        category: "shiori_event",
        versions: &["2.3.53"],
        hash: "ABCDEFABCDEF1234",
    }];

    let got = diff(
        &catalog(&LOWER, "2026-08-24T04:08:57.881Z"),
        &catalog(&UPPER, "2026-09-04T00:00:00.000Z"),
        &ledgers(),
    );
    assert_eq!(
        got,
        CatalogDiff {
            added: ids(&[]),
            removed: ids(&[]),
            changed: ids(&[ON_CLOSE]),
            removed_in_ledger: ids(&[]),
        }
    );
}

// ---------------------------------------------------------------------------
// 要件 8.3: 削除された id が台帳に現れるなら別に挙げる
// ---------------------------------------------------------------------------

/// 同じ id が 2 本の台帳にあっても、見直しの一覧には 1 度だけ現れる。
///
/// 台帳ごとに拾って継ぎ足す実装はここで赤になる。（同じ id が 2 本の台帳に載る
/// のは整合検査が別に咎める形だが〔要件 3.2〕、差分はそれを当てにしない。）
#[test]
fn an_id_in_two_ledgers_needs_review_only_once() {
    let mut two = ledgers();
    two[1].entries.insert(id(ON_BOOT), ledger_entry(ON_BOOT));
    two[1].file_order = two[1].entries.keys().cloned().collect();

    let got = diff(&current(), &next(), &two);
    assert_eq!(got.removed_in_ledger, ids(&[CURRENT_GHOST_NAME, ON_BOOT]));
}

/// 台帳が 1 本も無ければ見直しの一覧は空。削除の一覧は変わらない。
#[test]
fn with_no_ledgers_nothing_needs_review() {
    let got = diff(&current(), &next(), &[]);
    assert_eq!(got.removed_in_ledger, Vec::new());
    assert_eq!(got.removed, ids(&[CURRENT_GHOST_NAME, SYSTEM_DAY, ON_BOOT]));
}

// ---------------------------------------------------------------------------
// 並び（要件 7.3 の決まり方を差分にも通す）
// ---------------------------------------------------------------------------

/// 見直しの一覧は **id の順**で、台帳を渡した順ではない。
///
/// 見本の台帳は shiori・property の順に並べてあり、id の順（`ukadoc:list_p…` が
/// 先）と逆である。台帳ごとに拾って並べる実装は `OnBoot` を先に置くのでここで
/// 赤になる。台帳の並びを逆にしても答えは 1 バイトも変わらない。
#[test]
fn the_review_list_follows_id_order_not_ledger_order() {
    let forward = ledgers();
    let mut backward = ledgers();
    backward.reverse();
    assert_eq!(forward[0].domain, Domain::Shiori, "見本の並びの前提");
    assert_eq!(backward[0].domain, Domain::Property, "見本の並びの前提");

    let a = diff(&current(), &next(), &forward);
    let b = diff(&current(), &next(), &backward);
    assert_eq!(a.removed_in_ledger, ids(&[CURRENT_GHOST_NAME, ON_BOOT]));
    assert_eq!(a, b, "台帳の並びで答えが変わってはいけない");
}
