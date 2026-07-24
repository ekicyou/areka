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
//! ScopeStates 側 apply の責務）。
//!
//! # 抽選の固定消費順（D-7）
//!
//! scope 昇順（`ActorKey` 辞書順）→ Shell → Balloon → animation id 昇順。注入乱数列の消費順が
//! 一意に定まり、決定論テストが期待値を焼き込める。**bind ゲート不通過（`BindRandom` でその
//! bindgroup が OFF）のアニメには [`should_fire`] を呼ばない＝乱数を消費しない**（要件 3.1）。
//!
//! bind の書込 API（`apply_bind` 等）は一切呼ばない（read-only 参照のみ・要件 3.3）。

use std::collections::{HashMap, HashSet};

use areka_emo_compose::PatternFrame;
use areka_sakura::ActorKey;

use crate::output::DisplayCommand;
use crate::state::{PatternApplyOutcome, ScopeStates, Slot};
use crate::table::{AnimationTable, LoopFrame, LoopTrigger};
use crate::timeline::{frame_at, seeded_rng, should_fire, FrameStatus, LoopRng, LotteryBoundary};

/// SERIKO ループ構成（表 2 面＋乱数注入シーム）。boot 時に組み立てて [`LoopRuntime`] へ値渡しする。
///
/// `shell_table`／`balloon_table` は **surface ID 名前空間の別**であり能力の仕切りではない
/// （面種非依存・裁定 (a)）。emo2 は `balloon_table` が空（データ事実）。`rng` はコンストラクタ注入で、
/// 評価経路に実 entropy への直接依存を持たない（要件 7.1）。
pub struct SerikoLoopConfig {
    /// シェル表示エントリ用のアニメ表（surface ID 名前空間: shell）。
    pub shell_table: AnimationTable,
    /// バルーン表示エントリ用のアニメ表（surface ID 名前空間: balloon）。emo2 は空。
    pub balloon_table: AnimationTable,
    /// 1/N 抽選の乱数注入シーム（本番は `seeded_rng(seed)`・テストは注入列）。
    pub rng: LoopRng,
}

impl SerikoLoopConfig {
    /// 空表＋ダミー乱数（ループ完全不活性）。既存テスト・非 emo2 経路の非退行用。
    ///
    /// 表が空ゆえ抽選対象アニメが常にゼロ＝[`should_fire`] は決して呼ばれず乱数は消費されない
    /// （ダミー種は観測に現れない）。`on_tick` は常に空を返す（非退行）。
    pub fn disabled() -> Self {
        Self {
            shell_table: AnimationTable::empty(),
            balloon_table: AnimationTable::empty(),
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
    /// 表 2 面＋乱数注入シーム（spawn 時注入・以後不変の表＋可変 rng 状態）。
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
}

/// 固定消費順のための slot ランク（Shell を Balloon より前に置く・D-7）。
fn slot_rank(slot: Slot) -> u8 {
    match slot {
        Slot::Shell => 0,
        Slot::Balloon => 1,
    }
}

/// `frame_at` と同一の累積 wait デッドライン選択で現在コマ index を返す（`Stopped` の負 surface 値を
/// warn! 用に取り出すためだけの補助・分岐判定の正典は [`frame_at`]）。
fn current_frame_index(frames: &[LoopFrame], elapsed_ms: u64) -> Option<usize> {
    let mut acc: u64 = 0;
    let mut current: Option<usize> = None;
    for (i, f) in frames.iter().enumerate() {
        acc = acc.saturating_add(u64::from(f.wait_ms));
        if acc <= elapsed_ms {
            current = Some(i);
        } else {
            break;
        }
    }
    current
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
        }
    }

    /// 1 tick の統括。発行すべき指令列（通常 0〜2 件）を返す。発行自体は actor が行う（要件 6.3）。
    ///
    /// 手順: (1) 単調性ガード（`now < 前回` → `debug!`＋無視）。(2) 境界跨ぎ時のみ抽選（表示中×非再生中×
    /// bind ゲート通過のアニメへ固定消費順で [`should_fire`]・fire で playback 登録＋`info!`）。(3) 全再生中
    /// アニメへ [`frame_at`] を評価し slot ごとの新 [`PatternState`] を組む（`Pending`→エントリ除去＝再発火
    /// 残留の即時クリア・`Active`/`FinishedResidual`→コマ搬送・`Stopped`→除去＋playback 除去・
    /// `FinishedResidual`→コマ残留のまま playback 除去）。(4) slot ごとに `commit_pattern` し `Changed` を集約。
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
            ..
        } = self;
        let SerikoLoopConfig {
            shell_table,
            balloon_table,
            rng,
        } = config;

        // 表示中 slot を列挙し、固定消費順（scope 昇順→Shell→Balloon）へ整列する（D-7）。
        let mut shown = states.shown_slots();
        shown.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| slot_rank(a.1).cmp(&slot_rank(b.1))));

        // (2) 抽選（境界跨ぎ時のみ）: 表示中×非再生中×bind ゲート通過のアニメへ固定消費順で should_fire。
        if crossed {
            for (scope, slot, sid) in &shown {
                let table: &AnimationTable = match slot {
                    Slot::Shell => &*shell_table,
                    Slot::Balloon => &*balloon_table,
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
                    };
                    // (c) 1/N 抽選（ここで初めて乱数を消費）。
                    if should_fire(k, rng) {
                        playback
                            .entry(key)
                            .or_default()
                            .insert(anim.id, Playback { started_at_ms: now_ms });
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

        // (3)+(4) 進行と commit（表示中 slot のうち再生中エントリを持つものだけ）。
        let mut commands = Vec::new();
        for (scope, slot, sid) in &shown {
            let key = (scope.clone(), *slot);
            // 再生中エントリを持たない slot（Idle/IdleResidual のみ）は残留を保ったまま無評価・無発行。
            let has_playback = playback.get(&key).is_some_and(|pb| !pb.is_empty());
            if !has_playback {
                continue;
            }
            let table: &AnimationTable = match slot {
                Slot::Shell => &*shell_table,
                Slot::Balloon => &*balloon_table,
            };

            // 残留（非再生アニメのコマ）を保つため現 PatternState から開始し、再生中アニメのみを更新する。
            // 再発火したアニメは playback を持つのでここで frame_at のみに従い更新される＝残留の即時クリア。
            let mut new_pattern = states.current_pattern(scope, *slot).clone();

            // 再生中 animation id を昇順で処理（決定論の warn! 順・D-7 と同一方針）。
            let mut playing_ids: Vec<u32> =
                playback.get(&key).map_or(Vec::new(), |pb| pb.keys().copied().collect());
            playing_ids.sort_unstable();

            for anim_id in playing_ids {
                let started_at = playback
                    .get(&key)
                    .and_then(|pb| pb.get(&anim_id))
                    .map(|p| p.started_at_ms);
                let Some(started_at) = started_at else {
                    continue;
                };
                let elapsed = now_ms - started_at;

                // 表示中 surface のアニメ列から当該 id のコマ列を引く。
                let Some(anim) = table.animations(*sid).iter().find(|a| a.id == anim_id) else {
                    // 防御: 表に無い（surface 変化直後の齟齬等）→ playback もコマも落とす。
                    if let Some(pb) = playback.get_mut(&key) {
                        pb.remove(&anim_id);
                    }
                    new_pattern.remove(anim_id);
                    continue;
                };

                match frame_at(&anim.frames, elapsed) {
                    // 先頭デッドライン未到達＝ベース露出。再発火時はここで残留コマが即時クリアされる（討議 #2）。
                    FrameStatus::Pending => {
                        new_pattern.remove(anim_id);
                    }
                    // 現在コマ 1 枚を搬送（4.2）。Active は再生継続、FinishedResidual は残留のうえ playback 除去。
                    FrameStatus::Active(i) | FrameStatus::FinishedResidual(i) => {
                        let f = &anim.frames[i];
                        // frame_at が Active/FinishedResidual を返す時点で surface_id は非負（負は Stopped）。
                        new_pattern.set(
                            anim_id,
                            PatternFrame {
                                surface_id: f.surface_id as u32,
                                method: f.method.clone(),
                                x: f.x,
                                y: f.y,
                            },
                        );
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

            // (4) 差分反映: 変化した slot のみ指令を返す（Unchanged は無発行・要件 6.2）。
            if let PatternApplyOutcome::Changed(cmd) = states.commit_pattern(scope, *slot, new_pattern)
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolve::SurfaceTarget;
    use areka_emo_compose::{BindSet, EmoWorld, PatternState};
    use areka_parsers::shell::{
        Animation, AppendTarget, DefRef, DrawMethod, Interval, Pattern, Shell, Surface,
    };
    use std::sync::{Arc, Mutex};

    // ── テスト用の表構築（from_world 経由＝実構築パスを通す） ──────────────────

    /// コマ 1 本（overlay 固定・x/y=0）。
    fn pat(index: u32, surface_id: i64, wait: u32) -> Pattern {
        Pattern {
            index,
            method: DrawMethod::new("overlay".to_string()),
            surface_id,
            wait,
            x: 0,
            y: 0,
        }
    }

    /// animation 1 本を持つ surface 1 件。
    fn surface_with(id: u32, animations: Vec<Animation>) -> Surface {
        Surface {
            id,
            targets: vec![AppendTarget::Single(id)],
            elements: Vec::new(),
            collisions: Vec::new(),
            animations,
        }
    }

    fn world_of(surfaces: Vec<Surface>) -> EmoWorld {
        let definitions = (0..surfaces.len()).map(DefRef::Surface).collect();
        let shell = Shell {
            surfaces,
            appends: Vec::new(),
            aliases: Vec::new(),
            animation_sort: None,
            collision_sort: None,
            definitions,
        };
        EmoWorld::build(&shell)
    }

    /// surfaces から shell 表を build する。
    fn shell_table(surfaces: Vec<Surface>) -> AnimationTable {
        AnimationTable::from_world(&world_of(surfaces))
    }

    /// 単一 anim（id/interval/frames）を持つ surface から shell 表を build する。
    fn table_single(
        surface_id: u32,
        anim_id: u32,
        interval: Interval,
        frames: &[(i64, u32)],
    ) -> AnimationTable {
        let patterns = frames
            .iter()
            .enumerate()
            .map(|(i, (sid, wait))| pat(i as u32, *sid, *wait))
            .collect();
        shell_table(vec![surface_with(
            surface_id,
            vec![Animation {
                id: anim_id,
                interval,
                patterns,
            }],
        )])
    }

    // ── 注入乱数（消費回数を計数する列 rng） ──────────────────────────────────

    #[derive(Default)]
    struct RngProbe {
        calls: usize,
        values: std::collections::VecDeque<u32>,
    }

    /// 指定した値列を順に返し、呼ばれた回数を計数する rng を作る（尽きたら 1 を返す＝非発火）。
    /// 返す `Arc` から消費回数・残量を照合できる（bind OFF での**非消費**の檻・要件 3.1）。
    fn counting_rng(values: &[u32]) -> (LoopRng, Arc<Mutex<RngProbe>>) {
        let probe = Arc::new(Mutex::new(RngProbe {
            calls: 0,
            values: values.iter().copied().collect(),
        }));
        let handle = Arc::clone(&probe);
        let rng: LoopRng = Box::new(move |_bound: u32| -> u32 {
            let mut p = probe.lock().unwrap();
            p.calls += 1;
            p.values.pop_front().unwrap_or(1)
        });
        (rng, handle)
    }

    /// 常に発火する rng（`should_fire` は `rng(k)==0` で発火）。
    fn always_fire() -> LoopRng {
        Box::new(|_bound: u32| 0)
    }

    fn shown_shell(sid: u32) -> (ScopeStates, ActorKey) {
        shown_shell_with_binds(sid, BindSet::from_ids([]))
    }

    fn shown_shell_with_binds(sid: u32, binds: BindSet) -> (ScopeStates, ActorKey) {
        let mut states = ScopeStates::new(binds);
        let scope = ActorKey::from("0");
        states.apply(&scope, SurfaceTarget::Show(sid));
        (states, scope)
    }

    fn cfg(shell_table: AnimationTable, rng: LoopRng) -> SerikoLoopConfig {
        SerikoLoopConfig {
            shell_table,
            balloon_table: AnimationTable::empty(),
            rng,
        }
    }

    // ── 抽選ゲート: 表示中×非再生中×bind ゲート ───────────────────────────────

    /// bind OFF（BindRandom で bindgroup が current_binds に無い）→ **乱数を消費しない**・発火しない
    /// （要件 3.1・CRITICAL）。表示中でも境界を跨いでも rng は 1 度も呼ばれない。
    #[test]
    fn bindrandom_off_does_not_consume_rng() {
        // BindRandom{4}・anim id 7・bind ゲート id は 7。binds は空 ⇒ OFF。
        let table = table_single(10, 7, Interval::BindRandom { k: 4 }, &[(1412, 0)]);
        let (rng, probe) = counting_rng(&[0, 0, 0]); // もし呼ばれれば発火してしまう値
        let mut rt = LoopRuntime::new(cfg(table, rng));
        let (mut states, _scope) = shown_shell(10); // binds 空 ⇒ id 7 は OFF

        // 起動 tick（遅延初期化・非跨ぎ）→ 境界を跨ぐ tick。
        assert!(rt.on_tick(0, &mut states).is_empty(), "起動 tick は無発行");
        let cmds = rt.on_tick(1000, &mut states); // 境界跨ぎ
        assert!(cmds.is_empty(), "bind OFF は発火しない＝無発行");

        // 乱数は 1 度も消費されていない（ゲート不通過で should_fire 未呼出・要件 3.1）。
        assert_eq!(probe.lock().unwrap().calls, 0, "bind OFF で rng は非消費（要件 3.1）");
    }

    /// bind ON（bindgroup が current_binds に在る）＋境界跨ぎ＋発火 rng → 発火し先頭コマを発行する
    /// （要件 3.2/2.2）。rng は 1 回だけ消費される。
    #[test]
    fn bindrandom_on_fires_and_consumes_rng_once() {
        let table = table_single(10, 7, Interval::BindRandom { k: 4 }, &[(1412, 0), (1411, 150)]);
        let (rng, probe) = counting_rng(&[0]); // 1 回目 0 ⇒ 発火
        let mut rt = LoopRuntime::new(cfg(table, rng));
        // binds に id 7 を含める ⇒ ON。
        let (mut states, scope) = shown_shell_with_binds(10, BindSet::from_ids([7]));

        assert!(rt.on_tick(0, &mut states).is_empty());
        let cmds = rt.on_tick(1000, &mut states); // 跨ぎ＋発火＋elapsed0 で先頭コマ 1412
        assert_eq!(probe.lock().unwrap().calls, 1, "ON では rng を 1 回消費");
        assert_eq!(cmds.len(), 1, "発火＋先頭コマで 1 指令");
        match &cmds[0] {
            DisplayCommand::Show { scope: s, surface_id, pattern, .. } => {
                assert_eq!(s, &scope);
                assert_eq!(*surface_id, 10, "表示中 surface は 10");
                let f = pattern.get(7).expect("anim7 の現在コマ");
                assert_eq!(f.surface_id, 1412, "先頭コマ 1412（要件 2.2 先頭から）");
            }
            other => panic!("Show を期待: {other:?}"),
        }
    }

    /// Random は無条件ゲート（bind に依らず発火する・要件 3.2）。
    #[test]
    fn random_fires_unconditionally() {
        let table = table_single(10, 0, Interval::Random { k: 4 }, &[(2106, 0)]);
        let mut rt = LoopRuntime::new(cfg(table, always_fire()));
        let (mut states, _scope) = shown_shell(10); // binds 空でも Random は無条件

        rt.on_tick(0, &mut states);
        let cmds = rt.on_tick(1000, &mut states);
        assert_eq!(cmds.len(), 1, "Random は無条件で発火");
        match &cmds[0] {
            DisplayCommand::Show { pattern, .. } => {
                assert_eq!(pattern.get(0).unwrap().surface_id, 2106);
            }
            other => panic!("Show を期待: {other:?}"),
        }
    }

    /// 非表示 slot は評価対象ゼロ＝抽選も進行も走らず乱数も消費しない（要件 2.1 の表示中ゲート）。
    #[test]
    fn hidden_slot_consumes_no_rng() {
        let table = table_single(10, 0, Interval::Random { k: 4 }, &[(2106, 0)]);
        let (rng, probe) = counting_rng(&[0]);
        let mut rt = LoopRuntime::new(cfg(table, rng));
        // Show せず（未知 scope）＝表示中 slot が 1 つも無い。
        let mut states = ScopeStates::new(BindSet::from_ids([]));

        rt.on_tick(0, &mut states);
        let cmds = rt.on_tick(1000, &mut states);
        assert!(cmds.is_empty());
        assert_eq!(probe.lock().unwrap().calls, 0, "表示中 slot 皆無で rng 非消費");
    }

    /// 再生中アニメは再抽選対象外＝次の境界跨ぎで乱数を消費しない（要件 2.3）。
    #[test]
    fn playing_anim_is_not_relotteried() {
        // 長い残留アニメ（末尾非負・150ms 継続）。
        let table = table_single(10, 0, Interval::Random { k: 4 }, &[(2106, 0), (2110, 150)]);
        let (rng, probe) = counting_rng(&[0]); // 最初の跨ぎで 1 回だけ発火
        let mut rt = LoopRuntime::new(cfg(table, rng));
        let (mut states, _scope) = shown_shell(10);

        rt.on_tick(0, &mut states);
        rt.on_tick(1000, &mut states); // 発火（rng 1 回消費）・再生中へ
        // 次の境界（2000）で再生中ゆえ再抽選しない＝rng 追加消費なし。
        rt.on_tick(2000, &mut states);
        assert_eq!(
            probe.lock().unwrap().calls,
            1,
            "再生中アニメは再抽選されず rng は追加消費されない（要件 2.3）"
        );
    }

    // ── 終端意味論: Stopped（ベース復帰）／FinishedResidual（残留・再抽選可） ────

    /// kero 型（末尾 `-1`）: `-1` 到達でコマ除去＋playback 除去＝ベース復帰し、以降の境界で再抽選できる
    /// （rng 再消費・要件 4.3/2.3）。
    #[test]
    fn kero_negative_tail_restores_base_and_is_relotteriable() {
        // 2106(w0)/2110(w40)/-1(w80) ⇒ t=[0,40,120]。
        let table = table_single(10, 0, Interval::Random { k: 4 }, &[(2106, 0), (2110, 40), (-1, 80)]);
        let (rng, probe) = counting_rng(&[0, 0]); // 2 度の跨ぎで各 1 回発火
        let mut rt = LoopRuntime::new(cfg(table, rng));
        let (mut states, _scope) = shown_shell(10);

        rt.on_tick(0, &mut states);
        let c1 = rt.on_tick(1000, &mut states); // 発火・elapsed0 → 2106
        assert_eq!(pattern_of(&c1[0]).get(0).unwrap().surface_id, 2106);

        // 経過 120ms（1120）で -1 → Stopped → ベース復帰（空 pattern の Show）。
        let c2 = rt.on_tick(1120, &mut states);
        assert_eq!(c2.len(), 1, "停止でベース復帰の再発行");
        assert!(pattern_of(&c2[0]).is_empty(), "-1 停止でコマ除去＝空 pattern（ベース復帰・要件 4.3）");

        // 次の境界 2000 で再抽選できる（Idle へ戻っている）＝rng が再度消費される。
        rt.on_tick(2000, &mut states);
        assert_eq!(probe.lock().unwrap().calls, 2, "停止後は Idle ゆえ再抽選（rng 再消費）");
    }

    /// sakura 型（末尾非負）: 末尾到達で最終コマを残したまま playback 除去＝IdleResidual。残留は保たれ、
    /// 次の境界で再抽選できる（要件 4.4/2.3）。
    #[test]
    fn sakura_residual_keeps_frame_and_is_relotteriable() {
        // 1412(w0)/1411(w150)/1410(w22) ⇒ t=[0,150,172]。末尾非負。
        let table = table_single(10, 7, Interval::BindRandom { k: 4 }, &[(1412, 0), (1411, 150), (1410, 22)]);
        let (rng, probe) = counting_rng(&[0, 0]);
        let mut rt = LoopRuntime::new(cfg(table, rng));
        let (mut states, _scope) = shown_shell_with_binds(10, BindSet::from_ids([7]));

        rt.on_tick(0, &mut states);
        rt.on_tick(1000, &mut states); // 発火 → 1412
        // 経過 172ms（1172）で末尾 1410 残留。
        let c = rt.on_tick(1172, &mut states);
        assert_eq!(c.len(), 1);
        assert_eq!(pattern_of(&c[0]).get(7).unwrap().surface_id, 1410, "末尾コマ 1410 残留（要件 4.4）");

        // さらに時間が進んでも残留は不変（無発行）。
        assert!(rt.on_tick(1500, &mut states).is_empty(), "残留は恒久＝無発行（要件 6.2）");

        // 次の境界 2000 で再抽選できる（IdleResidual は抽選対象・要件 2.3/9.4）。
        rt.on_tick(2000, &mut states);
        assert_eq!(probe.lock().unwrap().calls, 2, "IdleResidual は再抽選対象（rng 再消費）");
    }

    /// 残留→再発火の**即時クリア**（討議 #2）: 先頭 wait>0 の合成テーブルで、末尾非負の残留状態から
    /// 再発火した tick に残留コマが即消えベース復帰し、先頭コマ deadline で改めて表示される。
    #[test]
    fn residual_is_immediately_cleared_on_refire() {
        // 先頭 wait=50>0・末尾非負の 2 コマ: 700(w50)/701(w50) ⇒ t=[50,100]。
        let table = table_single(10, 0, Interval::Random { k: 4 }, &[(700, 50), (701, 50)]);
        let mut rt = LoopRuntime::new(cfg(table, always_fire()));
        let (mut states, _scope) = shown_shell(10);

        rt.on_tick(0, &mut states);
        rt.on_tick(1000, &mut states); // 発火（started_at=1000）・elapsed0 → Pending（wait50 未到達）＝無コマ
        // 経過 100ms（1100）で末尾 701 残留。
        let c1 = rt.on_tick(1100, &mut states);
        assert_eq!(pattern_of(&c1[0]).get(0).unwrap().surface_id, 701, "末尾 701 残留");

        // 次の境界 2000 で再発火（always_fire）。elapsed0 → Pending → 残留 701 が即時クリア（ベース復帰）。
        let c2 = rt.on_tick(2000, &mut states);
        assert_eq!(c2.len(), 1, "再発火 tick で残留クリアの再発行");
        assert!(
            pattern_of(&c2[0]).is_empty(),
            "再発火の瞬間に残留コマが即時クリア＝ベース露出（討議 #2）"
        );

        // 先頭コマ deadline（started_at 2000＋50＝2050）で先頭コマ 700 が表示される。
        let c3 = rt.on_tick(2050, &mut states);
        assert_eq!(pattern_of(&c3[0]).get(0).unwrap().surface_id, 700, "先頭コマ 700 が deadline で表示");
    }

    // ── 固定消費順（scope 昇順→Shell→Balloon→id 昇順・D-7） ─────────────────

    /// 同一 surface に複数アニメ（id 昇順で消費）。注入列 [1,0] で id 昇順の 2 本目のみ発火することを示し、
    /// 消費順が animation id 昇順であることを檻に入れる（要件 2.4・D-7）。
    #[test]
    fn lottery_consumes_in_animation_id_order() {
        // 同 surface 10 に anim id 5 と anim id 2（宣言順は 5→2＝id 昇順ではない）。
        let s = surface_with(
            10,
            vec![
                Animation { id: 5, interval: Interval::Random { k: 4 }, patterns: vec![pat(0, 5000, 0)] },
                Animation { id: 2, interval: Interval::Random { k: 4 }, patterns: vec![pat(0, 2000, 0)] },
            ],
        );
        let table = shell_table(vec![s]);
        // 消費列 [1, 0]: 1 本目（id 昇順の先頭＝id 2）は 1（非発火）、2 本目（id 5）は 0（発火）。
        let (rng, probe) = counting_rng(&[1, 0]);
        let mut rt = LoopRuntime::new(cfg(table, rng));
        let (mut states, _scope) = shown_shell(10);

        rt.on_tick(0, &mut states);
        let cmds = rt.on_tick(1000, &mut states);
        assert_eq!(probe.lock().unwrap().calls, 2, "2 アニメ分の rng を消費");
        assert_eq!(cmds.len(), 1, "id 5 のみ発火");
        // id 2 は非発火（コマ無し）・id 5 は発火（コマ 5000）＝消費順が id 昇順（2→5）である証拠。
        let p = pattern_of(&cmds[0]);
        assert!(p.get(2).is_none(), "id 2 は非発火（消費列 1 番目に 1 が割当）");
        assert_eq!(p.get(5).unwrap().surface_id, 5000, "id 5 は発火（消費列 2 番目に 0 が割当）");
    }

    /// scope 昇順で消費する（D-7）。scope "0"→"1" の順で乱数を消費し、注入列 [1,0] で scope1 のみ発火。
    #[test]
    fn lottery_consumes_in_scope_ascending_order() {
        // 同一 shell_table 上に surface10（scope0 表示）と surface11（scope1 表示）。
        let s10 = surface_with(
            10,
            vec![Animation { id: 0, interval: Interval::Random { k: 4 }, patterns: vec![pat(0, 1000, 0)] }],
        );
        let s11 = surface_with(
            11,
            vec![Animation { id: 0, interval: Interval::Random { k: 4 }, patterns: vec![pat(0, 1100, 0)] }],
        );
        let table = shell_table(vec![s10, s11]);
        // [1, 0]: 先頭消費（scope "0"）は非発火、2 番目（scope "1"）は発火。
        let (rng, probe) = counting_rng(&[1, 0]);
        let mut rt = LoopRuntime::new(cfg(table, rng));
        let mut states = ScopeStates::new(BindSet::from_ids([]));
        let s0 = ActorKey::from("0");
        let s1 = ActorKey::from("1");
        states.apply(&s0, SurfaceTarget::Show(10));
        states.apply(&s1, SurfaceTarget::Show(11));

        rt.on_tick(0, &mut states);
        let cmds = rt.on_tick(1000, &mut states);
        assert_eq!(probe.lock().unwrap().calls, 2);
        assert_eq!(cmds.len(), 1, "scope1 のみ発火");
        match &cmds[0] {
            DisplayCommand::Show { scope, pattern, .. } => {
                assert_eq!(scope, &s1, "発火したのは scope \"1\"（scope 昇順で \"0\" が先に消費された証拠）");
                assert_eq!(pattern.get(0).unwrap().surface_id, 1100);
            }
            other => panic!("Show を期待: {other:?}"),
        }
    }

    /// Shell を Balloon より先に消費する（同一 scope・D-7）。注入列 [0,1] で Shell が発火・Balloon が非発火。
    #[test]
    fn lottery_consumes_shell_before_balloon() {
        let shell_t = table_single(10, 0, Interval::Random { k: 4 }, &[(2106, 0)]);
        let balloon_t = table_single(20, 1, Interval::Random { k: 4 }, &[(3000, 0)]);
        // 消費列 [0, 1]: Shell(先) 発火・Balloon(後) 非発火。
        let (rng, probe) = counting_rng(&[0, 1]);
        let mut rt = LoopRuntime::new(SerikoLoopConfig {
            shell_table: shell_t,
            balloon_table: balloon_t,
            rng,
        });
        let mut states = ScopeStates::new(BindSet::from_ids([]));
        let scope = ActorKey::from("0");
        states.apply(&scope, SurfaceTarget::Show(10)); // Shell shown surface 10
        states.apply_balloon(&scope, SurfaceTarget::Show(20)); // Balloon shown surface 20

        rt.on_tick(0, &mut states);
        let cmds = rt.on_tick(1000, &mut states);
        assert_eq!(probe.lock().unwrap().calls, 2);
        assert_eq!(cmds.len(), 1, "Shell のみ発火");
        assert!(
            matches!(&cmds[0], DisplayCommand::Show { .. }),
            "発火したのは Shell（Show）＝Shell が Balloon より先に消費（D-7）"
        );
    }

    // ── on_surface_changed / 単調性ガード ──────────────────────────────────

    /// `on_surface_changed` は当該 slot の playback を全除去する（surface 従属・要件 2.3）。
    /// 除去後は再抽選対象へ戻る（rng が再消費される）。
    #[test]
    fn on_surface_changed_removes_playback() {
        let table = table_single(10, 0, Interval::Random { k: 4 }, &[(2106, 0), (2110, 150)]);
        let (rng, probe) = counting_rng(&[0, 0]);
        let mut rt = LoopRuntime::new(cfg(table, rng));
        let (mut states, scope) = shown_shell(10);

        rt.on_tick(0, &mut states);
        rt.on_tick(1000, &mut states); // 発火・再生中（rng 1 回）
        // surface 切替連動で playback 除去。
        rt.on_surface_changed(&scope, Slot::Shell);
        // 次の境界で再抽選できる（playback が消えたので Idle）＝rng 再消費。
        rt.on_tick(2000, &mut states);
        assert_eq!(
            probe.lock().unwrap().calls,
            2,
            "on_surface_changed 後は Idle ゆえ再抽選（rng 再消費）"
        );
    }

    /// 非単調 tick（now < 前回）は無視され、状態を変えず空を返す（rng も消費しない・防御・要件 1.2）。
    #[test]
    fn non_monotonic_tick_is_ignored() {
        let table = table_single(10, 0, Interval::Random { k: 4 }, &[(2106, 0)]);
        let (rng, probe) = counting_rng(&[0]);
        let mut rt = LoopRuntime::new(cfg(table, rng));
        let (mut states, _scope) = shown_shell(10);

        rt.on_tick(1000, &mut states); // 初回（遅延初期化・境界 next=2000）
        // 過去へ戻る tick は無視。
        let cmds = rt.on_tick(500, &mut states);
        assert!(cmds.is_empty(), "非単調 tick は無発行");
        // 抽選も走らない＝rng 非消費（初回 1000 は境界跨ぎしない＝next は 2000）。
        assert_eq!(probe.lock().unwrap().calls, 0, "非単調 tick は rng を消費しない");
    }

    /// bind 書込 API は一切呼ばない（read-only）: on_tick 前後で current_binds が不変であることの反証。
    #[test]
    fn on_tick_never_writes_binds() {
        let table = table_single(10, 7, Interval::BindRandom { k: 4 }, &[(1412, 0)]);
        let mut rt = LoopRuntime::new(cfg(table, always_fire()));
        let (mut states, scope) = shown_shell_with_binds(10, BindSet::from_ids([7]));

        let before = states.current_binds(&scope).clone();
        rt.on_tick(0, &mut states);
        rt.on_tick(1000, &mut states); // 発火（BindRandom ON）
        let after = states.current_binds(&scope).clone();
        assert_eq!(before, after, "on_tick は bind を書き込まない（read-only・要件 3.3）");
    }

    /// `SerikoLoopConfig::disabled()` は空表＋ダミー乱数でループ完全不活性（非退行・常に無発行）。
    #[test]
    fn disabled_config_is_inert() {
        let mut rt = LoopRuntime::new(SerikoLoopConfig::disabled());
        let (mut states, _scope) = shown_shell(10);
        assert!(rt.on_tick(0, &mut states).is_empty());
        assert!(rt.on_tick(1000, &mut states).is_empty(), "空表は境界跨ぎでも無発行");
        assert!(rt.on_tick(5000, &mut states).is_empty());
    }

    /// 変化なし tick は無発行（冪等・要件 6.2）: 再生中でも同一コマが続く間は指令を返さない。
    #[test]
    fn unchanged_tick_emits_nothing() {
        // 先頭コマが長く続く（w0 の 2106 が 150ms 継続）。
        let table = table_single(10, 0, Interval::Random { k: 4 }, &[(2106, 0), (2110, 150)]);
        let mut rt = LoopRuntime::new(cfg(table, always_fire()));
        let (mut states, _scope) = shown_shell(10);

        rt.on_tick(0, &mut states);
        let c1 = rt.on_tick(1000, &mut states); // 発火・2106
        assert_eq!(c1.len(), 1);
        // 同じ 2106 が続く中間 tick（境界も跨がない）は無発行。
        assert!(rt.on_tick(1050, &mut states).is_empty(), "同一コマ継続は無発行（要件 6.2）");
        assert!(rt.on_tick(1100, &mut states).is_empty());
    }

    // ── 補助 ────────────────────────────────────────────────────────────────

    fn pattern_of(cmd: &DisplayCommand) -> &PatternState {
        match cmd {
            DisplayCommand::Show { pattern, .. } | DisplayCommand::ShowBalloon { pattern, .. } => {
                pattern
            }
            other => panic!("pattern を持つ指令を期待: {other:?}"),
        }
    }
}
