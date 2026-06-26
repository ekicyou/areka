# 設計レビュー: areka-P0-shiori-com

> 本レビューは `kiro-validate-design` / `design-review.md` プロセス（Analysis → Critical Issues → Strengths → GO/NO-GO）に従う非対話レビューである。
> 入力: `spec.json`（language=ja）, `requirements.md`, `design.md`, `research.md`, steering（`tech.md` / `structure.md`）, 正本 `doc/COMPAT_ARCHITECTURE.md` §5。

## Design Review Summary

設計は要件 R1〜R7 をトレーサビリティ表で網羅的に対応づけ、正本 §5（内部唯一 ABI=`IShiori`、in-proc 直結、push 用 `IShioriHost`、WinRT 非依存の HSTRING 取り回し）と steering（`unsafe` を ABI 層へ集約、`I` プレフィックス COM 命名、`windows::core::Result`/`thiserror` 規約、`crates/*` 分割）に整合している。research §8 でこのプロジェクト初の「カスタム `#[interface]` 定義」という唯一の外部技術不確実性を実証済みで、2 層 ABI＋独立クレート（Option C）という選定も根拠が明快。実装可能性は高く、残るのは ABI 契約の正確さに関わる局所的な詰めである。

## Critical Issues（≤3）

### 🔴 Critical Issue 1: HSTRING [out]-param の所有権規約が「最大の UB 源」と認識されているが、契約面で未確定
**Concern**: `IShiori::Request` の `out_response: *mut HSTRING` と `out_token` の callee 確保・caller 解放規約が設計上「raw 層で正確に実装すること」（Open Questions / Risks）止まりで、ABI 契約（事前/事後条件）として確定していない。設計自身が「HSTRING 所有権規約の誤りが唯一の UB 源」と明言している箇所が、実装者へ渡す契約として曖昧なまま残る。
**Impact**: 下流（`areka-P0-shiori-host-32`・pasta・areka 本体）が同 ABI を別実装するため、所有権規約が一意に決まっていないと実装間で解放責務が食い違い、二重解放/リークという最悪クラスの不具合が ABI 境界をまたいで再現困難な形で発生する。流動契約（D7）でも所有権規約だけは初版で固定すべき不変条件。
**Suggestion**: `Request` の Postconditions に「`out_response` は callee が `HSTRING` を move-out し caller が drop で解放（`windows-core` の標準 move セマンティクス）」を明記し、`Raise`/`Complete` の `*const HSTRING`（借用・呼び出し中のみ有効）と対比で 1 行ずつ規約化する。結合テストの「HSTRING 往復不変条件」検証項目に二重解放/リーク非発生（Drop 回数）の観点を追加する。
**Traceability**: R4.1, R4.3, R3.2, R3.4
**Evidence**: design.md「Service Interface（raw vtable 形）」Postconditions、「Open Questions / Risks → HSTRING 所有権」、「Implementation Notes → Risks」

### 🔴 Critical Issue 2: `IShioriHost` ポインタの寿命・参照カウント規約が散文どまりで、循環参照回避が契約化されていない
**Concern**: `Load(host: *mut c_void)` で areka 実装の sink を脳へ渡す経路について、「host は areka 本体が所有し脳へは借用相当を渡す」「脳は保持期間中 AddRef/Release を遵守」と散文で述べるが、`Load` の引数が借用（呼び出し中のみ有効）なのか脳が保持してよい所有参照なのかが ABI シグネチャ・事前/事後条件として一意でない。`unload` での解放順序も未規定。
**Impact**: research §4-3 が Research Needed として挙げた「脳⇄host 循環参照の回避策」がそのまま残存。脳が `IShioriHost` を AddRef して保持し、areka 本体が脳を保持すると循環参照でどちらも解放されず、`unload` 後もリークする。push（R6）の中核経路であり、下流実装が規約を取り違えると検知困難。
**Suggestion**: `Load` の Preconditions/Postconditions に「脳は host を AddRef して `unload` まで保持してよい／`unload` 受信時に脳は Release し以後 host を呼ばない」という保持・解放契約を明記する。または「host は脳に対し弱参照相当（保持しない借用）」のいずれかに ABI として確定し、循環回避の所有方向（areka→脳→host を非循環に保つ）を Boundary Commitments の不変条件へ昇格する。
**Traceability**: R6.1, R6.2, R2.2
**Evidence**: design.md「ライフサイクル → sink ライフタイム」、「IShioriHost → Preconditions」、research.md §4-3

### 🔴 Critical Issue 3: 相関トークン突合の所有者・並行性と「突合不能トークン」処理が areka 本体側に未確定
**Concern**: トークン発行主体は脳（§8.4 で確定）だが、areka 本体が未完了 request とトークンを対応付けて保持する突合テーブルの所有者・並行性が「areka 本体スレッドからの逐次呼び出しを前提とする最小実装」とのみ記され、`Complete` が脳の別タイミング（能動・遅延）から来る際のスレッド前提と、突合不能トークン時の error HRESULT を「誰が」返すかが曖昧。本仕様 File Structure Plan では突合保持は areka 本体（配線）側でスコープ外寄りに置かれている。
**Impact**: 遅延応答（R3.3/R6.4）は本仕様の主要フローだが、突合機構の責務境界が ABI と areka 配線の間で宙吊りになると、実装フェーズで「どこにテーブルを置くか」が再設計対象になりタスク分解が破綻しうる。並行前提が不明確だと `Complete` がワーカースレッドから来た場合のデータ競合を見落とす。
**Suggestion**: 「突合テーブルは areka 本体側 `IShioriHost` 実装が所有」「`Complete` の呼び出しスレッド前提（areka スレッドへマーシャル/チャネル投函のいずれか）」「突合不能トークンは host が定義済み error HRESULT を返す」の 3 点を Data Models / System Flows に最小確定する。本仕様が ABI 契約のみを定め突合実装を後続に委ねるなら、その責務移譲を Boundary Commitments に明示し受け皿（最小スタブ）の範囲を 1 文で限定する。
**Traceability**: R3.3, R6.1, R6.4
**Evidence**: design.md「IShioriHost → Postconditions」、「State Management → Concurrency」、「Data Models → CorrelationToken」、「Modified Files」

## Design Strengths

1. **要件トレーサビリティと正本整合が極めて高品質**: 23 個の受入基準すべてを Requirements Traceability 表で Components/Interfaces/Flows に対応づけ、正本 §5 の各論点（in-proc 直結・WinRT 非依存・push sink・x86 除外）と 1 対 1 で整合。research §8 で唯一の技術不確実性（windows-rs カスタム `#[interface]`）を公式ソース検証済みにし、D4（手書き 2 層）の必然性まで裏付けている。
2. **責務境界と複雑性の抑制が適切**: 独立クレート `shiori-abi`（`wintf`/`dola` 非依存）で下流 32bit ホストの共有を構造的に解決し、raw vtable 層と ergonomic 層の物理分離で `unsafe` を ABI 層へ集約する steering 規約を満たす。状態を脳側へ寄せ ABI を無状態に保つ、バージョニングを D7 の lockstep で省く等、P0 スコープに対し過不足ない簡素化判断ができている。

## Final Assessment

**Decision: GO**

**Rationale**: 設計はアーキテクチャ整合・要件網羅・複雑性の妥当性をいずれも満たし、実装パスは明快で受容可能リスク内にある。指摘した 3 件はいずれも ABI 境界の「契約文言の精緻化」（所有権・寿命・突合責務）であり、設計の根本的再構築ではなく Components/Interfaces の事前/事後条件への追記で解消できる範囲。流動契約（D7）下でも所有権・寿命規約だけは初版で固定する価値が高いため、設計ディスカッションでの優先確定を推奨する。

**Next Steps**:
1. 設計ディスカッション（`/kiro-design-discussion areka-P0-shiori-com`）で Critical Issues 1〜3 を ABI 契約文言として確定（design.md の Postconditions/Preconditions と Boundary Commitments への追記）。
2. 確定後 `/kiro-spec-tasks areka-P0-shiori-com` でタスク生成へ進む。
