//! 起動の指定（RealRun）と、子へ渡す形の組み立て・子の起こし方
//! （要件 1.1〜1.7・1.9・1.10・2.1・2.5・3.1・4.1・4.4・8.1〜8.3・9.1）。
//!
//! 子へ伝える形（環境変数の名前・位置の引数の並べ方）を綴るのは `child_command` の 1 か所だけ
//! （8.2・8.3）。起動オプションへ移すときはここを替える。

use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use crate::evidence::RunDirs;
use crate::i686::{self, I686Spec};
use crate::run::{Limits, Run};

/// 起動するもの（1.1）。既定は `Empty`。
#[derive(Debug, Clone, Default)]
pub enum Launch {
    /// ⑴ 検体の名前。写しのゴーストのフォルダ（と同梱バルーンのフォルダ名）を位置の引数で渡す。
    Sample {
        name: String,
        balloon: Option<String>,
    },
    /// ⑵ 任意のゴーストのフォルダ（とバルーンのフォルダ）を位置の引数で渡す。
    Ghost {
        folder: PathBuf,
        balloon: Option<PathBuf>,
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
    /// `areka_exe` は呼び手が `env!("CARGO_BIN_EXE_areka")` で渡す（D-8）。
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

    /// 子を起こす前に 1 度だけ呼ぶ（写しの書き換え・試験用の DLL を揃える＝1.3・2.5）。
    pub fn before_spawn(mut self, f: impl FnOnce(&Staged<'_>) + 'static) -> RealRun {
        self.before_spawn = Some(Box::new(f));
        self
    }

    /// 記憶の置き場（既定は `<回>\profile`・普通のファイルも可＝1.6）。
    pub fn profile_dir(mut self, path: impl AsRef<Path>) -> RealRun {
        self.profile_dir = Some(path.as_ref().to_path_buf());
        self
    }

    /// MCP の番号（既定は空いた番号＝4.1）。
    pub fn mcp_port(mut self, port: u16) -> RealRun {
        self.mcp_port = Some(port);
        self
    }

    /// 記録の段（既定は `info`）。窓口が頼る行の出所が絞られていれば info を足す（4.4）。
    pub fn rust_log(mut self, filter: &str) -> RealRun {
        self.rust_log = Some(filter.to_owned());
        self
    }

    /// 保険の自動終了（既定 45 秒）。
    pub fn safety_exit(mut self, after: Duration) -> RealRun {
        self.limits.safety_exit = after;
        self
    }

    /// どの待ちにも使う締切（3.3・既定 60 秒）。
    pub fn deadline(mut self, limit: Duration) -> RealRun {
        self.limits.deadline = limit;
        self
    }

    /// 任意の環境変数。窓口の既定と論理の指定より後に置く＝何でも上書きできる（1.6）。
    pub fn env(mut self, key: impl AsRef<OsStr>, value: impl AsRef<OsStr>) -> RealRun {
        self.env
            .push((key.as_ref().to_owned(), value.as_ref().to_owned()));
        self
    }

    /// 任意の引数。位置の引数（ゴースト・バルーンのフォルダ）の後ろに足す（1.6）。
    pub fn arg(mut self, arg: impl AsRef<OsStr>) -> RealRun {
        self.args.push(arg.as_ref().to_owned());
        self
    }

    /// 子が areka でない（窓口自身の試験の偽の子）ときだけ使う。32bit の仲介を揃えない（D-18）。
    pub fn without_host32_helper(mut self) -> RealRun {
        self.host32_helper = false;
        self
    }

    /// 仲介を揃える → 置き場と写しを作る → `before_spawn` → 子を起こす、の順で起こす。
    ///
    /// # Panics
    ///
    /// 仲介が揃わない（2.3・2.4）・検体を取れない・子を起こせない（実行ファイルの場所と理由＝1.10）。
    /// 起こせなかった回の置き場は、panic の途中で破棄されるので落ちた回として残る（6.1）。
    pub fn spawn(mut self) -> Run {
        let exe = abs(&self.areka_exe);
        // 仲介は exe の隣を見るので、先に exe を確かめて場所と理由で落とす（1.10）。
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
                run_dir: dirs.run(),
                i686_target: &i686_target,
            });
        }
        let port = self.mcp_port.unwrap_or_else(free_port);
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
        let child = cmd
            .spawn()
            .unwrap_or_else(|err| panic!("子を起こせませんでした: {} ({err})", exe.display()));
        Run::new(child, dirs, port, self.limits)
    }

    /// 子へ渡す形を組む。起動の約束の綴りはここだけが書く（8.2・8.3）。順は
    /// ⑴ 開発者の 4 つを外す（1.5）→ ⑵ 窓口の既定 → ⑶ 呼び手の論理の指定 → ⑷ 呼び手の `env`（1.6）。
    /// ⑵⑶ は値が決まった後の 1 回の指定にまとめて書く。パスはどれも絶対（`\\?\` なし＝1.9）。
    fn child_command(
        &self,
        positional: &[&Path],
        root: &Path,
        profile: &Path,
        port: u16,
    ) -> Command {
        let mut cmd = Command::new(abs(&self.areka_exe));
        for key in [
            "AREKA_ROOT",
            "AREKA_USER_ROOT",
            "AREKA_PROFILE_DIR",
            "AREKA_MCP_PORT",
        ] {
            cmd.env_remove(key);
        }
        let rust_log = compose_rust_log(self.rust_log.as_deref().unwrap_or("info"));
        cmd.env("AREKA_ROOT", abs(root))
            .env("AREKA_PROFILE_DIR", abs(profile))
            .env("AREKA_MCP_PORT", port.to_string())
            .env("AREKA_NO_ALERT", "1")
            .env("NO_COLOR", "1")
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
    run_dir: &'a Path,
    i686_target: &'a Path,
}

impl Staged<'_> {
    /// Sample・Ghost のときの起動するゴーストのフォルダ（Sample は写しの中）。
    pub fn ghost_folder(&self) -> Option<&Path> {
        self.ghost_folder
    }

    /// 子へ渡す根。
    pub fn root(&self) -> &Path {
        self.root
    }

    /// 回の置き場。
    pub fn run_dir(&self) -> &Path {
        self.run_dir
    }

    /// 試験用の 32bit の成果物を `dest_dir` へ揃えてそのパスを返す（2.5）。規則は仲介と同じ。
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

/// 位置の引数で起こす回にも空の根を渡す（1.4）。根を選ぶのは `SampleRoot`・`Root` だけ。
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
        Launch::Ghost { folder, balloon } => Resolved {
            ghost: Some(abs(folder)),
            balloon: balloon.as_deref().map(abs),
            root: empty,
        },
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

/// Windows の形の絶対パス（`canonicalize` の `\\?\` は使わない＝1.9）。
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

/// 空いた番号を束ねて得て放す（4.1）。子が束ねるまでに取られたら子の記録で分かる（9.2）。
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
const RELIED_TARGETS: [&str; 2] = ["areka_mcp::server", "areka::app_exit"];

/// 窓口が頼る出所の実効の段が info より絞られているときだけ `,<出所>=info` を足す（D-4・4.4）。
///
/// ⑴ 出所そのものか親を名指していれば足さない ⑵ 全体の段が info・debug・trace なら足さない
/// ⑶ 全体の段が warn・error・off（指定なし＝off）で名指しも無いときだけ足す。空は既定の `info`。
///
/// 「名指し」は `EnvFilter` と同じ規則で決める。`EnvFilter` は出所を `::` の切れ目なしの素の
/// 前方一致で照らす（tracing-subscriber 0.3.23 `filter/env/directive.rs:246`
/// `meta.target().starts_with(&target[..])`）。`areka=debug` は `areka_mcp::server` も覆うので、
/// そこへ `=info` を足すと debug を info へ下げてしまう。
fn compose_rust_log(base: &str) -> String {
    if base.trim().is_empty() {
        return "info".to_owned();
    }
    let mut global_open = false;
    let mut named: Vec<&str> = Vec::new();
    for d in base.split(',').map(str::trim).filter(|d| !d.is_empty()) {
        match d.split_once('=') {
            Some((target, _)) => named.push(target.split('[').next().unwrap_or("").trim()),
            None => match d.to_ascii_lowercase().as_str() {
                "info" | "debug" | "trace" => global_open = true,
                "warn" | "error" | "off" => global_open = false,
                _ => named.push(d.split('[').next().unwrap_or("")),
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
