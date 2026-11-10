use wintf::ecs::widget::text::label::{Label, TextDirection};

/// `Label::default()` のフィールド既定値（メイリオ/16pt/空文字/横書きLTR）を固定。
/// font_family の既定が日本語フォント名であることを含む（デバイス非依存）。
#[test]
fn test_label_default_values() {
    let label = Label::default();
    assert_eq!(label.text, "");
    assert_eq!(label.font_family, "メイリオ");
    assert_eq!(label.font_size, 16.0);
    assert_eq!(label.direction, TextDirection::HorizontalLeftToRight);
}
