//! 箱の束の受け取りと、箱を隠す印（task 7.3・要件 3.10・6.8）。
//!
//! 箱の束はテストの中に持つ surfaces.txt の文面を `parse_boxes` → `fold_boxes` で畳んで作る
//! （検体は読まない）。箱の面の登録と装着は `actor_box_sync_tests.rs` が見るので、ここの `World` は
//! 空か差し込み口だけでよい。COM は使わない。

use std::path::PathBuf;

use areka_sakura::contract::{ActorKey, CueCommand};
use bevy_ecs::prelude::World;

use super::box_sync_tests::OVERFLOW_MESSAGE;
use super::test_support::{
    box_layout, box_name, capture_checked, cue, emote, fake_resolver, items_at,
    spawn_reserved_slot, text,
};
use super::{SurfaceKeyResolver, TextLayerRuntime, TextSlotBinding};
use crate::draw::{DEFAULT_BALLOON_BACKGROUND, ResolvedFont};
use crate::place::{PlaceKey, TextPlace};
use crate::state::test_support::font;
use crate::state::{SurfaceKeyOutcome, TextItem, TextLayerConfig};

/// 前のシェル: サーフェス 0 は箱 a（スコープに従う）だけ。
const OLD_SHELL: &str = "\
balloon.a
{
size,100,50
}
surface0
{
element1,balloon,a,0,0
}
";

/// 新しいシェル: サーフェス 0 の先頭は箱 x（箱に閉じる・色と大きさを持つ）、1 は箱 a だけ。
const NEW_SHELL: &str = "\
balloon.a
{
size,100,50
}
balloon.x
{
size,80,40
font.follow,balloon
font.height,30
font.color.r,0
font.color.g,0
font.color.b,255
}
surface0
{
element1,balloon,x,0,0
element2,balloon,a,0,60
}
surface1
{
element1,balloon,a,0,0
}
";

/// 何を聞いても解決できない閉包（差し替えで閉包が入れ替わったかを見る）。
fn never_resolver() -> SurfaceKeyResolver {
    Box::new(|_: &str| SurfaceKeyOutcome::Unresolved)
}

fn place(actor: &str, place: TextPlace) -> PlaceKey {
    PlaceKey {
        actor: ActorKey::from(actor),
        place,
    }
}

/// 箱の束を差し替えた直後: 箱の場所の文字は空、各スコープの今のサーフェス番号は保たれ、
/// 行き先は新しい表での既定。普通のバルーンの場所には触れない（要件 6.8）。
#[test]
fn replacing_the_bundle_drops_box_text_keeps_surface_and_resets_destination() {
    let mut world = World::new();
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    let old = box_layout(OLD_SHELL);
    let old_a = TextPlace::Box(box_name(&old, 0, "a"));
    rt.set_box_layout(&mut world, old, fake_resolver(), vec![]);

    rt.apply_cue(&emote("0", "0"));
    rt.apply_cue(&text("0", "あ"));
    rt.apply_cue(&text("1", "い"));
    assert_eq!(
        items_at(&rt, "0", old_a.clone()),
        vec![TextItem::glyph("あ")]
    );

    let new = box_layout(NEW_SHELL);
    let new_x = TextPlace::Box(box_name(&new, 0, "x"));
    rt.set_box_layout(&mut world, new, never_resolver(), vec![]);

    let actor = ActorKey::from("0");
    assert_eq!(items_at(&rt, "0", old_a), vec![], "箱の場所の文字は捨てる");
    assert!(
        rt.state()
            .places()
            .all(|(key, _)| key.place == TextPlace::Balloon),
        "箱の場所は 1 つも残らない"
    );
    assert_eq!(
        rt.state().current_surface(&actor),
        Some(0),
        "サーフェス番号は保つ"
    );
    assert_eq!(
        rt.state().destination(&actor),
        new_x,
        "行き先は新しい表での既定（surface0 の先頭 x）"
    );
    assert_eq!(
        items_at(&rt, "1", TextPlace::Balloon),
        vec![TextItem::glyph("い")],
        "普通のバルーンの場所には触れない"
    );

    // 閉包も新しい束のものに入れ替わる（解決できない鍵は何も変えない）。
    rt.apply_cue(&emote("0", "1"));
    assert_eq!(rt.state().current_surface(&actor), Some(0));
    assert_eq!(rt.state().destination(&actor), new_x);
}

/// 箱の名前ごとの既定の見た目は、その箱の `balloon.*`ブレスから解決した 2 層（白を混色の相手）で、
/// 箱の種類は表より先に入る（Implementation Notes 5.4）。前の表に無かった箱に閉じる箱 x が
/// 新しい既定になるとき、前の箱 a の `\f` の指定は x へ写らない。
#[test]
fn box_traits_come_from_the_brace_and_arrive_before_the_index() {
    let mut world = World::new();
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    rt.set_box_layout(&mut world, box_layout(OLD_SHELL), fake_resolver(), vec![]);
    rt.apply_cue(&emote("0", "0"));
    rt.apply_cue(&font("0", &["color", "255", "0", "0"]));

    let new = box_layout(NEW_SHELL);
    let new_x = TextPlace::Box(box_name(&new, 0, "x"));
    let TextPlace::Box(x_name) = &new_x else {
        unreachable!("箱の場所")
    };
    let expected = ResolvedFont::resolve_with_background(
        &new.def(x_name).expect("x の定義").model,
        DEFAULT_BALLOON_BACKGROUND,
    )
    .looks;
    assert_eq!(expected.default.height, 30.0, "ブレスの font.height を読む");
    rt.set_box_layout(&mut world, new, fake_resolver(), vec![]);
    rt.apply_cue(&text("0", "う"));

    let state = rt
        .state()
        .place_state(&place("0", new_x))
        .expect("x の場所");
    assert_eq!(state.look_layers(), &expected, "箱の既定の 2 層");
    assert_eq!(
        state.current_look().color,
        (0, 0, 255),
        "箱に閉じる箱へスコープの指定は写らない（種類が表より先に入っている）"
    );
}

/// 箱を隠す印はスコープごとに立ち、台詞の頭（`ClearAll`）で文字の層が自分で下ろす。
#[test]
fn hide_flag_is_per_scope_and_cleared_at_the_head_of_a_talk() {
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    let zero = ActorKey::from("0");
    let one = ActorKey::from("1");
    assert!(!rt.hidden_boxes.contains(&zero), "始めは立っていない");

    rt.hide_boxes(&zero);
    assert!(rt.hidden_boxes.contains(&zero));
    assert!(!rt.hidden_boxes.contains(&one), "別のスコープには立たない");

    rt.apply_cue(&cue("0", 0.0, CueCommand::Clear));
    assert!(rt.hidden_boxes.contains(&zero), "\\c では下ろさない");

    rt.apply_cue(&cue("0", 0.0, CueCommand::ClearAll));
    assert!(!rt.hidden_boxes.contains(&zero), "台詞の頭で下ろす");
}

/// フォントを探す場所の順は受け取った順のまま外から読める（要件 3.10）。差し替えると入れ替わる。
#[test]
fn font_search_dirs_are_readable_in_the_given_order() {
    let mut world = World::new();
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    assert!(rt.box_font_dirs().is_empty(), "束が来るまでは空");

    let dirs = vec![PathBuf::from("shell"), PathBuf::from("ghost")];
    rt.set_box_layout(
        &mut world,
        box_layout(OLD_SHELL),
        fake_resolver(),
        dirs.clone(),
    );
    assert_eq!(rt.box_font_dirs(), dirs.as_slice());

    rt.set_box_layout(
        &mut world,
        box_layout(NEW_SHELL),
        fake_resolver(),
        vec![PathBuf::from("other")],
    );
    assert_eq!(rt.box_font_dirs(), [PathBuf::from("other")].as_slice());
}

/// 箱 a（100×50）を `(x, y)` に置くシェルの文面（指摘 1・要件 7）。
fn shell_with_box_at(x: i64, y: i64) -> String {
    format!("balloon.a\n{{\nsize,100,50\n}}\nsurface0\n{{\nelement1,balloon,a,{x},{y}\n}}\n")
}

/// 置き場所に `i64::MAX` を含む箱でも、位置に大きさを足す 4 つの足し算（当たりの矩形の右と下・
/// はみ出しの判断の x と y）は上限で止まる（要件 7.1・7.2）: はみ出しの警告は 2 度の同期でちょうど
/// 1 回出て、当たりの矩形は裏返らない。x だけ・y だけが上限の置き場所も並べるので、どの足し算を
/// ただの足し算に戻しても、試験の作りではあふれて落ちる（要件 7.3・y だけの置き場所は x 側が
/// 収まるので `||` が y の足し算まで届く）。
#[test]
fn box_at_the_integer_max_site_warns_once_and_keeps_its_hit_rect_upright() {
    for (x, y) in [(i64::MAX, 0), (0, i64::MAX), (i64::MAX, i64::MAX)] {
        let mut world = World::new();
        let (window, slot) = spawn_reserved_slot(&mut world);
        let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
        let layout = box_layout(&shell_with_box_at(x, y));
        let key = place("0", TextPlace::Box(box_name(&layout, 0, "a")));
        rt.set_box_layout(&mut world, layout, fake_resolver(), vec![]);
        rt.apply_cue(&emote("0", "0"));
        rt.apply_cue(&text("0", "あ"));
        let shell = TextSlotBinding::new(slot, window, 1.0, (200, 300), (200, 300));

        let ((), events) = capture_checked(|| {
            for _ in 0..2 {
                rt.sync_box_bindings(&mut world, &[(ActorKey::from("0"), Some((shell, 0)))]);
            }
        });
        assert_eq!(
            events
                .iter()
                .filter(|e| e.message() == OVERFLOW_MESSAGE)
                .count(),
            1,
            "({x}, {y}): 上限で止まった端はサーフェスの画像より大きい＝はみ出す"
        );

        rt.refresh_shown_boxes(&[key], 10.0);
        let [shown] = rt.shown_boxes(&ActorKey::from("0")) else {
            panic!("({x}, {y}): 文字の出ている箱 a が 1 つ載る")
        };
        assert!(
            shown.rect.right >= shown.rect.left && shown.rect.bottom >= shown.rect.top,
            "({x}, {y}): 当たりの矩形は裏返らない: {:?}",
            shown.rect
        );
    }
}
