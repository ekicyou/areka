//! 窓口自身の試験が使う偽の子（spec: areka-P0-real-run-mcp-kit 要件 10.1・設計「偽の子」）。
//!
//! 振る舞いは環境変数 `REAL_RUN_FAKE_STEPS` の手順（`;` 区切り・左から順）で選ぶ。
//! 手順が尽きたら終了コード 0 で終わる。知らない手順は標準エラーに書いて終了コード 64。
//!
//! - `out:<文>`・`err:<文>`: 標準出力・標準エラーへ 1 行。
//! - `flood:<KiB>`: 標準出力へその量（64 バイトの行の並び）を書く。
//! - `env`: 受け取った環境変数（[`ENV_NAMES`]）と位置の引数を 1 行ずつ出す
//!   （`env <名>=<値>`・無ければ `env <名> 無い`・引数は `arg <番号>=<値>`）。
//! - `listen`: `AREKA_MCP_PORT` の値で本物の `areka_mcp::start` を呼び、ツール `echo` を載せて立てたまま次へ。
//! - `listen-taken`: `AREKA_MCP_PORT` の番号（無ければ空き番号）を先に自分で束ねてから `start`
//!   （本物の「束ねられなかった」の行）。束ねた口と取っ手は立てたまま次へ。
//! - `listen-off`: `start(&[])`（本物の「待ち受けない」の行）。
//! - `listen-drop`: `start` の直後に取っ手を捨てて受け口を閉じ、生き続ける。
//! - `window`: 表示しない最上位の窓を 1 枚作って知らせを待つ。閉じる知らせで、本物の areka と同じ形の
//!   受け取りの行（`event="os_close_request"`）と終わり方の行（`event="app_exit" origin=KanadeStopped(Quit)`）を
//!   出所 `areka::app_exit` で出し、`REAL_RUN_FAKE_MARK` があればそのパスへ `closed` を書いて 0 で終わる。
//! - `window-deaf`: 同じ窓を作るが、閉じる知らせを無視する。
//! - `smoke`: `AREKA_APP_SMOKE_EXIT_MS` の時間の後に `event="app_exit" origin=Smoke` の行を出して 0 で終わる。
//! - `exit:<N>`: 終了コード N で終わる。`hang`: 何もせず生き続ける。
//!
//! 記録は areka と同じ `tracing-subscriber` の `fmt`（標準出力・着色なし・`RUST_LOG` で絞れる・既定 info）。
//! 窓口のライブラリは使わない（本物の受け口・窓の API・記録の部品だけ）。

use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use areka_mcp::{McpServer, ToolContent, ToolOutcome, ToolRegistry, ToolSpec};
use tracing_subscriber::EnvFilter;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, MSG, PostQuitMessage,
    RegisterClassW, WINDOW_EX_STYLE, WM_CLOSE, WNDCLASSW, WS_OVERLAPPEDWINDOW,
};
use windows::core::w;

/// `env` の手順で書き出す環境変数（窓口が子へ渡す名前）。
const ENV_NAMES: [&str; 8] = [
    "AREKA_ROOT",
    "AREKA_USER_ROOT",
    "AREKA_PROFILE_DIR",
    "AREKA_MCP_PORT",
    "AREKA_NO_ALERT",
    "NO_COLOR",
    "AREKA_APP_SMOKE_EXIT_MS",
    "RUST_LOG",
];

/// 終わり方の行の出所（本物の `quit_app`・`on_ghost_os_close` と同じ）。
const EXIT_TARGET: &str = "areka::app_exit";

/// `window-deaf` のとき真（窓の手続きが閉じる知らせを無視する）。
static DEAF: AtomicBool = AtomicBool::new(false);

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_ansi(false)
        .with_writer(std::io::stdout)
        .init();

    let steps = std::env::var("REAL_RUN_FAKE_STEPS").unwrap_or_default();
    // 立てた受け口（と先に束ねた口）は終わるまで持つ。
    let mut _alive: Vec<McpServer> = Vec::new();
    let mut _taken: Vec<std::net::TcpListener> = Vec::new();
    for step in steps.split(';').filter(|s| !s.is_empty()) {
        let (name, arg) = step.split_once(':').unwrap_or((step, ""));
        match name {
            "out" => println!("{arg}"),
            "err" => eprintln!("{arg}"),
            "flood" => flood(arg.parse().unwrap_or_else(|_| bad(step))),
            "env" => print_env(),
            "listen" => _alive.push(areka_mcp::start(
                &areka_mcp::read_port_candidates(),
                registry(),
                None,
            )),
            "listen-taken" => {
                let want = std::env::var(areka_mcp::PORT_ENV)
                    .ok()
                    .and_then(|v| v.trim().parse::<u16>().ok())
                    .unwrap_or(0);
                let taken = std::net::TcpListener::bind(("127.0.0.1", want))
                    .unwrap_or_else(|e| panic!("番号 {want} を先に束ねられなかった: {e}"));
                let port = taken.local_addr().expect("束ねた口の番地").port();
                _taken.push(taken);
                _alive.push(areka_mcp::start(&[port], registry(), None));
            }
            "listen-off" => _alive.push(areka_mcp::start(&[], registry(), None)),
            "listen-drop" => {
                drop(areka_mcp::start(
                    &areka_mcp::read_port_candidates(),
                    registry(),
                    None,
                ));
                hang();
            }
            "window" => window(false),
            "window-deaf" => window(true),
            "smoke" => {
                let ms = std::env::var("AREKA_APP_SMOKE_EXIT_MS")
                    .ok()
                    .and_then(|v| v.trim().parse::<u64>().ok())
                    .unwrap_or(0);
                std::thread::sleep(Duration::from_millis(ms));
                tracing::info!(
                    target: EXIT_TARGET,
                    event = "app_exit",
                    origin = %"Smoke",
                    closed = 0usize,
                    "[quit_app] 全窓を閉じ、終了を指示した"
                );
                quit(0);
            }
            "exit" => quit(arg.parse().unwrap_or_else(|_| bad(step))),
            "hang" => hang(),
            _ => bad(step),
        }
    }
    quit(0);
}

/// 知らない（読めない）手順: 標準エラーに書いて 64 で終わる。
fn bad(step: &str) -> ! {
    eprintln!("real-run-fake-areka: 知らない手順: {step:?}");
    quit(64);
}

/// 出力を流してから終わる（`process::exit` は破棄を走らせない）。
fn quit(code: i32) -> ! {
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
    std::process::exit(code);
}

fn hang() -> ! {
    loop {
        std::thread::park();
    }
}

fn flood(kib: usize) {
    let line = [b'x'; 63];
    let mut out = std::io::stdout().lock();
    for _ in 0..kib * 16 {
        out.write_all(&line).expect("標準出力へ書けない");
        out.write_all(b"\n").expect("標準出力へ書けない");
    }
}

fn print_env() {
    for name in ENV_NAMES {
        match std::env::var_os(name) {
            Some(v) => println!("env {name}={}", v.to_string_lossy()),
            None => println!("env {name} 無い"),
        }
    }
    for (i, a) in std::env::args_os().enumerate().skip(1) {
        println!("arg {i}={}", a.to_string_lossy());
    }
}

/// ツール `echo`: `text` を返す・`error` が真なら失敗の印・`die` が真なら答える前に 9 で終わる・
/// `after` が N なら N 回目の呼び出しから `text` を返し、それまでは空の文を返す。
fn registry() -> ToolRegistry {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "text": { "type": "string" },
            "error": { "type": "boolean" },
            "die": { "type": "boolean" },
            "after": { "type": "integer", "minimum": 1 }
        }
    });
    let spec = ToolSpec {
        name: "echo".to_owned(),
        title: Some("Echo".to_owned()),
        description: Some("returns text (fake areka for real-run-kit tests)".to_owned()),
        input_schema: schema.as_object().expect("object").clone(),
    };
    let calls = Arc::new(AtomicU64::new(0));
    let mut registry = ToolRegistry::default();
    registry.register(
        spec,
        Arc::new(move |args| {
            let n = calls.fetch_add(1, Ordering::SeqCst) + 1;
            Box::pin(async move {
                if args["die"].as_bool() == Some(true) {
                    quit(9);
                }
                let after = args["after"].as_u64().unwrap_or(1);
                let text = if n >= after {
                    args["text"].as_str().unwrap_or_default().to_owned()
                } else {
                    String::new()
                };
                ToolOutcome {
                    content: vec![ToolContent::Text(text)],
                    is_error: args["error"].as_bool() == Some(true),
                }
            })
        }),
    );
    registry
}

/// 表示しない最上位の窓を作り、知らせを回す。閉じる知らせを受けたら（`deaf` でなければ）0 で終わる。
fn window(deaf: bool) {
    DEAF.store(deaf, Ordering::SeqCst);
    // SAFETY: この関数のスレッドだけが窓を作って回す。引数はすべてこの場で作った有効な値。
    unsafe {
        let instance = GetModuleHandleW(None).expect("自分のモジュールの取っ手");
        let class = w!("RealRunFakeAreka");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(wnd_proc),
            hInstance: instance.into(),
            lpszClassName: class,
            ..Default::default()
        };
        if RegisterClassW(&wc) == 0 {
            panic!("窓の種類を登録できなかった");
        }
        // ShowWindow を呼ばないので表示されない。
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            class,
            w!("real-run-fake-areka"),
            WS_OVERLAPPEDWINDOW,
            0,
            0,
            100,
            100,
            None,
            None,
            Some(instance.into()),
            None,
        )
        .expect("窓を作れなかった");
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            DispatchMessageW(&msg);
        }
    }
    quit(0);
}

extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if msg == WM_CLOSE {
        if DEAF.load(Ordering::SeqCst) {
            return LRESULT(0);
        }
        tracing::info!(
            target: EXIT_TARGET,
            event = "os_close_request",
            scope = 0u32,
            kind = "ghost",
            "[os_close] OS の閉鎖要求をメニューの「終了」と同じ終了要求として kanade へ送る"
        );
        tracing::info!(
            target: EXIT_TARGET,
            event = "app_exit",
            origin = %"KanadeStopped(Quit)",
            closed = 1usize,
            "[quit_app] 全窓を閉じ、終了を指示した"
        );
        if let Some(mark) = std::env::var_os("REAL_RUN_FAKE_MARK") {
            std::fs::write(&mark, "closed").expect("目印のファイルを書けなかった");
        }
        // SAFETY: 窓を回しているスレッドの中から呼ぶ。
        unsafe { PostQuitMessage(0) };
        return LRESULT(0);
    }
    // SAFETY: 受けた引数をそのまま既定の手続きへ渡す。
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}
