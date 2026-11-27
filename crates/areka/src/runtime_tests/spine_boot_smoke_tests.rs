use shiori_host32_host::ExitKind;

use super::{
    LoopDriver, Progress, RecordedCall, ScriptedShioriBackend, SpineHarness, capture_logs,
    wait_until,
};

// ===========================================================================
// 足場のスモークテスト
// ===========================================================================

/// 観測可能な完了条件: ハーネスが scripted ghost を boot させ、Tick 注入に
/// より attach 準備状態まで **panic なく** 到達することをスモークレベルで固定する。
///
/// 檻に入れる判断分岐:
/// - **scripted boot 発火**: boot 系列（OnInitialize→username prefetch→OnFirstBoot→OnBoot→OnTranslate→basewareversion）が
///   scripted backend へ (method,id) 順で届く（＝「scripted ghost を boot させた」直接証跡）。
/// - **Tick 注入の疎通**: `dispatcher()` への Tick 送出が Ok（ghost スタック生存・時刻は注入だけで進める）。
/// - **attach 到達**: 実 GPU の World（MTA）＋合成 `GhostWindows` 上で `run_attach_phase`
///   が **panic せず** 完走し、縮退がバグを隠さない檻＝`planned==attached==2`（全 scope の
///   シェル装着成功）を装着サマリ `info!` で観測でき、ERROR は 0 件。
/// - **ハンドル生存**: attach 後も seriko worker は稼働中・dispatcher は再度 Tick を受理する。
///
/// 豊富な観測（S1 ピクセル readback・S2 typewriter・S3 `\b`・S5 close 握手）は兄弟のテストファイルが持つ。
#[test]
fn spine_harness_boots_scripted_ghost_and_reaches_attach_ready() {
    // 最小 talk（1 サーフェス cue・テキストなし）で決定論を単純に保つ。
    let mut harness = SpineHarness::boot(r"\s[0]\e");

    // ── (1) scripted boot 発火: boot 系列が backend へ (method,id) 順で届く ──
    // boot 系列は kanade スレッド上の同期往復のみで完走する（Tick 不要）。実スレッド境界を跨ぐため
    // [`wait_until`]（休みは芯が持つ）で下の照合の列の到達を待ってから照合する。boot 系列には
    // username prefetch GET（OnInitialize 後・OnFirstBoot 前）と、挨拶の台詞を再生の前に
    // かける OnTranslate（OnBoot の後・偽の SHIORI の既定は 204）が入る。
    // 打ち切りは反復回数でなく [`wait_until`] の進みの目印（SHIORI の呼び出しの数）で判じる。
    let mut boot_calls = Vec::new();
    let handle = harness.shiori_handle.clone();
    let calls = || handle.call_count();
    let _ = wait_until("起動系列 6 呼出", Progress::Count(&calls), || {
        boot_calls = harness.shiori_handle.non_status_calls();
        boot_calls.len() >= 6
    })
    .inspect_err(|failure| eprintln!("{failure}"));
    let projected: Vec<(&str, &str)> = boot_calls
        .iter()
        .map(|c| match c {
            RecordedCall::Notify { id, .. } => ("notify", id.as_str()),
            RecordedCall::Get { id, .. } => ("get", id.as_str()),
            RecordedCall::Unload => ("unload", ""),
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

    // ── (2) Tick 注入の疎通（ghost スタック生存・時刻は注入だけで進める） ──
    harness.inject_dispatcher_tick(1);

    // ── 実 EmoTextSink の UI ドレインが headless に pump できることを裏付ける
    //    （pending なし／ありに関わらず panic しない）。 ──
    harness.pump_text();

    // ── (3) attach 到達: run_attach_phase を GPU World＋合成 GhostWindows 上で駆動し、
    //    「計画件数＝実装着件数」を装着サマリで観測する（縮退がバグを隠さない檻）。 ──
    harness.attach_all();

    // ── (4) ハンドル生存: seriko worker 稼働中・dispatcher は再度 Tick を受理する ──
    assert!(
        !harness.seriko.is_finished(),
        "attach 到達時点で seriko worker は稼働中であるべき（実 sink 経路が生きている）"
    );
    harness.inject_dispatcher_tick(2);

    // ── 後片付け: 正規終了＋全ハンドル有界 join（hang させない・観測点＝有界 join のみ） ──
    harness.shutdown_bounded();
}

// ===========================================================================
// 先渡しが本番の鎖を通って文字の層へ届く
// ===========================================================================

/// 観測可能な完了条件: 本番の鎖（トークごとの写し → 文字の層の受け手 `EmoTextSink`）を通ったトークで、
/// 文字の層が先渡しを受け取った `debug!` 行が 1 件以上あり、「先渡しが無い」の warn が 0 件である。
///
/// 「先渡しが無い」の warn は文節の折り返しのバルーンで字を出すときだけ出る。この検査の土台の
/// バルーン（emo2-kakukaku）は `budoux_newline,1` を持つので、装着してから字を提示すれば warn の
/// 分岐を踏む。それでも warn 0 件は「出なかった」ことしか言えないので、判定の主は受け取りの行が
/// 1 件以上あること（受け手が先渡しを通さないと受け取りが 0 件になり、warn も 1 件出て赤になる）。
///
/// 文字の層の受け手の吸い出しはテストスレッド（＝UI pump スレッド）で回るので、捕捉は
/// このスレッドの `capture_logs` で足りる。先渡しはトークの初回 Tick で登録されるとき別スレッドから
/// 積まれるので、Tick・吸い出し・提示を 1 回ずつ捕捉しながら、受け取りの行が現れるまで待つ。
#[test]
fn spine_preview_reaches_text_layer_through_production_chain() {
    let mut harness = SpineHarness::boot(r"\s[0]こんにちは\e");

    // 字の提示（文節の折り返しなら warn の分岐）を踏めるよう、先に装着しておく。
    harness.attach_all();

    // 受け取りの行の後に本番の字の適用が届き、その後の回で提示まで回したら止める（空回しの字の適用は
    // 受け取りの行より前に出る）。warn 0 件の判定が、字を提示した後の回まで覆うようにするため。
    let mut logs: Vec<String> = Vec::new();
    let mut now = 1;
    let mut text_seen = false;
    let mut presented_after_text = false;
    // 目印は SHIORI の呼び出しの数＋捕まえたログの行の数（文字の層が受けて動いた分だけ増える）。
    let handle = harness.shiori_handle.clone();
    let seen = std::cell::Cell::new(0u64);
    let mark = || handle.call_count() + seen.get();
    let _ = wait_until(
        "文字の層の先渡しの受け取り",
        Progress::Count(&mark),
        || {
            logs.extend(capture_logs(|| {
                harness.inject_dispatcher_tick(now);
                harness.pump_text();
                harness.present_text(dola::time::testing::at_ms(10_000));
            }));
            now += 10;
            seen.set(logs.len() as u64);
            presented_after_text = text_seen;
            text_seen = logs
                .iter()
                .position(|l| {
                    l.contains("target=areka_emo_text") && l.contains("先渡しを受け取った")
                })
                .is_some_and(|i| logs[i..].iter().any(|l| l.contains("Text cue 適用")));
            presented_after_text
        },
    )
    .inspect_err(|failure| eprintln!("{failure}"));
    assert!(
        logs.iter()
            .any(|l| l.contains("target=areka_emo_text") && l.contains("先渡しを受け取った")),
        "先渡しの受け取り（「先渡しを受け取った」の debug! 行）が文字の層に現れない——受け手（EmoTextSink）が先渡しを通していない: {logs:?}"
    );
    assert!(
        presented_after_text,
        "受け取りの後に本番の字が届いて提示されるところまで進まない: {logs:?}"
    );
    let missing = logs
        .iter()
        .filter(|l| l.contains("level=WARN") && l.contains("先渡しが無い"))
        .count();
    assert_eq!(missing, 0, "「先渡しが無い」の warn が出た: {logs:?}");

    harness.shutdown_bounded();
}

/// 足場の後片付けは、ゴーストの降ろしの失敗を隠さない。
///
/// 標準の台本から `OnClose` だけを抜いて起こす。降ろしは `OnClose` を NOTIFY で送るので、
/// 台本に無い呼び出しで偽の SHIORI が panic し、`shutdown` は `Err` を返す。後片付けがそれを
/// `.expect` で落とすので、待つ側には `［相手が居ない］` の赤として届く。
///
/// # 非空虚性
///
/// - 後片付けが `shutdown` の戻りを捨てていれば、panic せずに通り抜け、`should_panic` が落ちる。
#[test]
#[should_panic(expected = "［相手が居ない］: 「spine ghost shutdown」")]
fn shutdown_bounded_fails_when_the_shiori_thread_panics() {
    let (backend, shiori_handle) = ScriptedShioriBackend::builder()
        .notify("OnInitialize", Ok(()))
        .get("OnFirstBoot", Ok(None))
        .get("OnBoot", Ok(Some(r"\s[0]\e".to_string())))
        .notify("basewareversion", Ok(()))
        .unload(Ok(ExitKind::Clean))
        .build();
    SpineHarness::boot_with(backend, shiori_handle, LoopDriver::Inert).shutdown_bounded();
}
