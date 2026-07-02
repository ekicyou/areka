# 設計検証レポート: areka-P0-parser-foundation

> 実行日: 2026-07-02 / 対象言語: ja / phase: design-generated
> 検証プロセス: kiro-validate-design（Analysis → Critical Issues → Strengths → GO/NO-GO）
> 非対話モードでの検証。入力: spec.json / requirements.md / design.md / research.md / steering。

---

## 設計レビュー要約

本設計は「純粋関数パイプライン（2 段・独立モジュール）」として、`charset::decode` と `kv::parse_kv` を既存 `areka_parsers` の兄弟トップレベルモジュールへ追加する。要件 ID 1.1–7.5 のすべてが Requirements Traceability 表に出現し、既存 `sakura` 規律（no Result・no panic・tracing のみ・in-source テスト・`#[non_exhaustive]`・最小派生・公開パス契約）を機械的に踏襲する。研究フェーズの持ち越し 6 項目（D1–D6）が根拠付きで確定しており、投機的抽象を排した実装準備完了レベルの設計である。

---

## フォーカス項目の実地検証（コードベース照合）

指示された 4 つのフォーカス項目を、設計文言だけでなく実コード・fixture・要件と突き合わせて確認した。

- **`charset::decode(bytes, DefaultEncoding) -> String` の純粋性（R3）と ANSI→CP932/SHIFT_JIS 固定写像**: シグネチャは `pub fn decode(bytes: &[u8], default: DefaultEncoding) -> String`（design.md §Service Interface）。OS ロケール非参照・引数のみで決定的（R3.1/3.2、決定事項 D6）。`Ansi→SHIFT_JIS(CP932)` / `Utf8→UTF_8` の固定写像は要件ディスカッション #1（既定は呼び出し側が引数指定・SHIORI/4 は UTF-8 固定）と整合。**適合**。
- **既存 `sakura` 規律の踏襲**: `crates/areka-parsers/src/sakura/mod.rs` を照合し、`mod`（非公開内部）＋`#[cfg(test)] mod *_tests;` 列挙＋`pub use` facade＋`validation_tests` の構成を確認。`model.rs:23-24` で `#[non_exhaustive]` ＋ `#[derive(Clone, Debug, PartialEq)]`（最小派生）を確認。設計の File Structure Plan・`DefaultEncoding` 型定義はこの規律に忠実。**適合**。
- **過剰実装・投機的抽象の排除（brief 禁止事項）**: `kv` を単一責務ゆえ内部 `parse.rs` 1 本に留め（sakura の 4 分割を機械的に真似ない）、戻り値を素朴 `BTreeMap` として NewType を導入しない設計判断は brief の「過剰・予測実装禁止」に合致。全コンポーネントが要件 ID にトレースされ orphan なし。**適合**。
- **`encoding_rs` を唯一の外部依存・`BTreeMap`・プリスキャン上限・SJIS ラウンドトリップ**: `encoding_rs` 0.8 を唯一の意図的追加依存（D5、ルート `[workspace.dependencies]` 集約）。KV に `BTreeMap<String,String>`（D4）。プリスキャン上限 4096 バイト＋非 ASCII 打ち切り（D1）。SJIS 合成は `encoding_rs::SHIFT_JIS.encode(<リテラル>)` ラウンドトリップ（D3、R7.2/7.3）。**適合**。

### fixture 由来リテラル期待値の照合（正本ファイル実測）
- `emo2-kakukaku/descript.txt` L1–L3 = `charset,UTF-8` / `type,balloon` / `name,kakukaku for emo-gs` → validation_tests #1 の期待値（`type→balloon`・`name→kakukaku for emo-gs`）と**完全一致**。
- `emo2-kakukaku/balloons0s.txt` L1/L2/L4 = `windowposition.x,266` / `windowposition.y,-129` / `wordwrappoint.x,-49`（charset 行なし）→ validation_tests #2 の期待値・「charset なしファイル」根拠と**完全一致**。fixture が実在し charset 宣言の無いファイルが存在するという設計前提を実証。

---

## Critical Issues（最重要 ≤3）

本設計に GO を妨げる重大な設計不整合は検出されなかった。以下は実装フェーズで留意すべき軽微な指摘（いずれも NO-GO 事由ではない）であり、参考として最大 3 件に絞って記す。

🟡 **Minor Issue 1**: BOM とプリスキャンの相互作用の実装記述にわずかな二義性がある
**Concern**: design.md §Implementation Notes / System Flows 補足は BOM を「非 ASCII 相当として扱うか読み飛ばして誤検出を防ぐ」と両論併記しており、実装者に一意の指針を与えていない。UTF-8 BOM（`EF BB BF`）はバイト値が `>= 0x80` ゆえプリスキャンの非 ASCII 打ち切り条件（D1）に素直に該当し得るが、その場合「先頭 BOM だけで宣言なし判定に至る」経路の是非が明文化されていない。
**Impact**: 実装差異が生じても寛容フォールバック（既定エンコード）で吸収され破綻はしないが、BOM 付き `charset,UTF-8` ファイルで宣言を取り逃す軽微な挙動差の余地。R5.2 の「BOM を悪影響なく扱う」は最終デコードで満たされるため機能要件は充足。
**Suggestion**: 実装タスクで「先頭 BOM バイト列を読み飛ばしてからプリスキャンを開始する」を一意の方針として固定し、prescan_tests に BOM 付き `charset,UTF-8` の検出ケースを 1 本追加する。
**Traceability**: R5.2, R1.1
**Evidence**: design.md「System Flows 補足」「Components / charset / Validation」

🟡 **Minor Issue 2**: クレート description の "std-only" 更新可否を実装タスクへ委譲している
**Concern**: `encoding_rs` 追加で description の "std-only" 文言が厳密には破れる点を、design.md「Modified Files」は「必要なら軽微更新（実装タスクで判断）」と留保している。判断が実装者に委ねられており設計としては未確定。
**Impact**: ドキュメント文言のみで機能・契約に影響なし。放置しても動作は正しい。
**Suggestion**: 承認済み意図的逸脱ゆえ、description を "minimal-dependency" 相当へ更新する方針を実装タスク受け入れ条件に一行明記して曖昧さを消す（任意）。
**Traceability**: research §1.5（std-only からの意図的逸脱）
**Evidence**: design.md「File Structure Plan / Modified Files」

🟡 **Minor Issue 3**: `for_label` のラベル正規化に対する固定テストの明示化
**Concern**: ukadoc 表記 `Shift_JIS` を `encoding_rs::Encoding::for_label` が解決できることは validation_tests で固定するとしているが（§Risks）、`Shift_JIS` 表記自体の解決確認テストが Unit/Integration のどのケースに含まれるか設計上の対応が間接的。
**Impact**: SJIS 合成テスト（validation #3）が実質的にこの解決をカバーするため、実害はほぼない。ラベル解決失敗時も既定フォールバックで破綻しない（R2.5）。
**Suggestion**: validation_tests #3 のコメントに「`for_label(b"Shift_JIS")` 解決の生き証人」と明記し、ラベル正規化の意図をテスト意図として固定する（任意）。
**Traceability**: R2.2, R2.5
**Evidence**: design.md「Components / charset / Risks」「Testing / Integration #3」

---

## 設計の強み（Strengths）

- **要件↔設計↔コードの三点整合が実証済み**: 要件 ID 1.1–7.5 が全数 Traceability 表に出現し、既存 `sakura` 規律（mod.rs / model.rs で実地確認）と fixture リテラル期待値（descript.txt / balloons0s.txt を実測照合）まで一致する。研究フェーズの 6 論点が D1–D6 として根拠付きで確定し、未解決事項ゼロ。
- **純粋性と寛容規律の構造的担保**: `encoding_rs` の `for_label -> Option` / `decode -> (Cow, _, had_errors)` が `Result`・panic を返さない API 形状であることを利用し、no Result・no panic・tracing のみという要件（R6）を「設計上の努力」でなく「依存の型形状」で構造的に満たしている。ANSI/UTF-8 の 2 値 enum ＋ `#[non_exhaustive]` は要件語彙と 1:1 対応しつつ将来拡張余地を残す好バランス。

---

## 最終評価

### 決定: **GO**

**根拠**: 既存アーキテクチャとの不整合・要件ギャップ・過剰複雑性はいずれも認められず、全要件 ID がトレースされ実装パスが具体（公開シグネチャ・内部分割・テスト採取元まで確定）。検出された 3 件はすべてドキュメント/テスト明示レベルの軽微指摘で、機能・契約・純粋性に影響せず寛容フォールバックで吸収される。実装着手に足るリスク許容範囲内。

### 次ステップ
- 上記 Minor Issue（特に #1 の BOM プリスキャン方針の一意化）を設計ディスカッションで確認・任意反映。
- `/kiro-spec-tasks areka-P0-parser-foundation` で実装タスクを生成する。
