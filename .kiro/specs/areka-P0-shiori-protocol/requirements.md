# Requirements Document

## Introduction

本仕様は、`IShiori`（COM）境界を流れる **正準 content プロトコル（json-rpc 2.0 ベース）の具体形** と、areka 内部の **正準イベントモデル（意味のある named フィールド）** を、単一の規範的契約として確定する。`areka-P0-shiori-com`（完了済み）は R1-6 で「`IShiori` 境界の content は単一正準プロトコルで表現する」という不変条件を置き、その具体形を設計判断 D5 として先送りした。本仕様はその D5 を着地させる。

確定する内容は、(a) ukadoc を正典とする全イベントカタログ（GET 系／NOTIFY 系の分類込み）、(b) 各イベントのフィールドスキーマ（意味名・型・必須/任意・ukadoc 上の `ReferenceN` 位置・応答側の意味）、(c) 意味名 ⇔ `Reference0/1/2…` を結ぶ **唯一の正本対応表**、(d) request／応答／遅延（`SHIORI_S_PENDING`）／Raise（通知）を json-rpc の `id`／`result`／`error`／notification へ写す封筒マッピングである。

正本はあくまで **対応表スキーマ1枚** であり、意味名と `ReferenceN` はそのスキーマから機械投影される2つの表示にすぎない。これにより、リファレンス脳・`areka-P0-shiori-host-32`・pasta（native 脳）が同一プロトコル・同一対応表を典拠に会話できる基準を与える。

上位設計の正本は `doc/COMPAT_ARCHITECTURE.md` §5、互換契約の典拠は ukadoc（沈黙/曖昧箇所は COMPAT §2 の沈黙ルールに従い areka 裁量＋対応表へ明記）。

## Boundary Context

- **In scope**:
  - ukadoc 全イベントカタログと GET（要求/応答を期待）／NOTIFY（通知のみ）の分類
  - 各イベントのフィールドスキーマ（意味名・型・必須/任意・応答意味）
  - 唯一の正本対応表（意味名＝canonical ⇔ `Reference0/1/2…`＝alias）
  - 正準 content プロトコル（json-rpc 2.0）の封筒・`id`/`result`/`error`/notification マッピング、相関トークン↔`id` 対応
  - エンコーディング/charset 規約（content 文字列の扱い）、沈黙ルール適用箇所の明記
  - バージョニング方針の宣言（`areka-P0-shiori-com` の D7 と整合）
- **Out of scope**:
  - COM ABI（`IShiori`/`IShioriHost` 面そのもの）→ `areka-P0-shiori-com`
  - さくらスクリプト／SAORI 本文の解釈・実行 → `areka-P0-sakura-script` ほか（content は不透明文字列として運ぶ）
  - レガシーテキスト ⇄ 正準モデルの **翻訳実装** → `areka-P0-shiori-host-32`（翻訳が従う対応表＝契約は本仕様が定義する）
  - トランスポート（HSTRING の取り回し）→ `areka-P0-shiori-com`
  - 「1スキーマ→2表示」の投影機構・キルスイッチのデータ構造・json-rpc 封筒の実装表現など HOW（→ 設計フェーズ）
- **Adjacent expectations**:
  - 上流 `areka-P0-shiori-com` は content を不透明 HSTRING として運ぶ。本仕様はその content の中身（正準プロトコル）の契約を定義し、ABI 面は変更しない。
  - 下流 `areka-P0-shiori-host-32` は本仕様の対応表に従ってレガシー wire を放出する。
  - 下流 `areka-P0-shiori-reference` / pasta native 脳は本仕様の語彙・対応表で話す。

## Requirements

### Requirement 1: 正準イベントカタログの所有と分類

**Objective:** As a 互換ベースウェアの実装者, I want ukadoc に基づく全イベントの単一カタログとその GET/NOTIFY 分類を持ちたい, so that 脳と areka が「どのイベントを扱い、応答を期待するか」を共通の基準で判断できる

#### Acceptance Criteria

1. The 正準イベントカタログ shall ukadoc に記載される全 SHIORI イベントを列挙し、その集合を本仕様が所有する単一のカタログとして定義する。
2. The 正準イベントカタログ shall 各イベントを GET 系（要求に対し応答を期待する）または NOTIFY 系（通知のみで応答を期待しない）のいずれかに分類する。
3. Where ukadoc がイベントの応答期待を明示しない場合, the 正準イベントカタログ shall 当該イベントの GET/NOTIFY 分類を沈黙ルール（COMPAT §2）に従って裁定し、その裁定を対応表に明記する。
4. The 正準イベントカタログ shall 各イベントを ukadoc 上のイベント ID（イベント名）で一意に識別する。

### Requirement 2: 正準語彙（意味名による params 表現）

**Objective:** As a 脳（SHIORI）の実装者, I want イベント引数を番号フィールドではなく意味のある名前で受け渡したい, so that 引数の意味がスキーマとして固定され、脳と areka が意味のある会話をできる

#### Acceptance Criteria

1. The 正準 content プロトコル shall 各イベントの引数（params）を意味名（named フィールド）で表現する。
2. The 正準イベントモデル shall 各イベントのフィールドについて、意味名・型・必須/任意の区別・ukadoc 上の `ReferenceN` 位置・応答側の意味を定義する。
3. The 正準イベントモデル shall 各フィールドの意味名を、対応表が定義する単一の正準名として用いる。

### Requirement 3: 単一正本対応表とエイリアスの従属

**Objective:** As a 互換契約の管理者, I want フィールド意味の正本を唯一の対応表に集約したい, so that 契約が host-32 や reference に分散せず、正本が常に1つに保たれる

#### Acceptance Criteria

1. The 唯一の正本対応表 shall フィールドの意味（意味名 ⇔ `Reference0/1/2…` の対応）を定義し、本仕様がその所有者となる。
2. The 唯一の正本対応表 shall 意味名（canonical）と `ReferenceN`（alias）を、1枚のスキーマから機械投影される2つの表示として規定する。
3. The `ReferenceN` 名 shall 対応表から導出される互換エイリアスとして扱われ、独立した権威を持たない。
4. If 意味名と `ReferenceN` エイリアスの解釈が食い違う場合, then the 正準モデル shall 意味名（canonical）の解釈を優先する。

### Requirement 4: json-rpc 封筒マッピング

**Objective:** As a 脳と areka 双方の実装者, I want request/応答/遅延/Raise が json-rpc 構造へ一意に対応してほしい, so that 相関トークンと即時/遅延/失敗/通知の区別が曖昧さなく伝わる

#### Acceptance Criteria

1. The 正準 content プロトコル shall json-rpc 2.0 を正準封筒とし、メソッド名を ukadoc イベント ID、params を意味名フィールドへ写す。
2. When 脳が要求に対し即時に応答する場合, the 正準 content プロトコル shall その応答を、要求の相関トークンに対応する `id` を持つ `result`（成功時）または `error`（失敗時）へ写す。
3. When 脳が要求に対し遅延応答（`SHIORI_S_PENDING`）を行う場合, the 正準 content プロトコル shall 先行して相関トークンに対応する `id` を返し、後続の完了通知で同一 `id` の `result` を配送する。
4. When 脳が能動的な wakeup（Raise）を発行する場合, the 正準 content プロトコル shall それを `id` を持たない notification として写す。
5. The 正準 content プロトコル shall 要求の相関トークンと json-rpc `id` の対応を規定どおり一意に対応付ける。

### Requirement 5: レガシー wire への放出方針とキルスイッチ

**Objective:** As a `areka-P0-shiori-host-32` の実装者, I want レガシー DLL wire へ何を必ず吐き、何を既定で併載し、どう抑制できるかを契約で知りたい, so that 旧 DLL を確実に動かしつつ、暴れる DLL を per-DLL で opt-out できる

#### Acceptance Criteria

1. The レガシー wire 放出契約 shall レガシー SHIORI/3.0 wire に対して、対応表に従った `ReferenceN` を必ず放出することを規定する。
2. The レガシー wire 放出契約 shall 既定でレガシー SHIORI/3.0 wire に対し、`ReferenceN` に加えて意味名エイリアスを併載する（Requirement 10 の併載基準に従う）。
3. The レガシー wire 放出契約 shall per-DLL（DLL 単位）で意味名エイリアスの併載を抑制できる opt-out フィルタ（暴れる DLL 用のキルスイッチ）を規定し、これを Requirement 10-3 の任意フィルタのレガシー wire における具体化とする。
4. The レガシー wire 放出契約 shall `Reference0/1/2…` を消去または別名へ置換しないこと（旧 DLL が要求するため）を不変条件として規定する。

### Requirement 6: 予約 SHIORI ヘッダとの非衝突

**Objective:** As a 互換契約の管理者, I want 意味名が予約済み SHIORI ヘッダと衝突しないことを保証したい, so that 意味名エイリアスを wire へ併載しても既存の予約ヘッダ解釈を壊さない

#### Acceptance Criteria

1. The 唯一の正本対応表 shall 意味名が予約 SHIORI ヘッダ（`ID`/`Sender`/`Charset`/`SecurityLevel`/`Status`/`Reference*` 等）と衝突しないことを保証する。
2. If 新たな意味名が予約 SHIORI ヘッダと衝突する場合, then the 対応表定義プロセス shall その意味名を非衝突となるよう是正する。

### Requirement 7: 沈黙ルールの裁定追跡

**Objective:** As a 互換契約の管理者, I want ukadoc が沈黙する裁定を対応表に記録したい, so that 互換の進捗が可視・反証可能になり、ukadoc 更新時に是正できる

#### Acceptance Criteria

1. Where ukadoc が裁定を沈黙する箇所（追加ヘッダの可否・意味割り当て・GET/NOTIFY 分類等）がある場合, the 唯一の正本対応表 shall その areka 裁量による裁定を対応表へ記録する（COMPAT §2）。
2. The 沈黙ルール記録 shall 各裁定について、典拠（ukadoc 条項の有無・SSP 二次参照の有無・areka 裁量の旨）を識別できる形で残す。
3. The 仕様 shall 契約抽出元の ukadoc 該当ページ（SHIORI Event／SHIORI Resource）のピン留めスナップショット（出典 URL・取得日・sha256）を保持し、典拠参照の再現性と ukadoc 更新時の差分検証を可能にする。

### Requirement 8: content 文字列のエンコーディング規約と不透明性

**Objective:** As a 脳と areka 双方の実装者, I want content 文字列の扱いと不透明フィールドの境界を契約で明確にしたい, so that さくらスクリプトや SAORI 引数を誤って解釈せずに運べる

#### Acceptance Criteria

1. The 正準 content プロトコル shall さくらスクリプト本文・SAORI 引数等の content を json-rpc の文字列フィールドへ不透明に載せ、その解釈を本仕様の対象外（別仕様）とする。
2. The 正準 content プロトコル shall content 文字列のエンコーディング/charset 規約を定義し、`Charset` ヘッダ等の文字集合情報との対応を規定する。
3. While content を不透明文字列として運ぶ間, the 正準 content プロトコル shall その文字列の中身を解釈・実行しない。

### Requirement 9: バージョニング方針の宣言

**Objective:** As a in-tree の全実装者（areka 本体・host-32・pasta）, I want 本契約のバージョニング方針を明示したい, so that プレリリース段階の契約変動と将来の凍結方針が共通理解になる

#### Acceptance Criteria

1. The 正準 content プロトコル shall そのバージョニング方針を、`areka-P0-shiori-com` の D7（プレリリース流動契約 → リリース時凍結）と整合する形で宣言する。
2. While プレリリース段階である間, the 正準 content プロトコル shall 後方互換保証を持たない流動契約とし、契約変更時は in-tree の全実装者を lockstep で更新する前提を宣言する。
3. The 正準 content プロトコル shall 将来の高レート通信余地（`areka-P0-shiori-com` の D6）を阻害しない封筒設計であること（通知＝応答不要・バッチ要求の許容等）を方針として宣言する。

### Requirement 10: 正準契約における意味名と `ReferenceN` の結合（併載基準＋任意フィルタ）

**Objective:** As a 互換契約の裁定者, I want 本仕様を「意味名と `ReferenceN` の結合（対応）を成立させる契約」として位置づけ、併載を契約上の基準としつつ非利用・フィルタを別レイヤに分離したい, so that 契約は両名の結合を一意に定義し、消費側の利用/非利用や areka 側のフィルタは契約を変更しない別ポリシーとして扱える

> **裁定（Q1・要件ディスカッション #1 で確定）**: 本仕様は「native 脳が何を使うか（消費側の都合）」ではなく「どういう契約が成立しているか」を定める。契約の本質は意味名と `ReferenceN` の **結合（対応）** であり、契約上の基準表現は **両名の併載** とする。pristine（意味名のみ）は契約からの省略ではなく、**areka 側の任意フィルタ**（例: native 経路を pristine 化）として実現する。消費側が `ReferenceN` を使わない判断も自由であり、いずれも契約を変更しない。

#### Acceptance Criteria

1. The 正準契約 shall 各イベントの正準 content 表現に、意味名と対応表由来の `ReferenceN` エイリアスの **両方** を含めることを基準（既定）として規定する（wire 経路によらず併載を契約上の基準とする）。
2. The 正準契約 shall 併載される意味名と `ReferenceN` が「両名の結合を表す単一スキーマの2投影」（Requirement 3）であり、両名が同一値を指すことを規定する。
3. Where 特定の wire 経路で `ReferenceN` を省略したい場合, the 正準契約 shall その省略を契約の変更ではなく **areka 側の任意フィルタオプション** として許容する（フィルタ機構の実装は設計フェーズ）。
4. The 正準契約 shall 消費側（脳）がいずれの名を読む/読まないかを消費側の自由とし、その選択が契約を変更しないことを規定する。
