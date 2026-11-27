//! 倍率と DPI の読み手の決定論テスト（偽の読み口と手で書いた表）。

use super::test_support::{applied, ratio};
use super::*;
use areka_sylphya::store::MemBackend;
use areka_sylphya::{
    AskerContext, AskerId, DottedResolution, PersistScope, ScopeRoots, SylphyaInit, SylphyaParts,
    parse_dotted, spawn_sylphya,
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

fn parts() -> SylphyaParts {
    spawn_sylphya(SylphyaInit::new(ScopeRoots::default(), MemBackend::new()))
}

fn reader() -> SylphyaReader {
    parts().reader
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
fn table_150() -> ScopeScaleCells {
    let t = ScopeScaleCells::default();
    t.set(0, TargetWindow::Shell, applied(144, 96, Some(144)));
    t.set(0, TargetWindow::Balloon, applied(144, 96, Some(144)));
    t
}

fn wired(table: ScopeScaleCells) -> SylphyaReader {
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

/// 150% の画面の例で 4 つの形が `150,150` と `144` を返す。
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
    let t = ScopeScaleCells::default();
    t.set(1, TargetWindow::Shell, applied(192, 96, Some(192)));
    t.set(1, TargetWindow::Balloon, applied(120, 144, Some(120)));
    let r = wired(t);
    let want = ["200,200", "83.33,83.33", "192", "120"];
    for (shape, want) in SHAPES.iter().zip(want) {
        assert_eq!(
            resolve(&r, &name(shape, "1")).as_deref(),
            Some(want),
            "{shape}"
        );
    }
}

/// 窓の無い番号・負の番号・書式の誤りは値なし。
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
            src.read_text(&parse_dotted(&name(shape, "0")).unwrap())
                .is_some(),
            "{shape}"
        );
        for sel in ["1", "-1", "x", "", "+1"] {
            let path = parse_dotted(&name(shape, sel)).unwrap();
            assert_eq!(src.read_text(&path), None, "{shape} {sel}");
        }
    }
}

/// 片方の窓だけの行は、もう片方の窓に値なし。
#[test]
fn row_with_one_window_has_no_value_for_the_other() {
    let t = ScopeScaleCells::default();
    t.set(0, TargetWindow::Shell, applied(144, 96, Some(144)));
    let r = wired(t);
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
    let t = ScopeScaleCells::default();
    t.set(0, TargetWindow::Shell, applied(144, 96, None));
    t.set(0, TargetWindow::Balloon, applied(144, 96, None));
    let r = wired(t);
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

/// 表を書き換えた後の読みは新しい値。
#[test]
fn reading_after_rewriting_the_table_returns_the_new_value() {
    let table = table_150();
    let r = wired(table.clone());
    table.set(0, TargetWindow::Shell, applied(192, 96, Some(192)));
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

/// 4 つの形はゴーストの層の速い住み場所に住む（ほかの層を名指しした読みは値なし）。
#[test]
fn four_shapes_live_in_the_ghost_layer() {
    let r = wired(table_150());
    let kvs = r.kvs(AskerId::new("test"));
    let want = ["150,150", "150,150", "144", "144"];
    for (shape, want) in SHAPES.iter().zip(want) {
        let key = name(shape, "0");
        assert_eq!(
            kvs.get_in(PersistScope::Ghost, &key).as_deref(),
            Some(want),
            "{shape}"
        );
        for layer in [
            PersistScope::App,
            PersistScope::Shell,
            PersistScope::Balloon,
        ] {
            assert_eq!(kvs.get_in(layer, &key), None, "{shape} {layer:?}");
        }
    }
}

/// 名前では書けない（書いた後も取っ手の値のまま・取っ手も変わらない）。
#[test]
fn names_are_not_writable() {
    let p = parts();
    let table = table_150();
    wire(&p.reader, Some(table.clone()));
    for shape in SHAPES {
        p.publisher
            .set(AskerId::new("test"), name(shape, "0"), "1".to_owned());
    }
    p.publisher.barrier().expect("番人が居る");
    let want = ["150,150", "150,150", "144", "144"];
    for (shape, want) in SHAPES.iter().zip(want) {
        assert_eq!(
            resolve(&p.reader, &name(shape, "0")).as_deref(),
            Some(want),
            "{shape}"
        );
    }
    assert_eq!(
        table.get(0, TargetWindow::Shell),
        Some(applied(144, 96, Some(144)))
    );
}

/// 層の値を捨てると 4 つとも値なし。
#[test]
fn clear_empties_every_shape() {
    let table = table_150();
    let r = wired(table.clone());
    ScopeScaleSource::new(TargetWindow::Shell, ScaleQuantity::Scaling, table).clear();
    for shape in SHAPES {
        assert_eq!(resolve(&r, &name(shape, "0")), None, "{shape}");
    }
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
