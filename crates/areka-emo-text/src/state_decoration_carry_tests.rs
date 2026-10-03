//! 装飾の持ち運び（design.md「装飾の持ち運び」・Testing Strategy の
//! `state_decoration_carry_tests.rs` の場面・要件 3.13〜3.17・3.19・5.4・10.5）。
//!
//! 箱の構成はテストの中の surfaces.txt の文面を `parse_boxes` → `fold_boxes` で畳んで作る
//! （検体は読まない）。`font.follow` も同じ畳み込みから引く:
//! - 箱 a: `font.follow` を書かない（＝スコープに従う）・大きさ 20・既定の色 (20,0,0)
//! - 箱 x: `font.follow,balloon`（箱に閉じる箱）・大きさ 30・既定の色 (30,0,0)
//! - 箱 c: `font.follow,scope`・大きさ 40・既定の色 (40,0,0)
//! - 普通のバルーン: 大きさ 16・既定の色 (10,0,0)
//! - 無効表示の色は既定の色の赤の成分に 1 を足したもの
//!
//! サーフェス: 0＝a・x（既定は a） 1＝x・a（既定は x） 2＝c だけ 3＝箱なし 4＝c・a（既定は c）
//!
//! 観測は「続けて書いた文字の見た目」（[`now`]）を主にする。箱の場所は最初の指令か空でない
//! 写しのときに生まれるので、場所の状態を直に覗くより、利用者に見えるものを確かめる方が確かである。

use std::collections::BTreeMap;

use areka_emo_compose::{BoxName, EmoWorld, FontFollow, fold_boxes};
use areka_parsers::shell::{parse, parse_boxes};
use areka_sakura::contract::{CueCommand, FONT_TAG_CARRIER};

use super::super::route_tests::{key, select, show};
use super::super::test_support::cue;
use super::super::{BoxTraits, SurfaceKeyOutcome, TextLayerState};
use crate::look::{LookLayers, StyleId, TextLook};
use crate::place::{PlaceKey, TextPlace};

const SHELL: &str = "\
balloon.a
{
size,100,50
}
balloon.x
{
size,80,40
font.follow,balloon
}
balloon.c
{
size,60,30
font.follow,scope
}
surface0
{
element1,balloon,a,0,0
element2,balloon,x,0,60
}
surface1
{
element1,balloon,x,0,0
element2,balloon,a,0,60
}
surface2
{
element1,balloon,c,0,0
}
surface4
{
element1,balloon,c,0,0
element2,balloon,a,0,40
}
";

const RED: (u8, u8, u8) = (255, 0, 0);
const BLUE: (u8, u8, u8) = (0, 0, 255);

/// 文面を畳んだ箱の表と、箱の名前ごとの `font.follow`。
fn folded() -> (BTreeMap<u32, Vec<BoxName>>, BTreeMap<BoxName, FontFollow>) {
    let world = EmoWorld::build(&parse(SHELL));
    let (layout, report) = fold_boxes(&parse_boxes(SHELL), &BTreeMap::new(), &world);
    assert_eq!(report.issues, vec![], "検体の文面は誤りを持たない");
    let mut follow = BTreeMap::new();
    let index = (0..=4)
        .map(|id| {
            let names: Vec<BoxName> = layout
                .placements(id)
                .iter()
                .map(|p| p.name.clone())
                .collect();
            for name in &names {
                follow.insert(name.clone(), layout.def(name).expect("箱の定義").follow);
            }
            (id, names)
        })
        .collect();
    (index, follow)
}

fn name(n: &str) -> BoxName {
    folded()
        .1
        .into_keys()
        .find(|b| b.as_str() == n)
        .expect("文面にある箱の名前")
}

fn bx(n: &str) -> TextPlace {
    TextPlace::Box(name(n))
}

/// 大きさと既定の色（赤の成分）から 2 層を作る（無効表示は赤の成分に 1 を足した色）。
fn layers(height: f32, r: u8) -> LookLayers {
    let mut looks = LookLayers::default();
    looks.default.height = height;
    looks.default.color = (r, 0, 0);
    looks.disable.height = height;
    looks.disable.color = (r + 1, 0, 0);
    looks
}

/// 場所の既定の 2 層（普通のバルーン＝16・箱は名前ごと）。
fn layers_of(place: &TextPlace) -> LookLayers {
    match place {
        TextPlace::Balloon => layers(16.0, 10),
        TextPlace::Box(n) => match n.as_str() {
            "a" => layers(20.0, 20),
            "x" => layers(30.0, 30),
            _ => layers(40.0, 40),
        },
    }
}

fn default_of(place: &TextPlace) -> TextLook {
    layers_of(place).default
}

fn traits() -> BTreeMap<BoxName, BoxTraits> {
    folded()
        .1
        .into_iter()
        .map(|(n, follow)| {
            let looks = layers_of(&TextPlace::Box(n.clone()));
            (n, BoxTraits { looks, follow })
        })
        .collect()
}

/// 箱の種類を箱の表より先に入れ（Implementation Notes 5.4）、スコープ 0・1 に普通のバルーンの 2 層を差す。
fn state() -> TextLayerState {
    let mut state = TextLayerState::default();
    state.set_box_traits(traits());
    state.set_box_index(folded().0);
    for actor in ["0", "1"] {
        state.set_look_layers(&key(actor), layers_of(&TextPlace::Balloon));
    }
    state
}

fn font(state: &mut TextLayerState, actor: &str, tokens: &[&str]) {
    state.apply_cue(&cue(
        actor,
        0.0,
        CueCommand::command_carrier(
            FONT_TAG_CARRIER,
            tokens.iter().map(|t| (*t).to_owned()).collect(),
        ),
    ));
}

fn red(state: &mut TextLayerState, actor: &str) {
    font(state, actor, &["color", "255", "0", "0"]);
}

fn blue(state: &mut TextLayerState, actor: &str) {
    font(state, actor, &["color", "0", "0", "255"]);
}

fn text(state: &mut TextLayerState, actor: &str, s: &str) {
    state.apply_cue(&cue(actor, 0.0, CueCommand::Text(s.into())));
}

fn at(actor: &str, place: TextPlace) -> PlaceKey {
    PlaceKey {
        actor: key(actor),
        place,
    }
}

/// 書いた文字の見た目（番号列を表で引き直したもの・配置層と同じ引き方）。
fn glyph(state: &TextLayerState, actor: &str, place: TextPlace, ordinal: usize) -> TextLook {
    let s = state.place_state(&at(actor, place)).expect("場所の状態");
    s.styles()
        .resolve(s.glyph_styles()[ordinal], &s.look_layers().default)
        .clone()
}

/// 今の行き先へ 1 文字書き、その文字の見た目を返す（続きの文字が受け取る見た目）。
fn now(state: &mut TextLayerState, actor: &str) -> TextLook {
    text(state, actor, "・");
    let place = state.destination(&key(actor));
    let last = state
        .place_state(&at(actor, place.clone()))
        .expect("書いた場所")
        .glyph_styles()
        .len()
        - 1;
    glyph(state, actor, place, last)
}

fn shared(state: &TextLayerState, actor: &str) -> TextPlace {
    state.routes[&key(actor)].shared.clone()
}

// ---- スコープに従う場所どうし（要件 3.13・3.17）

#[test]
fn follow_kind_comes_from_font_follow_and_unwritten_means_scope() {
    let follow = folded().1;
    assert_eq!(follow[&name("a")], FontFollow::Scope, "書かない＝スコープ");
    assert_eq!(follow[&name("c")], FontFollow::Scope);
    assert_eq!(follow[&name("x")], FontFollow::Balloon);
}

#[test]
fn spec_survives_select_between_scope_boxes() {
    // `font.follow,scope` の c → 書かない a（`\b[名前]`）。
    let mut state = state();
    show(&mut state, "0", 4);
    red(&mut state, "0");
    state.apply_cue(&select("0", "a"));
    assert_eq!(state.destination(&key("0")), bx("a"));
    let g = now(&mut state, "0");
    assert_eq!((g.color, g.height), (RED, 20.0));
    assert_eq!(shared(&state, "0"), bx("a"));
}

#[test]
fn spec_survives_surface_switch_in_three_directions() {
    let mut state = state();
    show(&mut state, "0", 0); // a
    red(&mut state, "0");
    // 箱 → 別の箱（6.2）。
    show(&mut state, "0", 2);
    assert_eq!(state.destination(&key("0")), bx("c"));
    assert_eq!(now(&mut state, "0").color, RED);
    assert_eq!(shared(&state, "0"), bx("c"));
    // 箱 → 普通のバルーン（6.4）。
    show(&mut state, "0", 3);
    assert_eq!(now(&mut state, "0").color, RED);
    assert_eq!(shared(&state, "0"), TextPlace::Balloon);
    // 普通のバルーン → 箱（6.9）。普通のバルーンで足した指定も一緒に移る。
    font(&mut state, "0", &["bold", "1"]);
    show(&mut state, "0", 0);
    let g = now(&mut state, "0");
    assert_eq!((g.color, g.bold), (RED, true));
    assert_eq!(shared(&state, "0"), bx("a"));
}

#[test]
fn unset_items_take_each_destination_default() {
    // 大きさの違う 2 つのブレス（a＝20・c＝40）と普通のバルーン（16）。
    let mut state = state();
    show(&mut state, "0", 0);
    red(&mut state, "0");
    assert_eq!(now(&mut state, "0").height, 20.0);
    show(&mut state, "0", 2);
    let g = now(&mut state, "0");
    assert_eq!((g.color, g.height), (RED, 40.0));
    show(&mut state, "0", 3);
    assert_eq!(now(&mut state, "0").height, 16.0);
}

#[test]
fn font_default_shows_the_destination_defaults() {
    let mut state = state();
    show(&mut state, "0", 0);
    red(&mut state, "0");
    font(&mut state, "0", &["default"]);
    assert_eq!(now(&mut state, "0"), default_of(&bx("a")));
    show(&mut state, "0", 2);
    assert_eq!(now(&mut state, "0"), default_of(&bx("c")));
}

#[test]
fn per_item_default_resolves_against_the_new_destination() {
    let mut state = state();
    show(&mut state, "0", 0);
    red(&mut state, "0");
    font(&mut state, "0", &["bold", "1"]);
    font(&mut state, "0", &["color", "default"]);
    assert_eq!(now(&mut state, "0").color, (20, 0, 0));
    show(&mut state, "0", 2);
    let g = now(&mut state, "0");
    assert_eq!(g.color, (40, 0, 0), "c の既定の色");
    assert!(g.bold, "他の指定は残る");
}

#[test]
fn disable_base_and_unowned_keys_move_with_the_spec() {
    let mut state = state();
    show(&mut state, "0", 0);
    font(&mut state, "0", &["disable"]);
    font(&mut state, "0", &["align", "center"]);
    show(&mut state, "0", 2);
    let g = now(&mut state, "0");
    assert_eq!((g.color, g.height), ((41, 0, 0), 40.0), "c の無効表示");
    let at_c = state.place_state(&at("0", bx("c"))).unwrap();
    assert_eq!(
        at_c.unowned_vocab().get("align").map(Vec::as_slice),
        Some(["center".to_owned()].as_slice())
    );
    show(&mut state, "0", 3);
    assert_eq!(now(&mut state, "0").color, (11, 0, 0));
    let at_balloon = state.actor_state(&key("0")).unwrap();
    assert!(at_balloon.unowned_vocab().contains_key("align"));
}

#[test]
fn spec_does_not_cross_scopes() {
    let mut state = state();
    show(&mut state, "0", 0);
    show(&mut state, "1", 0);
    red(&mut state, "0");
    assert_eq!(now(&mut state, "1").color, (20, 0, 0));
    show(&mut state, "1", 2);
    assert_eq!(now(&mut state, "1").color, (40, 0, 0));
    assert_eq!(now(&mut state, "0").color, RED);
    assert_eq!(shared(&state, "0"), bx("a"));
}

// ---- 箱に閉じる箱 x（要件 3.14〜3.16）

#[test]
fn closed_box_takes_nothing_in_and_leaks_nothing_out() {
    let mut state = state();
    show(&mut state, "0", 0);
    red(&mut state, "0");
    state.apply_cue(&select("0", "x"));
    assert_eq!(
        now(&mut state, "0"),
        default_of(&bx("x")),
        "外の赤は入らない"
    );
    blue(&mut state, "0");
    show(&mut state, "0", 2);
    assert_eq!(now(&mut state, "0").color, RED, "中の青は出ない");
}

#[test]
fn owner_moves_a_a_b_through_closed_box_and_return_keeps_its_spec() {
    let mut state = state();
    show(&mut state, "0", 0);
    assert_eq!(shared(&state, "0"), bx("a"));
    red(&mut state, "0");
    state.apply_cue(&select("0", "x"));
    assert_eq!(shared(&state, "0"), bx("a"), "x へ入っても持ち主は a");
    blue(&mut state, "0");
    show(&mut state, "0", 2); // B＝c
    assert_eq!(shared(&state, "0"), bx("c"));
    assert_eq!(now(&mut state, "0").color, RED);

    // x へ戻る（同じ台詞のあいだ）: 青のまま。
    show(&mut state, "0", 0);
    state.apply_cue(&select("0", "x"));
    assert_eq!(now(&mut state, "0").color, BLUE);
    // x の中の `\f[default]` は x だけを戻し、スコープの赤は残る。
    font(&mut state, "0", &["default"]);
    assert_eq!(now(&mut state, "0"), default_of(&bx("x")));
    show(&mut state, "0", 2);
    assert_eq!(now(&mut state, "0").color, RED);
}

#[test]
fn balloon_to_closed_box_to_balloon_by_surface() {
    let mut state = state();
    show(&mut state, "0", 3);
    red(&mut state, "0");
    show(&mut state, "0", 1); // 既定は x
    assert_eq!(state.destination(&key("0")), bx("x"));
    assert_eq!(shared(&state, "0"), TextPlace::Balloon);
    assert_eq!(now(&mut state, "0"), default_of(&bx("x")));
    blue(&mut state, "0");
    show(&mut state, "0", 3);
    assert_eq!(now(&mut state, "0").color, RED);
}

#[test]
fn talk_starting_in_closed_box_leaves_to_an_empty_spec() {
    let mut state = state();
    show(&mut state, "0", 1);
    state.apply_cue(&cue("0", 0.0, CueCommand::ClearAll));
    assert_eq!(state.destination(&key("0")), bx("x"));
    assert_eq!(shared(&state, "0"), TextPlace::Balloon);
    blue(&mut state, "0");
    show(&mut state, "0", 2);
    assert_eq!(now(&mut state, "0"), default_of(&bx("c")));
    assert_eq!(shared(&state, "0"), bx("c"));
}

// ---- 台詞の頭（要件 3.19・4.10）

#[test]
fn clear_all_leaves_no_spec_anywhere() {
    let mut state = state();
    show(&mut state, "0", 0);
    red(&mut state, "0");
    font(&mut state, "0", &["align", "center"]);
    state.apply_cue(&select("0", "x"));
    blue(&mut state, "0");
    show(&mut state, "1", 1);
    font(&mut state, "1", &["disable"]);
    state.apply_cue(&cue("0", 0.0, CueCommand::ClearAll));
    assert!(
        state.actors.len() >= 4,
        "a・x・普通のバルーン 2 つの場所がある"
    );
    for (place, s) in &state.actors {
        assert!(!s.decor.has_script(), "{place:?} に指定が残る");
        assert_eq!(s.current_look(), &s.look_layers().default, "{place:?}");
    }
    assert_eq!(shared(&state, "0"), bx("a"), "既定がスコープに従う箱");
    assert_eq!(
        shared(&state, "1"),
        TextPlace::Balloon,
        "既定が箱に閉じる箱"
    );
}

#[test]
fn spec_written_in_default_scope_box_after_clear_all_survives_switch() {
    let mut state = state();
    show(&mut state, "0", 0);
    state.apply_cue(&cue("0", 0.0, CueCommand::ClearAll));
    red(&mut state, "0"); // 行き先 a で書く
    show(&mut state, "0", 2);
    assert_eq!(now(&mut state, "0").color, RED);
    show(&mut state, "0", 3);
    assert_eq!(now(&mut state, "0").color, RED);
}

// ---- シェルの切替

#[test]
fn shell_switch_points_owner_at_the_new_default_with_the_spec() {
    let mut state = state();
    show(&mut state, "0", 0);
    red(&mut state, "0");

    // 同じ表: 既定 a（スコープに従う）。
    state.set_box_traits(traits());
    state.set_box_index(folded().0);
    assert_eq!(shared(&state, "0"), bx("a"));
    assert_eq!(now(&mut state, "0").color, RED);

    // 既定が箱に閉じる箱 x の表: 持ち主は普通のバルーン、x は自分の既定。
    state.set_box_traits(traits());
    state.set_box_index(BTreeMap::from([(0, vec![name("x"), name("a")])]));
    assert_eq!(state.destination(&key("0")), bx("x"));
    assert_eq!(shared(&state, "0"), TextPlace::Balloon);
    assert_eq!(now(&mut state, "0"), default_of(&bx("x")));
    state.apply_cue(&select("0", "a"));
    assert_eq!(now(&mut state, "0").color, RED);

    // 箱の無い表: 普通のバルーン。
    state.set_box_index(BTreeMap::new());
    assert_eq!(shared(&state, "0"), TextPlace::Balloon);
    assert_eq!(now(&mut state, "0").color, RED);
}

// ---- 書いた文字と、文字の届く前の既定

#[test]
fn written_glyph_styles_never_change() {
    let mut state = state();
    show(&mut state, "0", 3);
    red(&mut state, "0");
    text(&mut state, "0", "ふ");
    show(&mut state, "0", 0);
    font(&mut state, "0", &["bold", "1"]);
    text(&mut state, "0", "あ");
    let balloon_ids = state
        .actor_state(&key("0"))
        .unwrap()
        .glyph_styles()
        .to_vec();
    let a_ids = state
        .place_state(&at("0", bx("a")))
        .unwrap()
        .glyph_styles()
        .to_vec();

    font(&mut state, "0", &["default"]);
    state.apply_cue(&select("0", "x"));
    blue(&mut state, "0");
    show(&mut state, "0", 2);
    show(&mut state, "0", 3); // 普通のバルーンへ空の指定が写る
    font(&mut state, "0", &["italic", "1"]);
    show(&mut state, "0", 0);

    assert_eq!(
        state.actor_state(&key("0")).unwrap().glyph_styles(),
        balloon_ids
    );
    assert_eq!(
        state.place_state(&at("0", bx("a"))).unwrap().glyph_styles(),
        a_ids
    );
    let f = glyph(&state, "0", TextPlace::Balloon, 0);
    assert_eq!(
        (f.color, f.bold, f.italic, f.height),
        (RED, false, false, 16.0)
    );
    let a = glyph(&state, "0", bx("a"), 0);
    assert_eq!(
        (a.color, a.bold, a.italic, a.height),
        (RED, true, false, 20.0)
    );

    // シェルの切替の後も、普通のバルーンの文字は書いたときの見た目。
    state.set_box_index(folded().0);
    assert_eq!(glyph(&state, "0", TextPlace::Balloon, 0).color, RED);
}

#[test]
fn box_place_has_its_defaults_before_any_text() {
    let mut state = state();
    show(&mut state, "0", 1); // x（写しが走らないので場所はまだ無い）
    assert!(state.place_state(&at("0", bx("x"))).is_none());
    font(&mut state, "0", &["bold", "1"]);
    text(&mut state, "0", "x");
    let s = state.place_state(&at("0", bx("x"))).unwrap();
    assert_ne!(s.glyph_styles()[0], StyleId::DEFAULT);
    let g = glyph(&state, "0", bx("x"), 0);
    assert_eq!((g.height, g.color, g.bold), (30.0, (30, 0, 0), true));

    // 空でない写しで生まれる場所も、その箱の 2 層を持っている。
    show(&mut state, "0", 3);
    red(&mut state, "0");
    show(&mut state, "0", 2);
    let at_c = state.place_state(&at("0", bx("c"))).unwrap();
    assert_eq!(at_c.look_layers(), &layers_of(&bx("c")));
    assert_eq!(now(&mut state, "0").height, 40.0);
}

// ---- 箱の無い構成（要件 5.4・10.5）

#[test]
fn without_boxes_owner_never_leaves_balloon() {
    let mut state = TextLayerState::default();
    state.set_look_layers(&key("0"), layers_of(&TextPlace::Balloon));
    red(&mut state, "0");
    for outcome in [
        SurfaceKeyOutcome::Show(0),
        SurfaceKeyOutcome::Hide,
        SurfaceKeyOutcome::Show(1),
    ] {
        state.route_surface(&key("0"), outcome);
        assert_eq!(shared(&state, "0"), TextPlace::Balloon);
    }
    state.apply_cue(&select("0", "a"));
    assert_eq!(shared(&state, "0"), TextPlace::Balloon);
    assert_eq!(now(&mut state, "0").color, RED);
    assert!(
        state.actors.keys().all(|k| k.place == TextPlace::Balloon),
        "箱の場所は 1 つも生まれない"
    );
}
