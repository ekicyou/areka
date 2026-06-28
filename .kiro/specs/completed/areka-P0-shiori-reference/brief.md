# Brief: areka-P0-shiori-reference

> 上位設計の正本: `doc/COMPAT_ARCHITECTURE.md` §5。隣接（完成）: `areka-P0-shiori-com`（IShiori/IShioriHost ABI）。

## Problem
`areka-P0-shiori-com` で `IShiori`/`IShioriHost` ABI は定義・実装したが、脳（COM-SHIORI）の実装は**すべてテスト用モック（`#[cfg(test)]`）のみ**。非テストで実走可能な「正解見本」となるリファレンス脳が無く、(1) ABI が実アプリで動く証明、(2) 下流（`areka-P0-shiori-host-32` の DLL ホスト／`areka-P0-reference-ghost` の pasta）が満たすべき `IShiori` 契約の参照点、が欠落している。

## Current State
`shiori-abi` クレート（`IShiori`/`IShioriHost`/`ShioriExt`）完成。areka 側に `IShioriHost` sink ＋ in-proc アクティベーション最小受け皿（`ShioriSession`）あり。だが**製品コードに `IShiori` 実装（脳）は無く**、areka main から脳を挿して走らせる実走経路も無い。

## Desired Outcome
最小・非テストの**リファレンス COM-SHIORI（native 脳）**が `IShiori` を実装し、areka 本体から in-proc アクティベーションで挿して、request→応答／遅延→Complete／Raise の各経路が実アプリ上で動く。これが ABI の実走証明であり、`host-32`/`pasta` が `IShiori` を実装する際の「正解見本（リファレンス）」になる。

## Approach
`shiori-abi` 公開 API（`ShioriExt` / `#[implement(IShiori)]`）で最小の native 脳を**非テスト**として実装。content は**不透明のまま固定／エコー応答**（正準 json-rpc プロトコルの確定は別仕様 `areka-P0-shiori-protocol` へ委譲）。areka main に「リファレンス脳を `activate` して数往復ドライブする」最小デモ経路を足す。即時応答を基本に、遅延（`SHIORI_S_PENDING`＋`Complete`）と `Raise` も最小実演する。

## Scope
- **In**: 最小 native リファレンス脳（非テスト `#[implement(IShiori)]`・固定/エコー応答）、areka main からの実走デモ経路（`activate`→`request`→応答／遅延→`Complete`／`Raise` の疎通）、リファレンスとしての doc 化
- **Out**: 正準 json-rpc content プロトコルの定義（→ `areka-P0-shiori-protocol`）、32bit DLL ホスティング（→ `areka-P0-shiori-host-32`）、pasta 旗艦 native 脳（→ `areka-P0-reference-ghost`, M2）、さくらスクリプト解釈、DLL 適合（conformance）テストキット（host-32 実装過程で決定）

## Boundary Candidates
- リファレンス脳の `IShiori` 実装面（固定/エコー応答ロジック・遅延/Raise 最小実演）
- areka main の実走デモ配線（脳挿入・数往復ドライブ・後始末 unload）

## Out of Boundary
- content プロトコル具体形、DLL ホスト、pasta 旗艦、conformance キット、さくらスクリプト

## Upstream / Downstream
- **Upstream**: `areka-P0-shiori-com`（`IShiori`/`IShioriHost` ABI・実装済）
- **Downstream**: `areka-P0-shiori-host-32`（DLL 契約境界は host-32 実装過程で本リファレンスを見本に決定）、`areka-P0-reference-ghost`（pasta native、本リファレンスを起点に拡張）、`areka-P0-shiori-protocol`（リファレンス脳が将来 json-rpc を採用）

## Existing Spec Touchpoints
- **Adjacent**: `areka-P0-shiori-com`（ABI・完成）、`areka-P0-shiori-host-32`（DLL ホスト・brief のみ）、`areka-P0-reference-ghost`（M2 旗艦）

## Constraints
x64/CPU ネイティブ前提（x86 除外・shiori-com R5-3 踏襲）。content は不透明 HSTRING（R1-6）。流動契約（D7）。windows-core 0.62.2、edition 2024。
