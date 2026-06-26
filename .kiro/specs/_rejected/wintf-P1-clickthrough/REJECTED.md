# 却下記録: wintf-P1-clickthrough

- **却下日**: 2026-06-26
- **却下理由**: 完了済み仕様に超越されたため。
  - クリック透過は `completed/wintf-P0-click-through` ＋ `completed/event-hit-test-alpha-mask` が **UpdateLayeredWindow（`ULW_ALPHA`/`AC_SRC_ALPHA`）方式で実装完了済み**。
  - 本仕様は要件ドラフト0.1のまま生成・承認されておらず（`ready_for_implementation: false`）、内容は **旧アプローチ（DirectComposition＋透過マップ＋`WS_EX_LAYERED`/`WS_EX_TRANSPARENT`）** を前提としていた。この方式は `_rejected/wintf-P0-click-through-rgn` 系の検討を経て採られず、ULW方式へ移行済み。
- **歩む道が無い**: 機能は実現済み、前提アプローチは不採用。新戦略（互換ベースウェア）でも追加要件は生じない。
- 関連: `doc/COMPAT_ARCHITECTURE.md` §5, `doc/ROADMAP.md`「解決済み基盤資産」
