//! 当たり判定（`ClientHit`・`EmoPresenter::hit_region_client`）——窓 client 物理 px の入れ物の α を読み、
//! 実適用 k で縮約した点の ID → 名前で引く判定入口。

use super::{EmoPresenter, ScaleRatio, TargetId, World};
use wintf::ecs::{IdRead, surface_alpha_hits};

use super::target::PresentTarget;

/// 窓 client 物理 px の点に対する当たり判定結果（[`EmoPresenter::hit_region_client`] の戻り値）。
///
/// 所有権を持たない借用ビュー（マウス移動ごとの割当を生まない）。2 フィールドは同一の判定 1 回から
/// 生まれた対で、呼び手は分離して再計算してはならない。[`surface_point`] は縮約の結果そのもので、
/// 下流は横流しするのみ（縮約を二重に掛けない）。
///
/// [`surface_point`]: Self::surface_point
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientHit<'a> {
    /// 当たった領域名（無ければ `None`）。
    pub region: Option<&'a str>,
    /// 縮約後のサーフェス px 座標（作者定義空間）。SHIORI へ配信する「ローカル座標」の正準値。
    pub surface_point: (i64, i64),
}

impl EmoPresenter {
    /// 窓 client 物理 px の点を**実適用 k で縮約**して当たり判定を解決する（DPI 追従の正準判定入口）。
    ///
    /// k 適用後の生座標（`WM_MOUSEMOVE` 等）をそのまま受ける。÷k は本メソッドが吸収するので、呼び手が
    /// 前処理すると二重縮約になる。戻り値の [`ClientHit::surface_point`] が SHIORI へ配信する
    /// 「ローカル座標」の正準値。縮約の丸め権威は [`ScaleRatio::unscale_coord`] ただ 1 本。
    ///
    /// # k の真実源
    ///
    /// 私有 `PresentTarget::applied` の直読のみ。f32 の出口ビュー [`Self::applied_scale`] は丸めを
    /// 持ち込むので経由せず、`derive_scale` の再呼出もしない（モニタ DPI からの再導出は表示に実際に
    /// 掛かった k と食い違い得る）。判定のたびに読むので、窓 DPI 変化後は自動的に新しい k になる。
    ///
    /// # 縮退（いずれも panic せず定義された結果を返す）
    ///
    /// - 現サーフェス無し（未表示／`Hide`／空合成縮退／未登録 target）: `region` は `None`。
    ///   `surface_point` は有効 k（`applied` 不在なら [`ScaleRatio::ONE`]）で縮約した値を返す。これは
    ///   正常な縮退で、`warn!` は出さない（未表示 scope 上のマウス移動ごとに鳴らさない）。
    /// - 面はあるのに `applied` が不在: `warn!` を 1 行記録し、[`ScaleRatio::ONE`]（縮約なし）で照合を
    ///   続行する。`applied` と現サーフェスは同じ表示成立点 1 箇所で確定するので公開 API 経由では
    ///   到達不能な防御分岐で、到達するのは presenter の内部不変条件が破れたときだけ。上の正常縮退とは
    ///   別の事象なので同じ分岐にまとめてはならない。
    ///
    /// # 名前の引き方
    ///
    /// 今の土台の面（表示中の計画の `base_surface`）の当たり判定の並びが空なら `None`。そうでなければ
    /// クリックの透過と同じ 1 画素——`world` の描画面の入れ物の、client 物理 px `(x, y)` の α——を読み、
    /// 128 未満・面の外・読めないなら `None`（読めないはクリックの透過と合わせて読めない間に 1 回だけ
    /// wintf が `error!`）。それ以上なら縮約した点で表示の成立点で塗った ID の描き先を 1 点読み、ID を並びの位置へ戻して名前を返す。
    /// 描き先が読めない・無い・別の面を塗ったままのときは `None` にして `warn!` を 1 件だけ残し、塗り直しが
    /// 成功するまで繰り返さない。同じ点の読み直しは入れ物と描き先の控えが答える。
    ///
    /// # 観測
    ///
    /// k・縮約前座標・縮約後座標・解決 region を `debug!` 1 行で残す。実機サインオフは
    /// `RUST_LOG=areka_emo_present=debug` でこの行を grep して判定する。
    pub fn hit_region_client(
        &self,
        world: &World,
        target: TargetId,
        x: i64,
        y: i64,
    ) -> ClientHit<'_> {
        let t = self.targets.get(&target);
        // 未登録 target は正常縮退。
        let applied = t.and_then(|t| t.applied);
        let shown = t.is_some_and(|t| t.current_surface_id.is_some());

        let k = match (applied, shown) {
            (Some(k), _) => k,
            // 正常縮退（未登録／未表示）は鳴らさない。
            (None, false) => ScaleRatio::ONE,
            // 面はあるのに k が無い＝内部不変条件の破れ。黙って 1.0 へ倒さず必ず鳴らす。
            (None, true) => {
                tracing::warn!(
                    ?target,
                    client_x = x,
                    client_y = y,
                    "[hit_region_client] 表示中サーフェスがあるのに適用スケール未確定（applied 不在）——k=1.0 相当で照合を続行"
                );
                ScaleRatio::ONE
            }
        };

        let surface_point = (k.unscale_coord(x), k.unscale_coord(y));
        let region = t.and_then(|t| t.resolve_hit(world, target, (x, y), surface_point));

        tracing::debug!(
            ?target,
            // k の有理表現（既約 num/den）。`ScaleRatio` の num/den は非公開ゆえ `Debug` で出す。
            k_ratio = ?k,
            client_x = x,
            client_y = y,
            surface_x = surface_point.0,
            surface_y = surface_point.1,
            region = ?region,
            "[hit_region_client] client 物理 px を ÷k して当たり判定を解決"
        );

        ClientHit {
            region,
            surface_point,
        }
    }
}

impl PresentTarget {
    fn resolve_hit(
        &self,
        world: &World,
        target: TargetId,
        (cx, cy): (i64, i64),
        (sx, sy): (i64, i64),
    ) -> Option<&str> {
        let current = self.current_surface_id?;
        let base = self.shown_base?;
        let regions = self.emo_world.hit_regions(base).filter(|r| !r.is_empty())?;
        if !self.alpha_hits(world, cx, cy) {
            return None;
        }
        let (Ok(px), Ok(py)) = (u32::try_from(sx), u32::try_from(sy)) else {
            return None;
        };
        let picture = self
            .hit_picture
            .as_ref()
            .filter(|p| p.surface_id == current && p.base_surface == base);
        let read = picture.map_or(IdRead::Unreadable, |p| p.target.read(px, py));
        match (read, picture) {
            (IdRead::Id(id), Some(picture)) => {
                let index = picture.first + id.get() as usize - 1;
                let name = regions.get(index).map(|c| c.name.as_str());
                if name.is_none() {
                    tracing::error!(
                        ?target,
                        surface_id = picture.surface_id,
                        index,
                        regions = regions.len(),
                        "[hit_region_client] 描き先の ID が当たり判定の並びの外"
                    );
                }
                name
            }
            (IdRead::Empty | IdRead::Outside, _) => None,
            _ => {
                if !self.hit_notes.unreadable.replace(true) {
                    tracing::warn!(
                        ?target,
                        surface_id = ?self.current_surface_id,
                        painted = ?self.hit_picture.as_ref().map(|p| p.surface_id),
                        "[hit_region_client] 当たり判定の描き先が読めない（塗り直しまで当たり無し）"
                    );
                }
                None
            }
        }
    }

    /// クリックの透過と同じ答え（[`surface_alpha_hits`]）を、入れ物の client 物理 px で。
    fn alpha_hits(&self, world: &World, x: i64, y: i64) -> bool {
        match (self.mount.as_ref(), u32::try_from(x), u32::try_from(y)) {
            (Some(mount), Ok(x), Ok(y)) => surface_alpha_hits(world, mount.surface_entity(), x, y),
            _ => false,
        }
    }
}

#[cfg(test)]
#[path = "../presenter_nesting_hit_tests.rs"]
mod nesting_hit_tests;
