//! # Tree Iteration - ECS階層の汎用走査イテレータ
//!
//! このモジュールは、ECS階層システムにおける汎用的な走査アルゴリズムを提供します。
//!
//! ## 主要機能
//!
//! ### DepthFirstReversePostOrder
//!
//! 深さ優先・逆順・後順走査イテレータ。ヒットテスト（最前面優先）や
//! フォーカス管理など、Z順序を考慮した処理に使用します。
//!
//! # 例
//!
//! ```rust,ignore
//! use wintf::ecs::common::DepthFirstReversePostOrder;
//!
//! // ヒットテストでの使用例
//! let traversal = DepthFirstReversePostOrder::new(root);
//! for entity in traversal.iter(world) {
//!     if hit_test_entity(world, entity, point) {
//!         return Some(entity);
//!     }
//! }
//! ```

use bevy_ecs::prelude::*;

/// 深さ優先・逆順・後順走査イテレータ
///
/// # 走査順序
/// Children 配列の最後の要素（最前面）から走査し、
/// 子孫を全て返却してから親を返す。
///
/// # 例
/// ```text
/// Root
/// ├── Child1
/// │   ├── GC1a
/// │   └── GC1b
/// └── Child2 (最前面)
///     └── GC2a
///
/// 走査順: GC2a → Child2 → GC1b → GC1a → Child1 → Root
/// ```
///
/// # アルゴリズム
/// ## 初期化
/// 1. ルートを「子取り出し済みフラグ=OFF」で積む
///
/// ## next
/// 1. 最後の要素を取り出す
/// 2. 「子取り出し済みフラグ=ON」なら返却
/// 3. 自分を「取り出し済み」にして再度スタックに積む
/// 4. 子供がいたら、子供要素を逆順でスタックに積む（フラグはOFF）
/// 5. 1に戻る
pub struct DepthFirstReversePostOrder {
    /// (Entity, 子取り出し済みフラグ)
    stack: Vec<(Entity, bool)>,
}

impl DepthFirstReversePostOrder {
    /// 新しい走査状態を作成
    ///
    /// # Arguments
    /// - `root`: 走査開始エンティティ
    pub fn new(root: Entity) -> Self {
        Self {
            stack: vec![(root, false)],
        }
    }

    /// World を使用して次のエンティティを取得
    ///
    /// # Arguments
    /// - `world`: ECS World 参照
    ///
    /// # Returns
    /// 次のエンティティ、または走査完了時は None
    pub fn next(&mut self, world: &World) -> Option<Entity> {
        loop {
            // 1. 最後の要素を取り出す
            let (entity, expanded) = self.stack.pop()?;

            // 2. 「子取り出し済みフラグ=ON」なら返却
            if expanded {
                return Some(entity);
            }

            // 3. 自分を「取り出し済み」にして再度スタックに積む
            self.stack.push((entity, true));

            // 4. 子供がいたら、子供要素を順方向でスタックに積む（フラグはOFF）
            //    スタックは後入れ先出しなので、順方向で積むと最後の要素（最前面）が最初に pop される
            if let Some(children) = world.get::<Children>(entity) {
                for child in children.iter() {
                    self.stack.push((child, false));
                }
            }
            // 5. ループ先頭に戻る
        }
    }

    /// 走査結果をベクタとして収集
    ///
    /// # Arguments
    /// - `world`: ECS World 参照
    pub fn collect(&mut self, world: &World) -> Vec<Entity> {
        let mut result = Vec::new();
        while let Some(entity) = self.next(world) {
            result.push(entity);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::world::World;

    /// テスト用のヘルパー: WorldとQueryを作成し、走査順序を取得
    fn collect_traversal_order(world: &World, root: Entity) -> Vec<Entity> {
        DepthFirstReversePostOrder::new(root).collect(world)
    }

    /// 1. 基本走査順序テスト（6ノードツリー）
    ///
    /// ```text
    /// Root
    /// ├── Child1
    /// │   ├── GC1a
    /// │   └── GC1b
    /// └── Child2 (最前面)
    ///     └── GC2a
    ///
    /// 期待される走査順: [GC2a, Child2, GC1b, GC1a, Child1, Root]
    /// ```
    #[test]
    fn test_basic_traversal_order() {
        let mut world = World::new();

        // ツリー構築（spawn順: gc1a=0, gc1b=1, gc2a=2, child1=3, child2=4, root=5）
        let gc1a = world.spawn_empty().id();
        let gc1b = world.spawn_empty().id();
        let gc2a = world.spawn_empty().id();

        let child1 = world.spawn_empty().id();
        let child2 = world.spawn_empty().id();

        let root = world.spawn_empty().id();

        // 親子関係を設定
        // child1 の children: [gc1a, gc1b]
        // child2 の children: [gc2a]
        // root の children: [child1, child2] （child2 が最前面）
        world.entity_mut(child1).add_children(&[gc1a, gc1b]);
        world.entity_mut(child2).add_children(&[gc2a]);
        world.entity_mut(root).add_children(&[child1, child2]);

        // 走査実行
        let result = collect_traversal_order(&world, root);

        // 期待される順序:
        // 1. child2 の子孫（最前面）: gc2a
        // 2. child2 自身
        // 3. child1 の子孫（逆順）: gc1b, gc1a
        // 4. child1 自身
        // 5. root
        assert_eq!(result, vec![gc2a, child2, gc1b, gc1a, child1, root]);
    }

    /// 2. 単一ノード（子なし）テスト
    #[test]
    fn test_single_node() {
        let mut world = World::new();
        let root = world.spawn_empty().id();

        let result = collect_traversal_order(&world, root);

        assert_eq!(result, vec![root]);
    }

    /// 3. 深い階層テスト（4階層）
    ///
    /// ```text
    /// A
    /// └── B
    ///     └── C
    ///         └── D
    ///
    /// 期待される走査順: [D, C, B, A]
    /// ```
    #[test]
    fn test_deep_hierarchy() {
        let mut world = World::new();

        let d = world.spawn_empty().id();
        let c = world.spawn_empty().id();
        let b = world.spawn_empty().id();
        let a = world.spawn_empty().id();

        world.entity_mut(c).add_children(&[d]);
        world.entity_mut(b).add_children(&[c]);
        world.entity_mut(a).add_children(&[b]);

        let result = collect_traversal_order(&world, a);

        assert_eq!(result, vec![d, c, b, a]);
    }

    /// 4. 幅広ツリーテスト（4兄弟）
    ///
    /// ```text
    /// Root
    /// ├── C1
    /// ├── C2
    /// ├── C3
    /// └── C4 (最前面)
    ///
    /// 期待される走査順: [C4, C3, C2, C1, Root]
    /// ```
    #[test]
    fn test_wide_tree() {
        let mut world = World::new();

        let c1 = world.spawn_empty().id();
        let c2 = world.spawn_empty().id();
        let c3 = world.spawn_empty().id();
        let c4 = world.spawn_empty().id();
        let root = world.spawn_empty().id();

        world.entity_mut(root).add_children(&[c1, c2, c3, c4]);

        let result = collect_traversal_order(&world, root);

        // 最前面（C4）から走査
        assert_eq!(result, vec![c4, c3, c2, c1, root]);
    }

    // ===== W7b-T1 ギャップテスト =====
    // 既存 1〜4 は走査順序（基本/単一/深い/幅広）を固定しているが、`next` の逐次契約、
    // 走査完了後の None 終端、混在兄弟（子あり/子なしの interleaving）、collect が新規
    // イテレータで全件返す等価性が未固定だった。

    /// 5. `next` の逐次契約と走査完了後の None 終端
    ///
    /// 単一ノードに対し `next` を呼ぶと最初にそのノード、続けて呼ぶと `None`。
    /// さらに呼んでも `None` のまま（スタック空からの `pop()?` で安定終端）。
    #[test]
    fn test_next_returns_none_after_exhaustion() {
        let mut world = World::new();
        let root = world.spawn_empty().id();

        let mut traversal = DepthFirstReversePostOrder::new(root);
        assert_eq!(traversal.next(&world), Some(root));
        assert_eq!(traversal.next(&world), None);
        // 完了後の追加呼び出しも None（多重呼び出し安全）
        assert_eq!(traversal.next(&world), None);
    }

    /// 6. `next` 逐次取得が `collect` と同一順序を返す
    ///
    /// 同一ツリーに対し、`next` を 1 件ずつ呼んで構築した列が
    /// `collect`（whileループ）の結果と完全一致することを固定する。
    #[test]
    fn test_next_step_by_step_matches_collect() {
        let mut world = World::new();
        let gc = world.spawn_empty().id();
        let child = world.spawn_empty().id();
        let root = world.spawn_empty().id();
        world.entity_mut(child).add_children(&[gc]);
        world.entity_mut(root).add_children(&[child]);

        // collect 版
        let expected = DepthFirstReversePostOrder::new(root).collect(&world);

        // next 逐次版
        let mut traversal = DepthFirstReversePostOrder::new(root);
        let mut stepwise = Vec::new();
        while let Some(e) = traversal.next(&world) {
            stepwise.push(e);
        }

        assert_eq!(stepwise, expected);
        assert_eq!(stepwise, vec![gc, child, root]);
    }

    /// 7. 混在兄弟（子あり/子なし）の interleaving 順序
    ///
    /// ```text
    /// Root
    /// ├── A (子なし)
    /// └── B (最前面・子 B1 を持つ)
    ///     └── B1
    ///
    /// 期待: B の子孫 B1 → B → A → Root
    /// ```
    /// 最前面（最後の子 B）のサブツリーを先に完全消化してから、子なしの A、最後に Root。
    #[test]
    fn test_mixed_siblings_with_and_without_children() {
        let mut world = World::new();
        let b1 = world.spawn_empty().id();
        let a = world.spawn_empty().id();
        let b = world.spawn_empty().id();
        let root = world.spawn_empty().id();

        world.entity_mut(b).add_children(&[b1]);
        world.entity_mut(root).add_children(&[a, b]); // b が最前面

        let result = collect_traversal_order(&world, root);

        assert_eq!(result, vec![b1, b, a, root]);
    }

    /// 8. Children コンポーネントを持たないノードは葉として即返却される
    ///
    /// `world.get::<Children>(entity)` が None を返す枝（葉ノード）の終端を直接固定する。
    /// 子を一切持たないルートでも走査は自身 1 件で完了する。
    #[test]
    fn test_node_without_children_component_is_leaf() {
        let mut world = World::new();
        let leaf = world.spawn_empty().id();
        // Children を一切付与しない（get::<Children> は None）

        let mut traversal = DepthFirstReversePostOrder::new(leaf);
        assert_eq!(traversal.next(&world), Some(leaf));
        assert_eq!(traversal.next(&world), None);
    }
}
