# 先進坑: pilot-clickthrough-alpha-toggle

> この README は先進坑の**一次記録（正本）**である。本坑 spec の design はここの検証結果（go／違う／直す ＋ 学び）を参照し、
> 同じ結論を二重化しない。T1〜T8 の機械的な合否・証跡の詳細台帳は隣の `REPORT.md`（REPORT が根拠・README が結論）。
> （3 幕の各欄はタスク 6.1 で整備し、検証結果はタスク 6.2 で人間が確定する）

## 動機（なぜ掘るか）

- 対応する本坑 spec: `wintf-clickthrough-alpha-toggle`   ← 先進坑⟷本坑の traceability（本坑はこの go 判定を `_Depends(confirmed):` 前提依存とする）
- 確認したい方向 / 実現可能性 / 手順:
  - `WS_EX_LAYERED` 無し・`WS_EX_TRANSPARENT` 単独で、別プロセス（背面アプリ）へ**クリックが透過**するか（核心 Unknown・T2/T6）。
  - **DComp（GPU 合成）描画を捨てずに**別プロセス透過を成立させる第 4 の手（`WS_EX_TRANSPARENT` 動的トグル）の実現可能性。
  - 視覚的透過は `WS_EX_NOREDIRECTIONBITMAP` 窓上の **DirectComposition visual tree（per-pixel α）** で実現する（production 等価の描画経路）。GDI／DWM glass はこの窓では機能しないため不使用。当たり判定（クリック透過）は `WS_EX_TRANSPARENT` トグル単独が支配し、視覚機構と分離する。

## 概要（何を作ったか）

- 実装内容:
  - `WS_EX_NOREDIRECTIONBITMAP | WS_EX_TOPMOST | WS_EX_TRANSPARENT` の透過トップモスト窓（初期＝クリック透過 ON）。
  - DComp パイプライン（D3D11→DXGI／`IDCompositionSurface`→Visual／Target→Commit）＋ Direct2D で透明 Clear（α=0）＋窓中心・半径 200px の不透明円（α=1）を描画。クリック（`WM_LBUTTONDOWN`）で円の色をトグルして DComp 再描画。
  - 別 `std::thread` が 16ms 周期でカーソルを監視し、αマスク（窓中心・半径 200px の円。描画円と同一領域・物理座標）判定で、状態変化フレームのみ UI スレッドへ起床通知（`event_listener`・tokio 不使用）。
  - UI スレッド（`spawn_local`）が差分時のみ `WS_EX_TRANSPARENT` を加除（`SetWindowLongPtr`＋`SetWindowPos(SWP_FRAMECHANGED)`）。`WS_EX_NOREDIRECTIONBITMAP`／`WS_EX_TOPMOST` は保持。
- 実行法: `cargo run -p pilot --example pilot-clickthrough-alpha-toggle`
- 葉ノード隔離: コードは `crates/pilot/examples/pilot-clickthrough-alpha-toggle/` のみ。production への inbound 依存なし・新規依存なし。いつでも安全に捨てられる。

## 検証結果

- 判定: go / 違う / 直す   ← **未確定（人間が確定する・タスク 6.2／`REPORT.md` 総合判定）**
- 観測の要点（事実・詳細は `REPORT.md`）:
  - T1 ✅（`ShowWindow` 欠落の実装欠陥を修正後、円が表示）/ T3 ✅（円内クリック受領）/ T4 ✅（ON↔OFF 切替発火）。
  - **T2 ❌・T6 ❌（核心）**: `WS_EX_NOREDIRECTIONBITMAP`（DComp）窓では、`WS_EX_TRANSPARENT` を立てても（作成時・動的トグルとも）**クリックが背面別プロセスへ透過せず窓が受領し続ける**。ex_style に TRANSPARENT ビットが立つことはログで確認済み・座標交絡も排除済み。
- 暫定の学び（人間の go 判定の材料・判定そのものは保留）:
  - 「DComp 描画を捨てず `WS_EX_TRANSPARENT` **単独**トグルで別プロセス透過」という当初仮説は**この構成では不成立**。
  - 権威ソース調査でも `WS_EX_TRANSPARENT` は DWM/合成窓で効かず、定石は `WS_EX_LAYERED|WS_EX_TRANSPARENT`。しかし `WS_EX_LAYERED` は `WS_EX_NOREDIRECTIONBITMAP` の**代替＝排他**で DComp 描画と衝突しうる（R2.3 が layered 不付与を課した背景と一致）。同一状況の MS Q&A も**未解決**。
  - ⇒ 既存 **ULW ルート**（alpha-0 自動透過）の妥当性が相対的に補強された。次手候補（A〜D）は `REPORT.md` 参照（いずれも開発者の承認が必要）。
- 日付: 2026-06-30
