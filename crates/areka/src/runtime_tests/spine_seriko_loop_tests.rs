use super::test_support::{bump_char_window_dpi, drain_settled, opaque_count, variant_name};
use std::cell::Cell;

use super::{
    LoopRng, PresentCommand, Progress, SpineHarness, run_dpi_phase, shell_target, wait_until,
    wait_until_paced,
};

/// 常時発火 rng（1/N 抽選で必ず 0 を返す＝毎境界で抽選通過・`areka-seriko` の `looper_tests.rs` の `always_fire` と同旨）。
///
/// 「固定注入乱数列」の最小形（発見的 entropy 非依存）。毎境界で抽選を通すので定数 0 で足りる
/// （発火の順と回数は下の golden が固定する）。
fn always_fire_rng() -> LoopRng {
    Box::new(|_bound: u32| 0)
}

// ===========================================================================
// 実 emo2 まばたき 1 周の PresentCommand 列 golden（kero/sakura）＋ fixture 既定 OFF の対照
//
// `boot_live`/`inject_seriko_tick`/`always_fire_rng` の上に、実 emo2 fixture の実測アニメ
// （kero surface2100 `interval,random,4` 2106/2110/-1・sakura surface1000 `interval,bind+random,4`
// 1400 系 1412/1411/1410）を固定注入乱数＋scripted `send_tick` で 1 周歩かせ、発行される
// `PresentCommand` 列の **厳密 golden**（pattern 搬送 surface id の完全一致）を檻に入れる。
// あわせて「fixture 既定 OFF」の対照——`\![bind]` を通さない既定では always_fire でも bind
// ゲートが抽選を塞ぎ **一切発行しない**——を負の檻として固定する。
//
// 全て決定論（実 emo2 asset fixture＋固定注入 rng＋scripted send_tick・Close→join）。
// 時刻は注入で決まる（待ちの休みは時刻を進めない・`areka-runtime` の `spine_wait.rs`）。
// 実 SHIORI/pasta 非依存（surface は注入 cue で表示・実機/実 DPI サインオフは別）。
// ===========================================================================

/// `cmd` が scope0 shell 宛の `ShowSurface{shown_surface}` であることを検証し、その pattern が
/// `anim_id` に載せる現在コマ surface id を返す（コマ不在＝ベース復帰は `None`）。
///
/// golden の各 tick が運ぶのは常に「表示中 surface（`shown_surface`）の ShowSurface に、まばたき
/// アニメの現在コマを `pattern` へ載せたもの」。表示対象（偶数 TargetId＝shell）と表示中 surface の
/// 透過を都度 assert し、可変部（pattern のコマ surface id）を返して呼び手が golden 照合する。
fn shell_pattern_frame(cmd: &PresentCommand, shown_surface: u32, anim_id: u32) -> Option<u32> {
    match cmd {
        PresentCommand::ShowSurface {
            target,
            surface_id,
            pattern,
            ..
        } => {
            assert_eq!(
                *target,
                shell_target(0),
                "shell 表示対象（scope0・偶数 TargetId）"
            );
            assert_eq!(
                *surface_id, shown_surface,
                "表示中 surface（seriko 数値解決の透過・pattern はこの面のアニメに従属）"
            );
            pattern.get(anim_id).map(|f| f.surface_id)
        }
        other => panic!(
            "ShowSurface を期待（golden の各 tick は面表示指令）: {}",
            variant_name(other)
        ),
    }
}

/// dispatcher tick で OnBoot talk を駆動し、scope0 shell が `surface_id` を表示する（必要なら binds に
/// `require_bind` を含む）まで有界スピンする。観測できた指令は drain 済みゆえ、復帰後の rx は talk 由来
/// 指令について空——以降 `inject_seriko_tick` が発行する loop 指令だけを純粋に観測できる。
///
/// `apply` が真なら汲んだ指令を**実 presenter へ適用**する。ループ継続の檻は「表示が生きていること」を
/// readback で見るので、適用まで要る。偽なら指令は観測するだけで捨てる。
///
/// `require_bind` は `\![bind,...]` 貫通の証跡: bind 適用は表示中 scope で **binds 更新済みの Show 再発行**
/// （`apply_bind`→`BindApplyOutcome::Changed`）を生むため、「shell surface が該当 bind id を含んで表示された」
/// = current_binds へ当該 id が書き込まれた（＝bind ゲートが ON になった）ことの end-to-end 証跡になる。
///
/// # 有界性は進みの目印つきの [`wait_until_paced`]
///
/// 本関数は**実 async（SHIORI/pasta アクタ→seriko→[`PresentBridge`]）の到着をラッチで待つ待機
/// ループ**である。1 反復は「tick 注入＋drain」だけで極めて安価ゆえ、反復回数の上限は CPU 競合下
/// （大規模再ビルド直後の再スキャン圧など）で**数 ms で尽き**、製品コードが正常でも早合点で赤になる
/// （偽陽性）。ゆえに打ち切りは [`wait_until_paced`] の進みの目印（SHIORI の呼び出しの数と汲んだ指令の数の和）で
/// 決める（進まないまま待ちの芯の上限に届けば［止まった］）。Tick を入れる輪なので休みは
/// [`wait_until_paced`] が持ち、条件の中では眠らない（`areka-runtime` の `spine_wait.rs`）。
///
/// **目印を絞らない理由**: `LoopDriver::Live` でも loop ticker は起こさず、この輪は seriko の tick を
/// 注入しない。ゆえにまばたきの指令は出ず、汲む指令は OnBoot talk の有限の列（条件の面へ向かう歩み）
/// だけになる。受けた数がそのまま条件に向かう進みなので、面の数などへ絞る必要が無い。
///
/// **areka-P0-kero-balloon 要件 7.8（注入模擬時刻が観測窓を追い越して観測条件を壊すクラス）とは別クラス**であり、時刻の頭打ちでは
/// 直らない。判別根拠は 2 点——(a) 観測 `satisfied` は**ラッチ**で、一度成立したら後続 cue に壊されない。
/// (b) 本関数の呼出点の台本はいずれも `\s[...]\e` 系で `\w`／`\c` を含まず**全 cue が `at=0.0`** ＝観測を
/// 壊す後続 cue が構造的に存在しない。ゆえに注入時刻の前進は観測に有利にしか働かず、頭打ちは不要。
fn drive_shell_shown(
    harness: &mut SpineHarness,
    surface_id: u32,
    require_bind: Option<u32>,
    apply: bool,
) {
    let what = "OnBoot talk の scope0 shell の面の表示";
    let mut satisfied = false;
    let received = Cell::new(0u64);
    let shiori = harness.shiori_handle.clone();
    let probe = || shiori.call_count() + received.get();
    let mut now = 0u64;
    wait_until_paced(what, Progress::Count(&probe), || {
        now += 1;
        harness.inject_dispatcher_tick(now);
        for cmd in harness.wiring.drain_received() {
            received.set(received.get() + 1);
            if let PresentCommand::ShowSurface {
                target,
                surface_id: sid,
                binds,
                ..
            } = &cmd
                && *target == shell_target(0)
                && *sid == surface_id
                && require_bind.is_none_or(|id| binds.contains(id))
            {
                satisfied = true;
            }
            if apply {
                harness.wiring.apply_present(&mut harness.world, cmd);
            }
        }
        satisfied
    })
    .unwrap_or_else(|failure| {
        panic!(
            "{failure}／{what}: surface {surface_id}（require_bind={require_bind:?}）が出ない（boot→talk→sink 経路不通 or bind 未貫通）"
        )
    });
}

/// `now_ms` の seriko tick を 1 発直接注入し、最初の `PresentCommand` が rx へ届くまで有界
/// 待ち（[`wait_until`]・読むだけの待ち）、続けて [`drain_settled`] で残りを汲み切って
/// から「ちょうど 1 件」を確かめて返す。golden の各コマ遷移 tick は変化 1 件を発行するため、
/// この直列注入→回収で発行列の順序と内容を決定論的に照合できる。最初の 1 件で汲むのを止めると 2 件目を
/// 見ないので、余分な指令が出る退行を通してしまう。届かなければ件数 assert が落ちる（hang しない）。打ち切りは反復回数でなく [`wait_until`] の
/// 進みの目印（注入は 1 発のみで各反復は系を進めない純粋なポーリング＝反復は経過時間の代理にならない）。
/// 観測 `buf` は累積＝ラッチゆえ注入時刻の追い越しは構造的に無い。
fn seriko_tick_expect_one(harness: &mut SpineHarness, now_ms: u64) -> PresentCommand {
    harness.inject_seriko_tick(now_ms);
    let mut buf: Vec<PresentCommand> = Vec::new();
    // 目印は受けた指令の数（届けば即座に満たされるので、打ち切りは 30 秒届かなかったときの［止まった］）。
    let received = Cell::new(0u64);
    let _ = wait_until(
        "seriko の 1 tick の指令",
        Progress::Count(&|| received.get()),
        || {
            buf.extend(harness.wiring.drain_received());
            received.set(buf.len() as u64);
            !buf.is_empty()
        },
    )
    .inspect_err(|failure| eprintln!("{failure}"));
    drain_settled(harness, &mut buf);
    assert_eq!(
        buf.len(),
        1,
        "golden の各 tick はちょうど 1 指令を発行する（now_ms={now_ms}・実受信 {} 件・variants={:?}）",
        buf.len(),
        buf.iter().map(variant_name).collect::<Vec<_>>()
    );
    buf.into_iter().next().expect("len==1 を確認済み")
}

/// spine E2E-1（kero まばたき 1 周 golden）: 実 emo2 shell 表
/// ＋固定注入乱数（常時発火）で `boot_live` し、`\s[2100]`（kero まばたき surface・`interval,random,4`・
/// bind 非依存）を注入 cue で表示させたのち、`SerikoSink::send_tick` を直接注入して 1000ms 絶対グリッド
/// 境界を跨がせ 1 周を歩かせる。発行 `PresentCommand` 列の **厳密 golden**——pattern0=2106 → pattern1=2110
/// → `-1`（ベース復帰＝空 pattern）——を実 fixture 実測値（surfaces.txt `surface.append10,2100` の
/// `animation0` 2106(w0)/2110(w40)/-1(w80)・t=[0,40,120]）で固定する。
///
/// send_tick→LoopRuntime→emit_display→adapter→rx の end-to-end 配線が、実 emo2 表の実測列を
/// 忠実に運ぶことの決定論自動檻（実 SHIORI/pasta・GUI 非依存・実機/実 DPI サインオフは別）。
#[test]
fn spine_e2e_kero_blink_one_cycle_golden() {
    // 実表＋常時発火固定 rng でループ活性化（既存 spine テストは Inert＝非退行・本テストのみ Live）。
    let mut harness = SpineHarness::boot_live(
        r"\s[2100]\e",
        always_fire_rng(),
        areka_seriko::seeded_rng(0),
    );
    // 表示中ゲート前提: surface2100 を scope0 shell へ表示（seriko ScopeStates に記録）。bind 不要（Random）。
    drive_shell_shown(&mut harness, 2100, None, false);

    // 起動 tick（境界初期化・非跨ぎ・無発行）。以降は境界 1000 のみを跨いで 1 周を歩く（2000 未到達＝再抽選なし）。
    harness.inject_seriko_tick(0);

    // ── 1 周 golden（kero・-1 終端）: 2106 → 2110 → ベース復帰（空 pattern）。実 fixture t=[0,40,120]。 ──
    let c1 = seriko_tick_expect_one(&mut harness, 1000); // 発火＋elapsed0 → pattern0
    assert_eq!(
        shell_pattern_frame(&c1, 2100, 0),
        Some(2106),
        "kero 1 周 pattern0=2106（実 fixture surface.append10,2100 animation0）"
    );
    let c2 = seriko_tick_expect_one(&mut harness, 1040); // elapsed40 → pattern1
    assert_eq!(
        shell_pattern_frame(&c2, 2100, 0),
        Some(2110),
        "kero pattern1=2110（wait40・実 fixture）"
    );
    let c3 = seriko_tick_expect_one(&mut harness, 1120); // elapsed120 → -1 Stopped → ベース復帰
    assert_eq!(
        shell_pattern_frame(&c3, 2100, 0),
        None,
        "kero pattern3=-1 到達でコマ除去＝ベース復帰"
    );
    // ベース復帰は「空 pattern の ShowSurface」——sakura の末尾残留（1410 が残る）との決定的な対照点。
    match &c3 {
        PresentCommand::ShowSurface { pattern, .. } => assert!(
            pattern.is_empty(),
            "kero の -1 ベース復帰は空 pattern（sakura 残留と対照）"
        ),
        other => panic!("ShowSurface を期待: {}", variant_name(other)),
    }

    harness.shutdown_bounded();
}

/// spine E2E-2a（sakura まばたき 1 周 golden・bind ON 貫通）: 実 emo2
/// shell 表＋固定注入乱数（常時発火）で `boot_live` し、`\s[1000]`（sakura 着せ替え surface・`bind+random,4`
/// のまばたき animation1400 を持つ）＋`\![bind,まばたき,通常,1]`（実 sakura bindgroup 貫通）を OnBoot talk で
/// 流して 1400 bindgroup を **ON** にしたのち、`send_tick` 直接注入で 1 周を歩かせる。発行 `PresentCommand` 列の
/// **厳密 golden**——pattern1=1412 → pattern2=1411 → pattern3=1410（**残留**・`-1` なし）——を実 fixture 実測値
/// （surfaces.txt surface1000 `animation1400` 1412(w0)/1411(w150)/1410(w22)・t=[0,150,172]）で固定する。
///
/// `\![bind,まばたき,通常,1]` は OnBoot talk 内で sakura compile → Custom キャリア cue → broadcast →
/// seriko `apply_bind`（`bind_resolver.resolve(Sakura,"まばたき","通常")==1400`）で current_binds へ 1400 を
/// 書き込む実経路を通る（mayuna 成果物・read-only 参照）。`drive_shell_shown(.., Some(1400))` が binds に 1400 を
/// 含む Show 再発行の観測で貫通を担保する。kero（`-1`→空 pattern）と対照的に末尾コマが残留する。
#[test]
fn spine_e2e_sakura_blink_after_bind_one_cycle_golden() {
    // \s[1000]（sakura 着せ替え surface）＋ \![bind,まばたき,通常,1]（1400 bindgroup ON）。常時発火 rng で
    // ループ活性化（bind ゲート ON の 1400 のみ発火・半目 1401/ジトー 1402 は OFF ゆえ非発火）。
    let mut harness = SpineHarness::boot_live(
        r"\s[1000]\![bind,まばたき,通常,1]\e",
        always_fire_rng(),
        areka_seriko::seeded_rng(0),
    );
    // 表示中＋bind ON 前提: scope0 shell surface1000 が binds に 1400 を含んで表示される
    // （\![bind] 貫通で current_binds へ 1400 が書き込まれた end-to-end 証跡＝bind 再発行 Show）。
    drive_shell_shown(&mut harness, 1000, Some(1400), false);

    // 起動 tick（境界初期化）。以降は境界 1000 のみを跨ぎ 1 周を歩く（2000 未到達＝再抽選なし）。
    harness.inject_seriko_tick(0);

    // ── 1 周 golden（sakura・末尾残留）: 1412 → 1411 → 1410（残留・-1 なし）。実 fixture t=[0,150,172]。 ──
    let c1 = seriko_tick_expect_one(&mut harness, 1000); // 発火＋elapsed0 → pattern1
    assert_eq!(
        shell_pattern_frame(&c1, 1000, 1400),
        Some(1412),
        "sakura 1 周 pattern1=1412（実 fixture surface1000 animation1400・先頭 wait0）"
    );
    let c2 = seriko_tick_expect_one(&mut harness, 1150); // elapsed150 → pattern2
    assert_eq!(
        shell_pattern_frame(&c2, 1000, 1400),
        Some(1411),
        "sakura pattern2=1411（wait150・実 fixture）"
    );
    let c3 = seriko_tick_expect_one(&mut harness, 1172); // elapsed172 → pattern3 末尾非負 → 残留
    assert_eq!(
        shell_pattern_frame(&c3, 1000, 1400),
        Some(1410),
        "sakura pattern3=1410 残留（-1 なし末尾＝FinishedResidual）"
    );
    // 末尾は残留ゆえ空でない——kero の -1 ベース復帰（空 pattern）との決定的な対照点。
    match &c3 {
        PresentCommand::ShowSurface { pattern, .. } => assert!(
            !pattern.is_empty(),
            "sakura の末尾は最終コマ残留（空でない・kero の -1 と対照）"
        ),
        other => panic!("ShowSurface を期待: {}", variant_name(other)),
    }

    harness.shutdown_bounded();
}

/// spine E2E-2b（「fixture 既定 OFF」の対照檻）: fixture 既定
/// （`\![bind]` を **通さない**＝まばたき bindgroup 1400/1401/1402 は全 OFF）で surface1000 を表示し、
/// 常時発火 rng で境界を複数跨いでも **一切発行しない**ことを固定する。bind ゲート（`BindRandom` は
/// bindgroup ON のときだけ `should_fire` を呼ぶ）が抽選そのものを塞ぐため、抽選 → 再生 → pattern
/// 搬送が起きない。
///
/// # always_fire で「ゲートが塞ぐ」を積極証明する
///
/// 乱数を常時発火（1/N 抽選が呼ばれれば必ず通過）にしておくことで、もし bind ゲートが leak すれば
/// 1400/1401/1402 は **必ず**発火し pattern 搬送指令が現れる。それが現れない＝発行ゼロは「抽選が呼ばれて
/// いない（ゲートが塞いだ）」ことの証跡になる（sakura bind ON 版〔上〕が同じ rng+tick で発火するのと対照）。
/// surface1000 のまばたきは全て `bind+random`（無条件 `random` は無い）ため、既定 OFF では何も動かない。
#[test]
fn spine_e2e_sakura_blink_default_off_emits_nothing() {
    // fixture 既定（bind OFF・\![bind] なし）で surface1000 を表示。always_fire でもゲートが塞ぐ＝発行ゼロ。
    let mut harness = SpineHarness::boot_live(
        r"\s[1000]\e",
        always_fire_rng(),
        areka_seriko::seeded_rng(0),
    );
    drive_shell_shown(&mut harness, 1000, None, false); // bind なし＝1400/1401/1402 は全 OFF（既定）

    // 境界を複数跨ぐ seriko tick を注入し、発行が一切現れないことを固定する（起動 tick→1000/2000/…の境界跨ぎ）。
    harness.inject_seriko_tick(0); // 起動 tick（境界初期化）
    let mut emitted: Vec<PresentCommand> = Vec::new();
    for now in [1000u64, 2000, 3000, 4000, 5000] {
        harness.inject_seriko_tick(now); // 各々 1000ms 絶対グリッド境界を跨ぐ
        // 有界 settle drain（spine_s4 の負検証と同流儀）: 打ち切りは反復回数ではなく壁時計の
        // 最小持続と連続空観測で決まる。時刻注入は外側のこの `for` が担い、
        // ヘルパは回収しかしない＝時刻が観測を追い越さない。
        drain_settled(&mut harness, &mut emitted);
    }
    // 既定 OFF の檻: bind ゲート OFF は always_fire でも抽選を塞ぐ＝発行ゼロ（ゲート leak なら pattern 搬送指令が漏れる）。
    assert!(
        emitted.is_empty(),
        "既定 OFF: bind ゲート OFF は always_fire でも一切発行しない（ゲート leak 検出・実受信 {} 件・variants={:?}）",
        emitted.len(),
        emitted.iter().map(variant_name).collect::<Vec<_>>()
    );

    harness.shutdown_bounded();
}

// ===========================================================================
// areka-P0-emo-dpi-scaling 要件 4.3（進行中挙動が DPI 変化を跨いで失われない）の実経路檻
//
// 既存の DPI 変化 spine 2 本（`spine_dpi_change_refreshes_balloon_text_scale_on_real_attach`／
// `spine_dpi_change_while_balloon_hidden_lands_on_next_show`）はいずれも `SpineHarness::boot`
// ＝`LoopDriver::Inert` で起動しており、**活性ループと DPI 変化が一度も同居していない**。
// ゆえに areka-P0-emo-dpi-scaling 要件 4.3 の主張のうち (a) クラッシュ・表示消失なし／(b) 文字の状態保存は檻に
// 入っていたが、(c)「SERIKO ループが DPI 変化を跨いで進行し続ける」は
// 「ループ状態は presenter の外（seriko worker）にある」という**構造からの帰結**にとどまり、
// 一度も観測されていなかった。以下がその空白を閉じる。
// ===========================================================================

/// シェル target の適用 k（`applied_scale`）を読む短縮（`None`＝未表示は前提違反ゆえ panic）。
fn shell_applied_scale(harness: &SpineHarness, scope: u32) -> f32 {
    harness
        .wiring
        .presenter()
        .applied_scale(shell_target(scope))
        .expect("表示済みの shell target は適用 k を持つ")
}

/// シェル target の**物理寸権威**（`target_physical_size`＝`scaled_extent(applied, native)`）を読む短縮。
///
/// 読み戻し（`read_back`）は native 原寸ゆえ k が変わっても長さが変わらない
/// （`areka-P0-present-gpu-transform-scale` 要件 6.2）。「k が実描画へ届いたか」はこの照会で見る。
fn shell_physical_size(harness: &SpineHarness, scope: u32) -> (u32, u32) {
    harness
        .wiring
        .presenter()
        .target_physical_size(shell_target(scope))
        .expect("表示済みの shell target は物理寸を持つ")
}

/// seriko tick を 1 発注入して 1 件の指令を回収し、**実 presenter へ適用**したうえで、その指令が
/// 運んだ pattern の現在コマ surface id（コマ不在＝ベース復帰は `None`）と適用後の readback を返す。
fn seriko_tick_apply_one(
    harness: &mut SpineHarness,
    now_ms: u64,
    shown_surface: u32,
    anim_id: u32,
) -> (Option<u32>, Vec<u8>) {
    let cmd = seriko_tick_expect_one(harness, now_ms);
    let frame = shell_pattern_frame(&cmd, shown_surface, anim_id);
    harness.wiring.apply_present(&mut harness.world, cmd);
    let px = harness
        .wiring
        .read_back_target(shell_target(0))
        .unwrap_or_else(|e| {
            panic!("ループ指令適用後の shell scope0 read_back 失敗（表示が消えた）: {e:?}")
        });
    assert!(
        opaque_count(&px) > 0,
        "ループ指令適用後の shell readback が全透明（表示消失）: len={}",
        px.len()
    );
    (frame, px)
}

/// **活性 SERIKO ループを跨ぐ DPI 変化**（areka-P0-emo-dpi-scaling 要件 4.3 の残る主張 (c)）: 実 emo2 表＋常時発火
/// 固定 rng で `boot_live` し、kero まばたき（surface2100 `animation0`＝2106/2110/`-1`）を 1 周の**途中まで**
/// 歩かせた状態で**シェル窓の DPI を変える**。その後——
///
/// 1. DPI 変化直後の再表示が**進行中のコマ（2106）を載せたまま**新 k で成立する
///    （＝`refresh_scale` が `last_show` の pattern を捨てない＝進行中挙動を失わない）。
/// 2. ループは**リセットも停止もせず同じ 1 周の続き**（2110 → `-1` ベース復帰）を、実 fixture 実測の
///    golden どおりに発行し続ける（先頭 2106 へ戻らない・無発行にならない）。
/// 3. その全区間で shell の `read_back` が成立し全透明にならない（クラッシュ・表示消失なし）。
/// 4. DPI 変化以降の表示は一貫して**新 k の物理寸**（`target_physical_size`・旧 k の寸が残らない＝k 変化が
///    実描画へ届いている）。読み戻しバイト列は **native 原寸**のまま長さを変えない
///    （拡大は GPU の Visual 変換の領分・`areka-P0-present-gpu-transform-scale` 要件 6.2）。
///
/// # 既存 DPI 檻との差（なぜ本ケースが要るか）
///
/// 既存 2 本は `LoopDriver::Inert`（ループ完全不活性）で、変化させるのも**バルーン**窓の DPI である。
/// SERIKO ループが載るのはシェル表示スロットゆえ、活性ループ×シェル窓 DPI 変化という本番の組み合わせは
/// どこにも存在しなかった。本ケースはその 1 点だけを足す（(a)(b) の重複観測はしない）。
///
/// # 決定論（時刻は注入・乱数も注入）
///
/// tick は `send_tick` 直接注入（loop ticker 不起動）、乱数は常時発火の固定注入列。DPI は
/// [`bump_char_window_dpi`] が現在値と必ず異なる実機水準を選ぶ（「たまたま同値」で空虚化しない）。
///
/// # 実測の変異キル（2026-07-26 の実測）
///
/// - `EmoPresenter::refresh_scale` が再表示時に `last_show` の pattern を捨てて
///   `PatternState::default()` で `apply_show` する変異（＝DPI 変化で進行中の SERIKO コマを失う）:
///   `-p areka` では**本テストだけ**が落ち、落ちる assert も狙いどおり
///   「DPI 変化直後の再表示が進行中コマを失いベース面へ戻っている」である。同変異は
///   `-p areka-emo-present` では **1 本も落ちない**——`refresh_scale` の pattern 保存は
///   本テスト追加前まで repo 内のどの檻も観測していなかった（exclusive）。
/// - `apply_show` が k≠1 のとき `binds`／`pattern` を既定へ落とす変異では、本テストと
///   `areka-emo-present` の `show_surface_scales_layered_bind_and_pattern_content_with_single_k`
///   （同時追加檻）が**共倒れ**（shared）。ほかは 1 本も落ちない。
#[test]
fn spine_dpi_change_during_live_seriko_loop_keeps_loop_progressing() {
    // 実表＋常時発火固定 rng でループ活性化（既存 DPI 檻は Inert＝この組み合わせは本ケースが初）。
    let mut harness = SpineHarness::boot_live(
        r"\s[2100]\e",
        always_fire_rng(),
        areka_seriko::seeded_rng(0),
    );

    // 実 attach（視覚資源を本番経路で生成）→ 表示中ゲート成立まで talk を駆動して実適用する。
    harness.attach_all();
    drive_shell_shown(&mut harness, 2100, None, true);

    let base_k0 = harness
        .wiring
        .read_back_target(shell_target(0))
        .expect("前提: 初回 \\s[2100] 適用で shell の原寸面が確立される");
    assert!(
        opaque_count(&base_k0) > 0,
        "前提: DPI 変化前の shell が実描画されている"
    );
    let k_before = shell_applied_scale(&harness, 0);
    let physical_before = shell_physical_size(&harness, 0);

    // ── 1 周の途中まで歩かせる（起動 tick → 境界 1000 で発火＋elapsed0 → pattern0=2106） ──
    harness.inject_seriko_tick(0); // 起動 tick（境界初期化・非跨ぎ・無発行）
    let (frame1, frame1_k0) = seriko_tick_apply_one(&mut harness, 1000, 2100, 0);
    assert_eq!(
        frame1,
        Some(2106),
        "前提: DPI 変化前に kero 1 周の pattern0=2106 まで進んでいる（実 fixture animation0）"
    );
    assert_ne!(
        frame1_k0, base_k0,
        "前提: 進行中コマが実際に表示画素を変えている（変わらなければ以降の継続観測が空虚）"
    );

    // ── ここでシェル窓の DPI が変わる（モニタ跨ぎ・表示スケール変更の決定論的代替） ──
    let new_dpi = bump_char_window_dpi(&mut harness, 0);
    run_dpi_phase(&mut harness.wiring, &mut harness.world);
    let k_after = shell_applied_scale(&harness, 0);
    assert_ne!(
        k_after, k_before,
        "前提: DPI={new_dpi} で shell target の適用 k が実際に変わる（変わらなければ本ケースは空虚）"
    );

    // (3)(4) 表示は消えず、新 k の物理寸で載り直している。
    let frame1_k1 = harness
        .wiring
        .read_back_target(shell_target(0))
        .unwrap_or_else(|e| panic!("DPI 変化直後の shell read_back 失敗（表示消失）: {e:?}"));
    assert!(
        opaque_count(&frame1_k1) > 0,
        "DPI 変化直後の shell readback が全透明（表示消失）: len={}",
        frame1_k1.len()
    );
    // 読み戻しは **native 原寸**（拡大は GPU の Visual 変換・`areka-P0-present-gpu-transform-scale`
    // 要件 6.2）ゆえ、k が変わっても長さは変わらない。「k が実描画へ届いた」ことは物理寸権威
    // （`target_physical_size`＝`scaled_extent(applied, native)`）が変わることで見る。
    assert_eq!(
        frame1_k1.len(),
        frame1_k0.len(),
        "DPI 変化で読み戻し長が変わった（読み戻しは native 原寸のはず＝拡大が原寸バイト列へ漏れている）"
    );
    let physical_after = shell_physical_size(&harness, 0);
    assert_ne!(
        physical_after, physical_before,
        "DPI 変化後も旧 k の物理寸のまま（k が実描画へ届いていない）"
    );

    // ── (2) ループは同じ 1 周の続きを歩き続ける（先頭へ戻らない・止まらない） ──
    let (frame2, frame2_k1) = seriko_tick_apply_one(&mut harness, 1040, 2100, 0);
    assert_eq!(
        frame2,
        Some(2110),
        "DPI 変化を跨いでもループは同じ 1 周の続き（pattern1=2110）を発行する（先頭 2106 へ戻さない＝リセットなし）"
    );
    assert_eq!(
        shell_physical_size(&harness, 0),
        physical_after,
        "DPI 変化後のループ指令が旧 k の物理寸へ戻っている（k の一貫性が崩れた）"
    );
    assert_ne!(
        frame2_k1, frame1_k1,
        "コマ遷移が表示画素へ届いていない（ループは動いたが絵が更新されない）"
    );

    let (frame3, base_k1) = seriko_tick_apply_one(&mut harness, 1120, 2100, 0);
    assert_eq!(
        frame3, None,
        "DPI 変化を跨いだ 1 周が実 fixture どおり `-1` 終端（ベース復帰）まで到達する"
    );
    assert_eq!(
        shell_physical_size(&harness, 0),
        physical_after,
        "ベース復帰も新 k の物理寸で成立する"
    );

    // (1) DPI 変化直後の再表示が**進行中のコマを載せたまま**だったことの決定的な対照:
    //     同じ k のベース復帰の絵（base_k1）と異なる＝2106 のコマが再表示に生きていた。
    //     `refresh_scale` が pattern を捨てて素の面を出す実装なら、ここで両者が一致して落ちる。
    assert_ne!(
        frame1_k1, base_k1,
        "DPI 変化直後の再表示が進行中コマを失いベース面へ戻っている（進行中挙動の喪失）"
    );

    harness.shutdown_bounded();
}
