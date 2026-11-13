//! # state_decoration_incremental_tests — 装飾の積み増しの檻（純粋層・兄弟テスト）
//!
//! spec `emo-text-reveal-incremental` 件 2（要件 2.7／2.8）。台本の写しと当て直しが届いた分に
//! 比例すること（数え）、混ぜた台本で全部当て直す形（`REPLAY_ALL`）と結果が一致すること、
//! 寿命と記録が今と変わらないことを確かめる。
//!
//! [`super`]（`state_decoration.rs`）の子モジュールとして接続してある。
//!
//! 断りの無い要件の番号はこの spec のもの。箱の構成はテストの中の surfaces.txt の文面を
//! `parse_boxes` → `fold_boxes` で畳んで作る（検体は読まない）:
//! - 箱 a: `font.follow` を書かない（＝スコープに従う）・箱 c: `font.follow,scope`・箱 x: `font.follow,balloon`（箱に閉じる箱）
//! - サーフェス: 0＝a・c（既定は a） 1＝x・a（既定は x） 3＝箱なし（普通のバルーン）
//!
//! 「一から計算した値」は、場所ごとに 2 層の上で土台から当てた指定の列を `apply_font_tag` で順に当てた
//! 見た目と、そのうち所有外の答えを返す指定を順に入れた所有外のキー（要件 2.5 の不変条件）。

use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

use areka_emo_compose::{BoxName, FontFollow};
use areka_sakura::contract::{CueCommand, TalkCue};
use log_capture_kit::{CapturedEvent, capture};

use super::super::route_tests::{key, select, show};
use super::super::test_support::{cue, font, write};
use super::super::{SurfaceKeyOutcome, TextLayerState};
use super::{BaseLayer, BoxTraits, COPIED_SPECS, Decoration, REBASED_SPECS, REPLAY_ALL};
use crate::actor::test_support::{box_in, box_index, box_layout};
use crate::look::{LookLayers, Note, StyleId, StyleTable, TextLook, apply_font_tag};
use crate::place::{PlaceKey, TextPlace};

const SHELL: &str = "\
balloon.a
{
size,100,50
}
balloon.c
{
size,60,30
font.follow,scope
}
balloon.x
{
size,80,40
font.follow,balloon
}
surface0
{
element1,balloon,a,0,0
element2,balloon,c,0,60
}
surface1
{
element1,balloon,x,0,0
element2,balloon,a,0,60
}
";

/// 文面を畳んだ箱の表と、箱の名前ごとの `font.follow`（1 度だけ組む）。
type Folded = (BTreeMap<u32, Vec<BoxName>>, BTreeMap<BoxName, FontFollow>);

static FOLDED: LazyLock<Folded> = LazyLock::new(|| {
    let layout = box_layout(SHELL);
    let index = box_index(&layout, 0..=3);
    let follow = index
        .values()
        .flatten()
        .map(|name| (name.clone(), layout.def(name).expect("箱の定義").follow))
        .collect();
    (index, follow)
});

fn bx(n: &str) -> TextPlace {
    TextPlace::Box(box_in(&FOLDED.0, n))
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

/// 箱の既定の 2 層と種類（`shift` で大きさをずらすと違う値の 2 層になる）。
fn traits(shift: f32) -> BTreeMap<BoxName, BoxTraits> {
    FOLDED
        .1
        .iter()
        .map(|(n, follow)| {
            let (height, r) = match n.as_str() {
                "a" => (20.0, 20),
                "c" => (40.0, 40),
                _ => (30.0, 30),
            };
            let looks = layers(height + shift, r);
            let follow = *follow;
            (n.clone(), BoxTraits { looks, follow })
        })
        .collect()
}

/// 箱の種類を箱の表より先に入れ、スコープ 0・1 に普通のバルーンの 2 層を差す。
fn state() -> TextLayerState {
    let mut state = TextLayerState::default();
    state.set_box_traits(traits(0.0));
    state.set_box_index(FOLDED.0.clone());
    for actor in ["0", "1"] {
        state.set_look_layers(&key(actor), layers(16.0, 10));
    }
    state
}

fn f(state: &mut TextLayerState, actor: &str, tokens: &[&str]) {
    state.apply_cue(&font(actor, tokens));
}

fn clear_all(actor: &str) -> TalkCue {
    cue(actor, 0.0, CueCommand::ClearAll)
}

/// 数え 2 つを 0 にする。
fn zero_counts() {
    COPIED_SPECS.with(|c| c.set(0));
    REBASED_SPECS.with(|c| c.set(0));
}

fn counts() -> (usize, usize) {
    (
        COPIED_SPECS.with(std::cell::Cell::get),
        REBASED_SPECS.with(std::cell::Cell::get),
    )
}

/// 場所の装飾状態を、テストの側で一から計算した（今の見た目, 所有外のキー）。
fn scratch(d: &Decoration) -> (TextLook, BTreeMap<String, Vec<String>>) {
    let mut look = match d.base {
        BaseLayer::Default => d.layers.default.clone(),
        BaseLayer::Disable => d.layers.disable.clone(),
    };
    let mut unowned = BTreeMap::new();
    for tokens in &d.applied {
        let args: Vec<&str> = tokens.iter().map(String::as_str).collect();
        if let Ok(Some(Note::Unowned)) = apply_font_tag(&mut look, &d.layers, &args) {
            unowned.insert(tokens[0].clone(), tokens[1..].to_vec());
        }
    }
    (look, unowned)
}

/// 全部の場所が一から計算した値と等しいことを確かめる。
fn assert_scratch(state: &TextLayerState, at: &str) {
    for (place, s) in state.places() {
        let (look, unowned) = scratch(&s.decor);
        assert_eq!(s.current_look(), &look, "{at}: {place:?} の今の見た目");
        assert_eq!(
            s.unowned_vocab(),
            &unowned,
            "{at}: {place:?} の所有外のキー"
        );
    }
}

/// 比べる中身（場所, 今の見た目, 装飾番号列, 装飾の表, 所有外のキー）。
type PlaceView = (
    PlaceKey,
    TextLook,
    Vec<StyleId>,
    StyleTable,
    BTreeMap<String, Vec<String>>,
);

fn view(state: &TextLayerState) -> Vec<PlaceView> {
    state
        .places()
        .map(|(place, s)| {
            (
                place.clone(),
                s.current_look().clone(),
                s.glyph_styles().to_vec(),
                s.styles().clone(),
                s.unowned_vocab().clone(),
            )
        })
        .collect()
}

fn warn_count(events: &[CapturedEvent]) -> usize {
    events
        .iter()
        .filter(|e| e.level == tracing::Level::WARN)
        .count()
}

/// 同じ手順を、積み増しの作り（`REPLAY_ALL` 偽）と全部当て直す作り（真）の 2 つの状態へ流す。
struct Twin {
    inc: TextLayerState,
    all: TextLayerState,
    warns: usize,
}

impl Twin {
    fn new() -> Twin {
        Twin {
            inc: state(),
            all: state(),
            warns: 0,
        }
    }

    /// 1 手順を両方へ流し、全部の場所の中身・記録の件数・一から計算した値を比べる。
    fn step<R>(&mut self, at: &str, op: impl Fn(&mut TextLayerState) -> R) {
        let (_, inc_events) = capture(|| op(&mut self.inc));
        REPLAY_ALL.with(|c| c.set(true));
        let (_, all_events) = capture(|| op(&mut self.all));
        REPLAY_ALL.with(|c| c.set(false));
        let warns = warn_count(&inc_events);
        assert_eq!(warns, warn_count(&all_events), "{at}: 記録の件数");
        self.warns += warns;
        assert_eq!(view(&self.inc), view(&self.all), "{at}: 全部の場所の中身");
        assert_scratch(&self.inc, at);
        assert_scratch(&self.all, at);
    }
}

// ---- 写しと当て直しの数え（要件 2.7）

/// 普通のバルーンとスコープに従う箱 2 つの間を 200 回行き来しながら `\f` を 300 件当てる。
/// 写しと当て直しの合計は「300 × 行き先になった場所の数＋2 層の差し替えの当て直しの数」の 2 倍以下。
/// 写しのたびに全部当て直す形（`REPLAY_ALL`）では約 3 万になり赤になる。
#[test]
fn copy_and_rebase_counts_stay_proportional_to_specs_times_places() {
    let mut state = state();
    zero_counts();
    let actor = key("0");
    let mut specs = 0usize;
    let mut moves = 0usize;
    let mut destinations = BTreeSet::new();
    destinations.insert(state.destination(&actor));
    for i in 0..200usize {
        let before = state.destination(&actor);
        match i % 3 {
            0 => show(&mut state, "0", 0),
            1 => {
                state.apply_cue(&select("0", "c"));
            }
            _ => show(&mut state, "0", 3),
        }
        let now = state.destination(&actor);
        assert_ne!(before, now, "{i} 回目で行き先が替わる");
        moves += 1;
        destinations.insert(now);
        let n = if i % 2 == 0 { 2 } else { 1 };
        for _ in 0..n {
            let r = (specs % 250).to_string();
            f(&mut state, "0", &["color", &r, "0", "0"]);
            specs += 1;
        }
        write(&mut state, "0", "・");
        // 2 層の差し替え（違う値）を 2 回だけ混ぜる。
        if i == 70 || i == 140 {
            let shift = i as f32 / 70.0;
            state.set_look_layers(&actor, layers(16.0 + shift, 10));
            state.set_box_traits(traits(shift));
        }
    }
    assert_eq!((moves, specs), (200, 300));
    assert_eq!(
        destinations,
        BTreeSet::from([TextPlace::Balloon, bx("a"), bx("c")]),
        "行き先になった場所は 3 つ"
    );
    let (copied, rebased) = counts();
    let bound = 2 * (specs * destinations.len() + rebased);
    assert!(
        copied + rebased <= bound,
        "写し {copied} ＋当て直し {rebased} が上限 {bound} を超えた"
    );
    assert!(copied > 0, "写しが数えられていない");
}

// ---- 全部当て直す形との一致（要件 2.1・2.5・2.8）

/// 行き先の替わり・一括の戻し・台本の先頭・装飾の戻し・2 層の差し替え（違う値と同じ値）・箱に閉じる箱・
/// 所有外のキー・相対値・`\f[キー,default]` を混ぜた台本を 2 つの作りへ流し、各時点で等しい。
#[test]
fn mixed_script_matches_replay_all_at_every_point() {
    let mut t = Twin::new();
    t.step("台本の先頭", |s| s.apply_cue(&clear_all("0")));
    t.step("字", |s| write(s, "0", "あ"));
    t.step("赤", |s| f(s, "0", &["color", "255", "0", "0"]));
    t.step("a へ", |s| {
        show(s, "0", 0);
        write(s, "0", "い");
    });
    t.step("相対値と所有外", |s| {
        f(s, "0", &["height", "+4"]);
        f(s, "0", &["align", "center"]);
        write(s, "0", "う");
    });
    t.step("c へ", |s| s.apply_cue(&select("0", "c")));
    t.step("キーの既定と不正", |s| {
        f(s, "0", &["color", "default"]);
        f(s, "0", &["height", "abc"]);
        write(s, "0", "え");
    });
    t.step("x へ（箱に閉じる）", |s| show(s, "0", 1));
    t.step("x の指定", |s| {
        f(s, "0", &["bold", "1"]);
        write(s, "0", "お");
    });
    t.step("a へ戻る", |s| {
        s.apply_cue(&select("0", "a"));
        write(s, "0", "か");
    });
    t.step("同じ不正をもう一度", |s| {
        f(s, "0", &["height", "abc"])
    });
    t.step("箱の 2 層を違う値へ", |s| {
        s.set_box_traits(traits(3.0))
    });
    t.step("箱の 2 層を同じ値で", |s| {
        s.set_box_traits(traits(3.0))
    });
    t.step("バルーンの 2 層を同じ値で", |s| {
        s.set_look_layers(&key("0"), layers(16.0, 10));
    });
    t.step("バルーンの 2 層を違う値へ", |s| {
        s.set_look_layers(&key("0"), layers(18.0, 12));
    });
    t.step("バルーンへ", |s| {
        show(s, "0", 3);
        write(s, "0", "き");
    });
    t.step("無効表示へ戻して青", |s| {
        f(s, "0", &["disable"]);
        f(s, "0", &["color", "0", "0", "255"]);
        write(s, "0", "く");
    });
    t.step("a へ・影", |s| {
        show(s, "0", 0);
        f(s, "0", &["shadowcolor", "1", "2", "3"]);
        f(s, "0", &["height", "150%"]);
        write(s, "0", "け");
    });
    t.step("装飾の戻し（全部）", |s| s.reset_decoration(None));
    t.step("斜体で c へ", |s| {
        f(s, "0", &["italic", "1"]);
        s.apply_cue(&select("0", "c"));
        write(s, "0", "こ");
    });
    t.step("既定へ戻してバルーンへ", |s| {
        f(s, "0", &["default"]);
        show(s, "0", 3);
        write(s, "0", "さ");
    });
    t.step("箱の 2 層を元へ", |s| s.set_box_traits(traits(0.0)));
    t.step("次の台詞", |s| {
        s.apply_cue(&clear_all("0"));
        f(s, "0", &["height", "abc"]);
        f(s, "0", &["color", "9", "9", "9"]);
        show(s, "0", 0);
        write(s, "0", "し");
    });
    t.step("装飾の戻し（スコープ）", |s| {
        s.reset_decoration(Some(&key("0")))
    });
    t.step("別のスコープ", |s| {
        f(s, "1", &["color", "255", "0", "0"]);
        show(s, "1", 0);
        f(s, "1", &["align", "right"]);
        s.apply_cue(&select("1", "c"));
        write(s, "1", "す");
    });
    t.step("隠す", |s| {
        s.route_surface(&key("0"), SurfaceKeyOutcome::Hide);
        write(s, "0", "せ");
    });
    // 同じ不正な指定は 1 台詞 1 度（台本の先頭の後にもう 1 度）＝ 2 件。
    assert_eq!(t.warns, 2, "不正な指定の記録の件数");
}

// ---- 台詞の途中で生まれた場所（要件 2.5）

/// 台詞の途中で生まれた箱の場所が `` を受けてから行き先が替わっても、一から計算した値と等しい
/// （印が場所の生まれで振られ、別の場所の系列の頭とみなされない）。
///
/// 台本の先頭を流さないので、普通のバルーンは生まれたときの印のまま指定を 1 件持ち続ける。
/// 生まれたての c が同じ印を持つと、バルーンへ戻る写しが c の指定を「当て済み」とみなして赤が残る。
#[test]
fn place_born_mid_talk_matches_scratch_after_route_change() {
    let mut t = Twin::new();
    t.step("バルーンに赤", |s| {
        f(s, "0", &["color", "255", "0", "0"])
    });
    t.step("a へ（赤を写す）", |s| show(s, "0", 0));
    t.step("a を既定へ戻す", |s| f(s, "0", &["default"]));
    t.step("空の持ち主のまま c へ", |s| {
        s.apply_cue(&select("0", "c"));
    });
    t.step("c が生まれて青", |s| {
        f(s, "0", &["color", "0", "0", "255"]);
        write(s, "0", "あ");
    });
    t.step("古い印のバルーンへ", |s| {
        show(s, "0", 3);
        write(s, "0", "い");
    });
    let balloon = t
        .inc
        .place_state(&super::super::test_support::at("0", TextPlace::Balloon))
        .expect("バルーンの場所");
    assert_eq!(
        balloon.current_look().color,
        (0, 0, 255),
        "c の青が届き、バルーンの古い赤は残らない"
    );
}
