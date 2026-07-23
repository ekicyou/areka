//! 表示指令の変換と配送（薄い変換＋配送のみ・状態を持たない・R3.6）。
//!
//! `map_display_command`（`DisplayCommand`→`PresentCommand` の純変換・DD-5・`target_map` を利用）と
//! `PresentBridge`（seriko の `SurfaceOutput` 本番実装・`mpsc::Sender` へ非ブロック送出）を
//! 所有する。`ShowBalloon`／`HideBalloon` はバルーン表示対象へ `BindSet::default()` 付きで
//! 配送する（R5.1／R5.2）。送出失敗（受信端 drop）は `debug!`・非数値 scope の drop は
//! `warn!` で log-first 観測する（R3.7・design.md「Error Categories and Responses」）。
//!
//! 本ファイルは task 2.4（`map_display_command` 純変換）と task 2.5（`PresentBridge`＝
//! `SurfaceOutput` 本番実装・UI 配送）を実装する。

use areka_emo_compose::BindSet;
use areka_emo_present::PresentCommand;
use areka_seriko::{DisplayCommand, SurfaceOutput};

use crate::emo2_boot::target_map::{balloon_target, scope_of, shell_target};

/// `DisplayCommand` → `PresentCommand` の純変換（DD-5・`target_map` を利用）。
///
/// 可変状態・I/O を持たない純関数で、4 写像を与える（R3.6）:
/// - `Show { scope, surface_id, binds, pattern }` → シェル表示対象へ `ShowSurface`。`surface_id`
///   （数値）・`binds`・`pattern` を**非改変で転写**する（R3.1／R5.1）。
/// - `Hide { scope }` → シェル表示対象の `Hide`（R3.2）。
/// - `ShowBalloon { scope, surface_id, pattern }` → バルーン表示対象へ `ShowSurface`。`binds` は
///   `BindSet::default()`（空集合＝M-boot にバルーン着せ替えは存在しない・R3.3／R5.1）。
///   `surface_id` は seriko が解決済みの数値 id をそのまま転写し、**alias を再適用しない**（R5.3）。
///   `pattern` はシェル面 `Show` と同様に非改変で転写する（R5.1・cue 由来は空 pattern＝R5.4）。
/// - `HideBalloon { scope }` → バルーン表示対象の `Hide`（R3.4／R5.2）。
///
/// `reply` は常に `None`（撃ちっぱなし・fire-and-forget）。scope（`ActorKey`）が非数値のとき
/// （`scope_of` が `None`）は本関数も `None` を返し、呼び手（`PresentBridge::send`）が `warn!` ＋
/// 当該指令 drop で握り潰さず log-first 観測する（R3.7）。
///
/// `DisplayCommand` は `#[non_exhaustive]` でないため 4 variant の網羅 `match` で全経路を尽くす。
pub fn map_display_command(cmd: DisplayCommand) -> Option<PresentCommand> {
    Some(match cmd {
        // Show: シェル表示対象へそのまま。surface_id・binds・pattern は非改変で転写（R3.1／R5.1／R5.3）。
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
        // Hide: シェル表示対象の非表示（R3.2）。
        DisplayCommand::Hide { scope } => PresentCommand::Hide {
            target: shell_target(scope_of(&scope)?),
            reply: None,
        },
        // ShowBalloon: バルーン表示対象へ。binds は既定（空集合＝バルーン着せ替え無し・R3.3／R5.1）。
        // surface_id は seriko 解決済み数値 id をそのまま転写（alias 非再適用・R5.3）。pattern は
        // シェル面 Show と同様に非改変で転写する（R5.1・cue 由来は空 pattern＝R5.4）。
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
        // HideBalloon: バルーン表示対象の非表示（`\b[-1]` 相当・R3.4／R5.2）。
        DisplayCommand::HideBalloon { scope } => PresentCommand::Hide {
            target: balloon_target(scope_of(&scope)?),
            reply: None,
        },
    })
}

/// seriko の `SurfaceOutput` 本番実装（task 2.5）。
///
/// `spawn_seriko(out = PresentBridge)` で seriko worker スレッドへ move され（`O: SurfaceOutput
/// + Send + 'static`・実結線は task 5.1）、worker 上で受けた `DisplayCommand` を
/// [`map_display_command`] で `PresentCommand` へ純変換し、UI スレッドの表示層（frame drain 側の
/// `Receiver`）へ `mpsc::Sender` で**非ブロック**配送する（R3.7）。
///
/// 可変状態を一切持たず、フィールドは配送用 `Sender` 1 本のみ（R3.6「変換と配送のみを行い、
/// 状態を保持しない」）。変換は [`map_display_command`] の純関数へ、配送は `Sender::send` へ委ね、
/// 本型自身は両者を繋ぐだけの薄いアダプタに徹する（キャッシュ・カウンタ等の内部状態を持たない）。
pub struct PresentBridge {
    /// UI フレーム drain 側 `Receiver` への非ブロック配送口（唯一のフィールド＝無状態・R3.6）。
    tx: std::sync::mpsc::Sender<PresentCommand>,
}

impl PresentBridge {
    /// `PresentCommand` の配送先 `Sender`（UI フレーム drain 側）を与えて構築する。
    pub fn new(tx: std::sync::mpsc::Sender<PresentCommand>) -> Self {
        Self { tx }
    }
}

impl SurfaceOutput for PresentBridge {
    /// seriko worker から届いた `DisplayCommand` を写像し、成功時のみ UI へ非ブロック配送する。
    ///
    /// - 写像成功（数値 scope）: `PresentCommand` を `tx.send`。受信端 drop（＝shutdown 中の
    ///   期待事象）で失敗しても **panic せず** `debug!` で観測して破棄する（log-first・R3.7）。
    /// - 写像不能（非数値 scope＝[`map_display_command`] が `None`）: `warn!` で観測して drop する
    ///   （握り潰さない・design.md「Error Categories and Responses」配送時）。
    ///
    /// `SurfaceOutput::send` は infallible（`-> ()`）契約ゆえ、いかなる配送失敗でも panic させない。
    fn send(&mut self, command: DisplayCommand) {
        match map_display_command(command) {
            // 写像成功: UI フレーム drain 側へ非ブロック配送。受信端 drop（shutdown 中の期待事象）は
            // panic させず debug! で観測して破棄する（log-first・R3.7）。
            Some(cmd) => {
                if self.tx.send(cmd).is_err() {
                    tracing::debug!(
                        "PresentBridge: PresentCommand の配送先（UI receiver）が drop 済み（shutdown 中）— 破棄"
                    );
                }
            }
            // 写像不能（非数値 scope）: 握り潰さず warn! で観測して drop（R3.6/R3.7・DD-5）。
            None => {
                tracing::warn!(
                    "PresentBridge: 非数値 scope の DisplayCommand を写像できず drop しました"
                );
            }
        }
    }
}

/// 静的境界表明（撃ちっぱなし檻）: `PresentBridge` は `spawn_seriko(out)` で worker スレッドへ
/// move されるため `O: SurfaceOutput + Send + 'static` を満たす必要がある（実結線は task 5.1）。
/// `mpsc::Sender<T>` は `T: Send` のとき `Send`・`PresentCommand` は `'static` ゆえ現状は充足するが、
/// 将来 `!Send`／非 `'static` なフィールドが混入したらここでコンパイルを止める。
const _: fn() = || {
    fn assert_send_static<T: Send + 'static>() {}
    assert_send_static::<PresentBridge>();
};

#[cfg(test)]
mod tests {
    use super::*;
    use areka_emo_compose::{ComposeMethod, PatternFrame, PatternState};
    use areka_sakura::ActorKey;
    use std::sync::mpsc::{self, TryRecvError};
    use std::sync::{Arc, Mutex};
    use tracing::field::{Field, Visit};
    use tracing_subscriber::prelude::*;

    /// 非空の `PatternState`（animation `anim_id` に surface `surf` の `Overlay` コマ 1 枚）を作る
    /// （`PatternState::default()` と等価でない＝pattern 透過の非改変転写を反証する load-bearing 値）。
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
    /// `pattern` が非改変で透過することを反証する（R3.1／R5.1／R5.3）。`reply` は `None`。
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
                assert_eq!(target, shell_target(0), "Show は shell 表示対象（偶数）へ写像");
                assert_eq!(surface_id, 2100, "surface_id は非改変で転写されること");
                assert_eq!(got, binds, "binds はそのまま透過されること");
                assert_eq!(got_pattern, pattern, "pattern はそのまま非改変で透過されること（R5.1）");
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

    /// `ShowBalloon` → バルーン表示対象の `ShowSurface`。`surface_id`・`pattern` は非改変転写・
    /// `binds` は既定（空集合）（R3.3／R5.1／R5.3）。`reply` は `None`。
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
                assert_eq!(binds, BindSet::default(), "binds は既定（空集合）であること");
                assert_eq!(got_pattern, pattern, "pattern はそのまま非改変で透過されること（R5.1）");
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
    /// （target_map の DD-3 採番規約が adapter を貫くこと）。
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

    // ── task 2.5: PresentBridge（SurfaceOutput 本番実装）の檻 ──────────────────────
    //
    // ログ発火（warn/debug）を目視でなく実行テストで決定論的に檻へ入れるため、`log_capture.rs`
    // （areka-emo-compose／areka-emo-atlas）の確立パターンを、単一ファイル境界（adapter.rs）内へ
    // 最小インライン複製する。スレッドローカル `with_default` ゆえ `cargo test` 並行実行でも
    // 他スレッドの subscriber と干渉しない（決定論網羅マンデート）。

    /// イベントの `level`＋各フィールドを 1 行文字列へ整形して共有 Vec へ push する最小 Layer。
    #[derive(Clone, Default)]
    struct Capture(Arc<Mutex<Vec<String>>>);

    impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for Capture {
        fn on_event(
            &self,
            ev: &tracing::Event<'_>,
            _: tracing_subscriber::layer::Context<'_, S>,
        ) {
            let meta = ev.metadata();
            let mut line = format!("level={} target={}", meta.level(), meta.target());
            struct V<'a>(&'a mut String);
            impl Visit for V<'_> {
                fn record_debug(&mut self, f: &Field, v: &dyn std::fmt::Debug) {
                    use std::fmt::Write;
                    let _ = write!(self.0, " {}={:?}", f.name(), v);
                }
            }
            ev.record(&mut V(&mut line));
            // Mutex 汚染時もテストは失敗させたいので unwrap（本番経路ではない）。
            self.0.lock().unwrap().push(line);
        }
    }

    /// クロージャ `f` 実行中に**現在のスレッド**で発火した tracing イベントを 1 行 1 件で返す。
    fn capture_logs<F: FnOnce()>(f: F) -> Vec<String> {
        let cap = Capture::default();
        let logs = cap.0.clone();
        let subscriber = tracing_subscriber::registry().with(cap);
        tracing::subscriber::with_default(subscriber, f);
        let guard = logs.lock().unwrap();
        guard.clone()
    }

    /// 捕捉行のうち指定 level（例 `"WARN"`／`"DEBUG"`）の件数を数える。
    fn count_level(logs: &[String], level: &str) -> usize {
        let needle = format!("level={level}");
        logs.iter().filter(|l| l.contains(&needle)).count()
    }

    /// 有効指令 → 変換済み `PresentCommand` が mpsc 越しに受信側へ**ちょうど 1 件**届く（R3.7）。
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

        // PresentCommand は #[non_exhaustive] かつ非 PartialEq ゆえフィールド分解＋`_` arm で照合。
        match rx.try_recv().expect("変換済み PresentCommand が 1 件届くこと") {
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
                assert_eq!(got_pattern, pattern, "pattern はそのまま非改変で透過（R5.1）");
                assert!(reply.is_none(), "reply は常に None（撃ちっぱなし）");
            }
            _ => panic!("Show は ShowSurface へ写像されて届くべき"),
        }

        assert!(
            matches!(rx.try_recv(), Err(TryRecvError::Empty)),
            "配送はちょうど 1 件（余分な送出がないこと）"
        );
    }

    /// 非数値 scope → 何も送出せず `warn!` がちょうど 1 回発火する（R3.6/R3.7・log-first の決定論檻）。
    /// スレッドローカル捕捉 subscriber を差し込み、`Hide{scope:"側"}` 送出で (a) チャネルが空
    /// （何も送らない）・(b) WARN がちょうど 1 件発火・(c) 送出自体を行わないため send 失敗の DEBUG は
    /// 無い、ことを assert する。ログ発火を可視目視でなく実行テストで檻に入れる（決定論網羅マンデート）。
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

    /// 受信端 drop 後の送出は **panic せず** `debug!` 経路で握り潰す（R3.7・shutdown 中の期待事象）。
    /// `SurfaceOutput::send` は infallible 契約ゆえ配送失敗でもクラッシュしてはならない。`rx` を drop
    /// 後に有効 `Show` を送り、(a) 関数が正常復帰（panic しない）・(b) DEBUG がちょうど 1 件・(c) WARN は
    /// 無い（数値 scope ゆえ写像は成功）ことを決定論的に反証する。
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
        // ここへ到達した時点で send は panic しなかった（infallible 契約の実証）。

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
