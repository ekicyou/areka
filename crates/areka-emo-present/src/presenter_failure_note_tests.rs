//! 同じ原因の装置の失敗を、対象ごとに 1 回にまとめる（`animated-surface-log-flood` 要件 1.6・4.3・8.5）。
//!
//! 同じ対象で、成立をはさまずに同じ原因（文脈＋HRESULT）の装置の失敗が続くあいだ、最初の 1 回だけ
//! `error!`、2 回目からは同じ文言・同じ欄の `debug!` で残る。成立の後は `error!` に戻る。面が替わった
//! 適用でも、原因が同じ続きなら `debug!` のままである。`GraphicsCore` が無い道も同じ形でまとめる。
//!
//! 数えるのは失敗の行の文言だけ（同じ捕捉の中のほかの行に左右されない）。陽性の対照として、
//! どの場面も最初の 1 本が ERROR で出ていることを同じ捕捉の中で確かめる（捕捉 0 件で緑にならない）。

use super::*;

use std::time::Duration;

use tracing::Level as L;

use crate::display::{DisplayFault, arm_display_fault, clear_display_fault};

use super::test_support::{
    CapturedEvent, build_two_face_assets, capture, make_world_with_gpu, show_ok,
    spawn_window_with_dpi,
};

const TARGET: TargetId = TargetId(0);
const FACE_A: u32 = 1000;
const FACE_B: u32 = 3000;

/// 記録の失敗の行の文言（`command.rs` の `device_err`）。
const DEVICE_LINE: &str = "D3D/DXGI 呼び出しが失敗";
/// `GraphicsCore` が無い道の行の文言（`show.rs`）。
const NO_GFX_LINE: &str =
    "apply(ShowSurface): GraphicsCore 不在または DeviceContext 不在（表示を記録できない）";

fn attached(world: &mut World) -> EmoPresenter {
    let window = spawn_window_with_dpi(world, 96);
    let (emo_world, atlas, _a, _b) = build_two_face_assets(4, 3);
    let mut presenter = EmoPresenter::new();
    presenter
        .attach_target(world, TARGET, window, emo_world, atlas, 96)
        .expect("attach_target 失敗");
    presenter
}

/// `ShowSurface` を 1 本流し、reply を返す。
fn show(presenter: &mut EmoPresenter, world: &mut World, surface_id: u32) -> PresentOutcome {
    let (tx, rx) = reply_channel::<PresentOutcome>();
    presenter.apply(
        world,
        PresentCommand::ShowSurface {
            target: TARGET,
            surface_id,
            binds: BindSet::default(),
            pattern: PatternState::default(),
            reply: Some(tx),
        },
    );
    rx.recv_timeout(Duration::from_secs(10))
        .expect("reply を受信できない")
}

/// `CreateBitmap` へ失敗を注入して 1 本流す（武装の解除は `Drop` で必ず行う）。
fn show_with_fault(presenter: &mut EmoPresenter, world: &mut World, surface_id: u32) {
    struct Disarm;
    impl Drop for Disarm {
        fn drop(&mut self) {
            clear_display_fault();
        }
    }
    let _disarm = Disarm;
    arm_display_fault(DisplayFault::CreateBitmap);
    let outcome = show(presenter, world, surface_id);
    assert!(
        matches!(
            outcome,
            Err(PresentError::Device {
                context: "<injected:CreateBitmap>",
                ..
            })
        ),
        "注入が届いていない: {outcome:?}"
    );
}

/// 文言 `message` の行の水準を出た順に並べる。
fn levels_of(events: &[CapturedEvent], message: &str) -> Vec<tracing::Level> {
    events
        .iter()
        .filter(|e| e.message() == message)
        .map(|e| e.level)
        .collect()
}

/// 同じ原因の失敗を 3 回続けて注入すると ERROR 1 本と DEBUG 2 本（要件 4.3）。
#[test]
fn same_cause_three_times_is_one_error_then_two_debug() {
    let mut world = make_world_with_gpu();
    let mut presenter = attached(&mut world);
    let ((), events) = capture(|| {
        for _ in 0..3 {
            show_with_fault(&mut presenter, &mut world, FACE_A);
        }
    });
    assert_eq!(
        levels_of(&events, DEVICE_LINE),
        vec![L::ERROR, L::DEBUG, L::DEBUG]
    );
}

/// 成立の後の同じ失敗は ERROR に戻る（成立をはさめば別の続き）。
#[test]
fn same_cause_after_a_success_is_error_again() {
    let mut world = make_world_with_gpu();
    let mut presenter = attached(&mut world);
    let ((), events) = capture(|| {
        show_with_fault(&mut presenter, &mut world, FACE_A);
        show_with_fault(&mut presenter, &mut world, FACE_A);
        show_ok(&mut presenter, &mut world, TARGET, FACE_A);
        // 控えに当たらない面で失敗させる（当たると記録へ入らない）。
        show_with_fault(&mut presenter, &mut world, FACE_B);
    });
    assert_eq!(
        levels_of(&events, DEVICE_LINE),
        vec![L::ERROR, L::DEBUG, L::ERROR]
    );
}

/// 面が替わった適用でも、同じ原因の続きなら DEBUG のまま（単位は対象・原因・成立をはさまない）。
#[test]
fn a_surface_change_continuing_the_same_failure_stays_debug() {
    let mut world = make_world_with_gpu();
    let mut presenter = attached(&mut world);
    let ((), events) = capture(|| {
        show_with_fault(&mut presenter, &mut world, FACE_A);
        show_with_fault(&mut presenter, &mut world, FACE_B);
        show_with_fault(&mut presenter, &mut world, FACE_A);
    });
    assert_eq!(
        levels_of(&events, DEVICE_LINE),
        vec![L::ERROR, L::DEBUG, L::DEBUG]
    );
}

/// `GraphicsCore` が無い道も同じ形でまとめ、成立の後は ERROR に戻る。
#[test]
fn the_graphics_core_less_path_is_coalesced_the_same_way() {
    let mut world = make_world_with_gpu();
    let mut presenter = attached(&mut world);
    let no_gfx = |presenter: &mut EmoPresenter, world: &mut World, surface_id: u32| {
        let gfx = world
            .remove_resource::<GraphicsCore>()
            .expect("前提: GraphicsCore が載っている");
        let outcome = show(presenter, world, surface_id);
        world.insert_resource(gfx);
        assert!(
            matches!(
                outcome,
                Err(PresentError::Device {
                    hresult: 0,
                    context: "GraphicsCore resource",
                })
            ),
            "GraphicsCore 不在の道へ落ちていない: {outcome:?}"
        );
    };
    let ((), events) = capture(|| {
        no_gfx(&mut presenter, &mut world, FACE_A);
        no_gfx(&mut presenter, &mut world, FACE_B);
        no_gfx(&mut presenter, &mut world, FACE_A);
        show_ok(&mut presenter, &mut world, TARGET, FACE_A);
        no_gfx(&mut presenter, &mut world, FACE_B);
    });
    assert_eq!(
        levels_of(&events, NO_GFX_LINE),
        vec![L::ERROR, L::DEBUG, L::DEBUG, L::ERROR]
    );
}

/// 面 1000（有効）と面 7000（定義層皆無＝合成が `EmptyComposition`）を載せた対象を `target` に装着する。
fn attach_valid_and_empty(presenter: &mut EmoPresenter, world: &mut World, target: TargetId) {
    let window = spawn_window_with_dpi(world, 96);
    let (emo_world, atlas) = super::display_tests::build_assets_with_valid_and_empty(4, 3, 0x5A);
    presenter
        .attach_target(world, target, window, emo_world, atlas, 96)
        .expect("attach_target 失敗");
}

/// 合成の失敗（無い面）をはさむと原因を忘れる＝次の同じ装置の失敗は ERROR（装置以外の失敗）。
#[test]
fn a_compose_failure_in_between_forgets_the_note() {
    let mut world = make_world_with_gpu();
    let mut presenter = EmoPresenter::new();
    attach_valid_and_empty(&mut presenter, &mut world, TARGET);
    let ((), events) = capture(|| {
        show_with_fault(&mut presenter, &mut world, FACE_A);
        let outcome = show(&mut presenter, &mut world, 9999);
        assert!(
            matches!(outcome, Err(PresentError::Compose(_))),
            "前提: 無い面は合成の失敗: {outcome:?}"
        );
        show_with_fault(&mut presenter, &mut world, FACE_A);
    });
    assert_eq!(levels_of(&events, DEVICE_LINE), vec![L::ERROR, L::ERROR]);
}

/// 全透明の縮退（`EmptyComposition`）をはさんでも原因を忘れる。
#[test]
fn the_all_transparent_degrade_in_between_forgets_the_note() {
    let mut world = make_world_with_gpu();
    let mut presenter = EmoPresenter::new();
    attach_valid_and_empty(&mut presenter, &mut world, TARGET);
    let ((), events) = capture(|| {
        show_with_fault(&mut presenter, &mut world, FACE_A);
        let outcome = show(&mut presenter, &mut world, 7000);
        assert!(outcome.is_ok(), "前提: 縮退は reply Ok: {outcome:?}");
        show_with_fault(&mut presenter, &mut world, FACE_A);
    });
    assert!(
        events
            .iter()
            .any(|e| e.message().contains("全透明退化（EmptyComposition）")),
        "前提: 縮退の道を通っている"
    );
    assert_eq!(levels_of(&events, DEVICE_LINE), vec![L::ERROR, L::ERROR]);
}

/// まとめる単位は対象ごと＝別の対象（シェルとバルーンにあたる 2 つ）の同じ原因は、それぞれ最初が ERROR。
#[test]
fn each_target_logs_its_own_first_error() {
    let mut world = make_world_with_gpu();
    let mut presenter = EmoPresenter::new();
    let other = TargetId(1);
    attach_valid_and_empty(&mut presenter, &mut world, TARGET);
    attach_valid_and_empty(&mut presenter, &mut world, other);
    let fault_on = |presenter: &mut EmoPresenter, world: &mut World, target: TargetId| {
        struct Disarm;
        impl Drop for Disarm {
            fn drop(&mut self) {
                clear_display_fault();
            }
        }
        let _disarm = Disarm;
        arm_display_fault(DisplayFault::CreateBitmap);
        let (tx, rx) = reply_channel::<PresentOutcome>();
        presenter.apply(
            world,
            PresentCommand::ShowSurface {
                target,
                surface_id: FACE_A,
                binds: BindSet::default(),
                pattern: PatternState::default(),
                reply: Some(tx),
            },
        );
        let outcome = rx.recv_timeout(Duration::from_secs(10)).expect("reply");
        assert!(
            matches!(outcome, Err(PresentError::Device { .. })),
            "{outcome:?}"
        );
    };
    let ((), events) = capture(|| {
        fault_on(&mut presenter, &mut world, TARGET);
        fault_on(&mut presenter, &mut world, other);
        fault_on(&mut presenter, &mut world, TARGET);
        fault_on(&mut presenter, &mut world, other);
    });
    assert_eq!(
        levels_of(&events, DEVICE_LINE),
        vec![L::ERROR, L::ERROR, L::DEBUG, L::DEBUG]
    );
}
