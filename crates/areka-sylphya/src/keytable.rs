//! 鍵の表: 鍵ごとの住み場所・名前で書いてよいか・既定値を 1 つの表で決める。

use crate::actor::SetClass;
use crate::key::parse_dotted;
use crate::pull::PullShape;
use crate::vocab::dotted::{DOTTED_ROOTS, GENERIC_PROP_NAMES, SET_EFFECTIVE};

/// 値の住み場所。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Home {
    /// ロックなしで読む速い住み場所。
    Fast,
    /// メモリーの sqlite（揮発）。
    Memory,
    /// 層ごとのファイルの sqlite（覚える値）。
    File,
}

/// 外から来た名前で書いてよいか。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ByName {
    /// 書ける。
    Writable,
    /// 断る。
    Refused(Refusal),
}

/// 名前で書くのを断る理由。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// areka の中の決まった書き手だけが書く。
    FixedWriterOnly,
    /// 運行の命令として予約された名前（SET 有効群）。
    RuntimeCommand,
    /// 書けない正準の語彙。
    NotSettable,
}

/// 行の見つけ方（[`rule_of`] はこの並びの順に当てる）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyMatch {
    /// SET 有効群（[`SET_EFFECTIVE`]）に半角の英字の大小を区別せず完全一致。
    SetEffective,
    /// 名前の形（選択子を `(*)` に寄せた正準の文字・[`PullShape`] と同じ作り方）。
    Shape(&'static str),
    /// 根が [`DOTTED_ROOTS`] のどれか。
    Root,
    /// 葉が [`GENERIC_PROP_NAMES`] のどれか。
    Leaf,
    /// それ以外（自由な鍵・解けない名前）。
    Free,
}

/// 鍵の表の 1 行。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyRow {
    /// 行の見つけ方。
    pub matches: KeyMatch,
    /// 住み場所（`None` はどこにも書かない）。
    pub home: Option<Home>,
    /// 名前で書いてよいか。
    pub by_name: ByName,
    /// 値が無いときの既定値。
    pub default: Option<&'static str>,
}

const fn file(shape: &'static str) -> KeyRow {
    KeyRow {
        matches: KeyMatch::Shape(shape),
        home: Some(Home::File),
        by_name: ByName::Refused(Refusal::FixedWriterOnly),
        default: None,
    }
}

const fn fast(shape: &'static str) -> KeyRow {
    KeyRow {
        matches: KeyMatch::Shape(shape),
        home: Some(Home::Fast),
        by_name: ByName::Refused(Refusal::FixedWriterOnly),
        default: None,
    }
}

const fn memory(matches: KeyMatch, by_name: ByName) -> KeyRow {
    KeyRow {
        matches,
        home: Some(Home::Memory),
        by_name,
        default: None,
    }
}

static FREE_ROW: KeyRow = memory(KeyMatch::Free, ByName::Writable);

/// 本番の鍵の表。
pub static KEY_TABLE: &[KeyRow] = &[
    KeyRow {
        matches: KeyMatch::SetEffective,
        home: None,
        by_name: ByName::Refused(Refusal::RuntimeCommand),
        default: None,
    },
    file("areka.window.scope(*).x"),
    file("areka.window.scope(*).y"),
    file("areka.balloon.offset.scope(*).x"),
    file("areka.balloon.offset.scope(*).y"),
    file("areka.window.scope(*).adjacent"),
    file("areka.boot.count"),
    file("areka.vanish.count"),
    file("areka.last.ghost"),
    file("areka.last.balloon"),
    file("areka.last.shell"),
    file("areka.last.running"),
    file("areka.baseware.roots"),
    fast("currentghost.scope(*).scaling"),
    fast("currentghost.balloon.scope(*).scaling"),
    fast("currentghost.scope(*).currentmonitor.dpi"),
    fast("currentghost.balloon.scope(*).currentmonitor.dpi"),
    memory(KeyMatch::Root, ByName::Refused(Refusal::NotSettable)),
    memory(KeyMatch::Leaf, ByName::Refused(Refusal::NotSettable)),
    FREE_ROW,
];

/// 鍵の決まりを引く（解けない名前は自由な鍵）。
///
/// 見つけ方の順（SET 有効群 → 名前の形 → 根 → 葉 → 自由な鍵）で当て、表の行の並びには依らない。
/// 表は [`validate`] を通っている前提。自由な鍵の行が無い表では既定の自由な鍵の行を返す。
pub fn rule_of(table: &'static [KeyRow], key: &str) -> &'static KeyRow {
    let find = |pred: &dyn Fn(&KeyMatch) -> bool| table.iter().find(|row| pred(&row.matches));

    if SET_EFFECTIVE
        .iter()
        .any(|(name, _)| name.eq_ignore_ascii_case(key))
        && let Some(row) = find(&|m| *m == KeyMatch::SetEffective)
    {
        return row;
    }
    if let Ok(path) = parse_dotted(key) {
        let shape = PullShape::of(&path);
        if let Some(row) = find(&|m| matches!(m, KeyMatch::Shape(s) if *s == shape.as_str())) {
            return row;
        }
        let root = path
            .segs
            .first()
            .is_some_and(|s| DOTTED_ROOTS.contains(&s.name.as_str()));
        if root && let Some(row) = find(&|m| *m == KeyMatch::Root) {
            return row;
        }
        let leaf = path
            .segs
            .last()
            .is_some_and(|s| GENERIC_PROP_NAMES.contains(&s.name.as_str()));
        if leaf && let Some(row) = find(&|m| *m == KeyMatch::Leaf) {
            return row;
        }
    }
    find(&|m| *m == KeyMatch::Free).unwrap_or(&FREE_ROW)
}

/// 同じ形の行が 2 つ在る・形が解けない・自由な鍵の行が無い、を見つける。
pub fn validate(table: &[KeyRow]) -> Result<(), String> {
    let mut seen: Vec<KeyMatch> = Vec::with_capacity(table.len());
    for row in table {
        let matches = match row.matches {
            KeyMatch::Shape(shape) => {
                let parsed = PullShape::parse(shape)
                    .map_err(|e| format!("名前の形が解けない: {shape:?} ({e:?})"))?;
                if parsed.as_str() != shape {
                    return Err(format!(
                        "名前の形が正準の文字でない: {shape:?}（正準は {:?}）",
                        parsed.as_str()
                    ));
                }
                row.matches
            }
            other => other,
        };
        if seen.contains(&matches) {
            return Err(format!("同じ形の行が 2 つ在る: {matches:?}"));
        }
        seen.push(matches);
    }
    if !seen.contains(&KeyMatch::Free) {
        return Err("自由な鍵の行が無い".to_string());
    }
    Ok(())
}

/// SET 対象の名前を 3 つの分かれ方へ仕分ける（[`KEY_TABLE`] から導く・I/O なし）。
pub fn classify_set(key: &str) -> SetClass {
    match rule_of(KEY_TABLE, key).by_name {
        ByName::Writable => SetClass::StoreWrite,
        ByName::Refused(Refusal::RuntimeCommand) => SetClass::RuntimeCommand,
        ByName::Refused(Refusal::NotSettable | Refusal::FixedWriterOnly) => SetClass::NotSettable,
    }
}

#[cfg(test)]
#[path = "keytable_tests.rs"]
mod tests;
