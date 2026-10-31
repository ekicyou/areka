//! 矢印の鍵の読み（`marker.rs`）のテスト（balloon-scroll-arrows 要件 1.1〜1.6・4.2・7.1）。
//! 公開の入口 `parse_str` を通して、2 層の後勝ちに乗っていることごと固定する。

use super::parse_str;
use super::{ArrowKeys, KeyIssue, MarkerCoord, MarkerPoint};

fn keys(descript: &str, image: Option<&str>) -> ArrowKeys {
    parse_str(descript, image).arrow_keys().clone()
}

fn x0(value: &str) -> ArrowKeys {
    keys(&format!("arrow0.x,{value}\r\n"), None)
}

#[test]
fn coord_forms() {
    assert_eq!(x0("12").arrow0.x, Some(MarkerCoord::FromStart(12)));
    assert_eq!(x0("0").arrow0.x, Some(MarkerCoord::FromStart(0)));
    assert_eq!(x0("-12").arrow0.x, Some(MarkerCoord::FromEnd(-12)));
    assert_eq!(x0("--12").arrow0.x, Some(MarkerCoord::Literal(-12)));
    for k in [x0("12"), x0("0"), x0("-12"), x0("--12")] {
        assert!(k.issues.is_empty());
    }
}

#[test]
fn unreadable_values_are_absent_and_recorded() {
    for bad in ["abc", "---1", "", "  ", "--", "--1a"] {
        let k = x0(bad);
        assert_eq!(k.arrow0.x, None, "{bad:?}");
        // 空白は kv 層の trim で空になる（他の数値の鍵と同じ扱い）。
        assert_eq!(
            k.issues,
            vec![KeyIssue {
                key: "arrow0.x",
                value: bad.trim().to_owned()
            }],
            "{bad:?}"
        );
    }
}

#[test]
fn surrounding_spaces_are_trimmed_like_other_keys() {
    assert_eq!(x0(" -3 ").arrow0.x, Some(MarkerCoord::FromEnd(-3)));
}

#[test]
fn resolve_against_extent() {
    assert_eq!(MarkerCoord::FromStart(12).resolve(100), 12);
    assert_eq!(MarkerCoord::FromEnd(-12).resolve(100), 88);
    assert_eq!(MarkerCoord::Literal(-12).resolve(100), -12);
}

#[test]
fn all_keys_and_filename_pass_through() {
    let k = keys(
        "arrow0.x,1\r\narrow0.y,2\r\narrow1.x,-3\r\narrow1.y,--4\r\narrow.filename,myarrow\r\n",
        None,
    );
    assert_eq!(
        k,
        ArrowKeys {
            arrow0: MarkerPoint {
                x: Some(MarkerCoord::FromStart(1)),
                y: Some(MarkerCoord::FromStart(2)),
            },
            arrow1: MarkerPoint {
                x: Some(MarkerCoord::FromEnd(-3)),
                y: Some(MarkerCoord::Literal(-4)),
            },
            filename: Some("myarrow".to_owned()),
            issues: vec![],
        }
    );
}

#[test]
fn per_surface_file_overrides_descript() {
    let k = keys(
        "arrow0.x,1\r\narrow0.y,2\r\narrow.filename,a\r\n",
        Some("arrow0.x,-9\r\narrow.filename,b\r\n"),
    );
    assert_eq!(k.arrow0.x, Some(MarkerCoord::FromEnd(-9)));
    assert_eq!(k.arrow0.y, Some(MarkerCoord::FromStart(2)));
    assert_eq!(k.filename.as_deref(), Some("b"));
}

#[test]
fn one_sided_point_has_no_position() {
    let k = keys("arrow0.x,5\r\narrow1.y,6\r\n", None);
    assert_eq!(k.arrow0.both(), None);
    assert_eq!(k.arrow1.both(), None);
    let k = keys("arrow0.x,5\r\narrow0.y,6\r\n", None);
    assert_eq!(
        k.arrow0.both(),
        Some((MarkerCoord::FromStart(5), MarkerCoord::FromStart(6)))
    );
}

#[test]
fn no_keys_leaves_model_unchanged() {
    let text = "font.height,12\r\nvalidrect.left,3\r\n";
    let model = parse_str(text, None);
    assert_eq!(model.arrow_keys(), &ArrowKeys::default());
    // 鍵の無い定義では、鍵を足す前の作り（with_arrow_keys なし）と等しい。
    assert_eq!(model.clone().with_arrow_keys(ArrowKeys::default()), model);
    assert_eq!(model.font().height(), Some(12));
}
