//! ずれの命令をシェル／バルーンの対象の提示の命令へ写す橋渡しの兄弟テスト
//! （spec: seriko-pattern-calls task 5.4・要件 7.1・7.6・3.8）。

use areka_emo_present::PresentCommand;
use areka_sakura::ActorKey;
use areka_seriko::DisplayCommand;

use super::map_display_command;
use crate::emo2_boot::target_map::{balloon_target, shell_target};

/// シェルのずれはシェルの対象へ、バルーンのずれはバルーンの対象へ、値はそのまま写る。
#[test]
fn move_shift_maps_to_the_shell_and_balloon_targets_with_the_same_values() {
    let shell = map_display_command(DisplayCommand::MoveShift {
        scope: ActorKey::from("1"),
        x: -3,
        y: 9999,
    });
    assert!(
        matches!(shell, Some(PresentCommand::MoveShift { target, x: -3, y: 9999 }) if target == shell_target(1)),
        "シェルの対象へ値はそのまま"
    );
    let balloon = map_display_command(DisplayCommand::MoveShiftBalloon {
        scope: ActorKey::from("0"),
        x: 0,
        y: 0,
    });
    assert!(
        matches!(balloon, Some(PresentCommand::MoveShift { target, x: 0, y: 0 }) if target == balloon_target(0)),
        "バルーンの対象へ値はそのまま"
    );
}

/// 数値でないスコープは写せず `None`（呼び手が `warn!` で捨てる・今の写し方と同じ）。
#[test]
fn move_shift_with_non_numeric_scope_is_not_mapped() {
    for cmd in [
        DisplayCommand::MoveShift {
            scope: ActorKey::from("kero"),
            x: 1,
            y: 1,
        },
        DisplayCommand::MoveShiftBalloon {
            scope: ActorKey::from("kero"),
            x: 1,
            y: 1,
        },
    ] {
        assert!(map_display_command(cmd).is_none());
    }
}
