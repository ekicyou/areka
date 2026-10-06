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
use areka_sakura::ActorKey;

use crate::actor::SerikoClock;
use crate::output::DisplayCommand;
use crate::parts::PartClocks;
use crate::state::{PatternApplyOutcome, ScopeStates, Slot};
use crate::table::{AnimationTable, LoopAnimation, LoopFrame, LoopTrigger};
use crate::timeline::{
    AlwaysView, FrameStatus, LoopRng, LotteryBoundary, always_at, current_frame_index, frame_at,
    seeded_rng, should_fire,
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

/// 一番上 `surface_id` が `always` を持つか（`is_continuous()` が偽の表では必ず偽・要件 7.2）。
fn top_has_always(table: &AnimationTable, surface_id: u32) -> bool {
    table.is_continuous()
        && table
            .animations(surface_id)
            .iter()
            .any(|a| matches!(a.trigger, LoopTrigger::Always { .. }))
}

impl LoopRuntime {
    /// ループ構成を受けて再生状態ゼロの統括器を構築する（表・rng は注入済み）。
    pub(crate) fn new(config: SerikoLoopConfig) -> Self {
        Self {
            config,
            boundary: None,
            playback: HashMap::new(),
            last_seen: None,
            warned_negative: HashSet::new(),
            parts: PartClocks::default(),
            clock: None,
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
            let has_always = top_has_always(table, *sid);
            // 一番上の再生が無くても、一番上に `always` が在るか、見える部品に動くものが在れば通す。
            if !has_playback
                && !has_always
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
                            if let Some(pb) = playback.get_mut(&key) {
                                pb.remove(&anim_id);
                            }
                            tracing::info!(
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
                                let wkey = (scope.clone(), *slot, anim_id);
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
                        if let Some(pb) = playback.get_mut(&key) {
                            pb.remove(&anim_id);
                        }
                        tracing::info!(
                            scope = scope.as_str(),
                            slot = ?slot,
                            animation_id = anim_id,
                            "seriko: loop 停止（負 surface でベース復帰・要件 4.3）"
                        );
                    }
                }
            }

            // 空になった SlotPlayback は除去（Idle へ戻す・空 map を残さない）。
            if playback.get(&key).is_some_and(|pb| pb.is_empty()) {
                playback.remove(&key);
            }

            // 部品: 一番上の進行を済ませた絵で部品の欄を作り直す（一番上の抽選は全スコープぶん済み）。
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

        commands
    }

    /// surface 切替／Hide 連動: 当該 (scope, slot) の再生状態を全除去する（要件 2.3 の表示従属性）。
    ///
    /// ukadoc「そのサーフェスである間」＝再生とコマは表示中 surface に従属するため、面が変われば
    /// 再生状態は破棄される。PatternState のクリアは ScopeStates 側 apply の責務（本メソッドは playback のみ）。
    pub(crate) fn on_surface_changed(&mut self, scope: &ActorKey, slot: Slot) {
        self.playback.remove(&(scope.clone(), slot));
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
    /// 面が無い（`\s[-1]`・`\b[-1]` の後）なら、その面の回数つきの時計を捨てて何も返さない。
    /// バルーンの窓が閉じていれば時計を作らない（回数つきは捨てる）。
    /// 出来事の時刻は刻みの単調性の番人（`last_seen`）に入れない。刻みが 1 度も来ておらず `at_ms` も
    /// 無ければ時計を作らない（次の刻みで生まれる）。
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
            parts,
            last_seen,
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
            parts.drop_finite(scope, slot, table);
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

    /// バルーンの表の差し替え（spec: areka-P0-shell-balloon-switch 要件 3.5）。
    ///
    /// 全 scope のバルーン slot の再生中のループを捨て、次の抽選から新しい表で始め直す。
    /// シェル側の再生は触らない。
    pub(crate) fn replace_balloon_tables(&mut self, tables: BTreeMap<ActorKey, AnimationTable>) {
        self.config.balloon_tables = tables;
        self.forget_slot_kind(Slot::Balloon);
    }

    /// 全 scope の `slot` 種の再生状態（一番上の `always` を含む）・部品の時計・warn! 記録を捨てる
    /// （表の差し替えの共通後段・spec: areka-P0-surface-element-nesting 要件 5.8・
    /// spec: areka-P0-animated-image-playback 要件 3.6）。
    fn forget_slot_kind(&mut self, slot: Slot) {
        self.playback.retain(|(_, s), _| *s != slot);
        self.warned_negative.retain(|(_, s, _)| *s != slot);
        self.parts.clear(slot);
    }
}

#[cfg(test)]
#[path = "looper_always_tests.rs"]
mod always_tests;
#[cfg(test)]
#[path = "looper_balloon_tests.rs"]
mod balloon_tests;
#[cfg(test)]
#[path = "looper_parts_emo2_tests.rs"]
mod parts_emo2_tests;
#[cfg(test)]
#[path = "looper_parts_tests.rs"]
mod parts_tests;
#[cfg(test)]
#[path = "looper_replace_tests.rs"]
mod replace_tests;
#[cfg(test)]
#[path = "looper_tests.rs"]
pub(crate) mod tests;
