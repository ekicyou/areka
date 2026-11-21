//! 表示指令の変換と配送（薄い変換＋配送のみ・状態を持たない）。
//!
//! `map_display_command`（`DisplayCommand`→`PresentCommand` の純変換・`target_map` を利用）と
//! `PresentBridge`（seriko の `SurfaceOutput` 本番実装・`mpsc::Sender` へ非ブロック送出）を
//! 所有する。`ShowBalloon`／`HideBalloon` はバルーン表示対象へ `BindSet::default()` 付きで配送する。
//! 送出失敗（受信端 drop）は `debug!`、数でない、または大きすぎる scope の drop は `warn!` で観測する。
//!
//! 定義を替えた合図 `Rebased` は、橋渡しが持つ荷物の置き場と突き合わせて scope ごとの
//! `ReplaceTarget` へ写す。

use std::sync::PoisonError;

use areka_emo_compose::BindSet;
use areka_emo_present::PresentCommand;
use areka_seriko::{DisplayCommand, RebaseKind, RebasedShow, SurfaceOutput};
use wintf::ecs::world::tick_wake;

use crate::switch_assets::{SwapPayload, SwapSlot};
use crate::target_map::{balloon_target, scope_of, shell_target};

/// `DisplayCommand` → `PresentCommand` の純変換（`target_map` を利用）。
///
/// 可変状態・I/O を持たない純関数で、次の写像を与える:
/// - `Show { scope, surface_id, binds, pattern }` → シェル表示対象へ `ShowSurface`。`surface_id`
///   （数値）・`binds`・`pattern` を**非改変で転写**する。
/// - `Hide { scope }` → シェル表示対象の `Hide`。
/// - `ShowBalloon { scope, surface_id, pattern }` → バルーン表示対象へ `ShowSurface`。`binds` は
///   `BindSet::default()`（空集合＝バルーン着せ替えは無い）。`surface_id` は seriko が解決済みの数値 id を
///   そのまま転写し、**alias を再適用しない**。`pattern` はシェル面 `Show` と同様に非改変で転写する
///   （cue 由来は空 pattern）。
/// - `HideBalloon { scope }` → バルーン表示対象の `Hide`。
/// - `StageAck { scope, generation }` → バルーン表示対象の `StageAck`（世代はそのまま）。
/// - `MoveShift { scope, x, y }` / `MoveShiftBalloon { scope, x, y }` → シェル／バルーン表示対象の
///   `MoveShift`（値はそのまま）。
///
/// `reply` は常に `None`（撃ちっぱなし）。scope（`ActorKey`）が数でない、または大きすぎるとき
/// （`scope_of` が `None`）は本関数も `None` を返し、呼び手（`PresentBridge::send`）が `warn!` ＋
/// 当該指令 drop で観測する。
///
/// `DisplayCommand` は `#[non_exhaustive]` でないため網羅 `match` で全経路を尽くす
/// （定義を替えた合図 `Rebased` は写さず `None`＝`PresentBridge::send` が先に捌く）。
pub fn map_display_command(cmd: DisplayCommand) -> Option<PresentCommand> {
    Some(match cmd {
        DisplayCommand::Show {
            scope,
            surface_id,
            binds,
            pattern,
        } => PresentCommand::ShowSurface {
            target: shell_target(scope_of(&scope)?),
            surface_id,
            binds,
            pattern,
            reply: None,
        },
        DisplayCommand::Hide { scope } => PresentCommand::Hide {
            target: shell_target(scope_of(&scope)?),
            reply: None,
        },
        DisplayCommand::ShowBalloon {
            scope,
            surface_id,
            pattern,
        } => PresentCommand::ShowSurface {
            target: balloon_target(scope_of(&scope)?),
            surface_id,
            binds: BindSet::default(),
            pattern,
            reply: None,
        },
        // `\b[-1]` 相当。
        DisplayCommand::HideBalloon { scope } => PresentCommand::Hide {
            target: balloon_target(scope_of(&scope)?),
            reply: None,
        },
        // 定義を替えた合図は 1 件の命令へ写せない（scope ごとの置き換えは荷物と突き合わせて
        // `PresentBridge::send` が先に捌く）。純変換としては写す先が無い。
        DisplayCommand::Rebased { .. } => return None,
        DisplayCommand::StageAck { scope, generation } => PresentCommand::StageAck {
            target: balloon_target(scope_of(&scope)?),
            generation,
        },
        DisplayCommand::MoveShift { scope, x, y } => PresentCommand::MoveShift {
            target: shell_target(scope_of(&scope)?),
            x,
            y,
        },
        DisplayCommand::MoveShiftBalloon { scope, x, y } => PresentCommand::MoveShift {
            target: balloon_target(scope_of(&scope)?),
            x,
            y,
        },
    })
}

/// seriko の `SurfaceOutput` 本番実装。
///
/// `spawn_seriko(out = PresentBridge)` で seriko worker スレッドへ move され（`O: SurfaceOutput
/// + Send + 'static`）、worker 上で受けた `DisplayCommand` を
/// [`map_display_command`] で `PresentCommand` へ純変換し、UI スレッドの表示層（frame drain 側の
/// `Receiver`）へ `mpsc::Sender` で**非ブロック**配送する。
///
/// 可変状態を持たず、フィールドは配送用 `Sender` と、UI と共有する荷物の置き場（任意）だけ。
/// 変換は [`map_display_command`] の純関数へ、配送は `Sender::send` へ委ねる（キャッシュ・
/// カウンタ等の内部状態を持たない）。
pub struct PresentBridge {
    /// UI フレーム drain 側 `Receiver` への非ブロック配送口。
    tx: std::sync::mpsc::Sender<PresentCommand>,
    /// 差し替えの荷物の置き場。
    ///
    /// 置き場は UI が荷物を置き、合図 `Rebased` を受けたときにここから取り出すだけの共有の器である
    /// （既定は無し＝合図が来れば `error!` で何も送らない）。
    swap: Option<SwapSlot>,
}

impl PresentBridge {
    /// `PresentCommand` の配送先 `Sender`（UI フレーム drain 側）を与えて構築する（置き場は無し）。
    pub fn new(tx: std::sync::mpsc::Sender<PresentCommand>) -> Self {
        Self { tx, swap: None }
    }

    /// 差し替えの荷物の置き場を持たせる（起動の結線が渡す。置き場を強く持つのはこの橋渡しだけで、
    /// `RuntimeWiring` は弱い参照を持つ＝橋渡しが消えれば荷物と返信の送り手も消える）。
    pub(crate) fn with_swap_slot(mut self, slot: SwapSlot) -> Self {
        self.swap = Some(slot);
        self
    }
}

impl SurfaceOutput for PresentBridge {
    /// seriko worker から届いた `DisplayCommand` を写像し、成功時のみ UI へ非ブロック配送する。
    ///
    /// - 写像成功（`scope_of` が番号を返す scope）: `PresentCommand` を `tx.send`。受信端 drop（＝shutdown 中の
    ///   期待事象）で失敗しても **panic せず** `debug!` で観測して破棄する。
    /// - 写像不能（数でない、または大きすぎる scope＝[`map_display_command`] が `None`）: `warn!` で観測して drop する。
    ///
    /// `SurfaceOutput::send` は infallible（`-> ()`）契約ゆえ、いかなる配送失敗でも panic させない。
    fn send(&mut self, command: DisplayCommand) {
        // 定義を替えた合図は荷物と突き合わせて scope ごとの置き換えの命令へ写す。
        if let DisplayCommand::Rebased { epoch, kind, shows } = command {
            self.send_rebased(epoch, kind, shows);
            return;
        }
        match map_display_command(command) {
            Some(cmd) => self.deliver(cmd),
            None => {
                tracing::warn!(
                    "PresentBridge: 数でない、または大きすぎる scope の DisplayCommand を写像できず drop しました"
                );
            }
        }
    }
}

impl PresentBridge {
    /// 1 件を UI へ非ブロック配送する。受信端 drop（shutdown 中の期待事象）は panic させず
    /// `debug!` で観測して破棄する。
    fn deliver(&self, cmd: PresentCommand) {
        if self.tx.send(cmd).is_err() {
            tracing::debug!(
                "PresentBridge: PresentCommand の配送先（UI receiver）が drop 済み（shutdown 中）— 破棄"
            );
        }
        // 表示指令が UI へ届いた＝次の画面更新に仕事がある。送出の**後**に立てる——先に立てると、
        // 読み取りと送出の隙間で旗だけが倒れ、指令が次の起床まで置き去りになる。逆順なら余分に 1 回回るだけで済む。
        tick_wake::mark(tick_wake::PRESENT);
    }

    /// 合図 `Rebased` を、置き場の世代の一致する荷物と突き合わせて scope ごとの
    /// `ReplaceTarget` へ写す。
    ///
    /// - 対象の番号は `target_map` の `shell_target`／`balloon_target`。
    /// - 最初の表示は合図の同じ scope の面と着せ替え。合図に無い scope・面の無い scope は登録だけ。
    /// - 返信の送り手は荷物の同じ scope のものを載せる。
    /// - 荷物が無い・世代が違うときは `error!(rebased_payload_missing)` を 1 件残して何も送らない
    ///   （世代の違う荷物は置き場に残す）。UI は返信の送り手が届かないことで失敗を知る。
    fn send_rebased(&self, epoch: u64, kind: RebaseKind, shows: Vec<RebasedShow>) {
        // 鍵を持つのは取り出す一瞬だけ。毒は前の持ち主の panic で、中身の荷物は壊れていない。
        let payload = self.swap.as_ref().and_then(|slot| {
            slot.lock()
                .unwrap_or_else(PoisonError::into_inner)
                .take_if(|p| p.epoch == epoch)
        });
        let Some(SwapPayload {
            targets,
            mut replies,
            ..
        }) = payload
        else {
            tracing::error!(
                event = "rebased_payload_missing",
                epoch,
                ?kind,
                "PresentBridge: 合図の世代の荷物が置き場に無い — 置き換えの命令を送らない"
            );
            return;
        };
        let to_target = match kind {
            RebaseKind::Shell => shell_target,
            RebaseKind::Balloon => balloon_target,
        };
        for (scope, emo_world, atlas, author_dpi) in targets {
            let show = shows
                .iter()
                .find(|s| scope_of(&s.scope) == Some(scope))
                .and_then(|s| s.surface_id.map(|id| (id, s.binds.clone())));
            let reply = replies
                .iter()
                .position(|(s, _)| *s == scope)
                .map(|i| replies.swap_remove(i).1);
            self.deliver(PresentCommand::ReplaceTarget {
                target: to_target(scope),
                emo_world: Box::new(emo_world),
                atlas,
                author_dpi,
                show,
                reply,
            });
        }
    }
}

/// `PresentBridge` は `spawn_seriko(out)` で worker スレッドへ move されるため `O: SurfaceOutput + Send + 'static`
/// を満たす必要がある。将来 `!Send`／非 `'static` なフィールドが混入したらここでコンパイルを止める。
const _: fn() = || {
    fn assert_send_static<T: Send + 'static>() {}
    assert_send_static::<PresentBridge>();
};

#[cfg(test)]
#[path = "adapter_move_shift_tests.rs"]
mod move_shift_tests;
#[cfg(test)]
#[path = "adapter_rebased_tests.rs"]
mod rebased_tests;
#[cfg(test)]
#[path = "adapter_stage_tests.rs"]
mod stage_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::test_support::{capture_logs, count_level};
    use areka_emo_compose::{ComposeMethod, PatternFrame, PatternState};
    use areka_sakura::ActorKey;
    use std::sync::mpsc::{self, TryRecvError};

    /// 非空の `PatternState`（animation `anim_id` に surface `surf` の `Overlay` コマ 1 枚）を作る
    /// （`PatternState::default()` と等価でない＝pattern の非改変転写を反証する値）。
    fn pattern_of(anim_id: u32, surf: u32) -> PatternState {
        let mut p = PatternState::default();
        p.set(
            anim_id,
            PatternFrame {
                surface_id: surf,
                method: ComposeMethod::Overlay,
                x: 0,
                y: 0,
            },
        );
        p
    }

    /// `Show` → シェル表示対象の `ShowSurface`。`surface_id`（非自明値）・非既定 `binds`・非空
    /// `pattern` が非改変で透過することを反証する。`reply` は `None`。
    #[test]
    fn show_maps_to_shell_show_surface_transcribing_id_binds_and_pattern() {
        let binds = BindSet::from_ids([1100, 1500]);
        let pattern = pattern_of(7, 2110);
        let out = map_display_command(DisplayCommand::Show {
            scope: ActorKey::from("0"),
            surface_id: 2100,
            binds: binds.clone(),
            pattern: pattern.clone(),
        })
        .expect("数値 scope は Some を返すこと");

        match out {
            PresentCommand::ShowSurface {
                target,
                surface_id,
                binds: got,
                pattern: got_pattern,
                reply,
            } => {
                assert_eq!(
                    target,
                    shell_target(0),
                    "Show は shell 表示対象（偶数）へ写像"
                );
                assert_eq!(surface_id, 2100, "surface_id は非改変で転写されること");
                assert_eq!(got, binds, "binds はそのまま透過されること");
                assert_eq!(
                    got_pattern, pattern,
                    "pattern はそのまま非改変で透過されること"
                );
                assert!(reply.is_none(), "reply は常に None（撃ちっぱなし）");
            }
            _ => panic!("Show は ShowSurface へ写像されるべき"),
        }
    }

    /// `Hide` → シェル表示対象の `Hide`。`reply` は `None`。
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

    /// `ShowBalloon` → バルーン表示対象の `ShowSurface`。`surface_id`・`pattern` は非改変転写・
    /// `binds` は既定（空集合）。`reply` は `None`。
    #[test]
    fn show_balloon_maps_to_balloon_show_surface_with_default_binds() {
        let pattern = pattern_of(3, 9);
        let out = map_display_command(DisplayCommand::ShowBalloon {
            scope: ActorKey::from("0"),
            surface_id: 6,
            pattern: pattern.clone(),
        })
        .expect("数値 scope は Some を返すこと");

        match out {
            PresentCommand::ShowSurface {
                target,
                surface_id,
                binds,
                pattern: got_pattern,
                reply,
            } => {
                assert_eq!(
                    target,
                    balloon_target(0),
                    "ShowBalloon は balloon 表示対象（奇数）へ写像"
                );
                assert_eq!(surface_id, 6, "surface_id は非改変で転写（alias 非再適用）");
                assert_eq!(
                    binds,
                    BindSet::default(),
                    "binds は既定（空集合）であること"
                );
                assert_eq!(
                    got_pattern, pattern,
                    "pattern はそのまま非改変で透過されること"
                );
                assert!(reply.is_none(), "reply は常に None");
            }
            _ => panic!("ShowBalloon は ShowSurface へ写像されるべき"),
        }
    }

    /// `HideBalloon` → バルーン表示対象の `Hide`。`reply` は `None`。
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

    /// 大きすぎる scope（バルーンの対象の番号があふれる番号）の Show は写像せず `None`（panic しない）。
    #[test]
    fn a_show_for_a_too_large_scope_is_not_mapped() {
        assert!(
            map_display_command(DisplayCommand::Show {
                scope: ActorKey::from("2147483648"),
                surface_id: 2100,
                binds: BindSet::from_ids([1100]),
                pattern: PatternState::default(),
            })
            .is_none()
        );
    }

    /// 非数値 scope（例 "側"）は Show／Hide／ShowBalloon／HideBalloon の写像で `None`（呼び手が `warn!`＋drop）。
    #[test]
    fn non_numeric_scope_returns_none_for_show_hide_and_balloon_variants() {
        let bad = || ActorKey::from("側");

        assert!(
            map_display_command(DisplayCommand::Show {
                scope: bad(),
                surface_id: 2100,
                binds: BindSet::from_ids([1100]),
                pattern: PatternState::default(),
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
                pattern: PatternState::default(),
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
    /// （target_map の採番規約が adapter を貫くこと）。
    #[test]
    fn shell_and_balloon_target_parity_flows_through() {
        for scope in [0u32, 1] {
            let key = ActorKey::from(scope.to_string());

            let show = map_display_command(DisplayCommand::Show {
                scope: key.clone(),
                surface_id: 10,
                binds: BindSet::default(),
                pattern: PatternState::default(),
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
                pattern: PatternState::default(),
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

    // ── PresentBridge（SurfaceOutput 本番実装）の檻 ──────────────────────
    //
    // ログ発火（warn/debug）の捕捉は `frame::test_support` の共有の道具を使う。

    /// 有効指令 → 変換済み `PresentCommand` が mpsc 越しに受信側へ**ちょうど 1 件**届く。
    /// `Show{scope:"0"}` が `ShowSurface{shell_target(0), id, binds, reply:None}` として全フィールド
    /// 一致で到達し、2 度目の `try_recv` が `Empty`（余分な送出なし）であることを反証する。
    #[test]
    fn send_valid_delivers_mapped_present_command_exactly_once() {
        let (tx, rx) = mpsc::channel::<PresentCommand>();
        let mut bridge = PresentBridge::new(tx);

        let pattern = pattern_of(7, 2110);
        bridge.send(DisplayCommand::Show {
            scope: ActorKey::from("0"),
            surface_id: 2100,
            binds: BindSet::from_ids([1100]),
            pattern: pattern.clone(),
        });

        // PresentCommand は #[non_exhaustive] かつ非 PartialEq ゆえフィールド分解＋`_` arm で照合する。
        match rx
            .try_recv()
            .expect("変換済み PresentCommand が 1 件届くこと")
        {
            PresentCommand::ShowSurface {
                target,
                surface_id,
                binds,
                pattern: got_pattern,
                reply,
            } => {
                assert_eq!(target, shell_target(0), "Show は shell 表示対象へ配送");
                assert_eq!(surface_id, 2100, "surface_id は非改変で転写");
                assert_eq!(binds, BindSet::from_ids([1100]), "binds はそのまま透過");
                assert_eq!(got_pattern, pattern, "pattern はそのまま非改変で透過");
                assert!(reply.is_none(), "reply は常に None（撃ちっぱなし）");
            }
            _ => panic!("Show は ShowSurface へ写像されて届くべき"),
        }

        assert!(
            matches!(rx.try_recv(), Err(TryRecvError::Empty)),
            "配送はちょうど 1 件（余分な送出がないこと）"
        );
    }

    /// 非数値 scope → 何も送出せず `warn!` がちょうど 1 回発火する。
    /// `Hide{scope:"側"}` 送出で (a) チャネルが空・(b) WARN がちょうど 1 件・(c) 送出自体を行わないため
    /// send 失敗の DEBUG は無い、ことを assert する。
    #[test]
    fn send_non_numeric_scope_emits_warn_and_sends_nothing() {
        let (tx, rx) = mpsc::channel::<PresentCommand>();
        let mut bridge = PresentBridge::new(tx);

        let logs = capture_logs(|| {
            bridge.send(DisplayCommand::Hide {
                scope: ActorKey::from("側"),
            });
        });

        assert!(
            matches!(rx.try_recv(), Err(TryRecvError::Empty)),
            "非数値 scope では何も送出しないこと"
        );
        assert_eq!(
            count_level(&logs, "WARN"),
            1,
            "非数値 scope の drop で warn がちょうど 1 回発火すること: {logs:?}"
        );
        assert_eq!(
            count_level(&logs, "DEBUG"),
            0,
            "送出自体を行わないため send 失敗の debug は発火しないこと: {logs:?}"
        );
    }

    /// 受信端 drop 後の送出は **panic せず** `debug!` 経路で握り潰す（shutdown 中の期待事象）。
    /// `SurfaceOutput::send` は infallible 契約ゆえ配送失敗でもクラッシュしてはならない。`rx` を drop
    /// 後に有効 `Show` を送り、(a) panic しない・(b) DEBUG がちょうど 1 件・(c) WARN は無い
    /// （数値 scope ゆえ写像は成功）ことを反証する。
    #[test]
    fn send_after_receiver_dropped_does_not_panic_and_logs_debug() {
        let (tx, rx) = mpsc::channel::<PresentCommand>();
        let mut bridge = PresentBridge::new(tx);
        drop(rx); // 受信端消滅＝shutdown 中を模す。

        let logs = capture_logs(|| {
            bridge.send(DisplayCommand::Show {
                scope: ActorKey::from("0"),
                surface_id: 2100,
                binds: BindSet::default(),
                pattern: PatternState::default(),
            });
        });

        assert_eq!(
            count_level(&logs, "DEBUG"),
            1,
            "受信端 drop 時の send 失敗で debug がちょうど 1 回発火すること: {logs:?}"
        );
        assert_eq!(
            count_level(&logs, "WARN"),
            0,
            "数値 scope の写像は成功するため warn は発火しないこと: {logs:?}"
        );
    }
}
