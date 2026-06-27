# Requirements Document

## Project Description (Input)
`areka-P0-shiori-com` で `IShiori`/`IShioriHost` ABI は定義・実装済みだが、脳（COM-SHIORI）の実装はすべてテスト用モック（`#[cfg(test)]`）のみであり、非テストで実走可能な「正解見本」となるリファレンス脳が存在しない。このため、(1) ABI が実アプリで動く証明、(2) 下流（`areka-P0-shiori-host-32` の DLL ホスト／`areka-P0-reference-ghost` の pasta）が満たすべき `IShiori` 契約の参照点、が欠落している。

本仕様は、最小・非テストの**リファレンス COM-SHIORI（native 脳）**を `IShiori` を実装する形で提供し、areka 本体から in-proc アクティベーションで挿して、request→即時応答／遅延→Complete／Raise の各経路が実アプリ上で動くことを示す。content は不透明のまま固定／エコー応答とし、正準 content プロトコルの確定は別仕様 `areka-P0-shiori-protocol` に委譲する。上位設計の正本は `doc/COMPAT_ARCHITECTURE.md` §5。隣接（完成）は `areka-P0-shiori-com`（`IShiori`/`IShioriHost` ABI）。

## Introduction
本仕様は、`areka-P0-shiori-com` で確立された `IShiori`/`IShioriHost` ABI に対する、最小かつ非テストの**リファレンス実装（native 脳）**と、その**実走デモ経路**を定義する。リファレンス脳は `shiori-abi` の公開 API（`ShioriExt` / `#[implement(IShiori)]`）を用いて製品コード（非 `#[cfg(test)]`）として実装され、areka 本体は in-proc アクティベーションでこの脳を挿し、数往復のリクエストをドライブして後始末する。

狙いは二つある。第一に、ABI が実アプリ上で実際に動くことの**実走証明**を得ること。第二に、下流仕様（`areka-P0-shiori-host-32` の DLL ホスト、`areka-P0-reference-ghost` の pasta 旗艦脳）が `IShiori` を実装する際の**正解見本（リファレンス）**を提供することである。

content（リクエスト・応答・通知の本文）は本仕様では**不透明のまま固定／エコー**で扱う。正準 json-rpc content プロトコルの定義、32bit DLL ホスティング、pasta 旗艦脳、さくらスクリプト解釈、DLL 適合（conformance）テストキットは、いずれも隣接する別仕様の責務であり本仕様には含めない。

## Boundary Context
- **In scope（本仕様が責務を持つ範囲）**:
  - 最小の native リファレンス脳を、製品コード（非テスト）として `IShiori` を実装する形で提供すること（固定／エコー応答、遅延／Raise の最小実演を含む）
  - areka 本体から、リファレンス脳を in-proc アクティベーションで挿し、リクエストを数往復ドライブし、後始末（unload）する**実走デモ経路**
  - 即時応答（同期）、遅延応答（pending ＋後続完了）、能動通知（Raise）の各経路を実アプリ上で疎通させること
  - リファレンス脳と実走デモ経路の参照点としての**ドキュメント化**
- **Out of scope（本仕様が責務を持たない範囲）**:
  - 正準 json-rpc content プロトコルの定義・確定 → `areka-P0-shiori-protocol`
  - 過去互換 DLL（32bit shiori.dll 等）のホスティング → `areka-P0-shiori-host-32`
  - pasta（native 旗艦脳）の実装 → `areka-P0-reference-ghost`（M2）
  - さくらスクリプトの解釈・実行
  - DLL 適合（conformance）テストキット（`areka-P0-shiori-host-32` 実装過程で決定）
  - content の意味づけ・スキーマ・解析（本仕様では content は不透明文字列のまま固定／エコー）
  - 毎秒ポーリング等の上位タイミングロジック、x86（32bit）ネイティブ直結（x64／CPU ネイティブ前提）
- **Adjacent expectations（隣接仕様・既存資産への期待）**:
  - 上流 `areka-P0-shiori-com`（完成）が提供する `IShiori`/`IShioriHost` ABI と `shiori-abi` の公開 API（`ShioriExt`・`#[implement(IShiori)]`）を、本リファレンス脳は変更せずに利用すること
  - areka 本体が既に備える in-proc アクティベーション受け皿（`IShioriHost` sink／セッション規律：単一インフライト・遅延突き合わせ・タイムアウト・後始末）を、本仕様の実走デモ経路は利用すること
  - 下流 `areka-P0-shiori-host-32`／`areka-P0-reference-ghost` は、本リファレンス脳を `IShiori` 実装の見本として参照すること（DLL 契約境界は host-32 実装過程で本リファレンスを見本に決定）
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
1. When 実走デモ経路が起動するとき, the areka demo path shall リファレンス脳を in-proc アクティベーションで挿し（ロードし）、利用可能な状態にする。
2. When リファレンス脳がアクティベーションされた後, the areka demo path shall 即時応答・遅延応答・能動通知（Raise）の各経路を含む数往復のリクエストをドライブする。
3. When 数往復のドライブが完了したとき, the areka demo path shall リファレンス脳をアンロードして後始末する。
4. The areka demo path shall 即時応答・遅延応答・能動通知の各経路の疎通結果を、利用者または開発者が観測可能な形で示す。
5. While 遅延応答が未完了のとき, the areka demo path shall 既存セッション規律（単一インフライト・相関トークンの突き合わせ・タイムアウト）に従って完了を待ち合わせる。
6. If アクティベーション・リクエスト・後始末のいずれかが失敗したとき, then the areka demo path shall 失敗を判別可能な形で報告し、後始末を試みる。

### Requirement 7: リファレンスとしてのドキュメント化
**Objective:** 下流実装者として、リファレンス脳と実走デモ経路が「正解見本」として参照できるよう文書化されていてほしい。これにより、`host-32`／pasta が `IShiori` を実装する際の参照点が明確になる。

#### Acceptance Criteria
1. The reference documentation shall リファレンス脳が実装する `IShiori` の各経路（ロード／アンロード・即時応答・遅延応答・能動通知）を正解見本として説明する。
2. The reference documentation shall content を不透明・固定／エコーとして扱う方針と、正準 content プロトコルが別仕様（`areka-P0-shiori-protocol`）の責務である旨を明示する。
3. The reference documentation shall 下流（`areka-P0-shiori-host-32`・`areka-P0-reference-ghost`）が本リファレンスを見本として参照する位置づけを示す。

### Requirement 8: content の不透明性とスコープ境界
**Objective:** 統合者として、本仕様が content の意味づけに踏み込まずスコープ境界を守ることを保証したい。これにより、正準プロトコル・DLL ホスト・pasta 旗艦・conformance キットとの責務分離が崩れない。

#### Acceptance Criteria
1. The reference SHIORI brain shall リクエスト・応答・通知の content を、上流 ABI が定める不透明な文字列表現（UTF-16）のまま取り回し、解析・スキーマ検証・意味づけを行わない。
2. The reference SHIORI brain shall 正準 content プロトコル、32bit DLL ホスティング、pasta 旗艦脳、さくらスクリプト解釈、適合（conformance）テストキットを実装しない。
3. The reference SHIORI brain shall x64／CPU ネイティブ前提（x86 除外）に従い、上流 ABI の流動契約（変更時は in-tree 実装者を追従更新）に整合する。
