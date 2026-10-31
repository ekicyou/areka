//! `bind.rs` のテスト（bind.rs の中にあったものを純移動・areka-P0-mcp-reload 8.3 で 1,000 行を超えないように）。

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
use std::collections::{BTreeMap, BTreeSet};

/// sakura/kero 各 1 件の宣言を持つ小さな名前解決表を組む（`bindoption` 宣言なし）。
/// sakura: (腕, 上げ)→1100 / kero: (脚, 組む)→2100。
fn tiny_resolver() -> BindResolver {
    let mut sakura: BTreeMap<(String, String), u32> = BTreeMap::new();
    sakura.insert(("腕".into(), "上げ".into()), 1100);
    let mut kero: BTreeMap<(String, String), u32> = BTreeMap::new();
    kero.insert(("脚".into(), "組む".into()), 2100);
    BindResolver::new(sakura, kero, BindOptionDecls::default())
}

/// `mustselect` 宣言を持つ名前解決表を組む（bindopt 設計 D2）。
/// sakura: (目,笑)→1301 / (目,普)→1303 / (目,閉)→1304 / (紅,あり)→1207。
/// mustselect カテゴリ = {目}（紅は非宣言＝正典の既定）。
fn mustselect_resolver() -> BindResolver {
    let mut sakura: BTreeMap<(String, String), u32> = BTreeMap::new();
    sakura.insert(("目".into(), "笑".into()), 1301);
    sakura.insert(("目".into(), "普".into()), 1303);
    sakura.insert(("目".into(), "閉".into()), 1304);
    sakura.insert(("紅".into(), "あり".into()), 1207);
    let mut sakura_ms: BTreeSet<String> = BTreeSet::new();
    sakura_ms.insert("目".into());
    let options = BindOptionDecls {
        sakura_mustselect: sakura_ms,
        ..Default::default()
    };
    BindResolver::new(sakura, BTreeMap::new(), options)
}

/// ポリシー導出は名前空間ごと・`mustselect` 宣言カテゴリのみ MustSelect（bindopt 設計 D2）。
#[test]
fn policy_mustselect_only_declared_category_per_namespace() {
    let r = mustselect_resolver();
    assert_eq!(
        r.policy(BindNamespace::Sakura, "目"),
        BindChoicePolicy::MustSelect,
        "sakura で mustselect 宣言されたカテゴリは MustSelect（bindopt 設計 D2）"
    );
    assert_eq!(
        r.policy(BindNamespace::Sakura, "紅"),
        BindChoicePolicy::Default,
        "非宣言カテゴリ（紅）は正典の既定（Default）"
    );
    assert_eq!(
        r.policy(BindNamespace::Sakura, "未知"),
        BindChoicePolicy::Default,
        "未知カテゴリも正典の既定（Default）"
    );
    assert_eq!(
        r.policy(BindNamespace::Kero, "目"),
        BindChoicePolicy::Default,
        "kero では sakura の mustselect を引かない（名前空間隔離）"
    );
}

/// 空リゾルバ・`bindoption` 宣言なしの表は全カテゴリ既定（bindopt 設計 D3）。
#[test]
fn policy_empty_or_absent_is_default() {
    assert_eq!(
        BindResolver::empty().policy(BindNamespace::Sakura, "目"),
        BindChoicePolicy::Default
    );
    assert_eq!(
        tiny_resolver().policy(BindNamespace::Sakura, "腕"),
        BindChoicePolicy::Default,
        "bindoption 宣言のない表は全カテゴリ既定（Default）"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Task 5.1: `policy()` 導出マトリクスの檻（bindopt 4.2/4.3・設計 §Testing Strategy
// → Unit Tests（seriko・bind.rs）1-2）。宣言 3 形（mustselect／multiple／非宣言）に
// 併記・未知カテゴリを加えた全ケースを 2 名前空間で固定し、本体（sakura）／相方（kero）の
// 名前空間隔離も併せて固定する。`empty()` は全カテゴリ既定かつ名前解決が常に不成立
// ＝適用に到達しない（bindopt 設計 D3 の「実害なし」根拠）を檻に固定する。
// ─────────────────────────────────────────────────────────────────────

/// 導出マトリクス用の名前解決表（本体・相方それぞれに宣言 3 形＋併記を持たせる）。
///
/// sakura 宣言: mustselect={目}／multiple={髪}／併記={腕}（mustselect と multiple の両方）。
/// kero 宣言:   mustselect={脚}／multiple={尾}／併記={羽}。
/// 「紅」は両スコープとも名前表にのみ在って宣言なし＝正典の既定（[`BindChoicePolicy::Default`]）。
/// 本体と相方でカテゴリ名を一切重ねないため、名前空間を取り違えると必ず導出が崩れる。
fn policy_matrix_resolver() -> BindResolver {
    let mut sakura: BTreeMap<(String, String), u32> = BTreeMap::new();
    sakura.insert(("目".into(), "笑".into()), 1301);
    sakura.insert(("髪".into(), "長".into()), 1250);
    sakura.insert(("腕".into(), "上げ".into()), 1100);
    sakura.insert(("紅".into(), "あり".into()), 1207);
    let mut kero: BTreeMap<(String, String), u32> = BTreeMap::new();
    kero.insert(("脚".into(), "組む".into()), 2100);
    kero.insert(("尾".into(), "振り".into()), 2200);
    kero.insert(("羽".into(), "開き".into()), 2300);
    kero.insert(("紅".into(), "あり".into()), 2207);
    let names =
        |names: &[&str]| -> BTreeSet<String> { names.iter().map(|s| (*s).to_string()).collect() };
    let options = BindOptionDecls {
        sakura_mustselect: names(&["目", "腕"]),
        kero_mustselect: names(&["脚", "羽"]),
        sakura_multiple: names(&["髪", "腕"]),
        kero_multiple: names(&["尾", "羽"]),
    };
    BindResolver::new(sakura, kero, options)
}

/// 本体側（sakura）の導出マトリクス全ケース（bindopt 4.2/4.3）。
/// mustselect 宣言→MustSelect／multiple 宣言→Multiple／非宣言→既定／併記→Multiple／未知→既定。
#[test]
fn policy_matrix_sakura_covers_all_declaration_forms() {
    let r = policy_matrix_resolver();
    assert_eq!(
        r.policy(BindNamespace::Sakura, "目"),
        BindChoicePolicy::MustSelect,
        "sakura の mustselect 宣言カテゴリ（目）は MustSelect＝ちょうど 1 個（bindopt 4.3）"
    );
    assert_eq!(
        r.policy(BindNamespace::Sakura, "髪"),
        BindChoicePolicy::Multiple,
        "sakura の multiple 宣言カテゴリ（髪）は Multiple＝複数可（bindopt 4.3）"
    );
    assert_eq!(
        r.policy(BindNamespace::Sakura, "紅"),
        BindChoicePolicy::Default,
        "名前表に在るが bindoption 非宣言のカテゴリ（紅）は既定＝高々 1 個・解除可（bindopt 4.3）"
    );
    assert_eq!(
        r.policy(BindNamespace::Sakura, "腕"),
        BindChoicePolicy::Multiple,
        "mustselect と multiple の併記カテゴリ（腕）は複数可を優先（bindopt D4）"
    );
    assert_eq!(
        r.policy(BindNamespace::Sakura, "未知"),
        BindChoicePolicy::Default,
        "名前表にも宣言集合にも無い未知カテゴリは既定（bindopt 4.3）"
    );
}

/// 相方側（kero）の導出マトリクス全ケース——本体側と同一の規則が独立に成り立つ（bindopt 4.2/4.3）。
#[test]
fn policy_matrix_kero_covers_all_declaration_forms() {
    let r = policy_matrix_resolver();
    assert_eq!(
        r.policy(BindNamespace::Kero, "脚"),
        BindChoicePolicy::MustSelect,
        "kero の mustselect 宣言カテゴリ（脚）は MustSelect（bindopt 4.3）"
    );
    assert_eq!(
        r.policy(BindNamespace::Kero, "尾"),
        BindChoicePolicy::Multiple,
        "kero の multiple 宣言カテゴリ（尾）は Multiple＝複数可（bindopt 4.3）"
    );
    assert_eq!(
        r.policy(BindNamespace::Kero, "紅"),
        BindChoicePolicy::Default,
        "kero 側の非宣言カテゴリ（紅）も既定＝高々 1 個・解除可（bindopt 4.3）"
    );
    assert_eq!(
        r.policy(BindNamespace::Kero, "羽"),
        BindChoicePolicy::Multiple,
        "kero の併記カテゴリ（羽）も複数可を優先（bindopt D4）"
    );
    assert_eq!(
        r.policy(BindNamespace::Kero, "未知"),
        BindChoicePolicy::Default,
        "kero の未知カテゴリも既定（bindopt 4.3）"
    );
}

/// 併記（`mustselect+multiple`）は複数可を優先する（bindopt D4）——導出順の錨。
///
/// 同じ表で mustselect 単独宣言が MustSelect に留まることも併せて確かめ、
/// 「常に Multiple を返している」だけの退化した実装では緑にならないようにする。
#[test]
fn policy_both_options_declared_prefers_multiple() {
    let r = policy_matrix_resolver();
    assert_eq!(
        r.policy(BindNamespace::Sakura, "腕"),
        BindChoicePolicy::Multiple,
        "mustselect と multiple の両方に属するカテゴリは複数可（正典「multiple で複数のパーツを選択可能」・bindopt D4）"
    );
    assert_eq!(
        r.policy(BindNamespace::Kero, "羽"),
        BindChoicePolicy::Multiple,
        "併記の優先則は相方側でも同一（bindopt D4）"
    );
    assert_eq!(
        r.policy(BindNamespace::Sakura, "目"),
        BindChoicePolicy::MustSelect,
        "mustselect 単独宣言は MustSelect のまま（併記優先は併記のときだけ働く・bindopt D4）"
    );
    assert_eq!(
        r.policy(BindNamespace::Kero, "尾"),
        BindChoicePolicy::Multiple,
        "multiple 単独宣言も Multiple（併記と同じ値へ収束することの確認・bindopt D4）"
    );
}

/// 名前空間の隔離（bindopt 4.3）: 本体の宣言は相方で効かず、相方の宣言も本体で効かない。
///
/// mustselect・multiple・併記のいずれの宣言形についても、反対側の名前空間で引くと
/// 正典の既定（[`BindChoicePolicy::Default`]）へ落ちる。
#[test]
fn policy_is_namespace_isolated_between_sakura_and_kero() {
    let r = policy_matrix_resolver();
    for category in ["目", "髪", "腕"] {
        assert_eq!(
            r.policy(BindNamespace::Kero, category),
            BindChoicePolicy::Default,
            "本体（sakura）の宣言カテゴリ「{category}」を相方（kero）で引いても効かず既定（名前空間隔離・bindopt 4.3）"
        );
    }
    for category in ["脚", "尾", "羽"] {
        assert_eq!(
            r.policy(BindNamespace::Sakura, category),
            BindChoicePolicy::Default,
            "相方（kero）の宣言カテゴリ「{category}」を本体（sakura）で引いても効かず既定（名前空間隔離・bindopt 4.3）"
        );
    }
}

/// `empty()` は全カテゴリ既定であり、かつ名前解決が常に不成立ゆえ**適用に到達しない**
/// （bindopt 設計 D3 の「署名不変でも実害なし」根拠の檻）。
///
/// 既定は「高々 1 個」＝排他置換の意味を持つが、空リゾルバでは `resolve` が常に `None`
/// ＝呼び手（actor）が着せ替え ID を得られないため、排他置換の適用そのものに達しない。
/// 排他置換の対象集合（`category_ids`）も空である。
#[test]
fn empty_resolver_is_default_policy_and_never_reaches_apply() {
    let r = BindResolver::empty();
    for ns in [BindNamespace::Sakura, BindNamespace::Kero] {
        for category in ["目", "髪", "腕", "紅", "未知"] {
            assert_eq!(
                r.policy(ns, category),
                BindChoicePolicy::Default,
                "空リゾルバは宣言集合も空ゆえ全カテゴリが正典の既定（{ns:?}／{category}・bindopt 設計 D3）"
            );
            assert_eq!(
                r.resolve(ns, category, "任意"),
                None,
                "空リゾルバは名前解決が常に不成立＝適用に到達しない（{ns:?}／{category}・bindopt 設計 D3）"
            );
            assert_eq!(
                r.category_ids(ns, category),
                Vec::<u32>::new(),
                "空リゾルバでは排他置換の対象集合も空（{ns:?}／{category}・bindopt 設計 D3）"
            );
        }
    }
}

/// category_ids はカテゴリの全 ID を昇順・重複除去で返す（bindopt 2.1/3.1・排他置換の
/// off 対象集合）。
#[test]
fn category_ids_collects_category_members_ascending() {
    let r = mustselect_resolver();
    assert_eq!(
        r.category_ids(BindNamespace::Sakura, "目"),
        vec![1301, 1303, 1304],
        "カテゴリ「目」の全パーツ ID を昇順で集める（同カテゴリ他パーツ off の対象・bindopt 2.1/3.1）"
    );
    assert_eq!(
        r.category_ids(BindNamespace::Sakura, "紅"),
        vec![1207],
        "単一宣言のカテゴリは 1 要素"
    );
    assert_eq!(
        r.category_ids(BindNamespace::Sakura, "未知"),
        Vec::<u32>::new(),
        "宣言のないカテゴリは空 Vec（捏造しない）"
    );
    assert_eq!(
        r.category_ids(BindNamespace::Kero, "目"),
        Vec::<u32>::new(),
        "kero 名前空間は sakura の宣言を引かない（隔離）"
    );
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

/// scope 写像（D7・要件 14.2）: "0"→Sakura・"1"→Kero・2 以上の数→Char(n)・非数値→None。
#[test]
fn scope_namespace_maps_two_and_above_to_char() {
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
        Some(BindNamespace::Char(2)),
        "scope \"2\" は 3 人目（char2）の名前空間へ写像する（要件 14.2）"
    );
    assert_eq!(
        scope_namespace(&ActorKey::from("12")),
        Some(BindNamespace::Char(12)),
        "2 以上の数はその番号の char の名前空間（要件 14.2）"
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
// spec: areka-P0-offline-surface-dump task 2.3: シェルの読みからの既定と名前の表（要件 14.1〜14.3・5.2・5.9）。

/// 3 種類のスコープで既定と名前が分かれるシェルの descript.txt。`服` の A（3 番）は B（20 番）より
/// 前の行に書く（キーの順では `bindgroup20` が先＝行の順で並べないと赤）。B は 21 番でも書き直す。
const SCOPED_SHELL: &str = "charset,UTF-8\r\n\
sakura.bindgroup3.name,服,A\r\n\
sakura.bindgroup3.default,1\r\n\
sakura.bindgroup20.name,服,B\r\n\
sakura.bindgroup5.name,帽子,C\r\n\
kero.bindgroup30.name,尾,長い\r\n\
kero.bindgroup30.default,1\r\n\
char2.bindgroup41.name,服,赤\r\n\
char2.bindgroup40.name,服,青\r\n\
char2.bindgroup41.default,1\r\n\
char2.bindoption0.group,服,multiple\r\n\
sakura.bindgroup21.name,服,B\r\n";

/// シェルの descript.txt `shell` を持つゴーストを作業フォルダ（`target\\` の下）に組み、着せ替えの読みを返す。
pub(super) fn groups_of(shell: &str) -> areka_parsers::package::BindGroupDefaults {
    let work = sample_ghost_kit::WorkDir::new().expect("作業フォルダを取れる");
    let master = work.path().join("ghost").join("master");
    std::fs::create_dir_all(&master).expect("ghost/master を作る");
    std::fs::write(master.join("descript.txt"), "charset,UTF-8\r\nname,t\r\n")
        .expect("ゴーストの descript.txt を書く");
    let shell_dir = work.path().join("shell").join("master");
    std::fs::create_dir_all(&shell_dir).expect("shell/master を作る");
    std::fs::write(shell_dir.join("descript.txt"), shell).expect("シェルの descript.txt を書く");
    areka_parsers::package::resolve_with_shell(
        work.path(),
        areka_parsers::charset::DefaultEncoding::Utf8,
        None,
    )
    .expect("検体のシェルは解決できる")
    .bindgroups
}

/// 既定の組: スコープ 0 に `sakura.*`・1 に `kero.*`・2 に `char2.*`・書かれていない 3 と数でない
/// スコープは空（要件 14.1）。
#[test]
fn default_binds_from_groups_splits_by_scope() {
    let defaults = DefaultBinds::from_groups(&groups_of(SCOPED_SHELL));
    assert_eq!(defaults.for_scope(&ActorKey::from("0")).ids(), &[3]);
    assert_eq!(defaults.for_scope(&ActorKey::from("1")).ids(), &[30]);
    assert_eq!(defaults.for_scope(&ActorKey::from("2")).ids(), &[41]);
    assert!(
        defaults.for_scope(&ActorKey::from("3")).ids().is_empty(),
        "char3 の無いスコープ 3 は何も着ない"
    );
    assert!(defaults.for_scope(&ActorKey::from("x")).ids().is_empty());
}

/// 名前の表: 3 種類のスコープがそれぞれ自分の表を引き、ほかのスコープの名前は引かない。
/// `char2` の `multiple` は `char2` だけに効く（要件 14.2・14.3）。
#[test]
fn bind_resolver_from_groups_reads_three_kinds_of_scope() {
    let r = BindResolver::from_groups(&groups_of(SCOPED_SHELL));
    assert_eq!(r.resolve(BindNamespace::Sakura, "服", "A"), Some(3));
    assert_eq!(
        r.resolve(BindNamespace::Sakura, "服", "B"),
        Some(21),
        "同じ名前は後の宣言が勝つ"
    );
    assert_eq!(r.resolve(BindNamespace::Kero, "尾", "長い"), Some(30));
    assert_eq!(r.resolve(BindNamespace::Char(2), "服", "赤"), Some(41));
    assert_eq!(r.resolve(BindNamespace::Sakura, "服", "赤"), None);
    assert_eq!(r.resolve(BindNamespace::Char(2), "服", "A"), None);
    assert_eq!(r.resolve(BindNamespace::Char(3), "服", "赤"), None);
    assert_eq!(
        r.policy(BindNamespace::Char(2), "服"),
        BindChoicePolicy::Multiple
    );
    assert_eq!(
        r.policy(BindNamespace::Sakura, "服"),
        BindChoicePolicy::Default
    );
    assert_eq!(r.category_ids(BindNamespace::Char(2), "服"), vec![40, 41]);
    assert_eq!(r.name_of(BindNamespace::Char(2), 40), Some(("服", "青")));
}

/// カテゴリのパーツ名は descript.txt に書かれた行の順（同じ名前は 1 度）。無いカテゴリ・`char3` は空。
#[test]
fn category_parts_follow_written_order() {
    let r = BindResolver::from_groups(&groups_of(SCOPED_SHELL));
    assert_eq!(
        r.category_parts_in_order(BindNamespace::Sakura, "服"),
        vec!["A".to_owned(), "B".to_owned()]
    );
    assert_eq!(
        r.category_parts_in_order(BindNamespace::Char(2), "服"),
        vec!["赤".to_owned(), "青".to_owned()]
    );
    assert!(
        r.category_parts_in_order(BindNamespace::Sakura, "無い")
            .is_empty()
    );
    assert!(
        r.category_parts_in_order(BindNamespace::Char(3), "服")
            .is_empty()
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
    assert_eq!(
        after_off.ids(),
        &[] as &[u32],
        "on 済みを off で除去（脱衣・R3.3）"
    );
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
    assert_eq!(once, twice, "on→on の重複適用は結果集合が同値（冪等・D9）");
    assert_eq!(
        twice.ids(),
        &[1100, 1207],
        "重複適用でも 1100 は 1 件（dedup）"
    );
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
        seq.iter().fold(BindSet::default(), |acc, &(id, on)| {
            accumulate(&acc, id, on)
        })
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
    assert_eq!(
        after, base,
        "存在しない ID の off は入力集合と同値（no-op）"
    );
}

/// `DefaultBinds::uniform` はどのスコープ（数でないスコープも）にも同じ組を返す（今の振る舞い・要件 14.3）。
#[test]
fn default_binds_uniform_returns_same_set_for_every_scope() {
    let set = areka_emo_compose::BindSet::from_ids([1100, 1207]);
    let binds = DefaultBinds::uniform(set.clone());
    for scope in ["0", "1", "2", "x"] {
        assert_eq!(
            binds.for_scope(&ActorKey::from(scope)),
            &set,
            "scope {scope}"
        );
    }
}

/// `DefaultBinds::single` はそのスコープだけに組を持ち、ほかのスコープ・数でないスコープは空（要件 14.3）。
#[test]
fn default_binds_single_holds_one_scope_and_others_are_empty() {
    let set = areka_emo_compose::BindSet::from_ids([3000]);
    let binds = DefaultBinds::single(1, set.clone());
    assert_eq!(binds.for_scope(&ActorKey::from("1")), &set);
    let empty = areka_emo_compose::BindSet::default();
    for scope in ["0", "2", "x"] {
        assert_eq!(
            binds.for_scope(&ActorKey::from(scope)),
            &empty,
            "scope {scope}"
        );
    }
}

/// `DefaultBinds::default` はどのスコープにも空の組を返す（要件 14.3）。
#[test]
fn default_binds_default_is_empty_for_every_scope() {
    let binds = DefaultBinds::default();
    let empty = areka_emo_compose::BindSet::default();
    for scope in ["0", "1", "x"] {
        assert_eq!(
            binds.for_scope(&ActorKey::from(scope)),
            &empty,
            "scope {scope}"
        );
    }
}
