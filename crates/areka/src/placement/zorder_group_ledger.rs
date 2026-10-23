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
//! # 明示の記法は書かれた順のまま採る
//!
//! 明示モードは窓 1 枚単位で並べられるので、同じスコープのキャラ窓をバルーン窓より
//! 手前に置く指定（逆転）も、2 窓の間に他スコープが挟まる指定（非隣接）も書ける。
//! 正典（ukadoc の `\![set,zorder,...]`）はこの順に「厳密に制御される」とし、
//! 「バルーンはキャラ窓の直上」という縛りを書いていない。そこで areka は**書かれた順を
//! 組み替えない**（`areka-P0-zorder-scope-inversion` 要件 1・2。完了 spec
//! `areka-P0-scope-zorder-pinning` の要件 2.4 の「隣接ブロックへ寄せる」を覆した）。
//!
//! 各スコープの並びの形は [`Normalization::order`]（[`WrittenOrder`]）として返し、
//! 呼び出し側が受理の記録に載せる（要件 1.4・2.3）。隣り合い・逆転・非隣接・
//! 逆転かつ非隣接の 4 通りである。
//!
//! # 片方だけ指名されたスコープには相棒窓を補う
//!
//! あるスコープのバルーン窓だけ（あるいはキャラ窓だけ）が書かれたときは、指名されなかった
//! 相棒窓を——`bN` だけならすぐ奥に `sN`、`sN` だけならすぐ手前に `bN`——へ加える
//! （完了 spec の要件 2.6・本 spec の要件 3）。結果として、明示モードで名前の挙がった
//! スコープは必ず 2 窓とも並びに居る（補い済み）。補ったスコープは常に隣り合いになる。
//!
//! 補ったことは [`Normalization::implied_partner`] に**どちらの窓を補ったか**まで
//! 載せて返す（要件 3.1・3.2 の「加えたことを記録する」）。
//!
//! 補いは最後の段（⑹）に置く。**要素数の判定（⑶）より後**なので、
//! `\![set,zorder,b0]` は要素 1 個として無視されたままである（要件 3.4）——補いで
//! 窓 2 枚に化けさせて受理してはならない。数値モードは⑷の展開が既に `[Balloon, Char]` を
//! 作っているので、補いも記録も起こらない。
//!
//! # 語彙は小文字ちょうど・trim は冗長化
//!
//! `balloonN`／`surfaceN`／`bN`／`sN` は小文字ちょうどで一致させる
//! （`windowposition::classify_x_vocab` と同じ流儀）。大文字混じりを黙って通すと、
//! 正典に無い受理を areka が独自に増やすことになる——受け付けない側に倒し、
//! 拒否理由として記録できる形で返す（要件 8.1／8.3）。
//! トークン前後の空白は上流（さくらスクリプトの引数分割・kv 層）で既に落ちており、
//! ここでの `trim` はその冗長化である（実際に届く値に対しては恒等）。

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
    /// 既にいずれかのグループに属しているスコープを含んでいた（要件 3.2）。
    /// 伴う値は衝突したスコープを、要素列に現れた順で 1 度ずつ並べたもの。
    ///
    /// 突き合わせは**スコープ**で行う。明示モードの `sN`／`bN` と数値モードの `N` は
    /// 同じスコープを指すものとして扱う（要件 3.3）——本層へ届く時点でモードの区別は
    /// [`GroupElement`] へ畳まれているので、この同一視は台帳側に規則を足さずに成立する。
    CrossGroupRedesignation {
        /// 既に塞がっていたスコープ（初出順・重複なし）。
        scopes: Vec<u32>,
    },
}

/// グループの出所。解除（`\![reset,zorder]`）が落とすのはタグ由来だけである。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupSource {
    /// `\![set,zorder,...]` タグ由来。解除で落ちる（要件 4.1）。
    Tag,
    /// shell descript の `seriko.zorder` 由来の基底。解除で落ちない（要件 4.1）。
    Descript,
}

/// 前後関係を確定させる窓の並び 1 本。並びの左側ほど手前。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZOrderGroup {
    /// セッション内で単調増加する識別子。解除で空いても配り直さない。
    pub id: u32,
    /// 窓 1 枚単位の要素列（書かれた順＋補った相棒）。
    pub members: Vec<GroupElement>,
    /// 出所。
    pub source: GroupSource,
}

/// 2 枚とも書かれたスコープの、書かれた並びの形（左ほど手前・補った後の並びで数える）。
///
/// 要件 1.4（どのスコープを逆転させたか）と 2.3（どのスコープを離して並べたか）を
/// 1 語ずつ読めるよう 4 通りに分ける。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WrittenOrder {
    /// バルーンのすぐ奥にキャラ窓（組の重なりと同じ）。補ったスコープも常にこれ。
    Adjacent,
    /// キャラ窓がバルーンのすぐ手前（隣り合った逆転）。
    Inverted,
    /// バルーンが手前・キャラ窓が奥で、間に他の窓がある（非隣接）。
    Split,
    /// キャラ窓が手前・バルーンが奥で、間に他の窓がある（逆転かつ非隣接）。
    InvertedSplit,
}

impl WrittenOrder {
    /// 受理の行の `normalized=` 欄の語。`Adjacent` は既存の読み手のために `false` のまま
    /// （要件 7.4）。
    pub fn as_str(self) -> &'static str {
        match self {
            WrittenOrder::Adjacent => "false",
            WrittenOrder::Inverted => "inverted",
            WrittenOrder::Split => "split",
            WrittenOrder::InvertedSplit => "inverted-split",
        }
    }
}

/// 明示モードで名前の挙がったスコープ 1 つぶんの記録（要件 1.4・2.3・3.1・3.2 の材料）。
///
/// 台帳は書かれた順を組み替えない。記録が語るのは「書かれた並びの形」と
/// 「補った相棒窓」の 2 つだけであり、呼び出し側（受理の行）がそのまま記録できる値として返す。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Normalization {
    /// 記録の対象のスコープ。
    pub scope: u32,
    /// 書かれた並びの形（補ったスコープは `Adjacent`）。
    pub order: WrittenOrder,
    /// 指名されなかったので**こちらが補った**窓の種別（要件 3.1・3.2）。
    ///
    /// `Some(Char)` は `bN` だけが書かれたので `sN` をすぐ奥へ、`Some(Balloon)` は
    /// `sN` だけが書かれたので `bN` をすぐ手前へ加えたことを表す。2 窓とも書かれて
    /// いたスコープは `None`。`Some(_)` のとき `order` は必ず `Adjacent`。
    pub implied_partner: Option<GroupWindowKind>,
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
pub(crate) fn parse_scope_id(digits: &str) -> Option<u32> {
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
/// 受理したときは「窓 1 枚単位の要素列（左ほど手前・書かれた順＋補った相棒）」と、
/// スコープごとの並びの形と補いの記録を返す。採用できないときは [`ZOrderReject`] を返し、
/// 要素列は一切返さない（部分適用の禁止・要件 8.1）。
///
/// # 判定の順序
///
/// ⑴解釈できないトークン（8.1）→ ⑵モード混在（2.3）→ ⑶要素 2 個未満（1.6）→
/// ⑷展開 → ⑸同一窓の重複（3.4／3.5）→ ⑹相棒窓の補いと並びの形の記録。
/// トークンを 1 つずつ読む段で落ちれば要素列そのものが組めないので、解釈不能が
/// 他のどの拒否よりも先に立つ。混在した指定も要素列を組めないため、重複を数える
/// 前に落とす。どの順で落ちても「変更を一切行わない」ことは変わらない。
/// 補いは受理が確定した後にだけ走る——拒否する指定に足しても意味が無いからである。
///
/// # 第 2 戻り値
///
/// [`Normalization`] は並びの形と補いの記録である（モジュール doc「明示の記法は
/// 書かれた順のまま採る」）。数値モードの並びは作者が窓 1 枚単位で書いたものではなく
/// エンジンが組んだものなので、数値モードの受理では記録は常に空である。
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

    // ⑹ 相棒窓の補いと並びの形の記録（`areka-P0-zorder-scope-inversion` 要件 1〜3）。
    //    ⑶より後に置くので、要素 1 個のタグが補いで 2 個に化けて受理されることはない。
    //    ⑵で混在を落としているので、ここへ来るグループは数値モードか明示モードの
    //    どちらか一方ちょうどである。数値モードは⑷の展開が必ず `[Balloon, Char]` を
    //    作り、かつその並びは作者が窓単位で書いたものではないので、補うものも
    //    記録するものも無い。
    if has_numeric {
        return Ok((elements, Vec::new()));
    }

    Ok(fill_partner_windows(elements))
}

/// 明示モードの要素列を**書かれた順のまま**受け取り、片方だけ名指しのスコープにだけ
/// 相棒窓を 1 枚補い、各スコープの並びの形を記録する（⑹・`areka-P0-zorder-scope-inversion`
/// 要件 1〜3）。
///
/// - 補い: `sN` だけなら `bN` を `sN` のすぐ手前、`bN` だけなら `sN` を `bN` のすぐ奥へ。
///   2 枚とも書かれたスコープには何も足さず、動かしもしない（要件 3.5）。
/// - 分類: 2 枚とも書かれたスコープは、補った後の並びでのバルーンの位置 `b` と
///   キャラ窓の位置 `c` から `c == b + 1` → 隣り合い、`b == c + 1` → 逆転、
///   `b < c`（離れている）→ 非隣接、`c < b`（離れている）→ 逆転かつ非隣接。
///   補ったスコープは相棒をすぐ隣に挟むので常に隣り合い。
///
/// 記録は明示モードで現れたスコープごとに、書かれた初出順で 1 件ずつ出す。
/// 呼び出し前提は⑸の重複検査を通っていること（各窓が高々 1 回しか現れない）。
fn fill_partner_windows(elements: Vec<GroupElement>) -> (Vec<GroupElement>, Vec<Normalization>) {
    let written =
        |scope: u32, kind: GroupWindowKind| elements.contains(&GroupElement { scope, kind });

    // 補いは各スコープを高々 1 枚ぶん増やす＝上限は入力の 2 倍。
    let mut filled = Vec::with_capacity(elements.len() * 2);
    for element in &elements {
        let scope = element.scope;
        match element.kind {
            GroupWindowKind::Char if !written(scope, GroupWindowKind::Balloon) => {
                filled.push(GroupElement {
                    scope,
                    kind: GroupWindowKind::Balloon,
                });
                filled.push(*element);
            }
            GroupWindowKind::Balloon if !written(scope, GroupWindowKind::Char) => {
                filled.push(*element);
                filled.push(GroupElement {
                    scope,
                    kind: GroupWindowKind::Char,
                });
            }
            _ => filled.push(*element),
        }
    }

    let position_of = |scope: u32, kind: GroupWindowKind| {
        filled
            .iter()
            .position(|other| *other == GroupElement { scope, kind })
    };
    let mut normalizations: Vec<Normalization> = Vec::new();
    for element in &elements {
        let scope = element.scope;
        if normalizations.iter().any(|n| n.scope == scope) {
            continue;
        }
        let implied_partner = match (
            written(scope, GroupWindowKind::Balloon),
            written(scope, GroupWindowKind::Char),
        ) {
            (true, true) => None,
            (true, false) => Some(GroupWindowKind::Char),
            _ => Some(GroupWindowKind::Balloon),
        };
        let order = match (
            implied_partner,
            position_of(scope, GroupWindowKind::Balloon),
            position_of(scope, GroupWindowKind::Char),
        ) {
            (None, Some(balloon), Some(char_window)) => {
                if char_window == balloon + 1 {
                    WrittenOrder::Adjacent
                } else if balloon == char_window + 1 {
                    WrittenOrder::Inverted
                } else if balloon < char_window {
                    WrittenOrder::Split
                } else {
                    WrittenOrder::InvertedSplit
                }
            }
            // 補ったスコープは相棒をすぐ隣に挟んだので常に隣り合い。
            _ => WrittenOrder::Adjacent,
        };
        normalizations.push(Normalization {
            scope,
            order,
            implied_partner,
        });
    }

    (filled, normalizations)
}

// =============================================================================
// 台帳（グループの唯一の正本）
// =============================================================================

/// スコープ窓 Z 順グループの台帳。プロセス内の状態のみを持つ。
///
/// 保持するのは descript 由来の基底（高々 1 つ）とタグ由来のグループ列である。
/// 不変条件は design「Data Models / Domain Model」の 4 つ——⑴どのスコープも高々
/// 1 グループ ⑵グループ内で同一窓は 1 回 ⑶明示の記法で名前の挙がったスコープは
/// 2 窓とも並びに居る（補い済み） ⑷基底は高々 1 つ。⑵⑶は [`parse_zorder_tokens`] が要素列を組む
/// 段で既に成立しており、台帳は受け取った要素列を組み替えないことでそれを保つ。
/// ⑴は追加の入口で、⑷は基底の入口で、それぞれ本モジュールが守る。
///
/// # 保存しない（要件 1.5）
///
/// 台帳は保存の仕組みに一切接続しない。「ゴーストが終了するまで有効で、次回起動へ
/// 持ち越さない」は、値がプロセスと同じ寿命を持つこと＝**何もしないこと**で成立する。
///
/// # 窓の存在を知らない（要件 1.4）
///
/// 要素はスコープ番号と窓種別のままで持つ。まだ現れていない窓のスコープも取り除かず、
/// 実在する窓だけを選ぶのは射影（drain 相）の担当である。そのため台帳の判断分岐は
/// 実機・実ディスプレイ無しで全て検査できる（要件 10.1）。
///
/// # 読み出しの並び
///
/// [`ZOrderGroupLedger::groups`] は「基底が先頭・以降はタグの追加順」で返す。これは
/// 決定論のための**読み出し順**であって前後関係の規則ではない。異なるグループどうしの
/// 相対的な前後関係は固定の規則で決めない（要件 3.6）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ZOrderGroupLedger {
    /// 基底（あれば先頭）＋タグ由来のグループ列。`groups()` がそのまま貸し出す。
    groups: Vec<ZOrderGroup>,
    /// 先頭が descript 由来の基底かどうか。真なら `groups[0].source == Descript`。
    has_base: bool,
    /// 次に配るグループ ID。解除で空いた ID も配り直さない。
    next_id: u32,
    /// 内容が動いた回数。射影を組み直すかどうかの判定に使う。
    version: u64,
}

impl ZOrderGroupLedger {
    /// タグ由来のグループとして追加を試みる（要件 3.1／3.2／3.3）。
    ///
    /// `members` は [`parse_zorder_tokens`] が受理した要素列であること（呼び出し規約）。
    /// モード混在・タグ内重複・要素 2 個未満・解釈不能の 4 分岐は解釈の段で既に落ちて
    /// おり、台帳が返す拒否は [`ZOrderReject::CrossGroupRedesignation`] だけである。
    ///
    /// 拒否したときは台帳を**一切**変更しない（要件 8.1 の部分適用禁止）。`members` を
    /// 動かすのは検査を通った後だけなので、部分適用が起きないことは制御の流れで保証される。
    ///
    /// グループ数にも要素数にも上限検査を設けない（要件 3.7）。
    pub fn try_add_tag_group(&mut self, members: Vec<GroupElement>) -> Result<u32, ZOrderReject> {
        let scopes = self.colliding_scopes(&members);
        if !scopes.is_empty() {
            return Err(ZOrderReject::CrossGroupRedesignation { scopes });
        }

        let id = self.allocate_id();
        self.groups.push(ZOrderGroup {
            id,
            members,
            source: GroupSource::Tag,
        });
        self.version += 1;
        Ok(id)
    }

    /// shell descript 由来の基底を据える（要件 5.1／5.3）。
    ///
    /// 基底は高々 1 つなので、前の基底は残らない（不変条件⑷）。据え直した基底には
    /// 新しい ID を配る——同じ要素列でも「据え直された別のグループ」だからである。
    ///
    /// # タグ由来のグループとの関係（正典沈黙箇所の裁量）
    ///
    /// 終状態は [`ZOrderGroupLedger::reset_to_descript`] と一致させる＝タグ由来の
    /// グループは残さない。要件 5.1 は基底の適用を**起動時・タグ実行より前**と定めて
    /// おり、タグ由来のグループと衝突する状況は正典の経路では起こらない。そのうえで
    /// design の署名は拒否を返す口を持たないため、衝突しても「黙って落とす」（要件 8.3
    /// が禁じる）ことも「不変条件⑴を破ったまま載せる」こともできない。要件 4.1 が
    /// 「基底へ戻った状態」を唯一の形として既に定義しているので、それに合わせる。
    ///
    /// # 空の要素列
    ///
    /// [`parse_zorder_tokens`] が受理する要素列は必ず 2 個以上（要件 1.6）なので、
    /// 空列は正典の経路では届かない。届いた場合は**基底なし**として扱う——0 要素の
    /// グループを載せると、要件 4.2 の「基底が無ければ既定状態へ戻る」が
    /// 「0 要素の基底へ戻る」に化けて意味が変わるからである。
    pub fn set_descript_base(&mut self, members: Vec<GroupElement>) {
        // 何かが載っていたか、これから載るものがあるときだけ内容が動く。
        let changed = !self.groups.is_empty() || !members.is_empty();

        self.groups.clear();
        self.has_base = false;

        if !members.is_empty() {
            let id = self.allocate_id();
            self.groups.push(ZOrderGroup {
                id,
                members,
                source: GroupSource::Descript,
            });
            self.has_base = true;
        }

        if changed {
            self.version += 1;
        }
    }

    /// `\![reset,zorder]` の適用（要件 4.1／4.2／4.3）。
    ///
    /// タグ由来のグループを全て落として基底へ戻す。基底が無ければ既定状態（グループ
    /// 0 本）へ戻る。基底は ID ごと生き残るので、基底が押さえているスコープは解除の
    /// 後も再指定の拒否対象であり続ける（要件 5.5）。落とされた側のスコープは空くので、
    /// 新しい組み合わせとして受け付けられる（要件 4.3）。
    pub fn reset_to_descript(&mut self) {
        let keep = usize::from(self.has_base);
        if self.groups.len() == keep {
            // 落とすものが無い＝内容は動かない。版も進めない。
            return;
        }

        self.groups.truncate(keep);
        self.version += 1;
    }

    /// グループの読み口（射影が読む）。基底が先頭・以降はタグの追加順。
    pub fn groups(&self) -> &[ZOrderGroup] {
        &self.groups
    }

    /// 内容が動いた回数。射影を組み直すかどうかの判定に使う。
    ///
    /// **内容が実際に動いたときだけ**進む。拒否された追加も、落とすものが無い解除も、
    /// 何も載らない基底の据え直しも進めない——射影を組み直す理由が無いからである。
    ///
    /// # 本番の消費点が無い（意図した状態）
    ///
    /// design は変化の検出を「台帳の版か `GhostWindows` の構成」と書いているが、指令消化の相
    /// （`zorder_drain.rs`）は**組み上げた鎖の計画そのものの突き合わせ**を選んだ。
    /// 結果の比較は版の判定を厳密に包含し、さらに「計画の変わらない版の上昇で印が立ち、
    /// 適用系が空振りする」要件 6.1 の穴も塞ぐ。ゆえにこの読み口は本番から呼ばれない。
    ///
    /// 版そのものは design が定めた台帳の一部であり、状態遷移の檻
    /// （`zorder_group_ledger_state_tests.rs`）が「いつ進み、いつ進まないか」を全数で
    /// 押さえている。読み口を消すとその不変条件を主張する手段が無くなるので、
    /// **この 1 項目だけ**に許可を与える（モジュール全体の許可は結線の着地とともに撤去済み）。
    #[allow(dead_code)]
    pub fn version(&self) -> u64 {
        self.version
    }

    /// 既にいずれかのグループに属しているスコープを、要素列に現れた順で 1 度ずつ拾う。
    ///
    /// 基底もタグ由来も区別せず参加させる（要件 5.5）。
    fn colliding_scopes(&self, members: &[GroupElement]) -> Vec<u32> {
        let occupied: HashSet<u32> = self
            .groups
            .iter()
            .flat_map(|group| group.members.iter().map(|member| member.scope))
            .collect();

        let mut hit: Vec<u32> = Vec::new();
        for member in members {
            if occupied.contains(&member.scope) && !hit.contains(&member.scope) {
                hit.push(member.scope);
            }
        }
        hit
    }

    /// 次のグループ ID を配る。解除で空いた ID も配り直さない（セッション内単調増加）。
    ///
    /// 飽和は 1 セッションで 42 億回の受理を要するため到達しない。到達した場合でも
    /// 台帳の整合は崩れない（ID の一意性だけが失われる）ので、飽和側へ倒しておく。
    fn allocate_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        id
    }
}

#[cfg(test)]
#[path = "zorder_group_ledger_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "zorder_group_ledger_state_tests.rs"]
mod state_tests;

// 書かれた順のまま受け取る解釈の檻（`areka-P0-zorder-scope-inversion` 要件 1〜3）。
#[cfg(test)]
#[path = "zorder_group_ledger_written_order_tests.rs"]
mod written_order_tests;

// 要件 10.2 の 10 分岐と、それを覆う実行されるテストの対応表（表が現実からずれたらここで赤くなる）。
#[cfg(test)]
#[path = "zorder_group_branch_coverage_tests.rs"]
mod branch_coverage_tests;

// `areka-P0-zorder-scope-inversion` 要件 8.2 の分岐の対応表（wintf 側の錨もファイルを読んで確かめる）。
#[cfg(test)]
#[path = "zorder_inversion_branch_coverage_tests.rs"]
mod zorder_inversion_branch_coverage_tests;

// 先送りしたプロパティ `currentghost.seriko.zorder` の不提供を固定する檻（要件 13.1／13.2／13.5）。
// 台帳の兄弟に置くのは、要件 13.2 後段「窓の重なりの状態を変更しない」の主張に台帳が要るため。
#[cfg(test)]
#[path = "zorder_property_deferral_tests.rs"]
mod property_deferral_tests;
