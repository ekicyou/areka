//! `diff.rs` の在中テスト（要件 8.1・8.2・8.3・8.4）。
//!
//! # 見本は 2 つのカタログを手で組む
//!
//! スナップショットには 1 バイトも触らない（要件 8.4・6.2）。「新しいスナップショット
//! から作ったカタログ」に当たるものは、ここで値として組み立てた 2 つ目のカタログで
//! ある。だからこのファイルのテストは常時走るテストの中に居られる——差分の算出その
//! ものがスナップショットを要るのであって、判定は値と値の比較でしかない。
//!
//! # 4 つの一覧はすべて逐語で主張する
//!
//! 件数だけの主張は中身が全部誤っていても緑になる（タスク 1.5 の教訓）。そこで
//! **4 つの一覧を id の綴りで並べて等式にするテストを必ず通す**——1 件でも余れば、
//! 抜ければ、並びが違えば赤になる。個々のふるまいを見るテストには `contains` だけの
//! 主張もあるが、そういうテストは同じ一覧の等式を末尾に添えるか、等式を持つ
//! [`two_sample_catalogs_split_into_four_separate_lists`] と対で読むこと。
//!
//! # 「全部を挙げる」を守るために 1 一覧 2 件・間に非該当を挟む
//!
//! 違反を 1 件だけ置いた見本では「全部挙げる」と「最初の 1 件だけ挙げる」を区別
//! できない（タスク 4.2 の教訓）。見本の 4 つの一覧はいずれも 2 件以上で、しかも
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
/// ある。id 順に並べた見本は並びを 1 つも守らない（タスク 2.5 の教訓）ので、
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
///   この 1 行でだけ赤くなる（タスク 3.2・4.3 で 2 度素通りした緩め方）
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

/// 4 つの一覧が正しく分かれる（タスク 5 の完了条件そのもの）。
///
/// 4 つを 1 つの等式で主張するので、ある一覧の id が別の一覧へ紛れ込めば必ず赤に
/// なる。据え置きの 3 件（`system.month`・`OnClose`・`OnSecondChange`）はどこにも
/// 現れない。
#[test]
fn two_sample_catalogs_split_into_four_separate_lists() {
    let got = diff(&current(), &next(), &ledgers());
    assert_eq!(got, expected());
}

/// 追加は 2 件を**全部**挙げる。間に据え置きの `OnClose` を挟んであるので、
/// 「最初の 1 件で止まる」実装も「最初の据え置きで止まる」実装も赤になる。
#[test]
fn every_added_id_is_listed_not_just_the_first() {
    let got = diff(&current(), &next(), &ledgers());
    assert_eq!(got.added, ids(&[ON_CHOICE_SELECT, ON_FIRST_BOOT]));
    assert_eq!(got.added.len(), 2, "追加は 2 件のはず: {:?}", got.added);
}

/// 削除は 3 件を全部挙げる。間に据え置きの `system.month` と本文が変わった
/// `system.year` を挟んである。
#[test]
fn every_removed_id_is_listed_not_just_the_first() {
    let got = diff(&current(), &next(), &ledgers());
    assert_eq!(got.removed, ids(&[CURRENT_GHOST_NAME, SYSTEM_DAY, ON_BOOT]));
    assert_eq!(got.removed.len(), 3, "削除は 3 件のはず: {:?}", got.removed);
}

/// 本文が変わった項目は 3 件を全部挙げる。間に据え置きの `OnClose` と削除の
/// `OnBoot` を挟んである。
#[test]
fn every_changed_id_is_listed_not_just_the_first() {
    let got = diff(&current(), &next(), &ledgers());
    assert_eq!(
        got.changed,
        ids(&[SYSTEM_YEAR, ON_GHOST_CHANGED, ON_MOUSE_MOVE])
    );
    assert_eq!(got.changed.len(), 3, "変更は 3 件のはず: {:?}", got.changed);
}

/// 何も変わっていなければ 4 つの一覧はすべて空。
///
/// 「空である」だけの主張は、常に空を返す実装で無条件に真になる（タスク 1.6 の
/// 教訓）。上の非空のテストと対で置いてある。
#[test]
fn two_identical_catalogs_yield_four_empty_lists() {
    let got = diff(&current(), &current(), &ledgers());
    assert_eq!(got, CatalogDiff::default());
}

// ---------------------------------------------------------------------------
// 要件 8.2: 本文の変更はハッシュの比較だけで判じる
// ---------------------------------------------------------------------------

/// ハッシュだけが違い、ほかの列がすべて同じ項目は「本文が変わった」に挙がる。
///
/// 見本の `OnGhostChanged` は現行と新しいカタログで見出し・分類・版番号・URL が
/// 1 文字も違わず、ハッシュだけが違う。
#[test]
fn an_entry_whose_hash_alone_differs_is_changed() {
    let before = entry_of(&CURRENT_ROWS, ON_GHOST_CHANGED);
    let after = entry_of(&NEXT_ROWS, ON_GHOST_CHANGED);
    assert_eq!(before.title, after.title);
    assert_eq!(before.category, after.category);
    assert_eq!(before.versions, after.versions);
    assert_eq!(before.url, after.url);
    assert_ne!(before.hash, after.hash);

    let got = diff(&current(), &next(), &ledgers());
    assert!(
        got.changed.contains(&id(ON_GHOST_CHANGED)),
        "本文の変更に挙がっていない: {:?}",
        got.changed
    );
}

/// ハッシュが同じなら、見出しも分類も版番号も URL も変わった項目を「本文が
/// 変わった」に挙げない。
///
/// これは要件 8.2 をそのまま読んだ結果である。8.2 が比べよと言うのは**本文の
/// ハッシュ**だけで、8.1 が挙げよと言うのも「**本文**が変わった項目」だけである。
/// 見出しの改称は本文の変更ではないので、この道具は黙って見送る——見出しの
/// 移り変わりを追うのは差分の仕事ではなく、カタログを作り直したときに列がそのまま
/// 新しい値へ入れ替わる（要件 1.1）。
///
/// 見本の `OnSecondChange` は見出し・分類・版番号・URL が変わり、ハッシュだけが
/// 同じ。項目をまるごと比べる実装を当てると、このテストと
/// [`every_changed_id_is_listed_not_just_the_first`]・
/// [`two_sample_catalogs_split_into_four_separate_lists`]・
/// [`an_entry_whose_hash_differs_is_changed_even_when_every_other_column_moved_too`]
/// の 4 本が赤になる（実際に当てて確かめた。見本に `OnMouseMove` を足したので
/// 3 本から 4 本に増えた——数は当てて数え直すこと）。4 本が見ているのは見本の
/// 同じ 1 行で、その 1 行を見本から落とせばどれも何も守らなくなる。
#[test]
fn an_entry_whose_hash_is_unchanged_is_never_changed() {
    let before = entry_of(&CURRENT_ROWS, ON_SECOND_CHANGE);
    let after = entry_of(&NEXT_ROWS, ON_SECOND_CHANGE);
    assert_eq!(before.hash, after.hash);
    assert_ne!(before.title, after.title);
    assert_ne!(before.category, after.category);
    assert_ne!(before.versions, after.versions);
    assert_ne!(before.url, after.url);

    let got = diff(&current(), &next(), &ledgers());
    assert!(
        !got.changed.contains(&id(ON_SECOND_CHANGE)),
        "本文の変更に挙がってしまった: {:?}",
        got.changed
    );
    assert_eq!(
        got.changed,
        ids(&[SYSTEM_YEAR, ON_GHOST_CHANGED, ON_MOUSE_MOVE])
    );
}

/// ハッシュが違えば、見出しも分類も版番号も URL も**一緒に**変わった項目でも
/// 「本文が変わった」に挙げる。
///
/// 上の [`an_entry_whose_hash_is_unchanged_is_never_changed`] と対になる、もう
/// 片方の向きである。あちらだけでは「ハッシュだけで決める」と「ハッシュが違い、
/// **かつ見出しが同じ**なら決める」を見分けられない。後者は黙って取りこぼす向きの
/// 緩め方で、正典の改訂が節の中身と見出しを一緒に書き換えたとき、その項目は
/// `changed` から消える——台帳を見直す人は、見直しが要ることを永久に知らされない。
#[test]
fn an_entry_whose_hash_differs_is_changed_even_when_every_other_column_moved_too() {
    let before = entry_of(&CURRENT_ROWS, ON_MOUSE_MOVE);
    let after = entry_of(&NEXT_ROWS, ON_MOUSE_MOVE);
    assert_ne!(before.hash, after.hash);
    assert_ne!(before.title, after.title);
    assert_ne!(before.category, after.category);
    assert_ne!(before.versions, after.versions);
    assert_ne!(before.url, after.url);

    let got = diff(&current(), &next(), &ledgers());
    assert!(
        got.changed.contains(&id(ON_MOUSE_MOVE)),
        "本文の変更に挙がっていない: {:?}",
        got.changed
    );
    assert_eq!(
        got.changed,
        ids(&[SYSTEM_YEAR, ON_GHOST_CHANGED, ON_MOUSE_MOVE])
    );
}

/// 先頭が長く一致するハッシュどうしでも、最後の 1 桁まで見て比べる。
///
/// 見本の `OnMouseMove` の 2 つのハッシュは 16 桁のうち**先頭 15 桁が同じ**で、
/// 違うのは末尾の 1 桁だけ。先頭の何桁かだけを見る比較や桁を切り詰める比較は、
/// この項目だけを取りこぼす。この形の緩め方はこの spec で 2 度素通りしている
/// （タスク 3.2 の `starts_with`・タスク 4.3 の前方一致）ので、対の檻を置く。
#[test]
fn hashes_that_agree_on_a_long_prefix_are_still_compared_in_full() {
    let before = entry_of(&CURRENT_ROWS, ON_MOUSE_MOVE).hash;
    let after = entry_of(&NEXT_ROWS, ON_MOUSE_MOVE).hash;
    assert_eq!(before.len(), 16, "見本のハッシュは 16 桁のはず");
    assert_eq!(after.len(), 16, "見本のハッシュは 16 桁のはず");
    assert_eq!(before[..15], after[..15], "先頭 15 桁は同じはず");
    assert_ne!(before, after, "末尾の 1 桁は違うはず");

    let got = diff(&current(), &next(), &ledgers());
    assert!(
        got.changed.contains(&id(ON_MOUSE_MOVE)),
        "末尾 1 桁の違いを見落とした: {:?}",
        got.changed
    );
}

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

/// 台帳に現れる削除 id は 2 件を全部挙げ、台帳に無い削除 id は挙げない。
///
/// 見本の削除 3 件のうち `system.day` はどの台帳にも無い。しかもこの 1 件は
/// 挙がる 2 件の**間**に並ぶ id なので、「最初の 1 件だけ」も「削除をそのまま
/// 写す」も赤になる。
#[test]
fn only_removed_ids_present_in_a_ledger_need_review() {
    let got = diff(&current(), &next(), &ledgers());
    assert_eq!(got.removed_in_ledger, ids(&[CURRENT_GHOST_NAME, ON_BOOT]));
    assert_eq!(
        got.removed_in_ledger.len(),
        2,
        "見直しが要るのは 2 件のはず: {:?}",
        got.removed_in_ledger
    );
    assert!(
        !got.removed_in_ledger.contains(&id(SYSTEM_DAY)),
        "どの台帳にも無い削除 id が挙がった: {:?}",
        got.removed_in_ledger
    );
}

/// 見直しの一覧は削除の一覧から**引かない**。同じ id が両方に載る。
///
/// 要件 8.3 は「差分に削除された項目が含まれ、かつその id が台帳に現れる」ときに
/// その id を別に**明示する**と言う。つまり削除の一覧に載っていることが条件の側で
/// あって、明示は削除の一覧に**加える**印である。取り分けてしまうと「消えた項目の
/// 全部」を知りたい読み手が 2 つの一覧を足し合わせる羽目になる。
#[test]
fn an_id_needing_review_stays_in_the_removed_list_too() {
    let got = diff(&current(), &next(), &ledgers());
    for target in [CURRENT_GHOST_NAME, ON_BOOT] {
        assert!(
            got.removed.contains(&id(target)),
            "{target} が削除の一覧から抜けている: {:?}",
            got.removed
        );
        assert!(
            got.removed_in_ledger.contains(&id(target)),
            "{target} が見直しの一覧に無い: {:?}",
            got.removed_in_ledger
        );
    }
}

/// 台帳にあっても削除されていない id は見直しの一覧に入らない。
///
/// 見本の台帳は据え置きの `OnClose`・`system.month`・`OnSecondChange` と本文が
/// 変わった `system.year`・`OnGhostChanged` を持つ。「台帳の id を全部挙げる」
/// 実装はここで赤になる。
#[test]
fn a_ledger_id_that_was_not_removed_never_needs_review() {
    let got = diff(&current(), &next(), &ledgers());
    for target in [
        ON_CLOSE,
        SYSTEM_MONTH,
        ON_SECOND_CHANGE,
        SYSTEM_YEAR,
        ON_GHOST_CHANGED,
        ON_MOUSE_MOVE,
    ] {
        assert!(
            !got.removed_in_ledger.contains(&id(target)),
            "{target} が見直しの一覧に紛れ込んだ: {:?}",
            got.removed_in_ledger
        );
    }
    assert_eq!(got.removed_in_ledger, ids(&[CURRENT_GHOST_NAME, ON_BOOT]));
}

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

/// 4 つの一覧はいずれも id の byte 昇順で、逆順でも入力順でもない。
///
/// 期待値を綴りで並べた上の各テストがすでにこれを固定しているが、ここでは
/// 「並んでいること」を一覧そのものから確かめる。一覧の作り方を変えたときに、
/// 綴りの表を書き換えただけで緑に戻ることを防ぐ。
#[test]
fn all_four_lists_are_sorted_by_id() {
    let got = diff(&current(), &next(), &ledgers());
    for (name, list) in [
        ("added", &got.added),
        ("removed", &got.removed),
        ("changed", &got.changed),
        ("removed_in_ledger", &got.removed_in_ledger),
    ] {
        let mut sorted = list.clone();
        sorted.sort();
        assert_eq!(*list, sorted, "{name} が昇順でない: {list:?}");
        assert!(list.len() >= 2, "{name} は 2 件以上の見本のはず");
    }
}
