#[cfg(test)]
mod tests {
    use wintf::ecs::TextLayoutMetrics;
    use wintf::ecs::widget::text::TextLayoutResource;
    use wintf::ecs::widget::text::label::{Label, TextDirection};

    #[test]
    fn test_text_direction_enum() {
        let direction = TextDirection::default();
        assert_eq!(direction, TextDirection::HorizontalLeftToRight);

        let vertical = TextDirection::VerticalRightToLeft;
        assert_eq!(vertical, TextDirection::VerticalRightToLeft);
    }

    #[test]
    fn test_label_has_direction() {
        let label = Label {
            text: "Test".to_string(),
            direction: TextDirection::VerticalRightToLeft,
            ..Default::default()
        };
        assert_eq!(label.direction, TextDirection::VerticalRightToLeft);
    }

    #[test]
    fn test_text_layout_metrics() {
        let metrics = TextLayoutMetrics {
            width: 100.0,
            height: 200.0,
        };
        assert_eq!(metrics.width, 100.0);
        assert_eq!(metrics.height, 200.0);
    }

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

    /// `TextLayoutResource::empty()` は中身 None で `get()` も None を返す。
    /// COM/DirectWrite 不要な経路の特性化（`new()` は実 IDWriteTextLayout を要するため対象外）。
    #[test]
    fn test_text_layout_resource_empty_returns_none() {
        let res = TextLayoutResource::empty();
        assert!(res.get().is_none());
    }
}
