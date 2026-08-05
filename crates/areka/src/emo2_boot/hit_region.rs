//! 当たり判定 I/O 契約 [`HitRegion`] とリゾルバ [`resolve_hit_region`]（scope→target→現サーフェス
//! id→純関数を束ねる結線層・design「HitRegionContract」/「Resolver」）。
//!
//! # 座標空間の正準契約（areka-P0-collision-dpi-hittest・要件 5.4）
//!
//! areka は **DPI追従**（画面 DPI に追従してマスコットが k 倍で拡大表示される）を基本設計とする
//! （[[areka-dpi-following-core-design]]）。ゆえに当たり判定経路には 2 つの座標空間が現れ、本
//! モジュールがその**境界**に立つ。次の 5 点が本経路の座標契約の全体像であり、`input_events`
//! 側の記述は本節を参照する（正準はここ 1 箇所）。
//!
//! 1. **受け取る空間＝窓 client 物理 px**: [`resolve_hit_region`] の `(x, y)` は当該 shell 窓の
//!    client 物理 px（`WM_MOUSEMOVE` 等が運ぶ生座標・k 適用後の実表示画素空間）である。呼び手が
//!    座標を前処理してはならない（前処理は二重縮約になる）。
//! 2. **k が吸収される点＝presenter**: ÷k は [`EmoPresenter::hit_region_client`] が吸収する
//!    （実適用 k＝表示に実際に掛かった値を真実源として直読し、丸め権威
//!    `ScaleRatio::unscale_coord` 経由で縮約する）。本モジュールは縮約の**式を一切持たず**、
//!    結果を受け取って横流しするだけである。
//! 3. **変換を要しない層**: 送出間引き（`input_events::throttle`）の位置比較は**縮約前の
//!    client px 空間のまま**である（縮約すると移動検出の実効粒度が k 倍粗くなる・要件 6.8）。
//!    バルーン窓のヒット経路（`input_events::balloon`）も点は**無変換が正しい**——あちらは行ヒット
//!    矩形側を実適用 k で窓物理 px へ持ち上げており（`to_window_physical`）、シェルが「点を ÷k」
//!    するのに対しバルーンは「矩形を ×k」する**逆向きだが等価に正しい**整合である。バルーン経路へ
//!    ÷k を足すと二重縮約になり、正常動作を壊す。
//! 4. **SHIORI へ配信する座標＝サーフェス px**: [`HitRegion::surface_point`] が正典の「ローカル
//!    座標」として配信される値であり、作者定義のサーフェス px 空間（＝当たり判定識別子と同一空間）
//!    に揃う。**正典（ukadoc）の `OnMouseMove` Reference0/1 は「ローカル座標」としか規定しておらず
//!    空間を定義していない**（SSP は k=1.0 恒等ゆえ二義性が生じない）ため、これは areka 側の裁定
//!    である（要件 1.8）。ゴーストへ表示スケール k を露出しないための選択でもある。
//! 5. **判定挙動が変わるのは shell 窓のみ**: 本 spec が変更する当たり判定挙動は shell 窓（本
//!    モジュールの経路）に限られる。バルーン窓の判定挙動は不変であり、加えたのは理由説明の是正と
//!    退行検出の檻のみである（要件 5.5・6.4）。
//!
//! k=1.0（等倍）では縮約は厳密な恒等写像であり、受領座標・配信座標・判定結果のいずれも DPI追従
//! 導入前と完全に同一である（要件 1.5/1.9）。
//!
//! [`EmoPresenter::hit_region_client`]: areka_emo_present::EmoPresenter::hit_region_client
//!
//! # shell 窓専用（target 偶数）
//!
//! 本リゾルバは **shell 窓専用**（`super::target_map::shell_target` が返す偶数 target）であり、
//! balloon（奇数 target・`choice-render` の領分）は扱わない。正典 `OnMouseMove` の Reference に
//! バルーン識別子は存在せず、区別は areka 側の結線規律で担保する（design「HitRegionContract」・
//! 研究 §10.5）。
//!
//! # `crate::` フリー規律（この配置の決定的制約）
//!
//! **非テストコードは `crate::` パスを一切使わず、`super::target_map` と外部 crate
//! （`areka_emo_present`）のみを参照する。** この規律が probe（`examples/collision-probe.rs`・
//! Task 4.1）からの `#[path]` include を成立させる唯一の条件である（design「File Structure Plan →
//! 配置の決定的制約」）。同じ性質に依存する前例が `examples/window-placement.rs`
//! （`#[path = "../src/placement/mod.rs"]` include・`crate::` パス無しゆえ成立）である。
//!
//! これを破ると `collision-probe.rs`（ひいては `cargo build --examples`／`cargo test --workspace`）が
//! コンパイル不能となり、7.3 の実 DPI 証跡取得経路が失われる。`emo2_boot/mod.rs` 自体は非テスト
//! コードに `crate::is_benign_boot_error` の実呼出を持つため `#[path]` include 不能＝だから本ファイルは
//! `crate::` フリーでなければならない。`#[cfg(test)]` 内では `crate::`／`super::` を自由に使ってよい。

use areka_emo_present::EmoPresenter;

/// 当たり判定の解決結果（region/actor I/O 契約の**正本**・5.1/5.4）。
///
/// `region` は**不透明 String**であり、本層は意味解釈しない（5.2）。非該当・現サーフェス無しは
/// `None`（5.3・4.4）。`None` → 空文字 Reference4 の転写は `input-events` の責務であり、本層は行わない
/// （design「HitRegionContract」）。
///
/// **shell 窓専用**（`resolve_hit_region` が `shell_target` の偶数 target のみを解決する）であり、
/// balloon（奇数 target・`choice-render` の領分）は扱わない（研究 §10.5）。契約の定義箇所は本
/// モジュール1点であり、`input-events` は再定義せず本型を参照する（Coordination Notes C-1）。
///
/// 本 spec 内に production 消費者は未だ居ない（第一消費者は `input-events`＝roadmap W2、加えて
/// probe＝`examples/collision-probe.rs`・Task 4.1）。`areka` は bin crate ゆえ未使用 `pub` 項目は
/// dead_code 警告になる（baseline は警告皆無）。W2 の実配線で消費点が生えるまで `#[allow(dead_code)]`
/// で明示的に抑止する（消費者が生えたら除去可）。
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HitRegion {
    /// ゴースト scope（0=本体 / 1=相方 …）。型は `super::target_map` の正本（`u32`）へ揃える（研究 §10.5）。
    pub scope: u32,
    /// 当たり判定の領域名（不透明・搬送のため所有形）。
    pub region: Option<String>,
    /// 縮約後のサーフェス px 座標＝SHIORI へ配信する「ローカル座標」の正準値（要件 1.8・冒頭 doc 4）。
    ///
    /// [`region`](Self::region) と**同一の判定 1 回**から生まれた対であり、両者は同一空間に揃う。
    /// 生成点は presenter（`hit_region_client` 内の縮約ただ 1 箇所）であり、本層も下流も
    /// **横流しするのみで再縮約してはならない**（二重縮約の構造的排除・design §Data Models 不変条件 (1)）。
    pub surface_point: (i64, i64),
}

/// (scope, 窓 client 物理 px) → 解決済み [`HitRegion`]（4.1–4.4）。
///
/// scope→target（`super::target_map::shell_target`）→presenter の判定入口
/// （[`EmoPresenter::hit_region_client`]）を束ねる唯一の窓口。UI スレッド上で同期呼出可能
/// （channel 化・非同期化は不要＝4.2）。
///
/// **shell 窓専用**（target 偶数）。balloon（奇数 target）は扱わない（design「Resolver」）。
///
/// 座標 `(x, y)` は当該 shell 窓の **client 物理 px**（k 適用後の実表示画素空間）である。÷k 縮約は
/// presenter が吸収するため、本関数も呼び手も座標を前処理しない（前処理は二重縮約・冒頭 doc 1/2）。
/// k=1.0 では縮約が厳密な恒等写像となり、DPI追従導入前の挙動・期待値と完全に一致する（要件 1.5）。
///
/// # Postconditions
/// - 常に [`HitRegion`] を返す（`Option<HitRegion>` にしない）。`scope` は入力をそのまま反映する
///   （写像しない・Invariants）。
/// - 現サーフェスが解決できない（未表示・`Hide`・未登録 target）場合は `region: None`（4.4/5.3）。
///   このとき `surface_point` は有効 k（不在なら等倍相当）で縮約した値であり、判定が無くても座標
///   空間の契約は保たれる（presenter 側の縮退規約をそのまま受ける）。
/// - `surface_point` は presenter が返した縮約結果をそのまま授受した値である（要件 1.8・再縮約禁止）。
///
/// 解決された領域名（`&str`）は搬送のため所有形 `Option<String>` へ写す（`input-events` への受け渡しに
/// 必要な、本層で許容される唯一の割当）。`surface_point` は `Copy` ゆえ割当を生まない。
///
/// production 消費者（`input-events`＝W2・probe＝Task 4.1）が生えるまで dead_code 警告を明示抑止する
/// （[`HitRegion`] の doc 参照）。
///
/// [`EmoPresenter::hit_region_client`]: areka_emo_present::EmoPresenter::hit_region_client
#[allow(dead_code)]
pub fn resolve_hit_region(presenter: &EmoPresenter, scope: u32, x: i64, y: i64) -> HitRegion {
    let target = super::target_map::shell_target(scope);
    // ÷k は presenter 側（`hit_region_client`）で吸収済み。本層は region を所有形へ写し、
    // 縮約後座標は横流しするのみ（再縮約しない＝二重縮約の構造的排除）。
    let hit = presenter.hit_region_client(target, x, y);
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

    /// Testing Strategy 項目 17（4.4 の檻）: 未表示 scope。`EmoPresenter::new()` ＋ target 未 attach では
    /// 現サーフェス id が解決できず、`resolve_hit_region` は `region: None` を返す。GPU/表示なしで
    /// 決定論的に成立する。scope→target 写像（`shell_target`）の正しさは `target_map.rs` の既存テストが
    /// 所有する＝本 spec で再テストしない（証明済み配線）。
    ///
    /// DPI追従（areka-P0-collision-dpi-hittest）以後は `surface_point` も併せて固定する——未 attach
    /// target には適用 k が存在しないため等倍相当（縮約は恒等）で縮退し、受領した client 物理 px が
    /// そのまま配信空間の値になる（要件 1.6 の縮退規約と同根）。
    #[test]
    fn unshown_scope_resolves_region_none() {
        let presenter = EmoPresenter::new();
        // scope・座標は任意（未 attach ゆえ現サーフェス無し→region None）。scope は入力をそのまま反映する。
        for scope in [0u32, 1, 7] {
            let got = resolve_hit_region(&presenter, scope, 100, 100);
            assert_eq!(
                got,
                HitRegion {
                    scope,
                    region: None,
                    surface_point: (100, 100),
                },
                "未表示 scope {scope} は region None・座標は等倍縮退（scope はそのまま反映・4.4）"
            );
        }
    }

    /// surface_point 伝播檻（1.8・5.4）: `hit_region_client` が返した縮約後サーフェス px を
    /// `HitRegion.surface_point` がそのまま授受することを固定する。
    #[test]
    fn resolver_propagates_surface_point_from_client_hit() {
        let presenter = EmoPresenter::new();
        for (scope, x, y, expect) in [
            (0u32, 100i64, 100i64, (100i64, 100i64)),
            (1, 0, 0, (0, 0)),
            (7, -13, 4321, (-13, 4321)),
        ] {
            let got = resolve_hit_region(&presenter, scope, x, y);
            assert_eq!(
                got.surface_point, expect,
                "未表示 target の k は ONE 相当＝縮約は恒等（scope {scope}）"
            );
        }
    }
}
