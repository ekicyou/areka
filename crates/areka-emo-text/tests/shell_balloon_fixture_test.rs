//! # shell_balloon_fixture_test — 検体を読み手から文字の層まで通す（task 12.2・要件 10.4）
//!
//! 検体 `tests/fixtures/shell-balloon/surfaces.txt` を、読み手（`parse_boxes`・`parse`）→
//! 畳み込み（`fold_boxes`）→ 文字の層（`TextLayerRuntime::set_box_layout`）へ通し、台本を
//! 指令の列として当てて、場所ごとの文字・行き先・3 種類の箱の見た目を確かめる。GPU は使わない
//! （箱の束の受け取りは空の `World` で足りる・面は提示まで作られない）。
//!
//! 検体の箱:
//! - `tate`: `font.follow` を書かない（＝スコープに従う）・`font.height,16`・色 (0,0,0)
//! - `yoko`: `font.follow,scope`・`font.height,20`・色 (64,0,0)
//! - `fuda`: `font.follow,balloon`（箱に閉じる）・`font.height,12`・色 (0,0,128)
//!
//! サーフェス: 0＝tate・yoko（既定は tate） 1＝tate だけ 2＝箱なし 3＝fuda（`surface.append3` で足す）。
//! 普通のバルーンは装着しない（2 層は正典の既定＝大きさ 12・黒）。
//!
//! `\s` の解決は偽の閉包（数字ならその番号を表示・`-1` は非表示）。本番の閉包はシェルの
//! サーフェスの在不在で解くが、検体の 0〜3 はすべて在るので同じ結果になる。

use std::collections::BTreeMap;
use std::path::PathBuf;

use areka_emo_compose::{BoxLayout, EmoWorld, FontFollow, fold_boxes};
use areka_emo_text::actor::{SurfaceKeyResolver, TextLayerRuntime};
use areka_emo_text::look::TextLook;
use areka_emo_text::place::{PlaceKey, TextPlace};
use areka_emo_text::state::{SurfaceKeyOutcome, TextItem, TextLayerConfig};
use areka_parsers::charset::{DefaultEncoding, decode};
use areka_parsers::shell::{parse, parse_boxes};
use areka_sakura::contract::{ActorKey, CueCommand, FONT_TAG_CARRIER, TalkCue};
use bevy_ecs::prelude::World;

const RED: (u8, u8, u8) = (255, 0, 0);
const GREEN: (u8, u8, u8) = (0, 255, 0);
/// 検体の各ブレスの既定（色・大きさ）。
const TATE: ((u8, u8, u8), f32) = ((0, 0, 0), 16.0);
const YOKO: ((u8, u8, u8), f32) = ((64, 0, 0), 20.0);
const FUDA: ((u8, u8, u8), f32) = ((0, 0, 128), 12.0);
/// 普通のバルーン（未装着＝正典の既定）。
const BALLOON: ((u8, u8, u8), f32) = ((0, 0, 0), 12.0);

// ── 検体 → 読み手 → 畳み込み ──

fn fixture_text() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("shell-balloon")
        .join("surfaces.txt");
    let bytes = std::fs::read(&path)
        .unwrap_or_else(|e| panic!("検体 {} の読取に失敗した: {e}", path.display()));
    decode(&bytes, DefaultEncoding::Utf8)
}

fn layout() -> BoxLayout {
    let text = fixture_text();
    let world = EmoWorld::build(&parse(&text));
    let (layout, report) = fold_boxes(&parse_boxes(&text), &BTreeMap::new(), &world);
    assert_eq!(
        report.issues,
        vec![],
        "検体は正しく書いたシェル（報告 0 件）"
    );
    layout
}

fn names(layout: &BoxLayout, surface: u32) -> Vec<&str> {
    layout
        .placements(surface)
        .iter()
        .map(|p| p.name.as_str())
        .collect()
}

fn fake_resolver() -> SurfaceKeyResolver {
    Box::new(|key: &str| match key {
        "-1" => SurfaceKeyOutcome::Hide,
        _ => key
            .parse()
            .map_or(SurfaceKeyOutcome::Unresolved, SurfaceKeyOutcome::Show),
    })
}

/// 検体の箱の束を受け取った文字の層。
fn runtime() -> TextLayerRuntime {
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    rt.set_box_layout(&mut World::new(), layout(), fake_resolver(), Vec::new());
    rt
}

// ── 台本（指令の列） ──

fn cue(actor: &str, command: CueCommand) -> TalkCue {
    TalkCue {
        at: 0.0,
        actor: ActorKey::from(actor),
        command,
        duration: 0.0,
    }
}

fn s(actor: &str, id: &str) -> TalkCue {
    cue(actor, CueCommand::Emote { key: id.into() })
}

fn b(actor: &str, name: &str) -> TalkCue {
    cue(actor, CueCommand::BalloonSurface { key: name.into() })
}

fn t(actor: &str, text: &str) -> TalkCue {
    cue(actor, CueCommand::Text(text.into()))
}

fn f(actor: &str, tokens: &[&str]) -> TalkCue {
    cue(
        actor,
        CueCommand::command_carrier(
            FONT_TAG_CARRIER,
            tokens.iter().map(|t| (*t).to_owned()).collect(),
        ),
    )
}

fn c(actor: &str) -> TalkCue {
    cue(actor, CueCommand::Clear)
}

/// 台詞の頭（`ClearAll`）。
fn head() -> TalkCue {
    cue("0", CueCommand::ClearAll)
}

fn play(rt: &mut TextLayerRuntime, script: &[TalkCue]) {
    for cue in script {
        rt.apply_cue(cue);
    }
}

// ── 観測 ──

fn place(actor: &str, at: &str) -> PlaceKey {
    let place = match at {
        "" => TextPlace::Balloon,
        name => TextPlace::Box(
            layout()
                .surfaces()
                .flat_map(|(_, ps)| ps.iter().map(|p| p.name.clone()).collect::<Vec<_>>())
                .find(|n| n.as_str() == name)
                .expect("検体にある箱の名前"),
        ),
    };
    PlaceKey {
        actor: ActorKey::from(actor),
        place,
    }
}

/// 場所の文字（箱の名前 `""` は普通のバルーン・場所が無ければ空）。
fn text_at(rt: &TextLayerRuntime, actor: &str, at: &str) -> String {
    rt.state()
        .place_state(&place(actor, at))
        .map(|s| {
            s.items()
                .iter()
                .filter_map(|it| match it {
                    TextItem::Glyph { text } => Some(text.to_string()),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default()
}

fn dest(rt: &TextLayerRuntime, actor: &str) -> TextPlace {
    rt.state().destination(&ActorKey::from(actor))
}

/// 場所の最後の文字の見た目（配置層と同じく番号を表で引き直す）。
fn last_look(rt: &TextLayerRuntime, actor: &str, at: &str) -> TextLook {
    let s = rt
        .state()
        .place_state(&place(actor, at))
        .expect("書いた場所");
    let id = *s.glyph_styles().last().expect("1 字以上ある");
    s.styles().resolve(id, &s.look_layers().default).clone()
}

fn look(rt: &TextLayerRuntime, actor: &str, at: &str) -> ((u8, u8, u8), f32) {
    let l = last_look(rt, actor, at);
    (l.color, l.height)
}

// ── 検体そのもの ──

#[test]
fn fixture_folds_three_box_kinds_and_their_placements() {
    let layout = layout();
    assert_eq!(names(&layout, 0), ["tate", "yoko"], "element番号の昇順");
    assert_eq!(names(&layout, 1), ["tate"]);
    assert_eq!(names(&layout, 2), Vec::<&str>::new(), "箱の無いサーフェス");
    assert_eq!(names(&layout, 3), ["fuda"], "surface.append3 で足した箱");
    let pos = |id: u32| {
        let p = &layout.placements(id)[0];
        (p.x, p.y)
    };
    assert_eq!(pos(0), (10, 20));
    assert_eq!(pos(1), (240, 40), "同じ名前の箱を別の位置に置く");
    let def = |n: &str| {
        let TextPlace::Box(name) = place("0", n).place else {
            unreachable!("箱の名前")
        };
        let def = layout.def(&name).expect("箱の定義");
        (def.size, def.follow)
    };
    assert_eq!(
        def("tate"),
        ((120, 240), FontFollow::Scope),
        "書かない＝スコープ"
    );
    assert_eq!(def("yoko"), ((200, 80), FontFollow::Scope));
    assert_eq!(def("fuda"), ((160, 60), FontFollow::Balloon));
}

// ── 行き先と場所ごとの文字（要件 4.3・4.5・6.1〜6.5） ──

#[test]
fn script_routes_text_to_default_and_named_boxes_across_surfaces() {
    let mut rt = runtime();
    play(&mut rt, &[head(), s("0", "0"), t("0", "あい")]);
    assert_eq!(
        dest(&rt, "0"),
        place("0", "tate").place,
        "既定は element番号最小"
    );
    assert_eq!(text_at(&rt, "0", "tate"), "あい");

    // `\b[名前]` で切り替えても前の箱の文字は残る（4.3・4.5）。
    play(&mut rt, &[b("0", "yoko"), t("0", "うえ")]);
    assert_eq!(dest(&rt, "0"), place("0", "yoko").place);
    assert_eq!(text_at(&rt, "0", "yoko"), "うえ");
    assert_eq!(
        text_at(&rt, "0", "tate"),
        "あい",
        "切り替える前の箱は消えない"
    );

    // yoko の無い surface1: 既定の tate へ書き、yoko の文字は保持（6.2）。
    // tate は surface1 にもあるので続きへ書く（6.1）。
    play(&mut rt, &[s("0", "1"), t("0", "お")]);
    assert_eq!(dest(&rt, "0"), place("0", "tate").place);
    assert_eq!(text_at(&rt, "0", "tate"), "あいお");
    assert_eq!(text_at(&rt, "0", "yoko"), "うえ", "表示をやめても保持");

    // surface0 へ戻る: 行き先の tate は在るので保つ（6.1）。保持した yoko の文字は新しい
    // サーフェスにも置き場所がある＝再び出る（6.3・6.5）。続きは同じ箱へ書く。
    play(&mut rt, &[s("0", "0")]);
    assert_eq!(dest(&rt, "0"), place("0", "tate").place);
    assert_eq!(names(&layout(), 0), ["tate", "yoko"]);
    play(&mut rt, &[b("0", "yoko"), t("0", "か")]);
    assert_eq!(text_at(&rt, "0", "yoko"), "うえか");

    // 箱の無い surface2: 普通のバルーンへ書き、箱の文字は保持（6.4）。
    play(&mut rt, &[s("0", "2"), t("0", "き")]);
    assert_eq!(dest(&rt, "0"), TextPlace::Balloon);
    assert_eq!(text_at(&rt, "0", ""), "き");
    assert_eq!(text_at(&rt, "0", "tate"), "あいお");
    assert_eq!(text_at(&rt, "0", "yoko"), "うえか");

    // 箱へ戻ると普通のバルーンの文字は保持され、箱の続きへ書く（6.9 の逆向き）。
    play(&mut rt, &[s("0", "1"), t("0", "く")]);
    assert_eq!(text_at(&rt, "0", "tate"), "あいおく");
    assert_eq!(text_at(&rt, "0", ""), "き");
}

// ── `\c` は今の行き先の箱だけ（要件 7.1・7.2・4.6） ──

#[test]
fn clear_erases_only_the_destination_box() {
    let mut rt = runtime();
    play(
        &mut rt,
        &[
            head(),
            s("0", "0"),
            t("0", "あ"),
            b("0", "yoko"),
            t("0", "い"),
            s("1", "1"),
            t("1", "う"),
        ],
    );
    assert_eq!(
        text_at(&rt, "1", "tate"),
        "う",
        "\\1 の tate は \\0 の tate と別"
    );
    assert_eq!(text_at(&rt, "0", "tate"), "あ");

    play(&mut rt, &[c("0"), t("0", "え")]);
    assert_eq!(
        text_at(&rt, "0", "yoko"),
        "え",
        "消した箱の書き出しから書く"
    );
    assert_eq!(
        text_at(&rt, "0", "tate"),
        "あ",
        "同じスコープの他の箱は消えない"
    );
    assert_eq!(text_at(&rt, "1", "tate"), "う", "他のスコープは消えない");
}

// ── 選択肢は今の行き先の箱へ（要件 8.1） ──

#[test]
fn choice_lands_in_the_destination_box() {
    let mut rt = runtime();
    play(
        &mut rt,
        &[
            head(),
            s("0", "0"),
            b("0", "yoko"),
            t("0", "どっち"),
            cue(
                "0",
                CueCommand::Choice {
                    id: "OnA".into(),
                    text: "はい".into(),
                    references: vec!["r0".into()],
                },
            ),
        ],
    );
    let yoko = rt.state().place_state(&place("0", "yoko")).expect("yoko");
    assert_eq!(yoko.choices().len(), 1);
    let span = &yoko.choices()[0];
    assert_eq!(
        (
            span.id.as_str(),
            span.label.as_str(),
            span.glyph_range.clone()
        ),
        ("OnA", "はい", 3..5),
        "選択肢の文字は台詞の続きに並ぶ"
    );
    assert_eq!(text_at(&rt, "0", "yoko"), "どっちはい");
    assert!(
        rt.state()
            .place_state(&place("0", "tate"))
            .is_none_or(|s| s.choices().is_empty())
    );
    assert!(rt.choice_active(&ActorKey::from("0")));

    // `\c` でその箱を消すと選択肢も消える。
    play(&mut rt, &[c("0")]);
    assert!(!rt.choice_active(&ActorKey::from("0")));
}

// ── 3 種類の箱の見た目（要件 3.13〜3.17） ──

#[test]
fn scope_following_boxes_carry_the_spec_and_take_each_default() {
    // tate（書かない）・yoko（scope）・普通のバルーンはスコープの指定を持ち回る（3.13・3.17）。
    let mut rt = runtime();
    play(&mut rt, &[head(), s("0", "0"), t("0", "あ")]);
    assert_eq!(
        look(&rt, "0", "tate"),
        TATE,
        "台本の指定が無ければ定義の既定"
    );

    play(
        &mut rt,
        &[f("0", &["color", "255", "0", "0"]), t("0", "い")],
    );
    assert_eq!(look(&rt, "0", "tate"), (RED, 16.0));
    // `\b[名前]` での切り替え: 指定は保ち、指定していない大きさは yoko の既定。
    play(&mut rt, &[b("0", "yoko"), t("0", "う")]);
    assert_eq!(look(&rt, "0", "yoko"), (RED, 20.0));
    // サーフェスの切り替えに伴う切り替え（箱 → 普通のバルーン → 箱）。
    play(&mut rt, &[s("0", "2"), t("0", "え")]);
    assert_eq!(look(&rt, "0", ""), (RED, 12.0));
    play(&mut rt, &[s("0", "1"), t("0", "お")]);
    assert_eq!(look(&rt, "0", "tate"), (RED, 16.0));
    // `\f[default]` は今の行き先の定義の既定へ戻す。
    play(&mut rt, &[f("0", &["default"]), t("0", "か")]);
    assert_eq!(look(&rt, "0", "tate"), TATE);
    // 切り替える前に書いた文字の見た目は後の `\f` に染まらない。
    assert_eq!(look(&rt, "0", "yoko"), (RED, 20.0));
}

#[test]
fn balloon_following_box_keeps_its_own_spec_only() {
    let mut rt = runtime();
    play(
        &mut rt,
        &[
            head(),
            s("0", "0"),
            f("0", &["color", "255", "0", "0"]),
            t("0", "あ"),
        ],
    );
    assert_eq!(look(&rt, "0", "tate"), (RED, 16.0));

    // fuda へ入る: 他の場所の指定を当てない＝fuda の定義の既定だけ（3.14）。
    play(&mut rt, &[s("0", "3"), t("0", "い")]);
    assert_eq!(dest(&rt, "0"), place("0", "fuda").place);
    assert_eq!(look(&rt, "0", "fuda"), FUDA);
    // fuda の中の指定は fuda だけに持つ（3.14）。
    play(
        &mut rt,
        &[f("0", &["color", "0", "255", "0"]), t("0", "う")],
    );
    assert_eq!(look(&rt, "0", "fuda"), (GREEN, 12.0));

    // 出た先は fuda へ入る前のスコープの指定（3.16）。
    play(&mut rt, &[s("0", "0"), t("0", "え")]);
    assert_eq!(look(&rt, "0", "tate"), (RED, 16.0));
    play(&mut rt, &[b("0", "yoko"), t("0", "お")]);
    assert_eq!(look(&rt, "0", "yoko"), (RED, 20.0));
    play(&mut rt, &[s("0", "2"), t("0", "か")]);
    assert_eq!(look(&rt, "0", ""), (RED, 12.0));

    // 同じ台詞のあいだに fuda へ戻る: fuda の指定のまま続く（3.15）。
    play(&mut rt, &[s("0", "3"), t("0", "き")]);
    assert_eq!(look(&rt, "0", "fuda"), (GREEN, 12.0));
    // fuda の中の `\f[default]` は fuda の指定だけを戻し、スコープの指定は残る。
    play(&mut rt, &[f("0", &["default"]), t("0", "く")]);
    assert_eq!(look(&rt, "0", "fuda"), FUDA);
    play(&mut rt, &[s("0", "1"), t("0", "け")]);
    assert_eq!(look(&rt, "0", "tate"), (RED, 16.0));
}

#[test]
fn spec_is_per_scope_and_unspecified_scope_keeps_defaults() {
    // \0 の指定は \1 の箱へ及ばない（スコープごと・3.13）。
    let mut rt = runtime();
    play(
        &mut rt,
        &[
            head(),
            s("0", "0"),
            f("0", &["color", "255", "0", "0"]),
            t("0", "あ"),
            s("1", "0"),
            t("1", "い"),
            b("1", "yoko"),
            t("1", "う"),
            s("1", "2"),
            t("1", "え"),
        ],
    );
    assert_eq!(look(&rt, "0", "tate"), (RED, 16.0));
    assert_eq!(look(&rt, "1", "tate"), TATE);
    assert_eq!(look(&rt, "1", "yoko"), YOKO);
    assert_eq!(look(&rt, "1", ""), BALLOON);
}
