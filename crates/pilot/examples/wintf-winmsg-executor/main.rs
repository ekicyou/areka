//! 先進坑: wintf-winmsg-executor
//!
//! 対応本坑 spec: `.kiro/specs/wintf-winmsg-executor/brief.md`
//! 一次記録（動機・概要・検証結果）は隣の README.md（3 幕）を正本とする。
//!
//! 実行法: `cargo run -p pilot --example wintf-winmsg-executor`
//!
//! 検証ターゲット（README の合否基準と一致）:
//!   1. DwmFlush 60Hz スレッド → event_listener::Event notify → UI スレッドの
//!      spawn_local async tick タスクが listener.await で実 vblank cadence に追従起床する。
//!   2. tick 中の SetWindowPos→WM_WINDOWPOSCHANGED 再入で nested-message
//!      （new_checked_ex の RefCell）と擬似再入ガードが衝突しない（二重 tick / 再入実行なし）。
//!   3. util::Window::new_ex（*_ex API）の wndproc クロージャから共有状態（Pin<&S>）へ
//!      安全アクセスでき、GWLP_USERDATA 手詰めが不要。
//!   4. block_on の future 完了でメッセージループが panic せず清掃終了する。
//!   5. 【並行ストレス】縦 3 窓 × 3 tick タスクを 1 本の vsync が notify(MAX) で同時起床し、
//!      違う周期で動かしても破綻（crash/deadlock/二重tick/取りこぼし爆発）しない。
//!
//! 注: WS_EX_NOREDIRECTIONBITMAP の窓は redirection surface を持たず GDI(WM_PAINT) の描画が
//!     画面に出ない（DComp 描画が要る）。criterion 3 の NOREDIRECTIONBITMAP 生成・dispatch は
//!     headless 実行で実証済み。本ファイルの可視窓は GDI を見せるため ex_style=0（redirected）。
//!
//! 使い捨て前提・品質基準は緩くてよい（二坑モデル要件 5.4）。ただし葉ノード隔離は厳守。

use std::cell::{Cell, RefCell};
use std::pin::Pin;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use event_listener::Event;
use wintf_winmsg_executor::util::{Window, WindowMessage, WindowType};
use wintf_winmsg_executor::{block_on, spawn_local};
use windows::Win32::Foundation::{COLORREF, HWND, LRESULT, RECT};
use windows::Win32::Graphics::Dwm::DwmFlush;
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateSolidBrush, DeleteObject, EndPaint, FillRect, InvalidateRect, HGDIOBJ,
    PAINTSTRUCT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetClientRect, SendMessageW, SetWindowPos, ShowWindow, HWND_TOPMOST, SWP_NOACTIVATE,
    SWP_NOSIZE, SWP_NOZORDER, SWP_SHOWWINDOW, SW_SHOW, WINDOW_EX_STYLE, WM_APP, WM_ERASEBKGND,
    WM_PAINT, WM_WINDOWPOSCHANGED,
};

/// tick 中に自ウィンドウへ撃ち込む再入プローブメッセージ。
const WM_REENTRY_PROBE: u32 = WM_APP + 1;

/// 可視確認のため数秒回す（vblank cadence に依らず wall-clock で計る）。
const RUN_SECS: u64 = 8;

/// 可視ウィンドウのジオメトリ（目視用・縦 3 段）。
const WIN_W: i32 = 300;
const WIN_H: i32 = 90;
const BASE_X: i32 = 120;
const BASE_Y: i32 = 100;
const GAP: i32 = 24;

/// 各窓の個性（横バウンスの周期[frames]＝速度／距離、色脈動の係数）。違う周期で動かす。
struct Profile {
    period: u64, // 三角波の片道フレーム数（＝travel px・大きいほど遠く・ゆっくり）
    pulse_k: u64,
    rgb_axis: u8, // 0=R 1=G 2=B を脈動軸にする
}
const PROFILES: [Profile; 3] = [
    Profile { period: 70, pulse_k: 7, rgb_axis: 0 },
    Profile { period: 130, pulse_k: 3, rgb_axis: 1 },
    Profile { period: 200, pulse_k: 5, rgb_axis: 2 },
];

/// 窓ごとの共有状態（!Send で良い＝Rc）。ECS では Rc<RefCell<EcsWorld>> 相当。
#[derive(Default)]
struct Shared {
    idx: usize,
    frames: Cell<u64>,
    last: Cell<Option<Instant>>,
    intervals_ms: RefCell<Vec<f64>>,
    in_tick: Cell<bool>,
    wpos_total: Cell<u64>,
    wpos_during_tick: Cell<u64>,
    reentry_probe_sent: Cell<bool>,
    reentry_body_ran: Cell<bool>,
    double_tick_violation: Cell<bool>,
}

impl Shared {
    fn new(idx: usize) -> Self {
        Self {
            idx,
            ..Default::default()
        }
    }
}

/// 1 フレームぶんの処理（実 ECS は持ち込まず最小再現）。窓 idx を自分の周期で動かす。
fn frame(s: &Shared, hwnd: HWND) {
    if s.in_tick.replace(true) {
        s.double_tick_violation.set(true);
    }

    let now = Instant::now();
    if let Some(prev) = s.last.replace(Some(now)) {
        s.intervals_ms
            .borrow_mut()
            .push((now - prev).as_secs_f64() * 1000.0);
    }
    let f = s.frames.get() + 1;
    s.frames.set(f);

    // criterion 2 ＋ 目視: tick の最中に実 SetWindowPos を撃ち、OS に正当な WINDOWPOS* 付きの
    // WM_WINDOWPOSCHANGED を nested 生成（合成送出は user32 で AV ＝禁・先進坑の知見）。
    let p = &PROFILES[s.idx];
    let phase = (f % (2 * p.period)) as i32;
    let period = p.period as i32;
    let off = if phase < period { phase } else { 2 * period - phase };
    let y = BASE_Y + s.idx as i32 * (WIN_H + GAP);
    unsafe {
        let _ = SetWindowPos(
            hwnd,
            None,
            BASE_X + off,
            y,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        );
        let _ = InvalidateRect(Some(hwnd), None, false);
    }

    s.in_tick.set(false);
}

/// new_checked_ex でひとつ可視窓を作って表示する（redirected＝GDI が見える）。
fn make_window(shared: Rc<Shared>) -> Window<Rc<Shared>> {
    let win = Window::new_checked_ex(
        WindowType::TopLevel,
        WINDOW_EX_STYLE(0), // redirected（GDI WM_PAINT を見せるため NOREDIRECTIONBITMAP は付けない）
        shared,
        move |state: Pin<&Rc<Shared>>, msg: WindowMessage| -> Option<LRESULT> {
            let s: &Shared = state.get_ref();
            match msg.msg {
                WM_WINDOWPOSCHANGED => {
                    s.wpos_total.set(s.wpos_total.get() + 1);
                    if s.in_tick.get() {
                        s.wpos_during_tick.set(s.wpos_during_tick.get() + 1);
                    }
                    // 一度だけ自窓へ同期メッセージを撃って wndproc 再入を誘発。
                    // RefCell が借用中ゆえ再入は DefWindowProc へフォワードされ body は走らないはず。
                    if !s.reentry_probe_sent.replace(true) {
                        unsafe {
                            SendMessageW(msg.hwnd, WM_REENTRY_PROBE, None, None);
                        }
                    }
                    Some(LRESULT(0))
                }
                WM_REENTRY_PROBE => {
                    s.reentry_body_ran.set(true); // 走ったら RefCell が阻止できていない（NG）
                    Some(LRESULT(0))
                }
                WM_ERASEBKGND => Some(LRESULT(1)),
                WM_PAINT => {
                    unsafe {
                        let mut ps = PAINTSTRUCT::default();
                        let hdc = BeginPaint(msg.hwnd, &mut ps);
                        let mut rc = RECT::default();
                        let _ = GetClientRect(msg.hwnd, &mut rc);
                        let prof = &PROFILES[s.idx];
                        let pulse = (s.frames.get().wrapping_mul(prof.pulse_k) % 256) as u32;
                        let (mut r, mut g, mut b) = (40u32, 40u32, 40u32);
                        match prof.rgb_axis {
                            0 => r = pulse,
                            1 => g = pulse,
                            _ => b = pulse,
                        }
                        let brush = CreateSolidBrush(COLORREF(r | (g << 8) | (b << 16)));
                        FillRect(hdc, &rc, brush);
                        let _ = DeleteObject(HGDIOBJ(brush.0));
                        let _ = EndPaint(msg.hwnd, &ps);
                    }
                    Some(LRESULT(0))
                }
                _ => None,
            }
        },
    )
    .expect("new_checked_ex でウィンドウ生成に失敗");

    let idx = win.state().idx;
    let y = BASE_Y + idx as i32 * (WIN_H + GAP);
    unsafe {
        let _ = SetWindowPos(
            win.hwnd(),
            Some(HWND_TOPMOST),
            BASE_X,
            y,
            WIN_W,
            WIN_H,
            SWP_SHOWWINDOW,
        );
        let _ = ShowWindow(win.hwnd(), SW_SHOW);
    }
    win
}

fn main() {
    println!("=== pilot: wintf-winmsg-executor 先進坑（縦 3 窓・並行ストレス） ===");

    let event = Arc::new(Event::new());
    let done = Arc::new(AtomicBool::new(false));
    let notify_count = Arc::new(AtomicU64::new(0));

    // 3 窓 ＋ 3 共有状態を生成・表示。Window ハンドルは block_on 後まで生かす（drop で破棄）。
    let shareds: Vec<Rc<Shared>> = (0..3).map(|i| Rc::new(Shared::new(i))).collect();
    let _wins: Vec<Window<Rc<Shared>>> =
        shareds.iter().map(|s| make_window(s.clone())).collect();
    println!("3 visible windows shown @ x={BASE_X}, y={BASE_Y}.. (each {WIN_W}x{WIN_H}, topmost)");
    println!("periods(frames) = {:?} — 約{RUN_SECS}秒・違う周期で横バウンス＋色脈動", PROFILES.map(|p| p.period));

    // --- 1 本の vsync スレッド: DwmFlush → notify(MAX) で 3 タスク同時起床 ---
    let vsync = {
        let event = event.clone();
        let done = done.clone();
        let nc = notify_count.clone();
        thread::spawn(move || {
            let start = Instant::now();
            while start.elapsed() < Duration::from_secs(RUN_SECS) {
                let ok = unsafe { DwmFlush() }.is_ok();
                if !ok {
                    thread::sleep(Duration::from_millis(15));
                }
                nc.fetch_add(1, Ordering::Relaxed);
                event.notify(usize::MAX); // 全リスナ（3 タスク）を起床
            }
            done.store(true, Ordering::Release);
            for _ in 0..10 {
                event.notify(usize::MAX);
                thread::sleep(Duration::from_millis(2));
            }
        })
    };

    // --- UI スレッド: 窓ごとに tick タスクを spawn_local（3 タスク並行） ---
    let handles: Vec<_> = shareds
        .iter()
        .map(|s| {
            let event = event.clone();
            let done = done.clone();
            let shared = s.clone();
            let hwnd = _wins[shared.idx].hwnd();
            spawn_local(async move {
                loop {
                    let listener = event.listen();
                    if done.load(Ordering::Acquire) {
                        break;
                    }
                    listener.await;
                    if done.load(Ordering::Acquire) {
                        break;
                    }
                    frame(&shared, hwnd);
                }
            })
        })
        .collect();

    // criterion 4: 3 タスクの完了を待って清掃終了（panic しないか）。
    block_on(async {
        for h in handles {
            h.await;
        }
    });
    println!("block_on returned cleanly (no panic)");

    vsync.join().ok();

    // --- 集計 ---
    let notifies = notify_count.load(Ordering::Relaxed);
    println!("\n--- 計測結果（notify(MAX)={notifies} = vblank 数） ---");
    let mut all_ok_concurrency = true;
    for s in &shareds {
        let frames = s.frames.get();
        let intervals = s.intervals_ms.borrow();
        let avg = if intervals.is_empty() {
            0.0
        } else {
            intervals.iter().sum::<f64>() / intervals.len() as f64
        };
        let coverage = if notifies > 0 {
            frames as f64 / notifies as f64
        } else {
            0.0
        };
        let ng = s.reentry_body_ran.get() || s.double_tick_violation.get();
        if ng || coverage < 0.95 {
            all_ok_concurrency = false;
        }
        println!(
            "win{}: frames={frames} (cov={:.1}%) avg={avg:.2}ms wpos(total/in_tick)={}/{} reentry_body(NG)={} double_tick(NG)={}",
            s.idx,
            coverage * 100.0,
            s.wpos_total.get(),
            s.wpos_during_tick.get(),
            s.reentry_body_ran.get(),
            s.double_tick_violation.get(),
        );
    }

    let c2 = shareds.iter().all(|s| {
        !s.double_tick_violation.get() && !s.reentry_body_ran.get() && s.wpos_during_tick.get() > 0
    });
    let c3 = shareds.iter().all(|s| s.wpos_total.get() > 0);

    println!("\n--- criterion 一次判定（参考・最終は人間判断） ---");
    println!("1/5 並行 tick 起床・破綻なし     : {}", verdict(all_ok_concurrency));
    println!("2  nested-message × RefCell     : {}", verdict(c2));
    println!("3  *_ex state アクセス           : {}", verdict(c3));
    println!("4  清掃終了(panic なし)          : {}", verdict(true));
    println!(
        "\n総合一次判定: {}",
        if all_ok_concurrency && c2 && c3 {
            "go 候補（README 検証結果欄を人間が確定すること）"
        } else {
            "要確認（NG 項目あり・README に学びを記録）"
        }
    );
}

fn verdict(ok: bool) -> &'static str {
    if ok {
        "PASS"
    } else {
        "FAIL"
    }
}
