//! stay-on-top-directives タスク 7.1 の実機の確かめ（要件 8.1・8.2・8.4・既定 OFF）。
//!
//! 検体 emo2 を `real-run-kit` で起こし、試験のプロセスが出す「他のアプリの窓」（全面を覆う普通の窓）を
//! 前に出してから（`HWND_TOP` に続けて、ゴーストの窓の外の点を `SendInput` でクリックする）台本を送り、常に手前のオン・オフ・`\v`・後から出たバルーン・`\![set,zorder]` の組・
//! シェルの切り替え・ドラッグ・ゴーストの切り替えの後の重なりを、窓の重なりの順（`GetTopWindow` から
//! `GW_HWNDNEXT` で辿った順位）と `WS_EX_TOPMOST` を読んで判定する。R1（持ち主だけを帯へ上げると
//! 持たれる窓も上がるか）は、試験のプロセスから areka のキャラクターの窓へ `HWND_TOPMOST` を 1 回だけ
//! 当てて測り、すぐに戻す。
//!
//! 本物の窓を出し、ドラッグを `SendInput` で送るので、排他テストの机を持ち、画面のロックが無いときだけ回す。
//! 有効にするのは環境変数 `AREKA_STAY_ON_TOP_REAL_RUN`（空でない値）。未設定なら即 return する。
//! 画面が使えない（スクリーンセーバー・入力先の窓が無い）ときは areka を起こす前に、オンの前に他のアプリの
//! 窓がゴーストの窓より手前に来ない（8.1b）ときはその場で、無効な走りとして止める。
//! 先に要るもの: 32bit の仲介 `cargo build -p shiori-host32-helper --target i686-pc-windows-msvc`。
//!
//! 根・記憶の置き場・記録はすべて `target\sot-signoff\` の下。根は emo2 の複製に、シェルの写し `copy`
//! （名 `copy`）と、ゴーストの写し `ghost\emo2b`（名 `emo2b`・切り替えの先）を足して作る。
//! 判定は `judgments.txt`、手順と重なりの控えは `steps.txt`、走りの記録は `run.log`。

use real_run_kit::{Launch, RealRun, Run};
use sample_ghost_kit::SampleRoot;
use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::Path;
use std::sync::mpsc;
use std::thread::sleep;
use std::time::{Duration, Instant};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetThreadDpiAwarenessContext,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_MOUSE, MOUSE_EVENT_FLAGS, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_LEFTDOWN,
    MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MOVE, MOUSEEVENTF_VIRTUALDESK, MOUSEINPUT, SendInput,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GW_HWNDNEXT, GW_OWNER, GWL_EXSTYLE,
    GetForegroundWindow, GetMessageW, GetSystemMetrics, GetTopWindow, GetWindow, GetWindowLongPtrW,
    GetWindowRect, GetWindowTextW, GetWindowThreadProcessId, HWND_NOTOPMOST, HWND_TOP,
    HWND_TOPMOST, IsWindowVisible, MSG, RegisterClassW, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN,
    SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SPI_GETSCREENSAVERRUNNING, SWP_NOACTIVATE, SWP_NOMOVE,
    SWP_NOSIZE, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SetWindowPos, SystemParametersInfoW,
    TranslateMessage, WINDOW_EX_STYLE, WNDCLASSW, WS_EX_TOPMOST, WS_OVERLAPPEDWINDOW, WS_VISIBLE,
    WindowFromPoint,
};
use windows::core::{BOOL, w};

const REAL_RUN_ENV: &str = "AREKA_STAY_ON_TOP_REAL_RUN";
/// 判定の分岐（受け口・状態・帯・組・鎖・シェルの切り替えの組の保ち）を debug まで開ける。
const RUST_LOG: &str = "info,kanade=debug,areka::emo2_boot::window_state_cue=debug,\
areka::emo2_boot::zorder_cue=debug,areka::emo2_boot::frame::window_state=debug,\
areka::emo2_boot::frame::zorder_drain=debug,areka::emo2_boot::frame::zorder_descript=debug,\
areka::emo2_boot::frame::switch=debug,wintf::ecs::window=debug";
const SAKURA: &str = "むらさき";
const KERO: &str = "エモ";
const STATE_CHANGED: &str = "[window_state] 常に手前の状態が変わった";

/// 重なりの順の 1 枚（順位 0 が最も手前）。
#[derive(Clone, Debug)]
struct Z {
    hwnd: isize,
    rank: usize,
    pid: u32,
    visible: bool,
    topmost: bool,
    owner: isize,
    title: String,
    rect: [i32; 4],
}

/// ある時点の全部の最上位の窓と、areka の窓の分け方。
struct Snap {
    all: Vec<Z>,
    /// areka のキャラクターの窓（持たれる窓を持つ窓）。
    chars: Vec<isize>,
    /// バルーンの窓 → 自分のキャラクターの窓。
    balloons: HashMap<isize, isize>,
    other: Option<Z>,
}

impl Snap {
    fn get(&self, h: isize) -> Option<&Z> {
        self.all.iter().find(|z| z.hwnd == h)
    }
    /// 帯の働きの的（キャラクター＋バルーン・隠れた窓も含む）を手前から。
    fn targets(&self) -> Vec<&Z> {
        self.all
            .iter()
            .filter(|z| self.chars.contains(&z.hwnd) || self.balloons.contains_key(&z.hwnd))
            .collect()
    }
    fn visible_targets(&self) -> Vec<&Z> {
        self.targets().into_iter().filter(|z| z.visible).collect()
    }
    fn char_titled(&self, title: &str) -> Option<&Z> {
        self.chars
            .iter()
            .filter_map(|h| self.get(*h))
            .find(|z| z.title == title)
    }
    fn all_topmost(&self, on: bool) -> bool {
        let t = self.targets();
        !t.is_empty() && t.iter().all(|z| z.topmost == on)
    }
    /// 見えている的が全部「他のアプリの窓」より手前（`front=true`）／奥（false）。
    fn vs_other(&self, front: bool) -> bool {
        let Some(o) = &self.other else { return false };
        let v = self.visible_targets();
        !v.is_empty() && v.iter().all(|z| (z.rank < o.rank) == front)
    }
    /// 見えているバルーンが、自分のキャラクターのすぐ手前（間に見えている窓が無い）か。
    fn balloons_just_in_front(&self) -> (bool, String) {
        let mut ok = true;
        let mut out = String::new();
        for (b, c) in &self.balloons {
            let (Some(bz), Some(cz)) = (self.get(*b), self.get(*c)) else {
                continue;
            };
            if !bz.visible || !cz.visible {
                continue;
            }
            let between = self
                .all
                .iter()
                .filter(|z| z.visible && z.rank > bz.rank && z.rank < cz.rank)
                .count();
            let fine = bz.rank < cz.rank && between == 0;
            ok &= fine;
            let _ = write!(
                out,
                "{:#x}({}) rank {}→char {:#x} rank {} between={between}; ",
                b, bz.title, bz.rank, c, cz.rank
            );
        }
        (ok && !out.is_empty(), out)
    }
    /// 的の手前からの並び（hwnd）。
    fn order(&self) -> Vec<isize> {
        self.targets().iter().map(|z| z.hwnd).collect()
    }
}

/// 2 つの並びの、両方に在る窓の相対の順が同じか。
fn same_relative_order(a: &[isize], b: &[isize]) -> bool {
    let fa: Vec<_> = a.iter().filter(|h| b.contains(h)).collect();
    let fb: Vec<_> = b.iter().filter(|h| a.contains(h)).collect();
    !fa.is_empty() && fa == fb
}

fn per_monitor() {
    unsafe {
        SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
}

fn zinfo(h: HWND, rank: usize) -> Z {
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(h, Some(&mut pid)) };
    let ex = unsafe { GetWindowLongPtrW(h, GWL_EXSTYLE) } as u32;
    let mut buf = [0u16; 256];
    let n = unsafe { GetWindowTextW(h, &mut buf) } as usize;
    let mut r = Default::default();
    let _ = unsafe { GetWindowRect(h, &mut r) };
    let r: windows::Win32::Foundation::RECT = r;
    Z {
        hwnd: h.0 as isize,
        rank,
        pid,
        visible: unsafe { IsWindowVisible(h) }.as_bool(),
        topmost: ex & WS_EX_TOPMOST.0 != 0,
        owner: unsafe { GetWindow(h, GW_OWNER) }.map_or(0, |o| o.0 as isize),
        title: String::from_utf16_lossy(&buf[..n]),
        rect: [r.left, r.top, r.right, r.bottom],
    }
}

/// 最上位の窓を最も手前から辿る（上限 4096 枚）。
fn zorder() -> Vec<Z> {
    per_monitor();
    let mut out = Vec::new();
    let mut h = unsafe { GetTopWindow(None) }.ok();
    while let Some(cur) = h.filter(|h| !h.is_invalid()) {
        out.push(zinfo(cur, out.len()));
        if out.len() >= 4096 {
            break;
        }
        h = unsafe { GetWindow(cur, GW_HWNDNEXT) }.ok();
    }
    out
}

/// 記録の `owner-established` の行から（持たれる窓 → 持ち主）。組で持ち主を外した後も分けられるように。
fn owner_map_from_log(run: &Run) -> HashMap<isize, isize> {
    let field = |l: &str, k: &str| -> Option<isize> {
        let v = l.split(k).nth(1)?.split_whitespace().next()?;
        isize::from_str_radix(v.trim_start_matches("0x"), 16).ok()
    };
    run.output()
        .lines()
        .filter(|l| l.contains("[zorder-pair] owner-established"))
        .filter_map(|l| Some((field(l, "owned_hwnd=")?, field(l, "owner_hwnd=")?)))
        .collect()
}

fn snap(run: &Run, other: isize) -> Snap {
    let all = zorder();
    let pid = run.pid();
    let mine: HashMap<isize, &Z> = all
        .iter()
        .filter(|z| z.pid == pid)
        .map(|z| (z.hwnd, z))
        .collect();
    // 分け方は記録の組だけで作る（`GW_OWNER` は組の鎖や IME の窓を拾うので使わない）。
    let balloons: HashMap<isize, isize> = owner_map_from_log(run)
        .into_iter()
        .filter(|(b, c)| mine.contains_key(b) && mine.contains_key(c))
        .collect();
    let mut chars: Vec<isize> = balloons.values().copied().collect();
    chars.sort();
    chars.dedup();
    let other = all.iter().find(|z| z.hwnd == other).cloned();
    Snap {
        all,
        chars,
        balloons,
        other,
    }
}

/// 判定と手順の控え。
struct Notes {
    steps: String,
    judgments: Vec<(String, bool, String)>,
}

impl Notes {
    fn note(&mut self, s: impl AsRef<str>) {
        eprintln!("[sot] {}", s.as_ref());
        let _ = writeln!(
            self.steps,
            "{:?} {}",
            std::time::SystemTime::now(),
            s.as_ref()
        );
    }
    fn judge(&mut self, name: &str, ok: bool, detail: impl Into<String>) {
        let detail = detail.into();
        self.note(format!(
            "JUDGE {} {name}: {detail}",
            if ok { "PASS" } else { "FAIL" }
        ));
        self.judgments.push((name.to_string(), ok, detail));
    }
    /// 的と他のアプリの窓の順位・帯の印を控える。
    fn record(&mut self, label: &str, s: &Snap) {
        let mut rows: Vec<&Z> = s.targets();
        if let Some(o) = &s.other {
            rows.push(o);
        }
        rows.sort_by_key(|z| z.rank);
        for z in rows {
            let kind = if s.chars.contains(&z.hwnd) {
                "char".to_string()
            } else if let Some(c) = s.balloons.get(&z.hwnd) {
                format!("balloon(of {c:#x})")
            } else {
                "OTHER-APP".to_string()
            };
            self.note(format!(
                "z {label} rank={} hwnd={:#x} {kind} title={} visible={} topmost={} owner={:#x} rect={:?}",
                z.rank, z.hwnd, z.title, z.visible, z.topmost, z.owner, z.rect
            ));
        }
    }
}

fn lines_with(run: &Run, needle: &str) -> Vec<String> {
    run.output()
        .lines()
        .filter(|l| l.contains(needle))
        .map(str::to_string)
        .collect()
}

/// `needle` を含む行が `count` 本を超えるまで待つ（新しく出た最初の行を返す）。
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

fn json(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

/// 台本を送る。話している最中で断られたら 1 秒ごとに送り直す（最大 15 回）。
fn send(run: &mut Run, n: &mut Notes, script: &str) {
    for attempt in 0..=15 {
        let reply = run.call("sakurascript", &format!("{{\"script\":{}}}", json(script)));
        n.note(format!("send {script} attempt={attempt} reply={reply:?}"));
        let refused = reply
            .as_ref()
            .is_ok_and(|r| r.text.contains("cannot accept a script now"));
        if !refused {
            return;
        }
        sleep(Duration::from_secs(1));
    }
}

/// 状態の変わった行を待って送る（オン・オフ）。
fn send_state(run: &mut Run, n: &mut Notes, script: &str) -> Option<String> {
    let c = lines_with(run, STATE_CHANGED).len();
    send(run, n, script);
    let line = wait_new(run, STATE_CHANGED, c, 10);
    sleep(Duration::from_millis(800));
    line
}

/// 試験のプロセスの「他のアプリの窓」。専用のスレッドで作り、メッセージを回し続ける
/// （回さないと OS が応答なしの影の窓に差し替え、重なりの読みが狂う）。
fn spawn_other_window(rect: [i32; 4]) -> isize {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        per_monitor();
        unsafe extern "system" fn proc_(h: HWND, m: u32, w: WPARAM, l: LPARAM) -> LRESULT {
            unsafe { DefWindowProcW(h, m, w, l) }
        }
        let inst = unsafe { GetModuleHandleW(None) }.unwrap();
        let class = WNDCLASSW {
            lpfnWndProc: Some(proc_),
            hInstance: inst.into(),
            lpszClassName: w!("areka_sot_other_app"),
            ..Default::default()
        };
        unsafe { RegisterClassW(&class) };
        let h = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("areka_sot_other_app"),
                w!("stay-on-top other app"),
                WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                rect[0],
                rect[1],
                rect[2] - rect[0],
                rect[3] - rect[1],
                None,
                None,
                Some(inst.into()),
                None,
            )
        }
        .unwrap();
        tx.send(h.0 as isize).unwrap();
        let mut msg = MSG::default();
        while unsafe { GetMessageW(&mut msg, None, 0, 0) }.as_bool() {
            unsafe {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    });
    rx.recv().unwrap()
}

/// 窓を（入力先を変えずに）その帯の最上へ。
fn place(h: isize, after: HWND) -> Result<(), String> {
    unsafe {
        SetWindowPos(
            HWND(h as _),
            Some(after),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        )
    }
    .map_err(|e| format!("{e:?}"))
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

/// キャラクターの窓の上で、点の下がその窓になる点（中心に近い順に 9×9 の格子から）。
/// areka はカーソルの無い所で窓を透ける形にするので、点ごとにカーソルを乗せて 300 ms 待ってから見る。
fn hit_point(c: &Z) -> Option<(i32, i32)> {
    per_monitor();
    let (w, h) = (c.rect[2] - c.rect[0], c.rect[3] - c.rect[1]);
    let mut pts: Vec<(i32, i32)> = (1..10)
        .flat_map(|i| (1..10).map(move |j| (c.rect[0] + w * i / 10, c.rect[1] + h * j / 10)))
        .collect();
    let (cx, cy) = (c.rect[0] + w / 2, c.rect[1] + h / 2);
    pts.sort_by_key(|(x, y)| (x - cx).pow(2) + (y - cy).pow(2));
    pts.into_iter().find(|(x, y)| {
        mouse(MOUSE_EVENT_FLAGS(0), *x, *y);
        sleep(Duration::from_millis(300));
        unsafe { WindowFromPoint(POINT { x: *x, y: *y }) }.0 as isize == c.hwnd
    })
}

/// バルーンの可視の遷移の行（`scope` が `trigger` で `visible` になった）。
fn balloon_line(scope: u32, trigger: &str, visible: bool) -> String {
    format!(
        "[balloon-visibility] バルーンの可視状態が遷移した scope={scope} trigger=\"{trigger}\" visible={visible}"
    )
}

/// 2 つの scope のバルーンの遷移の行の今の本数。
fn balloon_counts(run: &Run, trigger: &str, visible: bool) -> [usize; 2] {
    [0, 1].map(|sc| lines_with(run, &balloon_line(sc, trigger, visible)).len())
}

/// R1 の測り: `root` だけへ `HWND_TOPMOST` を当て、`root` から持たれる鎖の全部の帯の印を読み、
/// 鎖の全部を `HWND_NOTOPMOST` で戻す。答えは合否でない（設計はどちらの答えでも正しく動く形）。
fn measure_r1(n: &mut Notes, label: &str, root: isize) {
    let all = zorder();
    let mut chain = vec![root];
    // 持ち主が鎖に入っている窓を鎖へ足す（鎖は短いので素朴にくり返す）。
    while let Some(z) = all
        .iter()
        .find(|z| z.owner != 0 && chain.contains(&z.owner) && !chain.contains(&z.hwnd))
    {
        chain.push(z.hwnd);
    }
    let r = place(root, HWND_TOPMOST);
    sleep(Duration::from_millis(300));
    let after = zorder();
    let top = |h: isize| {
        after
            .iter()
            .find(|z| z.hwnd == h)
            .map(|z| (z.topmost, z.title.clone()))
    };
    let owned_up = chain.len() > 1 && chain[1..].iter().all(|h| top(*h).is_some_and(|t| t.0));
    n.note(format!(
        "MEASURE R1 {label}: 当てた窓 {root:#x}（持ち主 {:#x}）set={r:?} 持たれる窓も上がる={owned_up} 鎖={:?}",
        all.iter().find(|z| z.hwnd == root).map_or(0, |z| z.owner),
        chain.iter().map(|h| (format!("{h:#x}"), top(*h))).collect::<Vec<_>>()
    ));
    for h in chain.iter().rev() {
        let _ = place(*h, HWND_NOTOPMOST);
    }
    sleep(Duration::from_millis(300));
}

/// 画面が使えない理由（スクリーンセーバーが動いている・入力先の窓が無い）。使えれば `None`。
/// shortcut: 入力の desktop の名は読まない（`Win32_System_StationsAndDesktops` が要る）。ロック中は
/// 入力先の窓が無いので、今はそれで拾う。拾えない形が出たら足す。
fn screen_unavailable() -> Option<String> {
    let mut running = BOOL(0);
    let r = unsafe {
        SystemParametersInfoW(
            SPI_GETSCREENSAVERRUNNING,
            0,
            Some((&raw mut running).cast()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
    };
    if r.is_err() || running.as_bool() {
        return Some(format!(
            "スクリーンセーバーが動いている（読みの返り {r:?}）"
        ));
    }
    if unsafe { GetForegroundWindow() }.0.is_null() {
        return Some("入力先の窓が無い（画面のロック中など）".into());
    }
    None
}

fn foreground_pid() -> u32 {
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(GetForegroundWindow(), Some(&mut pid)) };
    pid
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

/// `descript.txt`（UTF-8）の `name,` の行を差し替える。
fn rename_ghost(descript: &Path, name: &str) {
    let text = std::fs::read_to_string(descript).unwrap();
    let out: Vec<String> = text
        .lines()
        .map(|l| {
            if l.starts_with("name,") {
                format!("name,{name}")
            } else {
                l.to_string()
            }
        })
        .collect();
    std::fs::write(descript, out.join("\r\n") + "\r\n").unwrap();
}

#[test]
fn stay_on_top_directives_on_emo2() {
    if std::env::var(REAL_RUN_ENV)
        .map(|v| v.trim().is_empty())
        .unwrap_or(true)
    {
        return;
    }
    // 画面が使えない走りは無効（3 回目はスクリーンセーバーの間に走った）。areka を起こす前に止める。
    if let Some(why) = screen_unavailable() {
        panic!("無効な走り: {why}。画面のロックとスクリーンセーバーが無い間に回す");
    }
    let target =
        std::path::absolute(Path::new(env!("CARGO_MANIFEST_DIR")).join(r"..\..\target")).unwrap();
    assert!(!target.to_string_lossy().starts_with(r"\\?\"));
    let base = target.join("sot-signoff");
    if base.exists() {
        std::fs::remove_dir_all(&base).unwrap();
    }
    let root = base.join("root");
    let profile = base.join("profile");
    {
        let sample = SampleRoot::acquire("emo2").expect("検体 emo2");
        sample
            .add_shell_copy("master", "copy", "copy")
            .expect("シェルの写し");
        copy_dir(sample.root(), &root);
    }
    copy_dir(&root.join(r"ghost\emo2"), &root.join(r"ghost\emo2b"));
    rename_ghost(
        &root.join(r"ghost\emo2b\ghost\master\descript.txt"),
        "emo2b",
    );
    std::fs::create_dir_all(&profile).unwrap();
    let _ = std::fs::remove_dir_all(root.join(".nar-work"));

    let mut n = Notes {
        steps: String::new(),
        judgments: Vec::new(),
    };
    n.note(format!(
        "root={} profile={} rust_log={RUST_LOG}",
        root.display(),
        profile.display()
    ));
    let mut run = RealRun::new(env!("CARGO_BIN_EXE_areka"))
        .launch(Launch::Root(root.clone()))
        .profile_dir(&profile)
        .rust_log(RUST_LOG)
        .safety_exit(Duration::from_secs(300))
        .deadline(Duration::from_secs(90))
        .spawn();
    n.note(format!("pid={} port={}", run.pid(), run.mcp_port()));
    let done = wait_new(&run, "event=\"steady_talk_done\"", 0, 60);
    n.note(format!("steady_talk_done={done:?}"));
    // 起動の挨拶のバルーンを閉じ、面を既定へ（後から出るバルーンを見分けるため）。
    send(&mut run, &mut n, r"\0\s[0]\1\s[10]\e");
    sleep(Duration::from_secs(3));

    // ---- 他のアプリの窓を前に出す ----
    let s = snap(&run, 0);
    let v = s.visible_targets();
    let rect = if v.is_empty() {
        [0, 0, 1600, 1200]
    } else {
        [
            v.iter().map(|z| z.rect[0]).min().unwrap() - 40,
            v.iter().map(|z| z.rect[1]).min().unwrap() - 40,
            v.iter().map(|z| z.rect[2]).max().unwrap() + 40,
            v.iter().map(|z| z.rect[3]).max().unwrap() + 40,
        ]
    };
    let other = spawn_other_window(rect);
    sleep(Duration::from_millis(500));
    // 利用者と同じく、ゴーストの窓の外の点をクリックして前に出す（裏のプロセスの `HWND_TOP` だけでは
    // OS が並びを変えないことがある・4 回目）。点は左の縁の中ほど（ゴーストの窓の和より 40 px 外側）。
    // 他のアプリの窓は `DefWindowProcW` だけなので、クリックは活性になる以外に何も起こさない。
    let click_at = (rect[0] + 20, (rect[1] + rect[3]) / 2);
    let raise_other = |n: &mut Notes, label: &str| {
        let r = place(other, HWND_TOP);
        per_monitor();
        let (x, y) = click_at;
        let hit = unsafe { WindowFromPoint(POINT { x, y }) }.0 as isize;
        if hit == other {
            mouse(MOUSEEVENTF_LEFTDOWN, x, y);
            mouse(MOUSEEVENTF_LEFTUP, x, y);
        }
        sleep(Duration::from_millis(500));
        n.note(format!(
            "raise other app ({label}) => {r:?} click=({x},{y}) clicked={} hit={hit:#x}",
            hit == other
        ));
    };
    raise_other(&mut n, "start");
    let s0 = snap(&run, other);
    n.record("s0_other_front", &s0);
    n.note(format!(
        "foreground pid={} areka pid={}",
        foreground_pid(),
        run.pid()
    ));
    n.judge(
        "0 前置き: 他のアプリの窓が全部の見えているゴーストの窓より手前・帯の印なし",
        s0.vs_other(false) && s0.all_topmost(false),
        format!("chars={} balloons={}", s0.chars.len(), s0.balloons.len()),
    );

    // ---- 8.1 オフのあいだの \v（areka は裏のプロセス） ----
    let fg = foreground_pid();
    let c = lines_with(&run, "ゴーストの窓を今いる帯の最上へ出す").len();
    send(&mut run, &mut n, r"\v\e");
    let line = wait_new(&run, "ゴーストの窓を今いる帯の最上へ出す", c, 10);
    sleep(Duration::from_millis(800));
    let s1 = snap(&run, other);
    n.record("s1_v_off", &s1);
    n.judge(
        "8.1a オフの \\v で並びを保って他のアプリの窓の手前へ 1 回出る（帯の印なし）",
        s1.vs_other(true) && s1.all_topmost(false) && same_relative_order(&s0.order(), &s1.order()),
        format!(
            "line={line:?} 送る前の入力先 pid={fg}（areka={}） order0={:x?} order1={:x?}",
            run.pid(),
            s0.order(),
            s1.order()
        ),
    );
    let fg_after = foreground_pid();
    n.judge(
        "R-bg 裏のプロセスから手前へ出す指令で入力先の窓の手前へ出る・入力先は変わらない",
        fg != run.pid() && s1.vs_other(true) && fg_after == fg,
        format!("入力先 pid 前={fg} 後={fg_after} areka pid={}", run.pid()),
    );
    raise_other(&mut n, "after v");
    let s2 = snap(&run, other);
    n.record("s2_other_again", &s2);
    n.judge(
        "8.1b その後に他のアプリを前に出すと後ろへ回る",
        s2.vs_other(false),
        String::new(),
    );
    if !s2.vs_other(false) {
        // 直したことは、オンが他のアプリの奥から始まるときだけ示せる（2 回目の赤の形）。
        let _ = run.close();
        let _ = std::fs::write(base.join("run.log"), run.output());
        let _ = std::fs::write(base.join("steps.txt"), &n.steps);
        panic!(
            "無効な走り: オンの前に他のアプリの窓がゴーストの窓より手前に来ない（8.1b）。画面のロックと\
             スクリーンセーバーを確かめて回し直す（{}）",
            base.join("steps.txt").display()
        );
    }

    // ---- 8.1 オン ----
    // 後から出るバルーンを見分けるため、同じ台本でバルーンを閉じてからオンにする（emo2 のバルーンは
    // 自分では閉じず、独り言でまた開くので、時間で待たない）。閉じたかは記録の行で控える。
    let closed = balloon_counts(&run, "clear", false);
    let line = send_state(
        &mut run,
        &mut n,
        r"\0\s[0]\1\s[10]\![set,windowstate,stayontop]\e",
    );
    let now = balloon_counts(&run, "clear", false);
    n.note(format!(
        "balloons closed with on: scope0={} scope1={}",
        now[0] > closed[0],
        now[1] > closed[1]
    ));
    let s3 = snap(&run, other);
    n.record("s3_on", &s3);
    n.judge(
        "8.1c オンの後にキャラクターとバルーンの窓が手前（全部の窓に帯の印・隠れた窓も）",
        line.is_some() && s3.all_topmost(true) && s3.vs_other(true),
        format!("line={line:?}"),
    );
    n.judge(
        "R3 奥から順に当てて並びが保たれる（オンの前後）",
        same_relative_order(&s2.order(), &s3.order()),
        format!("before={:x?} after={:x?}", s2.order(), s3.order()),
    );
    raise_other(&mut n, "while on");
    let s4 = snap(&run, other);
    n.record("s4_on_other_raised", &s4);
    n.judge(
        "8.1d オンのあいだは他のアプリを前に出しても手前のまま",
        s4.vs_other(true) && s4.all_topmost(true),
        String::new(),
    );

    // ---- 8.1 後から出たバルーン・8.2 すぐ手前 ----
    let shown = balloon_counts(&run, "content", true);
    send(
        &mut run,
        &mut n,
        r"\0\s[0]手前のテスト\1\s[10]手前のテスト\e",
    );
    // 開いたことは時間でなく記録の行で待つ（2 つの scope とも）。
    for sc in [0, 1] {
        let l = wait_new(
            &run,
            &balloon_line(sc, "content", true),
            shown[sc as usize],
            10,
        );
        n.note(format!("balloon shown after on scope={sc} line={l:?}"));
    }
    sleep(Duration::from_millis(500));
    let s5 = snap(&run, other);
    n.record("s5_later_balloon", &s5);
    // 後から出たバルーン＝オンの行より後に `content visible=true` の行が出た scope のバルーン。
    // areka が閉じたバルーンの窓も IsWindowVisible は真なので、窓の可視では見分けない。
    let out = run.output();
    let after_on = line
        .as_deref()
        .and_then(|l| out.split_once(l))
        .map_or("", |(_, rest)| rest);
    let later: Vec<&Z> = [(0, SAKURA), (1, KERO)]
        .into_iter()
        .filter(|(sc, _)| after_on.contains(&balloon_line(*sc, "content", true)))
        .filter_map(|(_, title)| {
            s5.balloons
                .keys()
                .filter_map(|b| s5.get(*b))
                .find(|z| z.title == title)
        })
        .collect();
    let other_rank = s5.other.as_ref().map_or(0, |o| o.rank);
    n.judge(
        "8.1e 後から出たバルーンの窓も手前（帯の印・他のアプリより手前）",
        !later.is_empty() && later.iter().all(|z| z.topmost && z.rank < other_rank),
        format!(
            "オンの後に開いたバルーン={:x?}",
            later.iter().map(|z| (z.hwnd, &z.title)).collect::<Vec<_>>()
        ),
    );
    let (ok, d) = s5.balloons_just_in_front();
    n.judge(
        "8.2a オンのあいだバルーンが自分のキャラクターのすぐ手前",
        ok,
        d,
    );

    // ---- 8.2 \![set,zorder] の組の並び ----
    send(&mut run, &mut n, r"\![set,zorder,1,0]\e");
    sleep(Duration::from_secs(1));
    let s6 = snap(&run, other);
    n.record("s6_zorder_1_0", &s6);
    let kero_front = |s: &Snap| match (s.char_titled(KERO), s.char_titled(SAKURA)) {
        (Some(k), Some(sa)) => Some(k.rank < sa.rank),
        _ => None,
    };
    n.judge(
        "8.2b オンのあいだ \\![set,zorder,1,0] の並び（\\1 が \\0 より手前）が保たれ、帯から落ちない",
        kero_front(&s6) == Some(true) && s6.all_topmost(true),
        format!("kero_front={:?}", kero_front(&s6)),
    );

    // ---- 8.4・R2 シェルの切り替え ----
    let c = lines_with(&run, "skin_switch_done").len();
    send(&mut run, &mut n, r"\![change,shell,copy]\e");
    let line = wait_new(&run, "skin_switch_done", c, 30);
    sleep(Duration::from_secs(2));
    let s7 = snap(&run, other);
    n.record("s7_shell_switched", &s7);
    n.judge(
        "8.4 シェルの切り替えの後に組（\\1 が手前）と常に手前の状態が残る（R2）",
        line.is_some()
            && kero_front(&s7) == Some(true)
            && s7.all_topmost(true)
            && s7.vs_other(true),
        format!("line={line:?} kero_front={:?}", kero_front(&s7)),
    );
    let (ok, d) = s7.balloons_just_in_front();
    n.note(format!("shell switched balloons_just_in_front ok={ok} {d}"));

    // ---- R2 組み替え ----
    send(&mut run, &mut n, r"\![reset,zorder]\![set,zorder,0,1]\e");
    sleep(Duration::from_secs(1));
    let s8 = snap(&run, other);
    n.record("s8_zorder_0_1", &s8);
    n.judge(
        "R2a オンのあいだの組み替えで帯から落ちない（\\0 が手前に替わる）",
        kero_front(&s8) == Some(false) && s8.all_topmost(true),
        format!("kero_front={:?}", kero_front(&s8)),
    );

    // ---- R2 ドラッグ（SendInput） ----
    match s8.char_titled(SAKURA).and_then(hit_point) {
        Some((x, y)) => {
            n.note(format!("drag from {x},{y}"));
            mouse(MOUSE_EVENT_FLAGS(0), x, y);
            sleep(Duration::from_millis(300));
            mouse(MOUSEEVENTF_LEFTDOWN, x, y);
            sleep(Duration::from_millis(150));
            for i in 1..=15 {
                mouse(MOUSE_EVENT_FLAGS(0), x + 8 * i, y - 4 * i);
                sleep(Duration::from_millis(40));
            }
            sleep(Duration::from_millis(300));
            mouse(MOUSEEVENTF_LEFTUP, x + 120, y - 60);
            sleep(Duration::from_secs(1));
            let s9 = snap(&run, other);
            n.record("s9_dragged", &s9);
            let moved =
                s9.char_titled(SAKURA).map(|z| z.rect) != s8.char_titled(SAKURA).map(|z| z.rect);
            n.judge(
                "R2b オンのあいだのドラッグで帯から落ちない",
                moved && s9.all_topmost(true) && s9.vs_other(true),
                format!("moved={moved}"),
            );
        }
        None => n.judge(
            "R2b オンのあいだのドラッグで帯から落ちない",
            false,
            "押せる点が無い（点の下がキャラクターの窓にならない）",
        ),
    }

    // ---- オンのあいだの \v ----
    let before = snap(&run, other);
    send(&mut run, &mut n, r"\v\e");
    sleep(Duration::from_secs(1));
    let s10 = snap(&run, other);
    n.record("s10_v_on", &s10);
    n.judge(
        "8.1f オンのあいだの \\v は帯の中で並びを保つ（帯の印は付いたまま）",
        s10.all_topmost(true) && same_relative_order(&before.order(), &s10.order()),
        format!("before={:x?} after={:x?}", before.order(), s10.order()),
    );

    // ---- 8.1 オフ ----
    let line = send_state(&mut run, &mut n, r"\![set,windowstate,!stayontop]\e");
    let s11 = snap(&run, other);
    n.record("s11_off", &s11);
    raise_other(&mut n, "after off");
    let s12 = snap(&run, other);
    n.record("s12_off_other_raised", &s12);
    n.judge(
        "8.1g オフの後に帯の印が外れ、他のアプリを前に出すと後ろへ回る",
        line.is_some() && s11.all_topmost(false) && s12.vs_other(false),
        format!(
            "line={line:?} オフ直後の並び保持={}",
            same_relative_order(&s10.order(), &s11.order())
        ),
    );

    // ---- R1 持ち主だけを帯へ上げると持たれる窓も上がるか（試験から 1 回だけ当てて戻す） ----
    // ⑴ 鎖の根（持ち主の無い窓）へ当てる＝問いそのもの。⑵ 鎖の途中の窓（自分も持たれる）へ当てる＝1 回目の形。
    let root = s12
        .targets()
        .into_iter()
        .find(|z| z.owner == 0 && s12.chars.contains(&z.hwnd))
        .map(|z| z.hwnd);
    let middle = s12
        .chars
        .iter()
        .filter_map(|h| s12.get(*h))
        .find(|z| z.owner != 0)
        .map(|z| z.hwnd);
    match root {
        Some(h) => measure_r1(&mut n, "chain-root", h),
        None => n.note("MEASURE R1 chain-root 測れない: 持ち主の無いキャラクターの窓が無い"),
    }
    match middle {
        Some(h) => measure_r1(&mut n, "chain-middle", h),
        None => n.note("MEASURE R1 chain-middle 測れない: 持たれるキャラクターの窓が無い"),
    }
    let back = snap(&run, other);
    n.record("s12b_r1_restored", &back);
    n.note(format!(
        "R1 restored all_topmost(false)={}",
        back.all_topmost(false)
    ));

    // ---- 8.1 ゴーストの切り替えの後 ----
    let line = send_state(&mut run, &mut n, r"\![set,windowstate,stayontop]\e");
    let s13 = snap(&run, other);
    n.record("s13_on_again", &s13);
    let discard = lines_with(&run, "常に手前のまま降ろした").len();
    let switched = lines_with(&run, "ghost_switch_done").len();
    let talks = lines_with(&run, "event=\"steady_talk_done\"").len();
    send(&mut run, &mut n, r"\![change,ghost,emo2b]\e");
    let done = wait_new(&run, "ghost_switch_done", switched, 60);
    let disc = wait_new(&run, "常に手前のまま降ろした", discard, 5);
    let _ = wait_new(&run, "event=\"steady_talk_done\"", talks, 60);
    sleep(Duration::from_secs(2));
    let s14 = snap(&run, other);
    n.record("s14_ghost_switched", &s14);
    raise_other(&mut n, "after ghost switch");
    let s15 = snap(&run, other);
    n.record("s15_switched_other_raised", &s15);
    n.judge(
        "8.1h ゴーストの切り替えの後に帯の印が無く、他のアプリの後ろへ回る",
        line.is_some()
            && s13.all_topmost(true)
            && done.is_some()
            && s14.all_topmost(false)
            && s15.vs_other(false),
        format!("on={line:?} done={done:?} discard={disc:?}"),
    );

    // ---- 記録の確かめ ----
    let warns: Vec<String> = lines_with(&run, "[window_band]")
        .into_iter()
        .filter(|l| l.contains("WARN"))
        .collect();
    n.judge(
        "R2c 帯の働きの warn（帯を変えなかった窓・測れなかった窓）が無い",
        warns.is_empty(),
        format!("{} 行 {:?}", warns.len(), warns.first()),
    );
    n.note(format!(
        "window_band applied lines={}",
        lines_with(&run, "[window_band] applied").len()
    ));
    let ended = run.close();
    n.note(format!("ended={ended:?}"));
    let _ = std::fs::write(base.join("run.log"), run.output());
    drop(run);

    let _ = std::fs::write(base.join("steps.txt"), &n.steps);
    let mut j = String::new();
    for (name, ok, d) in &n.judgments {
        let _ = writeln!(j, "{} {name}: {d}", if *ok { "PASS" } else { "FAIL" });
    }
    let _ = std::fs::write(base.join("judgments.txt"), j);
    let failed: Vec<_> = n.judgments.iter().filter(|j| !j.1).map(|j| &j.0).collect();
    assert!(
        failed.is_empty(),
        "赤の判定: {failed:?}（{}）",
        base.join("judgments.txt").display()
    );
}
