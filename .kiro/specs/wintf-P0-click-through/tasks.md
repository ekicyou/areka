# Implementation Plan

## 実装タスク

- [ ] 1. WndProc Layer クリックスルー機能実装
- [ ] 1.1 (P) ヒットテスト結果に基づく分岐処理を実装
  - ヒットテスト結果が None（透明領域）の場合に HTTRANSPARENT (-1) を返却
  - ヒットテスト結果が Some（エンティティヒット）の場合に HTCLIENT (1) を返却
  - 両方の結果をキャッシュに格納し、再問い合わせ時に正しく返却
  - _Requirements: 1.1, 1.2, 1.5_

- [ ] 1.2 (P) ドラッグ操作中の安全性ガードを実装
  - ドラッグ状態（Preparing/JustStarted/Dragging）確認ロジックを追加
  - ドラッグ中は透明領域でも HTCLIENT を強制返却
  - ドラッグ操作の継続性を保証
  - _Requirements: 1.1_

- [ ] 1.3 (P) HTTRANSPARENT 定数の有効化とコメント更新
  - HTTRANSPARENT 定数から #[allow(dead_code)] アノテーションを除去
  - 既存コメント「HTTRANSPARENT を返すとマウスイベントがブロックされてしまう」を更新
  - WM_MOUSELEAVE ハンドラ実装済みによる問題解決を記載
  - _Requirements: 1.3, 3.2_

- [ ] 2. ECS Layer 透明領域判定機能実装 (P)
- [ ] 2.1 (P) Opacity/Brushes α値判定ロジックを実装
  - HitTestMode::Bounds 分岐で Opacity と Brushes.foreground.a の積を計算
  - 合成α値 < 128/255 (≈0.502) の場合に false（透明領域）を返却
  - AlphaMask の ALPHA_THRESHOLD と同一基準を採用
  - Opacity 未設定時は 1.0、Brushes.foreground が Inherit 時は親継承値または DEFAULT_FOREGROUND を使用
  - _Requirements: 1.6_

- [ ] 3. 手動テスト環境構築 (P)
- [ ] 3.1 (P) クリックスルーテストシーンを追加
  - HitTest::none() を持つクリックスルー領域を配置（黄色半透明矩形）
  - HitTest::bounds() を持つ通常領域を並べて配置（シアン半透明矩形）
  - 各領域にラベルを追加して視覚的に区別可能に
  - 既存デモアプリケーションに横並びレイアウトで追加
  - _Requirements: 4.4_

- [ ] 4. 自動テスト実装
- [ ] 4.1 WndProc Layer 分岐ロジックのテストを実装
  - ヒットテスト結果 None → HTTRANSPARENT 返却の検証
  - ヒットテスト結果 Some → HTCLIENT 返却の検証
  - ドラッグ状態が非 Idle → HTCLIENT 強制返却の検証
  - HTTRANSPARENT/HTCLIENT 両方のキャッシュ格納・取得の検証
  - _Requirements: 4.1, 4.2_

- [ ] 4.2 ECS Layer α値判定ロジックのテストを実装
  - Opacity(0.502) * foreground.a=1.0 → HTCLIENT 判定（境界値以上）
  - Opacity(0.501) * foreground.a=1.0 → 透明領域判定（境界値未満）
  - Opacity(0.4) * foreground.a=1.0 → 透明領域判定
  - Opacity(1.0) * foreground.a=0.4 → 透明領域判定（合成後透明）
  - _Requirements: 4.3_
