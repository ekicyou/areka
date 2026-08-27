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

// 本モジュールが載せるのは語彙＋トークン解釈＋拒否判定＋台帳の状態遷移（task 1.1〜1.3）
// までであり、非 test ビルド（bin 本体）からの消費点はまだ無い。drain 相の結線が着地する
// まで dead_code が出るが、これは段階実装の想定内である（`move_cue.rs:37` および
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
    /// 窓 1 枚単位の要素列（正規化済み）。
    pub members: Vec<GroupElement>,
    /// 出所。
    pub source: GroupSource,
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

// =============================================================================
// 台帳（グループの唯一の正本）
// =============================================================================

/// スコープ窓 Z 順グループの台帳。プロセス内の状態のみを持つ。
///
/// 保持するのは descript 由来の基底（高々 1 つ）とタグ由来のグループ列である。
/// 不変条件は design「Data Models / Domain Model」の 4 つ——⑴どのスコープも高々
/// 1 グループ ⑵グループ内で同一窓は 1 回 ⑶同一スコープの 2 窓は `[Balloon, Char]`
/// の隣接ブロック ⑷基底は高々 1 つ。⑵⑶は [`parse_zorder_tokens`] が要素列を組む
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
