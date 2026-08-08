use super::*;
use super::test_support::row;
use std::sync::mpsc;

// -------------------------------------------------------------------------
// 貫通ポインタ列の決定論檻（task 7.1・design Testing Strategy Integration Test 6
// 「貫通檻（R6.2 字義・条件付き）」／R6.1/6.2/6.3/6.4/6.5）
//
// STEP 0 裁定（本 task 冒頭で確認・GPU-gated）: `TextLayerRuntime::choice_hit_rows` は
// `choice_snapshot`（HashMap）のみを読み（actor.rs:389-393）、その population は present_actor が
// GPU present 成功時にのみ行う（actor.rs:204-205・688-709）。`apply_cue(Choice)` は逆に
// `choice_snapshot` を remove する（actor.rs:297）ため、headless では `choice_active==true` でも
// `choice_hit_rows` は常に空＝hit=None（既存檻 pressed_active_non_hit_rejected_with_reason_no_hit が
// 実証済み）。population 経路 present_frame は WucGraphicsResource/Compositor・GraphicsCore・
// dwrite_factory を要求し（actor.rs:532-576）headless では Err(Device) を返す——GPU 無しに snapshot を
// populate する手段は無い。ゆえに実ハンドラ経由で「Some(ordinal) 追従」「hit→send→choice_selected」の
// full pass-through を実演することは構造的に不可能。
//
// 設計裁定（Integration Test 6・設計バリデーション Issue 1 の非決定例外）どおり本檻は**分解形**で R6 を
// 満たす: 合成 `ChoiceHitRow`（上流契約型・pub フィールドで直接構築可＝task 3.1 と同じ正当性。fake
// runtime で hit を捏造するのではなく、上流照会契約の型そのものを純関数へ供給する）を純関数パイプライン
// （hit_choice_row→hover_action／click_selection）へ**注入座標列**（move 群＋click）で通し、発行は
// **実 mpsc seam**（`BalloonWiring::send_selection`→`ChoiceSelectionInbox::try_recv`）で観測する。
// GPU・実窓・sleep を一切用いない（R6.4）。判断分岐の実行網羅は 3.1/3.2/3.3、発行 seam は 2.2、配線存在／
// スケジュール登録は 6.1 の各檻が閉じており、本檻はそれらの上に「注入座標列→hover 追従→一度きり発行／
// 非発行」を貫通で固定する（R6.1/6.2/6.3/6.5）。
// -------------------------------------------------------------------------

/// 2 行の選択肢ヒット行（ordinal 0/1・非重複帯・窓物理 px）。上流契約型を直接構築する（task 3.1 同型）。
fn two_choice_rows() -> Vec<ChoiceHitRow> {
    vec![
        row(0, 0.0, 0.0, 100.0, 20.0),  // ordinal 0（上帯 y∈[0,20)）
        row(1, 0.0, 20.0, 100.0, 40.0), // ordinal 1（下帯 y∈[20,40)）
    ]
}

/// 移動ハンドラ（`on_balloon_pointer_moved`）の自前 last-injected 更新（⑤・design §204）を鏡写す:
/// `Inject` は注入値、`ResetOwnState` は None、`Keep`/`NoopInactive` は据え置き。
fn apply_last(action: HoverAction, last: Option<usize>) -> Option<usize> {
    match action {
        HoverAction::Inject(v) => v,
        HoverAction::ResetOwnState => None,
        HoverAction::Keep | HoverAction::NoopInactive => last,
    }
}

/// 注入座標列→hover 追従→hit クリック一度きり発行（貫通・R6.1/6.2/6.4/6.5・GPU/実窓/sleep 不要）。
///
/// 合成 2 行へ move 点列 [row0 内, row1 内, row1 内（同値再入）, 全行外] を注入し、各 move で
/// `hit_choice_row→hover_action` を評価しつつ移動ハンドラ同型に last-injected を更新して hover 軌跡を
/// 採取する。軌跡は注入列に追従する（Some(0)→Some(1)→Some(1)→None）。中間で「同値再入は Keep（再 Inject
/// しない）」「行外遷移は Inject(None)＝ハイライト解除」を明示 assert し、hover 追跡が誤って last を
/// 更新しなければ落ちる非自明性を担保する。続いて row1 上の click 点で `click_selection`→**実**
/// `BalloonWiring::send_selection`→`ChoiceSelectionInbox::try_recv` が発行を一度だけ観測する
/// （2 度目は Empty・R6.2）。
#[test]
fn injected_pointer_sequence_hover_follows_then_click_issues_once_via_inbox() {
    let rows = two_choice_rows();
    let scope = 7usize;

    // ── 注入 move 点列（バルーン窓 client 物理 px・**無変換**で照合される。行矩形が既に実適用 k
    // ×済みの窓物理 px ゆえ同一空間で一致する——k=1.0 だからではない。÷k 追加は二重縮約・R6.4）──
    let moves = [
        (50.0_f32, 10.0_f32), // row0 内 → Some(0)（初回 Inject）
        (50.0, 30.0),         // row1 内 → Some(1)（行遷移 Inject）
        (50.0, 30.0),         // row1 内（同値再入）→ Some(1)（Keep・再注入しない）
        (200.0, 200.0),       // 全行外 → None（Inject(None) 解除）
    ];

    let mut last: Option<usize> = None; // 移動ハンドラ ⑤ の自前 last-injected（初期未注入）。
    let mut trajectory: Vec<Option<usize>> = Vec::new();
    let mut actions: Vec<HoverAction> = Vec::new();
    for (x, y) in moves {
        // choice 表示中（active=true）——注入座標に対する現行 rows のヒットを ordinal へ展開し
        // 純関数核で遷移を決め、ハンドラ同型に last を更新する（毎 move 現行 rows を読む）。
        let hit = hit_choice_row(&rows, x, y).map(|i| rows[i].ordinal);
        let action = hover_action(true, hit, last);
        last = apply_last(action, last);
        actions.push(action);
        trajectory.push(last);
    }

    // hover 軌跡が注入座標列に追従する（R6.1）。
    assert_eq!(
        trajectory,
        vec![Some(0), Some(1), Some(1), None],
        "hover は注入座標列に追従する（row0→row1→row1 同値→行外解除）"
    );
    // 非自明性①: 同値再入（3 手目）は再注入せず Keep。last 更新が誤って Some(1) を保持しなければ
    // ここが Inject になり落ちる。
    assert_eq!(
        actions[2],
        HoverAction::Keep,
        "同値再入は Keep（hover 追跡が誤れば Inject になり落ちる非自明 assertion）"
    );
    // 非自明性②: 行外遷移（4 手目）は last=Some(1)→None のハイライト解除 Inject(None)（R1.3）。
    // 追跡が誤って last を None のままにしていれば Keep になり落ちる。
    assert_eq!(
        actions[3],
        HoverAction::Inject(None),
        "行外遷移はハイライト解除 Inject(None)（追跡が誤れば Keep になり落ちる非自明 assertion）"
    );

    // ── row1 上の click を実 mpsc seam へ通す（一度きり発行の貫通観測・R6.2）─────────────────────
    let (tx, rx) = mpsc::channel::<ChoiceSelection>();
    let wiring = BalloonWiring::new(tx);
    let inbox = ChoiceSelectionInbox(rx);

    // click 点 (50, 30) は row1（ordinal 1）内。click_selection が**現行**行から Some を構成する。
    let sel = click_selection(true, &rows, 50.0, 30.0, scope)
        .expect("row1 上の click は ChoiceSelection を構成する");
    // 押下ハンドラ同型に「Some のときだけ 1 回 send」する（発行の一度きり制御はエッジ検出＝1 send）。
    assert!(
        wiring.send_selection(sel),
        "Receiver 生存中の発行は成功（send_selection→mpsc）"
    );

    // seam の Receiver 経由で発行を一度だけ観測する（送信値は click 行から構成される・R6.2）。
    let received = inbox.0.try_recv().expect("発行した ChoiceSelection が届く");
    assert_eq!(received.id, "q1", "発行値は click 行（ordinal 1）から構成される");
    assert_eq!(received.label, "label1", "label も click 行から転写される");
    assert_eq!(
        received.scope, scope,
        "scope は引数（BalloonWindowMarker.scope）由来"
    );
    assert!(
        inbox.0.try_recv().is_err(),
        "発行は一度きり（2 度目の try_recv は Empty・R6.2）"
    );
}

/// 非 hit・stale・非表示（消滅）click は実 seam へ何も流さない（貫通・R6.2/6.3/6.4・GPU/実窓/sleep 不要）。
///
/// 単一の実 mpsc seam を張り、注入 click を `click_selection`→（`None` のとき非 send）で通す:
/// (a) 表示中・全行外座標＝非 hit → None → 非 send（R6.3）、(b) 現行 rows がクリック座標を覆わない
/// stale（レイアウト差替後）→ None → 非 send（現行ジオメトリのみ読む・R6.3）、(c) 非表示（消滅・
/// active=false）→ 矩形内座標でも短絡 None → 非 send（R6.3）。いずれの後も `try_recv` は Empty。
#[test]
fn injected_non_hit_stale_and_inactive_clicks_issue_nothing_via_inbox() {
    let rows = two_choice_rows();
    let scope = 7usize;

    let (tx, rx) = mpsc::channel::<ChoiceSelection>();
    let wiring = BalloonWiring::new(tx);
    let inbox = ChoiceSelectionInbox(rx);

    // 押下ハンドラ同型: Some のときだけ send する薄い注入経路（None は非 send＝棄却）。
    let try_click = |active: bool, current: &[ChoiceHitRow], x: f32, y: f32| {
        if let Some(sel) = click_selection(active, current, x, y, scope) {
            wiring.send_selection(sel);
        }
    };

    // (a) 表示中・全行外＝非 hit（R6.3）。
    try_click(true, &rows, 200.0, 200.0);
    // (b) stale: 現行 rows は (50,30) を覆わない別位置の行のみ（キャッシュではなく現行のみ読む・R6.3）。
    let stale_rows = [row(0, 500.0, 500.0, 540.0, 520.0)];
    try_click(true, &stale_rows, 50.0, 30.0);
    // (c) 非表示（消滅・active=false）——row1 内座標でも hit 判定より前に短絡 None（R6.3）。
    try_click(false, &rows, 50.0, 30.0);

    assert!(
        inbox.0.try_recv().is_err(),
        "非 hit／stale／非表示の click は一切 send されない（Inbox は Empty・R6.2/6.3）"
    );
}
