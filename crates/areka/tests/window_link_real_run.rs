//! areka-P0-window-link-directives タスク 9.2 の実機の確かめ（要件 9.7・9.8・既定 OFF）。
//!
//! 検体「窓際のぱすたさん」（`pasta-in-windows`）を `real-run-kit` で起こし、窓の組の連れ動き・
//! `ignore-sticky-window`・起こし直しでの重なり・位置初期化の 2 つの近道を、窓の矩形と記録で判定する。
//! 本物の窓を出し、マウスとキーを `SendInput` で送るので、排他テストの机を持ち、画面のロックが
//! 無いときだけ回す。確かめの間は拡大率を変えない。
//!
//! 有効にするのは環境変数 `AREKA_WINDOW_LINK_REAL_RUN`（空でない値）。未設定なら即 return する。
//! 先に要るもの: `tools/sample-fetch.ps1` で検体の書庫を取り寄せておく（検体は
//! `sample_ghost_kit::SampleRoot::acquire` が名前で引いて、使い捨ての複製を配る）。
//! 32bit の仲介 `cargo build -p shiori-host32-helper --target i686-pc-windows-msvc` も建てておく。
//!
//! 根・記憶の置き場・記録はすべて `target\wl-signoff\` の下（要件 9.8）。根は配られた複製の写しに、
//! ゴーストの中のバルーン `pasta_tategaki` を根の `balloon\` へ写して作る（ゴーストの中のバルーンを
//! 探すかどうかは `areka-P0-pasta-in-windows-verification` の持ち分なので、ここでは利用者が置いた形にする）。
//!
//! 判定は 1 行ずつ `target\wl-signoff\judgments.txt` に書き、走りの記録は `run1.log`・`run2.log` へ写す。

use real_run_kit::{Launch, RealRun, Run};
use sample_ghost_kit::SampleRoot;
use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::thread::sleep;
use std::time::{Duration, Instant};
use windows::Win32::Foundation::{HWND, LPARAM, POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MONITOR_DEFAULTTONULL, MONITORINFO, MonitorFromPoint,
};
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, GetDpiForWindow, SetThreadDpiAwarenessContext,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_KEYUP,
    MOUSE_EVENT_FLAGS, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
    MOUSEEVENTF_MOVE, MOUSEEVENTF_VIRTUALDESK, MOUSEINPUT, SendInput, VIRTUAL_KEY, VK_CONTROL,
    VK_F10, VK_F12, VK_SHIFT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GA_ROOT, GW_HWNDPREV, GWL_EXSTYLE, GetAncestor, GetCursorPos, GetForegroundWindow,
    GetSystemMetrics, GetWindow, GetWindowLongPtrW, GetWindowRect, GetWindowTextW,
    GetWindowThreadProcessId, HWND_TOP, IsWindowVisible, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN,
    SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetWindowPos,
    WS_EX_TRANSPARENT, WindowFromPoint,
};
use windows::core::BOOL;

const REAL_RUN_ENV: &str = "AREKA_WINDOW_LINK_REAL_RUN";
const RUST_LOG: &str = "info,kanade=debug,areka::placement=debug,areka::input_events=debug,\
areka::menu=debug,wintf::ecs::window_proc::keyboard=debug,wintf::ecs::clickthrough=debug";
const SAKURA: &str = "ぱすた";
const KERO: &str = "窓";

/// 見えている窓 1 つ（物理画素の矩形 左,上,右,下）。
#[derive(Clone, Debug)]
struct Win {
    hwnd: isize,
    title: String,
    rect: [i32; 4],
    dpi: u32,
}

fn per_monitor() {
    unsafe {
        SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
}

fn windows_of(pid: u32) -> Vec<Win> {
    per_monitor();
    unsafe extern "system" fn each(h: HWND, l: LPARAM) -> BOOL {
        let state: &mut (u32, Vec<Win>) = unsafe { &mut *(l.0 as *mut (u32, Vec<Win>)) };
        let (pid, out) = (state.0, &mut state.1);
        let mut p = 0u32;
        unsafe { GetWindowThreadProcessId(h, Some(&mut p)) };
        if p == pid && unsafe { IsWindowVisible(h) }.as_bool() {
            let mut buf = [0u16; 256];
            let n = unsafe { GetWindowTextW(h, &mut buf) } as usize;
            let mut r = RECT::default();
            let _ = unsafe { GetWindowRect(h, &mut r) };
            out.push(Win {
                hwnd: h.0 as isize,
                title: String::from_utf16_lossy(&buf[..n]),
                rect: [r.left, r.top, r.right, r.bottom],
                dpi: unsafe { GetDpiForWindow(h) },
            });
        }
        BOOL(1)
    }
    let mut state: (u32, Vec<Win>) = (pid, Vec::new());
    let _ = unsafe { EnumWindows(Some(each), LPARAM(&mut state as *mut _ as isize)) };
    // 画面の外に退避している窓（-32000 など）は数えない。
    state
        .1
        .retain(|w| w.rect[0] > -30000 && w.rect[2] > w.rect[0]);
    state.1
}

fn area(w: &Win) -> i64 {
    (w.rect[2] - w.rect[0]) as i64 * (w.rect[3] - w.rect[1]) as i64
}

fn mouse(flags: MOUSE_EVENT_FLAGS, x: i32, y: i32) -> u32 {
    per_monitor();
    let (vx, vy, vw, vh) = unsafe {
        (
            GetSystemMetrics(SM_XVIRTUALSCREEN),
            GetSystemMetrics(SM_YVIRTUALSCREEN),
            GetSystemMetrics(SM_CXVIRTUALSCREEN),
            GetSystemMetrics(SM_CYVIRTUALSCREEN),
        )
    };
    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: ((x - vx) as i64 * 65535 / (vw - 1) as i64) as i32,
                dy: ((y - vy) as i64 * 65535 / (vh - 1) as i64) as i32,
                mouseData: 0,
                dwFlags: flags | MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    unsafe { SendInput(&[input], size_of::<INPUT>() as i32) }
}

fn key(vk: VIRTUAL_KEY, up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: if up {
                    KEYEVENTF_KEYUP
                } else {
                    KEYBD_EVENT_FLAGS(0)
                },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

/// Ctrl＋Shift＋`vk` を押して離す。
fn chord(vk: VIRTUAL_KEY) -> u32 {
    let seq = [
        key(VK_CONTROL, false),
        key(VK_SHIFT, false),
        key(vk, false),
        key(vk, true),
        key(VK_SHIFT, true),
        key(VK_CONTROL, true),
    ];
    unsafe { SendInput(&seq, size_of::<INPUT>() as i32) }
}

/// 判定と手順の控え。
struct Notes {
    base: PathBuf,
    steps: String,
    judgments: Vec<(String, bool, String)>,
}

impl Notes {
    fn note(&mut self, s: impl AsRef<str>) {
        eprintln!("[wl] {}", s.as_ref());
        let _ = writeln!(self.steps, "{} {}", stamp(), s.as_ref());
    }
    fn judge(&mut self, name: &str, ok: bool, detail: impl Into<String>) {
        let detail = detail.into();
        self.note(format!(
            "JUDGE {} {name}: {detail}",
            if ok { "PASS" } else { "FAIL" }
        ));
        self.judgments.push((name.to_string(), ok, detail));
    }
    fn snapshot(&mut self, label: &str, wins: &[Win]) {
        for w in wins {
            self.note(format!(
                "rect {label} hwnd={:#x} title={} rect={:?} dpi={}",
                w.hwnd, w.title, w.rect, w.dpi
            ));
        }
    }
    fn save(&self) {
        let _ = std::fs::write(self.base.join("steps.txt"), &self.steps);
        let mut j = String::new();
        for (n, ok, d) in &self.judgments {
            let _ = writeln!(j, "{} {n}: {d}", if *ok { "PASS" } else { "FAIL" });
        }
        let _ = std::fs::write(self.base.join("judgments.txt"), j);
    }
}

fn stamp() -> String {
    format!("{:?}", std::time::SystemTime::now())
}

/// 記録の中の `needle` の行（標準出力＋標準エラー）。
fn lines_with(run: &Run, needle: &str) -> Vec<String> {
    run.output()
        .lines()
        .filter(|l| l.contains(needle))
        .map(str::to_string)
        .collect()
}

/// `needle` を含む行が `count` 本を超えるまで待つ（新しく出た行を返す）。
fn wait_new(run: &Run, needle: &str, count: usize, secs: u64) -> Option<String> {
    let until = Instant::now() + Duration::from_secs(secs);
    while Instant::now() < until {
        let ls = lines_with(run, needle);
        if ls.len() > count {
            return ls.get(count).cloned();
        }
        sleep(Duration::from_millis(100));
    }
    None
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap() {
        let e = e.unwrap();
        let dst = to.join(e.file_name());
        if e.file_type().unwrap().is_dir() {
            copy_dir(&e.path(), &dst);
        } else {
            std::fs::copy(e.path(), dst).unwrap();
        }
    }
}

/// 窓ごとの位置の差（左上）。
fn deltas(before: &[Win], after: &[Win]) -> Vec<(String, isize, i32, i32)> {
    let now: HashMap<isize, &Win> = after.iter().map(|w| (w.hwnd, w)).collect();
    before
        .iter()
        .filter_map(|b| {
            now.get(&b.hwnd).map(|a| {
                (
                    b.title.clone(),
                    b.hwnd,
                    a.rect[0] - b.rect[0],
                    a.rect[1] - b.rect[1],
                )
            })
        })
        .collect()
}

/// 題名ごとの最大の窓（キャラクターの窓とみなす）の左上。
fn top_left(wins: &[Win], title: &str) -> Option<(i32, i32)> {
    wins.iter()
        .filter(|w| w.title == title)
        .max_by_key(|w| area(w))
        .map(|w| (w.rect[0], w.rect[1]))
}

/// 題名ごとの最大の窓（キャラクターの窓とみなす）。
fn char_win(wins: &[Win], title: &str) -> Option<Win> {
    wins.iter()
        .filter(|w| w.title == title)
        .max_by_key(|w| area(w))
        .cloned()
}

/// ぱすたの絵の胴（論理 554×684 の (290,350)）の画面の点。拡大率で物理画素へ直す。
fn body_point(s: &Win) -> (i32, i32) {
    let k = |v: i32| v * s.dpi as i32 / 96;
    (s.rect[0] + k(290), s.rect[1] + k(350))
}

/// 点の下の窓と、点がその点のモニタの作業領域の中か。
fn hit_at(x: i32, y: i32) -> (isize, Option<[i32; 4]>) {
    per_monitor();
    let pt = POINT { x, y };
    let hwnd = unsafe { WindowFromPoint(pt) }.0 as isize;
    let monitor = unsafe { MonitorFromPoint(pt, MONITOR_DEFAULTTONULL) };
    let mut info = MONITORINFO {
        cbSize: size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    let work = (!monitor.is_invalid() && unsafe { GetMonitorInfoW(monitor, &mut info) }.as_bool())
        .then(|| {
            let r = info.rcWork;
            [r.left, r.top, r.right, r.bottom]
        });
    (hwnd, work)
}

/// 点の下の窓の根（`GetAncestor(.., GA_ROOT)`）。
fn root_of(hwnd: isize) -> isize {
    unsafe { GetAncestor(HWND(hwnd as _), GA_ROOT) }.0 as isize
}

fn pid_of(hwnd: isize) -> u32 {
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(HWND(hwnd as _), Some(&mut pid)) };
    pid
}

/// 点の下の窓の根が、ぱすたの窓より手前（`GW_HWNDPREV` を辿って見つかる）か。
fn covered_by(sakura: isize, hit: isize) -> bool {
    let target = root_of(hit);
    if target == 0 || target == sakura {
        return false;
    }
    let mut h = HWND(sakura as _);
    while let Ok(prev) = unsafe { GetWindow(h, GW_HWNDPREV) } {
        if prev.is_invalid() {
            break;
        }
        if prev.0 as isize == target {
            return true;
        }
        h = prev;
    }
    false
}

/// ぱすたの胴の点を確かめる: 手前を覆う窓を記録し、ぱすたの窓を（入力先を変えずに）いちばん手前へ
/// 上げ、「点へ乗せる → 300 ms 待つ → 点の下を見る」を最大 5 秒くり返す。点の下がぱすたの
/// キャラクターの窓で、作業領域の中なら使える点。使えなければ見分けの材料を付けて返す。
fn verified_body_point(s: &Win, n: &mut Notes) -> Result<(i32, i32), String> {
    let (x, y) = body_point(s);
    let (hit0, _) = hit_at(x, y);
    n.note(format!(
        "body point {x},{y}: covered={} hit={hit0:#x} hit_pid={}",
        covered_by(s.hwnd, hit0),
        pid_of(hit0)
    ));
    // 動かすのは areka 自身の窓の重ね順だけ（持ち主の付いたバルーンは一緒に上がる）。
    if let Err(e) = unsafe {
        SetWindowPos(
            HWND(s.hwnd as _),
            Some(HWND_TOP),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        )
    } {
        n.note(format!("raise {:#x} failed: {e:?}", s.hwnd));
    }
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        mouse(MOUSE_EVENT_FLAGS(0), x, y);
        sleep(Duration::from_millis(300));
        let (hit, work) = hit_at(x, y);
        let inside = work.is_some_and(|w| x >= w[0] && x < w[2] && y >= w[1] && y < w[3]);
        if hit == s.hwnd && inside {
            return Ok((x, y));
        }
        if Instant::now() >= until {
            let mut cursor = POINT::default();
            let cursor = unsafe { GetCursorPos(&mut cursor) }.map(|_| (cursor.x, cursor.y));
            let ex = unsafe { GetWindowLongPtrW(HWND(s.hwnd as _), GWL_EXSTYLE) };
            return Err(format!(
                "点 {x},{y}: 下の窓={hit:#x} pid={}（ぱすた={:#x}）作業領域={work:?} cursor={cursor:?} \
                 ex_style={ex:#x} transparent={} covered_after_raise={}",
                pid_of(hit),
                s.hwnd,
                ex & WS_EX_TRANSPARENT.0 as isize != 0,
                covered_by(s.hwnd, hit)
            ));
        }
    }
}

/// ぱすたを出してバルーンを閉じる（胴の点を覆い、起動の挨拶の終わりの面は胴が透けるので）。
/// 起動の挨拶（最初の `steady_talk_done`）が済むのを待ってから送り、受け付けられなければ 1 秒ごとに
/// 送り直す（最大 10 回）。閉じた行は最大 20 秒待つ（重いときは返事の 10 秒を過ぎて通る）。
fn show_sakura_and_close_balloon(run: &mut Run, n: &mut Notes) {
    let done = wait_new(run, "event=\"steady_talk_done\"", 0, 40);
    n.note(format!("steady_talk_done={done:?}"));
    let needle = "scope=0 trigger=\"clear\" visible=false";
    let c0 = lines_with(run, needle).len();
    for attempt in 0..=10 {
        let reply = run.call(
            "sakurascript",
            &format!("{{\"script\":{}}}", json(r"\0\s[1000]\e")),
        );
        n.note(format!("show sakura attempt={attempt} reply={reply:?}"));
        let refused = reply
            .as_ref()
            .is_ok_and(|r| r.text.contains("cannot accept a script now"));
        if !refused || attempt == 10 {
            break;
        }
        sleep(Duration::from_secs(1));
    }
    let line = wait_new(run, needle, c0, 20);
    n.note(format!("close balloon line={line:?}"));
    sleep(Duration::from_millis(500));
}

/// キャラクターの窓 `hwnd` の entity（`owner-established` の行の `peer=`・最後の行）。
fn char_entity(run: &Run, hwnd: isize) -> Option<String> {
    lines_with(run, &format!("owner_hwnd={hwnd:#X} "))
        .last()
        .and_then(|l| l.split("peer=").nth(1))
        .and_then(|r| r.split_whitespace().next())
        .map(str::to_string)
}

fn spawn(exe: &str, root: &Path, profile: &Path, exit: Duration) -> Run {
    RealRun::new(exe)
        .launch(Launch::Root(root.to_path_buf()))
        .profile_dir(profile)
        .rust_log(RUST_LOG)
        .safety_exit(exit)
        .deadline(Duration::from_secs(90))
        .spawn()
}

#[test]
fn window_link_directives_on_pasta_in_windows() {
    if std::env::var(REAL_RUN_ENV)
        .map(|v| v.trim().is_empty())
        .unwrap_or(true)
    {
        return;
    }
    let target = Path::new(env!("CARGO_MANIFEST_DIR")).join(r"..\..\target");
    // `canonicalize` の `\\?\` 付きの根では pasta が辞書を探せず LOAD に失敗する（real-run-kit の abs と同じ形にする）。
    let target = std::path::absolute(target).unwrap();
    assert!(
        !target.to_string_lossy().starts_with(r"\\?\"),
        "{}",
        target.display()
    );
    let sample = SampleRoot::acquire("pasta-in-windows")
        .unwrap_or_else(|e| panic!("検体 pasta-in-windows を取れない: {e}"));
    let base = target.join("wl-signoff");
    if base.exists() {
        std::fs::remove_dir_all(&base).unwrap();
    }
    let root = base.join("root");
    let profile = base.join("profile");
    copy_dir(&sample.root().join("ghost"), &root.join("ghost"));
    copy_dir(
        &sample.folder().join("pasta_tategaki"),
        &root.join(r"balloon\pasta_tategaki"),
    );
    // 根は写し終えたので、配られた複製はここで返す。
    drop(sample);
    std::fs::create_dir_all(&profile).unwrap();
    let exe = env!("CARGO_BIN_EXE_areka");
    let mut n = Notes {
        base: base.clone(),
        steps: String::new(),
        judgments: Vec::new(),
    };
    n.note(format!(
        "root={} profile={} rust_log={RUST_LOG}",
        root.display(),
        profile.display()
    ));

    // ---- 走り 1: 記憶なしで起こす → 組・ドラッグ・ignore-sticky-window ----
    let mut run = spawn(exe, &root, &profile, Duration::from_secs(150));
    n.note(format!("run1 pid={} port={}", run.pid(), run.mcp_port()));
    let groups = wait_new(&run, "event=\"sticky_groups\"", 0, 60)
        .or_else(|| wait_new(&run, "event=sticky_groups", 0, 1));
    n.judge(
        "1 起動で sticky_groups（descript_seed・{0,1}）",
        groups
            .as_deref()
            .is_some_and(|l| l.contains("descript_seed") && l.contains('0') && l.contains('1')),
        groups.clone().unwrap_or_else(|| "行なし".into()),
    );
    // 起動の段取り（窓あわせ・重ね順）が済むのを待つ。
    let _ = wait_new(&run, "sticky_follow_ignored", 0, 30);
    sleep(Duration::from_secs(4));
    let boot = windows_of(run.pid());
    n.snapshot("run1_boot", &boot);
    let boot_ignored = lines_with(&run, "sticky_follow_ignored");
    n.note(format!("boot sticky_follow_ignored={boot_ignored:?}"));
    n.judge(
        "1b 起動の連鎖の後で 2 つのキャラクターの窓が重なる（左上が同じ）",
        top_left(&boot, SAKURA).is_some() && top_left(&boot, SAKURA) == top_left(&boot, KERO),
        format!(
            "{SAKURA}={:?} {KERO}={:?}",
            top_left(&boot, SAKURA),
            top_left(&boot, KERO)
        ),
    );

    // ドラッグ: ぱすたのバルーンを閉じ、ぱすたの胴の点（確かめた点）を押して右上へ引く。
    // 左へ引くと 150% のモニタへ渡り、確かめの間に拡大率が変わるので右上へ引く。
    show_sakura_and_close_balloon(&mut run, &mut n);
    let before = windows_of(run.pid());
    let sakura = char_win(&before, SAKURA);
    let kero = char_win(&before, KERO);
    let point = match &sakura {
        Some(s) => verified_body_point(s, &mut n),
        None => Err("ぱすたの窓が無い".into()),
    };
    match (point, &sakura, &kero) {
        (Ok((x, y)), Some(s), Some(k)) => {
            let entity = char_entity(&run, s.hwnd);
            let starts = lines_with(&run, "[DragStartEvent] Dispatching").len();
            let follow_before = lines_with(&run, "sticky_follow").len();
            let balloon1 = lines_with(&run, "route=BalloonFollow").len();
            n.note(format!("drag at {x},{y} sakura_entity={entity:?}"));
            mouse(MOUSEEVENTF_LEFTDOWN, x, y);
            sleep(Duration::from_millis(150));
            let (sx, sy) = (12, -6); // 1 歩の量（合計 +240,-120）
            let mut mid = Vec::new();
            for i in 1..=20 {
                mouse(MOUSE_EVENT_FLAGS(0), x + sx * i, y + sy * i);
                sleep(Duration::from_millis(40));
                if i == 10 {
                    sleep(Duration::from_millis(200));
                    mid = windows_of(run.pid());
                }
            }
            sleep(Duration::from_millis(300));
            mouse(MOUSEEVENTF_LEFTUP, x + sx * 20, y + sy * 20);
            sleep(Duration::from_millis(800));
            let after = windows_of(run.pid());
            n.snapshot("run1_before_drag", &before);
            n.snapshot("run1_mid_drag", &mid);
            n.snapshot("run1_after_drag", &after);
            let start = wait_new(&run, "[DragStartEvent] Dispatching", starts, 2);
            n.judge(
                "2 ドラッグはぱすたのキャラクターの窓から始まる（DragStartEvent の entity）",
                entity.as_deref().is_some_and(|e| {
                    start
                        .as_deref()
                        .is_some_and(|l| l.contains(&format!("entity={e} ")))
                }),
                format!("entity={entity:?} line={start:?}"),
            );
            // 両キャラクターの窓だけが同じ 0 でない量動いたか（バルーンは作業領域の端で止まるので別に見る）。
            let chars = [s.hwnd, k.hwnd];
            let same = |ds: &[(String, isize, i32, i32)]| {
                let cs: Vec<_> = ds.iter().filter(|d| chars.contains(&d.1)).collect();
                cs.len() == 2
                    && (cs[0].2 != 0 || cs[0].3 != 0)
                    && cs[0].2 == cs[1].2
                    && cs[0].3 == cs[1].3
            };
            let d_mid = deltas(&before, &mid);
            let d_end = deltas(&before, &after);
            n.judge(
                "2a ドラッグの途中で両キャラクターの窓が同じ量動く",
                same(&d_mid),
                format!("{d_mid:?}"),
            );
            n.judge(
                "2b ドラッグの後で両キャラクターの窓が同じ量動いた",
                same(&d_end),
                format!("{d_end:?}"),
            );
            let balloon_follow: Vec<String> = lines_with(&run, "route=BalloonFollow")
                .into_iter()
                .skip(balloon1)
                .filter(|l| l.contains("scope=1 "))
                .collect();
            n.judge(
                "2d ドラッグの間に窓のバルーンが付いて動く（BalloonFollow scope=1）",
                !balloon_follow.is_empty(),
                format!(
                    "{} 行・先頭={:?}",
                    balloon_follow.len(),
                    balloon_follow.first()
                ),
            );
            let follow = wait_new(&run, "sticky_follow", follow_before, 5);
            let drag_line = lines_with(&run, "trigger=\"drag\"")
                .into_iter()
                .chain(lines_with(&run, "trigger=drag"))
                .find(|l| l.contains("sticky_follow"));
            n.judge(
                "2c sticky_follow（drag）が出る",
                drag_line.is_some(),
                drag_line.or(follow).unwrap_or_else(|| "行なし".into()),
            );
        }
        (point, _, _) => n.judge(
            "2 ドラッグ",
            false,
            format!(
                "押す点が使えない: {point:?} {SAKURA}={:?} {KERO}={:?}",
                sakura.as_ref().map(|w| w.hwnd),
                kero.as_ref().map(|w| w.hwnd)
            ),
        ),
    }

    // ignore-sticky-window: 窓（\1）だけを右へずらし、次に辞書の窓あわせと同じ台本で戻す。
    sleep(Duration::from_secs(3)); // ドラッグの終わりの付け直し（辞書の OnSecondChange）が済むのを待つ
    let ign0 = lines_with(&run, "sticky_follow_ignored").len();
    let before = windows_of(run.pid());
    let shift = r"\1\![move,--X=200,--Y=0,--time=0,--base=0,--base-offset=left.top,--move-offset=left.top,--option=ignore-sticky-window]\e";
    let reply = run.call("sakurascript", &format!("{{\"script\":{}}}", json(shift)));
    n.note(format!("sakurascript shift reply={reply:?}"));
    let ign = wait_new(&run, "sticky_follow_ignored", ign0, 10);
    sleep(Duration::from_millis(800));
    let shifted = windows_of(run.pid());
    n.snapshot("run1_after_shift", &shifted);
    let d = deltas(&before, &shifted);
    let only_kero = d.iter().all(|x| {
        if x.0 == SAKURA {
            x.2 == 0 && x.3 == 0
        } else {
            true
        }
    }) && d.iter().any(|x| x.0 == KERO && x.2 != 0);
    n.judge(
        "3a ignore-sticky-window の move で窓だけが動き、ぱすたは動かない",
        only_kero,
        format!("{d:?}"),
    );
    n.judge(
        "3b sticky_follow_ignored が出る",
        ign.is_some(),
        ign.unwrap_or_else(|| "行なし".into()),
    );
    // 辞書の「窓あわせ」と同じ台本（tategaki.pasta の mado_awase）。
    let ign1 = lines_with(&run, "sticky_follow_ignored").len();
    let awase = r"\0\s[1000]\1\s[10]\![move,--X=0,--Y=0,--time=0,--base=0,--base-offset=left.top,--move-offset=left.top,--option=ignore-sticky-window]\![reset,zorder]\![set,zorder,0,1]\0\e";
    let reply = run.call("sakurascript", &format!("{{\"script\":{}}}", json(awase)));
    n.note(format!("sakurascript awase reply={reply:?}"));
    let ign = wait_new(&run, "sticky_follow_ignored", ign1, 10);
    sleep(Duration::from_millis(800));
    let aligned = windows_of(run.pid());
    n.snapshot("run1_after_awase", &aligned);
    let d = deltas(&shifted, &aligned);
    n.judge(
        "3c 窓あわせで窓だけがぱすたへ戻り重なる・sticky_follow_ignored",
        ign.is_some()
            && d.iter().all(|x| {
                if x.0 == SAKURA {
                    x.2 == 0 && x.3 == 0
                } else {
                    true
                }
            })
            && top_left(&aligned, SAKURA) == top_left(&aligned, KERO),
        format!(
            "deltas={d:?} {SAKURA}={:?} {KERO}={:?} line={ign:?}",
            top_left(&aligned, SAKURA),
            top_left(&aligned, KERO)
        ),
    );
    let last1 = aligned.clone();
    let ended = run.close();
    n.note(format!("run1 ended={ended:?}"));
    let _ = std::fs::write(base.join("run1.log"), run.output());
    drop(run);

    // ---- 走り 2: 記憶を残して起こし直す → 重なり・近道 ----
    let mut run = spawn(exe, &root, &profile, Duration::from_secs(90));
    n.note(format!("run2 pid={} port={}", run.pid(), run.mcp_port()));
    let _ = wait_new(&run, "sticky_groups", 0, 60);
    sleep(Duration::from_secs(6));
    let reboot = windows_of(run.pid());
    n.snapshot("run2_boot", &reboot);
    n.judge(
        "4 起こし直すと 2 つの窓が重なったまま、前回の位置で出る",
        top_left(&reboot, SAKURA).is_some()
            && top_left(&reboot, SAKURA) == top_left(&reboot, KERO)
            && top_left(&reboot, SAKURA) == top_left(&last1, SAKURA),
        format!(
            "run2 {SAKURA}={:?} {KERO}={:?} / run1 終わり {SAKURA}={:?}",
            top_left(&reboot, SAKURA),
            top_left(&reboot, KERO),
            top_left(&last1, SAKURA)
        ),
    );
    n.note(format!(
        "run2 restore lines={:?}",
        lines_with(&run, "merge_scope")
    ));

    // 近道: ぱすたの胴の点（確かめた点）をクリックして入力先にし、入力先が areka のときだけ押す。
    show_sakura_and_close_balloon(&mut run, &mut n);
    let clicked = match char_win(&windows_of(run.pid()), SAKURA) {
        Some(s) => verified_body_point(&s, &mut n),
        None => Err("ぱすたの窓が無い".into()),
    };
    if let Ok((x, y)) = clicked {
        mouse(MOUSEEVENTF_LEFTDOWN, x, y);
        sleep(Duration::from_millis(80));
        mouse(MOUSEEVENTF_LEFTUP, x, y);
        sleep(Duration::from_millis(600));
    }
    let fg = unsafe { GetForegroundWindow() };
    let mut fg_pid = 0u32;
    unsafe { GetWindowThreadProcessId(fg, Some(&mut fg_pid)) };
    n.note(format!(
        "click={clicked:?} foreground hwnd={:#x} pid={fg_pid} (areka pid={})",
        fg.0 as isize,
        run.pid()
    ));
    if fg_pid != run.pid() {
        // 入力先が areka でない: 近道のキーをほかのアプリへ送らない。
        let why = format!(
            "入力先が areka でないのでキーを送らない: click={clicked:?} foreground pid={fg_pid}（areka pid={}）",
            run.pid()
        );
        n.judge(
            "5a Ctrl+Shift+F10 で balloon_pos_reset（trigger=shortcut）",
            false,
            why.clone(),
        );
        n.judge("5b Ctrl+Shift+F12 で reset_pos_menu_outcome", false, why);
    } else {
        let b0 = lines_with(&run, "balloon_pos_reset").len();
        let sent = chord(VK_F10);
        let line = wait_new(&run, "balloon_pos_reset", b0, 5);
        n.judge(
            "5a Ctrl+Shift+F10 で balloon_pos_reset（trigger=shortcut）",
            line.as_deref()
                .is_some_and(|l| l.contains("shortcut") && !l.contains("skipped")),
            format!(
                "sent={sent} line={line:?} ignored={:?}",
                lines_with(&run, "position_shortcut_ignored")
            ),
        );
        sleep(Duration::from_secs(1));
        let r0 = lines_with(&run, "reset_pos_menu_outcome").len();
        let sent = chord(VK_F12);
        let line = wait_new(&run, "reset_pos_menu_outcome", r0, 10);
        n.judge(
            "5b Ctrl+Shift+F12 で reset_pos_menu_outcome",
            line.is_some(),
            format!(
                "sent={sent} line={line:?} ignored={:?}",
                lines_with(&run, "position_shortcut_ignored")
            ),
        );
    }
    sleep(Duration::from_secs(2));
    n.snapshot("run2_after_shortcuts", &windows_of(run.pid()));
    n.note(format!("key lines={:?}", lines_with(&run, "key_down")));
    let ended = run.close();
    n.note(format!("run2 ended={ended:?}"));
    let _ = std::fs::write(base.join("run2.log"), run.output());
    drop(run);

    n.save();
    let failed: Vec<_> = n
        .judgments
        .iter()
        .filter(|j| !j.1)
        .map(|j| j.0.clone())
        .collect();
    assert!(
        failed.is_empty(),
        "赤の判定: {failed:?}（{}）",
        base.join("judgments.txt").display()
    );
}

/// JSON の文字列にする（逆斜線と引用符だけを逃がす）。
fn json(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}
