# 設計バリデーションレポート: areka-P0-host32-shiori-load

- **実施日**: 2026-07-02
- **対象**: `design.md`（693 行・WS-A/WS-B 二系統）／`requirements.md`（R1–R13・65 AC）／`research.md`（§2 windows-core ソース実証・§8 設計判断確定）／`brief.md`（D1–D7・locked ABI）
- **手法**: kiro-validate-design（design-review.md プロセス: Analysis → Critical Issues → Strengths → GO/NO-GO）。実コード照合込み（凍結 ipc・host/helper 現状・shiori-abi 現状・consumer 波及面）。

---

## Review Summary

設計は確定済み決定（brief D1–D7・discussion #1・research §8 (b)–(g)）を忠実に実現しており、65 の受入基準すべてがコンポーネント・フロー・テスト戦略へトレースされている。凍結境界の遵守は実コードで裏取りできた——`send_request` はタグ汎用＋per-call timeout（`ipc/src/lib.rs:325`）であり、LOAD-ack は既存 API のまま成立、ipc 改変ゼロという設計主張は正確。locked ABI（`shiori_factory`／`CreateInstance(load_dir, shiori_name, host: Ref, out: OutRef) -> Result<()>`／`Get -> HRESULT` 生返し／`Notify -> Result<()>`／Load・Unload 不在／snake_case 安全面・`ShioriExt` 廃止）も design §InterfaceLayer/§SafeSurface が寸分違わず反映し、research §2 のソース実証済み制約（vtable に `Result<T≠()>` 不可・非 unsafe 不可・Rust enum 不可）に抵触する要素は一つもない。残る指摘は実装規律レベルの非ブロッキング 2 点のみ。

### 検証マトリクス（要点）

| 観点 | 判定 | 根拠 |
|---|---|---|
| 凍結境界（R13.1） | ✅ | ipc 実コード照合: `MsgTag{Hello=1,Load=2,...}`・`send_request(target, self_hwnd, tag, payload, timeout, slot)` はタグ汎用＋per-call timeout。design は「定数追加も含め触らない・ack バイト値は host/helper ローカル定数」と明記。spawn 拡張は launch 契約のみ |
| locked ABI 忠実性（brief D6・research §2.3） | ✅ | vtable 3 interface・C 入口 `shiori_factory`（`extern "system"`・raw out・`shiori_create` 残置なし）・safe `create()->Result<IShiori>`・`Get` 成功 2 値の HRESULT 生返し・teardown=Drop・全 Host 操作 `Result<()>`——§2.3 の落とし込み案と design 本文が完全一致 |
| windows-core 制約適合（research §2.1） | ✅ | vtable 面に `Result<T≠()>` なし（`CreateInstance`/`Notify`/Host 4 操作=`Result<()>`・`Get`=HRESULT）。`GetOutcome`（Rust enum）は安全面のみ。`Ref`/`OutRef`/`&HSTRING`/`&mut` はすべて §2.1 で可と実証済みの形 |
| 設計判断 (b)–(g) の反映 | ✅ | (b) `Mutex<HashMap>` 内蔵ストア＋再入規約＋`SHIORI_E_PROPERTY_NOT_FOUND(0xA0A1_0004)`／(c) 位置引数 spawn＋env 3 種＋cwd=load_dir／(d) `HOST32_TESTDLL_DLL`→target 探索→明確 panic・一時 dir コピー・kernel32.dll で EntryNotFound 証明／(e) WS 並行・順序制約は submodule→proxy のみ／(f) NotLoaded 削除・CreateFailed/GetFailed/NotifyFailed/UnknownToken／(g) GetOutcome・LOAD_ACK_TIMEOUT=30s per-call・欠落 exit(2)・LOAD 再受領は冪等 ack[1] |
| 要件カバレッジ | ✅ | R1(3)+R2(4)+R3(6)+R4(7)+R5(4)+R6(4)+R7(7)+R8(6)+R9(4)+R10(5)+R11(4)+R12(6)+R13(5)=65 AC 全行トレース。境界正確（request=解決のみ・unload=Drop courtesy のみ・常駐/互換 factory=下流・ABI 証明=reference/mock） |
| リスク現実性 | ✅ | WndProc 同期 load×30s timeout・`RefCell<Option<Proxy>>` 単一 UI スレッド・HGLOBAL callee 解放（testdll が二重解放検出器を兼ねる）・i686 PowerShell 規律・submodule 前提（cargo 解決前提と明記）・波及 19 変更+4 新設のコンパイル駆動吸収——いずれも既知トラップに対処済み |
| 既存アーキ整合 | ✅ | 「純関数分類→WndProc 副作用」「arg 優先 env fallback・pub const」「三層テスト・silent skip 禁止」「`#[implement]`＋AsImpl」の既存慣行を全面踏襲。依存方向規律（pilot 隔離・WS 間相互依存禁止）明記 |

---

## Critical Issues（非ブロッキング・設計ディスカッションへの申し送り）

🔴 **Critical Issue 1**: `RefCell<Option<ShioriByteProxy>>` の再入借用規律が未明文化
**Concern**: helper の ack 送出（`send_copydata`＝SendMessageTimeoutW）はブロック中に WndProc 再入を許す。TriggerLoad 処理で `proxy` の `RefCell` 借用を ack 送出やその他のブロッキング呼出をまたいで保持すると、再入時に `BorrowMutError` panic＝helper クラッシュとなり R6.4（無クラッシュ生存）を破る。design は「single UI thread 前提で RefCell で足りる」とするのみで、借用スコープ規約を定めていない。
**Impact**: 単一 in-flight 規律下では現実的な再入経路は限定的だが、panic は R6 全体の観測契約を無効化する態様であり、実装時の借用スコープ次第で顕在化する。
**Suggestion**: 「`RefCell` 借用は proxy 確立／参照の最小スコープに限り、`send_copydata`（ack 送出）呼出中は借用を保持しない」を HelperLoadWiring の Invariant として tasks に明記（sink の再入規約 R10.3 と同型の一行契約）。
**Traceability**: R6.4・R4.1（Evidence: design §HelperLoadWiring・§Data Models「HelperShared」）

🔴 **Critical Issue 2**: R2.2（courtesy unload）の検証エビデンスが Testing Strategy に不在
**Concern**: `ShioriByteProxy::Drop` の courtesy `unload`→`FreeLibrary` は設計されている（§ShioriByteProxy）が、Testing Strategy の Unit/Integration/E2E いずれにも Drop 経路の検証項目がない。E2E は helper をプロセスとして扱うため、proxy の Drop が実際に走る系列（正常 shutdown）が本仕様のテスト計画上存在しない可能性がある。
**Impact**: R2.2/R2.3 が「実装はあるがエビデンスなし」で完了扱いになるおそれ。DoD ゲート（fresh-evidence）で差し戻しになる前に検証形を決めておくべき。
**Suggestion**: helper の i686 単体テストに「testdll を直接 `ShioriByteProxy::load`→drop→無 panic（＋可能なら testdll 側で unload 呼出を観測可能化——env 指定ファイルへのマーカー書出等の最小手段）」を 1 本追加するか、「Drop 実装はコードレビュー＋無 panic 観測で足る（unload 呼出自体は best-effort ゆえ観測必須としない）」と受入解釈を tasks で明文化する。
**Traceability**: R2.2・R2.3（Evidence: design §ShioriByteProxy「Drop teardown」・§Testing Strategy）

（第 3 の指摘なし）

---

## Design Strengths

1. **制約のソース実証に立脚した ABI 設計**: windows-core マクロの表現力を推測でなくマクロソース直読で確定させ（research §2.1 判定表）、その帰結（二層構造の必然・`Get` のみ HRESULT 生返し・`Result<T≠()>` 不可）を設計の各署名へ正確に落とし込んでいる。凍結 wire 側も `send_request` タグ汎用性を実コードで裏取りし「ipc 改変ゼロ」を主張でなく事実にしている。実装時の手戻りリスクが最小化された、証拠駆動の模範的設計。
2. **既存慣行への一貫した適合と波及の全量特定**: `PARENT_HWND_ENV` 同型拡張・三層テスト・silent-skip 禁止・`#[implement]` 慣行を踏襲しつつ、consumer 波及（19 変更＋4 新設）を実測でファイル単位に列挙。vtable 直呼びハック全廃・`ShioriExt` 廃止という負債返済まで波及範囲に組み込み、1 PR 完結の実行可能性が具体的に担保されている。

---

## Final Assessment

**Decision: GO**

**Rationale**: 確定済み決定（D1–D7・locked ABI・(b)–(g)）をすべて制約実証済みの形で忠実に実現し、65 AC 全トレース・凍結境界遵守・リスク対処が揃っている。指摘 2 件はいずれも実装規律／検証エビデンスの明文化であり、tasks 生成時に吸収可能（設計の再生成を要しない）。

**Next Steps**:
1. 設計ディスカッション（kiro-design-discussion）で Issue 1・2 の扱いを確定（いずれも tasks への一行明記で解消可能）
2. `/kiro-spec-tasks areka-P0-host32-shiori-load` でタスク生成へ進む（最初のタスク＝`git submodule update --init`・Issue 1 の Invariant と Issue 2 の検証形を該当タスクへ反映）
