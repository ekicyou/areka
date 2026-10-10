//! 場所ごとの提示と、箱の四角・選択肢の行の窓の座標での照会（task 7.5・要件 3.4・3.9・5.1・8.1・
//! 8.2・9.3）。
//!
//! 写し（`shown_boxes`）の更新と外し方・普通のバルーンの窓の文字の数・場所を指す照会と強調は
//! COM を使わずに見る。箱の面を実際に作って提示する通しの 1 本だけが COM を使う（描画の中身の
//! 確かめは 7.8）。箱の束はテストの中に持つ surfaces.txt の文面を畳んで作る（検体は読まない）。

use areka_sakura::contract::{ActorKey, CueCommand};
use bevy_ecs::hierarchy::Children;

use super::box_sync_tests::{Fixture, emote, fixture, layout, resolver, select, text};
use super::test_support::{choice_cue, com_world, cue};
use super::{
    ChoiceHitRow, HitRectPx, ResolvedBalloonText, TextLayerRuntime, TextSlotBinding, present_frame,
};
use crate::draw::DEFAULT_BALLOON_BACKGROUND;
use crate::place::{PlaceKey, TextPlace};
use crate::state::{SpanKind, TextLayerConfig};

/// 全部の文字が見えている時刻。
const LATE: f64 = 10.0;

fn actor() -> ActorKey {
    ActorKey::from("0")
}

fn rect(left: f32, top: f32, right: f32, bottom: f32) -> HitRectPx {
    HitRectPx {
        left,
        top,
        right,
        bottom,
    }
}

/// 写しの中身（名前・element番号・四角）を手前から。
fn shown(f: &Fixture) -> Vec<(String, u32, HitRectPx)> {
    f.rt.shown_boxes(&actor())
        .iter()
        .map(|b| (b.name.as_str().to_owned(), b.element, b.rect))
        .collect()
}

/// サーフェス 0 の箱 a（element1・(10,20)・100×50）と b（element2・(0,100)・60×40）に文字を書き、
/// 拡大率 2 で登録して、両方を提示したことにする。
fn two_boxes_shown() -> Fixture {
    let mut f = fixture();
    f.show("0");
    f.rt.apply_cue(&text("0", "あ"));
    f.rt.apply_cue(&select("0", "b"));
    f.rt.apply_cue(&text("0", "い"));
    f.sync(Some(2.0));
    let presented = [f.key(0, "a"), f.key(0, "b")];
    f.rt.refresh_shown_boxes(&presented, LATE);
    f
}

/// 文字が見えている箱の四角を、element番号の大きい順（手前から）に、窓の物理 px で並べる（要件 3.9）。
#[test]
fn shown_boxes_lists_visible_boxes_front_to_back_in_window_physical_px() {
    let f = two_boxes_shown();
    assert_eq!(
        shown(&f),
        vec![
            ("b".to_owned(), 2, rect(0.0, 200.0, 120.0, 280.0)),
            ("a".to_owned(), 1, rect(20.0, 40.0, 220.0, 140.0)),
        ],
        "四角は (箱の X, 箱の Y, X+幅, Y+高さ) × 拡大率"
    );
    assert!(
        f.rt.shown_boxes(&ActorKey::from("1")).is_empty(),
        "箱を持たないスコープは空"
    );
}

/// 提示しなかった箱と、文字が 1 字も見えていない箱は載せない（要件 9.3）。
#[test]
fn unpresented_or_unrevealed_boxes_are_not_shown() {
    let mut f = two_boxes_shown();
    let only_a = [f.key(0, "a")];
    f.rt.refresh_shown_boxes(&only_a, LATE);
    assert_eq!(
        shown(&f).into_iter().map(|s| s.0).collect::<Vec<_>>(),
        vec!["a".to_owned()],
        "提示した箱だけ"
    );
    f.rt.refresh_shown_boxes(&only_a, -1.0);
    assert_eq!(shown(&f), vec![], "まだ 1 字も見えていない");
    let balloon = [PlaceKey::balloon(&actor())];
    f.rt.refresh_shown_boxes(&balloon, LATE);
    assert_eq!(shown(&f), vec![], "普通のバルーンの場所は箱ではない");
}

/// `\c` は今の行き先の箱だけを写しから外す。他の箱はそのまま（要件 7.1・7.2）。
#[test]
fn clear_drops_only_the_destination_box() {
    let mut f = two_boxes_shown();
    f.rt.apply_cue(&cue("0", 0.0, CueCommand::Clear));
    assert_eq!(
        shown(&f).into_iter().map(|s| s.0).collect::<Vec<_>>(),
        vec!["a".to_owned()],
        "行き先 b だけが外れる"
    );
}

/// `\b[名前]` で行き先だけが替わっても、前の箱の文字は出たまま（要件 4.5）なので外さない。
/// 台本の `\s` の受け取りでも外さない（絵はまだ替わっていない・本 spec 要件 1.6・2.1）。
/// 置き場所の変わる絵の切替と絵の非表示は、その同期で外す（要件 6.2・6.6・本 spec 要件 1.1〜1.3）。
#[test]
fn only_the_picture_number_drops_boxes_from_the_snapshot() {
    let mut f = two_boxes_shown();
    let before = shown(&f);
    f.rt.apply_cue(&select("0", "a"));
    assert_eq!(shown(&f), before, "行き先だけが替わっても出ている箱は残る");
    for key in ["0", "2", "4", "-1"] {
        f.rt.apply_cue(&emote("0", key));
        assert_eq!(shown(&f), before, "\\s[{key}] の受け取りでは外さない");
        f.sync(Some(2.0));
        assert_eq!(shown(&f), before, "絵 0 のままの同期でも外さない");
    }

    // 絵 2 には b だけが別の位置 (0,0) に在る: a は名前が外れ、b は位置が替わる。
    f.picture = Some(2);
    f.sync(Some(2.0));
    assert_eq!(shown(&f), vec![], "置き場所が変わった箱はその同期で外す");

    let mut f = two_boxes_shown();
    f.picture = None;
    f.sync(Some(2.0));
    assert_eq!(shown(&f), vec![], "絵の非表示");
}

/// 台詞の頭・箱を隠す印・箱の束の差し替えは、そのスコープの写しを空にする（要件 6.7・6.8・6.10）。
#[test]
fn clear_all_hide_and_bundle_replacement_empty_the_snapshot() {
    let mut f = two_boxes_shown();
    f.rt.apply_cue(&cue("0", 0.0, CueCommand::ClearAll));
    assert_eq!(shown(&f), vec![], "台詞の頭");

    let mut f = two_boxes_shown();
    f.rt.hide_boxes(&actor());
    assert_eq!(shown(&f), vec![], "箱を隠す印");
    let presented = [f.key(0, "a"), f.key(0, "b")];
    f.rt.refresh_shown_boxes(&presented, LATE);
    assert_eq!(
        shown(&f),
        vec![],
        "印が立っているあいだは提示しても載せない"
    );

    let mut f = two_boxes_shown();
    f.rt.set_box_layout(&mut f.world, layout(), resolver(), vec![]);
    assert_eq!(shown(&f), vec![], "箱の束の差し替え");
}

/// 普通のバルーンの窓に今出ている文字の数: 表示している絵が箱を持つ面なら 0、そうでなければ普通の
/// バルーンの場所の見えている文字の数（要件 5.1・5.2・6.9・本 spec 要件 1.8）。台本の `\s` の
/// 受け取りには従わない。箱の表が空なら絵の番号に依らない（本 spec 要件 4.5）。
#[test]
fn balloon_shown_glyphs_follows_the_shown_picture_number() {
    let mut f = fixture();
    let a = actor();
    f.rt.apply_cue(&text("0", "あい"));
    // 絵 0・2 は箱を持ち、絵 4 は持たない。
    for (picture, expected, why) in [
        (Some(0), 0, "箱のある絵では窓に出さない"),
        (Some(2), 0, "箱のある絵では窓に出さない"),
        (Some(4), 2, "箱の無い絵では普通のバルーンの文字が出る"),
        (None, 2, "絵の非表示・未確立は箱が無い"),
    ] {
        assert_eq!(
            f.rt.balloon_shown_glyphs(&a, picture, LATE),
            expected,
            "絵 {picture:?}: {why}"
        );
    }
    f.rt.apply_cue(&emote("0", "0"));
    assert_eq!(
        f.rt.state().visible_glyphs(&a, LATE),
        2,
        "普通のバルーンの文字は保持している"
    );
    assert_eq!(
        f.rt.balloon_shown_glyphs(&a, Some(4), LATE),
        2,
        "\\s[0] を受け取っても絵が 4 のままなら窓に出す"
    );
    f.rt.apply_cue(&emote("0", "4"));
    assert_eq!(
        f.rt.balloon_shown_glyphs(&a, Some(0), LATE),
        0,
        "\\s[4] を受け取っても絵が 0 のままなら窓に出さない"
    );

    // 箱の表が空のシェル（箱の束がまだ無いランタイム）は、絵の番号に依らず本 spec の前と同じ値
    // （要件 5.4・本 spec 要件 4.5）。
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    rt.apply_cue(&text("0", "あい"));
    rt.apply_cue(&emote("0", "0"));
    for picture in [None, Some(0), Some(2), Some(4), Some(u32::MAX)] {
        assert_eq!(
            rt.balloon_shown_glyphs(&a, picture, LATE),
            2,
            "箱の表が空なら絵 {picture:?} でも同じ値"
        );
    }
}

/// 場所を指す照会と強調: 箱の場所の行と強調はその場所の鍵で持ち、普通のバルーンの口は
/// 普通のバルーンの場所と同じものを読む。箱の位置は箱の置き場所、普通のバルーンは (0,0)。
#[test]
fn place_addressed_hit_rows_hover_and_origin() {
    let mut f = two_boxes_shown();
    let a = actor();
    let box_a = f.key(0, "a");
    let balloon = PlaceKey::balloon(&a);
    let row = ChoiceHitRow {
        kind: SpanKind::Choice,
        ordinal: 0,
        id: "OnA".into(),
        label: "A".into(),
        references: vec![],
        rect: rect(1.0, 2.0, 3.0, 4.0),
    };
    f.rt.choice_snapshot
        .insert(box_a.clone(), vec![row.clone()]);
    assert_eq!(f.rt.choice_hit_rows_at(&box_a), std::slice::from_ref(&row));
    assert!(f.rt.choice_hit_rows_at(&balloon).is_empty());
    assert!(
        f.rt.choice_hit_rows(&a).is_empty(),
        "普通のバルーンの口は箱を読まない"
    );

    f.rt.inject_choice_hover_at(&box_a, Some(0));
    assert_eq!(f.rt.choice_hover.get(&box_a), Some(&Some(0)));
    assert_eq!(
        f.rt.choice_hover.get(&balloon),
        None,
        "普通のバルーンには入らない"
    );
    f.rt.inject_choice_hover(&a, Some(1));
    assert_eq!(f.rt.choice_hover.get(&balloon), Some(&Some(1)));
    assert_eq!(f.rt.choice_hover.get(&box_a), Some(&Some(0)));

    assert_eq!(f.rt.box_origin(&box_a), (10.0, 20.0));
    assert_eq!(f.rt.box_origin(&f.key(0, "b")), (0.0, 100.0));
    assert_eq!(f.rt.box_origin(&balloon), (0.0, 0.0));
}

/// 通し（COM）: 箱に選択肢を出すと、普通のバルーンと同じ並べ方で、箱の位置を足した窓の物理 px の
/// 当たり行が返る（要件 8.1・8.2）。箱の面はシェルの窓の差し込み口の直後の子になり、写しに載る。
#[test]
fn box_choice_rows_equal_balloon_rows_shifted_by_the_box_position() {
    let (mut world, window, slot) = com_world();
    let k = 1.5;
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    let layout = layout();
    rt.set_box_layout(&mut world, layout.clone(), resolver(), vec![]);

    // スコープ 0 は箱 a（サーフェス 0・(10,20)）へ、スコープ 1 は同じ定義を普通のバルーンとして。
    rt.apply_cue(&emote("0", "0"));
    for scope in ["0", "1"] {
        rt.apply_cue(&choice_cue(scope, 0.0, "OnYes", "はい", &["r0"]));
        rt.apply_cue(&choice_cue(scope, 0.2, "OnNo", "いいえ", &["r1"]));
    }
    let shell_image = (200u32, 300u32);
    let shell_physical = (300u32, 450u32);
    let shell = TextSlotBinding::new(slot, window, k, shell_physical, shell_image);
    rt.sync_box_bindings(&mut world, &[(actor(), Some((shell, 0)))]);

    let name_a = layout
        .placements(0)
        .iter()
        .find(|p| p.name.as_str() == "a")
        .expect("a")
        .name
        .clone();
    let def = layout.def(&name_a).expect("a の定義");
    let other = ActorKey::from("1");
    rt.register_actor(
        other.clone(),
        TextSlotBinding::new(slot, window, k, (150, 75), def.size),
        ResolvedBalloonText::resolve_with_background(
            &def.model,
            def.size,
            DEFAULT_BALLOON_BACKGROUND,
        ),
    );

    present_frame(&mut rt, &mut world, LATE).expect("提示フレーム");

    let box_a = PlaceKey {
        actor: actor(),
        place: TextPlace::Box(name_a),
    };
    let box_rows = rt.choice_hit_rows_at(&box_a).to_vec();
    let balloon_rows = rt.choice_hit_rows(&other).to_vec();
    // 箱 a は描いてよい範囲が狭く書き出しが右寄り（`origin.x,-30`）なので、選択肢は 1 字ずつ折り返す。
    assert!(box_rows.len() >= 2, "箱の選択肢の行: {box_rows:?}");
    assert_eq!(
        box_rows.len(),
        balloon_rows.len(),
        "普通のバルーンと同じ並べ方（同じ行の数）"
    );
    for (b, n) in box_rows.iter().zip(&balloon_rows) {
        assert_eq!((b.ordinal, &b.id, &b.label), (n.ordinal, &n.id, &n.label));
        let close = |x: f32, y: f32| (x - y).abs() < 1e-3;
        assert!(
            close(b.rect.left, n.rect.left + 10.0 * k)
                && close(b.rect.right, n.rect.right + 10.0 * k)
                && close(b.rect.top, n.rect.top + 20.0 * k)
                && close(b.rect.bottom, n.rect.bottom + 20.0 * k),
            "箱の行＝普通のバルーンの行＋箱の位置×拡大率: {:?} / {:?}",
            b.rect,
            n.rect
        );
    }
    assert!(
        rt.choice_hit_rows(&actor()).is_empty(),
        "箱の行は普通のバルーンの口に混ざらない"
    );

    // 面は窓の子で、差し込み口の直後に挿さる。
    let child = rt
        .surfaces
        .get(&box_a)
        .and_then(|r| r.surface.window_child())
        .expect("箱の面は窓の子を持つ");
    let children: Vec<bevy_ecs::entity::Entity> = world
        .get::<Children>(window)
        .expect("窓の子")
        .iter()
        .copied()
        .collect();
    let slot_at = children
        .iter()
        .position(|c| *c == slot)
        .expect("差し込み口");
    assert_eq!(children.get(slot_at + 1), Some(&child));

    // 写し: 文字の見えている箱 a が、窓の物理 px の四角で載る。
    let shown = rt.shown_boxes(&actor());
    assert_eq!(shown.len(), 1);
    assert_eq!(shown[0].rect, rect(15.0, 30.0, 165.0, 105.0));
}
