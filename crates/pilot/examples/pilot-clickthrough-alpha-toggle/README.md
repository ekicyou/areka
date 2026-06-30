# 先進坑: pilot-clickthrough-alpha-toggle

> この README は先進坑の**一次記録（正本）**である。本坑 spec の design はここの検証結果を参照し、
> 同じ結果を二重化しない。（3 幕の各欄は最終タスク 5.1 で整備し、検証結果は 5.2 で人間が確定する）

## 動機（なぜ掘るか）

- 対応する本坑 spec: `wintf-clickthrough-alpha-toggle`   ← 先進坑⟷本坑の traceability
- 確認したい方向 / 実現可能性 / 手順:
  - `WS_EX_LAYERED` 無し・`WS_EX_TRANSPARENT` 単独で、別プロセス（背面アプリ）へクリックが透過するか。
  - DComp（GPU 合成）描画を捨てずに別プロセス透過を成立させる第 4 の手の実現可能性。

## 概要（何を作ったか）

- 実装内容:
  - 透過トップモスト窓＋中央の不透明円。別 `std::thread` が 16ms 周期でカーソルを監視し、
    αマスク（窓中心・半径 200px の円）判定で `WS_EX_TRANSPARENT` を動的に付け外しする。
- 実行法: `cargo run -p pilot --example pilot-clickthrough-alpha-toggle`

## 検証結果

- 判定: go / 違う / 直す   ← いずれかを残す（人間が確定）
- 学び:
  - （得られた知見。本坑をクリーンに掘り直すための材料。コピペ donor にはしない）
- 日付: YYYY-MM-DD
