//! [`FontCatalog`]（`draw_catalog.rs`）の決定論テスト
//! （タスク 6.2・要件 9.1／9.2／9.3／9.4／9.7／9.8／13.2）。
//!
//! 実在するフォント名の問い合わせ先は**偽の集合**へ差し替えられるので、判定は実機の
//! フォント構成に依存しない。本番経路（システムのフォント集合）が偽物になっていない
//! ことは §5 が実際に [`FontCatalog::new`] を通して確かめる。
//!
//! 記録の件数は `log-capture-kit`（ワークスペース唯一の捕捉機構）で数える。件数 0 を
//! 主張する試験には、同じ窓・同じ発行点で「出る側」を置いた対照を添えてある。
//!
//! | § | 内容 | 要件 |
//! |---|---|---|
//! | §1 | 記述順に最初の実在フォントを採る | 9.1, 9.2 |
//! | §2 | フォントファイル名は読み飛ばして記録 1 件 | 9.3 |
//! | §3 | 全滅は既定へ戻して記録 1 件 | 9.4, 13.2 |
//! | §4 | 同じ候補列は引き直さない・記録も増えない | 9.7 |
//! | §5 | バルーン定義の候補列にも同じ規則／本番経路の対照 | 9.8 |
//! | §6 | 較正（先頭候補だけを採る誤り・鍵を連結する誤り） | 9.2, 13.5 |

use areka_parsers::balloon::{Font, FontColor};
use log_capture_kit::{CapturedEvent, capture};
use windows::Win32::Graphics::DirectWrite::DWRITE_FACTORY_TYPE_SHARED;
use wintf::com::dwrite::dwrite_create_factory;

use super::super::test_support::model_with_font;
use super::super::{DEFAULT_FONT_NAME, ResolvedFont};
use super::FontCatalog;

/// フォントファイル名を読み飛ばしたときの記録の本文。
const FONT_FILE: &str = "フォントファイル名の候補は読み込まない";
/// 候補が全滅したときの記録の本文。
const ALL_MISSING: &str = "フォントの候補が 1 つも見つからない";

/// 捕捉した記録のうち、本文に `needle` を含む `warn` の件数。
fn warns(events: &[CapturedEvent], needle: &str) -> usize {
    events
        .iter()
        .filter(|e| e.level == tracing::Level::WARN && e.message().contains(needle))
        .count()
}

/// `&[&str]` を候補列へ。
fn v(names: &[&str]) -> Vec<String> {
    names.iter().map(|n| (*n).to_owned()).collect()
}

/// 偽の集合を持つ台帳（この機械に何が入っていても判定が変わらない）。
fn catalog(installed: &[&str]) -> FontCatalog {
    FontCatalog::with_families(installed)
}

// ────────────────────────── §1 記述順に最初の実在フォント（R9.1／R9.2）

/// 先頭が無い候補列では第 2 候補が選ばれる。
#[test]
fn the_second_candidate_is_taken_when_the_first_one_is_not_installed() {
    let cat = catalog(&["Yu Gothic UI"]);
    assert_eq!(
        cat.family_for(&v(&["存在しないフォント", "Yu Gothic UI"])),
        Some("Yu Gothic UI".to_owned())
    );
}

/// 先頭が在れば先頭が選ばれる——優先度は書いた順であって集合の並びではない。
#[test]
fn the_first_installed_candidate_wins_in_written_order() {
    let cat = catalog(&["Meiryo", "Yu Gothic UI"]);
    assert_eq!(
        cat.family_for(&v(&["Meiryo", "Yu Gothic UI"])),
        Some("Meiryo".to_owned())
    );
    assert_eq!(
        cat.family_for(&v(&["Yu Gothic UI", "Meiryo"])),
        Some("Yu Gothic UI".to_owned())
    );
}

/// 空の候補列は解決しないが、記録も残さない。
#[test]
fn an_empty_candidate_list_resolves_to_nothing_without_a_record() {
    let cat = catalog(&["Yu Gothic UI"]);
    let (resolved, events) = capture(|| cat.family_for(&[]));
    assert_eq!(resolved, None);
    assert_eq!(events.len(), 0, "空列は失敗ではないので記録を残さない");
}

// ────────────────────────── §2 フォントファイル名の読み飛ばし（R9.3）

/// フォントファイル名の候補は読み込まずに次へ進み、記録は 1 件。
#[test]
fn a_font_file_candidate_is_skipped_with_exactly_one_record() {
    let cat = catalog(&["Yu Gothic UI"]);
    let (resolved, events) = capture(|| cat.family_for(&v(&["a.ttf", "Yu Gothic UI"])));

    assert_eq!(resolved, Some("Yu Gothic UI".to_owned()));
    assert_eq!(warns(&events, FONT_FILE), 1);
    assert_eq!(
        warns(&events, ALL_MISSING),
        0,
        "読み飛ばしても第 2 候補で解決したので全滅ではない"
    );
}

/// 3 つの拡張子すべてと、大文字小文字の混在を読み飛ばす。
#[test]
fn all_three_extensions_are_recognised_case_insensitively() {
    let cat = catalog(&["Yu Gothic UI"]);
    for name in ["a.ttf", "a.OTF", "A.TtC", "メイリオ.ttf"] {
        let (resolved, events) = capture(|| cat.family_for(&v(&[name, "Yu Gothic UI"])));
        assert_eq!(
            resolved,
            Some("Yu Gothic UI".to_owned()),
            "{name} は読み飛ばして次の候補へ進む"
        );
        assert_eq!(warns(&events, FONT_FILE), 1, "{name} の記録が 1 件");
    }
}

/// 拡張子を持たない普通の名前は読み飛ばさない（誤検出しない陰性対照）。
#[test]
fn an_ordinary_family_name_is_not_mistaken_for_a_file() {
    let cat = catalog(&["Yu Gothic UI"]);
    let (resolved, events) = capture(|| cat.family_for(&v(&["Yu Gothic UI"])));
    assert_eq!(resolved, Some("Yu Gothic UI".to_owned()));
    assert_eq!(warns(&events, FONT_FILE), 0);
}

// ────────────────────────── §3 全滅は既定へ（R9.4／R13.2）

/// 候補が全滅すると記録 1 件を残して解決せず、既定のフォントへ戻る。
#[test]
fn an_all_missing_list_records_once_and_falls_back_to_the_default_font() {
    let cat = catalog(&["Yu Gothic UI"]);
    let (resolved, events) = capture(|| cat.family_for(&v(&["無いフォント", "b.ttf"])));

    assert_eq!(resolved, None);
    assert_eq!(warns(&events, ALL_MISSING), 1);
    assert_eq!(
        warns(&events, FONT_FILE),
        1,
        "読み飛ばした候補の記録も別に 1 件残る"
    );

    let font = ResolvedFont::resolve(&model_with_font(Font::new(
        Some("無いフォント, b.ttf".to_owned()),
        None,
        FontColor::new(None, None, None),
    )));
    assert_eq!(
        cat.pick(&font).name,
        DEFAULT_FONT_NAME,
        "全滅した候補列は既定のフォントへ戻る"
    );
}

/// 試した候補が記録に添えられている。
#[test]
fn the_record_of_an_all_missing_list_carries_the_candidates_it_tried() {
    let cat = catalog(&[]);
    let ((), events) = capture(|| {
        cat.family_for(&v(&["無いフォント", "他も無い"]));
    });
    let record = events
        .iter()
        .find(|e| e.message().contains(ALL_MISSING))
        .expect("全滅の記録");
    let text = format!("{record:?}");
    assert!(
        text.contains("無いフォント") && text.contains("他も無い"),
        "試した候補が記録に含まれていない: {text}"
    );
}

// ────────────────────────── §4 同じ候補列は引き直さない（R9.7）

/// 2 度目は問い合わせずに記憶から返し、記録も増えない。
#[test]
fn the_same_candidate_list_is_resolved_once_and_records_once() {
    let cat = catalog(&["Yu Gothic UI"]);
    let ((), events) = capture(|| {
        cat.family_for(&v(&["a.ttf", "Yu Gothic UI"])); // 対照: 1 件出る側
        cat.family_for(&v(&["a.ttf", "Yu Gothic UI"])); // 2 度目は 0 件
        cat.family_for(&v(&["b.ttf", "Yu Gothic UI"])); // 別の候補列は別の鍵で 1 件
    });

    assert_eq!(
        warns(&events, FONT_FILE),
        2,
        "同じ列の 2 度目は 0 件・別の列は 1 件（合計 2 件）"
    );
    assert_eq!(
        cat.lookup_count(),
        2,
        "問い合わせは候補列ごとに 1 度（文字ごとに引き直さない）"
    );
}

/// 全滅の記録も候補列ごとに 1 度。
#[test]
fn an_all_missing_list_records_only_once_per_list() {
    let cat = catalog(&[]);
    let ((), events) = capture(|| {
        for _ in 0..3 {
            cat.family_for(&v(&["無いフォント"]));
        }
    });
    assert_eq!(warns(&events, ALL_MISSING), 1);
    assert_eq!(cat.lookup_count(), 1);
}

// ────────────────────────── §5 バルーン定義の候補列／本番経路（R9.8）

/// バルーン定義のカンマ区切りにも同じ「記述順に最初の実在」規則が効く。
#[test]
fn the_balloon_candidate_chain_follows_the_same_rule() {
    let cat = catalog(&["Yu Gothic UI"]);
    let font = ResolvedFont::resolve(&model_with_font(Font::new(
        Some("存在しないフォント, Yu Gothic UI".to_owned()),
        Some(20),
        FontColor::new(Some(1), Some(2), Some(3)),
    )));
    assert_eq!(font.name, "存在しないフォント", "解決前は先頭のまま");

    let picked = cat.pick(&font);
    assert_eq!(picked.name, "Yu Gothic UI", "第 2 候補が採られる");
    assert_eq!(picked.height, 20.0, "名前以外は複製");
    assert_eq!(picked.color, (1, 2, 3));
    assert_eq!(
        picked.looks.default.name,
        v(&["存在しないフォント", "Yu Gothic UI"]),
        "候補列そのものは見た目の側に残る"
    );
}

/// 本番経路（システムのフォント集合）が偽物になっていないことの対照。
#[test]
fn the_system_font_collection_is_the_real_source_in_production() {
    let factory = dwrite_create_factory(DWRITE_FACTORY_TYPE_SHARED)
        .expect("DWriteCreateFactory（デバイス非依存・headless 可）");
    let cat = FontCatalog::new(&factory).expect("システムのフォント集合を取得できる");

    assert_eq!(
        cat.family_for(&v(&[DEFAULT_FONT_NAME])),
        Some(DEFAULT_FONT_NAME.to_owned()),
        "既定のフォントは実在する（偽の集合は空なので、この行が本番経路の陽性対照）"
    );
    assert_eq!(
        cat.family_for(&v(&["この名前のフォントは存在しない"])),
        None,
        "実在しない名前は解決しない（恒真に Some を返していないことの陰性対照）"
    );
}

// ────────────────────────── §6 較正（過去の壊れ方を再現すると赤）

/// 較正 1: 「先頭候補だけを採る」旧い誤りを再現すると赤になる。
///
/// 旧 `ResolvedFont::resolve` は候補列の先頭だけを採り、残りを未消費のまま持っていた。
/// その振る舞いを写した下の関数は第 2 候補へ進めないので、実装と食い違う。
#[test]
fn taking_only_the_first_candidate_would_disagree_with_the_catalog() {
    let cat = catalog(&["Yu Gothic UI"]);
    let candidates = v(&["存在しないフォント", "Yu Gothic UI"]);

    /// 旧い誤り: 先頭だけを採る。
    fn first_only(candidates: &[String]) -> Option<String> {
        candidates.first().cloned()
    }

    assert_ne!(
        cat.family_for(&candidates),
        first_only(&candidates),
        "先頭候補だけを採る実装ならこの述語は緑にならない"
    );
    assert_eq!(cat.family_for(&candidates), Some("Yu Gothic UI".to_owned()));
}

/// 較正 2: 記録の鍵を「連結した綴り」にする誤りを再現すると赤になる。
///
/// 引用符がカンマを守るので、候補 2 つの列と「カンマを含む候補 1 つ」の列はどちらも
/// 到達する。カンマで連結すると同じ綴りになるので、鍵を連結すると片方の記録が
/// 無記録で消える（要件 13.5 違反）。
#[test]
fn a_joined_spelling_would_collapse_two_different_failures_into_one_key() {
    let two = v(&["a", "b"]);
    let one = v(&["a,b"]);
    assert_eq!(
        two.join(","),
        one.join(","),
        "この 2 つの候補列は連結すると同じ綴りになる"
    );
    assert_ne!(two, one, "候補列そのものは別物");

    let cat = catalog(&[]);
    let ((), events) = capture(|| {
        cat.family_for(&two);
        cat.family_for(&one);
    });
    assert_eq!(
        warns(&events, ALL_MISSING),
        2,
        "連結した綴りを鍵にすると 1 件へ潰れる——鍵は候補列そのものを保つこと"
    );
}

/// 較正 3: 読み飛ばしの記録と全滅の記録は別の種別として数える。
///
/// 候補 1 つだけのフォントファイル名は「読み飛ばし」と「全滅」の両方を起こす。
/// 種別を持たない鍵だと後の 1 件が先の 1 件に食われる。
#[test]
fn the_skip_record_and_the_all_missing_record_are_separate_kinds() {
    let cat = catalog(&["Yu Gothic UI"]);
    let (resolved, events) = capture(|| cat.family_for(&v(&["a.ttf"])));

    assert_eq!(resolved, None);
    assert_eq!(warns(&events, FONT_FILE), 1, "読み飛ばしの記録");
    assert_eq!(warns(&events, ALL_MISSING), 1, "全滅の記録");
}
