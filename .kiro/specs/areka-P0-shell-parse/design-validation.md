# 設計バリデーションレポート — areka-P0-shell-parse

> 対象: `.kiro/specs/areka-P0-shell-parse/design.md`（design-generated フェーズ）
> 実行モード: 非対話（NON-INTERACTIVE）。判定を直接産出する。
> 正典参照: requirements.md（確定済）・research.md・steering（`structure.md` Parser Crate 節）・ukadoc（SERIKO/surfaces.txt）。

## Design Review Summary

本設計は emo2 `surfaces.txt`（SERIKO/2.0 サブセット）→ 型付きシェルサーフェスモデルの純粋パーサを、確立済み `sakura` 四層パターン（`model ← lexer ← decode ← parse`）へ precise に接ぎ木する。要件 47 acceptance criteria 全 ID が Requirements Traceability 表に網羅され、要件ディスカッションで確定した 4 論点（ukadoc 準拠自前テスト主軸／descript・charset 非保持スキップ／sakura 四層規律／research 6 決定）が設計本文で漏れなく解決されている。アーキ整合・境界明瞭性・実行可能性いずれも実装移行に足る水準にあり、致命的な設計不整合は認められない。

## 判定観点別の確認結果

### (a) ukadoc 準拠の自前 in-source テスト（emo2 = スコープ＋スモークのみ）
- §Testing Strategy 冒頭に「**正典 = ukadoc**。テストは ukadoc 準拠の自前 in-source 断片を主軸とし、emo2 fixture は実サンプルのスモークテストとして併用」と明記。emo2 を「唯一の適合基準とはしない」（要件 10.2）を validation_tests §2 で再確認。合致。
- クレート跨ぎ `include_str!` は自前断片ゆえ不要、期待値リテラル直書き（sakura 規律）を明示。合致。

### (b) descript ヘッダ＋charset 行の寛容スキップ（モデル非保持）
- §System Flows「charset 行 / descript ブロックの寛容スキップ」＋ model の `Shell` が `surfaces/appends/aliases` の 3 フィールドのみ（descript/charset フィールドを持たない）で構造的に担保。要件 3.1/3.2/3.3 と整合。合致。
- 将来 header 保持は `#[non_exhaustive]` 拡張シーム、2 例目まで追加しない（要件 3.4/10.5）を Implementation Notes/Risks に明記。合致。

### (c) sakura 四層パターンの踏襲
- `model ← lexer ← decode ← parse` 一方向依存・公開面一点集約（`pub use`）・`Result` 無し寛容パース・opaque NewType（`new()`＋`as_str()`）・`#[non_exhaustive]` enum・`tracing` のみ・in-source `#[cfg(test)]` を §Architecture/§Components で全て踏襲。steering `structure.md` Parser Crate 節の方針とも一致。合致。

### (d) research.md 6 設計決定の解決
1. append 範囲展開: parse 時 inclusive 展開 `SurfaceAppend.targets: Vec<u32>`（§7.2.1）。解決。
2. surface 定義 vs append 統一/別型: 別トップレベル型 `SurfaceAppend`・内部 collision/animation 共有型（§7.2.2・要件 7.3）。解決。
3. 重複 alias 順序保持コンテナ: `Vec<SurfaceAlias>`（§7.2.3・要件 8.4）。解決。
4. 疎 pattern index: `Pattern.index: u32` を `Vec<Pattern>` 保持（§7.2.4・要件 5.4）。解決。
5. ルート型形状: `Shell { surfaces, appends, aliases }`（§7.2.5）。解決。
6. テスト定義: ukadoc 準拠自前主軸＋emo2 スモーク（§7.2.6）。解決。

全 6 項目が design.md で明示解決済。

## Critical Issues（≤3）

（実装移行を妨げる致命的問題は認められない。以下は設計ディスカッションで詰めると良い非ブロッキングの明確化候補であり、いずれも「実装上の詳細で fixture・ukadoc により一意に定まる」ため NO-GO 要因ではない。）

- **明確化候補 1: `surface.append` ヘッダ数値の二役解決の実装明瞭化**
  - 概要: `surface.append10,2100-2110,...`（ヘッダ `10`=カテゴリ番号・実ターゲットは後続列挙）と `surface.append2200 { ... }`（列挙なし・ヘッダ自身がターゲット）の 2 形を、「後続列挙があればそれのみ／無ければヘッダ自身」で切り替える決定（§7.4・decode Implementation Notes）。fixture で一意だが ukadoc に明示規定がない実装細目のため、lexer での「ヘッダ行＝ブロックヘッダ兼 CSV ターゲット列」二役の切り分け（lexer Risks 記載）を実装前に断片テストで固定すると安全。
  - Traceability: 要件 7.1/7.2  Evidence: design.md §7.4・§Semantic Layer decode Implementation Notes・§Syntax Layer lexer Risks

- **明確化候補 2: `Interval` パラメータ付き variant と `#[non_exhaustive]` の網羅テスト規律**
  - 概要: `Interval::Random{k}` / `BindRandom{k}` はパラメータ K を保持し、`#[non_exhaustive]` により外部クレートでの網羅 match が不可能になる。下流（shell-anim-engine）消費時のパターンマッチ規律（`_ =>` フォールバック要否）は本 spec 境界外だが、model_tests での variant 網羅アサーション方針（§Testing Strategy model_tests）を明示済みか実装時に確認するとよい。
  - Traceability: 要件 1.4/5.2/5.3/5.7  Evidence: design.md §Types Layer model（`Interval` 定義）・§Testing Strategy Unit Tests 1

## Design Strengths

- **要件被覆の網羅性と追跡性**: 全 47 acceptance criteria（1.1–11.4）が Requirements Traceability 表に重複なく Components/Interfaces/Flows へ写像され、各モデル型・decode 責務にも要件 ID が併記されている。orphan なく、実装者が要件↔設計↔ファイルを一意に辿れる。
- **確立パターンへの厳格な接ぎ木と過剰実装ガード**: sakura 四層規律を骨格・思想レベルで踏襲しつつ「lexer 実体は新規（構文クラス相違）」と流用可否を峻別。emo2 未使用機能を Non-Goals で列挙し、拡張は `#[non_exhaustive]` シームのみ・2 例目まで抽象追加しない（YAGNI）を型設計・Risks に一貫して埋め込んでおり、steering の過剰実装禁止方針と完全整合。

## Final Assessment

- **判定: GO**
- **根拠**: 要件ディスカッションで確定した 4 論点と research の 6 設計決定がすべて design.md 本文で解決され、アーキ整合・境界・追跡性・実行可能性に致命的欠落がない。残る 2 点は fixture/ukadoc で一意に定まる実装細目の明確化候補であり、実装移行のブロッカーではない。
- **次ステップ**: 明確化候補 1・2 を設計ディスカッションで軽く確認のうえ、`/kiro-spec-tasks areka-P0-shell-parse` でタスク生成へ進む。
