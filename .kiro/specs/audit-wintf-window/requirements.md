# Requirements Document

## Project Description (Input)

開発者は、areka のコードに残っているバグ・危ない所・負荷に弱い所・テストの抜け・遅い所・要らないコードを、一度まとめて直したい（2026-10-19 開発者「とにかく指摘事項は全部直す方針です。」）。今までは各 spec が自分の触る所だけを見てきて、全体を同じ物差しで見直したことはない。物差しは ponytail のプラグイン（起票のときは 5.3.2・利用者全体に入れてある・ギャップ分析でかけた版を記録に残す）の `/ponytail:ponytail-audit`。全 33 本の見直しのうち、この spec は **wintf の窓と窓の手続き**（`crates/wintf/src/ecs/window*`＝`window/` と `window_proc/`）を受け持つ。まだ audit をかけていない。指摘はギャップ分析で洗い出し、要件の討議で直す対象を確定し、設計で直し方を決め、実装で直す（開発者「原則、ギャップ分析でauditを実施して、設計以降で修正」）。直した後に同じ範囲へもう一度 audit をかけて新しい指摘が 0 件であること、全体テストが緑であること、直したバグ・危ない所の 1 件ずつに壊れたら落ちるテストがあることを目指す。正本は `brief.md` と roadmap の「テーマ別の決めごと」の「ponytail による見直し」。

## Introduction

この文書は、wintf の窓と窓の手続き（`crates/wintf/src/ecs/window*`）に `/ponytail:ponytail-audit` をかけて出た指摘を、3 つの組（必ず直す・直したほうがよい・できれば）とも全部直すための要件を定める。要件の段では「全部直す」という形だけを書き、指摘の中身は書かない。指摘の一覧はギャップ分析の報告に残し、要件の討議でこの文書に落として直す対象を確定する（要件 1.6）。完了した見直し 2 本から申し送られた 4 件は、ギャップ分析で 1 件ずつ判断し、要件の討議で扱いを決める（要件 2）。

受け持つ範囲の大きさ（着手のときの測り直し・`main` `30a343eb`）:

| 測った時点 | ファイル | うちコード | うちテスト | コード | テスト |
|---|---:|---:|---:|---:|---:|
| 起票のとき（`e3c2cbe4`・brief の値） | 64 | — | — | 11,314 行 | 20,426 行 |
| 着手のとき（`30a343eb`） | 69 | 27 | 42 | 10,599 行 | 20,281 行 |

着手のときの数は、git の管理下のファイルを数え、行は空行を除いて数えた（起票のときの数とは数え方が違うことがある）。テストに数えたのは、名前が `_tests.rs` で終わるファイル・`tests/` の下のファイル・名前が `_test_support.rs` で終わるファイルと、`window/window_pos/tests.rs`。69 ファイルは全部 Rust のファイルで、`window/` の下が 51、`window_proc/` の下が 18。1 ファイル 1,000 行の番人の例外表に載るファイルは 0 本。1,000 行に近いのは `window/zorder_pair_maintain_tests.rs`（978 行）・`window/zorder_chain_order_tests.rs`（977 行）・`window/command.rs`（941 行）ほか、900 行を越えるものが 8 本。

## Boundary Context

- **In scope**:
  - `crates/wintf/src/ecs/window*`（`window/` と `window_proc/`）の全ファイルに audit をかけて出た指摘の直し。
  - 直しのために要るテストの追加と直し。
  - 直しのために要る、範囲の外の呼び出し元の直し（要件 6 と要件 7 の約束に従う）。
  - 完了した見直しから申し送られた 4 件の判断（要件 2）。
- **Out of scope**:
  - 範囲の外のファイルの見直し（ほかの 32 本の見直しの持ち物。wintf の `runtime/` は完了した `audit-wintf-runtime`、`ecs/drag/`・`ecs/pointer/`・`ecs/clickthrough/` は同じウェーブの `audit-wintf-input` の持ち物）。
  - 新しい機能。
  - `vendors/` の下。
  - clippy の指摘の機械的な直し（`clippy-199-lints` の持ち物。audit が同じ所を指したときは、その指摘として直してよい）。
  - 開発者の裁定（roadmap の「生きている決まり」・記憶の決めごと・完了した spec の要件）を覆す直し。直さずに議題にする（要件 4）。
- **Adjacent expectations**:
  - `areka-placement`（`crates/areka-placement/`）は、この範囲の型と記録の綴り＝`MonitorRecord`（`window/transition_diag.rs`）・`Monitor`（`window/monitor.rs`）・`DESPAWNED_SKIP_TAG`（`window_proc/lifecycle.rs`・綴りは `[despawn-skip]`）・記録の的 `areka_placement::diag`（`window_proc/window_pos_tests.rs` の記録の絞り込み）を使う。同じウェーブ C20 で、それを受け持つ `audit-placement-core`・`audit-placement-follow` が走っている。この spec はそれらを足すことのほかは変えない（要件 7.1〜7.3・C20 の約束 ⑶）。
  - 同じ C20 のほかの 7 本（`wintf-visual-damage-redraw`・`wintf-winmsg-executor-import`・`audit-placement-core`・`audit-placement-follow`・`audit-wintf-input`・`audit-tools-perf-run`・`audit-tools-perf-judge`）とは、受け持つファイルが重ならない（棚卸㊲の測り）。`Cargo.lock` の席は `wintf-winmsg-executor-import` が使う（C20 の約束 ⑸）。
  - `zorder-chain-residue`（未着手）は、この範囲の `window/zorder_pair_maintain_always_on_top_tests.rs`（実窓の重なり順の間欠の赤＝A-1・明示で回す形に隔離済み）と `window/zorder_pair_maintain.rs` の説明（B-6）を扱う予定。audit が同じ所を指したときは、重なりを記録に残し、どちらの spec で直すかを要件の討議で決める（要件 1.7）。
  - `instant-to-interrupt-ticks`（v0.2.0）は、この範囲の `window/command.rs`・`window_proc/window_pos.rs` ほかに触るので、この spec の着地を待つ（roadmap の spec 台帳）。

## Requirements

### Requirement 1: 指摘の洗い出しと直す対象の確定

**Objective:** As a 開発者, I want 受け持つ範囲の指摘が 1 件も漏れずに一覧になり、直す対象が自分の裁定で確定していること, so that 「全部直す」の「全部」が何件かが確かめられ、設計と実装が直す対象をはっきり知っている

#### Acceptance Criteria

1. When ギャップ分析を始めるとき, the 本見直し shall `crates/wintf/src/ecs/window*` の下の git の管理下のファイルを全部数え、その一覧と数を記録に残す
2. If 範囲のファイルが、着手のときより後に（ほかの spec の着地などで）増えたり減ったりしたとき, the 本見直し shall 一覧と数を測り直し、増えたファイルも見直しの対象に入れる
3. When ギャップ分析を行うとき, the 本見直し shall 要件 1.1 の範囲のファイルだけを対象にして `/ponytail:ponytail-audit` をかける
4. The 本見直し shall audit の作法のうち「最大 20 件」を外し、指摘を件数の上限なしで全件出して、省いた件を残さない。また「読まなかった所（Not checked）」を外し、範囲のファイルを全部読んで、読まなかった所を残さない
5. The 本見直し shall 指摘の一覧を、audit の番号と 3 つの組（必ず直す・直したほうがよい・できれば）を付けたままギャップ分析の報告に残し、3 つの組それぞれの件数を 0 件の組も数字で書く
6. When 要件の討議で直す対象が確定したとき, the 本見直し shall 確定した指摘の一覧（audit の番号・組・直すか裁定で残すか）をこの文書に書き加える
7. If 指摘が `zorder-chain-residue` の扱う A-1・B-6 と重なるとき, the 本見直し shall その重なりをこの spec の記録に残し、どちらの spec で直すかを要件の討議で開発者に確かめる
8. If 直す対象の見立てが 1 spec 20 タスクを越えそうなとき, the 本見直し shall 要件の討議で、指摘の組ごとに同じ範囲の 2 本目の spec として切る案を開発者に出す

### Requirement 2: 完了した見直しからの申し送り

**Objective:** As a 開発者, I want 完了した見直しがこの範囲へ渡した 4 件が、ほかの指摘と同じ物差しで判断されていること, so that 「こちらで決める」と渡された件が宙に浮かない

#### Acceptance Criteria

1. When ギャップ分析を行うとき, the 本見直し shall `audit-wintf-runtime` から申し送られた 3 件と `audit-wintf-graphics-world` から申し送られた 1 件の 4 件を、1 件ずつ「指摘にする（audit の番号と組を付ける）」か「指摘にしない（理由を書く）」かに判断して報告に残す
2. The 本見直し shall 4 件として次を扱う: ⑴ 位置だけを指定した窓の生成のときの座標の換算（`window/window_pos/mod.rs` の `to_window_coords_for_creation` の素通し）を今のままでよいか ⑵ 生成に失敗した窓の印 `WindowCreationFailed` を外して立ち直らせる側をどこに置くか（今は一時的な失敗から自動で立ち直らない） ⑶ DPI の変わり目で窓の中心を保つ動き（`taffy_flex_demo` で副モニターの原点へ送った窓が、大きさの差の半分ずれて着く）が正しいか ⑷ `window_proc/window_pos.rs` の 2 つ目の、錠を取らない flush（③）を直す対象に入れるか
3. When 要件の討議を行うとき, the 本見直し shall 4 件それぞれの扱い（直す・直さずに残す・指摘にしない）を確定し、要件 1.6 の一覧に書き加える
4. If 4 件のどれかの直しが範囲の外（wintf の `runtime/` など）に及ぶとき, the 本見直し shall 要件 6 の約束に従って直す

### Requirement 3: 指摘を全部直す

**Objective:** As a 開発者, I want 3 つの組の指摘が全部直っていること, so that wintf の窓と窓の手続きのコードからバグ・危ない所・負荷に弱い所・テストの抜け・遅い所・要らないコードが一度に無くなる

#### Acceptance Criteria

1. The 本見直し shall 「必ず直す」「直したほうがよい」「できれば」の 3 つの組とも、要件の討議で確定した指摘を全部直す
2. The 本見直し shall 要件 4 の 3 種類のどれかに当たって開発者が残すと裁定した指摘と、要件 7.4 で開発者がほかのウェーブへ回すと裁定した指摘のほかは、直さずに残さない
3. The 本見直し shall 各指摘を、その指摘を解く最も小さな完全な変更で直し、消せるものは消す
4. When 指摘の直しで消す・まとめる・形を変える箇所が出たとき, the 本見直し shall その箇所を使う呼び出し元・テスト・fixture・設定まで直し切り、直しの途中の状態を残さない
5. If 直しが wintf の公開の口（クレートの外から使える型・関数・定数）を消したり形を変えたりするとき, the 本見直し shall 変わった公開の口の一覧を記録に残す

### Requirement 4: 直さずに残す指摘は開発者の裁定で決める

**Objective:** As a 開発者, I want 直さずに残す指摘を自分で裁定できること, so that 今までの決まりや互換を壊す直しが勝手に入らない

#### Acceptance Criteria

1. If 指摘の直しが次の 3 種類のどれかに当たるとき, the 本見直し shall その指摘を直さずに要件の討議の議題にし、開発者の裁定を仰ぐ: ⑴ 開発者の裁定（roadmap の「生きている決まり」・記憶の決めごと・完了した spec の要件）に反する直し ⑵ わざと置いた先送りの印を消す直し（`#[allow(dead_code)]` に実在の理由の説明を付けた語彙の口、または `shortcut:`・`ponytail:` の注釈のうち、書かれた限界が今の負荷を越えていないもの） ⑶ 外から見た振る舞いが変わる直しのうち、SSP との互換や ukadoc の正典に関わるもの
2. When ⑶ に当たるかを判断するとき, the 本見直し shall ukadoc の該当箇所（URL と原文の引用）で確かめてから議題にする
3. If 指摘の直しが 3 種類のどれにも当たらないとき, the 本見直し shall その指摘を残すことを議題にせず直す
4. When 開発者が指摘を残すと裁定したとき, the 本見直し shall 裁定とその理由をこの文書に残し、要件 5 の「新しい指摘 0 件」の数から外す

### Requirement 5: 直した後の確かめ

**Objective:** As a 開発者, I want 直しが新しい問題を生んでいないことを、報告ではなく測った結果で確かめられること, so that 見直しが終わったと安心して言える

#### Acceptance Criteria

1. When 確定した指摘を全部直し終えたとき, the 本見直し shall 要件 1.1 で確定した範囲（その後の増減を含む）に、要件 1.4 と同じ作法（件数の上限なし・全部読む）で `/ponytail:ponytail-audit` をもう一度かける
2. If もう一度かけた audit が新しい指摘（直しが生んだ指摘を含む）を出したとき, the 本見直し shall その指摘も要件 3・要件 4 と同じ扱いで直し、新しい指摘が 0 件になるまで audit をかけ直す
3. If かけ直しが 5 回目になっても 0 件に収まらず、5 回目の指摘に「必ず直す」と本番の振る舞いに関わるものが 1 件も無いとき, the 本見直し shall 開発者に一言知らせたうえで、5 回目の指摘を直したところでかけ直しを打ち切り、検証の記録に「5 回目で打ち切った・残りは直し済み」と書く（2026-11-06 `audit-seriko` で開発者が承認した形）
4. The 本見直し shall かけ直しの記録に、読んだファイルの数が範囲のファイルの数と一致することと、回ごとの 3 つの組それぞれの件数（0 件も数字で）と、0 件または打ち切りの判定と、要件 4 で残すと裁定された指摘の番号を書く
5. The 本見直し shall 直したバグと危ない所の 1 件ずつに、その直しが壊れたら落ちるテストを 1 本以上置く
6. The 本見直し shall 足したテストと直したテストを、毎回同じ結果になる（決定論の）テストにする
7. When 見直しを終えるとき, the 本見直し shall 全体テスト（`pwsh -NoProfile -File tools/test-all.ps1`）が緑であることを確かめる

### Requirement 6: 範囲の外へ直しが及ぶとき

**Objective:** As a 開発者, I want 範囲の外への直しが安全に、並んで走るほかの spec とぶつからずに入ること, so that ほかのクレートが壊れず、並行作業が止まらない

#### Acceptance Criteria

1. When 直しで関数や型の形を変えたとき, the 本見直し shall ツリー全体で呼び出し元を探して、全部直す
2. If 直しが及ぶ呼び出し元の範囲を受け持つ spec のセッションが走っているとき, the 本見直し shall そのセッションと申し合わせてから、その呼び出し元に触る
3. When コード・テスト・fixture を消すとき, the 本見直し shall 消す前に、ツリー全体（テスト・fixture・設定・文字列での参照を含む）で使われていないことを確かめ、確かめた結果を記録に残す
4. The 本見直し shall 範囲の外のファイルの、直しに要らない箇所を直さない（範囲の外の指摘を見つけても、その範囲の spec の持ち物として残す）
5. The 本見直し shall `vendors/` の下に手を入れない

### Requirement 7: C20 の並走の約束

**Objective:** As a 開発者, I want 見直しの直しが同じウェーブ C20 のほかの spec の邪魔をしないこと, so that 並走しても互いの作業を壊さない

#### Acceptance Criteria

1. While C20 のあいだ, the 本見直し shall `areka-placement` が使う wintf の型 `MonitorRecord` と `Monitor` を、欄やメソッドを足すことのほかは変えない（名前・欄の名前と型・既にある関数の形を変えず、消さない）（C20 の約束 ⑶）
2. While C20 のあいだ, the 本見直し shall 定数 `DESPAWNED_SKIP_TAG` の名前と綴り（`[despawn-skip]`）を変えず、消さない（C20 の約束 ⑶）
3. While C20 のあいだ, the 本見直し shall 記録の的 `areka_placement::diag` を名指す綴り（`window_proc/window_pos_tests.rs` の記録の絞り込みと、`window_proc/lifecycle.rs` の `DESPAWNED_SKIP_TAG` の説明）を変えず、消さない（C20 の約束 ⑶）
4. If 確定した指摘が要件 7.1〜7.3 の型や綴りを変えずには直せないとき, the 本見直し shall その指摘だけを直さずに議題にし、「`audit-placement-core`・`audit-placement-follow` のセッションと申し合わせて C20 のうちに直す」か「次のウェーブへ回す」かを開発者の裁定で決める
5. The 本見直し shall 依存を足さず、`Cargo.lock` を変えない（C20 の約束 ⑸＝`Cargo.lock` の席は `wintf-winmsg-executor-import`）
6. If 直しに新しい依存が要ると分かったとき, the 本見直し shall 依存を足さずに開発者へ議題として出す（足すなら `tech.md` への登記と開発者の承認が要る）

### Requirement 8: 守る決まり

**Objective:** As a 開発者, I want 見直しの直しがプロジェクトの決まりを守ること, so that 直しが別の決まりを破らない

#### Acceptance Criteria

1. The 本見直し shall 本番のファイルとテストのファイルのどちらも 1 ファイル 1,000 行の番人を赤にせず、直しで行が上限を越えるときは新しいファイルへ分け、番人の例外表へ新しいファイルを足さない
2. The 本見直し shall 記録の無い失敗の道を作らず、失敗は記録（`error!`）を残したうえで呼び出し元へ失敗（`Err`）として返す
3. The 本見直し shall 終了の道を正規の実装にし、終了を早めるための小細工を入れない
4. Where 直しのテストが実機の根や一時フォルダを使うとき, the 本見直し shall それらをワークツリーの `target` の下に置く
5. Where 直しのテストが GPU を使うとき, the 本見直し shall 実 GPU で回し、ソフトウェアの描き手へ置き換えない
6. If 文字化けに出会ったとき, the 本見直し shall コンソールの文字コードを変えて逃げず、読む側を直す
