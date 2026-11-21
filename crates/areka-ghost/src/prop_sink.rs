//! `areka.prop.set`（汎用プロパティ SET キュー）の受け口。
//!
//! broadcast された全 cue のうち [`PROP_SET_CUE_NAME`] のキャリアだけを選び、カウンタ key 族に
//! 限った書込 key 統制を通して `SylphyaPublisher::persist_put`（待たない）へ落とす。
//! `areka-runtime` の `MoveCueSink`（名前で自分の分を選ぶ）と `sylphya_wiring.rs` の
//! `make_username_resource_sink`（publisher を捕まえる sink）を合わせた形。

use areka_sakura::contract::{CueCommand, CueSink, TalkCue};
use areka_sylphya::{PersistKey, PersistScope, SylphyaPublisher};
use tracing::{debug, info, warn};

/// 汎用プロパティ SET キューのコマンド名（`areka.` 接頭辞で ukadoc 正典のコマンド名と衝突しない）。
///
/// 代入だけで、増分の `areka.prop.inc` は名前だけ予約して未実装。
pub const PROP_SET_CUE_NAME: &str = "areka.prop.set";

/// `areka.prop.set` キューの talk スレッド側の受け口。
///
/// [`PROP_SET_CUE_NAME`] のキャリアだけを選び、書込 key 統制（[`PersistKey::BootCount`]／
/// [`PersistKey::VanishCount`] のみ受理）を通して `SylphyaPublisher::persist_put` へ落とす。
/// 非キャリア・担当外のコマンド名・key 統制外・引数不足は、記録を残して読み飛ばす。
///
/// # 書込 key 統制
///
/// 位置 key（`WindowPos`／`BalloonOffset`）は cue 側から書けない。位置を永続へ書くのは
/// ユーザーのドラッグ 2 点だけに限る。
///
/// # Clone + Send
///
/// dispatcher が talk ごとに clone する（[`crate::sink::BootCueSink`]）。全 clone は同じ
/// アクター送信端を指す。
#[derive(Clone)]
pub struct PropSetCueSink {
    publisher: SylphyaPublisher,
}

impl PropSetCueSink {
    /// sylphya publisher（`boot` が保持する送信端）から sink を構築する。
    pub fn new(publisher: SylphyaPublisher) -> Self {
        Self { publisher }
    }
}

/// broadcast では担当外の cue（`Text`／`Emote`／他のコマンド名のキャリア）も届くので、
/// 名前で選び、[`PROP_SET_CUE_NAME`] 以外は記録を残して読み飛ばす。
impl CueSink for PropSetCueSink {
    fn emit(&mut self, cue: TalkCue) {
        // `Custom` で params が非正準のときは宛名で重さを分ける。自分宛の壊れ物は warn、
        // 他人宛・未知名は担当外なので debug。
        let Some((name, tokens)) = cue.command.as_command_carrier() else {
            match &cue.command {
                CueCommand::Custom { command, .. } if command == PROP_SET_CUE_NAME => warn!(
                    command = ?cue.command,
                    "PropSetCueSink: 自分宛（{PROP_SET_CUE_NAME}）の非正準 Custom params を良性スキップ"
                ),
                CueCommand::Custom { .. } => debug!(
                    command = ?cue.command,
                    "PropSetCueSink: 他人宛/未知名の非正準 Custom params を良性スキップ（担当外）"
                ),
                _ => debug!(
                    command = ?cue.command,
                    "PropSetCueSink: 非キャリア cue を良性スキップ（担当外・R6.2）"
                ),
            }
            return;
        };

        if name != PROP_SET_CUE_NAME {
            debug!(
                name,
                "PropSetCueSink: 担当外コマンド名を良性スキップ（名前自己選別）"
            );
            return;
        }

        if tokens.len() < 2 {
            warn!(
                token_count = tokens.len(),
                "PropSetCueSink: 引数不足（[key, 値] を要する）のため {PROP_SET_CUE_NAME} をスキップ"
            );
            return;
        }
        let key_token = tokens[0];
        let value = tokens[1];

        // カウンタ key 族だけ受理する。位置 key・未知 key は書かせない。
        let key = if key_token == PersistKey::BootCount.to_canonical_key() {
            PersistKey::BootCount
        } else if key_token == PersistKey::VanishCount.to_canonical_key() {
            PersistKey::VanishCount
        } else {
            warn!(
                key = key_token,
                "PropSetCueSink: カウンタ key 族外（位置 key／未知 key）は cue 側から書けない——warn スキップ（1.9 単一ライター規律）"
            );
            return;
        };

        // talk スレッドを止めないよう、Ghost スコープへ待たずに書く。
        self.publisher
            .persist_put(PersistScope::Ghost, vec![(key, value.to_string())]);
        info!(key = key_token, value, "prop_set_cue applied");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use areka_sakura::contract::{ActorKey, CueCommand, CueSink, TalkCue};
    use areka_sylphya::persist::FakePersistIo;
    use areka_sylphya::{
        PersistKey, PersistScope, ScopeRoots, SylphyaInit, load_scope, spawn_sylphya,
    };
    use std::path::PathBuf;
    use std::sync::Arc;

    /// `\![name,tokens...]` 汎用キャリア cue（正準形）。
    fn carrier_cue(actor: &str, name: &str, tokens: &[&str]) -> TalkCue {
        let toks: Vec<String> = tokens.iter().map(|s| s.to_string()).collect();
        TalkCue {
            at: 0.0,
            actor: ActorKey::from(actor),
            command: CueCommand::command_carrier(name, toks),
            duration: 0.0,
        }
    }

    /// ghost root 付きの共有 FakeIo の sylphya（書込を別ハンドルで観測する）。
    fn spawn_with_shared_io() -> (areka_sylphya::SylphyaParts, ScopeRoots, Arc<FakePersistIo>) {
        let shared = Arc::new(FakePersistIo::new());
        let roots = ScopeRoots {
            ghost: Some(PathBuf::from("/g")),
            ..ScopeRoots::default()
        };
        let parts = spawn_sylphya(SylphyaInit {
            roots: roots.clone(),
            io: Box::new(shared.clone()),
            runtime_sink: None,
        });
        (parts, roots, shared)
    }

    /// カウンタ key（`areka.boot.count`,"1"）は Ghost スコープへ書かれ、`load_scope` が `BootCount="1"` を観測する。
    #[test]
    fn boot_count_carrier_is_persisted() {
        let (parts, roots, shared) = spawn_with_shared_io();
        let mut sink = PropSetCueSink::new(parts.publisher.clone());

        sink.emit(carrier_cue(
            "0",
            PROP_SET_CUE_NAME,
            &["areka.boot.count", "1"],
        ));
        parts
            .publisher
            .barrier()
            .expect("barrier while actor alive");

        let loaded = load_scope(PersistScope::Ghost, &roots, &shared);
        assert!(
            loaded.contains(&(PersistKey::BootCount, "1".into())),
            "areka.prop.set + areka.boot.count は BootCount=\"1\" として永続される: {loaded:?}"
        );

        parts.publisher.close();
        parts.handle.join().expect("clean close joins");
    }

    /// `areka.vanish.count` もカウンタ key 族として受理される。
    #[test]
    fn vanish_count_carrier_is_persisted() {
        let (parts, roots, shared) = spawn_with_shared_io();
        let mut sink = PropSetCueSink::new(parts.publisher.clone());

        sink.emit(carrier_cue(
            "0",
            PROP_SET_CUE_NAME,
            &["areka.vanish.count", "7"],
        ));
        parts.publisher.barrier().expect("barrier");

        let loaded = load_scope(PersistScope::Ghost, &roots, &shared);
        assert!(
            loaded.contains(&(PersistKey::VanishCount, "7".into())),
            "areka.vanish.count はカウンタ key 族ゆえ受理される: {loaded:?}"
        );

        parts.publisher.close();
        parts.handle.join().expect("clean close joins");
    }

    /// 位置 key（`areka.window.scope.x`）は拒否し、何も書かない。
    #[test]
    fn window_pos_key_is_rejected_no_write() {
        let (parts, roots, shared) = spawn_with_shared_io();
        let mut sink = PropSetCueSink::new(parts.publisher.clone());

        // 2 トークンなので引数不足は通り、key 統制で弾かれる。
        let pos_key = PersistKey::WindowPos {
            scope: 0,
            axis: areka_sylphya::Axis::X,
        }
        .to_canonical_key();
        sink.emit(carrier_cue("0", PROP_SET_CUE_NAME, &[&pos_key, "123"]));
        parts.publisher.barrier().expect("barrier");

        let loaded = load_scope(PersistScope::Ghost, &roots, &shared);
        assert!(
            loaded.is_empty(),
            "位置 key は cue 側から書けない（何も永続されない）: {loaded:?}"
        );

        parts.publisher.close();
        parts.handle.join().expect("clean close joins");
    }

    /// 担当外のコマンド名（`move`/`bind`/未知名）は何も書かない。
    #[test]
    fn foreign_command_name_is_skipped_no_write() {
        for name in ["move", "bind", "unknownfoo"] {
            let (parts, roots, shared) = spawn_with_shared_io();
            let mut sink = PropSetCueSink::new(parts.publisher.clone());

            sink.emit(carrier_cue("0", name, &["areka.boot.count", "1"]));
            parts.publisher.barrier().expect("barrier");

            let loaded = load_scope(PersistScope::Ghost, &roots, &shared);
            assert!(
                loaded.is_empty(),
                "担当外名 {name} は良性スキップ（何も書かない）: {loaded:?}"
            );

            parts.publisher.close();
            parts.handle.join().expect("clean close joins");
        }
    }

    /// 引数不足（tokens < 2）は何も書かない。
    #[test]
    fn insufficient_args_is_skipped_no_write() {
        let (parts, roots, shared) = spawn_with_shared_io();
        let mut sink = PropSetCueSink::new(parts.publisher.clone());

        sink.emit(carrier_cue("0", PROP_SET_CUE_NAME, &["areka.boot.count"]));
        parts.publisher.barrier().expect("barrier");

        let loaded = load_scope(PersistScope::Ghost, &roots, &shared);
        assert!(
            loaded.is_empty(),
            "引数不足（<2 トークン）は何も書かない: {loaded:?}"
        );

        parts.publisher.close();
        parts.handle.join().expect("clean close joins");
    }

    /// 非キャリア cue（`Text` 等）も何も書かない。
    #[test]
    fn non_carrier_cue_is_skipped_no_write() {
        let (parts, roots, shared) = spawn_with_shared_io();
        let mut sink = PropSetCueSink::new(parts.publisher.clone());

        sink.emit(TalkCue {
            at: 0.0,
            actor: ActorKey::from("0"),
            command: CueCommand::Text("こんにちは".into()),
            duration: 0.0,
        });
        parts.publisher.barrier().expect("barrier");

        let loaded = load_scope(PersistScope::Ghost, &roots, &shared);
        assert!(loaded.is_empty(), "非キャリアは何も書かない: {loaded:?}");

        parts.publisher.close();
        parts.handle.join().expect("clean close joins");
    }

    /// `PropSetCueSink` が `BootCueSink` の境界を満たす（`Vec<Box<dyn BootCueSink>>` へ登録できる）。
    #[test]
    fn satisfies_boot_sink_bounds() {
        fn require_boot_sink<T: crate::sink::BootCueSink>() {}
        require_boot_sink::<PropSetCueSink>();
    }

    /// 起動記録は完走した時だけ書かれることの通しのテスト。
    ///
    /// 実の sakura 再生（`spawn_talk`・注入時刻 Tick）で、初回挨拶台本の末尾に付く epilogue
    /// （`areka.prop.set` / `areka.boot.count` / "1"）を駆動し、`PropSetCueSink` を実 sylphya へ結線する。
    /// - 完走すると末尾の SET cue が発火し、`BootCount="1"` が書かれる。
    /// - horizon 前に `Close` すると残 cue ごと捨てられ、SET は発火せず何も書かれない（次回も初回扱い）。
    ///
    /// 記録だけの sink も並べ、cue の発火そのものも固定する。
    mod completion_only_records_integration {
        use super::*;
        use areka_sakura::contract::{
            ChoiceWaiting, CueSink, SakuraMsg, ScriptOrigin, StartTalk, SystemVarSnapshot, TalkCue,
            TalkDone, TalkEndReason, TalkId,
        };
        use areka_sakura::spawn_talk;
        use areka_talk::EpilogueCommand;
        use std::sync::Mutex;
        use std::sync::mpsc;
        use std::time::Duration;

        /// `spawn_talk` の done ポートの境界（`From<TalkDone> + From<ChoiceWaiting>`）を満たす合流 enum。
        /// 台本に選択肢（`\q`）は無く `ChoiceWaiting` は発生しない。
        enum TalkNotice {
            Done(TalkDone),
            #[allow(dead_code)]
            ChoiceWaiting(ChoiceWaiting),
        }
        impl From<TalkDone> for TalkNotice {
            fn from(done: TalkDone) -> Self {
                TalkNotice::Done(done)
            }
        }
        impl From<ChoiceWaiting> for TalkNotice {
            fn from(waiting: ChoiceWaiting) -> Self {
                TalkNotice::ChoiceWaiting(waiting)
            }
        }

        /// broadcast で届いた全 cue を蓄積する観測 sink。
        #[derive(Clone)]
        struct ObservingSink {
            records: Arc<Mutex<Vec<TalkCue>>>,
        }
        impl ObservingSink {
            fn new() -> (Self, Arc<Mutex<Vec<TalkCue>>>) {
                let records = Arc::new(Mutex::new(Vec::new()));
                (
                    Self {
                        records: Arc::clone(&records),
                    },
                    records,
                )
            }
        }
        impl CueSink for ObservingSink {
            fn emit(&mut self, cue: TalkCue) {
                self.records
                    .lock()
                    .expect("ObservingSink records mutex poisoned")
                    .push(cue);
            }
        }

        /// 記録から carrier のコマンド名の列を取り出す。
        fn carrier_names(records: &Arc<Mutex<Vec<TalkCue>>>) -> Vec<String> {
            records
                .lock()
                .unwrap()
                .iter()
                .filter_map(|c| c.command.as_command_carrier().map(|(n, _)| n.to_string()))
                .collect()
        }

        /// 初回挨拶台本（`hi\e`）に起動記録の epilogue を付けたもの。horizon（`hi` の再生完了＝0.1s）の
        /// 末尾に `areka.prop.set` / `areka.boot.count` / "1" が duration 0 で付く。
        fn first_boot_start() -> StartTalk {
            StartTalk {
                talk_id: TalkId(8005),
                script: r"hi\e".to_string(),
                // `apply_boot_record_gate` が初回起動で据える epilogue と同じ形。
                epilogue: vec![EpilogueCommand {
                    name: PROP_SET_CUE_NAME.to_string(),
                    tokens: vec![PersistKey::BootCount.to_canonical_key(), "1".to_string()],
                }],
                origin: ScriptOrigin::GHOST,
                strict: false,
            }
        }

        /// 完走すると末尾の SET cue が発火し、`BootCount="1"` が書かれる。
        #[test]
        fn completion_fires_tail_set_cue_and_records_boot_count() {
            let (parts, roots, shared) = spawn_with_shared_io();
            let prop_sink = PropSetCueSink::new(parts.publisher.clone());
            let (observer, observed) = ObservingSink::new();

            let (done_tx, done_rx) = mpsc::channel::<TalkNotice>();
            let sinks: Vec<Box<dyn CueSink + Send>> = vec![Box::new(prop_sink), Box::new(observer)];
            let handle = spawn_talk(
                first_boot_start(),
                done_tx,
                sinks,
                SystemVarSnapshot::default(),
                None,
            );

            // 初回 Tick で時刻を刻む。挨拶 cue は出て、末尾の SET は保留。
            handle.inbox.send(SakuraMsg::Tick(0.0)).unwrap();
            // horizon を跨ぐ Tick で末尾の SET が発火し、完走する。
            handle.inbox.send(SakuraMsg::Tick(1.0)).unwrap();

            let TalkNotice::Done(done) = done_rx
                .recv_timeout(Duration::from_secs(5))
                .expect("horizon 到達で TalkDone（完走）")
            else {
                panic!("選択肢を含まない台本では ChoiceWaiting は流れない（TalkDone のみ）");
            };
            assert_eq!(
                done.reason,
                TalkEndReason::Ended,
                "`\\e` 終端の完走は Ended"
            );
            handle.actor.join().expect("body は正常終了する");

            // 待たない書込を barrier で確定してから観測する。
            parts
                .publisher
                .barrier()
                .expect("barrier while actor alive");
            let loaded = load_scope(PersistScope::Ghost, &roots, &shared);
            assert!(
                loaded.contains(&(PersistKey::BootCount, "1".into())),
                "完走時は末尾 SET cue が発火し BootCount=\"1\" が記録される: {loaded:?}"
            );

            assert!(
                carrier_names(&observed).contains(&PROP_SET_CUE_NAME.to_string()),
                "完走では末尾 SET cue が実際に発火・broadcast される: {:?}",
                carrier_names(&observed)
            );

            parts.publisher.close();
            parts.handle.join().expect("clean close joins");
        }

        /// horizon 前に `Close` すると、残 cue（SET）も捨てられ、何も書かれない（次回も初回扱い）。
        /// inbox は FIFO で単一スレッド処理なので、Tick は Close より前に処理される。
        #[test]
        fn interrupt_before_horizon_drops_set_cue_and_records_nothing() {
            let (parts, roots, shared) = spawn_with_shared_io();
            let prop_sink = PropSetCueSink::new(parts.publisher.clone());
            let (observer, observed) = ObservingSink::new();

            let (done_tx, done_rx) = mpsc::channel::<TalkNotice>();
            let sinks: Vec<Box<dyn CueSink + Send>> = vec![Box::new(prop_sink), Box::new(observer)];
            let handle = spawn_talk(
                first_boot_start(),
                done_tx,
                sinks,
                SystemVarSnapshot::default(),
                None,
            );

            handle.inbox.send(SakuraMsg::Tick(0.0)).unwrap();
            handle.inbox.send(SakuraMsg::Close).unwrap();

            let TalkNotice::Done(done) = done_rx
                .recv_timeout(Duration::from_secs(5))
                .expect("Close で中断 TalkDone")
            else {
                panic!("選択肢を含まない台本では ChoiceWaiting は流れない（TalkDone のみ）");
            };
            assert_eq!(
                done.reason,
                TalkEndReason::Interrupted,
                "horizon 前 Close は Interrupted"
            );
            handle.actor.join().expect("body は正常終了する");

            // 書込があれば barrier で確定させてから、ストアが不変なことを見る。
            parts
                .publisher
                .barrier()
                .expect("barrier while actor alive");
            let loaded = load_scope(PersistScope::Ghost, &roots, &shared);
            assert!(
                loaded.is_empty(),
                "horizon 前中断では SET cue が発火せずストアは不変（次回も初回扱い）: {loaded:?}"
            );

            assert!(
                !carrier_names(&observed).contains(&PROP_SET_CUE_NAME.to_string()),
                "中断では上流が player を捨て、末尾 SET cue も一緒に捨てられて発火しない: {:?}",
                carrier_names(&observed)
            );

            parts.publisher.close();
            parts.handle.join().expect("clean close joins");
        }
    }
}
