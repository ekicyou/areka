//! 引数の解釈・コマンドの表・`--help`・終了コードへの写像。
//!
//! 引数は表 1 本（[`COMMANDS`] の 14 行と [`HELP`]）から読む。コマンドごとの手書きの分岐は
//! 持たない。行が持つのは、名前・要る引数・任意の引数・`--wait` を取るか・使い方に出す 1 行・
//! 読んだ引数から [`Command`] を組む関数。`--help` の本文も同じ表から組む。
//!
//! 順は「引数を読む → `--help` ならここで終わる → 置き場所を決める → コマンドの手順」。
//! 使い方の誤りと `--help` は置き場所を決める前に終わるので、何も読み書きしない。
//!
//! 標準出力には結果だけ、断りと失敗は標準エラーへ出す。このファイルの文字列リテラルは全部
//! ASCII で、打たれた引数を文に映すときは ASCII の外の字を逃がす（[`escape_path`]）。

use std::ffi::OsString;
use std::io::{self, Write};
use std::sync::LazyLock;

use crate::error::{WatchError, escape_path};
use crate::home;
use crate::plan;

const EXE: &str = "areka-impl-watch";

/// 実行した結果。失敗は `Err`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// できた。
    Done,
    /// 当てはまらなかった。
    // 作るのはコマンドの手順（後のタスク）。「満たされない expect」の警告が出たら外す。
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "the command procedures arrive in later tasks")
    )]
    NotApplied,
    /// 使い方の誤り。
    Usage,
}

/// 結果から終了コードへの写像（ここ 1 か所。`--help` の `exit codes:` と同じ 4 つ）。
pub fn exit_code(result: &Result<Outcome, WatchError>) -> u8 {
    match result {
        Ok(Outcome::Done) => 0,
        Err(_) => 1,
        Ok(Outcome::Usage) => 2,
        Ok(Outcome::NotApplied) => 3,
    }
}

/// 引数の種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Opt {
    Id,
    Repo,
    Spec,
    Name,
    Purpose,
    Pr,
    Sha,
    Bug,
    Wait,
}

/// 引数の全部（使い方に並べる順）。
const OPTIONS: [Opt; 9] = [
    Opt::Id,
    Opt::Repo,
    Opt::Spec,
    Opt::Name,
    Opt::Purpose,
    Opt::Pr,
    Opt::Sha,
    Opt::Bug,
    Opt::Wait,
];

/// 値の形。
#[derive(Clone, Copy)]
enum Shape {
    /// 識別・リポジトリ・spec。ロックファイルの名前と端末の文にそのまま載る。
    Token,
    /// 名前・内容。何でもよい（端末へは出さない）。
    Text,
    Digits,
    Hex,
    /// 値を取らない。
    Flag,
}

impl Opt {
    const fn flag(self) -> &'static str {
        match self {
            Opt::Id => "--id",
            Opt::Repo => "--repo",
            Opt::Spec => "--spec",
            Opt::Name => "--name",
            Opt::Purpose => "--purpose",
            Opt::Pr => "--pr",
            Opt::Sha => "--sha",
            Opt::Bug => "--bug",
            Opt::Wait => "--wait",
        }
    }

    const fn shape(self) -> Shape {
        match self {
            Opt::Id | Opt::Repo | Opt::Spec => Shape::Token,
            Opt::Name | Opt::Purpose => Shape::Text,
            Opt::Pr => Shape::Digits,
            Opt::Sha => Shape::Hex,
            Opt::Bug | Opt::Wait => Shape::Flag,
        }
    }

    /// 使い方に出す綴り（`--id <id>`・値を取らないものは `--bug`）。
    fn shown(self) -> String {
        let flag = self.flag();
        match self.shape() {
            Shape::Flag => flag.to_owned(),
            _ => format!("{flag} <{}>", &flag[2..]),
        }
    }
}

impl Shape {
    fn accepts(self, value: &str) -> bool {
        match self {
            Shape::Token => {
                (1..=100).contains(&value.len())
                    && value
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
            }
            Shape::Text | Shape::Flag => true,
            Shape::Digits => !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit()),
            Shape::Hex => {
                (7..=40).contains(&value.len()) && value.bytes().all(|b| b.is_ascii_hexdigit())
            }
        }
    }

    /// 形の決まり（使い方と、誤りの文の `must be …` に同じ綴りで出す）。
    const fn rule(self) -> &'static str {
        match self {
            Shape::Token => "1-100 chars of A-Z a-z 0-9 . _ -",
            Shape::Text => "any text",
            Shape::Digits => "digits only",
            Shape::Hex => "7-40 hex digits",
            Shape::Flag => "no value",
        }
    }
}

/// 読んだコマンド。
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Command {
    Help,
    /// 状態を変えずに読む。
    Status,
    /// 状態ファイルを退避して空から始める。
    Clear,
    /// 見張り。`name` を省いたら `None` のまま渡す（識別を詰めない）。
    Watch {
        id: String,
        repo: String,
        name: Option<String>,
    },
    /// 再開の待ちだけ。
    Resume {
        id: String,
    },
    /// 状態を変える 1 回。`--wait` 付きなら続けて待つ（[`Invocation::wait`]）。
    Change(plan::Command),
}

/// 読んだ呼び出し。
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Invocation {
    /// 表の名前（ログの `command=` と端末の文に使う）。
    pub(crate) name: &'static str,
    pub(crate) command: Command,
    pub(crate) wait: bool,
}

/// 使い方の誤り。中身は ASCII の 1 行（`<コマンド>: <何が違うか>`）。
#[derive(Debug)]
pub(crate) struct UsageError(pub(crate) String);

/// 読んだ引数。同じ引数は 1 つだけ入る。
struct Found(Vec<(Opt, String)>);

impl Found {
    fn has(&self, opt: Opt) -> bool {
        self.0.iter().any(|(found, _)| *found == opt)
    }

    fn take(&mut self, opt: Opt) -> Option<String> {
        let at = self.0.iter().position(|(found, _)| *found == opt)?;
        Some(self.0.swap_remove(at).1)
    }

    /// 要る引数を取り出す。呼ぶのは、その行の「要る引数」に在るものだけ（無ければ [`parse`] が
    /// 先に断っているので、ここへ来たときは必ず在る）。
    fn need(&mut self, opt: Opt) -> String {
        self.take(opt).unwrap_or_default()
    }
}

/// 表の 1 行。
pub(crate) struct Spec {
    pub(crate) name: &'static str,
    pub(crate) required: &'static [Opt],
    pub(crate) optional: &'static [Opt],
    /// `--wait` を取るか。
    pub(crate) wait: bool,
    /// 使い方に出す 1 行: すること と、0・3 の意味（1 と 2 は全コマンドに共通）。
    note: &'static str,
    /// 読んだ引数から [`Command`] を組む。
    build: fn(&mut Found) -> Command,
}

impl Spec {
    /// この行が受ける引数（要る・任意・`--wait`）。
    fn accepted(&self) -> impl Iterator<Item = Opt> {
        let wait = self.wait.then_some(Opt::Wait);
        self.required
            .iter()
            .chain(self.optional)
            .copied()
            .chain(wait)
    }
}

/// コマンドの表（設計の「コマンドの表」と同じ 14 行・同じ並び）。
pub(crate) static COMMANDS: [Spec; 14] = [
    Spec {
        name: "watch",
        required: &[Opt::Id, Opt::Repo],
        optional: &[Opt::Name],
        wait: false,
        note: "join and stay as the presence mark until a stop request. 0=stop requested (or already stopped) 3=record gone",
        build: |f| Command::Watch {
            id: f.need(Opt::Id),
            repo: f.need(Opt::Repo),
            name: f.take(Opt::Name),
        },
    },
    Spec {
        name: "merge",
        required: &[Opt::Id, Opt::Repo, Opt::Spec],
        optional: &[Opt::Bug, Opt::Name],
        wait: true,
        note: "queue for the merge desk of the repo. 0=queued; with --wait 0=granted 3=request gone",
        build: |f| {
            Command::Change(plan::Command::Merge {
                id: f.need(Opt::Id),
                name: f.take(Opt::Name),
                repo: f.need(Opt::Repo),
                spec: f.need(Opt::Spec),
                bug: f.has(Opt::Bug),
            })
        },
    },
    Spec {
        name: "merged",
        required: &[Opt::Id, Opt::Pr, Opt::Sha],
        optional: &[],
        wait: false,
        note: "release the merge desk, record the last merge and leave. 0=done 3=not the holder",
        build: |f| {
            Command::Change(plan::Command::Merged {
                id: f.need(Opt::Id),
                pr: f.need(Opt::Pr),
                sha: f.need(Opt::Sha),
            })
        },
    },
    Spec {
        name: "loadtest",
        required: &[Opt::Id, Opt::Repo, Opt::Purpose],
        optional: &[Opt::Name],
        wait: true,
        note: "queue for the load-test desk. 0=queued; with --wait 0=granted 3=request gone",
        build: |f| {
            Command::Change(plan::Command::LoadTest {
                id: f.need(Opt::Id),
                name: f.take(Opt::Name),
                repo: f.need(Opt::Repo),
                purpose: f.need(Opt::Purpose),
            })
        },
    },
    Spec {
        name: "loadrunning",
        required: &[Opt::Id, Opt::Repo, Opt::Purpose],
        optional: &[Opt::Name],
        wait: false,
        note: "record an already running load test as the holder. 0=recorded 3=the desk has a holder",
        build: |f| {
            Command::Change(plan::Command::LoadRunning {
                id: f.need(Opt::Id),
                name: f.take(Opt::Name),
                repo: f.need(Opt::Repo),
                purpose: f.need(Opt::Purpose),
            })
        },
    },
    Spec {
        name: "loaddone",
        required: &[Opt::Id],
        optional: &[],
        wait: false,
        note: "release the load-test desk. 0=done 3=not the holder",
        build: |f| {
            Command::Change(plan::Command::LoadDone {
                id: f.need(Opt::Id),
            })
        },
    },
    Spec {
        name: "stopped",
        required: &[Opt::Id],
        optional: &[],
        wait: true,
        note: "report stopped. 0=recorded 3=no stop request; with --wait 0=resumed 3=record gone",
        build: |f| {
            Command::Change(plan::Command::Stopped {
                id: f.need(Opt::Id),
            })
        },
    },
    Spec {
        name: "resume",
        required: &[Opt::Id],
        optional: &[],
        wait: false,
        note: "wait for the resume only. 0=resumed 3=record gone",
        build: |f| Command::Resume {
            id: f.need(Opt::Id),
        },
    },
    Spec {
        name: "unstop",
        required: &[],
        optional: &[Opt::Id],
        wait: false,
        note: "withdraw the stop requests of everyone, or of one id. 0=done 3=nobody to unstop",
        build: |f| {
            Command::Change(plan::Command::Unstop {
                id: f.take(Opt::Id),
            })
        },
    },
    Spec {
        name: "cancel",
        required: &[Opt::Id],
        optional: &[],
        wait: false,
        note: "drop the requests and desks of the id; the participant stays. 0=done",
        build: |f| {
            Command::Change(plan::Command::Cancel {
                id: f.need(Opt::Id),
            })
        },
    },
    Spec {
        name: "leave",
        required: &[Opt::Id],
        optional: &[],
        wait: false,
        note: "drop the requests, desks and the participant record of the id. 0=done",
        build: |f| {
            Command::Change(plan::Command::Leave {
                id: f.need(Opt::Id),
            })
        },
    },
    Spec {
        name: "tick",
        required: &[],
        optional: &[],
        wait: false,
        note: "reclaim and replan only. 0=done",
        build: |_| Command::Change(plan::Command::Tick),
    },
    Spec {
        name: "status",
        required: &[],
        optional: &[],
        wait: false,
        note: "read without changing: write status.md and print a summary. 0=done",
        build: |_| Command::Status,
    },
    Spec {
        name: "clear",
        required: &[],
        optional: &[],
        wait: false,
        note: "set the state file aside and start empty, without asking. 0=done",
        build: |_| Command::Clear,
    },
];

/// `--help` の行。コマンドと同じ読み方を通す（余計な引数は使い方の誤り）。
static HELP: Spec = Spec {
    name: "--help",
    required: &[],
    optional: &[],
    wait: false,
    note: "print this text. 0=done",
    build: |_| Command::Help,
};

/// 表の全部の行（14 個のコマンドと `--help`）。
fn specs() -> impl Iterator<Item = &'static Spec> {
    COMMANDS.iter().chain([&HELP])
}

/// 使い方の末尾（終了コードの対応と、置き場所）。
const FOOT: &str = "
exit codes:
  0  done: granted, stop requested, resumed, or the state was changed
  1  failure: AREKA_IMPL_WATCH_HOME unset or not creatable, state file, lock, io, same wait already running
  2  usage error: nothing is read or written
  3  not applied: the request or the record is gone, or the condition did not hold

The state lives in the folder AREKA_IMPL_WATCH_HOME points to. See doc/impl-watch.md";

/// `--help` の本文（ASCII・末尾に改行なし）。表から組む。
pub fn usage() -> &'static str {
    static TEXT: LazyLock<String> = LazyLock::new(|| {
        let mut text = format!("usage: {EXE} <command> [--option <value> ...]\n\ncommands:\n");
        for spec in specs() {
            text += "  ";
            text += spec.name;
            for opt in spec.accepted() {
                text += &if spec.required.contains(&opt) {
                    format!(" {}", opt.shown())
                } else {
                    format!(" [{}]", opt.shown())
                };
            }
            text += &format!("\n      {}\n", spec.note);
        }
        text += "\noptions:\n";
        for opt in OPTIONS {
            text += &format!("  {:<10} {}\n", opt.flag(), opt.shape().rule());
        }
        text + FOOT
    });
    &TEXT
}

/// 引数を読む。`args` は実行ファイル名を落としたもの。何も読み書きしない。
pub(crate) fn parse(args: &[String]) -> Result<Invocation, UsageError> {
    let Some((name, rest)) = args.split_first() else {
        return Err(UsageError("no command given".to_owned()));
    };
    let Some(spec) = specs().find(|spec| spec.name == name) else {
        let name = escape_path(name);
        return Err(UsageError(format!("unknown command '{name}'")));
    };
    let refuse = |what: String| UsageError(format!("{}: {what}", spec.name));

    let mut found = Found(Vec::new());
    let mut rest = rest.iter();
    while let Some(arg) = rest.next() {
        let Some(opt) = OPTIONS.into_iter().find(|opt| opt.flag() == arg) else {
            let arg = escape_path(arg);
            return Err(refuse(format!("unexpected argument '{arg}'")));
        };
        let (flag, shape) = (opt.flag(), opt.shape());
        if !spec.accepted().any(|accepted| accepted == opt) {
            return Err(refuse(format!("{flag} is not accepted")));
        }
        if found.has(opt) {
            return Err(refuse(format!("{flag} given twice")));
        }
        let value = match shape {
            Shape::Flag => String::new(),
            // `--` で始まる語は値に取らない。空の変数を引用符なしで渡すと値が抜けて次の引数が
            // 値の場所へ来る（`--spec --bug`）ので、黙って読み違えずに断る。
            _ => rest
                .next()
                .filter(|value| !value.starts_with("--"))
                .cloned()
                .ok_or_else(|| refuse(format!("{flag} needs a value")))?,
        };
        if !shape.accepts(&value) {
            return Err(refuse(format!("{flag} must be {}", shape.rule())));
        }
        found.0.push((opt, value));
    }
    if let Some(missing) = spec.required.iter().find(|opt| !found.has(**opt)) {
        return Err(refuse(format!("missing {}", missing.flag())));
    }

    let wait = found.take(Opt::Wait).is_some();
    Ok(Invocation {
        name: spec.name,
        command: (spec.build)(&mut found),
        wait,
    })
}

/// 入口。結果は標準出力、断りと失敗は標準エラーへ出す。
///
/// `home_value` は環境変数 `AREKA_IMPL_WATCH_HOME` の値（テストは値を直に渡す）。
pub fn run(args: &[String], home_value: Option<OsString>) -> Result<Outcome, WatchError> {
    run_with(args, home_value, &mut io::stdout(), &mut io::stderr())
}

/// [`run`] の中身。出力の行き先を受け取る（テストが読めるようにするため）。
pub(crate) fn run_with(
    args: &[String],
    home_value: Option<OsString>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<Outcome, WatchError> {
    let result = dispatch(args, home_value, out, err);
    if let Err(failure) = &result {
        // 失敗の本文（ASCII の 1 行）。ここで書けなければ、伝える先がもう無い（終了コードは 1 のまま）。
        let _ = writeln!(err, "{failure}");
    }
    result
}

fn dispatch(
    args: &[String],
    home_value: Option<OsString>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<Outcome, WatchError> {
    let invocation = match parse(args) {
        Ok(invocation) => invocation,
        Err(UsageError(what)) => {
            let line = format!("usage error: {what}; see {EXE} --help");
            say(err, "stderr", &line)?;
            return Ok(Outcome::Usage);
        }
    };
    if invocation.command == Command::Help {
        say(out, "stdout", usage())?;
        return Ok(Outcome::Done);
    }
    home::resolve(home_value)?;
    not_wired_yet(&invocation, err)
}

/// コマンドの手順がつながるまでの仮の枝（タスク 5.2・5.3 が `invocation.command` の `match` に
/// 置き換える）。状態ファイルもログも開かず、成功を装わない。
fn not_wired_yet(invocation: &Invocation, err: &mut dyn Write) -> Result<Outcome, WatchError> {
    let line = format!(
        "{}: accepted, but this command is not wired yet; nothing was done",
        invocation.name
    );
    say(err, "stderr", &line)?;
    Ok(Outcome::Usage)
}

/// 1 行を書き出す。書けなかったことも黙って捨てない。
fn say(to: &mut dyn Write, op: &'static str, line: &str) -> Result<(), WatchError> {
    writeln!(to, "{line}").map_err(|err| WatchError::io(op, &err))
}

#[cfg(test)]
#[path = "cli_tests.rs"]
mod tests;
