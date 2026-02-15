# Research & Design Decisions: wintf-P0-click-through-rgn

## Summary
- **Feature**: `wintf-P0-click-through-rgn`
- **Discovery Scope**: Complex Integration（既存ECSシステム + 新規GDI API + DirectComposition互換性リスク）
- **Key Findings**:
  1. SetWindowRgn + WS_EX_NOREDIRECTIONBITMAP の互換性は公式ドキュメントで明示的に言及されておらず、実験検証が必須
  2. GDI リージョン API（CreateRectRgn, CombineRgn）は windows crate の `Win32_Graphics_Gdi` feature で利用可能（ワークスペース設定済み）
  3. 二層クリックスルー（SetWindowRgn = 粗いフィルタ, NCHITTEST = 精密フィルタ）の共存が設計上可能

## Research Log

### SetWindowRgn API の動作仕様
- **Context**: リージョン設定のライフサイクルと制約を把握
- **Sources Consulted**: [SetWindowRgn (MSDN)](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowrgn)
- **Findings**:
  - SetWindowRgn 呼び出し後、システムがリージョンハンドルの所有権を取得。呼び出し側はハンドルを DeleteObject してはならない
  - 呼び出し時に WM_WINDOWPOSCHANGING / WM_WINDOWPOSCHANGED が送信される
  - 座標はウィンドウ左上隅からの相対座標（クライアント領域ではない）
  - hRgn に NULL を渡すとリージョンがリセットされる（全領域が有効に戻る）
  - bRedraw=TRUE でシステムが再描画を実行
- **Implications**:
  - 毎回新しい HRGN を作成して SetWindowRgn に渡す。前のリージョンはシステムが管理
  - 座標変換: GlobalArrangement.bounds（スクリーン座標） → ウィンドウ相対座標への変換が必要
  - ドラッグ時全画面化: SetWindowRgn(hwnd, NULL, TRUE) でリージョンリセット

### CombineRgn / CreateRectRgn API の仕様
- **Context**: 矩形リージョンの合成パフォーマンスとAPI制約
- **Sources Consulted**: [CombineRgn (MSDN)](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-combinergn), [CreateRectRgnIndirect (MSDN)](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-createrectrgnindirect)
- **Findings**:
  - CombineRgn(dst, src1, src2, RGN_OR) で2つのリージョンを合成
  - 戻り値: NULLREGION / SIMPLEREGION / COMPLEXREGION / ERROR で結果タイプを判別
  - hrgnDst は事前に CreateRectRgn(0,0,0,0) 等で作成済みでなければならない
  - hrgnDst = hrgnSrc1 が許可される（インプレース合成）
  - CreateRectRgnIndirect は RECT を引数に取り、右辺・下辺は排他的（exclusive）
  - リージョン座標は 27-bit 符号付き整数で表現
- **Implications**:
  - 合成パターン: accumulator = CreateRectRgn(0,0,0,0); 各 bounds に対して temp = CreateRectRgn(bounds); CombineRgn(accumulator, accumulator, temp, RGN_OR); DeleteObject(temp)
  - 座標範囲: 27-bit = ±67,108,863 → 物理ピクセル座標としては十分
  - 右辺排他: D2D_RECT_F の right/bottom と一致（追加調整不要）

### SetWindowRgn + WS_EX_NOREDIRECTIONBITMAP 互換性
- **Context**: 最大リスク項目。DirectComposition と SetWindowRgn の共存可能性
- **Sources Consulted**: MSDN SetWindowRgn, DWM Overview, DirectComposition Concepts, StackOverflow, 各種フォーラム
- **Findings**:
  - **公式ドキュメントに互換性の明示的記述なし**
  - SetWindowRgn は DWM Step 1（OS レベル）でウィンドウ形状を定義。リージョン外はヒットテスト・描画の両方でスキップ
  - WS_EX_NOREDIRECTIONBITMAP はビットマップリダイレクトを無効化し、DirectComposition が直接画面に描画
  - **理論的考察**:
    - SetWindowRgn は DWM のヒットテスト領域を制御（入力側）
    - DirectComposition は描画パイプラインを制御（出力側）
    - 両者は異なるレイヤーで動作するため、共存が可能である可能性が高い
    - ただし、SetWindowRgn がビジュアルのクリッピングにも影響する場合、DirectComposition の描画が切り取られる可能性あり
  - **過去の事例**: 多くの shaped window（非矩形ウィンドウ）実装が SetWindowRgn を使用しているが、DirectComposition との組み合わせは一般的ではない
- **Implications**:
  - **実装初期に互換性テストを最優先で実施**
  - テスト内容: (1) SetWindowRgn 呼び出しが成功するか、(2) DirectComposition Visual の描画が維持されるか、(3) リージョン外のクリックが他プロセスに貫通するか
  - 失敗時のフォールバック: DirectComposition を破棄してレガシー描画に切り替え、またはアプローチ全体を破棄

### WM_TIMER ディスパッチと ECS World アクセス
- **Context**: 0.25秒タイマーの実装方式検討
- **Sources Consulted**: 既存コードベース（ecs_wndproc, win_thread_mgr, win_message_handler）
- **Findings**:
  - `ecs_wndproc` に WM_TIMER の match arm が存在しない（DefWindowProcW にフォールスルー）
  - `win_message_handler.rs` のトレイトに WM_TIMER シグネチャあり（None 返却のデフォルト実装）
  - VSync スレッドパターン: 別スレッド → PostMessageW(WM_USER+N) → メインスレッドで処理
  - message_window (HWND_MESSAGE) がタイマー設定先として利用可能
  - メッセージハンドラの World アクセス: `super::try_get_ecs_world()?` → `world.try_borrow_mut()?` → 操作
  - SetTimer は HWND に対して呼び出し、WM_TIMER はその HWND の wndproc に送信される
- **Implications**:
  - **方式A（SetTimer + 個別ウィンドウ HWND）**: ecs_wndproc に WM_TIMER arm 追加。WindowHandle 挿入時に SetTimer 呼び出し
  - **方式B（SetTimer + message_window）**: WinThreadMgr 初期化時に SetTimer。メインメッセージループ or 専用ハンドラで WM_TIMER をキャッチ
  - 方式A の方がシンプル（既存 ecs_wndproc パターンを活用）

### NCHITTEST との共存メカニズム
- **Context**: 既存のクリックスルー機能との関係
- **Sources Consulted**: 既存コード（nchittest_cache.rs）、MSDN WM_NCHITTEST
- **Findings**:
  - 既存パイプライン: WM_NCHITTEST → cached_nchittest → hit_test_in_window → HTTRANSPARENT / HTCLIENT
  - HTTRANSPARENT は「同一スレッドの兄弟ウィンドウ」のみに転送（クロスプロセス不可）
  - SetWindowRgn はリージョン外のクリックを OS レベルでスキップ（WM_NCHITTEST に到達しない）
  - SetWindowRgn リージョン内のクリックは通常通り WM_NCHITTEST に到達
- **Implications**:
  - **二層アーキテクチャ**: SetWindowRgn（粗い first-pass）→ NCHITTEST（精密 second-pass）
  - SetWindowRgn は矩形の union でクリックスルー領域を定義（粗い精度）
  - NCHITTEST は残りのエリアでピクセル単位の判定を継続
  - 両者は互いに干渉しない。既存の NCHITTEST コードは変更不要

### ドラッグ時リージョン拡張
- **Context**: ドラッグ操作中のリージョン動作
- **Sources Consulted**: 既存コード（drag/state.rs, drag/mod.rs, nchittest_cache.rs）
- **Findings**:
  - DragState（thread_local）: `read_drag_state()` で Idle/Preparing/JustStarted/Dragging/JustEnded を判別
  - WindowDragging（ECS マーカー）: `Query<Entity, With<WindowDragging>>` で検出
  - 既存の NCHITTEST ドラッグガード: ドラッグ中は HTCLIENT を強制返却
  - nchittest_cache.rs のドラッグ検知パターンが参考になる
- **Implications**:
  - ドラッグ中: SetWindowRgn(hwnd, NULL, FALSE) でリージョンリセット（全画面有効化）
  - ドラッグ終了: 次のタイマー更新でリージョン再構築
  - read_drag_state() をリージョン構築関数内で呼び出し

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| A. 独立モジュール + SetTimer (個別HWND) | `ecs/click_through_rgn.rs` + ecs_wndproc に WM_TIMER arm 追加 | リジェクション最容易、既存パターン活用、最小変更 | ecs_wndproc への変更1箇所 | **推奨** |
| B. 独立モジュール + SetTimer (message_window) | 同上 + message_window にタイマー設定 | ウィンドウ個別管理不要 | message_window は ecs_wndproc を使わない可能性 | 要検証 |
| C. ECS システム + フレームカウント | region_update_system を ECS schedule に追加 | ECS 統合が自然 | VSync 依存で 250ms 精度保証なし、リジェクション複雑 | 不採用 |
| D. 専用スレッド + PostMessage | VSync パターン踏襲 | 精度高い | スレッド管理オーバーヘッド、既存変更多い | 過剰 |

## Design Decisions

### Decision: タイマー方式 — SetTimer + 個別ウィンドウ HWND（方式A）
- **Context**: 0.25秒間隔のリージョン更新トリガーの実装方式
- **Alternatives Considered**:
  1. SetTimer + message_window — message_window のメッセージディスパッチ経路が不明確
  2. ECS システム + フレームカウント — VSync 依存で精度保証なし
  3. 専用スレッド — 実験的仕様に対してオーバーヘッドが過剰
- **Selected Approach**: SetTimer(hwnd, timer_id, 250, None) を WindowHandle 挿入時に呼び出し。WM_TIMER は ecs_wndproc に match arm を追加して処理
- **Rationale**: 既存の ecs_wndproc パターンを踏襲。World アクセスは既存ハンドラと同じ手法（try_get_ecs_world）。KillTimer + モジュール削除でリジェクション完了
- **Trade-offs**: ecs_wndproc に1箇所の match arm 追加が必要だが最小限の変更
- **Follow-up**: timer_id は TIMER_ID_CLICK_THROUGH_RGN 等の定数として定義

### Decision: 座標変換 — GlobalArrangement.bounds（スクリーン座標）→ ウィンドウ相対座標
- **Context**: SetWindowRgn のリージョン座標はウィンドウ左上隅からの相対座標
- **Alternatives Considered**:
  1. ScreenToClient で毎回変換
  2. WindowPos.position をオフセットとして減算
- **Selected Approach**: WindowPos.position（ウィンドウ左上スクリーン座標）を取得し、GlobalArrangement.bounds から減算してウィンドウ相対座標に変換
- **Rationale**: ScreenToClient はクライアント領域基準であり、SetWindowRgn はウィンドウ全体基準。WindowPos.position がウィンドウ左上を直接示すため最もシンプル
- **Trade-offs**: WS_OVERLAPPEDWINDOW の場合、ウィンドウ左上 ≠ クライアント領域左上。ただし wintf のデスクトップマスコット用途では non-client area がほぼ存在しないため問題にならない
- **Follow-up**: WS_POPUP スタイルの場合の動作を要検証（ウィンドウ左上 = クライアント左上が成立するか）

### Decision: リージョン構築パイプライン
- **Context**: エンティティの bounds をどのように HRGN に変換するか
- **Alternatives Considered**:
  1. 全エンティティを個別 CreateRectRgn + CombineRgn
  2. ビットマップ中間表現経由
- **Selected Approach**: アキュムレータパターン。空リージョン作成 → 各エンティティの bounds をグリッドスナップ → CreateRectRgn → CombineRgn(RGN_OR) で合成 → 最後に SetWindowRgn
- **Rationale**: 矩形直接合成は実験的仕様としてシンプルさを優先。ビットマップは将来の AlphaMask 対応時に拡張すればよい
- **Trade-offs**: エンティティ数 N に対して O(N) 回の CreateRectRgn + CombineRgn 呼び出し。リージョンの複雑度は矩形数に比例
- **Follow-up**: 大量エンティティ（100+）時のパフォーマンスプロファイリング

### Decision: モジュール配置と公開範囲
- **Context**: リジェクション容易性のためのモジュール設計
- **Selected Approach**: `ecs/click_through_rgn.rs` に全ロジックを集約。`pub(crate)` 公開。
- **Rationale**: 単一ファイルで完結させることで、モジュール削除 = 機能削除を実現。既存ファイルへの変更は最小限（3箇所: ecs/mod.rs, ecs/window_proc/mod.rs, ecs/window.rs）

## Risks & Mitigations
- **SetWindowRgn + WS_EX_NOREDIRECTIONBITMAP 互換性**: 実験検証タスクを実装初期に配置。失敗時は全アプローチ破棄（Req 6 で対応）
- **パフォーマンス劣化（CombineRgn O(N)）**: Req 8 のパフォーマンス測定で早期検知。16ms 超過時は最適化または方式変更
- **WM_TIMER 精度**: OS の timer resolution により ±50ms 程度のジッターあり。受容可能（Req 1.4）
- **ドラッグ操作中のちらつき**: ドラッグ開始→リージョンリセット→ドラッグ終了→リージョン再構築の遷移にラグ。bRedraw=FALSE で軽減

## References
- [SetWindowRgn (MSDN)](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowrgn) — API仕様、所有権移転の挙動
- [CombineRgn (MSDN)](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-combinergn) — リージョン合成の詳細
- [CreateRectRgnIndirect (MSDN)](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-createrectrgnindirect) — 矩形リージョン作成、右辺排他仕様
- [DWM Overview (MSDN)](https://learn.microsoft.com/en-us/windows/win32/dwm/dwm-overview) — DWM の描画パイプライン
- [DirectComposition Concepts (MSDN)](https://learn.microsoft.com/en-us/windows/win32/directcomp/directcomposition-concepts) — DComp アーキテクチャ
- [Mouse Input: WM_NCHITTEST (MSDN)](https://learn.microsoft.com/en-us/windows/win32/inputdev/about-mouse-input) — HTTRANSPARENT メカニズム
