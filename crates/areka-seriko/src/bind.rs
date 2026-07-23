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
}
