//! 画面を変えずに読む口 4 本（MCP の `dump_surface`／`dump_balloon` が使う）——最後に表示した絵・生の
//! surface ID の有無・単体の合成・最後に表示した絵の合成し直し。
//!
//! 加えて、別名の表と生の surface ID の集合の写し 2 本（MCP の台本の検査 `check_script` が使う）。
//!
//! 絵は計画を画面の外の描き手（`OffscreenRenderer`）で原寸に描く。どれも `&self` で、表示の状態・World を
//! 変えない（書き出しの控えと頁の葉の控えだけを満たす）。描けないときだけ `error!` を残す。

use std::collections::{BTreeMap, BTreeSet};

use areka_emo_compose::DrawPlan;

use super::target::PresentTarget;
use super::{
    BindSet, ComposeError, ComposedSurface, Composer, EmoPresenter, PatternState, PresentError,
    TargetId,
};
use crate::cache::CacheEntry;
use crate::gpu_atlas::GpuAtlas;
use crate::offscreen::{OffscreenRenderer, hardware_device};

impl PresentTarget {
    /// 計画を画面の外で原寸に描く。頁の控えがまだ無ければ新しい hardware の装置で作って控える
    /// （最初の表示が画面の装置で作り直す）。描けなければ `error!` を残して理由を返す。
    fn draw_offscreen(&self, target: TargetId, plan: &DrawPlan) -> Result<ComposedSurface, String> {
        let mut gpu = self.gpu.borrow_mut();
        let drawn = match gpu.as_mut() {
            Some(gpu) => OffscreenRenderer::render_plan(gpu, plan),
            None => hardware_device()
                .and_then(|device| GpuAtlas::new(&device, &self.atlas))
                .and_then(|fresh| OffscreenRenderer::render_plan(gpu.insert(fresh), plan)),
        };
        drawn.map_err(|e| {
            tracing::error!(
                ?target,
                surface_id = plan.surface_id,
                error = %e,
                "書き出し: 計画を画面の外で描けない"
            );
            match e {
                PresentError::Compose(ComposeError::Render { reason, .. }) => reason,
                other => other.to_string(),
            }
        })
    }

    /// 合成メモの項の書き出しの控え（初めて読まれたときに描いて満たす・描けなかったことも控える）。
    pub(super) fn snapshot<'a>(
        &self,
        target: TargetId,
        entry: &'a CacheEntry,
    ) -> Result<&'a ComposedSurface, PresentError> {
        let drawn = entry
            .snapshot
            .get_or_init(|| self.draw_offscreen(target, &entry.plan));
        drawn.as_ref().map_err(|reason| {
            ComposeError::Render {
                surface: entry.plan.surface_id,
                reason: reason.clone(),
            }
            .into()
        })
    }

    /// 新しい計画を組んで画面の外で描く（合成メモには入れない）。
    fn draw_fresh(
        &self,
        target: TargetId,
        surface_id: u32,
        binds: &BindSet,
        pattern: &PatternState,
    ) -> Result<ComposedSurface, ComposeError> {
        let plan =
            Composer::new().plan(&self.emo_world, &self.atlas, surface_id, binds, pattern)?;
        self.draw_offscreen(target, &plan)
            .map_err(|reason| ComposeError::Render {
                surface: surface_id,
                reason,
            })
    }
}

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

    /// target の別名の表（`surface.alias`）の写し。未登録の target は `None`。記録を出さない。
    ///
    /// target の `EmoWorld::alias_snapshot` をそのまま返す（再生の `SurfaceResolver` と同じ材料）。
    pub fn alias_snapshot(&self, target: TargetId) -> Option<BTreeMap<String, Vec<u32>>> {
        Some(self.targets.get(&target)?.emo_world.alias_snapshot())
    }

    /// target のシェルに在る生の surface ID の集合の写し（別名の表は見ない）。未登録の target は
    /// `None`。記録を出さない。[`Self::has_surface`] が真を返す ID の全部と一致する。
    pub fn surface_ids(&self, target: TargetId) -> Option<BTreeSet<u32>> {
        Some(self.targets.get(&target)?.emo_world.surface_ids().collect())
    }

    /// 最後に表示が成立した surface ID と、その合成済みの絵（原寸・乗算済み BGRA）。
    ///
    /// 未登録・一度も表示していない → `None`。`last_show` は `Hide` で消えないので、隠していても
    /// `Some`。絵が合成メモに無い（メモの全破棄の後で再表示がまだ）・画面の外で描けない →
    /// `Some((id, None))`。絵は合成メモの項の書き出しの控えで、初めて読まれたときに描く。
    /// メモは `get` で引く（最近使用順を動かさない・[`Self::read_back`] と同じ引き方）。
    pub fn last_shown(&self, target: TargetId) -> Option<(u32, Option<&ComposedSurface>)> {
        let t = self.targets.get(&target)?;
        let (surface_id, binds, pattern) = t.last_show.as_ref()?;
        let picture = t
            .cache
            .get(*surface_id, binds, pattern)
            .and_then(|entry| t.snapshot(target, entry).ok());
        Some((*surface_id, picture))
    }

    /// 最後に表示が成立したときの着せ替えの集合（静的な既定＋動的な着せ替え）。未登録・一度も表示して
    /// いない → `None`。`last_show` は `Hide` で消えないので、隠していても `Some`。記録を出さない。
    ///
    /// ゴーストの読み直しで着せ替えを作り直した窓へ持ち越すのに使う。
    pub fn last_binds(&self, target: TargetId) -> Option<&BindSet> {
        let (_, binds, _) = self.targets.get(&target)?.last_show.as_ref()?;
        Some(binds)
    }

    /// surface を単体で画面の外に描いて返す。未登録の target は `None`。
    ///
    /// 着せ替え＝最後に表示したときの集合（一度も表示していなければ空）、アニメーション＝空。
    /// target の `composer` は使わず新しい [`Composer`] で計画を組む（`&self` のまま・スクラッチを
    /// 共有しない）。合成メモには入れない。描けなければ `ComposeError::Render`。無い ID で呼ぶと合成の層が
    /// 自分で `error!` を出すので、呼び手は先に [`Self::has_surface`] で確かめる。
    pub fn compose_alone(
        &self,
        target: TargetId,
        surface_id: u32,
    ) -> Option<Result<ComposedSurface, ComposeError>> {
        let t = self.targets.get(&target)?;
        let empty = BindSet::default();
        let binds = t.last_show.as_ref().map_or(&empty, |(_, binds, _)| binds);
        Some(t.draw_fresh(target, surface_id, binds, &PatternState::default()))
    }

    /// 最後に見せた絵（surface・着せ替え・アニメーションの段）を新しく描き直す。未登録・未表示は None。
    ///
    /// [`Self::last_shown`] が `Some((id, None))` を返す間の読み口。
    /// [`Self::compose_alone`] と同じく新しい [`Composer`] の計画を描き、合成メモには入れない。
    pub fn compose_last_shown(
        &self,
        target: TargetId,
    ) -> Option<Result<ComposedSurface, ComposeError>> {
        let t = self.targets.get(&target)?;
        let (surface_id, binds, pattern) = t.last_show.as_ref()?;
        Some(t.draw_fresh(target, *surface_id, binds, pattern))
    }
}

#[cfg(test)]
#[path = "snapshot_tests.rs"]
mod tests;
