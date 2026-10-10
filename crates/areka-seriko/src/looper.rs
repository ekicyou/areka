//! looper: SERIKO ループ・ランタイムの統括層（二層時間＝毎秒抽選×サブ秒進行）。
//!
//! [`LoopRuntime`] はアクター本体が単独所有し（スレッド内・ロック不要）、per-(scope, slot) の
//! 再生状態（`SlotPlayback`）と 1 個の [`LotteryBoundary`] を保持する。1 tick ごとに
//! [`LoopRuntime::on_tick`] が (1) 単調性ガード → (2) 境界跨ぎ時のみ抽選 → (3) 全再生中アニメの
//! 進行（[`frame_at`]）→ (4) `commit_pattern` への差分反映、を統括し、発行すべき
//! [`DisplayCommand`] 列を返す（発行自体は actor の `emit_display` 単一点が行う・要件 6.3）。
//!
//! # 状態機械（per (scope, slot, animation)・design 状態図）
//!
//! `Idle`/`IdleResidual`（＝playback エントリなし）は抽選対象（2.3/9.4）。境界抽選で fire すると
//! `Playing`（playback エントリあり）へ入り、[`frame_at`] の進行で `Active`/`FinishedResidual`/
//! `Stopped` を辿る。`Stopped`（`-1` 等の負 surface）はコマ除去＋playback 除去でベース復帰（4.3）、
//! `FinishedResidual`（`-1` なし末尾）は最終コマを残したまま playback のみ除去＝`IdleResidual`（4.4）で
//! 再抽選対象へ戻る。**再発火の瞬間、残留コマは即時クリアされ**、以降の表示は [`frame_at`] の結果
//! のみで決まる（討議 #2 裁定：表示を直前 PatternState に依存させない・`Pending` 中はエントリなし＝
//! ベース露出）。surface 切替／Hide は当該 slot の playback を全除去する（PatternState クリアは
//! ScopeStates 側 apply の責務）。**bind 種（`BindRandom`）の再生中 ID が bind 集合から外れたときは、
//! 進行相で停止相当（コマ除去＋playback 除去）へ落とす**（bindopt 7.3・D9-2）。発火ゲートは発火を
//! 止めるだけで進行を止めないため、これが無いと「再生中に外れた」ID のコマが次 tick で復活する。
//!
//! # 抽選の固定消費順（D-7）
//!
//! scope 昇順（`ActorKey` 辞書順）→ Shell → Balloon → animation id 昇順。注入乱数列の消費順が
//! 一意に定まり、決定論テストが期待値を焼き込める。**bind ゲート不通過（`BindRandom` でその
//! bindgroup が OFF）のアニメには [`should_fire`] を呼ばない＝乱数を消費しない**（要件 3.1）。
//!
//! bind の書込 API（`apply_bind` 等）は一切呼ばない（read-only 参照のみ・要件 3.3）。

use std::collections::{BTreeMap, HashMap, HashSet};

use areka_emo_compose::{BindSet, PatternFrame, PatternState};
use areka_sakura::{ActorKey, CueCommand, TalkCue};

use crate::actor::SerikoClock;
use crate::output::DisplayCommand;
use crate::parts::PartClocks;
use crate::state::{PatternApplyOutcome, ScopeStates, Slot};
use crate::table::{AnimationTable, LoopAnimation, LoopFrame, LoopTrigger};
use crate::talk::{TalkEpoch, TalkFeed};
use crate::timeline::{
    AlwaysView, FrameStatus, LoopRng, LotteryBoundary, always_at, current_frame_index, frame_at,
    seeded_rng, should_fire,
};
use crate::trigger::{Armed, TalkWindow};

// 一番上の面の引き金の配線（構える・判定して始める・文字の列と数えの世話）。
#[path = "looper_trigger.rs"]
mod top_trigger;
use top_trigger::{
    arm_slot, fire_top_triggers, glyph_wall_ms, restart_talk, revealed_at, settle_talk,
};

/// SERIKO ループ構成（シェル表 1 面＋scope 別バルーン表＋乱数注入シーム）。boot 時に組み立てて
/// [`LoopRuntime`] へ値渡しする。
///
/// `shell_table`／`balloon_tables` は **surface ID 名前空間の別**であり能力の仕切りではない
/// （面種非依存・裁定 (a)）。emo2 は `balloon_tables` が全 scope 空（データ事実）。`rng` は
/// コンストラクタ注入で、評価経路に実 entropy への直接依存を持たない（要件 7.1）。
///
/// バルーン表だけが scope キーの写像である理由: シェル面は全 scope が同一 `Shell` から build される
/// ゆえ表の内容が scope 非依存（単数で足りる）。対してバルーン面は scope ごとに解決される系列
/// （`balloons*`／`balloonk*` 等）が異なるため、ある scope のバルーンが別 scope の系列由来の定義で
/// 駆動されないことを型で禁じる（要件 5.6）。
pub struct SerikoLoopConfig {
    /// シェル表示エントリ用のアニメ表（surface ID 名前空間: shell・全 scope 共通）。
    pub shell_table: AnimationTable,
    /// バルーン表示エントリ用の scope 別アニメ表（surface ID 名前空間: balloon・要件 5.6）。
    ///
    /// キーは `ActorKey`（boot 側の `u32` scope は転送時に `ActorKey::from(scope.to_string())` で
    /// 変換する＝attach／再追従と同一の既存写像語彙）。**不在 scope は空表意味論**（抽選対象ゼロ・
    /// 乱数非消費・panic なし）。emo2 は全 scope 空。
    pub balloon_tables: BTreeMap<ActorKey, AnimationTable>,
    /// 1/N 抽選の乱数注入シーム（本番は `seeded_rng(seed)`・テストは注入列）。
    pub rng: LoopRng,
}

impl SerikoLoopConfig {
    /// 空表＋ダミー乱数（ループ完全不活性）。既存テスト・非 emo2 経路の非退行用。
    ///
    /// シェル表が空・バルーン表の写像も空ゆえ抽選対象アニメが常にゼロ＝[`should_fire`] は決して
    /// 呼ばれず乱数は消費されない（ダミー種は観測に現れない）。`on_tick` は常に空を返す（非退行）。
    pub fn disabled() -> Self {
        Self {
            shell_table: AnimationTable::empty(),
            balloon_tables: BTreeMap::new(),
            rng: seeded_rng(0),
        }
    }
}

/// 1 本の再生中アニメの再生状態（開始絶対時刻のみ・経過は tick の `now_ms` との差で算出）。
#[derive(Debug, Clone, Copy)]
struct Playback {
    /// 再生開始絶対時刻（ms）。`frame_at` へ渡す経過＝`now_ms - started_at_ms`。
    started_at_ms: u64,
}

/// per-slot の再生中アニメ表（animation id → [`Playback`]）。エントリを持つ id が「再生中」＝抽選対象外。
type SlotPlayback = HashMap<u32, Playback>;

/// 二層時間の統括と per-(scope, slot) 再生状態の所有者（アクター本体が単独所有・要件 1.2/2.x/3.x/6.x）。
pub(crate) struct LoopRuntime {
    /// 表 2 面＋乱数注入シーム（spawn 時注入・表は差し替えの語 [`LoopRuntime::replace_shell_table`]／
    /// [`LoopRuntime::replace_balloon_tables`] でだけ替わる＋可変 rng 状態）。
    config: SerikoLoopConfig,
    /// 1000ms 絶対グリッド境界（毎秒抽選の写像・catch-up 1 回）。最初に観測した tick で遅延初期化する
    /// （`starting_at(now)` は now より厳密未来の次境界を起点にするため、起動直後 tick では発火しない）。
    boundary: Option<LotteryBoundary>,
    /// per-(scope, slot) の再生中アニメ表。エントリの有無が Idle/Playing を分ける。
    playback: HashMap<(ActorKey, Slot), SlotPlayback>,
    /// 直前 tick の `now_ms`（単調性ガード用・非単調 tick は無視する）。
    last_seen: Option<u64>,
    /// `-1` 以外の負 surface に対する warn! を (scope, slot, anim id) ごとに 1 回だけ発火するための記録
    /// （初回のみ warn!・要件 8.2）。
    warned_negative: HashSet<(ActorKey, Slot, u32)>,
    /// 部品の時計（spec: areka-P0-surface-element-nesting・シェルとバルーンの面）。面の表が
    /// [`AnimationTable::has_animated_parts`] で偽なら 1 度も触らない（要件 7.2・7.3）。
    parts: PartClocks,
    /// 刻みと同じ時計（spec: areka-P0-animated-image-playback 要件 1.6・3.1）。台本の合図を処理する
    /// ときに読む（[`LoopRuntime::event_ms`]）。無ければ出来事の時刻は直前の刻みの時刻。
    clock: Option<SerikoClock>,
    /// 一番上の面の引き金の状態（spec: areka-P0-seriko-trigger-intervals 要件 2・3）。面に入ったときに
    /// 生まれ、面が替わる・表を差し替えると消える。構える条件（一番上に `runonce`・`periodic`・`talk` が
    /// 在る、または表に `talk` が在る）を満たさない面では生まれない（要件 8.1）。
    armed: HashMap<(ActorKey, Slot), Armed>,
    /// 台本の 0 秒に当たる時計の読みの見積もり（spec: areka-P0-seriko-trigger-intervals 要件 6.4）。
    /// 表に `talk` が無い間は cue を写さないので空のまま。
    epoch: TalkEpoch,
    /// スコープごとの、文字が現れる時刻の列（要件 4.3・4.8）。表に `talk` が無い間は空のまま。
    feeds: HashMap<ActorKey, TalkFeed>,
    /// シェルかバルーンのどれかの表に `talk` が在るか（文字の cue を写すかの門・要件 8.1）。表を
    /// 差し替えるたびに組み直す。
    has_talk: bool,
}

/// 固定消費順のための slot ランク（Shell を Balloon より前に置く・D-7）。
fn slot_rank(slot: Slot) -> u8 {
    match slot {
        Slot::Shell => 0,
        Slot::Balloon => 1,
    }
}

/// 表のコマ 1 枚を合成へ渡すコマにする（一番上と部品で共有）。
///
/// [`frame_at`] が `Active`/`FinishedResidual` を返したコマにだけ使う（その時点で `surface_id` は
/// 非負・負は `Stopped`）。
pub(crate) fn pattern_frame(f: &LoopFrame) -> PatternFrame {
    PatternFrame {
        surface_id: f.surface_id as u32,
        method: f.method.clone(),
        x: f.x,
        y: f.y,
    }
}

/// 一番上の `always` の経過 `elapsed_ms` の答えを欄へ置く: 経過 0 のコマと同じなら載せない／
/// コマならコマ／何も出さないなら「消えている」（spec: areka-P0-animated-image-playback 要件 4.1・
/// 4.2・4.5・7.4）。再生は末尾でも負の番号でも捨てない（周の頭へ戻って続く）。
fn put_top_always(pattern: &mut PatternState, anim: &LoopAnimation, elapsed_ms: u64) {
    let LoopTrigger::Always { period_ms, laps } = anim.trigger else {
        return;
    };
    let rest = always_at(&anim.frames, period_ms, laps, 0);
    match always_at(&anim.frames, period_ms, laps, elapsed_ms) {
        now if now == rest => pattern.remove(anim.id),
        AlwaysView::Frame(i) => pattern.set(anim.id, pattern_frame(&anim.frames[i])),
        AlwaysView::Nothing => pattern.set_blank(anim.id),
    }
}

/// 一番上 `surface_id` の `always` に再生が無ければ `at_ms` で作る（乱数を引かない・要件 4.1）。
fn start_top_always(
    playback: &mut HashMap<(ActorKey, Slot), SlotPlayback>,
    table: &AnimationTable,
    scope: &ActorKey,
    slot: Slot,
    surface_id: u32,
    at_ms: u64,
) {
    for anim in table.animations(surface_id) {
        if !matches!(anim.trigger, LoopTrigger::Always { .. }) {
            continue;
        }
        let slot_playback = playback.entry((scope.clone(), slot)).or_default();
        if slot_playback.contains_key(&anim.id) {
            continue;
        }
        slot_playback.insert(
            anim.id,
            Playback {
                started_at_ms: at_ms,
            },
        );
        tracing::debug!(
            scope = scope.as_str(),
            slot = ?slot,
            animation_id = anim.id,
            "seriko: loop 一番上の always の再生が生まれた"
        );
    }
}

/// `slot` の面の着せ替えの集合（シェルはスコープの今の集合・バルーンの面は空・design「LoopRuntime」）。
fn slot_binds<'a>(states: &'a ScopeStates, scope: &ActorKey, slot: Slot) -> &'a BindSet {
    static EMPTY: std::sync::OnceLock<BindSet> = std::sync::OnceLock::new();
    match slot {
        Slot::Shell => states.current_binds(scope),
        Slot::Balloon => EMPTY.get_or_init(BindSet::default),
    }
}

/// `runonce`・`periodic`・`talk` の引き金か（spec: areka-P0-seriko-trigger-intervals）。
fn is_trigger_word(anim: &&LoopAnimation) -> bool {
    matches!(
        anim.trigger,
        LoopTrigger::Runonce | LoopTrigger::Periodic { .. } | LoopTrigger::Talk { .. }
    )
}

/// 一番上 `surface_id` が（`always` を持つか, `runonce`・`periodic`・`talk` を持つか）。1 回の走査で
/// 両方を求める（`is_continuous()` が偽の表では走査せず両方偽・要件 7.2・spec:
/// areka-P0-seriko-trigger-intervals 要件 8.1）。
fn top_kinds(table: &AnimationTable, surface_id: u32) -> (bool, bool) {
    if !table.is_continuous() {
        return (false, false);
    }
    let (mut always, mut trigger) = (false, false);
    for anim in table.animations(surface_id) {
        always |= matches!(anim.trigger, LoopTrigger::Always { .. });
        trigger |= is_trigger_word(&anim);
    }
    (always, trigger)
}

/// シェルかバルーンのどれかの表に `talk` が在るか（文字の cue を写すかの門）。
fn any_talk(config: &SerikoLoopConfig) -> bool {
    config.shell_table.has_talk() || config.balloon_tables.values().any(AnimationTable::has_talk)
}

/// 再生の終わりの記録。`talk` の再生は文字の到着ごとに起きうるので `debug!`、ほかは `info!`
/// （文言は同じ・spec: areka-P0-seriko-trigger-intervals 要件 7.3）。部品の進行も同じ決まりで使う。
macro_rules! log_play_end {
    ($talk:expr, $($arg:tt)+) => {
        if $talk {
            tracing::debug!($($arg)+)
        } else {
            tracing::info!($($arg)+)
        }
    };
}
pub(crate) use log_play_end;

/// 一番上の再生 1 本（`always` 以外）の経過 `elapsed` のコマを欄へ置く。終えていれば（負の番号で
/// 止まった・末尾に着いた）再生を捨てて終わりを記録する。刻みの進行と、引き金が終えた再生を
/// 入れ替えるとき（[`fire_top_triggers`]）の両方がここを通る。
fn put_top_play(
    new_pattern: &mut PatternState,
    playback: &mut HashMap<(ActorKey, Slot), SlotPlayback>,
    warned_negative: &mut HashSet<(ActorKey, Slot, u32)>,
    key: &(ActorKey, Slot),
    anim: &LoopAnimation,
    elapsed: u64,
) {
    let (scope, slot, anim_id) = (&key.0, key.1, anim.id);
    let is_talk = matches!(anim.trigger, LoopTrigger::Talk { .. });
    match frame_at(&anim.frames, elapsed) {
        // 先頭デッドライン未到達＝ベース露出。再発火時はここで残留コマが即時クリアされる（討議 #2）。
        FrameStatus::Pending => {
            new_pattern.remove(anim_id);
        }
        // 現在コマ 1 枚を搬送（4.2）。Active は再生継続、FinishedResidual は残留のうえ playback 除去。
        FrameStatus::Active(i) | FrameStatus::FinishedResidual(i) => {
            new_pattern.set(anim_id, pattern_frame(&anim.frames[i]));
            // 末尾非負到達（FinishedResidual）＝もう「再生中」ではない → playback のみ除去
            // （コマは残す・IdleResidual へ・4.4/9.4）。Active は再生継続でここは通らない。
            let is_last = i == anim.frames.len() - 1;
            if is_last {
                if let Some(pb) = playback.get_mut(key) {
                    pb.remove(&anim_id);
                }
                log_play_end!(
                    is_talk,
                    scope = scope.as_str(),
                    slot = ?slot,
                    animation_id = anim_id,
                    "seriko: loop 末尾残留（最終コマ保持・再抽選対象へ・要件 4.4/9.4）"
                );
            }
        }
        // 負 surface（`-1` 等）→ コマ除去＋playback 除去でベース復帰（4.3）。
        FrameStatus::Stopped => {
            // `-1` は正典駆動、それ以外の負値は初回のみ warn!（自アニメ停止扱い・他アニメ停止は非駆動・8.2）。
            if let Some(i) = current_frame_index(&anim.frames, elapsed) {
                let sid_val = anim.frames[i].surface_id;
                if sid_val != -1 {
                    let wkey = (scope.clone(), slot, anim_id);
                    if warned_negative.insert(wkey) {
                        tracing::warn!(
                            scope = scope.as_str(),
                            slot = ?slot,
                            animation_id = anim_id,
                            surface_id = sid_val,
                            "seriko: loop `-1` 以外の負 surface（自アニメ停止扱い・他アニメ停止 `-2` は非駆動・要件 8.2）"
                        );
                    }
                }
            }
            new_pattern.remove(anim_id);
            if let Some(pb) = playback.get_mut(key) {
                pb.remove(&anim_id);
            }
            log_play_end!(
                is_talk,
                scope = scope.as_str(),
                slot = ?slot,
                animation_id = anim_id,
                "seriko: loop 停止（負 surface でベース復帰・要件 4.3）"
            );
        }
    }
}

impl LoopRuntime {
    /// ループ構成を受けて再生状態ゼロの統括器を構築する（表・rng は注入済み）。
    pub(crate) fn new(config: SerikoLoopConfig) -> Self {
        let has_talk = any_talk(&config);
        Self {
            config,
            boundary: None,
            playback: HashMap::new(),
            last_seen: None,
            warned_negative: HashSet::new(),
            parts: PartClocks::default(),
            clock: None,
            armed: HashMap::new(),
            epoch: TalkEpoch::default(),
            feeds: HashMap::new(),
            has_talk,
        }
    }

    /// 時計つきにする（[`crate::actor::spawn_seriko_clocked`] の起動とテストの注入の口）。
    pub(crate) fn with_clock(mut self, clock: Option<SerikoClock>) -> Self {
        self.clock = clock;
        self
    }

    /// 台本の合図を処理する今の時刻（時計が無ければ `None`＝[`LoopRuntime::refresh`] が直前の刻みの
    /// 時刻を使う・design「時計の開始の時刻」）。
    pub(crate) fn event_ms(&self) -> Option<u64> {
        self.clock.as_ref().map(|clock| clock())
    }

    /// 届いた cue を 1 件、文字の時刻の写しへ渡す（spec: areka-P0-seriko-trigger-intervals 要件 4.3・
    /// 4.8・4.10・6.4・8.1）。
    ///
    /// どの表にも `talk` が無ければ何もしない（真偽 1 つで戻る）。在れば、種類を問わず起点の見積もりを
    /// 更新し、文字と選択肢の文字をそのスコープの列へ積み、消去はそのスコープ（全消去は全スコープ）の
    /// 現れなかった文字を捨てる。改行・待ち・`\!` のコマンドは数えない。「今」は時計、無ければ直前の
    /// 刻みの時刻で、どちらも無ければ写さない（次の cue で起点ができる）。記録は出さない（文字ごとの
    /// 記録を増やさない・要件 7.3）。
    pub(crate) fn observe_cue(&mut self, cue: &TalkCue) {
        if !self.has_talk {
            return;
        }
        let Some(now_ms) = self.event_ms().or(self.last_seen) else {
            return;
        };
        self.epoch.observe(cue.at, now_ms);
        match &cue.command {
            CueCommand::Text(text) | CueCommand::Choice { text, .. } => self
                .feeds
                .entry(cue.actor.clone())
                .or_default()
                .push_text(cue.at, cue.duration, text),
            CueCommand::Clear => {
                if let Some(feed) = self.feeds.get_mut(&cue.actor) {
                    restart_talk(feed, &mut self.armed, &cue.actor, cue.at);
                }
            }
            CueCommand::ClearAll => {
                for (scope, feed) in &mut self.feeds {
                    restart_talk(feed, &mut self.armed, scope, cue.at);
                }
            }
            _ => {}
        }
    }

    /// 1 tick の統括。発行すべき指令列（通常 0〜2 件）を返す。発行自体は actor が行う（要件 6.3）。
    ///
    /// 手順: (1) 単調性ガード（`now < 前回` → `debug!`＋無視）。(2) 境界跨ぎ時のみ抽選（表示中×非再生中×
    /// bind ゲート通過のアニメへ固定消費順で [`should_fire`]・fire で playback 登録＋`info!`）。(3) 全再生中
    /// アニメへ、まず**進行相の bind 判定**（bind 種かつ現在の bind 集合に非所属なら停止相当＝コマ除去＋
    /// playback 除去・bindopt 7.3/D9-2）を行い、残ったものへ [`frame_at`] を評価して slot ごとの新
    /// [`PatternState`] を組む（`Pending`→エントリ除去＝再発火残留の即時クリア・`Active`/`FinishedResidual`
    /// →コマ搬送・`Stopped`→除去＋playback 除去・`FinishedResidual`→コマ残留のまま playback 除去）。
    /// (4) slot ごとに `commit_pattern` し `Changed` を集約。
    ///
    /// 部品（spec: areka-P0-surface-element-nesting 要件 5.1・7.2・7.3）: 面の表が
    /// [`AnimationTable::has_animated_parts`] で真のときだけ、(3) の後に [`PartClocks::advance`] で
    /// 部品の欄を作り直してから (4) を 1 回行う。部品の抽選は一番上の抽選（全スコープぶん）の後に、
    /// スコープの昇順 → 面の種類 → 繰り返しの回 → 部品の番号 → animation の番号の順で引く。一番上の
    /// 再生が無い slot は、一番上に `always` が在るか見える部品に動くものが在るときだけ通す。偽の表は
    /// 今までの経路のまま。
    ///
    /// 進行の対象は [`ScopeStates::stage_slots`]（spec: areka-P0-animated-image-playback 要件 6.1）:
    /// 抽選の輪は今までどおり `shown_slots`。バルーンの面は窓が閉じていても評価するが、閉じている間は
    /// 時計を作らない（一番上の `always` の再生も部品の時計も・回数つきは捨てる）。
    ///
    /// 引き金（spec: areka-P0-seriko-trigger-intervals 要件 2・3・5.7・8.1）: 構える条件（一番上に
    /// `runonce`・`periodic`・`talk` が在る、または表に `talk` が在る）を満たす面は、(3) の頭で引き金の
    /// 状態を持つ（無ければこの刻みの時刻で構える・窓の開け閉めを写す）。一番上に 3 語の在る面では
    /// 一番上の再生が無くても通し、`runonce`・`periodic` を乱数なしで判定して、始まるものを判定が返した
    /// 開始の時刻で `playback` に入れる（その後は抽選の再生と同じ進行）。条件を満たさない面には状態が
    /// 生まれない。
    ///
    /// `talk`（要件 4）: 表に `talk` が在り窓が開いている面は、刻み 1 回につき文字の窓（前の刻みまでに
    /// 数えた数・今現れている数・文字が現れた壁時刻の写し）を 1 つ作り、一番上の `talk` を同じ判定に
    /// 掛ける（開始の時刻は区切りの文字が現れた時刻）。数えを進めて文字の列を刈り込むのは、全部の面が
    /// 窓を読み終えた刻みの最後（[`settle_talk`]）。
    pub(crate) fn on_tick(&mut self, now_ms: u64, states: &mut ScopeStates) -> Vec<DisplayCommand> {
        // (1) 単調性ガード（防御・実クロックでは非発生）。非単調 tick は状態を変えず無発行。
        if let Some(last) = self.last_seen {
            if now_ms < last {
                tracing::debug!(
                    now_ms,
                    last_seen = last,
                    "seriko: loop 非単調 tick を無視（防御・実クロックでは非発生・要件 1.2）"
                );
                return Vec::new();
            }
        }
        self.last_seen = Some(now_ms);

        // 境界跨ぎ判定（最初の観測 tick で遅延初期化＝起動直後は発火しない）。boundary の借用はここで閉じる。
        let crossed = {
            let boundary = self
                .boundary
                .get_or_insert_with(|| LotteryBoundary::starting_at(now_ms));
            boundary.poll(now_ms)
        };

        // 以降は config（表・rng）／playback／warned_negative を独立フィールドとして分離借用する
        // （表の不変参照と rng の可変参照が同一 config 借用で衝突しないようにする）。
        let LoopRuntime {
            config,
            playback,
            warned_negative,
            parts,
            armed,
            epoch,
            feeds,
            ..
        } = self;
        let SerikoLoopConfig {
            shell_table,
            balloon_tables,
            rng,
        } = config;
        // 不在 scope へ貸す空表（抽選対象ゼロ・乱数非消費・panic なし＝`disabled()` と同じ不活性・
        // 要件 5.6）。`BTreeMap::new()` は確保を伴わないため tick ごとの構築コストは無い。
        let empty_balloon_table = AnimationTable::empty();
        // 表示中 slot を列挙し、固定消費順（scope 昇順→Shell→Balloon）へ整列する（D-7）。
        let mut shown = states.shown_slots();
        shown.sort_by(|a, b| {
            a.0.cmp(&b.0)
                .then_with(|| slot_rank(a.1).cmp(&slot_rank(b.1)))
        });

        // (2) 抽選（境界跨ぎ時のみ）: 表示中×非再生中×bind ゲート通過のアニメへ固定消費順で should_fire。
        if crossed {
            for (scope, slot, sid) in &shown {
                let table: &AnimationTable = match slot {
                    Slot::Shell => &*shell_table,
                    // scope キー表引き（要件 5.6）。不在 scope は空表＝抽選対象ゼロ・乱数非消費。
                    Slot::Balloon => balloon_tables.get(scope).unwrap_or(&empty_balloon_table),
                };
                // animation id 昇順で消費（固定順・D-7）。
                let mut anims: Vec<&crate::table::LoopAnimation> =
                    table.animations(*sid).iter().collect();
                anims.sort_by_key(|a| a.id);

                for anim in anims {
                    let key = (scope.clone(), *slot);
                    // (a) 非再生中のみ対象（再生中は再抽選しない・乱数も消費しない・要件 2.3）。
                    let is_playing = playback
                        .get(&key)
                        .is_some_and(|pb| pb.contains_key(&anim.id));
                    if is_playing {
                        continue;
                    }
                    // (b) bind ゲート: BindRandom はその bindgroup が ON のときのみ判定。OFF なら
                    //     should_fire を呼ばず乱数を消費しない（要件 3.1・CRITICAL）。Random は無条件（3.2）。
                    let k = match anim.trigger {
                        LoopTrigger::Random { k } => k,
                        LoopTrigger::BindRandom { k } => {
                            if !states.current_binds(scope).contains(anim.id) {
                                continue;
                            }
                            k
                        }
                        // `always` は抽選しない（乱数を引く前に飛ばす。再生は進行の側・task 3.4）。
                        LoopTrigger::Always { .. } => continue,
                        // `runonce`・`periodic`・`talk` も抽選しない（乱数を引く前に飛ばす。始めるかは進行の
                        // 頭の判定が決める・spec: areka-P0-seriko-trigger-intervals 要件 5.7）。
                        LoopTrigger::Runonce
                        | LoopTrigger::Periodic { .. }
                        | LoopTrigger::Talk { .. } => continue,
                    };
                    // (c) 1/N 抽選（ここで初めて乱数を消費）。
                    if should_fire(k, rng) {
                        playback.entry(key).or_default().insert(
                            anim.id,
                            Playback {
                                started_at_ms: now_ms,
                            },
                        );
                        tracing::info!(
                            scope = scope.as_str(),
                            slot = ?slot,
                            animation_id = anim.id,
                            k,
                            "seriko: loop 抽選発火（再生開始・先頭コマから・要件 2.1/2.2）"
                        );
                    }
                }
            }
        }

        // (3)+(4) 進行と commit（進行の対象の面のうち再生中エントリを持つもの・一番上に `always` が在るもの・
        // 部品の経路では動く部品が見えるものだけ）。
        let mut commands = Vec::new();
        for (scope, slot, sid, open) in &states.stage_slots() {
            let key = (scope.clone(), *slot);
            // 再生中エントリを持たない slot（Idle/IdleResidual のみ）は残留を保ったまま無評価・無発行。
            let has_playback = playback.get(&key).is_some_and(|pb| !pb.is_empty());
            let table: &AnimationTable = match slot {
                Slot::Shell => &*shell_table,
                // 抽選相と同一の scope キー表引き（要件 5.6）。不在 scope は空表ゆえ下の
                // 「表に無い」防御腕へ落ち、playback とコマが除去される（panic なし）。
                Slot::Balloon => balloon_tables.get(scope).unwrap_or(&empty_balloon_table),
            };
            // 部品の経路を通す表か（偽なら部品の行を 1 つも通らない・要件 7.2・7.3）。
            let with_parts = table.has_animated_parts();
            let (has_always, has_trigger) = top_kinds(table, *sid);
            // 文字を数える面（表に `talk` が在り、窓が開いている）の、今現れている文字の数。
            let talks = table.has_talk();
            let now_seen = (talks && *open).then(|| revealed_at(epoch, feeds, scope, now_ms));
            // 引き金の状態は、構える条件（一番上に 3 語が在る、または表に `talk` が在る）を満たす面で
            // だけ持つ。無ければこの刻みの時刻で構え（表の差し替えの後の最初の刻み）、窓の開け閉めを写す。
            let state = (has_trigger || talks)
                .then(|| arm_slot(armed, &key, *sid, *open, now_ms, now_seen));
            // 一番上の再生が無くても、一番上に `always` か 3 語が在るか、見える部品に動くものが在れば通す。
            if !has_playback
                && !has_always
                && !has_trigger
                && !(with_parts
                    && parts.moving_visible(
                        *sid,
                        slot_binds(states, scope, *slot),
                        table,
                        states.current_pattern(scope, *slot),
                    ))
            {
                continue;
            }
            // 一番上の `always` は抽選を待たず、再生が無ければこの刻みの時刻で始まる（要件 4.1）。
            // 窓が閉じている間は作らない。
            if has_always && *open {
                start_top_always(playback, table, scope, *slot, *sid, now_ms);
            }
            // 残留（非再生アニメのコマ）を保つため現 PatternState から開始し、再生中アニメのみを更新する。
            // 再発火したアニメは playback を持つのでここで frame_at のみに従い更新される＝残留の即時クリア。
            let mut new_pattern = states.current_pattern(scope, *slot).clone();

            // 文字の窓（面 1 つに刻み 1 回）。一番上に `talk` が無く部品にだけ在る面でも作る。一番上と
            // 部品が同じ窓を読むので、数えを進めるのは刻みの最後（先に進めると後で読む側が区切りを
            // 見落とす）。
            let wall_ms = |glyph| glyph_wall_ms(epoch, feeds.get(scope), glyph, now_ms);
            let bounds = state.as_ref().and_then(|state| state.talk_window_bounds());
            let window = bounds
                .zip(now_seen)
                .map(|((base, prev_seen), now_seen)| TalkWindow {
                    base,
                    prev_seen,
                    now_seen,
                    wall_ms: &wall_ms,
                });

            // 一番上の 3 語は抽選を待たずに判定する（一番上に 3 語の在る面でだけ回す）。始まった再生は
            // 下の進行がこの刻みのうちに、開始の時刻からの経過の分だけ進める（要件 6.2）。
            if let Some(state) = state.filter(|_| has_trigger) {
                let anims = table.animations(*sid).iter().filter(is_trigger_word);
                fire_top_triggers(
                    state,
                    playback,
                    &mut new_pattern,
                    warned_negative,
                    &key,
                    anims,
                    now_ms,
                    window.as_ref(),
                );
            }

            // 再生中 animation id を昇順で処理（決定論の warn! 順・D-7 と同一方針）。
            let mut playing_ids: Vec<u32> = playback
                .get(&key)
                .map_or(Vec::new(), |pb| pb.keys().copied().collect());
            playing_ids.sort_unstable();

            for anim_id in playing_ids {
                let started_at = playback
                    .get(&key)
                    .and_then(|pb| pb.get(&anim_id))
                    .map(|p| p.started_at_ms);
                let Some(started_at) = started_at else {
                    continue;
                };
                // 出来事の時刻で生まれた再生は刻みより僅かに後に始まりうるので 0 で止める。
                let elapsed = now_ms.saturating_sub(started_at);

                // 表示中 surface のアニメ列から当該 id のコマ列を引く。
                let Some(anim) = table.animations(*sid).iter().find(|a| a.id == anim_id) else {
                    // 防御: 表に無い（surface 変化直後の齟齬等）→ playback もコマも落とす。
                    if let Some(pb) = playback.get_mut(&key) {
                        pb.remove(&anim_id);
                    }
                    new_pattern.remove(anim_id);
                    continue;
                };

                // `always` は周の頭へ戻って続く（末尾でも負の番号でも再生を捨てない・要件 4.2・4.5）。
                if matches!(anim.trigger, LoopTrigger::Always { .. }) {
                    put_top_always(&mut new_pattern, anim, elapsed);
                    continue;
                }

                // 進行相の bind 判定（bindopt 7.3/7.4・D9-2）: bind 種（`BindRandom`）でありながら現在の
                // bind 集合に属さない ID は、再生中であっても停止相当（コマ除去＋playback 除去）へ落とす。
                // 発火ゲート（上の `:contains` 判定）と同一述語の鏡映であり、面種にも依らない
                // （bind 非所属の bind 種はそもそも発火し得ない＝再生が続く根拠が無い）。
                //
                // **これが無いと「再生中に外れた」場合に次 tick でコマが復活する**——状態側の除去
                // （`commit_bind` の `drop_residual_frames`・bindopt 7.1）は playback を触らないため、
                // playback が残る限り下の [`frame_at`] が同じコマを置き直す（2026-08-11 実機で確定）。
                // `Random`（bind に属さない interval アニメ）はこの判定を通らず従来どおり（bindopt 7.4）。
                let dropped_by_bind = matches!(anim.trigger, LoopTrigger::BindRandom { .. })
                    && !states.current_binds(scope).contains(anim_id);
                if dropped_by_bind {
                    new_pattern.remove(anim_id);
                    if let Some(pb) = playback.get_mut(&key) {
                        pb.remove(&anim_id);
                    }
                    tracing::info!(
                        scope = scope.as_str(),
                        slot = ?slot,
                        animation_id = anim_id,
                        "seriko: loop bind から外れた ID の再生を停止（保持コマ除去・bindopt 7.3）"
                    );
                    continue;
                }

                put_top_play(
                    &mut new_pattern,
                    playback,
                    warned_negative,
                    &key,
                    anim,
                    elapsed,
                );
            }

            // 空になった SlotPlayback は除去（Idle へ戻す・空 map を残さない）。
            if playback.get(&key).is_some_and(|pb| pb.is_empty()) {
                playback.remove(&key);
            }

            // 部品: 一番上の進行を済ませた絵で部品の欄を作り直す（一番上の抽選は全スコープぶん済み）。
            // 部品の `talk` はこの面の文字の窓 `window` を借りる（部品は文字の数えを持たない）。
            if with_parts {
                parts.advance(
                    scope,
                    *slot,
                    *sid,
                    slot_binds(states, scope, *slot),
                    table,
                    now_ms,
                    crossed,
                    *open,
                    window.as_ref(),
                    rng,
                    &mut new_pattern,
                );
            }

            // (4) 差分反映: 変化した slot のみ指令を返す（Unchanged は無発行・要件 6.2）。
            if let PatternApplyOutcome::Changed(cmd) =
                states.commit_pattern(scope, *slot, new_pattern)
            {
                commands.push(cmd);
            }
        }

        // 全部の面が文字の窓を読み終えた: 数えを今現れている数まで進め、数え終えた文字を刈り込む
        // （表に `talk` が無ければ文字の列は空で、何もしない）。
        settle_talk(epoch, feeds, armed, now_ms);

        commands
    }

    /// surface 切替／Hide 連動: 当該 (scope, slot) の再生状態と引き金の状態を全除去する（要件 2.3 の
    /// 表示従属性）。
    ///
    /// ukadoc「そのサーフェスである間」＝再生とコマは表示中 surface に従属するため、面が変われば
    /// 再生状態は破棄される。PatternState のクリアは ScopeStates 側 apply の責務（本メソッドは playback のみ）。
    pub(crate) fn on_surface_changed(&mut self, scope: &ActorKey, slot: Slot) {
        self.playback.remove(&(scope.clone(), slot));
        // 引き金の状態も捨てる＝次の評価が「面に入った」として構え直す（同じ面の再指定・着せ替えは
        // ここへ来ないので構え直さない・spec: areka-P0-seriko-trigger-intervals 要件 2.3〜2.6・3.2）。
        self.armed.remove(&(scope.clone(), slot));
        // 残留 warn! 記録も当該 slot ぶんは無効化（新面での負 surface は再度 1 回 warn! されるべき）。
        self.warned_negative
            .retain(|(s, sl, _)| !(s == scope && *sl == slot));
    }

    /// シェルの表の差し替え（spec: areka-P0-shell-balloon-switch 要件 2.8）。
    ///
    /// 全 scope のシェル slot の再生中のループを捨て、次の抽選から新しい表で始め直す。
    /// 残留コマ（PatternState）の消去は [`ScopeStates::rebase_shell`] の責務。
    pub(crate) fn replace_shell_table(&mut self, table: AnimationTable) {
        self.config.shell_table = table;
        self.forget_slot_kind(Slot::Shell);
    }

    /// 面の切り替え・着せ替えの変化の直後に呼ぶ（spec: areka-P0-surface-element-nesting 要件 5.6・
    /// spec: areka-P0-animated-image-playback 要件 2.3・3.1・4.1・4.3）。
    ///
    /// 繰り返しの経路を通す表（[`AnimationTable::is_continuous`]）で、`scope` の `slot` の面が
    /// 進行の対象（[`ScopeStates::stage_slots`]・バルーンは知らせの面も）なら、保持している
    /// [`PatternState`] の写しを出来事の時刻 `at_ms`（無ければ直前の刻みの時刻）で作り直す: 一番上の `always` の再生が無ければその時刻で作って欄を置き、部品の欄は
    /// [`PartClocks::refresh`]（見えている `always` の時計を作る・回数つきの見えなくなった時計を捨てる・
    /// 抽選の時計と乱数は触らない）で作り直す。`commit_pattern` が変化を返したらその `Show` を返す。
    /// 面が無い（`\s[-1]`・`\b[-1]` の後）なら、その面の回数つきの時計と、部品の 3 語の時計・引き金の
    /// 状態を捨てて何も返さない（[`PartClocks::drop_hidden`]）。
    /// バルーンの窓が閉じていれば時計を作らない（回数つきは捨てる）。
    /// 出来事の時刻は刻みの単調性の番人（`last_seen`）に入れない。刻みが 1 度も来ておらず `at_ms` も
    /// 無ければ時計を作らない（次の刻みで生まれる）。
    ///
    /// 引き金（spec: areka-P0-seriko-trigger-intervals 要件 2・5.5・6.1）: 構える条件を満たす面に引き金の
    /// 状態が無ければ（＝[`LoopRuntime::on_surface_changed`] が捨てた後＝面に入った）出来事の時刻で構え、
    /// 一番上の `runonce` をその時刻で鳴らして頭のコマを載せる。状態が在れば（着せ替え・窓の知らせ）
    /// 構え直さず、窓の開け閉めだけを写す。時刻が分からなければ構えない（次の刻みで構える）。
    pub(crate) fn refresh(
        &mut self,
        scope: &ActorKey,
        slot: Slot,
        at_ms: Option<u64>,
        states: &mut ScopeStates,
    ) -> Option<DisplayCommand> {
        let LoopRuntime {
            config,
            playback,
            warned_negative,
            parts,
            last_seen,
            armed,
            epoch,
            feeds,
            ..
        } = self;
        let table = match slot {
            Slot::Shell => &config.shell_table,
            Slot::Balloon => config.balloon_tables.get(scope)?,
        };
        if !table.is_continuous() {
            return None;
        }
        let Some((_, _, sid, open)) = states
            .stage_slots()
            .into_iter()
            .find(|(s, sl, ..)| s == scope && *sl == slot)
        else {
            parts.drop_hidden(scope, slot, table);
            return None;
        };
        let at_ms = at_ms.or(*last_seen);
        let mut pattern = states.current_pattern(scope, slot).clone();
        if let Some(at) = at_ms {
            if open {
                start_top_always(playback, table, scope, slot, sid, at);
            }
            let key = (scope.clone(), slot);
            let always = table
                .animations(sid)
                .iter()
                .filter(|a| matches!(a.trigger, LoopTrigger::Always { .. }));
            for anim in always {
                if let Some(p) = playback.get(&key).and_then(|pb| pb.get(&anim.id)) {
                    put_top_always(&mut pattern, anim, at.saturating_sub(p.started_at_ms));
                }
            }
            // 引き金: 構える条件を満たす面でだけ構え（面に入った）、`runonce` を出来事の時刻で鳴らして
            // 頭のコマをこの評価の絵に載せる（待ちの在る頭は次の刻みが置く）。`periodic` は刻みで鳴らす。
            let (_, has_trigger) = top_kinds(table, sid);
            let talks = table.has_talk();
            if has_trigger || talks {
                // 文字はこの出来事の時刻までに現れた数の次から数える（`talk` は刻みで鳴らす）。
                let revealed = (talks && open).then(|| revealed_at(epoch, feeds, scope, at));
                let state = arm_slot(armed, &key, sid, open, at, revealed);
                let runonce = table
                    .animations(sid)
                    .iter()
                    .filter(|a| a.trigger == LoopTrigger::Runonce);
                let fired = fire_top_triggers(
                    state,
                    playback,
                    &mut pattern,
                    warned_negative,
                    &key,
                    runonce,
                    at,
                    None,
                );
                for (anim, started) in fired {
                    if let FrameStatus::Active(i) | FrameStatus::FinishedResidual(i) =
                        frame_at(&anim.frames, at.saturating_sub(started))
                    {
                        pattern.set(anim.id, pattern_frame(&anim.frames[i]));
                    }
                }
            }
        }
        // 部品は面の種類を問わない（バルーンの面の着せ替えの集合は空）。
        if table.has_animated_parts() {
            parts.refresh(
                scope,
                slot,
                sid,
                slot_binds(states, scope, slot),
                table,
                at_ms,
                open,
                &mut pattern,
            );
        }
        match states.commit_pattern(scope, slot, pattern) {
            PatternApplyOutcome::Changed(cmd) => Some(cmd),
            PatternApplyOutcome::Unchanged => None,
        }
    }

    /// `scope` の `slot` の面の回数つきの時計を全部捨てる（窓が閉じた知らせ・新しい出番・spec:
    /// areka-P0-animated-image-playback 要件 2.3・6.1）。表が無ければ何もしない。
    pub(crate) fn drop_finite(&mut self, scope: &ActorKey, slot: Slot) {
        let table = match slot {
            Slot::Shell => &self.config.shell_table,
            Slot::Balloon => match self.config.balloon_tables.get(scope) {
                Some(table) => table,
                None => return,
            },
        };
        self.parts.drop_finite(scope, slot, table);
    }

    /// バルーンの表の差し替え（spec: areka-P0-shell-balloon-switch 要件 3.5）。
    ///
    /// 全 scope のバルーン slot の再生中のループを捨て、次の抽選から新しい表で始め直す。
    /// シェル側の再生は触らない。
    pub(crate) fn replace_balloon_tables(&mut self, tables: BTreeMap<ActorKey, AnimationTable>) {
        self.config.balloon_tables = tables;
        self.forget_slot_kind(Slot::Balloon);
    }

    /// 全 scope の `slot` 種の再生状態（一番上の `always` を含む）・引き金の状態・部品の時計・warn! 記録を捨てる
    /// （表の差し替えの共通後段・spec: areka-P0-surface-element-nesting 要件 5.8・
    /// spec: areka-P0-animated-image-playback 要件 3.6）。「どれかの表に `talk` が在るか」も組み直し、
    /// 無くなったら文字の時刻の写しを捨てる（spec: areka-P0-seriko-trigger-intervals 要件 8.1）。
    fn forget_slot_kind(&mut self, slot: Slot) {
        self.playback.retain(|(_, s), _| *s != slot);
        self.armed.retain(|(_, s), _| *s != slot);
        self.warned_negative.retain(|(_, s, _)| *s != slot);
        self.parts.clear(slot);
        self.has_talk = any_talk(&self.config);
        if !self.has_talk {
            self.epoch = TalkEpoch::default();
            self.feeds.clear();
        }
    }
}

#[cfg(test)]
#[path = "looper_always_tests.rs"]
mod always_tests;
#[cfg(test)]
#[path = "looper_balloon_tests.rs"]
mod balloon_tests;
#[cfg(test)]
#[path = "looper_film_count_tests.rs"]
mod film_count_tests;
#[cfg(test)]
#[path = "looper_parts_emo2_tests.rs"]
pub(crate) mod parts_emo2_tests;
#[cfg(test)]
#[path = "looper_parts_tests.rs"]
mod parts_tests;
#[cfg(test)]
#[path = "looper_replace_tests.rs"]
mod replace_tests;
#[cfg(test)]
#[path = "looper_talk_tests.rs"]
mod talk_tests;
#[cfg(test)]
#[path = "looper_test_support.rs"]
mod test_support;
#[cfg(test)]
#[path = "looper_tests.rs"]
pub(crate) mod tests;
#[cfg(test)]
#[path = "looper_trigger_tests.rs"]
mod trigger_tests;
