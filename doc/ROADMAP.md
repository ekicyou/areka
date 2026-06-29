# areka ロードマップ — 移動済み（ポインタ）

> **このファイルはポインタ stub です。** ロードマップの**正本は [`.kiro/steering/roadmap.md`](../.kiro/steering/roadmap.md)** へ移動しました（2026-06-28・cc-sdd / kiro 標準テンプレート準拠）。

## なぜ移動したか

ロードマップを kiro ツールチェーンが標準パスで自動参照できるよう、`.kiro/steering/roadmap.md` を正本としました。

- **ツール自動発見**: `/kiro-discovery` 再入・`/kiro-spec-batch` は `.kiro/steering/roadmap.md` を参照する。`doc/ROADMAP.md` に置くと skill が焦点を当てられず、ドリフト（記載 vs 実態の乖離）が発生していた。
- **コンテキスト最小化との両立**: 正本 roadmap.md は `inclusion: manual` ゆえ毎セッションの自動ロードはされない（`focus.md` の lean ポインタだけが常駐）。kiro-P0-roadmap-management のコンテキスト最小化設計を維持したまま標準パスへ寄せた。

## 参照先

- 📍 ロードマップ正本: [`.kiro/steering/roadmap.md`](../.kiro/steering/roadmap.md)
- 📍 運用ガイド（参照/更新タイミング・配置ルール・件数集計）: [`.kiro/steering/focus.md`](../.kiro/steering/focus.md)
- 📍 設計判断の正本: [`doc/COMPAT_ARCHITECTURE.md`](COMPAT_ARCHITECTURE.md)

> 旧 `doc/ROADMAP.md`（v2.0・2026-06-26 畳み込み版の richな track 表・mermaid・畳み込みログ）の本文は git 履歴に保存されています。
