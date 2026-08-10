//! 当たり判定（`ClientHit`・`EmoPresenter::hit_region`・`EmoPresenter::hit_region_client`）——native
//! サーフェス px 直接照合と、窓 client 物理 px を実適用 k で縮約する正準判定入口。

use super::{EmoPresenter, RegionPriority, ScaleRatio, TargetId, hit_region_scaled};

/// 窓 client 物理 px の点に対する当たり判定結果（[`EmoPresenter::hit_region_client`] の戻り値）。
///
/// 所有権を持たない借用ビューであり、寿命は presenter の不変借用に従う（マウス移動ごとの割当を
/// 生まない）。フィールドは 2 つとも「同一の判定 1 回」から生まれた対であり、呼び手は
/// **両者を分離して再計算してはならない**——[`surface_point`] は縮約の結果そのもの（唯一の生成点は
/// [`areka_emo_compose::hit_region_scaled`]、未表示縮退時のみ `hit_region_client` 内の直接呼出）で
/// あり、下流は横流しするのみである（二重縮約の構造的排除・design §Data Models 不変条件 (1)）。
///
/// [`surface_point`]: Self::surface_point
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientHit<'a> {
    /// 当たった領域名（無ければ `None`）。k=1.0 では [`EmoPresenter::hit_region`] と完全一致する。
    pub region: Option<&'a str>,
    /// 縮約後のサーフェス px 座標（作者定義空間）。SHIORI へ配信する「ローカル座標」の正準値（要件 1.8）。
    pub surface_point: (i64, i64),
}

impl EmoPresenter {
    /// 現サーフェスの当たり判定領域名を解決する（`current_surface_id` → `EmoWorld::surface` → 純関数・R4.1/4.4）。
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
    /// [`areka_emo_compose::hit_region`] の呼出は本メソッド内で閉じ、戻り値の寿命は `&self` に従う
    /// （マウス移動ごとの割当を生まない・design §CurrentSurfaceRead Service Interface）。
    pub fn hit_region(&self, target: TargetId, x: i64, y: i64) -> Option<&str> {
        let t = self.targets.get(&target)?;
        let master = t.emo_world.surface(t.current_surface_id?)?;
        areka_emo_compose::hit_region(master, x, y, RegionPriority::Painter)
    }

    /// 窓 client 物理 px の点を**実適用 k で縮約**して当たり判定を解決する（DPI 追従の正準判定入口・
    /// 要件 1.1/1.4-1.7/4.5）。
    ///
    /// [`Self::hit_region`] が native サーフェス px を受けるのに対し、本メソッドは **k 適用後の窓 client
    /// 物理 px**（`WM_MOUSEMOVE` 等が運ぶ生座標）をそのまま受ける。÷k は本メソッドが吸収するため、
    /// 呼び手が座標を前処理してはならない（前処理すると二重縮約になる）。戻り値の
    /// [`ClientHit::surface_point`] が SHIORI へ配信する「ローカル座標」の正準値である（要件 1.8）。
    ///
    /// # k の真実源（要件 1.4/1.7）
    ///
    /// k は私有 [`PresentTarget::applied`] の**直読のみ**で得る。f32 の出口ビュー [`Self::applied_scale`]
    /// を経由せず（丸めを持ち込まない）、[`derive_scale`] を再呼出もしない（モニタ DPI からの再導出は
    /// 「表示に実際に掛かった k」と食い違い得る）。判定のたびに読むためスナップショットを保持せず、
    /// 窓 DPI 変化で `applied` が更新されれば以後の判定は自動的に新しい k で行われる——旧 k による
    /// 判定は構造的に残らない（要件 1.7）。
    ///
    /// # 縮退（いずれも panic せず定義された結果を返す）
    ///
    /// - **現サーフェス無し**（未表示／`Hide`／空合成縮退／未登録 target）: `region` は `None`
    ///   （[`Self::hit_region`] の縮退と同一）。`surface_point` は有効 k（`applied` 不在なら
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
    /// # 観測（要件 4.5）
    ///
    /// k・縮約前座標・縮約後座標・解決 region を `debug!` 1 行の構造化出力で残す。実機サインオフは
    /// `RUST_LOG=areka_emo_present=debug` でこの 1 行を grep して決定論的に判定する。
    ///
    /// 縮約の丸め権威は [`ScaleRatio::unscale_coord`] ただ 1 本であり、本メソッドはその式を持たない
    /// （正常経路は [`areka_emo_compose::hit_region_scaled`] へ委譲・未表示縮退時のみ座標を得るために
    /// 直接呼ぶ）。`&self` のみを取り World・GPU に依存しないため、判定はマウス移動ごとに安全に呼べる。
    pub fn hit_region_client(&self, target: TargetId, x: i64, y: i64) -> ClientHit<'_> {
        // k の真実源は私有 `applied` の直読ただ 1 つ（f32 非経由・`derive_scale` 再呼出なし）。
        // 判定ごとに読むため k 更新へ自動追従する（スナップショットを持たない＝要件 1.7）。
        // 現サーフェスも同じ不変借用から引く（`region` が引けない縮退でも座標契約は保つ）。
        let (applied, master) = match self.targets.get(&target) {
            Some(t) => (
                t.applied,
                t.current_surface_id.and_then(|id| t.emo_world.surface(id)),
            ),
            // 未登録 target は正常縮退（判定対象が存在しない＝異常ではない）。
            None => (None, None),
        };

        let k = match (applied, master.is_some()) {
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

        let (region, surface_point) = match master {
            // 正常経路: 縮約＋照合を合成純関数へ完全委譲（÷k の式を本層に持たない）。
            Some(master) => {
                let hit = hit_region_scaled(master, x, y, k, RegionPriority::Painter);
                (hit.region, hit.surface_point)
            }
            // 未表示縮退: 照合先が無いので座標だけ丸め権威で縮約する（式は持たず権威を呼ぶ）。
            None => (None, (k.unscale_coord(x), k.unscale_coord(y))),
        };

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
