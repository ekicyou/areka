//! `outcome` の決定論テスト（結果の 4 つの形）。

use super::*;
use crate::{ToolContent, ToolOutcome};

fn text(s: &str) -> ToolContent {
    ToolContent::Text(s.to_string())
}

#[test]
fn value_is_bare_text_without_ok_prefix() {
    assert_eq!(
        value("emo2"),
        ToolOutcome {
            content: vec![text("emo2")],
            is_error: false,
        }
    );
}

#[test]
fn ok_with_empty_note_is_ok_without_colon() {
    assert_eq!(
        ok(""),
        ToolOutcome {
            content: vec![text("OK")],
            is_error: false,
        }
    );
}

#[test]
fn ok_with_note_is_ok_colon_note() {
    assert_eq!(
        ok("reloaded"),
        ToolOutcome {
            content: vec![text("OK:reloaded")],
            is_error: false,
        }
    );
}

#[test]
fn ng_is_ng_colon_reason_and_is_error() {
    assert_eq!(
        ng("not implemented yet"),
        ToolOutcome {
            content: vec![text("NG:not implemented yet")],
            is_error: true,
        }
    );
}

#[test]
fn with_image_appends_one_png_after_text_and_keeps_is_error() {
    let png = "iVBORw0KGgo=".to_string();
    let image = ToolContent::Image {
        data: png.clone(),
        mime_type: "image/png".to_string(),
    };
    assert_eq!(
        with_image(ok(""), png.clone()),
        ToolOutcome {
            content: vec![text("OK"), image.clone()],
            is_error: false,
        }
    );
    assert_eq!(
        with_image(ng("x"), png),
        ToolOutcome {
            content: vec![text("NG:x"), image],
            is_error: true,
        }
    );
}
