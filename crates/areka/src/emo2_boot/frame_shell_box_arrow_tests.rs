// =============================================================================
// 箱の矢印の押下と箱の上のホイール（balloon-scroll-arrows task 6.3・要件 7.4〜7.6・11.1・11.2）
//
// 親（`frame_shell_box_integration_tests.rs`）の `Cage`——実 emo2 fixture ＋ 実 GPU で本番の
// `emo2_frame_system` を回す檻——で箱 `talk` にあふれる行を流し、矢印を出す。箱の四角の写し
// （`shown_boxes`）と矢印の当たりは提示（GPU）でしか埋まらないので、写しを作らず本物の提示で埋める。
// 押下はシェルの窓の受け口（`on_char_pointer_pressed`）へ、ホイールはキャラ窓の見張り
// （`char_watch_with`・OS の行数は注入）へ渡す。受け口が読む `Emo2Wiring` は呼ぶ間だけ world へ置く。
// =============================================================================

use std::cell::Cell;
use std::sync::Arc;

use areka_emo_present::balloon_marker::{MarkerImage, MarkerPair};
use areka_emo_text::place::{PlaceKey, TextPlace};
use areka_emo_text::scroll_marker::{MarkerKind, ScrollChange, ScrollDir, WheelLines};
use areka_kanade::{KanadeMsg, MouseEventKind};
use bevy_ecs::hierarchy::ChildOf;
use wintf::ecs::pointer::{Phase, PointerState, WheelDelta};
use wintf::ecs::{PhysicalPoint, Point};

use crate::emo2_boot::hit_region::HitRegion;
use crate::emo2_boot::user_break_cue::NoUserBreakSignal;
use crate::input_events::char_watch::char_watch_with;
use crate::input_events::user_break::{UserBreakWiring, drain_no_user_break_signals};
use crate::input_events::{MouseWiring, RegionSource, on_char_pointer_pressed};
use crate::placement::spawn::CharWindowMarker;

use super::*;

/// 10×5 の不透明な矢印。
fn arrow() -> Arc<MarkerImage> {
    Arc::new(MarkerImage {
        width: 10,
        height: 5,
        bgra: [255u8; 4].repeat(10 * 5),
    })
}

fn mock(scope: u32, x: i64, y: i64) -> HitRegion {
    HitRegion {
        scope,
        region: None,
        surface_point: (x, y),
    }
}

fn talk_place(cage: &Cage) -> PlaceKey {
    let actor = ActorKey::from("0");
    let shown = cage.wiring.runtime.borrow().shown_boxes(&actor).to_vec();
    PlaceKey {
        actor,
        place: TextPlace::Box(shown[0].name.clone()),
    }
}

/// 箱 `talk` に `lines` 行を流して提示する。`lines` が箱からあふれるなら手動の位置（自動の 1 行上）へ
/// 入れて提示し直し、上下の矢印を出す。
fn talk_lines(lines: usize) -> Cage {
    let mut cage = Cage::boot(SHELL, &[0]);
    cage.wiring
        .runtime
        .borrow_mut()
        .set_box_markers(BTreeMap::from([(
            ActorKey::from("0"),
            BTreeMap::from([(
                fold_boxes(
                    &parse_boxes(SHELL),
                    &BTreeMap::new(),
                    &EmoWorld::build(&parse(SHELL)),
                )
                .0
                .placements(0)[0]
                    .name
                    .clone(),
                MarkerPair {
                    up: Some(arrow()),
                    down: Some(arrow()),
                },
            )]),
        )]));
    cage.picture(0, BOX_SURFACE_ID);
    cage.surface(0, BOX_SURFACE);
    for i in 0..lines {
        if i > 0 {
            cage.cue(0, CueCommand::NewLine { ratio: 1.0 });
        }
        cage.text(0, "あ");
    }
    cage.frame();
    cage.frame();
    assert_eq!(
        cage.shown_boxes(0),
        vec!["talk".to_owned()],
        "前提: 箱の文字が出ている"
    );
    if lines > 1 {
        let place = talk_place(&cage);
        assert_eq!(
            cage.wiring
                .runtime
                .borrow_mut()
                .scroll_manual_at(&place, ScrollDir::Older, 1),
            ScrollChange::Started,
            "前提: 古い行が押し出されていて手動の位置に入る"
        );
        cage.frame();
    }
    cage
}

impl Cage {
    /// 箱の四角の中で `kind` の矢印に当たる点（シェルの窓のクライアント物理 px）。
    fn arrow_point(&self, kind: MarkerKind) -> (i32, i32) {
        let place = talk_place(self);
        let rt = self.wiring.runtime.borrow();
        let rect = rt.shown_boxes(&place.actor)[0].rect;
        (rect.top as i32..rect.bottom as i32)
            .flat_map(|y| (rect.left as i32..rect.right as i32).map(move |x| (x, y)))
            .find(|&(x, y)| rt.marker_at_point(&place, (x as f32, y as f32)) == Some(kind))
            .expect("前提: 箱に矢印が出ている")
    }

    /// 箱の四角の中で、矢印に当たらない点。
    fn body_point(&self) -> (i32, i32) {
        let place = talk_place(self);
        let rt = self.wiring.runtime.borrow();
        let rect = rt.shown_boxes(&place.actor)[0].rect;
        let at = (rect.left as i32 + 1, rect.top as i32 + 1);
        assert_eq!(rt.marker_at_point(&place, (at.0 as f32, at.1 as f32)), None);
        at
    }

    /// 送りが効いていない（自動の 1 行上のまま＝新しい側へ 1 行で自動へ戻る）。
    fn box_unmoved(&self) -> bool {
        let place = talk_place(self);
        matches!(
            self.wiring
                .runtime
                .borrow_mut()
                .scroll_manual_at(&place, ScrollDir::Newer, 1),
            ScrollChange::Stopped(_)
        )
    }

    fn shell_window(&mut self) -> Entity {
        self.world
            .query::<(Entity, &CharWindowMarker)>()
            .iter(&self.world)
            .find(|(_, m)| m.scope == 0)
            .map(|(e, _)| e)
            .expect("scope0 のシェルの窓")
    }

    /// `Emo2Wiring` を world へ置いて `f` を回し、戻す。
    fn with_wiring<R>(&mut self, f: impl FnOnce(&mut World) -> R) -> R {
        let wiring = std::mem::replace(&mut self.wiring, placeholder_wiring());
        self.world.insert_non_send(wiring);
        let out = f(&mut self.world);
        self.wiring = self
            .world
            .remove_non_send::<Emo2Wiring>()
            .expect("置いた結線");
        out
    }

    /// シェルの窓へ左押下を 1 回渡し、受け口の答えを返す。
    fn press(&mut self, (x, y): (i32, i32)) -> bool {
        let window = self.shell_window();
        let state = PointerState {
            client_point: Point { x, y },
            left_down: true,
            ..Default::default()
        };
        self.with_wiring(|world| {
            on_char_pointer_pressed(world, window, window, &Phase::Bubble(state))
        })
    }

    /// シェルの窓の子を持ち主にして縦の回転を 1 回渡し、送られたホイールの知らせと OS の行数を
    /// 読んだ回数を返す。
    fn wheel(&mut self, (x, y): (i32, i32), vertical: i16) -> (Vec<i32>, usize) {
        let window = self.shell_window();
        let (tx, rx) = mpsc::channel();
        self.world
            .insert_non_send(MouseWiring::new(tx, RegionSource::Mock(mock)));
        let owner = self.world.spawn(ChildOf(window)).id();
        self.world.entity_mut(owner).insert(PointerState {
            client_point: PhysicalPoint::new(x, y),
            wheel: WheelDelta {
                vertical,
                horizontal: 0,
            },
            ..Default::default()
        });
        let reads = Cell::new(0);
        self.with_wiring(|world| {
            char_watch_with(world, || {
                reads.set(reads.get() + 1);
                WheelLines::Lines(3)
            })
        });
        self.world.despawn(owner);
        self.world.remove_non_send::<MouseWiring>();
        let sent = rx
            .try_iter()
            .filter_map(|m| match m {
                KanadeMsg::Mouse(i) => match i.kind {
                    MouseEventKind::Wheel { delta } => Some(delta),
                    _ => None,
                },
                _ => None,
            })
            .collect();
        (sent, reads.get())
    }
}

/// 話し終えた後の箱の矢印の押下は箱を送って消費し（クリックにも中断にもしない）、矢印の外の
/// 押下は今までどおり箱の押下の判断へ流れる（要件 7.4）。
#[test]
fn finished_box_arrow_press_scrolls_and_is_not_a_click() {
    let mut cage = talk_lines(8);
    let (flag_tx, flag_rx) = mpsc::channel();
    let (lifecycle_tx, lifecycle_rx) = mpsc::channel();
    let (kanade_tx, kanade_rx) = mpsc::channel();
    cage.world
        .insert_non_send(UserBreakWiring::new(flag_rx, lifecycle_tx, kanade_tx));
    drop(flag_tx);

    let up = cage.arrow_point(MarkerKind::Up);
    assert!(
        cage.press(up),
        "矢印の押下は箱の前段が処理する（シェルへ送らない）"
    );
    assert!(!cage.box_unmoved(), "上の矢印で古い側へ送れた");
    assert!(
        lifecycle_rx.try_recv().is_err() && kanade_rx.try_recv().is_err(),
        "中断へ流れていない"
    );

    // 矢印の外: 送らず、今までどおりの箱の押下（本文の上の単発の左押下はシェルへの操作）。
    // GPU の装置の許可は 1 つずつなので、前の檻を返してから組み直す。
    drop(cage);
    let mut cage = talk_lines(8);
    let body = cage.body_point();
    assert!(!cage.press(body), "矢印の外は今までどおりシェルへの操作");
    assert!(cage.box_unmoved(), "矢印の外の押下で箱は送られない");
}

/// 話している最中の箱の矢印の押下は送らず、今までどおりの箱の押下（中断）へ流れる。
#[test]
fn talking_box_arrow_press_flows_to_the_usual_box_press() {
    let mut cage = talk_lines(8);
    let (flag_tx, flag_rx) = mpsc::channel();
    let (lifecycle_tx, _lifecycle_rx) = mpsc::channel();
    let (kanade_tx, _kanade_rx) = mpsc::channel();
    cage.world
        .insert_non_send(UserBreakWiring::new(flag_rx, lifecycle_tx, kanade_tx));
    flag_tx.send(NoUserBreakSignal::TalkStarted).unwrap();
    drain_no_user_break_signals(&mut cage.world);

    let up = cage.arrow_point(MarkerKind::Up);
    cage.press(up);
    assert!(cage.box_unmoved(), "話している最中は矢印として扱わない");
}

/// 隠れた行のある箱の上のホイールは箱を送り、`OnMouseWheel` にしない。OS の行数は 1 回だけ読む
/// （要件 7.5）。
#[test]
fn wheel_over_a_box_with_hidden_lines_scrolls_the_box_not_on_mouse_wheel() {
    let mut cage = talk_lines(8);
    let body = cage.body_point();
    let (sent, reads) = cage.wheel(body, 120);
    assert_eq!(sent, Vec::<i32>::new(), "OnMouseWheel を送らない");
    assert_eq!(reads, 1, "回転を箱へ回した時だけ OS の行数を読む");
    assert!(!cage.box_unmoved(), "箱が古い側へ送れた");
}

/// 隠れた行の無い箱の上のホイールは今までどおり `OnMouseWheel` になる（要件 7.6）。
#[test]
fn wheel_over_a_box_without_hidden_lines_is_on_mouse_wheel_as_before() {
    let mut cage = talk_lines(1);
    let place = talk_place(&cage);
    assert!(
        !cage.wiring.runtime.borrow().has_hidden_lines_at(&place),
        "前提: 隠れた行が無い"
    );
    let rect = cage.wiring.runtime.borrow().shown_boxes(&place.actor)[0].rect;
    let (sent, reads) = cage.wheel((rect.left as i32 + 1, rect.top as i32 + 1), 120);
    assert_eq!(sent, vec![120], "今までどおり OnMouseWheel");
    assert_eq!(reads, 0, "OS の行数は読まない");
}
