# 設計バリデーションレポート: areka-P0-file-slimming

> 実施 2026-08-07（kiro-validate-design・非対話実行）
> 対象: `design.md`（2026-08-07 生成・設計判断 #1〜#13 全裁定済）／入力: `requirements.md`（確定済）・`research.md`（第 2 版）・`.kiro/steering/`
> 検証方法: design-review.md の 4 基準（既存アーキテクチャ整合・設計一貫性・保守性・インターフェース設計）＋設計書中の検証可能な主張のコードベース実地照合

## Review Summary

設計は要件 7 系統・33 項目の全 Acceptance Criteria をトレーサビリティ表で網羅し、残っていた設計判断 #1〜#13 をすべて根拠付きで裁定している。設計書中の検証可能な主張をコードベースに対して実地照合した結果、**照合した全項目が実測どおり**であった（下記「実地照合の結果」）。実装を阻む致命的欠陥はなく、残る懸念は実装フェーズの計画粒度に関わる 2 点のみである。

### 実地照合の結果（高リスク主張のスポットチェック）

| 設計の主張 | 実測 | 判定 |
|---|---|---|
| `follow.rs` 総行 8,472・テストモジュール開始 :1997（本体 1,996 行） | 8,472 行・`#[cfg(test)]` は :1997 | ✅ 一致 |
| `frame.rs` 総行 4,660・`Emo2Wiring` impl 内の `#[cfg(test)]` メソッド 5 件（:301-346） | 4,660 行・当該範囲に `#[cfg(test)]` メソッド 5 件確認 | ✅ 一致 |
| `follow.rs` 本番本体は `crate::` を 0 件使用（13 件は全てテストモジュール内） | :2008 より前に `crate::` 0 件・ファイル全体で 13 件 | ✅ 一致 |
| `#[path]` 属性: `crates/*/src/` に 0 件・`crates/*/tests/` に 115 件 | src 0 件・tests 合計 115 件 | ✅ 一致 |
| `spine_e2e_test.rs` 総行 2,574・テストモジュール 10 個・入口 `tests/ghost.rs` の `#[path]` 読込 | 2,574 行・`mod` 宣言 10 件・`ghost.rs:17` に `#[path]` 確認 | ✅ 一致 |
| examples の placement 木 `#[path]` include（`window-placement.rs:107`・`collision-probe.rs:231`）と「非テストコードは `crate::` 不可」の先行裁定コメント | 両ファイルで確認（collision-probe に裁定コメント現存） | ✅ 一致 |
| steering `structure.md` L133-145（Integration/Unit Tests 規約）・L204（モジュール分割パターン）の引用 | 引用どおり | ✅ 一致 |
| `foo.rs`＋`foo/` 共存前例（`areka-emo-atlas/src/decode.rs`＋`decode/wic_arm.rs`） | 両方存在 | ✅ 一致 |
| `viewbox_draw.rs` の `fail_next_render` 注入シーム（所見送付対象） | 存在確認 | ✅ 一致 |

`#[path]` 読込ファイルの子モジュール解決規則（素の `mod X;` は E0583・案 A 脱落の決定的根拠）は research §2.3.3 が rustc 1.97.1 のスクラッチクレートで実測済みであり、Rust のモジュール解決意味論（`#[path]` 読込ファイルは mod-rs 相当として振る舞い、子はそのファイル自身のディレクトリ基準で解決）と整合する。

## Critical Issues（2 件・いずれも GO を妨げない改善指摘）

🔴 **Critical Issue 1**: テーマ分割の粒度・名前の大半が実装時決定のまま残っている
**Concern**: テーマ分割対象 24 テストモジュールのうち、初期テーマ案が設計で確定しているのは `follow.rs`・`frame.rs` の 2 本のみで、残り 22 モジュールは「×約 2〜3」の概数とともに「実装時に各モジュールの内容から決定」とされている。新規テストファイル約 110 本の過半の構成が tasks 生成時点で未確定になる。
**Impact**: 実装者の裁量幅が大きく、テーマ境界の切り方次第でレビュー差し戻し・対応表の再生成（Revalidation Trigger 発火）が反復するリスクがある。
**Suggestion**: tasks フェーズで各対象モジュールに「テーマ案の確定 → 分割 → 対応表追記 → リスト一致」の順序をタスク内ステップとして固定し、クレート単位コミットのゲート（7.2）に対応表の全単射検証を含めることを明記する。設計変更は不要。
**Traceability**: Requirement 1.7／1.8／2.9
**Evidence**: design.md §テーマ分割ポリシー（「他 22 モジュールのテーマ名は実装時に…決定」）・§File Structure Plan

🔴 **Critical Issue 2**: examples `#[path]` include 下での 3 階層モジュール解決は間接根拠のみ
**Concern**: research §2.3.3 の実測は「`#[path]` 読込ファイル → その子」の 2 階層までであり、follow 本体分割後の構造は「`#[path]` include された `placement/mod.rs` → `follow.rs` → `follow/anchor.rs`」の 3 階層になる。この形の example ビルド成立は意味論からの推論と in-repo 類例（`decode.rs`＋`decode/`、ただし example include 外）に依っており、当該構成そのものの実測はない。
**Impact**: 万一解決が期待と異なる場合、follow 本体分割コミット（Migration 順 13）の段階で examples ビルドが赤になり、ファサード形の再設計（Revalidation Trigger）へ戻る。
**Suggestion**: 設計はコミット前ゲート（`cargo build -p areka --examples` 緑）を既に置いており妥当。追加として、本体分割着手前（テスト分離フェーズ中）に最小サブモジュール 1 本での事前スモーク確認を行い、赤ならテスト分離の成果に影響させず裁定へ戻す、という順序を tasks に反映するとリスクがゼロ化できる。
**Traceability**: Requirement 4.1／4.3／2.7
**Evidence**: design.md §本体分割の制約 5・§Revalidation Triggers／research.md §2.3.3

## Design Strengths

1. **主張の全てが新鮮な実測に基づき、検証で 1 件の乖離も出なかった**: 行数・属性件数・行番号・steering 引用・前例の存在まで、スポットチェックした全項目がコードベースと完全一致した。「doc の主張は書く前に file:line で裏取り」という開発規律を設計文書自身が満たしている。
2. **裁定の連鎖が要件の最難点を構造的に解いている**: 案 C（`#[path]` フラット兄弟）の採用により本番ファイルのパス変更が 0 本になり（3.5 の一覧が空・他 spec の brief アンカー保全・examples include 無影響）、案 A の脱落根拠（E0583）は実測で確定済み。検証パイプライン（3 本立てリスト＋旧→新テスト名の全単射対応表＋行頭空白正規化の本文比較）は「テスト総数不変・本文不変」を人手レビューではなく機械判定に落とし込んでおり、68,921 行の移設を安全に受け入れ可能にしている。

## Final Assessment

**Decision: GO**

**Rationale**: 全 33 Acceptance Criteria が設計要素へ追跡可能で、移設方式・分割方式・検証方式の裁定はいずれも実測根拠を持ち、スポットチェックで事実誤認が 1 件も見つからなかった。指摘 2 件はいずれも設計の欠陥ではなく tasks フェーズでの計画反映事項であり、実装リスクは既存のコミットゲートで受け止められる範囲にある。

**Next Steps**:
1. 設計ディスカッション（kiro-design-discussion）で上記 2 件の扱いを確認
2. `/kiro-spec-tasks areka-P0-file-slimming` でタスク生成（Issue 1 の順序固定・Issue 2 の事前スモークをタスクへ反映）
