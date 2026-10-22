//! bind（着せ替え）構築モジュール。
//!
//! bindgroup の既定オン集合（番号）から、合成入力が要求する静的 bind 集合
//! （[`areka_emo_compose::BindSet`]）をアクター構築時に一度だけ組み立てる。
//!
//! bindgroup 番号は animation ID の**恒等写像**（決定3 / R-2）。中間変換・添字付け・
//! 特別扱いは一切行わず、番号をそのまま [`BindSet::from_ids`] へ流す。整列・重複除去は
//! `from_ids` が担う（`BindSet` は昇順・dedup 済み）。

/// bindgroup default 番号集合 → 静的 [`BindSet`]（恒等写像・`from_ids` が整列/dedup）。
///
/// bindgroup 番号 = animation ID（決定3 / R-2）。呼び手（アクター構築時）が渡した
/// 既定オン集合をそのまま `BindSet::from_ids` へ渡すのみ。状態を持たず、構築時に一度だけ
/// 呼ばれる純関数（要件 4.1）。
///
/// [`BindSet`]: areka_emo_compose::BindSet
/// [`BindSet::from_ids`]: areka_emo_compose::BindSet::from_ids
pub fn build_static_bindset(default_on: &[u32]) -> areka_emo_compose::BindSet {
    areka_emo_compose::BindSet::from_ids(default_on.iter().copied())
}

use std::collections::{BTreeMap, BTreeSet};

use areka_parsers::package::BindGroupDefaults;
use areka_sakura::ActorKey;

/// スコープの番号ごとの既定の着せ替えの組（spec: areka-P0-offline-surface-dump 要件 14.3）。
///
/// [`DefaultBinds::uniform`] はどのスコープにも同じ組を返す（1 組を全スコープで共有する今の
/// 振る舞い）。[`DefaultBinds::single`] は 1 つのスコープだけに組を持つ。[`DefaultBinds::for_scope`]
/// は書かれていないスコープ・数でないスコープに空の組（`uniform` なら共有の組）を返す。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DefaultBinds {
    /// スコープの番号ごとの組。
    by_scope: BTreeMap<u32, areka_emo_compose::BindSet>,
    /// `by_scope` に無いスコープへ返す組（`uniform` だけが空でない組を持つ）。
    fallback: areka_emo_compose::BindSet,
}

impl DefaultBinds {
    /// どのスコープにも同じ組（今の振る舞い）。
    pub fn uniform(binds: areka_emo_compose::BindSet) -> Self {
        Self {
            by_scope: BTreeMap::new(),
            fallback: binds,
        }
    }

    /// そのスコープだけに組を持つ（ほかのスコープは空の組）。
    pub fn single(scope: u32, binds: areka_emo_compose::BindSet) -> Self {
        Self {
            by_scope: BTreeMap::from([(scope, binds)]),
            fallback: areka_emo_compose::BindSet::default(),
        }
    }

    /// シェルの着せ替えの読みから、スコープ 0 に `sakura.*`・1 に `kero.*`・2 以上に `char*.*` の
    /// `default,1` を当てる。書かれていないスコープは空の組（要件 14.1）。
    pub fn from_groups(groups: &BindGroupDefaults) -> Self {
        let mut by_scope = BTreeMap::from([
            (0, build_static_bindset(&groups.sakura_default_on)),
            (1, build_static_bindset(&groups.kero_default_on)),
        ]);
        for (n, ids) in &groups.char_default_on {
            by_scope.insert(*n, build_static_bindset(ids));
        }
        Self {
            by_scope,
            fallback: areka_emo_compose::BindSet::default(),
        }
    }

    /// スコープで引く。書かれていないスコープ・数でないスコープは `fallback`（`uniform` 以外は空）。
    pub fn for_scope(&self, scope: &ActorKey) -> &areka_emo_compose::BindSet {
        scope
            .as_str()
            .parse::<u32>()
            .ok()
            .and_then(|n| self.by_scope.get(&n))
            .unwrap_or(&self.fallback)
    }
}

/// bind 名前解決の**名前空間**（本体側＝sakura／相方側＝kero を型で峻別する・D7）。
///
/// scope（`ActorKey`）から [`scope_namespace`] が写像し、[`BindResolver::resolve`] が
/// どの名前表を引くかを選ぶ。3 人目以降（`char{N}.bindgroup`）は番号つきの [`BindNamespace::Char`]。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindNamespace {
    /// 本体側（scope `"0"`・shell の `sakura.bindgroup`）。
    Sakura,
    /// 相方側（scope `"1"`・shell の `kero.bindgroup`）。M1 では名前表まで持つが機構は scope 非依存。
    Kero,
    /// 3 人目以降（scope `"N"`・N は 2 以上・shell の `char{N}.bindgroup`・要件 14.2）。
    Char(u32),
}

/// カテゴリの着せ替え選択ポリシー（ukadoc 正典の 3 値・bindopt 設計 D2）。
///
/// `bindoption*.group` のオプション宣言から [`BindResolver::policy`] が導出する。「排他か」
/// 「解除できるか」の**解釈**はこの型へ一元化され、適用側（actor）は値を照合するだけになる。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindChoicePolicy {
    /// `mustselect` 宣言: ちょうど 1 個（着衣＝排他置換・脱衣＝無視〔解除不可〕）。
    MustSelect,
    /// 非宣言（正典の既定）: 高々 1 個（着衣＝排他置換・脱衣＝除去〔解除可〕）。
    Default,
    /// `multiple` 宣言: 複数可（着衣＝加算・脱衣＝除去）。
    Multiple,
}

/// `bindoption` 宣言のスコープ別カテゴリ名集合（名前付き搬送・bindopt 設計 D2/D3）。
///
/// 同型の `BTreeSet<String>` が 4 本並ぶため、位置引数で直列に並べず名前付きフィールドで運ぶ
/// （取り違えをコンパイラに検出させる）。[`Default`] 実装＝全集合空＝全カテゴリが正典の既定
/// （[`BindChoicePolicy::Default`]）。
///
/// [`Default`]: std::default::Default
#[derive(Clone, Debug, Default)]
pub struct BindOptionDecls {
    /// 本体側（sakura）で `mustselect` と宣言されたカテゴリ名（bindopt 1.2）。
    pub sakura_mustselect: BTreeSet<String>,
    /// 相方側（kero）で `mustselect` と宣言されたカテゴリ名（bindopt 1.2）。
    pub kero_mustselect: BTreeSet<String>,
    /// 本体側（sakura）で `multiple` と宣言されたカテゴリ名（bindopt 1.1）。
    pub sakura_multiple: BTreeSet<String>,
    /// 相方側（kero）で `multiple` と宣言されたカテゴリ名（bindopt 1.1）。
    pub kero_multiple: BTreeSet<String>,
}

/// bind 名前解決スナップショット（所有・純関数・[`SurfaceResolver`] 同型）。
///
/// 本体側（sakura）・相方側（kero）を別々の所有スナップショット
/// （`BTreeMap<(カテゴリ, パーツ), 着せ替え ID>`）として保持し、`(カテゴリ名, パーツ名)` から
/// 着せ替え ID を引く。実行時 `EmoWorld` や parsers に依存せず、構築時に app 層（task 7.1）が
/// `MountModel` の名前転記から素データで組む（design「Allowed Dependencies」・seriko → parsers
/// 依存を追加しない）。状態を持たず同一入力に常に同一出力（純粋）。`Send`
/// （`BTreeMap<(String,String),u32>` は `Send`）。
///
/// [`SurfaceResolver`]: crate::SurfaceResolver
#[derive(Clone, Default)]
pub struct BindResolver {
    /// 本体側（sakura）の名前表。
    sakura: ScopeNames,
    /// 相方側（kero）の名前表。emo2 では空（Risks）。
    kero: ScopeNames,
    /// 3 人目以降（`char{N}`）の名前表（N ごと・要件 14.2）。
    chars: BTreeMap<u32, ScopeNames>,
}

/// 1 つの名前空間の名前表と `bindoption*.group` 宣言（bindopt 設計 D2/D3）。
///
/// `mustselect`／`multiple` は [`BindResolver::policy`] が 3 値ポリシーを導出する唯一の材料。
/// 全集合空＝全カテゴリが正典の既定（[`BindChoicePolicy::Default`]）。
#[derive(Clone, Default)]
struct ScopeNames {
    /// `(カテゴリ, パーツ)` → 着せ替え ID。
    table: BTreeMap<(String, String), u32>,
    /// `(行の番号, カテゴリ, パーツ)` を descript.txt の行の順に（[`BindResolver::from_groups`] だけが持つ）。
    written: Vec<(usize, String, String)>,
    /// `mustselect` と宣言されたカテゴリ名。
    mustselect: BTreeSet<String>,
    /// `multiple` と宣言されたカテゴリ名。
    multiple: BTreeSet<String>,
}

/// 名前表の無い `char{N}` へ返す空の表。
static NO_NAMES: ScopeNames = ScopeNames {
    table: BTreeMap::new(),
    written: Vec::new(),
    mustselect: BTreeSet::new(),
    multiple: BTreeSet::new(),
};

impl ScopeNames {
    /// 1 つのスコープの名前宣言 `(ID, カテゴリ, パーツ, 行の番号)`（転記順）と `mustselect`／`multiple`
    /// の転記から組む。転記順に入れるので同じ `(カテゴリ, パーツ)` は後の宣言が勝つ（parsers と同じ）。
    fn read<'a>(
        names: impl IntoIterator<Item = (u32, &'a str, &'a str, usize)>,
        mustselect: Option<&Vec<String>>,
        multiple: Option<&Vec<String>>,
    ) -> Self {
        let mut out = Self {
            mustselect: mustselect.into_iter().flatten().cloned().collect(),
            multiple: multiple.into_iter().flatten().cloned().collect(),
            ..Self::default()
        };
        for (id, category, part, line) in names {
            out.table.insert((category.to_owned(), part.to_owned()), id);
            out.written
                .push((line, category.to_owned(), part.to_owned()));
        }
        out.written.sort_by_key(|(line, _, _)| *line);
        out
    }
}

impl BindResolver {
    /// 本体側・相方側の名前表（所有）と `bindoption` 宣言集合から構築する。
    ///
    /// app 層（起動時資産構築）が `MountModel.bindgroups` の名前転記から素データで組む。
    /// 二重定義せず、渡された表をそのまま所有スナップショットとして保持する。
    ///
    /// `options` は `sakura/kero.bindoption*.group,カテゴリ,オプション` 由来のスコープ別
    /// カテゴリ名集合（bindopt 設計 D2/D3）。[`BindOptionDecls::default`]（全集合空）を渡せば
    /// 全カテゴリが正典の既定（[`BindChoicePolicy::Default`]）になる。
    pub fn new(
        sakura: BTreeMap<(String, String), u32>,
        kero: BTreeMap<(String, String), u32>,
        options: BindOptionDecls,
    ) -> Self {
        Self {
            sakura: ScopeNames {
                table: sakura,
                mustselect: options.sakura_mustselect,
                multiple: options.sakura_multiple,
                ..ScopeNames::default()
            },
            kero: ScopeNames {
                table: kero,
                mustselect: options.kero_mustselect,
                multiple: options.kero_multiple,
                ..ScopeNames::default()
            },
            chars: BTreeMap::new(),
        }
    }

    /// シェルの着せ替えの読み（`sakura.*`・`kero.*`・`char{N}.*`）から 3 種類のスコープの名前表と
    /// `mustselect`／`multiple` を組む（要件 14.2・14.3）。
    pub fn from_groups(groups: &BindGroupDefaults) -> Self {
        // 名前宣言の型は parsers の外から名指せないので、欄をその場で組に開く。
        macro_rules! decls {
            ($names:expr) => {
                $names
                    .into_iter()
                    .flatten()
                    .map(|n| (n.id, n.category.as_str(), n.part.as_str(), n.line))
            };
        }
        let mut chars = BTreeMap::new();
        for n in groups
            .char_names
            .keys()
            .chain(groups.char_mustselect.keys())
            .chain(groups.char_multiple.keys())
        {
            chars.entry(*n).or_insert_with(|| {
                ScopeNames::read(
                    decls!(groups.char_names.get(n)),
                    groups.char_mustselect.get(n),
                    groups.char_multiple.get(n),
                )
            });
        }
        Self {
            sakura: ScopeNames::read(
                decls!(Some(&groups.sakura_names)),
                Some(&groups.sakura_mustselect),
                Some(&groups.sakura_multiple),
            ),
            kero: ScopeNames::read(
                decls!(Some(&groups.kero_names)),
                Some(&groups.kero_mustselect),
                Some(&groups.kero_multiple),
            ),
            chars,
        }
    }

    /// 名前空間 `ns` の表（`char{N}` の表が無ければ空）。
    fn scope(&self, ns: BindNamespace) -> &ScopeNames {
        match ns {
            BindNamespace::Sakura => &self.sakura,
            BindNamespace::Kero => &self.kero,
            BindNamespace::Char(n) => self.chars.get(&n).unwrap_or(&NO_NAMES),
        }
    }

    /// カテゴリのパーツ名を、シェルの descript.txt に書かれた順（行の順・同じパーツ名は 1 度）で返す。
    /// 無いカテゴリは空（要件 5.2・5.9）。[`BindResolver::new`] で組んだ表は行を持たないので空。
    pub fn category_parts_in_order(&self, ns: BindNamespace, category: &str) -> Vec<String> {
        let mut parts: Vec<String> = Vec::new();
        for (_line, cat, part) in &self.scope(ns).written {
            if cat == category && !parts.contains(part) {
                parts.push(part.clone());
            }
        }
        parts
    }

    /// 空リゾルバ（本体側・相方側とも空表・`bindoption` 宣言も空）。
    ///
    /// bind 名前表を供給しない既存テスト（task 6.4）・bind cue 不在経路の追随用。空表は
    /// R4.3 のシームを「同一機構」で実現する——人工的な無効化コードを書かず、引きが自然に
    /// 解決不能（`None`）へ落ちる（D7）。宣言集合も空ゆえ [`policy`] は全カテゴリで正典の既定
    /// （[`BindChoicePolicy::Default`]）を返すが、名前表が空＝[`resolve`] が常に `None` ゆえ
    /// 適用へ到達せず、挙動への実害はない（bindopt 設計 D3）。
    ///
    /// **署名は不変**——並走 spec の無干渉前提として固定する（bindopt 設計 D3）。
    ///
    /// [`policy`]: BindResolver::policy
    /// [`resolve`]: BindResolver::resolve
    pub fn empty() -> Self {
        Self::default()
    }

    /// `(カテゴリ名, パーツ名)` を名前空間 `ns` の表から引き、着せ替え ID を返す（純粋・副作用なし）。
    ///
    /// 宣言済みの組み合わせは `Some(id)`、未宣言（キーが表に無い）は `None`（捏造しない・R3.7）。
    /// 名前空間は隔離され、`Sakura` で引くのは sakura 表のみ、`Kero` は kero 表のみ。解決不能の
    /// `error!`＋skip は呼び手（actor）の責務（R3.7）。
    pub fn resolve(&self, ns: BindNamespace, category: &str, part: &str) -> Option<u32> {
        let table = &self.scope(ns).table;
        // タプルキー `(String, String)` は借用キーでの直接引きができない（`Borrow` が
        // `(&str, &str)` を導かない）ため、引きキーを組んで引く。bind cue は低頻度ゆえ許容。
        table
            .get(&(category.to_string(), part.to_string()))
            .copied()
    }

    /// カテゴリの 3 値ポリシーを名前空間 `ns` の宣言集合から導出する（純粋・副作用なし）。
    ///
    /// 導出順は `multiple` 所属 → [`BindChoicePolicy::Multiple`]、次に `mustselect` 所属 →
    /// [`BindChoicePolicy::MustSelect`]、どちらでもなければ [`BindChoicePolicy::Default`]。
    /// 併記（`mustselect+multiple`）は正典文言「multiple で複数のパーツを選択可能」に従い
    /// **multiple 優先**（bindopt 設計 D4）。未知カテゴリ（名前表に無いカテゴリを含む）も
    /// 正典の既定＝`Default`。名前空間は隔離され、`Sakura` は sakura 集合のみ、`Kero` は
    /// kero 集合のみを見る。
    pub fn policy(&self, ns: BindNamespace, category: &str) -> BindChoicePolicy {
        let names = self.scope(ns);
        if names.multiple.contains(category) {
            BindChoicePolicy::Multiple
        } else if names.mustselect.contains(category) {
            BindChoicePolicy::MustSelect
        } else {
            BindChoicePolicy::Default
        }
    }

    /// カテゴリに属する全着せ替え ID を名前空間 `ns` の名前表から集める（純粋・副作用なし・
    /// bindopt 2.1/3.1）。
    ///
    /// `(カテゴリ, パーツ)→ID` 表を走査し `key.0 == category` の ID を昇順・重複除去して返す
    /// （排他置換の「同カテゴリ他パーツ off」の対象集合）。排他置換は mustselect
    /// （[`BindChoicePolicy::MustSelect`]・bindopt 3.1）と非宣言の既定
    /// （[`BindChoicePolicy::Default`]・bindopt 2.1）の**両方**の着衣で使われるため、本アクセサは
    /// カテゴリの宣言形に依存しない（名前表のみを引く）。宣言のないカテゴリは空 `Vec`。
    /// `BTreeSet` 経由で dedup＋昇順を保証する（emo2 は約 30 宣言ゆえ全走査で足りる）。
    pub fn category_ids(&self, ns: BindNamespace, category: &str) -> Vec<u32> {
        self.scope(ns)
            .table
            .iter()
            .filter(|((cat, _part), _id)| cat == category)
            .map(|(_key, id)| *id)
            .collect::<BTreeSet<u32>>()
            .into_iter()
            .collect()
    }

    /// 着せ替え ID から名前空間 `ns` の `(カテゴリ, パーツ)` を引く（[`resolve`] の逆引き・純粋）。
    ///
    /// 同じ ID に名前が複数あれば表の順（カテゴリ・パーツの昇順）で最初のもの。未宣言は `None`。
    /// ゴーストの読み直しで着せ替えを読み直したシェルの名前で出し直すのに使う（areka-P0-mcp-reload 要件 3.8）。
    ///
    /// [`resolve`]: BindResolver::resolve
    pub fn name_of(&self, ns: BindNamespace, id: u32) -> Option<(&str, &str)> {
        self.scope(ns)
            .table
            .iter()
            .find(|(_key, v)| **v == id)
            .map(|((cat, part), _id)| (cat.as_str(), part.as_str()))
    }
}

/// scope（`ActorKey`）→ bind 名前空間の写像（D7・純関数）。
///
/// `"0"` → [`Some(BindNamespace::Sakura)`]、`"1"` → [`Some(BindNamespace::Kero)`]、2 以上の数 →
/// `Some(BindNamespace::Char(n))`（要件 14.2）、それ以外（非数値）→ `None`（写像なし＝判定なし）。
/// 写像なしは呼び手で自然に解決不能へ落ちる。
///
/// [`Some(BindNamespace::Sakura)`]: BindNamespace::Sakura
/// [`Some(BindNamespace::Kero)`]: BindNamespace::Kero
pub fn scope_namespace(scope: &ActorKey) -> Option<BindNamespace> {
    match scope.as_str() {
        "0" => Some(BindNamespace::Sakura),
        "1" => Some(BindNamespace::Kero),
        other => other
            .parse::<u32>()
            .ok()
            .filter(|n| *n >= 2)
            .map(BindNamespace::Char),
    }
}

/// `\![bind,...]` トークン列の M1 類別（D8・純関数・不透明保持）。
///
/// [`parse_bind_directive`] が返す区分。本タスクは**類別のみ**を担い、各区分の
/// severity（実導出／`warn!`＋skip／`error!`＋skip）は付けない——それはアクター
/// （task 6.1）の責務（D8）。derive した `PartialEq`/`Eq` はテストの区分照合用。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BindDirective {
    /// 名前キー＋明示 on/off（`on` 欄が `"1"`/`"0"`）。実導出する（R4.1）。
    Apply {
        /// bindgroup カテゴリ名（trim 済み）。
        category: String,
        /// bindgroup パーツ名（trim 済み）。
        part: String,
        /// `"1"`→`true`・`"0"`→`false`。
        on: bool,
    },
    /// 数値欄が空欄／省略のトグル形。M1 では実導出せず安全縮退（`warn!`＋skip・R4.2）。
    Toggle {
        /// bindgroup カテゴリ名（trim 済み）。
        category: String,
        /// bindgroup パーツ名（trim 済み）。
        part: String,
    },
    /// パーツ名が空欄のカテゴリ単位形。M1 では実導出せず安全縮退（`warn!`＋skip・R4.2）。
    ///
    /// `on` はカテゴリ単位では厳密に action されない M1 縮退ゆえ寛容に保持する
    /// （`"1"`→`Some(true)`・`"0"`→`Some(false)`・欠落/空/その他→`None`）。不正値でも
    /// `Malformed` にはしない（縮退形の中での不透明保持）。
    CategoryWide {
        /// bindgroup カテゴリ名（trim 済み）。
        category: String,
        /// on/off の緩やかな保持（`None` = 未指定または非 `"1"`/`"0"`）。
        on: Option<bool>,
    },
    /// カテゴリ欠落／on/off 値破損（非 `"1"`/`"0"` の非空文字列）。`error!`＋skip（D8）。
    Malformed,
}

/// `\![bind,...]` のトークン列（カテゴリ, パーツ, 値, …）を M1 区分へ類別する純関数。
///
/// 各トークンは trim してから判定し、返る文字列にも trim 済みの値を格納する（parsers D2
/// の trim 規律に整合）。トークンは「不在」または「trim 後に空」のとき空とみなす。分岐は
/// design（`parse_bind_directive` 分岐・行 ~326）の順に厳密に従う:
///
/// 1. カテゴリ欠落/空 → [`BindDirective::Malformed`]。
/// 2. パーツ欠落/空 → [`BindDirective::CategoryWide`]（`on` は `"1"`/`"0"` のみ写し他は `None`）。
/// 3. 値 `"1"` → [`BindDirective::Apply`]`{ on: true }`。
/// 4. 値 `"0"` → [`BindDirective::Apply`]`{ on: false }`。
/// 5. 値 欠落/空 → [`BindDirective::Toggle`]。
/// 6. 値が非空の非 `"1"`/`"0"` 文字列 → [`BindDirective::Malformed`]（on/off 値破損）。
///
/// 第 4 トークン以降は無視する（正典に第 4 引数なし・不透明）。副作用なし・純粋。
/// 各区分の severity（実導出／`warn!`／`error!`）は付けない——アクター（task 6.1）が写す（D8）。
pub fn parse_bind_directive(tokens: &[&str]) -> BindDirective {
    // トークンを trim して取り出す。不在または trim 後空を「空」とみなす（`None` へ畳む）。
    let trimmed =
        |i: usize| -> Option<&str> { tokens.get(i).map(|t| t.trim()).filter(|t| !t.is_empty()) };
    let category = trimmed(0);
    let part = trimmed(1);
    let value = trimmed(2);

    // (1) カテゴリ欠落/空 → Malformed。
    let category = match category {
        Some(c) => c,
        None => return BindDirective::Malformed,
    };

    // (2) パーツ欠落/空 → CategoryWide（on は "1"/"0" のみ写し、他は None・寛容）。
    let part = match part {
        Some(p) => p,
        None => {
            let on = match value {
                Some("1") => Some(true),
                Some("0") => Some(false),
                _ => None,
            };
            return BindDirective::CategoryWide {
                category: category.to_string(),
                on,
            };
        }
    };

    // (3)-(6) 値の類別。
    match value {
        // (3)(4) 明示 on/off → Apply。
        Some("1") => BindDirective::Apply {
            category: category.to_string(),
            part: part.to_string(),
            on: true,
        },
        Some("0") => BindDirective::Apply {
            category: category.to_string(),
            part: part.to_string(),
            on: false,
        },
        // (5) 値欠落/空 → Toggle。
        None => BindDirective::Toggle {
            category: category.to_string(),
            part: part.to_string(),
        },
        // (6) 非空の非 "1"/"0" → on/off 値破損 → Malformed。
        Some(_) => BindDirective::Malformed,
    }
}

/// on/off 積算（純関数・[`BindSet`] は昇順 dedup ゆえ集合同値が冪等判定・D9・design 行 322）。
///
/// 現在の着せ替え集合 `current` に対し、対象 animation ID `id` を `on` に従って積算した
/// 新しい [`BindSet`] を返す。状態を持たず、同一入力に常に同一出力（純粋・副作用/ログ/panic
/// なし・R3.3/R3.4）。
///
/// - `on == true`  → `current` の全 ID に `id` を加えた集合。`from_ids` が dedup するため、
///   既に含む ID を重ねても集合として不変（冪等・D9）。
/// - `on == false` → `current` から `id` を除いた集合。含まない ID を除いても集合として不変。
///
/// 直前状態は保持したまま当該 ID のみ更新する（複数 cue 積算・R3.4）。結果は昇順 dedup 済みの
/// 正準形ゆえ、順序違い・重複適用でも集合同値に至る（D9 冪等ガードの単位＝結果 BindSet の同値）。
///
/// [`BindSet`]: areka_emo_compose::BindSet
pub fn accumulate(
    current: &areka_emo_compose::BindSet,
    id: u32,
    on: bool,
) -> areka_emo_compose::BindSet {
    if on {
        // 既存 ID に id を連結。from_ids が整列＋dedup するため重複適用は集合として no-op。
        areka_emo_compose::BindSet::from_ids(
            current.ids().iter().copied().chain(std::iter::once(id)),
        )
    } else {
        // id を除いた ID 群から再構築。含まない id の除去は集合として no-op。
        areka_emo_compose::BindSet::from_ids(current.ids().iter().copied().filter(|&x| x != id))
    }
}

#[cfg(test)]
#[path = "bind_tests.rs"]
mod tests;
