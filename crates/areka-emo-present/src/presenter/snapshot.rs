//! 画面を変えずに読む口 3 本（MCP の `dump_surface`／`dump_balloon` が使う・spec
//! `areka-P0-mcp-dump-images`）——最後に表示した絵・生の surface ID の有無・単体の合成。
//!
//! 3 本とも `&self` で、合成メモ・表示の状態・World を変えず、`tracing` の記録を自分では出さない
//! （失敗の判断と記録は呼び手のツールが持つ）。戻り値は既存の公開の型だけで、新しい公開の型は無い。

use super::{
    BindSet, ComposeError, ComposedSurface, Composer, EmoPresenter, PatternState, TargetId,
};

impl EmoPresenter {
    /// target のシェルに、生の surface ID が在るか（別名の表は見ない）。未登録の target は `None`。
    ///
    /// target の `EmoWorld` はシェル全体から組まれているので、スコープ用に分けた ID も在ると答える。
    pub fn has_surface(&self, target: TargetId, surface_id: u32) -> Option<bool> {
        Some(
            self.targets
                .get(&target)?
                .emo_world
                .surface(surface_id)
                .is_some(),
        )
    }

    /// 最後に表示が成立した surface ID と、その合成済みの絵（原寸・乗算済み BGRA）。
    ///
    /// 未登録・一度も表示していない → `None`。`last_show` は `Hide` で消えないので、隠していても
    /// `Some`。絵が合成メモに無い（メモの全破棄の後で再表示がまだ）→ `Some((id, None))`。
    /// メモは `get` で引く（最近使用順を動かさない・[`Self::read_back`] と同じ引き方）。
    pub fn last_shown(&self, target: TargetId) -> Option<(u32, Option<&ComposedSurface>)> {
        let t = self.targets.get(&target)?;
        let (surface_id, binds, pattern) = t.last_show.as_ref()?;
        let composed = t
            .cache
            .get(*surface_id, binds, pattern)
            .map(|entry| &entry.composed);
        Some((*surface_id, composed))
    }

    /// surface を単体で画面の外に合成して返す。未登録の target は `None`。
    ///
    /// 着せ替え＝最後に表示したときの集合（一度も表示していなければ空）、アニメーション＝空。
    /// target の `composer` は使わず新しい [`Composer`] で合成する（`&self` のまま・スクラッチを
    /// 共有しない）。合成メモには入れない。無い ID で呼ぶと合成の層が自分で `error!` を出すので、
    /// 呼び手は先に [`Self::has_surface`] で確かめる。
    pub fn compose_alone(
        &self,
        target: TargetId,
        surface_id: u32,
    ) -> Option<Result<ComposedSurface, ComposeError>> {
        let t = self.targets.get(&target)?;
        let empty = BindSet::default();
        let binds = t.last_show.as_ref().map_or(&empty, |(_, binds, _)| binds);
        Some(Composer::new().compose(
            &t.emo_world,
            &t.atlas,
            surface_id,
            binds,
            &PatternState::default(),
        ))
    }
}
