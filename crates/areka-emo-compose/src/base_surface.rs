//! 今の土台の面を答える 1 か所（計画の土台の面の欄・当たりの領域・ID の描き先の塗り直しの鍵が引く）。

use crate::bind::BindSet;
use crate::pattern::PatternState;
use crate::world::EmoWorld;

impl EmoWorld {
    /// 今の土台の面。base のコマも ON の着せ替えの pattern0 の base も無ければ surface_id。
    ///
    /// 計画が根の面の静的な層を置き換えるのと同じ判定で答え、状態を持たない（コマが終わるか着せ替えが
    /// OFF になれば、次の答えは元の面）。
    pub fn current_base_surface(
        &self,
        surface_id: u32,
        binds: &BindSet,
        pattern: &PatternState,
    ) -> u32 {
        crate::plan_base::root_base(self, surface_id, binds, pattern).unwrap_or(surface_id)
    }
}

#[cfg(test)]
#[path = "base_surface_tests.rs"]
mod tests;
