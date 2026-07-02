# クリック透過機構（`WS_EX_TRANSPARENT` 動的トグル）

本書は `wintf` のクリック透過機構——GPU 合成描画（WUC/DComp）を維持したまま、キャラクター描画領域以外の透明領域上のクリックを背面プロセスへ透過させる仕組み——の概要・仕組みの流れ・不採用手段の理由・API 使用例・既知の制約を記す。実装は `crates/wintf/src/ecs/clickthrough/`（`mod.rs`/`registry.rs`/`monitor.rs`/`controller.rs`）・`crates/wintf/src/win_style.rs`（`apply_click_through`）・`crates/wintf/src/runtime/mod.rs`（`wire_click_through`）・`crates/areka/src/main.rs`（DComp オプトイン＋窓登録）にある。

## 1. 概要: 二層分離（表示層／当たり判定層）

本機構の中核は、ウィンドウの「表示」と「当たり判定」を **独立した 2 層** として扱う設計である。

- **表示層（display layer）**: GPU 合成による見た目。WUC（Windows.UI.Composition）/ DComp（DirectComposition）の visual/content で構成する。キャラクターが 2D サーフェスか合成スワップチェーン（3D／Live2D 相当）かに依らず、描画内容はこの層が担う。
- **当たり判定層（hit-testing layer）**: そのウィンドウがクリックを受け取るか、背面プロセスへ通すか。HWND の `WS_EX_TRANSPARENT` 拡張スタイルの有無で決まる。

**本機構は当たり判定層のみを制御し、表示層には一切触れない。** ex-style の操作（`WS_EX_LAYERED` 同伴フラグの初回付与＋`WS_EX_TRANSPARENT` ビットの動的トグル）だけを行い、合成 visual/content の生成・更新・破棄には関与しない。これにより「別プロセス透過のために GPU 描画を諦める踏み絵」を回避する——見た目は GPU 合成のまま、当たり判定だけをカーソル位置に応じて動的に切り替える。

当たり判定の情報源は既存のシーングラフ・ヒットテスト `hit_test_in_window(&World, window, client_point) -> Option<Entity>` である。`Some(entity)` は「いずれかのエンティティにヒット（不透過・自窓で受領）」、`None` は「どのエンティティにもヒットせず（透過・背面へ通過）」を意味する。各エンティティの `HitTest` モード（`Bounds` 合成α／`AlphaMask` ピクセル単位／`NamedRegions`）はシーングラフ評価が honored するため、「実描画α」はこのツリー評価が体現する。GPU フレームバッファの CPU readback は要求しない。

## 2. 仕組みの流れ

```
[ワーカスレッド]                       [UI スレッド]
CursorMonitorBridge                    ClickThroughController (async loop)
  GetCursorPos (screen physical)
  移動を検知したら:
    latest_pos.store(pack(x,y))  ─┐
    cursor_event.notify(MAX)     ─┘（store→notify の順序を厳守）
                                       ↓ wake（カーソル移動 notify）
                                       ↑ wake（VSync tick・post-tick relay）
                                     listen-before-work:
                                       listener = wake_event.listen()   ← 待機の前に arm
                                       world.upgrade()（None=shutdown で終了）
                                       listener.await
                                       evaluate_targets(settled &World, ...)
                                         drag = snapshot_drag_state()（1 パス 1 回読み・ループ前）
                                         各対象窓について:
                                           未適用なら apply_layered_companion(hwnd)（初回 1 回・冪等）
                                           client = cursor_screen - WindowPos.position
                                           hit = hit_test_in_window(world, window, client)
                                           desired = resolve_transition(hit, &drag, last_applied)
                                           desired が None（差分なし/ドラッグ抑止）→ skip
                                           変化時のみ apply_click_through(hwnd, transparent) を 1 回
                                           Ok の時だけ registry.set_last_applied(window, desired)
```

判定フローの要点:

- **`WS_EX_LAYERED` 同伴フラグ（透過成立の必須条件）**: `WS_EX_TRANSPARENT` は**単独では別プロセスへのマウス透過を成立させない**（pilot 実証: DComp 窓では窓が全クリックを吸う）。機構は登録窓の初回評価時に `apply_layered_companion` で `WS_EX_LAYERED` を 1 回立てる（冪等・落とさない）。LAYERED はフラグのみで、レイヤード描画（`UpdateLayeredWindow`/`SetLayeredWindowAttributes`）は呼ばない。DComp 描画（`WS_EX_NOREDIRECTIONBITMAP`）と共存する（pilot 実測 ex_style `0x280028`）。
- **`WS_EX_TRANSPARENT` の動的トグル**: 透過 ON=ビット付与、OFF=ビット除去。`apply_click_through` が `SetWindowLongPtr(GWL_EXSTYLE)` ＋ `SetWindowPos(SWP_FRAMECHANGED)` で反映する。TRANSPARENT ビット以外の ex-style は保存する。
- **別スレッドのカーソル監視（`CursorMonitorBridge`）**: 専用ワーカスレッドが `GetCursorPos`（screen physical）を固定短周期（12ms）でポーリングし、UI スレッドの描画を阻害しない。ワーカは `&World`／ECS に一切触れない（座標取得のみ）。
- **順序不変条件（store→notify）**: ワーカは移動検知時に `latest_pos.store(...)` を **先に**、その後で `cursor_event.notify(usize::MAX)` を行う。逆順だと UI 側が 1 通知分古い座標を読む稀レースが生じる（`VsyncEventBridge` と同一規律）。
- **`event_listener` 起床（tokio 非使用）**: スレッド跨ぎ通知は既存の `event_listener` 起床パターンに倣う。tokio 等の外部非同期ランタイムは持ち込まない。UI ループは `wintf_winmsg_executor::spawn_local` で UI スレッドに投入される。
- **listen-before-work 規律**: UI ループは待機の **前** に `listener = wake_event.listen()` を arm する。処理中に届く notify を落とさない（取りこぼし防止）。
- **既存シーングラフ・ヒットテスト連動**: 判定は `hit_test_in_window(&World, window, client_point) -> Option<Entity>` に委ねる。`Some`→不透過、`None`→透過。座標変換（DPI／マルチモニタ／ウィンドウ移動）は既存 `hit_test_in_window` の変換チェーンへ委譲する（`client = cursor_screen - WindowPos.position` を i32 で計算し、呼び出し時に `PointF::new(x as f32, y as f32)` へキャスト）。
- **差分ガード（`last_applied`）**: `resolve_transition` が「今回望ましい状態」と `last_applied` を比較し、同一なら `None`（再適用しない）、異なる時のみ `Some(desired)`（ちょうど 1 回適用）を返す。真実源は `ClickThroughRegistry` で、書き戻しは `apply_click_through` が `Ok` の時のみ（`Err` は据え置き＋`warn!`・次サイクル再試行）。
- **二重起床（カーソル移動 notify ＋ VSync tick）**: UI ループはワーカのカーソル移動 notify と、VSync tick（vblank 毎）の relay の **いずれでも** 起床する（単一の共有 `Arc<Event>` を両者が notify・select 併用なし）。カーソルが静止していても表示シーングラフは更新され得る（SERIKO アニメ・サーフェス差し替えでαが変化）ため、tick 相乗りで毎フレーム再評価して起床契機への依存を断つ。差分ガードにより実際の `SetWindowPos` は変化時のみ発火するので、毎フレーム評価でも適用コストは増えない。
- **post-tick 評価**: ヒットテストは当該フレームの ECS 更新スケジュール完了後（`GlobalArrangement`／`AlphaMask`／`DragState` が確定した settled World）に実行する。tick 途中（レイアウト未確定・αマスク生成前）の中間状態は読まない。VSync tick は relay タスクが「vblank を待って `wake_event.notify`」する形で中継し、評価は常に settled な World を読む。

## 3. 不採用手段とその理由

本機構は `WS_EX_TRANSPARENT` 動的トグルを採る。以下の 3 手段は **採用しない**。

### (a) ULW（Layered Window / `UpdateLayeredWindow`）

ULW は CPU ビットマップ方式であり、`UpdateLayeredWindow` に渡す CPU 側ビットマップでウィンドウ内容とαを供給する。この方式は **GPU 合成（WUC/DComp）と併用できない**——GPU で合成した visual/content をそのまま表示に使えず、「別プロセス透過を得るために GPU 描画（3D 描画）を諦める踏み絵」になる。本機能の至上要件は GPU 合成描画を捨てないことなので、ULW は不採用。（既存 ULW バックエンドは検証期間中は並走残置し、撤去は別坑 `wintf-ulw-removal`。§6 の申し送り参照。）

### (b) `HTTRANSPARENT`（`WM_NCHITTEST` ハンドラ）

`WM_NCHITTEST` に応答して `HTTRANSPARENT` を返すと、そのウィンドウのヒットは **同一プロセス内の背後のウィンドウ** へ委譲される。しかし `HTTRANSPARENT` は **プロセス境界を越えられない**——別プロセスのウィンドウへクリックを透過させることはできない。本機能の中核要件（背面の別プロセスへの透過）を満たせないため不採用。したがって `WM_NCHITTEST`→`HTTRANSPARENT` ハンドラは別プロセス透過の手段として追加しない。

### (c) Layered 描画（`UpdateLayeredWindow`／`SetLayeredWindowAttributes`）

`WS_EX_LAYERED` を用いた **描画**（`UpdateLayeredWindow`／`SetLayeredWindowAttributes` による内容・α供給）は GPU 合成と両立しない（(a) と同根の CPU ビットマップ問題）。本機構は `WS_EX_LAYERED` を描画用途に使わない。DComp/WUC 経路では生成時に factory（`compute_ex_style`）が `WS_EX_LAYERED` を除去し `WS_EX_NOREDIRECTIONBITMAP` を付与するため、機構が登録窓の初回評価時に `apply_layered_companion` で立て直す。`WS_EX_LAYERED` は当たり判定を効かせる **同伴フラグ用途のみ**（ULW/SLWA 非呼出・pilot 実証の必須条件——これが無いと `WS_EX_TRANSPARENT` 単独ではマウス透過が効かず窓が全クリックを吸う）に許容し、本トグル API は `WS_EX_TRANSPARENT` ビットのみを触る。追加の ex-style 付与や `WM_NCHITTEST` ハンドラが必要と判断された場合は、独断追加せず理由を添えて依頼者へ確認する（LAYERED 同伴は 2026-07-02 実動検証を受け依頼者確認済み）。

## 4. API 使用例

### 4.1 ex-style トグル（`win_style::apply_click_through`／`apply_layered_companion`）

`apply_click_through` は対象 HWND の `WS_EX_TRANSPARENT` を `transparent` フラグに一致させ、`SetWindowPos(SWP_FRAMECHANGED)` で反映する。他の ex-style ビットには触れない。`apply_layered_companion` は透過成立の必須条件である `WS_EX_LAYERED` を同伴フラグとして立てる（冪等・落とさない・レイヤード描画非呼出）。

```rust
use wintf::win_style::{apply_click_through, apply_layered_companion};

// 同伴フラグ（必須条件・通常は機構が登録窓の初回評価で自動適用）。
apply_layered_companion(hwnd)?;
// 透過 ON（透明領域上・背面へ通過させたい）。
apply_click_through(hwnd, true)?;
// 透過 OFF（キャラ領域上・自窓で受領したい）。
apply_click_through(hwnd, false)?;
```

シグネチャ:

```rust
pub fn apply_click_through(hwnd: HWND, transparent: bool) -> windows::core::Result<()>;
pub fn apply_layered_companion(hwnd: HWND) -> windows::core::Result<()>;
```

通常は機構（`evaluate_targets`）が同伴フラグ適用・差分ガードを通したトグルを自動で行うため、アプリ側が直接叩く必要はない。

### 4.2 機構の起動（`ClickThroughController::start`）

`WinApp::run` の結線点（`wire_click_through`）が呼ぶ。ワーカ生成・event 共有・async ループの `spawn_local` を束ね、RAII ハンドル `ClickThroughHandle` を返す。

```rust
// runtime/mod.rs（wire_click_through 相当）
let registry = Rc::new(RefCell::new(ClickThroughRegistry::new()));
let wake_event = Arc::new(event_listener::Event::new());

let handle = ClickThroughController::start(
    Rc::downgrade(&self.world), // Weak<RefCell<EcsWorld>>: shutdown で upgrade None → ループ終了
    Rc::clone(&registry),      // Rc<RefCell<ClickThroughRegistry>>: start 後も register/remove 可能
    Arc::clone(&wake_event),   // Arc<event_listener::Event>: 二重起床の単一 wake event
);

// 登録面を World へ NonSend リソースとして挿入（areka が取得する）。
world.insert_non_send_resource(ClickThroughRegistryHandle::new(Rc::clone(&registry)));
```

シグネチャ:

```rust
pub(crate) fn start(
    world: Weak<RefCell<EcsWorld>>,
    registry: Rc<RefCell<ClickThroughRegistry>>,
    wake_event: Arc<event_listener::Event>,
) -> ClickThroughHandle;
```

二重起床は **単一の共有 `Arc<Event>`** を cursor worker（`CursorMonitorBridge::spawn(wake_event.clone())`）と VSync tick 源の両方が notify する方式（select 併用なし）。tick 源が post-tick で notify する結線は `wire_click_through` の VSync relay タスクが担う。

### 4.3 対象窓の登録（`ClickThroughRegistryHandle::register`）

areka のようなアプリ側は、窓生成後に window Entity と HWND を登録する。登録面は `pub` NonSend リソース `ClickThroughRegistryHandle` として World にあり、eval ループと同一 `Rc<RefCell<ClickThroughRegistry>>` を共有するため登録は即反映される。

```rust
// crates/areka/src/main.rs（register_click_through_windows システム相当）
fn register_click_through_windows(
    new_windows: Query<(Entity, &WindowHandle),
        (Added<WindowHandle>, Or<(With<ShellWindowMarker>, With<BalloonWindowMarker>)>)>,
    handle: Option<NonSend<ClickThroughRegistryHandle>>,
) {
    let Some(handle) = handle else { return; };
    for (entity, wh) in new_windows.iter() {
        handle.register(entity, wh.hwnd()); // 同一 Entity 再登録は dedupe（冪等）
    }
}
```

窓破棄時の除去は機構内 `prune_dead_targets`（Entity 生存確認・`evaluate_targets` 冒頭で自動除去）が担うため、アプリ側の明示 `remove` は必須ではない（despawn で十分）。

### 4.4 areka の DComp（WUC）オプトイン

areka の shell/balloon 窓を GPU 合成経路へ切り替える。`ex_style` は factory の `compute_ex_style` が `composition_mode` から自動計算する（`WS_EX_LAYERED` を外し `WS_EX_NOREDIRECTIONBITMAP` を付与）ため `WindowStyle` は据え置きでよい。

```rust
Window {
    title: "areka shell".to_string(),
    composition_mode: CompositionMode::DComp, // WUC（DComp）合成経路へオプトイン
    ..Default::default()
}
```

WUC 化により ULW の自動αヒットテストが失われるため、機構がαを評価できるよう shell/balloon の 2 窓を明示登録する（§4.3）。

## 5. 既知の制約

- **`SWP_FRAMECHANGED` の副作用**: `apply_click_through` は ex-style 反映に `SetWindowPos(SWP_FRAMECHANGED)` を要する。z オーダー・アクティベーション・位置・サイズへの副作用を避けるため、`SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE` を併用して FRAMECHANGED のみを要求する。pilot（先進坑）で WUC 合成・z オーダー・フォーカスとの共存を実測済みだが、本坑本体経路でも再確認する。
- **カーソル監視のポーリング周期**: ワーカは `GetCursorPos` を固定短周期（12ms・約 8–16ms 相当）でポーリングする。周期が短すぎると CPU 負荷が増えるため、前回座標との移動差分ガードで無駄な notify を抑える（移動が無ければ notify しない）。UI 側でも差分ガードするため二重に安全。ワーカは sleep で待つため `Drop` の `join()` は最大約 1 周期で復帰しハングしない。
- **ドラッグ中の透過抑止**: ウィンドウをドラッグ移動中（`DragState` が `Dragging` または直前 1 フレームの `JustStarted`）は、カーソルがキャラ領域から一時的に外れても透過 ON への切替を **抑止** する（強制 `Opaque`）。これによりドラッグ中の掴みが崩れない（R5 アンチフリッカ）。ドラッグ終了直後（`JustEnded`）のサイクルで抑止を解除し、現在のカーソル位置＋ヒットテストで再判定・再収束する。`Preparing`（押下のみ・閾値未到達）はまだドラッグ開始前なので非ドラッグ写像に委ねる。

## 6. 申し送り: ULW 撤去確定時に更新すべき対象

**本坑（`wintf-clickthrough-alpha-toggle`）では以下を実更新しない。** ULW ルートは本方式が完全に有効と判断されるまで検証期間として並走残置し、撤去は別坑 `wintf-ulw-removal` が担う。ULW 撤去が確定した時点で、以下の「ULW 一択」相当記述を更新する必要がある（R7.3／R10.3。ここでは更新対象を明示できる状態を保つのみ）:

- **`.kiro/steering/tech.md`**: 「透過の合成方式（旧『ULW 一択』結論を撤回・新方針確定・実装移行中）」の記述。決定済み方針（① 表示合成を WUC へ、② 別プロセス透過は `WS_EX_TRANSPARENT` 動的トグルへ、③ ULW ルートは除去）を、撤去完了に合わせて「移行中／並走」から「除去済み」へ更新する。
- **`.kiro/steering/roadmap.md`**: ULW 透過・`CompositionMode` enum・ULW アーム・`com/ulw.rs` を「非スコープ（残置）」としている記述。ULW 一式の除去（別 spec `wintf-ulw-removal`）の完了に合わせて更新する。
- **正本 `doc/COMPAT_ARCHITECTURE.md`**: 「描画/窓層 …… wintf（ULW透過・……）」および ULW 透過・クリック透過を土台完了として挙げる記述。ULW 撤去後の実体（`WS_EX_TRANSPARENT` 動的トグル一本化）へ更新する。これが設計判断確定時の更新対象の正本である。
