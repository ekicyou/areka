# Requirements Document

## Project Description (Input)
`areka-P0-shiori-com` で `IShiori`/`IShioriHost` ABI は定義・実装済みだが、脳（COM-SHIORI）の実装はすべてテスト用モック（`#[cfg(test)]`）のみであり、非テストで実走可能な「正解見本」となるリファレンス脳が存在しない。このため、(1) ABI が実アプリで動く証明、(2) 下流（`areka-P0-shiori-host-32` の DLL ホスト／`areka-P0-reference-ghost` の pasta）が満たすべき `IShiori` 契約の参照点、が欠落している。

本仕様は、最小・非テストの**リファレンス COM-SHIORI（native 脳）**を `IShiori` を実装する形で提供し、areka 本体から in-proc アクティベーションで挿して、request→即時応答／遅延→Complete／Raise の各経路が実アプリ上で動くことを示す。content は不透明のまま固定／エコー応答とする。正準 content プロトコルは完了仕様 `areka-P0-shiori-protocol` が確定済みで、その論理 SSOT は `doc/shiori/fragments/`（フラグメント群＋決定的結合）にある。本リファレンスは content 語彙を持たず（SSOT 二重定義禁止に整合）、不透明／エコーに留める。上位設計の正本は `doc/COMPAT_ARCHITECTURE.md` §5。隣接（完成）は `areka-P0-shiori-com`（`IShiori`/`IShioriHost` ABI）。

## Introduction
本仕様は、`areka-P0-shiori-com` で確立された `IShiori`/`IShioriHost` ABI に対する、最小かつ非テストの**リファレンス実装（native 脳）**と、その**実走デモ経路**を定義する。リファレンス脳は `shiori-abi` の公開 API（`ShioriExt` / `#[implement(IShiori)]`）を用いて製品コード（非 `#[cfg(test)]`）として実装され、areka 本体は in-proc アクティベーションでこの脳を挿し、数往復のリクエストをドライブして後始末する。

狙いは二つある。第一に、ABI が実アプリ上で実際に動くことの**実走証明**を得ること。第二に、下流仕様（`areka-P0-shiori-host-32` の DLL ホスト、`areka-P0-reference-ghost` の pasta 旗艦脳）が `IShiori` を実装する際の**正解見本（リファレンス）**を提供することである。

content（リクエスト・応答・通知の本文）は本仕様では**不透明のまま固定／エコー**で扱う。正準 json-rpc content プロトコルの定義、32bit DLL ホスティング、pasta 旗艦脳、さくらスクリプト解釈、DLL 適合（conformance）テストキットは、いずれも隣接する別仕様の責務であり本仕様には含めない。

## Boundary Context
- **In scope（本仕様が責務を持つ範囲）**:
  - 最小の native リファレンス脳を、製品コード（非テスト）として `IShiori` を実装する形で提供すること（固定／エコー応答、遅延／Raise の最小実演を含む）
  - COM-SHIORI（x64／ARM64）が `IShiori` を生成する唯一の純粋Cコンストラクタ・エクスポート契約（`shiori_create`）の定義・実装、および areka がそれ経由で脳を取得すること
  - areka 本体から、リファレンス脳を in-proc アクティベーションで挿し、リクエストを数往復ドライブし、後始末（unload）する**実走デモ経路**
  - 即時応答（同期）、遅延応答（pending ＋後続完了）、能動通知（Raise）の各経路を実アプリ上で疎通させること
  - リファレンス脳と実走デモ経路の参照点としての**ドキュメント化**
- **Out of scope（本仕様が責務を持たない範囲）**:
  - 正準 content プロトコルの定義・確定（完了済み・論理 SSOT＝`doc/shiori/fragments/`）→ `areka-P0-shiori-protocol`。本リファレンスは content 語彙を参照・複製しない（二重定義禁止に整合）
  - 過去互換 DLL（32bit shiori.dll 等）のホスティング → `areka-P0-shiori-host-32`
  - pasta（native 旗艦脳）の実装 → `areka-P0-reference-ghost`（M2）
  - さくらスクリプトの解釈・実行
  - DLL 適合（conformance）テストキット（`areka-P0-shiori-host-32` 実装過程で決定）
  - content の意味づけ・スキーマ・解析（本仕様では content は不透明文字列のまま固定／エコー）
  - 毎秒ポーリング等の上位タイミングロジック、x86（32bit）ネイティブ直結（対象は x64 ＋ ARM64／CPU ネイティブ前提・x86 除外）
- **Adjacent expectations（隣接仕様・既存資産への期待）**:
  - 上流 `areka-P0-shiori-com`（完成）が提供する `IShiori`/`IShioriHost` ABI と `shiori-abi` の公開 API（`ShioriExt`・`#[implement(IShiori)]`）を、本リファレンス脳は変更せずに利用すること
  - areka 本体が既に備える in-proc アクティベーション受け皿（`IShioriHost` sink／セッション規律：単一インフライト・遅延突き合わせ・タイムアウト・後始末）を、本仕様の実走デモ経路は利用すること
  - 下流 `areka-P0-shiori-host-32`／`areka-P0-reference-ghost` は、本リファレンス脳を `IShiori` 実装と純粋Cコンストラクタ（`shiori_create`）の見本として参照すること（COM 経路の生成入口契約は本仕様が確定。過去互換 flat-C／32bit ホスティング固有の DLL 境界は host-32 実装過程で決定）
  - `areka-P0-shiori-protocol` は、本リファレンス脳が将来 json-rpc content を採用する際の起点となること
  - ABI は流動契約として扱われ、上流 ABI が変動した場合は in-tree 実装者として本リファレンス脳も追従して更新されること

## Requirements

### Requirement 1: 非テストのリファレンス COM-SHIORI（native 脳）
**Objective:** ABI 利用者・下流実装者として、テスト用モックではなく製品コードとして実走する `IShiori` 実装の正解見本がほしい。これにより、ABI が実アプリで動く証明と、下流が従うべき契約の参照点が得られる。

#### Acceptance Criteria
1. The reference SHIORI brain shall 上流 `areka-P0-shiori-com` の `IShiori` インターフェイスを実装する。
2. The reference SHIORI brain shall 製品コード（非 `#[cfg(test)]`）として提供され、テストビルドに限定されずに実行可能である。
3. The reference SHIORI brain shall 既存の `shiori-abi` 公開 API を利用し、上流 ABI（`IShiori`/`IShioriHost`）の面を変更しない。
4. The reference SHIORI brain shall 最小の応答ロジック（固定応答またはエコー応答）に範囲を限定し、content の意味づけ・解析・スキーマ検証を行わない。

### Requirement 2: ライフサイクル（ロード／アンロード）
**Objective:** areka 本体の開発者として、リファレンス脳の初期化と終了を ABI のライフサイクル操作どおりに制御したい。これにより、脳のリソースを確実に確保・解放できる。

#### Acceptance Criteria
1. When areka 本体がリファレンス脳をロードするとき, the reference SHIORI brain shall 能動通知および遅延応答に使用する `IShioriHost`（sink）を受け取り、ロードを成功として完了する。
2. When areka 本体がリファレンス脳をアンロードするとき, the reference SHIORI brain shall 終了処理を行い、アンロードを成功として完了する。
3. While リファレンス脳がロードされていない状態のとき, the reference SHIORI brain shall リクエスト操作を有効な処理として受理せず、判別可能な失敗として報告する。
4. If ロードまたはアンロードが失敗したとき, then the reference SHIORI brain shall 失敗を呼び出し側へ判別可能な形で報告する。

### Requirement 3: 即時応答（同期リクエスト）
**Objective:** ABI 利用者として、リクエストに対する即時応答経路が実アプリで動くことを確認したい。これにより、同期呼び出し＋即時応答という最も基本的な経路の実走が証明される。

#### Acceptance Criteria
1. When areka 本体がリファレンス脳へリクエストを送るとき, the reference SHIORI brain shall リクエストを同期的なメソッド呼び出しとして受け取る。
2. When リファレンス脳がリクエストへ即時応答するとき, the reference SHIORI brain shall 応答文字列を伴う即時応答として結果を返す。
3. The reference SHIORI brain shall 即時応答文字列を、固定文字列または受信 content のエコーとして生成する。
4. The reference SHIORI brain shall リクエスト引数および応答文字列を、上流 ABI が定める不透明な文字列表現のまま取り回し、内容を解釈しない。

### Requirement 4: 遅延応答（pending ＋後続完了）
**Objective:** ABI 利用者として、即時に応答しない遅延経路（相関トークン発行と後続完了）が実アプリで動くことを確認したい。これにより、呼び出しをブロックしない遅延応答の実走が証明される。

#### Acceptance Criteria
1. When リファレンス脳がリクエストを遅延扱いとするとき, the reference SHIORI brain shall 即時の応答文字列を伴わない遅延結果を返し、後続応答と突き合わせるための相関トークンを発行する。
2. When リファレンス脳が遅延扱いとしたリクエストの応答を届けるとき, the reference SHIORI brain shall 受け取った `IShioriHost`（sink）の完了操作を用い、対応する相関トークンと応答文字列を渡す。
3. The reference SHIORI brain shall 即時応答経路と遅延応答経路の双方を、実走デモにおいて少なくとも一度ずつ実演する。
4. While 遅延扱いのリクエストが未完了のとき, the reference SHIORI brain shall 発行した相関トークンを完了時に突き合わせ可能な形で保持する。

### Requirement 5: 能動通知（Raise）
**Objective:** ABI 利用者として、脳が areka からの問い合わせを待たずに能動的に通知（wakeup）する経路が実アプリで動くことを確認したい。これにより、push 経路の実走が証明される。

#### Acceptance Criteria
1. When リファレンス脳が areka へ能動的に通知するとき, the reference SHIORI brain shall 受け取った `IShioriHost`（sink）の Raise 操作を用いて通知内容を渡す。
2. The reference SHIORI brain shall Raise の通知内容を、固定または既知の不透明文字列として渡し、内容を解釈しない。
3. The reference SHIORI brain shall 能動通知（Raise）を実走デモにおいて少なくとも一度実演する。

### Requirement 6: areka 本体からの実走デモ経路
**Objective:** 統合者として、areka 本体からリファレンス脳を挿して数往復ドライブし後始末する最小デモ経路がほしい。これにより、ABI の各経路が実アプリ上で end-to-end に疎通することを観測できる。

#### Acceptance Criteria
1. When 実走デモ経路が起動するとき, the areka demo path shall リファレンス脳を正準コンストラクタ `shiori_create`（要件 9）経由で取得し、in-proc アクティベーションで挿し（ロードし）、利用可能な状態にする。
2. When リファレンス脳がアクティベーションされた後, the areka demo path shall 即時応答・遅延応答・能動通知（Raise）の各経路を含む数往復のリクエストをドライブする。
3. When 数往復のドライブが完了したとき, the areka demo path shall リファレンス脳をアンロードして後始末する。
4. The areka demo path shall 即時応答・遅延応答・能動通知の各経路の疎通結果を、利用者または開発者が観測可能な形で示す。
5. While 遅延応答が未完了のとき, the areka demo path shall 既存セッション規律（単一インフライト・相関トークンの突き合わせ・タイムアウト）に従って完了を待ち合わせる。
6. If アクティベーション・リクエスト・後始末のいずれかが失敗したとき, then the areka demo path shall 失敗を判別可能な形で報告し、後始末を試みる。

### Requirement 7: リファレンスとしてのドキュメント化
**Objective:** 下流実装者として、リファレンス脳と実走デモ経路が「正解見本」として参照できるよう文書化されていてほしい。これにより、`host-32`／pasta が `IShiori` を実装する際の参照点が明確になる。

#### Acceptance Criteria
1. The reference documentation shall リファレンス脳が実装する `IShiori` の各経路（ロード／アンロード・即時応答・遅延応答・能動通知）を正解見本として説明する。
2. The reference documentation shall content を不透明・固定／エコーとして扱う方針と、正準 content プロトコルが完了仕様 `areka-P0-shiori-protocol`（論理 SSOT＝`doc/shiori/fragments/`）の責務であり、本リファレンスはそれを参照・複製しない旨を明示する。
3. The reference documentation shall 下流（`areka-P0-shiori-host-32`・`areka-P0-reference-ghost`）が本リファレンスを見本として参照する位置づけを示す。

### Requirement 8: content の不透明性とスコープ境界
**Objective:** 統合者として、本仕様が content の意味づけに踏み込まずスコープ境界を守ることを保証したい。これにより、正準プロトコル・DLL ホスト・pasta 旗艦・conformance キットとの責務分離が崩れない。

#### Acceptance Criteria
1. The reference SHIORI brain shall リクエスト・応答・通知の content を、上流 ABI が定める不透明な文字列表現（UTF-16）のまま取り回し、解析・スキーマ検証・意味づけを行わない。
2. The reference SHIORI brain shall 正準 content プロトコル、32bit DLL ホスティング、pasta 旗艦脳、さくらスクリプト解釈、適合（conformance）テストキットを実装しない。
3. The reference SHIORI brain shall x64 ＋ ARM64（CPU ネイティブ前提・x86 除外）に従い、上流 ABI の流動契約（変更時は in-tree 実装者を追従更新）に整合する。

### Requirement 9: 純粋Cコンストラクタ・エクスポート契約（COM-SHIORI 生成入口）
**Objective:** 下流実装者（`areka-P0-shiori-host-32`・`areka-P0-reference-ghost`）として、COM-SHIORI が `IShiori` 実体を生成する唯一の入口＝純粋Cコンストラクタの契約を、リファレンスが正解見本として定義・実装してほしい。これにより、DLL 境界の生成契約が一点に確定し、下流が同一の入口形を踏襲できる。

#### Acceptance Criteria
1. The reference COM-SHIORI shall `IShiori` 実体を生成する唯一の純粋Cコンストラクタ関数（正準名 `shiori_create`）をエクスポートし、`IShiori` の生成をこの入口に一元化する。
2. The shiori_create constructor shall 対象プラットフォーム（x64／ARM64）で最も標準的な呼出規約＝プラットフォーム標準 C ABI（Rust 表記 `extern "C"`）に従う。x86 を対象外とするため対象各プラットフォームでは呼出規約が一意に定まり、`extern "C"` と `extern "system"` は同一 ABI となる。
3. When shiori_create が `IShiori` 実体の生成に成功するとき, the shiori_create constructor shall 参照カウント 1 の `IShiori` を出力引数経由で呼び出し側へ渡し（`HRESULT shiori_create(IShiori** out)` 形）、成功を表す HRESULT を返す。
4. If shiori_create が生成に失敗したとき, then the shiori_create constructor shall 出力を生成せず、失敗を判別可能な HRESULT として返す。
5. The areka demo path shall shiori_create が返した `IShiori` を所有し、`Load(host)`→リクエスト数往復→`Unload` の後に参照を解放（Release）する。
6. The reference COM-SHIORI shall この純粋Cコンストラクタ・エクスポート契約を、下流（`areka-P0-shiori-host-32`・`areka-P0-reference-ghost`）が DLL 境界の生成契約の正解見本として参照できる形で提供する。
7. The reference COM-SHIORI shall 本コンストラクタ契約の対象を COM（x64／ARM64・in-proc）経路の生成入口に限定し、過去互換 flat-C（`load`/`unload`/`request`）・32bit DLL ホスティングは対象外（→ `areka-P0-shiori-host-32`）とする。
