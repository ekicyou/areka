//! 統合テストの共有の道具（`regression`・`balloon_face_e2e`・`bind_e2e`・`loop_integration` が `mod common;` で使う）。
//!
//! 各テストは別のクレートとして組まれ、使う道具がテストごとに違うので、使わない道具の警告を許す。
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::time::Duration;

use areka_emo_compose::{BindSet, EmoWorld};
use areka_sakura::{
    ActorKey, ChoiceWaiting, CueCommand, CueSink, SakuraMsg, ScriptOrigin, StartTalk,
    SystemVarSnapshot, TalkCue, TalkDone, TalkEndReason, TalkId, spawn_talk,
};
use areka_seriko::{
    BindResolver, DefaultBinds, DisplayCommand, MockSurfaceOutput, SerikoLoopConfig, SerikoSink,
    SurfaceResolver, build_static_bindset, spawn_seriko,
};

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
fn emo2_surfaces_txt() -> std::path::PathBuf {
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

/// emo2 の bindgroup default の既定でオンの集合。
pub fn emo2_binds() -> BindSet {
    build_static_bindset(&[1100, 1207, 1302, 1500, 1800])
}

/// 役者を起こし、`drive` で合図や刻みを送り、`close()`→`join()` で止めて、出た表示の指令を送った順に返す。
///
/// 同期は `close()`→`join()` の 1 点だけ（sleep・polling なし）。`join` が戻った時点で、先に送った
/// 合図と刻みは単一スレッドの FIFO で処理済みなので、記録をそのまま照合できる。
pub fn run_actor(
    resolver: SurfaceResolver,
    binds: BindSet,
    loop_config: SerikoLoopConfig,
    drive: impl FnOnce(&mut SerikoSink),
) -> Vec<DisplayCommand> {
    let out = MockSurfaceOutput::new();
    let records = out.records();
    let (mut sink, handle) = spawn_seriko(
        resolver,
        DefaultBinds::uniform(binds),
        BindResolver::empty(),
        loop_config,
        out,
    );
    drive(&mut sink);
    sink.close().expect("Close を送れること");
    handle.join().expect("Close で正常終了する");
    records.lock().expect("records mutex poisoned").clone()
}

/// ループを止めた役者へ `cues` を順に送り、出た表示の指令を返す（[`run_actor`] の合図だけの形）。
pub fn run_cues(
    resolver: SurfaceResolver,
    binds: BindSet,
    cues: impl IntoIterator<Item = TalkCue>,
) -> Vec<DisplayCommand> {
    run_actor(resolver, binds, SerikoLoopConfig::disabled(), |sink| {
        for cue in cues {
            CueSink::emit(sink, cue);
        }
    })
}

/// さくらスクリプトを直に入れた talk と seriko を起こし、`ticks` を順に入れて、出た表示の指令を送った順に返す。
///
/// seriko には空の別名の表（数値の鍵は数値の枝で解ける）・呼び手の静的な着せ替え `static_binds`・
/// 名前の表 `resolver` を渡す。broadcast の 2 つ目のスロットには呼び手の `text_sink` を差し込む。
///
/// 同期（sleep・polling なし）: 刻みを入れる → `TalkDone` を受ける（`\e` で自然に終わる）→ talk の
/// スレッドが終わり、move 済みの `SerikoSink`（seriko の inbox の唯一の `Sender`）が drop されて inbox が
/// 切れる → seriko を join する（唯一の同期点。先に送った `Cue` は FIFO の単一スレッドなので処理済み）
/// → 記録を写す。シナリオごとに新しい状態を立て、持ち越さない。
pub fn run_core(
    static_binds: BindSet,
    resolver: BindResolver,
    script: &str,
    ticks: &[f64],
    text_sink: Box<dyn CueSink + Send>,
) -> Vec<DisplayCommand> {
    let out = MockSurfaceOutput::new();
    let records = out.records();
    let (seriko_sink, seriko) = spawn_seriko(
        SurfaceResolver::new(BTreeMap::new()),
        DefaultBinds::uniform(static_binds),
        resolver,
        SerikoLoopConfig::disabled(),
        out,
    );

    // seriko_sink を clone せずに move する。talk のスレッドが終わったときの drop が seriko の inbox を切る。
    let (done_tx, done_rx) = std::sync::mpsc::channel::<TalkNotice>();
    let talk = spawn_talk(
        StartTalk {
            epilogue: Vec::new(),
            script: script.to_string(),
            talk_id: TalkId(8),
            origin: ScriptOrigin::GHOST,
            strict: false,
        },
        done_tx,
        vec![Box::new(seriko_sink), text_sink],
        SystemVarSnapshot::default(),
        None,
    );

    for &t in ticks {
        talk.inbox
            .send(SakuraMsg::Tick(t))
            .expect("Tick を投函できること");
    }

    // fixture は全て `\e` で終わるので Ended。
    let TalkNotice::Done(done) = done_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("fixture script は `\\e` で自然終端し TalkDone を返すべき")
    else {
        panic!("選択肢を含まない fixture では ChoiceWaiting は流れない（TalkDone のみ）");
    };
    assert_eq!(
        done.reason,
        TalkEndReason::Ended,
        "`\\e` は Ended で終端する"
    );

    talk.actor
        .join()
        .expect("talk body は Break 後に正常終了する");
    seriko.join().expect("seriko は disconnect で正常終了する");

    records.lock().expect("records mutex poisoned").clone()
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

/// シェルの descript.txt `shell` を持つゴーストを作業フォルダ（`target\\` の下）に組み、着せ替えの読みを返す
/// （クレートの中の `bind::tests::groups_of` と同じ組み方。統合テストからはそちらが見えない）。
pub fn groups_of(shell: &str) -> areka_parsers::package::BindGroupDefaults {
    let work = sample_ghost_kit::WorkDir::new().expect("作業フォルダを取れる");
    let master = work.path().join("ghost").join("master");
    std::fs::create_dir_all(&master).expect("ghost/master を作る");
    std::fs::write(master.join("descript.txt"), "charset,UTF-8\r\nname,t\r\n")
        .expect("ゴーストの descript.txt を書く");
    let shell_dir = work.path().join("shell").join("master");
    std::fs::create_dir_all(&shell_dir).expect("shell/master を作る");
    std::fs::write(shell_dir.join("descript.txt"), shell).expect("シェルの descript.txt を書く");
    areka_parsers::package::resolve_with_shell(
        work.path(),
        areka_parsers::charset::DefaultEncoding::Utf8,
        None,
    )
    .expect("検体のシェルは解決できる")
    .bindgroups
}
