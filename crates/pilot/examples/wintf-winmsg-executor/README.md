# 先進坑: wintf-winmsg-executor

> この README は本先進坑の**一次記録（正本）**である。本坑 spec の design はここの検証結果を参照し、
> 同じ結果を二重化しない（二坑モデル要件 3.5）。規律の正本は `.kiro/steering/two-tunnel.md`。

## 動機（なぜ掘るか）

- 対応する本坑 spec: `wintf-winmsg-executor`（`.kiro/specs/wintf-winmsg-executor/brief.md`）
- 確認したい方向 / 実現可能性 / 手順（＝「怪しい所」だけを掘る）:
  - wintf のメッセージループ・ウィンドウ起動・UI スレッド async を外部クレート
    `wintf-winmsg-executor`（v0.0.3・`winmsg-executor` フォーク）へ置き換え可能か。
  - **最大の不確実点**: 現状 VSync スレッド → `PostMessageW(WM_VSYNC)` 駆動の 60Hz ECS tick を、
    executor 上の async タスクへ移し、`event_listener` ブリッジで起床させる構造が**安定して回るか**。
  - executor の nested-message 処理（`new_checked` の `RefCell` 再入防止）と、ECS の再入ガード
    （`IS_TICK_FLUSH_IN_PROGRESS`）が衝突しないか。
  - よく分かっている所（基本的な窓生成・block_on/spawn_local の素の利用）は掘りすぎない。

## 概要（何を作ったか）

- 実装内容（`main.rs`・約 240 行・使い捨て品質）:
  - `util::Window::new_checked_ex` で `WS_EX_NOREDIRECTIONBITMAP` 付き TopLevel 窓を 1 枚生成。
    状態 S=`Rc<Shared>`（ECS の `Rc<RefCell<EcsWorld>>` 相当）をライブラリに預け、
    wndproc は `Pin<&S>` で受領（`GWLP_USERDATA` 手詰めゼロ）。
  - 別スレッドで `DwmFlush()` 60Hz ループ → `event_listener::Event::notify(1)`。
  - UI スレッドで `spawn_local` した async tick タスクが `event.listen().await` で起床 →
    1 フレーム処理（interval 計測＋フレームカウンタ）→ 再 `listen().await`。`block_on` で join。
  - tick の最中に**実 `SetWindowPos`（横バウンス＝三角波で実移動）**を撃ち、OS に正当な
    `WM_WINDOWPOSCHANGED`（valid `WINDOWPOS*`）を nested 生成させ、さらに wndproc 内から
    自窓へ同期 `SendMessage` を撃って wndproc 再入を誘発（RefCell ガード検証）。
  - **目視＋並行ストレス**: 縦 3 窓 × 3 tick タスクを生成し、1 本の vsync が
    `event.notify(usize::MAX)` で 3 タスクを毎 vblank 同時起床。各窓を**違う周期**
    （三角波 period=[70,130,200] frames）で横バウンスさせ、`WM_PAINT` で R/G/B 別軸の
    脈動色に塗る（borderless 矩形・約 8 秒で自走終了）。3 窓が別速度・別色で動けば
    「event_listener 起床→UI async tick→毎フレーム描画/移動」が並行で破綻なく回る可視証拠。
  - 可視窓は `ex_style=0`（redirected）。**`WS_EX_NOREDIRECTIONBITMAP` は GDI(WM_PAINT) の
    描画結果が合成されず不可視**（後述の学び）。NOREDIRECTIONBITMAP の生成・dispatch 自体は
    headless 実行で別途実証済み。
  - `block_on` の future 完了（3 タスク join）による清掃終了を確認（tail race 回避に終了時 notify を数発）。
- 実行法: `cargo run -p pilot --example wintf-winmsg-executor`（wall-clock 約 6 秒で自動終了）。
  - 注: worktree では前段に `git submodule update --init --recursive`（`vendors/pasta` 未populate 回避）。
  - 依存: `wintf-winmsg-executor = "0.0.5"`（検証は 0.0.3 で実施・0.0.5 で再ビルド確認済み＝API 互換。
    0.0.5 は共有クラスに `CS_DBLCLKS` ＋既定カーソルを内蔵＝本坑で wintf 側 dblclick 補填が不要化）,
    `event-listener = "5"`。

## 検証結果

- 判定: **go（開発者承認・2026-06-29）**。4 基準 ＋ 並行ストレス（縦 3 窓・違う周期）すべて PASS。
  開発者が可視動作を目視確認し go を確定（二坑モデル要件 6.3）。本坑 `wintf-winmsg-executor` の
  BLOCKED を解除する。
- 合否基準と実測（wall-clock 6 秒・120Hz 機・可視モード）:
  1. **tick 起床（PASS）**: notify 721 → frame 719（**coverage 99.7%**）、interval min5.64 / avg8.33 /
     max10.18ms（≈120Hz 追従）。スレッド跨ぎ `event_listener` 起床が安定動作（≈6 秒連続でも
     取りこぼし最小・破綻なし）。
  2. **nested-message × RefCell（PASS）**: tick 中の実 `SetWindowPos` 由来 `WM_WINDOWPOSCHANGED` を
     719 回 nested dispatch、`reentry_body_ran=false`（RefCell が wndproc 再入を阻止）、
     `double_tick=false`、デッドロックなし。
  3. **new_ex state アクセス（PASS）**: `WS_EX_NOREDIRECTIONBITMAP` 窓の wndproc から `Pin<&S>` 経由で
     共有状態へ到達（`GWLP_USERDATA` 手詰めゼロ）。
  4. **清掃終了（PASS）**: `block_on` の future 完了（3 タスク join）でメッセージループが
     panic せず復帰。
  5. **並行ストレス（PASS）**: 縦 3 窓 × 3 tick タスクを 1 本の vsync が `notify(usize::MAX)` で
     同時起床。8 秒・961 vblank で全窓 frames=960（**cov 99.9%**）・avg8.33ms・全窓
     `reentry_body_ran=false`／`double_tick=false`、クラッシュ/デッドロックなし。違う周期で
     動かしても executor は破綻しない。

- 学び（本坑をクリーンに掘り直すための材料・コピペ donor にはしない）:
  - **`WS_EX_NOREDIRECTIONBITMAP` ＝ GDI 不可視（重要）**: このスタイルの窓は redirection
    surface を持たないため、`WM_PAINT`/`BeginPaint`/GDI の描画結果が DWM に合成されず**画面に
    出ない**（メッセージ自体は届く）。可視化には **DirectComposition**（visual＋surface/swapchain）が
    必須。これは areka の ULW/DComp 合成方針とまさに一致する＝本坑/render 層では NOREDIRECTIONBITMAP
    窓を DComp で描く前提を崩さないこと。GDI を当てにしたデバッグ描画は不可視になる罠。
  - **起床機構**: cross-thread `Event::notify` → waker → executor 内部の `PostMessage(MSG_ID_WAKE,
    runnable*)` → UI スレッドの `block_on`/`spawn_local` タスクが `listen().await` から起床。
    **tokio 不要**で C# `TaskCompletionSource` 相当のスレッド跨ぎ起床が成立。これが本坑の 60Hz
    tick 駆動の中核に使える。
  - **リフレッシュレート非依存**: `DwmFlush` は実 vblank に同期するため 120Hz 機では ≈8.3ms 周期。
    **tick 設計を固定 16.67ms 前提にしない**（vblank cadence 追従＝可変）。areka 現行も DwmFlush
    駆動ゆえ同特性で、フレーム時間は実測ベースで扱うべき。
  - **再入の構造的軽減**: 新モデルは tick を「message から」ではなく「event_listener 起床の async
    タスク」で駆動するため、`WM_WINDOWPOSCHANGED` ハンドラ等から tick が再起動される旧来の再入
    （areka の `IS_TICK_FLUSH_IN_PROGRESS` が守っていた経路）が**構造的に発生しにくい**。加えて
    `new_checked_ex` の `RefCell` が wndproc 自体の modal/nested 再入をライブラリ標準で防ぐ。
    → 本坑では自前再入ガードの一部をライブラリ＋新モデルへ委譲できる見込み。
  - **state 機構＝GWLP_USERDATA 全廃**: `Window<S>` に `Rc<RefCell<EcsWorld>>` を預け、wndproc は
    `Pin<&S>` で受領する形へ。現行 `ecs_wndproc`（Entity を `GWLP_USERDATA` 格納し World へ
    dispatch）はこの state 機構へ移行可能。
  - **終了規律**: `block_on` は「メッセージループが future より先に quit すると panic」仕様
    （src/lib.rs の `expect("received unexpected quit message")`）。シャットダウンは **未完タスクを
    残したまま `PostQuitMessage` を撃たず、future を完了させてから**抜けるのが原則。
  - **落とし穴（重要・本坑で踏むな）**: `WM_WINDOWPOSCHANGED` 等 lParam が構造体ポインタの
    システムメッセージを `SendMessage` で**合成送出（lParam=NULL）すると user32 内で
    STATUS_ACCESS_VIOLATION**。テスト/駆動では OS 生成メッセージのみ扱い、合成しない。
    （本先進坑も初版でこれを踏み、実 `SetWindowPos` 駆動へ修正して解消。）
  - **バージョン**: 0.0.3 で検証。`util::get_instance_handle` は 0.0.4 で公開（本先進坑は `new_ex` が
    クラス登録を内部処理するため未使用）。本坑が DLL 文脈で HINSTANCE を要する場合は 0.0.4 の
    `get_instance_handle`（`__ImageBase` 方式＝`GetModuleHandle(NULL)` と違い DLL でも正しい）が有用。
- 日付: 2026-06-29
