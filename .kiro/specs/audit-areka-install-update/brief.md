# Brief: audit-areka-install-update

> 2026-10-19 `/kiro-discovery` で起票。開発者「今相談した、ポニーテールによるコードの見直しを、なるべく並行作業が出来るように複数スペックに起票して欲しい。とにかく指摘事項は全部直す方針です。」。同じ日の問いへの答え＝指摘を洗い出すのは各 spec の着手のとき・版の段は「今やる！最優先！」（同じ日のうちに改め＝v0.1.0 の直後の版 v0.1.1 へ移した・開発者「えーっと、ponytailをすぐかけたい！って言ってましたが、0.1.0（ぱすたが動くまで）の直後、0.1.1フェーズに変更してください。」・2026-10-22 さらに改め＝版の段から外し、v0.1.0 の後に隙間で取る優先度が高い列へ移した＝開発者「audit系スペックのロードマップ優先度を、Ver0.1.0が終わった後の、優先度が高いいつでもやれる項目とせよ。基本的に0.1.0が終わった後、隙間で実施したい。」・v0.2.0 の作り直しと重なる分は作り直しの前に済ませる＝roadmap の「版の段」が正本）・範囲は本番のクレート全部と開発の道具のクレートと `tools/` の台本と `pilot`。全 33 本のうちの 1 本で、分け方と共通の約束は roadmap の「テーマ別の決めごと」の「ponytail による見直し」が正本。行数は起票のとき（main `e3c2cbe4`）の実測＝着手のときに測り直す。

## Problem

開発者は、areka のコードに残っているバグ・危ない所・負荷に弱い所・テストの抜け・遅い所・要らないコードを、一度まとめて直したい。今までは、各 spec が自分の触る所だけを見てきた。全体を同じ物差しで見直したことはない。物差しは ponytail のプラグイン（5.3.2・利用者全体に入れてある）の `/ponytail:ponytail-audit`。この spec は **areka 本体のインストール・ネットワーク更新・MCP の受け口・根の一覧・説明書** を受け持つ。

## Current State

- 受け持つファイル: `crates/areka/src/` の `install*`・`update*`・`mcp*`・`roots*`（`roots.rs`・`roots/`・`session/roots/`）・`session/readme.rs`・`session/readme/`・`session/readme_tests.rs`（フォルダとファイル）。
- 大きさ: 107 ファイル・コード 8,922 行・テスト 21,355 行（`_tests.rs`・`tests/`・`_test_support.rs` をテストに数えた）。
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

- **Upstream**: ponytail のプラグイン 5.3.2（`~/.claude/plugins`・`/ponytail:ponytail-audit`）。前に着地を待つ spec＝`mcp-reload`（v0.1.0・2026-10-19 に着手中）。同 spec が `mcp/reload.rs`・`update/desk.rs` に触るので、その着地の後に着手する（着手のときに main を取り込んでから audit をかける）。2026-10-28 `/kiro-discovery` で足した待ち＝`session-crate-split`（本体の crate 分けの 6 本目）。`runtime-module-untangle` で `install_cue`・`update_cue`・`readme_cue` が `install/`・`update/`・`readme/` へ入り、`session-module-gather` で roots の芯（`mod`・`resolve`・`report`・`user_root`）と readme が `crates/areka/src/session/` へ、`session-crate-split` で `crates/areka-session/src/` へ移る（install・update・mcp と roots の `edit`・`desk`・`pick` は本体に残る）。各 spec がこの brief の「受け持つファイル」を書き換える。
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

## 追記（2026-11-06 `audit-sylphya-actor` の要件の討議で回された件）

`audit-sylphya-actor` の audit の指摘のうち、呼び手の乗り換えがこの spec の範囲に当たる 1 件を回した（2026-11-06 開発者の承認・`audit-sylphya-actor` の要件の討議 議題 4）。

- **件（audit #4）**: `SylphyaPublisher::barrier()` は期限の無い待ち。`crates/areka/src/install/desk.rs`（`runtime.sylphya_publisher().barrier()`）は UI スレッドから呼ぶので、書き手のアクターが遅い書き込みの最中だと UI が止まる。`audit-sylphya-actor` が足す期限つきの `barrier_within(Duration)` へ乗り換え、期限切れは記録を残して続ける。

## runtime-module-untangle からの申し送り（2026-11-13）

`runtime-module-untangle`（`emo2_boot` を `runtime` と `session` へ分けた spec）は振る舞いを変えなかった。作業中に直したくなった所は直さず、ここへ書いた。行番号は 2026-11-13（`93aca049f`）のもので、着手のときに確かめ直す。

- **気を付ける所: `update::update_cue`・`readme::readme_cue` は記録の決まりの前方一致の下に入った**。`crates/areka/src/log_history.rs` の `areka::update`（Update）・`areka::readme`（Status）に当たる。今は 2 つとも `info!` を持たないので記録の分け方は前と同じだが、直しで `info!` を足すと記録に残るようになる。足すなら `UNKEPT_TARGETS` へ足すか、残すのが正しいかを決める（`areka::install::install_cue` は `UNKEPT_TARGETS` に入れてある）。

## 2026-11-21 棚卸㊶の再測定（main `31ac3eaf`）
- **前提の変化**: 起票の後に測り直した節は無かった。待ちに挙げた `mcp-reload`（`areka-P0-mcp-reload`）・`runtime-module-untangle`・`audit-sylphya-actor` は着地済み＝残る待ちは `session-crate-split`（その前に `session-module-gather`）だけ。`audit-sylphya-actor` の `barrier_within(Duration)` は `crates/areka-sylphya/src/actor.rs:562` に在るが、`install/desk.rs:377` はまだ `barrier()`＝追記の件は残る。`input-crate-split`（PR#486）が触ったのは `install/mod.rs`（説明の 1 行）と `update/worker_tests.rs`（字面の見張りに `areka-input` を足した）だけ。PR#488 はコメントだけ。`log_history.rs:136` の `UNKEPT_TARGETS`（`areka::install::install_cue` だけ）は申し送りのまま。
- **触るファイル**: `crates/areka/src/` の `install*`・`update*`・`mcp*`・`roots*`・`readme*`。今の数えは 121 ファイル・約 35,600 行（`wc -l`・本番とテストの計。起票の 107 ファイル・30,277 行の約 1.2 倍）。上限に近いのは `update/worker_tests.rs` 967・`install/startup_tests.rs` 854。
  - `mcp/dump_balloon.rs`（216 行）は今も範囲の中＝`balloon-text-visual-tree` はこの spec の後（roadmap の見込み C28 のまま）。
  - `session-module-gather` で roots の芯（`mod`・`resolve`・`report`・`user_root`）と readme が `session/` へ、`session-crate-split` で `crates/areka-session/src/` へ移る。
- **ほかの brief から指される件**: `audit-areka-app` の申し送り（`audit-emo2-boot-frame` の 6 節コ・サ・シ）が、この範囲の 4 ファイルを挙げる＝`install/names_tests.rs:28`（`let _ = parts.handle.join();`）・`readme/readme_cue_tests.rs:24`（`capture_logs` の写し）・`install/desk_tests.rs:726`・`readme/opener_submit_tests.rs:31`（`boot_context` の写し）。この spec が直す形にしないと、C26 で `audit-areka-app` と同じファイルに触る。
- **規模**: 指摘の数で決まる（20 を越えたら指摘の組で同じ範囲の 2 本目）。
- **先に要るもの**: `session-crate-split`。範囲の `install/desk.rs`・`update/desk.rs`（C3〜C5 の口）・`install/names.rs`・`install/install_cue.rs`・`update/update_cue.rs` を向こうが書き換え、`mcp/` のパスの直しと roots・readme の移動は `session-module-gather` が行う＝先には始められない。見込み C26。
- **同じウェーブに置けない相手**: crate 分けの 2 本・`doc-compat-architecture-rename`（`install/judge.rs:90` が文書を名指す）・`balloon-text-visual-tree`（`mcp/dump_balloon.rs`・待つ側）。`audit-areka-app`・`audit-emo2-boot-switch` とは、上の件を持ち主が直せば 0。
- **優先度の区分**: A（開発者「とにかく指摘事項は全部直す方針です」）。要件定義のモデル: Opus（見直しの型どおり）。分割の案: 無し。
- **古くなった記述**: Upstream の「`mcp-reload`（v0.1.0・2026-10-19 に着手中）」は着地済み。Current State の大きさ（107 ファイル）は起票のときの数え。

## 申し送り（session-module-gather より・2026-11-22）

- **前提の変化**: `readme`・`roots` の芯は `crates/areka/src/session/readme*`・`crates/areka/src/session/roots/` へ移った（持ち主は変わらずこの spec）。`src/roots.rs` は新しい薄い入口。詳しくは `.kiro/specs/completed/session-module-gather/research.md` の §14.6。
- **件 1（所見の文のパス）**: この brief の所見の文（2026-11-13 の申し送りの節ほか）の移す前のパスは、要件の段で `session/…` へ読み替える。
- **件 2（呼び手の説明）**: `crates/areka/src/install/mod.rs:132`・`crates/areka/src/update/mod.rs:224` の doc は呼び手を `ghost_session::register_systems` と書く。関数ポインタ経由で今も正しいが、名目上の渡し手は `main.rs` の `BODY_SYSTEMS`。直すかどうかを見直しで決める。
- 完了の時にその場で直したもの: `crates/areka/src/update/worker.rs:40`・`:110` の `exe_dir()` のパス。
