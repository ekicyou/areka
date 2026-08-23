use super::*;

/// 本仕様の既定政策（作者基準 DPI 96・アプリ管理拡大率 1/1 固定シーム）。
fn policy_96() -> ScalePolicy {
    ScalePolicy::new(DEFAULT_AUTHOR_DPI, ScaleRatio::ONE)
}

/// 縮退フラグが 1 つも立っていない（＝正常経路を通った）ことを主張する。
fn assert_no_degradation(d: ScaleDecision) {
    assert!(
        !d.dpi_missing && !d.window_dpi_invalid && !d.anisotropic && !d.author_dpi_normalized,
        "正常経路では縮退分岐が立たない: {d:?}"
    );
}

/// 要件 1.1: 正常経路は `窓 DPI ÷ author_dpi` の既約有理数を返す（DPI 対照表・連続 k）。
///
/// 125%（120/96＝5/4）が段階丸めで潰れないことが要件 2.2 の前提であるため、
/// 非整数倍の水準を対照表へ含める。
#[test]
fn derive_scale_follows_dpi_table() {
    for (dpi, expect) in [
        (96u16, (1u32, 1u32)),
        (120, (5, 4)),
        (144, (3, 2)),
        (168, (7, 4)),
        (192, (2, 1)),
        (72, (3, 4)),
    ] {
        let k = derive_scale(policy_96(), Some((dpi, dpi)));
        assert_eq!(
            k,
            ScaleRatio::new(expect.0, expect.1).unwrap(),
            "dpi={dpi}: 窓 DPI ÷ author_dpi の既約有理数"
        );
        assert_no_degradation(classify(policy_96(), Some((dpi, dpi))));
    }
    // 隣接水準が同一 k へ潰れない（整数段階量子化の不在＝要件 2.2 の連続性）。
    assert_ne!(
        derive_scale(policy_96(), Some((120, 120))),
        derive_scale(policy_96(), Some((144, 144)))
    );
}

/// 要件 1.1: author_dpi は k の分母として実際に効く（96 ハードコードでない証拠）。
#[test]
fn derive_scale_uses_declared_author_dpi_as_denominator() {
    let p = ScalePolicy::new(120, ScaleRatio::ONE);
    assert_eq!(
        derive_scale(p, Some((240, 240))),
        ScaleRatio::new(2, 1).unwrap(),
        "author_dpi=120・窓 240 → 2/1"
    );
    assert_eq!(
        derive_scale(p, Some((96, 96))),
        ScaleRatio::new(4, 5).unwrap(),
        "author_dpi が窓 DPI より大きければ k<1（縮小）"
    );
}

/// 要件 1.3: 窓 DPI ＝ author_dpi かつ app_scale=ONE のとき恒等（k=1.0・等倍表示と同一）。
#[test]
fn derive_scale_is_identity_when_window_dpi_equals_author_dpi() {
    assert!(derive_scale(policy_96(), Some((96, 96))).is_identity());
    // 作者が 144 を宣言していれば恒等点も 144 へ移る（恒等は 96 固定ではない）。
    let p144 = ScalePolicy::new(144, ScaleRatio::ONE);
    assert!(derive_scale(p144, Some((144, 144))).is_identity());
    assert!(!derive_scale(p144, Some((96, 96))).is_identity());
    assert!(!derive_scale(policy_96(), Some((192, 192))).is_identity());
}

/// 要件 1.4: DPI 不在（`DPI` component 取得不能）は `app_scale × 1/1` へ縮退する。
///
/// アプリ管理拡大率が非 ONE でも「DPI 由来 k のみ 1/1 になる」ことまで固定する
/// （app_scale ごと捨てる実装との差を検出する）。
#[test]
fn derive_scale_without_dpi_degrades_to_app_scale() {
    assert_eq!(
        derive_scale(policy_96(), None),
        ScaleRatio::ONE,
        "app=ONE なら k=1.0（表示を失わない）"
    );
    let app2 = ScaleRatio::new(2, 1).unwrap();
    assert_eq!(
        derive_scale(ScalePolicy::new(96, app2), None),
        app2,
        "DPI 由来 k のみ 1/1 へ縮退し、アプリ管理拡大率は保たれる"
    );

    let d = classify(policy_96(), None);
    assert!(d.dpi_missing, "DPI 不在分岐が選択される: {d:?}");
    assert!(!d.window_dpi_invalid && !d.anisotropic && !d.author_dpi_normalized);
    // 正常経路では立たない（分岐の非空虚性）。
    assert!(!classify(policy_96(), Some((96, 96))).dpi_missing);
}

/// 設計 D2: `dpi_x != dpi_y` は dpi_x を採用する（単一スカラー規約）。
///
/// 「パニックしない」ではなく**採用軸そのもの**を固定する——dpi_y 採用や平均採用なら落ちる。
#[test]
fn derive_scale_adopts_dpi_x_when_axes_differ() {
    assert_eq!(
        derive_scale(policy_96(), Some((192, 96))),
        ScaleRatio::new(2, 1).unwrap(),
        "dpi_x=192 を採用（dpi_y=96 採用なら 1/1 になる）"
    );
    assert_eq!(
        derive_scale(policy_96(), Some((96, 192))),
        ScaleRatio::ONE,
        "dpi_x=96 を採用（dpi_y=192 採用なら 2/1 になる）"
    );
    assert_eq!(
        derive_scale(policy_96(), Some((120, 192))),
        ScaleRatio::new(5, 4).unwrap(),
        "非整数 k でも dpi_x 採用（平均採用なら 13/8 相当になる）"
    );

    let d = classify(policy_96(), Some((192, 96)));
    assert!(d.anisotropic, "異軸分岐が選択される: {d:?}");
    assert!(!d.dpi_missing && !d.window_dpi_invalid);
    assert!(
        !classify(policy_96(), Some((96, 96))).anisotropic,
        "同軸では立たない（非空虚性）"
    );
}

/// 要件 1.1/1.4: `author_dpi == 0` は 96 へ正規化される（分母ゼロで表示を失わない）。
///
/// 構造体リテラル直書き（[`ScalePolicy::new`] 迂回）でも [`derive_scale`] 側の最終防衛が
/// 効くことを固定する。
#[test]
fn derive_scale_normalizes_zero_author_dpi() {
    let bare = ScalePolicy {
        author_dpi: 0,
        app_scale: ScaleRatio::ONE,
    };
    for dpi in [96u16, 120, 192] {
        assert_eq!(
            derive_scale(bare, Some((dpi, dpi))),
            derive_scale(policy_96(), Some((dpi, dpi))),
            "author_dpi=0 は author_dpi=96 と同一の k を与える（dpi={dpi}）"
        );
    }
    let d = classify(bare, Some((192, 192)));
    assert!(d.author_dpi_normalized, "正規化分岐が選択される: {d:?}");
    assert!(!d.dpi_missing && !d.window_dpi_invalid);
    assert!(
        !classify(policy_96(), Some((192, 192))).author_dpi_normalized,
        "非ゼロでは立たない（非空虚性）"
    );

    // 構築時（正規の入口）にも正規化される。
    assert_eq!(
        ScalePolicy::new(0, ScaleRatio::ONE).author_dpi,
        DEFAULT_AUTHOR_DPI
    );
    assert_eq!(ScalePolicy::new(120, ScaleRatio::ONE).author_dpi, 120);
    assert_eq!(ScalePolicy::default(), policy_96());
}

/// 要件 1.4: 窓 DPI が 0（不正値）でも比を構築せず `app_scale × 1/1` へ縮退する。
#[test]
fn derive_scale_degrades_on_zero_window_dpi() {
    assert_eq!(derive_scale(policy_96(), Some((0, 0))), ScaleRatio::ONE);
    let app2 = ScaleRatio::new(2, 1).unwrap();
    assert_eq!(
        derive_scale(ScalePolicy::new(96, app2), Some((0, 0))),
        app2,
        "アプリ管理拡大率は保たれる"
    );

    let d = classify(policy_96(), Some((0, 96)));
    assert!(d.window_dpi_invalid, "窓 DPI 不正分岐が選択される: {d:?}");
    assert!(!d.dpi_missing, "DPI 不在とは別分岐（別ログ）");
    assert!(
        d.anisotropic,
        "0 と 96 は異軸でもある（フラグは独立に立つ）"
    );
}

/// 要件 1.6: 最終拡大率＝アプリ管理拡大率 × DPI 由来 k（2 因子乗算が実在する証拠）。
///
/// 本仕様の本番値は `app_scale == ONE` 固定だが、非 ONE が正しく乗るかを固定しておかないと
/// 「シームがある」という主張は空虚になる（将来 spec が導入した瞬間に壊れる）。
#[test]
fn derive_scale_multiplies_app_scale_seam() {
    let app2 = ScaleRatio::new(2, 1).unwrap();
    assert_eq!(
        derive_scale(ScalePolicy::new(96, app2), Some((120, 120))),
        ScaleRatio::new(5, 2).unwrap(),
        "アプリ 2.0 × DPI 1.25 = 2.5"
    );
    let half = ScaleRatio::new(1, 2).unwrap();
    assert_eq!(
        derive_scale(ScalePolicy::new(96, half), Some((192, 192))),
        ScaleRatio::ONE,
        "アプリ 0.5 × DPI 2.0 = 1.0（相殺）"
    );
    // ONE 固定シームは DPI 由来 k をそのまま通す（恒等元）。
    assert_eq!(
        derive_scale(policy_96(), Some((120, 120))),
        ScaleRatio::new(5, 4).unwrap()
    );
}

/// Invariants: 同一入力→同一出力（純関数・隠れた可変状態を持たない）。
///
/// 「初回のみ警告」のような抑止状態を内部へ持てば、同一入力の反復で分岐結果が変化し得る。
/// 反復して `ScaleDecision` ごと一致することで、そのような状態が無いことを固定する。
#[test]
fn derive_scale_is_deterministic() {
    let p = ScalePolicy::new(96, ScaleRatio::new(3, 2).unwrap());
    for dpi in [
        None,
        Some((96u16, 96u16)),
        Some((120, 96)),
        Some((0, 0)),
        Some((192, 192)),
    ] {
        let first = classify(p, dpi);
        for _ in 0..3 {
            assert_eq!(classify(p, dpi), first, "dpi={dpi:?}: 分岐結果が反復不変");
            assert_eq!(
                derive_scale(p, dpi),
                first.scale,
                "dpi={dpi:?}: k が反復不変"
            );
        }
    }
}

/// 非パニック: 極値（`u16::MAX`・0 の全組合せ）でも有効な k を返す（表示を失わない）。
#[test]
fn derive_scale_never_panics_and_always_yields_usable_scale() {
    for author in [0u16, 1, 96, u16::MAX] {
        for dpi in [
            None,
            Some((0u16, 0u16)),
            Some((1, 1)),
            Some((u16::MAX, 1)),
            Some((1, u16::MAX)),
            Some((u16::MAX, u16::MAX)),
        ] {
            let policy = ScalePolicy {
                author_dpi: author,
                app_scale: ScaleRatio::ONE,
            };
            let k = derive_scale(policy, dpi);
            assert!(
                k.as_f32() > 0.0 && k.as_f32().is_finite(),
                "author={author} dpi={dpi:?}: k は常に正の有限値（表示が消えない）"
            );
            // 有効な k は寸法へ適用でき、非ゼロ原寸は最小 1px を保つ（丸め権威側の契約）。
            assert!(k.scale_len(100) >= 1);
        }
    }
}

// ── 縮退ログ発火の檻（task 6.2・task 1.4 申し送りの回収）─────────────────────────
//
// task 1.4 時点で檻に入っていたのは私有 `ScaleDecision` の**分岐選択**だけであり、
// `error!`／`warn!` が実際に発火するかは無検査だった（steering `logging.md` の
// 「ログ無し失敗経路の禁止」＝縮退の唯一の観測点が空証明のまま）。ここで実行テストへ落とす。
//
// # 硬化機構は 1 箇所にしかない（「スレッドローカルゆえ安全」は誤り）
//
// 捕捉窓そのものは共有 crate `log-capture-kit` へ委譲する（spec:
// areka-P0-test-cage-determinism・要件 1.5／2.2）。以前ここに在った常駐 probe と最小
// subscriber は、同 crate `balloon.rs` の檻へも写し取られており、写し損ねた側だけが静かに
// 嘘をつく形だったため撤去した。硬化の定義はワークスペースで 1 箇所しか無い。
//
// 「`with_default` はスレッドローカルだから並行実行でも干渉しない」は**誤り**である。
// 差し替わるのはスレッドローカルの既定 dispatcher だけで、「そのログを評価するか」を決める
// callsite の interest キャッシュは**プロセス全体で 1 つ**しかなく、その発行点をプロセス内で
// 最初に踏んだスレッドの判定が焼き付く（`tracing-core` の `DefaultCallsite::interest()` →
// `register()` → `Dispatchers::rebuilder()` の経路。捕捉窓を持たないスレッドの既定は
// `NoSubscriber` で、その `register_callsite` は `Interest::never()` を返す）。焼き付いた
// `never` は `interest.is_never()` の早期 return でイベントを捨てるため、**捕捉窓の内側でも
// 取りこぼす**——起きるのは他テストのイベントの混入ではなく、自分の観測の欠落である。
// 本ファイルの縮退ログ callsite は捕捉しない他テスト（`derive_scale_*` の値検査群・
// `presenter.rs` の GPU テスト群）と共有されているため、この経路は実在する。結果、不在の
// 主張は捕捉 0 件のまま静かに緑になり（偽陰性）、存在の主張は捕捉 0 件で確率的に赤になる
// （偽陽性）。
//
// 共有機構は ⑴ プロセス寿命の probe 常駐（`has_just_one` を恒久的に偽へ落として `never` の
// 合成を封じる）⑵ 窓の内側での interest 再計算（常駐より前に焼かれた分の解消）⑶ 窓の内側で
// 発火する対照イベント（番兵）による空振り検出、の 3 点でこれを塞ぐ。番兵は返却前に取り除か
// れるので呼出側の件数・主張は変わらない。捕捉されるのは呼出スレッドで同期的に発火した
// イベントだけである点は移行前と同じ。逐条解説（`tracing-core` の実コード引用つき）は
// `log_capture_kit` の crate doc と同 crate の `src/probe.rs` にある。

use log_capture_kit::CapturedEvent;

/// 欠落を失敗にしてフィールドの **Debug 表現**を引く拡張（移行前の `CapturedEvent::field` と
/// 同一規則——フィールド名も契約のうちなので、無ければ全フィールドを添えて落とす）。
///
/// メソッド名を `field` にしないのは、[`CapturedEvent`] の固有メソッド `field`
/// （`Option<&str>` を返す）が拡張トレイトより優先され、拡張トレイト側が到達不能になるためである（この crate の呼出側は `assert_eq!` で `&str` と比べているので実際にはコンパイルが通らないが、balloon 側の同型は引用符剥がしが黙って消えて緑のままになる）。
trait ExpectField {
    /// フィールドの Debug 表現。欠落は panic。
    fn expect_field(&self, name: &str) -> &str;
}

impl ExpectField for CapturedEvent {
    fn expect_field(&self, name: &str) -> &str {
        self.field(name)
            .unwrap_or_else(|| panic!("ログフィールド `{name}` が無い: {:?}", self.fields))
    }
}

/// クロージャ実行中に**現在のスレッド**で発火した tracing イベントを戻り値と共に返す。
///
/// 捕捉と硬化は硬化機構の唯一の定義元 [`log_capture_kit::capture`] が行う。捕捉が働いて
/// いなければ空の結果を静かに返さず panic する。
fn capture<R, F: FnOnce() -> R>(f: F) -> (R, Vec<CapturedEvent>) {
    log_capture_kit::capture(f)
}

/// メッセージに `needle` を含むイベントが**ちょうど 1 件**在ることを主張して返す。
fn expect_one<'a>(events: &'a [CapturedEvent], needle: &str) -> &'a CapturedEvent {
    let hits: Vec<&CapturedEvent> = events
        .iter()
        .filter(|e| e.message().contains(needle))
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "`{needle}` を含むログがちょうど 1 件ではない: {events:?}"
    );
    hits[0]
}

/// メッセージに `needle` を含むイベント数。
fn count_msg(events: &[CapturedEvent], needle: &str) -> usize {
    events
        .iter()
        .filter(|e| e.message().contains(needle))
        .count()
}

/// 要件 1.4／設計「Error Handling」: DPI 不在縮退は **`error!`** を発火する。
///
/// レベル（error）とメッセージ識別子・構造化フィールド（`author_dpi`/`app_scale`/`k`）を
/// 契約として固定する。`k` フィールドには**実適用値**が載る（app_scale 非 ONE で `1.0` 固定
/// でないことまで見るので、ログ値を定数直書きへ変異させると落ちる）。
#[test]
fn derive_scale_missing_dpi_emits_error_log() {
    let p = policy_96();
    let (k, events) = capture(|| derive_scale(p, None));
    assert_eq!(k, ScaleRatio::ONE);

    let ev = expect_one(&events, "窓 DPI を取得できない");
    assert_eq!(
        ev.level,
        tracing::Level::ERROR,
        "DPI 取得不能は error 格（warn/debug へ落とすと縮退が観測できない）: {ev:?}"
    );
    assert_eq!(ev.expect_field("author_dpi"), "96");
    assert_eq!(ev.expect_field("app_scale"), "1.0");
    assert_eq!(ev.expect_field("k"), "1.0");
    assert_eq!(
        events.len(),
        1,
        "他分岐のログを巻き添えで出さない: {events:?}"
    );

    // app_scale 非 ONE では k フィールドもそれに追随する（定数 1.0 直書きでない証拠）。
    let app2 = ScalePolicy::new(96, ScaleRatio::new(2, 1).expect("2/1"));
    let (k2, events2) = capture(|| derive_scale(app2, None));
    assert_eq!(k2.as_f32(), 2.0);
    let ev2 = expect_one(&events2, "窓 DPI を取得できない");
    assert_eq!(ev2.expect_field("k"), "2.0");
    assert_eq!(ev2.expect_field("app_scale"), "2.0");
}

/// 要件 1.4／設計「Error Handling」: 窓 DPI 不正（0）縮退は **`error!`** を発火し、
/// DPI 不在とは**別メッセージ**（別分岐であることがログから判別できる）。
#[test]
fn derive_scale_zero_window_dpi_emits_error_log() {
    let p = policy_96();
    let (k, events) = capture(|| derive_scale(p, Some((0, 0))));
    assert_eq!(k, ScaleRatio::ONE);

    let ev = expect_one(&events, "窓 DPI が不正");
    assert_eq!(
        ev.level,
        tracing::Level::ERROR,
        "窓 DPI 不正は error 格: {ev:?}"
    );
    assert_eq!(ev.expect_field("dpi_x"), "0");
    assert_eq!(ev.expect_field("dpi_y"), "0");
    assert_eq!(ev.expect_field("author_dpi"), "96");
    assert_eq!(ev.expect_field("k"), "1.0");
    assert_eq!(
        count_msg(&events, "取得できない"),
        0,
        "DPI 不在の文言と混ざらない（分岐の識別子が別）: {events:?}"
    );
    // dpi_x == dpi_y == 0 ゆえ異軸警告は立たない（0,0 は同軸）。
    assert_eq!(events.len(), 1, "1 分岐 1 ログ: {events:?}");
}

/// 設計 D2: 異軸 DPI（`dpi_x != dpi_y`）は **`warn!`**＋採用軸の実値を残す。
///
/// 「無言で dpi_y を捨てた」痕跡が必ず残ることが D2 の観測条件であり、
/// `dpi_x`/`dpi_y` 両方がフィールドに載ることまで契約に含める。
#[test]
fn derive_scale_anisotropic_dpi_emits_warn_log() {
    let p = policy_96();
    let (k, events) = capture(|| derive_scale(p, Some((192, 96))));
    assert_eq!(k, ScaleRatio::new(2, 1).expect("2/1"));

    let ev = expect_one(&events, "異軸 DPI");
    assert_eq!(
        ev.level,
        tracing::Level::WARN,
        "異軸 DPI は warn 格（表示は成立するので error ではない）: {ev:?}"
    );
    assert_eq!(ev.expect_field("dpi_x"), "192");
    assert_eq!(ev.expect_field("dpi_y"), "96", "捨てた軸の値も残す");
    assert_eq!(events.len(), 1, "1 分岐 1 ログ: {events:?}");
}

/// 要件 1.1/1.4: `author_dpi == 0` の最終防衛（[`derive_scale`] 側）は **`warn!`**＋
/// 生の宣言値と正規化後の値を並べて残す。
///
/// [`ScalePolicy::new`] 迂回（構造体リテラル直書き）で入っても無言では通らない。
#[test]
fn derive_scale_zero_author_dpi_emits_warn_log() {
    let bare = ScalePolicy {
        author_dpi: 0,
        app_scale: ScaleRatio::ONE,
    };
    let (k, events) = capture(|| derive_scale(bare, Some((192, 192))));
    assert_eq!(k, ScaleRatio::new(2, 1).expect("192/96 = 2/1"));

    let ev = expect_one(&events, "derive_scale: author_dpi=0");
    assert_eq!(ev.level, tracing::Level::WARN, "{ev:?}");
    assert_eq!(
        ev.expect_field("author_dpi"),
        "0",
        "生の宣言値（正規化前）を載せる"
    );
    assert_eq!(ev.expect_field("normalized"), "96");
    assert_eq!(events.len(), 1, "1 分岐 1 ログ: {events:?}");
}

/// 要件 1.1/1.4: 正規の入口 [`ScalePolicy::new`] の 0 正規化も **`warn!`**（無言正規化の禁止）。
///
/// [`derive_scale`] 側の最終防衛とは**別メッセージ**（どちらの層で正規化されたか判別できる）。
#[test]
fn scale_policy_new_zero_author_dpi_emits_warn_log() {
    let (p, events) = capture(|| ScalePolicy::new(0, ScaleRatio::ONE));
    assert_eq!(p.author_dpi, DEFAULT_AUTHOR_DPI);

    let ev = expect_one(&events, "ScalePolicy: author_dpi=0");
    assert_eq!(ev.level, tracing::Level::WARN, "{ev:?}");
    assert_eq!(ev.expect_field("author_dpi"), "0");
    assert_eq!(ev.expect_field("normalized"), "96");
    assert_eq!(events.len(), 1, "1 分岐 1 ログ: {events:?}");

    // 非ゼロ構築は無言（「常に warn」変異の非空虚性）。
    let (p2, events2) = capture(|| ScalePolicy::new(120, ScaleRatio::ONE));
    assert_eq!(p2.author_dpi, 120);
    assert!(events2.is_empty(), "正常構築は無言: {events2:?}");
}

/// 正常経路は**完全に無言**（`debug!` すら出さない）。
///
/// 上の 5 本が主張する「レベル・フィールド」の非空虚性はここで担保される——
/// 「常にログを出す」実装なら、DPI 対照表の全水準でこのテストが落ちる。
#[test]
fn derive_scale_normal_path_is_silent() {
    let p = policy_96();
    for dpi in [96u16, 120, 144, 168, 192, 72] {
        let (_, events) = capture(|| derive_scale(p, Some((dpi, dpi))));
        assert!(events.is_empty(), "正常経路は無言（dpi={dpi}）: {events:?}");
    }
    // 非 ONE の app_scale でも同様（2 因子合成は縮退ではない）。
    let app = ScalePolicy::new(144, ScaleRatio::new(3, 2).expect("3/2"));
    let (_, events) = capture(|| derive_scale(app, Some((168, 168))));
    assert!(events.is_empty(), "2 因子合成も無言: {events:?}");
}

/// 縮退フラグは独立に立ち、**各々が自分のログを出す**（1 本にまとめて握り潰さない）。
#[test]
fn derive_scale_emits_each_degradation_log_independently() {
    // author_dpi=0 かつ DPI 不在 → warn（正規化）＋ error（DPI 不在）の 2 本。
    let bare = ScalePolicy {
        author_dpi: 0,
        app_scale: ScaleRatio::ONE,
    };
    let (k, events) = capture(|| derive_scale(bare, None));
    assert_eq!(k, ScaleRatio::ONE);
    assert_eq!(count_msg(&events, "derive_scale: author_dpi=0"), 1);
    assert_eq!(count_msg(&events, "窓 DPI を取得できない"), 1);
    assert_eq!(
        count_msg(&events, "異軸 DPI"),
        0,
        "DPI 不在時に存在しない dpi_x/dpi_y を騙らない: {events:?}"
    );
    assert_eq!(events.len(), 2, "{events:?}");

    // dpi_x=0 かつ dpi_y=96 → error（窓 DPI 不正）＋ warn（異軸）の 2 本。
    let (k, events) = capture(|| derive_scale(policy_96(), Some((0, 96))));
    assert_eq!(k, ScaleRatio::ONE);
    assert_eq!(count_msg(&events, "窓 DPI が不正"), 1);
    assert_eq!(count_msg(&events, "異軸 DPI"), 1);
    assert_eq!(events.len(), 2, "{events:?}");
}

/// Invariants（設計 D2 の実装時是正）: 縮退ログは**毎回**出る。
///
/// 「初回のみ警告」の抑止状態を持てば純関数性が壊れる、というのが 1.4 レビューの裁定であり、
/// その裁定は「同じ入力を反復したときログ件数が呼出回数に比例する」ことでしか観測できない
/// （[`derive_scale_is_deterministic`] は戻り値の反復不変までしか見ていない）。
#[test]
fn derive_scale_repeats_degradation_log_on_every_call() {
    let p = policy_96();
    let (_, events) = capture(|| {
        for _ in 0..3 {
            derive_scale(p, Some((192, 96)));
        }
    });
    assert_eq!(
        count_msg(&events, "異軸 DPI"),
        3,
        "抑止状態（once / 初回のみ）を持たない: {events:?}"
    );

    let (_, events) = capture(|| {
        for _ in 0..3 {
            derive_scale(p, None);
        }
    });
    assert_eq!(count_msg(&events, "窓 DPI を取得できない"), 3, "{events:?}");
}
