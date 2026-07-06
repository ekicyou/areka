//! アクター駆動モジュール — inbox メッセージ・SurfaceSink ブリッジ・停止経路の土台。
//!
//! 本タスク（3.1）は **受け口**（`SurfaceSink` を実装する [`SerikoSink`］ブリッジ）と
//! inbox メッセージ列挙（[`SerikoMsg`]）・停止経路の「形」（mpsc チャネル）だけを持つ。
//! アクター本体の spawn（`spawn_seriko`）・inbox ハンドラ（resolve→state→emit）・単一発行点
//! （`emit_display`）は後続タスク（3.2）の領分でここには置かない。
//!
//! # 結線契約（受け口＝trait 実装）
//!
//! sakura（④）の surface 系出力先は [`areka_sakura::SurfaceSink`] 一本であり、その実装
//! [`SerikoSink`] が唯一の差し込み口となる（追加の口を設けない・ghost-setup 期待）。
//! `SerikoSink::emit` は届いた [`TalkCue`] を [`SerikoMsg::Cue`] として専用 inbox（std mpsc）
//! へ橋渡しする。
//!
//! # 停止経路の形
//!
//! inbox は std mpsc チャネル。停止は 2 経路——(1) [`SerikoMsg::Close`] 受領、(2) 全 [`SerikoSink`]
//! （＝全 `Sender`）drop による受信端の `RecvError`——で、後続 3.2 が結ぶ `run_inbox` ループを
//! 正常終了させる。本タスクは列挙とチャネル形のみ用意し、ループ自体は結線しない。
//!
//! # 失敗経路のログ規律（infallible・silent failure 禁止）
//!
//! `SurfaceSink::emit` は infallible（`()` 返し）。inbox 全受信端が消失した後の送出は
//! `send` が `Err` を返すが、`unwrap`／`expect` で panic させず [`tracing::error!`] で
//! 観測して戻る（log-first・R6.3／通常入力で panic しない・R6.4）。

use areka_sakura::{SurfaceSink, TalkCue};

/// seriko アクターの inbox メッセージ（areka-actor inbox 規約・投函経路は inbox 一貫）。
///
/// 共有 Close 型は無い規約に従い、`SakuraMsg::Close` を先例に自前 `Close` を持つ（DD3）。
#[derive(Debug)]
pub enum SerikoMsg {
    /// surface 系発火（`SerikoSink::emit` が橋渡しする・到着順に適用＝R1.5）。
    Cue(TalkCue),
    /// kanade 由来の停止指令（areka-actor 停止規約の Close 相当・正常終了させる）。
    Close,
}

/// [`SurfaceSink`] を実装する送出ブリッジ（sakura dispatcher が保持する結線契約）。
///
/// inbox（std mpsc）の `Sender` を内包し、届いた発火を [`SerikoMsg::Cue`] として橋渡しする。
/// この trait 実装が唯一の受け口＝差し込み口であり、追加の注入メソッドは設けない。
pub struct SerikoSink {
    tx: std::sync::mpsc::Sender<SerikoMsg>,
}

impl SerikoSink {
    /// inbox の `Sender` からブリッジを組む。
    ///
    /// 後続 3.2 の `spawn_seriko` がアクター inbox の送信端から構築する。std mpsc の
    /// `Sender` は `Clone`（複製すれば複数 sink 口へ配れる）だが、本タスクでは単一送出端で足りる。
    pub(crate) fn new(tx: std::sync::mpsc::Sender<SerikoMsg>) -> Self {
        Self { tx }
    }
}

impl SurfaceSink for SerikoSink {
    /// 1 発火を inbox へ橋渡しする（infallible）。
    ///
    /// 受信端（inbox／アクター）が消失していると `send` は `Err` を返すが、`unwrap`／`expect`
    /// では panic するため用いず、[`tracing::error!`] で落とした発火を記録して戻る
    /// （silent failure 禁止・R6.3／通常運転で panic しない・R6.4）。
    fn emit(&mut self, cue: TalkCue) {
        if let Err(err) = self.tx.send(SerikoMsg::Cue(cue)) {
            // Err の内側に move された SerikoMsg::Cue から発火の識別情報を復元してログへ載せる。
            let SerikoMsg::Cue(dropped) = err.0 else {
                unreachable!("emit が送るのは常に SerikoMsg::Cue");
            };
            tracing::error!(
                at = dropped.at,
                scope = %dropped.actor,
                command = ?dropped.command,
                "seriko inbox が消失: surface 発火を配送できず破棄した（受信端全消失）"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use areka_sakura::{ActorKey, CueCommand, SurfaceSink, TalkCue};

    /// テスト用の TalkCue（Shell 系 Emote・at/actor 込み）を組む。
    fn emote_cue(at: f64, scope: &str, key: &str) -> TalkCue {
        TalkCue {
            at,
            actor: ActorKey::from(scope),
            command: CueCommand::Emote { key: key.into() },
        }
    }

    /// 受信端が生きていれば `emit` が `SerikoMsg::Cue` を inbox へ橋渡しする（R1.1/1.2）。
    /// trait 実装＝結線契約（追加の口を設けず SurfaceSink 経由でのみ届く）。
    #[test]
    fn emit_forwards_cue_to_inbox() {
        let (tx, rx) = std::sync::mpsc::channel::<SerikoMsg>();
        let mut sink = SerikoSink::new(tx);

        SurfaceSink::emit(&mut sink, emote_cue(1.5, "0", "smile"));

        match rx.recv() {
            Ok(SerikoMsg::Cue(cue)) => {
                assert_eq!(cue.at, 1.5, "at が保たれる");
                assert_eq!(cue.actor, ActorKey::from("0"), "actor が保たれる");
                assert!(
                    matches!(cue.command, CueCommand::Emote { ref key } if key == "smile"),
                    "Emote{{key}} が保たれる: {:?}",
                    cue.command
                );
            }
            other => panic!("Cue が inbox へ届くこと: {other:?}"),
        }
    }

    /// 受信端（inbox/アクター）消失後に `emit` を呼んでも、`error!` ログを残しつつ
    /// panic せず正常に戻る（infallible 契約・send 失敗を黙殺しない・R6.3/6.4）。
    #[test]
    fn emit_after_receiver_gone_logs_no_panic() {
        let (tx, rx) = std::sync::mpsc::channel::<SerikoMsg>();
        let mut sink = SerikoSink::new(tx);
        // 受信端を drop＝アクター停止後（inbox 全受信端消失）を模す。
        drop(rx);

        // emit は infallible。send 失敗経路（Err）を通り、panic せず戻ること自体が合格条件。
        let logs = capture_logs(|| {
            SurfaceSink::emit(&mut sink, emote_cue(0.0, "0", "smile"));
        });

        // silent failure 禁止（R6.3）: send 失敗が error! として観測できること。
        assert!(
            logs.contains("level=ERROR"),
            "send 失敗が error! ログとして発火すること: {logs}"
        );
        assert!(
            logs.contains("target=areka_seriko"),
            "本クレート target で発火すること: {logs}"
        );
    }

    /// テスト専用 tracing 捕捉ハーネス（emo-compose/kanade の log_capture 流儀・
    /// スレッドローカル `with_default` ゆえ並行テスト安全）。
    fn capture_logs<F: FnOnce()>(f: F) -> String {
        use std::sync::{Arc, Mutex};
        use tracing::field::{Field, Visit};
        use tracing_subscriber::prelude::*;

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
                self.0.lock().unwrap().push(line);
            }
        }

        let cap = Capture::default();
        let logs = cap.0.clone();
        let subscriber = tracing_subscriber::registry().with(cap);
        tracing::subscriber::with_default(subscriber, f);
        let guard = logs.lock().unwrap();
        guard.join("\n")
    }
}
