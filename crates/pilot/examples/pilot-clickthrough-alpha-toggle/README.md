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

- 判定: **go**（開発者承認・2026-07-01）
- 結論（核心 Unknown は**肯定的に決着**・詳細は `REPORT.md`）:
  - **`WS_EX_TRANSPARENT` 動的トグルで、DComp 描画を捨てないまま別プロセスへのクリック透過が成立した**（円外＝透過／円内＝受領／見える円と反応領域の座標一致を人間が実機確認："一致しました"）。T1/T2/T3/T4 ✅、T6 機構確認済（背面ブラウザ明示テストのみ残）、T5 ✅暫定、T7 ✅部分、T8 未検証。
- 成立に必要だった**2つの必須条件**（当初仕様に無かった＝重要知見）:
  1. **`WS_EX_LAYERED` を"同伴フラグ"として立てる**（`UpdateLayeredWindow`/`SetLayeredWindowAttributes` は呼ばない＝レイヤード描画には使わない）。無いと `WS_EX_TRANSPARENT` 単独では DComp 窓でマウス透過が効かない。DComp 描画はフラグ併設でも消えず共存。
  2. **枠なし窓（`WS_POPUP`＝client==window）**。DComp は窓原点から合成するため、非クライアント（タイトルバー）があると見える円と反応領域が縦にずれる。
- 学び:
  - 当初仮説「DComp を捨てず `WS_EX_TRANSPARENT` **単独**で透過」は不正確——正しくは「`WS_EX_LAYERED` フラグ併設（レイヤード描画は非使用）＋枠なし」。R2.3「layered 無し」制約は撤回し本坑へ反映。
  - コピペ donor にはしない。本坑は上記レシピをクリーンに掘り直す。
- 日付: 2026-07-01
