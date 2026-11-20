//! 当たり判定（`ClientHit`・`EmoPresenter::hit_region`・`EmoPresenter::hit_region_client`）——native
//! サーフェス px 直接照合と、窓 client 物理 px を実適用 k で縮約し α → ID → 名前で引く正準判定入口。

use super::{EmoPresenter, ScaleRatio, TargetId};
#[cfg(test)]
use areka_emo_compose::{RegionPriority, hit::hit_region_in};
use areka_parsers::shell::Collision;
use wintf::ecs::IdRead;

use super::target::PresentTarget;

/// 窓 client 物理 px の点に対する当たり判定結果（[`EmoPresenter::hit_region_client`] の戻り値）。
///
/// 所有権を持たない借用ビューであり、寿命は presenter の不変借用に従う（マウス移動ごとの割当を
/// 生まない）。フィールドは 2 つとも「同一の判定 1 回」から生まれた対であり、呼び手は
/// **両者を分離して再計算してはならない**——[`surface_point`] は縮約の結果そのもの（唯一の生成点は
/// `hit_region_client` 内の [`ScaleRatio::unscale_coord`] の呼出）であり、下流は横流しするのみである（二重縮約の構造的排除・design §Data Models 不変条件 (1)）。
///
/// [`surface_point`]: Self::surface_point
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientHit<'a> {
    /// 当たった領域名（無ければ `None`）。
    pub region: Option<&'a str>,
    /// 縮約後のサーフェス px 座標（作者定義空間）。SHIORI へ配信する「ローカル座標」の正準値（要件 1.8）。
    pub surface_point: (i64, i64),
}

impl EmoPresenter {
    /// 現サーフェスの当たり判定領域名を解決する（`current_surface_id` → `EmoWorld::hit_regions` → 純関数・R4.1/4.4）。
    ///
    /// 座標は **native サーフェス px**（k 適用前の合成座標系）で解釈される。窓 client 物理 px は k 倍
    /// された座標系ゆえ、k≠1.0 では呼び手が渡す前に ÷k する必要がある——**その変換は本メソッドの責務
    /// ではない**。÷k を吸収する**正準の呼び手**は姉妹メソッド [`Self::hit_region_client`] であり、
    /// `areka-P0-collision-dpi-hittest`（W5）で**実装済み**である。production の判定入口はそちらであって
    /// 本メソッドではない——本メソッドを窓 client 物理 px で直接呼ぶと k≠1.0 で当たり判定がずれる。
    /// k=1.0 の窓では両座標系が一致するため、本メソッドの挙動は k 導入の前後で完全に不変である
    /// （[`Self::hit_region_client`] の `region` とも k=1.0 で完全一致する）。現サーフェス無し（未表示／
    /// `Hide`／空合成
    /// 縮退／未登録 target）は `None`（R4.4）。重なりは画家のアルゴリズム（後定義が手前・[`RegionPriority::Painter`]）で
    /// 解決する。`EmoWorld` を presenter 外へ露出しない（`&SurfaceMaster` を外へ出さない）ため純関数
    /// [`areka_emo_compose::hit::hit_region_in`] の呼出は本メソッド内で閉じ、戻り値の寿命は `&self` に従う
    /// （マウス移動ごとの割当を生まない・design §CurrentSurfaceRead Service Interface）。
    /// 本番の呼び手は無いのでテスト専用（native 座標の照合の檻だけが使う）。
    #[cfg(test)]
    pub(crate) fn hit_region(&self, target: TargetId, x: i64, y: i64) -> Option<&str> {
        let t = self.targets.get(&target)?;
        // 子から持ち込んだ領域が在ればその列（surface-element-nesting 要件 4.1・4.2）。
        let regions = t.emo_world.hit_regions(t.current_surface_id?)?;
        hit_region_in(regions, x, y, RegionPriority::Painter)
    }

    /// 窓 client 物理 px の点を**実適用 k で縮約**して当たり判定を解決する（DPI 追従の正準判定入口・
    /// 要件 1.1/1.4-1.7/4.5）。
    ///
    /// `Self::hit_region`（テスト専用）が native サーフェス px を受けるのに対し、本メソッドは **k 適用後の窓 client
    /// 物理 px**（`WM_MOUSEMOVE` 等が運ぶ生座標）をそのまま受ける。÷k は本メソッドが吸収するため、
    /// 呼び手が座標を前処理してはならない（前処理すると二重縮約になる）。戻り値の
    /// [`ClientHit::surface_point`] が SHIORI へ配信する「ローカル座標」の正準値である（要件 1.8）。
    ///
    /// # k の真実源（要件 1.4/1.7）
    ///
    /// k は私有 `PresentTarget::applied` の**直読のみ**で得る。f32 の出口ビュー [`Self::applied_scale`]
    /// を経由せず（丸めを持ち込まない）、`derive_scale` を再呼出もしない（モニタ DPI からの再導出は
    /// 「表示に実際に掛かった k」と食い違い得る）。判定のたびに読むためスナップショットを保持せず、
    /// 窓 DPI 変化で `applied` が更新されれば以後の判定は自動的に新しい k で行われる——旧 k による
    /// 判定は構造的に残らない（要件 1.7）。
    ///
    /// # 縮退（いずれも panic せず定義された結果を返す）
    ///
    /// - **現サーフェス無し**（未表示／`Hide`／空合成縮退／未登録 target）: `region` は `None`
    ///   （`Self::hit_region` の縮退と同一）。`surface_point` は有効 k（`applied` 不在なら
    ///   [`ScaleRatio::ONE`]）で縮約した値を返す——判定が無くても座標空間の契約は保つ。これは
    ///   **正常な縮退**であり `warn!` を出さない（未表示 scope 上のマウス移動ごとに鳴らさない）。
    /// - **面はあるのに `applied` が不在**（k 取得不能）: `warn!` を 1 行記録したうえで
    ///   [`ScaleRatio::ONE`]（＝縮約なし）で照合を続行し、当たり判定そのものを失わせない
    ///   （要件 1.6・ログ無し失敗経路の禁止）。これは**現行の公開 API 経由では到達不能な防御分岐**
    ///   である——`applied` と現サーフェス（`current_surface_id`・`emo_world` の面）は同じ表示成立点
    ///   1 箇所で確定するため、「面はあるのに k が無い」状態を外から作れない。到達し得るのは presenter の
    ///   内部不変条件が破れた場合のみであり、その事実こそが `warn!` の伝える情報である。ゆえに警告は
    ///   上の正常縮退とは**明確に別の事象**であり、両者を同じ分岐にまとめてはならない。
    ///
    /// # 名前の引き方
    ///
    /// 縮約した点で表示中の絵の α マスクが立っていない（面の外を含む）・当たり判定の並びが空なら
    /// `None`。そうでなければ表示の成立点で塗った ID の描き先を 1 点読み、ID を並びの位置へ戻して
    /// 名前を返す。描き先が読めない・無い・別のサーフェスを塗ったままのときは `None` にして
    /// `warn!` を 1 件だけ残し、塗り直しが成功するまで繰り返さない。
    ///
    /// # 観測（要件 4.5）
    ///
    /// k・縮約前座標・縮約後座標・解決 region を `debug!` 1 行の構造化出力で残す。実機サインオフは
    /// `RUST_LOG=areka_emo_present=debug` でこの 1 行を grep して決定論的に判定する。
    ///
    /// 縮約の丸め権威は [`ScaleRatio::unscale_coord`] ただ 1 本であり、本メソッドはその式を持たない。
    /// `&self` のみを取り World に依存しない。GPU を読むのは α が立つ点だけで、同じ点の読み直しは
    /// 描き先の控えが答える。
    pub fn hit_region_client(&self, target: TargetId, x: i64, y: i64) -> ClientHit<'_> {
        // k の真実源は私有 `applied` の直読ただ 1 つ（f32 非経由・`derive_scale` 再呼出なし）。
        // 判定ごとに読むため k 更新へ自動追従する（スナップショットを持たない＝要件 1.7）。
        // 現サーフェスも同じ不変借用から引く（`region` が引けない縮退でも座標契約は保つ）。
        // 判定の列は子から持ち込んだ領域が在ればその列（surface-element-nesting 要件 4.1・4.2）。
        let t = self.targets.get(&target);
        // 未登録 target は正常縮退（判定対象が存在しない＝異常ではない）。
        let applied = t.and_then(|t| t.applied);
        let regions = t.and_then(|t| {
            t.current_surface_id
                .and_then(|id| t.emo_world.hit_regions(id))
        });

        let k = match (applied, regions.is_some()) {
            (Some(k), _) => k,
            // 正常縮退（未登録／未表示）: k が無いのは当然ゆえ鳴らさない（マウス移動ごとの警告を作らない）。
            (None, false) => ScaleRatio::ONE,
            // 要件 1.6: 面はあるのに k が無い＝内部不変条件の破れ。黙って 1.0 へ倒さず必ず鳴らす。
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
        let region = t
            .zip(regions)
            .and_then(|(t, regions)| t.resolve_hit(target, regions, surface_point));

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
    fn resolve_hit<'a>(
        &self,
        target: TargetId,
        regions: &'a [Collision],
        (sx, sy): (i64, i64),
    ) -> Option<&'a str> {
        let (Ok(px), Ok(py)) = (u32::try_from(sx), u32::try_from(sy)) else {
            return None;
        };
        if !self.shown_mask.as_ref().is_some_and(|m| m.is_hit(px, py)) || regions.is_empty() {
            return None;
        }
        let picture = self
            .hit_picture
            .as_ref()
            .filter(|p| Some(p.surface_id) == self.current_surface_id);
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
}

#[cfg(test)]
#[path = "../presenter_nesting_hit_tests.rs"]
mod nesting_hit_tests;
