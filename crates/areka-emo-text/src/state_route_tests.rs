//! 文字の行き先の決まり方（spec `areka-P0-shell-balloon` の design.md「文字の行き先の決まり方」の表の 11 行・要件 4.1〜4.4・
//! 4.7〜4.10・5.4・6.1・6.2・6.4・6.6・6.9・10.1）。
//!
//! 断りの無い要件の番号と design.md の参照は spec `areka-P0-shell-balloon` のもの（別の spec のものはその場で名を書く）。
//!
//! 箱の名前は公開の構築口を持たないので、テストの中に持つ surfaces.txt の文面を
//! `parse_boxes` → `fold_boxes` で畳んで取り出す（検体は読まない）。
//! 組み立ての支援（`pub(super)` のもの）は `state_place_tests.rs`・`state_anchor_tests.rs`・
//! `state_decoration_carry_tests.rs` も使う。

use std::collections::BTreeMap;
use std::sync::LazyLock;

use areka_emo_compose::BoxName;
use areka_sakura::contract::{FONT_TAG_CARRIER, TalkCue};
use log_capture_kit::{CapturedEvent, capture};

use super::test_support::{cue, items_of};
use super::*;
use crate::actor::test_support::{box_in, box_index, box_layout, capture_checked};

/// 箱の構成:
/// - 0: element1=a・element2=b（既定は a）
/// - 1: element1=b・element3=a（既定は b。a も在る）
/// - 2: element1=c だけ
/// - 3: 箱なし（表に載らない）
const SHELL: &str = "\
balloon.a
{
size,100,50
}
balloon.b
{
size,80,40
}
balloon.c
{
size,60,30
}
surface0
{
element1,balloon,a,0,0
element2,balloon,b,0,60
}
surface1
{
element3,balloon,a,10,10
element1,balloon,b,0,0
}
surface2
{
element1,balloon,c,0,0
}
";

/// 文面を畳んだ箱の表（サーフェス番号 → element番号の昇順の名前・1 度だけ組む）。
static INDEX: LazyLock<BTreeMap<u32, Vec<BoxName>>> =
    LazyLock::new(|| box_index(&box_layout(SHELL), 0..=3));

pub(super) fn index() -> BTreeMap<u32, Vec<BoxName>> {
    INDEX.clone()
}

pub(super) fn boxed(name: &str) -> TextPlace {
    TextPlace::Box(box_in(&INDEX, name))
}

pub(super) fn key(actor: &str) -> ActorKey {
    ActorKey::from(actor)
}

pub(super) fn state_with_boxes() -> TextLayerState {
    let mut state = TextLayerState::default();
    state.set_box_index(index());
    state
}

pub(super) fn select(actor: &str, name: &str) -> TalkCue {
    cue(
        actor,
        0.0,
        CueCommand::BalloonSurface {
            key: name.to_owned(),
        },
    )
}

pub(super) fn show(state: &mut TextLayerState, actor: &str, id: u32) {
    state.route_surface(&key(actor), SurfaceKeyOutcome::Show(id));
}

fn warns(events: &[CapturedEvent]) -> Vec<&CapturedEvent> {
    events
        .iter()
        .filter(|e| e.level == tracing::Level::WARN)
        .collect()
}

pub(super) fn place_items(state: &TextLayerState, actor: &str, place: TextPlace) -> Vec<TextItem> {
    state
        .place_state(&PlaceKey {
            actor: key(actor),
            place,
        })
        .map_or_else(Vec::new, |s| s.items().to_vec())
}

// ---- 表の行 1〜3: 解決できた `\s`（表示）

#[test]
fn row1_same_name_box_on_new_surface_keeps_destination() {
    // 要件 6.1: a に書いている途中で、a も持つサーフェス 1（既定は b）へ替えても a のまま。
    let mut state = state_with_boxes();
    show(&mut state, "0", 0);
    assert_eq!(state.destination(&key("0")), boxed("a"));
    show(&mut state, "0", 1);
    assert_eq!(state.current_surface(&key("0")), Some(1));
    assert_eq!(
        state.destination(&key("0")),
        boxed("a"),
        "既定の b へ替えない"
    );
}

#[test]
fn row2_other_boxes_only_moves_to_first_box() {
    // 要件 6.2: a のサーフェス 0 から c だけのサーフェス 2 へ → 先頭の c。
    let mut state = state_with_boxes();
    show(&mut state, "0", 0);
    show(&mut state, "0", 2);
    assert_eq!(state.current_surface(&key("0")), Some(2));
    assert_eq!(state.destination(&key("0")), boxed("c"));
}

#[test]
fn row2_from_balloon_to_surface_with_boxes_moves_to_first_box() {
    // 要件 4.1・4.2・6.9: 普通のバルーンから箱のあるサーフェスへ → element番号が一番小さい箱。
    let mut state = state_with_boxes();
    assert_eq!(state.destination(&key("0")), TextPlace::Balloon);
    show(&mut state, "0", 1);
    assert_eq!(
        state.destination(&key("0")),
        boxed("b"),
        "書いた順でなく element番号の昇順の先頭"
    );
}

#[test]
fn row3_surface_without_boxes_moves_to_balloon() {
    // 要件 6.4: 箱のサーフェスから箱の無いサーフェスへ → 普通のバルーン（番号は新しい番号）。
    let mut state = state_with_boxes();
    show(&mut state, "0", 0);
    show(&mut state, "0", 3);
    assert_eq!(state.current_surface(&key("0")), Some(3));
    assert_eq!(state.destination(&key("0")), TextPlace::Balloon);
}

// ---- 表の行 4〜6: 非表示・解決できない

#[test]
fn row4_hide_sets_hidden_and_balloon() {
    // 要件 6.6.
    let mut state = state_with_boxes();
    show(&mut state, "0", 0);
    state.route_surface(&key("0"), SurfaceKeyOutcome::Hide);
    assert_eq!(state.current_surface(&key("0")), None);
    assert_eq!(state.destination(&key("0")), TextPlace::Balloon);
}

#[test]
fn row5_unresolved_changes_nothing() {
    let mut state = state_with_boxes();
    show(&mut state, "0", 0);
    state.apply_cue(&select("0", "b"));
    let before = state.clone();
    state.route_surface(&key("0"), SurfaceKeyOutcome::Unresolved);
    assert_eq!(state, before);
    assert_eq!(state.current_surface(&key("0")), Some(0));
    assert_eq!(state.destination(&key("0")), boxed("b"));
}

#[test]
fn row6_unresolved_on_an_unseen_scope_creates_nothing() {
    // 番号は読めるが今のシェルに無いサーフェスでは、閉包が Unresolved を返す（閉包の答えは
    // ここでは直に渡す）。記録の無いスコープでも何も作らない。
    let mut state = state_with_boxes();
    let before = state.clone();
    state.route_surface(&key("1"), SurfaceKeyOutcome::Unresolved);
    assert_eq!(state, before);
    assert_eq!(state.current_surface(&key("1")), None);
    assert_eq!(state.destination(&key("1")), TextPlace::Balloon);
}

// ---- 表の行 7: シェルの切替

#[test]
fn row7_shell_switch_keeps_surface_and_redraws_default_and_drops_box_text() {
    // 要件 6.8: 番号は保ち、行き先は新しい表での既定へ。箱の場所の文字は捨て、
    // 普通のバルーンの場所の文字は残す。
    let mut state = state_with_boxes();
    state.apply_cue(&cue("0", 0.0, CueCommand::Text("前".into())));
    show(&mut state, "0", 0);
    state.apply_cue(&select("0", "b"));
    state.apply_cue(&cue("0", 0.0, CueCommand::Text("箱".into())));
    assert_eq!(place_items(&state, "0", boxed("b")).len(), 1);

    state.set_box_index(index());
    assert_eq!(state.current_surface(&key("0")), Some(0));
    assert_eq!(state.destination(&key("0")), boxed("a"), "新しい表での既定");
    assert_eq!(
        place_items(&state, "0", boxed("b")),
        vec![],
        "箱の文字は捨てる"
    );
    assert_eq!(items_of(&state, "0"), &[TextItem::glyph("前")]);

    // 箱の無い表へ替えれば普通のバルーンへ。
    state.set_box_index(BTreeMap::new());
    assert_eq!(state.current_surface(&key("0")), Some(0));
    assert_eq!(state.destination(&key("0")), TextPlace::Balloon);
}

// ---- 表の行 8〜10: `\b`

#[test]
fn row8_named_box_on_current_surface_switches_without_warning() {
    // 要件 4.3・4.9.
    let mut state = state_with_boxes();
    show(&mut state, "0", 0);
    let (_, events) = capture_checked(|| state.apply_cue(&select("0", "b")));
    assert_eq!(state.destination(&key("0")), boxed("b"));
    assert!(
        warns(&events).is_empty(),
        "在る名前は警告しない: {events:?}"
    );
}

#[test]
fn row9_missing_name_warns_once_with_name_surface_scope_and_keeps_destination() {
    // 要件 4.4・10.1: c はサーフェス 0 に無い。
    let mut state = state_with_boxes();
    show(&mut state, "0", 0);
    let (_, events) = capture(|| state.apply_cue(&select("0", "c")));
    assert_eq!(state.destination(&key("0")), boxed("a"));
    let warned = warns(&events);
    assert_eq!(warned.len(), 1, "{events:?}");
    assert_eq!(warned[0].field_str("name"), Some("c"));
    assert_eq!(warned[0].field("surface"), Some("0"));
    assert_eq!(warned[0].field("actor"), Some("0"));

    // 非表示なら番号の代わりに「非表示」。
    state.route_surface(&key("0"), SurfaceKeyOutcome::Hide);
    let (_, events) = capture(|| state.apply_cue(&select("0", "a")));
    assert_eq!(state.destination(&key("0")), TextPlace::Balloon);
    let warned = warns(&events);
    assert_eq!(warned.len(), 1, "{events:?}");
    assert_eq!(warned[0].field("surface"), Some("非表示"));
}

#[test]
fn row10_integer_balloon_surface_does_nothing() {
    // 要件 4.8: `\b[2]`・`\b[-1]` は普通のバルーンへの指定（seriko の担当）。
    let mut state = state_with_boxes();
    show(&mut state, "0", 0);
    state.apply_cue(&select("0", "b"));
    let before = state.clone();
    let ((), events) = capture_checked(|| {
        state.apply_cue(&select("0", "2"));
        state.apply_cue(&select("0", "-1"));
    });
    assert_eq!(state, before);
    assert!(warns(&events).is_empty(), "{events:?}");
}

// ---- 表の行 11: 台詞の頭

#[test]
fn row11_clear_all_resets_every_scope_to_default_and_keeps_surface() {
    // 要件 4.10.
    let mut state = state_with_boxes();
    show(&mut state, "0", 0);
    state.apply_cue(&select("0", "b"));
    show(&mut state, "1", 1);
    state.apply_cue(&select("1", "a"));
    state.apply_cue(&cue("0", 0.0, CueCommand::ClearAll));
    assert_eq!(state.current_surface(&key("0")), Some(0));
    assert_eq!(state.destination(&key("0")), boxed("a"));
    assert_eq!(state.current_surface(&key("1")), Some(1));
    assert_eq!(state.destination(&key("1")), boxed("b"));
}

// ---- 宛先: 文字・改行・`\_l`・選択肢・`\f`・`\c` は今の行き先の場所へ

#[test]
fn content_cues_go_to_current_destination() {
    let mut state = state_with_boxes();
    show(&mut state, "0", 0);
    state.apply_cue(&select("0", "b"));
    state.apply_cue(&cue("0", 0.0, CueCommand::Text("あ".into())));
    state.apply_cue(&cue("0", 0.0, CueCommand::NewLine { ratio: 1.0 }));
    state.apply_cue(&cue(
        "0",
        0.0,
        CueCommand::Cursor {
            x: "1".into(),
            y: String::new(),
        },
    ));
    state.apply_cue(&cue(
        "0",
        0.0,
        CueCommand::Choice {
            id: "q".into(),
            text: "い".into(),
            references: vec![],
        },
    ));
    state.apply_cue(&cue(
        "0",
        0.0,
        CueCommand::command_carrier(FONT_TAG_CARRIER, vec!["bold".into(), "1".into()]),
    ));

    let b = PlaceKey {
        actor: key("0"),
        place: boxed("b"),
    };
    let at_b = state.place_state(&b).expect("箱 b の場所");
    assert_eq!(at_b.items().len(), 4, "文字・改行・カーソル・選択肢");
    assert_eq!(at_b.choices().len(), 1);
    assert!(at_b.current_look().bold, "\\f も行き先の場所へ");
    assert!(
        state.actor_state(&key("0")).is_none(),
        "普通のバルーンには何も届かない"
    );

    // `\c` の宛先も今の行き先の場所。
    state.apply_cue(&cue("0", 0.0, CueCommand::Clear));
    assert!(state.place_state(&b).unwrap().is_empty());
    assert_eq!(
        state.clear_count(&key("0")),
        0,
        "普通のバルーンの場所は消していない"
    );
}

// ---- スコープごと（要件 4.7）と箱の無い構成（要件 5.4）

#[test]
fn routes_are_per_scope() {
    let mut state = state_with_boxes();
    show(&mut state, "0", 0);
    show(&mut state, "1", 0);
    state.apply_cue(&select("0", "b"));
    assert_eq!(state.destination(&key("0")), boxed("b"));
    assert_eq!(
        state.destination(&key("1")),
        boxed("a"),
        "他のスコープは変えない"
    );
}

#[test]
fn empty_box_index_never_leaves_balloon() {
    let mut state = TextLayerState::default();
    let cues = [
        cue("0", 0.0, CueCommand::Text("あ".into())),
        select("0", "a"),
        cue("0", 0.0, CueCommand::ClearAll),
        cue("0", 0.0, CueCommand::Text("い".into())),
    ];
    for c in &cues {
        state.apply_cue(c);
        assert_eq!(state.destination(&key("0")), TextPlace::Balloon);
    }
    for outcome in [
        SurfaceKeyOutcome::Show(0),
        SurfaceKeyOutcome::Hide,
        SurfaceKeyOutcome::Unresolved,
    ] {
        state.route_surface(&key("0"), outcome);
        assert_eq!(state.destination(&key("0")), TextPlace::Balloon);
    }
    state.apply_cue(&cue("0", 0.0, CueCommand::Text("う".into())));
    assert_eq!(
        items_of(&state, "0"),
        &[TextItem::glyph("い"), TextItem::glyph("う")]
    );
}

// ---- 空回しの写し（装飾の持ち運びの場面は state_decoration_carry_tests.rs）

/// 箱 a は大きさ 20・スコープに従う、箱 b は大きさ 30・箱に閉じる、箱 c は大きさ 40・スコープに従う。
fn state_with_traits() -> TextLayerState {
    let mut state = state_with_boxes();
    let traits = [
        ("a", 20.0, areka_emo_compose::FontFollow::Scope),
        ("b", 30.0, areka_emo_compose::FontFollow::Balloon),
        ("c", 40.0, areka_emo_compose::FontFollow::Scope),
    ]
    .into_iter()
    .map(|(name, height, follow)| {
        let mut looks = crate::look::LookLayers::default();
        looks.default.height = height;
        looks.disable.height = height;
        (box_in(&INDEX, name), BoxTraits { looks, follow })
    })
    .collect();
    state.set_box_traits(traits);
    state
}

/// 空回しの写しだけが印を持ち、元の状態の印は偽のまま（本番の状態で印は立たない・`areka-P0-budoux-reveal-reflow` 要件 2.8）。
/// 無言であることの判定は `lookahead_tests.rs` の検査 5 が受け持つ。
#[test]
fn rehearsal_copy_marks_only_the_copy() {
    let mut state = state_with_traits();
    show(&mut state, "0", 0);
    let copy = state.rehearsal_copy();
    assert!(copy.rehearsal, "写しには印が立つ");
    assert!(!state.rehearsal, "元の状態の印は偽のまま");
    assert_eq!(
        TextLayerState {
            rehearsal: false,
            ..copy
        },
        state,
        "印のほかは元の状態と同じ"
    );
}

// ---- シェルの切替の戻り先（descript-window-keys 要件 3.2・3.5）

/// 新しいシェルの面の番号の集合と、スコープ "0" の候補の並び。
fn fallback(surface_ids: &[u32], candidates: &[u32]) -> SurfaceFallback {
    SurfaceFallback {
        surface_ids: surface_ids.iter().copied().collect(),
        candidates: BTreeMap::from([(key("0"), candidates.to_vec())]),
    }
}

#[test]
fn fall_back_keeps_surface_present_in_new_shell() {
    let mut state = state_with_boxes();
    show(&mut state, "0", 2);
    let before = state.clone();
    assert!(!state.fall_back_surfaces(&fallback(&[0, 2], &[0])));
    assert_eq!(state, before, "今の面が在れば何も替えない");
}

#[test]
fn fall_back_moves_to_descript_surface() {
    let mut state = state_with_boxes();
    show(&mut state, "0", 9);
    assert!(state.fall_back_surfaces(&fallback(&[0, 3], &[3, 0])));
    assert_eq!(state.current_surface(&key("0")), Some(3), "descript の面");
}

#[test]
fn fall_back_drops_to_builtin_surface() {
    let mut state = state_with_boxes();
    show(&mut state, "0", 9);
    assert!(state.fall_back_surfaces(&fallback(&[2, 3], &[7, 3])));
    assert_eq!(state.current_surface(&key("0")), Some(3), "組み込みの面");
}

#[test]
fn fall_back_hides_when_no_candidate_exists() {
    let mut state = state_with_boxes();
    show(&mut state, "0", 9);
    assert!(state.fall_back_surfaces(&fallback(&[2], &[7, 8])));
    assert_eq!(state.current_surface(&key("0")), None);
    assert_eq!(state.destination(&key("0")), TextPlace::Balloon);
}

#[test]
fn fall_back_to_surface_with_boxes_routes_to_its_default_box() {
    let mut state = state_with_boxes();
    show(&mut state, "0", 9);
    assert_eq!(state.destination(&key("0")), TextPlace::Balloon);
    assert!(state.fall_back_surfaces(&fallback(&[0, 1], &[1, 0])));
    assert_eq!(state.current_surface(&key("0")), Some(1));
    assert_eq!(state.destination(&key("0")), boxed("b"), "面 1 の既定の箱");
}

#[test]
fn fall_back_with_empty_surface_ids_changes_nothing() {
    // 面の番号の集合が空＝材料なし（seriko の fall_back_shell_surfaces と同じ）。
    let mut state = state_with_boxes();
    show(&mut state, "0", 9);
    let before = state.clone();
    assert!(!state.fall_back_surfaces(&SurfaceFallback::default()));
    assert_eq!(state, before);
}
