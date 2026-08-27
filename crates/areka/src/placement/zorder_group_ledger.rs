//! スコープ窓 Z 順グループの台帳（正本）と、タグ／descript 共通のトークン解釈・
//! 拒否判定（純関数・Win32／ECS 非依存）。
//!
//! 本モジュールは `\![set,zorder,...]` と shell descript の `seriko.zorder` の
//! **どちらからも同じ形で呼ばれる**解釈層である（design「ZOrderGroupLedger」・
//! 要件 5.2「descript の値をタグと同じ書式で解釈する」）。タグを実行したスコープを
//! 見ないので、解釈の結果は呼び出し元のスコープに依らない（要件 1.7）。
//!
//! # 何を持ち、何を持たないか
//!
//! 正本は scope 番号と窓種別のままで持つ。`Entity` も `HWND` も知らないため、
//! 判断の分岐は実機・実ディスプレイ無しで全て検査できる（要件 10.1）。
//! Win32／`bevy_ecs`／`tracing` のいずれにも依存しない——拒否は値として返し、
//! 記録は呼び出し側が行う（design「Error Handling」・要件 8.3）。
//!
//! # 数値モードは明示モードの特例である
//!
//! 数値モードの要素 `N` は解釈の時点で `[Balloon(N), Char(N)]` の 2 枚へ展開する
//! （要件 1.2「バルーン窓をキャラ窓の直上に保ったままスコープ単位のかたまりとして
//! 並べる」）。以降の層は明示モードの要素列 1 種類だけを見ればよい。
//!
//! # 要素数は展開前に数える
//!
//! 要件 1.6 の「要素 2 個未満」は**展開前の要素**で数える。用語定義が数値モードの
//! 要素をスコープ単位と定めており、要件 1.1 も「2 個以上のスコープ ID」と言うから
//! である。展開後の窓枚数で数えると `\![set,zorder,0]` が窓 2 枚として受理されてしまい、
//! 要件 1.1 と食い違う。
//!
//! # 同一スコープの 2 窓は隣接ブロックへ寄せる
//!
//! 明示モードは窓 1 枚単位で並べられるので、同じスコープのキャラ窓をバルーン窓より
//! 手前に置く指定（反転）も、2 窓の間に他スコープが挟まる指定（非隣接）も書ける。
//! どちらも「バルーン窓はそのスコープのキャラ窓の直上」という既存の不変条件
//! （要件 6.3）と衝突するため、areka は不変条件の側を優先し、**そのスコープの要素が
//! 最初に現れた位置**へ `[Balloon, Char]` の隣接ブロックとして寄せる（要件 2.4）。
//! 反転は「最初に現れたのがキャラ窓だった」場合、非隣接は「2 枚目までの間に他スコープが
//! 挟まっていた」場合にすぎず、規則は 1 つで足りる（design「スコープブロック正規化」・
//! research R6）。
//!
//! 採用しなかったことは呼び出し側が記録できるよう [`Normalization`] として返す
//! （要件 8.3）。`reordered` の意味は**「そのスコープについて、作者が書いた順を
//! そのままの形では採用しなかった」**である——バルーンの直後にキャラ窓と書かれて
//! いたときだけ `false`、反転も非隣接も `true` になる。design は欄を宣言するのみで
//! 述語を書いていないため、本モジュールでこう定める。
//!
//! # 語彙は小文字ちょうど・trim は冗長化
//!
//! `balloonN`／`surfaceN`／`bN`／`sN` は小文字ちょうどで一致させる
//! （`windowposition::classify_x_vocab` と同じ流儀）。大文字混じりを黙って通すと、
//! 正典に無い受理を areka が独自に増やすことになる——受け付けない側に倒し、
//! 拒否理由として記録できる形で返す（要件 8.1／8.3）。
//! トークン前後の空白は上流（さくらスクリプトの引数分割・kv 層）で既に落ちており、
//! ここでの `trim` はその冗長化である（実際に届く値に対しては恒等）。

// 本 task（1.1）が載せるのは語彙＋トークン解釈＋拒否判定の純関数までであり、非 test
// ビルド（bin 本体）からの消費点はまだ無い。台帳本体（task 1.3）と drain 相の結線が
// 着地するまで dead_code が出るが、これは段階実装の想定内である（`move_cue.rs:37` および
// `input_events/throttle.rs:13-17` と同じ扱い。**本 allow は結線着地時に撤去する**）。
#![allow(dead_code)]

use std::collections::HashSet;

// =============================================================================
// 語彙
// =============================================================================

/// グループの要素が指す窓の種別。1 スコープはこの 2 種を 1 枚ずつ持つ。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GroupWindowKind {
    /// バルーン窓（`balloonN`／`bN`）。
    Balloon,
    /// キャラ窓＝立ち絵の窓（`surfaceN`／`sN`）。
    Char,
}

/// グループの要素＝窓 1 枚。並びの左側ほど手前。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GroupElement {
    /// スコープ番号（本体側 0・相方側 1・追加キャラ 2 以降）。
    pub scope: u32,
    /// 窓種別。
    pub kind: GroupWindowKind,
}

/// 指定を採用しなかった理由。呼び出し側がそのまま記録できる値として返す
/// （design「Error Handling」＝ warn ＋受け取ったトークン列・要件 8.3）。
///
/// どの理由で落ちても**そのタグ・その descript 行による変更は一切行わない**
/// （部分適用の禁止・要件 8.1）。要素列は `Ok` の側にしか存在しないので、
/// 部分適用が起きないことは戻り値の型そのもので保証される。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ZOrderReject {
    /// 数値のみの要素と `b`／`s` を伴う要素が 1 つの指定に混在していた（要件 2.3）。
    ///
    /// 混在は指定全体の性質であって特定の要素の罪ではないので、値を伴わない。
    /// 記録に載せる「受け取ったトークン列」は呼び出し側が元から持っている。
    ModeMixed,
    /// 同じ窓を指す要素が 2 回以上現れた（要件 3.4）。伴う値は最初に重複した窓。
    ///
    /// 数えるのは**窓**であってスコープではない。`bN` と `sN` は別々の窓を指すので、
    /// 両方が並ぶことは重複ではない（要件 3.5）。
    DuplicateElement {
        /// 2 回目に現れた窓。
        element: GroupElement,
    },
    /// 要素が 2 個に満たない（要件 1.6）。伴う値は展開前に数えた要素数。
    TooFewElements {
        /// 解釈できた要素の個数（0 または 1）。
        count: usize,
    },
    /// 解釈できないトークンがあった（要件 8.1）。伴う値は受け取ったトークンそのもの
    /// （`trim` 前・大小文字もそのまま）。
    UnparsableToken {
        /// 解釈できなかったトークン。
        token: String,
    },
}

/// 同一スコープの 2 窓を `[Balloon, Char]` の隣接ブロックへ寄せた記録（要件 2.4 の材料）。
///
/// 明示モードの指定順が同一スコープ内でキャラ窓をバルーン窓より手前に置くことを
/// 要求しても、「バルーンはキャラ窓の直上」という既存の不変条件を優先する。
/// 採用しなかったことは呼び出し側が記録できるよう、値として返す（要件 8.3）。
///
/// 記録が出るのは**2 窓そろったスコープ**についてだけである。窓が 1 枚しか書かれて
/// いないスコープは寄せる相手が居ない＝調停そのものが起きない。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Normalization {
    /// 調整の対象となったスコープ。
    pub scope: u32,
    /// 作者が書いた順をそのままの形では採用しなかったか。
    ///
    /// `false` はバルーン窓の**直後**にキャラ窓と書かれていた場合ちょうど。
    /// 反転（キャラ窓が先）も非隣接（間に他スコープ）も `true` になる
    /// （モジュール doc「同一スコープの 2 窓は隣接ブロックへ寄せる」）。
    pub reordered: bool,
}

// =============================================================================
// トークン解釈
// =============================================================================

/// 明示モード完全形のバルーン窓接頭辞。
const BALLOON_PREFIX: &str = "balloon";
/// 明示モード完全形のキャラ窓接頭辞。
const CHAR_PREFIX: &str = "surface";
/// 明示モード省略形のバルーン窓接頭辞（要件 2.2）。
const BALLOON_SHORT_PREFIX: char = 'b';
/// 明示モード省略形のキャラ窓接頭辞（要件 2.2）。
const CHAR_SHORT_PREFIX: char = 's';

/// 1 トークンの解釈結果。数値モードと明示モードの区別を、混在判定（要件 2.3）が
/// 終わるまで保つための内部表現。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParsedElement {
    /// 数値モードの要素（スコープ単位）。展開すると窓 2 枚になる。
    Scope(u32),
    /// 明示モードの要素（窓 1 枚単位）。
    Window(GroupElement),
}

/// スコープ ID の数字列を読む。`+1` や `-1`、空文字、桁あふれは受け付けない。
///
/// `str::parse` は `+1` を通してしまうため、先に ASCII 数字だけであることを確かめる
/// ——正典に無い書式を areka が独自に受理しないようにするためである。
fn parse_scope_id(digits: &str) -> Option<u32> {
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    digits.parse::<u32>().ok()
}

/// 1 トークンを要素として解釈する。解釈できなければ `None`。
///
/// 完全形（`balloon`／`surface`）を省略形（`b`／`s`）より先に試す。`balloon1` は
/// `b` 接頭辞にも当たるので、順序を入れ替えると `alloon1` を数字列として読もうとして
/// 落ちる。完全形の接頭辞に当たった時点で判定は確定させ、省略形へは落とさない。
fn interpret_token(token: &str) -> Option<ParsedElement> {
    let body = token.trim();

    if let Some(digits) = body.strip_prefix(BALLOON_PREFIX) {
        return window_element(digits, GroupWindowKind::Balloon);
    }
    if let Some(digits) = body.strip_prefix(CHAR_PREFIX) {
        return window_element(digits, GroupWindowKind::Char);
    }
    if let Some(digits) = body.strip_prefix(BALLOON_SHORT_PREFIX) {
        return window_element(digits, GroupWindowKind::Balloon);
    }
    if let Some(digits) = body.strip_prefix(CHAR_SHORT_PREFIX) {
        return window_element(digits, GroupWindowKind::Char);
    }

    parse_scope_id(body).map(ParsedElement::Scope)
}

/// 接頭辞を落とした残りを窓 1 枚の要素として組む。
fn window_element(digits: &str, kind: GroupWindowKind) -> Option<ParsedElement> {
    parse_scope_id(digits).map(|scope| ParsedElement::Window(GroupElement { scope, kind }))
}

/// トークン列を正規化済みの要素列へ解釈する（タグ・descript 共通・呼び出し元スコープ非依存）。
///
/// 受理したときは「窓 1 枚単位の要素列（左ほど手前）」と、同一スコープ内の隣接を
/// 優先して指定順を調整した記録を返す。採用できないときは [`ZOrderReject`] を返し、
/// 要素列は一切返さない（部分適用の禁止・要件 8.1）。
///
/// # 判定の順序
///
/// ⑴解釈できないトークン（8.1）→ ⑵モード混在（2.3）→ ⑶要素 2 個未満（1.6）→
/// ⑷展開 → ⑸同一窓の重複（3.4／3.5）→ ⑹スコープブロック正規化（2.4）。
/// トークンを 1 つずつ読む段で落ちれば要素列そのものが組めないので、解釈不能が
/// 他のどの拒否よりも先に立つ。混在した指定も要素列を組めないため、重複を数える
/// 前に落とす。どの順で落ちても「変更を一切行わない」ことは変わらない。
/// 正規化は受理が確定した後にだけ走る——拒否する指定を整えても意味が無いからである。
///
/// # 第 2 戻り値
///
/// [`Normalization`] は要件 2.4 の調整記録である（モジュール doc「同一スコープの
/// 2 窓は隣接ブロックへ寄せる」）。数値モードは⑷の展開の時点で必ず隣接ブロックに
/// なるうえ、その並びは作者が書いた指定順ではなくエンジンが組んだものなので、
/// 数値モードの受理では調整記録は常に空である。
pub fn parse_zorder_tokens(
    tokens: &[&str],
) -> Result<(Vec<GroupElement>, Vec<Normalization>), ZOrderReject> {
    // ⑴ 解釈できないトークンがあれば、その場で指定全体を落とす（要件 8.1）。
    let mut parsed = Vec::with_capacity(tokens.len());
    for token in tokens {
        match interpret_token(token) {
            Some(element) => parsed.push(element),
            None => {
                return Err(ZOrderReject::UnparsableToken {
                    token: (*token).to_owned(),
                });
            }
        }
    }

    // ⑵ 数値モードと明示モードの混在（要件 2.3）。
    let has_numeric = parsed
        .iter()
        .any(|element| matches!(element, ParsedElement::Scope(_)));
    let has_explicit = parsed
        .iter()
        .any(|element| matches!(element, ParsedElement::Window(_)));
    if has_numeric && has_explicit {
        return Err(ZOrderReject::ModeMixed);
    }

    // ⑶ 要素数は展開前に数える（要件 1.6・モジュール doc「要素数は展開前に数える」）。
    if parsed.len() < 2 {
        return Err(ZOrderReject::TooFewElements {
            count: parsed.len(),
        });
    }

    // ⑷ 数値モードを窓 2 枚へ展開する（要件 1.2）。バルーンが自スコープのキャラ窓の
    //    直上に来るよう `[Balloon, Char]` の順で並べる。
    let mut elements = Vec::with_capacity(parsed.len() * 2);
    for element in &parsed {
        match *element {
            ParsedElement::Scope(scope) => {
                elements.push(GroupElement {
                    scope,
                    kind: GroupWindowKind::Balloon,
                });
                elements.push(GroupElement {
                    scope,
                    kind: GroupWindowKind::Char,
                });
            }
            ParsedElement::Window(window) => elements.push(window),
        }
    }

    // ⑸ 同じ窓を指す要素の重複（要件 3.4）。数えるのは窓であってスコープではないので、
    //    `bN` と `sN` の併存は重複にならない（要件 3.5）。
    let mut seen: HashSet<GroupElement> = HashSet::with_capacity(elements.len());
    for element in &elements {
        if !seen.insert(*element) {
            return Err(ZOrderReject::DuplicateElement { element: *element });
        }
    }

    // ⑹ スコープブロック正規化（要件 2.4・research R6 の一元処理）。
    //    ⑵で混在を落としているので、ここへ来るグループは数値モードか明示モードの
    //    どちらか一方ちょうどである。数値モードは⑷の展開が必ず `[Balloon, Char]` の
    //    隣接ブロックを作り、かつその並びは作者の指定順ではないので、寄せるものも
    //    記録するものも無い（要件 2.4 は「明示モードの指定順が」と明示モードを名指しする）。
    if has_numeric {
        return Ok((elements, Vec::new()));
    }

    Ok(normalize_scope_blocks(elements))
}

/// 同一スコープの 2 窓を `[Balloon, Char]` の隣接ブロックへ寄せる（明示モード専用）。
///
/// 寄せ先は**そのスコープの要素が最初に現れた位置**である。反転（`s1,b1`）も
/// 非隣接（`b1,s0,s1,b0`）もこの 1 つの規則で片付く（モジュール doc「同一スコープの
/// 2 窓は隣接ブロックへ寄せる」）。
///
/// 2 窓そろっていないスコープの要素は書かれた位置のまま残す。動かすのは 1 スコープの
/// 2 窓だけなので、他スコープの要素どうしの相対順は入力のまま保たれる——要件 2.5 の
/// 「グループに属さない窓を動かさない」と同じ発想を、グループの内側にも効かせる。
///
/// 呼び出し前提は⑸の重複検査を通っていること。各スコープの各窓が高々 1 回しか
/// 現れないため、位置は探索で一意に定まる。
fn normalize_scope_blocks(elements: Vec<GroupElement>) -> (Vec<GroupElement>, Vec<Normalization>) {
    let position_of = |target: GroupElement| elements.iter().position(|other| *other == target);

    let mut normalized = Vec::with_capacity(elements.len());
    let mut normalizations = Vec::new();

    for (index, element) in elements.iter().enumerate() {
        let scope = element.scope;
        let (Some(balloon_at), Some(char_at)) = (
            position_of(GroupElement {
                scope,
                kind: GroupWindowKind::Balloon,
            }),
            position_of(GroupElement {
                scope,
                kind: GroupWindowKind::Char,
            }),
        ) else {
            // 2 窓そろっていないスコープ＝寄せる相手が居ない。位置も記録も動かさない。
            normalized.push(*element);
            continue;
        };

        if index != balloon_at.min(char_at) {
            // 同じスコープの 2 枚目。ブロックは 1 枚目の位置で既に組み終えている。
            continue;
        }

        normalized.push(GroupElement {
            scope,
            kind: GroupWindowKind::Balloon,
        });
        normalized.push(GroupElement {
            scope,
            kind: GroupWindowKind::Char,
        });
        normalizations.push(Normalization {
            scope,
            // 書かれたとおりに採用できたのは「バルーンの直後にキャラ窓」のときだけ。
            // 反転（`char_at < balloon_at`）も非隣接（間に他スコープ）もここを通らない。
            reordered: char_at != balloon_at + 1,
        });
    }

    (normalized, normalizations)
}

#[cfg(test)]
#[path = "zorder_group_ledger_tests.rs"]
mod tests;
