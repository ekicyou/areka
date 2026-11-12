# Brief: audit-shiori-host32

> 2026-10-19 `/kiro-discovery` で起票。開発者「今相談した、ポニーテールによるコードの見直しを、なるべく並行作業が出来るように複数スペックに起票して欲しい。とにかく指摘事項は全部直す方針です。」。同じ日の問いへの答え＝指摘を洗い出すのは各 spec の着手のとき・版の段は「今やる！最優先！」（同じ日のうちに改め＝v0.1.0 の直後の版 v0.1.1 へ移した・開発者「えーっと、ponytailをすぐかけたい！って言ってましたが、0.1.0（ぱすたが動くまで）の直後、0.1.1フェーズに変更してください。」・2026-10-22 さらに改め＝版の段から外し、v0.1.0 の後に隙間で取る優先度が高い列へ移した＝開発者「audit系スペックのロードマップ優先度を、Ver0.1.0が終わった後の、優先度が高いいつでもやれる項目とせよ。基本的に0.1.0が終わった後、隙間で実施したい。」・v0.2.0 の作り直しと重なる分は作り直しの前に済ませる＝roadmap の「版の段」が正本）・範囲は本番のクレート全部と開発の道具のクレートと `tools/` の台本と `pilot`。全 33 本のうちの 1 本で、分け方と共通の約束は roadmap の「テーマ別の決めごと」の「ponytail による見直し」が正本。行数は起票のとき（main `e3c2cbe4`）の実測＝着手のときに測り直す。

## Problem

開発者は、areka のコードに残っているバグ・危ない所・負荷に弱い所・テストの抜け・遅い所・要らないコードを、一度まとめて直したい。今までは、各 spec が自分の触る所だけを見てきた。全体を同じ物差しで見直したことはない。物差しは ponytail のプラグイン（5.3.2・利用者全体に入れてある）の `/ponytail:ponytail-audit`。この spec は **32bit SHIORI の中継と SHIORI の ABI・試験用の DLL** を受け持つ。

## Current State

- 受け持つファイル: `crates/shiori-host32-host/`・`shiori-host32-helper/`・`shiori-host32-ipc/`・`shiori-abi/`・`shiori4-testdll/`・`shiori-host32-testdll/`・`shiori-host32-testdll-loadu/` 全部。
- 大きさ: 51 ファイル・コード 9,500 行・テスト 5,005 行（`_tests.rs`・`tests/`・`_test_support.rs` をテストに数えた）。
- まだ audit をかけていない。指摘はギャップ分析で洗い出す（起票のときに出さなかったのは、着手までにコードが変わって指摘が古びるため）。

## Desired Outcome

- 受け持つ範囲に `/ponytail:ponytail-audit` をかけて出た指摘が、3 つの組（必ず直す・直したほうがよい・できれば）とも全部直っている（開発者「とにかく指摘事項は全部直す方針」）。
- 直した後に同じ範囲へもう一度 audit をかけて、新しい指摘が 0 件（直しが生んだ指摘も直す）。
- 全体テスト（`pwsh -NoProfile -File tools/test-all.ps1`）が緑。直したバグ・危ない所には、それが壊れたら落ちるテストが 1 本ずつある。

## Approach

`/kiro-start` の各段で、次のように進める（2026-10-19 開発者「原則、ギャップ分析でauditを実施して、設計以降で修正」）。

- **要件**: 「受け持つ範囲の audit の指摘を、3 つの組とも全部直す」という形だけを書く。指摘の中身はまだ書かない。
- **ギャップ分析**: 受け持つ範囲だけを対象にして `/ponytail:ponytail-audit` をかける。指摘の一覧はギャップ分析の報告に残す。番号は audit の番号をそのまま使う。audit の作法のうち 2 つは外す。
  - 「最大 20 件」は外す。全件を出し、省いた件を残さない。
  - 「読まなかった所（Not checked）」は残さない。範囲のファイルは全部読む。
- **要件の討議**: 指摘の一覧を要件に落として、直す対象を確定する。直さずに残す 3 種類（「Constraints」）は、ここで議題にする。
- **設計**: 指摘ごとの直し方を決める（何を消すか・どこにまとめるか・範囲の外の呼び出し元への影響）。直し方は ponytail の「いちばん小さな完全な変更」に従う。消せるものは消し、呼び出し元・テスト・fixture まで直し切る。
- **タスク・実装**: 直す。最後に同じ範囲へ audit をかけ直して、0 件を確かめる（検証の段のタスク）。

## Scope

- **In**: 受け持つファイルの指摘の直し・そのためのテストの追加と直し・直しのために要る範囲の外の呼び出し元の直し（下の「Constraints」の約束に従う）。
- **Out**: 範囲の外の見直し（ほかの 32 本の持ち物）・新しい機能・`vendors/`・clippy の指摘の機械的な直し（`clippy-199-lints` の持ち物。audit が同じ所を指したら、その指摘としては直してよい）。

## Boundary Candidates

- 指摘の組（必ず直す・直したほうがよい・できれば）ごと、またはファイルのまとまりごとにタスクを切る。

## Out of Boundary

- ほかの 32 本の範囲のファイル（直しが及ぶときの約束は「Constraints」）。
- 開発者の裁定（roadmap の「生きている決まり」・記憶の決めごと・完了した spec の要件）を覆す直し。直さずに議題にする。

## Upstream / Downstream

- **Upstream**: ponytail のプラグイン 5.3.2（`~/.claude/plugins`・`/ponytail:ponytail-audit`）。前に着地を待つ spec＝なし（すぐ着手できる）。
- **Downstream**: なし。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `clippy-199-lints`（同じクレートに触る）。同じ範囲に触る未着手の spec は、着手のときに roadmap の spec 台帳で確かめる。

## Constraints

- **範囲の外へ直しが及ぶとき**: 関数の形を変えたら、ツリー全体で呼び出し元を探して全部直す。相手の範囲の見直しの spec が走っていれば、そのセッションと申し合わせてから触る。
- **直さずに残してよいのは次の 3 つだけ**。どれも要件の討議で開発者の裁定を仰ぐ。
  1. 開発者の裁定に反する直し。
  2. わざと置いた先送りの印。`#[allow(dead_code)]` に実在の理由の doc を付けた語彙の口（実装規律「語彙完備・配線ゼロの追跡」）と、`shortcut:`・`ponytail:` の注釈で限界が今の負荷を越えていないものがこれに当たる。
  3. 外から見た振る舞いが変わる直しのうち、SSP との互換や ukadoc の正典に関わるもの。ukadoc で確かめてから決める。
- **守る決まり**:
  - 1 ファイル 1,000 行の番人を越えない。足すなら新しいファイルへ。
  - 決定論のテストで網羅する。
  - 記録の無い失敗の道を作らない（`error!`＋`Err`）。
  - 終了の道は正規の実装にする。
  - 実機の根と一時フォルダは、ワークツリーの `target` の下に置く。
  - GPU のテストは実 GPU で回す。
  - 文字化けをコンソールの文字コードで逃げない。
  - 依存を足すなら `tech.md` へ登記して、開発者の承認を得る。
- 消す前に、ツリー全体（テスト・fixture・設定・文字列での参照を含む）で使われていないことを確かめる。
- 1 spec 20 タスクを越えそうなら、要件の討議で、指摘の組ごとに同じ範囲の 2 本目として切ってよい。

## 2026-10-23 `shiori4-api` からの申し送り

- 開発者の決定（10-23「COM-SHIORI インターフェースの作りかけは排除し、shiori4 に完全移行」）で、`crates/shiori-abi/`（COM の面）と `crates/shiori4-testdll/` は `shiori4-api` が消す。この spec が着手するときに、まだ残っていれば受け持ちから外してよい（直しても消えるファイル）。

## audit-ghost-catalog の申し送り（2026-11-06 同 spec の完了の棚卸で追記）

- **件 1: i686 の helper が素の `LoadLibraryW` で SHIORI の DLL を読む**。`crates/shiori-host32-helper/src/shiori_proxy.rs` は依存の探し先を絞っていない。`audit-ghost-catalog` は同じプロセス内の読み込み（`crates/areka-ghost/src/shiori_inproc.rs`）を `LoadLibraryExW` と `LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_DEFAULT_DIRS` へ直した（同 spec の #28）。helper の側も同じ形へそろえる。
- **件 2: helper が親の環境変数をそのまま受け継ぎ、`PASTA_DEBUG` が立っていると pasta が 127.0.0.1:9276 を取り合う**。`crates/shiori-host32-host/src/process_host.rs`（262〜272 行目あたり）は環境を消さずに helper を起こす。`crates/real-run-kit/src/launch.rs`（213〜214 行目）は子へ `PASTA_DEBUG=0` を渡して避けている。`audit-ghost-catalog` は実物の pasta のテストを錠で 1 本ずつにして当座をしのいだ（`crates/areka-ghost/tests/ghost/common.rs` の `real_pasta_turn`）。helper を起こす所で `PASTA_DEBUG=0` を渡すか、渡す口を設けるかを決める。
- 起票の訳（2 件とも）: 持ち主のこの spec の受け持ちの範囲と重なる。

## 2026-11-13 棚卸㊱の再測定（main 069f1a48）

- 前提の変化: 範囲の 7 クレートは前の棚卸（main `18e4cb30`）から変わっていない（`placement-crate-split`・`wintf-visual-tree` も触っていない）。`shiori4-api`（COM の面 `shiori-abi` と `shiori4-testdll` を消す）はまだ着手していない＝2 つは残っている。上の申し送りどおり受け持ちから外してよく、外せば COM の面の使い手 `crates/areka/src/{reference_brain,shiori_demo,shiori_host,shiori_session}.rs` に及ぶ直しも出ない。`audit-ghost-catalog` の申し送り 2 件は今も当たる＝helper の `crates/shiori-host32-helper/src/shiori_proxy.rs` 97 行目は素の `LoadLibraryW`、host の `crates/shiori-host32-host/src/process_host.rs` 270〜272 行目は環境を消さずに足すだけで `PASTA_DEBUG` を渡していない。
- 触るファイル: 7 クレートの .rs 46 本・15,736 行（起票の 51 ファイル・14,505 行）。`shiori-abi`・`shiori4-testdll` を外すと 37 本・12,836 行。1,000 行に近いのは `shiori-host32-host/src/client.rs` 957 行・`shiori3.rs` 879 行・`shiori-host32-helper/src/shiori_proxy.rs` 819 行＝足すなら新しいファイルへ。
- `crates/areka/` の約束（C19「見直しは `crates/areka/` に触らない」）: 範囲の中だけの直しなら触らない。`shiori-host32-host` の公開の型（`ExitKind`・`HelperStatus`・`RequestError`・`ShutdownError`）を変えると `crates/areka/src/emo2_boot/spine.rs` 75 行目・`spine_hold_support.rs` 25 行目とそのテストに及ぶ＝同じ C19 の `runtime-module-untangle` が移す所。公開の型は変えない直し方を設計で選ぶ（件 2 の `PASTA_DEBUG=0` も `process_host.rs` の `spawn` の中で閉じられる）。本体の外の使い手（`areka-ghost`・`areka-kanade`・`pilot`）は C19 のほかの spec と重ならない。
- 規模: 指摘の数で決まる（20 を越えたら要件の討議で同じ範囲の 2 本目へ）。
- 先に要るもの: 無い。C19 の見込みのほかの 7 本とは、公開の型を変えない限り重なり 0。
- 優先度の区分: A（開発者 10-19「とにかく指摘事項は全部直す方針です」）。
- 要件定義のモデル: Opus（見直しの型が決まっている・新しい境界も順序の約束も作らない）。
- 分割の案: 無し（要るなら指摘の組で 2 本目）。
- 見つけた穴・古くなった記述: Current State の大きさ（51 ファイル・14,505 行）は古い＝上の数。
