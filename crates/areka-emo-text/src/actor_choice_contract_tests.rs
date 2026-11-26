use areka_sakura::contract::{ActorKey, CueCommand};
use dola::time::testing::ms;

use super::test_support::{
    choice_cue, com_world, count_px, cue, cursor_model, geo_model, inked, opaque_count, read_back,
    row_px,
};
use super::{ResolvedBalloonText, TextLayerRuntime, TextSlotBinding, present_frame};
use crate::choice::ResolvedChoiceStyle;
use crate::state::TextLayerConfig;
use crate::state::test_support::font;

/// `choice_active`: スパン非空で true・スパン無し／未知 actor で false。
#[test]
fn choice_active_reflects_span_set_presence() {
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    let choosing = ActorKey::from("0");
    let plain = ActorKey::from("1");
    let unknown = ActorKey::from("9");

    rt.apply_cue(&choice_cue("0", ms(0), "q", "はい", &[]));
    rt.apply_cue(&cue("1", ms(0), CueCommand::Text("ただの本文".into())));

    assert!(rt.choice_active(&choosing), "Choice cue 消費 actor は true");
    assert!(
        !rt.choice_active(&plain),
        "Text のみ（選択肢スパン無し）actor は false"
    );
    assert!(!rt.choice_active(&unknown), "未知 actor は false");

    // 現存しない ordinal の hover を注入しても panic せず、照会はスパンの実状態のまま。
    rt.inject_choice_hover(&choosing, Some(999));
    assert!(rt.choice_active(&choosing), "stale ordinal 注入後も true");
}

/// `choice_hit_rows`: 未知 actor・選択肢無し actor は空 slice を返す
/// （スナップショットは提示（`present_frame`）が作るので、提示の前は選択肢があっても空）。
#[test]
fn choice_hit_rows_empty_for_unknown_or_no_snapshot_actor() {
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    let unknown = ActorKey::from("9");
    assert!(
        rt.choice_hit_rows(&unknown).is_empty(),
        "未知 actor は空 slice"
    );

    // 選択肢スパンを持つ actor でも、提示の前はスナップショットが無い＝空 slice。
    let actor = ActorKey::from("0");
    rt.apply_cue(&choice_cue("0", ms(0), "q", "はい", &[]));
    assert!(rt.choice_hit_rows(&actor).is_empty(), "提示の前は空 slice");
}

/// 提示成功直後の `choice_hit_rows` は、描画された選択肢行と整合する非退化矩形を
/// 選択肢数ぶん返す。続く `NoChange` フレームではスナップショットが不変で再描画も起きない。
#[test]
fn present_populates_choice_hit_rows_and_nochange_preserves_snapshot() {
    let (mut world, window, slot) = com_world();
    let actor = ActorKey::from("0");
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    rt.apply_cue(&choice_cue("0", ms(0), "OnYes", "はい", &["r0"]));
    rt.apply_cue(&choice_cue("0", ms(200), "OnNo", "いいえ", &["r1"]));
    let image = (120u32, 60u32);
    rt.register_actor(
        actor.clone(),
        TextSlotBinding::new(slot, window, 1.0, image, image),
        ResolvedBalloonText::resolve(&geo_model(), image),
    );

    present_frame(&mut rt, &mut world, ms(10000)).expect("提示フレーム");
    let rows: Vec<super::ChoiceHitRow> = rt.choice_hit_rows(&actor).to_vec();
    assert_eq!(rows.len(), 2, "選択肢数と同数のヒット行が並ぶ");
    assert_eq!(rows[0].ordinal, 0);
    assert_eq!(rows[0].id, "OnYes");
    assert_eq!(rows[0].label, "はい");
    assert_eq!(rows[0].references, vec!["r0".to_string()]);
    assert_eq!(rows[1].ordinal, 1);
    assert_eq!(rows[1].id, "OnNo");
    assert_eq!(rows[1].label, "いいえ");
    assert_eq!(rows[1].references, vec!["r1".to_string()]);
    for r in &rows {
        assert!(r.rect.left < r.rect.right, "行内幅>0: {:?}", r.rect);
        assert!(r.rect.top < r.rect.bottom, "行高>0: {:?}", r.rect);
    }
    // 横並び 2 選択肢: 配送順 1 の行内範囲は 0 の右側（k=1・committed=0・原点 0＝隣接）。
    assert!(
        rows[1].rect.left >= rows[0].rect.right - 0.5,
        "配送順 1 は 0 の右側に配置される: {:?} / {:?}",
        rows[0].rect,
        rows[1].rect
    );

    // NoChange フレーム: 状態不変で再提示＝再描画は起きず、スナップショットは不変。
    let calls_before = rt
        .draw_stats(&actor)
        .expect("draw_stats")
        .draw_text_layout_calls;
    present_frame(&mut rt, &mut world, ms(10000)).expect("NoChange 再提示");
    let calls_after = rt
        .draw_stats(&actor)
        .expect("draw_stats")
        .draw_text_layout_calls;
    assert_eq!(
        calls_after, calls_before,
        "NoChange フレームは再描画しない（DrawTextLayout 不変）"
    );
    assert_eq!(
        rt.choice_hit_rows(&actor),
        rows.as_slice(),
        "NoChange フレームは直前スナップショットを不変に保つ（更新スキップ）"
    );
}

/// **装飾で大きくした選択肢は当たり帯も大きくなる**（ブロック軸・本番の 1 フレーム経路）。
///
/// 帯の丈を既定の大きさで**アクターに 1 つ**決めると、`\f[height,40]` の
/// 選択肢が表示 42 画素でも帯 14 画素で、文字の上下がクリックできない。値そのものは
/// `choice_tests.rs::line_bands_scale_with_each_line_own_em` が固定し、本テストは
/// それが `present_frame` の本番経路まで繋がっていることを固定する。
///
/// 較正: `choice.rs` の `line_bands` を「全行に既定の大きさで 1 つ」を配る形へ戻すと、
/// 2 行の帯の丈が等しくなって赤。
#[test]
fn a_decorated_choice_gets_a_taller_hit_band_through_the_production_frame() {
    let (mut world, window, slot) = com_world();
    let actor = ActorKey::from("0");
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    // 1 行目＝既定の大きさの選択肢・2 行目＝`\f[height,40]` の選択肢。
    rt.apply_cue(&choice_cue("0", ms(0), "OnSmall", "ちいさい", &[]));
    rt.apply_cue(&cue("0", ms(0), CueCommand::NewLine { ratio: 1.0 }));
    rt.apply_cue(&font("0", &["height", "40"]));
    rt.apply_cue(&choice_cue("0", ms(200), "OnBig", "おおきい", &[]));
    let image = (240u32, 200u32);
    rt.register_actor(
        actor.clone(),
        TextSlotBinding::new(slot, window, 1.0, image, image),
        ResolvedBalloonText::resolve(&geo_model(), image),
    );

    present_frame(&mut rt, &mut world, ms(10000)).expect("提示フレーム");

    let rows: Vec<super::ChoiceHitRow> = rt.choice_hit_rows(&actor).to_vec();
    assert_eq!(rows.len(), 2, "選択肢 2 つぶんのヒット行");
    let small = rows[0].rect.bottom - rows[0].rect.top;
    let big = rows[1].rect.bottom - rows[1].rect.top;
    assert!(
        big > small * 2.0,
        "\\f[height,40] の選択肢の当たり帯が既定の大きさのまま——\
         帯を行ごとに引き直していない（既定 {small} / 装飾 {big}）"
    );
}

// 描画＋字下げの読み戻し（COM・headless・draw==hit）
#[test]
fn choice_rows_render_at_indented_positions_readback_pixel_cage() {
    const FONT_H: f32 = 12.0;
    // 行送りは正典の式をそのまま呼ぶ（文字の大きさ + 行間 正典表）。
    // 式をここに書き写さないので、行間の既定値が変わっても本テストは自動で追随する。
    let pitch: f32 = TextLayerConfig::default().line_pitch(FONT_H); // 12 + 2 = 14.0
    let indent_x: f32 = 5.0 * FONT_H; // 5em = 60.0
    let indent_y: f32 = 2.0 * pitch; // 2lh = 28.0
    let (mut world, window, slot) = com_world();
    let actor = ActorKey::from("0");
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    rt.apply_cue(&cue(
        "0",
        ms(0),
        CueCommand::Cursor {
            x: "5em".into(),
            y: "2lh".into(),
        },
    ));
    rt.apply_cue(&choice_cue("0", ms(0), "OnYes", "はい", &["r0"]));
    rt.apply_cue(&cue("0", ms(100), CueCommand::NewLine { ratio: 1.0 }));
    rt.apply_cue(&choice_cue("0", ms(100), "OnNo", "いいえ", &["r1"]));
    rt.apply_cue(&cue("0", ms(200), CueCommand::NewLine { ratio: 1.0 }));
    rt.apply_cue(&choice_cue("0", ms(200), "OnMaybe", "どちらでも", &["r2"]));
    let image = (200u32, 100u32);
    rt.register_actor(
        actor.clone(),
        TextSlotBinding::new(slot, window, 1.0, image, image),
        ResolvedBalloonText::resolve(&geo_model(), image),
    );
    present_frame(&mut rt, &mut world, ms(10000)).expect("提示フレーム");
    let rows: Vec<super::ChoiceHitRow> = rt.choice_hit_rows(&actor).to_vec();
    assert_eq!(rows.len(), 3);
    assert_eq!(
        (rows[0].ordinal, rows[1].ordinal, rows[2].ordinal),
        (0, 1, 2)
    );
    assert_eq!(rows[0].id, "OnYes");
    assert_eq!(rows[1].id, "OnNo");
    assert_eq!(rows[2].id, "OnMaybe");
    assert_eq!(rows[0].rect.left, indent_x, "先頭選択肢 5em 字下げ");
    assert_eq!(rows[0].rect.top, indent_y, "先頭選択肢 2lh 字下げ");
    assert_eq!(rows[1].rect.left, 0.0);
    assert!(rows[0].rect.top < rows[1].rect.top && rows[1].rect.top < rows[2].rect.top);
    let width = image.0;
    let bytes = read_back(&rt, &actor);
    let band = |r: &super::ChoiceHitRow| {
        let (_, y0, _, y1) = row_px(r);
        (y0, y1)
    };
    let (r0y0, r0y1) = band(&rows[0]);
    assert!(
        count_px(&bytes, width, (indent_x as u32, r0y0, width, r0y1), inked) > 0,
        "字下げ位置にインク（draw==hit）"
    );
    assert_eq!(
        count_px(
            &bytes,
            width,
            (0, r0y0, (indent_x as u32).saturating_sub(10), r0y1),
            inked
        ),
        0,
        "字下げ前は空白"
    );
    present_frame(&mut rt, &mut world, ms(10000)).expect("NoChange 再提示");
    let bytes2 = read_back(&rt, &actor);
    assert_eq!(bytes, bytes2, "決定論");
}

/// 選択肢スパンを持たない actor は提示後もヒット行が空。
#[test]
fn present_with_no_choices_yields_empty_choice_hit_rows() {
    let (mut world, window, slot) = com_world();
    let actor = ActorKey::from("0");
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    rt.apply_cue(&cue("0", ms(0), CueCommand::Text("ただの本文".into())));
    let image = (120u32, 60u32);
    rt.register_actor(
        actor.clone(),
        TextSlotBinding::new(slot, window, 1.0, image, image),
        ResolvedBalloonText::resolve(&geo_model(), image),
    );
    present_frame(&mut rt, &mut world, ms(10000)).expect("提示フレーム");
    assert!(
        rt.choice_hit_rows(&actor).is_empty(),
        "選択肢スパンの無い actor は提示後もヒット行が空（decorate/derive とも恒等）"
    );
}

// hover の画素とダーティ限定（COM・headless）
//
// cursor.* 由来の SquareFill（塗り=(105,25,25)・文字=(255,255,255)）のバルーンで hover on/off を
// 注入し、(a) 塗り色画素と白文字画素が ordinal-0 のセグメント矩形に現れ、解除で塗り色が消えること、
// (b) どのトグルフレームも当該 Choice 行だけを再描画すること（全域再描画ではない）を固定する。
// draw_text_layout_calls は「ダーティ矩形数 × 交差住人数」で数えるので、全域へ縮退すれば増分は
// 全住人数になる。トグルフレームの増分 1 が全域再描画でない証拠になる。
#[test]
fn hover_toggle_paints_fill_and_stays_dirty_limited_readback_pixel_cage() {
    let (mut world, window, slot) = com_world();
    let actor = ActorKey::from("0");
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    // 3 選択肢を別行へ置く（全域再描画なら増分 3・ダーティ限定なら 1）。
    rt.apply_cue(&choice_cue("0", ms(0), "OnYes", "はい", &["r0"]));
    rt.apply_cue(&cue("0", ms(100), CueCommand::NewLine { ratio: 1.0 }));
    rt.apply_cue(&choice_cue("0", ms(100), "OnNo", "いいえ", &["r1"]));
    rt.apply_cue(&cue("0", ms(200), CueCommand::NewLine { ratio: 1.0 }));
    rt.apply_cue(&choice_cue("0", ms(200), "OnMaybe", "どちらでも", &["r2"]));
    let image = (200u32, 100u32);
    rt.register_actor(
        actor.clone(),
        TextSlotBinding::new(slot, window, 1.0, image, image),
        // cursor.* 由来の SquareFill スタイルを運ぶバルーン（未指定 geo_model は Invert）。
        ResolvedBalloonText::resolve(&cursor_model(), image),
    );
    let width = image.0;

    // premultiplied BGRA・α=255 なので B=25,G=25,R=105,A=255 が塗り色の厳密表現。
    let fill = |px: &[u8]| px[0] == 25 && px[1] == 25 && px[2] == 105 && px[3] == 255;
    // 白文字は全チャネル閾値でアンチエイリアスの端を除いた芯を数える。
    let white = |px: &[u8]| px[0] >= 200 && px[1] >= 200 && px[2] >= 200 && px[3] == 255;
    let fill_in_rect = |b: &[u8], r: &super::ChoiceHitRow| count_px(b, width, row_px(r), fill);
    let white_in_rect = |b: &[u8], r: &super::ChoiceHitRow| count_px(b, width, row_px(r), white);
    let fill_total = |b: &[u8]| count_px(b, width, (0, 0, u32::MAX, u32::MAX), fill);
    let calls = |rt: &TextLayerRuntime| -> u64 {
        rt.draw_stats(&actor)
            .expect("draw_stats")
            .draw_text_layout_calls
    };

    present_frame(&mut rt, &mut world, ms(10000)).expect("ベースライン提示");
    let rows: Vec<super::ChoiceHitRow> = rt.choice_hit_rows(&actor).to_vec();
    assert_eq!(rows.len(), 3, "3 選択肢＝3 行（ダーティ限定を測る台）");
    let base_bytes = read_back(&rt, &actor);
    assert_eq!(
        fill_total(&base_bytes),
        0,
        "hover 無しでは塗り色（105,25,25）画素は 1 つも無い"
    );
    let base_calls = calls(&rt);

    rt.inject_choice_hover(&actor, Some(0));
    present_frame(&mut rt, &mut world, ms(10000)).expect("hover on 提示");
    let hover_calls = calls(&rt);
    let hover_delta = hover_calls - base_calls;
    assert_eq!(
        hover_delta, 1,
        "hover on はダーティ限定＝当該行 1 枚のみ再描画（増分 1）: {hover_delta}"
    );
    let hover_bytes = read_back(&rt, &actor);
    assert!(
        fill_in_rect(&hover_bytes, &rows[0]) > 0,
        "hover 行に塗り色（105,25,25）画素が載る: {:?}",
        rows[0].rect
    );
    assert!(
        white_in_rect(&hover_bytes, &rows[0]) > 0,
        "hover 行に白文字（255,255,255）画素が載る: {:?}",
        rows[0].rect
    );
    // 塗りは hover 行だけ。
    assert_eq!(
        fill_in_rect(&hover_bytes, &rows[1]),
        0,
        "非 hover 行 1 には塗り色画素が載らない"
    );

    // 決定論（NoChange 再提示）: 同一状態の再 present は再描画せずバイト同一。
    present_frame(&mut rt, &mut world, ms(10000)).expect("hover NoChange 再提示");
    assert_eq!(
        calls(&rt),
        hover_calls,
        "hover 状態不変の再提示は再描画しない（DrawTextLayout 不変）"
    );
    let hover_bytes2 = read_back(&rt, &actor);
    assert_eq!(
        hover_bytes, hover_bytes2,
        "決定論（hover 状態・バイト同一）"
    );
    let steady_calls = calls(&rt);

    rt.inject_choice_hover(&actor, None);
    present_frame(&mut rt, &mut world, ms(10000)).expect("hover off 提示");
    let off_calls = calls(&rt);
    let off_delta = off_calls - steady_calls;
    assert_eq!(
        off_delta, 1,
        "hover off もダーティ限定＝当該行 1 枚のみ再描画（増分 1）: {off_delta}"
    );
    let off_bytes = read_back(&rt, &actor);
    // 塗り色画素が面全域から消滅する（素描画へ戻る）。
    assert_eq!(
        fill_total(&off_bytes),
        0,
        "hover off で塗り色（105,25,25）画素が消滅する"
    );

    // 決定論（hover off 状態の NoChange 再提示）: バイト同一。
    present_frame(&mut rt, &mut world, ms(10000)).expect("off NoChange 再提示");
    let off_bytes2 = read_back(&rt, &actor);
    assert_eq!(
        off_bytes, off_bytes2,
        "決定論（hover off 状態・バイト同一）"
    );
}

// 矩形反転縮退の画素（COM・headless）。
//
// `hover_toggle_paints_fill_and_stays_dirty_limited_readback_pixel_cage` を cursor.\* 未指定の
// バルーン（`geo_model`＝Invert 縮退）で行う。矩形反転縮退は「セグメント矩形＝バルーン既定
// font.color 塗り・文字色＝各成分 255−c」で、hover 行のセグメント矩形が既定文字色（黒）で全域不透明に
// なり、その上に反転文字色（白）のグリフが載る。hover 解除で塗りが消えて素描画へ戻る。
// 期待色は黒と決め打ちせず、モデルの既定 font 色を読んで 255−c で算出する。
#[test]
fn invert_hover_paints_default_font_color_fill_and_inverted_text_readback_pixel_cage() {
    let (mut world, window, slot) = com_world();
    let actor = ActorKey::from("0");
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    rt.apply_cue(&choice_cue("0", ms(0), "OnYes", "はい", &["r0"]));
    rt.apply_cue(&cue("0", ms(100), CueCommand::NewLine { ratio: 1.0 }));
    rt.apply_cue(&choice_cue("0", ms(100), "OnNo", "いいえ", &["r1"]));
    rt.apply_cue(&cue("0", ms(200), CueCommand::NewLine { ratio: 1.0 }));
    rt.apply_cue(&choice_cue("0", ms(200), "OnMaybe", "どちらでも", &["r2"]));
    let image = (200u32, 100u32);
    let resolved = ResolvedBalloonText::resolve(&geo_model(), image);
    assert_eq!(
        resolved.choice_style,
        ResolvedChoiceStyle::Invert,
        "cursor.* 未指定バルーンは choice_style=Invert（矩形反転縮退）"
    );
    // バルーン既定文字色を読む。geo_model は既定黒 (0,0,0)。
    let (fr, fg, fb) = resolved.font.color;
    assert_eq!(
        (fr, fg, fb),
        (0, 0, 0),
        "geo_model 既定文字色は黒（読取値・既定が変われば期待塗り/文字色も再計算せよ）"
    );
    // Invert::paint の正規形（塗り＝既定 font 色・文字＝各成分 255−c）を独立に算出して照合する。
    let (tr, tg, tb) = (255 - fr, 255 - fg, 255 - fb); // 反転文字色＝白 (255,255,255)
    assert_eq!(
        ResolvedChoiceStyle::Invert.paint((fr, fg, fb)),
        Some(((fr, fg, fb), (tr, tg, tb))),
        "Invert::paint＝(既定 font 色, 255−c)"
    );
    rt.register_actor(
        actor.clone(),
        TextSlotBinding::new(slot, window, 1.0, image, image),
        resolved,
    );
    let width = image.0;
    let height = image.1;

    // premultiplied BGRA・α=255。倍率 1.0 なので窓の物理 px＝image px。
    // 矩形の内側（アンチエイリアスの端を避けて 1px 内側・面の内側へ切り詰め）で数え、縁取りに惑わされない。
    let interior = |r: &super::ChoiceHitRow| -> (u32, u32, u32, u32) {
        let (x0, y0, x1, y1) = row_px(r);
        (
            x0 + 1,
            y0 + 1,
            x1.min(width).saturating_sub(1),
            y1.min(height).saturating_sub(1),
        )
    };
    let area = |(x0, y0, x1, y1): (u32, u32, u32, u32)| -> usize {
        (x1.saturating_sub(x0) as usize) * (y1.saturating_sub(y0) as usize)
    };
    // 不透明画素数。塗りが矩形を全域不透明にしたかの判定に使う。
    let opaque_in = |b: &[u8], rect| count_px(b, width, rect, |px| px[3] == 255);
    // 既定文字色（塗り色）の厳密一致画素数。BGRA なので B=fb,G=fg,R=fr,A=255。
    let fill_in = |b: &[u8], rect| {
        count_px(b, width, rect, |px| {
            px[0] == fb && px[1] == fg && px[2] == fr && px[3] == 255
        })
    };
    // 反転文字色（白）近傍の画素数。各チャネル ±55（255→≥200）で数え、上の白閾値と同じ。
    let text_in = |b: &[u8], rect| {
        count_px(b, width, rect, |px| {
            px[0].abs_diff(tb) <= 55
                && px[1].abs_diff(tg) <= 55
                && px[2].abs_diff(tr) <= 55
                && px[3] == 255
        })
    };
    let calls = |rt: &TextLayerRuntime| -> u64 {
        rt.draw_stats(&actor)
            .expect("draw_stats")
            .draw_text_layout_calls
    };

    present_frame(&mut rt, &mut world, ms(10000)).expect("ベースライン提示");
    let rows: Vec<super::ChoiceHitRow> = rt.choice_hit_rows(&actor).to_vec();
    assert_eq!(rows.len(), 3, "3 選択肢＝3 行（hover 行限定を測る台）");
    let base = read_back(&rt, &actor);
    let r0_int = interior(&rows[0]);
    let r0_area = area(r0_int);
    assert!(r0_area > 0, "行 0 の interior 領域は非空");
    assert_eq!(
        text_in(&base, r0_int),
        0,
        "hover 無しでは反転文字色（{tr},{tg},{tb}）画素は無い"
    );
    assert!(
        opaque_in(&base, r0_int) < r0_area,
        "hover 無しの矩形 interior は塗り未充填（透明ギャップ有り）: {}/{r0_area}",
        opaque_in(&base, r0_int)
    );
    let base_fill = fill_in(&base, r0_int); // 素描画のグリフ画素数＝背景塗り増分の基準。
    let base_calls = calls(&rt);

    rt.inject_choice_hover(&actor, Some(0));
    present_frame(&mut rt, &mut world, ms(10000)).expect("hover on 提示");
    let hover_calls = calls(&rt);
    let hover_delta = hover_calls - base_calls;
    assert_eq!(
        hover_delta, 1,
        "hover on はダーティ限定＝当該行 1 枚のみ再描画（増分 1）: {hover_delta}"
    );
    let hov = read_back(&rt, &actor);
    assert_eq!(
        rt.choice_hit_rows(&actor).len(),
        3,
        "hover 後もヒット行は 3"
    );
    // 背景塗り: hover 行の矩形は既定文字色で全域不透明になる。
    assert_eq!(
        opaque_in(&hov, r0_int),
        r0_area,
        "hover 行の矩形 interior は既定文字色塗りで全画素不透明（反転縮退の背景塗り）"
    );
    // 塗り色は既定 font 色で、素描画（グリフのみ）より画素が増える。
    let hover_fill = fill_in(&hov, r0_int);
    assert!(
        hover_fill > base_fill,
        "hover で既定文字色（{fr},{fg},{fb}）の背景塗り画素が増える: base={base_fill} hover={hover_fill}"
    );
    // 反転文字色のグリフが矩形に載る。
    let hover_text = text_in(&hov, r0_int);
    assert!(
        hover_text > 0,
        "hover 行に反転文字色（{tr},{tg},{tb}）画素が載る"
    );
    // 背景塗りが反転文字より支配的＝「矩形塗り＋反転文字」の対になっている。
    assert!(
        hover_fill > hover_text,
        "背景塗り画素（{hover_fill}）は反転文字画素（{hover_text}）より支配的（矩形＝背景・文字＝ストローク）"
    );
    // 塗りは hover 行だけ。
    let r1_int = interior(&rows[1]);
    assert!(
        opaque_in(&hov, r1_int) < area(r1_int),
        "非 hover 行 1 の矩形は塗り未充填（塗りは hover 行に限定）"
    );
    assert_eq!(
        text_in(&hov, r1_int),
        0,
        "非 hover 行 1 に反転文字色（白）画素は無い"
    );

    // 決定論（NoChange 再提示）: 同一状態の再 present は再描画せずバイト同一。
    present_frame(&mut rt, &mut world, ms(10000)).expect("hover NoChange 再提示");
    assert_eq!(
        calls(&rt),
        hover_calls,
        "hover 状態不変の再提示は再描画しない（DrawTextLayout 不変）"
    );
    let hov2 = read_back(&rt, &actor);
    assert_eq!(hov, hov2, "決定論（hover 状態・バイト同一）");
    let steady_calls = calls(&rt);

    rt.inject_choice_hover(&actor, None);
    present_frame(&mut rt, &mut world, ms(10000)).expect("hover off 提示");
    let off_delta = calls(&rt) - steady_calls;
    assert_eq!(
        off_delta, 1,
        "hover off もダーティ限定＝当該行 1 枚のみ再描画（増分 1）: {off_delta}"
    );
    let off = read_back(&rt, &actor);
    // 反転縮退の塗りが消え、素描画へ戻る。
    assert!(
        opaque_in(&off, r0_int) < r0_area,
        "hover off で矩形 interior は未充填へ戻る（既定文字色塗りが消滅）: {}/{r0_area}",
        opaque_in(&off, r0_int)
    );
    assert_eq!(
        text_in(&off, r0_int),
        0,
        "hover off で反転文字色（{tr},{tg},{tb}）画素が消滅する"
    );

    // 決定論（hover off 状態の NoChange 再提示）: バイト同一。
    present_frame(&mut rt, &mut world, ms(10000)).expect("off NoChange 再提示");
    let off2 = read_back(&rt, &actor);
    assert_eq!(off, off2, "決定論（hover off 状態・バイト同一）");
}

// ── 帯の寄せ: 実フォントの hover 移動・解除で塗りが消し残らない読み戻し ──

/// 実フォント `Yu Gothic UI`・正典 `font.height,28` ＋ `cursor.*` 指定のバルーン定義。
///
/// 行ボックス丈が行送り（30）より高いので帯の寄せが 0 でない値になる——既定
/// ＭＳ ゴシック（比 1.0）の檻では寄せが 0 のままで、この経路は 1 度も踏まれない
/// （既定フォント盲点）。
fn real_font_cursor_model() -> areka_parsers::balloon::BalloonModel {
    use areka_parsers::balloon::{BalloonCursor, CursorColor, Font, FontColor, Origin, ValidRect};
    super::test_support::balloon_model_of(
        Origin::new(Some(0), Some(0)),
        ValidRect::new(None, None, None, None),
        Font::new(
            Some("Yu Gothic UI".to_owned()),
            Some(28),
            FontColor::new(Some(0), Some(0), Some(0)),
        ),
        None,
        None,
    )
    .with_cursor(BalloonCursor::new(
        Some("square".to_string()),
        CursorColor::new(Some(105), Some(25), Some(25)),
        CursorColor::new(None, None, None),
        CursorColor::new(Some(255), Some(255), Some(255)),
        None,
    ))
}

/// 実フォントで帯が行ボックスの中央へ寄った状態で hover を
/// **移動**させ、さらに**解除**しても、前の行の塗りが 1 画素も残らない。
///
/// ダーティ矩形は行の em ボックス（`run_size` の行送りの軸＝行の丈）＋実測インクはみ出しで組まれるので、
/// 帯の遠端側の超過分を `band_offset + band_extent − 行の丈` で数えないと、寄せた分だけ帯の下が
/// クリップの外へ落ちて塗りが残る（`expand_overhang_for_band`）。寄せを数えないままでは
/// hover を移した後の面に前の行の帯の下端が残り、この檻が赤くなる。
///
/// 併せて、ヒット矩形の上端が**行矩形の上端 ＋ 寄せ**であることと、寄せが描画とヒットへ
/// 同一に配られること（読み戻した塗りの上端がヒット矩形の上端に一致）も固定する。
#[test]
fn hover_move_and_release_leave_no_residue_with_band_offset_readback() {
    use windows::Win32::Graphics::DirectWrite::DWRITE_FACTORY_TYPE_SHARED;
    use wintf::com::dwrite::dwrite_create_factory;

    use crate::choice::{highlight_band_extent, highlight_band_offset};
    use crate::draw::{DWriteMetrics, ResolvedFont};
    use crate::layout::GlyphMetrics;
    use crate::writing::WritingMode;

    const FONT_H: f32 = 28.0;
    let model = real_font_cursor_model();

    // 実フォントが在ることを先に確かめる（代替フォントへ縮退すると寄せが 0 になり、
    // 「寄せが効いていない」のか「フォントが無い」のかが区別できないまま赤くなる）。
    let factory = dwrite_create_factory(DWRITE_FACTORY_TYPE_SHARED).expect("DWrite factory");
    let metrics = DWriteMetrics::new(
        &factory,
        &ResolvedFont::resolve(&model),
        WritingMode::HorizontalTb,
        &TextLayerConfig::default(),
    )
    .expect("DWriteMetrics 生成");
    let advance = metrics.advance("あ", FONT_H);
    assert!(
        advance < FONT_H,
        "実フォント Yu Gothic UI が見つからない（「あ」の送りが {advance} ＝ em {FONT_H} 以上の等幅値へ縮退している）。本檻は実フォントの行ボックス丈を前提にしているので、代替フォントのまま緑にしない"
    );
    let pitch = metrics.line_pitch(FONT_H);
    let band = highlight_band_extent(FONT_H, metrics.line_box_height(FONT_H), pitch);
    let offset = highlight_band_offset(metrics.line_box_height(FONT_H), band);
    assert!(
        offset > 0.0,
        "実フォントの行ボックス丈 {} は帯の丈 {band} より高く、寄せは正になる（実測 {offset}）",
        metrics.line_box_height(FONT_H)
    );

    let (mut world, window, slot) = com_world();
    let actor = ActorKey::from("0");
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    rt.apply_cue(&choice_cue("0", ms(0), "OnYes", "はい", &["r0"]));
    rt.apply_cue(&cue("0", ms(100), CueCommand::NewLine { ratio: 1.0 }));
    rt.apply_cue(&choice_cue("0", ms(100), "OnNo", "いいえ", &["r1"]));
    let image = (240u32, 140u32);
    rt.register_actor(
        actor.clone(),
        TextSlotBinding::new(slot, window, 1.0, image, image),
        ResolvedBalloonText::resolve(&model, image),
    );
    let (width, height) = image;

    // 塗り色は premultiplied BGRA の (25,25,105,255)。
    let fill = |px: &[u8]| px[0] == 25 && px[1] == 25 && px[2] == 105 && px[3] == 255;
    // 面全域の塗り色画素数。
    let fill_total = |b: &[u8]| count_px(b, width, (0, 0, u32::MAX, u32::MAX), fill);
    // ヒット矩形の内側の塗り色画素数。
    let fill_in_row = |b: &[u8], r: &super::ChoiceHitRow| count_px(b, width, row_px(r), fill);
    // 面全域で塗り色画素を持つ行の範囲（上端, 下端）——読み戻した帯の縦位置。
    let fill_rows = |b: &[u8]| -> Option<(u32, u32)> {
        let mut span: Option<(u32, u32)> = None;
        for y in 0..height {
            if count_px(b, width, (0, y, width, y + 1), fill) > 0 {
                span = Some(match span {
                    None => (y, y),
                    Some((t, _)) => (t, y),
                });
            }
        }
        span
    };
    let read = |rt: &TextLayerRuntime| -> Vec<u8> { read_back(rt, &actor) };

    present_frame(&mut rt, &mut world, ms(10000)).expect("ベースライン提示");
    let rows: Vec<super::ChoiceHitRow> = rt.choice_hit_rows(&actor).to_vec();
    assert_eq!(rows.len(), 2, "2 選択肢＝2 ヒット行");
    assert_eq!(
        fill_total(&read(&rt)),
        0,
        "hover 無しでは塗りが 1 画素も無い"
    );
    // ヒット矩形の上端は行矩形の上端（0 / 行送り 1 つぶん）＋寄せ。
    assert_eq!(
        (rows[0].rect.top, rows[1].rect.top),
        (offset, pitch + offset),
        "ヒット矩形の上端は行矩形の上端 ＋ 寄せ {offset}（寄せを配らないと 0 と {pitch} で赤）"
    );
    assert_eq!(
        rows[0].rect.bottom - rows[0].rect.top,
        band,
        "寄せても帯の丈は変わらない"
    );

    rt.inject_choice_hover(&actor, Some(0));
    present_frame(&mut rt, &mut world, ms(10000)).expect("hover 0 提示");
    let first = read(&rt);
    assert!(
        fill_in_row(&first, &rows[0]) > 0,
        "ordinal 0 の帯が塗られる"
    );
    // 読み戻した塗りの縦位置がヒット矩形の縦位置と一致する（描画とヒットへ同一の寄せ）。
    let painted = fill_rows(&first).expect("塗りが在る以上、その縦範囲も在る");
    assert_eq!(
        (painted.0, painted.1 + 1),
        (rows[0].rect.top as u32, rows[0].rect.bottom as u32),
        "読み戻した塗り y{}..{} はヒット矩形 {:?} と同じ位置（描画とヒットの単一導出）",
        painted.0,
        painted.1,
        rows[0].rect
    );

    rt.inject_choice_hover(&actor, Some(1));
    present_frame(&mut rt, &mut world, ms(10000)).expect("hover 1 提示");
    let moved = read(&rt);
    assert!(
        fill_in_row(&moved, &rows[1]) > 0,
        "ordinal 1 の帯が塗られる"
    );
    let residue = fill_in_row(&moved, &rows[0]);
    assert_eq!(
        residue, 0,
        "hover を移した後、前の行（ordinal 0・{:?}）に塗りが {residue} 画素消し残った——ダーティ帯の超過分が寄せ {offset} を数えていない（`areka-P0-emo2-conformance-e2e` R13.5）",
        rows[0].rect
    );

    rt.inject_choice_hover(&actor, None);
    present_frame(&mut rt, &mut world, ms(10000)).expect("hover 解除提示");
    let off = read(&rt);
    let left_over = fill_total(&off);
    assert_eq!(
        left_over, 0,
        "hover 解除の後、面全域に塗りが {left_over} 画素消し残った（`areka-P0-emo2-conformance-e2e` R13.5）"
    );
    assert!(opaque_count(&off) > 0, "解除後も素のグリフインクは描かれる");
}
