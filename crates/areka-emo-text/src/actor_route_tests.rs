//! 文字の層の実行時の `\s` の解決と場所ごとの表（task 7.1・要件 4.1・5.4・8.4・10.5）。
//!
//! 箱の名前は公開の構築口を持たないので、テストの中に持つ surfaces.txt の文面を
//! `parse_boxes` → `fold_boxes` で畳んで箱の表を作る（検体は読まない）。COM は使わない。

use std::collections::BTreeMap;

use areka_emo_compose::{BoxName, EmoWorld, fold_boxes};
use areka_parsers::shell::{parse, parse_boxes};
use areka_sakura::contract::{ActorKey, CueCommand};

use super::test_support::{choice_cue, cue};
use super::{SurfaceKeyResolver, TextLayerRuntime};
use crate::place::{PlaceKey, TextPlace};
use crate::state::{ChoiceSpan, SpanKind, SurfaceKeyOutcome, TextItem, TextLayerConfig};

/// サーフェス 0 は箱 a・b（先頭は a）、1 は箱 b だけ、2 は箱なし（表に載らない）。
const SHELL: &str = "\
balloon.a
{
size,100,50
}
balloon.b
{
size,80,40
}
surface0
{
element1,balloon,a,0,0
element2,balloon,b,0,60
}
surface1
{
element1,balloon,b,0,0
}
";

fn index() -> BTreeMap<u32, Vec<BoxName>> {
    let world = EmoWorld::build(&parse(SHELL));
    let (layout, report) = fold_boxes(&parse_boxes(SHELL), &BTreeMap::new(), &world);
    assert_eq!(report.issues, vec![], "文面は誤りを持たない");
    (0..=2)
        .map(|id| {
            let names = layout
                .placements(id)
                .iter()
                .map(|p| p.name.clone())
                .collect();
            (id, names)
        })
        .collect()
}

fn boxed(name: &str) -> TextPlace {
    let found = index()
        .into_values()
        .flatten()
        .find(|n| n.as_str() == name)
        .expect("文面にある箱の名前");
    TextPlace::Box(found)
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

fn emote(actor: &str, key: &str) -> areka_sakura::contract::TalkCue {
    cue(actor, 0.0, CueCommand::Emote { key: key.into() })
}

/// 箱の表を入れた（閉包の有無は呼び手が決める）ランタイム。
fn runtime_with_boxes() -> TextLayerRuntime {
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    rt.state.set_box_index(index());
    rt
}

fn items_at(rt: &TextLayerRuntime, actor: &str, place: TextPlace) -> Vec<TextItem> {
    rt.state()
        .place_state(&PlaceKey {
            actor: ActorKey::from(actor),
            place,
        })
        .map_or_else(Vec::new, |s| s.items().to_vec())
}

/// 閉包が無いあいだは、箱の表があっても `\s` を読まず、行き先は普通のバルーンのまま（要件 5.4）。
#[test]
fn without_resolver_emote_never_leaves_balloon() {
    let mut rt = runtime_with_boxes();
    let actor = ActorKey::from("0");
    for key in ["0", "1", "-1", "2", "x"] {
        rt.apply_cue(&emote("0", key));
        assert_eq!(
            rt.state().destination(&actor),
            TextPlace::Balloon,
            "\\s[{key}]"
        );
        assert_eq!(rt.state().current_surface(&actor), None, "番号も持たない");
    }
    rt.apply_cue(&cue("0", 0.0, CueCommand::Text("あ".into())));
    assert_eq!(
        items_at(&rt, "0", TextPlace::Balloon),
        vec![TextItem::glyph("あ")]
    );
}

/// 偽の閉包と箱の表を入れると、`\s` の後の行き先はそのサーフェスの先頭の箱になる（要件 4.1）。
#[test]
fn with_resolver_emote_routes_to_first_box_of_surface() {
    let mut rt = runtime_with_boxes();
    rt.set_surface_resolver(fake_resolver());
    let actor = ActorKey::from("0");

    rt.apply_cue(&emote("0", "0"));
    assert_eq!(
        rt.state().destination(&actor),
        boxed("a"),
        "surface0 の先頭は a"
    );
    assert_eq!(rt.state().current_surface(&actor), Some(0));
    rt.apply_cue(&cue("0", 0.0, CueCommand::Text("あ".into())));
    assert_eq!(items_at(&rt, "0", boxed("a")), vec![TextItem::glyph("あ")]);
    assert_eq!(
        items_at(&rt, "0", TextPlace::Balloon),
        vec![],
        "普通のバルーンには入らない"
    );

    rt.apply_cue(&emote("0", "1"));
    assert_eq!(
        rt.state().destination(&actor),
        boxed("b"),
        "surface1 の先頭は b"
    );

    rt.apply_cue(&emote("0", "zzz"));
    assert_eq!(
        rt.state().destination(&actor),
        boxed("b"),
        "解決できない鍵は何も変えない"
    );
    assert_eq!(rt.state().current_surface(&actor), Some(1));

    rt.apply_cue(&emote("0", "2"));
    assert_eq!(
        rt.state().destination(&actor),
        TextPlace::Balloon,
        "箱の無いサーフェス"
    );

    rt.apply_cue(&emote("0", "0"));
    rt.apply_cue(&emote("0", "-1"));
    assert_eq!(rt.state().destination(&actor), TextPlace::Balloon, "非表示");
    assert_eq!(rt.state().current_surface(&actor), None);

    // 別のスコープは動かない。
    assert_eq!(
        rt.state().destination(&ActorKey::from("1")),
        TextPlace::Balloon
    );
}

/// 選択肢が箱の場所にだけあっても、スコープの「選択肢があるか」は真（要件 8.4）。
/// `\c` でその箱を消すと偽へ戻る。
#[test]
fn choice_active_sees_every_place_of_the_scope() {
    let mut rt = runtime_with_boxes();
    rt.set_surface_resolver(fake_resolver());
    let actor = ActorKey::from("0");

    rt.apply_cue(&emote("0", "0"));
    rt.apply_cue(&choice_cue("0", 0.0, "ID", "はい", &[]));
    assert!(
        rt.state()
            .actor_state(&actor)
            .is_none_or(|s| s.choices().is_empty()),
        "普通のバルーンの場所には選択肢が無い"
    );
    assert!(rt.choice_active(&actor), "箱の場所の選択肢も数える");
    assert!(!rt.choice_active(&ActorKey::from("1")), "別のスコープは偽");

    rt.apply_cue(&cue("0", 0.0, CueCommand::Clear));
    assert!(!rt.choice_active(&actor), "\\c で箱を消すと偽");
}

/// アンカーの範囲だけがあるスコープは、「選択肢があるか」は偽のまま（バルーンの時間切れを
/// 止めるのは選択肢だけ）、「押せる範囲があるか」は真。範囲が箱の場所にあっても同じ。
/// 選択肢が加わると両方が真になる。
#[test]
fn anchor_only_scope_is_hit_active_but_not_choice_active() {
    let mut rt = runtime_with_boxes();
    let actor = ActorKey::from("0");
    assert!(!rt.hit_active(&actor), "範囲が無ければ偽");

    let box_a = PlaceKey {
        actor: actor.clone(),
        place: boxed("a"),
    };
    rt.state.push_span_for_test(
        &box_a,
        ChoiceSpan {
            kind: SpanKind::Anchor,
            ordinal: 0,
            id: "x".into(),
            label: "あ".into(),
            references: vec![],
            glyph_range: 0..1,
        },
    );
    assert!(!rt.choice_active(&actor), "アンカーは選択肢に数えない");
    assert!(rt.hit_active(&actor), "アンカーも押せる範囲");
    assert!(!rt.hit_active(&ActorKey::from("1")), "別のスコープは偽");

    rt.apply_cue(&choice_cue("0", 0.0, "ID", "はい", &[]));
    assert!(rt.choice_active(&actor), "選択肢が加われば真");
    assert!(rt.hit_active(&actor));
}

/// `\c` が強調の記録を消すのは今の行き先の場所だけ（普通のバルーンの強調は残る）。
#[test]
fn clear_drops_hover_of_the_destination_place_only() {
    let mut rt = runtime_with_boxes();
    rt.set_surface_resolver(fake_resolver());
    let actor = ActorKey::from("0");
    rt.inject_choice_hover(&actor, Some(0));

    rt.apply_cue(&emote("0", "0"));
    rt.apply_cue(&cue("0", 0.0, CueCommand::Clear));
    assert_eq!(
        rt.choice_hover.get(&PlaceKey::balloon(&actor)),
        Some(&Some(0)),
        "箱を消しても普通のバルーンの強調は残る"
    );

    rt.apply_cue(&emote("0", "2"));
    rt.apply_cue(&cue("0", 0.0, CueCommand::Clear));
    assert_eq!(
        rt.choice_hover.get(&PlaceKey::balloon(&actor)),
        None,
        "普通のバルーンが行き先なら消える"
    );
}
