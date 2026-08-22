use areka_sakura::contract::{ActorKey, CueCommand};

use super::test_support::{choice_cue, com_world, cue, cursor_model, geo_model, opaque_count};
use super::{ResolvedBalloonText, TextLayerRuntime, TextSlotBinding, present_frame};
use crate::choice::ResolvedChoiceStyle;
use crate::state::TextLayerConfig;

/// Observable（3.5/4.1）: 現存しない ordinal で `inject_choice_hover` を呼んでもパニックせず、
/// `choice_active` は選択肢スパンの実状態を反映し続ける（選択肢あり＝true）。
#[test]
fn inject_choice_hover_nonexistent_ordinal_does_not_panic_and_choice_active_holds() {
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    let actor = ActorKey::from("0");
    rt.apply_cue(&choice_cue("0", 0.0, "OnYes", "はい", &["r0"]));
    assert!(
        rt.choice_active(&actor),
        "選択肢スパンありで choice_active=true"
    );

    // 現存スパン（ordinal 0）に無い ordinal 999 を注入——panic せず縮退（debug ログ）。
    rt.inject_choice_hover(&actor, Some(999));
    // 実存 ordinal 0 の注入・解除も panic しない。
    rt.inject_choice_hover(&actor, Some(0));
    rt.inject_choice_hover(&actor, None);

    // stale ordinal 注入後も choice_active はスパンの実状態を反映し続ける（hover は照会に影響しない）。
    assert!(
        rt.choice_active(&actor),
        "stale ordinal 注入後も choice_active はスパン実状態を反映"
    );
}

/// `choice_active`（1.3）: スパン非空で true・スパン無し／未知 actor で false。
#[test]
fn choice_active_reflects_span_set_presence() {
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    let choosing = ActorKey::from("0");
    let plain = ActorKey::from("1");
    let unknown = ActorKey::from("9");

    rt.apply_cue(&choice_cue("0", 0.0, "q", "はい", &[]));
    rt.apply_cue(&cue("1", 0.0, CueCommand::Text("ただの本文".into())));

    assert!(rt.choice_active(&choosing), "Choice cue 消費 actor は true");
    assert!(
        !rt.choice_active(&plain),
        "Text のみ（選択肢スパン無し）actor は false"
    );
    assert!(!rt.choice_active(&unknown), "未知 actor は false");
}

/// `choice_hit_rows`（3.2）: 未知 actor・選択肢無し actor は空 slice を返す
/// （スナップショット population は task 8.2——8.1 では常に空）。
#[test]
fn choice_hit_rows_empty_for_unknown_or_no_snapshot_actor() {
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    let unknown = ActorKey::from("9");
    assert!(
        rt.choice_hit_rows(&unknown).is_empty(),
        "未知 actor は空 slice"
    );

    // 選択肢スパンを持つ actor でも 8.1 時点ではスナップショット未 population＝空 slice。
    let actor = ActorKey::from("0");
    rt.apply_cue(&choice_cue("0", 0.0, "q", "はい", &[]));
    assert!(
        rt.choice_hit_rows(&actor).is_empty(),
        "スナップショット未 population（8.2 前）は空 slice"
    );
}

/// `ResolvedBalloonText` は balloon cursor モデルから解決した `choice_style` を運ぶ（Integration）。
/// cursor.\* 未指定の geo_model → `Invert`（未指定バルーン＝M1 実導出）。
#[test]
fn resolved_balloon_text_carries_choice_style_from_cursor_model() {
    let image = (120u32, 60u32);
    let resolved = ResolvedBalloonText::resolve(&geo_model(), image);
    assert_eq!(
        resolved.choice_style,
        ResolvedChoiceStyle::Invert,
        "cursor.* 未指定バルーンは choice_style=Invert（既定文字色反転縮退）"
    );
}

/// Observable（3.1/3.2/3.3/5.2）: 提示成功直後の `choice_hit_rows` が、描画された選択肢行と整合する
/// 非退化矩形を選択肢数ぶん返し（配送順 ordinal・下流構成材料を忠実同梱）、後続 `NoChange` フレームでは
/// 直前スナップショットが不変のまま保たれる（再描画も更新も起きない）。
#[test]
fn present_populates_choice_hit_rows_and_nochange_preserves_snapshot() {
    let (mut world, window, slot) = com_world();
    let actor = ActorKey::from("0");
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    // 2 選択肢を配送順に注入（ordinal 0="はい"・1="いいえ"）。
    rt.apply_cue(&choice_cue("0", 0.0, "OnYes", "はい", &["r0"]));
    rt.apply_cue(&choice_cue("0", 0.2, "OnNo", "いいえ", &["r1"]));
    let image = (120u32, 60u32);
    rt.register_actor(
        actor.clone(),
        TextSlotBinding::new(slot, window, 1.0, image, image),
        ResolvedBalloonText::resolve(&geo_model(), image),
    );

    // 全リビール済み時刻で提示＝present 成功時のスナップショット population。
    present_frame(&mut rt, &mut world, 10.0).expect("提示フレーム");
    let rows: Vec<super::ChoiceHitRow> = rt.choice_hit_rows(&actor).to_vec();
    assert_eq!(rows.len(), 2, "選択肢数と同数のヒット行が並ぶ");
    // 配送順 ordinal＋下流構成材料（id/label/references）を忠実転写。
    assert_eq!(rows[0].ordinal, 0);
    assert_eq!(rows[0].id, "OnYes");
    assert_eq!(rows[0].label, "はい");
    assert_eq!(rows[0].references, vec!["r0".to_string()]);
    assert_eq!(rows[1].ordinal, 1);
    assert_eq!(rows[1].id, "OnNo");
    assert_eq!(rows[1].label, "いいえ");
    assert_eq!(rows[1].references, vec!["r1".to_string()]);
    // 矩形は非退化（描かれた選択肢行の文字幅×行高）。
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
    present_frame(&mut rt, &mut world, 10.0).expect("NoChange 再提示");
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

// task 9.1: 描画＋字下げの readback 統合檻（COM・headless・R7.1/7.4・draw==hit の R3.3）
#[test]
fn choice_rows_render_at_indented_positions_readback_pixel_cage() {
    const FONT_H: f32 = 12.0;
    let pitch: f32 = (FONT_H * 1.25).ceil(); // 15.0
    let indent_x: f32 = 5.0 * FONT_H; // 5em = 60.0
    let indent_y: f32 = 2.0 * pitch; // 2lh = 30.0
    let (mut world, window, slot) = com_world();
    let actor = ActorKey::from("0");
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    rt.apply_cue(&cue(
        "0",
        0.0,
        CueCommand::Cursor {
            x: "5em".into(),
            y: "2lh".into(),
        },
    ));
    rt.apply_cue(&choice_cue("0", 0.0, "OnYes", "はい", &["r0"]));
    rt.apply_cue(&cue("0", 0.1, CueCommand::NewLine { ratio: 1.0 }));
    rt.apply_cue(&choice_cue("0", 0.1, "OnNo", "いいえ", &["r1"]));
    rt.apply_cue(&cue("0", 0.2, CueCommand::NewLine { ratio: 1.0 }));
    rt.apply_cue(&choice_cue("0", 0.2, "OnMaybe", "どちらでも", &["r2"]));
    let image = (200u32, 100u32);
    rt.register_actor(
        actor.clone(),
        TextSlotBinding::new(slot, window, 1.0, image, image),
        ResolvedBalloonText::resolve(&geo_model(), image),
    );
    present_frame(&mut rt, &mut world, 10.0).expect("提示フレーム");
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
    let surface = rt.surface(&actor).expect("供給面");
    let width = image.0;
    let bytes = surface.read_back().expect("read_back");
    let ink_in_rect = |b: &[u8], x0: u32, y0: u32, x1: u32, y1: u32| -> usize {
        let mut n = 0usize;
        for y in y0..y1 {
            for x in x0..x1 {
                if b[((y * width + x) * 4) as usize + 3] != 0 {
                    n += 1;
                }
            }
        }
        n
    };
    let band = |r: &super::ChoiceHitRow| (r.rect.top as u32, r.rect.bottom.ceil() as u32);
    let (r0y0, r0y1) = band(&rows[0]);
    assert!(
        ink_in_rect(&bytes, indent_x as u32, r0y0, width, r0y1) > 0,
        "字下げ位置にインク（draw==hit）"
    );
    assert_eq!(
        ink_in_rect(&bytes, 0, r0y0, (indent_x as u32).saturating_sub(10), r0y1),
        0,
        "字下げ前は空白"
    );
    assert!(opaque_count(&bytes) > 0);
    present_frame(&mut rt, &mut world, 10.0).expect("NoChange 再提示");
    let bytes2 = rt
        .surface(&actor)
        .expect("供給面")
        .read_back()
        .expect("read_back");
    assert_eq!(bytes, bytes2, "決定論");
}

/// Observable（6.3・非退行）: 選択肢スパンを持たない actor は提示後もヒット行が空
/// （annotate/derive とも恒等・新規スクロール判定なし）。
#[test]
fn present_with_no_choices_yields_empty_choice_hit_rows() {
    let (mut world, window, slot) = com_world();
    let actor = ActorKey::from("0");
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    rt.apply_cue(&cue("0", 0.0, CueCommand::Text("ただの本文".into())));
    let image = (120u32, 60u32);
    rt.register_actor(
        actor.clone(),
        TextSlotBinding::new(slot, window, 1.0, image, image),
        ResolvedBalloonText::resolve(&geo_model(), image),
    );
    present_frame(&mut rt, &mut world, 10.0).expect("提示フレーム");
    assert!(
        rt.choice_hit_rows(&actor).is_empty(),
        "選択肢スパンの無い actor は提示後もヒット行が空（decorate/derive とも恒等）"
    );
}

/// Observable（3.3/4.x 配線・単一導出）: hover 注入が decorate→render まで配線され、ハイライト塗りで
/// 非透明ピクセルが増える（Invert 縮退＝塗り＝既定黒文字色の矩形が hover 行へ載る）。ヒット行照会自体は
/// hover 非依存（count 不変）。
#[test]
fn present_with_hover_adds_highlight_pixels_over_non_hover_frame() {
    let (mut world, window, slot) = com_world();
    let actor = ActorKey::from("0");
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    rt.apply_cue(&choice_cue("0", 0.0, "OnYes", "はい", &["r0"]));
    rt.apply_cue(&choice_cue("0", 0.2, "OnNo", "いいえ", &["r1"]));
    let image = (120u32, 60u32);
    rt.register_actor(
        actor.clone(),
        TextSlotBinding::new(slot, window, 1.0, image, image),
        ResolvedBalloonText::resolve(&geo_model(), image),
    );

    // hover 無しで提示（素の選択肢テキスト）＝スナップショット population・hover 未注入。
    present_frame(&mut rt, &mut world, 10.0).expect("hover 無し提示");
    let read = |rt: &TextLayerRuntime| -> usize {
        opaque_count(
            &rt.surface(&actor)
                .expect("供給面")
                .read_back()
                .expect("read_back"),
        )
    };
    let plain = read(&rt);
    assert!(plain > 0, "選択肢テキストが描画される");
    assert_eq!(
        rt.choice_hit_rows(&actor).len(),
        2,
        "hover 前もヒット行は 2"
    );

    // ordinal 0 を hover 注入→再提示（choice_marker 変化で per-line 増分ダーティ＝Update）。
    rt.inject_choice_hover(&actor, Some(0));
    present_frame(&mut rt, &mut world, 10.0).expect("hover 提示");
    let hovered = read(&rt);
    assert!(
        hovered > plain,
        "hover が decorate→render へ配線され、ハイライト塗りで非透明ピクセルが増える: {plain} -> {hovered}"
    );
    // ヒット行照会は hover 非依存（count 不変）。
    assert_eq!(
        rt.choice_hit_rows(&actor).len(),
        2,
        "hover 後もヒット行は 2"
    );
}

// task 9.2: hover 画素檻＋ダーティ限定檻（COM・headless・R4.4/7.2/7.4）
//
// Observable: cursor.* 由来の SquareFill（塗り=(105,25,25)・文字=(255,255,255)）を持つバルーンで、
// hover on/off を注入する対フレームにおいて (a) 塗り色画素と白文字画素が ordinal-0 セグメント矩形へ
// 出現し、hover 解除で塗り色画素が消滅する（7.2）／(b) いずれのトグルフレームも当該 Choice 行のみが
// 再描画されるダーティ限定であり、全域再描画（全 Choice 行の再描画）ではない（4.4 の COM 檻・7.4）。
// draw_text_layout_calls は「ダーティ矩形数 × 交差住人数」の積で計上される（全域縮退なら増分＝全住人数）
// ため、トグルフレームの増分＝1（hover 行 1 枚のみ）で「全域再描画非発生」を決定論に固定する。
#[test]
fn hover_toggle_paints_fill_and_stays_dirty_limited_readback_pixel_cage() {
    let (mut world, window, slot) = com_world();
    let actor = ActorKey::from("0");
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    // 3 選択肢を別行へ配置（NewLine 区切り）——hover 行が 1 行に限定されることを観測する台
    // （全域再描画なら 3 行ぶん、ダーティ限定なら 1 行ぶんの描画増分になる）。
    rt.apply_cue(&choice_cue("0", 0.0, "OnYes", "はい", &["r0"]));
    rt.apply_cue(&cue("0", 0.1, CueCommand::NewLine { ratio: 1.0 }));
    rt.apply_cue(&choice_cue("0", 0.1, "OnNo", "いいえ", &["r1"]));
    rt.apply_cue(&cue("0", 0.2, CueCommand::NewLine { ratio: 1.0 }));
    rt.apply_cue(&choice_cue("0", 0.2, "OnMaybe", "どちらでも", &["r2"]));
    let image = (200u32, 100u32);
    rt.register_actor(
        actor.clone(),
        TextSlotBinding::new(slot, window, 1.0, image, image),
        // cursor.* 由来の SquareFill スタイルを運ぶバルーン（未指定 geo_model は Invert）。
        ResolvedBalloonText::resolve(&cursor_model(), image),
    );
    let width = image.0;

    // ── 画素プローブ（premultiplied BGRA・α=255 ゆえ B=25,G=25,R=105,A=255 が塗り色の厳密表現）──
    // 矩形内の塗り色（105,25,25）画素数。
    let fill_in_rect = |b: &[u8], r: &super::ChoiceHitRow| -> usize {
        let x0 = r.rect.left.floor().max(0.0) as u32;
        let x1 = (r.rect.right.ceil() as u32).min(width);
        let y0 = r.rect.top.floor().max(0.0) as u32;
        let y1 = (r.rect.bottom.ceil() as u32).min(image.1);
        let mut n = 0usize;
        for y in y0..y1 {
            for x in x0..x1 {
                let i = ((y * width + x) * 4) as usize;
                if b[i] == 25 && b[i + 1] == 25 && b[i + 2] == 105 && b[i + 3] == 255 {
                    n += 1;
                }
            }
        }
        n
    };
    // 矩形内の白文字（≈255,255,255）画素数——全チャネル閾値で AA 端を除いた芯を数える。
    let white_in_rect = |b: &[u8], r: &super::ChoiceHitRow| -> usize {
        let x0 = r.rect.left.floor().max(0.0) as u32;
        let x1 = (r.rect.right.ceil() as u32).min(width);
        let y0 = r.rect.top.floor().max(0.0) as u32;
        let y1 = (r.rect.bottom.ceil() as u32).min(image.1);
        let mut n = 0usize;
        for y in y0..y1 {
            for x in x0..x1 {
                let i = ((y * width + x) * 4) as usize;
                if b[i] >= 200 && b[i + 1] >= 200 && b[i + 2] >= 200 && b[i + 3] == 255 {
                    n += 1;
                }
            }
        }
        n
    };
    // 面全域の塗り色画素数（hover 解除で塗りが完全消滅することの檻）。
    let fill_total = |b: &[u8]| -> usize {
        let mut n = 0usize;
        let mut i = 0usize;
        while i + 3 < b.len() {
            if b[i] == 25 && b[i + 1] == 25 && b[i + 2] == 105 && b[i + 3] == 255 {
                n += 1;
            }
            i += 4;
        }
        n
    };
    let calls = |rt: &TextLayerRuntime| -> u64 {
        rt.draw_stats(&actor)
            .expect("draw_stats")
            .draw_text_layout_calls
    };

    // ── ベースライン（hover 無し・全リビール済み）: 素の選択肢テキスト＝塗り色画素は皆無 ──
    present_frame(&mut rt, &mut world, 10.0).expect("ベースライン提示");
    let rows: Vec<super::ChoiceHitRow> = rt.choice_hit_rows(&actor).to_vec();
    assert_eq!(rows.len(), 3, "3 選択肢＝3 行（ダーティ限定を測る台）");
    let base_bytes = rt
        .surface(&actor)
        .expect("供給面")
        .read_back()
        .expect("read_back");
    assert_eq!(
        fill_total(&base_bytes),
        0,
        "hover 無しでは塗り色（105,25,25）画素は 1 つも無い"
    );
    let base_calls = calls(&rt);

    // ── hover on: inject Some(0) → present → readback ──
    rt.inject_choice_hover(&actor, Some(0));
    present_frame(&mut rt, &mut world, 10.0).expect("hover on 提示");
    let hover_calls = calls(&rt);
    let hover_delta = hover_calls - base_calls;
    // 4.4/7.4: hover トグルは当該 Choice 行 1 枚のみ再描画する（全域＝3 行ぶんではない）。
    assert_eq!(
        hover_delta, 1,
        "hover on はダーティ限定＝当該行 1 枚のみ再描画（増分 1）: {hover_delta}"
    );
    assert!(
        hover_delta < rows.len() as u64,
        "全域再描画（全 {} 行）ではない: 増分 {hover_delta}",
        rows.len()
    );
    let hover_bytes = rt
        .surface(&actor)
        .expect("供給面")
        .read_back()
        .expect("read_back");
    // 7.2: ordinal-0 セグメント矩形へ塗り色＋白文字画素が出現する。
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
    // 非 hover 行（ordinal 1/2）には塗り色画素が載らない（面全域の塗り＝hover 行のぶんだけ）。
    assert_eq!(
        fill_in_rect(&hover_bytes, &rows[1]),
        0,
        "非 hover 行 1 には塗り色画素が載らない"
    );

    // 決定論（NoChange 再提示）: 同一状態の再 present は再描画せずバイト同一。
    present_frame(&mut rt, &mut world, 10.0).expect("hover NoChange 再提示");
    assert_eq!(
        calls(&rt),
        hover_calls,
        "hover 状態不変の再提示は再描画しない（DrawTextLayout 不変）"
    );
    let hover_bytes2 = rt
        .surface(&actor)
        .expect("供給面")
        .read_back()
        .expect("read_back");
    assert_eq!(
        hover_bytes, hover_bytes2,
        "決定論（hover 状態・バイト同一）"
    );
    let steady_calls = calls(&rt);

    // ── hover off: inject None → present → readback ──
    rt.inject_choice_hover(&actor, None);
    present_frame(&mut rt, &mut world, 10.0).expect("hover off 提示");
    let off_calls = calls(&rt);
    let off_delta = off_calls - steady_calls;
    // 4.4/7.4: 解除も当該行 1 枚のみ再描画（全域再描画非発生）。
    assert_eq!(
        off_delta, 1,
        "hover off もダーティ限定＝当該行 1 枚のみ再描画（増分 1）: {off_delta}"
    );
    assert!(
        off_delta < rows.len() as u64,
        "hover off も全域再描画ではない: 増分 {off_delta}"
    );
    let off_bytes = rt
        .surface(&actor)
        .expect("供給面")
        .read_back()
        .expect("read_back");
    // 7.2: 塗り色画素が面全域から消滅する（素描画へ戻る）。
    assert_eq!(
        fill_total(&off_bytes),
        0,
        "hover off で塗り色（105,25,25）画素が消滅する"
    );

    // 決定論（hover off 状態の NoChange 再提示）: バイト同一。
    present_frame(&mut rt, &mut world, 10.0).expect("off NoChange 再提示");
    let off_bytes2 = rt
        .surface(&actor)
        .expect("供給面")
        .read_back()
        .expect("read_back");
    assert_eq!(
        off_bytes, off_bytes2,
        "決定論（hover off 状態・バイト同一）"
    );
}

// task 9.3: 矩形反転縮退の pixel 檻（COM・headless・R4.3/6.1/7.2）。
//
// 9.2（`hover_toggle_paints_fill_and_stays_dirty_limited_readback_pixel_cage`）を cursor.\* 未指定
// バルーン（`geo_model`＝Invert 縮退）で反映する。正典確定「矩形反転縮退: セグメント矩形＝
// バルーン既定 font.color 塗り・文字色＝各成分 255−c」を画素で固定する: hover 行のセグメント矩形が
// **既定文字色（読取値・黒(0,0,0)）で全域不透明化**され、その上に**反転文字色（255−c＝白(255,255,255)）**の
// グリフが載る（黒矩形＋白文字＝古典反転と同観）。hover 解除で塗りが消え素描画へ戻る。
// 期待色はモデルの既定 font 色を **READ して 255−c で算出**する（黒と決め打ちしない・変異は assert で赤）。
#[test]
fn invert_hover_paints_default_font_color_fill_and_inverted_text_readback_pixel_cage() {
    let (mut world, window, slot) = com_world();
    let actor = ActorKey::from("0");
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    // 3 選択肢を別行へ配置（9.2 と同型・hover 行が 1 行に限定されることを測る台）。
    rt.apply_cue(&choice_cue("0", 0.0, "OnYes", "はい", &["r0"]));
    rt.apply_cue(&cue("0", 0.1, CueCommand::NewLine { ratio: 1.0 }));
    rt.apply_cue(&choice_cue("0", 0.1, "OnNo", "いいえ", &["r1"]));
    rt.apply_cue(&cue("0", 0.2, CueCommand::NewLine { ratio: 1.0 }));
    rt.apply_cue(&choice_cue("0", 0.2, "OnMaybe", "どちらでも", &["r2"]));
    let image = (200u32, 100u32);
    // cursor.\* 未指定 geo_model → Invert（矩形反転縮退・task 5.3 resolve）。
    let resolved = ResolvedBalloonText::resolve(&geo_model(), image);
    assert_eq!(
        resolved.choice_style,
        ResolvedChoiceStyle::Invert,
        "cursor.* 未指定バルーンは choice_style=Invert（矩形反転縮退）"
    );
    // バルーン既定文字色を READ（決め打ちしない）＝ geo_model は既定黒 (0,0,0)。
    let (fr, fg, fb) = resolved.font.color;
    assert_eq!(
        (fr, fg, fb),
        (0, 0, 0),
        "geo_model 既定文字色は黒（読取値・既定が変われば期待塗り/文字色も再計算せよ）"
    );
    // Invert::paint 正規形（塗り＝既定 font 色・文字＝各成分 255−c）を独立算出して固定する。
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

    // ── 画素プローブ（premultiplied BGRA・α=255・k=1.0/committed=0 ゆえ窓物理 px＝image px）──
    // rect の floor/ceil 画素境界（9.2 と同流儀）。
    let bounds = |r: &super::ChoiceHitRow| -> (u32, u32, u32, u32) {
        let x0 = r.rect.left.floor().max(0.0) as u32;
        let x1 = (r.rect.right.ceil() as u32).min(width);
        let y0 = r.rect.top.floor().max(0.0) as u32;
        let y1 = (r.rect.bottom.ceil() as u32).min(height);
        (x0, x1, y0, y1)
    };
    // 矩形 interior（AA 端を避けて 1px 内側へ）——「塗りが全域不透明化する」を端の縁取りに惑わされず測る。
    let interior = |r: &super::ChoiceHitRow| -> (u32, u32, u32, u32) {
        let (x0, x1, y0, y1) = bounds(r);
        (x0 + 1, x1.saturating_sub(1), y0 + 1, y1.saturating_sub(1))
    };
    let area = |(x0, x1, y0, y1): (u32, u32, u32, u32)| -> usize {
        (x1.saturating_sub(x0) as usize) * (y1.saturating_sub(y0) as usize)
    };
    // 不透明画素数（α=255・塗りが矩形を全域不透明化することの述語）。
    let opaque_in = |b: &[u8], (x0, x1, y0, y1): (u32, u32, u32, u32)| -> usize {
        let mut n = 0usize;
        for y in y0..y1 {
            for x in x0..x1 {
                let i = ((y * width + x) * 4) as usize;
                if b[i + 3] == 255 {
                    n += 1;
                }
            }
        }
        n
    };
    // 既定文字色（塗り色）(fr,fg,fb) の厳密一致画素数——BGRA ゆえ B=fb,G=fg,R=fr,A=255。
    let fill_in = |b: &[u8], (x0, x1, y0, y1): (u32, u32, u32, u32)| -> usize {
        let mut n = 0usize;
        for y in y0..y1 {
            for x in x0..x1 {
                let i = ((y * width + x) * 4) as usize;
                if b[i] == fb && b[i + 1] == fg && b[i + 2] == fr && b[i + 3] == 255 {
                    n += 1;
                }
            }
        }
        n
    };
    // 反転文字色（255−c＝白）近傍画素数——AA 芯を各チャネル ±55（255→≥200・9.2 白閾値と等価）で数える。
    let text_in = |b: &[u8], (x0, x1, y0, y1): (u32, u32, u32, u32)| -> usize {
        let mut n = 0usize;
        for y in y0..y1 {
            for x in x0..x1 {
                let i = ((y * width + x) * 4) as usize;
                let db = (b[i] as i16 - tb as i16).abs();
                let dg = (b[i + 1] as i16 - tg as i16).abs();
                let dr = (b[i + 2] as i16 - tr as i16).abs();
                if db <= 55 && dg <= 55 && dr <= 55 && b[i + 3] == 255 {
                    n += 1;
                }
            }
        }
        n
    };
    let calls = |rt: &TextLayerRuntime| -> u64 {
        rt.draw_stats(&actor)
            .expect("draw_stats")
            .draw_text_layout_calls
    };

    // ── ベースライン（hover 無し・全リビール済み）: 素描画＝既定文字色（黒）の文字・反転文字（白）皆無 ──
    present_frame(&mut rt, &mut world, 10.0).expect("ベースライン提示");
    let rows: Vec<super::ChoiceHitRow> = rt.choice_hit_rows(&actor).to_vec();
    assert_eq!(rows.len(), 3, "3 選択肢＝3 行（hover 行限定を測る台）");
    let base = rt
        .surface(&actor)
        .expect("供給面")
        .read_back()
        .expect("read_back");
    let r0_int = interior(&rows[0]);
    let r0_area = area(r0_int);
    assert!(r0_area > 0, "行 0 の interior 領域は非空");
    // ベースライン: 反転文字色（白）画素は無い（素描画＝既定黒文字）。
    assert_eq!(
        text_in(&base, r0_int),
        0,
        "hover 無しでは反転文字色（{tr},{tg},{tb}）画素は無い"
    );
    // ベースライン: 矩形 interior は塗り未充填（透明ギャップ有り＝全画素不透明ではない）。
    assert!(
        opaque_in(&base, r0_int) < r0_area,
        "hover 無しの矩形 interior は塗り未充填（透明ギャップ有り）: {}/{r0_area}",
        opaque_in(&base, r0_int)
    );
    let base_fill = fill_in(&base, r0_int); // 素描画の既定色（黒）グリフ画素数＝背景塗り増分の基準。
    let base_calls = calls(&rt);

    // ── hover on: inject Some(0) → present → readback ──
    rt.inject_choice_hover(&actor, Some(0));
    present_frame(&mut rt, &mut world, 10.0).expect("hover on 提示");
    let hover_calls = calls(&rt);
    let hover_delta = hover_calls - base_calls;
    // 4.4/7.4: hover トグルは当該 Choice 行 1 枚のみ再描画する（全域＝3 行ぶんではない）。
    assert_eq!(
        hover_delta, 1,
        "hover on はダーティ限定＝当該行 1 枚のみ再描画（増分 1）: {hover_delta}"
    );
    assert!(
        hover_delta < rows.len() as u64,
        "全域再描画（全 {} 行）ではない: 増分 {hover_delta}",
        rows.len()
    );
    let hov = rt
        .surface(&actor)
        .expect("供給面")
        .read_back()
        .expect("read_back");
    // 7.2/4.3【背景塗り】: hover 行のセグメント矩形は既定文字色（黒）塗りで interior 全域が不透明化する。
    assert_eq!(
        opaque_in(&hov, r0_int),
        r0_area,
        "hover 行の矩形 interior は既定文字色塗りで全画素不透明（反転縮退の背景塗り）"
    );
    // 7.2/4.3【塗り色＝既定 font 色】: 既定色 (fr,fg,fb) の塗り画素が素描画（グリフのみ）より大幅増する。
    let hover_fill = fill_in(&hov, r0_int);
    assert!(
        hover_fill > base_fill,
        "hover で既定文字色（{fr},{fg},{fb}）の背景塗り画素が増える: base={base_fill} hover={hover_fill}"
    );
    // 7.2/4.3【反転文字色】: 反転文字色（255−c＝白）のグリフ画素が矩形に載る。
    let hover_text = text_in(&hov, r0_int);
    assert!(
        hover_text > 0,
        "hover 行に反転文字色（{tr},{tg},{tb}）画素が載る"
    );
    // 背景塗り（黒）が反転文字ストローク（白）より支配的＝「矩形塗り＋反転文字」の対を構造保証。
    assert!(
        hover_fill > hover_text,
        "背景塗り画素（{hover_fill}）は反転文字画素（{hover_text}）より支配的（矩形＝背景・文字＝ストローク）"
    );
    // 塗りは hover 行に限定: 非 hover 行（ordinal 1）は未充填・反転文字も無い。
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
    present_frame(&mut rt, &mut world, 10.0).expect("hover NoChange 再提示");
    assert_eq!(
        calls(&rt),
        hover_calls,
        "hover 状態不変の再提示は再描画しない（DrawTextLayout 不変）"
    );
    let hov2 = rt
        .surface(&actor)
        .expect("供給面")
        .read_back()
        .expect("read_back");
    assert_eq!(hov, hov2, "決定論（hover 状態・バイト同一）");
    let steady_calls = calls(&rt);

    // ── hover off: inject None → present → readback ──
    rt.inject_choice_hover(&actor, None);
    present_frame(&mut rt, &mut world, 10.0).expect("hover off 提示");
    let off_delta = calls(&rt) - steady_calls;
    // 4.4/7.4: 解除も当該行 1 枚のみ再描画（全域再描画非発生）。
    assert_eq!(
        off_delta, 1,
        "hover off もダーティ限定＝当該行 1 枚のみ再描画（増分 1）: {off_delta}"
    );
    let off = rt
        .surface(&actor)
        .expect("供給面")
        .read_back()
        .expect("read_back");
    // 7.2/4.3: 反転縮退の塗りが消える（素描画へ戻る）。interior は未充填へ・反転文字（白）消滅。
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
    present_frame(&mut rt, &mut world, 10.0).expect("off NoChange 再提示");
    let off2 = rt
        .surface(&actor)
        .expect("供給面")
        .read_back()
        .expect("read_back");
    assert_eq!(off, off2, "決定論（hover off 状態・バイト同一）");
}
