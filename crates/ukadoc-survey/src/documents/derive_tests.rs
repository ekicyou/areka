//! [`super`]（機械で決まる値の導出）の在中テスト。
//!
//! 相手にするのは**テストの中で組み立てた値だけ**である。`doc/ukadoc-coverage/` の
//! 実ファイルは 1 行も読まない——実データを相手にする判定は統合テストの担当で
//! （設計 D-3）、ここで読むと文書の書きかけがそのまま純粋層のテストを赤くする。
//!
//! 骨組みも [`super::super::parse`] を通さずに直に組む。読み手を通すと、読み手が
//! まだ落としていない形（単独項目を `bundle` で指す等）を試せない。
//!
//! 守るのはタスク 1.3 の完了状態の 4 つと、落ちるべき経路の全部である。
//!
//! - 導出の鍵が 4 台帳の全 id と一致すること
//! - 帰属の無い id で落ちること
//! - 比較鍵の序列（壊れ方 ＞ テーマ数 ＞ 資産 ＞ 共有度）
//! - 共有度が自分を含むこと

use super::*;
use crate::documents::{PriorityBlank, RankRow, Stage, StageCount, Tally, Template};
use crate::ledger::LedgerEntry;
use crate::model::PageName;

// ---------------------------------------------------------------------------
// 見本を組み立てる道具
// ---------------------------------------------------------------------------

/// 見本の id。
fn id(raw: &str) -> EntryId {
    EntryId::parse(raw).expect("見本の id は要件 1.9 の 2 形のいずれかのはず")
}

/// 名前付き束。`machine`・`hand`・`domains`・`themes` は導出が読まない欄なので、
/// **わざと空**にしてある——導出がそこを読んでいたら赤くなる。
fn named(name: &str, foundation: &str, breakage: Breakage, members: &[&str]) -> NamedBundle {
    NamedBundle {
        name: name.to_owned(),
        single: false,
        machine: Vec::new(),
        members: members.iter().map(|raw| id(raw)).collect(),
        hand: Vec::new(),
        domains: Vec::new(),
        foundation: foundation.to_owned(),
        breakage,
        themes: Vec::new(),
        reason: String::new(),
    }
}

/// 単独項目。名前は id そのもので、`foundation` は持たない（読み手が禁じている）。
fn single_of(raw: &str, breakage: Breakage) -> NamedBundle {
    NamedBundle {
        name: raw.to_owned(),
        single: true,
        machine: Vec::new(),
        members: vec![id(raw)],
        hand: Vec::new(),
        domains: Vec::new(),
        foundation: String::new(),
        breakage,
        themes: Vec::new(),
        reason: "束の振る舞いを書けない".to_owned(),
    }
}

/// `[tally]` は導出が読まない。数え直しは判定 ⑶-f の担当なので 0 で埋める。
fn linkage_of(bundles: Vec<NamedBundle>) -> Linkage {
    Linkage {
        bundles: bundles
            .into_iter()
            .map(|bundle| (bundle.name.clone(), bundle))
            .collect(),
        tally: Tally {
            target: 0,
            from_machine: 0,
            by_hand: 0,
            singles: 0,
            alias_excluded: 0,
            not_applicable_excluded: 0,
            singles_by_domain: Domain::ALL.into_iter().map(|domain| (domain, 0)).collect(),
        },
    }
}

/// 順位の行 1 つ。
fn rank_of(stage: Stage, rank: usize, target: RankTarget) -> RankRow {
    RankRow {
        stage,
        rank,
        target,
        assets: 0,
        shared: 0,
        exception: None,
        insufficient: false,
    }
}

/// 束を名指す順位の行。
fn bundle_row(stage: Stage, rank: usize, name: &str) -> RankRow {
    rank_of(stage, rank, RankTarget::Bundle(name.to_owned()))
}

/// 単独項目を並べる順位の行。
fn singles_row(stage: Stage, rank: usize, raws: &[&str]) -> RankRow {
    rank_of(
        stage,
        rank,
        RankTarget::Singles(raws.iter().map(|raw| id(raw)).collect()),
    )
}

/// 見本の `briefing.md`。導出が読むのは `ranks` と `templates` だけなので、
/// 残りは 0 と空で埋める。
fn briefing_of(ranks: Vec<RankRow>, template_ids: &[&str]) -> Briefing {
    Briefing {
        ranks,
        stages: Stage::ALL
            .into_iter()
            .map(|stage| {
                (
                    stage,
                    StageCount {
                        bundles: 0,
                        singles: 0,
                        items: 0,
                    },
                )
            })
            .collect(),
        barriers: Vec::new(),
        afters: Vec::new(),
        priority_blank: PriorityBlank {
            alias: 0,
            not_applicable: 0,
        },
        owner_completed: Vec::new(),
        templates: vec![Template {
            name: "ポストと狛犬".to_owned(),
            shiori: "里々".to_owned(),
            url: "https://example.invalid/".to_owned(),
            fetched_on: "2026-09-12".to_owned(),
            files: vec!["dic.txt".to_owned()],
            mapping: "見出し名の一致".to_owned(),
            fallback: false,
            ids: template_ids.iter().map(|raw| id(raw)).collect(),
        }],
    }
}

/// 台帳の 1 項目。導出が読むのは `status` と `values` だけ。
fn entry_of(raw: &str, status: Status, values: &[&str]) -> LedgerEntry {
    LedgerEntry {
        id: id(raw),
        status,
        introduced: String::new(),
        alias_of: None,
        supersedes: Vec::new(),
        owner: String::new(),
        priority: "Z9".to_owned(),
        values: values.iter().map(|value| (*value).to_owned()).collect(),
        links: Vec::new(),
        note: String::new(),
    }
}

/// 台帳 1 本。
fn ledger_of(domain: Domain, entries: Vec<LedgerEntry>) -> Ledger {
    let file_order: Vec<EntryId> = entries.iter().map(|entry| entry.id.clone()).collect();
    Ledger {
        domain,
        pages: vec![PageName::new("list_shiori_event")],
        entries: entries
            .into_iter()
            .map(|entry| (entry.id.clone(), entry))
            .collect(),
        file_order,
    }
}

// ---------------------------------------------------------------------------
// 見本の世界
// ---------------------------------------------------------------------------

const ON_BOOT: &str = "ukadoc:list_shiori_event:OnBoot:1";
const ON_MINUTE: &str = "ukadoc:list_shiori_event:OnMinuteChange:1";
const ON_ALIAS: &str = "ukadoc:list_shiori_event:OnOldName:1";
const ON_LONELY: &str = "ukadoc:list_shiori_event:OnLonely:1";
const INTERVAL: &str = "ukadoc:descript_plugin:secondchangeinterval:1";
const NOT_TARGET: &str = "ukadoc:descript_shell:nothing:1";

/// 束の名前。
const CLOCK: &str = "時刻の刻み";
/// 基盤の見出し。
const CLOCK_FOUNDATION: &str = "時刻イベントの発火路";

/// 見本の 2 台帳。対象 4 状態・別名・対象外を 1 件ずつ持つ。
fn ledgers() -> Vec<Ledger> {
    vec![
        ledger_of(
            Domain::Shiori,
            vec![
                entry_of(ON_BOOT, Status::Implemented, &["気配"]),
                entry_of(ON_MINUTE, Status::Absent, &["気配", "更新"]),
                entry_of(ON_ALIAS, Status::Alias, &[]),
                entry_of(ON_LONELY, Status::VocabularyOnly, &["掛け合い"]),
            ],
        ),
        ledger_of(
            Domain::Assets,
            vec![
                entry_of(INTERVAL, Status::Degraded, &["気配"]),
                entry_of(NOT_TARGET, Status::NotApplicable, &[]),
            ],
        ),
    ]
}

/// 見本の帰属。名前付き束 1 つと単独項目 1 つで、対象 4 件をちょうど分ける。
fn linkage() -> Linkage {
    linkage_of(vec![
        named(
            CLOCK,
            CLOCK_FOUNDATION,
            Breakage::Silent,
            &[ON_BOOT, ON_MINUTE, INTERVAL],
        ),
        single_of(ON_LONELY, Breakage::Visual),
    ])
}

/// 見本の順位。段階 E の順位を **2** にしてあるのは、導出が番号を振り直さず
/// 書かれた数をそのまま写すことを釘付けするためである（密な順位は文書の側の主張）。
fn briefing() -> Briefing {
    briefing_of(
        vec![
            bundle_row(Stage::A, 1, CLOCK),
            singles_row(Stage::E, 2, &[ON_LONELY]),
        ],
        &[ON_BOOT, "ukadoc:list_shiori_event:OnNotAMember:1"],
    )
}

/// 失敗の本文。緑のときは何が返ったかを見せて落ちる。
fn failure(result: Result<BTreeMap<EntryId, String>, SurveyError>) -> String {
    match result {
        Ok(values) => panic!("落ちるはずが緑になった: {values:?}"),
        Err(err) => err.to_string(),
    }
}

// ---------------------------------------------------------------------------
// priorities — 鍵と値
// ---------------------------------------------------------------------------

/// 導出の鍵は台帳の全 id と**一致**する（部分集合ではない）。値も逐語で見る
/// ——鍵だけを数えると、全部が空文字でも緑になる。
#[test]
fn the_keys_match_every_id_in_the_ledgers_and_the_values_are_the_stage_and_the_rank() {
    let derived = priorities(&linkage(), &briefing(), &ledgers()).expect("見本は導けるはず");

    let keys: Vec<&str> = derived.keys().map(EntryId::as_str).collect();
    let mut wanted = vec![
        INTERVAL, NOT_TARGET, ON_ALIAS, ON_BOOT, ON_LONELY, ON_MINUTE,
    ];
    wanted.sort_unstable();
    assert_eq!(keys, wanted, "鍵が 4 台帳の全 id と一致しない");

    let value = |raw: &str| derived.get(&id(raw)).expect("鍵は在るはず").as_str();
    assert_eq!(value(ON_BOOT), "A1", "束の項目は段階 1 文字＋順位");
    assert_eq!(value(ON_MINUTE), "A1", "同じ束の項目は同じ値");
    assert_eq!(value(INTERVAL), "A1", "ドメインが違っても同じ束なら同じ値");
    assert_eq!(
        value(ON_LONELY),
        "E2",
        "単独項目も 1 つの束として番号を持つ"
    );
}

/// `alias`・`not-applicable` は空文字（要件 7.2）。束に属さないことは失敗にしない。
#[test]
fn alias_and_not_applicable_are_blank_without_belonging_to_a_bundle() {
    let derived = priorities(&linkage(), &briefing(), &ledgers()).expect("見本は導けるはず");

    assert_eq!(derived.get(&id(ON_ALIAS)).map(String::as_str), Some(""));
    assert_eq!(derived.get(&id(NOT_TARGET)).map(String::as_str), Some(""));
}

// ---------------------------------------------------------------------------
// priorities — 落ちる経路
// ---------------------------------------------------------------------------

/// どの束にも属さない対象項目は、id と文書名を添えて落ちる（タスク 1.3）。
///
/// 較正: 同じ id を束に入れれば緑になる——落ちる理由が「帰属が無いこと」であって
/// 「項目が 1 件増えたこと」ではないと示す。
#[test]
fn a_target_item_in_no_bundle_fails_naming_the_id_and_the_document() {
    let stray = "ukadoc:list_shiori_event:OnStray:1";
    let mut ledgers = ledgers();
    ledgers[0]
        .entries
        .insert(id(stray), entry_of(stray, Status::Absent, &[]));

    let body = failure(priorities(&linkage(), &briefing(), &ledgers));
    assert!(body.contains(stray), "id が本文に無い: {body}");
    assert!(
        body.contains("doc/ukadoc-coverage/linkage.md"),
        "文書名が本文に無い: {body}"
    );

    let fixed = linkage_of(vec![
        named(
            CLOCK,
            CLOCK_FOUNDATION,
            Breakage::Silent,
            &[ON_BOOT, ON_MINUTE, INTERVAL, stray],
        ),
        single_of(ON_LONELY, Breakage::Visual),
    ]);
    let derived = priorities(&fixed, &briefing(), &ledgers).expect("帰属を足せば導けるはず");
    assert_eq!(derived.get(&id(stray)).map(String::as_str), Some("A1"));
}

/// 順位表に無い束は、束名と文書名を添えて落ちる。
#[test]
fn a_bundle_missing_from_the_rank_table_fails_naming_the_bundle_and_the_document() {
    let briefing = briefing_of(vec![bundle_row(Stage::A, 1, CLOCK)], &[]);

    let body = failure(priorities(&linkage(), &briefing, &ledgers()));
    assert!(body.contains(ON_LONELY), "束名が本文に無い: {body}");
    assert!(
        body.contains("doc/ukadoc-coverage/briefing.md"),
        "文書名が本文に無い: {body}"
    );
}

/// 帰属に無い束名を順位表が指していたら落ちる。
#[test]
fn a_rank_row_naming_a_bundle_absent_from_the_linkage_fails() {
    let briefing = briefing_of(
        vec![
            bundle_row(Stage::A, 1, CLOCK),
            bundle_row(Stage::B, 1, "在らぬ束"),
            singles_row(Stage::E, 2, &[ON_LONELY]),
        ],
        &[],
    );

    let body = failure(priorities(&linkage(), &briefing, &ledgers()));
    assert!(body.contains("在らぬ束"), "束名が本文に無い: {body}");
    assert!(
        body.contains("doc/ukadoc-coverage/briefing.md"),
        "文書名が本文に無い: {body}"
    );
}

/// 単独項目を `bundle` で指すことも、名前付き束を `singles` に混ぜることも落とす
/// （設計 `documents::parse`「読み取り時の形の検査」のうち、`linkage.md` を要る
/// 跨ぎの 1 項目。読み手は単独で判定できないのでここが引き受ける）。
#[test]
fn the_rank_row_must_name_a_named_bundle_in_bundle_and_singles_in_singles() {
    let as_bundle = briefing_of(
        vec![
            bundle_row(Stage::A, 1, CLOCK),
            bundle_row(Stage::E, 2, ON_LONELY),
        ],
        &[],
    );
    let body = failure(priorities(&linkage(), &as_bundle, &ledgers()));
    assert!(body.contains(ON_LONELY), "束名が本文に無い: {body}");
    assert!(body.contains("singles"), "直し方が本文に無い: {body}");

    // 名前付き束の名前が id の綴りでも、`single` でなければ `singles` に混ぜられない。
    let id_named = linkage_of(vec![
        named(
            ON_BOOT,
            CLOCK_FOUNDATION,
            Breakage::Silent,
            &[ON_BOOT, ON_MINUTE, INTERVAL],
        ),
        single_of(ON_LONELY, Breakage::Visual),
    ]);
    let as_single = briefing_of(
        vec![
            singles_row(Stage::A, 1, &[ON_BOOT]),
            singles_row(Stage::E, 2, &[ON_LONELY]),
        ],
        &[],
    );
    let body = failure(priorities(&id_named, &as_single, &ledgers()));
    assert!(body.contains(ON_BOOT), "束名が本文に無い: {body}");
    assert!(body.contains("bundle"), "直し方が本文に無い: {body}");
}

/// 同じ束が順位表に 2 度現れたら落ちる（「同じ束の項目は同じ値」が壊れる）。
#[test]
fn a_bundle_ranked_twice_fails() {
    let briefing = briefing_of(
        vec![
            bundle_row(Stage::A, 1, CLOCK),
            bundle_row(Stage::B, 3, CLOCK),
            singles_row(Stage::E, 2, &[ON_LONELY]),
        ],
        &[],
    );

    let body = failure(priorities(&linkage(), &briefing, &ledgers()));
    assert!(body.contains(CLOCK), "束名が本文に無い: {body}");
}

/// 同じ id が 2 つの束に属していたら落ちる。
#[test]
fn an_item_in_two_bundles_fails() {
    let linkage = linkage_of(vec![
        named(
            CLOCK,
            CLOCK_FOUNDATION,
            Breakage::Silent,
            &[ON_BOOT, INTERVAL],
        ),
        named(
            "分の刻み",
            "別の発火路",
            Breakage::Silent,
            &[ON_BOOT, ON_MINUTE],
        ),
        single_of(ON_LONELY, Breakage::Visual),
    ]);
    let briefing = briefing_of(
        vec![
            bundle_row(Stage::A, 1, CLOCK),
            bundle_row(Stage::A, 2, "分の刻み"),
            singles_row(Stage::E, 2, &[ON_LONELY]),
        ],
        &[],
    );

    let body = failure(priorities(&linkage, &briefing, &ledgers()));
    assert!(body.contains(ON_BOOT), "id が本文に無い: {body}");
    assert!(
        body.contains("doc/ukadoc-coverage/linkage.md"),
        "文書名が本文に無い: {body}"
    );
}

/// `unclassified` は段階も空文字も持てないので落ちる。要件 7.1 は対象 4 状態、
/// 7.2 は別名と対象外しか決めておらず、7 つ目の状態には行き先が無い。
#[test]
fn an_unclassified_item_fails_naming_the_ledger() {
    let mut ledgers = ledgers();
    let raw = "ukadoc:list_shiori_event:OnUnknown:1";
    ledgers[0]
        .entries
        .insert(id(raw), entry_of(raw, Status::Unclassified, &[]));

    let body = failure(priorities(&linkage(), &briefing(), &ledgers));
    assert!(body.contains(raw), "id が本文に無い: {body}");
    assert!(
        body.contains("doc/ukadoc-coverage/ledger/shiori.toml"),
        "台帳の名前が本文に無い: {body}"
    );
}

// ---------------------------------------------------------------------------
// derive_bundle
// ---------------------------------------------------------------------------

/// テーマは構成 id の `values` の**和集合**（重複は 1 度きり）。
#[test]
fn the_themes_are_the_union_of_the_values_of_the_members() {
    let linkage = linkage();
    let bundle = linkage.bundles.get(CLOCK).expect("見本の束は在るはず");

    let derived = derive_bundle(bundle, &linkage, &briefing(), &ledgers());

    let themes: Vec<&str> = derived.themes.iter().map(String::as_str).collect();
    assert_eq!(themes, vec!["更新", "気配"], "3 件の values の和集合でない");
}

/// ドメインは構成 id を持つ台帳のもの。
#[test]
fn the_domains_are_the_domains_of_the_ledgers_that_hold_the_members() {
    let linkage = linkage();
    let clock = linkage.bundles.get(CLOCK).expect("見本の束は在るはず");
    let lonely = linkage.bundles.get(ON_LONELY).expect("単独項目は在るはず");

    let both = derive_bundle(clock, &linkage, &briefing(), &ledgers());
    assert_eq!(
        both.domains,
        BTreeSet::from([Domain::Shiori, Domain::Assets]),
        "2 本の台帳に跨っている"
    );

    let one = derive_bundle(lonely, &linkage, &briefing(), &ledgers());
    assert_eq!(one.domains, BTreeSet::from([Domain::Shiori]));
}

/// 資産の広さはテンプレート辞書の語彙との交わりの**件数**。
/// 見本の語彙は 2 件で、うち 1 件は構成 id でない——数え過ぎればここが赤くなる。
#[test]
fn the_assets_count_only_the_members_that_appear_in_the_template_vocabulary() {
    let linkage = linkage();
    let clock = linkage.bundles.get(CLOCK).expect("見本の束は在るはず");
    let lonely = linkage.bundles.get(ON_LONELY).expect("単独項目は在るはず");

    assert_eq!(
        derive_bundle(clock, &linkage, &briefing(), &ledgers()).assets,
        1,
        "語彙 2 件のうち構成 id は 1 件だけ"
    );
    assert_eq!(
        derive_bundle(lonely, &linkage, &briefing(), &ledgers()).assets,
        0,
        "語彙に無い束は 0"
    );
}

/// 共有度は同じ `foundation` を持つ束の数で、**自分を含む**。
///
/// 単独項目は `foundation` を持たない（読み手が禁じている）ので 0 になる——
/// 基盤が無いものに共有度は無い。foundation が空の束どうしを数え合わせると、
/// 単独項目の全数が共有度になってしまう。
#[test]
fn the_shared_count_includes_the_bundle_itself() {
    let linkage = linkage_of(vec![
        named(CLOCK, CLOCK_FOUNDATION, Breakage::Silent, &[ON_BOOT]),
        named("分の刻み", CLOCK_FOUNDATION, Breakage::Silent, &[ON_MINUTE]),
        named(
            "装いの取り替え",
            "描画の差し替え路",
            Breakage::Visual,
            &[INTERVAL],
        ),
        single_of(ON_LONELY, Breakage::Visual),
    ]);
    let briefing = briefing_of(Vec::new(), &[]);
    let shared = |name: &str| {
        derive_bundle(
            linkage.bundles.get(name).expect("見本の束は在るはず"),
            &linkage,
            &briefing,
            &ledgers(),
        )
        .shared
    };

    assert_eq!(shared(CLOCK), 2, "同じ基盤の 2 束（自分を含む）");
    assert_eq!(shared("分の刻み"), 2, "相手から見ても 2");
    assert_eq!(shared("装いの取り替え"), 1, "自分だけでも 1");
    assert_eq!(shared(ON_LONELY), 0, "基盤の無い単独項目は 0");
}

// ---------------------------------------------------------------------------
// axis_key
// ---------------------------------------------------------------------------

/// 序列は 壊れ方 ＞ テーマ数 ＞ 資産 ＞ 共有度（要件 6.1）。
/// 上位の根拠が下位の根拠をすべて押し切ることを 1 段ずつ見る。
#[test]
fn the_axis_key_ranks_breakage_over_themes_over_assets_over_shared() {
    assert!(
        axis_key(Breakage::Silent, 0, 0, 0) > axis_key(Breakage::Explicit, 9, 9, 9),
        "壊れ方が最上位でない"
    );
    assert!(
        axis_key(Breakage::Explicit, 2, 0, 0) > axis_key(Breakage::Explicit, 1, 9, 9),
        "テーマ数が資産より上でない"
    );
    assert!(
        axis_key(Breakage::Explicit, 1, 2, 0) > axis_key(Breakage::Explicit, 1, 1, 9),
        "資産が共有度より上でない"
    );
    assert!(
        axis_key(Breakage::Explicit, 1, 1, 2) > axis_key(Breakage::Explicit, 1, 1, 1),
        "共有度が効いていない"
    );
}

/// 壊れ方の重みは 黙って壊れる ＞ 明示エラー ＞ 見た目の差 ＞ 該当なし（設計 D-9）。
#[test]
fn the_breakage_weight_follows_the_declared_order() {
    let key = |breakage| axis_key(breakage, 0, 0, 0);
    assert!(key(Breakage::Silent) > key(Breakage::Explicit));
    assert!(key(Breakage::Explicit) > key(Breakage::Visual));
    assert!(key(Breakage::Visual) > key(Breakage::NotApplicable));
}

/// 4 つが同じ値なら鍵も等しい（要件 6.7 の同順位）。
#[test]
fn four_equal_grounds_give_an_equal_key() {
    assert_eq!(
        axis_key(Breakage::Visual, 3, 4, 5),
        axis_key(Breakage::Visual, 3, 4, 5)
    );
}
