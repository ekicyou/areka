//! `TalkLookahead` の検査（design.md「純粋な部品の検査（`lookahead_tests.rs`）」）。
//!
//! 本番の側は `note_cue` → `advance_state` の順に合図を流す（実行時の `apply_cue` と同じ並び）。
//! 区間の全文は私有の欄を直接読んで、場所 × 区間の番号の表をまるごと比べる。

use std::collections::BTreeMap;

use areka_emo_compose::{EmoWorld, fold_boxes};
use areka_parsers::shell::{parse, parse_boxes};
use areka_sakura::contract::{ActorKey, CueCommand, TalkCue};

use super::*;
use crate::look::LookLayers;
use crate::place::TextPlace;
use crate::state::TextItem;

/// 箱の構成: サーフェス 0 は element1=a・element2=b（既定は a）。
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
";

/// 箱の表を入れた状態（箱の名前は公開の構築口を持たないので文面を畳んで取り出す）。
fn boxed_state() -> TextLayerState {
    let world = EmoWorld::build(&parse(SHELL));
    let (layout, report) = fold_boxes(&parse_boxes(SHELL), &BTreeMap::new(), &world);
    assert_eq!(report.issues, vec![], "検体の文面は誤りを持たない");
    let names = layout
        .placements(0)
        .iter()
        .map(|p| p.name.clone())
        .collect();
    let mut state = TextLayerState::default();
    state.set_box_index(BTreeMap::from([(0, names)]));
    state
}

/// `\s[番号]` の鍵を番号のまま解く閉包。
fn resolve(key: &str) -> SurfaceKeyOutcome {
    key.parse()
        .map_or(SurfaceKeyOutcome::Unresolved, SurfaceKeyOutcome::Show)
}

fn cue(actor: &str, at: f64, command: CueCommand) -> TalkCue {
    TalkCue {
        at,
        actor: ActorKey::from(actor),
        command,
        duration: 0.0,
    }
}

fn text(actor: &str, at: f64, s: &str) -> TalkCue {
    cue(actor, at, CueCommand::Text(s.to_owned()))
}

fn place(actor: &str, place: TextPlace) -> PlaceKey {
    PlaceKey {
        actor: ActorKey::from(actor),
        place,
    }
}

/// 箱の場所の鍵（名前は表から引く）。
fn box_place(state: &TextLayerState, actor: &str, name: &str) -> PlaceKey {
    let found = state
        .places()
        .find_map(|(key, _)| match &key.place {
            TextPlace::Box(n) if n.as_str() == name => Some(key.place.clone()),
            _ => None,
        })
        .expect("本番の状態にその箱の場所がある");
    place(actor, found)
}

/// 本番の適用: 行き先を求めて数え、`advance_state` で進める（実行時の `apply_cue` と同じ並び）。
fn deliver(
    look: &mut TalkLookahead,
    state: &mut TextLayerState,
    resolve: Option<&dyn Fn(&str) -> SurfaceKeyOutcome>,
    cues: &[TalkCue],
) {
    for c in cues {
        let dest = PlaceKey {
            actor: c.actor.clone(),
            place: state.destination(&c.actor),
        };
        look.note_cue(c, &dest);
        advance_state(state, resolve, c);
    }
}

fn content(state: &TextLayerState, key: &PlaceKey) -> ActorTextState {
    state.place_state(key).expect("場所に状態がある").clone()
}

/// 1. 行き先が本番と同じ（要件 2.2）: `\s`・名前の `\b` で箱へ行く列を、空回しと本番で流す。
/// 空回しの控えた全文（場所 × 番号）が、本番の出終わった状態の字のある場所と過不足なく同じ。
#[test]
fn rehearsal_destinations_match_production() {
    let mut state = boxed_state();
    let resolve: &dyn Fn(&str) -> SurfaceKeyOutcome = &resolve;
    // 前のトークの字（頭の全消去で消える）。
    advance_state(&mut state, Some(resolve), &text("0", 0.0, "まえ"));
    let talk = [
        cue("0", 0.0, CueCommand::ClearAll),
        cue("0", 0.0, CueCommand::Emote { key: "0".into() }),
        text("0", 0.1, "あい"),
        cue("0", 0.2, CueCommand::BalloonSurface { key: "b".into() }),
        text("0", 0.3, "うえ"),
        cue("0", 0.4, CueCommand::NewLine { ratio: 1.0 }),
        text("0", 0.5, "お"),
        text("1", 0.6, "かき"),
    ];
    let mut look = TalkLookahead::default();
    look.install(&state, Some(resolve), &talk);
    deliver(&mut look, &mut state, Some(resolve), &talk);

    let a = box_place(&state, "0", "a");
    let b = box_place(&state, "0", "b");
    let sakura_balloon = place("0", TextPlace::Balloon);
    let kero = place("1", TextPlace::Balloon);
    assert!(
        state
            .place_state(&sakura_balloon)
            .is_some_and(ActorTextState::is_empty),
        "前提: 普通のバルーンの前の字は頭の全消去で消え、字は箱へ行った"
    );
    // 番号は頭の全消去 1 回＝どの場所も 1。
    let expected = BTreeMap::from([
        ((a.clone(), 1), content(&state, &a)),
        ((b.clone(), 1), content(&state, &b)),
        ((kero.clone(), 1), content(&state, &kero)),
    ]);
    assert_eq!(look.sections, expected);
}

/// 2. 区間の番号（要件 4.1）: 頭の全消去と途中の `\c` で全文が分かれる。先渡しと頭の全消去の
/// 間に場所が作られても（バルーンの装着）、番号はずれない。
#[test]
fn section_numbers_split_on_clears_and_survive_place_creation() {
    let actor = ActorKey::from("0");
    let balloon = PlaceKey::balloon(&actor);
    let talk = [
        cue("0", 0.0, CueCommand::ClearAll),
        text("0", 0.1, "あ"),
        cue("0", 0.2, CueCommand::Clear),
        text("0", 0.3, "い"),
    ];
    // 空回しの出発点には場所が無い（全消去は状態の消去の回数を進めない）。
    let mut state = TextLayerState::default();
    let mut look = TalkLookahead::default();
    look.install(&state, None, &talk);
    assert_eq!(look.sections.len(), 2, "区間は 2 つ: {:?}", look.sections);

    // 先渡しの後、頭の全消去の前に装着で場所ができる（状態の消去の回数はここから 1 つずれる）。
    state.set_look_layers(&actor, LookLayers::default());
    deliver(&mut look, &mut state, None, &talk[..2]);
    assert_eq!(
        look.number(&balloon),
        1,
        "頭の全消去の後の番号（状態の数えは {}）",
        state.clear_count(&actor)
    );
    assert_eq!(
        look.sections.get(&(balloon.clone(), 1)),
        Some(&content(&state, &balloon))
    );

    deliver(&mut look, &mut state, None, &talk[2..]);
    assert_eq!(look.number(&balloon), 2, "途中の \\c の後の番号");
    assert_eq!(
        look.sections.get(&(balloon.clone(), 2)),
        Some(&content(&state, &balloon))
    );
    let before_clear = look
        .sections
        .get(&(balloon.clone(), 1))
        .expect("消去の前の区間");
    assert_eq!(
        before_clear.items(),
        &[TextItem::glyph("あ")],
        "消去の前の区間は消去の前の字だけ"
    );
}

/// 3. 持ち越し（要件 2.6）: 何も足さない区間は残り、足した区間は置き換わり、
/// 消された区間は次の `install` で消える。
#[test]
fn sections_carry_over_until_replaced_or_cleared() {
    let sakura = place("0", TextPlace::Balloon);
    let kero = place("1", TextPlace::Balloon);
    let mut state = TextLayerState::default();
    let mut look = TalkLookahead::default();

    let first = [
        cue("0", 0.0, CueCommand::ClearAll),
        text("0", 0.1, "あい"),
        text("1", 0.2, "かき"),
    ];
    look.install(&state, None, &first);
    deliver(&mut look, &mut state, None, &first);
    let kero_first = content(&state, &kero);
    assert_eq!(
        look.sections,
        BTreeMap::from([
            ((sakura.clone(), 1), content(&state, &sakura)),
            ((kero.clone(), 1), kero_first.clone()),
        ])
    );

    // 2 本目は頭の全消去なしで本体側に足し、相方側を `\c` で消す。
    let second = [text("0", 1.0, "さし"), cue("1", 1.1, CueCommand::Clear)];
    look.install(&state, None, &second);
    deliver(&mut look, &mut state, None, &second);
    assert_eq!(
        look.sections,
        BTreeMap::from([
            // 足した区間は空回しの側（前の字＋足した字）に置き換わる。
            ((sakura.clone(), 0), content(&state, &sakura)),
            // 何も足さない区間は前のものが残る（消されるのは後の合図）。
            ((kero.clone(), 0), kero_first),
        ])
    );

    // 相方側は消されて区間が進んだ＝次の install で前の区間の全文は消える。
    look.install(&state, None, &[]);
    assert_eq!(
        look.sections,
        BTreeMap::from([((sakura.clone(), 0), content(&state, &sakura))])
    );
}
