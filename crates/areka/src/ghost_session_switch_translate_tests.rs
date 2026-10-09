//! 切替の間の `OnTranslate` の送り先（spec `areka-P0-translate-pipeline` 要件 3.5）。
//!
//! 2 体の実行系を実際に作るのはこの crate の切替の土台（`ghost_switch_test_support`）なので、
//! ここで確かめる（kanade と SHIORI はゴーストごとに 1 組＝送り出しの台詞は前のゴーストの、
//! 挨拶は次のゴーストの SHIORI へ翻訳を頼む）。判定は集めてから 1 回・降ろすのは有界に行う。

use wintf::ecs::widget::bitmap_source::WintfTaskPool;

use crate::emo2_boot::ghost_switch::SwitchInFlight;
use crate::emo2_boot::ghost_switch_test_support::{FakeShiori, SwitchRig, standard_script};
use crate::emo2_boot::spine::RecordedCall;

/// A の `OnBoot` の台本: 台詞の途中で B への切替（`OnGhostChanging` を送らせる）を命じる。
const A_TO_B: &str = "\\0A\\![change,ghost,B,--option=raise-event]\\e";
/// A の `OnGhostChanging` が返す送り出しの台本。
const A_SEND_OFF: &str = "\\0Aから送り出します\\e";
/// B の `OnGhostChanged` が返す挨拶の台本。
const B_GREETING: &str = "\\0Bが来ました\\e";

/// `folder` の偽の SHIORI が受けた `OnTranslate` の（Ref2＝元のイベント、Ref0＝台詞）の列
/// （起こした回をまたいで起こした順）。
fn translations(rig: &SwitchRig, folder: &str) -> Vec<(String, String)> {
    rig.calls(folder)
        .iter()
        .flatten()
        .filter_map(|c| match c {
            RecordedCall::Get { id, references } if id == "OnTranslate" => Some((
                references.get(2).cloned().unwrap_or_default(),
                references.first().cloned().unwrap_or_default(),
            )),
            _ => None,
        })
        .collect()
}

/// 台本の切替命令で A から B へ切り替えると、A の送り出しの台詞の `OnTranslate` は A の
/// SHIORI にだけ、B の挨拶の `OnTranslate` は B の SHIORI にだけ届く（要件 3.5）。
///
/// # 非空虚性
/// 送り出しと挨拶は別々の台詞なので、どちらかが相手の SHIORI へ届くと列の比較が赤になる。
#[test]
fn switch_sends_each_on_translate_to_its_own_ghost() {
    let mut rig = SwitchRig::new(vec![
        (
            "A",
            FakeShiori::Scripted(Box::new(|| {
                standard_script(A_TO_B).get("OnGhostChanging", Ok(Some(A_SEND_OFF.to_owned())))
            })),
        ),
        (
            "B",
            FakeShiori::Scripted(Box::new(|| {
                standard_script("\\0B\\e").get("OnGhostChanged", Ok(Some(B_GREETING.to_owned())))
            })),
        ),
    ]);
    // 切替先の窓の準備が閉包を投函する先（`Input` の段に作業プールの取り出しの系は無いので走らない）。
    rig.world.insert_resource(WintfTaskPool::with_threads(1));
    rig.plant_boot_record("A");
    rig.plant_boot_record("B");
    rig.boot("A");
    let steady = rig.wait_steady();
    // B の挨拶の翻訳が届き、迎え入れの定常の通知を処理する（予約が消える）まで回す。
    let welcomed = rig.pump_talking_until(|rig| {
        rig.exit_requested()
            || (!translations(rig, "B").is_empty()
                && rig.world.get_non_send::<SwitchInFlight>().is_none())
    });
    let a = translations(&rig, "A");
    let b = translations(&rig, "B");
    let shutdown_ok = rig.shutdown();

    assert_eq!(
        (steady, welcomed, a, b, shutdown_ok),
        (
            true,
            true,
            vec![
                ("OnBoot".to_owned(), A_TO_B.to_owned()),
                ("OnGhostChanging".to_owned(), A_SEND_OFF.to_owned()),
            ],
            vec![("OnGhostChanged".to_owned(), B_GREETING.to_owned())],
            true,
        ),
        "切替の翻訳の送り先が違う（A の定常・B の迎え入れまで届いた・A の SHIORI が受けた \
         OnTranslate・B の SHIORI が受けた OnTranslate・降ろせた）"
    );
}
