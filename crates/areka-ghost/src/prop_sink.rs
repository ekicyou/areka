//! `areka.prop.set`（汎用プロパティ SET キュー）の名前自己選別消費 sink（design C7・R3.4/6.2/7.1）。
//!
//! 統一プロパティシステム（sylphya）の**書込側対応物**——broadcast された全 cue のうち、
//! キャリア正準形（`Custom` の String Array）であり自らのコマンド名リテラル
//! [`PROP_SET_CUE_NAME`] を運ぶものだけを名前自己選別し、**カウンタ key 族限定**の書込 key 統制を
//! 通したうえで `SylphyaPublisher::persist_put`（fire-and-forget・talk スレッドをブロックしない）へ
//! 落とす（design C7 Event Contract）。
//!
//! 消費先例は `MoveCueSink`（emo2_boot/move_cue.rs・名前自己選別・非担当は記録付き良性スキップ）と
//! `make_username_resource_sink`（sylphya_wiring.rs・publisher 捕獲 sink）——本 sink は両者の合成同型。

use areka_sakura::contract::{CueCommand, CueSink, TalkCue};
use areka_sylphya::{PersistKey, PersistScope, SylphyaPublisher};
use tracing::{debug, info, warn};

/// 汎用プロパティ SET キューのコマンド名リテラル（design C7・`areka.` 接頭辞＝baseware 内部キュー
/// 名前空間ゆえ ukadoc 正典コマンド名（裸単語）と構造的に非衝突）。
///
/// 増分語彙 `areka.prop.inc` は名前のみ将来予約であり M1 は不実装（本 sink は assignment のみ）。
pub const PROP_SET_CUE_NAME: &str = "areka.prop.set";

/// `areka.prop.set` キューの talk スレッド側消費 sink（design C7「Event Contract」・R3.4）。
///
/// broadcast された全 cue のうち、キャリア正準形（`Custom` の String Array）であり自らのコマンド名
/// リテラル [`PROP_SET_CUE_NAME`] を運ぶものだけを名前自己選別し、**カウンタ key 族限定**の書込 key
/// 統制（[`PersistKey::BootCount`]／[`PersistKey::VanishCount`] のみ受理）を通したうえで
/// `SylphyaPublisher::persist_put`（fire-and-forget）へ落とす。それ以外（非キャリア・担当外
/// コマンド名・key 統制外・引数不足）は**記録付き良性スキップ**へ縮退する——無音破棄でも panic でも
/// ない（R6.2・log-first）。
///
/// # 書込 key 統制（design C7 step 3・research §4-8 (ii) 決着）
///
/// 位置 key（`WindowPos`／`BalloonOffset`）は cue 側から**書けない**（warn スキップ）——位置の永続
/// ライターをユーザードラッグ 2 点に限る 1.9 の単一ライター規律を cue 側から侵食させない。受理は
/// カウンタ key 族（`BootCount`／`VanishCount`）のみ。
///
/// # Clone + Send（boot 型境界）
///
/// dispatcher は talk ごとに sink を clone する（`GhostBootOptions.sinks` は
/// `CueSink + Clone + Send + 'static`＝[`crate::sink::BootCueSink`]）。内側 [`SylphyaPublisher`] は
/// `Clone + Send` ゆえ `#[derive(Clone)]` がそのまま成立し、全 clone は同一アクター送信端＝配送意味は
/// 同一（掲示板モデルの単一鏡像へ収斂）。
#[derive(Clone)]
pub struct PropSetCueSink {
    /// sylphya アクターへの投函端（Clone 可・全 clone は同一鏡像へ収斂）。
    publisher: SylphyaPublisher,
}

impl PropSetCueSink {
    /// sylphya publisher（`boot()` が保持する送信端・task 7.1 が生成）から sink を構築する。
    pub fn new(publisher: SylphyaPublisher) -> Self {
        Self { publisher }
    }
}

/// 演者非依存の単一出力契約 [`areka_sakura::contract::CueSink`]（＝`dola::cue::CueSink`）を実装する
/// （`GhostBootOptions.sinks` の broadcast 登録先が要求する形・task 7.1 が登録）。broadcast 下では
/// 担当外 cue（`Text`／`Emote`／他コマンド名のキャリア等）も本 sink へ届くが、名前選別で
/// [`PROP_SET_CUE_NAME`] 以外は記録付き良性スキップへ縮退する（duration honor には触れない・R6.2）。
impl CueSink for PropSetCueSink {
    fn emit(&mut self, cue: TalkCue) {
        // 1) キャリア抽出。非キャリア（Text/Emote 等の担当外 broadcast）は良性スキップ。
        //    Custom なのに非正準 params のときの severity は**宛名規律**で分ける: 宛名
        //    （`Custom{command}` フィールド・params 非正準でも読める）が自分宛の壊れ物ゆえ warn・
        //    他人宛/未知名＝担当外ゆえ debug（報告責任は宛名の担当者）。
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

        // 2) 名前自己選別（消費者自身のコマンド名リテラル）。PROP_SET_CUE_NAME のみ解釈する
        //    （duration honor には触れない）。dola は名前語彙を持たず、一意性は名前空間接頭辞規約
        //    ＋本 sink 檻（名前リテラル固定）が担保する（W4 中は台帳登記後送・design）。
        if name != PROP_SET_CUE_NAME {
            debug!(
                name,
                "PropSetCueSink: 担当外コマンド名を良性スキップ（名前自己選別）"
            );
            return;
        }

        // 3) 引数不足（tokens < 2）は warn スキップ（assignment 形＝[key, value] を要する）。
        if tokens.len() < 2 {
            warn!(
                token_count = tokens.len(),
                "PropSetCueSink: 引数不足（[key, 値] を要する）のため {PROP_SET_CUE_NAME} をスキップ"
            );
            return;
        }
        let key_token = tokens[0];
        let value = tokens[1];

        // 4) 書込 key 統制（design C7 step 3）: カウンタ key 族（BootCount／VanishCount）のみ受理。
        //    位置 key（WindowPos／BalloonOffset）・未知 key は warn スキップ——位置の永続ライターを
        //    ユーザードラッグ 2 点に限る 1.9 の単一ライター規律を cue 側から侵食させない。
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

        // 5) 受理: Ghost スコープへ fire-and-forget で write-through（talk スレッドをブロックしない）。
        //    実機 grep 証跡の info を 1 本出す（design Monitoring: `prop_set_cue applied`）。
        self.publisher
            .persist_put(PersistScope::Ghost, vec![(key, value.to_string())]);
        info!(
            key = key_token,
            value,
            "prop_set_cue applied"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use areka_sakura::contract::{ActorKey, CueCommand, CueSink, TalkCue};
    use areka_sylphya::persist::{FakePersistIo, PersistIo};
    use areka_sylphya::{
        load_scope, spawn_sylphya, PersistKey, PersistScope, ScopeRoots, SylphyaInit,
    };
    use std::path::PathBuf;
    use std::sync::Arc;

    /// `\![name,tokens...]` 汎用キャリア cue を組む（正準形＝`Custom` の String Array）。
    fn carrier_cue(actor: &str, name: &str, tokens: &[&str]) -> TalkCue {
        let toks: Vec<String> = tokens.iter().map(|s| s.to_string()).collect();
        TalkCue {
            at: 0.0,
            actor: ActorKey::from(actor),
            command: CueCommand::command_carrier(name, toks),
            duration: 0.0,
        }
    }

    /// 同一 [`FakePersistIo`] を `Arc` 共有する委譲 IO（テスト専用・actor.rs 先例の再実装）。
    ///
    /// `FakePersistIo` は内部 `Mutex` で Clone 不可ゆえ、write-through が実 IO シームを通ったことは
    /// 「アクターへ渡した IO と同じ store」を別ハンドルの [`load_scope`] で観測して確認する。
    #[derive(Clone)]
    struct SharedFakeIo(Arc<FakePersistIo>);
    impl PersistIo for SharedFakeIo {
        fn read(&self, path: &std::path::Path) -> std::io::Result<Option<String>> {
            self.0.read(path)
        }
        fn commit(&self, path: &std::path::Path, content: &str) -> std::io::Result<()> {
            self.0.commit(path, content)
        }
    }

    /// ghost root 付きの共有 FakeIo sylphya を起動する（write-through を別ハンドルで観測するため）。
    fn spawn_with_shared_io() -> (
        areka_sylphya::SylphyaParts,
        ScopeRoots,
        SharedFakeIo,
    ) {
        let shared = SharedFakeIo(Arc::new(FakePersistIo::new()));
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

    /// 受理: `areka.prop.set` + カウンタ key（`areka.boot.count`,"1"）は Ghost スコープへ
    /// write-through され、別ハンドルの `load_scope` が `BootCount="1"` を観測する（design Testing §6）。
    #[test]
    fn boot_count_carrier_is_persisted() {
        let (parts, roots, shared) = spawn_with_shared_io();
        let mut sink = PropSetCueSink::new(parts.publisher.clone());

        sink.emit(carrier_cue("0", PROP_SET_CUE_NAME, &["areka.boot.count", "1"]));
        parts.publisher.barrier().expect("barrier while actor alive");

        let loaded = load_scope(PersistScope::Ghost, &roots, &shared);
        assert!(
            loaded.contains(&(PersistKey::BootCount, "1".into())),
            "areka.prop.set + areka.boot.count は BootCount=\"1\" として永続される: {loaded:?}"
        );

        parts.publisher.close();
        parts.handle.join().expect("clean close joins");
    }

    /// 受理: `areka.vanish.count` もカウンタ key 族ゆえ受理される（VanishCount）。
    #[test]
    fn vanish_count_carrier_is_persisted() {
        let (parts, roots, shared) = spawn_with_shared_io();
        let mut sink = PropSetCueSink::new(parts.publisher.clone());

        sink.emit(carrier_cue("0", PROP_SET_CUE_NAME, &["areka.vanish.count", "7"]));
        parts.publisher.barrier().expect("barrier");

        let loaded = load_scope(PersistScope::Ghost, &roots, &shared);
        assert!(
            loaded.contains(&(PersistKey::VanishCount, "7".into())),
            "areka.vanish.count はカウンタ key 族ゆえ受理される: {loaded:?}"
        );

        parts.publisher.close();
        parts.handle.join().expect("clean close joins");
    }

    /// 書込 key 統制: 位置 key（`areka.window.scope(0).x`）は**拒否**＝warn スキップ・何も書かない
    /// （1.9 単一ライター規律を cue 側から侵食させない・design C7 step 3）。
    #[test]
    fn window_pos_key_is_rejected_no_write() {
        let (parts, roots, shared) = spawn_with_shared_io();
        let mut sink = PropSetCueSink::new(parts.publisher.clone());

        // 位置 key の正準文字列（2 トークンで引数不足ゲートは通過し、key 統制で弾かれる）。
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

    /// 名前自己選別: 担当外コマンド名（`move`/`bind`/未知名）は debug 良性スキップ＝何も書かない。
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

    /// 引数不足（tokens < 2）は warn スキップ＝何も書かない。
    #[test]
    fn insufficient_args_is_skipped_no_write() {
        let (parts, roots, shared) = spawn_with_shared_io();
        let mut sink = PropSetCueSink::new(parts.publisher.clone());

        // key のみ（値トークン欠落）＝引数不足。
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

    /// 非キャリア cue（`Text` 等の担当外 broadcast）も良性スキップ＝何も書かない。
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

    /// `PropSetCueSink` は `BootCueSink`（`CueSink + Clone + Send + 'static`）境界を充足する
    /// （task 7.1 が `Vec<Box<dyn BootCueSink>>` へ登録できることの compile 檻）。
    #[test]
    fn satisfies_boot_sink_bounds() {
        fn require_boot_sink<T: crate::sink::BootCueSink>() {}
        require_boot_sink::<PropSetCueSink>();
    }
}
