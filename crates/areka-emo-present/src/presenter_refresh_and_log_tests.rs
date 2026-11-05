use super::*;

use areka_emo_compose::BindSet;
use wintf::ecs::{GraphicsCommandList, GraphicsCore};

use super::test_support::{
    CapturedEvent, INFO_LINE_MESSAGE, applies, arrangement_of, attach_hit_target,
    build_target_assets, build_two_face_assets, capture, force_current_surface, hide_ok,
    make_world_with_gpu, mount_entities, set_window_dpi, show_ok, spawn_window_with_dpi,
    tamper_face_1000_slot,
};

// ── applied_scale／refresh_scale（タスク 3.5・design Flow 2）───────────────────────────────

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

    presenter.attach_target(TargetId(0), window, emo_world, atlas, 96);
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

/// タスク 3.5 の名指し受け入れ基準・要件 4.1/4.2／5.3 観測完了: k=1/1 で表示を確立したのち窓 `DPI` を
/// 192 へ差し替えて `refresh_scale` を呼ぶと——(a) 戻り値が `scaled_extent(2/1, native)`、
/// (b) `applied_scale` が 2.0、(c) **引き当てが成立する**（`cache_hit=true`＝再合成しない）、
/// (d) 配置は原寸のまま `scale` だけが 2.0 へ動き、(e) メモの原寸バイトは k に依らず同一。
///
/// (c) が本仕様で加わった観測点である（付録 A′ #13）。エントリは原寸の面・原寸マスク・原寸の
/// 表示記録だけを持ち **k はキーに参加しない**ので、k だけが変わった再適用は同じエントリに当たる
/// ——「k 変化＝ミス＋再サンプル」だった旧正典と正反対であり、ヒットを主張しなければ静かに
/// 再合成へ退化しても緑のままになる。
///
/// さらに (f) `refresh_scale` が返した要求は**消費済み**であり、続く `take_pending_resize` は
/// `None` を返す——タスク 4.2 が `run_dpi_phase`（`refresh_scale`）と drain フェーズ
/// （`take_pending_resize`）の**両方**を呼ぶため、同一の reconcile が二度出ないことが結線契約である。
#[test]
fn refresh_scale_after_dpi_change_reapplies_new_k() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 96);
    // golden は `build_target_assets` が同一入力を直接合成して返す原寸バイト（k を含まない）。
    let (emo_world, atlas, native_golden) = build_target_assets(6, 5, 0x91);
    let k2 = ScaleRatio::new(2, 1).unwrap();
    let native_size = (6, 5);
    let scaled_size = k2.scaled_extent(native_size.0, native_size.1);
    assert_eq!(scaled_size, (12, 10), "前提: 原寸と物理寸が弁別可能");

    let mut presenter = EmoPresenter::new();
    presenter.attach_target(TargetId(0), window, emo_world, atlas, 96);

    // k=1/1 の表示確立（初回表示が積む k₀ 補正要求は取り出して捨てる）。
    show_ok(&mut presenter, &mut world, TargetId(0), 1000);
    assert_eq!(presenter.applied_scale(TargetId(0)), Some(1.0));
    assert_eq!(
        presenter.read_back(TargetId(0)).expect("read_back 失敗"),
        native_golden,
        "前提: 表示に渡すメモは原寸の合成バイト（R2.4/R3.2/R8.2 の golden 一致）"
    );
    let (surface_entity, _slot) = mount_entities(&presenter, TargetId(0));
    assert_eq!(
        arrangement_of(&world, surface_entity),
        (native_size, (1.0, 1.0)),
        "前提: 確立時の配置は原寸・係数 1.0"
    );
    // 表示記録は k を含まない＝k 変化で作り直されないことを後段で突き合わせる。
    let display_before = world
        .get::<GraphicsCommandList>(surface_entity)
        .cloned()
        .expect("表示確立後は surface entity に表示記録がある");
    assert_eq!(
        presenter.take_pending_resize(TargetId(0)),
        Some(native_size),
        "前提: 初回表示の要求を取り出しておく"
    );

    // モニタ跨ぎ移動・表示スケール変更の決定論的代替（WM_DPICHANGED 相当）。
    set_window_dpi(&mut world, window, 192);

    // (a) 戻り値＝新物理寸。同時に表示成立点のログを捕まえて (c) を読む。
    let (got, events) = capture(|| presenter.refresh_scale(&mut world, TargetId(0)));
    assert_eq!(
        got,
        Some(scaled_size),
        "DPI 変化後の refresh_scale が新物理寸を返さない（再導出・再表示が走っていない）"
    );
    // (b) 照会値が新 k へ追随。
    assert_eq!(
        presenter.applied_scale(TargetId(0)),
        Some(2.0),
        "refresh_scale 後も照会値が旧 k のまま（要件 4.2 の一貫更新が成立していない）"
    );
    // (c) 引き当てが成立している（要件 5.3・k はキーに参加しない）。
    let ev = events
        .iter()
        .find(|e| e.message() == INFO_LINE_MESSAGE)
        .unwrap_or_else(|| panic!("再表示の成立点ログが出ていない: {events:?}"));
    assert_eq!(
        ev.field("cache_hit"),
        Some("true"),
        "k だけが変わった再適用が外れている（k がキーに混ざっている＝要件 5.3 違反）"
    );
    // (d) 配置は原寸のまま係数だけが動く（拡大は wintf の変換行列が掛ける）。
    assert_eq!(
        arrangement_of(&world, surface_entity),
        (native_size, (2.0, 2.0)),
        "配置が新 k を映していない（原寸のままか、寸へ k を焼き込んでいる）"
    );
    assert_eq!(
        presenter.target_physical_size(TargetId(0)),
        Some(scaled_size),
        "照会の物理寸が丸め権威の式と一致しない"
    );
    // (e) メモの原寸バイトは k に依らず同一。ヒットゆえ表示記録も作り直されていない。
    assert_eq!(
        presenter.read_back(TargetId(0)).expect("read_back 失敗"),
        native_golden,
        "k 変化でメモの原寸バイトが書き換わった（k を含む面を作っている）"
    );
    assert_eq!(
        world.get::<GraphicsCommandList>(surface_entity),
        Some(&display_before),
        "ヒットしたのに表示記録が作り直された（記録が k に依存している）"
    );

    // (f) 要求は refresh_scale が消費済み＝drain フェーズと二重に resize しない（タスク 4.2 の結線契約）。
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
/// 観測で固定する——(1) メモのスロットを同一キーのまま**別の絵・空の表示記録**で改竄しておき、
/// surface entity に載っている表示記録が**改竄後のもの（空）にならない**こと（再表示していれば
/// ヒットして改竄したエントリが entity へ書かれる）、(2) 表示成立点のログが**1 件も出ていない**こと。
///
/// 改竄の効き目を読む口が `read_back` から entity の表示記録へ移ったのは、`read_back` が
/// **メモそのもの**を返す形になったためである——改竄したメモを読み返しても「メモを改竄できた」
/// ことしか分からない（恒真）。画面へ渡っている当のものは surface entity の
/// `GraphicsCommandList` であり、そこへ書くのは表示成立点の `set_display` だけである。
///
/// さらに (3) 未消費の窓寸 reconcile 要求を**握り潰さない**ことを確認する——ゲート不成立時に
/// `pending_resize` を触る実装は、drain フェーズが拾うはずだった初回表示の要求を消してしまう。
#[test]
fn refresh_scale_without_dpi_change_does_nothing() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 192);
    let (emo_world, atlas, golden_1000, golden_3000) = build_two_face_assets(6, 5);
    let k2 = ScaleRatio::new(2, 1).unwrap();
    // 窓 192／author 96 ゆえ k=2/1。メモは原寸で持つので golden は k に依らず同一である。
    let scaled_size = k2.scaled_extent(6, 5);

    let mut presenter = EmoPresenter::new();
    presenter.attach_target(TargetId(0), window, emo_world, atlas, 96);
    show_ok(&mut presenter, &mut world, TargetId(0), 1000);
    assert_eq!(
        presenter.read_back(TargetId(0)).expect("read_back 失敗"),
        golden_1000
    );
    // 初回表示の要求は**あえて取り出さない**（(3) の握り潰し検査のため）。

    // 改竄前に、画面へ渡っている表示記録を控える（改竄で入れる空リストと弁別可能であること）。
    let (surface_entity, _slot) = mount_entities(&presenter, TargetId(0));
    let display_before = world
        .get::<GraphicsCommandList>(surface_entity)
        .cloned()
        .expect("表示確立後は surface entity に表示記録がある");
    assert_ne!(
        display_before,
        GraphicsCommandList::empty(),
        "前提: 確立した表示記録は空でない（改竄値と弁別できる）"
    );

    // 改竄プローブ: 同一キーのスロットを別の絵（面 3000 の原寸合成）・空の表示記録で上書きする。
    tamper_face_1000_slot(
        &mut presenter,
        TargetId(0),
        (6, 5),
        GraphicsCommandList::empty(),
    );

    // DPI は据え置き（k 不変）。
    let (got, events) = capture(|| presenter.refresh_scale(&mut world, TargetId(0)));

    assert_eq!(got, None, "k 不変なのに新物理寸を返している");
    // (1) 改竄したエントリが画面へ載っていない＝再表示していない。
    assert_eq!(
        world.get::<GraphicsCommandList>(surface_entity),
        Some(&display_before),
        "k 不変なのに再表示している（改竄した空の表示記録が画面へ載った＝無駄な表示更新）"
    );
    // 改竄そのものは効いている（プローブの空振りで (1) が緑になっていない）。
    assert_eq!(
        presenter.read_back(TargetId(0)).expect("read_back 失敗"),
        golden_3000,
        "プローブがメモへ届いていない（(1) の不変が恒真になる）"
    );
    // (2) 表示成立点のログが 1 件も出ていない。
    assert!(
        applies(&events).0 == 0,
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

    presenter.attach_target(TargetId(0), window, emo_world, atlas, 96);
    // 一度も show していない状態で DPI を変える。
    set_window_dpi(&mut world, window, 192);

    let (got, events) = capture(|| presenter.refresh_scale(&mut world, TargetId(0)));

    assert_eq!(got, None, "再表示入力が無いのに新物理寸を返している");
    assert!(
        applies(&events).0 == 0,
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
        "表示中のメモが在る（表示していないのに資源を作った）"
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
    presenter.attach_target(TargetId(0), window, emo_world, atlas, 96);
    show_ok(&mut presenter, &mut world, TargetId(0), 1000);
    let _ = presenter.take_pending_resize(TargetId(0));

    // `\s[-1]` 相当で非表示にする（キャッシュ・装着・last_show は保持される）。
    hide_ok(&mut presenter, &mut world, TargetId(0));
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
    let (got, events) = capture(|| presenter.refresh_scale(&mut world, TargetId(0)));

    assert_eq!(got, None, "非表示 target が新物理寸を報告している");
    assert!(
        applies(&events).0 == 0,
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
    presenter.attach_target(TargetId(0), window, emo_world, atlas, 192);
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

    let (got, events) = capture(|| presenter.refresh_scale(&mut world, TargetId(0)));

    assert_eq!(
        got, None,
        "DPI 不在を 96 で捏造している（k=1/2 と誤導出して再表示が走った）"
    );
    assert!(
        applies(&events).0 == 0,
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
/// `EmoWorld` から面が消えた場合に実在する状況であり、GPU 資源を 2 つ目に作らない。合成の失敗は
/// k の値に依らず同じ位置で起きるので、表示確立**後**の失敗はこの注入で作れる（記録の失敗を
/// 注入した場合の前状態維持は `presenter_display_failure_tests.rs` が 4 点 × 3 場面で見る）。
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
    presenter.attach_target(TargetId(0), window, emo_world, atlas, 96);
    show_ok(&mut presenter, &mut world, TargetId(0), 1000);
    assert_eq!(
        presenter.take_pending_resize(TargetId(0)),
        Some((4, 3)),
        "前提: 初回表示の要求を取り出しておく"
    );

    // 前表示の証跡を注入前に控える（`read_back` は使えない——下の註釈）。
    let (surface_entity, _slot) = mount_entities(&presenter, TargetId(0));
    let display_before = world
        .get::<GraphicsCommandList>(surface_entity)
        .cloned()
        .expect("表示確立後は surface entity に表示記録がある");

    // 失敗注入: 再表示入力の surface id を解決不能値へ差し替える。
    //
    // これ以降 `read_back` は使えない——読み戻しはメモを `last_show` のキーで引く形になったため、
    // 注入そのものが「表示中エントリが引けない」状態を作ってしまう（前表示が失われたのではなく、
    // 注入した偽のキーで引いているだけである）。前表示の維持は ⑴ 画面へ渡っている表示記録
    // （`GraphicsCommandList`）と ⑵ メモに残る**本物のキー**のエントリのバイト、の 2 つで読む。
    presenter
        .targets
        .get_mut(&TargetId(0))
        .unwrap()
        .last_show
        .as_mut()
        .expect("前提: 表示成立済みゆえ last_show は Some")
        .0 = 9999;

    set_window_dpi(&mut world, window, 192);
    let (got, events) = capture(|| presenter.refresh_scale(&mut world, TargetId(0)));

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
        world.get::<GraphicsCommandList>(surface_entity),
        Some(&display_before),
        "失敗したのに画面へ渡している表示記録が差し替わった（前表示維持の違反）"
    );
    assert_eq!(
        presenter
            .targets
            .get(&TargetId(0))
            .unwrap()
            .cache
            .get(1000, &BindSet::default(), &PatternState::default())
            .expect("失敗でメモの表示中エントリが消えた（前表示維持の違反）")
            .composed
            .bytes(),
        native_golden.as_slice(),
        "失敗したのにメモの原寸バイトが変わった（前表示維持の違反）"
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
    let err = events
        .iter()
        .find(|e| {
            e.field("message")
                .is_some_and(|m| m.contains("refresh_scale: 再表示が成立せず"))
        })
        .unwrap_or_else(|| panic!("refresh_scale の失敗が無言（専用ログが無い）: {events:?}"));
    assert_eq!(
        err.level,
        tracing::Level::ERROR,
        "再表示失敗が error! でない（要件 4.4 のログ規律）"
    );
    assert!(
        applies(&events).0 == 0,
        "失敗したのに表示成立点のログが出ている: {events:?}"
    );
}

/// 捕捉イベント列のうち **R1.6 の防御 warn** の件数（level と固有文言の双方で選別する）。
fn applied_absent_warn_count(events: &[CapturedEvent]) -> usize {
    events.iter().filter(|e| is_applied_absent_warn(e)).count()
}

/// R1.6 の防御 warn か。
fn is_applied_absent_warn(e: &CapturedEvent) -> bool {
    e.level == tracing::Level::WARN
        && e.field("message")
            .is_some_and(|m| m.contains("適用スケール未確定"))
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
/// 捕捉窓・同一 callsite** で観測するため、陰性主張は「捕捉が死んでいない」
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

    let ((defensive_hit, defensive_miss, unshown, unregistered), events) = capture(|| {
        // (1) 防御分岐: 面あり・applied なし → 鳴る。
        let a = presenter.hit_region_client(TargetId(0), 180, 96);
        let b = presenter.hit_region_client(TargetId(0), 360, 192);
        // (2) 正常縮退: 同一 callsite に対して鳴らない側を同一スコープで観測する。
        let c = presenter.hit_region_client(TargetId(1), 180, 96);
        let d = presenter.hit_region_client(TargetId(9), 180, 96);
        (a, b, c, d)
    });

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
            .find(|e| is_applied_absent_warn(e) && e.field("client_x") == Some(want_x))
            .unwrap_or_else(|| {
                panic!("client_x={want_x} の防御 warn が捕捉されていない: {events:?}")
            });
        assert_eq!(
            warn.field("target"),
            Some("TargetId(0)"),
            "warn が対象 target を載せていない（どの窓で不変条件が破れたか特定できない）: {:?}",
            warn.fields
        );
        assert_eq!(
            warn.field("client_y"),
            Some(want_y),
            "warn が client 座標を載せていない: {:?}",
            warn.fields
        );
    }

    // (d) 正常縮退は同一 callsite で **1 件も増やさない**（陰性）。4 呼出は 1 つの窓で捕捉して
    //     いるので、(c) の「ちょうど 2 件」が防御呼出 2 回分と一致することがそのまま陰性の証拠に
    //     なる（述語を applied.is_none() 単独へ潰すと 3 件以上で (c) が鳴る）。
    assert_eq!(
        unshown.region, None,
        "未表示 scope は正常縮退（region なし）"
    );
    assert_eq!(
        unregistered.region, None,
        "未登録 target は正常縮退（region なし）"
    );
}

/// 装置の作り直し（audit-wintf-graphics-world 6 番・要件 3.1）: `GraphicsCore` を入れ替えた後の
/// `refresh_after_device_change` は控えを捨てて出し直し、面の命令の列が新しい物になる。印が
/// 立たない回は何もしない（列はそのまま）。
#[test]
fn device_recreation_rerecords_display_list() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 96);
    let (emo_world, atlas, _golden) = build_target_assets(4, 3, 0x5a);
    let mut presenter = EmoPresenter::new();
    presenter.attach_target(TargetId(0), window, emo_world, atlas, 96);
    show_ok(&mut presenter, &mut world, TargetId(0), 1000);
    let (surface_entity, _slot) = mount_entities(&presenter, TargetId(0));
    let list_of = |world: &World| {
        world
            .get::<GraphicsCommandList>(surface_entity)
            .cloned()
            .expect("表示確立後は surface entity に表示記録がある")
    };
    let before = list_of(&world);

    // 初回は印を控えるだけ・2 回目は印が立っていないので何もしない。
    presenter.refresh_after_device_change(&mut world);
    presenter.refresh_after_device_change(&mut world);
    assert_eq!(
        list_of(&world),
        before,
        "装置が変わっていないのに出し直した"
    );

    world.insert_resource(GraphicsCore::new().expect("GraphicsCore::new 失敗"));
    presenter.refresh_after_device_change(&mut world);
    assert_ne!(
        list_of(&world),
        before,
        "装置を作り直した後も古い装置の命令の列のまま（控えを捨てて出し直していない）"
    );
    assert_eq!(presenter.target_visible(TargetId(0)), Some(true));
}
