//! 装着の置き換え（シェル・バルーンの差し替え・`areka-P0-shell-balloon-switch` 要件 4.6）。
//!
//! 「登録を消す口」[`EmoPresenter::detach_target`] を持つ。表から target を外し、装着の子 2 つを
//! 消して、新しい装着へ引き継ぐ値（[`DetachedState`]）を返す。本番の呼び手は置き換えの手順だけで、
//! 登録を消したまま戻さない経路は作らない。

use super::{EmoPresenter, Entity, PresentError, ScaleRatio, TargetId, VisibilityOwnership, World};

/// 登録を消したときに、新しい装着へ引き継ぐ値。
///
/// 合成の入力（`EmoWorld`・アトラス・キャッシュ）と拡大政策は引き継がない——新しいシェル・
/// バルーンの資産と作者 DPI から作り直すものだからである。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DetachedState {
    /// 装着先の窓（差し替えの前後で同じ窓を使う）。
    pub window: Entity,
    /// 可視性の持ち主（バルーン窓は `External` のまま引き継ぐ）。
    pub ownership: VisibilityOwnership,
    /// 実際に表示へ適用していた k（表示が一度も成立していなければ `None`）。
    pub applied: Option<ScaleRatio>,
    /// 表示中サーフェスの素の大きさ（k 適用前）。
    pub native_size: Option<(u32, u32)>,
    /// まだ取り出されていない窓寸の要求（引き継がないと窓寸合わせが失われる）。
    pub pending_resize: Option<(u32, u32)>,
}

impl EmoPresenter {
    /// target の登録を消し、装着の子 2 つ（面と文字層スロット）を窓から消して、引き継ぐ値を返す。
    ///
    /// 未登録なら `error!` を残して `Err(TargetNotAttached)` を返し、表は変えない。一度も表示して
    /// いない target（装着の子が無い）も外せる。
    pub fn detach_target(
        &mut self,
        world: &mut World,
        target: TargetId,
    ) -> Result<DetachedState, PresentError> {
        let Some(old) = self.targets.remove(&target) else {
            tracing::error!(?target, "detach_target: 未装着ターゲット");
            return Err(PresentError::TargetNotAttached(target));
        };
        let had_mount = old.mount.is_some();
        if let Some(mount) = old.mount {
            mount.despawn(world);
        }
        tracing::debug!(?target, had_mount, "detach_target: 登録と装着の子を消した");
        Ok(DetachedState {
            window: old.window,
            ownership: old.ownership,
            applied: old.applied,
            native_size: old.native_size,
            pending_resize: old.pending_resize,
        })
    }
}
