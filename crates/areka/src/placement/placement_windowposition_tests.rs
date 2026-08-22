use super::resolver::{PointPx, SizePx};
use super::shared_test_support::{
    TempDir, WA, balloon_root, emo2_root, synth_declared_dpi_ghost, with_com_initialized,
};
use super::test_support::{LogEvent, capture_logs};
use super::*;

// ------------------------------------------------------------------
// windowposition → 初期既定位置（task 4.3・要件 3.2/3.4/3.5/3.6/6.3/7.6）
// ------------------------------------------------------------------

/// 観測点 4（design Monitoring・要件 6.3/7.6）: `prepare_stages` が scope ごとに
/// **scope・wp 生値・バルーンの左右・変換後の調整量**を info レベルで記録する。
///
/// 実機サインオフ（task 6.1）は本行を `RUST_LOG=info` で grep し、SSP 実測と突合して
/// x 方向の符号規約を確定させた（R7.6）。ゆえに**フィールド名と値の形そのものが契約**
/// である。`balloon_side` は調整量の計算に効かなくなった後も、基本位置がどちらだったかを
/// ログだけで再構成するために記録し続ける（実機診断の要）。
#[test]
fn prepare_windowposition_logs_scope_raw_side_and_converted_adjust() {
    with_com_initialized(|| {
        let (result, events) = capture_logs(|| {
            prepare_ghost_windows_with_work_area(&emo2_root(), &balloon_root(), WA, Some(96))
        });
        result.expect("emo2 fixture の配置準備は成功する");

        let hits: Vec<&LogEvent> = events
            .iter()
            .filter(|e| {
                e.message()
                    .contains("windowposition を初期既定位置の調整量へ")
            })
            .collect();
        assert_eq!(
            hits.len(),
            2,
            "scope ごとに 1 行（emo2 は 2 スコープ）: {events:?}"
        );

        // scope0（本体側・`balloons0s.txt` の 266,-129・バルーンはキャラ左）。
        assert_eq!(hits[0].level, tracing::Level::INFO);
        assert_eq!(hits[0].field("scope"), "0");
        assert_eq!(hits[0].field("windowposition_x"), "Some(266)");
        assert_eq!(hits[0].field("windowposition_y"), "Some(-129)");
        assert_eq!(hits[0].field("balloon_side"), "Left");
        assert_eq!(hits[0].field("adjust_dx"), "266");
        assert_eq!(hits[0].field("adjust_dy"), "-129");
        assert_eq!(hits[0].field("adjusted"), "true");

        // scope1（相方側・`balloonk0s.txt` の -190,-75・バルーンはキャラ右）。
        assert_eq!(hits[1].field("scope"), "1");
        assert_eq!(hits[1].field("windowposition_x"), "Some(-190)");
        assert_eq!(hits[1].field("windowposition_y"), "Some(-75)");
        assert_eq!(hits[1].field("balloon_side"), "Right");
        assert_eq!(
            hits[1].field("adjust_dx"),
            "-190",
            "生値 −190 は Right 側でも反転せず画面 −x のまま（SSP 実測で確定・R7.6）"
        );
        assert_eq!(hits[1].field("adjust_dy"), "-75");
    });
}

/// **参照実装（SSP）実測との突合**（要件 7.6 の実機確定・task 6.1）。
///
/// areka は「ukadoc 準拠 SSP 代替」であり、正典が沈黙する `windowposition.x` の
/// 基本位置・符号の向きについては **SSP の実挙動が正解表**である。実機（DPI 120＝
/// k=5/4・DPI aware）で同一ゴースト emo2 ＋バルーン emo2-kakukaku を SSP に読ませた
/// 実測（task 6.1）:
///
/// | 窓 | SSP 実測 | 相対オフセット |
/// |---|---|---|
/// | むらさき（scope 0・balloon Left） | 477×683 @ (3363,1417) | — |
/// | むらさき/Balloon | 500×280 @ (3195,1256) | (−168, −161) |
/// | エモ（scope 1・balloon Right） | 420×500 @ (3006,1600) | — |
/// | エモ/Balloon | 360×253 @ (3189,1507) | (+183, −93) |
///
/// 逆算すると SSP は `windowposition.x` を**左右で反転しない素の画面座標オフセット**
/// として適用している（sakura wp x=+266 → −500+332＝−168・kero wp x=−190 →
/// +420−237＝+183。どちらも「正＝画面右／負＝画面左」）。
///
/// 許容差は **各成分 1px**（`ScaleRatio::scale_len` の round half away from zero が
/// 266×1.25=332.5→333・190×1.25=237.5→238・75×1.25=93.75→94 と切り上げるのに対し
/// SSP は 332／237／93 と切り捨て寄りに落とすため）。丸め権威は既存のまま変えない
/// （新しい丸め規約を導入しない・要件 3.6）。
#[test]
fn prepare_emo2_matches_ssp_balloon_offsets_at_dpi_120() {
    /// SSP 実測のバルーン相対オフセット（scope 0＝Left 置き）。
    const SSP_SCOPE0_OFFSET: PointPx = PointPx { x: -168, y: -161 };
    /// SSP 実測のバルーン相対オフセット（scope 1＝Right 置き）。
    const SSP_SCOPE1_OFFSET: PointPx = PointPx { x: 183, y: -93 };

    /// 各成分が 1px 以内で一致することを確認する（丸め権威由来の差のみ許容）。
    fn assert_within_1px(actual: PointPx, ssp: PointPx, scope: usize) {
        assert!(
            (actual.x - ssp.x).abs() <= 1 && (actual.y - ssp.y).abs() <= 1,
            "scope {scope}: SSP 実測 {ssp:?} と areka 算出 {actual:?} の差が 1px を超える"
        );
    }

    with_com_initialized(|| {
        // 実機と同じ DPI 120（emo2 の作者基準 DPI は 96 ゆえ k=5/4）。
        let p = prepare_ghost_windows_with_work_area(&emo2_root(), &balloon_root(), WA, Some(120))
            .expect("emo2 fixture の配置準備は成功する");

        let s0 = &p.placements[0];
        let s1 = &p.placements[1];

        // 突合の錨: Right 置きの基本位置はキャラ窓幅に依存するため、まず窓寸が
        // SSP と一致していることを確かめる（一致しなければオフセット比較が無意味）。
        assert_eq!(
            s1.char_size,
            SizePx { w: 420, h: 500 },
            "scope1 キャラ窓は SSP 実測 420×500 と一致する（336×400 の 5/4 倍）"
        );
        assert_eq!(
            s0.balloon_size,
            SizePx { w: 500, h: 280 },
            "scope0 バルーンは SSP 実測 500×280 と一致する"
        );

        // areka の算出値（既存丸め権威のまま）。
        assert_eq!(
            s0.balloon_offset,
            PointPx { x: -167, y: -161 },
            "scope0: 基本位置 −500 ＋ 素の調整量 +333（266×5/4）"
        );
        assert_eq!(
            s1.balloon_offset,
            PointPx { x: 182, y: -94 },
            "scope1: 基本位置 +420 ＋ 素の調整量 −238（−190×5/4・左右で反転しない）"
        );

        assert_within_1px(s0.balloon_offset, SSP_SCOPE0_OFFSET, 0);
        assert_within_1px(s1.balloon_offset, SSP_SCOPE1_OFFSET, 1);

        // 恒等式は SSP 突合の検体でも保たれる。
        for s in &p.placements {
            assert_eq!(
                s.balloon_offset,
                PointPx {
                    x: s.balloon_pos.x - s.char_pos.x,
                    y: s.balloon_pos.y - s.char_pos.y
                },
                "scope {}: balloon_offset ≡ balloon_pos − char_pos",
                s.scope
            );
        }
    });
}

/// 要件 3.6: `windowposition` 由来の調整量は**バルーン軸の k**（`scaling.balloon`）で
/// 拡縮する（`windowposition` はバルーン作者の空間で書かれた値ゆえ・shell の k ではない）。
///
/// 検体（shell=120／balloon=144／primary=240）は k_shell=2/1・k_balloon=5/3 と
/// **異なる 2 値**ゆえ取り違えが観測できる（既存の
/// [`prepare_declared_author_dpi_drives_each_axis_k0_and_attach_value`] と同じ流儀）:
/// - 権威（balloon 軸 5/3）: 266×5/3＝443.33→443・129×5/3＝ちょうど 215
/// - 誤って shell 軸 2/1 を使うと 532／258 になり本檻が落ちる
///
/// 合成バルーンは `balloons0.png` の 1 枚のみゆえ両 scope が本体側系列へ収束し、
/// **同一の wp 生値**が side（left／right）に依らず同一の調整量になることも同時に固定する
/// （R7.6 の実機確定形）。
#[test]
fn prepare_windowposition_adjust_scales_with_balloon_k_not_shell_k() {
    with_com_initialized(|| {
        let root = TempDir::new();
        let (ghost_root, balloon_dir) =
            synth_declared_dpi_ghost(&root, "120", "144", Some((266, -129)));

        let p = prepare_ghost_windows_with_work_area(&ghost_root, &balloon_dir, WA, Some(240))
            .expect("windowposition 宣言つき合成ゴーストの準備は成功する");

        // balloon 寸は k=5/3 で 667×373（既存檻と同値）。
        let s0 = &p.placements[0];
        assert_eq!(s0.balloon_size, SizePx { w: 667, h: 373 });
        assert_eq!(
            s0.balloon_offset,
            // 基本位置（左隣＝−667）＋調整量 (+443, −215)
            PointPx { x: -224, y: -215 },
            "balloon 軸 k=5/3 で拡縮（shell 軸 2/1 なら (−135, −258) になる）"
        );

        let s1 = &p.placements[1];
        assert_eq!(
            s1.balloon_offset,
            // 基本位置（右隣＝+char_w）＋調整量 (+443, −215)（side に依らず同一）
            PointPx {
                x: s1.char_size.w + 443,
                y: -215
            },
            "Right 側も balloon 軸 k=5/3 で +443（shell 軸 2/1 なら +532 になる）"
        );
    });
}

/// **檻 9 の配置面の対（R3.3 の実機確定形・R7.6／task 6.1）**: 同一の `windowposition`
/// 生値は、バルーンの置き側（Left／Right）に依らず**同一の調整量**として基本位置へ
/// 加算される。
///
/// SSP 実測で確定した規約は「`windowposition.x` は左右で反転しない素の画面座標
/// オフセット」であり、置き側が変えるのは**基本位置だけ**（Left＝`char_x − balloon_w`／
/// Right＝`char_x + char_w`）である。左右反転を再導入すると Right 側の寄与が
/// −266 になり本檻が落ちる。
///
/// 検体は合成ゴースト（shell/balloon とも DPI 96＝k₀ 1/1 ゆえ生値がそのまま調整量に
/// なる）で、`balloons0.png` の 1 枚しか持たないため両 scope が同一のバルーン定義
/// ——すなわち**同一の wp 生値**——を読む。違うのは `balloon.alignment` だけである。
#[test]
fn prepare_windowposition_adjust_is_identical_for_both_balloon_sides() {
    with_com_initialized(|| {
        let root = TempDir::new();
        let (ghost_root, balloon_dir) =
            synth_declared_dpi_ghost(&root, "96", "96", Some((266, -129)));

        let (result, events) = capture_logs(|| {
            prepare_ghost_windows_with_work_area(&ghost_root, &balloon_dir, WA, Some(96))
        });
        let p = result.expect("windowposition 宣言つき合成ゴーストの準備は成功する");

        // (a) 観測点 4: 置き側は Left／Right と異なるのに調整量は同一。
        let hits: Vec<&LogEvent> = events
            .iter()
            .filter(|e| {
                e.message()
                    .contains("windowposition を初期既定位置の調整量へ")
            })
            .collect();
        assert_eq!(hits.len(), 2, "scope ごとに 1 行: {events:?}");
        assert_eq!(hits[0].field("balloon_side"), "Left");
        assert_eq!(hits[1].field("balloon_side"), "Right");
        assert_eq!(hits[0].field("adjust_dx"), "266");
        assert_eq!(
            hits[1].field("adjust_dx"),
            "266",
            "Right 側でも符号は反転しない（反転すれば -266 になる）"
        );
        assert_eq!(hits[0].field("adjust_dy"), "-129");
        assert_eq!(hits[1].field("adjust_dy"), "-129");

        // (b) 配置面: 基本位置からの寄与が両 scope で一致する
        //     （置き側が変えるのは基本位置だけ）。
        let s0 = &p.placements[0];
        let s1 = &p.placements[1];
        let contribution_left = s0.balloon_offset.x - (-s0.balloon_size.w);
        let contribution_right = s1.balloon_offset.x - s1.char_size.w;
        assert_eq!(contribution_left, 266, "Left 置きの基本位置は −balloon_w");
        assert_eq!(contribution_right, 266, "Right 置きの基本位置は +char_w");
        assert_eq!(
            contribution_left, contribution_right,
            "同一 wp 生値の寄与は置き側に依らず同一（R3.3 の実機確定形）"
        );
        assert_eq!(s0.balloon_offset.y, -129);
        assert_eq!(s1.balloon_offset.y, -129);
    });
}

/// 要件 3.4／design Postconditions: `windowposition` の数値指定が**どの層にも無い**
/// とき（かつ descript の `balloon.offsetx/offsety` も無いとき）`balloon_offset` は
/// `None` のまま＝resolver 出力は本仕様適用前と bit 同一である。
///
/// 観測面は resolver の出力そのもの: left 置き＝`−balloon_w`・right 置き＝`+char_w`
/// （P5 の基本位置ちょうど・y はゼロ）。
#[test]
fn prepare_without_windowposition_keeps_resolver_output_bit_identical() {
    with_com_initialized(|| {
        let root = TempDir::new();
        // 作者基準 DPI も 96／96（k₀=1/1）＝調整量以外の変化要因を持たない検体。
        let (ghost_root, balloon_dir) = synth_declared_dpi_ghost(&root, "96", "96", None);

        let p = prepare_ghost_windows_with_work_area(&ghost_root, &balloon_dir, WA, Some(96))
            .expect("windowposition 無宣言の合成ゴーストの準備は成功する");

        let s0 = &p.placements[0];
        assert_eq!(
            s0.balloon_offset,
            PointPx {
                x: -s0.balloon_size.w,
                y: 0
            },
            "scope0（left 置き）は基本位置ちょうど＝本仕様適用前と同一"
        );
        let s1 = &p.placements[1];
        assert_eq!(
            s1.balloon_offset,
            PointPx {
                x: s1.char_size.w,
                y: 0
            },
            "scope1（right 置き）も基本位置ちょうど＝本仕様適用前と同一"
        );
    });
}

/// 要件 3.5: 永続化されたバルーン相対位置があるとき、復元 merge（`persist.rs`・無改変）は
/// **永続値を優先**し、`windowposition` 由来の初期既定位置は結果に一切残らない。
///
/// 檻の作り: 同じ永続 entries を (a) 本準備の出力（windowposition 反映済み）と
/// (b) `balloon_offset` を潰した対照へ適用し、**両者の復元結果が一致する**ことを見る。
/// 一致すれば復元後の値は永続値のみに依存する＝供給元の増加が優先順位を変えていない。
#[test]
fn prepare_windowposition_yields_to_persisted_balloon_offset() {
    use areka_sylphya::{Axis, PersistKey};

    with_com_initialized(|| {
        let p = prepare_ghost_windows_with_work_area(&emo2_root(), &balloon_root(), WA, Some(96))
            .expect("emo2 fixture の配置準備は成功する");
        // 前提: windowposition が実際に効いている（対照が空虚一致にならないことの確認）。
        assert_ne!(
            p.placements[0].balloon_offset,
            PointPx {
                x: -p.placements[0].balloon_size.w,
                y: 0
            },
            "windowposition 反映前の値と同じでは檻が無意味"
        );

        let entry = |scope: u32, axis: Axis, v: &str| {
            (PersistKey::BalloonOffset { scope, axis }, v.to_string())
        };
        let entries = vec![
            entry(0, Axis::X, "-512"),
            entry(0, Axis::Y, "-48"),
            entry(1, Axis::X, "640"),
            entry(1, Axis::Y, "-16"),
        ];
        let snapshot = follow::MonitorSnapshot {
            work_areas: vec![WA],
        };

        // 対照: windowposition の寄与を取り除いた（基本位置ちょうどの）配置。
        let control: Vec<ScopePlacement> = p
            .placements
            .iter()
            .map(|s| {
                let balloon_offset = PointPx { x: 0, y: 0 };
                ScopePlacement {
                    balloon_pos: s.char_pos,
                    balloon_offset,
                    ..*s
                }
            })
            .collect();

        let restored =
            persist::apply_restored_placements(p.placements.clone(), &entries, &snapshot);
        let restored_control = persist::apply_restored_placements(control, &entries, &snapshot);

        assert_eq!(
            restored, restored_control,
            "永続値がある scope の復元結果は windowposition に依存しない（要件 3.5）"
        );
    });
}
