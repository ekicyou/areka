use bevy_ecs::prelude::*;

/// appearance ダイジェスト版番号。ノード内で単調 ++。
///
/// 含める:   VisualDrawContent / VisualClip / VisualFlags
/// 含めない: VisualTransform（to_world を ε 比較 / scroll を blit で扱うため）
///           z順（lis が構造 diff で検出）
///
/// 不変条件: transform 由来で bump してはならない。破るとスクロールが
///           全ノード内容 damage 化し、フェーズ3 の blit を無効化する。
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[repr(transparent)]
pub struct VisualVersion(pub u64);

impl VisualVersion {
    #[inline]
    pub fn bump(&mut self) {
        self.0 = self.0.wrapping_add(1);
    }
}
