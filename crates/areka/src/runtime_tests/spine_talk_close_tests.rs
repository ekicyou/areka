use std::cell::Cell;

use super::test_support::{boot_finished, opaque_count, variant_name};
use super::{
    ActorKey, PresentCommand, Progress, RecordedCall, SpineHarness, run_text_phase, shell_target,
    wait_until, wait_until_paced,
};

// ===========================================================================
// task 6.3 spine 観測ケース（S2 talk→typewriter／S5 close 握手）
//
// 6.1 の `SpineHarness`＋6.2 の観測ヘルパの上に構築する。S2 は実 sink 経路の末端
// （seriko→shell 面切替の実描画 readback／emo-text typewriter の `present_frame` 駆動→
// text 供給面 readback）まで観測境界を延ばす（R8.2/R8.5）。時刻は注入 Tick と
// 注入 talk_time で決まる（R8.3）・実 GPU（MTA・R8.4）・x64 完結（R8.6）。
// ===========================================================================

/// 装着済み balloon text actor の供給面（emo-text `TextLayerRuntime::surface`）の非透明画素数。
///
/// 呼び手は装着の後（文字の面が作られた後）でしか使わない。供給面が無い・読み戻せないのは文字の相の
/// 壊れなので、0 と数えず panic で落とす（0 と数えると Clear の檻と pre-reveal の檻が空振りする）。
/// S2 の typewriter リビール観測（`present_frame` 駆動後の text 供給面 readback）に使う。`harness.runtime` は `wiring.runtime` と同一 `Rc<RefCell<..>>`（clone）ゆえ、
/// `run_text_phase`（`present_frame`）が更新した供給面をそのまま読み戻せる（借用は逐次・非重複）。
fn text_surface_opaque(harness: &SpineHarness, actor: &ActorKey) -> usize {
    let rt = harness.runtime.borrow();
    let bytes = rt
        .surface(actor)
        .expect("装着済み")
        .read_back()
        .expect("読み戻せる");
    opaque_count(&bytes)
}

/// spine S2（talk→typewriter・R2.1/2.2/2.3/2.4・R3.1・R8.2/R8.5）: `\s[2100]`（シェル面切替）＋
/// テキスト＋`\c`（Clear）を含む scripted OnBoot 台本を実 sink 経路で流し、
/// (1) 受信 `PresentCommand` 列に `ShowSurface{shell_target(0), surface_id:2100}` が現れ、apply 後の
/// shell readback が**非表示から surface2100 の実描画へ遷移**すること（初回面表示の実描画・R2.4/R3.1・
/// defect #5 ゆえ attach 時の初期 surface0 baseline は無い＝シェルは初回 `\s` まで非表示）、
/// (2) テキスト cue の typewriter リビールを注入 `talk_time` の階段値で駆動し、text 供給面の
/// `opaque_count` が**単一 talk 内で単調非減少**・pre-reveal（t=0.0）全透明・`Clear`（at=1.40）後の
/// 全域透明（R8.5・R2 系の檻）を檻に入れる。
///
/// # 二段配送で単一 talk 内のリビール→Clear を分離（talk_clock 既知制約に整合）
///
/// emo-text の cue は**到着即時適用**（state.rs `apply_cue`）: `Text` は追記＋per-glyph リビール時刻
/// `r_i=max(r_{i-1}+interval, at)`（`interval = cue.duration / N`・配送 duration 由来＝
/// areka-P0-cue-playback-duration で `char_wait` を撤去）確定、`Clear` は**配送即時にバッファ全消去**（時刻ゲートではない）。
/// リビールの時刻ゲートは `visible(t)=|{i:r_i≤t}|` のみ。よって単調非減少の階段は「Text 配送済み・
/// Clear 未配送」のバッファに注入 `talk_time`（clock 非経由・R8.3）を振って観測し（Phase 1）、その後
/// dispatcher の elapsed を Clear（`\_w[1000]`＝at=0.05+D+1.00=1.40・D=0.35）超へ進めて Clear を
/// 配送し全消去を観測する（Phase 2）。台本の `\_w[50]`（Text at=0.05）により t=0.0 は先頭グリフ r_0=0.05 未達で全透明。単調
/// 述語の適用範囲を単一 talk 内（Clear 配送前のリビール区間）に限定することで、talk 跨ぎの epoch
/// リベース逆行（talk_clock 既知制約）を対象外にする（設計 Testing Strategy S2）。`present_frame` は
/// 各 t で全域再描画（残渣なし・決定論・R7.3）ゆえ、注入 t に対し `opaque_count` は `visible(t)` の
/// 単調性をそのまま反映する。
///
/// # validrect 外非透明なし（best-effort・CONCERNS 相当）
///
/// text 供給面は validrect 寸ちょうどのクリップ面（draw_readback_test が「readback は validrect 寸の
/// BGRA 密配列＝validrect 外の画素は供給面に存在しない」を単体で固定済み）であり、非透明画素は構造上
/// validrect 内に閉じる。本 spine は実 balloon fixture 由来の validrect 寸を再導出せず、単調非減少＋
/// Clear 後全透明（R2 系の本質）を主檻とし、validrect 外非透明なしは供給面クリップの構造的帰結として
/// draw_readback_test の単体檻に委ねる（parent 指示の best-effort）。
#[test]
fn spine_s2_talk_drives_surface_switch_and_typewriter_reveal() {
    // \s[2100]（シェル面切替・actor "0"）→ \_w[50] 後にテキスト（typewriter・at=0.05・再生時間
    // D=7 文字×50ms=0.35s）→ \_w[1000] 後に \c（Clear・at=1.05+D=1.40）。Text と Clear の間に大きな
    // 待ちを置き、二段配送（Text のみ→Clear）で「単一 talk 内のリビール」→「Clear の全消去」を
    // 分離できるようにする。
    let mut harness = SpineHarness::boot(r"\s[2100]\_w[50]アヒルやアヒル\_w[1000]\c\e");
    let actor = ActorKey::from("0");

    // ── attach（shell/balloon 装着・text actor 登録）: S1 と同じ planned==attached==2 前提 ──
    harness.attach_all();

    // ── シェルは初回 `\s` cue まで非表示（defect #5・2026-07-13 実機#5）: `\s[2100]` 適用前の shell
    //    scope0 は未表示＝`read_back` Err（合成面なし＝透過）。attach で surface0 を焼き付けない。 ──
    assert!(
        harness.wiring.read_back_target(shell_target(0)).is_err(),
        "S2 前提: shell scope0 は初回 \\s cue 前は非表示（未表示・read_back Err・defect #5）"
    );

    // ── Phase 1: シェル面指令（\s[2100]）＋テキスト cue **のみ**を配送する。dispatcher は active talk へ
    //    最初に届いた Tick の now を base に焼き（dispatcher.rs `on_tick`: `base_now.get_or_insert(now)`）、
    //    以降 elapsed=(now−base)/1000 秒で start_time 順に cue を解放する。本台本の実タイムラインは
    //    Emote@0.0 → Wait@0.0(d=0.05) → Text@0.05(d=D) → Wait@0.05+D(d=1.00) → **Clear@1.05+D**。
    //    D は areka-sakura の `duration::text_playback_duration`（文字数×`CHAR_NOMINAL_MS`=50ms）で、
    //    `compile.rs` は Text cue に対しても `offset += D` を進める（後続 cue はテキスト再生完了後へ整列）。
    //    「アヒルやアヒル」＝7 文字ゆえ **D=0.35s・Clear@1.40s**。Clear は**配送即時にバッファ全消去**
    //    （時刻ゲートではない・state.rs apply_cue）ゆえ、観測したい Text 到達より先に Clear を解放させて
    //    はならない。よって注入模擬時刻は Clear 境界の手前で**頭打ち**にし、頭打ち後は時刻を据え置いた
    //    まま実 async をポンプし続ける（ループ上限は「何回ポンプするか」の意味しか持たない）。壁時計
    //    deadline の延長や反復上限の拡大では直らない——時刻が進む限り Clear が先に解放されて観測条件
    //    そのものが壊れるため（R7.8）。 ──
    // base 起点の固定: base は「active talk がある状態で処理した最初の Tick の now」なので、talk 起動を
    // 観測する（＝cue 由来の指令が rx に現れる）まで now を据え置く。これで base は TICK_BASE_MS 近傍に
    // 焼かれ、頭打ち後の elapsed が Text@0.05 に十分届くことも同時に保証される。
    const TICK_BASE_MS: u64 = 5;
    // 頭打ち値: base≥0 ゆえ elapsed≤(TICK_MAX_MS−0)/1000＝1.040s < Clear@1.40s（余裕 0.36s・無条件に
    // 観測窓の手前）。**警告**: 台本のテキスト長を変えると Clear 時刻が 0.05+文字数×0.05+1.00 で動くので、
    // 頭打ち値を再計算すること。
    const TICK_MAX_MS: u64 = 1_040;
    // ポンプ回数の上限は待ちの芯の打ち切り（下）で与える。観測を壊す Clear の解放は上記 TICK_MAX_MS の頭打ちが防いでおり
    // （areka-P0-kero-balloon 要件 7.8 の是正は適用済み）、待機時間の延長は観測に有利にしか働かない。
    let mut show_cmds: Vec<PresentCommand> = Vec::new();
    let mut now = TICK_BASE_MS;
    // ── 是正は 2 段（PR #96 の `talk_started` ゲート ＋ areka-P0-kero-balloon 要件 7.8 の頭打ち） ─────────────
    //
    //  (1) **talk 起動を観測するまで `now` を進めない**。dispatcher の `base_now` は「talk が active に
    //      なった後の初回 Tick」で確定する（areka-ghost `dispatcher.rs::on_tick`）ため、観測前に進めた
    //      分は base に呑まれて無意味な一方、観測が遅れたときだけ elapsed を余計に進めてしまう。
    //      `\s[2100]` は Emote@0.0＝elapsed 0.0 で due ゆえ、`now` 据え置きのまま必ず到達する。
    //  (2) 観測後は 5ms/反復で進めるが **[`TICK_MAX_MS`] で頭打ち**にする。Text（at=0.05）は確実に
    //      解放され、**Clear（at=1.40・上のタイムライン導出を参照）**へは構造的に到達しないので、
    //      リビール観測がどれだけ遅れても条件が不成立へ倒れない＝レースが消える。
    //
    // **時刻の訂正（2026-08-01・マージ時）**: PR #96 は Clear を `at=1.05` と記していたが、これは
    // `compile.rs` が Text cue に対しても `offset += D` を進める分（D＝文字数×50ms＝0.35s）を落として
    // いる。実測は**掃引で確定**（頭打ち 1.395s＝緑／1.405s＝赤）＝**Clear@1.40**。#96 の頭打ち 500ms も
    // 本 spec の 1_040ms も、いずれも真の境界 1.40s の手前ゆえ**両者とも是正として正しく機能する**
    // （#96 の判定が変わるわけではない）。ここでは根拠が実測で裏取りされている本 spec の定数へ統一する。
    //
    // 打ち切りは待ちの芯（[`wait_until_paced`]）が決める（頭打ち後は反復が仕事量を表さないため回数では
    // 区切らない）。進みの目印は SHIORI の呼び出しの数（受け口は死活の問い合わせを記録しない）と
    // 受けた表示指令の数の和で、相手が 30 秒進まなければ `［止まった］`（areka-P0-ghost-session-test-load-flake 要件 2.1・2.2・
    // `load-repro.md` の 4.3 で自前の締切が赤だった）。Tick を入れる輪なので休みは [`wait_until_paced`] が
    // 持つ（`areka-runtime` の `spine_wait.rs`）。
    let mut talk_started = false;
    let shiori = harness.shiori_handle.clone();
    let shown = Cell::new(0u64);
    let probe = || shiori.call_count() + shown.get();
    let reached = wait_until_paced(
        "S2: シェル面指令＋テキスト cue の到達",
        Progress::Count(&probe),
        || {
            if talk_started {
                // 小刻みに進め、テキスト到達（at=0.05）を解放する（頭打ちゆえ Clear@1.40 へ届かない）。
                now = (now + 5).min(TICK_MAX_MS);
            }
            harness.inject_dispatcher_tick(now);
            show_cmds.extend(harness.wiring.drain_received());
            shown.set(show_cmds.len() as u64);
            if !talk_started {
                // この talk 由来の Emote@0.0 が present まで抜けた＝base_now は据え置き値で確定済み。
                talk_started = show_cmds.iter().any(|c| {
                    matches!(c, PresentCommand::ShowSurface { target, surface_id, .. }
                        if *target == shell_target(0) && *surface_id == 2100)
                });
            }
            harness.pump_text();
            // テキスト cue 到達確認: 完全リビール域 t=0.30 で非透明になれば runtime へ流入済み。
            run_text_phase(&mut harness.wiring, &mut harness.world, Some(0.30));
            // 文字の面は最初に描く文字が届いた present で作られるので、作られる前は「まだ」と読む。
            let attached = harness.runtime.borrow().is_attached(&actor);
            talk_started && attached && text_surface_opaque(&harness, &actor) > 0
        },
    );
    if let Err(failure) = reached {
        panic!(
            "{failure}／S2: シェル面指令＋テキスト cue が有界内に runtime へ到達しない（boot→talk→sink 経路不通）"
        );
    }

    // ── (1) シェル面切替（R2.4/R3.1）: 受信列に ShowSurface{shell_target(0),2100} が現れる ──
    let idx = match show_cmds.iter().position(|c| {
        matches!(c, PresentCommand::ShowSurface { target, surface_id, .. }
            if *target == shell_target(0) && *surface_id == 2100)
    }) {
        Some(i) => i,
        None => panic!(
            "S2: \\s[2100] のシェル面切替 ShowSurface{{shell_target(0),2100}} が受信列に無い: variants={:?}",
            show_cmds.iter().map(variant_name).collect::<Vec<_>>()
        ),
    };
    let show = show_cmds.remove(idx);
    match &show {
        PresentCommand::ShowSurface {
            target,
            surface_id,
            reply,
            ..
        } => {
            assert_eq!(
                *target,
                shell_target(0),
                "シェル表示対象（偶数 TargetId・DD-3）"
            );
            assert_eq!(
                *surface_id, 2100,
                "surface_id は 2100（\\s[2100]・seriko 数値解決の透過）"
            );
            assert!(reply.is_none(), "reply は None（撃ちっぱなし）");
        }
        _ => unreachable!("position で ShowSurface を選別済み"),
    }

    // apply 後、shell が非表示から surface2100 の実描画へ遷移する（初回面表示の実描画まで観測・R8.2）。
    // defect #5 ゆえ attach 時の初期 surface0 baseline は存在せず、hidden（read_back Err）→shown
    // （opaque_count>0）が `\s[2100]` の実描画の証跡になる。
    harness.wiring.apply_present(&mut harness.world, show);
    let after_switch = harness
        .wiring
        .read_back_target(shell_target(0))
        .expect("\\s[2100] 適用後は shell scope0 の原寸面が確立され read_back 可能");
    assert!(
        opaque_count(&after_switch) > 0,
        "S2: \\s[2100] 適用で shell scope0 が surface2100 の実描画へ遷移（非表示→非全透明・R3.1/R8.2）"
    );

    // ── (2) typewriter（R2.2/2.3/R8.5）: Clear 未配送（テキストのみの単一 talk バッファ）に対し注入
    //    talk_time 階段で present_frame を駆動し、text 供給面の opaque_count が単調非減少であること・
    //    pre-reveal（t=0.0＜先頭グリフ r_0=0.05）が全透明であることを固定する。 ──
    let staircase: Vec<f64> = (0..=18).map(|i| i as f64 * 0.05).collect(); // 0.00,0.05,...,0.90
    let mut counts: Vec<usize> = Vec::with_capacity(staircase.len());
    for &t in &staircase {
        run_text_phase(&mut harness.wiring, &mut harness.world, Some(t));
        counts.push(text_surface_opaque(&harness, &actor));
    }
    assert_eq!(
        counts[0], 0,
        "S2: pre-reveal（t=0.0＜先頭グリフ r_0=0.05）は text 供給面が全透明（opaque_count==0）: {counts:?}"
    );
    for i in 1..counts.len() {
        assert!(
            counts[i] >= counts[i - 1],
            "S2: typewriter の opaque_count は単一 talk 内で単調非減少（注入 talk_time 階段）: {counts:?}"
        );
    }
    assert!(
        *counts.last().expect("staircase は非空") > 0,
        "S2: リビールが実際に進行し text が描画される（末尾 t=0.90 で非全透明・非空虚な檻）: {counts:?}"
    );

    // ── Phase 2＋Clear 後全域透明（R8.5）: dispatcher elapsed を Clear（at=1.40）超へ進めて Clear を
    //    配送する。Clear は配送即時にバッファを全消去する（state.rs apply_cue）ため、Clear 配送後は
    //    どの注入 talk_time でも text 供給面が全域透明（premultiplied 全 0）へ戻る。配送前は
    //    present_frame(2.0)＝全リビール（非透明）ゆえ、0 への遷移が Clear 到達の観測点になる。 ──
    //    打ち切りは [`wait_until_paced`] の進みの目印（SHIORI の呼び出しの数。Clear は文字の層へ届き、表示の
    //    指令として汲む物が無いので足さない）。Clear 後に全域透明を覆す cue は台本に無い（`\c` が最終 cue）＝観測はラッチ。 ──
    let what = "S2: Clear cue の runtime への到達";
    let calls = || shiori.call_count();
    wait_until_paced(what, Progress::Count(&calls), || {
        now += 50; // 大きめに進め Clear（at=1.40s・上のタイムライン導出）を配送させる
        harness.inject_dispatcher_tick(now);
        harness.pump_text();
        run_text_phase(&mut harness.wiring, &mut harness.world, Some(2.0));
        text_surface_opaque(&harness, &actor) == 0
    })
    .unwrap_or_else(|failure| panic!("{failure}／{what}: \\_w[1000]\\c の配送が完了しない"));
    // Clear 配送後は「リビール済みだった」区間の talk_time（t=0.30）でも全域透明（Clear の全消去・R8.5）。
    run_text_phase(&mut harness.wiring, &mut harness.world, Some(0.30));
    assert_eq!(
        text_surface_opaque(&harness, &actor),
        0,
        "S2: Clear 配送後は t=0.30（リビール域）でも text 供給面が全域透明（Clear の全消去・R8.5）"
    );

    harness.shutdown_bounded();
}

/// spine S5（close 握手・R6.1/6.2/6.3・R8.3）: `shutdown(CloseReason::User { scope: 0 })`（ForceQuit 経路＝
/// OnClose NOTIFY→Unload）で ghost 一式を畳み、(a) OnClose 台本が消化され（`ScriptedShioriHandle` に
/// `Notify{OnClose}`→`Unload` が順に記録される）、(b) `shutdown` が有界時間で `Ok` を返し、(c) seriko
/// worker（＋ghost 内部の全ハンドルは `shutdown` が内部 join）が有界 join で完了する（timeout=panic
/// ゆえ hang すれば test FAIL）ことを固定する（設計 Testing Strategy S5・ghost spine S 系の手法）。
///
/// # ForceQuit 経路の OnClose は NOTIFY（GET でない）
///
/// `GhostRuntime::shutdown` は常に ForceQuit 横断遷移で終了する（close talk を発行しない）。ゆえに
/// OnClose は片道 NOTIFY として消化され（標準台本 `SpineHarness::boot` が `notify("OnClose")` を
/// 台本化済み）、続けて正規 clean shutdown の `Unload`（`Ok(ExitKind::Clean)`）が呼ばれる。close talk
/// 駆動の Quit 経路（OnClose GET→`\-`）は ghost spine S4 の担当領域であり、本 spine の主眼は
/// 「areka 側の実 sink 結線（seriko 含む）が shutdown で hang せず有界 join する」ことの檻に置く。
#[test]
fn spine_s5_close_handshake_consumes_onclose_and_joins_all_handles_bounded() {
    // 標準台本（OnClose NOTIFY＋Unload(Clean)）で boot。最小 OnBoot talk（\s[0]\e）。
    let harness = SpineHarness::boot(r"\s[0]\e");

    // 起動の系列（OnInitialize/username/OnFirstBoot/OnBoot/OnTranslate/basewareversion・最後は
    // basewareversion）が終わるまで待つ（OnClose を起動の呼び出しと分ける）。
    // 打ち切りは待ちの芯（[`wait_until`]）が決める。進みの目印は SHIORI の呼び出しの数（受け口は
    // 死活の問い合わせを記録しない）で、相手が 30 秒進まなければ `［止まった］`（`load-repro.md` の 4.3 で目印なしの待ちが赤だった）。
    let calls = || harness.shiori_handle.call_count();
    let mut boot_calls = Vec::new();
    let waited = wait_until(
        "S5 前提: 起動の系列（最後は basewareversion）",
        Progress::Count(&calls),
        || {
            boot_calls = harness.shiori_handle.non_status_calls();
            boot_finished(&boot_calls)
        },
    );
    let failure = waited.err().map(|f| format!("{f}／")).unwrap_or_default();
    assert!(
        boot_finished(&boot_calls),
        "{failure}S5 前提: 起動の系列（最後は basewareversion）が有界内に終わる: {boot_calls:?}"
    );

    // (b)(c) 降ろしが有界時間で Ok を返し、seriko worker が有界 join で終わることは
    // `shutdown_bounded` が確かめる。呼び出しの記録は降ろしの後に照合するので handle を残す。
    let shiori_handle = harness.shiori_handle.clone();
    harness.shutdown_bounded();

    // (a) OnClose 台本消化: Notify{OnClose}→Unload が順に記録される（ForceQuit close 握手→clean unload）。
    let calls = shiori_handle.non_status_calls();
    let onclose_idx = calls
        .iter()
        .position(|c| matches!(c, RecordedCall::Notify { id, .. } if id == "OnClose"));
    let unload_idx = calls.iter().position(|c| matches!(c, RecordedCall::Unload));
    assert!(
        onclose_idx.is_some(),
        "S5: OnClose NOTIFY が消化される（ForceQuit close 握手）: {calls:?}"
    );
    assert!(
        unload_idx.is_some(),
        "S5: Unload が呼ばれる（正規 clean shutdown）: {calls:?}"
    );
    assert!(
        onclose_idx < unload_idx,
        "S5: OnClose→Unload の順（close 握手→unload）: {calls:?}"
    );
}
