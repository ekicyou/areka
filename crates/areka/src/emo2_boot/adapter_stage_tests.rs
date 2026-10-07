//! 合図 `StageAck` をバルーンの対象へ写す橋渡しの兄弟テスト
//! （spec: areka-P0-animated-image-playback task 5.1・要件 2.3・6.1）。

use areka_emo_present::PresentCommand;
use areka_sakura::ActorKey;
use areka_seriko::DisplayCommand;

use super::map_display_command;
use crate::emo2_boot::target_map::balloon_target;

/// スコープ 1 の合図はバルーン 1 の対象（シェルの対象ではない）へ、世代はそのまま写る。
#[test]
fn stage_ack_maps_to_the_balloon_target_with_the_same_generation() {
    let out = map_display_command(DisplayCommand::StageAck {
        scope: ActorKey::from("1"),
        generation: 7,
    });
    match out {
        Some(PresentCommand::StageAck { target, generation }) => {
            assert_eq!(target, balloon_target(1), "バルーンの対象（奇数）");
            assert_eq!(generation, 7);
        }
        _ => panic!("StageAck を期待"),
    }
}

/// 数値でないスコープは写せず `None`（呼び手が `warn!` で捨てる・今の写し方と同じ）。
#[test]
fn stage_ack_with_non_numeric_scope_is_not_mapped() {
    assert!(
        map_display_command(DisplayCommand::StageAck {
            scope: ActorKey::from("kero"),
            generation: 1,
        })
        .is_none()
    );
}
