use super::{
    RecordedCall, SpineHarness, capture_logs, count_level, run_attach_phase, run_text_phase,
    spin_wait_until,
};

// ===========================================================================
// task 6.1 スモークテスト
// ===========================================================================

/// 観測可能な完了条件（tasks.md task 6.1）: ハーネスが scripted ghost を boot させ、Tick 注入に
/// より attach 準備状態まで **panic なく** 到達することをスモークレベルで固定する（R8.1/8.3/8.4/8.6）。
///
/// 檻に入れる判断分岐:
/// - **scripted boot 発火**: boot 系列（OnInitialize→OnFirstBoot→OnBoot→basewareversion）が
///   scripted backend へ (method,id) 順で届く（＝「scripted ghost を boot させた」直接証跡）。
/// - **Tick 注入の疎通**: `dispatcher()` への Tick 送出が Ok（ghost スタック生存・sleep 不使用）。
/// - **attach 到達**: headless GPU World（WARP・MTA）＋合成 `GhostWindows` 上で `run_attach_phase`
///   が **panic せず** 完走し、DD-12 の縮退がバグを隠さない檻＝`planned==attached==2`（全 scope の
///   シェル装着成功）を装着サマリ `info!` で観測でき、ERROR は 0 件。
/// - **ハンドル生存**: attach 後も seriko worker は稼働中・dispatcher は再度 Tick を受理する。
///
/// 豊富な観測（S1 ピクセル readback・S2 typewriter・S3 `\b`・S5 close 握手）は 6.2/6.3 の担当。
#[test]
fn spine_harness_boots_scripted_ghost_and_reaches_attach_ready() {
    // 最小 talk（1 サーフェス cue・テキストなし）で決定論を単純に保つ。
    let mut harness = SpineHarness::boot(r"\s[0]\e");

    // ── (1) scripted boot 発火: boot 系列が backend へ (method,id) 順で届く ──
    // boot 系列は kanade スレッド上の同期往復のみで完走する（Tick 不要）。実スレッド境界を跨ぐため
    // 有界スピン待機（sleep なし・yield_now のみ）で 6 呼出の到達を待ってから照合する。task 8.2 の
    // username prefetch GET（OnInitialize 後・OnFirstBoot 前・R9.1/9.2）と、挨拶の台詞を再生の前に
    // かける OnTranslate（OnBoot の後・偽の SHIORI の既定は 204）が加わり boot 系列は 6 呼出。
    // 打ち切りは反復回数でなく [`spin_wait_until`] の時刻期限（反復は経過時間の代理にならない）。
    let mut boot_calls = Vec::new();
    spin_wait_until(|| {
        boot_calls = harness.shiori_handle.non_status_calls();
        boot_calls.len() >= 6
    });
    let projected: Vec<(&str, &str)> = boot_calls
        .iter()
        .map(|c| match c {
            RecordedCall::Notify { id, .. } => ("notify", id.as_str()),
            RecordedCall::Get { id, .. } => ("get", id.as_str()),
            RecordedCall::Unload => ("unload", ""),
            RecordedCall::Status => ("status", ""),
        })
        .collect();
    assert!(
        projected.len() >= 6,
        "scripted boot 系列が有界内に発火しない（scripted ghost を boot できていない）: {boot_calls:?}"
    );
    assert_eq!(
        &projected[..6],
        &[
            ("notify", "OnInitialize"),
            ("get", "username"),
            ("get", "OnFirstBoot"),
            ("get", "OnBoot"),
            ("get", "OnTranslate"),
            ("notify", "basewareversion"),
        ],
        "boot 系列が正典順序（OnInitialize→username prefetch→OnFirstBoot→OnBoot→OnTranslate→basewareversion）で発火していない"
    );

    // ── (2) Tick 注入の疎通（ghost スタック生存・sleep 不使用・R8.3） ──
    harness.inject_dispatcher_tick(1);

    // ── 実 ClockedTextSink<EmoTextSink> の UI ドレインが headless に pump できることを裏付ける
    //    （pending なし／ありに関わらず panic しない・R8 の実 sink 経路の疎通確認）。 ──
    harness.pump_text();

    // ── (3) attach 到達: run_attach_phase を GPU World＋合成 GhostWindows 上で駆動し、DD-12 の
    //    「計画件数＝実装着件数」を装着サマリで観測する（縮退がバグを隠さない檻・R8.1）。 ──
    let logs = capture_logs(|| run_attach_phase(&mut harness.wiring, &mut harness.world));
    assert!(
        logs.iter()
            .any(|l| l.contains("planned=2") && l.contains("attached=2")),
        "attach 到達（planned=2 attached=2＝全 scope のシェル装着成功）が観測できない: {logs:?}"
    );
    assert_eq!(
        count_level(&logs, "ERROR"),
        0,
        "attach フェーズで ERROR が発火した（装着失敗・log-first）: {logs:?}"
    );

    // ── (4) ハンドル生存: seriko worker 稼働中・dispatcher は再度 Tick を受理する ──
    assert!(
        !harness.seriko.is_finished(),
        "attach 到達時点で seriko worker は稼働中であるべき（実 sink 経路が生きている）"
    );
    harness.inject_dispatcher_tick(2);

    // ── 後片付け: 正規終了＋全ハンドル有界 join（hang させない・R8.3 の観測点＝有界 join のみ） ──
    harness.shutdown_bounded();
}

// ===========================================================================
// budoux-reveal-reflow task 6.2: 先渡しが本番の鎖を通って文字の層へ届く
// ===========================================================================

/// 観測可能な完了条件（budoux-reveal-reflow tasks.md 6.2・要件 2.1）: 本番の鎖（トークごとの写し →
/// 時刻の受け手の飾り `ClockedTextSink` → 文字の層の受け手 `EmoTextSink`）を通ったトークで、
/// 文字の層が先渡しを受け取った `debug!` 行が 1 件以上あり、「先渡しが無い」の warn が 0 件である。
///
/// 「先渡しが無い」の warn は文節の折り返しのバルーンで字を出すときだけ出る。この検査の土台の
/// バルーン（emo2-kakukaku）は `budoux_newline,1` を持つので、装着してから字を提示すれば warn の
/// 分岐を踏む。それでも warn 0 件は「出なかった」ことしか言えないので、判定の主は受け取りの行が
/// 1 件以上あること（飾りが先渡しを内側へ通さないと受け取りが 0 件になり、warn も 1 件出て赤になる）。
///
/// 文字の層の受け手の吸い出しはテストスレッド（＝UI pump スレッド）で回るので、捕捉は
/// このスレッドの `capture_logs` で足りる。先渡しはトークの初回 Tick で登録されるとき別スレッドから
/// 積まれるので、Tick・吸い出し・提示を 1 回ずつ捕捉しながら、受け取りの行が現れるまで待つ。
#[test]
fn spine_preview_reaches_text_layer_through_production_chain() {
    let mut harness = SpineHarness::boot(r"\s[0]こんにちは\e");

    // 字の提示（文節の折り返しなら warn の分岐）を踏めるよう、先に装着しておく。
    let logs = capture_logs(|| run_attach_phase(&mut harness.wiring, &mut harness.world));
    assert_eq!(
        count_level(&logs, "ERROR"),
        0,
        "attach で ERROR なし: {logs:?}"
    );

    let mut logs: Vec<String> = Vec::new();
    let mut now = 1;
    let reached = spin_wait_until(|| {
        logs.extend(capture_logs(|| {
            harness.inject_dispatcher_tick(now);
            harness.pump_text();
            run_text_phase(&mut harness.wiring, &mut harness.world, Some(10.0));
        }));
        now += 10;
        logs.iter()
            .any(|l| l.contains("target=areka_emo_text") && l.contains("先渡しを受け取った"))
    });
    assert!(
        reached,
        "先渡しの受け取り（「先渡しを受け取った」の debug! 行）が文字の層に現れない——飾り（ClockedTextSink）か受け手（EmoTextSink）が先渡しを通していない: {logs:?}"
    );
    let missing = logs
        .iter()
        .filter(|l| l.contains("level=WARN") && l.contains("先渡しが無い"))
        .count();
    assert_eq!(missing, 0, "「先渡しが無い」の warn が出た: {logs:?}");

    harness.shutdown_bounded();
}
