use bevy_ecs::prelude::*;
use std::collections::HashMap;
use taffy::TaffyError;
use taffy::prelude::*;

/// taffyのStyle
#[derive(Component, Debug, Clone, PartialEq, Default)]
#[repr(transparent)]
pub struct TaffyStyle(pub(crate) Style);

// taffy 0.13 の Style は CompactLength（calc 式用の *const () タグ付きポインタ）を含むため
// 自動では Send/Sync にならない。`TaffyStyle::new` は公開なので calc の値も入りうるが、
// wintf が使う `TaffyTree<()>` は calc のポインタを一度も読まない（`resolve_calc_value` は
// 0 を返すだけ）。ポインタの先に触れないので、スレッドをまたいでも安全。
unsafe impl Send for TaffyStyle {}
unsafe impl Sync for TaffyStyle {}

impl TaffyStyle {
    pub fn new(style: Style) -> Self {
        Self(style)
    }

    /// テスト用: 内部Styleへの参照を取得
    pub fn style(&self) -> &Style {
        &self.0
    }
}

/// レイアウト計算結果
#[derive(Component, Debug, Clone, PartialEq, Copy, Default)]
#[repr(transparent)]
pub struct TaffyComputedLayout(pub(crate) Layout);

impl From<Layout> for TaffyComputedLayout {
    fn from(layout: Layout) -> Self {
        Self(layout)
    }
}

/// Taffyレイアウトエンジンとエンティティマッピングを管理するリソース
#[derive(Resource)]
pub struct TaffyLayoutResource {
    /// Taffyレイアウトツリー
    tree: TaffyTree<()>,
    /// Entity → NodeId マッピング
    entity_to_node: HashMap<Entity, NodeId>,
}

// SAFETY: TaffyTree は内部のノードに taffy::Style を保持し、taffy 0.13 の Style は
// CompactLength（calc 式用の *const () タグ付きポインタ）を含むため自動では Send/Sync に
// ならない。`TaffyTree<()>` は calc のポインタを一度も読まない（`resolve_calc_value` は
// 0 を返すだけ）ので、スレッドをまたいでも安全。
// （同根の根拠は上記 `unsafe impl ... for TaffyStyle` を参照）
unsafe impl Send for TaffyLayoutResource {}
unsafe impl Sync for TaffyLayoutResource {}

impl Default for TaffyLayoutResource {
    fn default() -> Self {
        Self {
            tree: TaffyTree::new(),
            entity_to_node: HashMap::new(),
        }
    }
}

impl TaffyLayoutResource {
    /// 新しいレイアウトノードを作成し、エンティティとマッピングする
    pub fn create_node(&mut self, entity: Entity) -> Result<NodeId, TaffyError> {
        let node_id = self.tree.new_leaf(Style::default())?;
        self.entity_to_node.insert(entity, node_id);
        Ok(node_id)
    }

    /// エンティティに対応するレイアウトノードを削除する
    ///
    /// 木から先に消し、消せたときだけ対応を消す（失敗しても対応の無い節を残さない）。
    pub fn remove_node(&mut self, entity: Entity) -> Result<(), TaffyError> {
        if let Some(&node_id) = self.entity_to_node.get(&entity) {
            self.tree.remove(node_id)?;
            self.entity_to_node.remove(&entity);
        }
        Ok(())
    }

    /// エンティティに対応するNodeIdを取得
    pub fn get_node(&self, entity: Entity) -> Option<NodeId> {
        self.entity_to_node.get(&entity).copied()
    }

    /// Taffyツリーへの参照を取得
    pub fn taffy(&self) -> &TaffyTree<()> {
        &self.tree
    }

    /// Taffyツリーへの可変参照を取得
    pub fn taffy_mut(&mut self) -> &mut TaffyTree<()> {
        &mut self.tree
    }
}
