//! 引数の解釈・コマンドの表・`--help`（`cli` の子モジュール `args`）。
//!
//! 引数は表 1 本（[`COMMANDS`] の 14 行と [`HELP`]）から読む。コマンドごとの手書きの分岐は
//! 持たない。行が持つのは、名前・要る引数・任意の引数・`--wait` を取るか・使い方に出す 1 行・
//! 読んだ引数から [`Command`] を組む関数。`--help` の本文も同じ表から組む。
//!
//! 何も読み書きしない。このファイルの文字列リテラルは全部 ASCII（打たれた引数を誤りの文に
//! 映すときは ASCII の外の字を逃がす）。

use std::sync::LazyLock;

use crate::error::escape_path;
use crate::plan;
use crate::state::DeskSet;

pub(super) const EXE: &str = "areka-impl-watch";

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
    /// 識別・リポジトリ・spec。ロックファイルの名前と端末の文に載る（識別は小文字に寄せた形で）。
    Token,
    /// 名前・内容。`--` で始まらなければ何でもよい（端末へは出さない）。
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
            Shape::Text => "any text not starting with --",
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
    /// 名前の行には出さず、`note` の次の 1 行で示す別の形（今は `clear --repo` だけ）。
    pub(crate) variant: Option<Variant>,
    /// 使い方に出す 1 行: すること と、0・3 の意味（1 と 2 は全コマンドに共通）。
    note: &'static str,
    /// 読んだ引数から [`Command`] を組む。
    build: fn(&mut Found) -> Command,
}

/// 表の行の別の形: 1 つの引数と、`--help` に足す 1 行の説明。
pub(crate) struct Variant {
    pub(crate) opt: Opt,
    pub(crate) note: &'static str,
}

impl Spec {
    /// 名前の行に出す引数（要る・任意・`--wait`）。
    fn accepted(&self) -> impl Iterator<Item = Opt> {
        let wait = self.wait.then_some(Opt::Wait);
        self.required
            .iter()
            .chain(self.optional)
            .copied()
            .chain(wait)
    }

    /// 引数の読みが受ける引数（名前の行の引数と、別の形の引数）。
    fn parsed(&self) -> impl Iterator<Item = Opt> {
        self.accepted()
            .chain(self.variant.as_ref().map(|variant| variant.opt))
    }
}

/// コマンドの表（設計の「コマンドの表」と同じ 14 行・同じ並び）。
pub(crate) static COMMANDS: [Spec; 14] = [
    Spec {
        name: "watch",
        required: &[Opt::Id, Opt::Repo],
        optional: &[Opt::Name],
        wait: false,
        variant: None,
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
        variant: None,
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
        variant: None,
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
        variant: None,
        note: "queue for the load-test desk. 0=queued; with --wait 0=granted 3=request gone",
        build: |f| {
            // 版 2 への切り替えの間の読み替え（3.3 が `request` の行へ替える）。
            Command::Change(plan::Command::Request {
                id: f.need(Opt::Id),
                name: f.take(Opt::Name),
                repo: f.need(Opt::Repo),
                desks: DeskSet::TestSole,
                purpose: f.need(Opt::Purpose),
            })
        },
    },
    Spec {
        name: "loadrunning",
        required: &[Opt::Id, Opt::Repo, Opt::Purpose],
        optional: &[Opt::Name],
        wait: false,
        variant: None,
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
        variant: None,
        note: "release the load-test desk. 0=done 3=not the holder",
        build: |f| {
            // 版 2 への切り替えの間の読み替え（3.3 が `return` の行へ替える）。
            Command::Change(plan::Command::Return {
                id: f.need(Opt::Id),
                desks: None,
            })
        },
    },
    Spec {
        name: "stopped",
        required: &[Opt::Id],
        optional: &[],
        wait: true,
        variant: None,
        note: "report stopped. 0=recorded 3=no stop request; with --wait 0=resumed, or stop requested again 3=record gone",
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
        variant: None,
        note: "wait for the resume only. 0=resumed, or stop requested again 3=record gone",
        build: |f| Command::Resume {
            id: f.need(Opt::Id),
        },
    },
    Spec {
        name: "unstop",
        required: &[],
        optional: &[Opt::Id],
        wait: false,
        variant: None,
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
        variant: None,
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
        variant: None,
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
        variant: None,
        note: "reclaim and replan only. 0=done",
        build: |_| Command::Change(plan::Command::Tick),
    },
    Spec {
        name: "status",
        required: &[],
        optional: &[],
        wait: false,
        variant: None,
        note: "read without changing: write status.md and print a summary. 0=done",
        build: |_| Command::Status,
    },
    Spec {
        name: "clear",
        required: &[],
        optional: &[],
        wait: false,
        variant: Some(Variant {
            opt: Opt::Repo,
            note: "leave everyone of the repo and empty its merge desk (the last merge stays). prints \"cleared repo=<repo>; left: <ids or none>\". 0=done",
        }),
        note: "set the state file aside and start empty, without asking. 0=done",
        build: |f| match f.take(Opt::Repo) {
            Some(repo) => Command::Change(plan::Command::ClearRepo { repo }),
            None => Command::Clear,
        },
    },
];

/// `--help` の行。コマンドと同じ読み方を通す（余計な引数は使い方の誤り）。
static HELP: Spec = Spec {
    name: "--help",
    required: &[],
    optional: &[],
    wait: false,
    variant: None,
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
  0  done: granted, stop requested (or requested again), resumed, or the state was changed
  1  failure: AREKA_IMPL_WATCH_HOME unset, not absolute or not creatable, state file, lock, io, same wait already running
  2  usage error: nothing is read or written
  3  not applied: the request or the record is gone, or the condition did not hold

--id is folded to lower case: A and a are the same participant.
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
            if let Some(variant) = &spec.variant {
                text += &format!(
                    "      {} {}: {}\n",
                    spec.name,
                    variant.opt.shown(),
                    variant.note
                );
            }
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
        if !spec.parsed().any(|accepted| accepted == opt) {
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
                .ok_or_else(|| {
                    refuse(format!(
                        "{flag} needs a value (a value cannot start with --)"
                    ))
                })?,
        };
        if !shape.accepts(&value) {
            return Err(refuse(format!("{flag} must be {}", shape.rule())));
        }
        // 識別だけは小文字に寄せる（寄せるのはここ 1 か所。大小文字を区別しないファイル
        // システムで、`A` と `a` が別の参加者のまま同じ居る印を取り合わないように）。
        let value = match opt {
            Opt::Id => value.to_ascii_lowercase(),
            _ => value,
        };
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

// 版 2 への切り替えの間は読み込まない（3.3 が新しい行へ書き直して戻す）。
// #[cfg(test)]
// #[path = "cli_args_tests.rs"]
// mod tests;
