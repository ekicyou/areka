//! parts: 部品の時計（spec: areka-P0-surface-element-nesting 要件 5.1〜5.8・5.11〜5.14・8.3）。
//!
//! 部品＝element定義で置かれた子と、pattern定義が指して今の絵に出ているサーフェス。部品の
//! animation は、そのサーフェスを一番上に表示したときと同じ列（[`AnimationTable::animations`]）を
//! 同じ規則（境界の抽選・[`frame_at`] の進行・`-1` の停止・末尾の保持）で動かす。
//!
//! 時計の入れ物の鍵は (スコープ, 面の種類)、中の鍵は ([`PartKey`], animation の番号)（spec:
//! areka-P0-animated-image-playback 要件 3.5・6.6）。element定義の子も pattern定義の先も同じ鍵を
//! 使うので、同じ番号は 1 つの時計で揃って動く（要件 5.3・5.4・5.14）。評価する部品は見える部品と、
//! 一番上と見える部品に置かれた動く絵の子（[`areka_emo_compose::NestTable::visible_films`]）。見えない
//! 部品には触らない（抽選・進行・欄への書き込みのどれもしない・要件 5.11）。`Playing` は開始の時刻を
//! 持ち続けるので、見えなかった間も進んでいた扱いになり、戻った刻みに経過から今のコマが決まる（要件 5.7）。
//!
//! `always`（手書きと動く絵の子・spec: areka-P0-animated-image-playback 要件 2.3・2.7・2.8・3.1〜3.5・
//! 4.4・7.4）は抽選せず乱数も引かない。時計が無ければ評価の時刻で生まれ、[`always_at`] の答えを欄へ
//! 書く（経過 0 と同じなら載せない）。回数つき（`laps` が在る）の時計は、評価のとき見えていなければ
//! 捨てる（次に見えたら頭から）。面が隠れたときは [`PartClocks::drop_hidden`] で入れ物 1 つの回数つきの
//! 時計を全部捨てる。終わりなしの時計は捨てない。
//!
//! `runonce`・`periodic`・`talk`（spec: areka-P0-seriko-trigger-intervals 要件 5.4・5.6・5.7・5.9・8.1）も
//! 抽選せず乱数も引かない。評価は 2 段: 1 段目（今までの評価）は在る時計を進めるだけで、2 段目が
//! 「見えると決まった部品」にだけ引き金の状態（[`Armed`]）を構えて判定する（[`fire_part_triggers`]）。
//! 鳴った再生のコマが別の部品を見せたら 1 段目へ戻り、同じ刻みのうちに子のコマまで出す。見えなく
//! なった部品の 3 語の時計と引き金の状態は捨てる（再び見えた刻みが新しい起点で、途中のコマから
//! 始まらない）。窓が閉じたら 3 語の時計を捨てて引き金の状態を隠す。表に 3 語が無ければ 2 段目は
//! 呼ばず、引き金の状態は生まれない。部品は文字の数えを持たない: `talk` は、刻みが渡す面の文字の窓を
//! 一番上と同じ判定に掛けて鳴る（要件 4.1。後から見えた部品も数え直さない）。
//!
//! 抽選の時計を動かすのは刻み（[`PartClocks::advance`]・以下 `advance`）だけ。面の切り替え・
//! 着せ替えの変化の直後は [`PartClocks::refresh`] が抽選せずに今のコマを求める（要件 5.6）。`refresh` が
//! 書き換えるのは `always` の時計（出来事の時刻で生まれる・回数つきの見えなくなったものを捨てる）と、
//! 3 語の時計（見えた部品の `runonce` が出来事の時刻で始まる・見えなくなった部品のものを捨てる）だけ。
//! 時計は [`PartClocks::clear`]（その面の種類の表の差し替え）で捨てる（要件 5.8）。抽選の発火・停止・
//! 末尾での保持の記録は `advance` だけが出す。`always` の時計の誕生と破棄は、生まれた・捨てたところで出す。

use std::collections::{BTreeMap, HashMap, HashSet};
use std::num::{NonZeroU32, NonZeroU64};

use areka_emo_compose::{BindSet, Cell, FilmId, PartKey, PatternFrame, PatternState};
use areka_sakura::ActorKey;

use crate::looper::{log_play_end, pattern_frame};
use crate::state::Slot;
use crate::table::{AnimationTable, LoopAnimation, LoopTrigger};
use crate::timeline::{
    AlwaysView, FrameStatus, LoopRng, always_at, current_frame_index, frame_at, should_fire,
};
use crate::trigger::{Armed, TalkWindow};

// 部品の引き金の配線（2 段目: 構える・判定して時計を作る）。
#[path = "parts_trigger.rs"]
mod part_trigger;
use part_trigger::fire_part_triggers;

/// animation 1 本の時計。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PartAnim {
    /// 再生中（開始の時刻）。今のコマは `frame_at(経過)` で求める。
    Playing { started_at_ms: u64 },
    /// `-1` の無い末尾まで進んで、最後のコマを保っている（抽選の対象）。
    Residual { frame_index: usize },
}

/// 入れ物の中の時計の鍵（部品, animation の番号）。
type ClockKey = (PartKey, u32);

/// (スコープ, 面の種類) の入れ物 1 つ。
#[derive(Default)]
struct Container {
    /// (部品, animation の番号) → 時計。
    clocks: BTreeMap<ClockKey, PartAnim>,
    /// 見えている部品 → 引き金の状態（spec: areka-P0-seriko-trigger-intervals 要件 5.9）。`runonce`・
    /// `periodic`・`talk` を持つ部品が見えている間だけ在る（部品は文字の数えを持たない）。
    armed: BTreeMap<PartKey, Armed>,
}

/// スコープ → 面の種類の 2 段で持つのは、刻みごとにスコープの名前を写さずに引くため。
type Containers = HashMap<ActorKey, HashMap<Slot, Container>>;

/// 記録の欄 `part` に部品を書く形（作者のサーフェスは番号だけ・動く絵の子は `film:番号`）。
struct Label(PartKey);

impl std::fmt::Display for Label {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            PartKey::Surface(id) => write!(f, "{id}"),
            PartKey::Film(FilmId(id)) => write!(f, "film:{id}"),
        }
    }
}

/// (スコープ, 面の種類) × (部品, animation の番号) ごとの時計（持ち主は `LoopRuntime`）。
#[derive(Default)]
pub(crate) struct PartClocks {
    /// スコープ → 面の種類 → (部品, animation の番号) → 時計。
    clocks: Containers,
    /// `-1` 以外の負の番号の `warn!` を (スコープ, 面の種類, 部品, animation の番号) ごとに初回だけ
    /// 出した記録。
    warned_negative: HashSet<(ActorKey, Slot, ClockKey)>,
    /// `visible_parts` の作業用の列（刻みごとの確保を避けて使い回す）。
    visible: Vec<u32>,
    /// `visible_films` の作業用の列（使い回す）。
    films: Vec<FilmId>,
    /// その刻みで評価した部品の番号（昇順・使い回す）。
    evaluated: Vec<u32>,
    /// 部品 1 つの animation を番号の昇順に回すための添字の列（使い回す）。
    order: Vec<usize>,
}

/// 時計 1 本を `now_ms` で見た答え（`advance` と `refresh` で共有）。
enum Look {
    /// 時計が無いか、先頭の待ちの前（コマ無し）。
    Nothing,
    /// このコマを出す（再生中か、保っているコマ）。
    Frame(usize),
    /// `-1` の無い末尾に着いた（このコマを出す。`advance` は `Residual` へ移す）。
    Finished(usize),
    /// 負の番号のコマに着いた（`advance` は時計を消す）。
    Stopped { surface_id: i64 },
}

fn look(anim: &LoopAnimation, clock: Option<PartAnim>, now_ms: u64) -> Look {
    match clock {
        None => Look::Nothing,
        Some(PartAnim::Residual { frame_index }) => Look::Frame(frame_index),
        Some(PartAnim::Playing { started_at_ms }) => {
            let elapsed = now_ms.saturating_sub(started_at_ms);
            match frame_at(&anim.frames, elapsed) {
                FrameStatus::Pending => Look::Nothing,
                FrameStatus::Active(i) => Look::Frame(i),
                FrameStatus::FinishedResidual(i) => Look::Finished(i),
                FrameStatus::Stopped => Look::Stopped {
                    surface_id: current_frame_index(&anim.frames, elapsed)
                        .map_or(-1, |i| anim.frames[i].surface_id),
                },
            }
        }
    }
}

/// 着せ替えの番人の答え。
enum Gate {
    /// 抽選する（頻度 K）。
    Lottery(u32),
    /// `always`（抽選しない・乱数を引かない）。
    Always {
        period_ms: NonZeroU64,
        laps: Option<NonZeroU32>,
    },
    /// `runonce`・`periodic`・`talk`（抽選しない・乱数を引かない・着せ替えに依らない・spec:
    /// areka-P0-seriko-trigger-intervals 要件 5.7）。1 段目は在る時計を進めるだけで、始めるかは
    /// 2 段目の判定が決める。
    Trigger,
    /// 対象外（`bind+random` で `binds` に無い・要件 5.12）。
    Off,
}

fn gate(anim: &LoopAnimation, binds: &BindSet) -> Gate {
    match anim.trigger {
        LoopTrigger::Random { k } => Gate::Lottery(k),
        LoopTrigger::BindRandom { k } if binds.contains(anim.id) => Gate::Lottery(k),
        LoopTrigger::BindRandom { .. } => Gate::Off,
        LoopTrigger::Always { period_ms, laps } => Gate::Always { period_ms, laps },
        LoopTrigger::Runonce | LoopTrigger::Periodic { .. } | LoopTrigger::Talk { .. } => {
            Gate::Trigger
        }
    }
}

/// `runonce`・`periodic`・`talk` の引き金か。
fn is_trigger(anim: &LoopAnimation) -> bool {
    matches!(
        anim.trigger,
        LoopTrigger::Runonce | LoopTrigger::Periodic { .. } | LoopTrigger::Talk { .. }
    )
}

/// 入れ物 1 つの「見えなくなったら捨てる時計」（回数つきの `always` と 3 語）のうち、`drop(部品)` が
/// 真のものを捨てる（捨てたら `debug!`）。抽選の時計と終わりなしの `always` の時計は残す。
fn drop_transient_where(
    clocks: &mut BTreeMap<ClockKey, PartAnim>,
    table: &AnimationTable,
    scope: &ActorKey,
    slot: Slot,
    drop: impl Fn(PartKey) -> bool,
) {
    clocks.retain(|&(part, id), _| {
        if !drop(part) {
            return true;
        }
        let Some(anim) = table.part_animations(part).iter().find(|a| a.id == id) else {
            return true;
        };
        if matches!(anim.trigger, LoopTrigger::Always { laps: Some(_), .. }) {
            tracing::debug!(
                scope = scope.as_str(),
                slot = ?slot,
                part = %Label(part),
                animation_id = id,
                "seriko: part 回数つきの時計を捨てた"
            );
        } else if is_trigger(anim) {
            tracing::debug!(
                scope = scope.as_str(),
                slot = ?slot,
                part = %Label(part),
                animation_id = id,
                "seriko: part 引き金の時計を捨てた"
            );
        } else {
            return true;
        }
        false
    });
}

/// `always` の経過 `elapsed_ms` の答えを欄へ書く: 経過 0 のコマと同じなら載せない（要件 7.4）／
/// コマならコマ（絵を指すなら子の欄）／何も出さないなら、経過 0 のコマが在るときだけ「消えている」。
fn write_always(
    pattern: &mut PatternState,
    part: PartKey,
    anim: &LoopAnimation,
    period_ms: NonZeroU64,
    laps: Option<NonZeroU32>,
    elapsed_ms: u64,
) {
    let rest = always_at(&anim.frames, period_ms, laps, 0);
    match always_at(&anim.frames, period_ms, laps, elapsed_ms) {
        now if now == rest => {}
        AlwaysView::Frame(i) => write(pattern, part, anim, i),
        AlwaysView::Nothing => {
            // 子のコマは全部が絵を指すので「何も出さない」は来ない（子の欄は「消えている」を持てない）。
            if let PartKey::Surface(id) = part {
                pattern.set_part_blank(id, anim.id);
            }
        }
    }
}

/// `always` の時計の開始の時刻。時計が無ければ `at_ms` で作る（生まれたら `debug!`）。`at_ms` が
/// 無い（時刻が分からない）ときは作らず `None`（経過 0＝欄に載せない）。
fn always_start(
    clocks: &mut BTreeMap<ClockKey, PartAnim>,
    key: ClockKey,
    at_ms: Option<u64>,
    scope: &ActorKey,
    slot: Slot,
    finite: bool,
) -> Option<u64> {
    if let Some(PartAnim::Playing { started_at_ms }) = clocks.get(&key) {
        return Some(*started_at_ms);
    }
    let at = at_ms?;
    clocks.insert(key, PartAnim::Playing { started_at_ms: at });
    tracing::debug!(
        scope = scope.as_str(),
        slot = ?slot,
        part = %Label(key.0),
        animation_id = key.1,
        finite,
        "seriko: part always の時計が生まれた"
    );
    Some(at)
}

/// 回数つきの時計と 3 語の時計のうち、今の絵（見える部品 `visible`・見える子 `films`）に見えていない
/// ものを捨てる（次に見えたら頭から・要件 2.3・2.7）。見えなくなった部品の引き金の状態も捨てる
/// （再び見えた刻みが新しい起点・spec: areka-P0-seriko-trigger-intervals 要件 5.6・5.9）。
fn drop_unseen_transient(
    container: &mut Container,
    table: &AnimationTable,
    scope: &ActorKey,
    slot: Slot,
    visible: &[u32],
    films: &[FilmId],
) {
    let seen = |part: PartKey| match part {
        PartKey::Surface(id) => visible.binary_search(&id).is_ok(),
        PartKey::Film(film) => films.binary_search(&film).is_ok(),
    };
    drop_transient_where(&mut container.clocks, table, scope, slot, |part| {
        !seen(part)
    });
    container.armed.retain(|&part, _| seen(part));
}

/// 窓が閉じた: 回数つきの時計と 3 語の時計を全部捨て、引き金の状態を隠す（閉じている間は回数つきの
/// 時計が無く、欄は経過 0 へ戻る・spec: areka-P0-animated-image-playback 要件 2.3・6.1。3 語は
/// 閉じている間は鳴らず、開いた時刻が `periodic` の新しい起点になる。`runonce` の 1 回は使い済みの
/// まま・spec: areka-P0-seriko-trigger-intervals 要件 5.5）。
fn close(container: &mut Container, table: &AnimationTable, scope: &ActorKey, slot: Slot) {
    drop_transient_where(&mut container.clocks, table, scope, slot, |_| true);
    container.armed.values_mut().for_each(Armed::hide);
}

/// `anims` を animation の番号の昇順に回す添字を `order` に入れる。
fn sort_by_id(anims: &[LoopAnimation], order: &mut Vec<usize>) {
    order.clear();
    order.extend(0..anims.len());
    order.sort_unstable_by_key(|&i| anims[i].id);
}

/// [`rebuild`] が部品 1 つに頼む評価の段。
#[derive(Clone, Copy)]
enum Phase {
    /// 1 段目: 抽選・`always`・在る時計の進行（見える扱いになった部品と動く絵の子に 1 回ずつ）。
    Evaluate,
    /// 2 段目: 見えると決まった部品の `runonce`・`periodic`・`talk` を判定する（spec:
    /// areka-P0-seriko-trigger-intervals 要件 5.6・5.9）。再生が始まったら真を返す。
    Trigger,
}

/// 部品の欄と動く絵の子の欄のうち、`keep(部品, animation の番号)` が偽のものを外す（コマと「消えて
/// いる」の両方を残す。欄 1 つを外す口が無いので、全部を消して残すものを置き直す）。
fn retain_cells(pattern: &mut PatternState, keep: impl Fn(PartKey, u32) -> bool) {
    enum Kept {
        Frame(PatternFrame),
        Blank,
        Picture(u32),
    }
    let kept: Vec<(PartKey, u32, Kept)> = pattern
        .cells()
        .filter_map(|(key, id, cell)| {
            let key = key.filter(|&key| keep(key, id))?;
            let cell = match cell {
                Cell::Frame(f) => Kept::Frame(f.clone()),
                Cell::Blank => Kept::Blank,
                Cell::Picture(picture) => Kept::Picture(picture),
                Cell::Rest => return None,
            };
            Some((key, id, cell))
        })
        .collect();
    pattern.clear_parts();
    for (key, id, cell) in kept {
        match (key, cell) {
            (PartKey::Surface(p), Kept::Frame(f)) => pattern.set_part(p, id, f),
            (PartKey::Surface(p), Kept::Blank) => pattern.set_part_blank(p, id),
            (PartKey::Film(film), Kept::Picture(picture)) => pattern.set_film(film, picture),
            _ => {}
        }
    }
}

/// 部品の欄を空にし、見える部品が増えなくなるまで［求める → 未評価を番号の昇順に 1 段目の `step`］を
/// 回す。評価済みの集合は増えるだけなので必ず止まる。
///
/// 表に `runonce`・`periodic`・`talk` が在るときだけ、ここで 2 段目へ進む（spec:
/// areka-P0-seriko-trigger-intervals 要件 5.6・5.9・8.1）: 見えると決まった部品のうち、この評価で
/// まだ判定していないものに 2 段目の `step` を回す。再生が始まったら（そのコマが別の部品を見せうる
/// ので）1 段目へ戻り、新しく見えた部品を評価してから、その部品を判定する＝同じ評価のうちに子まで
/// 出る。部品 1 つを 2 度判定しない（判定済みの集合も増えるだけなので必ず止まる）。1 段目の評価を
/// 受けただけで見えると決まらなかった部品は、2 段目に来ない。
///
/// 次に、同じ刻みの後の回で見えなくなった部品（着せ替えのコマが pattern0 を置き換えた先など）の
/// コマを欄から外す。見えない部品の辺は `visible_parts` がたどらないので、外しても `visible` は
/// 変わらない（部品の欄の番号は `visible` に含まれる・design.md「Data Models」）。最後に、一番上と
/// 見える部品に置かれた動く絵の子を番号の昇順に 1 段目の `step` へ渡す（子は element を持たないので
/// 見える部品を増やさない）。終わったとき `visible`・`films` は今の絵の見える部品・見える子。
#[allow(clippy::too_many_arguments)]
fn rebuild(
    table: &AnimationTable,
    surface_id: u32,
    binds: &BindSet,
    pattern: &mut PatternState,
    visible: &mut Vec<u32>,
    evaluated: &mut Vec<u32>,
    films: &mut Vec<FilmId>,
    mut step: impl FnMut(Phase, PartKey, &mut PatternState) -> bool,
) {
    pattern.clear_parts();
    evaluated.clear();
    let nest = table.nest_table();
    // この評価で 2 段目を済ませた部品（昇順）。
    let mut judged: Vec<u32> = Vec::new();
    loop {
        nest.visible_parts(surface_id, binds, pattern, visible);
        let mut added = false;
        for &part in visible.iter() {
            let Err(at) = evaluated.binary_search(&part) else {
                continue;
            };
            evaluated.insert(at, part);
            added = true;
            step(Phase::Evaluate, PartKey::Surface(part), pattern);
        }
        if added {
            continue;
        }
        if !table.has_triggers() {
            break;
        }
        let mut started = false;
        for &part in visible.iter() {
            let Err(at) = judged.binary_search(&part) else {
                continue;
            };
            judged.insert(at, part);
            started |= step(Phase::Trigger, PartKey::Surface(part), pattern);
        }
        if !started {
            break;
        }
    }

    if evaluated.len() > visible.len() {
        retain_cells(pattern, |key, _| match key {
            PartKey::Surface(p) => visible.binary_search(&p).is_ok(),
            PartKey::Film(_) => false,
        });
    }

    nest.visible_films(surface_id, visible, films);
    for &film in films.iter() {
        step(Phase::Evaluate, PartKey::Film(film), pattern);
    }
}

/// 部品 `part` の animation `id` の欄を「載っていない」へ戻す（載っていなければ何もしない）。
fn unwrite(pattern: &mut PatternState, part: PartKey, id: u32) {
    if pattern.cell(Some(part), id) != Cell::Rest {
        retain_cells(pattern, |key, cell_id| (key, cell_id) != (part, id));
    }
}

/// 時計 1 本を `now_ms` まで進めて欄へ書く（刻みの進行。抽選の animation も 3 語も同じ）: 再生中なら
/// 経過から今のコマ・`-1` の無い末尾に着いたら最後のコマを保つ時計へ移す・負の番号のコマに着いたら
/// 時計を消す。コマを書いたら真（時計が無い・先頭の待ちの前・止まったは偽）。
#[allow(clippy::too_many_arguments)]
fn progress(
    clocks: &mut BTreeMap<ClockKey, PartAnim>,
    warned_negative: &mut HashSet<(ActorKey, Slot, ClockKey)>,
    pattern: &mut PatternState,
    scope: &ActorKey,
    slot: Slot,
    part: PartKey,
    anim: &LoopAnimation,
    now_ms: u64,
) -> bool {
    let key = (part, anim.id);
    let label = Label(part);
    // `talk` の再生は文字の到着ごとに起きうるので、終わりの記録は `debug!`（ほかは `info!`・文言は
    // 同じ・spec: areka-P0-seriko-trigger-intervals 要件 7.3）。
    let is_talk = matches!(anim.trigger, LoopTrigger::Talk { .. });
    // 進行: 開始からの経過で今のコマを決める（見えなかった間も進んでいた扱い・要件 5.7）。
    match look(anim, clocks.get(&key).copied(), now_ms) {
        Look::Nothing => false,
        Look::Frame(i) => {
            write(pattern, part, anim, i);
            true
        }
        Look::Finished(i) => {
            clocks.insert(key, PartAnim::Residual { frame_index: i });
            log_play_end!(
                is_talk,
                scope = scope.as_str(),
                part = %label,
                animation_id = anim.id,
                "seriko: part 末尾残留（最終コマ保持・再抽選対象へ・要件 5.1）"
            );
            write(pattern, part, anim, i);
            true
        }
        Look::Stopped { surface_id } => {
            // `-1` は正典駆動、それ以外の負値は初回だけ warn!（自アニメ停止扱い・他アニメ停止は非駆動）。
            if surface_id != -1 && warned_negative.insert((scope.clone(), slot, key)) {
                tracing::warn!(
                    scope = scope.as_str(),
                    part = %label,
                    animation_id = anim.id,
                    surface_id,
                    "seriko: part `-1` 以外の負 surface（自アニメ停止扱い・他アニメ停止 `-2` は非駆動）"
                );
            }
            clocks.remove(&key);
            log_play_end!(
                is_talk,
                scope = scope.as_str(),
                part = %label,
                animation_id = anim.id,
                "seriko: part 停止（負 surface でベース復帰・要件 5.1）"
            );
            false
        }
    }
}

/// コマ `i` を部品 `part` の欄へ書く（絵を指すコマは動く絵の子の欄へ）。
fn write(pattern: &mut PatternState, part: PartKey, anim: &LoopAnimation, i: usize) {
    let Some(f) = anim.frames.get(i) else {
        return;
    };
    match part {
        PartKey::Surface(id) => pattern.set_part(id, anim.id, pattern_frame(f)),
        PartKey::Film(film) => {
            if let Some(picture) = f.picture {
                pattern.set_film(film, picture);
            }
        }
    }
}

impl PartClocks {
    /// 刻み 1 回。`scope` の `slot` の面の見える部品を抽選（境界を跨いだ刻みだけ）・進行させ、
    /// `pattern` の部品の欄を今の絵の分で作り直す。`open` が偽（バルーンの窓が閉じている）なら
    /// 時計を作らず（抽選もしない）、回数つきの時計を全部捨てる＝今ある終わりなしの時計だけが進む
    /// （spec: areka-P0-animated-image-playback 要件 6.1）。
    ///
    /// `talk` はこの面の文字の窓（一番上の配線が刻み 1 回につき 1 つ作る。表に `talk` が無い面・
    /// 窓が閉じている面では `None`）。見えると決まった部品の `talk` は、この窓を一番上と同じ判定に
    /// 掛けて鳴る＝部品は文字の数えを持たず、面の数えを借りる（後から見えた部品も数え直さない・
    /// 見えなかった間に越えた区切りは後から鳴らない・spec: areka-P0-seriko-trigger-intervals
    /// 要件 4.1・5.4・5.6）。数えを進めるのは呼び手（全部の面が窓を読み終えた刻みの最後）。
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn advance(
        &mut self,
        scope: &ActorKey,
        slot: Slot,
        surface_id: u32,
        binds: &BindSet,
        table: &AnimationTable,
        now_ms: u64,
        crossed: bool,
        open: bool,
        talk: Option<&TalkWindow<'_>>,
        rng: &mut LoopRng,
        pattern: &mut PatternState,
    ) {
        let PartClocks {
            clocks,
            warned_negative,
            visible,
            films,
            evaluated,
            order,
        } = self;
        if !clocks.contains_key(scope) {
            clocks.insert(scope.clone(), HashMap::new());
        }
        let Some(clocks) = clocks.get_mut(scope) else {
            return;
        };
        let container = clocks.entry(slot).or_default();
        if !open {
            close(container, table, scope, slot);
        }
        let Container { clocks, armed } = &mut *container;
        // この評価の 1 段目が進める前に再生中だった 3 語の時計の（鍵, 開始の時刻）。2 段目が「開始の
        // 時刻に再生中だったか」を測るのに読む（1 段目が片付けた再生も、ここには残る）。
        let mut was_playing: Vec<(ClockKey, u64)> = Vec::new();

        // 部品 1 つの評価（animation の番号の昇順・design.md「部品 1 つの評価」）。
        let step = |phase: Phase, part: PartKey, pattern: &mut PatternState| {
            let anims = table.part_animations(part);
            if let Phase::Trigger = phase {
                let started = fire_part_triggers(
                    clocks,
                    armed,
                    order,
                    anims,
                    scope,
                    slot,
                    part,
                    now_ms,
                    open,
                    &was_playing,
                    talk,
                    false,
                );
                for anim in &started {
                    // 始まった再生を、開始の時刻からこの刻みまでの経過の分だけ進める。コマが無ければ
                    // （頭の待ちの前・もう止まった）、1 段目が書いた前の再生のコマを外す＝鳴り直した
                    // 後は新しい再生の経過だけで決まる。
                    if !progress(
                        clocks,
                        warned_negative,
                        pattern,
                        scope,
                        slot,
                        part,
                        anim,
                        now_ms,
                    ) {
                        unwrite(pattern, part, anim.id);
                    }
                }
                return !started.is_empty();
            }
            sort_by_id(anims, order);
            for &i in order.iter() {
                let anim = &anims[i];
                let key = (part, anim.id);
                let label = Label(part);
                let k = match gate(anim, binds) {
                    Gate::Lottery(k) => k,
                    // 着せ替えの種類で有効でないものは抽選せず、時計を消す（要件 5.12）。
                    Gate::Off => {
                        if clocks.remove(&key).is_some() {
                            tracing::info!(
                                scope = scope.as_str(),
                                part = %label,
                                animation_id = anim.id,
                                "seriko: part bind から外れた ID の再生を停止（保持コマ除去・要件 5.12）"
                            );
                        }
                        continue;
                    }
                    // `always`: 乱数を引かず、時計が無ければこの時刻で作る（要件 3.1・4.1・4.4）。
                    // 窓が閉じている間は作らない（無ければ経過 0）。
                    Gate::Always { period_ms, laps } => {
                        let at = open.then_some(now_ms);
                        let started = always_start(clocks, key, at, scope, slot, laps.is_some())
                            .unwrap_or(now_ms);
                        let elapsed = now_ms.saturating_sub(started);
                        write_always(pattern, part, anim, period_ms, laps, elapsed);
                        continue;
                    }
                    // 3 語: 抽選せず乱数も引かない。在る時計を進めるだけ（始めるかは 2 段目）。
                    Gate::Trigger => {
                        if let Some(&PartAnim::Playing { started_at_ms }) = clocks.get(&key) {
                            was_playing.push((key, started_at_ms));
                        }
                        progress(
                            clocks,
                            warned_negative,
                            pattern,
                            scope,
                            slot,
                            part,
                            anim,
                            now_ms,
                        );
                        continue;
                    }
                };
                // 境界を跨いだ刻みで再生中でなければ抽選（保っているコマは当たれば捨てる）。
                // 窓が閉じている間は時計を作らないので抽選もしない。
                let playing = matches!(clocks.get(&key), Some(PartAnim::Playing { .. }));
                if crossed && open && !playing && should_fire(k, rng) {
                    clocks.insert(
                        key,
                        PartAnim::Playing {
                            started_at_ms: now_ms,
                        },
                    );
                    tracing::info!(
                        scope = scope.as_str(),
                        part = %label,
                        animation_id = anim.id,
                        k,
                        "seriko: part 抽選発火（再生開始・先頭コマから・要件 5.1）"
                    );
                }
                progress(
                    clocks,
                    warned_negative,
                    pattern,
                    scope,
                    slot,
                    part,
                    anim,
                    now_ms,
                );
            }
            false
        };

        rebuild(
            table, surface_id, binds, pattern, visible, evaluated, films, step,
        );

        drop_unseen_transient(container, table, scope, slot, visible, films);
    }

    /// 面の切り替え・着せ替えの変化の直後（出来事の時刻 `at_ms`）に、抽選せずに `pattern` の部品の欄を
    /// 作り直す（要件 5.6・spec: areka-P0-animated-image-playback 要件 3.1・4.1）。見える部品の求め方は
    /// `advance` と同じ。見えている `always` の時計が無ければ `at_ms` で作り、回数つきの見えなくなった
    /// 時計を捨てる。抽選の animation の時計は書き換えない（末尾に着いたコマ・負の番号のコマも、時計を
    /// 移さず・消さずに、出す・出さないだけを決める）。`bind+random` で `binds` に無い animation の
    /// コマは書かない（時計を消すのは次の `advance`・要件 5.12）。`at_ms` が無い（刻みが 1 度も来て
    /// いない）ときは時計を作らず、経過 0 の時刻で読む。`open` が偽なら `advance` と同じく時計を
    /// 作らず、回数つきの時計を全部捨てる。
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn refresh(
        &mut self,
        scope: &ActorKey,
        slot: Slot,
        surface_id: u32,
        binds: &BindSet,
        table: &AnimationTable,
        at_ms: Option<u64>,
        open: bool,
        pattern: &mut PatternState,
    ) {
        let PartClocks {
            clocks,
            visible,
            films,
            evaluated,
            order,
            ..
        } = self;
        let container = clocks
            .entry(scope.clone())
            .or_default()
            .entry(slot)
            .or_default();
        if !open {
            close(container, table, scope, slot);
        }
        let Container { clocks, armed } = &mut *container;
        let now_ms = at_ms.unwrap_or(0);
        // 窓が閉じている間は時計を作らない（今ある時計を読むだけ）。
        let create_at = at_ms.filter(|_| open);
        let step = |phase: Phase, part: PartKey, pattern: &mut PatternState| {
            let anims = table.part_animations(part);
            if let Phase::Trigger = phase {
                // 時刻が分からなければ構えない（次の刻みが、その時刻で構える）。
                let Some(at) = at_ms else {
                    return false;
                };
                // 出来事の直後に鳴るのは `runonce` だけ（文字の窓は渡さない）。鳴るのは構えた評価か、
                // 閉じた窓で構えて最初に開いた評価なので、前の再生は無い（閉じている間の 3 語の時計は
                // 捨ててある）。
                let started = fire_part_triggers(
                    clocks,
                    armed,
                    order,
                    anims,
                    scope,
                    slot,
                    part,
                    at,
                    open,
                    &[],
                    None,
                    true,
                );
                for anim in &started {
                    // 頭のコマをこの評価の絵に載せる（待ちの在る頭は次の刻みが置く）。
                    if let Look::Frame(i) | Look::Finished(i) =
                        look(anim, clocks.get(&(part, anim.id)).copied(), at)
                    {
                        write(pattern, part, anim, i);
                    }
                }
                return !started.is_empty();
            }
            sort_by_id(anims, order);
            for &i in order.iter() {
                let anim = &anims[i];
                let key = (part, anim.id);
                match gate(anim, binds) {
                    Gate::Lottery(_) | Gate::Trigger => {}
                    Gate::Off => continue,
                    Gate::Always { period_ms, laps } => {
                        if let Some(started) =
                            always_start(clocks, key, create_at, scope, slot, laps.is_some())
                        {
                            let elapsed = now_ms.saturating_sub(started);
                            write_always(pattern, part, anim, period_ms, laps, elapsed);
                        }
                        continue;
                    }
                }
                if let Look::Frame(i) | Look::Finished(i) =
                    look(anim, clocks.get(&key).copied(), now_ms)
                {
                    write(pattern, part, anim, i);
                }
            }
            false
        };
        rebuild(
            table, surface_id, binds, pattern, visible, evaluated, films, step,
        );
        drop_unseen_transient(container, table, scope, slot, visible, films);
    }

    /// 一番上の再生が無い刻みで部品の経路を通すか: `pattern` の絵に見える部品か見える動く絵の子に、
    /// 動く animation を持つものが在るか（作業用の列を使い回す・design.md「Performance & Scalability」）。
    ///
    /// 保持している部品の欄に載る番号は見える部品に含まれ（design.md「Data Models」の不変条件）、
    /// 欄のコマはその部品の animation のコマなので、「部品の欄が空でない」もこの判定に含まれる。
    pub(crate) fn moving_visible(
        &mut self,
        surface_id: u32,
        binds: &BindSet,
        table: &AnimationTable,
        pattern: &PatternState,
    ) -> bool {
        let nest = table.nest_table();
        nest.visible_parts(surface_id, binds, pattern, &mut self.visible);
        if self
            .visible
            .iter()
            .any(|&p| !table.animations(p).is_empty())
        {
            return true;
        }
        nest.visible_films(surface_id, &self.visible, &mut self.films);
        self.films
            .iter()
            .any(|&f| !table.part_animations(PartKey::Film(f)).is_empty())
    }

    /// `scope` の `slot` の面の回数つきの時計を全部捨てる（終わりなしは残す・要件 2.3）。3 語の時計も
    /// 捨て、引き金の状態は隠す（窓が閉じたのと同じ扱い＝開いた時刻が `periodic` の新しい起点・
    /// `runonce` の 1 回は使い済みのまま・spec: areka-P0-seriko-trigger-intervals 要件 5.5）。呼ぶのは
    /// 窓の新しい出番（task 5.1）。
    pub(crate) fn drop_finite(&mut self, scope: &ActorKey, slot: Slot, table: &AnimationTable) {
        if let Some(container) = self.clocks.get_mut(scope).and_then(|c| c.get_mut(&slot)) {
            close(container, table, scope, slot);
        }
    }

    /// 面が隠れた（`\s[-1]`・`\b[-1]`）: `scope` の `slot` の面の回数つきの時計と 3 語の時計を全部捨て、
    /// 引き金の状態も捨てる（面が戻ったら、見えた部品はその時刻から数え直し、`runonce` はもう 1 回
    /// 鳴る・spec: areka-P0-seriko-trigger-intervals 要件 2.6・5.9）。抽選の時計と終わりなしの時計は
    /// 残す。呼ぶのは面が隠れた後の `LoopRuntime::refresh`。
    pub(crate) fn drop_hidden(&mut self, scope: &ActorKey, slot: Slot, table: &AnimationTable) {
        if let Some(container) = self.clocks.get_mut(scope).and_then(|c| c.get_mut(&slot)) {
            drop_transient_where(&mut container.clocks, table, scope, slot, |_| true);
            container.armed.clear();
        }
    }

    /// 全スコープの `slot` の面の時計と、その面の負の番号の `warn!` の記録を捨てる（その面の種類の
    /// 表の差し替え・要件 5.8・spec: areka-P0-animated-image-playback 要件 3.6）。空になったスコープの
    /// 入れ物も残さない。
    pub(crate) fn clear(&mut self, slot: Slot) {
        self.clocks.retain(|_, slots| {
            slots.remove(&slot);
            !slots.is_empty()
        });
        self.warned_negative.retain(|(_, s, _)| *s != slot);
    }

    /// `scope` の `slot` の面で引き金の状態を持つ部品（昇順・テストの観測用）。
    #[cfg(test)]
    pub(crate) fn armed_parts(&self, scope: &ActorKey, slot: Slot) -> Vec<PartKey> {
        self.clocks
            .get(scope)
            .and_then(|slots| slots.get(&slot))
            .map_or(Vec::new(), |c| c.armed.keys().copied().collect())
    }

    /// `scope` のシェルの面の (部品の番号, animation の番号) の時計を引く（テストの観測用）。
    #[cfg(test)]
    pub(crate) fn clock(&self, scope: &ActorKey, part: u32, animation_id: u32) -> Option<PartAnim> {
        self.clock_at(scope, Slot::Shell, PartKey::Surface(part), animation_id)
    }

    /// `scope` の `slot` の面の (部品, animation の番号) の時計を引く（テストの観測用）。
    #[cfg(test)]
    pub(crate) fn clock_at(
        &self,
        scope: &ActorKey,
        slot: Slot,
        part: PartKey,
        animation_id: u32,
    ) -> Option<PartAnim> {
        self.clocks
            .get(scope)?
            .get(&slot)?
            .clocks
            .get(&(part, animation_id))
            .copied()
    }

    /// 最後の `clear` の後（または作ってから）`advance` がどのスコープでも 1 度も走っていないか
    /// （テストの観測用・`advance` はスコープの入れ物を先に作る）。
    #[cfg(test)]
    pub(crate) fn is_empty(&self) -> bool {
        self.clocks.is_empty()
    }
}

#[cfg(test)]
#[path = "parts_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "parts_always_tests.rs"]
mod always_tests;

#[cfg(test)]
#[path = "parts_film_tests.rs"]
pub(crate) mod film_tests;

#[cfg(test)]
#[path = "parts_trigger_tests.rs"]
mod trigger_tests;
