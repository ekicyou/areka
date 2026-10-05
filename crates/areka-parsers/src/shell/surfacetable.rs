//! 表情の表の転記（parse_surfacetable）— surfacetable.txt の文面を行の列へ写す。
//!
//! 転記だけを行い、載せる・載せないの判断・既定の名前・並べ替えは持たない（使い手の
//! `get_expression_table` の側）。`__disabled` の group の行も `__parts` の行も名前の省略も
//! そのまま写す。読めない行は行番号だけを返し、記録もファイルの読み取りもしない（失敗しない）。
//!
//! 行の読み分け（上から順に当てる）:
//! - 落とす空白は ASCII の空白とタブだけ（全角の空白は字として残す）。行はまず前後を落とす。
//! - 行は最初の `,` で前と後ろに分ける。前（見出し語・サーフェス ID）は前後を落とし、
//!   後ろ（グループ名・名前）は書かれたとおり。`scope` の数値だけは前後を落としてから読む。
//! - 見出し語 `charset`・`version`・`option`・`group`・`scope` は大小を区別しない。
//! - 数値は ASCII の数字だけの並びで `u32` に収まるもの（先頭の 0 は許す）。
//! - 空行・`//`・`{` だけの行・`version`・`option`・知っている `charset` は読み飛ばす。
//! - `}` だけの行は今の group を閉じる。行末の `}`（`100,黒塗り}`）は名前の一部。
//! - `scope` は今の group の既に写した行にも当てる（後が勝つ）。group の外・数値でないなら読めない行。

/// surfacetable.txt の転記。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SurfaceTable {
    /// `サーフェスID,名前` の行（書かれた順）。
    pub rows: Vec<SurfaceTableRow>,
    /// 読めなかった行の行番号（1 から数える・書かれた順）。
    pub unreadable: Vec<usize>,
}

/// `サーフェスID,名前` の 1 行。
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceTableRow {
    pub id: u32,
    /// 名前（省略なら空。`__parts` や行末の `}` もそのまま）。
    pub name: String,
    /// 属する group のグループ名（group の外・グループ名が空なら空。`__disabled` もそのまま）。
    pub group: String,
    /// 属する group の scope（group の外・scope の行が無ければ 0）。
    pub scope: u32,
}

/// 開いている group（グループ名・scope・その group の最初の行の位置）。
struct OpenGroup {
    name: String,
    scope: u32,
    first_row: usize,
}

/// surfacetable.txt の文面（文字コードを読み終えた文字列）を行の列へ転記する。
pub fn parse_surfacetable(text: &str) -> SurfaceTable {
    let mut table = SurfaceTable::default();
    let mut group: Option<OpenGroup> = None;

    for (index, raw) in text.lines().enumerate() {
        let line = trim_blank(raw);
        if line.is_empty() || line.starts_with("//") || line == "{" {
            continue;
        }
        if line == "}" {
            group = None;
            continue;
        }
        let Some((head, tail)) = line.split_once(',') else {
            table.unreadable.push(index + 1);
            continue;
        };
        let head = trim_blank(head);
        let keyword = |word: &str| head.eq_ignore_ascii_case(word);

        if keyword("version") || keyword("option") {
            continue;
        }
        if keyword("charset") {
            if encoding_rs::Encoding::for_label(tail.as_bytes()).is_none() {
                table.unreadable.push(index + 1);
            }
            continue;
        }
        if keyword("group") {
            group = Some(OpenGroup {
                name: tail.to_string(),
                scope: 0,
                first_row: table.rows.len(),
            });
            continue;
        }
        if keyword("scope") {
            match (group.as_mut(), number(trim_blank(tail))) {
                (Some(open), Some(scope)) => {
                    open.scope = scope;
                    for row in &mut table.rows[open.first_row..] {
                        row.scope = scope;
                    }
                }
                _ => table.unreadable.push(index + 1),
            }
            continue;
        }
        match number(head) {
            Some(id) => table.rows.push(SurfaceTableRow {
                id,
                name: tail.to_string(),
                group: group.as_ref().map(|g| g.name.clone()).unwrap_or_default(),
                scope: group.as_ref().map_or(0, |g| g.scope),
            }),
            None => table.unreadable.push(index + 1),
        }
    }
    table
}

/// 前後の ASCII の空白とタブだけを落とす（全角の空白は残す）。
fn trim_blank(s: &str) -> &str {
    s.trim_matches([' ', '\t'])
}

/// ASCII の数字だけの並びで `u32` に収まるもの（`+5`・`-1`・空は数値でない）。
fn number(s: &str) -> Option<u32> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}
