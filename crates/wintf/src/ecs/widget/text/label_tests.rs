//! 向きの設定（19 番）: 4 つの向きの組み合わせと、その書式で配置が作れること（DirectWrite だけ）

use super::*;
use crate::com::dwrite::{DWriteFactoryExt, dwrite_create_factory};
use crate::ecs::graphics::GraphicsCore;
use crate::ecs::widget::brushes::Brushes;
use crate::ecs::widget::text::draw_labels;
use bevy_ecs::prelude::*;
use windows::core::HSTRING;

#[test]
fn test_apply_direction_sets_reading_and_flow_for_all_directions() {
    let factory = dwrite_create_factory(DWRITE_FACTORY_TYPE_SHARED).expect("DWrite factory");
    let cases = [
        (
            TextDirection::HorizontalLeftToRight,
            DWRITE_READING_DIRECTION_LEFT_TO_RIGHT,
            DWRITE_FLOW_DIRECTION_TOP_TO_BOTTOM,
        ),
        (
            TextDirection::HorizontalRightToLeft,
            DWRITE_READING_DIRECTION_RIGHT_TO_LEFT,
            DWRITE_FLOW_DIRECTION_TOP_TO_BOTTOM,
        ),
        (
            TextDirection::VerticalRightToLeft,
            DWRITE_READING_DIRECTION_TOP_TO_BOTTOM,
            DWRITE_FLOW_DIRECTION_RIGHT_TO_LEFT,
        ),
        (
            TextDirection::VerticalLeftToRight,
            DWRITE_READING_DIRECTION_TOP_TO_BOTTOM,
            DWRITE_FLOW_DIRECTION_LEFT_TO_RIGHT,
        ),
    ];
    for (direction, reading, flow) in cases {
        let format = factory
            .create_text_format(
                &HSTRING::from("メイリオ"),
                None::<&IDWriteFontCollection>,
                DWRITE_FONT_WEIGHT_NORMAL,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                16.0,
                &HSTRING::from("ja-JP"),
            )
            .expect("text format");
        apply_direction(&format, direction).expect("apply_direction");
        unsafe {
            assert_eq!(format.GetReadingDirection(), reading, "{direction:?}");
            assert_eq!(format.GetFlowDirection(), flow, "{direction:?}");
        }
        // DirectWrite は組み合わせを配置を使う所で確かめるので、配置の寸法まで取る
        let layout = factory
            .create_text_layout(&HSTRING::from("あいう"), &format, 0.0, 0.0)
            .expect("text layout");
        let mut metrics = DWRITE_TEXT_METRICS::default();
        unsafe { layout.GetMetrics(&mut metrics) }
            .unwrap_or_else(|e| panic!("GetMetrics {direction:?}: {e:?}"));
    }
}

/// 文字の描く係が骨組みを通り、4 つの向きのどれでも命令の列を入れる（実 GPU）
#[test]
fn test_draw_labels_records_command_list_for_all_directions() {
    let mut world = World::new();
    world.insert_resource(GraphicsCore::new().expect("GraphicsCore"));
    let entities: Vec<Entity> = [
        TextDirection::HorizontalLeftToRight,
        TextDirection::HorizontalRightToLeft,
        TextDirection::VerticalRightToLeft,
        TextDirection::VerticalLeftToRight,
    ]
    .into_iter()
    .map(|direction| {
        world
            .spawn((
                Label {
                    text: "あいう".to_string(),
                    direction,
                    ..Default::default()
                },
                Brushes::default(),
            ))
            .id()
    })
    .collect();
    let mut schedule = Schedule::default();
    schedule.add_systems(draw_labels);
    schedule.run(&mut world);
    for entity in entities {
        let list = world.get::<GraphicsCommandList>(entity).expect("命令の列");
        assert!(list.command_list().is_some(), "{entity:?} は描けている");
    }
}

/// 配置が作れない文字（大きさ 0）は、骨組みの道で空の列が入り、毎回やり直さない（7.1 二度目の見直し N2・実 GPU）
#[test]
fn label_whose_layout_cannot_be_built_gets_empty_list() {
    let mut world = World::new();
    world.insert_resource(GraphicsCore::new().expect("GraphicsCore"));
    let entity = world
        .spawn((
            Label {
                text: "あいう".to_string(),
                font_size: 0.0,
                ..Default::default()
            },
            Brushes::default(),
        ))
        .id();
    let mut schedule = Schedule::default();
    schedule.add_systems(draw_labels);
    schedule.run(&mut world);
    let list = world
        .get::<GraphicsCommandList>(entity)
        .expect("空の列が入る（Without<GraphicsCommandList> から外れる）");
    assert!(list.command_list().is_none());
}
