//! 観測の規則: 数え方（1 フレームの判定）。見分け方（標本点・絵と当たり判定の判別）は `signature.rs`。
//!
//! 規則の唯一の定義は design.md §Observer。ここはそれを写しただけで、閾値を変えるなら先に design を直す。
//! World を読む部分（tick の記録・当たり判定の採取）と、観測の窓・集計ログ・較正の合否もここ。
#![allow(dead_code)] // TickRecord の h_p・h_q・h_both（Debug 出力のログでしか読まない）

use std::sync::{Arc, Mutex, MutexGuard};

use bevy_ecs::prelude::*;
use tracing::{debug, error, info, trace};
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Dwm::{DWMWA_CLOAKED, DwmGetWindowAttribute};
use windows::Win32::System::Performance::QueryPerformanceCounter;
use windows::Win32::UI::WindowsAndMessaging::{
    GW_HWNDPREV, GetClassNameW, GetWindow, GetWindowRect, GetWindowTextW, IsWindowVisible,
};
use wintf::ecs::{DPI, FrameCount, WindowHandle, WindowPos};

pub use crate::signature::*;

// ---------------------------------------------------------------------------
// 型
// ---------------------------------------------------------------------------

/// tick の終わりの記録（当たり判定側・UI スレッドが書く）。
#[derive(Clone, Copy, Debug)]
pub struct TickRecord {
    pub tick: u32,
    /// 当たり判定に当てた判別対（`Observer::sigs` の添字）。取り込み側は同じ対で絵を判別する。
    pub pair: usize,
    pub ended_qpc: i64,
    pub rect: RECT,
    pub hit: HitClass,
    pub h_p: f32,
    pub h_q: f32,
    pub h_both: f32,
    pub covered: bool,
    pub k_ok: bool,
    pub slot_hit: bool,
}

/// 取り込んだ 1 フレームの記録（絵側・取り込みスレッドが書く）。
#[derive(Clone, Copy, Debug)]
pub struct FrameRecord {
    pub present_qpc: i64,
    pub accumulated: u32,
    pub tick: Option<u32>,
    pub picture: PictureClass,
    pub a: f32,
    pub b: f32,
    pub ab_p: f32,
    pub ab_q: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub frames: u32,
    pub missed: u32,
    pub unmeasurable: u32,
    pub mixed: u32,
    /// 混在の内訳: 反映待ち（揃う前の 絵＝from ∧ 当たり判定＝to）。
    pub pending: u32,
    pub empty: u32,
    pub stale: u32,
    pub size: u32,
}

/// 較正の項（design §SwapDriver `Calibration`）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Calib {
    Static,
    Empty,
    Mixed,
    Stale,
    Size,
}

/// 観測の種類（最終の並べ出しの順: 較正 → 本番 → 面の切り替え）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Calib(Calib),
    Swap,
    FaceSwitch,
}

/// 観測の閉じ方。
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum End {
    /// 揃って 30 tick 経った。
    Done,
    /// 180 tick で揃わなかった。最後に数えたフレームの（絵, 当たり判定, 矩形）。
    Incomplete(Option<(Class, Class, RECT)>),
    /// 差し替えそのものが失敗した（数えずに閉じた・理由つき）。
    Unmeasurable(&'static str),
}

/// 1 本の観測。
#[derive(Debug)]
pub struct Observation {
    pub name: String,
    pub kind: Kind,
    /// 判別対（`Observer::sigs` の添字）。
    pub pair: usize,
    /// 出発と到達（`Class::P` か `Class::Q`）。
    pub from: Class,
    pub to: Class,
    /// P と Q の原寸（k=1.0 なので物理寸）。
    pub size_p: (u32, u32),
    pub size_q: (u32, u32),
    pub request_tick: u32,
    /// 揃った最初のフレーム（観測の中の通し番号, tick）。
    pub settled: Option<(u32, u32)>,
    pub counts: Counts,
    /// 要求の直前のフレームが無かった。
    pub no_prev: bool,
    /// `None` は開いたまま（打ち切りの並べ出しでは「途中」）。
    pub end: Option<End>,
    /// 猶予で閉じた後に届いた、窓の中の tick と組のフレームの数（数えていない＝取りこぼしと同じく 0 と書かない印）。
    pub late_dropped: u32,
}

impl Observation {
    pub fn new(name: &str, sig: &Signature, from: Class, to: Class, request_tick: u32) -> Self {
        Self {
            name: name.to_string(),
            kind: Kind::Swap,
            pair: PAIR_ASSET,
            from,
            to,
            size_p: sig.size_p,
            size_q: sig.size_q,
            request_tick,
            settled: None,
            counts: Counts::default(),
            no_prev: false,
            end: None,
            late_dropped: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// tick の記録（UI スレッド・FrameFinalize の最後）
// ---------------------------------------------------------------------------

/// `Observer::sigs` の添字: 資産の差し替え (A0, B0)。
pub const PAIR_ASSET: usize = 0;
/// `Observer::sigs` の添字: 面の切り替え (A0, A2)。
pub const PAIR_FACE: usize = 1;

/// 取り込みスレッドと共有する記録。どちらも時刻の昇順に足すだけ。
#[derive(Default)]
pub struct Shared {
    pub ticks: Vec<TickRecord>,
    pub frames: Vec<FrameRecord>,
}

/// 判別対の名前（行の `pair=`・P／Q の読み替え用）。
pub const PAIR_NAMES: [&str; 2] = ["(A0,B0)", "(A0,A2)"];

pub fn lock(m: &Mutex<Shared>) -> MutexGuard<'_, Shared> {
    m.lock().unwrap_or_else(|p| {
        error!("共有の記録を持ったまま別のスレッドが panic した — 記録はそのまま使う");
        p.into_inner()
    })
}

/// tick の記録と観測の窓・集計に要るもの。
#[derive(Resource)]
pub struct Observer {
    pub window: Entity,
    /// 判別対の標本点（`PAIR_ASSET`・`PAIR_FACE`）。取り込みスレッドも同じものを使う。
    pub sigs: Arc<[Signature; 2]>,
    /// 今の tick の当たり判定に当てる対。`open` で観測の対に切り替わる。
    pub active: usize,
    pub shared: Arc<Mutex<Shared>>,
    /// 開いている観測（同時に 1 本だけ）。
    open: Option<Open>,
    /// 閉じた観測（閉じた順）。
    done: Vec<Observation>,
    /// 猶予で閉じた観測の後から届くフレームの見張り。
    late: Option<Late>,
    /// 最終の並べ出しを済ませた（以後は数えない）。
    finished: bool,
}

/// 猶予で閉じた観測（`done[done_idx]`）の窓の最後の tick と、次に読む `Shared::frames` の添字。
#[derive(Clone, Copy, Debug)]
struct Late {
    done_idx: usize,
    last_tick: u32,
    cursor: usize,
}

/// 各 tick の最後に、時刻・窓の矩形・実際の当たり判定・覆い・拡大率を共有の記録へ足す。
///
/// 窓がまだ無い tick と、上限時間で窓を消した後の tick は記録しない。
pub fn tick_record_system(world: &mut World) {
    let world = &*world;
    let Some(obs) = world.get_resource::<Observer>() else {
        return;
    };
    let Some(hwnd) = world.get::<WindowHandle>(obs.window).map(|h| h.hwnd) else {
        return;
    };
    let tick = world.resource::<FrameCount>().0;
    let mut ended_qpc = 0i64;
    // SAFETY: 出力先はこの関数のローカル変数。
    if let Err(e) = unsafe { QueryPerformanceCounter(&mut ended_qpc) } {
        error!(tick, error = %e, "QueryPerformanceCounter が失敗 — この tick を記録しない");
        return;
    }
    let mut actual = RECT::default();
    // SAFETY: WindowHandle が付いている間の窓の HWND。
    if let Err(e) = unsafe { GetWindowRect(hwnd, &mut actual) } {
        error!(tick, error = %e, "GetWindowRect が失敗 — この tick を記録しない");
        return;
    }
    // この tick と組になるフレームが映すのは、この tick の後の flush を済ませた窓（下の QueuedRect）。
    let queued = world.get_resource::<QueuedRect>().and_then(|q| q.0);
    let rect = effective_rect(queued, tick, actual);
    let dpi = world.get::<DPI>(obs.window).copied();
    let k_ok = dpi.is_some_and(|d| d.dpi_x == 96 && d.dpi_y == 96);
    let (hit, h_p, h_q, h_both, slot_hit) = classify_hit(world, obs.window, &obs.sigs[obs.active]);
    let cover = covering_window(hwnd, &rect);
    let rec = TickRecord {
        tick,
        pair: obs.active,
        ended_qpc,
        rect,
        hit,
        h_p,
        h_q,
        h_both,
        covered: cover.is_some(),
        k_ok,
        slot_hit,
    };

    let mut shared = lock(&obs.shared);
    let last = shared.ticks.last().copied();
    // 前の tick に記録した（予測した）矩形は、その tick の flush の後＝今の実際の矩形のはず。
    if last.is_some_and(|l| l.rect != actual) {
        error!(
            tick,
            predicted = ?last.map(|l| l.rect),
            ?actual,
            "前の tick の矩形の予測が実際と違う — その tick のフレームの切り出し・大きさの判定が狂っている"
        );
    }
    if rect != actual {
        debug!(
            tick,
            ?actual,
            ?rect,
            "この tick に積まれた窓の移動を矩形に先取りした"
        );
    }
    if last.is_none_or(|l| l.k_ok != k_ok) {
        info!(tick, k_ok, ?dpi, "拡大率（k_ok=false の tick は測れない）");
    }
    if last.is_none_or(|l| l.covered != rec.covered) {
        match cover {
            Some(h) => debug!(tick, covering = %describe(h), "窓が覆われた（この間は測れない）"),
            None => debug!(tick, "窓の上に覆いは無い"),
        }
    }
    if last.is_some_and(|l| l.tick >= tick || l.ended_qpc > ended_qpc) {
        error!(
            ?last,
            ?rec,
            "tick の記録が昇順でない — 突き合わせの前提が外れた"
        );
    }
    trace!(?rec, "tick の記録");
    shared.ticks.push(rec);
    if shared.ticks.len() % 60 == 1 {
        debug!(
            n = shared.ticks.len(),
            tick,
            active = obs.active,
            ?hit,
            h_p,
            h_q,
            h_both,
            covered = rec.covered,
            k_ok,
            slot_hit,
            ?rect,
            "tick の記録（60 件ごと）"
        );
    }
}

/// この tick に積まれた窓の移動（tick 番号・移動後の窓の矩形）。
///
/// wintf は tick の中では `SetWindowPos` を積むだけで（`apply_window_pos_changes`・UISetup）、実際の
/// 移動は tick の後の `flush_window_pos_commands` で起きる。`FrameFinalize` の `GetWindowRect` は
/// まだ前の寸を返すが、この tick と組になるフレームは flush の後に出るので、移動後の寸を映す。
/// そこで UISetup の `apply_window_pos_changes` の後に、積まれた `WindowPos` を同じ変換で写しておく。
/// `FrameFinalize` で書いた `WindowPos` は次の tick の UISetup で積まれるので、自然に次の tick に付く。
#[derive(Resource, Default)]
pub struct QueuedRect(pub Option<(u32, RECT)>);

/// UISetup で `apply_window_pos_changes` の後に置く（同じ `Changed<WindowPos>` を見る）。
pub fn queued_rect_system(
    obs: Res<Observer>,
    frame: Res<FrameCount>,
    windows: Query<(&WindowHandle, Ref<WindowPos>)>,
    mut queued: ResMut<QueuedRect>,
) {
    let Ok((handle, wp)) = windows.get(obs.window) else {
        return;
    };
    if !wp.is_changed() {
        return;
    }
    // apply_window_pos_changes と同じ変換（失敗時は元の値で積むのも同じ）。
    let (x, y, w, h) = wp.to_window_coords(handle).unwrap_or_else(|e| {
        error!(error = %e, "窓の座標の変換に失敗 — wintf と同じく元の値で予測する");
        let p = wp.position.unwrap_or_default();
        let s = wp.size.unwrap_or_default();
        (p.x, p.y, s.width, s.height)
    });
    let rect = RECT {
        left: x,
        top: y,
        right: x + w,
        bottom: y + h,
    };
    trace!(tick = frame.0, ?rect, "この tick に積まれた窓の移動");
    queued.0 = Some((frame.0, rect));
}

/// tick の記録に書く矩形: この tick に移動が積まれていればその移動後、無ければ今の実際の矩形。
pub fn effective_rect(queued: Option<(u32, RECT)>, tick: u32, actual: RECT) -> RECT {
    match queued {
        Some((t, r)) if t == tick => r,
        _ => actual,
    }
}

/// z 順で自窓より上にあって見えている窓のうち、矩形が自窓と交わる最初のもの。
fn covering_window(me: HWND, rect: &RECT) -> Option<HWND> {
    let mut cur = me;
    loop {
        // SAFETY: 窓の列挙だけ。尽きたら Err で抜ける。
        cur = unsafe { GetWindow(cur, GW_HWNDPREV) }.ok()?;
        // SAFETY: 列挙で得た HWND（消えていれば各呼び出しが失敗するだけ）。
        if !unsafe { IsWindowVisible(cur) }.as_bool() || is_cloaked(cur) {
            continue;
        }
        let mut r = RECT::default();
        if unsafe { GetWindowRect(cur, &mut r) }.is_err() {
            continue;
        }
        if r.left.max(rect.left) < r.right.min(rect.right)
            && r.top.max(rect.top) < r.bottom.min(rect.bottom)
        {
            return Some(cur);
        }
    }
}

/// DWM が隠している（可視の印はあるが画面に出ない）窓か。
fn is_cloaked(hwnd: HWND) -> bool {
    let mut cloaked = 0u32;
    // SAFETY: 出力先は u32 1 つ（DWMWA_CLOAKED の型）。
    unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            &mut cloaked as *mut u32 as *mut _,
            size_of::<u32>() as u32,
        )
    }
    .is_ok_and(|_| cloaked != 0)
}

/// ログ用: 窓のクラス名と題名。
fn describe(hwnd: HWND) -> String {
    let (mut class, mut title) = ([0u16; 256], [0u16; 256]);
    // SAFETY: 書き込み先は固定長のローカル配列。
    let (c, t) = unsafe {
        (
            GetClassNameW(hwnd, &mut class),
            GetWindowTextW(hwnd, &mut title),
        )
    };
    format!(
        "{hwnd:?} class={:?} title={:?}",
        String::from_utf16_lossy(&class[..c.max(0) as usize]),
        String::from_utf16_lossy(&title[..t.max(0) as usize])
    )
}

// ---------------------------------------------------------------------------
// 1 フレームの判定
// ---------------------------------------------------------------------------

/// 1 フレームを観測に数える（4 種は重複計上）。`tick` は `frame` と組になった tick の記録。
pub fn judge_frame(obs: &mut Observation, frame: &FrameRecord, tick: &TickRecord) {
    let idx = obs.counts.frames;
    obs.counts.frames += 1;
    obs.counts.missed += frame.accumulated.saturating_sub(1);

    let why = match (frame.picture, tick.hit) {
        (Class::Unmeasurable(w), _) | (_, Class::Unmeasurable(w)) => Some(w),
        _ if tick.covered => Some(Why::Covered),
        _ if !tick.k_ok => Some(Why::ScaleNotOne),
        _ if tick.slot_hit => Some(Why::SlotHit),
        _ if frame.accumulated > 1 => Some(Why::Dropped),
        _ => None,
    };
    if let Some(why) = why {
        obs.counts.unmeasurable += 1;
        debug!(name = %obs.name, idx, ?why, "測れないフレーム");
        return;
    }

    let (pic, hit) = (frame.picture, tick.hit);
    let (from, to) = (obs.from, obs.to);
    let moving = from != to;
    let before = obs.settled.is_none();

    if before {
        // 揃った: 要求より後のフレームで 絵＝to ∧ 当たり判定＝to（大きさは条件に入れない）。
        if tick.tick >= obs.request_tick && pic == to && hit == to {
            obs.settled = Some((idx, tick.tick));
        }
    } else if moving && (pic == from || pic == Class::Both) {
        obs.counts.stale += 1;
    }

    let dt = frame.present_qpc - tick.ended_qpc;
    if pic != hit || pic == Class::Both {
        obs.counts.mixed += 1;
        debug!(name = %obs.name, idx, ?pic, ?hit, dt, "混在");
    }
    if moving && before && pic == from && hit == to {
        obs.counts.pending += 1;
    }
    if moving && pic == to && hit == from {
        error!(name = %obs.name, idx, ?pic, ?hit, dt, "逆向きの組（絵＝到達先・当たり判定＝出発）— 突き合わせの前提が外れた");
    }
    if pic == Class::Neither {
        obs.counts.empty += 1;
    }
    let native = match pic {
        Class::P => Some(obs.size_p),
        Class::Q => Some(obs.size_q),
        _ => None,
    };
    if native.is_some_and(|s| s != rect_size(&tick.rect)) {
        obs.counts.size += 1;
        debug!(name = %obs.name, idx, ?pic, ?hit, tick = tick.tick, rect = ?rect_size(&tick.rect), ?native, dt, "大きさの食い違い");
    }
}

// ---------------------------------------------------------------------------
// 観測の窓（design Key Decision 8・§Observer 観測の窓）
// ---------------------------------------------------------------------------

/// 揃った tick の後に数え続ける tick 数。
pub const SETTLE_TICKS: u32 = 30;
/// 要求から揃うまで待つ tick 数（超えたら「未完」）。
pub const GIVE_UP_TICKS: u32 = 180;
/// 窓の最後の tick より後のフレームが来ない（画面が変わらない）ときに閉じるまでの猶予の tick 数。
///
/// フレームは取り込みの遅れ（`AcquireNextFrame` の待ち 16 ms＋判別）だけ後から共有の記録に届く。
/// 窓の外のフレームが 1 枚来れば猶予を待たずに閉じる。実測で約 120 tick/s なので 12 tick ≒ 100 ms。
pub const GRACE_TICKS: u32 = 12;

/// 開いている観測と、共有の記録のどこまで数えたか。
#[derive(Debug)]
pub struct Open {
    obs: Observation,
    /// 次に読む `Shared::frames` の添字。
    cursor: usize,
    /// 要求より前の tick と組になった最後のフレーム（窓の先頭に含める）。
    prev: Option<(FrameRecord, TickRecord)>,
    /// 窓の先頭（直前のフレーム）を数え終えた。
    started: bool,
    /// 窓の最後の tick より後のフレームが来た（それまでのフレームは届き切っている）。
    beyond: bool,
    /// 最後に数えたフレームの（絵, 当たり判定, 矩形）。
    last: Option<(Class, Class, RECT)>,
}

impl Open {
    /// 開いた時点で最後に取り込まれたフレームから読む（要求より前の tick なら直前のフレームの候補。
    /// 後から遅れて届いた要求より前のフレームがあれば、それに置き換わる）。
    pub fn new(obs: Observation, s: &Shared) -> Self {
        Self {
            obs,
            cursor: s.frames.len().saturating_sub(1),
            prev: None,
            started: false,
            beyond: false,
            last: None,
        }
    }

    /// 窓の最後の tick（揃ったら揃った tick＋30、揃う前は要求＋180）。
    fn end_tick(&self) -> u32 {
        match self.obs.settled {
            Some((_, t)) => t + SETTLE_TICKS,
            None => self.obs.request_tick + GIVE_UP_TICKS,
        }
    }

    /// 届いたフレームを数え、閉じてよければ `true`。`now` は今の tick。
    pub fn step(&mut self, s: &Shared, now: u32) -> bool {
        while !self.beyond && self.cursor < s.frames.len() {
            let f = s.frames[self.cursor];
            let t = f.tick.and_then(|n| {
                s.ticks
                    .binary_search_by_key(&n, |t| t.tick)
                    .ok()
                    .map(|i| s.ticks[i])
            });
            let Some(t) = t else {
                error!(name = %self.obs.name, ?f, "フレームと組の tick の記録が無い — 数えない");
                self.cursor += 1;
                continue;
            };
            if t.tick < self.obs.request_tick {
                self.prev = Some((f, t));
            } else if t.tick > self.end_tick() {
                self.beyond = true;
                break;
            } else {
                self.start();
                self.judge(f, t);
            }
            self.cursor += 1;
        }
        self.beyond || now >= self.end_tick() + GRACE_TICKS
    }

    /// 窓の先頭として直前のフレームを数える（1 回だけ）。
    fn start(&mut self) {
        if std::mem::replace(&mut self.started, true) {
            return;
        }
        match self.prev.take() {
            Some((f, t)) => self.judge(f, t),
            None => {
                self.obs.no_prev = true;
                info!(name = %self.obs.name, "直前のフレーム無し");
            }
        }
    }

    fn judge(&mut self, mut f: FrameRecord, t: TickRecord) {
        if t.pair != self.obs.pair {
            f.picture = Class::Unmeasurable(Why::OtherPair);
        }
        judge_frame(&mut self.obs, &f, &t);
        self.last = Some((f.picture, t.hit, t.rect));
    }

    /// 閉じる（揃っていれば「完了」、揃っていなければ「未完」）。集計の 1 行を出す。
    pub fn close(mut self) -> Observation {
        self.start();
        self.obs.end = Some(match self.obs.settled {
            Some(_) => End::Done,
            None => End::Incomplete(self.last),
        });
        info!("観測: {}", row(&self.obs));
        self.obs
    }
}

impl Observer {
    pub fn new(window: Entity, sigs: Arc<[Signature; 2]>, shared: Arc<Mutex<Shared>>) -> Self {
        Self {
            window,
            sigs,
            active: PAIR_ASSET,
            shared,
            open: None,
            done: Vec::new(),
            late: None,
            finished: false,
        }
    }

    /// 要求 tick で観測を開く。当たり判定はこの tick の記録から `pair` で判別する
    /// （tick の記録より前の段で呼ぶこと）。前の観測が開いたままなら、それを閉じてから開く。
    pub fn open(
        &mut self,
        pair: usize,
        kind: Kind,
        name: &str,
        from: Class,
        to: Class,
        request_tick: u32,
    ) {
        self.scan_late(&lock(&self.shared.clone()));
        if self.open.is_some() {
            error!(next = name, "前の観測が開いたまま次を開く — 前のを今閉じる");
            self.close_open();
        }
        let obs = Observation {
            kind,
            pair,
            ..Observation::new(name, &self.sigs[pair], from, to, request_tick)
        };
        self.active = pair;
        self.open = Some(Open::new(obs, &lock(&self.shared)));
        info!(name, pair = PAIR_NAMES[pair], request_tick, "観測を開いた");
    }

    /// 差し替えの失敗: 開いている観測を数えずに「測れない」で閉じる（design §Error Handling）。
    pub fn abort(&mut self, why: &'static str) {
        let Some(o) = self.open.take() else {
            error!(why, "閉じる観測が開いていない");
            return;
        };
        let mut obs = o.obs;
        obs.end = Some(End::Unmeasurable(why));
        info!("観測: {}", row(&obs));
        self.done.push(obs);
    }

    pub fn is_closed(&self) -> bool {
        self.open.is_none()
    }

    /// 開いている観測が揃った tick（揃う前・閉じた後は `None`）。
    pub fn settled_tick(&self) -> Option<u32> {
        self.open.as_ref()?.obs.settled.map(|(_, t)| t)
    }

    /// 届いたフレームを開いている観測に数え、窓が終わっていれば閉じる。
    pub fn step(&mut self, now: u32) {
        if self.finished {
            return;
        }
        let close = {
            let shared = self.shared.clone();
            let s = lock(&shared);
            self.scan_late(&s);
            self.open.as_mut().is_some_and(|o| o.step(&s, now))
        };
        if close {
            self.close_open();
        }
    }

    /// 開いている観測を閉じる。窓の外のフレームを見ずに（猶予で）閉じたなら、後から届く窓の中の
    /// フレームを見張る（窓の外のフレームで閉じたなら、それより前のフレームは届き切っている）。
    fn close_open(&mut self) {
        let Some(o) = self.open.take() else { return };
        let late = (!o.beyond).then(|| Late {
            done_idx: self.done.len(),
            last_tick: o.end_tick(),
            cursor: o.cursor,
        });
        self.done.push(o.close());
        if late.is_some() {
            self.late = late;
        }
    }

    /// 猶予で閉じた観測の後に届いた、窓の中の tick と組のフレームを数えて `error!` に出す。
    fn scan_late(&mut self, s: &Shared) {
        let Some(mut l) = self.late.take() else {
            return;
        };
        while let Some(f) = s.frames.get(l.cursor) {
            if f.tick.is_some_and(|t| t > l.last_tick) {
                return; // 窓の外のフレームが来た: 窓の中のフレームはもう来ない
            }
            let o = &mut self.done[l.done_idx];
            o.late_dropped += 1;
            error!(
                name = %o.name,
                tick = ?f.tick,
                late_dropped = o.late_dropped,
                "窓の中のフレームが閉じた後に届いた — 数えていない"
            );
            l.cursor += 1;
        }
        self.late = Some(l);
    }

    /// 最終の並べ出し（4.8・6.6）。`partial` は上限時間での打ち切り（開いている観測は「途中」で並べる）。
    pub fn summarize(&mut self, partial: bool, now: u32) -> Verdict {
        self.finished = true;
        let (ticks, frames) = {
            let shared = self.shared.clone();
            let s = lock(&shared);
            self.scan_late(&s);
            if let Some(o) = self.open.as_mut() {
                o.step(&s, now);
            }
            (s.ticks.len(), s.frames.len())
        };
        let open = self.open.take().map(|mut o| {
            o.start();
            o.obs
        });
        let verdict = calibration_verdict(&self.done);
        if let Verdict::Failed(names) = &verdict {
            error!(failed = ?names, "本番の数は無効（較正が期待と違う）");
        }
        if partial {
            info!(
                closed = self.done.len(),
                open = open.as_ref().map_or("無し", |o| o.name.as_str()),
                "打ち切り: 上限時間までに閉じた観測と、開いていた観測（途中）を並べる"
            );
        }
        let group = |k: Kind| match k {
            Kind::Calib(_) => 0,
            Kind::Swap => 1,
            Kind::FaceSwitch => 2,
        };
        for (g, label) in ["較正", "本番", "面の切り替え"].into_iter().enumerate() {
            let rows: Vec<String> = self
                .done
                .iter()
                .chain(open.as_ref())
                .filter(|o| group(o.kind) == g)
                .map(row)
                .collect();
            info!("{label} {} 行", rows.len());
            for r in rows {
                info!("  {r}");
            }
        }
        match floor(&self.done) {
            Some(f) => info!(
                floor = f,
                "反映待ちの床（面の切り替えの pending の大きい方）"
            ),
            None => info!("反映待ちの床: 閉じた面の切り替えの観測が無い — 床は無し"),
        }
        let calib_run = self
            .done
            .iter()
            .filter(|o| matches!(o.kind, Kind::Calib(_)))
            .count();
        match &verdict {
            Verdict::Passed => info!(
                calib_run,
                "較正: 合格（閉じた較正の項の数。0 項も合格とする）"
            ),
            Verdict::Failed(names) => error!(calib_run, failed = ?names, "較正: 不合格"),
        }
        info!(ticks, frames, "tick とフレームの記録の件数");
        verdict
    }
}

/// 各 tick の最後（tick の記録の後）に、届いたフレームを観測に数える。
pub fn observe_system(mut observer: ResMut<Observer>, frame: Res<FrameCount>) {
    observer.step(frame.0);
}

// ---------------------------------------------------------------------------
// 較正の合否・床・集計の行
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    Passed,
    /// 期待と違った較正の観測の名前。
    Failed(Vec<String>),
}

/// 閉じた較正の観測から合否を決める（design §Observer 較正の合否）。較正が 0 項なら合格。
pub fn calibration_verdict(done: &[Observation]) -> Verdict {
    let failed: Vec<String> = done
        .iter()
        .filter(|o| {
            let c = o.counts;
            match o.kind {
                Kind::Calib(Calib::Static) => {
                    c.frames == 0 || c.mixed + c.empty + c.stale + c.size != 0
                }
                Kind::Calib(Calib::Empty) => c.empty == 0,
                Kind::Calib(Calib::Mixed) => c.mixed == 0,
                Kind::Calib(Calib::Stale) => c.stale == 0,
                Kind::Calib(Calib::Size) => c.size == 0,
                Kind::Swap | Kind::FaceSwitch => false,
            }
        })
        .map(|o| o.name.clone())
        .collect();
    if failed.is_empty() {
        Verdict::Passed
    } else {
        Verdict::Failed(failed)
    }
}

/// 反映待ちの床: 閉じた面の切り替えの観測の `pending` の大きい方。無ければ `None`。
pub fn floor(done: &[Observation]) -> Option<u32> {
    done.iter()
        .filter(|o| o.kind == Kind::FaceSwitch)
        .map(|o| o.counts.pending)
        .max()
}

/// 集計の 1 行（観測を閉じたときと最終の並べ出しで同じ形）。0 も書く。
pub fn row(o: &Observation) -> String {
    let c = o.counts;
    let end = match o.end {
        None => "途中".to_string(),
        Some(End::Done) => "完了".to_string(),
        Some(End::Incomplete(None)) => "未完（フレーム無し）".to_string(),
        Some(End::Unmeasurable(why)) => format!("測れない（{why}）"),
        Some(End::Incomplete(Some((p, h, r)))) => {
            let (w, hh) = rect_size(&r);
            format!(
                "未完（最後: 絵={p:?}・当たり判定={h:?}・矩形={w}x{hh}@({},{})）",
                r.left, r.top
            )
        }
    };
    let (sf, st) = match o.settled {
        Some((i, t)) => (i.to_string(), t.saturating_sub(o.request_tick).to_string()),
        None => ("-".to_string(), "-".to_string()),
    };
    format!(
        "{} pair={} {end} frames={} missed={} unmeasurable={} mixed={} pending={} empty={} \
         stale={} size={} late_dropped={} settled_after_frames={sf} settled_after_ticks={st}{}",
        o.name,
        PAIR_NAMES[o.pair],
        c.frames,
        c.missed,
        c.unmeasurable,
        c.mixed,
        c.pending,
        c.empty,
        c.stale,
        c.size,
        o.late_dropped,
        if o.no_prev {
            " 直前のフレーム無し"
        } else {
            ""
        }
    )
}

#[cfg(test)]
#[path = "observe_tests.rs"]
mod tests;
