//! 当たり判定 I/O 契約 [`HitRegion`] とリゾルバ [`resolve_hit_region`]（scope→target→現サーフェス
//! id→純関数を束ねる結線層）。
//!
//! # 座標空間の正準契約
//!
//! areka は **DPI追従**（画面 DPI に追従してマスコットが k 倍で拡大表示される）を基本設計とする。ゆえに当たり判定経路には 2 つの座標空間が現れ、本
//! モジュールがその**境界**に立つ。次の 5 点が本経路の座標契約の全体像であり、`areka_input`
//! 側の記述は本節を参照する（正準はここ 1 箇所）。
//!
//! 1. **受け取る空間＝窓 client 物理 px**: [`resolve_hit_region`] の `(x, y)` は当該 shell 窓の
//!    client 物理 px（`WM_MOUSEMOVE` 等が運ぶ生座標・k 適用後の実表示画素空間）である。呼び手が
//!    座標を前処理してはならない（前処理は二重縮約になる）。
//! 2. **k が吸収される点＝presenter**: ÷k は [`EmoPresenter::hit_region_client`] が吸収する
//!    （実適用 k＝表示に実際に掛かった値を真実源として直読し、丸め権威
//!    `ScaleRatio::unscale_coord` 経由で縮約する）。本モジュールは縮約の**式を一切持たず**、
//!    結果を受け取って横流しするだけである。
//! 3. **変換を要しない層**: 送出間引き（`areka_input::throttle`）の位置比較は**縮約前の
//!    client px 空間のまま**である（縮約すると移動検出の実効粒度が k 倍粗くなる）。
//!    バルーン窓のヒット経路（`areka_input::balloon`）も点は**無変換が正しい**——あちらは行ヒット
//!    矩形側を実適用 k で窓物理 px へ持ち上げており（`to_window_physical`）、シェルが「点を ÷k」
//!    するのに対しバルーンは「矩形を ×k」する**逆向きだが等価に正しい**整合である。バルーン経路へ
//!    ÷k を足すと二重縮約になり、正常動作を壊す。
//! 4. **SHIORI へ配信する座標＝サーフェス px**: [`HitRegion::surface_point`] が正典の「ローカル
//!    座標」として配信される値であり、作者定義のサーフェス px 空間（＝当たり判定識別子と同一空間）
//!    に揃う。**正典（ukadoc）の `OnMouseMove` Reference0/1 は「ローカル座標」としか規定しておらず
//!    空間を定義していない**（SSP は k=1.0 恒等ゆえ二義性が生じない）ため、これは areka 側の裁定
//!    である。ゴーストへ表示スケール k を露出しないための選択でもある。
//! 5. **判定挙動が変わるのは shell 窓のみ**: DPI追従で変わる当たり判定挙動は shell 窓（本
//!    モジュールの経路）に限られる。バルーン窓の判定挙動は不変である。
//!
//! k=1.0（等倍）では縮約は厳密な恒等写像であり、受領座標・配信座標・判定結果のいずれも DPI追従
//! 導入前と完全に同一である。
//!
//! [`EmoPresenter::hit_region_client`]: areka_emo_present::EmoPresenter::hit_region_client
//!
//! # shell 窓専用（target 偶数）
//!
//! 本リゾルバは **shell 窓専用**（`super::target_map::shell_target` が返す偶数 target）であり、
//! balloon（奇数 target・`choice-render` の領分）は扱わない。正典 `OnMouseMove` の Reference に
//! バルーン識別子は存在せず、区別は areka 側の結線規律で担保する。
//!
//! probe（`examples/collision-probe.rs`）は `areka_runtime::hit_region`・`target_map` を crate の公開面で `use` する（`#[path]` では取り込まない）。

use areka_emo_present::EmoPresenter;
use bevy_ecs::world::World;

/// 当たり判定の解決結果（region/actor I/O 契約の**正本**）。
///
/// `region` は**不透明 String**であり、本層は意味解釈しない。非該当・現サーフェス無しは `None`。
/// `None` → 空文字 Reference4 の転写は `input-events` の責務であり、本層は行わない。
///
/// **shell 窓専用**（`resolve_hit_region` が `shell_target` の偶数 target のみを解決する）であり、
/// balloon（奇数 target・`choice-render` の領分）は扱わない。契約の定義箇所は本
/// モジュール1点であり、`input-events` は再定義せず本型を参照する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HitRegion {
    /// ゴースト scope（0=本体 / 1=相方 …）。型は `super::target_map` の正本（`u32`）へ揃える。
    pub scope: u32,
    /// 当たり判定の領域名（不透明・搬送のため所有形）。
    pub region: Option<String>,
    /// 縮約後のサーフェス px 座標＝SHIORI へ配信する「ローカル座標」の正準値（冒頭 doc 4）。
    ///
    /// [`region`](Self::region) と**同一の判定 1 回**から生まれた対であり、両者は同一空間に揃う。
    /// 生成点は presenter（`hit_region_client` 内の縮約ただ 1 箇所）であり、本層も下流も
    /// **横流しするのみで再縮約してはならない**（二重縮約の構造的排除）。
    pub surface_point: (i64, i64),
}

/// (scope, 窓 client 物理 px) → 解決済み [`HitRegion`]。
///
/// scope→target（`super::target_map::shell_target`）→presenter の判定入口
/// （[`EmoPresenter::hit_region_client`]）を束ねる唯一の窓口。UI スレッド上で同期呼出可能
/// （channel 化・非同期化は不要）。
///
/// **shell 窓専用**（target 偶数）。balloon（奇数 target）は扱わない。
///
/// `world` は描画面の入れ物を持つ World（名前の解決はクリックの透過と同じ 1 画素の α を読む）。
///
/// 座標 `(x, y)` は当該 shell 窓の **client 物理 px**（k 適用後の実表示画素空間）である。÷k 縮約は
/// presenter が吸収するため、本関数も呼び手も座標を前処理しない（前処理は二重縮約・冒頭 doc 1/2）。
/// k=1.0 では縮約が厳密な恒等写像となり、DPI追従導入前の挙動・期待値と完全に一致する。
///
/// # Postconditions
/// - 常に [`HitRegion`] を返す（`Option<HitRegion>` にしない）。`scope` は入力をそのまま反映する
///   （写像しない・Invariants）。
/// - 現サーフェスが解決できない（未表示・`Hide`・未登録 target）場合は `region: None`。
///   このとき `surface_point` は有効 k（不在なら等倍相当）で縮約した値であり、判定が無くても座標
///   空間の契約は保たれる（presenter 側の縮退規約をそのまま受ける）。
/// - `surface_point` は presenter が返した縮約結果をそのまま授受した値である（再縮約禁止）。
///
/// 解決された領域名（`&str`）は搬送のため所有形 `Option<String>` へ写す（`input-events` への受け渡しに
/// 必要な、本層で許容される唯一の割当）。`surface_point` は `Copy` ゆえ割当を生まない。
///
/// [`EmoPresenter::hit_region_client`]: areka_emo_present::EmoPresenter::hit_region_client
pub fn resolve_hit_region(
    world: &World,
    presenter: &EmoPresenter,
    scope: u32,
    x: i64,
    y: i64,
) -> HitRegion {
    let target = super::target_map::shell_target(scope);
    let hit = presenter.hit_region_client(world, target, x, y);
    HitRegion {
        scope,
        region: hit.region.map(str::to_owned),
        surface_point: hit.surface_point,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use areka_emo_present::EmoPresenter;
    use bevy_ecs::world::World;

    /// 未表示 scope。`EmoPresenter::new()` ＋ target 未 attach では
    /// 現サーフェス id が解決できず、`resolve_hit_region` は `region: None` を返す。GPU/表示なしで
    /// 決定論的に成立する。scope→target 写像（`shell_target`）の正しさは `target_map.rs` の既存テストが
    /// 所有する＝ここでは再テストしない。
    ///
    /// `surface_point` も併せて固定する——未 attach target には適用 k が存在しないため等倍相当
    /// （縮約は恒等）で縮退し、受領した client 物理 px がそのまま配信空間の値になる。
    #[test]
    fn unshown_scope_resolves_region_none() {
        let presenter = EmoPresenter::new();
        let world = World::new();
        // scope・座標は任意（未 attach ゆえ現サーフェス無し→region None）。scope は入力をそのまま反映する。
        for scope in [0u32, 1, 7] {
            let got = resolve_hit_region(&world, &presenter, scope, 100, 100);
            assert_eq!(
                got,
                HitRegion {
                    scope,
                    region: None,
                    surface_point: (100, 100),
                },
                "未表示 scope {scope} は region None・座標は等倍縮退（scope はそのまま反映）"
            );
        }
    }
}
