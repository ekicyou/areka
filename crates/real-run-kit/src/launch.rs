//! 起動の指定（RealRun）と、子へ渡す形の組み立て・子の起こし方。
//!
//! 子へ伝える形（環境変数の名前・位置の引数の並べ方）を組むのは `child_command` の 1 か所だけ。起動オプションへ移すときはここを替える。MCP の番号の環境変数の名前だけは、呼び手の
//! `env` から同じ名前を探す `port` も使うので、どちらも `areka_mcp::PORT_ENV` を引く。

use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use areka_mcp::PORT_ENV;

use crate::evidence::RunDirs;
use crate::i686::{self, I686Spec};
use crate::mcp::McpChannel;
use crate::run::{Limits, Run};

/// 起動するもの。既定は `Empty`。
#[derive(Debug, Clone, Default)]
pub enum Launch {
    /// ⑴ 検体の名前。写しのゴーストのフォルダ（と同梱バルーンのフォルダ名）を位置の引数で渡す。
    Sample {
        name: String,
        balloon: Option<String>,
    },
    /// ⑶ 検体の写しの根を根として渡す（どのゴーストを起こすかは areka が根から決める）。
    SampleRoot { name: String },
    /// ⑶ 任意の根。
    Root(PathBuf),
    /// ⑷ その回だけの空の根。
    #[default]
    Empty,
}

/// 起こす直前の呼び手の書き換え。
type BeforeSpawn = Box<dyn FnOnce(&Staged<'_>)>;

/// 起動の指定。`spawn` で子を起こして走りの取っ手を返す。
pub struct RealRun {
    areka_exe: PathBuf,
    launch: Launch,
    before_spawn: Option<BeforeSpawn>,
    profile_dir: Option<PathBuf>,
    mcp_port: Option<u16>,
    rust_log: Option<String>,
    limits: Limits,
    env: Vec<(OsString, OsString)>,
    args: Vec<OsString>,
    host32_helper: bool,
}

impl RealRun {
    /// `areka_exe` は呼び手が `env!("CARGO_BIN_EXE_areka")` で渡す。
    /// `<target>\<profile>\areka.exe` の形（`<target>` を 2 つ上として仲介の元を探す）。
    pub fn new(areka_exe: impl AsRef<Path>) -> RealRun {
        RealRun {
            areka_exe: areka_exe.as_ref().to_path_buf(),
            launch: Launch::Empty,
            before_spawn: None,
            profile_dir: None,
            mcp_port: None,
            rust_log: None,
            limits: Limits::default(),
            env: Vec::new(),
            args: Vec::new(),
            host32_helper: true,
        }
    }

    /// 起動するもの（既定は `Launch::Empty`）。
    pub fn launch(mut self, what: Launch) -> RealRun {
        self.launch = what;
        self
    }

    /// 子を起こす前に 1 度だけ呼ぶ（写しの書き換え・試験用の DLL を揃える）。
    pub fn before_spawn(mut self, f: impl FnOnce(&Staged<'_>) + 'static) -> RealRun {
        self.before_spawn = Some(Box::new(f));
        self
    }

    /// 記憶の置き場（既定は `<回>\profile`・普通のファイルも可）。
    pub fn profile_dir(mut self, path: impl AsRef<Path>) -> RealRun {
        self.profile_dir = Some(path.as_ref().to_path_buf());
        self
    }

    /// MCP の番号（既定は空いた番号）。`env` で `AREKA_MCP_PORT` を積むとそちらが勝つ。
    pub fn mcp_port(mut self, port: u16) -> RealRun {
        self.mcp_port = Some(port);
        self
    }

    /// 記録の段（既定は `info`）。全体の段が info より絞られていて、窓口が頼る行の出所を名指す指定が
    /// 無いときは、その出所に info を足す。名指す指定があれば、その段が何であっても足さない。
    pub fn rust_log(mut self, filter: &str) -> RealRun {
        self.rust_log = Some(filter.to_owned());
        self
    }

    /// 保険の自動終了（既定は `run.rs` の `Limits::default`）。
    pub fn safety_exit(mut self, after: Duration) -> RealRun {
        self.limits.safety_exit = after;
        self
    }

    /// どの待ちにも使う締切（既定は `run.rs` の `Limits::default`）。
    pub fn deadline(mut self, limit: Duration) -> RealRun {
        self.limits.deadline = limit;
        self
    }

    /// 任意の環境変数。窓口の既定と論理の指定より後に置く＝何でも上書きできる。
    /// `AREKA_MCP_PORT` を積んだときは、最後の値が子の待ち受ける番号で窓口が話す番号
    /// （番号として読めなければ `spawn` が落ちる）。
    pub fn env(mut self, key: impl AsRef<OsStr>, value: impl AsRef<OsStr>) -> RealRun {
        self.env
            .push((key.as_ref().to_owned(), value.as_ref().to_owned()));
        self
    }

    /// 任意の引数。位置の引数（ゴースト・バルーンのフォルダ）の後ろに足す。
    pub fn arg(mut self, arg: impl AsRef<OsStr>) -> RealRun {
        self.args.push(arg.as_ref().to_owned());
        self
    }

    /// 子が areka でない（窓口自身の試験の偽の子）ときだけ使う。32bit の仲介を揃えない。
    pub fn without_host32_helper(mut self) -> RealRun {
        self.host32_helper = false;
        self
    }

    /// 仲介を揃える → 置き場と写しを作る → `before_spawn` → 子を起こす、の順で起こす。
    ///
    /// # Panics
    ///
    /// 仲介が揃わない・検体を取れない・子を起こせない（実行ファイルの場所と理由）。
    /// 回の置き場を作り終えた後に落ちたときだけ、置き場は panic の途中で破棄されて落ちた回として残る。
    /// それより前（実行ファイル・仲介・置き場と検体の写し）で落ちた回は残らない。
    pub fn spawn(mut self) -> Run {
        let exe = abs(&self.areka_exe);
        // 仲介は exe の隣を見るので、先に exe を確かめて場所と理由で落とす。
        assert!(
            exe.is_file(),
            "子を起こせませんでした: {}（ファイルが無い）",
            exe.display()
        );
        let i686_target = i686_target(&exe);
        if self.host32_helper {
            let helper = i686::HELPER;
            let beside = exe.parent().unwrap_or(Path::new("."));
            i686::ensure(
                &helper,
                std::env::var_os(helper.env_var),
                &i686_target,
                beside,
            );
        }
        let dirs = RunDirs::create(sample_name(&self.launch));
        let r = resolve(&self.launch, &dirs);
        if let Some(f) = self.before_spawn.take() {
            f(&Staged {
                ghost_folder: r.ghost.as_deref(),
                root: &r.root,
                i686_target: &i686_target,
            });
        }
        let port = self.port();
        let positional: Vec<&Path> = r
            .ghost
            .iter()
            .chain(&r.balloon)
            .map(|p| p.as_path())
            .collect();
        let profile = self.profile_dir.as_deref().unwrap_or(dirs.profile());
        let mut cmd = self.child_command(&positional, &r.root, profile, port);
        cmd.stdin(Stdio::null())
            .stdout(log_file(&dirs.stdout_log()))
            .stderr(log_file(&dirs.stderr_log()));
        let mcp = McpChannel::new(port);
        let child = cmd
            .spawn()
            .unwrap_or_else(|err| panic!("子を起こせませんでした: {} ({err})", exe.display()));
        Run::new(child, dirs, mcp, port, self.limits)
    }

    /// 子が待ち受ける番号＝窓口が話す番号。呼び手の `env` の最後の `AREKA_MCP_PORT` が勝つ
    /// （子と同じく前後の空白を除いて u16 として読む。Windows の環境変数の名前は大文字小文字を見ない）。
    ///
    /// # Panics
    ///
    /// その値を番号として読めないとき（子は既定の候補へ移り、窓口はどの番号か分からない）。
    fn port(&self) -> u16 {
        let set = self
            .env
            .iter()
            .rev()
            .find(|(k, _)| k.eq_ignore_ascii_case(PORT_ENV));
        match set {
            Some((_, v)) => v
                .to_str()
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or_else(|| {
                    panic!(
                        "env の {PORT_ENV}={v:?} を番号（0〜65535）として読めない。番号は `.mcp_port(n)` で渡す"
                    )
                }),
            None => self.mcp_port.unwrap_or_else(free_port),
        }
    }

    /// 子へ渡す形を組む。起動の約束の綴りはここだけが書く（MCP の番号の名前は
    /// `areka_mcp::PORT_ENV` を引く）。順は
    /// ⑴ 開発者の書ける根を外す → ⑵ 窓口の既定 → ⑶ 呼び手の論理の指定 → ⑷ 呼び手の `env`。
    /// ⑵⑶ は値が決まった後の 1 回の指定にまとめて書く（開発者のほかの変数はここで上書きされる）。
    /// パスはどれも絶対（`\\?\` なし）。
    fn child_command(
        &self,
        positional: &[&Path],
        root: &Path,
        profile: &Path,
        port: u16,
    ) -> Command {
        let mut cmd = Command::new(abs(&self.areka_exe));
        cmd.env_remove("AREKA_USER_ROOT");
        let rust_log = compose_rust_log(self.rust_log.as_deref().unwrap_or("info"));
        cmd.env("AREKA_ROOT", abs(root))
            .env("AREKA_PROFILE_DIR", abs(profile))
            .env(PORT_ENV, port.to_string())
            .env("AREKA_NO_ALERT", "1")
            .env("NO_COLOR", "1")
            // 開発者の PASTA_DEBUG=1 を受け継ぐと、SSP などのほかの pasta が握る 9276 とぶつかり LOAD に失敗する
            .env("PASTA_DEBUG", "0")
            .env(
                "AREKA_APP_SMOKE_EXIT_MS",
                self.limits.safety_exit.as_millis().to_string(),
            )
            .env("RUST_LOG", rust_log);
        for (k, v) in &self.env {
            cmd.env(k, v);
        }
        cmd.args(positional.iter().map(|p| abs(p)));
        cmd.args(&self.args);
        cmd
    }
}

/// `before_spawn` に渡す、起こす直前の置き場。
pub struct Staged<'a> {
    ghost_folder: Option<&'a Path>,
    root: &'a Path,
    i686_target: &'a Path,
}

impl Staged<'_> {
    /// Sample のときの起動するゴーストのフォルダ（写しの中）。
    pub fn ghost_folder(&self) -> Option<&Path> {
        self.ghost_folder
    }

    /// 子へ渡す根。
    pub fn root(&self) -> &Path {
        self.root
    }

    /// 試験用の 32bit の成果物を `dest_dir` へ揃えてそのパスを返す。規則は仲介と同じ。
    ///
    /// # Panics
    ///
    /// 揃えられないとき（仲介と同じ文）。
    pub fn ensure_i686(
        &self,
        file_name: &str,
        env_var: &str,
        package: &str,
        dest_dir: &Path,
    ) -> PathBuf {
        let spec = I686Spec {
            file_name,
            env_var,
            package,
        };
        i686::ensure(&spec, std::env::var_os(env_var), self.i686_target, dest_dir)
    }
}

/// 起動するものから決めた位置の引数と根。
#[derive(Debug)]
struct Resolved {
    ghost: Option<PathBuf>,
    balloon: Option<PathBuf>,
    root: PathBuf,
}

/// 写しを取る検体の名前。
fn sample_name(what: &Launch) -> Option<&str> {
    match what {
        Launch::Sample { name, .. } | Launch::SampleRoot { name } => Some(name),
        _ => None,
    }
}

/// 位置の引数で起こす回にも空の根を渡す。根を選ぶのは `SampleRoot`・`Root` だけ。
fn resolve(what: &Launch, dirs: &RunDirs) -> Resolved {
    let empty = dirs.root().to_path_buf();
    let sample = || dirs.sample().expect("検体の名前で置き場を作ったはず");
    match what {
        Launch::Sample { balloon, .. } => {
            let s = sample();
            let balloon = balloon.as_deref().map(|b| {
                s.balloon(b)
                    .unwrap_or_else(|err| panic!("検体のバルーン {b:?}: {err}"))
                    .to_path_buf()
            });
            Resolved {
                ghost: Some(s.folder().to_path_buf()),
                balloon,
                root: empty,
            }
        }
        Launch::SampleRoot { .. } => Resolved {
            ghost: None,
            balloon: None,
            root: sample().root().to_path_buf(),
        },
        Launch::Root(root) => Resolved {
            ghost: None,
            balloon: None,
            root: abs(root),
        },
        Launch::Empty => Resolved {
            ghost: None,
            balloon: None,
            root: empty,
        },
    }
}

/// Windows の形の絶対パス（`canonicalize` の `\\?\` は使わない）。
fn abs(path: &Path) -> PathBuf {
    std::path::absolute(path)
        .unwrap_or_else(|err| panic!("絶対パスにできません: {} ({err})", path.display()))
}

/// `<target>\<profile>\areka.exe` から `<target>\i686-pc-windows-msvc`。
fn i686_target(exe: &Path) -> PathBuf {
    let target = exe.parent().and_then(Path::parent).unwrap_or_else(|| {
        panic!(
            "areka.exe の 2 つ上のフォルダがありません: {}",
            exe.display()
        )
    });
    target.join("i686-pc-windows-msvc")
}

/// 空いた番号を束ねて得て放す。子が束ねるまでに取られたら子の記録で分かる。
fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .and_then(|l| l.local_addr())
        .map(|a| a.port())
        .unwrap_or_else(|err| panic!("空いた MCP の番号を取れません: {err}"))
}

fn log_file(path: &Path) -> File {
    File::create(path)
        .unwrap_or_else(|err| panic!("出力のファイルを作れません: {} ({err})", path.display()))
}

/// 窓口が頼る行の出所（待受の行・閉じる知らせの受け取りと終わり方の行）。
const RELIED_TARGETS: [&str; 2] = ["areka_mcp::server", "areka_session::app_exit"];

/// 全体の段が info より絞られていて、窓口が頼る出所を名指す指定が無いときだけ、その出所に
/// `,<出所>=info` を足す。
///
/// ⑴ 全体の段が info・debug・trace（数字の 3〜5）なら、どの出所にも足さない ⑵ 全体の段が warn・
/// error・off（数字の 0〜2・指定なし＝off）なら、出所ごとに、名指す指定が無いものにだけ足す
/// ⑶ 名指す指定がある出所には、その段が何であっても足さない（呼び手が `=off` で閉じた出所は閉じたまま）。
/// 空は既定の `info`。span を付けた指定（`[` を含む・例 `areka_mcp::server[req]=debug`）は
/// その span の中の行だけを開けるので名指しに数えない。
///
/// 「名指し」は `EnvFilter` と同じ規則で決める。`EnvFilter` は出所を `::` の切れ目なしの素の
/// 前方一致で照らす（tracing-subscriber 0.3.23 `filter/env/directive.rs`
/// `meta.target().starts_with(&target[..])`）。`areka=debug` は `areka_mcp::server` も覆うので、
/// そこへ `=info` を足すと debug を info へ下げてしまう。
fn compose_rust_log(base: &str) -> String {
    if base.trim().is_empty() {
        return "info".to_owned();
    }
    let mut global_open = false;
    let mut named: Vec<&str> = Vec::new();
    for d in base
        .split(',')
        .map(str::trim)
        .filter(|d| !d.is_empty() && !d.contains('['))
    {
        match d.split_once('=') {
            Some((target, _)) => named.push(target.trim()),
            None => match d.to_ascii_lowercase().as_str() {
                // 数字の段は EnvFilter の読みと同じ（0＝off・1＝error・2＝warn・3＝info・4＝debug・5＝trace）。
                "info" | "debug" | "trace" | "3" | "4" | "5" => global_open = true,
                "warn" | "error" | "off" | "0" | "1" | "2" => global_open = false,
                _ => named.push(d),
            },
        }
    }
    let mut out = base.to_owned();
    if global_open {
        return out;
    }
    for t in RELIED_TARGETS {
        let is_named = named.iter().any(|n| !n.is_empty() && t.starts_with(n));
        if !is_named {
            out.push_str(&format!(",{t}=info"));
        }
    }
    out
}

#[cfg(test)]
#[path = "launch_tests.rs"]
mod tests;
