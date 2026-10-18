//! 窓を残して実行系だけを組み直す本体の決定論テスト（areka-P0-mcp-reload task 7.2・要件 4.2・4.4・
//! design「GhostSession の口」・Flow 3）。
//!
//! 切替の土台（[`SwitchRig`]）で A を起こし、置き場の単位に [`rebuild_runtime_in_place`] を当てる。
//! 確かめること: 組み直せたら同じ単位に新しい実行系が付き（seriko・loop ticker・束は残る）、新しい
//! SHIORI は `OnInitialize` の Ref0＝`reload` で起きる／起こせなければ実行系の無い単位が置き場に戻り、
//! seriko・loop ticker・束が残って、最後に降ろすと seriko の join が戻る。

use areka_kanade::KanadeMsg;

use super::*;
use crate::ConfigInputs;
use crate::emo2_boot::ghost_switch_test_support::{FakeShiori, SwitchRig, standard_script};
use crate::emo2_boot::spine::RecordedCall;
use crate::ghost_session::GhostBootInputsSource;

/// A を起こして定常に着かせる（起動記録を置く＝`OnFirstBoot` を飛ばす）。
fn running_a() -> SwitchRig {
    let mut rig = SwitchRig::new(vec![(
        "A",
        FakeShiori::Scripted(Box::new(|| {
            standard_script(r"\0A\e").get("username", Ok(None))
        })),
    )]);
    rig.plant_boot_record("A");
    rig.boot("A");
    assert!(rig.wait_steady(), "A が定常に着く");
    rig
}

/// 置き場の単位の窓の側（seriko・loop ticker・束）と実行系の有無。単位が無ければ `None`。
fn slot_shape(rig: &SwitchRig) -> Option<(bool, bool, bool, bool)> {
    let slot = rig.world.non_send::<GhostSlot>();
    slot.0.as_ref().map(|s| {
        (
            s.seriko.is_some(),
            s.loop_ticker.is_some(),
            s.sink_sources().is_some(),
            s.runtime().is_some(),
        )
    })
}

/// 送出端の先に生きた kanade が居るか（空の照会を投げ、返事が来れば真）。
fn kanade_answers(kanade: &std::sync::mpsc::Sender<KanadeMsg>) -> bool {
    let (reply, receiver) = areka_actor::reply_channel();
    kanade
        .send(KanadeMsg::ResourceQuery { ids: vec![], reply })
        .is_ok()
        && receiver
            .recv_timeout(std::time::Duration::from_secs(30))
            .is_ok()
}

/// 組み直せた（要件 4.2）: 同じ単位に新しい実行系が付き、窓の側は残り、単位の送出端の先に生きた
/// kanade が居る。新しい SHIORI は `OnInitialize` の Ref0＝`reload` で起き（由来 `Reloaded{Shiori}`）、
/// 台詞を求める GET（`OnBoot` ほか）を受けない。
///
/// # 非空虚性
/// 付け直さないと実行系が無くて赤。由来を渡し違えると `OnInitialize` の Ref0 が違うか `OnBoot` が来て赤。
#[test]
fn rebuilt_runtime_is_attached_to_the_same_unit() {
    let mut rig = running_a();
    let old_kanade = rig
        .world
        .non_send::<GhostSlot>()
        .0
        .as_ref()
        .and_then(|s| s.kanade().cloned());

    let answer = rebuild_runtime_in_place(&mut rig.world, "A");
    let shape = slot_shape(&rig);
    let new_answers = rig
        .world
        .non_send::<GhostSlot>()
        .0
        .as_ref()
        .and_then(|s| s.kanade().cloned())
        .is_some_and(|k| kanade_answers(&k));
    let old_dead = old_kanade.is_some_and(|k| !kanade_answers(&k));
    let booted = rig.wait_for(|| {
        rig.calls("A").get(1).is_some_and(|c| {
            c.iter()
                .any(|c| matches!(c, RecordedCall::Notify { id, .. } if id == "basewareversion"))
        })
    });
    let calls = rig.calls("A");
    assert!(
        rig.shutdown(),
        "組み直した単位を降ろせる（seriko の join が戻る）"
    );

    assert!(matches!(answer, RuntimeRebuild::Rebuilt), "{answer:?}");
    assert_eq!(
        (shape, new_answers, old_dead, booted),
        (Some((true, true, true, true)), true, true, true),
        "(単位の形 (seriko・ticker・束・実行系)・新しい kanade が答える・古い kanade は死んだ・新しい起動が進んだ)"
    );
    let new = &calls[1];
    assert!(
        new.iter().any(|c| matches!(
            c,
            RecordedCall::Notify { id, references } if id == "OnInitialize"
                && references.first().map(String::as_str) == Some("reload")
        )),
        "新しい SHIORI は OnInitialize(Ref0=reload) を受ける: {new:?}"
    );
    assert!(
        new.iter().all(
            |c| !matches!(c, RecordedCall::Get { id, .. } if id == "OnBoot" || id == "OnFirstBoot")
        ),
        "起動の台詞を求めない: {new:?}"
    );
}

/// 起こせなかった（要件 4.4・design Flow 3「失敗」）: 実行系の無い単位が置き場に戻り、seriko・loop
/// ticker・束が残る（呼び手が既定へ戻すときの `take_down` が正しく降ろせる）。最後に降ろすと join が戻る。
///
/// 起こす根を実在しない場所へ向けた作り口に差し替えて、2 回目の起動を同期で失敗させる。
///
/// # 非空虚性
/// 単位を置き場へ戻さないと `None` で赤。実行系を外さないまま戻すと実行系ありで赤。
#[test]
fn boot_failure_returns_the_unit_without_runtime_to_the_slot() {
    let mut rig = running_a();
    let original = rig
        .world
        .remove_non_send::<GhostBootInputsSource>()
        .expect("作り口が据わっている");
    rig.world.insert_non_send(GhostBootInputsSource(Box::new(
        move |cfg: &ConfigInputs, origin| {
            let mut inputs = (original.0)(cfg, origin);
            inputs.wiring.ghost_root = cfg.ghost_root.join("実在しない根");
            inputs
        },
    )));

    let answer = rebuild_runtime_in_place(&mut rig.world, "A");
    let shape = slot_shape(&rig);
    let down = rig.shutdown();

    assert!(matches!(answer, RuntimeRebuild::BootFailed), "{answer:?}");
    assert_eq!(
        (shape, down),
        (Some((true, true, true, false)), true),
        "(単位の形 (seriko・ticker・束・実行系)・降ろすと join が戻る)"
    );
}
