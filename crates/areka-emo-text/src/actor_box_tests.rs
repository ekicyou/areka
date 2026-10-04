//! 箱の束の受け取りと、箱を隠す印（task 7.3・要件 3.10・6.8）。
//!
//! 箱の束はテストの中に持つ surfaces.txt の文面を `parse_boxes` → `fold_boxes` で畳んで作る
//! （検体は読まない）。箱の面はまだ作られない（面の登録は 7.4）ので、`World` は空でよい。COM は使わない。

use std::collections::BTreeMap;
use std::path::PathBuf;

use areka_emo_compose::{BoxLayout, EmoWorld, fold_boxes};
use areka_parsers::shell::{parse, parse_boxes};
use areka_sakura::contract::{ActorKey, CueCommand, FONT_TAG_CARRIER, TalkCue};
use bevy_ecs::prelude::World;

use super::test_support::cue;
use super::{SurfaceKeyResolver, TextLayerRuntime};
use crate::draw::{DEFAULT_BALLOON_BACKGROUND, ResolvedFont};
use crate::place::{PlaceKey, TextPlace};
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

fn layout(text: &str) -> BoxLayout {
    let world = EmoWorld::build(&parse(text));
    let (layout, report) = fold_boxes(&parse_boxes(text), &BTreeMap::new(), &world);
    assert_eq!(report.issues, vec![], "文面は誤りを持たない");
    layout
}

/// 文面にある箱の場所（名前は公開の構築口を持たないので表から拾う）。
fn boxed(layout: &BoxLayout, surface: u32, name: &str) -> TextPlace {
    let found = layout
        .placements(surface)
        .iter()
        .find(|p| p.name.as_str() == name)
        .expect("文面にある箱の名前");
    TextPlace::Box(found.name.clone())
}

/// 偽の閉包: 数字ならその番号を表示、`-1` は非表示、それ以外は解決できない。
fn fake_resolver() -> SurfaceKeyResolver {
    Box::new(|key: &str| match key {
        "-1" => SurfaceKeyOutcome::Hide,
        _ => key
            .parse()
            .map_or(SurfaceKeyOutcome::Unresolved, SurfaceKeyOutcome::Show),
    })
}

/// 何を聞いても解決できない閉包（差し替えで閉包が入れ替わったかを見る）。
fn never_resolver() -> SurfaceKeyResolver {
    Box::new(|_: &str| SurfaceKeyOutcome::Unresolved)
}

fn emote(actor: &str, key: &str) -> TalkCue {
    cue(actor, 0.0, CueCommand::Emote { key: key.into() })
}

fn text(actor: &str, t: &str) -> TalkCue {
    cue(actor, 0.0, CueCommand::Text(t.into()))
}

fn font(actor: &str, tokens: &[&str]) -> TalkCue {
    cue(
        actor,
        0.0,
        CueCommand::command_carrier(
            FONT_TAG_CARRIER,
            tokens.iter().map(|t| (*t).to_owned()).collect(),
        ),
    )
}

fn place(actor: &str, place: TextPlace) -> PlaceKey {
    PlaceKey {
        actor: ActorKey::from(actor),
        place,
    }
}

fn items_at(rt: &TextLayerRuntime, actor: &str, at: TextPlace) -> Vec<TextItem> {
    rt.state()
        .place_state(&place(actor, at))
        .map_or_else(Vec::new, |s| s.items().to_vec())
}

/// 箱の束を差し替えた直後: 箱の場所の文字は空、各スコープの今のサーフェス番号は保たれ、
/// 行き先は新しい表での既定。普通のバルーンの場所には触れない（要件 6.8）。
#[test]
fn replacing_the_bundle_drops_box_text_keeps_surface_and_resets_destination() {
    let mut world = World::new();
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    let old = layout(OLD_SHELL);
    let old_a = boxed(&old, 0, "a");
    rt.set_box_layout(&mut world, old, fake_resolver(), vec![]);

    rt.apply_cue(&emote("0", "0"));
    rt.apply_cue(&text("0", "あ"));
    rt.apply_cue(&text("1", "い"));
    assert_eq!(
        items_at(&rt, "0", old_a.clone()),
        vec![TextItem::glyph("あ")]
    );

    let new = layout(NEW_SHELL);
    let new_x = boxed(&new, 0, "x");
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
    assert_eq!(
        rt.state().destination(&actor),
        boxed(&layout(NEW_SHELL), 0, "x")
    );
}

/// 箱の名前ごとの既定の見た目は、その箱の `balloon.*`ブレスから解決した 2 層（白を混色の相手）で、
/// 箱の種類は表より先に入る（Implementation Notes 5.4）。前の表に無かった箱に閉じる箱 x が
/// 新しい既定になるとき、前の箱 a の `\f` の指定は x へ写らない。
#[test]
fn box_traits_come_from_the_brace_and_arrive_before_the_index() {
    let mut world = World::new();
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    rt.set_box_layout(&mut world, layout(OLD_SHELL), fake_resolver(), vec![]);
    rt.apply_cue(&emote("0", "0"));
    rt.apply_cue(&font("0", &["color", "255", "0", "0"]));

    let new = layout(NEW_SHELL);
    let new_x = boxed(&new, 0, "x");
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

    rt.apply_cue(&cue("1", 0.0, CueCommand::Clear));
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
    rt.set_box_layout(&mut world, layout(OLD_SHELL), fake_resolver(), dirs.clone());
    assert_eq!(rt.box_font_dirs(), dirs.as_slice());

    rt.set_box_layout(
        &mut world,
        layout(NEW_SHELL),
        fake_resolver(),
        vec![PathBuf::from("other")],
    );
    assert_eq!(rt.box_font_dirs(), [PathBuf::from("other")].as_slice());
}
