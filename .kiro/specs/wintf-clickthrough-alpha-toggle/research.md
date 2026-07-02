# ギャップ分析: wintf-clickthrough-alpha-toggle

> 本書は kiro-validate-gap による**分析と選択肢の提示**であり、最終決定ではない。要件ディスカッションと design フェーズの入力とする。
> 対象: 確定済み `requirements.md`（R1〜R10）／`brief.md`／steering（tech.md・structure.md・roadmap.md）／pilot 一次記録（`crates/pilot/examples/pilot-clickthrough-alpha-toggle/` README・REPORT）。
> 分析日: 2026-07-02

## 1. 分析サマリ（結論の要約）

- **本坑は「新規機構の追加」**が主体で、既存コードの破壊的改修はほぼ不要。核心（`WS_EX_TRANSPARENT` 動的トグル＋別スレッドのカーソル監視＋αマスク問い合わせ＋差分適用）は現状の wintf に**存在しない capability** であり、新規に構築する（`brief.md` の記載どおり葉ノード的追加）。
- **接続素材はほぼ全て揃っている**: `WinStyle` に `WS_EX_TRANSPARENT(bool)` / `WS_EX_LAYERED(bool)` ビルダー既存、`AlphaMask::is_hit`/`from_pbgra32` 既存、`event_listener` スレッド跨ぎ起床の実装済み前例（`VsyncEventBridge`）あり、ドラッグ状態機械（`DragState`）既存。**カーソル監視ワーカと動的 ex-style トグルだけが空白**。
- **最大の設計論点は「αマスクの取得源」**: pilot は固定円マスク（仮）。本坑 R2.3 は「本体の実描画 α バッファ」を要求。だが CPU-read 可能な合成 α（`WindowD3D11Compositor` の staging_bitmap）は **ULW 専用経路**で、ULW 撤去とともに消える。WUC/GPU 合成経路には CPU-read α バッファが今は無い。→ **per-widget の `AlphaMask`（WIC ロード時生成・静止画 α）を集約する経路**が現実的な第一候補。「実描画 α」の解釈確定が要件ディスカッションの最重要議題。
- **座標系が要注意**: `AlphaMask` は「画像ピクセル座標」、pilot は「物理座標の固定円」、既存 `hit_test_entity` は「スクリーン物理座標→bounds 正規化→マスク座標」。カーソル監視ワーカ（スクリーン物理座標 `GetCursorPos`）から αマスク判定までの**座標変換チェーン**（DPI／マルチモニタ／ウィンドウ移動）を破綻なく通すのが R8 の肝。
- **リスク/工数は中程度（M〜L）**。単体の機構は小さいが、スレッド跨ぎ・座標変換・ドラッグ抑止・WUC 描画非破壊・既存 ULW 並走の各接合点で回帰しやすい。pilot が go 済みで機構リスクは解消済みだが、「本体αマスク統合」と「本体 UI スレッド（`spawn_local`）への結線」は pilot に無い新規領域。

## 2. 現状調査（Current State：接続先の実測）

### 2.1 拡張スタイル系（R6・R1）
- `crates/wintf/src/win_style.rs`: `WinStyle` ビルダーに **`WS_EX_TRANSPARENT(flag: bool)`（L247）** と **`WS_EX_LAYERED(flag: bool)`（L287）** が既存。`commit(hwnd)`（L24）は `SetWindowLongPtr(GWL_STYLE/GWL_EXSTYLE)` で反映するが **`SetWindowPos(SWP_FRAMECHANGED)` を呼ばない**（pilot は FRAMECHANGED 必須と報告）。→ トグル用の適用関数は `commit` をそのまま流用できず、ex-style だけを差分適用し FRAMECHANGED を伴う新規パスが要る。
- `crates/wintf/src/runtime/window_factory.rs`: `compute_ex_style(composition_mode, style)`（L64）が**生成時のみ**の ex-style 算出（ULW→`WS_EX_LAYERED`／DComp→`WS_EX_NOREDIRECTIONBITMAP` かつ LAYERED 除去）。**動的トグル経路は無い**。`apply_initial_state`（L216）が `SetWindowLongPtrW(GWL_STYLE)`＋`SetWindowPos(SWP_FRAMECHANGED)` の生成後反映を実装済み（トグル実装の参考パターン）。
- `crates/wintf/src/ecs/window/components.rs`: `WindowStyle{ style, ex_style }`（L159）／`CompositionMode{ ULW(default), DComp }`（L100・**生成後不変**と明記）。`WindowStyle::from_hwnd`（L182）で現在値取得可。
- **`WS_EX_NOREDIRECTIONBITMAP` は `CompositionMode::DComp` のみ**。areka 本体（`crates/areka/src/main.rs` L181-189, L242-250）は現状 **`CompositionMode` 既定＝ULW** で、ex_style=`WS_EX_LAYERED|WS_EX_TOOLWINDOW|WS_EX_TOPMOST`、style=`WS_POPUP|WS_VISIBLE`（枠なし）。→ 本坑を実際に効かせるには areka を WUC(DComp) 経路へ移す必要があり、これは `wintf-dcomp-to-wuc-migration`／`wintf-ulw-removal` との**順序依存**（本坑単体では「機構を wintf に用意」までで、実効化は移行後）。

### 2.2 αマスク・ヒットテスト（R2）
- `crates/wintf/src/ecs/widget/bitmap_source/alpha_mask.rs`: `AlphaMask`（ビットパック 1bpp・閾値 128）。`from_pbgra32(pixels,w,h,stride)`（L33）／`is_hit(x,y)`（L67）＝**画像ピクセル座標**での判定。Send+Sync 自動導出（`Vec<u8>`＋`u32`）＝**ワーカースレッドへ安全に共有可能**。
- `crates/wintf/src/ecs/widget/bitmap_source/systems.rs`: `generate_alpha_mask_system`（L373）が `Added<BitmapSourceResource>`＋`HitTestMode::AlphaMask` 時に `WintfTaskPool` で非同期生成し `BitmapSourceResource.set_alpha_mask` で格納。**α は WIC デコード時の静止画 α**（GPU 合成後の実描画 α ではない）。
- `crates/wintf/src/ecs/layout/hit_test/mod.rs`: `HitTestMode{None,Bounds(default),AlphaMask,NamedRegions}`。`hit_test_entity`（L164）が **スクリーン物理座標→`global.bounds` 正規化→マスク座標→`is_hit`** の変換チェーンを既に実装（本坑のカーソル判定が再利用/参照すべき正典）。`hit_test`（L437）でツリー走査、`hit_test_in_window`（L464）でクライアント座標起点。
- **注意**: `AlphaMask` は個々の `BitmapSource` widget 単位。ウィンドウ内に複数 widget があれば「ウィンドウ全体の当たり判定」は各 widget の OR。areka では shell 画像が 1 子 entity（`main.rs` L212）なので当面は単純だが、汎用機構としては複数 widget 集約の設計が要る。

### 2.3 スレッド跨ぎ通知（R4）
- `crates/wintf/src/runtime/tick_bridge.rs`: **`VsyncEventBridge`（L43）が唯一の前例**。専用 `std::thread`（`thread::Builder::new().name(...).spawn`）＋`Arc<event_listener::Event>`＋`Arc<AtomicBool>` stop_flag＋`Drop`で `stop→join` の RAII。UI 側は `spawn_local` の async ループ（`run_async_tick` L212）で `event.listen().await`→処理→再 arm（取りこぼし防止規律）。→ **カーソル監視ワーカはこの構造を 1:1 で踏襲するのが最も自然**（tokio 不使用 R4.2 も自動的に満たす）。
- `crates/wintf/src/runtime/window_factory.rs`＋`WndState`／`make_wndproc`: UI スレッド async は `wintf_winmsg_executor::spawn_local`。ex-style 適用は UI スレッドで実行（R4.3）＝この spawn_local ループから `SetWindowLongPtr` を呼ぶ。
- **空白**: `GetCursorPos` の使用箇所はワークスペースに皆無（grep 0 件）。カーソル監視ワーカは完全新規。

### 2.4 ドラッグ（R5）
- `crates/wintf/src/ecs/drag/state/mod.rs`: `DragState`（Idle/Preparing/JustStarted/Dragging/JustEnded）＋`thread_local!` 管理＋`snapshot_drag_state()`（L215）で読み取り専用スナップショット取得可。`DragState::Dragging` に `hwnd`／`initial_window_pos` 保持。
- `crates/wintf/src/ecs/drag/context.rs`: `WindowDragContextResource`（`Arc<Mutex<WindowDragContext>>`）が **ECS↔wndproc スレッド間**でドラッグ情報を転送（Send+Sync）。→ カーソル監視ワーカ／トグル適用器が「ドラッグ中か」を知る導管として**既存の Arc 共有パターンを再利用**できる（R5 の透過抑止フラグ）。pilot の `dragging: Arc<AtomicBool>` に相当する共有状態を、この既存機構に接ぐか別 `Arc<AtomicBool>` を新設するかが設計論点。

### 2.5 合成描画・CPU-read α（R2.3「実描画 α」の可用性）
- `crates/wintf/src/ecs/graphics/compositor.rs`: `WindowD3D11Compositor` が `staging_bitmap`（`D2D1_BITMAP_OPTIONS_CPU_READ`・L21/L99）で合成済み α を CPU 読み取り可能。**ただしこれは ULW 専用経路**（structure.md: 「`compositor.rs`/`compositor_systems/` は ULW 専用ゆえ ULW 除去で撤去対象」）。
- `crates/wintf/src/ecs/graphics/wuc_resource.rs`／`com/wuc.rs`／`ecs/graphics/systems/init.rs`: WUC 経路（`Compositor`/`DesktopWindowTarget`/`CompositionDrawingSurface`）は **CPU-read α バッファを持たない**（GPU 合成のみ）。`init.rs` L215 で `CompositionMode::DComp` の window に `WucGraphicsResource` を遅延初期化。
- **含意（重要）**: 「本体の実描画 α バッファ」を GPU 合成結果から読む道は、WUC 移行後は**存在しない**（別途 CPU readback を新設しない限り）。→ 本坑での「実描画 α」は現実的には **per-widget `AlphaMask`（静止画 α）を当たり判定源にする**解釈が有力。この解釈確定は R2.3/R2.4（「表示更新に追随」）の実装可否を左右する最重要論点。

### 2.6 ドキュメント正典（R7・R10）
- `.kiro/steering/tech.md` L83-86（旧「ULW 一択」撤回・新方針）、`.kiro/steering/roadmap.md` L181 付近（「ULW 一択」動機節）、`doc/COMPAT_ARCHITECTURE.md`（設計正本）が R7.3／R10.3 の「更新対象を明示できる状態」の対象。本坑では**撤去せず並走**（R7.1）ゆえ、これら記述の実更新は ULW 撤去確定時（別坑）に回る。

## 3. 要件→資産マップ（ギャップタグ）

| 要件 | 必要能力 | 既存資産 | ギャップ |
|---|---|---|---|
| R1 GPU 合成維持クリック透過 | WUC/DComp 描画を壊さず ex-style トグル | WUC 経路一式・pilot go 実証 | **Missing**: 動的トグル機構（新規）。**Constraint**: areka は現状 ULW 既定＝実効化は WUC 移行後 |
| R2 αマスク連動受領 | カーソル位置の α 当たり判定 | `AlphaMask::is_hit`・`hit_test_entity` 変換チェーン | **Missing**: カーソル→α 問い合わせ経路。**Unknown**: 「実描画 α」の取得源（静止画 α か GPU readback か） |
| R3 カーソル監視＋差分最適化 | 別実行文脈の位置監視＋前回状態比較 | `VsyncEventBridge` 構造・`WintfTaskPool` | **Missing**: `GetCursorPos` ワーカ（新規・grep 0 件）・差分ガード |
| R4 event_listener 通知（tokio 不使用） | ワーカ→UI 起床＋UI スレッド適用 | `event_listener`＋`spawn_local` 前例完備 | **Low**: 前例踏襲で充足 |
| R5 ドラッグ中透過抑止 | ドラッグ中フラグで ON 抑止＋終了再収束 | `DragState`・`WindowDragContextResource`（Arc 共有） | **Missing**: 抑止フラグの導管選定（既存流用 vs 新 `Arc<AtomicBool>`） |
| R6 ex-style 構成制約 | TRANSPARENT 動的・LAYERED 同伴・NCHITTEST 不使用 | `WinStyle` ビルダー・pilot レシピ確定 | **Low**: レシピ確定済（要「独断で LAYERED/NCHITTEST 追加しない」規律の実装反映） |
| R7 ULW 並走・非破壊 | ULW 撤去せず既存機能維持 | ULW 経路現存・`CompositionMode` 残置 | **Low**: 追加のみ＝非破壊は構造的に容易 |
| R8 高 DPI・マルチモニタ座標一致 | スクリーン物理⇔αマスク座標の対応維持 | `hit_test` の物理座標変換・`DPI` component・pilot PMv2 検証（部分） | **Unknown/Constraint**: ワーカ座標系（`GetCursorPos`＝物理）→ウィンドウ→bounds→マスクの変換破綻回避（DPI 変化・モニタ跨ぎ） |
| R9 リリース互換・依存最小 | opt-level z / lto・32bit・新規大型クレート無 | 既存依存のみで実装可（windows・event_listener） | **Low**: 新規クレート不要見込み |
| R10 ドキュメント整備 | `docs/click_through.md` 新規 | pilot README/REPORT が下敷き | **Missing**: `docs/click_through.md`（新規作成タスク） |

## 4. 実装アプローチ選択肢

### Option A: 既存グラフィクス/入力サブシステムへ機構を織り込む（拡張中心）
カーソル監視ワーカと ex-style トグル器を `runtime/`（`tick_bridge.rs` 隣接）へ、α 問い合わせを `hit_test` 再利用で `ecs/` 側へ薄く足す。ドラッグ抑止は既存 `WindowDragContextResource`/`DragState` に相乗り。
- ✅ 既存 `VsyncEventBridge` の RAII/event_listener パターンを最大流用＝R3/R4 が最短。既存ドラッグ機構と自然統合。
- ✅ 新規ファイル最小・依存追加ゼロ。
- ❌ `runtime/` と `ecs/hit_test`・`drag` に跨る責務が散り、機構の可搬性（将来 ULW と一緒に語れる単位）が弱い。
- ❌ 「本体コードを推測で書き換えない」(R6.5) 制約下で既存ファイルへ手を入れる範囲の事前提示が増える。

### Option B: 独立した click-through サブシステムを新設（新規中心）
`ecs/clickthrough/`（または `runtime/clickthrough.rs`）に「カーソル監視ワーカ＋α問い合わせ＋差分適用器＋ドラッグ抑止フラグ」を自己完結モジュールとして新設。既存へは最小の結線（window 生成時の登録・drag 状態の read-only 参照）のみ。
- ✅ 責務が 1 モジュールに凝集＝pilot の「表示層/当たり判定層の分離」を構造として表現。ULW 撤去や将来 3D/Live2D 拡張時に触る単位が明確。
- ✅ R6.5（変更対象の事前提示）に沿いやすい＝既存改変が「結線点だけ」に絞られる。
- ✅ 単体テスト隔離が容易（差分ガード・座標変換をワーカから切り出してテスト可能）。
- ❌ α 問い合わせで `hit_test` ロジックと二重化する懸念（共通化の設計が要る）。
- ❌ 新規モジュール分の設計コストが Option A より高い。

### Option C: ハイブリッド（新規モジュール＋既存資産の明示的再利用）— 分析上の推奨軸
機構の骨格（ワーカ・トグル器・差分状態）は Option B の独立モジュールとして新設しつつ、
(1) α 判定は既存 `hit_test_entity`／`AlphaMask::is_hit` を**共有ヘルパとして呼ぶ**（二重化回避）、
(2) スレッド跨ぎは `VsyncEventBridge` を**テンプレに新 `CursorMonitorBridge` を作る**、
(3) ドラッグ抑止は既存 `WindowDragContextResource` の Arc 共有に**新フラグ（`Arc<AtomicBool>`）を相乗り or 併設**。
- ✅ 凝集（B の利点）と既存資産流用（A の利点）を両取り。回帰面と可搬性のバランスが良い。
- ✅ 段階実装しやすい: ①ワーカ＋起床（tick_bridge 相当）→②α問い合わせ結線→③トグル器→④ドラッグ抑止→⑤docs、の increment。
- ❌ 「共有ヘルパ」の置き場所と借用（World への read アクセスをワーカ非同期でどう安全に得るか）の設計判断が必要（ワーカは α スナップショットを Arc で受け取り、World は触らない、が安全側）。

## 5. 工数・リスク

- **工数見積: M〜L（1週間前後〜やや超）**。個々の部品は小さいが、スレッド跨ぎ×座標変換×ドラッグ抑止×WUC 非破壊×ULW 並走の接合点が多い。pilot go により機構リスクは既に低減。
- **リスク: 中**。
  - 中: 座標変換チェーン（R8）— DPI 変化・モニタ跨ぎ・ウィンドウ移動中の物理⇔マスク座標対応。pilot は「一致」を人間確認したが高 DPI 150%・モニタ跨ぎは pilot でも未検証（T7 部分/T6 明示未）。
  - 中: 「実描画 α」の定義（R2.3/R2.4）— 静止画 `AlphaMask` 採用なら SERIKO アニメ等で「表示更新に追随」（R2.4）が限定的になりうる。GPU readback は WUC に無く新設は大工事。
  - 低〜中: ワーカから α 判定に必要な状態（マスク・bounds・ウィンドウ位置・DPI）を**どのスナップショットで**渡すか（World 直アクセスは UI スレッド専有ゆえ不可）。
  - 低: ex-style トグルの `SWP_FRAMECHANGED` 副作用（WUC 描画・z オーダー・アクティベーションへの影響）— pilot で共存確認済み。

## 6. design フェーズへの申し送り（Research Needed / 決定待ち）

R1. **「本体の実描画 α バッファ」(R2.3) の具体的取得源**: (a) per-widget `AlphaMask`（WIC 静止画 α・現実的）／(b) WUC GPU 合成結果の CPU readback（新設・大工事・現状経路なし）／(c) ハイブリッド。R2.4「表示更新に追随」の充足度が (a)/(b) で変わる。**最重要**。
R2. **カーソル監視ワーカが α 判定に使う状態の受け渡し方式**: ワーカは UI スレッド World を触れない。α マスク＋bounds＋ウィンドウ位置＋DPI の**スナップショットを Arc 共有**（`VsyncEventBridge` 同様の Arc パターン）か、ワーカは `GetCursorPos` だけ行い判定は UI スレッド側で行うか。判定の実行スレッド境界の確定。
R3. **座標変換チェーンの正典**（R8）: `GetCursorPos`（スクリーン物理）→ウィンドウ client 原点→`global.bounds` 正規化→マスク座標。既存 `hit_test_in_window` を再利用するか、ワーカ用に軽量複製するか。DPI 変化・モニタ跨ぎ時の再スナップショット契機。
R4. **ドラッグ抑止フラグの導管**（R5）: 既存 `WindowDragContextResource`（Arc<Mutex>）へ相乗りか、pilot 同様 `dragging: Arc<AtomicBool>` を新設して drag state 遷移時に更新するか。終了時の「再収束」notify の発火点（`JustEnded` 遷移）。
R5. **適用単位（per-window）とマルチウィンドウ**: トグルは HWND 単位。areka は shell/balloon の 2 窓。各窓に監視器を持つか、単一ワーカが全対象窓を巡回するか。ウィンドウ内複数 widget の α 集約（OR）方針。
R6. **areka 実効化と移行順序**（R1/R7）: 本坑は「wintf に機構を用意」まで。areka を `CompositionMode::DComp`(WUC) へ切替える実効化は `wintf-dcomp-to-wuc-migration`／`wintf-ulw-removal` との順序に依存。本坑スコープが「機構提供のみ」か「areka での実動確認まで」かの線引き。
R7. **`SetWindowPos(SWP_FRAMECHANGED)` の副作用範囲**: トグル毎の FRAMECHANGED が WUC 合成・z オーダー・フォーカスへ与える影響の設計時確認（pilot は共存を実測、本坑本体経路で再確認要）。
R8. **ex-style 差分適用 API の置き場**: `WinStyle::commit` は FRAMECHANGED 非対応。トグル専用の最小 API（TRANSPARENT ビットのみ add/remove＋FRAMECHANGED）を `win_style.rs` に足すか、新モジュールに閉じるか（R6.4/R6.5 の「独断で追加しない」規律との整合）。
R9. **「独断で追加しない」制約の運用**（R6.4/R6.5・R9.3）: `WS_EX_LAYERED` 同伴は pilot 実証済で要件本文にも織り込み済（R6.2）。それ以外の ex-style／NCHITTEST／依存追加が実装中に必要化した場合の依頼者確認フローを design に明記。
