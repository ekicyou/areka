# 設計の検証: runtime-crate-split

- 対象: `.kiro/specs/runtime-crate-split/design.md`（2026-11-16 生成・未承認）
- 入力: `spec.json`（language = ja）・`requirements.md`・`design.md`・`research.md`・steering（`product.md`・`tech.md`・`structure.md`）
- 進め方: `kiro-validate-design`（`rules/design-review.md`: Analysis → Critical Issues → Strengths → GO/NO-GO）。対話なしで判定まで出す。

## 設計の検証の要約

設計は、要件 1〜7 の全項目を部品・段・確かめに結び付けており、失敗がどれも組み立てかテストの赤として出る形（「失敗の扱い」）にそろえてある。前例 `placement-crate-split` の型（`test-support` 機能・`test_open`・読み口）にそのまま乗っていて、実装の道筋ははっきりしている。ただし、落ちの報告の据え方（D6）と私有の項目の開け方（D3 ⑵）の 2 か所に、「振る舞いを変えない」（要件 6.1・6.8・7.1）を破りうる穴があり、どちらも設計の討議で小さな直しで閉じられる。

## コードでの確かめ（安く確かめられた所）

| 設計の主張 | 確かめた結果 |
|---|---|
| 家の型 `test_open` が `areka-placement` にある（D3 ⑵） | ある。`dpi_sync.rs`・`balloon_limit.rs`・`balloon_lock.rs`・`lib.rs` の 4 か所で、`#[cfg(feature = "test-support")] pub use test_open::…` と `#[cfg(not(...))] pub(crate) use test_open::…` の形。定義は `mod test_open { use super::*; … }` の**中へ移す**形である（下の 重大な問題 2 に効く）。 |
| `#[used]`・`.CRT$XCU` の初期化子が、rlib の依存から本体のテストの実行ファイルへ入る（D6） | 未実測（設計も「未実測」と書き、本体のテスト 1 本で赤として気づく形と、代案〔4 つ目の写し〕へ戻る道を置いている）。今の `crash_report_test_support.rs` 自身の注は「テストの実行ファイルの根のクレートの中に置くので、リンクで落ちない」と、根の crate に置くことを落ちない理由にしている。加えて、残る場合に別の問題が出る（重大な問題 1）。 |
| 999 行のファイルが 1,000 行以下に収まる（D9・FileLength） | 番人の判定は `lines > 1000`（`workspace_scan/mod.rs` 149 行）なので 1,000 行ちょうどは通る。`frame/switch_finish_tests.rs`（999 行）は D9 で「伸びない」とされる一方、D5 の口を据える 15 本にも入っている。World の組み立ては `rig_on` 1 か所（179・866・935 行の `World::new()` はどれも `rig_on` へ渡す）なので、そこへ 1 行足すと 1,000 行ちょうど。道の書き換え（`crate::runtime::` 16 字 → `areka_runtime::` 15 字・6 か所）は短くなる向き。収まるが余裕は 0（軽い指摘 1）。 |
| 口の倒れ先に頼る本体の外のテスト（research 6 章 3 の未確認） | 本体の外で runtime の相を回す重いテスト（`shell_balloon_switch_session_lap_tests`・`mcp/dump_*_gpu_tests`）は `SwitchRig`（`register_systems` で口を据える）を通る。設計は research の未確認の項目を閉じた記録を持たない（軽い指摘 2）。 |
| 本体の統合テストが本体の exe を使う | `crates/areka/tests/` の実機テスト（`smoke_boot_loop_exit.rs`・`emo2_real_run/common.rs`・`stay_on_top_real_run.rs` ほか）は `env!("CARGO_BIN_EXE_areka")`、つまり同じ `cargo test` で組まれた `areka.exe` を起こす。 |

## 重大な問題

🔴 **重大な問題 1**: 落ちの報告の初期化子が、`cargo test` で組まれる `areka.exe` にも入りうる（D6）
**懸念**: D6 は `crash_report_test_support` を `#[cfg(any(test, feature = "test-support"))]` で立て、`#[used]` の初期化子を機能つきの rlib から本体のテストの実行ファイルへ入れる。resolver 2 では、`cargo test`（`tools/test-all.ps1` も同じ）が本体の dev-dependency の `test-support` を `areka-runtime` の 1 回の組み立てにまとめるため、同じ走行で組まれる本体の bin（`CARGO_BIN_EXE_areka`）も機能つきの rlib を結ぶ。D6 が当てにしている「rlib の `#[used]` が結びで残る」が成り立つと、実機テストが起こす `areka.exe` の中で、読み込みの初めに `CoIncrementMTAUsage`・`SetUnhandledExceptionFilter`・最初の受け手（AV を標準エラーへ書く）が走る。成り立たなければ D6 そのものが立たない。どちらの場合も設計の形のままでは済まない。
**影響**: 要件 7.6 の実機 3 本（「振る舞いを変えない」の証跡）が、本番と違う exe（暗黙の MTA で COM の初期化漏れが隠れる・落ちの受け手が差し替わる）で走る。設計の「リリースのビルドには入らない。振る舞いは変わらない」（RuntimeCrate の実装の注）は、実機テストの exe については当たらない。
**提案**: `#[used] #[unsafe(link_section = ".CRT$XCU")] static INSTALL` だけを、`areka-runtime` の側では `#[cfg(test)]` に絞る。`install` は `test-support` で `pub fn` にし、本体の `#[cfg(test)] mod runtime_tests` の `mod.rs` に同じ 3 行の `static`（`= areka_runtime::crash_report_test_support::install`）を置く。中身の写しは作らず（3.3）、置く所はテストの実行ファイルの根の crate（今の注の理由と同じ）になり、bin には入らない。`installed()` の確かめ 1 本はそのまま使える。
**対応する要件**: 3.3・3.8・6.8・7.1・7.6
**根拠**: design.md「設計の段で決めたこと」D6・「TestBinaryGuard」・「RuntimeCrate」の実装の注（危うさ）・「申し送りと危うさ」3

🔴 **重大な問題 2**: `test_open` に関数を移すと、その中のログの出所の名前が変わる（D3 ⑵）
**懸念**: 家の型 `test_open` は、定義そのものを `mod test_open { use super::*; … }` の中へ移す（`areka-placement` の 4 か所で確かめた）。D3 ⑵ は「私有か `pub(super)` の関数・型・定数」をこの型で開くとするが、tracing の既定の出所は `module_path!()` なので、移した関数の中の `error!` ほかの出所は `areka_runtime::frame::switch::test_open` のように `::test_open` が付く。たとえば本体へ移る `frame/switch_tests.rs` は `use super::run_switch_phase;`（28 行）で私有の相の関数を使い、`run_switch_phase` には `error!` がある（`frame/switch.rs` の 113・125 行あたり）。
**影響**: 本番のログの行の出所が、要件 6.1 が決めた形（`areka_runtime::<モジュールの道>`）からずれる。`RUST_LOG` の前方一致は通るが、出所を逐語で比べるテスト（`shell_cursor_assets_tests` の `TARGET` ほか）・ログの行の見本・`get_log` での見え方が変わり、要件 6.8・7.1 の「ログの中身は出所の名前のほかは変えない」に反しうる。
**提案**: D3 ⑵ の対象を、tracing の呼び出しを中に持たない項目（型・定数・ログを出さない関数）に限ると書き足す。ログを出す関数は ⑷ の渡し口（`test_support` の `pub fn` から元の関数をそのまま呼ぶ）で開き、本体へ移すテストの呼び方を 1 か所ずつ書き換える。段 3 の確かめに「`test_open` の中に `info!|debug!|warn!|error!|trace!|#[instrument]` が 0 件」の grep を足す。
**対応する要件**: 6.1・6.4・6.8・7.1（2.1 の可視性の決まりと D3）
**根拠**: design.md「設計の段で決めたこと」D3 ⑵・「BodyRuntimeTests」・「設計の討議へ回すこと」1

## 軽い指摘（重大ではない・討議で拾えばよい）

1. **`frame/switch_finish_tests.rs` の行数の書き方**: D9・FileLength は「`crate::runtime::` の書き換えだけ＝伸びない」とするが、同じファイルが D5 の 15 本にも入る。`rig_on` に 1 行足して 1,000 行ちょうど（番人は `> 1000` で赤なので通る）。D3 ⑷ の書き換えが 1 行でも増えれば超えるので、段 6 の確かめの対象に名指しで入れ、表の記述を「1 行増えて 1,000 行（余裕 0）」に直すとよい。
2. **research 6 章 3 の未確認（本体の外で口を据えずに口を読む相を回すテスト）を閉じた記録が無い**: 今回の確かめでは、重いテストは `SwitchRig`（`register_systems`）を通っていた。設計に「段 3 で `session_hooks_missing` が本体のテストの記録に 0 件」を確かめに足すと、据え忘れが記録つきの `NO_BODY` として黙って通る道も閉じる。

## 設計の強み

1. **公開面をコンパイラに数えさせる段の組み方（D2）**: 段 1 だけの一時の別名 `use areka_runtime as runtime;` で、E0603・E0616・E0624・E0364・`private_interfaces` から `pub` にする項目を 1 件ずつ出させ、段 4 で別名を消す。数え漏れがコンパイルの赤として出るので、「外から使われる分だけ `pub`」（2.1・2.2）を手の数えに頼らずに守れる。
2. **黙って通る形を残さない作り（「失敗の扱い」・D1・D9）**: 代わりの親で子のテストの中身をほぼ変えずに移し（999 行のファイルを伸ばさない）、口の据え忘れ・読み先の欠け・木の走査の狭まり・番人の表の古いパスを、どれもコンパイルかテストの赤に結び付けている。多重集合の見込みの差（3 件）も先に表にしてある。

## 最終の判定

- **判定: GO**（重大な問題 2 件を設計の討議で決めることが条件）
- **理由**: 境目・依存の向き・前例の型との合致は確かで、要件の全項目に実装の道筋がある。重大な問題 2 件はどちらも数行の直しで閉じられ、構成を作り直す必要はない。
- **次にすること**:
  1. `/kiro-design-discussion runtime-crate-split` で、重大な問題 1（初期化子の `static` を本体の `#[cfg(test)]` に置くか）と 2（`test_open` の対象からログを出す関数を外すか）を、設計の「設計の討議へ回すこと」1・4 と合わせて決める。
  2. 決めた形を design.md の D3・D6・TestBinaryGuard・FileLength に書き戻す。
  3. `/kiro-spec-tasks runtime-crate-split` でタスクを作る。
