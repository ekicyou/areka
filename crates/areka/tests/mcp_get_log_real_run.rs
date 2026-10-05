//! 実プロセス・実ソケットの `get_log` の試験（spec: areka-P0-mcp-log-history task 5.1・
//! 要件 1.1・1.7・1.8・4.6・4.7・6.3）。
//!
//! emo2 の検体を argv に渡して areka を起こし、手書きの HTTP/1.1 で `get_log`（`log_type=status`）を
//! 送る。`RUST_LOG=warn,areka::boot_config=info` で標準出力から消えた info が履歴に残ること・
//! 履歴の本文が標準出力の同じ行と 1 字も違わないこと（逆斜線入りのパスと二重引用符が JSON を
//! 通って戻る）・各行の頭の形と番号の昇順を判定する。
//!
//! 実装規律:
//! - 標準出力・標準エラーはパイプでなくファイルへ向ける（パイプが詰まって子が止まるのを避ける）。
//!   置き場は `CARGO_TARGET_TMPDIR`（ワークツリーの `target\tmp`）の下だけ。
//! - 答えを得た後は `AREKA_APP_SMOKE_EXIT_MS` の自動終了を待つ（外から止めると helper と検体の
//!   複製の後始末が areka の終了処理を通らない）。締切を超えたときだけ自分が起こした子を止めて失敗。
//! - 本文の JSON 文字列はテストの中の小さな復号で読む（`serde_json` を依存に足さない）。
//! - i686 の helper を `areka.exe` の隣に揃える手順は本ファイルに自前で持つ
//!   （`smoke_boot_loop_exit.rs` は触らない＝要件 8.4）。

use sample_ghost_kit::SampleRoot;
use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus};
use std::time::{Duration, Instant};

/// 問い合わせと終了待ちの締切（起動から数える）。
const DEADLINE: Duration = Duration::from_secs(60);
/// 問い合わせ・終了確認の間隔。
const POLL_INTERVAL: Duration = Duration::from_millis(250);
/// 標準出力から消え（`RUST_LOG=warn` の外の info）、履歴には残るはずの文。
const REAL_WINDOWS: &str = "本物のゴースト窓を開きました";
/// 本文を比べる出来事の目印。
const GHOST_RESOLVED: &str = "event=\"ghost_resolved\"";
/// 標準出力の行で本文が始まる前置き（`<時刻>  INFO areka::boot_config: <本文>`）。
const BOOT_CONFIG_PREFIX: &str = "areka::boot_config: ";

/// 一時の置き場（`target\tmp` の下）。破棄で中身ごと消す。
struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("mcp-get-log-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("profile"))
            .unwrap_or_else(|e| panic!("一時の置き場を作れません（{}）: {e}", dir.display()));
        Self(dir)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// PE の機械種別が i386（0x014c）か（x64 の helper は 32bit の SHIORI DLL を読めない）。
fn is_i686_pe(path: &Path) -> bool {
    let Ok(b) = std::fs::read(path) else {
        return false;
    };
    let at = |i: usize, n: usize| b.get(i..i + n);
    let Some(pe) = at(0x3c, 4).map(|s| u32::from_le_bytes(s.try_into().unwrap()) as usize) else {
        return false;
    };
    at(pe + 4, 2) == Some(&[0x4c, 0x01][..])
}

/// `areka.exe` の隣に i686 の `shiori-host32-helper.exe` を揃える。隣が i686 ならそのまま。
/// 無ければ env `HOST32_HELPER_EXE` → `<target>/i686-pc-windows-msvc/{debug,release}` の順で探し、
/// 一時名へ書いてから改名で置く（並走するテストの子が書きかけを起動しない）。
fn ensure_helper_beside_areka() {
    const NAME: &str = "shiori-host32-helper.exe";
    const ENV: &str = "HOST32_HELPER_EXE";
    let areka = Path::new(env!("CARGO_BIN_EXE_areka"));
    let dir = areka.parent().expect("areka.exe の親");
    let dest = dir.join(NAME);
    if is_i686_pe(&dest) {
        return;
    }
    // CARGO_BIN_EXE_areka＝<target>/<profile>/areka.exe → <target>
    let target = areka
        .ancestors()
        .nth(2)
        .expect("areka.exe の 2 つ上は target")
        .join("i686-pc-windows-msvc");
    let explicit = std::env::var_os(ENV).map(PathBuf::from).inspect(|p| {
        assert!(
            is_i686_pe(p),
            "{ENV}={} が指すファイルが無いか i686 ではありません（i686 の {NAME} を先にビルド）",
            p.display()
        )
    });
    let src = explicit
        .into_iter()
        .chain(["debug", "release"].map(|p| target.join(p).join(NAME)))
        .find(|p| is_i686_pe(p))
        .unwrap_or_else(|| {
            panic!(
                "i686 の {NAME} が見つかりません（env {ENV} → {}\\{{debug,release}}）。\
                 PowerShell で先に建ててください: \
                 cargo build -p shiori-host32-helper --target i686-pc-windows-msvc",
                target.display()
            )
        });
    let tmp = dir.join(format!("{NAME}.{}.tmp", std::process::id()));
    std::fs::copy(&src, &tmp)
        .unwrap_or_else(|e| panic!("{} → {} の複製に失敗: {e}", src.display(), tmp.display()));
    std::fs::rename(&tmp, &dest)
        .unwrap_or_else(|e| panic!("{} → {} の改名に失敗: {e}", tmp.display(), dest.display()));
}

/// `127.0.0.1:0` を束ねて空きの番号を得て放す。
fn free_port() -> u16 {
    TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
        .and_then(|l| l.local_addr())
        .expect("空きの番号を得られる")
        .port()
}

/// `get_log`（`log_type=status`）を 1 本送り、`text` の中身を返す。
/// つながらない・応答が読めない・`text` が無いときは `None`（呼び手が繰り返す）。
fn call_get_log_status(addr: SocketAddr) -> Option<String> {
    let body = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_log","arguments":{"log_type":"status"}}}"#;
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(1)).ok()?;
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .ok()?;
    let request = format!(
        "POST /api/mcp/v1 HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\n\
         Accept: application/json, text/event-stream\r\nContent-Length: {}\r\n\
         Connection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(request.as_bytes()).ok()?;
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).ok()?;
    let raw = String::from_utf8(raw).ok()?;
    let (_, json) = raw.split_once("\r\n\r\n")?;
    // 値の中の `"` は `\"` に逃がされるので、鍵 `"text":"` は本文の中には現れない。
    let start = json.find(r#""text":""#)? + r#""text":""#.len();
    Some(decode_json_string(&json[start..]))
}

/// JSON 文字列の中身（開きの `"` の直後から）を閉じの `"` まで復号する。
fn decode_json_string(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars();
    let hex4 = |chars: &mut std::str::Chars| {
        let h: String = chars.by_ref().take(4).collect();
        u32::from_str_radix(&h, 16).unwrap_or_else(|_| panic!("\\u の後が 16 進 4 桁でない: {h}"))
    };
    while let Some(c) = chars.next() {
        match c {
            '"' => return out,
            '\\' => match chars.next() {
                Some('n') => out.push('\n'),
                Some('r') => out.push('\r'),
                Some('t') => out.push('\t'),
                Some('b') => out.push('\u{8}'),
                Some('f') => out.push('\u{c}'),
                Some('u') => {
                    let mut code = hex4(&mut chars);
                    if (0xD800..0xDC00).contains(&code) {
                        // サロゲートの組（`😀`）。
                        assert_eq!(chars.next(), Some('\\'), "下位サロゲートが続かない");
                        assert_eq!(chars.next(), Some('u'), "下位サロゲートが続かない");
                        code = 0x10000 + ((code - 0xD800) << 10) + (hex4(&mut chars) - 0xDC00);
                    }
                    out.push(char::from_u32(code).expect("正しい符号位置"));
                }
                Some(other) => out.push(other), // `\\`・`\"`・`\/`
                None => panic!("JSON 文字列が逃がしの途中で終わった"),
            },
            c => out.push(c),
        }
    }
    panic!("JSON 文字列が閉じていない")
}

/// 行の頭 `#<数字> <yyyy/mm/dd hh:mm> [STAT] STAT : ` を読み、番号と本文を返す。
fn parse_status_line(line: &str) -> Option<(u64, &str)> {
    let rest = line.strip_prefix('#')?;
    let (id, rest) = rest.split_once(' ')?;
    if id.is_empty() || !id.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let stamp = rest.get(..16)?;
    let shape_ok = stamp.bytes().enumerate().all(|(i, b)| match i {
        4 | 7 => b == b'/',
        10 => b == b' ',
        13 => b == b':',
        _ => b.is_ascii_digit(),
    });
    if !shape_ok {
        return None;
    }
    Some((
        id.parse().ok()?,
        rest.get(16..)?.strip_prefix(" [STAT] STAT : ")?,
    ))
}

/// 子が `DEADLINE` までに終わるのを待つ。超えたら自分が起こした子を止めて失敗。
fn wait_exit(child: &mut Child, start: Instant, diag: impl Fn() -> String) -> ExitStatus {
    loop {
        if let Some(status) = child.try_wait().expect("子の状態を取れる") {
            return status;
        }
        if start.elapsed() >= DEADLINE {
            let _ = child.kill();
            let _ = child.wait();
            panic!(
                "子が締切（{DEADLINE:?}）内に終わりませんでした。\n{}",
                diag()
            );
        }
        std::thread::sleep(POLL_INTERVAL);
    }
}

#[test]
fn get_log_status_over_real_socket_keeps_lines_hidden_from_stdout_and_bodies_intact() {
    ensure_helper_beside_areka();
    let emo2 = SampleRoot::acquire("emo2").expect("emo2 は登記済みの検体");
    let ghost = emo2.folder();
    let balloon = emo2.balloon("emo2-kakukaku").expect("emo2 の同梱バルーン");
    assert!(ghost.is_absolute(), "検体は絶対パス: {}", ghost.display());
    let scratch = Scratch::new();
    let out_path = scratch.0.join("stdout.log");
    let err_path = scratch.0.join("stderr.log");
    let port = free_port();
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, port));

    let file = |p: &Path| std::fs::File::create(p).expect("出力のファイルを作れる");
    let mut child = Command::new(env!("CARGO_BIN_EXE_areka"))
        .args([ghost.as_os_str(), balloon.as_os_str()])
        .env("AREKA_MCP_PORT", port.to_string())
        .env("RUST_LOG", "warn,areka::boot_config=info")
        .env("NO_COLOR", "1")
        .env("AREKA_NO_ALERT", "1")
        .env("AREKA_APP_SMOKE_EXIT_MS", "20000")
        .env("AREKA_PROFILE_DIR", scratch.0.join("profile"))
        .env_remove("AREKA_ROOT")
        .stdout(file(&out_path))
        .stderr(file(&err_path))
        .spawn()
        .expect("areka を起こせる");
    let start = Instant::now();
    let read = |p: &Path| std::fs::read_to_string(p).unwrap_or_default();
    let diag = || {
        format!(
            "--- child stdout ---\n{}\n--- child stderr ---\n{}",
            read(&out_path),
            read(&err_path)
        )
    };

    // 目当ての行が出るまで問い合わせ、そのたびに子の終了も見る。
    let answer = loop {
        if let Some(status) = child.try_wait().expect("子の状態を取れる") {
            let all = diag();
            if all.contains("起動窓を開けません") && all.contains("モニタ列挙に失敗")
            {
                assert!(!status.success(), "起動窓を開けないのに exit 0。\n{all}");
                eprintln!("note: モニタ 0 台環境のため告知＋非 0 終了で受理");
                return;
            }
            panic!("答えを得る前に子が終わりました（status={status:?}）。\n{all}");
        }
        if start.elapsed() >= DEADLINE {
            let _ = child.kill();
            let _ = child.wait();
            panic!(
                "締切（{DEADLINE:?}）内に「{REAL_WINDOWS}」を含む status の答えを得られませんでした。\n{}",
                diag()
            );
        }
        match call_get_log_status(addr) {
            Some(text) if text.contains(REAL_WINDOWS) => break text,
            _ => std::thread::sleep(POLL_INTERVAL),
        }
    };

    // 自動終了を待ってから標準出力を読む。
    let answered = start.elapsed();
    let status = wait_exit(&mut child, start, diag);
    eprintln!(
        "note: 答え {answered:?}・終了 {:?}（{status:?}）",
        start.elapsed()
    );
    let stdout = read(&out_path);
    let ctx = || format!("--- status answer ---\n{answer}\n{}", diag());

    // 判定 1: 標準出力から消えた info が履歴に残る（要件 1.7・6.3）。
    assert!(
        !stdout.contains(REAL_WINDOWS),
        "RUST_LOG=warn なのに標準出力に「{REAL_WINDOWS}」がある。\n{}",
        ctx()
    );

    // 判定 3: 各行の頭の形と番号の昇順（継続行はタブで始まり、直前の行に属する）。
    let mut records: Vec<(u64, &str)> = Vec::new();
    for line in answer.split("\r\n") {
        if line.starts_with('\t') {
            assert!(!records.is_empty(), "先頭が継続行。\n{}", ctx());
            continue;
        }
        let parsed = parse_status_line(line)
            .unwrap_or_else(|| panic!("行の頭の形が違う: {line:?}\n{}", ctx()));
        if let Some((prev, _)) = records.last() {
            assert!(parsed.0 > *prev, "番号が昇順でない: {line:?}\n{}", ctx());
        }
        records.push(parsed);
    }

    // 判定 2: ghost_resolved の本文が標準出力の同じ行と 1 字も違わない（要件 1.8・4.6・4.7）。
    let in_history: Vec<&str> = records
        .iter()
        .map(|(_, body)| *body)
        .filter(|b| b.contains(GHOST_RESOLVED))
        .collect();
    let in_stdout: Vec<&str> = stdout
        .lines()
        .filter(|l| l.contains(GHOST_RESOLVED))
        .filter_map(|l| {
            l.split_once(BOOT_CONFIG_PREFIX)
                .map(|(_, b)| b.trim_end_matches('\r'))
        })
        .collect();
    assert_eq!(
        in_history.len(),
        1,
        "履歴の ghost_resolved が 1 行でない。\n{}",
        ctx()
    );
    assert!(
        in_history[0].contains('\\') && in_history[0].contains('"'),
        "ghost_resolved の本文に逆斜線と引用符が無い（往復を踏んでいない）。\n{}",
        ctx()
    );
    assert_eq!(
        in_history,
        in_stdout,
        "履歴と標準出力の本文が違う。\n{}",
        ctx()
    );
}
