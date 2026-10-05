//! `TalkLookahead` の検査（design.md「純粋な部品の検査（`lookahead_tests.rs`）」）。
//!
//! 本番の側は `note_cue` → `advance_state` の順に合図を流す（実行時の `apply_cue` と同じ並び）。
//! 区間の全文は私有の欄を直接読んで、場所 × 区間の番号の表をまるごと比べる。
//! 記録の件数は `log-capture-kit` で数える（捕捉は呼んだスレッドだけ＝検査は全部このスレッドで回す）。

use std::collections::BTreeMap;

use areka_emo_compose::{EmoWorld, fold_boxes};
use areka_parsers::shell::{parse, parse_boxes};
use areka_sakura::contract::{ActorKey, CueCommand, FONT_TAG_CARRIER, TalkCue};
use log_capture_kit::{CapturedEvent, capture};

use super::*;
use crate::look::LookLayers;
use crate::place::TextPlace;
use crate::segment::segment_plan;
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

/// 検体の箱の表を状態へ入れる（箱の名前は公開の構築口を持たないので文面を畳んで取り出す）。
fn set_boxes(state: &mut TextLayerState) {
    let world = EmoWorld::build(&parse(SHELL));
    let (layout, report) = fold_boxes(&parse_boxes(SHELL), &BTreeMap::new(), &world);
    assert_eq!(report.issues, vec![], "検体の文面は誤りを持たない");
    let names = layout
        .placements(0)
        .iter()
        .map(|p| p.name.clone())
        .collect();
    state.set_box_index(BTreeMap::from([(0, names)]));
}

/// 箱の表を入れた状態。
fn boxed_state() -> TextLayerState {
    let mut state = TextLayerState::default();
    set_boxes(&mut state);
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

/// `\f[…]` の合図。
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

/// 区間の全文の表を、場所 × 番号 → 内容の表として読む（区切りの覚えは見ない）。
fn contents(look: &TalkLookahead) -> BTreeMap<(PlaceKey, u64), ActorTextState> {
    look.sections
        .iter()
        .map(|(key, section)| (key.clone(), section.content.clone()))
        .collect()
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
    assert_eq!(contents(&look), expected);
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
        contents(&look).get(&(balloon.clone(), 1)),
        Some(&content(&state, &balloon))
    );

    deliver(&mut look, &mut state, None, &talk[2..]);
    assert_eq!(look.number(&balloon), 2, "途中の \\c の後の番号");
    assert_eq!(
        contents(&look).get(&(balloon.clone(), 2)),
        Some(&content(&state, &balloon))
    );
    let before_clear = &look
        .sections
        .get(&(balloon.clone(), 1))
        .expect("消去の前の区間")
        .content;
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
        contents(&look),
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
        contents(&look),
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
        contents(&look),
        BTreeMap::from([((sakura.clone(), 0), content(&state, &sakura))])
    );
}

/// 捕捉した記録のうち `warn` だけ。
fn warns(events: &[CapturedEvent]) -> Vec<&CapturedEvent> {
    events
        .iter()
        .filter(|e| e.level == tracing::Level::WARN)
        .collect()
}

/// 先渡しの無い状態へ `cues` を本番と同じ規則で流した、本体側の普通のバルーンの内容。
fn arrive(cues: &[TalkCue]) -> ActorTextState {
    let mut state = TextLayerState::default();
    for c in cues {
        advance_state(&mut state, None, c);
    }
    state
        .place_state(&place("0", TextPlace::Balloon))
        .cloned()
        .unwrap_or_default()
}

/// `basis` が全文を返したか。
fn is_full(basis: &Basis<'_>) -> bool {
    matches!(basis, Basis::Full { .. })
}

/// 先渡し `talk` を空回しし、届いた内容 `arrived` で本体側の `basis` を 2 度聞く。
/// 返すのは 2 度の答え（全文か）と、2 度の間に出た `warn`。
fn judge(talk: &[TalkCue], arrived: &ActorTextState) -> ([bool; 2], Vec<CapturedEvent>) {
    let sakura = place("0", TextPlace::Balloon);
    let mut look = TalkLookahead::default();
    look.install(&TextLayerState::default(), None, talk);
    let (answers, events) = capture(|| {
        [
            is_full(&look.basis(&sakura, arrived)),
            is_full(&look.basis(&sakura, arrived)),
        ]
    });
    (answers, warns(&events).into_iter().cloned().collect())
}

/// 4. 見分け（要件 2.7）: 字の列・字ごとの装飾番号・装飾の表の 3 つとも届いたぶんが全文の
/// 先頭と一致するときだけ全文。それぞれ 1 つだけが食い違う入力で「届いた字で」になり、
/// warn は（場所, 区間）につき 1 度。字の無い場所は warn しない（印も使わない）。
#[test]
fn basis_requires_all_three_prefixes_and_warns_once_per_section() {
    let sakura = place("0", TextPlace::Balloon);

    // 一致（対照）: 全文は「あ」→ 太字 →「い」、届いたのは「あ」まで。区切りは初めて要ったときに 1 度。
    let talk = [
        text("0", 0.0, "あ"),
        font("0", &["bold", "1"]),
        text("0", 0.1, "い"),
    ];
    let mut look = TalkLookahead::default();
    look.install(&TextLayerState::default(), None, &talk);
    let full = look.sections[&(sakura.clone(), 0)].content.clone();
    assert!(
        look.sections[&(sakura.clone(), 0)].plan.is_none(),
        "区切りは空回しでは計算しない"
    );
    let (answer, events) = capture(|| match look.basis(&sakura, &arrive(&talk[..1])) {
        Basis::Full { content, plan } => Some((content.clone(), plan.clone())),
        Basis::Arrived => None,
    });
    assert_eq!(
        answer,
        Some((full.clone(), segment_plan(full.items()))),
        "届いたぶんが先頭と一致すれば全文と全文の区切り"
    );
    assert_eq!(warns(&events).len(), 0, "一致では warn しない");
    assert_eq!(
        look.sections[&(sakura.clone(), 0)].plan,
        Some(segment_plan(full.items())),
        "区切りは全文に対して覚える"
    );
    assert!(
        is_full(&look.basis(&sakura, &full)),
        "出終わった（全文と同じ）内容も一致"
    );

    // 食い違い 3 種: それぞれ他の 2 つは先頭と一致していることを確かめてから聞く。
    let cases: [(&str, Vec<TalkCue>, Vec<TalkCue>, usize, usize); 3] = [
        (
            "字の列",
            vec![text("0", 0.0, "あい")],
            vec![text("0", 0.0, "かき")],
            0,
            2,
        ),
        (
            "字ごとの装飾番号",
            talk.to_vec(),
            vec![font("0", &["bold", "1"]), text("0", 0.0, "あ")],
            1,
            1,
        ),
        (
            "装飾の表",
            vec![font("0", &["bold", "1"]), text("0", 0.0, "あい")],
            vec![font("0", &["italic", "1"]), text("0", 0.0, "あ")],
            2,
            1,
        ),
    ];
    for (what, talk, arrived_cues, broken, arrived_glyphs) in cases {
        let arrived = arrive(&arrived_cues);
        let mut rehearsed = TalkLookahead::default();
        rehearsed.install(&TextLayerState::default(), None, &talk);
        let full = &rehearsed.sections[&(sakura.clone(), 0)].content;
        let prefixes = [
            full.items().starts_with(arrived.items()),
            full.glyph_styles().starts_with(arrived.glyph_styles()),
            full.styles().starts_with(arrived.styles()),
        ];
        let mut expected = [true; 3];
        expected[broken] = false;
        assert_eq!(
            prefixes, expected,
            "{what}: 前提＝食い違うのはこの 1 つだけ"
        );
        let full_glyphs = full.glyph_styles().len();

        let (answers, warned) = judge(&talk, &arrived);
        assert_eq!(answers, [false, false], "{what}: 届いた字で");
        assert_eq!(warned.len(), 1, "{what}: 2 度聞いても warn は 1 度");
        let fields = warned[0].fields_map();
        assert_eq!(
            warned[0].field_str("reason"),
            Some("届いた字が先渡しと食い違う")
        );
        assert_eq!(fields.get("actor"), Some(&"0"));
        assert_eq!(fields.get("place"), Some(&"Balloon"));
        assert_eq!(
            fields.get("arrived_glyphs"),
            Some(&arrived_glyphs.to_string().as_str())
        );
        assert_eq!(
            fields.get("full_glyphs"),
            Some(&full_glyphs.to_string().as_str())
        );
    }

    // 先渡しが無い・区間ごと・場所ごと・字の無い場所。
    let kero = place("1", TextPlace::Balloon);
    let mut look = TalkLookahead::default();
    let glyph = arrive(&[text("0", 0.0, "か")]);
    let no_glyph = arrive(&[cue("0", 0.0, CueCommand::NewLine { ratio: 1.0 })]);
    assert!(no_glyph.glyph_styles().is_empty() && !no_glyph.items().is_empty());
    let (answers, events) = capture(|| {
        let mut answers = Vec::new();
        // 字の無い内容は warn しない・印も使わない。
        answers.push(is_full(&look.basis(&sakura, &no_glyph)));
        answers.push(is_full(&look.basis(&sakura, &ActorTextState::default())));
        answers.push(is_full(&look.basis(&sakura, &glyph)));
        answers.push(is_full(&look.basis(&sakura, &glyph)));
        // 別の場所は別に 1 度。
        answers.push(is_full(&look.basis(&kero, &glyph)));
        // `\c` で区間が進めば、同じ場所でも次の区間で 1 度。
        look.note_cue(&cue("0", 0.0, CueCommand::Clear), &sakura);
        answers.push(is_full(&look.basis(&sakura, &glyph)));
        answers.push(is_full(&look.basis(&sakura, &glyph)));
        answers
    });
    assert_eq!(answers, vec![false; 7], "全文が無ければ届いた字で");
    let warned: Vec<(Option<&str>, Option<&str>, Option<&str>)> = warns(&events)
        .iter()
        .map(|e| {
            (
                e.field("actor"),
                e.field_str("reason"),
                e.field("full_glyphs"),
            )
        })
        .collect();
    let none = (Some("0"), Some("先渡しが無い"), Some("0"));
    assert_eq!(
        warned,
        vec![none, (Some("1"), Some("先渡しが無い"), Some("0")), none],
        "（場所, 区間）につき 1 度・字の無い内容では出さない"
    );
}

/// 4（続き）. `install` の持ち越しと warn 済みの印: 何も足さない区間は全文と印をそのまま
/// 持ち越す（同じ食い違いを 2 度記録しない）。空回しが番号 0 の全文を求め直した場所は、
/// 全文と同じく印も空回しの側（印なし）を採る＝新しい全文との食い違いは改めて 1 度記録する。
#[test]
fn install_carries_the_warned_mark_only_with_the_carried_section() {
    let sakura = place("0", TextPlace::Balloon);
    let mut state = TextLayerState::default();
    let mut look = TalkLookahead::default();
    let first = [text("0", 0.0, "あ")];
    look.install(&state, None, &first);
    // 先渡しに無い字が届く（食い違い）。
    deliver(&mut look, &mut state, None, &[text("0", 0.0, "か")]);

    let count = |look: &mut TalkLookahead, state: &TextLayerState| {
        let arrived = content(state, &sakura);
        let (full, events) = capture(|| is_full(&look.basis(&sakura, &arrived)));
        assert!(!full, "前提: 食い違っている");
        warns(&events).len()
    };
    assert_eq!(count(&mut look, &state), 1, "初回の食い違い");
    assert_eq!(count(&mut look, &state), 0, "同じ区間では 2 度目を出さない");

    // 何も足さない先渡し: 全文（「あ」）と印を持ち越す。
    look.install(&state, None, &[]);
    assert_eq!(
        contents(&look),
        BTreeMap::from([((sakura.clone(), 0), arrive(&first))])
    );
    assert_eq!(count(&mut look, &state), 0, "持ち越した区間の印は残る");

    // 足す先渡し: 今の状態（「か」）から求め直す。そこへまた先渡しに無い字が届く。
    look.install(&state, None, &[text("0", 1.0, "う")]);
    deliver(&mut look, &mut state, None, &[text("0", 1.0, "さ")]);
    assert_eq!(
        count(&mut look, &state),
        1,
        "求め直した全文との食い違いは改めて 1 度"
    );
    assert_eq!(count(&mut look, &state), 0);
}

/// 5. 空回しは無言（要件 2.8）: 状態の層が warn を出す 4 つ（選択肢の字が空・箱の無い
/// `\b[名前]`・適用できない `\f`・上下付きのまま字を足す）を含む列で、`install` の間の warn が
/// 0 件。続く本番の適用の warn は、先渡し無しで同じ列を流したときと同じ。
#[test]
fn rehearsal_is_silent_and_production_warns_are_unchanged() {
    let talk = [
        cue(
            "0",
            0.0,
            CueCommand::Choice {
                id: "OnPick".to_owned(),
                text: String::new(),
                references: Vec::new(),
            },
        ),
        cue(
            "0",
            0.1,
            CueCommand::BalloonSurface {
                key: "nobox".to_owned(),
            },
        ),
        font("0", &["bold", "yes"]),
        font("0", &["sub", "1"]),
        text("0", 0.2, "あ"),
    ];
    let messages = |events: &[CapturedEvent]| -> Vec<String> {
        warns(events)
            .iter()
            .map(|e| e.message().to_owned())
            .collect()
    };

    // 先渡し無し（対照）: 4 つの発行点がどれも出る＝捕捉が空振りしていない
    // （`\f[sub,1]` は指定そのものも「表示は変えない」と記録するので、件数は 5）。
    let ((), without) = capture(|| {
        let mut state = TextLayerState::default();
        for c in &talk {
            advance_state(&mut state, None, c);
        }
    });
    for needle in [
        "Choice cue の text が空",
        "\\b[名前] の箱が今のサーフェスに無い",
        "\\f の指定を適用できない",
        "上下付きが有効なまま文字を追記した",
    ] {
        assert!(
            messages(&without).iter().any(|m| m.contains(needle)),
            "対照: 「{needle}」が出る {:?}",
            messages(&without)
        );
    }

    let mut state = TextLayerState::default();
    let mut look = TalkLookahead::default();
    let ((), rehearsal) = capture(|| look.install(&state, None, &talk));
    assert_eq!(
        messages(&rehearsal),
        Vec::<String>::new(),
        "空回しの間は warn しない"
    );
    assert_eq!(
        contents(&look).len(),
        1,
        "前提: 空回しは列を流しきって全文を控えた"
    );

    let ((), production) = capture(|| deliver(&mut look, &mut state, None, &talk));
    assert_eq!(
        messages(&production),
        messages(&without),
        "本番の warn は先渡し無しのときと同じ"
    );
}

/// 4（続き）. 箱の束の差し替え（`forget_boxes`）: 箱の場所の全文・`\c` の数え・warn 済みの印を
/// 捨て、普通のバルーンの全文と数えは残す。印は番号の変わらない箱（b）で見る。
#[test]
fn forget_boxes_drops_box_sections_and_counts_only() {
    let mut state = boxed_state();
    let resolve: &dyn Fn(&str) -> SurfaceKeyOutcome = &resolve;
    let talk = [
        cue("0", 0.0, CueCommand::ClearAll),
        cue("0", 0.0, CueCommand::Emote { key: "0".into() }),
        text("0", 0.1, "あ"),
        cue("0", 0.2, CueCommand::Clear),
        text("0", 0.3, "い"),
        cue("0", 0.4, CueCommand::BalloonSurface { key: "b".into() }),
        text("0", 0.5, "う"),
        text("1", 0.6, "か"),
        cue("1", 0.7, CueCommand::Clear),
        text("1", 0.8, "き"),
    ];
    let mut look = TalkLookahead::default();
    look.install(&state, Some(resolve), &talk);
    deliver(&mut look, &mut state, Some(resolve), &talk);
    let a = box_place(&state, "0", "a");
    let b = box_place(&state, "0", "b");
    let kero = place("1", TextPlace::Balloon);
    let arrived = content(&state, &a);
    assert!(
        is_full(&look.basis(&a, &arrived)),
        "前提: 箱の場所に全文がある"
    );
    assert_eq!(
        [look.number(&a), look.number(&b), look.number(&kero)],
        [2, 1, 2],
        "前提: 頭の全消去と途中の \\c"
    );
    // 箱 b に warn 済みの印を付けておく（食い違う字を聞く）。
    let stray = arrive(&[text("0", 0.0, "さ")]);
    let (_, marked) = capture(|| is_full(&look.basis(&b, &stray)));
    assert_eq!(warns(&marked).len(), 1, "前提: 箱の場所の印");

    let balloons: BTreeMap<_, _> = contents(&look)
        .into_iter()
        .filter(|((key, _), _)| key.place == TextPlace::Balloon)
        .collect();
    assert_eq!(
        balloons.keys().collect::<Vec<_>>(),
        vec![&(kero.clone(), 1), &(kero.clone(), 2)],
        "前提: 普通のバルーンの全文は相方側の 2 区間"
    );

    look.forget_boxes();
    assert_eq!(contents(&look), balloons, "箱の場所の全文だけが消える");
    assert_eq!(
        [look.number(&a), look.number(&b), look.number(&kero)],
        [1, 1, 2],
        "箱の場所の \\c の数えだけが消える"
    );
    let (full, events) = capture(|| {
        [
            is_full(&look.basis(&a, &arrived)),
            is_full(&look.basis(&b, &stray)),
        ]
    });
    assert_eq!(full, [false, false], "箱の場所は全文が無い");
    let warned: Vec<Option<&str>> = warns(&events)
        .iter()
        .map(|e| e.field_str("reason"))
        .collect();
    assert_eq!(
        warned,
        vec![Some("先渡しが無い"); 2],
        "箱 b の印も消えた＝改めて 1 度記録する"
    );
}

/// バルーンの定義の見た目（ukadoc の既定と違う）。
fn attached_layers() -> LookLayers {
    LookLayers::from_balloon(
        vec!["Meiryo".to_owned()],
        20.0,
        (255, 0, 0),
        (255, 255, 255),
        (0, 0, 255),
        &[],
        &[],
    )
}

/// 合図を 1 つずつ本番で流し、流すたびに場所 `key` の `basis` を聞く（場所に状態がある間だけ）。
/// 返すのは答え（全文か）の列と、その間に出た `warn` の件数。
fn deliver_judging(
    look: &mut TalkLookahead,
    state: &mut TextLayerState,
    resolve: Option<&dyn Fn(&str) -> SurfaceKeyOutcome>,
    cues: &[TalkCue],
    key: &PlaceKey,
) -> (Vec<bool>, usize) {
    let (answers, events) = capture(|| {
        let mut answers = Vec::new();
        for c in cues {
            deliver(look, state, resolve, std::slice::from_ref(c));
            if let Some(arrived) = state.place_state(key).cloned() {
                answers.push(is_full(&look.basis(key, &arrived)));
            }
        }
        answers
    });
    (answers, warns(&events).len())
}

/// 6(a). やり直し（設計の討議 1）: 先渡しの後にバルーンの定義の見た目が差し込まれても、
/// やり直せば `\f` を含む台本は最後まで全文で配置され、warn は 0 件。やり直さない対照は
/// 装飾付きの字が届いた所で「届いた字で」になり、warn は 1 件。
#[test]
fn reinstall_after_attaching_look_keeps_full_basis() {
    let actor = ActorKey::from("0");
    let balloon = PlaceKey::balloon(&actor);
    let talk = [
        cue("0", 0.0, CueCommand::ClearAll),
        text("0", 0.1, "あ"),
        font("0", &["bold", "1"]),
        text("0", 0.2, "い"),
    ];
    let run = |reinstall: bool| {
        let mut state = TextLayerState::default();
        let mut look = TalkLookahead::default();
        look.install(&state, None, &talk);
        state.set_look_layers(&actor, attached_layers());
        let ((), events) = capture(|| {
            if reinstall {
                look.reinstall(&state, None);
            }
        });
        assert_eq!(warns(&events).len(), 0, "やり直しの間は warn しない");
        deliver_judging(&mut look, &mut state, None, &talk, &balloon)
    };
    assert_eq!(run(true), (vec![true; 4], 0), "やり直せば最後まで全文");
    assert_eq!(
        run(false),
        (vec![true, true, true, false], 1),
        "対照: 装飾付きの字で届いた字で"
    );
}

/// 6(b). 先渡しが箱の表と解決の閉包より先に届いても、それらを入れた後にやり直せば、
/// `\s` で箱へ行く字は箱の場所の全文で配置される。やり直さない対照は全文が無い。
#[test]
fn reinstall_after_box_layout_rehearses_into_boxes() {
    let resolve: &dyn Fn(&str) -> SurfaceKeyOutcome = &resolve;
    let talk = [
        cue("0", 0.0, CueCommand::ClearAll),
        cue("0", 0.0, CueCommand::Emote { key: "0".into() }),
        text("0", 0.1, "あい"),
    ];
    let run = |reinstall: bool| {
        let mut state = TextLayerState::default();
        let mut look = TalkLookahead::default();
        look.install(&state, None, &talk);
        set_boxes(&mut state);
        if reinstall {
            look.reinstall(&state, Some(resolve));
        }
        deliver(&mut look, &mut state, Some(resolve), &talk);
        let a = box_place(&state, "0", "a");
        let arrived = content(&state, &a);
        let (full, _) = capture(|| is_full(&look.basis(&a, &arrived)));
        (full, contents(&look), BTreeMap::from([((a, 1), arrived)]))
    };
    let (full, table, expected) = run(true);
    assert!(full, "やり直せば箱の場所は全文");
    assert_eq!(table, expected, "全文は箱の場所にだけある");
    let (full, table, expected) = run(false);
    assert!(!full, "対照: やり直さなければ全文が無い");
    assert_ne!(table, expected);
}

/// 6(c). 途中まで届けてからやり直す: 数えは 0 に戻らず（途中の `\c` の後でも番号が合う）、
/// 今より前の区間の全文は残り、今の区間の全文の先頭は届いた内容と一致する。
#[test]
fn reinstall_midway_keeps_counts_and_earlier_sections() {
    let actor = ActorKey::from("0");
    let balloon = PlaceKey::balloon(&actor);
    let talk = [
        cue("0", 0.0, CueCommand::ClearAll),
        text("0", 0.1, "あ"),
        cue("0", 0.2, CueCommand::Clear),
        text("0", 0.3, "い"),
        font("0", &["bold", "1"]),
        text("0", 0.4, "う"),
    ];
    let mut state = TextLayerState::default();
    let mut look = TalkLookahead::default();
    look.install(&state, None, &talk);
    deliver(&mut look, &mut state, None, &talk[..4]);
    let earlier = look.sections[&(balloon.clone(), 1)].content.clone();
    state.set_look_layers(&actor, attached_layers());
    // 食い違う字を聞いて今の区間に warn 済みの印を付ける（やり直しは印を消す）。
    let stray = arrive(&[text("0", 0.0, "さ")]);
    let stray_warns = |look: &mut TalkLookahead| {
        let (full, events) = capture(|| is_full(&look.basis(&balloon, &stray)));
        assert!(!full, "前提: 食い違う");
        warns(&events).len()
    };
    assert_eq!(stray_warns(&mut look), 1, "前提: 今の区間の印");

    look.reinstall(&state, None);
    assert_eq!(
        stray_warns(&mut look),
        1,
        "やり直しは warn 済みの印を消す＝新しい全文との食い違いは改めて 1 度"
    );
    assert_eq!(look.number(&balloon), 2, "数えは 0 に戻らない");
    assert_eq!(
        contents(&look).keys().cloned().collect::<Vec<_>>(),
        vec![(balloon.clone(), 1), (balloon.clone(), 2)]
    );
    assert_eq!(
        look.sections[&(balloon.clone(), 1)].content,
        earlier,
        "今より前の区間は残る"
    );
    assert!(
        begins_with(
            &look.sections[&(balloon.clone(), 2)].content,
            &content(&state, &balloon)
        ),
        "今の区間の全文の先頭は届いた内容"
    );
    assert_eq!(
        deliver_judging(&mut look, &mut state, None, &talk[4..], &balloon),
        (vec![true, true], 0),
        "残りは全文で配置される"
    );
    assert_eq!(
        contents(&look)[&(balloon.clone(), 2)],
        content(&state, &balloon),
        "全文は出終わった内容"
    );
}

/// 6(d). 残りの合図が何も足さない場所でも、やり直しの後は今の内容が今の区間の全文になる
/// （見た目が載せ直された後の内容と合わせる）。
#[test]
fn reinstall_keeps_current_content_where_nothing_is_added() {
    let actor = ActorKey::from("0");
    let sakura = PlaceKey::balloon(&actor);
    let kero = place("1", TextPlace::Balloon);
    let talk = [text("0", 0.0, "あ"), text("1", 0.1, "か")];
    let mut state = TextLayerState::default();
    let mut look = TalkLookahead::default();
    look.install(&state, None, &talk);
    deliver(&mut look, &mut state, None, &talk[..1]);
    state.set_look_layers(&actor, attached_layers());
    assert_ne!(
        contents(&look).get(&(sakura.clone(), 0)),
        Some(&content(&state, &sakura)),
        "前提: 見た目の差し込みで今の内容は全文と違う"
    );

    look.reinstall(&state, None);
    deliver(&mut look, &mut state, None, &talk[1..]);
    assert_eq!(
        contents(&look),
        BTreeMap::from([
            ((sakura.clone(), 0), content(&state, &sakura)),
            ((kero.clone(), 0), content(&state, &kero)),
        ])
    );
}

/// 6(e). 先渡しの列に無い合図が届くと列を捨て、以後のやり直しは何も変えない。
/// 対照（列どおりに届く）では同じやり直しが全文を変える。
#[test]
fn stray_cue_drops_upcoming_and_reinstall_becomes_noop() {
    let actor = ActorKey::from("0");
    let talk = [text("0", 0.0, "あ"), text("0", 0.1, "い")];
    let run = |first: TalkCue| {
        let mut state = TextLayerState::default();
        let mut look = TalkLookahead::default();
        look.install(&state, None, &talk);
        deliver(&mut look, &mut state, None, &[first]);
        let kept = look.upcoming.is_some();
        state.set_look_layers(&actor, attached_layers());
        let before = contents(&look);
        look.reinstall(&state, None);
        (kept, before == contents(&look))
    };
    assert_eq!(
        run(talk[0].clone()),
        (true, false),
        "対照: やり直しが全文を変える"
    );
    assert_eq!(
        run(text("0", 0.0, "か")),
        (false, true),
        "列に無い合図で列を捨て、やり直しは何も変えない"
    );
}
