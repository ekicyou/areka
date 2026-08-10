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

use std::collections::{BTreeMap, HashMap, HashSet};

use areka_emo_compose::PatternFrame;
use areka_sakura::ActorKey;

use crate::output::DisplayCommand;
use crate::state::{PatternApplyOutcome, ScopeStates, Slot};
use crate::table::{AnimationTable, LoopFrame, LoopTrigger};
use crate::timeline::{frame_at, seeded_rng, should_fire, FrameStatus, LoopRng, LotteryBoundary};

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
            balloon_tables,
            rng,
        } = config;
        // 不在 scope へ貸す空表（抽選対象ゼロ・乱数非消費・panic なし＝`disabled()` と同じ不活性・
        // 要件 5.6）。`BTreeMap::new()` は確保を伴わないため tick ごとの構築コストは無い。
        let empty_balloon_table = AnimationTable::empty();

        // 表示中 slot を列挙し、固定消費順（scope 昇順→Shell→Balloon）へ整列する（D-7）。
        let mut shown = states.shown_slots();
        shown.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| slot_rank(a.1).cmp(&slot_rank(b.1))));

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
                // 抽選相と同一の scope キー表引き（要件 5.6）。不在 scope は空表ゆえ下の
                // 「表に無い」防御腕へ落ち、playback とコマが除去される（panic なし）。
                Slot::Balloon => balloon_tables.get(scope).unwrap_or(&empty_balloon_table),
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
#[path = "looper_tests.rs"]
mod tests;
