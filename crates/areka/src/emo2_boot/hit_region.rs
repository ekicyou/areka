//! 当たり判定 I/O 契約 [`HitRegion`] とリゾルバ [`resolve_hit_region`]（scope→target→現サーフェス
//! id→純関数を束ねる結線層・design「HitRegionContract」/「Resolver」）。
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
}

/// (scope, 窓 client 物理 px) → 解決済み [`HitRegion`]（4.1–4.4）。
///
/// scope→target（`super::target_map::shell_target`）→現サーフェス id→純関数（`EmoPresenter::hit_region`
/// が内部で適用）を束ねる唯一の窓口。UI スレッド上で同期呼出可能（channel 化・非同期化は不要＝4.2）。
///
/// **shell 窓専用**（target 偶数）。balloon（奇数 target）は扱わない（design「Resolver」）。座標
/// `(x, y)` は当該 shell 窓の client 物理 px であり、k=1.0 契約によりサーフェス px と同一空間で照合される
/// （4.3）。
///
/// # Postconditions
/// - 常に [`HitRegion`] を返す（`Option<HitRegion>` にしない）。`scope` は入力をそのまま反映する
///   （写像しない・Invariants）。
/// - 現サーフェスが解決できない（未表示・`Hide`・未登録 target）場合は `region: None`（4.4/5.3）。
///
/// 解決された領域名（`&str`）は搬送のため所有形 `Option<String>` へ写す（`input-events` への受け渡しに
/// 必要な、本層で許容される唯一の割当）。
///
/// production 消費者（`input-events`＝W2・probe＝Task 4.1）が生えるまで dead_code 警告を明示抑止する
/// （[`HitRegion`] の doc 参照）。
#[allow(dead_code)]
pub fn resolve_hit_region(presenter: &EmoPresenter, scope: u32, x: i64, y: i64) -> HitRegion {
    let target = super::target_map::shell_target(scope);
    let region = presenter.hit_region(target, x, y).map(str::to_owned);
    HitRegion { scope, region }
}

#[cfg(test)]
mod tests {
    use super::*;
    use areka_emo_present::EmoPresenter;

    /// Testing Strategy 項目 17（4.4 の檻）: 未表示 scope。`EmoPresenter::new()` ＋ target 未 attach では
    /// 現サーフェス id が解決できず、`resolve_hit_region` は `HitRegion { scope, region: None }` を返す。
    /// GPU/表示なしで決定論的に成立する。scope→target 写像（`shell_target`）の正しさは `target_map.rs` の
    /// 既存テストが所有する＝本 spec で再テストしない（証明済み配線）。
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
                },
                "未表示 scope {scope} は region None（scope はそのまま反映・4.4）"
            );
        }
    }
}
