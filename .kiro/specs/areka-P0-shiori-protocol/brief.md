# Brief: areka-P0-shiori-protocol

> 上位設計の正本: `doc/COMPAT_ARCHITECTURE.md` §5。隣接（完成）: `areka-P0-shiori-com`（content を不透明のまま運ぶ ABI）。

## Problem
`areka-P0-shiori-com` は `IShiori` 境界の content を「単一正準プロトコル（json-rpc 採用判断）」とする不変条件（R1-6）を置いたが、**具体形は設計判断 D5 として不透明のまま先送り**した。脳と areka が実際に意味のある会話をするには、正準 content プロトコル（json-rpc 2.0 の具体メソッド面・`id`/`result`/`error` マッピング）の確定が要る。

## Current State
shiori-com の content は opaque HSTRING。`research.md` §7 D5 に「json-rpc 2.0 採用候補、即時/遅延/失敗・相関トークンが `id`/`result`/`error` に対応」「json-rpc そのものにする案」と記録のみ。具体的なメソッド語彙・スキーマ・バージョニングは未定義。

## Desired Outcome
`IShiori` 境界を流れる正準 content プロトコル（json-rpc 2.0 ベース）の具体形が定義され、request/応答/遅延（`SHIORI_S_PENDING`）/Raise（通知）が json-rpc の `id`/`result`/`error`/notification 構造に明確にマップされる。リファレンス脳・host-32・pasta が同一プロトコルを話せる基準になる。

## Approach
json-rpc 2.0 を正準プロトコルに採用。request=メソッド呼び出し、遅延=`id` 先行返却＋後続 `Complete` で `result` 配送、Raise=notification（`id` なし）にマップ。相関トークン↔`id` 対応を定義。最小メソッド語彙（古典 SHIORI の `hrequest` 相当＋必要イベント面）を定義。さくらスクリプト本文は json-rpc の文字列フィールドに**不透明に載せる**（解釈は別仕様）。protocol version の扱いはリリース時凍結方針（D7）に従い、高レート通信余地（D6）を阻害しない。

## Scope
- **In**: 正準 content プロトコル（json-rpc 2.0）の具体メソッド面、`id`/`result`/`error`/notification マッピング、相関トークン↔`id` 対応、最小メソッド語彙（SHIORI イベント面）、エンコーディング規約
- **Out**: COM ABI（`IShiori` 面・→ `areka-P0-shiori-com`）、さくらスクリプト解釈（→ sakura-script）、DLL レガシーテキスト↔json-rpc 翻訳（→ `areka-P0-shiori-host-32`）、トランスポート（HSTRING 取り回しは shiori-com）

## Boundary Candidates
- json-rpc 封筒（メソッド/`id`/`result`/`error`/notification）
- 相関トークン↔`id` 対応
- メソッド語彙（SHIORI イベント面の最小セット）

## Out of Boundary
- COM ABI、さくらスクリプト、DLL 翻訳、トランスポート

## Upstream / Downstream
- **Upstream**: `areka-P0-shiori-com`（content を運ぶ ABI）
- **Downstream**: `areka-P0-shiori-reference`（リファレンス脳が採用）、`areka-P0-shiori-host-32`（レガシー↔正準 翻訳の対象）、`areka-P0-reference-ghost`（pasta が話す）

## Existing Spec Touchpoints
- **Adjacent**: `areka-P0-shiori-com`（設計判断 D5 をここで着地）、`areka-P0-shiori-reference`（最初の話者）

## Constraints
json-rpc 2.0 準拠。content は HSTRING/UTF-16 で運ぶ（shiori-com）。流動契約（D7）。高レート通信余地（D6）を阻害しない設計。
