# Bevy ECS Hierarchy API 完全ガイド

> **出典**: https://claude.ai/public/artifacts/3ae95deb-9612-48bf-913c-eebb7b9ee764  
> **取得日**: 2026年2月16日  
> **対象バージョン**: Bevy ECS 0.16 - 0.18

Bevy の親子関係（Hierarchy）API は Bevy 0.16で根本的に刷新された。旧来の `Parent`/`Children` システムは、汎用的な ECS Relationship システムの上に再構築され、`ChildOf`/`Children` コンポーネントへと移行した。本レポートでは、現行の最新API（0.16〜0.18）を中心に、基本コンポーネントからツリー走査、階層構築コマンドまでを段階的に整理する。

---

## 1. 基盤となる2つのコンポーネント: `ChildOf` と `Children`

Bevy の Hierarchy は 2つのコンポーネントの連携 で成り立つ。**`ChildOf` が「真のデータ（source of truth）」**であり、**`Children` はそこから自動同期されるキャッシュ**という関係にある。

### `ChildOf` — 子エンティティが持つリレーションシップ

```rust
// 定義（bevy::ecs::hierarchy）
pub struct ChildOf(pub Entity);

impl ChildOf {
    pub fn parent(&self) -> Entity  // 親エンティティを返す
}
```

`ChildOf` は `Relationship` トレイトを実装する **イミュータブルコンポーネント**（`Component<Mutability = Immutable>`）である。子エンティティ側に付与し、引数に親のEntityを指定する。**挿入・変更・削除時にコンポーネントフックが発火し、対応する親の `Children` が自動更新される**。

```rust
// 最もシンプルな親子関係の構築
let parent = commands.spawn(Player).id();
let child = commands.spawn((Weapon, ChildOf(parent))).id();
// → parent に Children コンポーネントが自動挿入され、child が登録される

// 親の変更（リペアレンティング）
commands.entity(child).insert(ChildOf(new_parent));
// → 旧親の Children から除去、新親の Children に追加（自動）

// 親子関係の解除
commands.entity(child).remove::<ChildOf>();
```

**旧バージョンとの対応**: Bevy 0.15以前の `Parent` コンポーネントが `ChildOf` にリネームされた。`Parent` は子に付いていたにもかかわらず名前が「Parent」だったため混乱を招いていた。また `Deref` 実装と `.get()` メソッドは廃止され、`.parent()` メソッドに一本化された。

---

### `Children` — 親エンティティに自動付与されるターゲットコレクション

```rust
// 定義（bevy::ecs::hierarchy）
pub struct Children(/* 内部: Vec<Entity> */);
```

`Children` は `RelationshipTarget` トレイトを実装し、**`Deref<Target = [Entity]>` を持つ**。つまり**スライスとして直接操作できる**。

```rust
// 主要メソッド
impl Children {
    pub fn swap(&mut self, a: usize, b: usize)
    pub fn sort_by<F>(&mut self, compare: F)
    pub fn sort_by_key<K, F>(&mut self, compare: F)
    pub fn sort_unstable_by<F>(&mut self, compare: F)
    pub fn sort_unstable_by_key<K, F>(&mut self, compare: F)
}
```

**`Children` は 直接操作すべきではない**（並べ替えのための `sort_*` 系メソッドは例外）。子エンティティの追加・削除は常に `ChildOf` の挿入・削除を通じて行う。

```rust
fn process_children(query: Query<(Entity, &Children)>) {
    for (parent, children) in &query {
        println!("子の数: {}", children.len());
        for &child in children.iter() {
            // 各子エンティティに対する処理
        }
        // インデックスアクセスも可能
        let first_child = children[0];
    }
}
```

**重要な挙動**: 親エンティティを `despawn()` すると、すべての子孫が自動的にdespawnされる。0.15以前は `despawn_recursive()` が必要だったが、0.16以降はデフォルト動作になった。

---

## 2. Query による直接アクセスと基本パターン

最も基本的な走査は `Query` で `ChildOf` や `Children` を取得し、手動でたどる方法である。

### 子 → 親をたどる

```rust
fn find_parent(
    child_query: Query<&ChildOf, With<Weapon>>,
    parent_query: Query<&Name>,
) {
    for child_of in &child_query {
        let parent_entity = child_of.parent();
        if let Ok(name) = parent_query.get(parent_entity) {
            println!("親の名前: {}", name);
        }
    }
}
```

### 親 → 子をたどる

```rust
fn iterate_children(
    parent_query: Query<(Entity, &Children), With<Player>>,
    child_query: Query<&Name>,
) {
    for (parent, children) in &parent_query {
        for &child in children.iter() {
            if let Ok(name) = child_query.get(child) {
                println!("子の名前: {}", name);
            }
        }
    }
}
```

### `Children` を直接 Query で含めるパターン

UIでよく使われる、親のコンポーネントと `Children` を同時に取得するパターン:

```rust
fn button_system(
    interaction_query: Query<(&Interaction, &Children, &mut ImageNode),
        (Changed<Interaction>, With<Button>)>,
    mut text_query: Query<&mut Text>,
) {
    for (interaction, children, mut image) in &interaction_query {
        let mut text = text_query.get_mut(children[0]).unwrap();
        match interaction {
            Interaction::Pressed => { **text = "Pressed".to_string(); }
            _ => {}
        }
    }
}
```

---

## 3. Query 上のツリー走査メソッド群

Bevy 0.16以降、以前 `HierarchyQueryExt` トレイトで提供されていた走査メソッドは **`Query` の固有メソッド**（inherent method）に昇格した。これらは汎用の `Relationship`/`RelationshipTarget` システム上に実装されているため、`ChildOf`/`Children` 以外のカスタムリレーションシップにも同じメソッドが使える。

### 走査メソッド一覧

| メソッド名 | 必要な Query | 戻り値 | 説明 |
|-----------|-------------|--------|------|
| `related(entity)` | `Query<&ChildOf>` | `Result<Entity>` | 親を1つ返す |
| `relationship_sources(entity)` | `Query<&Children>` | `Result<&[Entity]>` | 直接の子一覧 |
| `root_ancestor(entity)` | `Query<&ChildOf>` | `Entity` | 最上位の祖先（自分自身の場合あり） |
| `iter_ancestors(entity)` | `Query<&ChildOf>` | `impl Iterator<Item=Entity>` | 祖先を順に上へ |
| `iter_descendants(entity)` | `Query<&Children>` | `DescendantIter` | 幅優先で全子孫 |
| `iter_descendants_depth_first(entity)` | `Query<&Children>` | `DescendantDepthFirstIter` | 深さ優先で全子孫 |
| `iter_leaves(entity)` | `Query<&Children>` | `impl Iterator<Item=Entity>` | 葉ノードのみ（深さ優先） |
| `iter_siblings(entity)` | `Query<&ChildOf>` | `impl Iterator<Item=Entity>` | 兄弟（自分自身を除く） |

### 各メソッドの使用例

**`iter_descendants` — 全子孫を幅優先で走査（最頻出）**

```rust
fn move_scene_entities(
    moved_scene: Query<Entity, With<MovedScene>>,
    children: Query<&Children>,
    mut transforms: Query<&mut Transform>,
) {
    for scene_entity in &moved_scene {
        for entity in children.iter_descendants(scene_entity) {
            if let Ok(mut transform) = transforms.get_mut(entity) {
                transform.translation.y += 1.0;
            }
        }
    }
}
```

**`iter_ancestors` — 祖先チェーンを上方向に走査**

```rust
fn find_root(
    entity: Single<Entity, With<Leaf>>,
    parent_query: Query<&ChildOf>,
) {
    for ancestor in parent_query.iter_ancestors(*entity) {
        println!("祖先: {:?}", ancestor);
    }
    // または root_ancestor で一気に最上位へ
    let root = parent_query.root_ancestor(*entity);
}
```

**`iter_leaves` — 葉ノード（子を持たないエンティティ）を列挙**

```rust
fn count_leaves(
    root: Single<Entity, With<TreeRoot>>,
    children_query: Query<&Children>,
) {
    let leaf_count = children_query.iter_leaves(*root).count();
    println!("葉ノード数: {}", leaf_count);
}
```

**`iter_siblings` — 兄弟エンティティの走査**

```rust
fn highlight_siblings(
    selected: Single<Entity, With<Selected>>,
    child_of_query: Query<&ChildOf>,
    mut colors: Query<&mut Sprite>,
) {
    for sibling in child_of_query.iter_siblings(*selected) {
        if let Ok(mut sprite) = colors.get_mut(sibling) {
            sprite.color = Color::srgb(1.0, 1.0, 0.0);
        }
    }
}
```

### `iter_descendants` とフィルタに関する既知の注意点

**`Query<&Children, With<SomeFilter>>` のようにフィルタ付きで `iter_descendants` を使うと、フィルタは返却される値だけでなく走査自体にも適用される**。つまりフィルタに一致しないノードの配下はスキップされてしまう（GitHub Issue #18686）。特定の型の子孫だけ取り出したい場合は、フィルタなしの `Query<&Children>` で走査し、別の Query でフィルタリングする。

```rust
// ❌ フィルタが走査を阻害する
fn bad(root: Entity, q: Query<&Children, With<Leaf>>) {
    for e in q.iter_descendants(root) { /* 途中で止まる可能性 */ }
}

// ✅ 走査と判定を分離する
fn good(root: Entity, all_children: Query<&Children>, leaves: Query<(), With<Leaf>>) {
    for e in all_children.iter_descendants(root) {
        if leaves.contains(e) { /* Leaf のみ処理 */ }
    }
}
```

---

## 4. 旧API: HierarchyQueryExt トレイト（Bevy 0.15以前）

Bevy 0.15以前では、`bevy_hierarchy` クレートが提供する 拡張トレイト `HierarchyQueryExt` でツリー走査が行われていた。0.16で `Query` の固有メソッドに吸収されたが、旧コードの理解のために残しておく。

```rust
// bevy_hierarchy 0.15 のトレイト定義
pub trait HierarchyQueryExt<'w, 's, D: QueryData, F: QueryFilter> {
    fn parent(&'w self, entity: Entity) -> Option<Entity>;        // Query<&Parent>
    fn children(&'w self, entity: Entity) -> &'w [Entity];        // Query<&Children>
    fn root_ancestor(&'w self, entity: Entity) -> Entity;         // Query<&Parent>
    fn iter_leaves(&'w self, entity: Entity)
        -> impl Iterator<Item = Entity> + 'w;                     // Query<&Children>
    fn iter_siblings(&'w self, entity: Entity)
        -> impl Iterator<Item = Entity>;                          // Query<(Option<&Parent>, Option<&Children>)>
    fn iter_descendants(&'w self, entity: Entity)
        -> DescendantIter<'w, 's, D, F>;                         // Query<&Children>
    fn iter_descendants_depth_first(&'w self, entity: Entity)
        -> DescendantDepthFirstIter<'w, 's, D, F>;               // Query<&Children>
    fn iter_ancestors(&'w self, entity: Entity)
        -> AncestorIter<'w, 's, D, F>;                           // Query<&Parent>
}
```

**0.15→0.16 の対応は以下のとおり**:

| Bevy 0.15 (HierarchyQueryExt) | Bevy 0.16+ (Query固有メソッド) |
|-------------------------------|--------------------------------|
| `parent_query.parent(entity)` | `child_of_query.related(entity)` |
| `children_query.children(entity)` | `children_query.relationship_sources(entity)` |
| `children_query.iter_descendants(entity)` | `children_query.iter_descendants(entity)` （同名） |
| `parent_query.iter_ancestors(entity)` | `child_of_query.iter_ancestors(entity)` （同名） |

---

## 5. EntityCommands による階層の構築と操作

### `BuildChildren` トレイト — 階層操作の中心

`EntityCommands` と `EntityWorldMut` に実装されており、コマンドキュー経由または即座に階層を操作できる。

```rust
pub trait BuildChildren {
    fn with_children(&mut self, f: impl FnOnce(&mut ChildSpawnerCommands)) -> &mut Self;
    fn with_child<B: Bundle>(&mut self, bundle: B) -> &mut Self;
    fn add_child(&mut self, child: Entity) -> &mut Self;
    fn add_children(&mut self, children: &[Entity]) -> &mut Self;
    fn insert_children(&mut self, index: usize, children: &[Entity]) -> &mut Self;
    fn remove_children(&mut self, children: &[Entity]) -> &mut Self;
    fn clear_children(&mut self) -> &mut Self;
    fn replace_children(&mut self, children: &[Entity]) -> &mut Self;
    fn set_parent(&mut self, parent: Entity) -> &mut Self;
    fn remove_parent(&mut self) -> &mut Self;
}
```

### `with_children` — クロージャで複数の子をスポーン

```rust
commands.spawn(Player)
    .with_children(|spawner: &mut ChildSpawnerCommands| {
        spawner.spawn((Weapon, Name::new("Sword")));
        spawner.spawn((Shield, Name::new("Buckler")));
        let parent = spawner.target_entity(); // 親の Entity を取得
    });
```

0.15 では引数が `&mut ChildBuilder` だったが、0.16 で `&mut ChildSpawnerCommands` にリネームされた。`parent_entity()` も `target_entity()` に変更。

### `with_child` — 単一の子を手軽にスポーン

```rust
// with_children より簡潔に1つの子を追加
commands.spawn(Player)
    .with_child(Weapon)
    .with_child(Shield);
```

### 既存エンティティの操作

```rust
let child1 = commands.spawn(Weapon).id();
let child2 = commands.spawn(Shield).id();
let child3 = commands.spawn(Potion).id();

commands.entity(parent)
    .add_child(child1)                      // 1つの子を追加
    .add_children(&[child2, child3])        // 複数の子を追加
    .insert_children(0, &[child3])          // インデックス0に挿入
    .remove_children(&[child2]);            // child2 を切り離し（despawn はしない）

commands.entity(child1).set_parent(new_parent);  // 親を変更
commands.entity(child1).remove_parent();          // 親子関係を解除
```

### Despawn 関連

```rust
// 親とすべての子孫を再帰的に削除（0.16+ のデフォルト動作）
commands.entity(parent).despawn();

// 子孫のみ削除し、親は残す
commands.entity(parent).despawn_related::<Children>();

// 子を切り離してから親だけ削除したい場合
commands.entity(parent).remove::<Children>().despawn();
```

---

## 6. 宣言的な階層構築: `children!` マクロと `Children::spawn`

Bevy 0.16 で導入された最大の改善の一つが、階層を**バンドルとして宣言的に記述できる仕組み**である。`with_children` のクロージャ方式と比べ、データとして構成可能で、関数からの返却もできる。

### `children!` マクロ（推奨）

```rust
commands.spawn((
    Player,
    children![
        (RightHand, children![Glove, Sword]),
        (LeftHand, children![Glove, Shield]),
    ],
));
```

これは内部的に以下と等価:

```rust
commands.spawn((
    Player,
    Children::spawn((
        Spawn((RightHand, Children::spawn((Spawn(Glove), Spawn(Sword))))),
        Spawn((LeftHand, Children::spawn((Spawn(Glove), Spawn(Shield))))),
    )),
));
```

### `SpawnWith` — クロージャで動的にスポーン

```rust
commands.spawn((
    Name::new("Root"),
    Children::spawn((
        Spawn(Name::new("StaticChild")),
        SpawnWith(|spawner: &mut ChildSpawner| {
            for i in 0..5 {
                spawner.spawn(Name::new(format!("Dynamic_{i}")));
            }
        }),
    )),
));
```

### `SpawnIter` — イテレータベースのスポーン

```rust
commands.spawn((
    Fleet,
    Children::spawn(
        SpawnIter(["Alpha", "Bravo", "Charlie"]
            .into_iter()
            .map(|n| (Ship, Name::new(n))))
    ),
));
```

### 関数からバンドルとして返す

`children!` で構築した階層は `impl Bundle` として返却でき、**ウィジェットパターン**の実現に最適:

```rust
fn player_bundle(name: &str) -> impl Bundle {
    (
        Player,
        Name::new(name),
        children![
            (RightHand, children![Glove, Sword]),
            (LeftHand, children![Glove, Shield]),
        ],
    )
}

// 使用
commands.spawn(player_bundle("Hero"));
```

### カスタムリレーションシップ用の `related!` マクロ

```rust
#[derive(Component)]
#[relationship(relationship_target = LikedBy)]
struct Likes(pub Entity);

#[derive(Component)]
#[relationship_target(relationship = Likes)]
struct LikedBy(Vec<Entity>);

commands.spawn((
    Name::new("Monica"),
    related!(LikedBy[
        Name::new("Naomi"),
        Name::new("Dwight"),
    ]),
));
```

---

## バージョン別API対応まとめ

| 機能 | Bevy 0.15 以前 | Bevy 0.16+ |
|------|---------------|-----------|
| 親参照コンポーネント | `Parent` | `ChildOf` |
| 子一覧コンポーネント | `Children`（手動管理） | `Children`（`RelationshipTarget` 自動同期） |
| ツリー走査 | `HierarchyQueryExt` 拡張トレイト | `Query` 固有メソッド |
| 親取得 | `parent_query.parent(e)` | `child_of_query.related(e)` |
| 子一覧取得 | `children_query.children(e)` | `children_query.relationship_sources(e)` |
| 階層スポーン | `with_children` クロージャのみ | `children!` マクロ / `Children::spawn` / `with_children` |
| 再帰 despawn | `despawn_recursive()` が必要 | `despawn()` がデフォルトで再帰 |
| 子孫のみ despawn | `despawn_descendants()` | `despawn_related::<Children>()` |
| クレート | `bevy_hierarchy`（別クレート） | `bevy_ecs` に統合 |

---

## 実践的な設計指針

Bevy の Hierarchy API を効果的に使うには、いくつかのポイントを押さえるとよい:

1. **`ChildOf` が唯一の真実**であり、**`Children` は読み取り専用のキャッシュ**として扱うべきである。親子関係の変更は常に `ChildOf` の挿入・削除で行う。

2. ツリー全体を検索する場合は `iter_descendants` が最も汎用的だが、**フィルタ付き Query を渡すと走査が途中で打ち切られる**（Issue #18686）ため、走査と判定を分離する。

3. 階層構築には `children!` マクロを使うと、データとして扱えるためテストや再利用が容易になる。`with_children` はその場限りの構築に適している。

4. `despawn()` の仕様変更（0.16以降は再帰がデフォルト）に注意し、子だけを削除したい場合は `despawn_related::<Children>()` を使う。

5. `Query<&Children>` を使う場合、**`Children` の順序はエンティティの挿入順序（`ChildOf` 挿入順序）を保持する**ため、レイアウトや描画順序の権威的ソースとして利用できる。

---

**本ガイドは Bevy 0.18.0 時点の情報に基づいています。**
