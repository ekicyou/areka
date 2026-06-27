# Brief: areka-P0-shiori-protocol

> 上位設計の正本: `doc/COMPAT_ARCHITECTURE.md` §5。隣接（完成）: `areka-P0-shiori-com`（content を不透明のまま運ぶ ABI）。
> 互換契約の典拠: ukadoc（正典）。沈黙/曖昧箇所は COMPAT §2 の沈黙ルールに従い areka 裁量＋対応表へ明記。

## Problem
`areka-P0-shiori-com` は `IShiori` 境界の content を「単一正準プロトコル（json-rpc 採用判断）」とする不変条件（R1-6）を置いたが、**具体形は設計判断 D5 として不透明のまま先送り**した。さらに伺かの SHIORI/3.0 では各イベントの引数が `Reference0/1/2…` の**番号フィールド**で運ばれ、意味がスキーマとして固定されていない。脳と areka が意味のある会話をするには、(a) どの ukadoc イベントを扱うか、(b) 各イベントのフィールドが何を意味するか、(c) 番号フィールド↔意味名の対応、を**単一の契約**として確定する必要がある。契約を host-32 や reference へ分散させると正本が散る。

## Current State
shiori-com の content は opaque HSTRING。`research.md` §7 D5 に「json-rpc 2.0 採用候補、即時/遅延/失敗・相関トークンが `id`/`result`/`error` に対応」と記録のみ。具体的なメソッド語彙・イベントカタログ・フィールドスキーマ・`Reference0/1/2…` の意味割り当て・バージョニングは未定義。host-32 brief は「DLL 境界契約は実装過程でリファレンスを見本に決定」とあるが、**規範的な対応表の正本所有者が未確定**。

## Desired Outcome
`IShiori` 境界を流れる**正準 content プロトコル（json-rpc 2.0 ベース）の具体形**と、**areka 内部の正準イベントモデル（意味のある named フィールド）**が定義される。具体的には:
- **ukadoc 全イベントカタログ**（GET 系＝要求/応答を期待、NOTIFY 系＝通知のみ、の分類込み）。
- 各イベントの**フィールドスキーマ**: 意味名・型・必須/任意・ukadoc 上の `ReferenceN` 位置・応答側の意味。
- **`Reference0/1/2…` ⇄ 意味名の規範的対応表**を本仕様が**正本として所有**。レガシー(ukadoc SHIORI/3.0)と native(json-rpc named params)の双方が、この単一正準モデルへ写像する。
- request/応答/遅延（`SHIORI_S_PENDING`）/Raise（通知）の json-rpc `id`/`result`/`error`/notification マッピング。
リファレンス脳・host-32・pasta が同一プロトコル・同一対応表を典拠に話せる基準になる。

## Approach
json-rpc 2.0 を正準封筒に採用し、**メソッド名＝ukadoc イベント ID、params＝意味名フィールド（named）**とする。番号フィールド `ReferenceN` は areka 内部には持ち込まず、対応表を介して named params へ正規化する。ukadoc を正典に全イベントを棚卸しし、イベントごとに「ReferenceN→意味名→型→説明、応答の意味」を表に落とす（ukadoc が沈黙する箇所は COMPAT §2 に従い areka 裁量＋表へ明記）。遅延＝`id` 先行返却＋後続 `Complete` で `result` 配送、Raise＝notification（`id` なし）にマップ。さくらスクリプト本文・SAORI 引数等の content は json-rpc の文字列フィールドに**不透明に載せる**（解釈は別仕様）。protocol version はリリース時凍結方針（D7）に従い、高レート通信余地（D6）を阻害しない。

## Scope
- **In**:
  - **ukadoc 全イベントカタログ**と GET/NOTIFY 分類
  - 各イベントの**フィールドスキーマ**（意味名・型・必須/任意・応答意味）
  - **`Reference0/1/2…` ⇄ 意味名の規範的対応表**（本仕様が正本所有）
  - 正準 content プロトコル（json-rpc 2.0）の封筒・`id`/`result`/`error`/notification マッピング、相関トークン↔`id` 対応
  - エンコーディング/charset 規約（content 文字列の扱い）、沈黙ルール適用箇所の明記
  - バージョニング方針の宣言（D7 整合）
- **Out**:
  - COM ABI（`IShiori` 面・→ `areka-P0-shiori-com`）
  - さくらスクリプト/SAORI 本文の解釈・実行（→ sakura-script ほか。content は不透明）
  - レガシーテキスト⇄正準モデルの**翻訳実装**（→ `areka-P0-shiori-host-32`。**ただし翻訳が従う対応表＝契約は本仕様が定義**）
  - トランスポート（HSTRING 取り回しは shiori-com）

## Boundary Candidates
- イベントカタログ（GET/NOTIFY 集合の確定）
- フィールドスキーマ＋`ReferenceN`↔意味名 対応表（規範的契約）
- json-rpc 封筒（メソッド/`id`/`result`/`error`/notification）と相関トークン↔`id` 対応
- 応答側意味（Value/さくらスクリプト・各種応答ヘッダの意味割り当て）

## Out of Boundary
- COM ABI、トランスポート、さくらスクリプト/SAORI 解釈、レガシー翻訳の**実装コード**

## Upstream / Downstream
- **Upstream**: `areka-P0-shiori-com`（content を運ぶ ABI／D5 をここで着地）
- **Downstream**:
  - `areka-P0-shiori-reference`（リファレンス脳がこの語彙・対応表で話す）
  - `areka-P0-shiori-host-32`（レガシー SHIORI/3.0 ⇄ 正準モデルの翻訳を、本仕様の対応表に従って実装）
  - `areka-P0-reference-ghost` / pasta native 脳（native 経路でこの語彙を話す）
  - `areka-P0-seriko-runtime` / `areka-P0-sakura-script`（イベント発火・応答 content の消費側）

## Existing Spec Touchpoints
- **Adjacent**: `areka-P0-shiori-com`（設計判断 D5 をここで着地）、`areka-P0-shiori-reference`（最初の話者）、`areka-P0-shiori-host-32`（対応表の消費者）

## Constraints
- ukadoc を互換契約の正典とする（SSP 実挙動は二次参照、判断は対応表へ明記）。
- wire 上の `Reference0/1/2…` は ukadoc 固定であり改名不可。意味名は **areka 内部の正準モデル**として持ち、対応表で橋渡しする。
- json-rpc 2.0 準拠。content は HSTRING/UTF-16 で運ぶ（shiori-com）。流動契約（D7）。高レート通信余地（D6）を阻害しない設計。
