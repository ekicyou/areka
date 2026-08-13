//! `windowposition`（数値指定）→ バルーン窓の初期既定位置の調整量（純関数群）。
//!
//! バルーン定義（`descript.txt` ＋ 面別上書き `{接頭辞}{ID}s.txt` の 2 層マージ結果）の
//! `windowposition` を、画面座標系（物理 px）の調整量へ変換し
//! [`PlacementConfig`] の `balloon_offset` 供給欄へ合流させる（R3.2/3.3/3.4/3.6）。
//!
//! 本モジュールは `resolver.rs` の配置式 P1〜P5 を**無改変**に保つための供給層である
//! ——P5 は `balloon_offset.unwrap_or((0, 0))` を既に加算しているため、供給元を足すだけで
//! 正典化が成立する（design D1'）。恒等式 `balloon_offset ≡ balloon_pos − char_pos`
//! （`resolver.rs` の恒久事後条件）は P5 の加算入力が変わるだけなので自動的に保たれる。
//!
//! 依存規約（placement/mod.rs）: `config`（`PlacementConfig`）と
//! `areka-emo-compose`（`ScaleRatio` 丸め権威）のみを消費し、wintf/bevy_ecs を import しない。
//!
//! `windowposition.x` の符号規約は実機（SSP 参照実装）で確定済みであり、バルーンの
//! 置き側（`BalloonSide`）に依存しない——ゆえに本モジュールは `BalloonSide` を受け取らない
//! （[`to_screen_adjust`] の「符号規約」節・R7.6／task 6.1）。

use areka_emo_compose::ScaleRatio;
use tracing::warn;

use super::config::PlacementConfig;

/// 符号付き長さの k 倍（**大きさは既存丸め権威 [`ScaleRatio::scale_len`] へ委譲**し、
/// 符号だけを本層で保存する・R3.6）。
///
/// 本仕様は新しい丸め規約を導入しない——`scale_len` の規約（round half away from zero・
/// 非ゼロ長は最小 1px・恒等 k は素通し）をそのまま継承する。
///
/// `i32::MIN` は [`i32::unsigned_abs`] で絶対値を取るためパニックしない。`scale_len` は
/// `u32::MAX` へ飽和するのみゆえ、物理 px 通貨（i32）への収め直しは本層の責務であり
/// `±i32::MAX` へ飽和させる（ラップアラウンドしない）。
pub(crate) fn scale_signed(v: i32, k: ScaleRatio) -> i32 {
    let scaled = k.scale_len(v.unsigned_abs());
    let magnitude = i32::try_from(scaled).unwrap_or(i32::MAX);
    if v < 0 { -magnitude } else { magnitude }
}

/// windowposition 生値（作者空間・model 由来）→ 画面座標の調整量（物理 px）。
///
/// - x: **素の画面座標オフセット**（正＝画面右／負＝画面左）。バルーンがキャラ窓の
///   どちら側に置かれるかに依らず反転しない（R3.3 の実機確定形・下記「符号規約」）。
/// - y: 下が正＝画面座標系と同符号ゆえ変換しない（R3.3）。
/// - k: 大きさは [`ScaleRatio`] 権威（`scale_len`）へ委譲し符号を保存する
///   （新しい丸め規約を導入しない・R3.6）。
/// - 片軸のみ指定なら他軸は正典既定値 0 として扱う（R3.4）。
///
/// **両軸とも未指定なら `None`** を返す（R3.4）。呼び手はこれを
/// [`apply_windowposition`] へそのまま渡すことで `balloon_offset` を一切触らず、
/// resolver 出力を本仕様適用前と厳密同一に保つ。
///
/// # 符号規約（実機確定済み・R7.6／task 6.1）
///
/// 正典（ukadoc）は x 方向の基本位置を明示せず、`windowposition.x` を「数値指定の場合
/// シェル側が+、シェルから離れる側が-」と記述する。しかし **参照実装 SSP の実挙動は
/// 左右で反転しない素の画面座標オフセット**であり、areka は互換ベースウェアゆえ
/// **SSP の実測を正**とする（この文言との乖離は互換対応表の記録対象）。
///
/// 実機実測（DPI 120＝k=5/4・ゴースト emo2 ＋バルーン emo2-kakukaku・SSP）:
///
/// | scope | 置き側 | wp.x 生値 | 基本位置 | SSP のバルーン相対 x |
/// |---|---|---|---|---|
/// | 0（sakura） | Left | +266 | `char_x − balloon_w`＝−500 | −168（＝−500 **+332**） |
/// | 1（kero） | Right | −190 | `char_x + char_w`＝+420 | +183（＝+420 **−237**） |
///
/// どちらも `wp.x × k` を**そのまま画面 x へ加算**した値である（Left で +332・
/// Right で −237）。左右で符号を反転させると kero 側が +658 となり実測から 475px ずれる。
///
/// # 丸めの 1px 差（許容）
///
/// `266×1.25 = 332.5`・`190×1.25 = 237.5`・`75×1.25 = 93.75` において SSP は
/// 332／237／93（切り捨て寄り）を採り、areka は既存丸め権威 [`ScaleRatio::scale_len`]
/// （round half away from zero）ゆえ 333／238／94 を採る。R3.6 が新しい丸め規約の導入を
/// 禁じているため、areka は既存権威のままこの 1px 差を許容する（SSP へ合わせ込まない）。
pub fn to_screen_adjust(wp_x: Option<i32>, wp_y: Option<i32>, k: ScaleRatio) -> Option<(i32, i32)> {
    if wp_x.is_none() && wp_y.is_none() {
        return None;
    }
    // 正典既定値 0（片軸欠落は「調整量 0」であり「調整量なし」ではない・R3.4）。
    let dx = scale_signed(wp_x.unwrap_or(0), k);
    let dy = scale_signed(wp_y.unwrap_or(0), k);
    Some((dx, dy))
}

/// [`to_screen_adjust`] の結果を `cfg.scopes[scope].balloon_offset` へ合流させる
/// （既存値があれば加算・無ければ設定・`None` なら無操作）。
///
/// 供給点をこの欄に置くことで `resolver.rs` の配置式 P1〜P5 は無改変で済む（design D1'）
/// ——P5 は `balloon_offset.unwrap_or((0, 0))` を既に加算しているためである。
///
/// # 注意（単位空間の混在・意図的）
///
/// 本欄は**物理 px の加算欄**である。`windowposition` 由来の調整量は k 適用済みの
/// 物理 px で合流するが、既存供給元である descript の `balloon.offsetx`/`offsety` は
/// **非スケールの生値**のまま加算される——後者の規約温存は本仕様の Out of scope（W5 対象外）。
/// emo2 は descript offset が `None` ゆえ顕在化しないが、将来の取り違えを封じるため
/// ここに明記する（design Service Interface の同注記の転記）。
pub fn apply_windowposition(cfg: &mut PlacementConfig, scope: usize, adjust: Option<(i32, i32)>) {
    let Some((dx, dy)) = adjust else {
        // 調整量なし＝`balloon_offset` を触らない（現行と bit 同一・R3.4）。
        return;
    };
    let Some(sc) = cfg.scopes.get_mut(&scope) else {
        // 未収載 scope には合流先が無い（resolver は未収載を `ScopeConfig::default()` で
        // 配置する契約ゆえ、ここで表を生やすと別の意味論を作ってしまう）。
        warn!(
            scope,
            dx,
            dy,
            "windowposition: 未収載 scope への合流要求を無視した（配置表に当該 scope が無い）"
        );
        return;
    };
    // 別供給元（descript の非スケール生値）が既にあれば加算する（飽和・ラップしない）。
    // 単位空間の混在は意図的（本関数の doc「注意（単位空間の混在・意図的）」を参照）。
    sc.balloon_offset = Some(match sc.balloon_offset {
        Some((ox, oy)) => (ox.saturating_add(dx), oy.saturating_add(dy)),
        None => (dx, dy),
    });
}

#[cfg(test)]
mod tests {
    use areka_emo_compose::ScaleRatio;

    use super::super::config::{PlacementConfig, ScopeConfig};
    use super::{apply_windowposition, to_screen_adjust};

    /// テスト用 k（非ゼロ前提）。
    fn k(num: u32, den: u32) -> ScaleRatio {
        ScaleRatio::new(num, den).expect("テストの k は非ゼロ")
    }

    /// scope 集合を持つ最小の `PlacementConfig`（`balloon_offset` は既定 None）。
    fn cfg_with_scopes(scopes: &[usize]) -> PlacementConfig {
        let mut cfg = PlacementConfig {
            scopes: Default::default(),
            zorder_raw: None,
            sticky_window_raw: None,
            shell_dpi_raw: None,
        };
        for &s in scopes {
            cfg.scopes.insert(s, ScopeConfig::default());
        }
        cfg
    }

    /// 檻 9（全分岐の直積）: x（正／負／未指定）× y（負／正／未指定）×
    /// k（恒等／非恒等 5/4）＝ 18 通り。
    ///
    /// **本檻の主張は「調整量はバルーンの置き側（`BalloonSide`）に依らず同一である」**
    /// ——`windowposition.x` は SSP 実測どおり素の画面座標オフセットであり、Left／Right で
    /// 反転しない（R3.3 の実機確定形・R7.6／task 6.1）。関数が side を受け取らないこと自体が
    /// その主張の構造的な施行であり、本檻は**同一 wp 生値が置き側の別なく単一の期待値へ
    /// 落ちること**を全分岐で固定する。side を跨いだ同一性を実配置で観測する檻は
    /// `mod.rs` の `prepare_windowposition_adjust_is_identical_for_both_balloon_sides`
    /// （両 scope が Left／Right で同一の wp 生値を読む合成 fixture）が担う。
    ///
    /// 期待値は literal（実装式の再実装ではない）。x の実値は emo2 実測——
    /// sakura（scope 0・Left）`windowposition` x=+266・kero（scope 1・Right）x=−190。
    /// この 2 値がどちらも**生値の符号のまま**現れることが実機確定の帰結である。
    /// k=5/4 の期待値は既存丸め権威 `ScaleRatio::scale_len`（round half away from zero）
    /// に一致する: 266→332.5→333・190→237.5→238・129→161.25→161・75→93.75→94。
    #[test]
    fn to_screen_adjust_is_side_independent_across_missing_axis_and_scale_matrix() {
        /// 恒等 k。
        const K1: (u32, u32) = (1, 1);
        /// 非恒等 k＝120/96＝5/4（丸めが観測できる比・実機サインオフと同じ比）。
        const K54: (u32, u32) = (120, 96);

        let chk = |wp_x: Option<i32>,
                   wp_y: Option<i32>,
                   (num, den): (u32, u32),
                   expected: Option<(i32, i32)>| {
            assert_eq!(
                to_screen_adjust(wp_x, wp_y, k(num, den)),
                expected,
                "wp=({wp_x:?},{wp_y:?}) k={num}/{den}"
            );
        };

        // ---- k=1/1（生値そのまま・x も y も無変換） ----
        // x=+266 は Left 置きの sakura 実値。画面 +x（右）へそのまま加算される。
        chk(Some(266), Some(-129), K1, Some((266, -129)));
        chk(Some(266), Some(75), K1, Some((266, 75)));
        chk(Some(266), None, K1, Some((266, 0)));
        // x=−190 は Right 置きの kero 実値。置き側で反転せず画面 −x（左）へ加算される
        // ——反転させると SSP 実測から 475px ずれる（`to_screen_adjust` の符号規約表）。
        chk(Some(-190), Some(-129), K1, Some((-190, -129)));
        chk(Some(-190), Some(75), K1, Some((-190, 75)));
        chk(Some(-190), None, K1, Some((-190, 0)));
        chk(None, Some(-129), K1, Some((0, -129)));
        chk(None, Some(75), K1, Some((0, 75)));
        chk(None, None, K1, None);

        // ---- k=5/4（大きさのみ権威で丸め・符号は保存） ----
        chk(Some(266), Some(-129), K54, Some((333, -161)));
        chk(Some(266), Some(75), K54, Some((333, 94)));
        chk(Some(266), None, K54, Some((333, 0)));
        chk(Some(-190), Some(-129), K54, Some((-238, -161)));
        chk(Some(-190), Some(75), K54, Some((-238, 94)));
        chk(Some(-190), None, K54, Some((-238, 0)));
        chk(None, Some(-129), K54, Some((0, -161)));
        chk(None, Some(75), K54, Some((0, 94)));
        chk(None, None, K54, None);
    }

    /// 両軸未指定は `None`＝`balloon_offset` を触らない＝resolver 出力が現行と厳密同一
    /// （R3.4）。k が非恒等でも `None` のままであること（k が「指定あり」を作らない）。
    #[test]
    fn to_screen_adjust_returns_none_when_both_axes_unspecified() {
        for (num, den) in [(1, 1), (120, 96), (96, 192)] {
            assert_eq!(
                to_screen_adjust(None, None, k(num, den)),
                None,
                "k={num}/{den}"
            );
        }
    }

    /// 縮小 k でも「非ゼロ長は最小 1px」という既存権威 `ScaleRatio::scale_len` の
    /// 規約をそのまま継承する（本仕様で新しい丸め規約を導入しない・R3.6）。
    /// k=1/4 で |1| → round(0.25)=0 → 権威が 1 へ引き上げる。符号は保存される。
    #[test]
    fn to_screen_adjust_inherits_scale_len_minimum_one_rule() {
        let quarter = k(96, 384);
        assert_eq!(to_screen_adjust(Some(1), Some(-1), quarter), Some((1, -1)));
        assert_eq!(
            to_screen_adjust(Some(-1), Some(-1), quarter),
            Some((-1, -1))
        );
        // 0 は 0 のまま（存在しない調整量を作らない）。
        assert_eq!(to_screen_adjust(Some(0), Some(0), quarter), Some((0, 0)));
    }

    /// 縮小 k の丸めも権威委譲（266 × 1/2 = 133・129 × 1/2 = 64.5 → 65＝0 から遠い側）。
    #[test]
    fn to_screen_adjust_delegates_half_away_from_zero_on_shrink() {
        let half = k(96, 192);
        assert_eq!(
            to_screen_adjust(Some(266), Some(-129), half),
            Some((133, -65))
        );
        assert_eq!(
            to_screen_adjust(Some(-190), Some(-75), half),
            Some((-95, -38))
        );
    }

    /// i32 域の飽和（`scale_len` は u32::MAX へ飽和するのみゆえ i32 検査は本層の責務）。
    /// 負側も `-i32::MAX` へ飽和し、`i32::MIN` の絶対値取得でパニックしない。
    #[test]
    fn to_screen_adjust_saturates_into_i32_without_panic() {
        let double = k(192, 96);
        assert_eq!(
            to_screen_adjust(Some(i32::MAX), Some(i32::MAX), double),
            Some((i32::MAX, i32::MAX))
        );
        assert_eq!(
            to_screen_adjust(Some(i32::MIN), Some(i32::MAX), double),
            Some((-i32::MAX, i32::MAX))
        );
        assert_eq!(
            to_screen_adjust(Some(i32::MIN), Some(i32::MIN), k(1, 1)),
            Some((-i32::MAX, -i32::MAX))
        );
        assert_eq!(
            to_screen_adjust(Some(i32::MAX), Some(i32::MIN), k(1, 1)),
            Some((i32::MAX, -i32::MAX))
        );
    }

    /// 調整量なし（両軸未指定）＋ descript offset なし → `balloon_offset` は `None` のまま
    /// ＝resolver 出力が本仕様適用前と bit 同一（R3.4）。
    #[test]
    fn apply_windowposition_none_leaves_balloon_offset_untouched() {
        let mut cfg = cfg_with_scopes(&[0, 1]);
        apply_windowposition(&mut cfg, 0, None);
        apply_windowposition(&mut cfg, 1, None);
        assert_eq!(cfg.scopes[&0].balloon_offset, None);
        assert_eq!(cfg.scopes[&1].balloon_offset, None);
    }

    /// 調整量なし＋既存 descript offset あり → 既存値をそのまま温存する。
    #[test]
    fn apply_windowposition_none_preserves_existing_descript_offset() {
        let mut cfg = cfg_with_scopes(&[0]);
        cfg.scopes.get_mut(&0).expect("scope 0").balloon_offset = Some((30, -5));
        apply_windowposition(&mut cfg, 0, None);
        assert_eq!(cfg.scopes[&0].balloon_offset, Some((30, -5)));
    }

    /// 既存値が無ければ設定する。
    #[test]
    fn apply_windowposition_sets_offset_when_absent() {
        let mut cfg = cfg_with_scopes(&[0, 1]);
        apply_windowposition(&mut cfg, 1, Some((190, -75)));
        assert_eq!(cfg.scopes[&1].balloon_offset, Some((190, -75)));
        // 他 scope へ漏れない。
        assert_eq!(cfg.scopes[&0].balloon_offset, None);
    }

    /// 既存値（別供給元＝descript `balloon.offsetx/offsety`）があれば加算合流する。
    #[test]
    fn apply_windowposition_adds_to_existing_descript_offset() {
        let mut cfg = cfg_with_scopes(&[0]);
        cfg.scopes.get_mut(&0).expect("scope 0").balloon_offset = Some((30, -5));
        apply_windowposition(&mut cfg, 0, Some((266, -129)));
        assert_eq!(cfg.scopes[&0].balloon_offset, Some((296, -134)));
    }

    /// 加算は飽和（ラップアラウンドしない）。
    #[test]
    fn apply_windowposition_addition_saturates() {
        let mut cfg = cfg_with_scopes(&[0]);
        cfg.scopes.get_mut(&0).expect("scope 0").balloon_offset = Some((i32::MAX, i32::MIN));
        apply_windowposition(&mut cfg, 0, Some((1, -1)));
        assert_eq!(cfg.scopes[&0].balloon_offset, Some((i32::MAX, i32::MIN)));
    }

    /// 未収載 scope は合流先が無い（`ScopeConfig` を勝手に生やさない）。
    /// resolver は未収載 scope を `ScopeConfig::default()` で配置する契約ゆえ、
    /// ここで表を膨らませると別の意味論を作ってしまう。
    #[test]
    fn apply_windowposition_ignores_unknown_scope() {
        let mut cfg = cfg_with_scopes(&[0]);
        apply_windowposition(&mut cfg, 7, Some((1, 2)));
        assert!(!cfg.scopes.contains_key(&7));
        assert_eq!(cfg.scopes[&0].balloon_offset, None);
    }
}
