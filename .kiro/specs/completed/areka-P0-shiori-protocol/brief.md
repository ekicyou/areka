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
- **唯一の正本対応表**（意味名 ⇔ `Reference0/1/2…`）を本仕様が所有。意味名と `ReferenceN` は**この1枚のスキーマから機械投影される2つの表示**であり、正本は常に1つ。
- request/応答/遅延（`SHIORI_S_PENDING`）/Raise（通知）の json-rpc `id`/`result`/`error`/notification マッピング。
リファレンス脳・host-32・pasta が同一プロトコル・同一対応表を典拠に話せる基準になる。

## 設計モデルの核心（訂正済みの前提）
- **SHIORI/3.0 は wire 上の追加ヘッダを禁じない。** 本物の DLL は知らないキーを無視する。ゆえに `Reference0` の隣に意味名（`X` 等）を**併載しても害が無い**（改名不可なのは「`Reference0` を**消す/別名へ置換する**」ことだけ。**増やす**のは可）。
- したがって意味名は「areka 内部限定」ではなく、**全 wire に投影しうる正準語彙**。
- **正本＝対応表スキーマ1枚。** `Reference0…` と意味名はその投影（rendering）にすぎず、契約は分散しない。

## Approach
json-rpc 2.0 を正準封筒に採用し、**メソッド名＝ukadoc イベント ID、params＝意味名フィールド（named）**とする。ukadoc を正典に全イベントを棚卸しし、イベントごとに「ReferenceN→意味名→型→説明、応答の意味」を**唯一の対応表**へ落とす（ukadoc 沈黙箇所は COMPAT §2 に従い areka 裁量＋表へ明記）。意味名を canonical、`ReferenceN` を**そのスキーマから生成される互換エイリアス**と位置づける。遅延＝`id` 先行返却＋後続 `Complete` で `result` 配送、Raise＝notification（`id` なし）にマップ。さくらスクリプト本文・SAORI 引数等の content は json-rpc の文字列フィールドに**不透明に載せる**（解釈は別仕様）。protocol version はリリース時凍結方針（D7）に従い、高レート通信余地（D6）を阻害しない。

## 要件の種（Requirement Seeds → 要件フェーズで EARS 化）
> 以下は WHAT（契約として保証すべきこと）。`/kiro-start` の要件生成で requirements.md（EARS）へ昇格させる。本 brief は継続のための種置き場。

- **R-種1 正準語彙**: 正準 content の params は**意味名（named）**で表現される。
- **R-種2 単一正本**: フィールド意味は**唯一の正本対応表**が定義し、本仕様が所有する。
- **R-種3 エイリアスの従属**: `ReferenceN` は対応表から**導出される互換エイリアス**であり、独立した権威を持たない。意味名と食い違う場合は**意味名が勝つ**。
- **R-種4 レガシー wire 放出**: host-32 はレガシー DLL wire へ `ReferenceN` を**必ず**吐く（旧 DLL が要求）。意味名エイリアスは**任意で併載でき、per-DLL で切替可能**（暴れる DLL 用キルスイッチ）。
- **R-種5 予約名非衝突**: 意味名は予約 SHIORI ヘッダ（`ID`/`Sender`/`Charset`/`SecurityLevel`/`Status`/`Reference*` 等）と衝突しない。
- **R-種6 沈黙ルール追跡**: ukadoc が沈黙する裁定（追加ヘッダの可否・意味割り当て等）は**対応表に記録**する（COMPAT §2）。
- **R-種7 封筒マッピング**: request/応答/遅延/Raise が json-rpc `id`/`result`/`error`/notification へ規定どおり対応する。

### 未決の要件ノブ（要件フェーズで決定）
- **Q1 native wire のエイリアス併載**: COM-SHIORI（native 脳向け）wire に `ReferenceN` エイリアスも載せるか／意味名のみ（pristine）か。**契約面の WHAT**なので設計ノブではなく要件として裁定する。暫定推奨＝**native は意味名のみ**、レガシー wire のみ併載（R-種4）。

## Scope
- **In**:
  - **ukadoc 全イベントカタログ**と GET/NOTIFY 分類
  - 各イベントの**フィールドスキーマ**（意味名・型・必須/任意・応答意味）
  - **唯一の正本対応表**（意味名 ⇔ `Reference0/1/2…`、canonical=意味名 / alias=ReferenceN）
  - 正準 content プロトコル（json-rpc 2.0）の封筒・`id`/`result`/`error`/notification マッピング、相関トークン↔`id` 対応
  - エンコーディング/charset 規約（content 文字列の扱い）、沈黙ルール適用箇所の明記
  - バージョニング方針の宣言（D7 整合）
- **Out**:
  - COM ABI（`IShiori` 面・→ `areka-P0-shiori-com`）
  - さくらスクリプト/SAORI 本文の解釈・実行（→ sakura-script ほか。content は不透明）
  - レガシーテキスト⇄正準モデルの**翻訳実装**（→ `areka-P0-shiori-host-32`。**ただし翻訳が従う対応表＝契約は本仕様が定義**）
  - トランスポート（HSTRING 取り回しは shiori-com）

## 設計に降ろす残り（HOW・要件ではない）
- 「1スキーマ→2表示」を**どの機構で投影するか**（対応表レジストリ／コード生成／実行時変換）。
- キルスイッチ設定の置き場・データ構造、host-32 翻訳モジュールの構造。
- json-rpc 封筒の具体的な実装表現。

## Boundary Candidates
- イベントカタログ（GET/NOTIFY 集合の確定）
- フィールドスキーマ＋意味名⇔`ReferenceN` 対応表（規範的契約・唯一の正本）
- json-rpc 封筒（メソッド/`id`/`result`/`error`/notification）と相関トークン↔`id` 対応
- 応答側意味（Value/さくらスクリプト・各種応答ヘッダの意味割り当て）

## Out of Boundary
- COM ABI、トランスポート、さくらスクリプト/SAORI 解釈、レガシー翻訳の**実装コード**

## Upstream / Downstream
- **Upstream**: `areka-P0-shiori-com`（content を運ぶ ABI／D5 をここで着地）
- **Downstream**:
  - `areka-P0-shiori-reference`（リファレンス脳がこの語彙・対応表で話す）
  - `areka-P0-shiori-host-32`（レガシー SHIORI/3.0 ⇄ 正準モデルの翻訳を、本仕様の対応表に従って実装。R-種4 の放出方針を実装）
  - `areka-P0-reference-ghost` / pasta native 脳（native 経路でこの語彙を話す）
  - `areka-P0-seriko-runtime` / `areka-P0-sakura-script`（イベント発火・応答 content の消費側）

## Existing Spec Touchpoints
- **Adjacent**: `areka-P0-shiori-com`（設計判断 D5 をここで着地）、`areka-P0-shiori-reference`（最初の話者）、`areka-P0-shiori-host-32`（対応表の消費者）

## Constraints
- ukadoc を互換契約の正典とする（SSP 実挙動は二次参照、判断は対応表へ明記）。
- レガシー DLL wire の `Reference0/1/2…` は**消去/改名不可**（旧 DLL が要求）。ただし SHIORI/3.0 は追加ヘッダを禁じないため、意味名エイリアスの**併載は可**。正本はあくまで対応表スキーマ1枚、wire 上の各名はその投影。
- json-rpc 2.0 準拠。content は HSTRING/UTF-16 で運ぶ（shiori-com）。流動契約（D7）。高レート通信余地（D6）を阻害しない設計。
