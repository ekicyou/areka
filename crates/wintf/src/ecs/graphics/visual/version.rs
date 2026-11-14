use bevy_ecs::prelude::*;

/// appearance ダイジェスト版番号。ノード内で単調 ++。
///
/// 含める:   VisualDrawContent / Visual（clip・is_visible。opacity・transform_origin の変化でも上がる）
/// 含めない: VisualTransform（to_world を ε 比較 / スクロールを平行移動の写しで扱うため）
///           z順（lis が構造 diff で検出）
///
/// 不変条件: transform 由来で bump してはならない。破るとスクロールが
///           全ノード内容 damage 化し、平行移動の写し（`graphics/damage.rs`）を無効化する。
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct VisualVersion(pub u64);

impl VisualVersion {
    #[inline]
    pub fn bump(&mut self) {
        self.0 = self.0.wrapping_add(1);
    }
}
