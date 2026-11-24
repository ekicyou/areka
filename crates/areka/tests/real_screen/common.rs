//! 実画面の試験が共有する補助（入力の送出・記録の待ち・判定の控え・写し）。
//!
//! 各試験が `#[path]` で取り込むので、使わない項目が出る。

#![allow(dead_code)]

use real_run_kit::Run;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::thread::sleep;
use std::time::{Duration, Instant};
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetThreadDpiAwarenessContext,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_MOUSE, MOUSE_EVENT_FLAGS, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_MOVE,
    MOUSEEVENTF_VIRTUALDESK, MOUSEINPUT, SendInput,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
};

pub fn per_monitor() {
    // SAFETY: 呼ぶスレッドだけの DPI 認識を切り替える。ポインタを渡さず、前の値は使わない。
    unsafe {
        SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
}

/// 仮想画面の絶対座標へカーソルを動かし、`flags` のボタン操作を送る。送れた入力の数を返す。
pub fn mouse(flags: MOUSE_EVENT_FLAGS, x: i32, y: i32) -> u32 {
    per_monitor();
    // SAFETY: 引数も出力のポインタも無い値取得で、いつ呼んでもよい。
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
    // SAFETY: 初期化済みの INPUT 1 件の配列を渡し、大きさ引数は INPUT 自身の大きさ。
    unsafe { SendInput(&[input], size_of::<INPUT>() as i32) }
}

/// 記録の中の `needle` の行（標準出力＋標準エラー）。
pub fn lines_with(run: &Run, needle: &str) -> Vec<String> {
    run.output()
        .lines()
        .filter(|l| l.contains(needle))
        .map(str::to_string)
        .collect()
}

/// `needle` を含む行が `count` 本を超えるまで待つ（新しく出た最初の行を返す）。
pub fn wait_new(run: &Run, needle: &str, count: usize, secs: u64) -> Option<String> {
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

/// JSON の文字列にする（逆斜線と引用符だけを逃がす）。
pub fn json(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

/// `from` の中身を `to` へ再帰で写す（写せなければ場所と理由で落とす）。
pub fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap_or_else(|e| panic!("{} を作れません: {e}", to.display()));
    for entry in std::fs::read_dir(from).unwrap_or_else(|e| panic!("{}: {e}", from.display())) {
        let path = entry.unwrap().path();
        let dest = to.join(path.file_name().unwrap());
        if path.is_dir() {
            copy_dir(&path, &dest);
        } else {
            std::fs::copy(&path, &dest)
                .unwrap_or_else(|e| panic!("{} を写せません: {e}", path.display()));
        }
    }
}

/// 判定と手順の控え。`tag` は標準エラーへ出す行の頭の印。
pub struct Notes {
    pub tag: &'static str,
    pub base: PathBuf,
    pub steps: String,
    pub judgments: Vec<(String, bool, String)>,
}

impl Notes {
    pub fn new(tag: &'static str, base: PathBuf) -> Self {
        Self {
            tag,
            base,
            steps: String::new(),
            judgments: Vec::new(),
        }
    }
    pub fn note(&mut self, s: impl AsRef<str>) {
        eprintln!("[{}] {}", self.tag, s.as_ref());
        let _ = writeln!(
            self.steps,
            "{:?} {}",
            std::time::SystemTime::now(),
            s.as_ref()
        );
    }
    pub fn judge(&mut self, name: &str, ok: bool, detail: impl Into<String>) {
        let detail = detail.into();
        self.note(format!(
            "JUDGE {} {name}: {detail}",
            if ok { "PASS" } else { "FAIL" }
        ));
        self.judgments.push((name.to_string(), ok, detail));
    }
    /// `steps.txt` と `judgments.txt` を `base` へ書く。
    pub fn save(&self) {
        let _ = std::fs::write(self.base.join("steps.txt"), &self.steps);
        let mut j = String::new();
        for (n, ok, d) in &self.judgments {
            let _ = writeln!(j, "{} {n}: {d}", if *ok { "PASS" } else { "FAIL" });
        }
        let _ = std::fs::write(self.base.join("judgments.txt"), j);
    }
}
