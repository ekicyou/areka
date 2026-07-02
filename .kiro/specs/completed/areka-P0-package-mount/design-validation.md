# 設計バリデーションレポート（areka-P0-package-mount）

> 言語: ja / フェーズ: design-generated / 対象: `crates/areka-parsers`（`package` module 追加）
> 実行モード: 非対話（レビュー結果のみを提示・GO/NO-GO 判定）
> 正典参照: ukadoc（SSP 仕様）を第一情報源として照合。emo2 fixture は最小サンプル扱い。

## Design Review Summary

本設計は要件（Req 1–5）を高い忠実度で追跡し、ukadoc 正典・emo2 実 fixture・foundation の既存 API シグネチャの三方向すべてと実測レベルで整合している。`sakura` 確立パターンの踏襲（`model`＋`resolve` の最小分割・`#[non_exhaustive]`＋最小 derive・in-source `#[cfg(test)]`）が明確で、境界（所有/非所有・スコープ外）の宣言も精密。実装着手可能な品質に達しており、残る論点は「兄弟パーサとの意図的な非対称（`Result`／`std::fs`）をどこまで module doc で明文化するか」の程度問題に限られる。

## 照合済みエビデンス（正典・コード・fixture）

- **ukadoc `seriko.defaultsurfacedirectoryname` 既定 = `master`**: 正典で確認（`descript_ghost`）。設計 §System Flows の shelldir 既定・`DEFAULT_SHELL_DIR="master"` は正しい（Req 3.1）。
- **ukadoc: `shell/master` は仕様上必須**: `dev_shell_error`「伺かの仕様上、shell/masterフォルダは必ず必要」で確認。shell dir 不在を致命（`ShellDirMissing`）とする設計判断①を正典が支持（Req 3.3）。
- **ukadoc `shiori,ファイル名` の canonical 既定 = `shiori.dll`**: 正典で確認。設計は Req 2.3 に従い既定への推測を明示的に禁止（`Option::None`）しており、正典既定を知りつつ推測しない判断は妥当。
- **foundation API シグネチャ一致（実コード照合）**: `charset::decode(bytes: &[u8], default: DefaultEncoding) -> String`（`charset/decode.rs:24`）、`DefaultEncoding::Utf8`（`charset/model.rs:18`）、`kv::parse_kv(text: &str) -> BTreeMap<String,String>`（`kv/parse.rs:20`）。設計の合成フローと完全一致。
- **`decode` は charset 宣言を prescan して優先**（`charset/decode.rs:24-44`）: emo2 の `charset,UTF-8` 宣言があるため、渡す `DefaultEncoding::Utf8` に関わらず UTF-8 が選定される。設計の「decode が BOM/宣言を吸収」という記述は正確で、既定引数 `Utf8` の選択は安全。
- **`parse_kv` はカンマ無し行を自動スキップ**（`kv/parse.rs:26-28`）: emo2 の `id.emo2`（カンマ無し）が自然に無視される設計主張は実装で裏付けあり（Req 5.2）。
- **emo2 fixture の実レイアウト一致**: `ghost/master/descript.txt`・`ghost/master/pasta.dll`・`shell/master/`・スコープ外の `install.txt`／`emo2-kakukaku/`／`delete.txt` が実在。emo2 descript.txt は `name,えも？？`／`sakura.name,むらさき`／`kero.name,エモ`／`shiori,pasta.dll`・`seriko.defaultsurfacedirectoryname` 不在を確認。§Testing Strategy の期待値（`shiori.file==Some("pasta.dll")`・shell 既定 `master` フォールバック・名前 3 値）はすべて実 fixture と一致。
- **steering `structure.md:222-229` 整合**: `areka-parsers` = 純粋パーサ群（`std`＋`tracing`）、`package` は `sakura` パターンへ接ぎ木、emo2 使用分のみ（過剰実装禁止）という steering 指針を設計は踏襲。

## Critical Issues（最大 3・設計ディスカッションへ供給）

### 🔴 Critical Issue 1: 兄弟パーサ規約からの二重逸脱（`Result` / `std::fs`）の module doc 明文化が要点
- **Concern**: steering `structure.md:226` は parser 規約を「`pub fn parse(&str) -> Vec<Model>`・**`Result` 無しの寛容パース**」と定義するが、本設計は (a) `Result<MountModel, MountError>` を返し、(b) 入力を `&str` でなく `&Path` とし `std::fs` I/O を持つ、の 2 点で逸脱する。設計はこれを Req 5.1（マウントは不在という現実の失敗を持つ）と「I/O を `resolve` submodule に局所化＋`lib.rs` doc 補記」で正当化している。
- **Impact**: 逸脱自体は要件駆動で妥当だが、明文化が `lib.rs` doc の 1 行補記に留まると、後続の `shell-parse`／`balloon-parse` 実装者が「parser ファミリは Result 無し純粋」という steering 規約と本 module の実体との差を誤読し、規約が浸食される恐れ（module 規約の一貫性リスク）。
- **Suggestion**: `package/mod.rs` の module doc に「本 module は parser ファミリ内で唯一 I/O（`std::fs` 読取のみ）と `Result` を持つ。理由＝マウントは物理不在という現実の失敗を持つため（Req 5.1・`sakura` の寛容パースと意図的に非対称）」を明記し、逸脱が局所的・意図的であることをコード近傍で固定する。steering 側の追随更新可否は完了時に判断。
- **Traceability**: Req 5.1, Req 4.2 / steering `structure.md:223-229`
- **Evidence**: design.md §Existing Architecture Analysis「逸脱点」・§Error Strategy・§File Structure Plan（Modified Files）

### 🔴 Critical Issue 2: `StartPointUnreadable` が要件本文の failure 列挙に明示されていない（設計先行の追加 variant）
- **Concern**: 設計は `MountError` に 3 variant（`StartPointMissing`／`StartPointUnreadable`／`ShellDirMissing`）を置くが、要件 Req 1.6・5.1 が明示するのは「起点**不在**」と「マウント先**ディレクトリ欠落**」であり、「descript.txt は在るが `std::fs::read` が失敗（権限・ロック等）」= `StartPointUnreadable` は要件本文に直接の acceptance がない。設計判断として妥当（黙って空を返さない方針 Req 1.6 の精神に沿う）だが、要件トレーサビリティ表では 1.1/5.1 に紐付けられている。
- **Impact**: 下流（`ghost-setup`／`host-32`）が `MountError` を網羅 `match` する際、要件だけを読むと 2 variant を想定し、3 つ目で `#[non_exhaustive]` フォールバックに落ちる。実害は小さい（`#[non_exhaustive]` で前方互換）が、失敗境界の Revalidation Trigger（design §Revalidation Triggers「どの欠落が致命か」）に該当するため合意の明示が望ましい。
- **Suggestion**: `StartPointUnreadable` を「Req 1.6/5.1 の観測可能失敗原則から派生した I/O エラー枝」と設計内で 1 行位置づけるか、テスト方針に読取失敗ケースの扱い（emo2 では発生しないため unit で誘発するか否か）を明記して合意を固定する。
- **Traceability**: Req 1.1, Req 1.6, Req 5.1
- **Evidence**: design.md §State Management（`MountError` enum）・§Error Categories・§Requirements Traceability（1.1 行）

### 🔴 Critical Issue 3: `resolve_tests` の起点読取失敗（`StartPointUnreadable`）テストが Testing Strategy に不在
- **Concern**: §Testing Strategy の unit test は 5 本（起点不在・shiori 未指定・shell 既定・shell 指定不在・type 欠落受理）を挙げるが、`MountError` の 3 variant のうち `StartPointUnreadable` を発火する試験が列挙されていない。`StartPointMissing`／`ShellDirMissing` はカバーされるが、読取失敗枝は未検証のまま実装される可能性。
- **Impact**: 定義した failure variant の 1/3 が実行検証されないと、`std::io::ErrorKind` の受け渡し・early-return 経路にリグレッションが入っても検知できない。純粋 loader ゆえ影響は限定的だが、「欠落＝明示的失敗」（Req 5.1）の中核を成す型の一部が無試験になる。
- **Suggestion**: Windows で確実に読取失敗を誘発する手段（例: descript.txt をディレクトリとして作成し `read` を `ErrorKind` エラーにする、または権限操作）でのケース追加可否を検討。誘発が不安定なら「本 variant は I/O 契約として保持するが unit では非検証（消費側で観測）」と Testing Strategy に明記して意図を残す。
- **Traceability**: Req 5.1, Req 1.1
- **Evidence**: design.md §Testing Strategy（Unit Tests 1–5）・§State Management（`StartPointUnreadable`）

## Design Strengths

1. **正典・実コード・fixture の三方向整合が実測で裏付けられている**: ukadoc（`master` 既定・`shell/master` 必須・`shiori.dll` 既定）、foundation の実 API シグネチャ、emo2 descript.txt の実値までが設計記述と一致。特に「所在ベース識別＝`type` 分岐を作らない」「`id.emo2` はカンマ無しゆえ `parse_kv` が自動スキップ」の主張はコードで検証済みで、過剰実装禁止（Req 5.2）を実装前に構造で担保している。
2. **境界宣言と Revalidation Triggers の精度**: This Spec Owns / Out of Boundary / Allowed Dependencies / Revalidation Triggers が I/O 契約（`MountModel`／`MountError`）の生成者・消費者関係まで含めて明示され、パス表現（`PathBuf`）・エンコード変換責務（消費側境界）・致命/非致命の分類が下流（`ghost-setup`／`host-32`）と再検証条件込みで固定されている。単一クレート内 module 追加という最小侵襲パターンの選択も steering 方針に忠実。

## Final Assessment

**Decision: GO**

**Rationale**: 全要件（Req 1.1–5.3）がコンポーネント・インタフェース・フローへ追跡され、ukadoc 正典・foundation 実 API・emo2 実 fixture の三方向すべてと実測整合している。critical issues 3 点はいずれも「意図的逸脱の明文化」「先行追加した failure variant の合意・試験カバレッジ」に関する明確化事項であり、アーキテクチャの根本的欠陥・要件未達・過大な複雑性のいずれにも該当しない（`#[non_exhaustive]` で前方互換も確保済み）。受容可能なリスクの範囲。

**Next Steps**:
- 設計ディスカッション（`/kiro-design-discussion areka-P0-package-mount`）で上記 3 論点（module doc への逸脱明記・`StartPointUnreadable` の要件位置づけ・読取失敗テストの扱い）を合意。
- 合意反映後 `/kiro-spec-tasks areka-P0-package-mount` でタスク生成へ進む。
