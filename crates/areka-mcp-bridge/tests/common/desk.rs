//! テスト用の Desktop の役（中継の入口 `run` をメモリ上の管 `tokio::io::duplex` で回す）。
//!
//! 部品のテスト（`src/lib.rs` が `cfg(test)` で読む）と通しのテスト（`tests/real_receiver_test.rs`）が
//! この同じ 1 ファイルを `#[path]` で読む。根が違うので `crate::` で引かず、中継の項目は読む側の根が
//! 見せる `cache`・`config`・`service` を `super::` で引く。

use std::time::Duration;

use rmcp::service::QuitReason;
use serde_json::{Value, json};
use temp_path_kit::TempPath;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, DuplexStream};
use tokio::task::JoinHandle;

use super::cache::Cache;
use super::config::{Config, Timeouts};
use super::service::run;

const PIPE: usize = 64 * 1024;
/// 待つ上限。過ぎたら panic でテストを赤にする。
pub(crate) const WAIT: Duration = Duration::from_secs(10);
pub(crate) const CACHE_DIR: &str = "mcp-bridge";

/// 確かめの上限を長く取った時間（負荷の下で受け口の答えが遅れても areka を飛ばさない）。
pub(crate) fn patient() -> Timeouts {
    Timeouts {
        verify: WAIT,
        ..Timeouts::default()
    }
}

/// Desktop の役: 中継の標準入力へ行を書き、標準出力の行を読む。
pub(crate) struct Desk {
    stdin: DuplexStream,
    stdout: BufReader<DuplexStream>,
    run: JoinHandle<QuitReason>,
    /// 控えの置き場の根（落とすと消える）。
    pub(crate) root: TempPath,
}

impl Desk {
    /// この候補の列で中継を回す。確かめの上限は長く取る（[`patient`]）。
    pub(crate) fn start(candidates: Vec<u16>) -> Desk {
        Desk::start_at(TempPath::under_target("bridge-service"), candidates)
    }

    /// 控えの置き場の根を受け取って回す（[`Desk::close`] が返した根を渡すと、起動し直した中継に
    /// なる＝前の中継が残した控えを読む）。
    pub(crate) fn start_at(root: TempPath, candidates: Vec<u16>) -> Desk {
        Desk::launch(root, candidates, patient())
    }

    /// 待つ時間を差し替えて回す（短い応答の上限など）。
    pub(crate) fn start_with(candidates: Vec<u16>, timeouts: Timeouts) -> Desk {
        let root = TempPath::under_target("bridge-service");
        Desk::launch(root, candidates, timeouts)
    }

    fn launch(root: TempPath, candidates: Vec<u16>, timeouts: Timeouts) -> Desk {
        let config = Config::from_parts(candidates, Some(root.child(CACHE_DIR)), timeouts);
        let (stdin, bridge_in) = tokio::io::duplex(PIPE);
        let (bridge_out, stdout) = tokio::io::duplex(PIPE);
        Desk {
            stdin,
            stdout: BufReader::new(stdout),
            run: tokio::spawn(run(config, bridge_in, bridge_out)),
            root,
        }
    }

    /// 置き場にある控え（別の `Cache` で読む＝起動し直した中継が読むのと同じ）。
    pub(crate) fn cache(&self) -> Cache {
        Cache::new(Some(self.root.child(CACHE_DIR)))
    }

    pub(crate) async fn send(&mut self, message: Value) {
        let line = format!("{message}\n");
        let written = self.stdin.write_all(line.as_bytes()).await;
        written.expect("標準入力へ書ける");
    }

    /// 標準出力の次の 1 行。
    pub(crate) async fn next(&mut self) -> Value {
        let mut line = String::new();
        let read = tokio::time::timeout(WAIT, self.stdout.read_line(&mut line)).await;
        read.expect("期限内に 1 行出る").expect("1 行読める");
        serde_json::from_str(&line).expect("標準出力の 1 行は JSON")
    }

    /// `id` 付きの要求を 1 つ送り、次の 1 行を返す。
    pub(crate) async fn ask(&mut self, id: Value, method: &str, params: Value) -> Value {
        self.send(request(id, method, params)).await;
        self.next().await
    }

    /// 標準入力を閉じる。中継は入力の終わりで終わり、標準出力に読んでいない行は残っていない
    /// （出るのは応答の行だけ＝通知には何も出ない）。控えの置き場の根を返す。
    pub(crate) async fn close(mut self) -> TempPath {
        drop(self.stdin);
        let quit = tokio::time::timeout(WAIT, self.run).await;
        let quit = quit.expect("期限内に終わる").expect("入口が panic しない");
        assert!(matches!(quit, QuitReason::Closed), "{quit:?}");
        let mut rest = String::new();
        let read = self.stdout.read_to_string(&mut rest).await;
        read.expect("標準出力を終わりまで読める");
        assert_eq!(rest, "", "読んでいない行が残っている");
        self.root
    }
}

pub(crate) fn request(id: Value, method: &str, params: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params})
}

/// `2026-07-28` で話す Desktop が要求ごとに `params` へ載せる `_meta`。
pub(crate) fn stateless_meta() -> Value {
    json!({
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientCapabilities": {},
        "io.modelcontextprotocol/clientInfo": {"name": "desktop", "version": "1"},
    })
}
