//! 表示指令の変換と配送（薄い変換＋配送のみ・状態を持たない・R3.6）。
//!
//! `map_display_command`（`DisplayCommand`→`PresentCommand` の純変換・DD-5・`target_map` を利用）と
//! `PresentBridge`（seriko の `SurfaceOutput` 本番実装・`mpsc::Sender` へ非ブロック送出）を
//! 所有する。`ShowBalloon`／`HideBalloon` はバルーン表示対象へ `BindSet::default()` 付きで
//! 配送する（R5.1／R5.2）。送出失敗（受信端 drop）は `debug!`・非数値 scope の drop は
//! `warn!` で log-first 観測する（R3.7・design.md「Error Categories and Responses」）。
//!
//! 本ファイルは task 2.4（`map_display_command` 純変換）を実装する。`PresentBridge`
//! （`SurfaceOutput` 本番実装・UI 配送）は後続 task 2.5 が担う。

use areka_emo_compose::BindSet;
use areka_emo_present::PresentCommand;
use areka_seriko::DisplayCommand;

use crate::emo2_boot::target_map::{balloon_target, scope_of, shell_target};

/// `DisplayCommand` → `PresentCommand` の純変換（DD-5・`target_map` を利用）。
///
/// 可変状態・I/O を持たない純関数で、4 写像を与える（R3.6）:
/// - `Show { scope, surface_id, binds }` → シェル表示対象へ `ShowSurface`。`surface_id`（数値）と
///   `binds` を**非改変で転写**する（R3.1）。
/// - `Hide { scope }` → シェル表示対象の `Hide`（R3.2）。
/// - `ShowBalloon { scope, surface_id }` → バルーン表示対象へ `ShowSurface`。`binds` は
///   `BindSet::default()`（空集合＝M-boot にバルーン着せ替えは存在しない・R3.3／R5.1）。
///   `surface_id` は seriko が解決済みの数値 id をそのまま転写し、**alias を再適用しない**（R5.3）。
/// - `HideBalloon { scope }` → バルーン表示対象の `Hide`（R3.4／R5.2）。
///
/// `reply` は常に `None`（撃ちっぱなし・fire-and-forget）。scope（`ActorKey`）が非数値のとき
/// （`scope_of` が `None`）は本関数も `None` を返し、呼び手（`PresentBridge::send`）が `warn!` ＋
/// 当該指令 drop で握り潰さず log-first 観測する（R3.7）。
///
/// `DisplayCommand` は `#[non_exhaustive]` でないため 4 variant の網羅 `match` で全経路を尽くす。
pub fn map_display_command(cmd: DisplayCommand) -> Option<PresentCommand> {
    Some(match cmd {
        // Show: シェル表示対象へそのまま。surface_id・binds は非改変で転写（R3.1／R5.3）。
        DisplayCommand::Show {
            scope,
            surface_id,
            binds,
        } => PresentCommand::ShowSurface {
            target: shell_target(scope_of(&scope)?),
            surface_id,
            binds,
            reply: None,
        },
        // Hide: シェル表示対象の非表示（R3.2）。
        DisplayCommand::Hide { scope } => PresentCommand::Hide {
            target: shell_target(scope_of(&scope)?),
            reply: None,
        },
        // ShowBalloon: バルーン表示対象へ。binds は既定（空集合＝バルーン着せ替え無し・R3.3／R5.1）。
        // surface_id は seriko 解決済み数値 id をそのまま転写（alias 非再適用・R5.3）。
        DisplayCommand::ShowBalloon { scope, surface_id } => PresentCommand::ShowSurface {
            target: balloon_target(scope_of(&scope)?),
            surface_id,
            binds: BindSet::default(),
            reply: None,
        },
        // HideBalloon: バルーン表示対象の非表示（`\b[-1]` 相当・R3.4／R5.2）。
        DisplayCommand::HideBalloon { scope } => PresentCommand::Hide {
            target: balloon_target(scope_of(&scope)?),
            reply: None,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use areka_sakura::ActorKey;

    /// `Show` → シェル表示対象の `ShowSurface`。`surface_id`（非自明値）と非既定 `binds` が
    /// 非改変で透過することを反証する（R3.1／R5.3）。`reply` は `None`。
    #[test]
    fn show_maps_to_shell_show_surface_transcribing_id_and_binds() {
        let binds = BindSet::from_ids([1100, 1500]);
        let out = map_display_command(DisplayCommand::Show {
            scope: ActorKey::from("0"),
            surface_id: 2100,
            binds: binds.clone(),
        })
        .expect("数値 scope は Some を返すこと");

        match out {
            PresentCommand::ShowSurface {
                target,
                surface_id,
                binds: got,
                reply,
            } => {
                assert_eq!(target, shell_target(0), "Show は shell 表示対象（偶数）へ写像");
                assert_eq!(surface_id, 2100, "surface_id は非改変で転写されること");
                assert_eq!(got, binds, "binds はそのまま透過されること");
                assert!(reply.is_none(), "reply は常に None（撃ちっぱなし）");
            }
            _ => panic!("Show は ShowSurface へ写像されるべき"),
        }
    }

    /// `Hide` → シェル表示対象の `Hide`（R3.2）。`reply` は `None`。
    #[test]
    fn hide_maps_to_shell_hide() {
        let out = map_display_command(DisplayCommand::Hide {
            scope: ActorKey::from("1"),
        })
        .expect("数値 scope は Some を返すこと");

        match out {
            PresentCommand::Hide { target, reply } => {
                assert_eq!(target, shell_target(1), "Hide は shell 表示対象へ写像");
                assert!(reply.is_none(), "reply は常に None");
            }
            _ => panic!("Hide は Hide へ写像されるべき"),
        }
    }

    /// `ShowBalloon` → バルーン表示対象の `ShowSurface`。`surface_id` は非改変転写・`binds` は
    /// 既定（空集合）（R3.3／R5.1／R5.3）。`reply` は `None`。
    #[test]
    fn show_balloon_maps_to_balloon_show_surface_with_default_binds() {
        let out = map_display_command(DisplayCommand::ShowBalloon {
            scope: ActorKey::from("0"),
            surface_id: 6,
        })
        .expect("数値 scope は Some を返すこと");

        match out {
            PresentCommand::ShowSurface {
                target,
                surface_id,
                binds,
                reply,
            } => {
                assert_eq!(
                    target,
                    balloon_target(0),
                    "ShowBalloon は balloon 表示対象（奇数）へ写像"
                );
                assert_eq!(surface_id, 6, "surface_id は非改変で転写（alias 非再適用）");
                assert_eq!(binds, BindSet::default(), "binds は既定（空集合）であること");
                assert!(reply.is_none(), "reply は常に None");
            }
            _ => panic!("ShowBalloon は ShowSurface へ写像されるべき"),
        }
    }

    /// `HideBalloon` → バルーン表示対象の `Hide`（R3.4／R5.2）。`reply` は `None`。
    #[test]
    fn hide_balloon_maps_to_balloon_hide() {
        let out = map_display_command(DisplayCommand::HideBalloon {
            scope: ActorKey::from("1"),
        })
        .expect("数値 scope は Some を返すこと");

        match out {
            PresentCommand::Hide { target, reply } => {
                assert_eq!(
                    target,
                    balloon_target(1),
                    "HideBalloon は balloon 表示対象へ写像"
                );
                assert!(reply.is_none(), "reply は常に None");
            }
            _ => panic!("HideBalloon は Hide へ写像されるべき"),
        }
    }

    /// 非数値 scope（例 "側"）は 4 写像すべてで `None`（呼び手が `warn!`＋drop）。
    #[test]
    fn non_numeric_scope_returns_none_for_all_variants() {
        let bad = || ActorKey::from("側");

        assert!(
            map_display_command(DisplayCommand::Show {
                scope: bad(),
                surface_id: 2100,
                binds: BindSet::from_ids([1100]),
            })
            .is_none(),
            "Show の非数値 scope は None"
        );
        assert!(
            map_display_command(DisplayCommand::Hide { scope: bad() }).is_none(),
            "Hide の非数値 scope は None"
        );
        assert!(
            map_display_command(DisplayCommand::ShowBalloon {
                scope: bad(),
                surface_id: 6,
            })
            .is_none(),
            "ShowBalloon の非数値 scope は None"
        );
        assert!(
            map_display_command(DisplayCommand::HideBalloon { scope: bad() }).is_none(),
            "HideBalloon の非数値 scope は None"
        );
    }

    /// 複数 scope（"0"/"1"）でシェル（偶数）／バルーン（奇数）表示対象の偶奇が写像へ流れる
    /// （target_map の DD-3 採番規約が adapter を貫くこと）。
    #[test]
    fn shell_and_balloon_target_parity_flows_through() {
        for scope in [0u32, 1] {
            let key = ActorKey::from(scope.to_string());

            let show = map_display_command(DisplayCommand::Show {
                scope: key.clone(),
                surface_id: 10,
                binds: BindSet::default(),
            })
            .expect("数値 scope は Some");
            match show {
                PresentCommand::ShowSurface { target, .. } => {
                    assert_eq!(target, shell_target(scope));
                    assert_eq!(target.0 % 2, 0, "shell 表示対象は偶数（scope {scope}）");
                }
                _ => panic!("Show は ShowSurface"),
            }

            let show_balloon = map_display_command(DisplayCommand::ShowBalloon {
                scope: key.clone(),
                surface_id: 10,
            })
            .expect("数値 scope は Some");
            match show_balloon {
                PresentCommand::ShowSurface { target, .. } => {
                    assert_eq!(target, balloon_target(scope));
                    assert_eq!(target.0 % 2, 1, "balloon 表示対象は奇数（scope {scope}）");
                }
                _ => panic!("ShowBalloon は ShowSurface"),
            }
        }
    }
}
