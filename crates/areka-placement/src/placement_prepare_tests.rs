use std::fs;

use areka_ghost::sylphya_wiring::profile_areka_root;
use wintf::ecs::DPI;

use super::follow::OffsetBase;
use super::resolver::{PointPx, SizePx};
use super::shared_test_support::{TempDir, k};
use super::*;
use crate::sample_test_support::{balloon_root, emo2_root};
use crate::test_support::{WA, synth_declared_dpi_ghost, with_com_initialized};

// emo2 fixture の native 採寸値（共有の 1 組）。k₀ 適用後の期待値はこれらの `scaled_extent` 倍
// として書く。バルーン面 0 は **scope ごとに解決した系列**の実寸である。
use crate::sample_test_support::{
    BALLOON0_H, BALLOON0_W, BALLOON1_H, BALLOON1_W, SCOPE0_H, SCOPE0_W, SCOPE1_H, SCOPE1_W,
};

// ------------------------------------------------------------------
// 観測可能な完了状態: emo2 fixture 相当のパスで 2 スコープぶんの
// ScopePlacement を含む PreparedPlacement が返る
// ------------------------------------------------------------------

/// emo2 fixture＋合成 work area で `prepare_ghost_windows_with_work_area` が
/// 2 スコープの `ScopePlacement`（resolver P1〜P5 の厳密値）と titles を返す。
///
/// 期待値の根拠（emo2 実測: alignment=bottom・defaultx=0×2・
/// balloon.alignment=left/right・寸法 434×687／336×400・balloon は scope 別で
/// scope0=400×224（`balloons0.png`）／scope1=288×203（`balloonk0.png`）):
/// - scope0: char=(1920−434, 1040−687)=(1486,353)・基本位置＝左隣=(1486−400)=(1086,353)
/// - scope1: char=(1486−336, 1040−400)=(1150,640)・基本位置＝右隣=(1150+336)=(1486,640)
///
/// scope1 の X は `base_x(n≥1) = char_x(n−1) − w(n)`——引くのは**自スコープの幅**
/// （336）であり、前スコープの幅（434）ではない（隣接・隙間 0）。
/// これにより scope1 の右端 1150+336=1486 が scope0 の左端 1486 と一致する。
///
/// バルーン寸が scope 別になっても**基本位置**が動かないのは、右置き（scope1）の基準 x が
/// キャラ窓の右端＝バルーン幅に依存しないためである（resolver P5 は無改変）。
///
/// 基本位置へ当該 scope の `windowposition` 由来の調整量が合流する。x は**左右で反転しない素の画面座標オフセット**
/// （SSP 実測で確定・`windowposition.rs` の `to_screen_adjust`「符号規約」節）ゆえ、
/// k₀=1/1 の本検体では wp 生値がそのまま加算される:
/// - scope0（`balloons0s.txt` の `266,-129`・side=Left）→ 調整量 (+266,−129)
///   → balloon=(1086+266, 353−129)=(1352,224)・offset=(−400+266, −129)=(−134,−129)
/// - scope1（`balloonk0s.txt` の `-190,-75`・side=Right）→ 調整量 (−190,−75)
///   → balloon=(1486−190, 640−75)=(1296,565)・offset=(336−190, −75)=(146,−75)
///
/// 恒等式 `balloon_offset ≡ balloon_pos − char_pos` は両 scope で保たれている
/// （P5 の加算入力が増えただけ）——これは配置式の出力時点の事後条件であり、
/// `windowposition.limit` の補正は下流の関門が持つ。
/// SSP 実測との数値突合は
/// [`prepare_emo2_matches_ssp_balloon_offsets_at_dpi_120`]（実機と同じ k=5/4）が担う。
#[test]
fn prepare_emo2_returns_two_scope_placements() {
    with_com_initialized(|| {
        // primary DPI 96＝emo2 の作者基準 DPI（無宣言＝96）ゆえ k₀=1/1。
        let p = prepare_ghost_windows_with_work_area(&emo2_root(), &balloon_root(), WA, Some(96))
            .expect("emo2 fixture の配置準備は成功する");

        assert_eq!(p.placements.len(), 2, "emo2 は 2 スコープ（DD6）");

        let s0 = &p.placements[0];
        assert_eq!(s0.scope, 0);
        assert_eq!(s0.char_pos, PointPx { x: 1486, y: 353 });
        assert_eq!(
            s0.char_size,
            SizePx {
                w: SCOPE0_W,
                h: SCOPE0_H
            }
        );
        assert_eq!(
            s0.balloon_pos,
            PointPx { x: 1352, y: 224 },
            "本体側は balloons0s.txt の windowposition 266,-129 を反映（要件 3.2）"
        );
        assert_eq!(
            s0.balloon_size,
            SizePx {
                w: BALLOON0_W,
                h: BALLOON0_H
            }
        );
        assert_eq!(s0.balloon_offset, PointPx { x: -134, y: -129 });

        let s1 = &p.placements[1];
        assert_eq!(s1.scope, 1);
        assert_eq!(s1.char_pos, PointPx { x: 1150, y: 640 });
        assert_eq!(
            s1.char_size,
            SizePx {
                w: SCOPE1_W,
                h: SCOPE1_H
            }
        );
        assert_eq!(
            s1.balloon_pos,
            PointPx { x: 1296, y: 565 },
            "相方側は balloonk0s.txt の windowposition -190,-75 を反映（本体側の値ではない）"
        );
        assert_eq!(
            s1.balloon_size,
            SizePx {
                w: BALLOON1_W,
                h: BALLOON1_H
            },
            "相方側は自分の系列（balloonk0.png）の寸——本体側 400×224 ではない"
        );
        assert_eq!(s1.balloon_offset, PointPx { x: 146, y: -75 });

        // titles は MountModel.names 由来
        assert_eq!(p.titles.title(0), "むらさき");
        assert_eq!(p.titles.title(1), "エモ");
    });
}

/// 決定論形: 実機実測と同条件——DPI 120（k=5/4）・
/// scope0 543×859・scope1 420×500——で既定配置したとき、相方（scope1）の右端が
/// 本体（scope0）の左端と一致する（隣接・gap 0）。
///
/// **前提錨**: 先に両 scope の実表示寸が実機観測値（543×859／420×500）であることを
/// 確認する。寸が変われば以下の隣接 assert は別の幾何を検定することになるため、
/// 黙って追随せず前提の側で落ちる形にしておく。
///
/// **許容差は置かない（厳密等式）**。丸めは `ScaleRatio` 経由の**寸法算出**にのみ現れ、
/// 位置の算出（`char_x(1) = char_x(0) − w(1)`・クランプ）は整数演算のみゆえ成分誤差が
/// 生じ得ない。実機ログ側の判定に許容差 1px があるのは別チャネルである。
#[test]
fn prepare_emo2_at_dpi_120_places_scopes_adjacent() {
    with_com_initialized(|| {
        // 実機と同じ DPI 120（emo2 の作者基準 DPI は 96 ゆえ k=5/4）。
        let p = prepare_ghost_windows_with_work_area(&emo2_root(), &balloon_root(), WA, Some(120))
            .expect("emo2 fixture の配置準備は成功する");

        let s0 = &p.placements[0];
        let s1 = &p.placements[1];

        // 前提錨: 実表示寸が実機観測値と一致する（434×687／336×400 の 5/4 倍）。
        assert_eq!(
            s0.char_size,
            SizePx { w: 543, h: 859 },
            "scope0 キャラ窓は実機観測 543×859（434×687 の 5/4 倍）"
        );
        assert_eq!(
            s1.char_size,
            SizePx { w: 420, h: 500 },
            "scope1 キャラ窓は実機観測 420×500（336×400 の 5/4 倍）"
        );

        // 隣接（本丸）: scope1 の右端＝scope0 の左端。
        assert_eq!(
            s1.char_pos.x + s1.char_size.w,
            s0.char_pos.x,
            "実機同条件（DPI 120）で scope1 右端＝scope0 左端（隣接・gap 0（areka-P0-scope-chain-gap 2.1/2.2））"
        );

        // 連鎖の絶対値も固定する（WA 右端 1920 基準: 1920−543=1377・1377−420=957）。
        assert_eq!(
            s0.char_pos.x, 1377,
            "scope0 は work area 右端密着（1920−543）"
        );
        assert_eq!(
            s1.char_pos.x, 957,
            "scope1 は scope0 左端から**自スコープ幅**ぶん左（1377−420）"
        );
    });
}

/// 薄いラッパ `prepare_ghost_windows` は実モニタ列挙で primary work area を
/// 取得し、同じ work area を与えた合成経路と同一の結果を返す。
/// モニタ 0 台の環境（headless）では `PlacementError::Monitor` が返る
/// （環境寛容: どちらの分岐でも決定論的に検証する）。
#[test]
fn prepare_ghost_windows_uses_primary_monitor() {
    with_com_initialized(|| {
        let monitors = wintf::ecs::window::monitor::enumerate_monitors();
        match prepare_ghost_windows(&emo2_root(), &balloon_root()) {
            Ok(p) => {
                assert!(
                    !monitors.is_empty(),
                    "モニタ 0 台で Ok は誤成功（work area の出所がない）"
                );
                assert_eq!(p.placements.len(), 2);
                // 同一 work area **と同一 primary DPI** を注入した合成経路と一致
                // （薄いラッパは委譲のみ・実機の DPI がそのまま k₀ の分子になる）
                let primary = primary_monitor(&monitors);
                let wa = work_area_of(primary).expect("primary work area");
                let q = prepare_ghost_windows_with_work_area(
                    &emo2_root(),
                    &balloon_root(),
                    wa,
                    primary.map(|m| m.dpi),
                )
                .expect("合成経路も成功する");
                assert_eq!(p.placements, q.placements);
            }
            Err(PlacementError::Monitor { .. }) => {
                assert!(
                    monitors.is_empty(),
                    "モニタが存在するのに Monitor エラーが返った"
                );
            }
            Err(other) => panic!("Monitor 以外の Err は契約外: {other:?}"),
        }
    });
}

// ------------------------------------------------------------------
// Send 契約・失敗経路・永続化なし
// ------------------------------------------------------------------

/// `PreparedPlacement` は Send（Send な結果のみ返す）。
#[test]
fn prepared_placement_is_send() {
    fn assert_send<T: Send>() {}
    assert_send::<PreparedPlacement>();
}

/// 準備段階の失敗（ghost_root 不在→Mount）は
/// `PlacementError` として呼び手へ返る（告知と、終了するかどうかは呼び手の分担）。
/// 失敗は load（work area の検査＝モニタ 0 台の `Err` より前）で起きるため headless でも決定論。
#[test]
fn prepare_missing_root_returns_mount_err_to_caller() {
    // 不在の root は共通窓口が配るディレクトリの下に取る（窓口が作るのは配るディレクトリ
    // だけで、その下の `no_such_ghost` は作らないので確実に不在のまま・プロセス間でも一意）。
    let temp = TempDir::new();
    let root = temp.path().join("no_such_ghost");
    let err = prepare_ghost_windows(&root, &balloon_root()).expect_err("不在 root は Err");
    assert!(
        matches!(err, PlacementError::Mount(_)),
        "Mount variant 以外が返った: {err:?}"
    );
}

/// `prepare_ghost_windows` は位置の記憶を一切読み書きしない:
/// (a) 実行後、記憶の置き場（`profile/areka`）を作らない（書き込みなし）
/// (b) 偽の保存位置を持つ `sylphya.toml` を置いても出力が変わらず、ファイルも変わらない
///     （読み込みなし）
///
/// 記憶を戻すのは本体の結線が呼ぶ [`persist::load_restored_state`] と
/// [`persist::restore_and_judge`] で、準備はそのどちらも呼ばない。置き場は
/// `load_restored_state` と同じ `profile_areka_root(ghost/master)`。
#[test]
fn prepare_never_reads_or_writes_the_saved_state() {
    with_com_initialized(|| {
        // 最小の合成ゴースト（emo2 実 PNG を 2 枚だけ複写・作者の DPI は既定の 96）
        let root = TempDir::new();
        let (ghost_root, balloon_dir) =
            synth_declared_dpi_ghost(root.path(), &emo2_root(), &balloon_root(), "96", "96", None);

        let before = prepare_ghost_windows_with_work_area(&ghost_root, &balloon_dir, WA, Some(96))
            .expect("合成ゴーストの準備は成功する");

        // (a) 書き込みなし: 記憶の置き場が作られていない
        let profile = profile_areka_root(&ghost_root.join("ghost").join("master"));
        assert!(
            !profile.exists(),
            "準備が記憶の置き場を作った: {}",
            profile.display()
        );

        // (b) 読み込みなし: 作業領域の中の偽の保存位置を置いても出力不変（復元しない）
        fs::create_dir_all(&profile).expect("create profile/areka");
        let saved = profile.join("sylphya.toml");
        let junk = "format-version = 1\n[window.0]\nx = \"100\"\ny = \"100\"\n";
        fs::write(&saved, junk).expect("plant sylphya.toml");
        assert!(
            !persist::load_restored_state(&ghost_root).is_empty(),
            "前提: 置いた記憶は本番の読み手で読める"
        );

        let after = prepare_ghost_windows_with_work_area(&ghost_root, &balloon_dir, WA, Some(96))
            .expect("記憶を置いた後も準備は成功する");
        assert_eq!(
            before.placements, after.placements,
            "記憶の有無で配置が変わった（準備の中に復元の道がある・2.11 違反）"
        );
        assert_eq!(before.titles, after.titles);
        assert_eq!(
            fs::read_to_string(&saved).expect("記憶が残る"),
            junk,
            "準備が記憶を書き換えた"
        );
    });
}

// ------------------------------------------------------------------
// 起動時 k₀ の導出
// ------------------------------------------------------------------

/// 表示 DPI を両軸へ写すとき、`u16` に収まらない値は切り詰めず `u16::MAX` へ飽和する。
#[test]
fn square_dpi_saturates_instead_of_truncating() {
    assert_eq!(square_dpi(144), DPI::from_dpi(144, 144));
    // 70000 を `as u16` で切り詰めると 4464 になる。
    assert_eq!(square_dpi(70_000), DPI::from_dpi(u16::MAX, u16::MAX));
}

/// k₀ = primary モニタ DPI ÷ 作者基準 DPI（既約有理・連続 k）。
///
/// 整数倍（192/96=2/1）だけでなく **非整数倍**（120/96=5/4・112/96=7/6）も
/// 既約有理数として正しく導出されることを固定する（整数段階へ丸めない）。
#[test]
fn build_measure_scaling_derives_k0_from_primary_and_author_dpi() {
    let author = AuthorDpi::DEFAULT; // 96/96（emo2 fixture と同じ無宣言ケース）

    let (two, _) = build_measure_scaling(Some(192), author);
    assert_eq!(two.shell, k(2, 1), "192/96 は既約 2/1");
    assert_eq!(two.balloon, k(2, 1));

    let (five_quarters, _) = build_measure_scaling(Some(120), author);
    assert_eq!(five_quarters.shell, k(5, 4), "120/96 は既約 5/4（125%）");

    let (seven_sixths, _) = build_measure_scaling(Some(112), author);
    assert_eq!(seven_sixths.shell, k(7, 6), "112/96 は既約 7/6（非整数 k）");

    let (identity, _) = build_measure_scaling(Some(96), author);
    assert_eq!(
        identity.shell,
        ScaleRatio::ONE,
        "primary DPI＝作者基準 DPI は恒等 k=1/1（要件 1.3/7.2）"
    );
    assert_eq!(identity.balloon, ScaleRatio::ONE);
}

/// shell と balloon は**それぞれ自分の**作者基準 DPI で k₀ を得る（2 軸独立）。
///
/// 取り違え（shell の k をバルーンへ適用する／2 値を入れ替える）は
/// コンパイルを通ってしまう静かな誤表示ゆえ、両軸に**異なる**宣言値を与えて檻に入れる。
#[test]
fn build_measure_scaling_uses_each_axis_own_author_dpi() {
    let (scaling, _) = build_measure_scaling(
        Some(192),
        AuthorDpi {
            shell: 96,
            balloon: 144,
        },
    );
    assert_eq!(scaling.shell, k(2, 1), "shell は 192/96＝2/1");
    assert_eq!(
        scaling.balloon,
        k(4, 3),
        "balloon は 192/144＝4/3（shell の k ではない）"
    );

    // 入れ替え検出（同じ 2 値を逆に宣言すると k も逆になる）。
    let (swapped, _) = build_measure_scaling(
        Some(192),
        AuthorDpi {
            shell: 144,
            balloon: 96,
        },
    );
    assert_eq!(
        swapped.shell,
        k(4, 3),
        "shell 宣言 144 は shell の k を決める"
    );
    assert_eq!(
        swapped.balloon,
        k(2, 1),
        "balloon 宣言 96 は balloon の k を決める"
    );
}

/// primary モニタ DPI 取得不能（`None`／0）は **96 相当**へ縮退する。恒等を一律に返すのではなく、
/// 96 を分子として作者基準 DPI との比を取る（作者が 96 以外を宣言していれば k₀≠1）。
/// 縮退は値を返すだけなので、`error!` の 1 行が唯一の手がかりになる（その行も確かめる）。
#[test]
fn build_measure_scaling_degrades_to_96_equivalent_when_primary_dpi_unobtainable() {
    for missing in [None, Some(0)] {
        let ((default_author, _), events) =
            log_capture_kit::capture(|| build_measure_scaling(missing, AuthorDpi::DEFAULT));
        let err = log_capture_kit::expect_one(&events, "primary モニタ DPI を取得できない");
        assert_eq!(err.level, tracing::Level::ERROR, "{missing:?}");
        assert_eq!(
            log_capture_kit::ExpectField::expect_field(err, "fallback_dpi"),
            "96",
            "{missing:?}"
        );
        assert_eq!(
            default_author.shell,
            ScaleRatio::ONE,
            "author 96 に対する 96 相当は恒等（従来の native 採寸・{missing:?}）"
        );
        assert_eq!(default_author.balloon, ScaleRatio::ONE);

        let (declared, _) = build_measure_scaling(
            missing,
            AuthorDpi {
                shell: 192,
                balloon: 48,
            },
        );
        assert_eq!(
            declared.shell,
            k(1, 2),
            "96 相当 ÷ author 192＝1/2（恒等へ丸めない・{missing:?}）"
        );
        assert_eq!(declared.balloon, k(2, 1), "96 相当 ÷ author 48＝2/1");
    }
}

/// 実 fixture・偽装境界の end-to-end: 注入した primary DPI から
/// 導かれた k₀ 倍後の**物理窓寸**で `ScopePlacement` が組み上がる。
///
/// - primary 96（＝emo2 の作者基準 DPI）の native 寸は
///   [`prepare_emo2_returns_two_scope_placements`] が確かめる
/// - primary 192: 全寸がちょうど 2 倍
/// - primary 112（k=7/6・非整数）: 丸めは単一権威 `scaled_extent` に一致する。
///   とくに 687×7/6 は **ちょうど 801.5** ゆえ、round half away from zero なら 802・
///   切り捨て（`len*num/den`）なら 801 に分かれる（丸め規約の取り違え検出）。
///   f32 近道の検出は別檻 [`prepare_k0_rounding_matches_integer_authority_not_f32`]
///   が担う（本検体は f32 でも偶然 802 に一致するため判別力を持たない）。
#[test]
fn prepare_emo2_scales_window_sizes_by_k0() {
    with_com_initialized(|| {
        let doubled =
            prepare_ghost_windows_with_work_area(&emo2_root(), &balloon_root(), WA, Some(192))
                .expect("k₀=2/1 の準備は成功する");
        assert_eq!(
            doubled.placements[0].char_size,
            SizePx {
                w: SCOPE0_W * 2,
                h: SCOPE0_H * 2
            },
            "primary DPI 192 なら scope0 キャラ窓は物理 2 倍（要件 3.3）"
        );
        assert_eq!(
            doubled.placements[1].char_size,
            SizePx {
                w: SCOPE1_W * 2,
                h: SCOPE1_H * 2
            }
        );
        // バルーン窓も自軸の k₀ で追従する。寸は **scope ごとに解決した系列**の
        // 原寸の 2 倍であり、全 placement で同一ではない。
        assert_eq!(
            doubled.placements[0].balloon_size,
            SizePx {
                w: BALLOON0_W * 2,
                h: BALLOON0_H * 2
            },
            "scope0 は本体側系列（400×224）の 2 倍"
        );
        assert_eq!(
            doubled.placements[1].balloon_size,
            SizePx {
                w: BALLOON1_W * 2,
                h: BALLOON1_H * 2
            },
            "scope1 は相方側系列（288×203）の 2 倍"
        );

        let seven_sixths =
            prepare_ghost_windows_with_work_area(&emo2_root(), &balloon_root(), WA, Some(112))
                .expect("k₀=7/6 の準備は成功する");
        assert_eq!(
            seven_sixths.placements[0].char_size,
            SizePx { w: 506, h: 802 },
            "非整数 k₀ でも丸めは scaled_extent 単一権威（687×7/6＝ちょうど 801.5→802）"
        );
    });
}

/// **宣言された**作者基準 DPI が (a) 採寸 k₀ と (b) attach 搬送値の双方へ効くこと。
///
/// emo2 fixture は shell/balloon とも DPI 無宣言＝96 で、これは正典既定と**同値**ゆえ
/// 「宣言を読んだ」のか「既定を返した」のかを区別できない。ここでは
/// shell=120／balloon=144 を**実際に宣言する**合成ゴーストを組み、既定（96）とも
/// 互いとも異なる 3 値で檻に入れる:
///
/// - 既定へ差し替える誤り → k が 240/96=5/2 になり寸法が落ちる
/// - shell/balloon の取り違え → char に 5/3・balloon に 2/1 が乗って落ちる
#[test]
fn prepare_declared_author_dpi_drives_each_axis_k0_and_attach_value() {
    with_com_initialized(|| {
        let root = TempDir::new();
        // shell=120（125% 原稿）・balloon=144（150% 原稿）——既定 96 とも互いとも異なる 3 値。
        let (ghost_root, balloon_dir) = synth_declared_dpi_ghost(
            root.path(),
            &emo2_root(),
            &balloon_root(),
            "120",
            "144",
            None,
        );

        let p = prepare_ghost_windows_with_work_area(&ghost_root, &balloon_dir, WA, Some(240))
            .expect("宣言 DPI 付き合成ゴーストの準備は成功する");

        // (a) attach 搬送値＝宣言値そのもの（既定 96 でも入れ替えでもない）。
        assert_eq!(
            p.author_dpi,
            AuthorDpi {
                shell: 120,
                balloon: 144
            },
            "宣言された作者基準 DPI が読まれ、そのまま attach 側へ搬送される"
        );

        // (b) 採寸 k₀: char は 240/120＝2/1・balloon は 240/144＝5/3（軸ごとに別）。
        // scope0 の合成寸は emo2 実 surface0 単体＝434×687（native 値と一致する検体）。
        assert_eq!(
            p.placements[0].char_size,
            SizePx {
                w: SCOPE0_W * 2,
                h: SCOPE0_H * 2
            },
            "char は shell 宣言 120 由来の k=2/1（balloon の 5/3 ではない）"
        );
        // 本補助の合成バルーンは本体側 `balloons0.png` しか持たないため、全 scope が
        // 本体側系列へ収束する（後方互換——`balloonk*` 不在なら全 scope 同一寸）。
        for placement in &p.placements {
            assert_eq!(
                placement.balloon_size,
                // 400×5/3＝666.67→667・224×5/3＝373.33→373（scaled_extent 単一権威）
                SizePx { w: 667, h: 373 },
                "balloon は balloon 宣言 144 由来の k=5/3（shell の 2/1 ではない）"
            );
        }
    });
}

/// f32 近道の混入検出（寸法の丸めは `scaled_extent` だけが行う）:`scaled_extent` の整数演算と
/// `as_f32` 乗算が **1px 食い違う**検体を実経路へ通す。
///
/// 検体: 作者基準 DPI 168・primary DPI 186 → k₀=31/28。emo2 の scope0 幅 434 に対し
/// - 権威（整数）: (2×434×31+28)/(2×28) = 481（真値ちょうど 480.5 → 0 から遠い側）
/// - `(434.0 × as_f32(31/28)).round()`: 480（31/28 の f32 表現が真値より小さい）
///
/// k₀ の導出がどこかで f32 を経由していれば 480 になり本檻が落ちる。
#[test]
fn prepare_k0_rounding_matches_integer_authority_not_f32() {
    with_com_initialized(|| {
        let root = TempDir::new();
        let (ghost_root, balloon_dir) = synth_declared_dpi_ghost(
            root.path(),
            &emo2_root(),
            &balloon_root(),
            "168",
            "96",
            None,
        );

        let p = prepare_ghost_windows_with_work_area(&ghost_root, &balloon_dir, WA, Some(186))
            .expect("k₀=31/28 の準備は成功する");

        assert_eq!(
            p.placements[0].char_size,
            // 434→481・687→(2×687×31+28)/56=761
            SizePx { w: 481, h: 761 },
            "丸めは scaled_extent（整数）の値——f32 経由なら幅が 480 になる"
        );

        // 本検体が実際に f32 と食い違うことの自己検証（檻の有効性そのものを固定する）。
        let via_f32 = (SCOPE0_W as f32 * k(31, 28).as_f32()).round() as i32;
        assert_eq!(via_f32, 480, "f32 経路は 480（＝この検体は判別力を持つ）");
    });
}

// ------------------------------------------------------------------
// shell `seriko.zorder` の搬送
// ------------------------------------------------------------------

/// 準備の中間結果を、`seriko.zorder` の有無だけ変えて組む（他の入力は最小の空）。
///
/// 採寸結果を空にしてあるのは、搬送の主張が寸法にも配置にも一切依らないからである
/// （この値は窓を 1 mm も動かさない）。
fn stages_with_shell(shell_kv_text: &str) -> PreparedStages {
    let source = source::DescriptSource {
        ghost_kv: areka_parsers::kv::parse_kv(""),
        shell_kv: areka_parsers::kv::parse_kv(shell_kv_text),
        shell_dir: std::path::PathBuf::new(),
        titles: GhostTitles::from_scope_titles(Vec::new()),
    };
    PreparedStages {
        cfg: config::build_placement_config(&source.ghost_kv, &source.shell_kv),
        sizes: MeasuredSizes { scopes: Vec::new() },
        titles: GhostTitles::from_scope_titles(Vec::new()),
        author_dpi: AuthorDpi::DEFAULT,
        default_basis: reset_pos::DefaultPlacementBasis {
            source,
            balloon: reseed::BalloonPlacementInputs {
                author_dpi: 96,
                windowpositions: Default::default(),
            },
        },
        // 採寸 DPI は既定の 96（この助手は寸法にも配置にも依らないので値は判定に効かない）。
        measure_dpi: DPI::from_dpi(96, 96),
        default_surfaces: DefaultSurfaces::builtin(),
        balloon_dontmove: Default::default(),
    }
}

/// shell が `seriko.zorder` を宣言していれば、その**生の値がそのまま**準備の結果に載る。
///
/// 起動の結線（`wire_runtime`）はこの欄から重なりの基底を受け取る。ここで落ちると
/// 設定は本番で一切効かないが、配置も採寸も何一つ変わらないので他のどの檻にも映らない。
#[test]
fn prepare_carries_the_shell_zorder_value_verbatim() {
    let prepared = stages_with_shell(
        "seriko.zorder,1,0
",
    )
    .resolve(WA);

    assert_eq!(
        prepared.zorder_raw.as_deref(),
        Some("1,0"),
        "shell の seriko.zorder が準備の結果へ搬送されていない（解釈は下流なので生のまま）"
    );
}

/// shell の `seriko.sticky-window` も Z の順と同じ形で、生の値のまま準備の結果に載る。宣言が無ければ `None`。
#[test]
fn prepare_carries_the_shell_sticky_window_value_verbatim() {
    let declared = stages_with_shell(
        "seriko.sticky-window,1,0
",
    )
    .resolve(WA);
    let undeclared = stages_with_shell(
        "seriko.zorder,1,0
",
    )
    .resolve(WA);

    assert_eq!(
        (
            declared.sticky_window_raw.as_deref(),
            undeclared.sticky_window_raw
        ),
        (Some("1,0"), None),
        "seriko.sticky-window が生のまま準備の結果へ運ばれていない（宣言あり, 宣言なし）"
    );
}

/// 逆側——宣言が無ければ `None` のまま運ばれる（基底なしで起動する）。
///
/// 片側だけでは「常に `Some` を作る」形と区別が付かない。
#[test]
fn prepare_carries_no_zorder_value_when_the_shell_declares_none() {
    let prepared = stages_with_shell(
        "seriko.dpi,120
",
    )
    .resolve(WA);

    assert_eq!(
        prepared.zorder_raw, None,
        "宣言が無いのに重なりの値が生まれた（既定＝非強制が壊れる）"
    );
}

// ------------------------------------------------------------------
// 採寸 DPI の単一決定点と基準対への搬送
// ------------------------------------------------------------------

/// `build_measure_scaling` が返す**採寸 DPI**は、同じ呼出が k₀ の**分子**へ採った値と
/// 常に一致する（単一の決定点）。
///
/// 分子を内部で決めて捨て、呼び手が同じ判断（`primary_dpi` の有無と 0 の弾き）を
/// もう一度書く形だと、片方だけ直したときに採寸 k₀ と基準 DPI が静かに食い違う。
/// 食い違いは**表示 DPI が変わるまで観測されない**ゆえ、ここで構造として封じる。
/// 縮退の腕（`None`／`Some(0)`＝[`FALLBACK_PRIMARY_DPI`]）も同じ規則で固定する。
#[test]
fn build_measure_scaling_returns_the_dpi_it_took_as_numerator() {
    let declared = AuthorDpi {
        shell: 192,
        balloon: 48,
    };
    let cases = [
        (Some(192), AuthorDpi::DEFAULT, 192),
        (Some(120), AuthorDpi::DEFAULT, 120),
        (Some(112), declared, 112),
        (Some(96), AuthorDpi::DEFAULT, 96),
        // 縮退の 2 腕: 恒等へ丸めず 96 相当を分子に採る。
        (None, AuthorDpi::DEFAULT, FALLBACK_PRIMARY_DPI),
        (Some(0), declared, FALLBACK_PRIMARY_DPI),
    ];
    for (primary_dpi, author, expected_dpi) in cases {
        let (scaling, measure_dpi) = build_measure_scaling(primary_dpi, author);
        assert_eq!(
            measure_dpi, expected_dpi,
            "採用した表示 DPI を返す（primary_dpi={primary_dpi:?}）"
        );
        assert_eq!(
            scaling.shell,
            k(measure_dpi, u32::from(author.shell)),
            "shell 軸の k₀ の分子は返した採寸 DPI と同一（primary_dpi={primary_dpi:?}）"
        );
        assert_eq!(
            scaling.balloon,
            k(measure_dpi, u32::from(author.balloon)),
            "balloon 軸の k₀ の分子も同一——軸が違っても分子は 1 つ（primary_dpi={primary_dpi:?}）"
        );
    }
}

/// 配置解決の出力が**基準対**を載せる: 値は配置式が出した既定のオフセットそのもの、
/// 基準 DPI は採寸に使った表示 DPI。
///
/// 採寸 DPI は**起動時の主モニタ DPI**であって、窓がいま載っているモニタの DPI では
/// ない（意図された挙動）。縮退の腕でも基準対は未係留にならず、96 相当が
/// 刻まれることを併せて固定する。
/// 縮退の腕は、primary モニタ DPI が取れなくても準備が成功して 2 スコープの窓寸が得られる
/// ことの end-to-end の確かめも兼ねる（縮退の k₀ そのものは
/// [`build_measure_scaling_degrades_to_96_equivalent_when_primary_dpi_unobtainable`] が持つ）。
#[test]
fn prepare_stamps_measure_dpi_onto_balloon_offset_base() {
    with_com_initialized(|| {
        for (primary_dpi, expected_dpi) in [
            (Some(120), 120u32),
            (Some(192), 192),
            (None, FALLBACK_PRIMARY_DPI),
        ] {
            let prepared = prepare_ghost_windows_with_work_area(
                &emo2_root(),
                &balloon_root(),
                WA,
                primary_dpi,
            )
            .expect("準備は成功する");
            assert_eq!(prepared.placements.len(), 2);
            for p in &prepared.placements {
                assert_eq!(
                    p.balloon_offset_base,
                    OffsetBase {
                        offset: p.balloon_offset,
                        dpi: Some(DPI::from_dpi(expected_dpi as u16, expected_dpi as u16)),
                    },
                    "scope {} の基準対＝(配置式の既定 offset, 採寸 DPI)（primary_dpi={primary_dpi:?}）",
                    p.scope
                );
            }
        }
    });
}
