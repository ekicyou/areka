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

use std::collections::BTreeMap;

use areka_sakura::ActorKey;

/// bind 名前解決の**名前空間**（本体側＝sakura／相方側＝kero を型で峻別する・D7）。
///
/// scope（`ActorKey`）から [`scope_namespace`] が写像し、[`BindResolver::resolve`] が
/// どちらの名前表を引くかを選ぶ。M1 では `char2+`（`char*.bindgroup`）の名前空間は持たない。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindNamespace {
    /// 本体側（scope `"0"`・shell の `sakura.bindgroup`）。
    Sakura,
    /// 相方側（scope `"1"`・shell の `kero.bindgroup`）。M1 では名前表まで持つが機構は scope 非依存。
    Kero,
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
pub struct BindResolver {
    /// 本体側（sakura）: `(カテゴリ, パーツ)` → 着せ替え ID。
    sakura: BTreeMap<(String, String), u32>,
    /// 相方側（kero）: `(カテゴリ, パーツ)` → 着せ替え ID。emo2 では空（Risks）。
    kero: BTreeMap<(String, String), u32>,
}

impl BindResolver {
    /// 本体側・相方側の名前表（所有）から構築する。
    ///
    /// app 層（task 7.1）が `MountModel.bindgroups` の名前転記から素データで組む。
    /// 二重定義せず、渡された表をそのまま所有スナップショットとして保持する。
    pub fn new(
        sakura: BTreeMap<(String, String), u32>,
        kero: BTreeMap<(String, String), u32>,
    ) -> Self {
        Self { sakura, kero }
    }

    /// 空リゾルバ（本体側・相方側とも空表）。
    ///
    /// bind 名前表を供給しない既存テスト（task 6.4）・bind cue 不在経路の追随用。空表は
    /// R4.3 のシームを「同一機構」で実現する——人工的な無効化コードを書かず、引きが自然に
    /// 解決不能（`None`）へ落ちる（D7）。
    pub fn empty() -> Self {
        Self {
            sakura: BTreeMap::new(),
            kero: BTreeMap::new(),
        }
    }

    /// `(カテゴリ名, パーツ名)` を名前空間 `ns` の表から引き、着せ替え ID を返す（純粋・副作用なし）。
    ///
    /// 宣言済みの組み合わせは `Some(id)`、未宣言（キーが表に無い）は `None`（捏造しない・R3.7）。
    /// 名前空間は隔離され、`Sakura` で引くのは sakura 表のみ、`Kero` は kero 表のみ。解決不能の
    /// `error!`＋skip は呼び手（actor）の責務（R3.7）。
    pub fn resolve(&self, ns: BindNamespace, category: &str, part: &str) -> Option<u32> {
        let table = match ns {
            BindNamespace::Sakura => &self.sakura,
            BindNamespace::Kero => &self.kero,
        };
        // タプルキー `(String, String)` は借用キーでの直接引きができない（`Borrow` が
        // `(&str, &str)` を導かない）ため、引きキーを組んで引く。bind cue は低頻度ゆえ許容。
        table
            .get(&(category.to_string(), part.to_string()))
            .copied()
    }
}

/// scope（`ActorKey`）→ bind 名前空間の写像（D7・純関数）。
///
/// `"0"` → [`Some(BindNamespace::Sakura)`]、`"1"` → [`Some(BindNamespace::Kero)`]、
/// それ以外（`"2"` 以降・非数値）→ `None`（写像なし＝判定なし・該当しないスコープ）。一般の
/// `u32` parse ではなく**明示的な `"0"`/`"1"`**のみを写す（`char2+` の bindgroup は M1 未取込
/// ゆえ写像なしが正直。M-dual が写像表を拡張する）。写像なしは呼び手で自然に解決不能へ落ちる。
///
/// [`Some(BindNamespace::Sakura)`]: BindNamespace::Sakura
/// [`Some(BindNamespace::Kero)`]: BindNamespace::Kero
pub fn scope_namespace(scope: &ActorKey) -> Option<BindNamespace> {
    match scope.as_str() {
        "0" => Some(BindNamespace::Sakura),
        "1" => Some(BindNamespace::Kero),
        _ => None,
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
    let trimmed = |i: usize| -> Option<&str> {
        tokens.get(i).map(|t| t.trim()).filter(|t| !t.is_empty())
    };
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
        areka_emo_compose::BindSet::from_ids(current.ids().iter().copied().chain(std::iter::once(id)))
    } else {
        // id を除いた ID 群から再構築。含まない id の除去は集合として no-op。
        areka_emo_compose::BindSet::from_ids(current.ids().iter().copied().filter(|&x| x != id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 恒等写像（KEY・emo2 追験値）: 既定オン集合の番号がそのまま bind 集合の
    /// animation ID になる。emo2 実測の `default,1` 集合 `{1100,1207,1302,1500,1800}`
    /// （既に昇順）を渡し、`ids()` が同一スライスを返すことを全値比較で確認する。
    #[test]
    fn identity_mapping_emo2_default_on() {
        let set = build_static_bindset(&[1100, 1207, 1302, 1500, 1800]);
        assert_eq!(
            set.ids(),
            &[1100, 1207, 1302, 1500, 1800],
            "bindgroup 番号は animation ID の恒等写像（決定3 / R-2）: 変換なし"
        );
    }

    /// 整列＋dedup パススルー: 未整列・重複入力を渡すと、`from_ids` により
    /// 昇順・重複除去された集合へ正規化される（発明された変換ではなく忠実な passthrough）。
    #[test]
    fn sort_and_dedup_passthrough() {
        let set = build_static_bindset(&[1800, 1100, 1100, 1207]);
        assert_eq!(
            set.ids(),
            &[1100, 1207, 1800],
            "未整列・重複入力は from_ids がそのまま整列/dedup する（集合の恒等・独自変換なし）"
        );
    }

    /// 空入力: kero 側は emo2 で空（design Risks）。空スライスから空 BindSet を得る。
    #[test]
    fn empty_input_yields_empty_bindset() {
        let set = build_static_bindset(&[]);
        assert_eq!(set.ids(), &[] as &[u32], "空の既定オン集合は空の bind 集合");
        assert_eq!(
            set,
            areka_emo_compose::BindSet::default(),
            "空入力は既定（空）BindSet と等価"
        );
    }

    // ─────────────────────────────────────────────────────────────────────
    // Task 3.1: bind 名前解決（BindResolver）とスコープ写像（scope_namespace）の純関数群。
    // ─────────────────────────────────────────────────────────────────────

    use areka_sakura::ActorKey;
    use std::collections::BTreeMap;

    /// sakura/kero 各 1 件の宣言を持つ小さな名前解決表を組む。
    /// sakura: (腕, 上げ)→1100 / kero: (脚, 組む)→2100。
    fn tiny_resolver() -> BindResolver {
        let mut sakura: BTreeMap<(String, String), u32> = BTreeMap::new();
        sakura.insert(("腕".into(), "上げ".into()), 1100);
        let mut kero: BTreeMap<(String, String), u32> = BTreeMap::new();
        kero.insert(("脚".into(), "組む".into()), 2100);
        BindResolver::new(sakura, kero)
    }

    /// 宣言済みの (カテゴリ, パーツ) は名前空間ごとに着せ替え ID を返す（R3.2）。
    #[test]
    fn resolve_declared_returns_id_per_namespace() {
        let r = tiny_resolver();
        assert_eq!(
            r.resolve(BindNamespace::Sakura, "腕", "上げ"),
            Some(1100),
            "sakura 名前空間で宣言済みの組み合わせは着せ替え ID を得る（R3.2）"
        );
        assert_eq!(
            r.resolve(BindNamespace::Kero, "脚", "組む"),
            Some(2100),
            "kero 名前空間で宣言済みの組み合わせは着せ替え ID を得る（R4.3・空表でない場合）"
        );
    }

    /// 名前空間は隔離される: sakura 宣言のキーを kero で引くと未宣言＝None（R3.7）。
    #[test]
    fn resolve_is_namespace_isolated() {
        let r = tiny_resolver();
        assert_eq!(
            r.resolve(BindNamespace::Kero, "腕", "上げ"),
            None,
            "sakura に宣言されたキーを kero 名前空間で引いても解決しない（名前空間隔離）"
        );
        assert_eq!(
            r.resolve(BindNamespace::Sakura, "脚", "組む"),
            None,
            "kero に宣言されたキーを sakura 名前空間で引いても解決しない（名前空間隔離）"
        );
    }

    /// 未宣言の (カテゴリ, パーツ) は None（捏造しない・R3.7）。
    #[test]
    fn resolve_unknown_returns_none() {
        let r = tiny_resolver();
        assert_eq!(
            r.resolve(BindNamespace::Sakura, "腕", "下げ"),
            None,
            "宣言されていないパーツは解決不能（None・捏造しない・R3.7）"
        );
        assert_eq!(
            r.resolve(BindNamespace::Sakura, "髪", "上げ"),
            None,
            "宣言されていないカテゴリは解決不能（None・R3.7）"
        );
    }

    /// 空リゾルバ（両表とも空）はすべて None（panic せず・R4.3 の空表同一機構）。
    #[test]
    fn empty_resolver_resolves_all_none() {
        let r = BindResolver::empty();
        assert_eq!(
            r.resolve(BindNamespace::Sakura, "腕", "上げ"),
            None,
            "空リゾルバは sakura でも解決しない（R4.3・空表＝自然な解決不能）"
        );
        assert_eq!(
            r.resolve(BindNamespace::Kero, "脚", "組む"),
            None,
            "空リゾルバは kero でも解決しない（emo2 に kero bindgroup 無し・R4.3）"
        );
    }

    /// scope 写像（D7）: "0"→Sakura・"1"→Kero・その他→None（写像なし＝判定なし）。
    #[test]
    fn scope_namespace_maps_zero_one_only() {
        assert_eq!(
            scope_namespace(&ActorKey::from("0")),
            Some(BindNamespace::Sakura),
            "scope \"0\" は sakura 名前空間へ写像する（D7）"
        );
        assert_eq!(
            scope_namespace(&ActorKey::from("1")),
            Some(BindNamespace::Kero),
            "scope \"1\" は kero 名前空間へ写像する（D7）"
        );
        assert_eq!(
            scope_namespace(&ActorKey::from("2")),
            None,
            "scope \"2\" 以降は写像なし＝判定なし（char2+ は M1 未取込・D7）"
        );
        assert_eq!(
            scope_namespace(&ActorKey::from("balloon0")),
            None,
            "非数値 scope も写像なし（明示 \"0\"/\"1\" のみ・一般 parse でない・D7）"
        );
        assert_eq!(
            scope_namespace(&ActorKey::from("x")),
            None,
            "その他の scope 識別子は判定なし（None・D7）"
        );
    }

    // ─────────────────────────────────────────────────────────────────────
    // Task 3.2: bind コマンド引数の類別（parse_bind_directive）純関数。
    // Observable = 正常形（Apply）/トグル形（Toggle）/カテゴリ単位形（CategoryWide）/
    // 破損形（Malformed）それぞれが期待どおり区分される（D8・R4.1/R4.2）。
    // ─────────────────────────────────────────────────────────────────────

    /// 正常形（R4.1）: 名前キー＋明示 on/off。"1"→on:true・"0"→on:false の Apply。
    #[test]
    fn parse_apply_explicit_on_off() {
        assert_eq!(
            parse_bind_directive(&["腕", "伸び", "1"]),
            BindDirective::Apply {
                category: "腕".into(),
                part: "伸び".into(),
                on: true
            },
            "値 \"1\" は Apply{{on:true}}（実導出・R4.1）"
        );
        assert_eq!(
            parse_bind_directive(&["腕", "伸び", "0"]),
            BindDirective::Apply {
                category: "腕".into(),
                part: "伸び".into(),
                on: false
            },
            "値 \"0\" は Apply{{on:false}}（実導出・R4.1）"
        );
    }

    /// トグル形（R4.2）: 数値欄が空欄／省略 → Toggle（M1 縮退・warn+skip は actor 責務）。
    #[test]
    fn parse_toggle_empty_or_absent_value() {
        assert_eq!(
            parse_bind_directive(&["腕", "伸び"]),
            BindDirective::Toggle {
                category: "腕".into(),
                part: "伸び".into()
            },
            "値トークン省略は Toggle（R4.2）"
        );
        assert_eq!(
            parse_bind_directive(&["腕", "伸び", ""]),
            BindDirective::Toggle {
                category: "腕".into(),
                part: "伸び".into()
            },
            "値トークンが空欄（trim 後空）は Toggle（R4.2）"
        );
    }

    /// カテゴリ単位形（R4.2）: パーツ名が空欄／省略 → CategoryWide。
    /// on はカテゴリ単位では厳密に action されない M1 縮退ゆえ寛容: "1"→Some(true)・
    /// "0"→Some(false)・欠落/空/その他→None（Malformed にしない）。
    #[test]
    fn parse_category_wide_part_empty_or_absent() {
        assert_eq!(
            parse_bind_directive(&["腕", "", ""]),
            BindDirective::CategoryWide {
                category: "腕".into(),
                on: None
            },
            "パーツ空欄・値空欄は CategoryWide{{on:None}}（R4.2）"
        );
        assert_eq!(
            parse_bind_directive(&["腕"]),
            BindDirective::CategoryWide {
                category: "腕".into(),
                on: None
            },
            "パーツトークン省略は CategoryWide{{on:None}}（R4.2）"
        );
        assert_eq!(
            parse_bind_directive(&["腕", "", "1"]),
            BindDirective::CategoryWide {
                category: "腕".into(),
                on: Some(true)
            },
            "パーツ空欄＋値 \"1\" は CategoryWide{{on:Some(true)}}（R4.2）"
        );
        assert_eq!(
            parse_bind_directive(&["腕", "", "0"]),
            BindDirective::CategoryWide {
                category: "腕".into(),
                on: Some(false)
            },
            "パーツ空欄＋値 \"0\" は CategoryWide{{on:Some(false)}}（R4.2）"
        );
        assert_eq!(
            parse_bind_directive(&["腕", "", "abc"]),
            BindDirective::CategoryWide {
                category: "腕".into(),
                on: None
            },
            "パーツ空欄＋不正値でも CategoryWide は寛容に on:None（Malformed にしない・R4.2）"
        );
    }

    /// 破損形: カテゴリ欠落／空欄 → Malformed（error+skip は actor 責務）。
    #[test]
    fn parse_malformed_category_missing() {
        assert_eq!(
            parse_bind_directive(&[]),
            BindDirective::Malformed,
            "全トークン欠落は Malformed（カテゴリ欠落）"
        );
        assert_eq!(
            parse_bind_directive(&["", "伸び", "1"]),
            BindDirective::Malformed,
            "カテゴリが空欄（trim 後空）は Malformed"
        );
        assert_eq!(
            parse_bind_directive(&["   ", "伸び", "1"]),
            BindDirective::Malformed,
            "カテゴリが空白のみ（trim 後空）は Malformed"
        );
    }

    /// 破損形: on/off 値の破損（非 "1"/"0" の非空文字列） → Malformed。
    #[test]
    fn parse_malformed_bad_on_off_value() {
        assert_eq!(
            parse_bind_directive(&["腕", "伸び", "2"]),
            BindDirective::Malformed,
            "値 \"2\" は on/off 値破損 → Malformed"
        );
        assert_eq!(
            parse_bind_directive(&["腕", "伸び", "abc"]),
            BindDirective::Malformed,
            "値 \"abc\" は on/off 値破損 → Malformed"
        );
    }

    /// 第 4 トークン以降は無視（正典に第 4 引数なし・不透明）。
    #[test]
    fn parse_ignores_fourth_token() {
        assert_eq!(
            parse_bind_directive(&["腕", "伸び", "1", "extra"]),
            BindDirective::Apply {
                category: "腕".into(),
                part: "伸び".into(),
                on: true
            },
            "第 4 トークン \"extra\" は無視され Apply{{on:true}} のまま"
        );
    }

    /// trim: 各トークンは trim され、返る文字列も trim 済みが格納される（parsers D2 整合）。
    #[test]
    fn parse_trims_tokens_and_stores_trimmed() {
        assert_eq!(
            parse_bind_directive(&[" 腕 ", " 伸び ", "1"]),
            BindDirective::Apply {
                category: "腕".into(),
                part: "伸び".into(),
                on: true
            },
            "前後空白は trim され trim 済み値が格納される"
        );
    }

    // ─────────────────────────────────────────────────────────────────────
    // Task 3.3: 着せ替え集合の on/off 積算（accumulate）純関数。
    // Observable = 冪等（on→on・off→off で結果集合不変）＋順序不感（適用列の順序を
    // 入れ替えても同一 BindSet）。BindSet が昇順 dedup ゆえ集合同値が冪等判定の正準形（D9）。
    // ─────────────────────────────────────────────────────────────────────

    use areka_emo_compose::BindSet;

    /// 基本（R3.3）: 空集合に on で追加→単一集合、続けて off で除去→空集合。
    #[test]
    fn accumulate_basic_on_then_off() {
        let empty = BindSet::default();
        let after_on = accumulate(&empty, 1100, true);
        assert_eq!(after_on.ids(), &[1100], "空集合に on で ID を追加（着衣）");
        let after_off = accumulate(&after_on, 1100, false);
        assert_eq!(after_off.ids(), &[] as &[u32], "on 済みを off で除去（脱衣・R3.3）");
        assert_eq!(after_off, empty, "off で元の空集合へ戻る");
    }

    /// on は直前状態を保持したまま当該 ID を追加する（複数 cue 積算・R3.4）。
    #[test]
    fn accumulate_on_preserves_existing() {
        let base = BindSet::from_ids([1100]);
        let after = accumulate(&base, 1207, true);
        assert_eq!(
            after.ids(),
            &[1100, 1207],
            "既存 1100 を保持したまま 1207 を追加（直前状態保持・R3.4）"
        );
    }

    /// off は当該 ID のみ除去し、他は保持する（R3.3/R3.4）。
    #[test]
    fn accumulate_off_removes_only_target() {
        let base = BindSet::from_ids([1100, 1207]);
        let after = accumulate(&base, 1100, false);
        assert_eq!(after.ids(), &[1207], "1100 のみ除去し 1207 は保持（R3.3）");
    }

    /// 冪等 on→on（D9）: 同一 (id, on=true) を 2 回適用しても 1 回と同一集合。
    #[test]
    fn accumulate_idempotent_on_on() {
        let base = BindSet::from_ids([1207]);
        let once = accumulate(&base, 1100, true);
        let twice = accumulate(&once, 1100, true);
        assert_eq!(
            once, twice,
            "on→on の重複適用は結果集合が同値（冪等・D9）"
        );
        assert_eq!(twice.ids(), &[1100, 1207], "重複適用でも 1100 は 1 件（dedup）");
    }

    /// 冪等 off→off（D9）: 含まない ID を off しても不変、2 回連打でも不変。
    #[test]
    fn accumulate_idempotent_off_off() {
        let base = BindSet::from_ids([1207]);
        let once = accumulate(&base, 1100, false);
        let twice = accumulate(&once, 1100, false);
        assert_eq!(once, base, "含まない ID の off は集合として no-op（不変）");
        assert_eq!(twice, base, "off→off 連打でも不変（冪等・D9）");
    }

    /// 順序不感（KEY Observable・D9）: 適用列の順序を入れ替えても同一 BindSet。
    /// [on 1207, on 1100] と [on 1100, on 1207] が同一集合へ至る（fold で畳む）。
    #[test]
    fn accumulate_order_insensitive() {
        let seq_a = [(1207u32, true), (1100u32, true)];
        let seq_b = [(1100u32, true), (1207u32, true)];
        let fold = |seq: &[(u32, bool)]| {
            seq.iter()
                .fold(BindSet::default(), |acc, &(id, on)| accumulate(&acc, id, on))
        };
        let result_a = fold(&seq_a);
        let result_b = fold(&seq_b);
        assert_eq!(
            result_a, result_b,
            "適用順序を入れ替えても同一 BindSet（順序不感・D9）"
        );
        assert_eq!(result_a.ids(), &[1100, 1207], "両順序とも昇順正準形へ収束");
    }

    /// on→off 往復（D9）: on で追加した ID を off で戻すと元集合へ復帰する。
    #[test]
    fn accumulate_on_off_round_trip() {
        let base = BindSet::from_ids([1302, 1500]);
        let after_on = accumulate(&base, 1100, true);
        let after_off = accumulate(&after_on, 1100, false);
        assert_eq!(
            after_off, base,
            "on→off 往復は元集合へ復帰（直前状態保持・D9）"
        );
    }

    /// off of absent id は no-op（入力と同値・R3.3 の縮退経路）。
    #[test]
    fn accumulate_off_absent_is_noop() {
        let base = BindSet::from_ids([1100, 1207]);
        let after = accumulate(&base, 9999, false);
        assert_eq!(after, base, "存在しない ID の off は入力集合と同値（no-op）");
    }
}
