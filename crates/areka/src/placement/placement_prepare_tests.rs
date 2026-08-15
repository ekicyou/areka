use std::fs;

use super::shared_test_support::{
    TempDir, WA, balloon_root, emo2_root, synth_declared_dpi_ghost, with_com_initialized,
};
use super::resolver::{PointPx, SizePx};
use super::*;

/// emo2 fixture の native 採寸値（`measure.rs` テストの同名定数と同一の実測値）。
/// k₀ 適用後の期待値はこれらの `scaled_extent` 倍として書く。
const SCOPE0_W: i32 = 434;
const SCOPE0_H: i32 = 687;
const SCOPE1_W: i32 = 336;
const SCOPE1_H: i32 = 400;
/// バルーン面 0 は **scope ごとに解決した系列**の実寸である（要件 3.1）——
/// scope0＝本体側 `balloons0.png`・scope1＝相方側 `balloonk0.png`。
const BALLOON0_W: i32 = 400;
const BALLOON0_H: i32 = 224;
const BALLOON1_W: i32 = 288;
const BALLOON1_H: i32 = 203;

/// 既約有理 k のテスト用短縮構築（0 入力はテストの誤りゆえ expect）。
fn k(num: u32, den: u32) -> ScaleRatio {
    ScaleRatio::new(num, den).expect("テスト入力は非ゼロ")
}

// ------------------------------------------------------------------
// 観測可能な完了状態（tasks 6.1）: emo2 fixture 相当のパスで 2 スコープぶんの
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
/// （336）であり、前スコープの幅（434）ではない（隣接・隙間 0・scg 2.1/2.2）。
/// これにより scope1 の右端 1150+336=1486 が scope0 の左端 1486 と一致する。
///
/// バルーン寸が scope 別になっても**基本位置**が動かないのは、右置き（scope1）の基準 x が
/// キャラ窓の右端＝バルーン幅に依存しないためである（resolver P5 は無改変）。
///
/// **task 4.3（要件 3.2/7.2）＋ task 6.1 の実機確定（R7.6）**: 基本位置へ当該 scope の
/// `windowposition` 由来の調整量が合流する。x は**左右で反転しない素の画面座標オフセット**
/// （SSP 実測で確定・`windowposition.rs` の `to_screen_adjust`「符号規約」節）ゆえ、
/// k₀=1/1 の本検体では wp 生値がそのまま加算される:
/// - scope0（`balloons0s.txt` の `266,-129`・side=Left）→ 調整量 (+266,−129)
///   → balloon=(1086+266, 353−129)=(1352,224)・offset=(−400+266, −129)=(−134,−129)
/// - scope1（`balloonk0s.txt` の `-190,-75`・side=Right）→ 調整量 (−190,−75)
///   → balloon=(1486−190, 640−75)=(1296,565)・offset=(336−190, −75)=(146,−75)
///
/// 恒等式 `balloon_offset ≡ balloon_pos − char_pos` は両 scope で保たれている
/// （P5 の加算入力が増えただけ）——これは配置式の出力時点の事後条件であり、
/// `windowposition.limit` の補正は下流の関門が持つ（windowposition-limit DD6）。
/// SSP 実測との数値突合は
/// [`prepare_emo2_matches_ssp_balloon_offsets_at_dpi_120`]（実機と同じ k=5/4）が担う。
#[test]
fn prepare_emo2_returns_two_scope_placements() {
    with_com_initialized(|| {
        // primary DPI 96＝emo2 の作者基準 DPI（無宣言＝96）ゆえ k₀=1/1（要件 7.2 の錨）。
        let p =
            prepare_ghost_windows_with_work_area(&emo2_root(), &balloon_root(), WA, Some(96))
                .expect("emo2 fixture の配置準備は成功する");

        assert_eq!(p.placements.len(), 2, "emo2 は 2 スコープ（DD6）");

        let s0 = &p.placements[0];
        assert_eq!(s0.scope, 0);
        assert_eq!(s0.char_pos, PointPx { x: 1486, y: 353 });
        assert_eq!(s0.char_size, SizePx { w: 434, h: 687 });
        assert_eq!(
            s0.balloon_pos,
            PointPx { x: 1352, y: 224 },
            "本体側は balloons0s.txt の windowposition 266,-129 を反映（要件 3.2）"
        );
        assert_eq!(s0.balloon_size, SizePx { w: 400, h: 224 });
        assert_eq!(s0.balloon_offset, PointPx { x: -134, y: -129 });

        let s1 = &p.placements[1];
        assert_eq!(s1.scope, 1);
        assert_eq!(s1.char_pos, PointPx { x: 1150, y: 640 });
        assert_eq!(s1.char_size, SizePx { w: 336, h: 400 });
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

        // 恒等式（resolver.rs の**出力時点**の事後条件・windowposition-limit DD6）は
        // 供給元が増えても保たれる。本検体は配置準備の出力＝下流の関門より上流ゆえ、
        // limit 補正は 1 度も掛かっていない。
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

        // titles は MountModel.names 由来（source T-I5 と同値）
        assert_eq!(p.titles.title(0), "むらさき");
        assert_eq!(p.titles.title(1), "エモ");
    });
}

/// **要件 2.3 の決定論形**（design C3 の新設檻）: 実機実測と同条件——DPI 120（k=5/4）・
/// scope0 543×859・scope1 420×500——で既定配置したとき、相方（scope1）の右端が
/// 本体（scope0）の左端と一致する（隣接・gap 0・scg 2.1/2.2）。
///
/// **前提錨**: 先に両 scope の実表示寸が実機観測値（543×859／420×500）であることを
/// 確認する。寸が変われば以下の隣接 assert は別の幾何を検定することになるため、
/// 黙って追随せず前提の側で落ちる形にしておく。
///
/// **許容差は置かない（厳密等式）**。丸めは `ScaleRatio` 経由の**寸法算出**にのみ現れ、
/// 位置の算出（`char_x(1) = char_x(0) − w(1)`・クランプ）は整数演算のみゆえ成分誤差が
/// 生じ得ない。要件 2.3 の許容差 1px は実機ログ側の判定に適用される別チャネルである。
#[test]
fn prepare_emo2_at_dpi_120_places_scopes_adjacent() {
    with_com_initialized(|| {
        // 実機と同じ DPI 120（emo2 の作者基準 DPI は 96 ゆえ k=5/4）。
        let p =
            prepare_ghost_windows_with_work_area(&emo2_root(), &balloon_root(), WA, Some(120))
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
            "実機同条件（DPI 120）で scope1 右端＝scope0 左端（隣接・gap 0（scg 2.1/2.2））"
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
        assert_ne!(
            s1.char_pos.x,
            s0.char_pos.x - s0.char_size.w,
            "前スコープ幅を引く旧式へ戻ってはならない（幅差 543−420＝123px の隙間・scg 2.1/2.2）"
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
// Send 契約・失敗経路・永続化なし（2.11）
// ------------------------------------------------------------------

/// `PreparedPlacement` は Send（design「main.rs seam」: Send な結果のみ返す）。
#[test]
fn prepared_placement_is_send() {
    fn assert_send<T: Send>() {}
    assert_send::<PreparedPlacement>();
}

/// 準備段階の失敗（ghost_root 不在→Mount）はこの関数内でフォールバックせず
/// `PlacementError` として呼び手（シーム）へ返る（DD14 の分担）。
/// 失敗は load（モニタ列挙より前）で起きるため headless でも決定論。
#[test]
fn prepare_missing_root_returns_mount_err_to_caller() {
    let root = std::env::temp_dir()
        .join("areka_placement_prepare_missing_root")
        .join("no_such_ghost");
    let err =
        prepare_ghost_windows(&root, &balloon_root()).expect_err("不在 root は Err");
    assert!(
        matches!(err, PlacementError::Mount(_)),
        "Mount variant 以外が返った: {err:?}"
    );
}

/// `prepare_ghost_windows` は永続を一切読み書きしない（2.11・A1）:
/// (a) 実行後どこにも ghost.dat を生成しない（書き込みなし）
/// (b) 偽の保存位置を持つ ghost.dat を plant しても出力が変わらない（読み込みなし）
///
/// 永続ストアの実体は現在、統一プロパティシステム（sylphya）の別ファイル
/// `sylphya.toml` にある——これは本 spec（areka-P0-position-persist）が
/// **消費**する別系統であり、SSP 由来の `ghost.dat` とは無関係。復元の結線は
/// placement シームの [`persist`] モジュール（と後続タスクの merge シーム）が担い、
/// `prepare_ghost_windows` 自身は依然として永続を一切読み書きしない（A1＝prepare
/// 不触は真のまま）。本檻は legacy な ghost.dat probe でその不触性を固定し続ける
/// （sylphya.toml 系統でも同じく prepare は不触）。
#[test]
fn prepare_never_reads_or_writes_ghost_dat() {
    with_com_initialized(|| {
        // 最小の合成ゴースト（emo2 実 PNG を 2 枚だけ複写・descript は emo2 同値キー）
        let root = TempDir::new();
        let ghost_master = root.path().join("ghost").join("master");
        let shell_master = root.path().join("shell").join("master");
        fs::create_dir_all(&ghost_master).expect("create ghost/master");
        fs::create_dir_all(&shell_master).expect("create shell/master");
        fs::write(
            ghost_master.join("descript.txt"),
            "charset,UTF-8\nname,えも\nsakura.name,むらさき\nkero.name,エモ\n",
        )
        .expect("ghost descript");
        fs::write(
            shell_master.join("descript.txt"),
            "charset,UTF-8\nseriko.alignmenttodesktop,bottom\nsakura.defaultx,0\nkero.defaultx,0\nsakura.balloon.alignment,left\nkero.balloon.alignment,right\n",
        )
        .expect("shell descript");
        fs::write(
            shell_master.join("surfaces.txt"),
            "surface0\n{\nelement0,overlay,surface0.png,0,0\n}\nsurface10\n{\nelement0,overlay,surface10.png,0,0\n}\n",
        )
        .expect("surfaces.txt");
        fs::copy(
            emo2_root().join("shell/master/surface0.png"),
            shell_master.join("surface0.png"),
        )
        .expect("surface0.png 複写");
        fs::copy(
            emo2_root().join("shell/master/surface10.png"),
            shell_master.join("surface10.png"),
        )
        .expect("surface10.png 複写");

        let before =
            prepare_ghost_windows_with_work_area(root.path(), &balloon_root(), WA, Some(96))
                .expect("合成ゴーストの準備は成功する");

        // (a) 書き込みなし: ghost.dat がどこにも生成されていない
        for dir in [root.path(), ghost_master.as_path(), shell_master.as_path()] {
            assert!(
                !dir.join("ghost.dat").exists(),
                "ghost.dat が生成された: {}",
                dir.display()
            );
        }

        // (b) 読み込みなし: 偽の保存位置を plant しても出力不変（復元しない）
        let junk = "position.0.x,9999\r\nposition.0.y,9999\r\n";
        fs::write(root.path().join("ghost.dat"), junk).expect("plant root ghost.dat");
        fs::write(ghost_master.join("ghost.dat"), junk).expect("plant ghost ghost.dat");

        let after =
            prepare_ghost_windows_with_work_area(root.path(), &balloon_root(), WA, Some(96))
                .expect("plant 後も準備は成功する");
        assert_eq!(
            before.placements, after.placements,
            "ghost.dat の有無で配置が変わった（復元経路が存在する疑い・2.11 違反）"
        );
        assert_eq!(before.titles, after.titles);
    });
}

// ------------------------------------------------------------------
// 起動時 k₀ の導出（task 4.3・design D7／Flow 3 手順2・要件 1.4/3.3）
// ------------------------------------------------------------------

/// k₀ = primary モニタ DPI ÷ 作者基準 DPI（既約有理・連続 k・D2/D7）。
///
/// 整数倍（192/96=2/1）だけでなく **非整数倍**（120/96=5/4・112/96=7/6）も
/// 既約有理数として正しく導出されることを固定する（整数段階へ丸めない・要件 2.2）。
#[test]
fn build_measure_scaling_derives_k0_from_primary_and_author_dpi() {
    let author = AuthorDpi::DEFAULT; // 96/96（emo2 fixture と同じ無宣言ケース）

    let two = build_measure_scaling(Some(192), author);
    assert_eq!(two.shell, k(2, 1), "192/96 は既約 2/1");
    assert_eq!(two.balloon, k(2, 1));

    let five_quarters = build_measure_scaling(Some(120), author);
    assert_eq!(five_quarters.shell, k(5, 4), "120/96 は既約 5/4（125%）");

    let seven_sixths = build_measure_scaling(Some(112), author);
    assert_eq!(seven_sixths.shell, k(7, 6), "112/96 は既約 7/6（非整数 k）");

    let identity = build_measure_scaling(Some(96), author);
    assert_eq!(
        identity.shell,
        ScaleRatio::ONE,
        "primary DPI＝作者基準 DPI は恒等 k=1/1（要件 1.3/7.2）"
    );
    assert_eq!(identity.balloon, ScaleRatio::ONE);
}

/// shell と balloon は**それぞれ自分の**作者基準 DPI で k₀ を得る（2 軸独立・要件 1.1）。
///
/// 取り違え（shell の k をバルーンへ適用する／2 値を入れ替える）は
/// コンパイルを通ってしまう静かな誤表示ゆえ、両軸に**異なる**宣言値を与えて檻に入れる。
#[test]
fn build_measure_scaling_uses_each_axis_own_author_dpi() {
    let scaling = build_measure_scaling(
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
    let swapped = build_measure_scaling(
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

/// primary モニタ DPI 取得不能（`None`／0）は **96 相当**へ縮退する
/// （design「Error Handling」の boot 行・要件 1.4）。恒等を一律に返すのではなく、
/// 96 を分子として作者基準 DPI との比を取る（作者が 96 以外を宣言していれば k₀≠1）。
#[test]
fn build_measure_scaling_degrades_to_96_equivalent_when_primary_dpi_unobtainable() {
    for missing in [None, Some(0)] {
        let default_author = build_measure_scaling(missing, AuthorDpi::DEFAULT);
        assert_eq!(
            default_author.shell,
            ScaleRatio::ONE,
            "author 96 に対する 96 相当は恒等（従来の native 採寸・{missing:?}）"
        );
        assert_eq!(default_author.balloon, ScaleRatio::ONE);

        let declared = build_measure_scaling(
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

/// 要件 3.3／7.2 の錨（実 fixture・偽装境界の end-to-end）: 注入した primary DPI から
/// 導かれた k₀ 倍後の**物理窓寸**で `ScopePlacement` が組み上がる。
///
/// - primary 96（＝emo2 の作者基準 DPI）: 既存期待値のまま（434×687 等・要件 7.2）
/// - primary 192: 全寸がちょうど 2 倍
/// - primary 112（k=7/6・非整数）: 丸めは単一権威 `scaled_extent` に一致する。
///   とくに 687×7/6 は **ちょうど 801.5** ゆえ、round half away from zero なら 802・
///   切り捨て（`len*num/den`）なら 801 に分かれる（丸め規約の取り違え検出）。
///   f32 近道の検出は別檻 [`prepare_k0_rounding_matches_integer_authority_not_f32`]
///   が担う（本検体は f32 でも偶然 802 に一致するため判別力を持たない）。
#[test]
fn prepare_emo2_scales_window_sizes_by_k0() {
    with_com_initialized(|| {
        let native =
            prepare_ghost_windows_with_work_area(&emo2_root(), &balloon_root(), WA, Some(96))
                .expect("k₀=1/1 の準備は成功する");
        assert_eq!(
            native.placements[0].char_size,
            SizePx {
                w: SCOPE0_W,
                h: SCOPE0_H
            },
            "primary DPI＝作者基準 DPI では既存の native 寸のまま（要件 7.2）"
        );

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
        // 原寸の 2 倍であり、全 placement で同一ではない（要件 3.1）。
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

/// 要件 1.4: primary モニタ DPI が取得できなくても**採寸は成立し窓寸が得られる**
/// （表示を失わない縮退）。96 相当ゆえ emo2（author 96）では native 寸に一致する。
#[test]
fn prepare_emo2_without_primary_dpi_still_measures() {
    with_com_initialized(|| {
        let degraded =
            prepare_ghost_windows_with_work_area(&emo2_root(), &balloon_root(), WA, None)
                .expect("primary DPI 不明でも準備は成功する（表示を失わない）");
        assert_eq!(
            degraded.placements[0].char_size,
            SizePx {
                w: SCOPE0_W,
                h: SCOPE0_H
            },
            "96 相当の k₀ ＝ 従来と同じ native 採寸"
        );
        assert_eq!(degraded.placements.len(), 2);
    });
}

/// **宣言された**作者基準 DPI が (a) 採寸 k₀ と (b) attach 搬送値の双方へ効くこと
/// （design Flow 3 手順1〜3・要件 1.1/3.3）。
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
        let (ghost_root, balloon_dir) = synth_declared_dpi_ghost(&root, "120", "144", None);

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
        // 本体側系列へ収束する（要件 3.7 の後方互換——`balloonk*` 不在なら全 scope 同一寸）。
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

/// f32 近道の混入検出（D4・記憶 areka の丸め単一権威）: `scaled_extent` の整数演算と
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
        let (ghost_root, balloon_dir) = synth_declared_dpi_ghost(&root, "168", "96", None);

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


/// design Flow 3 手順1（**1 度だけ読む**）: 準備が読んだ作者基準 DPI が
/// `PreparedPlacement` に載って attach 側（`wire_emo2_boot`→`attach_target`）へ渡る。
///
/// 単一読取の証拠は「読取器（task 2.1）を直接呼んだ値と、準備の戻り値が一致すること」。
/// 本番 fixture（emo2）は 96/96 ゆえ既定との区別が付かない——宣言値との区別は
/// [`prepare_declared_author_dpi_drives_each_axis_k0_and_attach_value`] が担う。
#[test]
fn prepare_emo2_carries_author_dpi_read_once_for_attach() {
    with_com_initialized(|| {
        let src = source::load_descript_source(&emo2_root()).expect("emo2 fixture は読める");
        let expected = AuthorDpi {
            shell: src.shell_author_dpi(),
            balloon: source::load_balloon_author_dpi(&balloon_root()),
        };
        assert_eq!(
            expected,
            AuthorDpi {
                shell: 96,
                balloon: 96
            },
            "emo2 は shell/balloon とも DPI 無宣言＝正典既定 96（task 2.1 実測）"
        );

        let p =
            prepare_ghost_windows_with_work_area(&emo2_root(), &balloon_root(), WA, Some(192))
                .expect("準備は成功する");
        assert_eq!(
            p.author_dpi, expected,
            "採寸 k₀ に使った宣言値がそのまま attach 側へ搬送される（読取は 1 度）"
        );
    });
}
