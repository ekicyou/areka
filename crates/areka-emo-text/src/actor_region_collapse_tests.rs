//! # actor_region_collapse_tests — 潰れた描画範囲は状態の変わり目に 1 回だけ記録する（件 2）
//!
//! 出典 spec: `areka-P0-balloon-text-bugs`（要件 **2.3**〜**2.5**・**2.7**・**4.2**、design
//! 「潰れた描画範囲（件 2）」）。
//!
//! 解決（[`TextRegion::resolve`](crate::region::TextRegion::resolve)）は毎フレーム走るので記録を
//! 出さない（`region_tests.rs` が固定する）。記録は登録口
//! （[`TextLayerRuntime::note_region_collapse`]）が、場所ごとの記憶と突き合わせて
//! 「潰れに入った」ときに `warn!` 1 回、「戻った」ときに `info!` 1 回だけ書く。
//!
//! ## 固定するもの（GPU なし・登録の口だけ）
//!
//! 1. 潰れた領域の登録で警告 1 回（場所の名前・4 辺・画像の大きさ）。
//! 2. 同じ値の登録し直し（装着の口の再呼び出し・値の同じ再追従）で 0 回。
//! 3. 正の領域で情報 1 回（幅と高さ）・さらに正のままの登録で 0 回。
//! 4. 箱の登録→外し→登録で 1 回（記憶は登録を外しても消えない）。
//! 5. 箱を隠す・出すで 0 回。箱の束を差し替えても記憶は残り、正の定義で戻れば情報 1 回。
//!
//! 片付け（1.3）と提示（1.4）の檻は同じファイルの後の節に足す。
//!
//! ## 0 件の主張が恒真にならないようにする
//!
//! 捕捉窓の内側で対照の `error!` を 1 件出し、その 1 件が数えられていることを件数の主張と
//! 同時に確かめる（`actor_region_warn_tests.rs` と同じ流儀）。

use std::collections::BTreeMap;

use areka_emo_compose::{BoxLayout, EmoWorld, fold_boxes};
use areka_parsers::balloon::{BalloonModel, parse_str};
use areka_parsers::shell::{parse, parse_boxes};
use areka_sakura::contract::ActorKey;
use bevy_ecs::entity::Entity;
use bevy_ecs::prelude::World;
use log_capture_kit::{CapturedEvent, capture};

use super::box_sync_tests::{emote, resolver, text};
use super::test_support::spawn_reserved_slot;
use super::{TextLayerRuntime, TextSlotBinding};
use crate::place::{PlaceKey, TextPlace};
use crate::state::TextLayerConfig;

/// 潰れに入ったときの警告の文（実機の確かめが探す語・design「記録の一覧」）。
const COLLAPSED: &str =
    "文字の描画範囲が潰れている（幅か高さが 0 以下）——このバルーンの文字は描かない";
/// 戻ったときの情報の文。
const RECOVERED: &str = "文字の描画範囲が戻った——次のフレームから文字を描く";

/// 普通のバルーンの画像の大きさ（image px）。
const IMAGE: (u32, u32) = (100, 80);

/// 左 50・右 100−60＝40 で幅が負に潰れる定義。
const COLLAPSED_BALLOON: &str = concat!("validrect.left,50\n", "validrect.right,-60\n",);

/// 左 10・右 100−10＝90 で幅 80・高さ 80 の正の定義。
const HEALTHY_BALLOON: &str = concat!("validrect.left,10\n", "validrect.right,-10\n",);

/// 箱 c は幅が潰れている（左 60・右 100−50＝50）。サーフェス 0 と 1 の両方に置く。
const SHELL_COLLAPSED: &str = "\
balloon.c
{
size,100,50
validrect.left,60
validrect.right,-50
}
surface0
{
element1,balloon,c,0,0
}
surface1
{
element1,balloon,c,10,10
}
";

/// 同じ名前の箱 c を正の範囲で定義し直した束（左 10・右 100−10＝90・高さ 50）。
const SHELL_HEALTHY: &str = "\
balloon.c
{
size,100,50
validrect.left,10
validrect.right,-10
}
surface0
{
element1,balloon,c,0,0
}
";

/// シェルの窓のサーフェスの画像の大きさ。
const SHELL_IMAGE: (u32, u32) = (200, 300);

fn model(source: &str) -> BalloonModel {
    parse_str(source, None)
}

fn box_layout(source: &str) -> BoxLayout {
    let world = EmoWorld::build(&parse(source));
    let (layout, report) = fold_boxes(&parse_boxes(source), &BTreeMap::new(), &world);
    assert_eq!(report.issues, vec![], "文面は誤りを持たない");
    layout
}

/// 捕捉窓の中で `f` を走らせ、`(潰れの警告, 戻りの情報)` の一覧を返す。対照の `error!` が
/// 数えられていることもここで確かめる。
fn capturing(f: impl FnOnce()) -> (Vec<CapturedEvent>, Vec<CapturedEvent>) {
    let ((), events) = capture(|| {
        tracing::error!("捕捉窓が生きていることの対照イベント");
        f();
    });
    assert_eq!(
        events
            .iter()
            .filter(|e| e.level == tracing::Level::ERROR)
            .count(),
        1,
        "捕捉窓の対照イベントが数えられていない。この窓の件数の主張は証拠にならない"
    );
    let pick = |level: tracing::Level, message: &str| -> Vec<CapturedEvent> {
        events
            .iter()
            .filter(|e| e.level == level && e.message() == message)
            .cloned()
            .collect()
    };
    (
        pick(tracing::Level::WARN, COLLAPSED),
        pick(tracing::Level::INFO, RECOVERED),
    )
}

fn number(event: &CapturedEvent, name: &str) -> f32 {
    let raw = event
        .field(name)
        .unwrap_or_else(|| panic!("欄 {name} が記録に載っていない"));
    raw.parse()
        .unwrap_or_else(|_| panic!("欄 {name} の値 {raw} を数値として読めない"))
}

struct Balloon {
    rt: TextLayerRuntime,
    window: Entity,
    slot: Entity,
    actor: ActorKey,
}

impl Balloon {
    fn new() -> Balloon {
        let mut world = World::new();
        let (window, slot) = spawn_reserved_slot(&mut world);
        Balloon {
            rt: TextLayerRuntime::new(TextLayerConfig::default()),
            window,
            slot,
            actor: ActorKey::from("1"),
        }
    }

    fn binding(&self) -> TextSlotBinding {
        TextSlotBinding::new(self.slot, self.window, 1.0, IMAGE, IMAGE)
    }

    /// 装着の口（`register_actor_view` の内側）。
    fn attach(&mut self, source: &str) {
        let binding = self.binding();
        self.rt
            .register_actor_binding(self.actor.clone(), binding, &model(source));
    }

    /// 再追従の口（`refresh_actor_scale` の内側）。
    fn refresh(&mut self, source: &str) -> bool {
        let binding = self.binding();
        self.rt
            .refresh_actor_binding(&self.actor, binding, &model(source))
    }
}

// ── 普通のバルーン（要件 2.3〜2.5） ──

/// 潰れた領域の装着で警告 1 回。どのバルーンのどの値かが読める（名前・4 辺・画像の大きさ）。
#[test]
fn attaching_a_collapsed_region_warns_once_with_the_name_and_values() {
    let mut b = Balloon::new();
    let (warns, infos) = capturing(|| b.attach(COLLAPSED_BALLOON));

    assert_eq!(warns.len(), 1, "潰れに入ったら警告 1 回（要件 2.3）");
    assert_eq!(infos.len(), 0);
    let warn = &warns[0];
    assert_eq!(warn.field_str("balloon"), Some("スコープ1のバルーン"));
    assert_eq!(number(warn, "left"), 50.0);
    assert_eq!(number(warn, "top"), 0.0);
    assert_eq!(number(warn, "right"), 40.0);
    assert_eq!(number(warn, "bottom"), 80.0);
    assert_eq!(warn.field("image_size"), Some("(100.0, 80.0)"));
}

/// 同じ値の登録し直し（装着の口の再呼び出し＝隠して出し直す結線・値の同じ再追従）は 0 回。
#[test]
fn registering_the_same_collapsed_region_again_logs_nothing() {
    let mut b = Balloon::new();
    b.attach(COLLAPSED_BALLOON);

    let (warns, infos) = capturing(|| {
        b.attach(COLLAPSED_BALLOON);
        assert!(!b.refresh(COLLAPSED_BALLOON), "値が同じなら再追従しない");
        b.attach(COLLAPSED_BALLOON);
    });
    assert_eq!(
        (warns.len(), infos.len()),
        (0, 0),
        "潰れたままの登録し直しは記録しない（要件 2.2・2.5）"
    );
}

/// 正に戻る再追従で情報 1 回（幅と高さ）。正のままの登録し直しは 0 回。
#[test]
fn returning_to_a_positive_region_logs_info_once() {
    let mut b = Balloon::new();
    b.attach(COLLAPSED_BALLOON);

    let (warns, infos) = capturing(|| {
        assert!(b.refresh(HEALTHY_BALLOON), "領域が変われば再追従する");
    });
    assert_eq!(warns.len(), 0);
    assert_eq!(infos.len(), 1, "戻ったら情報 1 回（要件 2.4）");
    assert_eq!(infos[0].field_str("balloon"), Some("スコープ1のバルーン"));
    assert_eq!(number(&infos[0], "width"), 80.0);
    assert_eq!(number(&infos[0], "height"), 80.0);

    let (warns, infos) = capturing(|| {
        b.attach(HEALTHY_BALLOON);
        assert!(!b.refresh(HEALTHY_BALLOON));
    });
    assert_eq!((warns.len(), infos.len()), (0, 0), "正のままは記録しない");
}

/// 初めから正の領域は、装着しても戻りの情報を出さない（記憶に無い場所は「戻った」ではない）。
#[test]
fn attaching_a_healthy_region_logs_nothing() {
    let mut b = Balloon::new();
    let (warns, infos) = capturing(|| b.attach(HEALTHY_BALLOON));
    assert_eq!((warns.len(), infos.len()), (0, 0));
}

/// 正→潰れ→正→潰れの往復は、変わり目ごとに 1 回ずつ。
#[test]
fn each_transition_logs_exactly_once() {
    let mut b = Balloon::new();
    let (warns, infos) = capturing(|| {
        b.attach(HEALTHY_BALLOON);
        b.refresh(COLLAPSED_BALLOON);
        b.refresh(HEALTHY_BALLOON);
        b.refresh(COLLAPSED_BALLOON);
    });
    assert_eq!((warns.len(), infos.len()), (2, 1));
}

/// 名前を入れたバルーンは、その名前で記録する（粗い定義の警告と同じ名前の欄）。
#[test]
fn the_balloon_field_uses_the_label_the_wiring_set() {
    let mut b = Balloon::new();
    b.rt.set_balloon_label(&b.actor.clone(), "小さいバルーン".to_owned());
    let (warns, _) = capturing(|| b.attach(COLLAPSED_BALLOON));
    assert_eq!(warns.len(), 1);
    assert_eq!(warns[0].field_str("balloon"), Some("小さいバルーン"));
}

// ── 箱（要件 2.7） ──

struct Shell {
    world: World,
    rt: TextLayerRuntime,
    window: Entity,
    slot: Entity,
    picture: Option<u32>,
}

impl Shell {
    fn new() -> Shell {
        let mut world = World::new();
        let (window, slot) = spawn_reserved_slot(&mut world);
        let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
        rt.set_box_layout(&mut world, box_layout(SHELL_COLLAPSED), resolver(), vec![]);
        Shell {
            world,
            rt,
            window,
            slot,
            picture: None,
        }
    }

    /// 台本の `\s` と絵の差し替えが届いた状態にして、箱 c に文字を置く。
    fn speak_into_box(&mut self) {
        self.show(0);
        self.rt.apply_cue(&text("0", "あ"));
    }

    fn show(&mut self, surface: u32) {
        self.rt.apply_cue(&emote("0", &surface.to_string()));
        self.picture = Some(surface);
    }

    /// 毎フレームの箱の同期（拡大率 1）。
    fn sync(&mut self) {
        let shell = self.picture.map(|n| {
            (
                TextSlotBinding::new(self.slot, self.window, 1.0, SHELL_IMAGE, SHELL_IMAGE),
                n,
            )
        });
        self.rt
            .sync_box_bindings(&mut self.world, &[(ActorKey::from("0"), shell)]);
    }

    fn box_key(&self) -> PlaceKey {
        self.rt
            .box_sites
            .keys()
            .find(|k| k.place != TextPlace::Balloon)
            .cloned()
            .expect("箱 c が登録されている")
    }
}

/// 潰れた箱の登録で警告 1 回。名前の欄は `balloon.*`ブレスの名前。
#[test]
fn registering_a_collapsed_box_warns_once_with_the_box_name() {
    let mut s = Shell::new();
    s.speak_into_box();
    let (warns, infos) = capturing(|| s.sync());

    assert_eq!(
        (warns.len(), infos.len()),
        (1, 0),
        "箱も潰れに入ったら 1 回"
    );
    let TextPlace::Box(name) = s.box_key().place else {
        unreachable!("箱の場所");
    };
    let warn = &warns[0];
    assert_eq!(warn.field_str("balloon"), Some(name.as_str()));
    assert_eq!(number(warn, "left"), 60.0);
    assert_eq!(number(warn, "right"), 50.0);
    assert_eq!(number(warn, "top"), 0.0);
    assert_eq!(number(warn, "bottom"), 50.0);
    assert_eq!(warn.field("image_size"), Some("(100.0, 50.0)"));
}

/// 箱の登録→外し→登録（絵の切り替えで置き場所が変わる）で記録は 1 回だけ。
#[test]
fn register_unregister_register_a_collapsed_box_warns_once() {
    let mut s = Shell::new();
    s.speak_into_box();
    let (warns, infos) = capturing(|| {
        s.sync();
        s.show(1);
        s.sync();
        s.picture = None;
        s.sync();
        assert!(s.rt.box_sites.is_empty(), "絵が無いので登録を外した");
        s.show(0);
        s.sync();
        assert!(!s.rt.box_sites.is_empty(), "登録し直した");
    });
    assert_eq!(
        (warns.len(), infos.len()),
        (1, 0),
        "記憶は登録を外しても消えない"
    );
}

/// 箱を隠す・出す（隠す印で登録を外し、印を下ろして登録し直す）で 0 回。
#[test]
fn hiding_and_showing_a_collapsed_box_logs_nothing() {
    let mut s = Shell::new();
    s.speak_into_box();
    s.sync();

    let (warns, infos) = capturing(|| {
        s.rt.hide_boxes(&ActorKey::from("0"));
        s.sync();
        assert!(s.rt.box_sites.is_empty(), "隠す印で登録を外した");
        s.rt.hidden_boxes.clear();
        s.sync();
        assert!(!s.rt.box_sites.is_empty(), "印を下ろして登録し直した");
    });
    assert_eq!(
        (warns.len(), infos.len()),
        (0, 0),
        "隠す・出すで記録は増えない（要件 2.5）"
    );
}

/// 箱の束を差し替えても記憶は残る: 潰れたままの束なら 0 回、正の束で戻れば情報 1 回。
#[test]
fn replacing_the_bundle_keeps_the_memory_and_recovery_logs_info_once() {
    let mut s = Shell::new();
    s.speak_into_box();
    s.sync();

    let (warns, infos) = capturing(|| {
        s.rt.set_box_layout(
            &mut s.world,
            box_layout(SHELL_COLLAPSED),
            resolver(),
            vec![],
        );
        s.speak_into_box();
        s.sync();
    });
    assert_eq!(
        (warns.len(), infos.len()),
        (0, 0),
        "潰れたままの束の差し替え"
    );

    let (warns, infos) = capturing(|| {
        s.rt.set_box_layout(&mut s.world, box_layout(SHELL_HEALTHY), resolver(), vec![]);
        s.speak_into_box();
        s.sync();
    });
    assert_eq!(
        (warns.len(), infos.len()),
        (0, 1),
        "正の束で戻ったら情報 1 回"
    );
    assert_eq!(number(&infos[0], "width"), 80.0);
    assert_eq!(number(&infos[0], "height"), 50.0);
}
