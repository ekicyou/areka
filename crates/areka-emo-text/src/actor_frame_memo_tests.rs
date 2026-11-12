//! 配置の覚え（audit-emo-text-state-actor 指摘 8）の鍵の欄ごとの檻（実 GPU・`present_frame` の本番の道）。
//!
//! 鍵の欄ごとに「その欄だけ変えたフレームで作り直す」と、「何も変えない 2 フレーム目は作り直さない」を
//! 固定する。作り直しの回数はテストだけの数 `frame_rebuilds` で見る。各テストは捕まえる欄だけが変わる
//! ように組む（鍵の欄を 1 つ外すと、その欄のテストが落ちる）:
//!
//! | テスト | 捕まえる欄 |
//! |---|---|
//! | 見える字の数 | `visible` |
//! | 項目（改行の印） | `items` |
//! | 同じ数の書き直し（`\c` の後に同じ字） | `clears` |
//! | 選択肢（`\_a` の開き） | `choices` |
//! | 押した選択肢 | `settled` |
//! | 強調 | `hover` |
//! | 先読みの入れ直し | `lookahead_rev` |
//! | 配置の入力の差し替え | 配置の入力（`input`） |
//!
//! `\f` だけで今の見た目が変わり字が増えないフレームは作り直さない（今の見た目は鍵にしない）。
//!
//! 覚えを毎フレーム捨てた描き手と覚えを使う描き手とで、読み戻しの画素が同じことを見る。
//!
//! 届いた字の区切りの覚え（文節の折り返しで先読みが無い道）は、区切り直しの回数をテストだけの数
//! `BreakMemo::runs` で見る: 見える字だけ増えるフレームは区切り直さず、項目が増えるか消して同じ数を
//! 書き直すと区切り直す。最後の 1 本は区切りの覚えの有無で読み戻しの画素が同じことを見る。

use areka_sakura::contract::{ActorKey, CueCommand, TalkCue};
use bevy_ecs::prelude::World;

use super::test_support::{balloon, choice, com_world, cue, fake_resolver, geo_model, instant};
use super::{ResolvedBalloonText, TextLayerRuntime, TextSlotBinding, present_frame};
use crate::place::PlaceKey;
use crate::state::TextLayerConfig;
use crate::state::test_support::font;
use crate::wrap::WrapMode;

/// バルーン面の代表寸。
const IMAGE: (u32, u32) = (240, 160);

/// 全部の字が見える十分に後の時刻。
const SETTLED: f64 = 100.0;

/// 台本を当てて装着し、時刻 `at` の 1 フレーム目（作り直し 1 回）まで進めた組。
struct Rig {
    world: World,
    rt: TextLayerRuntime,
    place: PlaceKey,
}

impl Rig {
    fn new(script: &[TalkCue], at: f64) -> Rig {
        Rig::with_wrap(script, at, WrapMode::CharByChar)
    }

    fn with_wrap(script: &[TalkCue], at: f64, wrap: WrapMode) -> Rig {
        let (world, window, slot) = com_world();
        let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
        for cue in script {
            rt.apply_cue(cue);
        }
        let mut resolved = ResolvedBalloonText::resolve(&geo_model(), IMAGE);
        resolved.wrap = wrap;
        rt.register_actor(
            ActorKey::from("0"),
            TextSlotBinding::new(slot, window, 1.0, IMAGE, IMAGE),
            resolved,
        );
        let mut rig = Rig {
            world,
            rt,
            place: balloon("0"),
        };
        assert_eq!(rig.frame(at), 1, "装着のフレームで 1 度作る");
        rig
    }

    /// 文節の折り返しで装着し、時刻 `at` の 1 フレーム目まで進めた組（先読みが無いので届いた字で区切る）。
    fn budoux(script: &[TalkCue], at: f64) -> Rig {
        Rig::with_wrap(script, at, WrapMode::BudouxWordWrap)
    }

    /// 時刻 `at` の 1 フレームを回し、届いた字の区切り直しの回数の累計を返す。
    fn break_runs(&mut self, at: f64) -> u32 {
        self.frame(at);
        self.rt
            .surfaces
            .get(&self.place)
            .expect("装着済みの描き手")
            .break_memo
            .runs
    }

    /// 時刻 `at` の 1 フレームを回し、作り直しの回数の累計を返す。
    fn frame(&mut self, at: f64) -> u32 {
        present_frame(&mut self.rt, &mut self.world, at).expect("フレーム");
        self.rt
            .frame_rebuilds(&self.place)
            .expect("装着済みの描き手")
    }
}

fn instant_text(body: &str) -> TalkCue {
    instant("0", CueCommand::Text(body.to_owned()))
}

#[test]
fn unchanged_second_frame_does_not_rebuild() {
    let mut rig = Rig::new(&[instant_text("アヒル"), choice("0", "A")], SETTLED);
    assert_eq!(rig.frame(SETTLED), 1, "何も変えないフレームは作り直さない");
    assert_eq!(
        rig.frame(SETTLED + 1.0),
        1,
        "時刻だけ進んでも見える字が同じなら作り直さない"
    );
}

#[test]
fn more_visible_glyphs_rebuild() {
    // 再生時間つきの字（1 字 0.05 秒）: t=0 で 1 字、t=0.06 で 2 字が見える。
    let mut rig = Rig::new(&[cue("0", 0.0, CueCommand::Text("アヒル".into()))], 0.0);
    assert_eq!(rig.frame(0.06), 2, "見える字が増えたフレームで作り直す");
}

#[test]
fn new_item_rebuilds() {
    let mut rig = Rig::new(&[instant_text("アヒル")], SETTLED);
    // 改行の印は項目を 1 つ足すが、見える字は増やさない。
    rig.rt
        .apply_cue(&instant("0", CueCommand::NewLine { ratio: 1.0 }));
    assert_eq!(rig.frame(SETTLED), 2, "項目が増えたフレームで作り直す");
}

#[test]
fn same_count_rewrite_after_clear_rebuilds() {
    let mut rig = Rig::new(&[instant_text("アヒル")], SETTLED);
    // 同じフレームで消して同じ数を書き直す: 項目・見える字・見た目は前と同じで、消した回数だけ進む。
    rig.rt.apply_cue(&instant("0", CueCommand::Clear));
    rig.rt.apply_cue(&instant_text("カモメ"));
    assert_eq!(
        rig.frame(SETTLED),
        2,
        "消して同じ数を書き直したフレームで作り直す"
    );
}

#[test]
fn anchor_open_rebuilds() {
    let mut rig = Rig::new(&[instant_text("アヒル")], SETTLED);
    // `\_a` の開きは項目を増やさずに範囲を 1 つ足す。
    rig.rt.apply_cue(&instant(
        "0",
        CueCommand::AnchorBegin {
            id: "x".into(),
            references: Vec::new(),
        },
    ));
    assert_eq!(
        rig.frame(SETTLED),
        2,
        "選択肢の数が変わったフレームで作り直す"
    );
}

#[test]
fn settling_choices_rebuilds() {
    let mut rig = Rig::new(&[instant_text("アヒル"), choice("0", "A")], SETTLED);
    rig.rt.note_choice_wait_began();
    let place = rig.place.clone();
    rig.rt.settle_choices_at(&place);
    assert_eq!(rig.frame(SETTLED), 2, "選択肢を済ませたフレームで作り直す");
}

#[test]
fn hover_change_rebuilds() {
    let mut rig = Rig::new(&[instant_text("アヒル"), choice("0", "A")], SETTLED);
    rig.rt.inject_choice_hover(&ActorKey::from("0"), Some(0));
    assert_eq!(rig.frame(SETTLED), 2, "強調が変わったフレームで作り直す");
}

#[test]
fn font_tag_only_does_not_rebuild() {
    let mut rig = Rig::new(&[instant_text("アヒル")], SETTLED);
    // `\f` だけは項目を増やさずに今の見た目を変える。配置は字ごとの番号列しか読まない。
    rig.rt.apply_cue(&font("0", &["bold", "1"]));
    assert_eq!(
        rig.frame(SETTLED),
        1,
        "今の見た目だけ変わって字が増えないフレームは作り直さない"
    );
    // 太字の字が届けば項目が増えるので作り直す。
    rig.rt.apply_cue(&instant_text("カ"));
    assert_eq!(rig.frame(SETTLED), 2, "字が届いたフレームで作り直す");
}

#[test]
fn lookahead_reinstall_rebuilds() {
    let mut rig = Rig::new(&[instant_text("アヒル")], SETTLED);
    // `\s` の解決の閉包を差し込む口は、全文の先読みを入れ直すだけ（状態も配置の入力も変えない）。
    rig.rt.set_surface_resolver(fake_resolver());
    assert_eq!(
        rig.frame(SETTLED),
        2,
        "先読みを入れ直したフレームで作り直す"
    );
}

#[test]
fn layout_input_replacement_rebuilds() {
    let mut rig = Rig::new(&[instant_text("アヒル")], SETTLED);
    // 配置の入力だけを差し替える（`register_actor` は先読みも入れ直すので、欄を直に替える）。
    rig.rt.layout_input.insert(
        rig.place.clone(),
        ResolvedBalloonText::resolve(&geo_model(), (200, 120)),
    );
    assert_eq!(
        rig.frame(SETTLED),
        2,
        "配置の入力が替わったフレームで作り直す"
    );
}

/// 覚えを使う描き手と、覚えを毎フレーム捨てる描き手とで、同じ台本と同じ時刻の列の読み戻しが同じ。
#[test]
fn read_back_is_the_same_with_and_without_the_memo() {
    let script = [
        cue("0", 0.0, CueCommand::Text("アヒル".into())),
        choice("0", "A"),
    ];
    let mut kept = Rig::new(&script, 0.0);
    let mut dropped = Rig::new(&script, 0.0);
    let actor = ActorKey::from("0");
    let steps: [(f64, Option<usize>); 6] = [
        (0.0, None),
        (0.06, None),
        (0.06, Some(0)),
        (0.2, Some(0)),
        (0.2, None),
        (0.2, None),
    ];
    for (at, hover) in steps {
        for rig in [&mut kept, &mut dropped] {
            rig.rt.inject_choice_hover(&actor, hover);
        }
        kept.frame(at);
        let place = dropped.place.clone();
        dropped
            .rt
            .surfaces
            .get_mut(&place)
            .expect("装着済みの描き手")
            .frame_memo = None;
        dropped.frame(at);
        assert_eq!(
            super::test_support::read_back(&kept.rt, &actor),
            super::test_support::read_back(&dropped.rt, &actor),
            "t={at} hover={hover:?} で覚えの有無の画素が同じ"
        );
    }
    let kept_n = kept.rt.frame_rebuilds(&kept.place).expect("描き手");
    let dropped_n = dropped.rt.frame_rebuilds(&dropped.place).expect("描き手");
    assert!(
        kept_n < dropped_n,
        "覚えを使う側は作り直しが少ない（覚えが効いたフレームがある）: {kept_n} < {dropped_n}"
    );
}

#[test]
fn more_visible_glyphs_reuse_the_breaks() {
    let mut rig = Rig::budoux(&[cue("0", 0.0, CueCommand::Text("アヒルの子".into()))], 0.0);
    assert_eq!(rig.break_runs(0.0), 1, "装着のフレームで 1 度区切る");
    assert_eq!(rig.frame(0.06), 2, "見える字が増えたので配置は作り直す");
    assert_eq!(rig.break_runs(0.2), 1, "項目が同じなら区切り直さない");
}

#[test]
fn new_item_rebreaks() {
    let mut rig = Rig::budoux(&[instant_text("アヒル")], SETTLED);
    rig.rt.apply_cue(&instant_text("の子"));
    assert_eq!(rig.break_runs(SETTLED), 2, "項目が増えたら区切り直す");
}

#[test]
fn same_count_rewrite_after_clear_rebreaks() {
    let mut rig = Rig::budoux(&[instant_text("アヒル")], SETTLED);
    rig.rt.apply_cue(&instant("0", CueCommand::Clear));
    rig.rt.apply_cue(&instant_text("カモメ"));
    assert_eq!(
        rig.break_runs(SETTLED),
        2,
        "消して同じ数を書き直したら区切り直す"
    );
}

/// 区切りの覚えを使う描き手と、区切りの覚えを毎フレーム捨てる描き手とで、読み戻しが同じ。
#[test]
fn read_back_is_the_same_with_and_without_the_break_memo() {
    let script = [cue("0", 0.0, CueCommand::Text("アヒルの子とカモメ".into()))];
    let mut kept = Rig::budoux(&script, 0.0);
    let mut dropped = Rig::budoux(&script, 0.0);
    let actor = ActorKey::from("0");
    let place = dropped.place.clone();
    for (step, at) in [0.06, 0.2, 0.3, SETTLED, SETTLED].into_iter().enumerate() {
        if step == 3 {
            // 消して同じ数を書き直す（鍵の消した回数だけが進む）。
            for rig in [&mut kept, &mut dropped] {
                rig.rt.apply_cue(&instant("0", CueCommand::Clear));
                rig.rt.apply_cue(&instant_text("カモメの子とアヒル"));
            }
        }
        kept.frame(at);
        dropped
            .rt
            .surfaces
            .get_mut(&place)
            .expect("装着済みの描き手")
            .break_memo = Default::default();
        dropped.frame(at);
        assert_eq!(
            super::test_support::read_back(&kept.rt, &actor),
            super::test_support::read_back(&dropped.rt, &actor),
            "t={at} 手順 {step} で区切りの覚えの有無の画素が同じ"
        );
    }
    let runs = |rig: &Rig| {
        rig.rt
            .surfaces
            .get(&rig.place)
            .expect("描き手")
            .break_memo
            .runs
    };
    assert_eq!(
        runs(&kept),
        2,
        "覚えを使う側は装着と書き直しの 2 度だけ区切る"
    );
}
