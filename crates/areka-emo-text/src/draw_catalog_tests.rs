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
//! | §7 | 顔に太字・斜体の指定を重ねる規則（檻 U7） | 4.2〜4.5 |
//! | §8 | GDI の名前での引き直し（偽の台帳・檻 U1〜U6） | 3.1〜3.7, 3.11, 6.2 |
//! | §9 | 実際の書体の集まりでの中太（檻 R1） | 3.5, 3.10, 6.3 |

use areka_parsers::balloon::{Font, FontColor};
use log_capture_kit::{CapturedEvent, capture};
use windows::Win32::Graphics::DirectWrite::{
    DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE,
    DWRITE_FONT_STYLE_ITALIC, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT,
    DWRITE_FONT_WEIGHT_BLACK, DWRITE_FONT_WEIGHT_BOLD, DWRITE_FONT_WEIGHT_LIGHT,
    DWRITE_FONT_WEIGHT_MEDIUM, DWRITE_FONT_WEIGHT_NORMAL,
};
use wintf::com::dwrite::dwrite_create_factory;

use super::super::test_support::model_with_font;
use super::super::{DEFAULT_FONT_NAME, ResolvedFont};
use super::{FontCatalog, FontFace};

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

// ---- §7 顔に太字・斜体の指定を重ねる規則（檻 U7・要件 4.2〜4.5） ----

/// 太さと斜体だけを変えた顔（家族名・幅は規則に関わらない）。
fn face(weight: DWRITE_FONT_WEIGHT, style: DWRITE_FONT_STYLE) -> FontFace {
    FontFace {
        weight,
        style,
        ..FontFace::regular("Yu Gothic")
    }
}

/// 家族名で当たった候補の顔は Regular（太さ・幅・斜体とも NORMAL・要件 3.3）。
#[test]
fn regular_face_keeps_the_family_and_is_normal_everywhere() {
    let f = FontFace::regular("Yu Gothic");
    assert_eq!(f.family, "Yu Gothic");
    assert_eq!(f.weight, DWRITE_FONT_WEIGHT_NORMAL);
    assert_eq!(f.stretch, DWRITE_FONT_STRETCH_NORMAL);
    assert_eq!(f.style, DWRITE_FONT_STYLE_NORMAL);
}

/// 太字の指定は名前の太さと BOLD の大きい方、指定が無ければ名前の太さ（要件 4.2・4.3）。
#[test]
fn bold_takes_the_heavier_of_the_named_weight_and_bold() {
    let table = [
        (DWRITE_FONT_WEIGHT_NORMAL, false, DWRITE_FONT_WEIGHT_NORMAL),
        (DWRITE_FONT_WEIGHT_NORMAL, true, DWRITE_FONT_WEIGHT_BOLD),
        (DWRITE_FONT_WEIGHT_MEDIUM, false, DWRITE_FONT_WEIGHT_MEDIUM),
        (DWRITE_FONT_WEIGHT_MEDIUM, true, DWRITE_FONT_WEIGHT_BOLD),
        (DWRITE_FONT_WEIGHT_LIGHT, false, DWRITE_FONT_WEIGHT_LIGHT),
        (DWRITE_FONT_WEIGHT_LIGHT, true, DWRITE_FONT_WEIGHT_BOLD),
        (DWRITE_FONT_WEIGHT_BLACK, false, DWRITE_FONT_WEIGHT_BLACK),
        (DWRITE_FONT_WEIGHT_BLACK, true, DWRITE_FONT_WEIGHT_BLACK),
    ];
    for (named, bold, expected) in table {
        assert_eq!(
            face(named, DWRITE_FONT_STYLE_NORMAL).weight_with(bold),
            expected,
            "名前の太さ {} ＋ 太字 {bold}",
            named.0
        );
    }
    assert_eq!(DWRITE_FONT_WEIGHT_BOLD.0, 700, "中太＋太字は 700");
    assert_eq!(DWRITE_FONT_WEIGHT_BLACK.0, 900, "Black＋太字は 900");
}

/// 斜体の指定は ITALIC、指定が無ければ名前の斜体（要件 4.4・4.5）。
#[test]
fn italic_overrides_and_absence_keeps_the_named_style() {
    let table = [
        (DWRITE_FONT_STYLE_NORMAL, false, DWRITE_FONT_STYLE_NORMAL),
        (DWRITE_FONT_STYLE_NORMAL, true, DWRITE_FONT_STYLE_ITALIC),
        (DWRITE_FONT_STYLE_ITALIC, false, DWRITE_FONT_STYLE_ITALIC),
        (DWRITE_FONT_STYLE_ITALIC, true, DWRITE_FONT_STYLE_ITALIC),
    ];
    for (named, italic, expected) in table {
        assert_eq!(
            face(DWRITE_FONT_WEIGHT_NORMAL, named).style_with(italic),
            expected,
            "名前の斜体 {} ＋ 斜体 {italic}",
            named.0
        );
    }
}

// ---- §8 GDI の名前での引き直し（偽の台帳・檻 U1〜U6・要件 3.1〜3.7／3.11／6.2） ----

/// 「Yu Gothic」家族の中太（偽の GDI の表が返す顔）。
fn medium() -> FontFace {
    FontFace {
        weight: DWRITE_FONT_WEIGHT_MEDIUM,
        ..FontFace::regular("Yu Gothic")
    }
}

/// 家族名に「Meiryo」、GDI の表に「Yu Gothic Medium」→ 中太を持つ偽の台帳。
fn gdi_catalog() -> FontCatalog {
    FontCatalog::with_faces(&["Meiryo"], &[("Yu Gothic Medium", medium())])
}

/// U1（3.3）: 家族名で当たる候補は Regular の顔（太さ・幅・斜体とも NORMAL）。
#[test]
fn u1_a_family_name_hit_is_the_regular_face() {
    let cat = gdi_catalog();
    assert_eq!(
        cat.face_for(&v(&["Meiryo"])),
        Some(FontFace::regular("Meiryo"))
    );
}

/// U2（3.1・3.2）: 家族名に無く GDI の名前で在る候補は、その名前が指す家族・太さの顔。
#[test]
fn u2_a_gdi_name_hit_is_the_face_the_name_points_to() {
    let cat = gdi_catalog();
    let face = cat
        .face_for(&v(&["Yu Gothic Medium"]))
        .expect("GDI の名前で当たる");
    assert_eq!(face, medium());
    assert_eq!(
        face.family, "Yu Gothic",
        "家族名は GDI の名前ではなく家族の名前"
    );
    assert_eq!(face.weight.0, 500);
    assert_eq!(
        cat.family_for(&v(&["Yu Gothic Medium"])),
        Some("Yu Gothic".to_owned()),
        "家族名だけの口も同じ解決を通る"
    );
}

/// U3（3.4）: 書いた順に引き、家族名と GDI の名前のどちらも「入っている」扱い。
#[test]
fn u3_candidates_are_tried_in_written_order_across_both_lookups() {
    let cat = gdi_catalog();
    assert_eq!(
        cat.face_for(&v(&["Yu Gothic Medium", "Meiryo"])),
        Some(medium())
    );
    assert_eq!(
        cat.face_for(&v(&["無いフォント", "Yu Gothic Medium", "Meiryo"])),
        Some(medium())
    );
    assert_eq!(
        cat.face_for(&v(&["Meiryo", "Yu Gothic Medium"])),
        Some(FontFace::regular("Meiryo"))
    );

    // 同じ名前が家族名にも GDI の名前にも在るときは家族名が先（Regular の顔）。
    let both = FontCatalog::with_faces(&["Yu Gothic"], &[("Yu Gothic", medium())]);
    assert_eq!(
        both.face_for(&v(&["Yu Gothic"])),
        Some(FontFace::regular("Yu Gothic"))
    );
}

/// U4（3.5）: GDI の名前の照合は大小を問わない。
#[test]
fn u4_gdi_names_match_case_insensitively() {
    let cat = gdi_catalog();
    for name in ["YU GOTHIC MEDIUM", "yu gothic medium", "Yu Gothic Medium"] {
        assert_eq!(cat.face_for(&v(&[name])), Some(medium()), "{name}");
    }
}

/// U5（3.6・3.7）: 全滅は None・記録 1 度・既定／渡した顔へ戻る。ファイル名と 31 単位を
/// 超える候補は GDI の表に同じ名前があっても引かない。GDI で見つからないだけでは記録しない。
#[test]
fn u5_all_missing_falls_back_and_files_and_long_names_are_not_looked_up() {
    let long = "a".repeat(32);
    let cat = FontCatalog::with_faces(&[], &[("a.ttf", medium()), (long.as_str(), medium())]);

    let (resolved, events) = capture(|| {
        let first = cat.face_for(&v(&["無いフォント", "他も無い"]));
        let second = cat.face_for(&v(&["無いフォント", "他も無い"]));
        (first, second)
    });
    assert_eq!(resolved, (None, None));
    assert_eq!(warns(&events, ALL_MISSING), 1, "2 度目の解決でも増えない");
    assert_eq!(
        events.len(),
        1,
        "見つからない名前は GDI の側で記録しない（全滅の 1 件だけ）: {events:?}"
    );

    assert_eq!(
        cat.face_or_default(&v(&["無いフォント"])),
        FontFace::regular(DEFAULT_FONT_NAME)
    );
    assert_eq!(cat.face_or(&v(&["無いフォント"]), &medium()), medium());
    assert_eq!(
        cat.face_or(
            &v(&["無いフォント", "Yu Gothic Medium"]),
            &FontFace::regular("x")
        ),
        FontFace::regular("x"),
        "表に無い名前は全滅として渡した顔"
    );

    assert_eq!(
        cat.face_for(&v(&["a.ttf"])),
        None,
        "ファイル名は GDI でも引かない"
    );
    assert_eq!(
        cat.face_for(std::slice::from_ref(&long)),
        None,
        "31 単位を超える候補は GDI では引かない"
    );
    assert_eq!(
        cat.face_for(&["a".repeat(31)]),
        None,
        "表に無い 31 単位は（引いて）無い"
    );
    let at_limit = FontCatalog::with_faces(&[], &[("b".repeat(31).as_str(), medium())]);
    assert_eq!(
        at_limit.face_for(&["b".repeat(31)]),
        Some(medium()),
        "対照: 31 単位ちょうどは GDI で引く"
    );
}

/// U6（3.11）: 同じ候補列の 2 度目は問い合わせない（GDI で当たった列も同じ）。
#[test]
fn u6_a_gdi_resolved_list_is_not_looked_up_twice() {
    let cat = gdi_catalog();
    let list = v(&["無いフォント", "Yu Gothic Medium"]);
    assert_eq!(cat.face_for(&list), Some(medium()));
    assert_eq!(cat.face_for(&list), Some(medium()));
    assert_eq!(cat.family_for(&list), Some("Yu Gothic".to_owned()));
    assert_eq!(cat.face_or_default(&list), medium());
    assert_eq!(
        cat.lookup_count(),
        1,
        "どの口から引いても 1 度だけ問い合わせる"
    );
}

/// U8（3.2・3.10）: ぱすたの候補列を束縛のフォントへ解決すると、名前は家族名・顔は中太。
/// 組み立てたばかりの束縛は未解決の Regular の顔（先頭候補の名前）を持つ。
#[test]
fn u8_pick_puts_the_resolved_face_on_the_bound_font() {
    let font = ResolvedFont::resolve(&model_with_font(Font::new(
        Some("游ゴシック Medium, Yu Gothic Medium, 游ゴシック, Yu Gothic, メイリオ".to_owned()),
        None,
        FontColor::new(None, None, None),
    )));
    assert_eq!(font.face, FontFace::regular("游ゴシック Medium"));

    // GDI の表の鍵は大小を問わない（「游ゴシック medium」で引ける）。
    let cat = FontCatalog::with_faces(&["メイリオ"], &[("游ゴシック medium", medium())]);
    let picked = cat.pick(&font);
    assert_eq!(picked.name, "Yu Gothic");
    assert_eq!(picked.face, medium());
    assert_eq!(picked.face.weight.0, 500);
}

// ---- §9 実際の書体の集まりでの中太（檻 R1・要件 3.5／3.10／6.3） ----

/// R1: 本番の台帳で「Yu Gothic Medium」ほかの GDI の名前が中太（500）の顔になり、
/// その家族名は家族名として引き直せる。ぱすたの候補列も 500・無い名前だけの列は None。
#[test]
fn r1_real_collection_resolves_gdi_names_to_the_medium_face() {
    let factory = dwrite_create_factory(DWRITE_FACTORY_TYPE_SHARED)
        .expect("DWriteCreateFactory（デバイス非依存・headless 可）");
    let cat = FontCatalog::new(&factory).expect("システムのフォント集合を取得できる");
    assert!(
        cat.family_for(&v(&["Yu Gothic"])).is_some()
            && cat.family_for(&v(&["游ゴシック"])).is_some(),
        "実フォント 游ゴシック（Yu Gothic）がこの機械に無い。本試験の期待値は当該フォントの\
         中太を前提にしているので、代替フォントへ縮退したまま緑にしない"
    );

    let (_, events) = capture(|| {
        for name in ["Yu Gothic Medium", "游ゴシック Medium", "yu gothic medium"] {
            let face = cat.face_for(&v(&[name])).expect(name);
            assert_eq!(face.weight.0, 500, "{name} は中太");
            assert_eq!(
                cat.face_for(std::slice::from_ref(&face.family)),
                Some(FontFace::regular(&face.family)),
                "{name} の家族名 {} は家族名として引き直せる",
                face.family
            );
        }
        let pasta = v(&[
            "游ゴシック Medium",
            "Yu Gothic Medium",
            "游ゴシック",
            "Yu Gothic",
            "BIZ UDPゴシック",
            "メイリオ",
        ]);
        assert_eq!(
            cat.face_for(&pasta).expect("ぱすたの候補列").weight.0,
            500,
            "ぱすたの候補列は中太"
        );
        assert_eq!(cat.face_for(&v(&["NoSuchFontXYZ"])), None);
    });
    assert_eq!(
        events
            .iter()
            .filter(|e| e.level == tracing::Level::ERROR)
            .count(),
        0,
        "見つからない名前は想定外の失敗ではない（error! を残さない）: {events:?}"
    );
    assert_eq!(warns(&events, ALL_MISSING), 1, "対照: 全滅の記録は出ている");
}
