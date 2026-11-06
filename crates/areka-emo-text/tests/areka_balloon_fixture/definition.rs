//! 文のファイルの中身・本番の読み込みの記録・描画範囲の解け方を固定する。
//!
//! 出典 spec: `default-balloon-selfmade`（要件 **12.4**・**3.5**・**5.1**・**5.3**〜**5.9**・
//! **6.2**・**6.3**・**6.5**・設計「描画範囲の解け方」「Testing Strategy」の `definition.rs`）。
//!
//! 期待値は `test_support.rs` の表と、設計「描画範囲の解け方」の表から手で書く。

use std::collections::BTreeMap;

use areka_emo_atlas::UseSelfAlpha;
use areka_emo_present::balloon::load_balloon_use_self_alpha;
use areka_emo_text::actor::ResolvedBalloonText;
use areka_parsers::balloon::BalloonModel;
use areka_parsers::kv::parse_kv;
use log_capture_kit::{CapturedEvent, capture};

use crate::support::fixture::{
    assert_balloon_loading_records, load_face0_with_records, record_lines,
};

use super::test_support::{
    EXPECTED_DESCRIPT, EXPECTED_FACE_SETTINGS, EXPECTED_INSTALL, balloon_root, expected_region,
    expected_wrap_threshold, scope_face0_file, scope_image_size,
};

/// 1 行目に置く文字コードの宣言（要件 5.3）。
const CHARSET_LINE: &str = "charset,UTF-8";

/// 鍵と値の読み物ではないが、文字コードと 1 行目は同じ規約で書く文のファイル（設計「文のファイル」）。
const README: &str = "readme.txt";

/// scope ごとの面 0 の期待値（`descript.txt` に面別の設定を重ねた宣言値）。面 0 の絵の名前・
/// 寸法・解いた範囲・折り返し基準は `test_support.rs` の表から引く。
struct Face0Expectation {
    scope: u32,
    /// モデルが写す宣言値 `origin.x`・`origin.y`。
    origin: (i32, i32),
    /// モデルが写す宣言値 `validrect.left`・`top`・`right`・`bottom`。
    validrect: (i32, i32, i32, i32),
    /// モデルが写す宣言値 `wordwrappoint.x`・`y`。
    wordwrappoint: (i32, i32),
    /// 面の系列解決で本体側の系列へ縮退する面 `(面 id, 接頭辞)`（`warn!` の欄）。
    /// 相方側は面 2・3 を持たない（要件 2.2）のでちょうど 2 件、本体側は 0 件。
    expected_fallback: &'static [(u32, &'static str)],
}

const FACE0: [Face0Expectation; 2] = [
    Face0Expectation {
        scope: 0,
        origin: (14, 11),
        validrect: (0, 0, 326, -20),
        wordwrappoint: (-30, 0),
        expected_fallback: &[],
    },
    Face0Expectation {
        scope: 1,
        origin: (14, 14),
        validrect: (0, 0, 326, -14),
        wordwrappoint: (-30, 0),
        expected_fallback: &[(2, "balloons"), (3, "balloons")],
    },
];

/// 書体の宣言（`descript.txt` の手本の値・要件 5.6）。
const EXPECTED_FONT_NAME: &str = "ＭＳ ゴシック";
const EXPECTED_FONT_HEIGHT: u32 = 12;
const EXPECTED_FONT_COLOR: (u8, u8, u8) = (0, 0, 0);

/// 鍵と値を比べる文のファイル（`descript.txt`・`install.txt`・面別の設定 6 つ）。
fn kv_tables() -> Vec<(&'static str, &'static [(&'static str, &'static str)])> {
    let mut tables: Vec<(&str, &[(&str, &str)])> = vec![
        ("descript.txt", EXPECTED_DESCRIPT),
        ("install.txt", EXPECTED_INSTALL),
    ];
    tables.extend(EXPECTED_FACE_SETTINGS);
    tables
}

/// 書庫のファイルを生のバイトで読む。読めない・空は名指しで落とす（「対象 0 件で緑」を作らない）。
fn read_bytes(name: &str) -> Vec<u8> {
    let path = balloon_root().join(name);
    let bytes = std::fs::read(&path)
        .unwrap_or_else(|e| panic!("{} を読めない（実在が前提）: {e}", path.display()));
    assert!(!bytes.is_empty(), "{} が空である", path.display());
    bytes
}

/// 印なしの UTF-8 として読み、本文を返す（要件 5.3・5.8）。
fn read_utf8(name: &str) -> String {
    let bytes = read_bytes(name);
    assert!(
        !bytes.starts_with(&[0xEF, 0xBB, 0xBF]),
        "{name} の先頭に UTF-8 の印（BOM）がある"
    );
    String::from_utf8(bytes).unwrap_or_else(|e| panic!("{name} が UTF-8 として読めない: {e}"))
}

/// 各文ファイルが印なしの UTF-8 で、1 行目が `charset,UTF-8`（要件 5.3）。
#[test]
fn every_text_file_is_bomless_utf8_starting_with_the_charset_line() {
    let mut names: Vec<&str> = kv_tables().iter().map(|(n, _)| *n).collect();
    names.push(README);
    for name in names {
        let text = read_utf8(name);
        let first = text.lines().next().unwrap_or_default();
        assert_eq!(
            first, CHARSET_LINE,
            "{name} の 1 行目が `{CHARSET_LINE}` でない"
        );
    }
}

/// 鍵と値の並びが期待値の表と鍵の集合ごと一致する（要件 3.5・5.1・5.4〜5.7・5.9・6.3）。
///
/// 集合が一致するので、表に無い `windowpos.*`・`onlinemarker.filename`／`interval`・`validrect.right,0`
/// は書庫にも無い。
/// 同じ鍵の重複は `parse_kv` が後勝ちで潰すので、鍵を持つ行の数でも突き合わせる。
#[test]
fn key_value_lists_equal_the_expected_tables_including_the_key_set() {
    for (name, expected) in kv_tables() {
        let text = read_utf8(name);
        let actual = parse_kv(&text);
        let expected_map: BTreeMap<String, String> = expected
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        assert_eq!(
            expected_map.len(),
            expected.len(),
            "{name}: 期待値の表に同じ鍵が重なっている"
        );
        assert_eq!(
            actual, expected_map,
            "{name}: 鍵と値の集合が期待値の表と違う"
        );
        let kv_lines = text.lines().filter(|l| l.contains(',')).count();
        assert_eq!(
            kv_lines,
            expected.len(),
            "{name}: 鍵を持つ行が {kv_lines} 行（期待 {}・同じ鍵の重複か余計な行）",
            expected.len()
        );
    }
}

/// 本番の読み込みの経路を同じ捕捉の窓で走らせ、（面 0 のモデル, 面 0 のファイル名, 記録）を返す。
fn load_with_records(scope: u32) -> (BalloonModel, String, Vec<CapturedEvent>) {
    let (faces, model, events) = load_face0_with_records(&balloon_root(), scope);
    let face0_file = faces
        .iter()
        .find(|f| f.surface_id == 0)
        .map(|f| f.file_name.clone())
        .expect("面 0 の実在は load_face0_with_records が確かめている");
    (model, face0_file, events)
}

/// 本番の読み込みで、記録が期待どおり（`ERROR` 0 件・`WARN` は相方側の面 2・3 の縮退 2 件だけ）で、
/// モデルの値が表のとおり（要件 5.8・5.4〜5.6・6.5）。
#[test]
fn production_loading_records_only_the_expected_fallbacks_and_models_the_declared_values() {
    for exp in &FACE0 {
        let scope = exp.scope;
        let (model, face0_file, events) = load_with_records(scope);
        assert_balloon_loading_records(scope, &events, exp.expected_fallback);

        assert_eq!(
            face0_file,
            scope_face0_file(scope),
            "scope {scope}: 面 0 のファイル名"
        );
        let o = model.origin();
        assert_eq!(
            (o.x(), o.y()),
            (Some(exp.origin.0), Some(exp.origin.1)),
            "scope {scope}: origin"
        );
        let v = model.validrect();
        assert_eq!(
            (v.left(), v.top(), v.right(), v.bottom()),
            (
                Some(exp.validrect.0),
                Some(exp.validrect.1),
                Some(exp.validrect.2),
                Some(exp.validrect.3)
            ),
            "scope {scope}: validrect"
        );
        let w = model.wordwrappoint();
        assert_eq!(
            (w.x(), w.y()),
            (Some(exp.wordwrappoint.0), Some(exp.wordwrappoint.1)),
            "scope {scope}: wordwrappoint"
        );
        let font = model.font();
        let c = font.color();
        assert_eq!(
            (font.name(), font.height(), (c.r(), c.g(), c.b())),
            (
                Some(EXPECTED_FONT_NAME),
                Some(EXPECTED_FONT_HEIGHT),
                (
                    Some(EXPECTED_FONT_COLOR.0),
                    Some(EXPECTED_FONT_COLOR.1),
                    Some(EXPECTED_FONT_COLOR.2)
                )
            ),
            "scope {scope}: 書体（名前・高さ・色）"
        );
    }
}

/// 透過の宣言が起動時と同じ入口で `use_self_alpha,1` に読める（要件 5.9）。同じ窓で、扱いを決めた
/// `info!` がちょうど 1 件出る（記録を出す入口を通った対照）ことを見てから、WARN 以上が 0 件と言う。
#[test]
fn use_self_alpha_is_declared_on() {
    let root = balloon_root();
    let (use_self_alpha, events) = capture(|| load_balloon_use_self_alpha(&root));
    assert_eq!(
        use_self_alpha,
        UseSelfAlpha::On,
        "`use_self_alpha,1` のはず"
    );
    let decided: Vec<(Option<&str>, Option<&str>)> = events
        .iter()
        .filter(|e| e.target == "areka_emo_present::self_alpha")
        .filter(|e| e.level == tracing::Level::INFO)
        .map(|e| (e.field_str("kind"), e.field_str("treatment")))
        .collect();
    assert_eq!(
        decided,
        vec![(Some("balloon"), Some("1"))],
        "透過の扱いを決めた記録（対照）"
    );
    let warns: Vec<String> = events
        .iter()
        .zip(record_lines(&events))
        .filter(|(e, _)| matches!(e.level, tracing::Level::WARN | tracing::Level::ERROR))
        .map(|(_, line)| line)
        .collect();
    assert!(warns.is_empty(), "WARN 以上の記録がある: {warns:?}");
}

/// 解いた描画範囲が設計「描画範囲の解け方」の表と同じで、潰れず、折り返し基準が範囲の内、
/// origin が宣言どおり（要件 6.2・6.5）。警告の条件と同じ判定で見る。
#[test]
fn resolved_region_matches_the_design_table() {
    for exp in &FACE0 {
        let scope = exp.scope;
        let (model, _, _) = load_with_records(scope);
        let region = ResolvedBalloonText::resolve(&model, scope_image_size(scope)).region;
        assert_eq!(
            (region.left(), region.top(), region.right(), region.bottom()),
            expected_region(scope),
            "scope {scope}: 範囲（左・上・右・下）"
        );
        assert!(!region.is_collapsed(), "scope {scope}: 範囲が潰れている");
        assert_eq!(
            region.wrap_threshold(),
            expected_wrap_threshold(scope),
            "scope {scope}: 折り返し基準"
        );
        assert!(
            region.wrap_threshold() <= region.inline_limit(),
            "scope {scope}: 折り返し基準 {} が遠辺 {} の外",
            region.wrap_threshold(),
            region.inline_limit()
        );
        // 範囲の外の origin は書字開始角へ捨てられる。宣言どおりなら捨てられていない。
        assert_eq!(
            region.start(),
            (exp.origin.0 as f32, exp.origin.1 as f32),
            "scope {scope}: origin が宣言どおりに解けていない"
        );
    }
}
