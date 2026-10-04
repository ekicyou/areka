//! ログの履歴（areka-P0-mcp-log-history）。
//!
//! ログの出来事を SSP の 5 種別（error・script・network・update・status）へ振り分ける。
//! 規則は [`RULES`] の 1 表だけに書き、[`classify`] が判定の順に当てる。
//! 振り分けは tracing の受け口にも World にも触れない純粋な関数で、テストは値を直に渡す。

// 本番の呼び手（履歴の層・出口の据え付け）が生えるまでの一時的な抑止。
// areka-P0-mcp-log-history task 2.3 で外す。
#![allow(dead_code)]

use tracing::Level;

/// 種別（SSP の 5 語）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    Error,
    Script,
    Network,
    Update,
    Status,
}

impl Kind {
    const ALL: [Kind; 5] = [
        Kind::Error,
        Kind::Script,
        Kind::Network,
        Kind::Update,
        Kind::Status,
    ];

    /// `log_type` の語（小文字）。
    pub(crate) fn word(self) -> &'static str {
        match self {
            Kind::Error => "error",
            Kind::Script => "script",
            Kind::Network => "network",
            Kind::Update => "update",
            Kind::Status => "status",
        }
    }

    /// 5 語と大文字小文字の違いを除いて一致すれば Some（前後の空白は削らない・空は None）。
    pub(crate) fn from_log_type(word: &str) -> Option<Kind> {
        Kind::ALL
            .into_iter()
            .find(|k| k.word().eq_ignore_ascii_case(word))
    }

    /// `[<種別>]` の既定の語（Error / SSTP / Info / Info / STAT）。
    pub(crate) fn default_label(self) -> &'static str {
        match self {
            Kind::Error => "Error",
            Kind::Script => "SSTP",
            Kind::Network | Kind::Update => "Info",
            Kind::Status => "STAT",
        }
    }

    /// `<名>` の既定（status は STAT、ほかは [SYSTEM]）。
    pub(crate) fn default_name(self) -> &'static str {
        match self {
            Kind::Status => "STAT",
            _ => "[SYSTEM]",
        }
    }
}

/// 取り決めの target: 再生した台本を script 種別へ残す（info 以上で出す約束）。
pub(crate) const TARGET_SCRIPT: &str = "areka::log::script";
/// 取り決めの target: error 種別へ直接残す（info 以上で出す約束）。
pub(crate) const TARGET_ERROR: &str = "areka::log::error";

/// info の行を種別へ振る規則 1 行。
pub(crate) struct Rule {
    pub target: &'static str,
    pub kind: Kind,
    /// true は完全一致だけ。false は自身か `::` で区切った下のモジュール。
    pub exact: bool,
}

impl Rule {
    /// target がこの行に当たるか（`areka::update` は `areka::update::desk` に当たり、
    /// `areka::updater` には当たらない）。
    pub(crate) fn matches(&self, target: &str) -> bool {
        match target.strip_prefix(self.target) {
            Some("") => true,
            Some(rest) => !self.exact && rest.starts_with("::"),
            None => false,
        }
    }
}

const fn rule(target: &'static str, kind: Kind, exact: bool) -> Rule {
    Rule {
        target,
        kind,
        exact,
    }
}

/// 上から順に当て、最初に当たった行の種別になる（network の行は update の行より先）。
pub(crate) const RULES: &[Rule] = &[
    rule("areka::install::fetch_url", Kind::Network, false),
    rule("areka_update::winhttp", Kind::Network, false),
    rule("areka_update::fetch", Kind::Network, false),
    rule("areka_update", Kind::Update, false),
    rule("areka::update", Kind::Update, false),
    rule("areka::install", Kind::Update, false),
    // 下のモジュールまで含めるとアプリ本体の info が全部 status になるので完全一致だけ。
    rule("areka", Kind::Status, true),
    rule("areka::boot_config", Kind::Status, false),
    rule("areka::boot_resolve", Kind::Status, false),
    rule("areka::ghost_session", Kind::Status, false),
    rule("areka::emo2_boot::ghost_switch", Kind::Status, false),
    rule("ghost-boot", Kind::Status, false),
    rule("ghost-shutdown", Kind::Status, false),
];

/// 出来事のレベルと target から種別を決める。どれにも当たらなければ None（残さない）。
///
/// 判定の順: debug・trace は残さない → 取り決めの script → 取り決めの error
/// → warn 以上は error → info は [`RULES`] を上から。
pub(crate) fn classify(level: Level, target: &str) -> Option<Kind> {
    // tracing の `Level` は詳しいほど大きい（TRACE > DEBUG > INFO > WARN > ERROR）。
    if level > Level::INFO {
        return None;
    }
    if target == TARGET_SCRIPT {
        return Some(Kind::Script);
    }
    if target == TARGET_ERROR || level < Level::INFO {
        return Some(Kind::Error);
    }
    RULES.iter().find(|r| r.matches(target)).map(|r| r.kind)
}

/// 欄 1 つ（tracing の訪問で得る 2 つの形）。
pub(crate) struct FieldText<'a> {
    pub name: &'a str,
    /// `{:?}` の形（文字列で渡された欄は引用符つき）。
    pub debug: &'a str,
    /// 文字列で渡された欄の生の値（そうでなければ None）。
    pub raw: Option<&'a str>,
}

/// 置き場へ積む前の記録。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Draft {
    pub kind: Kind,
    pub label: String,
    pub name: String,
    pub body: String,
}

/// 本文の上限（`char` の数）。
pub(crate) const BODY_CAP_CHARS: usize = 4096;
/// 上限を超えた本文の末尾に付ける印。
pub(crate) const TRUNCATED_SUFFIX: &str = " ...(truncated)";

/// 出来事 1 件から記録の下書きを作る。種別に当たらなければ None。欄は出来事に書かれた順で渡す。
///
/// 本文は `message` の欄の後に残りの欄を ` 名前=値`（値は `{:?}` の形）で続ける。
/// 取り決めの target の行だけ `ghost`・`label` を名と表示の語として読み本文から除く
/// （同名が複数なら最後のもの）。`log.` で始まる欄（`log` クレートから橋渡しされた行）は除く。
pub(crate) fn draft<'a>(
    level: Level,
    target: &str,
    fields: impl IntoIterator<Item = FieldText<'a>>,
) -> Option<Draft> {
    let kind = classify(level, target)?;
    let convention = target == TARGET_SCRIPT || target == TARGET_ERROR;
    let mut name = None;
    let mut label = None;
    let mut message = None;
    let mut rest = String::new();
    for f in fields {
        match f.name {
            "message" => message = Some(f.debug),
            "ghost" if convention => name = Some(f.raw.unwrap_or(f.debug)),
            "label" if convention => label = Some(f.raw.unwrap_or(f.debug)),
            n if n.starts_with("log.") => {}
            n => {
                rest.push(' ');
                rest.push_str(n);
                rest.push('=');
                rest.push_str(f.debug);
            }
        }
    }
    let mut body = match message {
        Some(m) => format!("{m}{rest}"),
        // メッセージが無ければ欄だけ（先頭の空白は付けない）。
        None => rest.strip_prefix(' ').unwrap_or(&rest).to_string(),
    };
    if let Some((cut, _)) = body.char_indices().nth(BODY_CAP_CHARS) {
        body.truncate(cut);
        body.push_str(TRUNCATED_SUFFIX);
    }
    Some(Draft {
        kind,
        label: label.unwrap_or(kind.default_label()).to_string(),
        name: name.unwrap_or(kind.default_name()).to_string(),
        body,
    })
}

#[cfg(test)]
#[path = "log_history_tests.rs"]
mod tests;
