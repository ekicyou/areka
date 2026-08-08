use super::*;

use std::time::Duration;

use areka_actor::reply_channel;
use areka_emo_compose::BindSet;

use super::test_support::{
    attach_hit_target, build_target_assets, build_two_face_assets, force_current_surface,
    make_world_with_gpu, scaled_golden, set_window_dpi, show_ok, spawn_window_with_dpi,
};

// ── 表示成立点 info ログ（設計 D10・要件 6.1/6.3）の檻 ──────────────────────────────────
// 実機サインオフ（R6.3）は有界 auto-exit で起動して `RUST_LOG` を grep し、**このログのフィールド名と
// 値**から「2 水準が異なる k・異なる物理寸で描かれた」ことを決定論的に判定する。ゆえに level が
// `info` であることと D10 各フィールドが正しい値で在ることは観測状態と同格の契約であり、檻に入れる。
//
// 捕捉は **`tracing` 単体**（本 crate の既存依存）で組む——`tracing-subscriber` は dev-dependency に
// 無く、要件 7.3（新規外部依存の禁止）ゆえ足さない。`with_default` は **スレッドローカル**の既定
// subscriber を差すため、並列実行される他テストのイベントを取り込まない（`set_global_default` は
// プロセス大域＝並列テストで混線するため使わない）。

/// 捕捉した 1 イベント（level ＋ フィールド名 → Debug 表現）。
#[derive(Debug, Clone)]
struct CapturedEvent {
    level: tracing::Level,
    fields: std::collections::HashMap<String, String>,
}

/// 全フィールドを Debug 表現で拾う visitor。
///
/// [`tracing::field::Visit`] の `record_u64`/`record_f64`/`record_bool` 等はすべて既定実装が
/// `record_debug` へ転送するため、`record_debug` 1 本の実装で型を問わず全フィールドを捕捉できる。
struct FieldGrab(std::collections::HashMap<String, String>);

impl tracing::field::Visit for FieldGrab {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.0
            .insert(field.name().to_string(), format!("{value:?}"));
    }
}

/// イベントを溜めるだけの最小 subscriber（span は使わないので new_span は固定 id を返す）。
#[derive(Clone, Default)]
struct CaptureSubscriber(std::sync::Arc<std::sync::Mutex<Vec<CapturedEvent>>>);

impl tracing::Subscriber for CaptureSubscriber {
    fn enabled(&self, _meta: &tracing::Metadata<'_>) -> bool {
        true
    }
    fn new_span(&self, _attrs: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }
    fn record(&self, _span: &tracing::span::Id, _values: &tracing::span::Record<'_>) {}
    fn record_follows_from(&self, _span: &tracing::span::Id, _follows: &tracing::span::Id) {}
    fn event(&self, event: &tracing::Event<'_>) {
        let mut grab = FieldGrab(std::collections::HashMap::new());
        event.record(&mut grab);
        self.0
            .lock()
            .expect("捕捉バッファの毒化なし")
            .push(CapturedEvent {
                level: *event.metadata().level(),
                fields: grab.0,
            });
    }
    fn enter(&self, _span: &tracing::span::Id) {}
    fn exit(&self, _span: &tracing::span::Id) {}
}

/// 要件 6.1/6.3 観測完了（設計 D10 の観測ログ）: 表示成立点で **`info` レベル**のログが出て、
/// k 導出値（`k`・`k_ratio`）・`author_dpi`・`window_dpi`・native 寸・scaled 寸が揃う。
///
/// k=2/1・native 4×3・物理 8×6 という**互いに弁別可能**な値で組むため、native と scaled の取り違え・
/// k の取り違えはすべて RED になる。`info!` を `debug!` へ落とす改変も level assert が捕まえる
/// （R6.3 の `RUST_LOG` grep は既定の観測条件で info を読むため、level 自体が契約である）。
#[test]
fn display_success_emits_d10_observation_log_at_info() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 192);
    let (emo_world, atlas, _golden) = build_target_assets(4, 3, 0x88);

    let mut presenter = EmoPresenter::new();
    presenter
        .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
        .expect("attach_target 失敗");

    let cap = CaptureSubscriber::default();
    tracing::subscriber::with_default(cap.clone(), || {
        show_ok(&mut presenter, &mut world, TargetId(0), 1000);
    });

    let events = cap.0.lock().expect("捕捉バッファ").clone();
    let ev = events
        .iter()
        .find(|e| {
            e.fields
                .get("message")
                .is_some_and(|m| m.contains("表示・マスクを更新"))
        })
        .unwrap_or_else(|| panic!("表示成立点のログが出ていない: {events:?}"));

    assert_eq!(
        ev.level,
        tracing::Level::INFO,
        "表示成立点の観測ログが info レベルでない（R6.3 の RUST_LOG grep が既定条件で読めない）"
    );

    let field = |name: &str| -> String {
        ev.fields
            .get(name)
            .unwrap_or_else(|| panic!("D10 フィールド `{name}` が無い: {:?}", ev.fields))
            .clone()
    };

    // k 導出値: f32 の照会表現と、既約有理表現（num/den）の双方。
    assert_eq!(field("k"), "2.0", "k（f32）が実適用値でない");
    let k_ratio = field("k_ratio");
    assert!(
        k_ratio.contains("num: 2") && k_ratio.contains("den: 1"),
        "k_ratio に既約 num/den が出ていない: {k_ratio}"
    );

    // 導出の両入力（分母＝作者基準 DPI・分子側＝窓 DPI）。
    assert_eq!(field("author_dpi"), "96");
    assert_eq!(
        field("window_dpi"),
        "Some((192, 192))",
        "窓 DPI が出ていない（不在＝要件 1.4 縮退も None として観測できる必要がある）"
    );

    // 適用寸: native（k 適用前）と scaled（k 適用後・実際に窓へ載る物理寸）が弁別可能に揃う。
    assert_eq!(field("native_w"), "4");
    assert_eq!(field("native_h"), "3");
    assert_eq!(
        field("scaled_w"),
        "8",
        "scaled 寸が native のまま（k が届いていない）"
    );
    assert_eq!(
        field("scaled_h"),
        "6",
        "scaled 寸が native のまま（k が届いていない）"
    );

    // 状態照合の結果（初回表示ゆえ差分あり＝窓寸 reconcile 要求を積んだ）。
    assert_eq!(field("size_changed"), "true");
    assert_eq!(field("surface_id"), "1000");
    assert_eq!(field("target_id"), "TargetId(0)");
}

// ── applied_scale／refresh_scale（タスク 3.5・design Flow 2）───────────────────────────────

/// 捕捉イベント列に「表示成立点のログ」が在るか（＝`apply_show` が表示を成立させたか）。
///
/// `refresh_scale` が「何もしなかった」ことの証明に使う——戻り値 `None` だけでは
/// 「再表示したが同寸だった」と区別できないため、表示成立そのものの有無を観測する。
fn has_display_success_log(events: &[CapturedEvent]) -> bool {
    events.iter().any(|e| {
        e.fields
            .get("message")
            .is_some_and(|m| m.contains("表示・マスクを更新"))
    })
}

/// 要件 1.2 観測完了（照会契約 `applied_scale`）: 未登録 target と表示成立前は `None`、k≠1 の表示
/// 成立後は**実適用 k** を返す。
///
/// 恒常 1.0 を返す実装・`attach` 時点で 1.0 を確定させる実装のいずれも RED になる
/// （表示成立前に `Some(1.0)` が出れば「まだ何も適用していない」を塗り潰している）。
#[test]
fn applied_scale_is_none_before_display_and_reports_applied_k_after() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 192);
    let (emo_world, atlas, _golden) = build_target_assets(4, 3, 0x90);

    let mut presenter = EmoPresenter::new();
    assert_eq!(
        presenter.applied_scale(TargetId(7)),
        None,
        "未登録 target は None"
    );

    presenter
        .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
        .expect("attach_target 失敗");
    assert_eq!(
        presenter.applied_scale(TargetId(0)),
        None,
        "attach しただけ（表示成立前）は None——1.0 で塗り潰さない"
    );

    show_ok(&mut presenter, &mut world, TargetId(0), 1000);

    assert_eq!(
        presenter.applied_scale(TargetId(0)),
        Some(2.0),
        "表示成立後は実適用 k（192/96=2.0）を返す"
    );
    // 同一の単一真実源（`applied`）から出る 2 経路が一致する。
    assert_eq!(
        presenter.applied_scale(TargetId(0)),
        presenter.text_slot_view(TargetId(0)).map(|v| v.scale()),
        "applied_scale と TextSlotView::scale() が乖離している（真実源が 2 つある）"
    );
}

/// タスク 3.5 の名指し受け入れ基準・要件 4.1/4.2 観測完了: k=1/1 で表示を確立したのち窓 `DPI` を
/// 192 へ差し替えて `refresh_scale` を呼ぶと——(a) 戻り値が `scaled_extent(2/1, native)`、
/// (b) `applied_scale` が 2.0、(c) readback が k=2/1 のリサンプル結果と全バイト一致する。
///
/// さらに (d) `refresh_scale` が返した要求は**消費済み**であり、続く `take_pending_resize` は
/// `None` を返す——タスク 4.2 が `run_dpi_phase`（`refresh_scale`）と drain フェーズ
/// （`take_pending_resize`）の**両方**を呼ぶため、同一の reconcile が二度出ないことが結線契約である。
#[test]
fn refresh_scale_after_dpi_change_reapplies_new_k() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 96);
    let (emo_world, atlas, native_golden) = build_target_assets(6, 5, 0x91);
    // 同一入力を独立に再現して k=2/1 の golden を作る（presenter の内部値の追認ではない）。
    let (probe_world, probe_atlas, _) = build_target_assets(6, 5, 0x91);
    let k2 = ScaleRatio::new(2, 1).unwrap();
    let (scaled_bytes, native_size, scaled_size) =
        scaled_golden(&probe_world, &probe_atlas, 1000, k2);
    assert_eq!(native_size, (6, 5));
    assert_eq!(scaled_size, (12, 10));

    let mut presenter = EmoPresenter::new();
    presenter
        .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
        .expect("attach_target 失敗");

    // k=1/1 の表示確立（初回表示が積む k₀ 補正要求は取り出して捨てる）。
    show_ok(&mut presenter, &mut world, TargetId(0), 1000);
    assert_eq!(presenter.applied_scale(TargetId(0)), Some(1.0));
    assert_eq!(
        presenter.read_back(TargetId(0)).expect("read_back 失敗"),
        native_golden,
        "前提: k=1/1 の表示は等倍 native 合成"
    );
    assert_eq!(
        presenter.take_pending_resize(TargetId(0)),
        Some(native_size),
        "前提: 初回表示の要求を取り出しておく"
    );

    // モニタ跨ぎ移動・表示スケール変更の決定論的代替（WM_DPICHANGED 相当）。
    set_window_dpi(&mut world, window, 192);

    // (a) 戻り値＝新物理寸。
    assert_eq!(
        presenter.refresh_scale(&mut world, TargetId(0)),
        Some(scaled_size),
        "DPI 変化後の refresh_scale が新物理寸を返さない（再導出・再表示が走っていない）"
    );
    // (b) 照会値が新 k へ追随。
    assert_eq!(
        presenter.applied_scale(TargetId(0)),
        Some(2.0),
        "refresh_scale 後も照会値が旧 k のまま（要件 4.2 の一貫更新が成立していない）"
    );
    // (c) 実際に画面へ載った画素が k=2/1 のリサンプル結果。
    let rb = presenter.read_back(TargetId(0)).expect("read_back 失敗");
    assert_eq!(
        rb, scaled_bytes,
        "表示バイトが k=2/1 のリサンプル結果と一致しない（照会値だけ更新して絵を更新していない）"
    );
    assert_ne!(rb, native_golden, "前提: 2 水準の絵は弁別可能");

    // (d) 要求は refresh_scale が消費済み＝drain フェーズと二重に resize しない（タスク 4.2 の結線契約）。
    assert_eq!(
        presenter.take_pending_resize(TargetId(0)),
        None,
        "refresh_scale が返した要求が drain 側にも残っている（同一フレームで二重 resize になる）"
    );
}

/// 要件 4.1 観測完了（**k 不変なら何もしない**）: DPI を変えずに `refresh_scale` を呼んでも
/// `None` を返し、**再表示を一切行わない**。
///
/// 「何もしない」は戻り値だけでは証明できない（同寸再表示でも `None` になる）ため、2 つの独立した
/// 観測で固定する——(1) キャッシュスロットを同一キーのまま**別の絵**で改竄しておき、readback が
/// 改竄後の絵に**ならない**こと（再表示していればヒットして改竄画が載る）、(2) 表示成立点のログが
/// **1 件も出ていない**こと。
///
/// さらに (3) 未消費の窓寸 reconcile 要求を**握り潰さない**ことを確認する——ゲート不成立時に
/// `pending_resize` を触る実装は、drain フェーズが拾うはずだった初回表示の要求を消してしまう。
#[test]
fn refresh_scale_without_dpi_change_does_nothing() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 192);
    let (emo_world, atlas, _g1000, _g3000) = build_two_face_assets(6, 5);
    let (probe_world, probe_atlas, _, _) = build_two_face_assets(6, 5);
    let k2 = ScaleRatio::new(2, 1).unwrap();
    let (scaled_1000, _native, scaled_size) =
        scaled_golden(&probe_world, &probe_atlas, 1000, k2);

    let mut presenter = EmoPresenter::new();
    presenter
        .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
        .expect("attach_target 失敗");
    show_ok(&mut presenter, &mut world, TargetId(0), 1000);
    assert_eq!(
        presenter.read_back(TargetId(0)).expect("read_back 失敗"),
        scaled_1000
    );
    // 初回表示の要求は**あえて取り出さない**（(3) の握り潰し検査のため）。

    // 改竄プローブ: 同一キーのスロットを別の絵（面 3000 の k 適用結果）で上書きする。
    let tampered = {
        let mut composer = Composer::new();
        let native = composer
            .compose(
                &probe_world,
                &probe_atlas,
                3000,
                &BindSet::default(),
                &PatternState::default(),
            )
            .expect("面 3000 の合成は Ok");
        let mut scaled = ComposedSurface::new(0, 0);
        resample(&native, k2, &mut scaled);
        scaled
    };
    let tampered_bytes = tampered.bytes().to_vec();
    assert_ne!(
        tampered_bytes, scaled_1000,
        "プローブ前提: 別の絵であること"
    );
    presenter
        .targets
        .get_mut(&TargetId(0))
        .unwrap()
        .cache
        .insert(
            1000,
            BindSet::default(),
            PatternState::default(),
            k2,
            tampered,
        );

    // DPI は据え置き（k 不変）。
    let cap = CaptureSubscriber::default();
    let got = tracing::subscriber::with_default(cap.clone(), || {
        presenter.refresh_scale(&mut world, TargetId(0))
    });

    assert_eq!(got, None, "k 不変なのに新物理寸を返している");
    // (1) 改竄画が載っていない＝再表示していない。
    assert_eq!(
        presenter.read_back(TargetId(0)).expect("read_back 失敗"),
        scaled_1000,
        "k 不変なのに再表示している（改竄画が画面へ載った＝無駄な表示更新）"
    );
    // (2) 表示成立点のログが 1 件も出ていない。
    let events = cap.0.lock().expect("捕捉バッファ").clone();
    assert!(
        !has_display_success_log(&events),
        "k 不変なのに表示成立点のログが出ている（再表示が走った）: {events:?}"
    );
    // (3) 未消費の要求を握り潰していない（drain フェーズが拾えること）。
    assert_eq!(
        presenter.take_pending_resize(TargetId(0)),
        Some(scaled_size),
        "ゲート不成立の refresh_scale が未消費の窓寸 reconcile 要求を消している（取りこぼし）"
    );
}

/// 要件 4.1 観測完了（**再表示入力が無ければ何もしない**）: 一度も表示が成立していない target は
/// DPI が変わっても `refresh_scale` が `None`＝副作用なしであること。
///
/// 実際に閉じるのは**可視ゲート**である（`visible` と `last_show` はいずれも表示成立点でのみ
/// 真になるため、未表示 target は `visible == false` で先に弾かれる）。`last_show` ゲートは
/// 多層防御であり、可視ゲートを外す変異を単独で捕まえる（設計の 3 ゲート記述をそのまま保つ）。
///
/// 未登録 target も同様に `None`（登録有無で panic しない）——こちらが本テストの非自明な檻。
#[test]
fn refresh_scale_without_last_show_input_does_nothing() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 96);
    let (emo_world, atlas, _golden) = build_target_assets(4, 3, 0x92);

    let mut presenter = EmoPresenter::new();
    assert_eq!(
        presenter.refresh_scale(&mut world, TargetId(7)),
        None,
        "未登録 target は None（panic しない）"
    );

    presenter
        .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
        .expect("attach_target 失敗");
    // 一度も show していない状態で DPI を変える。
    set_window_dpi(&mut world, window, 192);

    let cap = CaptureSubscriber::default();
    let got = tracing::subscriber::with_default(cap.clone(), || {
        presenter.refresh_scale(&mut world, TargetId(0))
    });

    assert_eq!(got, None, "再表示入力が無いのに新物理寸を返している");
    let events = cap.0.lock().expect("捕捉バッファ").clone();
    assert!(
        !has_display_success_log(&events),
        "再表示入力が無いのに表示が成立している: {events:?}"
    );
    assert_eq!(
        presenter.applied_scale(TargetId(0)),
        None,
        "表示は依然として未成立"
    );
    assert_eq!(presenter.current_surface_id(TargetId(0)), None);
    assert_eq!(
        presenter.take_pending_resize(TargetId(0)),
        None,
        "副作用（窓寸 reconcile 要求）が生じている"
    );
    assert!(
        presenter.read_back(TargetId(0)).is_err(),
        "供給面が生成されている（表示していないのに資源を作った）"
    );
}

/// 要件 4.1/3.2 観測完了（**`Hide` 済み target を蘇らせない**）: 非表示の target は DPI が変わっても
/// 再表示しない——DPI 変化は「見えているものを描き直す」事象であって表示を復活させる事象ではない。
///
/// 可視ゲートを外した実装では、`Hide` した窓が DPI 変化だけで再出現する（`current_surface_id` が
/// `Some` に戻る）ため RED になる。
#[test]
fn refresh_scale_does_not_resurrect_hidden_target() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 96);
    let (emo_world, atlas, _golden) = build_target_assets(4, 3, 0x93);

    let mut presenter = EmoPresenter::new();
    presenter
        .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
        .expect("attach_target 失敗");
    show_ok(&mut presenter, &mut world, TargetId(0), 1000);
    let _ = presenter.take_pending_resize(TargetId(0));

    // `\s[-1]` 相当で非表示にする（キャッシュ・供給面・last_show は保持される）。
    let (tx, rx) = reply_channel::<PresentOutcome>();
    presenter.apply(
        &mut world,
        PresentCommand::Hide {
            target: TargetId(0),
            reply: Some(tx),
        },
    );
    assert!(matches!(
        rx.recv_timeout(Duration::from_secs(10)),
        Ok(Ok(()))
    ));
    assert_eq!(
        presenter.current_surface_id(TargetId(0)),
        None,
        "前提: 非表示"
    );
    assert!(
        presenter
            .targets
            .get(&TargetId(0))
            .unwrap()
            .last_show
            .is_some(),
        "前提: Hide しても再表示入力は保持される（可視ゲートだけが再表示を止める）"
    );

    set_window_dpi(&mut world, window, 192);
    let cap = CaptureSubscriber::default();
    let got = tracing::subscriber::with_default(cap.clone(), || {
        presenter.refresh_scale(&mut world, TargetId(0))
    });

    assert_eq!(got, None, "非表示 target が新物理寸を報告している");
    let events = cap.0.lock().expect("捕捉バッファ").clone();
    assert!(
        !has_display_success_log(&events),
        "非表示 target が DPI 変化だけで再表示された（蘇生）: {events:?}"
    );
    let t = presenter.targets.get(&TargetId(0)).unwrap();
    assert!(!t.visible, "非表示のままでなければならない");
    assert_eq!(
        presenter.current_surface_id(TargetId(0)),
        None,
        "現サーフェスが復活している（蘇生した）"
    );
    assert_eq!(
        presenter.applied_scale(TargetId(0)),
        Some(1.0),
        "再表示していない以上、実適用 k は前値のまま"
    );
    assert_eq!(
        presenter.take_pending_resize(TargetId(0)),
        None,
        "副作用（窓寸 reconcile 要求）が生じている"
    );
}

/// 要件 1.4/4.1 観測完了（**DPI 取得不能を 96 で捏造しない**・ゲート判定の帰属可能性）: 窓の `DPI`
/// component が失われても `refresh_scale` は縮退 k（`app_scale × 1/1`）を導出し、前回適用 k と等しい
/// ため**再表示しない**。
///
/// # `author_dpi` に **192**（非 96）を使う理由
///
/// `apply_show` 側の縮退テストと同じ論法である。author_dpi=96 で組むと、縮退の答（1/1）と
/// 「component 不在を 96 で捏造した場合の答」（96/96＝1/1）が数値として区別できず、
/// `world.get::<DPI>(..)` に `.or(Some((96, 96)))` を足す実装ミスを素通しさせる。author_dpi=192 なら
/// 捏造時の k は `96/192 = 1/2` となり、前回適用 k（1/1）と**異なる**ためゲートを通過して再表示が走り、
/// 戻り値が `Some((2, 2))` になる——本テストはそれを RED として捕らえる。
#[test]
fn refresh_scale_does_not_fabricate_dpi_when_component_is_absent() {
    let mut world = make_world_with_gpu();
    // 窓 DPI 192・author_dpi 192 ゆえ k=1/1（縮退値と一致するが、捏造値 96/192=1/2 とは一致しない）。
    let window = spawn_window_with_dpi(&mut world, 192);
    let (emo_world, atlas, native_golden) = build_target_assets(4, 3, 0x95);

    let mut presenter = EmoPresenter::new();
    presenter
        .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 192)
        .expect("attach_target 失敗");
    show_ok(&mut presenter, &mut world, TargetId(0), 1000);
    assert_eq!(
        presenter.applied_scale(TargetId(0)),
        Some(1.0),
        "前提: 192/192 で k=1/1"
    );
    let _ = presenter.take_pending_resize(TargetId(0));

    // DPI 取得不能の決定論的代替（本番では起こらない＝component を落とす）。
    world.entity_mut(window).remove::<DPI>();
    assert!(
        world.get::<DPI>(window).is_none(),
        "前提: DPI component 不在"
    );

    let cap = CaptureSubscriber::default();
    let got = tracing::subscriber::with_default(cap.clone(), || {
        presenter.refresh_scale(&mut world, TargetId(0))
    });

    assert_eq!(
        got, None,
        "DPI 不在を 96 で捏造している（k=1/2 と誤導出して再表示が走った）"
    );
    let events = cap.0.lock().expect("捕捉バッファ").clone();
    assert!(
        !has_display_success_log(&events),
        "DPI 不在の縮退で再表示が走っている: {events:?}"
    );
    assert_eq!(
        presenter.applied_scale(TargetId(0)),
        Some(1.0),
        "縮退後も実適用 k は 1/1 のまま"
    );
    assert_eq!(
        presenter.read_back(TargetId(0)).expect("read_back 失敗"),
        native_golden,
        "表示が縮小されている（96 捏造で 1/2 が適用された）"
    );
}

/// 要件 4.4 観測完了（**再表示の失敗は前 k・前表示を維持し、黙らない**）: `refresh_scale` の内部
/// 再 show が失敗しても、直前の k による表示がそのまま残る。
///
/// 失敗は `last_show` の surface id を解決不能値へ差し替えて注入する——ゴースト再読込で
/// `EmoWorld` から面が消えた場合に実在する状況であり、かつ 2 個目の `Compositor` を作らない
/// （要件 5.3 の AV 非再導入）。供給面生成の失敗経路は初回表示でしか通らない（`chain` が既に在る）
/// ため、表示確立**後**の失敗を作るにはこの注入が要る。
///
/// `apply_show` 自身も失敗を error! するが、それは「合成に失敗した」ことしか語らない。DPI 追従の
/// 文脈（どの k からどの k への再導出が落ちたか・前表示を維持したこと）は `refresh_scale` でしか
/// 分からないため、本経路は専用の error! を出す（無言の失敗経路を作らない）。
#[test]
fn refresh_scale_failure_keeps_previous_display_and_k() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 96);
    let (emo_world, atlas, native_golden) = build_target_assets(4, 3, 0x94);

    let mut presenter = EmoPresenter::new();
    presenter
        .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
        .expect("attach_target 失敗");
    show_ok(&mut presenter, &mut world, TargetId(0), 1000);
    assert_eq!(
        presenter.take_pending_resize(TargetId(0)),
        Some((4, 3)),
        "前提: 初回表示の要求を取り出しておく"
    );

    // 失敗注入: 再表示入力の surface id を解決不能値へ差し替える。
    presenter
        .targets
        .get_mut(&TargetId(0))
        .unwrap()
        .last_show
        .as_mut()
        .expect("前提: 表示成立済みゆえ last_show は Some")
        .0 = 9999;

    set_window_dpi(&mut world, window, 192);
    let cap = CaptureSubscriber::default();
    let got = tracing::subscriber::with_default(cap.clone(), || {
        presenter.refresh_scale(&mut world, TargetId(0))
    });

    assert_eq!(
        got, None,
        "失敗したのに新物理寸を報告している（要件 4.4 違反）"
    );

    // 前 k・前表示・現サーフェスがすべて据え置き（表示を失わない）。
    assert_eq!(
        presenter.applied_scale(TargetId(0)),
        Some(1.0),
        "失敗したのに照会値が新 k へ動いている（前 k 維持の違反）"
    );
    assert_eq!(
        presenter.read_back(TargetId(0)).expect("read_back 失敗"),
        native_golden,
        "失敗したのに表示が失われた／変わった（前表示維持の違反）"
    );
    assert_eq!(
        presenter.current_surface_id(TargetId(0)),
        Some(1000),
        "失敗したのに現サーフェスが失われた"
    );
    assert!(
        presenter.targets.get(&TargetId(0)).unwrap().visible,
        "失敗で表示が消えている（表示を失わない縮退の違反）"
    );
    assert_eq!(
        presenter.take_pending_resize(TargetId(0)),
        None,
        "失敗が窓寸 reconcile 要求を積んでいる"
    );

    // 無言の失敗経路を作らない: refresh_scale 固有の error! が出ている。
    let events = cap.0.lock().expect("捕捉バッファ").clone();
    let err = events
        .iter()
        .find(|e| {
            e.fields
                .get("message")
                .is_some_and(|m| m.contains("refresh_scale: 再表示が成立せず"))
        })
        .unwrap_or_else(|| panic!("refresh_scale の失敗が無言（専用ログが無い）: {events:?}"));
    assert_eq!(
        err.level,
        tracing::Level::ERROR,
        "再表示失敗が error! でない（要件 4.4 のログ規律）"
    );
    assert!(
        !has_display_success_log(&events),
        "失敗したのに表示成立点のログが出ている: {events:?}"
    );
}

/// 捕捉イベント列のうち **R1.6 の防御 warn** の件数（level と固有文言の双方で選別する）。
fn applied_absent_warn_count(events: &[CapturedEvent]) -> usize {
    events
        .iter()
        .filter(|e| {
            e.level == tracing::Level::WARN
                && e.fields
                    .get("message")
                    .is_some_and(|m| m.contains("適用スケール未確定"))
        })
        .count()
}

/// 要件 1.6 観測完了（DD-5 の防御分岐・**述語そのものの檻**）: 「表示中サーフェスがあるのに
/// `applied` が無い」状態でのみ `warn!` が呼出 1 回につき 1 件鳴り、正常縮退（未表示 scope・
/// 未登録 target）では **1 件も鳴らない**。あわせて (a) panic しない (b) k=1.0 と同一結果を返す、
/// を固定する。
///
/// # なぜ「鳴る」「鳴らない」を 1 つの subscriber 下で観測するのか
///
/// `tracing` の callsite interest はプロセス大域にキャッシュされるため、「ログが出ない」ことの
/// 主張は subscriber 未設置の並列テストが同一 callsite を先に踏むと**恒真に**なり得る（檻が
/// 何も守らなくなる）。本テストは陽性（1 件鳴る）と陰性（追加で鳴らない）を**同一の
/// `with_default` スコープ・同一 callsite** で観測するため、陰性主張は「捕捉が死んでいない」
/// ことが同じ捕捉列で証明された上でのみ成立する。
///
/// # 殺す誤実装
///
/// - 述語を `applied.is_none()` 単独へ潰す（未表示・未登録でも鳴る）→ warn 件数 4 で RED
///   （実挙動としては未表示 scope 上のマウス移動ごとにログ洪水を作る退行）
/// - `warn!` を削除する → warn 件数 0 で RED
/// - `applied` 不在時に `ScaleRatio::ONE` 以外で続行する → region/surface_point 期待で RED
///   （点 (180,96) は k=1 で `Head`・k=2 なら `None`、点 (360,192) は k=1 で `None`・k=2 なら
///   `Head` と**双方向に**割れるので、恒等以外の k は必ずどちらかで外れる）
/// - `applied` 不在で panic／早期 return する → 判定そのものが取れず RED
#[test]
fn applied_absent_with_visible_surface_warns_once_and_degradations_stay_silent() {
    let mut world = World::new();
    let mut presenter = EmoPresenter::new();

    // T0: 面あり・applied なし（R1.6 の防御分岐・現行公開 API では作れない状態）。
    attach_hit_target(&mut presenter, &mut world, TargetId(0));
    force_current_surface(&mut presenter, TargetId(0), 1000);
    // T1: attach のみ（未表示 scope）＝正常縮退。TargetId(9) は未登録＝正常縮退。
    attach_hit_target(&mut presenter, &mut world, TargetId(1));

    // 前提の明示（檻が空虚でないこと＝狙った状態が本当に組めていること）。
    assert_eq!(
        presenter.applied_ratio(TargetId(0)),
        None,
        "前提: T0 は applied 不在"
    );
    assert_eq!(
        presenter.current_surface_id(TargetId(0)),
        Some(1000),
        "前提: T0 は表示中サーフェスあり"
    );
    assert_eq!(
        presenter.current_surface_id(TargetId(1)),
        None,
        "前提: T1 は未表示"
    );

    let cap = CaptureSubscriber::default();
    let (defensive_hit, defensive_miss, unshown, unregistered) =
        tracing::subscriber::with_default(cap.clone(), || {
            // (1) 防御分岐: 面あり・applied なし → 鳴る。
            let a = presenter.hit_region_client(TargetId(0), 180, 96);
            let b = presenter.hit_region_client(TargetId(0), 360, 192);
            // (2) 正常縮退: 同一 callsite に対して鳴らない側を同一スコープで観測する。
            let c = presenter.hit_region_client(TargetId(1), 180, 96);
            let d = presenter.hit_region_client(TargetId(9), 180, 96);
            (a, b, c, d)
        });

    let events = cap.0.lock().expect("捕捉バッファ").clone();

    // (a) panic しない: ここへ到達している時点で 4 呼出すべてが値を返している。
    // (b) k=1.0 と同一結果（ScaleRatio::ONE 相当で続行している）。
    assert_eq!(
        defensive_hit.region,
        Some("Head"),
        "applied 不在は k=1.0 相当で照合を続行すること（判定を失わせない・要件 1.6）"
    );
    assert_eq!(defensive_hit.surface_point, (180, 96), "k=1.0 相当＝無縮約");
    assert_eq!(
        defensive_miss.region, None,
        "k=1.0 相当なら (360,192) は領域外（k=2 で続行していれば Head になり RED）"
    );
    assert_eq!(defensive_miss.surface_point, (360, 192));
    // 既存の native px 入口（k を一切参照しない）と region が完全一致する＝「k=1.0 と同一結果」。
    assert_eq!(
        defensive_hit.region,
        presenter.hit_region(TargetId(0), 180, 96),
        "縮退結果は k=1.0（無縮約）の既存入口と一致すること"
    );
    assert_eq!(
        defensive_miss.region,
        presenter.hit_region(TargetId(0), 360, 192),
        "縮退結果は k=1.0（無縮約）の既存入口と一致すること"
    );

    // (c) warn 経路を実際に通る（陽性）。防御分岐の呼出 1 回につき warn 1 件ゆえ、
    //     防御呼出 2 回で **ちょうど 2 件**（0 なら防御ログ欠落＝ログ無し失敗経路）。
    assert_eq!(
        applied_absent_warn_count(&events),
        2,
        "面あり・applied なしの呼出 1 回につき warn 1 件が鳴ること（防御呼出 2 回＝2 件。\
         0 なら防御ログ欠落・3 以上なら述語が正常縮退まで巻き込んでいる）: {events:?}"
    );
    // 各 warn が「どの窓のどの座標で内部不変条件が破れたか」を載せている（呼出ごとの識別）。
    for (want_x, want_y) in [("180", "96"), ("360", "192")] {
        let warn = events
            .iter()
            .find(|e| {
                e.level == tracing::Level::WARN
                    && e.fields
                        .get("message")
                        .is_some_and(|m| m.contains("適用スケール未確定"))
                    && e.fields.get("client_x").map(String::as_str) == Some(want_x)
            })
            .unwrap_or_else(|| {
                panic!("client_x={want_x} の防御 warn が捕捉されていない: {events:?}")
            });
        assert_eq!(
            warn.fields.get("target").map(String::as_str),
            Some("TargetId(0)"),
            "warn が対象 target を載せていない（どの窓で不変条件が破れたか特定できない）: {:?}",
            warn.fields
        );
        assert_eq!(
            warn.fields.get("client_y").map(String::as_str),
            Some(want_y),
            "warn が client 座標を載せていない: {:?}",
            warn.fields
        );
    }

    // (d) 正常縮退は同一 callsite で **1 件も増やさない**（陰性）。捕捉が生きていることは
    //     直前の陽性 1 件が証明済みゆえ、この陰性主張は恒真になり得ない。
    assert_eq!(
        unshown.region, None,
        "未表示 scope は正常縮退（region なし）"
    );
    assert_eq!(
        unregistered.region, None,
        "未登録 target は正常縮退（region なし）"
    );
    assert_eq!(
        applied_absent_warn_count(&events),
        2,
        "未表示 scope・未登録 target で warn を足してはならない（述語を applied.is_none() 単独へ\
         潰すとマウス移動ごとのログ洪水になる。防御呼出 2 回分の 2 件から増えない）: {events:?}"
    );
}
