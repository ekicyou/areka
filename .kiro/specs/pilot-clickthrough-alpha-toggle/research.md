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
- **【要件ディスカッション #1 で解決】** 描画とマスクを**一致**させる方針を採用（Option A）。固定円 (960,540) は破棄し、**ウィンドウ中央の半径 200px の円を描画し、その同一円を実スクリーン物理座標で判定**する（四角は廃し円に統一＝開発者「手抜きなら円」）。これにより T2/T3/T6 の観測が曖昧にならず、窓位置からの実算出ゆえマルチモニタ/高 DPI（T7）にも自動整合。要件 R2.2/R4.1/R4.4/R6/R7.2 を更新済み。**設計フェーズに残る詳細**: GetCursorPos 物理座標と窓クライアント中心円の物理位置を求める具体手順（窓位置取得＋client→screen 変換、必要なら per-monitor DPI 補助）。

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

---

## 7. 設計フェーズ synthesis と確定設計判断（kiro-spec-design）

> 本節は設計生成フェーズ（discovery type: light／既存先進坑パターンの拡張・integration-focused）で §3 の持ち越し論点を確定した記録。API ギャップはゼロ（§2 総括）のため外部 web research は不要、既存 example `wintf-winmsg-executor/main.rs` と workspace `windows` features の確認で十分と判断。

### Decision 1: スレッド跨ぎ共有モデル（§3.1 / §6.4 を確定）
- **Alternatives**: (a) ワーカ判定→UI 適用（HWND を跨がない）/ (b) ワーカ直接 API 適用（unsafe impl Send）。
- **Selected**: (a)。ワーカは `GetCursorPos`＋`GetWindowRect`（読み取り専用 API・別スレッド可）＋ `alpha_is_opaque` 判定のみ。望ましい状態は `Arc<AtomicBool> desired_passthrough` で公開、`SetWindowLongPtr`/`SetWindowPos`（スタイル変更）は窓所有 UI スレッドの spawn_local タスクのみが実行。
- **Rationale**: Win32 で window スタイル変更は所有スレッドが行うのが安全（別スレッド変更は再入リスク）。既存 example の event_listener パターンにそのまま乗る。検証対象（OS 挙動）にスレッド要因ノイズを混ぜない。
- **notify 方式**: 「変化時のみ notify」を採用（毎フレーム notify ではなく）。ワーカが判定主体である責務分離と整合し、UI 起床自体を抑制。UI 側でも `applied` と比較する二重ガードで R5.4（非変化時 API 非呼出）を保証。

### Decision 2: 座標手順（§3.2 ディスカッション #1 の残課題＝具体手順を確定）
- **Context**: 描画円＝マスク円一致方針（Option A・固定(960,540)破棄）は要件ディスカッションで決定済み。残るは GetCursorPos 物理座標と窓クライアント中心円の物理位置を求める具体手順。
- **Selected**: PMv2 認識プロセスでは `GetCursorPos` も `GetWindowRect` も**仮想スクリーンの物理ピクセル座標**を返すため、両者を同一基準で比較でき **DPI 変換不要**。円中心 = 窓矩形中心 `((left+right)/2,(top+bottom)/2)`、半径 200（物理px）、`dx*dx+dy*dy<=r*r` で判定。描画も PMv2 でクライアントが物理pxゆえ同一中心・同一半径の円が一致。
- **Rationale**: 窓位置からの実算出ゆえマルチモニタ/高DPI（T7）に自動整合。プライマリ前提（R7.3 禁止）に陥らない。
- **Follow-up**: 実機で見た目と判定がずれる場合のみ per-monitor 補助（`MonitorFromPoint`/`GetDpiForMonitor`）を検討（T7・既存 `crates/wintf/src/ecs/window/monitor.rs` に GetDpiForMonitor 実利用例あり）。

### Decision 3: REPORT.md「指定フォーマット」定義（§3.3 を確定）
- **Selected**: REPORT.md は T1〜T8 合否台帳（合否欄＋証跡欄）＋必須合格基準充足＋総合判定（go/違う/直す＋日付＋理由・学び・人間記入）。design.md「Testing Strategy」にテンプレート全文を定義。
- **役割分担**: README 3 幕＝動機/概要/検証結果サマリ（結論・正本）、REPORT＝T1〜T8 詳細台帳（根拠）。README が結論・REPORT が根拠。two-tunnel.md「本坑 design は README の検証結果を参照し二重化しない」と整合（go 知見の正本は README 側）。

### Decision 4: 状態変化最適化の起点フレーム（§3.4 を確定）
- **Selected**: 窓を初期 ex_style=`WS_EX_TRANSPARENT|WS_EX_TOPMOST`（クリックスルー ON）で生成。UI 側 `applied` 初期値も ON に一致。ワーカ初回判定は `last`=未確定とし初回のみ無条件に desired 確定＋notify。
- **Rationale**: 「起動時にカーソルが円内」でも初回 1 回だけ正しく OFF へ適用、以降は変化時のみ。初期生成状態（ON）とカーソルが実際に円外なら applied 一致で API 呼出ゼロ。「変化時のみ API 呼出」と「起動時の正しい初期状態保証」を両立。

### Decision 5: 視覚的透過の実現機構（R2.2/R2.3 を確定・設計再実行で解決）

- **Context**: 旧 design.md は「透過の見え方（全域透明＋円のみ可視）は実機確認が要る」と視覚的透過機構を runtime 観測に先送りしていた。これは「どの Win32 API で視覚透過を達成するか」という設計判断を runtime に punt した欠陥。本 pilot の検証対象は `WS_EX_TRANSPARENT` 動的トグルゆえ、視覚機構自体が透明領域でクリックを透過させてはならない（さもないとトグル検証が汚染され pilot が無意味になる）。
- **Alternatives と棄却理由**:
  - (a) `WS_EX_LAYERED` + `SetLayeredWindowAttributes(LWA_COLORKEY)`: カラーキー透明ピクセルが**自動でクリック透過**しトグル検証を汚染。さらに `WS_EX_LAYERED` 付与が必要で R2.3（layered 不付与）違反。**棄却**。
  - (b) `SetWindowRgn` で円クリップ: クリップ除外領域がリージョン経由で**クリック透過**しトグルを汚染。任意ピクセル単位αマスク（本来用途）も表現不能。**棄却**。
  - (c) **DWM extend-frame glass**（`DwmExtendFrameIntoClientArea(hwnd, &MARGINS{ -1,-1,-1,-1 })`）: 非 layered・redirected 窓に sheet-of-glass を適用。GPU（DWM）合成で視覚透過。クライアントは黒で塗った領域がガラス（背面透過）として抜け、不透明円は黒以外の単色で可視。
- **Selected**: (c) DWM extend-frame glass。
- **Rationale**: DWM ガラス透過は**純粋に視覚効果**であり窓は全矩形でヒットテストされ続ける。よってクリック透過は `WS_EX_TRANSPARENT` トグル単独で制御され、視覚機構が検証対象を汚染しない。これは本坑が採る DirectComposition シナリオ（DComp も視覚αを与えつつヒットテストは全矩形・トグルで制御）を忠実に写し、`tech.md`「DComp 描画を捨てられない」前提と「`WS_EX_LAYERED` 不付与」制約の双方と整合する。
- **依存**: `DwmExtendFrameIntoClientArea` ＝ `windows::Win32::Graphics::Dwm`（feature `Win32_Graphics_Dwm`・workspace 有効済み）、`MARGINS` ＝ `windows::Win32::UI::Controls`（feature `Win32_UI_Controls`・workspace 有効済み）。**新規依存・新規 feature 不要**（workspace Cargo.toml line 66/77 で確認）。
- **GDI 注意点（既知・軽微）**: GDI はαを書かないため「背景＝黒（→透過）／不透明円＝黒以外の単色（→可視）」の塗り分け規約を用いる。円縁の軽微なエッジ・アーティファクトは先進坑（品質緩和・R1.5）で許容。実機で初めて判明する不確実点ではない。
- **核心 Unknown との分離**: 視覚的透過は本決定で確定済み。残る実機 Unknown は `WS_EX_TRANSPARENT` 単独でのプロセス越え**クリック**透過の成否（§3.5・T2/T6）のみ。DWM ガラスはヒットテストに無影響ゆえこの核心 Unknown の純度を保つ。

### Synthesis 結論
- **Build-vs-adopt**: 全インフラ（窓生成・wndproc・block_on/spawn_local・event_listener 起床・AtomicBool done・GDI 描画）は既存 example から adopt。新規 build は αマスク純関数 1 個と状態差分トグルのみ＝新規概念ほぼゼロ（§4 Option A・§5 工数 S を追認）。
- **Simplification**: 単一 `main.rs` 集約（先進坑ゆえファイル分割せず・要件 5.4 品質緩和）。production・pilot/lib.rs・pilot/Cargo.toml への変更ゼロ（依存追加なし）。
- **視覚的透過機構（Decision 5）**: DWM extend-frame glass を adopt（プラットフォーム native 機構）。layered/colorkey/region は検証汚染ゆえ build/adopt いずれも棄却。新規依存ゼロ。
- **残置 Unknown（設計で潰せない）**: `WS_EX_TRANSPARENT` 単独のプロセス越え**クリック**透過の成否（§3.5・T2/T6）＝go ゲート本体・実機検証専用。視覚的透過は Decision 5 で確定済みゆえ Unknown から除外。

### 設計レビューゲート結果
- Mechanical: 全 numeric requirement ID（R1〜R10 の全 .M）が traceability 表＋コンポーネントブロックに出現／Boundary 4 節 populated／File Structure 具体パス／orphan component なし／boundary↔file 整合（examples-only）。**PASS**。
- Judgment: 責務境界明示・契約具体（rust シグネチャ・state model・Ordering）・§3 持ち越し 4 論点すべて確定・spec gap なし。**修復パス 0 回で PASS**。

### 設計再実行（merge mode・視覚的透過機構の確定）
- **トリガ**: 実装中に判明した設計欠陥＝旧 design.md/design-validation.md が視覚的透過機構を runtime 観測に先送り（「透過の見え方は実機確認が要る」を Risks に記載）。これは「どの Win32 API で視覚透過を達成するか」という設計判断の punt であり欠陥。
- **解決**: Decision 5（DWM extend-frame glass）で確定。design.md に「視覚的透過方式」節を新設し、TransparentWindow コンポーネント・Technology Stack・Allowed Dependencies・File Structure・Traceability(R2)・Open Questions を merge 更新。旧 Risks の視覚透過先送り項目を除去（核心 Unknown＝クリック透過は維持）。
- **再レビューゲート**: Mechanical 全 ID 維持・boundary/file/orphan 健全＝**PASS**。Judgment 視覚透過機構決定済み・棄却代替明記・核心 Unknown 純度維持・spec gap なし＝**修復パス 0 回で PASS**。
