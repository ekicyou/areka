//! 統合テストの共有の道具（`cue_sequence`・`regression`・`balloon_face_e2e`・`bind_e2e` が `mod common;` で使う）。
//!
//! 各テストは別のクレートとして組まれ、使う道具がテストごとに違うので、使わない道具の警告を許す。
#![allow(dead_code)]

use areka_emo_compose::EmoWorld;
use areka_sakura::{ActorKey, ChoiceWaiting, CueCommand, CueSink, TalkCue, TalkDone};
use areka_seriko::SurfaceResolver;

/// テスト用の Shell 系 `TalkCue`（`Emote{key}`・at/actor 込み）を組む。
pub fn emote_cue(at: f64, scope: &str, key: &str) -> TalkCue {
    TalkCue {
        at,
        actor: ActorKey::from(scope),
        command: CueCommand::Emote { key: key.into() },
        duration: 0.0, // 表情切替は瞬時（明示的 0）。
    }
}

/// emo2 検体の `shell/master/surfaces.txt`（検体の窓口から得る）。
///
/// 窓口が返すのは借用なので、`SampleRoot` を **プロセス寿命で保持**する。段 ③ で `Drop` が
/// 展開した複製を消すため、関数内の一時値にすると借用の元がその場で消える。
pub fn emo2_surfaces_txt() -> std::path::PathBuf {
    use std::sync::LazyLock;

    use sample_ghost_kit::SampleRoot;

    static EMO2: LazyLock<SampleRoot> =
        LazyLock::new(|| SampleRoot::acquire("emo2").expect("emo2 は登記済みの検体"));

    EMO2.folder()
        .join("shell")
        .join("master")
        .join("surfaces.txt")
}

/// 実 emo2 shell fixture の surfaces.txt を parse し、alias スナップショットから解決層を組む。
///
/// 検体の在処は窓口から得るので、テストの CWD に依存しない（絶対パス）。
/// emo-compose の world.rs テストと同一 fixture・同一経路（parse→`EmoWorld::build`→
/// `alias_snapshot`）で、要件 7.3 の「alias 実データ追験」を満たす。
pub fn emo2_resolver() -> SurfaceResolver {
    let path = emo2_surfaces_txt();
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("emo2 surfaces.txt を読めること: {}: {e}", path.display()));
    let shell = areka_parsers::shell::parse(&content);
    let world = EmoWorld::build(&shell);
    let snapshot = world.alias_snapshot();
    // 非自明性: emo2 は alias を含む（空マップの vacuous pass を防ぐ）。
    assert!(!snapshot.is_empty(), "emo2 fixture は alias を含む");
    SurfaceResolver::new(snapshot)
}

/// `spawn_talk` の done ポート境界（`D: From<TalkDone> + From<ChoiceWaiting>`）を満たす
/// test-local な合流 enum。両通知は同一ポートを流れる（DD-6）が、本 E2E の fixture に選択肢
/// （`\q`）は無く `ChoiceWaiting` は発生しない——境界充足のためだけの受け皿である。
pub enum TalkNotice {
    Done(TalkDone),
    /// 本 e2e は選択待ち通知を観測しないが、`spawn_talk` の `D` 境界
    /// （`From<TalkDone> + From<ChoiceWaiting>`）を満たすため受け口だけ用意する。
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

/// text 系（→emo text-layer⑥）に相当する broadcast の 2 つ目のスロットを破棄する test-local sink。
///
/// 表示系（seriko）が発行する表示指令だけを観測するテストで、text 側へ届く全 cue を握って捨てる（記録しない）。
pub struct NullTextSink;

impl CueSink for NullTextSink {
    fn emit(&mut self, _cue: TalkCue) {
        // 破棄のみ。
    }
}
