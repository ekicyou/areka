//! 正準 key モデル: `PropPath` / `PathSeg` / `Selector` と点付き key パーサ（セレクタ 5 形）。
//!
//! 点付き key 文字列を [`PropPath`] へ完全解釈する（解釈の成否と解決の成否は分離——
//! 文法は全形受理、値は backing 次第）。design §「key モデル＋語彙台帳」Service Interface に従う。
//!
//! ## セレクタ 5 形の写像（design §key.rs コメント）
//!
//! | 形 | 例 | 表現 |
//! |----|----|------|
//! | ①括弧名選択 | `ghostlist(名前)` | `name="ghostlist"`, `selector=Some(ByName("名前"))` |
//! | ②`.index(ID)` | `…​.index(3)` | セグメント `name="index"`, `selector=Some(ByIndex(3))` |
//! | ③`.current` | `…​.current` | `name="current"`, `selector=None` |
//! | ④`.count` | `…​.count` | `name="count"`, `selector=None` |
//! | ⑤数値括弧 | `scope(0)` | `name="scope"`, `selector=Some(ByIndex(0))` |
//!
//! ## パーサ規則
//!
//! `.` で分割し、各セグメントは末尾に `(...)` を 1 つ持ち得る。括弧内容が全て数字なら
//! [`Selector::ByIndex`]、そうでなければ [`Selector::ByName`]。ただし名 `index` のセグメントは
//! 括弧内容が数値必須（非数値 → [`KeyParseError::BadIndex`]・タスク「非数値 index」を可観測にする
//! design §key.rs 裁定）。括弧内に `.` を含む名前は非対応（分割規則が優先・design 明記の単純分割）。

/// 点付き key の正準表現（セレクタ 5 形を完全収容・R1.3）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PropPath {
    pub segs: Vec<PathSeg>,
}

/// 1 セグメント（名 ＋ 任意のセレクタ）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PathSeg {
    pub name: String,
    pub selector: Option<Selector>,
}

/// 括弧セレクタ。名選択（`ByName`）と数値選択（`ByIndex`）の 2 形。
///
/// `.current` / `.count` はセレクタ無しの名前セグメントとして表現する（design §key.rs）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Selector {
    /// ①括弧名選択 例: `ghostlist(名前)`。
    ByName(String),
    /// ⑤数値括弧 例: `scope(0)`・②`.index(ID)` は `name="index"` ＋ `ByIndex`。
    ByIndex(u32),
}

/// `parse_dotted` の決定論的エラー。`at` は 0 始まりのセグメント序数。
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum KeyParseError {
    /// 入力が空文字列。
    #[error("empty key")]
    Empty,
    /// `(` に対応する `)` が無い（セグメント {at}）。
    #[error("unclosed paren at segment {at}")]
    UnclosedParen { at: usize },
    /// 空セグメント（先頭/末尾/連続ドット、または `(` 前の空名）（セグメント {at}）。
    #[error("empty segment at {at}")]
    EmptySegment { at: usize },
    /// 数値であるべき括弧内容が u32 として解釈できない（セグメント {at}）。
    #[error("bad index at segment {at}")]
    BadIndex { at: usize },
}

/// 点付き key 文字列 → [`PropPath`]。純粋・no I/O・同一入力→同一 Ok/Err（決定論・R2.5）。
pub fn parse_dotted(input: &str) -> Result<PropPath, KeyParseError> {
    if input.is_empty() {
        return Err(KeyParseError::Empty);
    }
    let mut segs = Vec::new();
    for (at, raw) in input.split('.').enumerate() {
        segs.push(parse_seg(raw, at)?);
    }
    Ok(PropPath { segs })
}

/// 1 セグメントの解釈。`at` はセグメント序数（エラー位置）。
fn parse_seg(seg: &str, at: usize) -> Result<PathSeg, KeyParseError> {
    match seg.find('(') {
        Some(open) => {
            // 括弧付き: 末尾が ')' でなければ未閉。
            if !seg.ends_with(')') {
                return Err(KeyParseError::UnclosedParen { at });
            }
            let name = &seg[..open];
            // open の '(' と末尾の ')' を除いた内容。
            let inner = &seg[open + 1..seg.len() - 1];
            if name.is_empty() {
                return Err(KeyParseError::EmptySegment { at });
            }
            let is_numeric = !inner.is_empty() && inner.bytes().all(|b| b.is_ascii_digit());
            let selector = if is_numeric {
                // 数値括弧 → ByIndex。u32 に収まらなければ BadIndex。
                match inner.parse::<u32>() {
                    Ok(n) => Selector::ByIndex(n),
                    Err(_) => return Err(KeyParseError::BadIndex { at }),
                }
            } else if name == "index" {
                // index(...) は数値必須（非数値 index → BadIndex・design §key.rs 裁定）。
                return Err(KeyParseError::BadIndex { at });
            } else {
                Selector::ByName(inner.to_string())
            };
            Ok(PathSeg {
                name: name.to_string(),
                selector: Some(selector),
            })
        }
        None => {
            if seg.is_empty() {
                return Err(KeyParseError::EmptySegment { at });
            }
            Ok(PathSeg {
                name: seg.to_string(),
                selector: None,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- セレクタ 5 形（design §key.rs のコメント写像）---

    #[test]
    fn form1_bracket_name_selection() {
        // ①括弧名選択: ghostlist(名前) → ByName("名前")
        let p = parse_dotted("ghostlist(名前)").unwrap();
        assert_eq!(p.segs.len(), 1);
        assert_eq!(p.segs[0].name, "ghostlist");
        assert_eq!(p.segs[0].selector, Some(Selector::ByName("名前".to_string())));
    }

    #[test]
    fn form2_index_id() {
        // ②.index(ID) → segment name="index" + ByIndex(ID)
        let p = parse_dotted("activeghostlist(foo).index(3)").unwrap();
        assert_eq!(p.segs.len(), 2);
        assert_eq!(p.segs[1].name, "index");
        assert_eq!(p.segs[1].selector, Some(Selector::ByIndex(3)));
    }

    #[test]
    fn form3_current() {
        // ③.current → name="current", selector=None
        let p = parse_dotted("currentghost.current").unwrap();
        assert_eq!(p.segs.len(), 2);
        assert_eq!(p.segs[1].name, "current");
        assert_eq!(p.segs[1].selector, None);
    }

    #[test]
    fn form4_count() {
        // ④.count → name="count", selector=None
        let p = parse_dotted("ghostlist.count").unwrap();
        assert_eq!(p.segs.len(), 2);
        assert_eq!(p.segs[1].name, "count");
        assert_eq!(p.segs[1].selector, None);
    }

    #[test]
    fn form5_numeric_bracket() {
        // ⑤数値括弧: scope(0) → name="scope", selector=ByIndex(0)
        let p = parse_dotted("areka.window.scope(0).x").unwrap();
        assert_eq!(p.segs.len(), 4);
        assert_eq!(p.segs[0].name, "areka");
        assert_eq!(p.segs[0].selector, None);
        assert_eq!(p.segs[2].name, "scope");
        assert_eq!(p.segs[2].selector, Some(Selector::ByIndex(0)));
        assert_eq!(p.segs[3].name, "x");
    }

    // --- 複数セグメント ---

    #[test]
    fn multi_segment_ext_property() {
        let p = parse_dotted("activeghostlist(x).ext.property").unwrap();
        assert_eq!(
            p,
            PropPath {
                segs: vec![
                    PathSeg { name: "activeghostlist".into(), selector: Some(Selector::ByName("x".into())) },
                    PathSeg { name: "ext".into(), selector: None },
                    PathSeg { name: "property".into(), selector: None },
                ]
            }
        );
    }

    #[test]
    fn multi_segment_system_baseware() {
        let p = parse_dotted("system.baseware").unwrap();
        assert_eq!(
            p,
            PropPath {
                segs: vec![
                    PathSeg { name: "system".into(), selector: None },
                    PathSeg { name: "baseware".into(), selector: None },
                ]
            }
        );
    }

    #[test]
    fn single_plain_segment() {
        let p = parse_dotted("baseware").unwrap();
        assert_eq!(p.segs.len(), 1);
        assert_eq!(p.segs[0].name, "baseware");
        assert_eq!(p.segs[0].selector, None);
    }

    // --- 不正形（決定論的 KeyParseError）---

    #[test]
    fn err_empty_input() {
        assert_eq!(parse_dotted(""), Err(KeyParseError::Empty));
    }

    #[test]
    fn err_double_dot_empty_segment() {
        assert_eq!(parse_dotted("a..b"), Err(KeyParseError::EmptySegment { at: 1 }));
    }

    #[test]
    fn err_leading_dot_empty_segment() {
        assert_eq!(parse_dotted(".a"), Err(KeyParseError::EmptySegment { at: 0 }));
    }

    #[test]
    fn err_trailing_dot_empty_segment() {
        assert_eq!(parse_dotted("a."), Err(KeyParseError::EmptySegment { at: 1 }));
    }

    #[test]
    fn err_unclosed_paren() {
        assert_eq!(parse_dotted("foo(bar"), Err(KeyParseError::UnclosedParen { at: 0 }));
    }

    #[test]
    fn err_bad_index_non_numeric() {
        // index(...) は数値必須（非数値 index → BadIndex・design §key.rs 裁定）
        assert_eq!(parse_dotted("index(abc)"), Err(KeyParseError::BadIndex { at: 0 }));
    }

    #[test]
    fn err_bad_index_overflow() {
        // 数値括弧が u32 に収まらない → BadIndex
        assert_eq!(
            parse_dotted("scope(99999999999999)"),
            Err(KeyParseError::BadIndex { at: 0 })
        );
    }

    #[test]
    fn err_empty_name_before_paren() {
        assert_eq!(parse_dotted("(x)"), Err(KeyParseError::EmptySegment { at: 0 }));
    }

    // --- 非 index 名の非数値括弧は ByName（design 写像）---

    #[test]
    fn non_index_non_numeric_paren_is_byname() {
        let p = parse_dotted("balloonlist(main)").unwrap();
        assert_eq!(p.segs[0].selector, Some(Selector::ByName("main".to_string())));
    }

    // --- 決定論（同一入力→同一 Ok/Err）---

    #[test]
    fn determinism_ok_repeatable() {
        let a = parse_dotted("areka.window.scope(2).y");
        let b = parse_dotted("areka.window.scope(2).y");
        assert_eq!(a, b);
    }

    #[test]
    fn determinism_err_repeatable() {
        let a = parse_dotted("index(abc)");
        let b = parse_dotted("index(abc)");
        assert_eq!(a, b);
        assert!(a.is_err());
    }
}
