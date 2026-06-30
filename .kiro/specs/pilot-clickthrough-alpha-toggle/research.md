# ギャップ分析: pilot-clickthrough-alpha-toggle（先進坑 / pilot）

> 本書は確定済み requirements.md と既存コードベースの差分分析である。先進坑（throwaway・葉ノード隔離）ゆえ、production 統合（本坑 `wintf-clickthrough-alpha-toggle` の領分）ではなく「`WS_EX_TRANSPARENT` 動的トグル方式で別プロセス透過が成立するかを最小 example で実証するために何が必要か」に絞る。
> 一次成果物はコードではなく知見（go／違う／直す）。本書は設計フェーズの判断材料であり、実装の最終決定ではない。

## 1. 現状調査（既存資産）

### 1.1 先進坑インフラ（検疫所）
- `crates/pilot/`: 空 lib（`src/lib.rs` は `pub` item ゼロ＝命綱の構造的担保）＋ `examples/` のみ。`publish = false`、葉ノード。
- `crates/pilot/Cargo.toml`: 探索依存が既に整備済み。`wintf-winmsg-executor = "0.0.5"`（UI スレッド基盤・crates.io、vendor ではない）、`event-listener = "5"`、`windows = { workspace = true }`、`windows-core = { workspace = true }`。**新規依存追加は不要**（要件 10.1/10.2 と完全整合）。tokio は不在。
- `crates/pilot/examples/_template/`（`main.rs` ＋ `README.md` 3 幕雛形）: コピー元。要件 1.4 が指定するコピー着手元。
- `crates/pilot/examples/wintf-winmsg-executor/`（既存先進坑）: 本 pilot が踏襲すべき確立パターンの実例。

### 1.2 既存 example から得られる確立パターン（wintf-winmsg-executor/main.rs）
- **ウィンドウ生成**: `wintf_winmsg_executor::util::{Window, WindowMessage, WindowType}`。`Window::new_ex` / `new_checked_ex(WindowType::TopLevel, WINDOW_EX_STYLE(..), state, wndproc_closure)` で生成。**ex_style を生成時に `WINDOW_EX_STYLE` で直接渡せる**ため、`WS_EX_TRANSPARENT | WS_EX_TOPMOST` の初期付与が容易。
- **wndproc クロージャ**: `move |state: Pin<&S>, msg: WindowMessage| -> Option<LRESULT>`。`msg.msg`/`msg.hwnd` で分岐、`None` 返却で `DefWindowProc` フォールバック。`WM_LBUTTONDOWN`/`WM_PAINT`/`WM_ERASEBKGND` の受領・描画はこのクロージャ内で完結（要件 6 のクリック受領＋色トグルに直結）。GWLP_USERDATA 手詰め不要、state は `Pin<&S>` で安全アクセス。
- **メッセージループ**: `block_on(async { .. })` で駆動、`spawn_local` で UI スレッド async タスク。
- **スレッド跨ぎ起床**: `Arc<event_listener::Event>` を別 `std::thread` から `event.notify(usize::MAX)`、UI 側 async で `event.listen(); listener.await;`。**tokio 非依存**（要件 3.3/10.2 が要求する手段の既存実証）。`AtomicBool` で done フラグ＝ワーカ正常終了（要件 8.2 のパターン）。
- **共有状態**: `Rc<Shared>`（`!Send` 許容・UI スレッド内）。本 pilot は別スレッドのカーソルワーカが HWND/状態へ触れるため、要件 2.5 が許す Win32 慣例（`unsafe impl Send` ラッパ）が新たに要る点が差分（後述 3.1）。
- **描画**: GDI（`BeginPaint`/`FillRect`/`CreateSolidBrush`）。redirected 窓（ex_style に NOREDIRECTIONBITMAP を付けない）なら GDI が画面に出る。本 pilot の不透明四角は GDI FillRect で十分。

### 1.3 Win32 API バインディングの可用性（windows 0.62.2・workspace features）
workspace `[workspace.dependencies.windows].features` に必要 feature が全て有効:
- `Win32_UI_WindowsAndMessaging` → `SetWindowLongPtrW`/`GetWindowLongPtrW`/`GWL_EXSTYLE`/`SetWindowPos`/`SWP_FRAMECHANGED`/`GetCursorPos`/`WM_LBUTTONDOWN`/`WS_EX_*`。
- `Win32_UI_HiDpi` → `SetProcessDpiAwarenessContext`/`DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2`/`PhysicalToLogicalPointForPerMonitorDPI`（必要時）。
- `Win32_Graphics_Gdi` → 描画＋`MonitorFromPoint`/`GetDpiForMonitor`（マルチモニタ DPI 整合の補助）。
- `Win32_Foundation` / `Win32_System_Threading` → `HWND`/`POINT`/`RECT`/`LRESULT`。
既存 wintf コード（`win_style.rs`・`com/ulw.rs`・`runtime/window_factory.rs` 等 14 ファイル）が同 API 群を実利用済み＝バインディング・呼出規約は実証済み。**API 不足のギャップは無い。**

### 1.4 隣接資産（pilot では触れない＝Out of scope 確認）
- 完了済み `event-hit-test-alpha-mask`（既存αヒットテスト）、ULW 切替基盤（`com/ulw.rs`・`CompositionMode`）は本 pilot では参照も改変もしない（要件 Boundary・brief「Adjacent」と整合）。

## 2. 要件→資産マップ（ギャップ tag: Missing / Unknown / Constraint）

| 要件 | 必要技術要素 | 既存資産 | Gap |
|---|---|---|---|
| R1 葉ノード隔離 | examples/ のみ・inbound ゼロ・_template コピー | `crates/pilot` 構造・_template | なし（構造で担保済み・Constraint=人手レビュー規律） |
| R2 透過トップモスト窓＋不透明領域 | `WS_EX_TRANSPARENT` 単独・`WS_EX_LAYERED` 不付与・topmost・WM_NCHITTEST 不介入 | `Window::new_ex(ex_style)` パターン | Missing: WS_EX_TRANSPARENT 単独窓の実装（新規・パターンは既存） |
| R3 カーソル監視ワーカ | 16ms 周期・別 std::thread・event_listener 起床・GetCursorPos | event_listener 起床パターン（既存 example） | Missing: GetCursorPos 周期取得ワーカ（新規） |
| R4 αマスク差し替えシーム | 円判定関数（中心(960,540) r=200）・差替可能シーム | なし | Missing: 純関数 1 個（容易） |
| R5 状態変化最適化＋スタイル適用 | 前回状態比較・変化時のみ SetWindowLongPtr(GWL_EXSTYLE)+SetWindowPos(SWP_FRAMECHANGED)・ログ | API は実証済み | Missing: トグル＆差分ロジック（新規・核心） |
| R6 クリック受領＋色トグル | WndProc で WM_LBUTTONDOWN・色トグル・ログ | wndproc クロージャ＋GDI 描画（既存 example） | なし（パターン流用） |
| R7 DPI 認識・マルチモニタ整合 | SetProcessDpiAwarenessContext(PMv2)・座標一致 | API 可用 | Unknown: 円判定座標（物理/論理）とマルチモニタ整合の扱い（後述 3.2） |
| R8 終了処理 | 窓 close→プロセス＋ワーカ正常終了 | AtomicBool done パターン（既存 example） | なし（パターン流用） |
| R9 手動検証・REPORT・README | T1〜T8 手動・REPORT.md 指定フォーマット・README 3 幕 | _template README 3 幕雛形 | Constraint: REPORT.md フォーマット要定義（後述 3.3） |
| R10 技術・可搬性制約 | Rust 2024・windows 0.62.2・event_listener 5・tokio 禁止・32bit 可搬 | Cargo.toml 整備済み | なし（依存追加不要） |

**総括**: API レベルのギャップはゼロ（全 Win32 API がバインド済み・既存 example に全インフラパターンが揃う）。実装ギャップは「WS_EX_TRANSPARENT 単独窓＋カーソルワーカ＋状態差分トグル」という新規結線のみ。難所は API ではなく**OS 挙動の不確実性**（WS_EX_TRANSPARENT 単独でプロセス越え透過が本当に成立するか＝そもそも本 pilot が潰しに来た当の検証点）と DPI 座標系である。

## 3. 設計判断として浮上した論点（要件ディスカッションへ送る）

### 3.1 HWND/状態のスレッド跨ぎ共有方式（要件 2.5 / 3.x）
カーソルワーカ（別 std::thread）が現在状態を読み、トグル要否を判定し、UI スレッドへ起こす。HWND は `!Send`。共有モデルの選択肢:
- **(a) ワーカは判定のみ→UI スレッドで API 適用**: ワーカは GetCursorPos＋円判定し、状態変化時だけ `event.notify`。実際の `SetWindowLongPtr`/`SetWindowPos` は UI スレッド async タスク内で実行（HWND をスレッド跨ぎしない。望ましい状態は `AtomicBool`/`Atomic` で受け渡し）。Win32 的にも window スタイル変更は所有スレッドが行うのが安全。
- **(b) ワーカが直接 API 適用**: `unsafe impl Send for AppState`（HWND ラップ）でワーカから直接 `SetWindowPos`。brief が許す Win32 慣例だが、別スレッドからの window スタイル変更は推奨されず再入リスク。
- 推奨の方向性: (a)（責務分離・既存 example の event_listener パターンにそのまま乗る）。ただし「16ms 周期で常に notify か、変化時のみ notify か」は要件 5.4（非変化時 API 非呼出）と整合させる設計判断。**→ 開発者確認候補。**

### 3.2 DPI 座標系（要件 7・T7）— 最大の Unknown
- `GetCursorPos` は**物理ピクセル**を返す（PMv2 認識プロセスでは仮想スクリーン物理座標）。
- 円判定の中心 (960,540) r=200 が「物理ピクセルか論理 DIP か」未確定。要件 4.1 は数値を物理座標として固定しているように読めるが、要件 7.2 は「見た目の不透明領域と一致」を要求。不透明四角の描画（GDI・window client 座標）と GetCursorPos（仮想スクリーン物理座標）の座標系を一致させる必要がある。
- マルチモニタ高 DPI（150% 等）では、窓位置・不透明四角の物理矩形と GetCursorPos 物理座標を同じ基準で比較すれば一致するはず。**判定対象は「カーソルが不透明四角の物理矩形内か」であって固定円である必要はない**可能性（円は仮αマスクの代理）。
- 論点: αマスク円判定の入力座標系（物理 vs 論理）と、不透明四角の画面上物理矩形との整合をどう取るか。PMv2 で物理統一が単純だが、(960,540) 固定値がプライマリ前提（要件 7.3 はプライマリ専用前提を禁止）と矛盾しないか。**→ 設計フェーズで Research Needed、かつ開発者確認候補。**

### 3.3 REPORT.md の「指定フォーマット」定義（要件 9.5）
要件は REPORT.md を「指定フォーマット」で作れと言うが、フォーマット本体はどこにも定義がない（brief の go 基準表＝T1〜T8 ／合格基準 T1・T2・T3・T4・T6 必須・T5/T7/T8 条件付き、が事実上の骨子）。README 3 幕（_template）とは別物。
- 論点: REPORT.md のテンプレート（T1〜T8 各項目の合否欄＋証跡欄＋総合 go/違う/直す＋日付）を設計フェーズで定義する必要。README 3 幕との役割分担（README=動機/概要/検証結果サマリ、REPORT=T1〜T8 詳細記録）。**→ 設計フェーズで定義。**

### 3.4 状態変化最適化の起点フレーム（要件 5.3/5.4）
初回フレーム（前回状態なし）の扱い: 起動直後にカーソルが円内/円外いずれでも一度は ex_style を確定適用すべきか、それとも初期 ex_style を ON（透過）で生成し変化時のみ適用か。「変化時のみ API 呼出」と「起動時に正しい初期状態を保証」の両立を設計で明示。**→ 軽微な設計判断。**

### 3.5 WS_EX_TRANSPARENT 動的付け外しの OS 挙動（核心の Unknown）
本 pilot の存在理由そのもの。`SetWindowLongPtr(GWL_EXSTYLE)` 後に `SetWindowPos(.., SWP_FRAMECHANGED|SWP_NOMOVE|SWP_NOSIZE|SWP_NOZORDER|SWP_NOACTIVATE)` でスタイル変更を確定させる定石。`WS_EX_LAYERED` 無し・`WS_EX_TRANSPARENT` 単独でプロセス境界を越えてクリックが背面別プロセスへ落ちるか＝T2/T6 が実機で検証する点。**ここは設計で潰せず実機検証専用**（要件 9・go ゲートの本体）。要件 10.4「不確実点は推測で進めず質問」に従い、実装中の挙動疑義は開発者へ確認。

## 4. 実装アプローチ選択肢

### Option A: 既存 example（wintf-winmsg-executor）パターンを最大流用した単一 main.rs
- _template をコピーし、`Window::new_ex` ＋ wndproc クロージャ ＋ event_listener ワーカ ＋ block_on を踏襲。state を `Rc<AppState>`（UI 内）＋望ましい状態の `Atomic` 受け渡し（3.1(a)）。
- トレードオフ: ✅ 既存実証パターンに乗れる・最速・新規概念ほぼゼロ ✅ 葉ノード隔離・依存追加不要 ❌ 1 ファイルに集約され可読性は緩い（が先進坑ゆえ許容＝要件 5.4）。

### Option B: ワーカが直接 API 適用（unsafe impl Send ラッパ）
- brief 許容の Win32 慣例。HWND を Send ラップしワーカから直接 `SetWindowPos`。
- トレードオフ: ✅ UI↔ワーカの状態受け渡しが単純 ❌ 別スレッドからの window スタイル変更は非推奨・再入/競合リスク ❌ 検証対象（OS 挙動）にスレッド要因のノイズが混じる。

### Option C: ハイブリッド（判定はワーカ・適用は UI・最適化を段階導入）
- まず毎フレーム notify→UI で差分判定・適用（単純）で T1〜T6 を通し、次に「ワーカ側で変化時のみ notify」へ最適化（要件 5.4 の厳密充足）。
- トレードオフ: ✅ 段階検証でリスク分離（核心の OS 挙動を先に潰す）✅ 最適化は後付け ❌ 計画がやや増える。

**設計フェーズへの推奨方向（決定ではない）**: Option A を基線に、状態適用は UI スレッド側（3.1(a)）。状態変化最適化（R5）の充足のため、判定主体（ワーカ）が「変化時のみ notify」または「望ましい状態 Atomic を更新し UI が差分検出」のいずれかを設計で確定。核心の OS 挙動（3.5）と DPI 座標系（3.2）は実機検証＋開発者確認に委ねる。

## 5. 工数・リスク

- **工数: S（1〜3 日）**。既存 example に全インフラパターンが揃い、新規 Win32 API もバインド済み。実装は新規結線（透過窓＋カーソルワーカ＋差分トグル＋GDI 四角）のみ。コード量は単一 main.rs 規模。
- **リスク: Medium**。コードのリスクは低い（パターン流用・API 実証済み）が、本 pilot の本質的リスク（=掘る理由）は **OS 挙動の不確実性**: ① WS_EX_TRANSPARENT 単独でのプロセス越え透過の成否（3.5・T2/T6）、② 高 DPI/マルチモニタでの座標一致（3.2・T7）。これらは実装ではなく実機手動検証で判明する性質ゆえ、go/違う/直すのいずれに転んでも知見として価値がある（pilot の設計通り）。

## 6. 設計フェーズへ持ち越す Research Needed
1. WS_EX_TRANSPARENT 単独（WS_EX_LAYERED 無し）の別プロセス透過挙動の実機確認（T2/T6）— 設計で潰せず、go ゲートの本体。
2. PMv2 下の GetCursorPos 座標系（物理）と GDI 描画矩形・αマスク円中心(960,540)の座標整合、マルチモニタでの一致条件（T7・3.2）。
3. REPORT.md「指定フォーマット」の具体テンプレート定義（T1〜T8 合否＋証跡＋総合判定）と README 3 幕との役割分担（3.3）。
4. 状態変化最適化の責務配置（ワーカ判定→UI 適用 / 変化時のみ notify）と初回フレームの初期状態確定（3.1・3.4）。

---
_本書は kiro-validate-gap が生成したギャップ分析（情報提供・選択肢提示であり最終決定ではない）。次フェーズ: 要件ディスカッション → `/kiro-design pilot-clickthrough-alpha-toggle`。_
