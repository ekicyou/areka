//! bind（着せ替え）構築モジュール。
//!
//! bindgroup の既定オン集合（番号）から、合成入力が要求する静的 bind 集合
//! （[`areka_emo_compose::BindSet`]）をアクター構築時に一度だけ組み立てる。
//!
//! bindgroup 番号は animation ID の**恒等写像**。中間変換・添字付け・
//! 特別扱いは一切行わず、番号をそのまま [`BindSet::from_ids`] へ流す。整列・重複除去は
//! `from_ids` が担う（`BindSet` は昇順・dedup 済み）。

/// bindgroup default 番号集合 → 静的 [`BindSet`]（恒等写像・`from_ids` が整列/dedup）。
///
/// bindgroup 番号 = animation ID。呼び手（アクター構築時）が渡した
/// 既定オン集合をそのまま `BindSet::from_ids` へ渡すのみ。状態を持たず、構築時に一度だけ
/// 呼ばれる純関数。
///
/// [`BindSet`]: areka_emo_compose::BindSet
/// [`BindSet::from_ids`]: areka_emo_compose::BindSet::from_ids
pub fn build_static_bindset(default_on: &[u32]) -> areka_emo_compose::BindSet {
    areka_emo_compose::BindSet::from_ids(default_on.iter().copied())
}

use std::collections::{BTreeMap, BTreeSet};

use areka_parsers::package::{BindGroupDefaults, ScopeBinds};
use areka_sakura::ActorKey;

/// スコープの番号ごとの既定の着せ替えの組。
///
/// [`DefaultBinds::uniform`] はどのスコープにも同じ組を返す（1 組を全スコープで共有する）。
/// [`DefaultBinds::single`] は 1 つのスコープだけに組を持つ。[`DefaultBinds::for_scope`]
/// は書かれていないスコープ・数でないスコープに空の組（`uniform` なら共有の組）を返す。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DefaultBinds {
    /// スコープの番号ごとの組。
    by_scope: BTreeMap<u32, areka_emo_compose::BindSet>,
    /// `by_scope` に無いスコープへ返す組（`uniform` だけが空でない組を持つ）。
    fallback: areka_emo_compose::BindSet,
}

impl DefaultBinds {
    /// どのスコープにも同じ組。
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
    /// `default,1` を当てる。書かれていないスコープは空の組。
    pub fn from_groups(groups: &BindGroupDefaults) -> Self {
        Self {
            by_scope: groups
                .iter()
                .map(|(n, binds)| (*n, build_static_bindset(&binds.default_on)))
                .collect(),
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

/// bind 名前解決の**名前空間**（本体側＝sakura／相方側＝kero を型で峻別する）。
///
/// scope（`ActorKey`）から [`scope_namespace`] が写像し、[`BindResolver::resolve`] が
/// どの名前表を引くかを選ぶ。3 人目以降（`char{N}.bindgroup`）は番号つきの [`BindNamespace::Char`]。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindNamespace {
    /// 本体側（scope `"0"`・shell の `sakura.bindgroup`）。
    Sakura,
    /// 相方側（scope `"1"`・shell の `kero.bindgroup`）。名前表まで持つが機構は scope 非依存。
    Kero,
    /// 3 人目以降（scope `"N"`・N は 2 以上・shell の `char{N}.bindgroup`）。
    Char(u32),
}

/// カテゴリの着せ替え選択ポリシー（ukadoc 正典の 3 値）。
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

/// bind 名前解決スナップショット（所有・純関数・[`SurfaceResolver`] 同型）。
///
/// 本体側（sakura）・相方側（kero）を別々の所有スナップショット
/// （`BTreeMap<(カテゴリ, パーツ), 着せ替え ID>`）として保持し、`(カテゴリ名, パーツ名)` から
/// 着せ替え ID を引く。実行時 `EmoWorld` に依存しない。
/// 状態を持たず同一入力に常に同一出力（純粋）。`Send`
/// （`BTreeMap<(String,String),u32>` は `Send`）。
///
/// [`SurfaceResolver`]: crate::SurfaceResolver
#[derive(Clone, Default)]
pub struct BindResolver {
    /// 本体側（sakura）の名前表。
    sakura: ScopeNames,
    /// 相方側（kero）の名前表。emo2 では空。
    kero: ScopeNames,
    /// 3 人目以降（`char{N}`）の名前表（N ごと）。
    chars: BTreeMap<u32, ScopeNames>,
}

/// 1 つの名前空間の名前表と `bindoption*.group` 宣言。
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
    /// 1 つのスコープの名前宣言（転記順）と `mustselect`／`multiple` の転記から組む。
    /// 転記順に入れるので同じ `(カテゴリ, パーツ)` は後の宣言が勝つ。
    fn read(binds: &ScopeBinds) -> Self {
        let mut out = Self {
            mustselect: binds.mustselect.iter().cloned().collect(),
            multiple: binds.multiple.iter().cloned().collect(),
            ..Self::default()
        };
        for n in &binds.names {
            out.table.insert((n.category.clone(), n.part.clone()), n.id);
            out.written
                .push((n.line, n.category.clone(), n.part.clone()));
        }
        out.written.sort_by_key(|(line, _, _)| *line);
        out
    }
}

impl BindResolver {
    /// シェルの着せ替えの読み（`sakura.*`・`kero.*`・`char{N}.*`）から 3 種類のスコープの名前表と
    /// `mustselect`／`multiple` を組む。
    pub fn from_groups(groups: &BindGroupDefaults) -> Self {
        let mut out = Self::default();
        for (n, binds) in groups {
            let names = ScopeNames::read(binds);
            match n {
                0 => out.sakura = names,
                1 => out.kero = names,
                _ => {
                    out.chars.insert(*n, names);
                }
            }
        }
        out
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
    /// 無いカテゴリは空。
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
    /// bind 名前表を供給しない経路（テスト・bind cue 不在経路）用。空表なので引きは自然に
    /// 解決不能（`None`）へ落ち、人工的な無効化コードは要らない。宣言集合も空ゆえ [`policy`] は
    /// 全カテゴリで正典の既定（[`BindChoicePolicy::Default`]）を返すが、[`resolve`] が常に `None`
    /// なので適用へ到達せず、挙動への実害はない。
    ///
    /// [`policy`]: BindResolver::policy
    /// [`resolve`]: BindResolver::resolve
    pub fn empty() -> Self {
        Self::default()
    }

    /// `(カテゴリ名, パーツ名)` を名前空間 `ns` の表から引き、着せ替え ID を返す（純粋・副作用なし）。
    ///
    /// 宣言済みの組み合わせは `Some(id)`、未宣言（キーが表に無い）は `None`（捏造しない）。
    /// 名前空間は隔離され、`Sakura` で引くのは sakura 表のみ、`Kero` は kero 表のみ。解決不能の
    /// `error!`＋skip は呼び手（actor）の責務。
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
    /// **multiple 優先**。未知カテゴリ（名前表に無いカテゴリを含む）も
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

    /// カテゴリに属する全着せ替え ID を名前空間 `ns` の名前表から集める（純粋・副作用なし）。
    ///
    /// `(カテゴリ, パーツ)→ID` 表を走査し `key.0 == category` の ID を昇順・重複除去して返す
    /// （排他置換の「同カテゴリ他パーツ off」の対象集合）。排他置換は mustselect
    /// （[`BindChoicePolicy::MustSelect`]）と非宣言の既定
    /// （[`BindChoicePolicy::Default`]）の**両方**の着衣で使われるため、本アクセサは
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
    /// ゴーストの読み直しで着せ替えを読み直したシェルの名前で出し直すのに使う。
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

/// scope（`ActorKey`）→ bind 名前空間の写像（純関数）。
///
/// `"0"` → [`Some(BindNamespace::Sakura)`]、`"1"` → [`Some(BindNamespace::Kero)`]、2 以上の数 →
/// `Some(BindNamespace::Char(n))`、それ以外（非数値）→ `None`（写像なし＝判定なし）。
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

/// `\![bind,...]` トークン列の類別（純関数・不透明保持）。
///
/// [`parse_bind_directive`] が返す区分。区分を当てるのは `apply_bind_directive`（Toggle は今の
/// 着脱の反対・カテゴリ単位の形は定義の順に 1 つずつ）で、`Malformed` の `error!`＋skip は
/// アクター（`actor_bind`）が出す。derive した `PartialEq`/`Eq` はテストの区分照合用。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum BindDirective {
    /// 名前キー＋明示 on/off（`on` 欄が `"1"`/`"0"`）。実導出する。
    Apply {
        /// bindgroup カテゴリ名（trim 済み）。
        category: String,
        /// bindgroup パーツ名（trim 済み）。
        part: String,
        /// `"1"`→`true`・`"0"`→`false`。
        on: bool,
    },
    /// 数値欄が空欄／省略のトグル形。今の着脱の反対を当てる（`apply_bind_directive`）。
    Toggle {
        /// bindgroup カテゴリ名（trim 済み）。
        category: String,
        /// bindgroup パーツ名（trim 済み）。
        part: String,
    },
    /// パーツ名が空欄のカテゴリ単位形。定義の順にパーツを 1 つずつ当てる（`apply_bind_directive`）。
    ///
    /// `on` は `"1"`→`Some(true)`・`"0"`→`Some(false)`・欠落/空/その他→`None`（`None` はパーツごとのトグル）。
    /// 不正値でも `Malformed` にはしない。
    CategoryWide {
        /// bindgroup カテゴリ名（trim 済み）。
        category: String,
        /// on/off の緩やかな保持（`None` = 未指定または非 `"1"`/`"0"`）。
        on: Option<bool>,
    },
    /// カテゴリ欠落／on/off 値破損（非 `"1"`/`"0"` の非空文字列）。`error!`＋skip。
    Malformed,
}

/// `\![bind,...]` のトークン列（カテゴリ, パーツ, 値, …）を区分へ類別する純関数。
///
/// 各トークンは trim してから判定し、返る文字列にも trim 済みの値を格納する。
/// トークンは「不在」または「trim 後に空」のとき空とみなす。分岐は次の順に厳密に従う:
///
/// 1. カテゴリ欠落/空 → [`BindDirective::Malformed`]。
/// 2. パーツ欠落/空 → [`BindDirective::CategoryWide`]（`on` は `"1"`/`"0"` のみ写し他は `None`）。
/// 3. 値 `"1"` → [`BindDirective::Apply`]`{ on: true }`。
/// 4. 値 `"0"` → [`BindDirective::Apply`]`{ on: false }`。
/// 5. 値 欠落/空 → [`BindDirective::Toggle`]。
/// 6. 値が非空の非 `"1"`/`"0"` 文字列 → [`BindDirective::Malformed`]（on/off 値破損）。
///
/// 第 4 トークン以降は無視する（正典に第 4 引数なし・不透明）。副作用なし・純粋。
/// 区分を当てるのは `apply_bind_directive`、`Malformed` の `error!` はアクターが出す。
pub(crate) fn parse_bind_directive(tokens: &[&str]) -> BindDirective {
    let trimmed =
        |i: usize| -> Option<&str> { tokens.get(i).map(|t| t.trim()).filter(|t| !t.is_empty()) };
    let category = trimmed(0);
    let part = trimmed(1);
    let value = trimmed(2);

    let category = match category {
        Some(c) => c,
        None => return BindDirective::Malformed,
    };

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

    match value {
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
        None => BindDirective::Toggle {
            category: category.to_string(),
            part: part.to_string(),
        },
        Some(_) => BindDirective::Malformed,
    }
}

/// [`apply_named_bind`] の結果（記録は呼び手が出す）。
#[derive(Clone, Debug, PartialEq)]
pub enum NamedBindResult {
    /// 名前を解決して当てた（`id` は解決した着せ替え ID）。
    Applied {
        id: u32,
        outcome: crate::state::BindApplyOutcome,
    },
    /// scope に名前の空間が無い（数でない scope）。
    NoNamespace,
    /// (カテゴリ, パーツ) が宣言に無い。
    NotFound,
    /// mustselect のカテゴリの脱衣は外さない（正典・解除不可）。
    MustSelectKept { id: u32 },
}

/// 名前で 1 パーツを当てる決め方（名前の解決 → 方針 → 当てる）。画面の `\![bind]` と
/// `--dump-bind` が共有する。複数可でないカテゴリの着衣は排他置換、
/// 複数可の着衣は加算・mustselect 以外の脱衣は除去、mustselect の脱衣は外さない。
pub fn apply_named_bind(
    states: &mut crate::state::ScopeStates,
    resolver: &BindResolver,
    scope: &ActorKey,
    category: &str,
    part: &str,
    on: bool,
) -> NamedBindResult {
    let Some(ns) = scope_namespace(scope) else {
        return NamedBindResult::NoNamespace;
    };
    let Some(id) = resolver.resolve(ns, category, part) else {
        return NamedBindResult::NotFound;
    };
    let policy = resolver.policy(ns, category);
    if !on && policy == BindChoicePolicy::MustSelect {
        return NamedBindResult::MustSelectKept { id };
    }
    let outcome = if on && policy != BindChoicePolicy::Multiple {
        states.apply_bind_exclusive(scope, &resolver.category_ids(ns, category), id)
    } else {
        states.apply_bind(scope, id, on)
    };
    NamedBindResult::Applied { id, outcome }
}

/// パーツ 1 つを当てた結果と、当てたパーツ・着脱（記録の欄を呼び手が埋めるため）。
#[derive(Clone, Debug, PartialEq)]
pub struct AppliedBind {
    pub category: String,
    /// 当てた 1 つのパーツの名前（カテゴリ単位形でパーツが無いときは空）。
    pub part: String,
    /// 実際に当てた着脱（トグル形は反対にした後の値）。
    pub on: bool,
    pub result: NamedBindResult,
}

/// 読んだ着せ替えの指令を、パーツ 1 つずつ [`apply_named_bind`] で当てる。返す列は当てた順。
///
/// SSP の測りに合わせ、トグル形は今の着脱の反対を 1 回、カテゴリ単位形は定義の順
/// （[`BindResolver::category_parts_in_order`]）に `0`／`1`／その時点の反対を 1 つずつ当てる。
/// カテゴリにパーツが無ければ `NotFound`、スコープに名前の空間が無ければ `NoNamespace` を 1 つ返す
/// （空の列を返さない）。`Malformed` は呼び手が先に除く前提で、渡されたら空の列（状態を変えない）。
pub fn apply_bind_directive(
    states: &mut crate::state::ScopeStates,
    resolver: &BindResolver,
    scope: &ActorKey,
    directive: &BindDirective,
) -> Vec<AppliedBind> {
    // `on` が `None` ならトグル（今着ていれば脱ぐ・着ていない／解けない名前なら着る）。
    let apply_one =
        |states: &mut crate::state::ScopeStates, category: &str, part: &str, on: Option<bool>| {
            let on = on.unwrap_or_else(|| {
                scope_namespace(scope)
                    .and_then(|ns| resolver.resolve(ns, category, part))
                    .is_none_or(|id| !states.current_binds(scope).contains(id))
            });
            AppliedBind {
                category: category.to_owned(),
                part: part.to_owned(),
                on,
                result: apply_named_bind(states, resolver, scope, category, part, on),
            }
        };
    match directive {
        BindDirective::Apply { category, part, on } => {
            vec![apply_one(states, category, part, Some(*on))]
        }
        BindDirective::Toggle { category, part } => vec![apply_one(states, category, part, None)],
        BindDirective::CategoryWide { category, on } => {
            let ns = scope_namespace(scope);
            let parts = ns
                .map(|ns| resolver.category_parts_in_order(ns, category))
                .unwrap_or_default();
            if parts.is_empty() {
                return vec![AppliedBind {
                    category: category.clone(),
                    part: String::new(),
                    on: on.unwrap_or(true),
                    result: if ns.is_some() {
                        NamedBindResult::NotFound
                    } else {
                        NamedBindResult::NoNamespace
                    },
                }];
            }
            parts
                .iter()
                .map(|part| apply_one(states, category, part, *on))
                .collect()
        }
        BindDirective::Malformed => Vec::new(),
    }
}

/// on/off 積算（純関数・[`BindSet`] は昇順 dedup ゆえ集合同値が冪等判定）。
///
/// 現在の着せ替え集合 `current` に対し、対象 animation ID `id` を `on` に従って積算した
/// 新しい [`BindSet`] を返す。状態を持たず、同一入力に常に同一出力（純粋・副作用/ログ/panic
/// なし）。
///
/// - `on == true`  → `current` の全 ID に `id` を加えた集合。`from_ids` が dedup するため、
///   既に含む ID を重ねても集合として不変（冪等）。
/// - `on == false` → `current` から `id` を除いた集合。含まない ID を除いても集合として不変。
///
/// 直前状態は保持したまま当該 ID のみ更新する（複数 cue 積算）。結果は昇順 dedup 済みの
/// 正準形ゆえ、順序違い・重複適用でも集合同値に至る。
///
/// [`BindSet`]: areka_emo_compose::BindSet
pub(crate) fn accumulate(
    current: &areka_emo_compose::BindSet,
    id: u32,
    on: bool,
) -> areka_emo_compose::BindSet {
    if on {
        areka_emo_compose::BindSet::from_ids(
            current.ids().iter().copied().chain(std::iter::once(id)),
        )
    } else {
        areka_emo_compose::BindSet::from_ids(current.ids().iter().copied().filter(|&x| x != id))
    }
}

#[cfg(test)]
#[path = "bind_tests.rs"]
pub(crate) mod tests;

#[cfg(test)]
#[path = "bind_named_tests.rs"]
mod named_tests;

#[cfg(test)]
#[path = "bind_forms_tests.rs"]
mod forms_tests;
