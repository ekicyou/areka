# Brief: audit-areka-input-menu

> 2026-10-19 `/kiro-discovery` で起票。開発者「今相談した、ポニーテールによるコードの見直しを、なるべく並行作業が出来るように複数スペックに起票して欲しい。とにかく指摘事項は全部直す方針です。」。同じ日の問いへの答え＝指摘を洗い出すのは各 spec の着手のとき・版の段は「今やる！最優先！」（同じ日のうちに改め＝v0.1.0 の直後の版 v0.1.1 へ移した・開発者「えーっと、ponytailをすぐかけたい！って言ってましたが、0.1.0（ぱすたが動くまで）の直後、0.1.1フェーズに変更してください。」・2026-10-22 さらに改め＝版の段から外し、v0.1.0 の後に隙間で取る優先度が高い列へ移した＝開発者「audit系スペックのロードマップ優先度を、Ver0.1.0が終わった後の、優先度が高いいつでもやれる項目とせよ。基本的に0.1.0が終わった後、隙間で実施したい。」・v0.2.0 の作り直しと重なる分は作り直しの前に済ませる＝roadmap の「版の段」が正本）・範囲は本番のクレート全部と開発の道具のクレートと `tools/` の台本と `pilot`。全 33 本のうちの 1 本で、分け方と共通の約束は roadmap の「テーマ別の決めごと」の「ponytail による見直し」が正本。行数は起票のとき（main `e3c2cbe4`）の実測＝着手のときに測り直す。

## Problem

開発者は、areka のコードに残っているバグ・危ない所・負荷に弱い所・テストの抜け・遅い所・要らないコードを、一度まとめて直したい。今までは、各 spec が自分の触る所だけを見てきた。全体を同じ物差しで見直したことはない。物差しは ponytail のプラグイン（5.3.2・利用者全体に入れてある）の `/ponytail:ponytail-audit`。この spec は **areka 本体の入力の出来事と右クリックメニュー** を受け持つ。

## Current State

- 受け持つファイル: `crates/areka/src/` の `input_events*`・`menu*`（フォルダとファイル）。
- 大きさ: 60 ファイル・コード 6,072 行・テスト 12,976 行（`_tests.rs`・`tests/`・`_test_support.rs` をテストに数えた）。
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

- **Upstream**: ponytail のプラグイン 5.3.2（`~/.claude/plugins`・`/ponytail:ponytail-audit`）。前に着地を待つ spec＝`input-crate-split`（2026-10-28 `/kiro-discovery`・本体の crate 分けの 4 本目）。受け持つファイルのうち `input_events/` はその着地で `crates/areka-input/src/` へ移り（`shortcut.rs` は `crates/areka/src/menu/` へ・`file_drop.rs` は本体に残る）、menu は本体に残る。同 spec がこの brief の「受け持つファイル」を書き換える。
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

## 2026-11-01 棚卸㉝の後の開発者の決め

2026-11-01 開発者の承認（棚卸㉝の後・ツールチップは「境界領域です。リアクター描画にした方が良い可能性がある。WinUIで描画できる要素ですからね」）＝範囲から右クリックメニュー（`menu/`）の部分を外し、入力の出来事だけにする。メニューはリアクターが来れば作り直すので、見直しても捨てることになる。外した部分は windows-reactor 待ちの段の申し送り。範囲のファイルの一覧は要件の段で直す。

## 2026-11-17 棚卸㊴の申し送り（`collisionex-regions` から）

- `crates/areka/src/input_events/mod.rs` 518 行の説明「The Hand／collisionex は実装しない（7.4）」は、`collisionex-regions`（C22）が着地すると古くなる。`collisionex-regions` は areka 本体のファイルに触らない約束で C22 に入ったので、この 1 行はこの spec が直す（`input-crate-split` の後なら新しい crate の側の同じ行）。

## 2026-11-20 棚卸㊵の再測定（main 67e0c70e）
- **前提の変化**: `collisionex-regions`（11-17・`2a9fa0a0f`）が着地し、上の申し送りの説明は今は誤り＝`crates/areka/src/input_events/mod.rs:518`「The Hand／collisionex は実装しない（7.4）」（直すのはこの spec・`input-crate-split` の後なら新しい crate の同じ行）。`audit-placement-follow-residue`（11-17）は `input_events/drag_test_support.rs`・`drag_tests.rs` を、`audit-emo2-boot-spine`（11-19）は `balloon_choice_settle_tests.rs` を直した（テストの支えの寄せ・振る舞いは同じ）。
- **触るファイル**（2026-11-01 の決めで menu を外した後）: `crates/areka/src/input_events/` の 50 ファイル（本番 20・5,374 行／テスト 30・10,964 行・`wc -l`）。長いのは `mod.rs` 612・`choice_drain.rs` 550・`balloon.rs` 464。`input-crate-split` の後は `crates/areka-input/src/` へ移り、`shortcut.rs` は本体の `menu/` へ、`file_drop.rs` は本体に残る。
- **規模**: 指摘の数で決まる（20 を越えたら指摘の組で同じ範囲の 2 本目へ）。
- **先に要るもの**: `input-crate-split`（未完了・見込み C23・範囲の全部を動かす）。✅ `audit-placement-follow-residue` は済み。
- **同じウェーブに置けない相手**: `input-crate-split`（待つ）。範囲に `file_drop.rs` を残すなら `session-module-gather`・`session-crate-split` とも重なる（`file_drop.rs:14` の `use crate::ghost_session::GhostSlot` のパスを 2 本とも直す）＝見込みの C25 で `session-crate-split` と並べられるのは、`file_drop.rs` を範囲から外すか、`session-crate-split` が出し直しを置いて `file_drop.rs` に触らないときだけ。外すなら C24（`session-module-gather` の隣）にも上げられる。
- **優先度の区分**: A（開発者「とにかく指摘事項は全部直す方針です」）。
- **要件定義のモデル**: Opus（見直しの型どおり）。
- **分割の案**: 無し（指摘の数しだいで要件の討議で切る）。
- **見つけた穴**: ⑴ `shortcut.rs`（キーの近道・本番 83 行）が `menu/` へ移ると、menu を外した決めのため、どの見直しの範囲にも入らなくなる＝要件の段で範囲に残すかを決める。⑵ `file_drop.rs` が本体に残るときの範囲（上）。⑶ 本体の `readme.rs:93` の説明が `input_events/choice_drain.rs` のパスを名指す（`input-crate-split` の後に古くなる・その spec の文書のパスの直しに入る）。
