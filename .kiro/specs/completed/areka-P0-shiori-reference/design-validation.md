# 設計検証レポート: areka-P0-shiori-reference

> 検証種別: 技術設計品質レビュー（実装移行可否ゲート）
> 対象: `.kiro/specs/areka-P0-shiori-reference/design.md`
> 言語: ja（spec.json 準拠）
> 実コード照合: 読み取り専用で `crates/shiori-abi/src/`・`crates/areka/src/`（shiori_session.rs / shiori_host.rs / main.rs / *_e2e_tests.rs）を確認済み

## 設計レビューサマリ

本設計は、`areka-P0-shiori-com` で確立した `IShiori`/`IShioriHost` ABI に対し、テストモック（`MockBrain`/`DeferringBrain`/`StatefulBrain`）の和集合を単一の製品コード脳へ昇格し、`shiori_create`（純粋C コンストラクタ）とフラグゲート付きデモドライバで実走させる、責務分離の明確な拡張設計である。上流 ABI シグネチャ・`CorrelationTokenAllocator::next()`・`ShioriExt`・areka 側受け皿（`ShioriSession`/`ShioriHostSink`）・昇格元モックの挙動はすべて実コードと一致し、`shiori_create`/`reference_brain`/`shiori_demo`/`run_demo` が未存在の真の新規であることも確認した。9 要件すべてに Requirements Traceability の対応があり、実装経路は明瞭でリスクは許容範囲。

## クリティカルイシュー（最大 3 件）

本検証では**実装移行を阻むクリティカルイシューは検出されなかった**。以下は設計ディスカッションで確認するに足る軽微な観点であり、いずれも GO を妨げない（参考: 設計反復で解消可能）。

### 観点 A（軽微・実装詳細）: `#[no_mangle]` / `extern "C"` の Rust 2024 表記
- **内容**: §Components → shiori_create の Service Interface が `#[no_mangle] pub extern "C" fn shiori_create(...)` と記す。本ワークスペースは edition 2024（ルート `Cargo.toml` `edition = "2024"`・`crates/areka/Cargo.toml` `edition.workspace = true`）であり、2024 では `#[unsafe(no_mangle)]` および `unsafe extern "C"` 形が要求される。
- **影響**: 設計上の概念（生成入口の一意性・`HRESULT shiori_create(IShiori** out)` 形）は正しく、表記は実装時に自明に解消できる。見本性（下流が踏襲する形）を損なわないよう実装時に edition 2024 形を採ることだけ確認したい。
- **トレーサビリティ**: 要件 9.2 / 9.6。
- **エビデンス**: design.md §Components and Interfaces → shiori_create → Service Interface（コードブロック）。

### 観点 B（軽微・記述の精緻化）: `out` 型と `IShiori**` 写像の安全境界
- **内容**: シグネチャは `*mut *mut core::ffi::c_void` で受け「`IShiori` へ写す」と注記するが、move-out（refcount 1・失敗時 out 未書込）の writes-on-success 規律と、`out` 非 NULL 前提の扱い（Preconditions「有効な書込先」）の境界記述が概念レベルに留まる。
- **影響**: 既存テストモックの `core::ptr::write(out_response, ..)` 技法と同型で実装可能なため実装阻害はない。下流見本としての正確性を担保するうえで、失敗時 out 未書込の不変条件をテスト（§Testing Strategy に既記載「失敗時 out 未書込」）で固定すれば十分。
- **トレーサビリティ**: 要件 9.3 / 9.4。
- **エビデンス**: design.md §Components → shiori_create → Service Interface / Postconditions、§Testing Strategy → Unit Tests。

## 設計の強み

1. **既存資産との整合と非循環依存の明確さ**: 上流 ABI（不変採用）・areka 受け皿（`ShioriSession` 単一 in-flight／`poll_completions` drain／`expire_if_elapsed` 決定的タイムアウト）・vtable 直呼びイディオム・`from_raw_borrowed`+`cloned()` の host 保持を、実コードと完全一致する形で踏襲している。依存方向（areka→脳→host・下向きのみ）と Revalidation Triggers（流動契約 D7 追従）が明示され、境界侵食リスクが低い。
2. **スコープ規律と content 不透明性の徹底**: content を不透明 HSTRING のまま固定／エコーに限定し、正準プロトコル（`areka-P0-shiori-protocol`／`doc/shiori/fragments/`）の語彙を参照・複製しない方針が Non-Goals・Boundary Commitments・要件 8 と一貫。テストモックの和集合昇格という「新規ロジックなし」の方針が、見本性とリスク低減を両立している。

## 最終評価

- **判定: GO**
- **根拠**: 既存アーキテクチャ整合・要件網羅（9 要件全トレース）・実装経路明瞭で、設計主張が実コードと一致。検出された 2 観点はいずれも実装詳細／記述精緻化レベルでクリティカルではない。
- **次ステップ**: 設計ディスカッション（kiro-design-discussion）で観点 A（edition 2024 の `unsafe` 表記）・観点 B（move-out 安全境界の記述）を確認のうえ、`/kiro-spec-tasks areka-P0-shiori-reference` でタスク生成へ進む。
