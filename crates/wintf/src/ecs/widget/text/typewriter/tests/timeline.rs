//! 時間表と隠す範囲をクラスタと UTF-16 で数える

use super::*;
use crate::ecs::widget::text::typewriter_draw::hidden_text_range;
use crate::ecs::widget::text::typewriter_layout::timeline_items;

/// 項目を (種類, クラスタ番号) に縮めて並びを比べやすくする。
fn item_kinds(items: &[TimelineItem]) -> Vec<(&'static str, Option<u32>)> {
    items
        .iter()
        .map(|item| match item {
            TimelineItem::Glyph { cluster_index, .. } => ("glyph", Some(*cluster_index)),
            TimelineItem::Wait { .. } => ("wait", None),
            TimelineItem::FireEvent { .. } => ("event", None),
        })
        .collect()
}

#[test]
fn timeline_counts_surrogate_pair_as_one_cluster() {
    // 𠮷 は UTF-16 で 2 つ・クラスタは 1 つ。
    let tokens = vec![TypewriterToken::Text("𠮷野家です".into())];
    let items = timeline_items(&tokens, &[2, 1, 1, 1, 1], 0.1);
    assert_eq!(
        item_kinds(&items),
        (0..5).map(|i| ("glyph", Some(i))).collect::<Vec<_>>()
    );
}

#[test]
fn timeline_puts_glyph_after_wait_when_cluster_spans_two_chars() {
    // ❤️ は 2 文字（U+2764 U+FE0F）で 1 クラスタ。「!」は待ちの後ろに来る。
    let tokens = vec![
        TypewriterToken::Text("\u{2764}\u{FE0F}ok".into()),
        TypewriterToken::Wait(1.0),
        TypewriterToken::Text("!".into()),
    ];
    let items = timeline_items(&tokens, &[2, 1, 1, 1], 0.1);
    assert_eq!(
        item_kinds(&items),
        vec![
            ("glyph", Some(0)),
            ("glyph", Some(1)),
            ("glyph", Some(2)),
            ("wait", None),
            ("glyph", Some(3)),
        ]
    );
    let TimelineItem::Glyph { show_at, .. } = items[4] else {
        panic!("last item must be a glyph");
    };
    assert_eq!(show_at, ms(1400));
}

#[test]
fn hidden_range_is_counted_in_utf16() {
    // 𠮷野家です は UTF-16 で 6。𠮷 を 1 つ見せたら (2, 4) を隠す。
    let lengths = [2, 1, 1, 1, 1];
    assert_eq!(hidden_text_range(&lengths, 1), Some((2, 4)));
    assert_eq!(hidden_text_range(&lengths, 0), Some((0, 6)));
    assert_eq!(hidden_text_range(&lengths, 5), None);
    assert_eq!(hidden_text_range(&lengths, 9), None);
}

/// 時間表の項目の (種類, 頭からの時刻, 長さ)。
fn item_times(items: &[TimelineItem]) -> Vec<(&'static str, TimeSpan, TimeSpan)> {
    items
        .iter()
        .map(|item| match item {
            TimelineItem::Glyph { show_at, .. } => ("glyph", *show_at, TimeSpan::ZERO),
            TimelineItem::Wait { start_at, duration } => ("wait", *start_at, *duration),
            TimelineItem::FireEvent { fire_at, .. } => ("event", *fire_at, TimeSpan::ZERO),
        })
        .collect()
}

/// 長さにできない待ち（負・NaN・無限大）は長さ 0 になり、後ろの字の時刻を動かさない。
#[test]
fn unrepresentable_waits_become_zero_length() {
    let tokens = vec![
        TypewriterToken::Text("ab".into()),
        TypewriterToken::Wait(-1.0),
        TypewriterToken::Text("c".into()),
        TypewriterToken::Wait(f64::NAN),
        TypewriterToken::Text("d".into()),
        TypewriterToken::Wait(f64::INFINITY),
    ];
    let items = timeline_items(&tokens, &[1, 1, 1, 1], 0.1);
    assert_eq!(
        item_times(&items),
        vec![
            ("glyph", ms(100), TimeSpan::ZERO),
            ("glyph", ms(200), TimeSpan::ZERO),
            ("wait", ms(200), TimeSpan::ZERO),
            ("glyph", ms(300), TimeSpan::ZERO),
            ("wait", ms(300), TimeSpan::ZERO),
            ("glyph", ms(400), TimeSpan::ZERO),
            ("wait", ms(400), TimeSpan::ZERO),
        ]
    );
}

/// 長さにできない 1 字の待ちの既定は長さ 0 になり、字は待ちの指定の時刻だけで現れる。
#[test]
fn unrepresentable_default_char_wait_becomes_zero_length() {
    let tokens = vec![
        TypewriterToken::Text("a".into()),
        TypewriterToken::Wait(0.5),
        TypewriterToken::Text("b".into()),
    ];
    for char_wait in [f64::NAN, -0.05, f64::NEG_INFINITY] {
        let items = timeline_items(&tokens, &[1, 1], char_wait);
        assert_eq!(
            item_times(&items),
            vec![
                ("glyph", TimeSpan::ZERO, TimeSpan::ZERO),
                ("wait", TimeSpan::ZERO, ms(500)),
                ("glyph", ms(500), TimeSpan::ZERO),
            ],
            "char_wait = {char_wait}"
        );
    }
}

/// 行程の終わりは、最後に済む項目（待ちは待ち終わり）の時刻。空なら 0。
#[test]
fn timeline_end_is_where_last_item_finishes() {
    let target = Entity::from_raw_u32(1).unwrap();
    assert_eq!(wait_then_fire_timeline(target).end(), ms(1400));

    let mut trailing_wait = make_glyph_timeline(2, ms(100));
    trailing_wait.items.push(TimelineItem::Wait {
        duration: ms(1000),
        start_at: ms(200),
    });
    assert_eq!(trailing_wait.end(), ms(1200));

    assert_eq!(make_glyph_timeline(0, ms(100)).end(), TimeSpan::ZERO);
}
