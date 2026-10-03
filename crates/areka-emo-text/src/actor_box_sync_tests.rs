//! 毎フレームの箱の同期（task 7.4・要件 3.1〜3.3・3.5・3.9・6.1〜6.3・6.5・9.3）。
//!
//! 面はまだ作られない（箱の面の提示は 7.5）ので、ここでは「登録の状態」（装着先・配置の入力・
//! 置き場所）で見る。`World` は予約スロットの entity を得るためだけに使い、COM は使わない。
//! 箱の束はテストの中に持つ surfaces.txt の文面を畳んで作る（検体は読まない）。

use std::collections::BTreeMap;

use areka_emo_compose::{BoxLayout, BoxName, BoxPlacement, EmoWorld, fold_boxes};
use areka_parsers::shell::{parse, parse_boxes};
use areka_sakura::contract::{ActorKey, CueCommand, TalkCue};
use bevy_ecs::entity::Entity;
use bevy_ecs::prelude::World;
use log_capture_kit::{CapturedEvent, capture};

use super::test_support::{cue, spawn_reserved_slot};
use super::{ResolvedBalloonText, SurfaceKeyResolver, TextLayerRuntime, TextSlotBinding};
use crate::draw::DEFAULT_BALLOON_BACKGROUND;
use crate::place::{PlaceKey, TextPlace};
use crate::state::{SurfaceKeyOutcome, TextItem, TextLayerConfig};

/// サーフェス 0 は箱 a・b、1 は a を別の位置に、2 は b だけ、3 は a をサーフェスからはみ出す位置に置く。
/// 4 は箱を持たない（画像だけのサーフェス）。
const SHELL: &str = "\
balloon.a
{
size,100,50
validrect.left,10
validrect.right,-20
origin.x,-30
}
balloon.b
{
size,60,40
}
surface0
{
element1,balloon,a,10,20
element2,balloon,b,0,100
}
surface1
{
element1,balloon,a,30,40
}
surface2
{
element1,balloon,b,0,0
}
surface3
{
element1,balloon,a,150,0
}
";

/// シェルの窓のサーフェスの画像の大きさ（はみ出しの判定に使う native 原寸）。
pub(super) const SHELL_IMAGE: (u32, u32) = (200, 300);

/// はみ出しの警告の文言（発行点と 1 文字も違わないこと）。
const OVERFLOW_MESSAGE: &str =
    "箱がサーフェスの画像からはみ出している——採ったうえで、はみ出した部分は窓の端で切れる";

/// 登録したときの記録の文言（同じ置き場所で何もしないことを数えるのに使う）。
const REGISTER_MESSAGE: &str = "箱の置き場所を登録した（面は次の提示で作る）";

pub(super) fn layout() -> BoxLayout {
    let world = EmoWorld::build(&parse(SHELL));
    let (layout, report) = fold_boxes(&parse_boxes(SHELL), &BTreeMap::new(), &world);
    assert_eq!(report.issues, vec![], "文面は誤りを持たない");
    layout
}

pub(super) fn name(layout: &BoxLayout, surface: u32, name: &str) -> BoxName {
    layout
        .placements(surface)
        .iter()
        .find(|p| p.name.as_str() == name)
        .expect("文面にある箱の名前")
        .name
        .clone()
}

/// 偽の閉包: 数字ならその番号を表示、`-1` は非表示。
pub(super) fn resolver() -> SurfaceKeyResolver {
    Box::new(|key: &str| match key {
        "-1" => SurfaceKeyOutcome::Hide,
        _ => key
            .parse()
            .map_or(SurfaceKeyOutcome::Unresolved, SurfaceKeyOutcome::Show),
    })
}

pub(super) fn emote(actor: &str, key: &str) -> TalkCue {
    cue(actor, 0.0, CueCommand::Emote { key: key.into() })
}

pub(super) fn text(actor: &str, t: &str) -> TalkCue {
    cue(actor, 0.0, CueCommand::Text(t.into()))
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

pub(super) struct Fixture {
    pub(super) world: World,
    pub(super) rt: TextLayerRuntime,
    pub(super) layout: BoxLayout,
    pub(super) window: Entity,
    pub(super) slot: Entity,
}

pub(super) fn fixture() -> Fixture {
    let mut world = World::new();
    let (window, slot) = spawn_reserved_slot(&mut world);
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    let layout = layout();
    rt.set_box_layout(&mut world, layout.clone(), resolver(), vec![]);
    Fixture {
        world,
        rt,
        layout,
        window,
        slot,
    }
}

impl Fixture {
    pub(super) fn shell(&self, scale: f32) -> TextSlotBinding {
        let physical = (
            (SHELL_IMAGE.0 as f32 * scale).ceil() as u32,
            (SHELL_IMAGE.1 as f32 * scale).ceil() as u32,
        );
        TextSlotBinding::new(self.slot, self.window, scale, physical, SHELL_IMAGE)
    }

    pub(super) fn sync(&mut self, scale: Option<f32>) {
        let shell = scale.map(|k| self.shell(k));
        self.rt
            .sync_box_bindings(&mut self.world, &[(ActorKey::from("0"), shell)]);
    }

    pub(super) fn key(&self, surface: u32, box_name: &str) -> PlaceKey {
        PlaceKey {
            actor: ActorKey::from("0"),
            place: TextPlace::Box(name(&self.layout, surface, box_name)),
        }
    }

    /// 登録済みの装着先と置き場所（無ければ `None`）。
    fn registered(&self, surface: u32, box_name: &str) -> Option<(TextSlotBinding, BoxPlacement)> {
        let key = self.key(surface, box_name);
        let binding = self.rt.routing.get(&key).copied();
        let site = self.rt.box_sites.get(&key).cloned();
        assert_eq!(
            binding.is_some(),
            self.rt.layout_input.contains_key(&key),
            "装着先と配置の入力は対で登録する"
        );
        assert_eq!(
            binding.is_some(),
            site.is_some(),
            "装着先と置き場所は対で登録する"
        );
        binding.zip(site)
    }

    fn items(&self, surface: u32, box_name: &str) -> Vec<TextItem> {
        self.rt
            .state()
            .place_state(&self.key(surface, box_name))
            .map_or_else(Vec::new, |s| s.items().to_vec())
    }
}

fn messages(events: &[CapturedEvent], message: &str) -> usize {
    events.iter().filter(|e| e.message() == message).count()
}

/// 文字を持つ箱だけが登録される。置き場所が同じなら何度呼んでも登録し直さない。
#[test]
fn only_boxes_with_text_register_and_same_placement_is_a_no_op() {
    let mut f = fixture();
    f.rt.apply_cue(&emote("0", "0"));
    f.sync(Some(1.0));
    assert_eq!(f.registered(0, "a"), None, "文字の無い箱は登録しない");

    f.rt.apply_cue(&text("0", "あ"));
    let ((), events) = capture(|| f.sync(Some(1.0)));
    assert_eq!(messages(&events, REGISTER_MESSAGE), 1, "初めての登録");
    let (binding, site) = f.registered(0, "a").expect("文字を持つ箱 a");
    assert_eq!((site.element, site.x, site.y), (1, 10, 20));
    assert_eq!(binding.slot, f.slot, "差し込み口はシェルの窓のもの");
    assert_eq!(binding.window, f.window);
    assert_eq!(
        binding.image_size,
        (100, 50),
        "箱の大きさを画像の大きさとして持つ"
    );
    assert_eq!(f.registered(0, "b"), None, "文字の無い箱 b は登録しない");

    let ((), events) = capture(|| {
        f.sync(Some(1.0));
        f.sync(Some(1.0));
    });
    assert_eq!(
        messages(&events, REGISTER_MESSAGE),
        0,
        "置き場所が同じなら登録し直さない"
    );
    assert_eq!(f.registered(0, "a").map(|r| r.1), Some(site));
}

/// 配置の入力は普通のバルーンと同じ式で、箱の大きさを画像の大きさとし、背景は白（要件 3.1〜3.3）。
/// `validrect`・`origin` の負の値は箱の反対側の端からの距離のまま通る。
#[test]
fn layout_input_uses_the_box_size_as_the_image_size_with_white_background() {
    let mut f = fixture();
    f.rt.apply_cue(&emote("0", "0"));
    f.rt.apply_cue(&text("0", "あ"));
    f.sync(Some(1.0));

    let key = f.key(0, "a");
    let def = f.layout.def(&name(&f.layout, 0, "a")).expect("a の定義");
    let expected = ResolvedBalloonText::resolve_with_background(
        &def.model,
        def.size,
        DEFAULT_BALLOON_BACKGROUND,
    );
    assert_eq!(f.rt.layout_input.get(&key), Some(&expected));
    let region = expected.region;
    assert_eq!(region.left(), 10.0, "validrect.left は箱の左上から");
    assert_eq!(
        region.right(),
        80.0,
        "validrect.right の負の値は箱の右端から（100−20）"
    );
    assert_eq!(region.top(), 0.0, "書かれていない validrect.top は箱の上端");
    assert_eq!(
        region.bottom(),
        50.0,
        "書かれていない validrect.bottom は箱の下端"
    );
}

/// 拡大率が変わると登録が作り直される（要件 3.5）。文字の進み具合は保たれる。
#[test]
fn scale_change_rebuilds_the_registration_and_keeps_text() {
    let mut f = fixture();
    f.rt.apply_cue(&emote("0", "0"));
    f.rt.apply_cue(&text("0", "あい"));
    f.sync(Some(1.0));
    let before = f.registered(0, "a").expect("登録済み").0;

    let ((), events) = capture(|| f.sync(Some(2.0)));
    assert_eq!(messages(&events, REGISTER_MESSAGE), 1, "作り直す");
    let after = f.registered(0, "a").expect("登録済み").0;
    assert_ne!(before, after);
    assert_eq!(after.scale, 2.0);
    assert_eq!(
        after.surface_size,
        (200, 100),
        "物理寸は箱の大きさ × 拡大率"
    );
    assert_eq!(
        after.image_size,
        (100, 50),
        "画像の大きさは拡大率に依らない"
    );
    assert_eq!(
        f.items(0, "a"),
        vec![TextItem::glyph("あ"), TextItem::glyph("い")],
        "文字の進み具合は保つ"
    );
}

/// 同じ名前の箱が新しいサーフェスにあれば、その位置で登録し直す（要件 6.1）。
#[test]
fn same_name_on_the_next_surface_moves_the_registration() {
    let mut f = fixture();
    f.rt.apply_cue(&emote("0", "0"));
    f.rt.apply_cue(&text("0", "あ"));
    f.sync(Some(1.0));

    f.rt.apply_cue(&emote("0", "1"));
    f.sync(Some(1.0));
    let (_, site) = f.registered(1, "a").expect("新しいサーフェスの a");
    assert_eq!((site.x, site.y), (30, 40), "新しいサーフェスでの位置");
    assert_eq!(f.items(1, "a"), vec![TextItem::glyph("あ")]);
}

/// 名前が外れた箱は登録が片付き文字は残り、戻ると登録し直されて文字の進み具合が保たれる
/// （要件 6.2・6.3）。今の行き先ではない箱も、文字を持てば新しいサーフェスで登録される（要件 6.5）。
#[test]
fn box_leaving_the_surface_is_unregistered_and_comes_back_with_its_text() {
    let mut f = fixture();
    f.rt.apply_cue(&emote("0", "0"));
    f.rt.apply_cue(&text("0", "あ"));
    f.rt.apply_cue(&select("0", "b"));
    f.rt.apply_cue(&text("0", "い"));
    f.sync(Some(1.0));
    assert!(f.registered(0, "a").is_some() && f.registered(0, "b").is_some());

    // サーフェス 2 には b だけ: a は片付き、行き先でない b も登録されたまま（要件 6.5）。
    f.rt.apply_cue(&emote("0", "2"));
    f.sync(Some(1.0));
    assert_eq!(f.registered(0, "a"), None, "名前が外れた箱は片付ける");
    assert_eq!(f.items(0, "a"), vec![TextItem::glyph("あ")], "文字は残る");
    let (_, b_site) = f.registered(2, "b").expect("b はサーフェス 2 にもある");
    assert_eq!((b_site.x, b_site.y), (0, 0));

    // 箱の無い（表に無い）サーフェス 4 と非表示: どの箱も片付く（要件 6.4・6.6）。
    f.rt.apply_cue(&emote("0", "4"));
    f.sync(Some(1.0));
    assert_eq!(f.registered(0, "b"), None);
    f.rt.apply_cue(&emote("0", "-1"));
    f.sync(Some(1.0));
    assert!(f.rt.box_sites.is_empty(), "非表示では何も登録しない");

    // 戻る: 保持していた文字ごと登録し直す。
    f.rt.apply_cue(&emote("0", "0"));
    f.sync(Some(1.0));
    assert!(f.registered(0, "a").is_some() && f.registered(0, "b").is_some());
    assert_eq!(f.items(0, "a"), vec![TextItem::glyph("あ")]);
    assert_eq!(f.items(0, "b"), vec![TextItem::glyph("い")]);
}

/// 箱を隠す印が立っている、またはシェルの窓が未確立なら登録しない（文字は残る）。
#[test]
fn hidden_scope_or_missing_shell_window_unregisters() {
    let mut f = fixture();
    f.rt.apply_cue(&emote("0", "0"));
    f.rt.apply_cue(&text("0", "あ"));
    f.sync(None);
    assert_eq!(f.registered(0, "a"), None, "シェルの窓が未確立");

    f.sync(Some(1.0));
    assert!(f.registered(0, "a").is_some());
    f.rt.hide_boxes(&ActorKey::from("0"));
    f.sync(Some(1.0));
    assert_eq!(f.registered(0, "a"), None, "隠す印が立っている");
    assert_eq!(f.items(0, "a"), vec![TextItem::glyph("あ")], "文字は残る");

    // 渡されないスコープもシェルの窓が未確立と同じ。
    f.rt.hidden_boxes.clear();
    f.sync(Some(1.0));
    assert!(f.registered(0, "a").is_some());
    f.rt.sync_box_bindings(&mut f.world, &[]);
    assert_eq!(f.registered(0, "a"), None, "渡されないスコープ");
}

/// 箱の束を差し替えると登録も片付く（要件 6.8）。
#[test]
fn replacing_the_bundle_drops_registrations() {
    let mut f = fixture();
    f.rt.apply_cue(&emote("0", "0"));
    f.rt.apply_cue(&text("0", "あ"));
    f.sync(Some(1.0));
    assert!(!f.rt.box_sites.is_empty());

    f.rt.set_box_layout(&mut f.world, layout(), resolver(), vec![]);
    assert!(f.rt.box_sites.is_empty(), "置き場所の登録は残らない");
    assert!(
        f.rt.routing.keys().all(|k| k.place == TextPlace::Balloon),
        "箱の装着先は残らない"
    );
}

/// はみ出す置き場所は採ったうえで、最初に面にするとき 1 度だけ警告する（サーフェス番号・名前・
/// 箱の四角・サーフェスの大きさ）。収まる置き場所は警告しない。
#[test]
fn overflowing_box_is_kept_and_warned_once() {
    let mut f = fixture();
    f.rt.apply_cue(&emote("0", "0"));
    f.rt.apply_cue(&text("0", "あ"));
    let ((), events) = capture(|| f.sync(Some(1.0)));
    assert_eq!(
        messages(&events, OVERFLOW_MESSAGE),
        0,
        "収まる箱は警告しない"
    );

    f.rt.apply_cue(&emote("0", "3"));
    let ((), events) = capture(|| f.sync(Some(1.0)));
    let warns: Vec<&CapturedEvent> = events
        .iter()
        .filter(|e| e.message() == OVERFLOW_MESSAGE)
        .collect();
    assert_eq!(warns.len(), 1, "1 度だけ");
    let warn = warns[0];
    assert_eq!(warn.level, tracing::Level::WARN);
    assert_eq!(warn.field("surface"), Some("3"));
    assert_eq!(warn.field_str("name"), Some("a"));
    assert_eq!(warn.field("rect"), Some("(150, 0, 100, 50)"));
    assert_eq!(warn.field("surface_size"), Some("(200, 300)"));
    assert!(f.registered(3, "a").is_some(), "はみ出しても採る");

    // 離れて戻っても、拡大率が変わって作り直しても 2 度目は出さない。
    f.rt.apply_cue(&emote("0", "0"));
    f.sync(Some(1.0));
    f.rt.apply_cue(&emote("0", "3"));
    let ((), events) = capture(|| {
        f.sync(Some(1.0));
        f.sync(Some(2.0));
    });
    assert_eq!(messages(&events, OVERFLOW_MESSAGE), 0, "2 度目は出さない");
}

/// 箱の面を窓の子のどこへ挿すか: 差し込み口の直後（面を持つ箱が他に無いとき）。
/// 差し込み口が窓の子に無ければ挿す位置は無い。
#[test]
fn box_child_index_is_right_after_the_slot() {
    let mut f = fixture();
    f.rt.apply_cue(&emote("0", "0"));
    f.rt.apply_cue(&text("0", "あ"));
    f.sync(Some(1.0));
    let key = f.key(0, "a");
    let picture = f.world.spawn_empty().id();
    let other = f.world.spawn_empty().id();
    assert_eq!(
        f.rt.box_child_index(&[other, f.slot, picture], &key),
        Some(2)
    );
    assert_eq!(f.rt.box_child_index(&[other, picture], &key), None);
    assert_eq!(
        f.rt.box_child_index(&[f.slot], &f.key(0, "b")),
        None,
        "登録されていない箱"
    );
}
