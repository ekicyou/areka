//! 倍率と DPI の読み手の決定論テスト（偽の読み口と手で書いた表・要件 3・4・7.2）。

use super::*;
use areka_sylphya::{
    AskerContext, AskerId, DottedResolution, MirrorImage, SharedMirror, parse_dotted,
};

const SHAPES: [&str; 4] = [
    "currentghost.scope({}).scaling",
    "currentghost.balloon.scope({}).scaling",
    "currentghost.scope({}).currentmonitor.dpi",
    "currentghost.balloon.scope({}).currentmonitor.dpi",
];

fn name(shape: &str, sel: &str) -> String {
    shape.replace("{}", sel)
}

fn ratio(num: u32, den: u32) -> ScaleRatio {
    ScaleRatio::new(num, den).unwrap()
}

fn applied(num: u32, den: u32, dpi_x: Option<u16>) -> AppliedScale {
    AppliedScale {
        ratio: ratio(num, den),
        dpi_x,
    }
}

fn reader() -> SylphyaReader {
    SylphyaReader::new(SharedMirror::new(Arc::new(MirrorImage::empty())))
}

fn resolve(reader: &SylphyaReader, name: &str) -> Option<String> {
    let asker = AskerContext {
        asker: AskerId::new("test"),
    };
    match reader.resolve_dotted_str(&asker, name) {
        DottedResolution::Value(v) => Some(v),
        DottedResolution::NotFound => None,
    }
}

/// 作者の基準 96 の絵を 150% の画面（DPI 144）に出したスコープ 0 の表。
fn table_150() -> SharedScopeScaleTable {
    let mut t = ScopeScaleTable::default();
    t.set(0, TargetWindow::Shell, applied(144, 96, Some(144)));
    t.set(0, TargetWindow::Balloon, applied(144, 96, Some(144)));
    Arc::new(RwLock::new(t))
}

fn wired(table: SharedScopeScaleTable) -> SylphyaReader {
    let r = reader();
    wire(&r, Some(table));
    r
}

#[test]
fn percent_text_table() {
    for (num, den, want) in [
        (96, 96, "100"),
        (144, 96, "150"),
        (192, 96, "200"),
        (120, 144, "83.33"),
        (135, 120, "112.5"),
        (1, 3, "33.33"),
        (2, 3, "66.67"),
    ] {
        assert_eq!(percent_text(ratio(num, den)), want, "{num}/{den}");
    }
}

#[test]
fn scaling_text_joins_the_same_percent_twice() {
    assert_eq!(scaling_text(ratio(144, 96)), "150,150");
    assert_eq!(scaling_text(ratio(1, 3)), "33.33,33.33");
}

/// 150% の画面の例で 4 つの形が `150,150` と `144` を返す（要件 3.1・3.2・3.5・3.8・3.11）。
#[test]
fn four_shapes_answer_from_the_table() {
    let r = wired(table_150());
    let want = ["150,150", "150,150", "144", "144"];
    for (shape, want) in SHAPES.iter().zip(want) {
        assert_eq!(
            resolve(&r, &name(shape, "0")).as_deref(),
            Some(want),
            "{shape}"
        );
    }
}

/// 窓ごとに別の行を読む（シェルとバルーンで倍率が違う）。
#[test]
fn shell_and_balloon_read_their_own_row() {
    let mut t = ScopeScaleTable::default();
    t.set(1, TargetWindow::Shell, applied(192, 96, Some(192)));
    t.set(1, TargetWindow::Balloon, applied(120, 144, Some(120)));
    let r = wired(Arc::new(RwLock::new(t)));
    let want = ["200,200", "83.33,83.33", "192", "120"];
    for (shape, want) in SHAPES.iter().zip(want) {
        assert_eq!(
            resolve(&r, &name(shape, "1")).as_deref(),
            Some(want),
            "{shape}"
        );
    }
}

/// 窓の無い番号・負の番号・書式の誤りは値なし（要件 3.9）。
#[test]
fn scope_without_window_or_malformed_selector_has_no_value() {
    let table = table_150();
    for (i, shape) in SHAPES.iter().enumerate() {
        let window = if i % 2 == 0 {
            TargetWindow::Shell
        } else {
            TargetWindow::Balloon
        };
        let quantity = if i < 2 {
            ScaleQuantity::Scaling
        } else {
            ScaleQuantity::Dpi
        };
        let src = ScopeScaleSource::new(window, quantity, table.clone());
        // 対照: 窓のある番号には答える（値なしが読み手の故障でないことの確かめ）。
        assert!(
            src.read(&parse_dotted(&name(shape, "0")).unwrap())
                .is_some(),
            "{shape}"
        );
        for sel in ["1", "-1", "x", "", "+1"] {
            let path = parse_dotted(&name(shape, sel)).unwrap();
            assert_eq!(src.read(&path), None, "{shape} {sel}");
        }
    }
}

/// 片方の窓だけの行は、もう片方の窓に値なし。
#[test]
fn row_with_one_window_has_no_value_for_the_other() {
    let mut t = ScopeScaleTable::default();
    t.set(0, TargetWindow::Shell, applied(144, 96, Some(144)));
    let r = wired(Arc::new(RwLock::new(t)));
    assert_eq!(
        resolve(&r, &name(SHAPES[0], "0")).as_deref(),
        Some("150,150")
    );
    assert_eq!(resolve(&r, &name(SHAPES[1], "0")), None);
    assert_eq!(resolve(&r, &name(SHAPES[3], "0")), None);
}

/// DPI の無い行は、倍率には答え DPI は値なし。
#[test]
fn row_without_dpi_has_no_dpi_value() {
    let mut t = ScopeScaleTable::default();
    t.set(0, TargetWindow::Shell, applied(144, 96, None));
    t.set(0, TargetWindow::Balloon, applied(144, 96, None));
    let r = wired(Arc::new(RwLock::new(t)));
    assert_eq!(
        resolve(&r, &name(SHAPES[0], "0")).as_deref(),
        Some("150,150")
    );
    assert_eq!(
        resolve(&r, &name(SHAPES[1], "0")).as_deref(),
        Some("150,150")
    );
    assert_eq!(resolve(&r, &name(SHAPES[2], "0")), None);
    assert_eq!(resolve(&r, &name(SHAPES[3], "0")), None);
}

/// 表を書き換えた後の読みは新しい値（要件 4.1）。
#[test]
fn reading_after_rewriting_the_table_returns_the_new_value() {
    let table = table_150();
    let r = wired(table.clone());
    table
        .write()
        .unwrap()
        .set(0, TargetWindow::Shell, applied(192, 96, Some(192)));
    assert_eq!(
        resolve(&r, &name(SHAPES[0], "0")).as_deref(),
        Some("200,200")
    );
    assert_eq!(resolve(&r, &name(SHAPES[2], "0")).as_deref(), Some("192"));
    // 書き換えなかったバルーンは前の値のまま。
    assert_eq!(
        resolve(&r, &name(SHAPES[1], "0")).as_deref(),
        Some("150,150")
    );
}

/// 毒された鍵でも中身で答える（要件 4.3）。
#[test]
fn poisoned_lock_still_answers() {
    let table = table_150();
    let r = wired(table.clone());
    let t = table.clone();
    let _ = std::thread::spawn(move || {
        let _guard = t.write().unwrap();
        panic!("鍵を毒す");
    })
    .join();
    assert!(table.is_poisoned());
    assert_eq!(
        resolve(&r, &name(SHAPES[0], "0")).as_deref(),
        Some("150,150")
    );
}

/// 配線の無い起動（`None`）は空の表で登録する＝どの名前も値なし。
#[test]
fn wire_without_table_answers_nothing() {
    let r = reader();
    wire(&r, None);
    for shape in SHAPES {
        assert_eq!(resolve(&r, &name(shape, "0")), None, "{shape}");
    }
}

#[path = "ghost_session_scope_scale_boot_tests.rs"]
mod boot;
