//! 観測の規則: 見分け方（標本点・絵と当たり判定の判別）と数え方（1 フレームの判定）。
//!
//! 規則の唯一の定義は design.md §Observer。ここはそれを写しただけで、閾値を変えるなら先に design を直す。
//! World を読む部分（tick の記録・当たり判定の採取）と、観測の窓・集計ログ・較正の合否もここ。
#![allow(dead_code)] // Kind::FaceSwitch・Calib の他の項などは 3.x の台本が使う

use std::sync::{Arc, Mutex, MutexGuard};

use areka_emo_compose::ComposedSurface;
use bevy_ecs::name::Name;
use bevy_ecs::prelude::*;
use tracing::{debug, error, info, trace};
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Dwm::{DWMWA_CLOAKED, DwmGetWindowAttribute};
use windows::Win32::System::Performance::QueryPerformanceCounter;
use windows::Win32::UI::WindowsAndMessaging::{
    GW_HWNDPREV, GetClassNameW, GetWindow, GetWindowRect, GetWindowTextW, IsWindowVisible,
};
use wintf::ecs::{DPI, FrameCount, PointF, WindowHandle, WindowPos, hit_test_in_window};

/// 共通範囲を切る格子の升数（縦横とも・実装 2.2 で 8 から 16 へ改訂＝design §Observer）。
const GRID: u32 = 16;
/// 「不透明」の α の下限。
const OPAQUE: u8 = 128;
/// 「両方」の点の色差（各チャネルの差の最大）の下限。
const COLOR_DIFF: u8 = 48;
/// 取り込み画素と合成色の一致の許容（各チャネル）。
const TOLERANCE: u8 = 12;
/// 集合が使える点の数の下限。
const MIN_POINTS: usize = 16;
const HI: f32 = 0.9;
const MID: f32 = 0.5;
const LO: f32 = 0.1;

type Bgr = [u8; 3];

// ---------------------------------------------------------------------------
// 型
// ---------------------------------------------------------------------------

/// 絵・当たり判定の判別結果（design の `PictureClass`／`HitClass` は同じ値の集合なので 1 つにした）。
///
/// 観測の `from`／`to` も `P`／`Q` で持つので、`絵 == to` のような比較がそのまま書ける。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    P,
    Q,
    Both,
    Neither,
    Unmeasurable(Why),
}
pub type PictureClass = Class;
pub type HitClass = Class;

/// 測れない理由。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Why {
    Ambiguous,
    Covered,
    ScaleNotOne,
    SlotHit,
    Dropped,
    CaptureLost,
    OutsideOutput,
    /// 観測と違う判別対で判別したフレーム（対を切り替える前の tick の直前のフレーム）。
    OtherPair,
}

/// 判別対 (P, Q) の標本点。座標は窓（＝面）の左上からの物理 px。色は α=255 の点だけなので色そのもの。
#[derive(Debug)]
pub struct Signature {
    pub only_p: Vec<(u32, u32, Bgr)>,
    pub only_q: Vec<(u32, u32, Bgr)>,
    /// (x, y, P の色, Q の色)
    pub both: Vec<(u32, u32, Bgr, Bgr)>,
    pub size_p: (u32, u32),
    pub size_q: (u32, u32),
}

impl Signature {
    /// 「P だけ」を使うか（16 点未満なら無しとして扱う。点そのものはログ用に残す）。
    pub fn has_only_p(&self) -> bool {
        self.only_p.len() >= MIN_POINTS
    }
}

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
// 標本点の導出
// ---------------------------------------------------------------------------

/// premultiplied BGRA の画像の見え方（`ComposedSurface` は画素を書く口が crate 内限りなので、検査はこれで作る）。
struct Img<'a> {
    w: u32,
    h: u32,
    stride: usize,
    bytes: &'a [u8],
}

impl<'a> Img<'a> {
    fn of(s: &'a ComposedSurface) -> Self {
        Self {
            w: s.width(),
            h: s.height(),
            stride: s.stride() as usize,
            bytes: s.bytes(),
        }
    }

    fn px(&self, x: u32, y: u32) -> [u8; 4] {
        let i = y as usize * self.stride + x as usize * 4;
        [
            self.bytes[i],
            self.bytes[i + 1],
            self.bytes[i + 2],
            self.bytes[i + 3],
        ]
    }
}

fn bgr(px: [u8; 4]) -> Bgr {
    [px[0], px[1], px[2]]
}

fn color_diff(a: Bgr, b: Bgr) -> u8 {
    (0..3).map(|i| a[i].abs_diff(b[i])).max().unwrap_or(0)
}

/// 判別対 (P, Q) の標本点を導出する。見分けられない対は `Err`。
pub fn derive_signature(p: &ComposedSurface, q: &ComposedSurface) -> Result<Signature, String> {
    derive(&Img::of(p), &Img::of(q))
}

fn derive(p: &Img, q: &Img) -> Result<Signature, String> {
    let (cw, ch) = (p.w.min(q.w), p.h.min(q.h));
    let mut sig = Signature {
        only_p: Vec::new(),
        only_q: Vec::new(),
        both: Vec::new(),
        size_p: (p.w, p.h),
        size_q: (q.w, q.h),
    };
    for gy in 0..GRID {
        for gx in 0..GRID {
            let (mut op, mut oq, mut bo) = (None, None, None);
            for y in gy * ch / GRID..(gy + 1) * ch / GRID {
                for x in gx * cw / GRID..(gx + 1) * cw / GRID {
                    let (a, b) = (p.px(x, y), q.px(x, y));
                    if op.is_none() && a[3] == 255 && b[3] < OPAQUE {
                        op = Some((x, y, bgr(a)));
                    }
                    if oq.is_none() && b[3] == 255 && a[3] < OPAQUE {
                        oq = Some((x, y, bgr(b)));
                    }
                    if bo.is_none()
                        && a[3] == 255
                        && b[3] == 255
                        && color_diff(bgr(a), bgr(b)) >= COLOR_DIFF
                    {
                        bo = Some((x, y, bgr(a), bgr(b)));
                    }
                }
            }
            sig.only_p.extend(op);
            sig.only_q.extend(oq);
            sig.both.extend(bo);
        }
    }
    if sig.only_q.len() < MIN_POINTS || sig.both.len() < MIN_POINTS {
        return Err(format!(
            "見分けられない対（P だけ {}・Q だけ {}・両方 {}。Q だけと両方が各 {MIN_POINTS} 点以上要る）",
            sig.only_p.len(),
            sig.only_q.len(),
            sig.both.len()
        ));
    }
    Ok(sig)
}

// ---------------------------------------------------------------------------
// 絵と当たり判定の判別
// ---------------------------------------------------------------------------

/// 一致した割合。分母（範囲内の点）が 0 なら NaN（どの閾値も満たさず「測れない」へ落ちる）。
fn ratio(results: impl Iterator<Item = Option<bool>>) -> f32 {
    let (mut n, mut hit) = (0u32, 0u32);
    for r in results.flatten() {
        n += 1;
        hit += r as u32;
    }
    if n == 0 {
        f32::NAN
    } else {
        hit as f32 / n as f32
    }
}

fn near(px: Bgr, c: Bgr) -> bool {
    color_diff(px, c) <= TOLERANCE
}

/// 窓の矩形を切り出した画素（BGRA・`stride` バイト／行）から絵を判別する。→ (class, a, b, ab_p, ab_q)
///
/// 矩形の外に出た点は割合の分母から外す。
pub fn classify_picture(
    sig: &Signature,
    rect: &RECT,
    pixels: &[u8],
    stride: usize,
) -> (PictureClass, f32, f32, f32, f32) {
    let (w, h) = rect_size(rect);
    let at = |x: u32, y: u32| -> Option<Bgr> {
        if x >= w || y >= h {
            return None;
        }
        let i = y as usize * stride + x as usize * 4;
        pixels.get(i..i + 3).map(|s| [s[0], s[1], s[2]])
    };
    let check = |x: u32, y: u32, c: Bgr| {
        at(x, y).map(|px| {
            let m = near(px, c);
            trace!(x, y, ?px, ?c, m, "標本点の色");
            m
        })
    };
    let a = ratio(sig.only_p.iter().map(|&(x, y, c)| check(x, y, c)));
    let b = ratio(sig.only_q.iter().map(|&(x, y, c)| check(x, y, c)));
    let ab_p = ratio(sig.both.iter().map(|&(x, y, c, _)| check(x, y, c)));
    let ab_q = ratio(sig.both.iter().map(|&(x, y, _, c)| check(x, y, c)));
    (
        picture_rule(sig.has_only_p(), a, b, ab_p, ab_q),
        a,
        b,
        ab_p,
        ab_q,
    )
}

/// 割合 → 絵の判別（design §Observer 絵の判別）。
fn picture_rule(has_only_p: bool, a: f32, b: f32, ab_p: f32, ab_q: f32) -> PictureClass {
    let neither = b <= LO && ab_p <= LO && ab_q <= LO;
    if has_only_p {
        if a >= HI && ab_p >= HI && b <= LO {
            Class::P
        } else if b >= HI && ab_q >= HI && a <= LO {
            Class::Q
        } else if a >= MID && b >= MID {
            Class::Both
        } else if a <= LO && neither {
            Class::Neither
        } else {
            Class::Unmeasurable(Why::Ambiguous)
        }
    } else if ab_p >= HI && b <= LO {
        Class::P
    } else if b >= HI && ab_q >= HI {
        Class::Q
    } else if b >= MID && ab_p >= MID {
        Class::Both
    } else if neither {
        Class::Neither
    } else {
        Class::Unmeasurable(Why::Ambiguous)
    }
}

/// 当たりの割合 → 当たり判定の判別（design §Observer 当たり判定の判別）。
///
/// h_p＝「P だけ」、h_q＝「Q だけ」、h_both＝「両方」の各集合で当たった割合。
/// 「P だけ」が無い対では `Both` は Q と区別できないので出ない。
pub fn hit_rule(has_only_p: bool, h_p: f32, h_q: f32, h_both: f32) -> HitClass {
    if has_only_p {
        if h_p >= HI && h_q <= LO {
            Class::P
        } else if h_q >= HI && h_p <= LO {
            Class::Q
        } else if h_p >= HI && h_q >= HI {
            Class::Both
        } else if h_p <= LO && h_q <= LO {
            Class::Neither
        } else {
            Class::Unmeasurable(Why::Ambiguous)
        }
    } else if h_q <= LO && h_both >= HI {
        Class::P
    } else if h_q >= HI && h_both >= HI {
        Class::Q
    } else if h_q <= LO && h_both <= LO {
        Class::Neither
    } else {
        Class::Unmeasurable(Why::Ambiguous)
    }
}

/// 文字層の子の名前（`areka-emo-present` の mount が付ける）。
const TEXT_SLOT: &str = "emo-text-layer-slot";

/// 3 集合の各点で実際の当たり判定を引く。→ (class, h_p, h_q, h_both, 文字層の子が当たったか)
///
/// 点は面の左上からの物理 px。k=1.0 では窓のクライアント座標と 1:1（窓の原点＝面の原点）なので、
/// 画素の中心をそのまま `hit_test_in_window` に渡す。
pub fn classify_hit(
    world: &World,
    window: Entity,
    sig: &Signature,
) -> (HitClass, f32, f32, f32, bool) {
    let mut slot_hit = false;
    let mut probe = |x: u32, y: u32| {
        let e = hit_test_in_window(world, window, PointF::new(x as f32 + 0.5, y as f32 + 0.5));
        trace!(x, y, ?e, "標本点の当たり");
        slot_hit |= e
            .and_then(|e| world.get::<Name>(e))
            .is_some_and(|n| n.as_str() == TEXT_SLOT);
        Some(e.is_some())
    };
    let h_p = ratio(sig.only_p.iter().map(|&(x, y, _)| probe(x, y)));
    let h_q = ratio(sig.only_q.iter().map(|&(x, y, _)| probe(x, y)));
    let h_both = ratio(sig.both.iter().map(|&(x, y, _, _)| probe(x, y)));
    (
        hit_rule(sig.has_only_p(), h_p, h_q, h_both),
        h_p,
        h_q,
        h_both,
        slot_hit,
    )
}

pub fn rect_size(r: &RECT) -> (u32, u32) {
    (
        (r.right - r.left).max(0) as u32,
        (r.bottom - r.top).max(0) as u32,
    )
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
mod tests {
    use super::*;
    use Class::*;

    const SP: (u32, u32) = (335, 205);
    const SQ: (u32, u32) = (400, 224);

    fn sig() -> Signature {
        Signature {
            only_p: vec![],
            only_q: vec![],
            both: vec![],
            size_p: SP,
            size_q: SQ,
        }
    }

    /// 合成した (絵, 当たり判定, 矩形) の 1 フレームを数える。
    fn feed(obs: &mut Observation, pic: Class, hit: Class, tick: u32, size: (u32, u32)) {
        feed_with(obs, pic, hit, tick, size, 1, |_| {});
    }

    /// `tweak` で tick の記録に測れない印（覆い・拡大率・文字層）を立てる。
    fn feed_with(
        obs: &mut Observation,
        pic: Class,
        hit: Class,
        tick: u32,
        (w, h): (u32, u32),
        accumulated: u32,
        tweak: fn(&mut TickRecord),
    ) {
        let mut t = TickRecord {
            tick,
            pair: PAIR_ASSET,
            ended_qpc: tick as i64 * 100,
            rect: RECT {
                left: 10,
                top: 20,
                right: 10 + w as i32,
                bottom: 20 + h as i32,
            },
            hit,
            h_p: 0.0,
            h_q: 0.0,
            h_both: 0.0,
            covered: false,
            k_ok: true,
            slot_hit: false,
        };
        tweak(&mut t);
        let f = FrameRecord {
            present_qpc: tick as i64 * 100 + 50,
            accumulated,
            tick: Some(tick),
            picture: pic,
            a: 0.0,
            b: 0.0,
            ab_p: 0.0,
            ab_q: 0.0,
        };
        judge_frame(obs, &f, &t);
    }

    #[test]
    fn judge_frame_counts_by_the_rules() {
        let mut o = Observation::new("t P→Q", &sig(), P, Q, 10);
        feed(&mut o, Q, Q, 9, SQ); // 0: 要求より前 → 揃ったにならない
        feed(&mut o, P, Q, 10, SP); // 1: 反映待ち（混在の内訳）
        feed(&mut o, Unmeasurable(Why::Ambiguous), Q, 10, SQ); // 2: 測れない
        feed(&mut o, Q, P, 10, SQ); // 3: 逆向き → 混在・反映待ちではない
        feed(&mut o, Neither, Neither, 10, SP); // 4: 空（混在ではない）
        feed(&mut o, Both, Q, 10, SQ); // 5: 混在・揃う前なので残りではない
        feed(&mut o, Q, Q, 11, SQ); // 6: 揃った
        feed(&mut o, P, Q, 12, SP); // 7: 混在・残り・揃った後なので反映待ちではない
        feed(&mut o, Both, Both, 13, SQ); // 8: 「両方」は混在・残り
        feed(&mut o, Q, Q, 14, SP); // 9: 大きさの食い違い
        feed_with(&mut o, P, P, 15, SP, 1, |t| t.covered = true); // 10: 覆い → 測れない
        feed_with(&mut o, P, Q, 16, SP, 3, |_| {}); // 11: 取りこぼし 2 → 測れない
        assert_eq!(o.settled, Some((6, 11)));
        assert_eq!(
            o.counts,
            Counts {
                frames: 12,
                missed: 2,
                unmeasurable: 3,
                mixed: 5,
                pending: 1,
                empty: 1,
                stale: 2,
                size: 1,
            }
        );

        // from == to（静止）では古い絵の残りを数えない。
        let mut s = Observation::new("t P→P", &sig(), P, P, 1);
        feed(&mut s, P, P, 1, SP);
        feed(&mut s, P, P, 2, SP);
        assert_eq!(s.settled, Some((0, 1)));
        assert_eq!(
            s.counts,
            Counts {
                frames: 2,
                ..Counts::default()
            }
        );
    }

    /// w×h の premultiplied BGRA（x が `xs` の範囲なら `color`・α=255、他は透明）。
    fn image(w: u32, h: u32, xs: std::ops::Range<u32>, color: Bgr) -> Vec<u8> {
        let mut v = vec![0u8; (w * h * 4) as usize];
        for y in 0..h {
            for x in xs.clone() {
                let i = ((y * w + x) * 4) as usize;
                v[i..i + 4].copy_from_slice(&[color[0], color[1], color[2], 255]);
            }
        }
        v
    }

    #[test]
    fn signature_and_picture_rule_on_tiny_images() {
        let (red, blue) = ([0, 0, 255], [255, 0, 0]);
        fn img(bytes: &[u8]) -> Img<'_> {
            Img {
                w: 64,
                h: 64,
                stride: 256,
                bytes,
            }
        }
        let (p, q) = (image(64, 64, 0..48, red), image(64, 64, 16..64, blue));
        // 升 16 列（1 升 4 px）: 0–3 列は P だけ・4–11 列は両方・12–15 列は Q だけ（各列 16 升）。
        let sig = derive(&img(&p), &img(&q)).unwrap();
        assert_eq!(
            (sig.only_p.len(), sig.only_q.len(), sig.both.len()),
            (64, 64, 128)
        );

        let rect = RECT {
            left: 0,
            top: 0,
            right: 64,
            bottom: 64,
        };
        assert_eq!(classify_picture(&sig, &rect, &p, 256).0, P);
        assert_eq!(classify_picture(&sig, &rect, &q, 256).0, Q);
        assert_eq!(
            classify_picture(&sig, &rect, &vec![0u8; 64 * 64 * 4], 256).0,
            Neither
        );
        let mut over = q.clone(); // P を Q の上に描いた絵
        over[..].chunks_mut(4).zip(p.chunks(4)).for_each(|(o, s)| {
            if s[3] == 255 {
                o.copy_from_slice(s)
            }
        });
        assert_eq!(classify_picture(&sig, &rect, &over, 256).0, Both);

        // 同じ絵どうしは見分けられない。
        assert!(derive(&img(&p), &img(&p)).is_err());

        // 当たり判定の規則（「P だけ」が無い対では Both が出ない）。
        assert_eq!(hit_rule(true, 1.0, 1.0, 1.0), Both);
        assert_eq!(hit_rule(false, f32::NAN, 1.0, 1.0), Q);
        assert_eq!(hit_rule(false, f32::NAN, 0.0, 1.0), P);
    }

    /// 拡大率が 1 でない・文字層の子が当たった tick は、混在や空の組でも 4 種に入らず測れないに数える。
    #[test]
    fn scale_and_slot_hit_are_unmeasurable() {
        let tweaks: [fn(&mut TickRecord); 2] = [|t| t.k_ok = false, |t| t.slot_hit = true];
        for tweak in tweaks {
            let mut o = Observation::new("t P→Q", &sig(), P, Q, 0);
            feed_with(&mut o, P, Q, 1, SP, 1, tweak);
            feed_with(&mut o, Neither, Neither, 2, SP, 1, tweak);
            assert_eq!(
                o.counts,
                Counts {
                    frames: 2,
                    unmeasurable: 2,
                    ..Counts::default()
                }
            );
        }
    }

    /// 「P だけ」が無い対の絵の「両方」は b と ab_p で決まる（ab_q ではない）。
    #[test]
    fn picture_both_without_only_p_uses_ab_p() {
        let nan = f32::NAN;
        assert_eq!(picture_rule(false, nan, 0.5, 0.5, 0.0), Both);
        assert_eq!(
            picture_rule(false, nan, 0.5, 0.49, 0.5),
            Unmeasurable(Why::Ambiguous)
        );
        assert_eq!(
            picture_rule(false, nan, 0.49, 0.5, 0.0),
            Unmeasurable(Why::Ambiguous)
        );
    }

    // ---------------------------------------------------------------------
    // 観測の窓・較正の合否・床・集計の行（2.4）
    // ---------------------------------------------------------------------

    fn tick_rec(tick: u32, hit: Class) -> TickRecord {
        TickRecord {
            tick,
            pair: PAIR_ASSET,
            ended_qpc: tick as i64 * 100,
            rect: RECT {
                left: 160,
                top: 160,
                right: 160 + SQ.0 as i32,
                bottom: 160 + SQ.1 as i32,
            },
            hit,
            h_p: 0.0,
            h_q: 0.0,
            h_both: 0.0,
            covered: false,
            k_ok: true,
            slot_hit: false,
        }
    }

    fn frame_at(tick: u32, picture: Class) -> FrameRecord {
        FrameRecord {
            present_qpc: tick as i64 * 100 + 50,
            accumulated: 1,
            tick: Some(tick),
            picture,
            a: 0.0,
            b: 0.0,
            ab_p: 0.0,
            ab_q: 0.0,
        }
    }

    /// tick 1..=300 の記録（要求 tick 10 より前は当たり判定 P、以後は Q）。
    fn shared_p_then_q() -> Shared {
        Shared {
            ticks: (1..=300)
                .map(|t| tick_rec(t, if t < 10 { P } else { Q }))
                .collect(),
            frames: Vec::new(),
        }
    }

    fn open_p_to_q(s: &Shared) -> Open {
        let obs = Observation::new("t A0→B0", &sig(), P, Q, 10);
        Open::new(obs, s)
    }

    #[test]
    fn window_starts_at_last_frame_before_request_and_closes_30_ticks_after_settling() {
        let mut s = shared_p_then_q();
        s.frames.push(frame_at(8, P));
        s.frames.push(frame_at(9, P));
        let mut o = open_p_to_q(&s);
        // 要求より前の tick のフレームが遅れて届いたら、それが「直前のフレーム」になる（空）。
        s.frames.push(frame_at(9, Neither));
        s.frames.push(frame_at(10, P)); // 反映待ち
        s.frames.push(frame_at(11, Q)); // 揃った（tick 11）→ 閉じるのは tick 41
        s.frames.push(frame_at(20, Q));
        s.frames.push(frame_at(41, Q)); // 窓の最後の tick は数える
        assert!(
            !o.step(&s, 41),
            "41 より後のフレームも猶予も無いうちは閉じない"
        );
        s.frames.push(frame_at(42, Q)); // 窓の外: 数えないが、閉じてよい印になる
        assert!(o.step(&s, 42));
        let obs = o.close();
        assert_eq!(obs.settled, Some((2, 11)));
        assert!(!obs.no_prev);
        assert_eq!(obs.end, Some(End::Done));
        // 直前（空・当たり P）は空かつ混在、tick 10（絵 P・当たり Q・矩形は Q の寸）は反映待ちかつ大きさの食い違い。
        assert_eq!(
            obs.counts,
            Counts {
                frames: 5,
                mixed: 2,
                pending: 1,
                empty: 1,
                size: 1,
                ..Counts::default()
            }
        );
    }

    #[test]
    fn window_closes_after_grace_when_no_later_frame_arrives() {
        let mut s = shared_p_then_q();
        s.ticks[4].pair = PAIR_FACE; // tick 5 は別の対で記録された
        s.frames.push(frame_at(5, P));
        let mut o = open_p_to_q(&s);
        s.frames.push(frame_at(11, Q)); // 揃った → 閉じるのは tick 41
        assert!(!o.step(&s, 41 + GRACE_TICKS - 1));
        assert!(o.step(&s, 41 + GRACE_TICKS));
        let obs = o.close();
        assert_eq!(obs.end, Some(End::Done));
        assert_eq!(obs.settled, Some((1, 11)));
        // 別の対で判別した直前のフレームは測れない。
        assert_eq!(
            obs.counts,
            Counts {
                frames: 2,
                unmeasurable: 1,
                ..Counts::default()
            }
        );
    }

    #[test]
    fn window_gives_up_180_ticks_after_request_and_keeps_the_last_pair() {
        let mut s = shared_p_then_q();
        s.ticks[99].covered = true; // tick 100 は覆われている
        let mut o = open_p_to_q(&s); // 直前のフレーム無し
        s.frames.push(frame_at(100, Q)); // 絵も当たり判定も到達先だが測れない → 揃ったにならない
        for t in (110..=190).step_by(10) {
            s.frames.push(frame_at(t, P)); // 反映待ちのまま
        }
        assert!(!o.step(&s, 190));
        s.frames.push(frame_at(191, P));
        assert!(o.step(&s, 191));
        let obs = o.close();
        assert!(obs.no_prev);
        assert_eq!(obs.settled, None);
        assert_eq!(
            obs.end,
            Some(End::Incomplete(Some((P, Q, tick_rec(190, Q).rect))))
        );
        assert_eq!(obs.counts.frames, 10);
        assert_eq!(obs.counts.unmeasurable, 1);
        assert_eq!(obs.counts.pending, 9);
        // 猶予でも閉じる（フレームが 1 枚も来ない）。
        let s = shared_p_then_q();
        let mut o = open_p_to_q(&s);
        assert!(!o.step(&s, 10 + GIVE_UP_TICKS + GRACE_TICKS - 1));
        assert!(o.step(&s, 10 + GIVE_UP_TICKS + GRACE_TICKS));
        assert_eq!(o.close().end, Some(End::Incomplete(None)));
    }

    /// 猶予で閉じた後に届いた窓の中のフレームは、数えずに「遅着」として観測の行に出る。
    #[test]
    fn frames_arriving_after_a_grace_close_are_reported_as_late() {
        let shared = Arc::new(Mutex::new(shared_p_then_q()));
        let push = |t, pic| shared.lock().unwrap().frames.push(frame_at(t, pic));
        let mut ob = Observer::new(
            Entity::PLACEHOLDER,
            Arc::new([sig(), sig()]),
            shared.clone(),
        );
        ob.open(PAIR_ASSET, Kind::Swap, "t A0→B0", P, Q, 10);
        push(11, Q); // 揃った → 窓の最後は tick 41
        ob.step(11);
        ob.step(41 + GRACE_TICKS); // 窓の外のフレームが来ないまま猶予で閉じる
        assert!(ob.is_closed());
        assert_eq!(ob.done[0].counts.frames, 1); // 直前のフレーム無し・tick 11 の 1 枚
        push(40, Q); // 窓の中 → 遅着
        ob.step(41 + GRACE_TICKS + 1);
        push(41, P); // 窓の中 → 遅着（最終の並べ出しでも拾う）
        ob.summarize(false, 41 + GRACE_TICKS + 2);
        push(41, Q); // 並べ出しの後は数えない
        ob.step(41 + GRACE_TICKS + 3);
        let o = &ob.done[0];
        assert_eq!(o.late_dropped, 2);
        assert_eq!(o.counts.frames, 1, "遅着は 4 種にも frames にも数えない");
        assert!(row(o).contains(" late_dropped=2 "), "{}", row(o));

        // 窓の外のフレームで閉じたときは、後から来るのは窓の外だけなので遅着は 0。
        let shared = Arc::new(Mutex::new(shared_p_then_q()));
        let push = |t| shared.lock().unwrap().frames.push(frame_at(t, Q));
        let mut ob = Observer::new(
            Entity::PLACEHOLDER,
            Arc::new([sig(), sig()]),
            shared.clone(),
        );
        ob.open(PAIR_ASSET, Kind::Swap, "t A0→B0", P, Q, 10);
        push(11);
        push(42);
        ob.step(42);
        assert!(ob.is_closed());
        push(43);
        ob.step(43);
        assert_eq!(ob.done[0].late_dropped, 0);
    }

    fn closed(kind: Kind, name: &str, counts: Counts) -> Observation {
        Observation {
            kind,
            counts,
            end: Some(End::Done),
            ..Observation::new(name, &sig(), P, Q, 0)
        }
    }

    #[test]
    fn calibration_verdict_follows_design() {
        let c = |f: fn(&mut Counts)| {
            let mut c = Counts {
                frames: 3,
                ..Counts::default()
            };
            f(&mut c);
            c
        };
        let good = vec![
            closed(Kind::Calib(Calib::Static), "calib-static", c(|_| {})),
            closed(Kind::Calib(Calib::Empty), "calib-empty", c(|c| c.empty = 1)),
            closed(Kind::Calib(Calib::Mixed), "calib-mixed", c(|c| c.mixed = 2)),
            closed(Kind::Calib(Calib::Stale), "calib-stale", c(|c| c.stale = 1)),
            closed(Kind::Calib(Calib::Size), "calib-size", c(|c| c.size = 1)),
            // 本番の行は合否に関わらない。
            closed(Kind::Swap, "reattach@update A→B", c(|c| c.mixed = 9)),
        ];
        assert_eq!(calibration_verdict(&good), Verdict::Passed);
        assert_eq!(calibration_verdict(&[]), Verdict::Passed, "較正 0 項は合格");

        let bad = vec![
            closed(
                Kind::Calib(Calib::Static),
                "calib-static",
                c(|c| c.size = 1),
            ),
            closed(
                Kind::Calib(Calib::Static),
                "calib-static-0",
                Counts::default(),
            ),
            closed(Kind::Calib(Calib::Empty), "calib-empty", c(|c| c.mixed = 1)),
            closed(Kind::Calib(Calib::Mixed), "calib-mixed", c(|c| c.empty = 1)),
            closed(Kind::Calib(Calib::Stale), "calib-stale", c(|c| c.size = 1)),
            closed(Kind::Calib(Calib::Size), "calib-size", c(|c| c.stale = 1)),
        ];
        assert_eq!(
            calibration_verdict(&bad),
            Verdict::Failed(vec![
                "calib-static".into(),
                "calib-static-0".into(),
                "calib-empty".into(),
                "calib-mixed".into(),
                "calib-stale".into(),
                "calib-size".into(),
            ])
        );
    }

    #[test]
    fn floor_is_the_larger_pending_of_face_switches() {
        let p = |n| Counts {
            pending: n,
            ..Counts::default()
        };
        assert_eq!(floor(&[closed(Kind::Swap, "x", p(5))]), None);
        assert_eq!(
            floor(&[
                closed(Kind::FaceSwitch, "face-switch@update A0→A2", p(1)),
                closed(Kind::Swap, "x", p(5)),
                closed(Kind::FaceSwitch, "face-switch@update A2→A0", p(2)),
            ]),
            Some(2)
        );
    }

    #[test]
    fn row_spells_out_every_count_including_zeros() {
        let mut o = closed(
            Kind::Swap,
            "remove-then-attach@update A→B",
            Counts {
                frames: 7,
                ..Counts::default()
            },
        );
        o.request_tick = 10;
        o.settled = Some((2, 13));
        assert_eq!(
            row(&o),
            "remove-then-attach@update A→B pair=(A0,B0) 完了 frames=7 missed=0 unmeasurable=0 \
             mixed=0 pending=0 empty=0 stale=0 size=0 late_dropped=0 settled_after_frames=2 \
             settled_after_ticks=3"
        );
        o.settled = None;
        o.no_prev = true;
        o.end = Some(End::Incomplete(Some((P, Both, tick_rec(1, P).rect))));
        let r = row(&o);
        assert!(
            r.contains("未完（最後: 絵=P・当たり判定=Both・矩形=400x224@(160,160)）"),
            "{r}"
        );
        assert!(
            r.contains("settled_after_frames=- settled_after_ticks=-"),
            "{r}"
        );
        assert!(r.ends_with("直前のフレーム無し"), "{r}");
        o.end = None;
        assert!(row(&o).contains(" 途中 "));
    }

    /// 当たり判定の閾値の境（0.9 と 0.1 は含む）。
    #[test]
    fn tick_rect_takes_the_move_queued_in_the_same_tick_only() {
        let r = |w| RECT {
            left: 160,
            top: 160,
            right: 160 + w,
            bottom: 365,
        };
        let actual = r(335);
        assert_eq!(effective_rect(Some((7, r(400))), 7, actual), r(400));
        assert_eq!(effective_rect(Some((6, r(400))), 7, actual), actual);
        assert_eq!(effective_rect(None, 7, actual), actual);
    }

    #[test]
    fn hit_rule_thresholds_are_inclusive() {
        let nan = f32::NAN;
        assert_eq!(hit_rule(true, 0.9, 0.9, nan), Both);
        assert_eq!(hit_rule(true, 0.89, 0.9, nan), Unmeasurable(Why::Ambiguous));
        assert_eq!(hit_rule(true, 0.9, 0.1, nan), P);
        assert_eq!(hit_rule(true, 0.9, 0.11, nan), Unmeasurable(Why::Ambiguous));
        assert_eq!(hit_rule(true, 0.1, 0.1, nan), Neither);
        assert_eq!(hit_rule(false, nan, 0.1, 0.9), P);
        assert_eq!(
            hit_rule(false, nan, 0.1, 0.89),
            Unmeasurable(Why::Ambiguous)
        );
    }
}
