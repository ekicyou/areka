# 設計バリデーション報告: areka-P0-host32-shiori-load

> 対象: FINALIZED な `design.md`（要件 1〜6・27 criteria）に対する実装準備性レビュー。
> 手続き: `kiro-validate-design` の REVIEW プロセス（Analysis → Critical Issues → Strengths → GO/NO-GO）。
> 実行モード: 非対話（ユーザ照会なし）。言語: ja（spec.json.language）。日付: 2026-07-02。
> グラウンディング: 凍結上流 transport（`shiori-host32-ipc`／`shiori-host32-host`）と helper 現状コードを一次確認済み。

---

## Design Review Summary

設計は「純ロジック分類器＋副作用 WndProc」既存パターンの忠実な再適用と、`unsafe` FFI を `ShioriByteProxy` 単一型に集約する構造で、上流凍結 seam を一切改変せず LOAD セマンティクスを結線する。要件 6／27 criteria は Requirements Traceability 表で全網羅・追跡可能で、境界（request 呼出しない／unload は courtesy のみ／常駐 lifecycle 非所有）も明確に切られている。wire-neutrality・load-ack 形状・fixture 隔離・cdecl ABI いずれも凍結上流の実コードと突き合わせて成立を確認でき、実装準備性は高い。

### wire-neutrality 一次確認（設計主張の裏取り）

- **load-ack が真に wire 中立であること**を凍結上流の実コードで確認:
  - 親側 `classify_inbound` は `Ok((MsgTag::Response, payload)) => StoreResponse`（`parent_window.rs:91`）を**送出タグ非依存**で処理し、`send_request(tag, ...)` は**任意 `MsgTag`** を受け付ける（`parent_window.rs:320-341` / `lib.rs:325-341`）。ゆえに親が `send_request(MsgTag::Load, &[], t)` を発行→helper が `MsgTag::Response` の 1 byte bool を返す設計は、既存の再入 RESPONSE 経路（`slot.clear→SendMessageTimeout→StoreResponse→slot.take`）へそのまま乗る。**新タグ・新フレーム形式は不要**。
  - `MsgTag` enum は `{Hello=1, Load=2, Request=3, Response=4, Unload=5}`（`lib.rs:44-55`）で `Load=2` が既定義。設計は定義を改変しない。
- **helper 側差し替え点**も現状と一致: `classify_inbound` は現在 `Ok((tag,_)) => IgnoreKnown(tag)` の総称アームで `Load` を無視（`main.rs:82`）。設計の「`LoadDll` バリアント追加＋`Load` 分岐差し替え＋classify_tests 期待更新（`main.rs:307-313`）」は最小差分で整合。
- **spawn 契約拡張**: 現状 `spawn(helper_exe, ghostdir, parent_hwnd)` は parent_hwnd を arg1＋env、ghostdir を `current_dir` のみで運ぶ（`process_host.rs:125-137`）。設計の「load_dir・SHIORI 名を arg＋env fallback で追加（`HOST32_LOAD_DIR`/`HOST32_SHIORI_NAME`）」は既存 `parent_hwnd_from_env`（`main.rs:230-237`）の 2 経路パターンの横展開で、wire には及ばない launch パラメーター拡張。

### 解決済み決定の忠実な実現（判定）

- **(a) load_dir＋SHIORI 名を起動パラメーターで供給・LOAD はトリガ**: 実現（design.md §Decision(a)・spawn Service Interface §363-368）。
- **(b) load-ack 形状（`Response` 1 byte bool・`[1]`=成功）**: 実現・凍結経路で裏取り済（上記）。
- **(c) fixture = host-32 トラック所有 i686 cdylib `shiori-host32-testdll`（lib name "shiori"・`crate-type=["cdylib"]`）・emo2 pasta は `HOST32_PASTA_DLL` env-gated・pilot 非依存**: 実現（§File Structure・§test 資産層）。
- **(d) courtesy unload+FreeLibrary on Drop**: 実現（`ShioriByteProxy::Drop` §300・§Decision(d)）。
- **ABI（cdecl・bool 1byte・load 入力 HGLOBAL は DLL 解放）**: `LoadFn/UnloadFn/RequestFn` 型定義（§276-280）と一致。i686 usize=32bit／u64 幅演算は既存 `copydata_payload` 踏襲を明記（R6.2）。

---

## Critical Issues（最大 3・設計ディスカッションへ供給）

本レビューでは**実装を止める NO-GO 級の critical issue は検出されなかった**。以下は GO を前提とした実装前の留意点（すべて設計内で言及済み・裁量範囲内）であり、ディスカッションで確認すれば足りる。

### 🟡 留意点 1: spawn シグネチャ変更の cwd 挙動と既存呼出箇所の破壊的更新
- **Concern**: 設計の新 `spawn(helper_exe, load_dir, shiori_name, parent_hwnd)`（§363-368）は現行 `spawn(helper_exe, ghostdir, parent_hwnd)` から引数を並べ替え・`ghostdir`（`current_dir` 専用）を `load_dir` へ置換する。「cwd 依存をやめる」方針だが、現行 `current_dir(ghostdir)` を残すか捨てるかが interface シグネチャからは一意でなく、既存呼出（`echo_roundtrip.rs`）は同一 PR 内でコンパイルエラーになる。
- **Impact**: cwd 挙動の取りこぼしは E2E での DLL パス解決に影響しうる。ただし設計は Open Questions #2 で arg 順を「実装時に固定」と明記し、Implementation Notes §377 で「既存呼出は更新が必要（薄い破壊的変更）」を認識済み。
- **Suggestion**: 実装時に arg 順（arg1=parent_hwnd 既存維持・arg2=load_dir・arg3=shiori_name）と env 名を cross-task 契約として固定し、cwd を load_dir に合わせるか明示的に不使用とするかを 1 行で確定する。
- **Traceability**: R1.2 / R1.5。
- **Evidence**: design.md §spawn 起動パラメーター拡張（§343-377）・Open Questions #2（§442）。

### 🟡 留意点 2: ABI 一次源（vendors/pasta）が本 worktree で未展開
- **Concern**: `vendors/pasta` submodule 未展開ゆえ `pasta_shiori/src/windows.rs`（:50/63/76）のバイト正確な署名を本 worktree で確認できず、`[patch.crates-io] pasta_core` の path 先欠落で workspace cargo が壊れうる。設計 ABI は pilot 二次記録依拠。
- **Impact**: 実装前提であってブロッカーではない（pilot go 済＝同一 ABI 実証済）。ただし production 実装前に一次確認しないと fn 署名・シンボル名の齟齬リスクが残る。
- **Suggestion**: 実装着手前に `git submodule update --init` を実行し、`load(HGLOBAL,usize)->bool` / `unload()->bool` / `request(HGLOBAL,*mut usize)->HGLOBAL` を一次源で再確認（設計 Open Questions #1・Risk R1 で既に前提化）。
- **Traceability**: R6.3。
- **Evidence**: design.md §Open Questions #1（§441）・research.md §11 R1。

### 🟡 留意点 3: cdylib の x64/i686 ビルドと E2E での i686 成果物解決
- **Concern**: `shiori-host32-testdll` は `members=["crates/*"]` に自動包含され x64 通常ビルドでも cdylib が x64 でビルドされうる。E2E は i686 helper 越しゆえ i686 成果物を確実に指す必要がある。
- **Impact**: 誤って x64 DLL を load させると i686 helper で `LoadLibraryFailed`（ビットネス不一致）。設計は §399 で「E2E は env＋i686 target 探索で明示解決・無ければ panic」を規定済で、失敗も観測契約内（crash なし）。
- **Suggestion**: `echo_roundtrip.rs` の `HOST32_HELPER_EXE` 方式に倣い testdll 解決 env（例 `HOST32_TESTDLL`）＋i686 target 配下探索の優先順を実装時に固定。
- **Traceability**: R5.1 / R5.7 / R6.1 / R6.4。
- **Evidence**: design.md §最小 SHIORI DLL fixture Implementation Notes（§396-399）。

---

## Design Strengths

1. **凍結 seam の忠実な尊重と最小差分**: load-ack を新タグでなく既存 `MsgTag::Response`（1 byte bool）で既存再入経路に乗せ、LOAD を空トリガに純化、パスは起動パラメーター経由（wire を通らない）とした設計は、上流凍結 wire/framing/`MsgTag` を一切改変しない。親側 `classify_inbound` の送出タグ非依存 RESPONSE 処理・`send_request` の任意タグ受理という既存実装で成立が裏取りできる（真に wire 中立）。

2. **`unsafe` FFI の単一型集約と葉ノード隔離**: `ShioriByteProxy` に module handle・3 fn ポインタ・HGLOBAL 所有権規約（load 入力=callee 解放）・ANSI 符号化・Drop teardown・`transmute`／生ポインタを閉じ込め、`ProxyError` で一様に crash なし失敗報告する構造は安全境界が明快。fixture を host-32 トラック所有 cdylib として持ち `crates/pilot` を一切参照しないことで、失敗パス（`load→false`・エクスポート欠落）を決定的テスト化しつつ葉ノード隔離を自然遵守する。

---

## Final Assessment

### Decision: **GO**

### Rationale
要件 6／27 criteria を全追跡し、境界（request 未呼出／unload courtesy のみ／常駐 lifecycle 非所有）と凍結 seam 不改変を凍結上流の実コードで裏取りできる。検出された 3 点はいずれも設計内で言及済みの実装裁量・前提（submodule 展開・arg 順固定・cdylib ターゲット解決）であり、アーキテクチャ不整合・要件欠落・失敗リスクの過大はない。

### Next Steps
1. `/kiro-design-discussion areka-P0-host32-shiori-load`（対話・chat window）で留意点 1〜3 を確認・固定。
2. 実装着手前に `git submodule update --init`（ABI 一次確認・workspace cargo 健全性回復）。
3. 合意後 `/kiro-spec-tasks areka-P0-host32-shiori-load` でタスク生成。
