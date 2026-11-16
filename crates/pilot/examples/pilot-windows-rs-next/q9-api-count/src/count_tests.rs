use super::*;
use scip::types::{Document, Index, Occurrence};

const W: &str = "rust-analyzer cargo windows 0.62.2 ";

#[test]
fn kind_of_type() {
    let s = format!("{W}Win32/Foundation/HWND#");
    assert_eq!(classify(&s), Some(Kind::Type));
}

#[test]
fn kind_of_interface_method() {
    // rust-analyzer は固有の impl のメソッドを `impl#[型]名().` と綴る
    let s = format!("{W}Win32/Graphics/Direct2D/impl#[ID2D1RenderTarget]Clear().");
    assert_eq!(classify(&s), Some(Kind::Method));
    let s = format!("{W}Win32/Graphics/Direct2D/ID2D1RenderTarget#Clear().");
    assert_eq!(classify(&s), Some(Kind::Method));
}

#[test]
fn kind_of_function() {
    let s = format!("{W}Win32/UI/WindowsAndMessaging/CreateWindowExW().");
    assert_eq!(classify(&s), Some(Kind::Function));
}

#[test]
fn kind_of_constant_and_field() {
    let s = format!("{W}Win32/UI/WindowsAndMessaging/WM_PAINT.");
    assert_eq!(classify(&s), Some(Kind::Constant));
    let s = "rust-analyzer cargo windows-numerics 0.3.1 Vector2#X.";
    assert_eq!(classify(s), Some(Kind::Field));
}

#[test]
fn kind_of_macro() {
    let s = "rust-analyzer cargo windows_strings 0.5.1 w!";
    assert_eq!(classify(s), Some(Kind::Macro));
}

#[test]
fn other_packages_and_locals_are_not_counted() {
    assert_eq!(
        classify("rust-analyzer cargo bevy_ecs 0.19.1 world/World#"),
        None
    );
    assert_eq!(
        classify("rust-analyzer cargo windows-bindgen 0.1 Foo#"),
        None
    );
    assert_eq!(classify("local 12"), None);
    assert_eq!(classify(&format!("{W}Win32/Foundation/")), None);
}

#[test]
fn test_code_by_path() {
    assert!(!is_test_path("crates/wintf/src/com/d2d.rs"));
    assert!(is_test_path("crates/wintf/src/com/d2d_tests.rs"));
    assert!(is_test_path("crates/wintf/tests/layout_test.rs"));
    assert!(is_test_path("crates/wintf/examples/demo.rs"));
    assert!(is_test_path("crates/areka-update/src/testkit.rs"));
    assert!(is_test_path("crates/wintf/src/test_crash_report.rs"));
    assert!(is_test_path(
        "crates/areka/src/install/procedure_test_support.rs"
    ));
    assert!(is_test_path("crates/wintf/src/ecs/test_support.rs"));
    assert!(is_test_path("crates/areka/src/placement_tests/mod.rs"));
    assert!(is_test_path(
        "crates/wintf/src/ecs/layout/hit_test/tests_ex/opacity.rs"
    ));
    assert!(!is_test_path("crates/wintf/src/ecs/layout/hit_test/mod.rs"));
    assert!(!is_test_path(
        "crates/wintf/src/ecs/pointer/nchittest_cache.rs"
    ));
}

#[test]
fn crate_marks() {
    assert!(!is_test_tool("wintf"));
    assert!(!is_test_tool("shiori-host32-helper"));
    assert!(is_test_tool("real-run-kit"));
    assert!(is_test_tool("shiori4-testdll"));
    assert!(is_test_tool("surface-compare"));
}

fn doc(path: &str, symbols: &[&str]) -> Document {
    let mut d = Document::new();
    d.relative_path = path.into();
    for s in symbols {
        let mut o = Occurrence::new();
        o.symbol = (*s).into();
        d.occurrences.push(o);
    }
    d
}

#[test]
fn tally_counts_distinct_and_occurrences_and_skips_pilot() {
    let hwnd = format!("{W}Win32/Foundation/HWND#");
    let cw = format!("{W}Win32/UI/WindowsAndMessaging/CreateWindowExW().");
    let mut index = Index::new();
    index
        .documents
        .push(doc("crates/wintf/src/a.rs", &[&hwnd, &hwnd, &cw]));
    index
        .documents
        .push(doc("crates\\wintf\\src\\a_tests.rs", &[&hwnd]));
    index
        .documents
        .push(doc("crates/real-run-kit/src/lib.rs", &[&hwnd]));
    index
        .documents
        .push(doc("crates/pilot/src/lib.rs", &[&hwnd, &cw]));
    index.documents.push(doc("vendors/x/src/lib.rs", &[&hwnd]));
    let t = tally(&index).unwrap();

    let names: Vec<_> = t.keys().map(|k| k.krate.as_str()).collect();
    assert!(!names.contains(&"pilot"));
    let prod = &t[&Part {
        krate: "wintf".into(),
        test_code: false,
    }];
    assert_eq!(prod.len(), 2);
    assert_eq!(prod[&hwnd], (Kind::Type, 2));

    // 出荷する（本番＋試験）の和: HWND は 1 つと数え、出現は 3 回
    let all = sum(&t, |p| !is_test_tool(&p.krate));
    assert_eq!(all.distinct[Kind::Type as usize], 1);
    assert_eq!(all.occurrences[Kind::Type as usize], 3);
    assert_eq!(all.distinct[Kind::Function as usize], 1);

    let md = render(&t);
    assert!(md.contains(
        "\n| wintf | 出荷する | 本番 | 1 (2) | 0 (0) | 1 (1) | 0 (0) | 2 (3) | 0 (0) | 0 (0) |\n"
    ));
    assert!(md.contains(
        "\n| **合計** | 出荷する | 本番＋試験 | 1 (3) | 0 (0) | 1 (1) | 0 (0) | 2 (4) | 0 (0) | 0 (0) |\n"
    ));
    assert!(md.contains("| real-run-kit | 試験の道具 | 本番 |"));
}

#[test]
fn tally_fails_when_no_document_is_under_crates() {
    // 綴りが合わずに 1 つも拾えない索引を、全部 0 の表で通さない
    let hwnd = format!("{W}Win32/Foundation/HWND#");
    let mut index = Index::new();
    index.documents.push(doc("src/a.rs", &[&hwnd]));
    index.documents.push(doc("vendors/x/src/lib.rs", &[&hwnd]));
    index
        .documents
        .push(doc("crates/pilot/src/lib.rs", &[&hwnd]));
    let err = tally(&index).unwrap_err();
    assert!(err.contains("no documents under crates/ matched"), "{err}");
}
