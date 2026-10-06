# Requirements Document

> 本文の事実は **2026-10-05・本ブランチ**（main `44fc0a61`）で引き直したもの。コードは「何の定義か」（関数名・型名＋ファイルパス）で指し、行番号では指さない。
> 「要件の段での暫定の裁定」の表は、brief が議題として残した点と、要件を書く途中で答えが要った点に対する**推奨案による暫定の確定**であり、要件ディスカッションで覆せる（覆したら該当要件も改める）。

## Project Description (Input)

**誰の何が困っているか**: AI エージェント（Claude Code など）でゴーストを作る人は、MCP の `dump_balloon` でバルーンの絵を続けて撮る。areka を常駐させている利用者は、撮られている間もゴーストがいつもどおり動くことを求める。前の spec（`areka-P0-mcp-dump-images`・2026-10-05 完了）は実機で要件を満たしたが、`dump_balloon` は 1 回ごとに UI スレッドを最大 14.2 ms 塞ぐ（1 フレームの線 16 ms に近い）。続けて撮るとゴーストの描画が 1 フレーム飛ぶおそれがある。ほかに、完了時の棚卸で後始末が 5 件残った（下の「残った件」）。

**今の状態**: 乗算を戻す・重ね合わせ・PNG・base64 は別のスレッド（`mcp-encode`）で行う。UI スレッドに残るのは判断と絵の写しで、バルーンはこれに文字の面の GPU からの読み戻しが加わる（読み戻しは写しと読み出しを 1 回の呼び出しの中で行い、GPU の仕事の終わりを UI スレッドで待つ）。実機（配布用のビルド・拡大率 200%）の UI スレッドの側は、キャラクター 1 枚で最大 0.31 ms、バルーンで 5 回 1,436〜14,233 µs（最大は最初の 1 回）。

**何を変えるか**: `dump_balloon` の UI スレッドの側を、文字の面の大きさに依らず線から十分に離す（答えが後のフレームになるのは可・描画は遅らせない）。届かないはずの失敗に専用の文言を置く。起動の直後の「装着の前」に預けた呼び出しも、符号化を UI スレッドの外で行う。符号化のスレッドの記録をテストで数える。預けている間にゴーストが替わったときの振る舞いを確かめて塞ぐ。

> 起票: 2026-10-05 `areka-P0-mcp-dump-images` の完了時の棚卸。経緯と再測定は同じフォルダの [brief.md](brief.md)。前の spec は [completed/areka-P0-mcp-dump-images](../completed/areka-P0-mcp-dump-images/)（要件・設計・設計の検証・実機確認 `verification/signoff.md`）。

## Introduction

### 残った件（前の spec の tasks.md の Implementation Notes と、完了前の検証の報告から）

| # | 件 | 今どうなっているか（2026-10-05） | 本 spec |
|---|---|---|---|
| 1 | バルーンの文字の面の読み戻しが UI スレッドに残る | `crates/areka/src/mcp/dump_balloon.rs` の `answer` が、UI スレッドで `crates/areka-emo-text/src/surface.rs` の `TextSurface::read_back`（GPU の写し → 読み出しを同じ呼び出しの中で行う）を呼ぶ | 要件 1 |
| 2 | 届かない枝が判断の文言を `error!` つきで返す | `crates/areka/src/mcp/dump_surface.rs` の `answer` の、指定した surface を単体で描く口（`compose_alone`）が「無い」を返す腕が、判断の文言 `NG:No such scope in this ghost` を想定外の失敗の関数 `fail`（`error!` 1 件）で返す。前の spec の要件 4.6（判断の失敗は ERROR を出さない）の精神と食い違う | 要件 2 |
| 3 | 装着の前に預けた呼び出しは UI スレッドで符号化まで行い、panic を受けない | 2 本の `handle` が、装着の前は `crates/areka/src/mcp/mod.rs` の `later` へ「覗く関数」を預ける。覗く関数は `Step::here` で UI スレッドで符号化まで仕上げ、`catch_unwind` が無い（`later` の口が答えをその場で受け取る形のため）。起きるのは起動の直後に 1 度だけ | 要件 3 |
| 4 | 符号化のスレッドの記録をテストで数えられない | テストの記録の捕まえ方（共有のテストの道具 `log_capture_kit`）は呼んだスレッドの記録だけを捕まえる。GPU を通るテストの「ERROR 0 件」は UI スレッドの記録だけを見ている。成功の記録の `ui_us`・`encode_us` の欄を確かめるテストも無い | 要件 5 |
| 5 | 預けている間にゴーストが替わると、別のゴーストの絵を返しうる（未確認） | 覗く関数は毎フレーム、その時点の World の結線状態を読み直す。前の spec の設計の検証（`design-validation.md` の軽い指摘 b）が挙げたが、差の一覧にも実機確認にも無い | 要件 4 |
| 6 | 前の spec の要件 7.7 ⑵（`sakurascript` で表情を変えてから撮る）を本来の手順で撮り直す | `sakurascript` ツールが未実装のため、開発者の了承のうえ代わりの手順で確かめた | **本 spec の外**（裁定 1） |

### 要件の段での暫定の裁定（要件ディスカッションで覆せる）

| # | 議題 | 暫定の裁定 | 根拠 | 載せた要件 |
|---|---|---|---|---|
| 1 | 件 6 を本 spec に残すか | **残さない**。`sakurascript` が入った後の撮り直し 1 回は `areka-P0-mcp-kanade-tools` の実機確認で行う。本 spec は待ちなしで閉じる | brief「同じウェーブ C4 の約束」（項目 1〜5 だけ）・roadmap の C4 の行「`mcp-dump-images-residue`（項目 1〜5）」 | Boundary Context |
| 2 | 件 3 を MCP の共有のファイル（`crates/areka/src/mcp/mod.rs` の `later`）で直すか、ツールのファイルの中だけで直すか | **ツールのファイルの中だけで直す**。`mod.rs` に触らない。預けた呼び出しも、符号化を UI スレッドの外で行い、仕上がるまで答えを待たせる（答えが 1 フレーム以上後になるだけで、描画は遅らせない） | brief「同じウェーブ C4 の約束」。`mcp-author-tools`・`mcp-ghost-name-match` と並べられる | 3.1〜3.5・Boundary Context |
| 3 | 「線から十分に離れる」の数 | **1 回の呼び出しが UI スレッドを塞ぐ時間を、どのフレームでも 2 ms 以内**（線 16 ms の 8 分の 1）とする。最初の 1 回を含む | 同じ機械・同じビルドでキャラクター 1 枚は最大 0.31 ms。読み戻しを待たなければ、バルーンの UI スレッドの仕事はキャラクターと同じ種類（判断と絵の写し）だけになる | 1.1・7.1 |
| 4 | 件 2 の専用の文言 | **`NG:the shell of this scope is not ready`**（`error!` 1 件） | `dump_balloon` の同じ種類の届かない枝が既に `NG:the balloon of this scope is not ready` を使っている。判断の文言 4 つ（窓・スコープ・surface ID・未表示）のどれとも違う | 2.1・2.2 |
| 5 | 件 5 が起きるなら、塞ぐか、差の一覧に載せるか | **塞ぐ**。預けている間に、呼び出しを解決したゴーストが今動いているゴーストでなくなったら、絵を返さず **`NG:Specified ghost is not active`**（入口の解決の失敗と同じ文言・判断の失敗なので ERROR を出さない）で答える。起きるかどうかの確かめの結果も残す | 別のゴーストの絵を、解決したゴーストの名前を添えて返すのは誤った答え。入口の文言は「解決したゴーストが今は動いていない」と同じ意味で、新しい文言を増やさずに済む。**要件ディスカッション（2026-10-05）で確定**: 確かめるのは絵を写す前（装着を待つ間）だけ。写した後に替わっても写した時点の絵を返す（全部の答えを後から答える置き場に通す案は取らない） | 4.1〜4.5 |

### 正典

本 spec は前の spec の振る舞い（SSP ヘルプ「その他の機能」と ukadoc `\![execute,dumpballoon,…]`・`\![execute,dumpsurface,…]` に合わせたもの）を変えない。新しい正典の事実は持ち込まない。

## Boundary Context

- **In scope**:
  - 残った件 1〜5（上の表）。
  - `dump_balloon` の UI スレッドの側の所要時間の、実機での測り直し。
  - 決定論テスト（符号化のスレッドの記録を数えること・成功の記録の欄を含む）と実機確認の記録（本 spec の `verification/signoff.md`）。
- **Out of scope**:
  - 件 6（前の spec の要件 7.7 ⑵ の撮り直し）。`areka-P0-mcp-kanade-tools` の実機確認で行う（裁定 1）。
  - 新しいツール。文言の SSP 寄せ（`areka-P0-mcp-strict-errors`）。`sakurascript`・`raise_event`・`get_status` の実装（`areka-P0-mcp-kanade-tools` ほか）。
  - 全体テストの負荷の下の揺れ（`areka-P0-ghost-session-test-load-flake`）。
  - 前の spec の要件 1〜6 の振る舞いの変更（要件 6 で保つ。例外は要件 2 の新しい文言 1 つ）。
- **Adjacent expectations**:
  - 同じウェーブ C4 の約束（brief・`.kiro/steering/roadmap.md` の C4 の行）: `crates/areka/src/mcp/mod.rs` に触らない。文字の層（`crates/areka-emo-text/src/`）に新しいファイルを足さない（`budoux-reveal-reflow` が `lib.rs` を持つ）。触ってよい既存のファイルは、brief「触るファイル」の `crates/areka/src/mcp/{dump_surface.rs, dump_surface_judge.rs, dump_balloon.rs}` と各テスト（`dump_*_tests.rs`・`dump_*_gpu_tests.rs`・`dump_surface_gpu_test_support.rs`）と、件 1 で要るときの `crates/areka-emo-text/src/surface.rs`（今 817 行。足して 1,000 行を超えない）。ツールのファイルの子モジュールとして新しいファイルを足すのは可。これ以外（共有のテストの道具 `log_capture_kit` を含む）に触る要が出たら止めて報告する。
  - 文字の層の読み戻しの口を変えるときも、今の呼び手（文字の層のテスト・見本のプログラム・`crates/areka/src/emo2_boot/` のテスト・`dump_balloon` の GPU を通るテスト）が受け取る結果は変えない。
  - `mcp-tool-entrances` から受け取るもの（10 秒の待ちの上限・終了の途中の答え・ゴーストの解決・`later` の置き場）は変えない。
  - `areka-P0-mcp-kanade-tools`: 件 6 を引き継ぐ。引き継ぎは要件ディスカッション（2026-10-05）で相手の brief の末尾の節「`mcp-dump-images-residue` からの引き継ぎ」に書いた（相手はまだ着手していないので、文書の追記だけ本 spec のブランチで行った）。
- **範囲外の観測（記録だけ）**: 前の spec の実機で、`surface` を指定した `dump_surface`（指定の surface を単体で描く経路）の UI スレッドの側は 2,226 µs だった。線 16 ms の内で、描画を飛ばすおそれは無いので本 spec では扱わない。要件 1.1 の 2 ms は `dump_balloon` だけに掛ける。

## Requirements

### Requirement 1: `dump_balloon` がゴーストの描画を引っかけない

**Objective:** As a areka を常駐させている利用者, I want エージェントがバルーンを続けて撮っても描画が飛ばないこと, so that 撮られていることに気付かずにゴーストと過ごせる

#### Acceptance Criteria

1. When `dump_balloon` が届く, the areka shall その呼び出しが UI スレッドを塞ぐ時間を、どのフレームでも 2 ms 以内とする（配布用のビルド・最初の 1 回を含む・裁定 3）。
2. The areka shall バルーンの文字の面の中身が GPU から読み出せるようになるのを、UI スレッドで待たない（GPU の仕事の終わりを待つ時間が UI スレッドに現れず、文字の面の大きさで伸びない。読めた中身を手元へ写す分は背景の写しと同じ種類の仕事として 1.1 の 2 ms で抑える・設計ディスカッション議題 1）。
3. The areka shall 返すバルーンの絵の背景と文字を、どちらも呼び出しを受けたときの同じ時点の内容とする（答えを返すまでの間に台詞が進んだり消えたりしても、後の内容を混ぜない）。
4. While 答えを呼び出しを受けたフレームより後のフレームまで待たせている, the areka shall 描画とアニメーションと台詞の進みを遅らせない（遅れるのは答えだけ）。
5. If 文字の面の読み出しが失敗する, then the areka shall 本文 `NG:` ＋理由の英文・`isError: true`・画像なしで答え、`error!` を 1 件残す（ツール名・スコープ・失敗の理由を載せる。文字の層とツールの両方で記録して 2 件にしない。GPU の仕事がまだ終わっていないだけの「まだ読めない」は失敗として扱わず、記録も残さない）。
6. If 読み出しを待っている間に areka の終了が始まる, then the areka shall 終了の途中の答えを入口の決まり（`mcp-tool-entrances`）のままとし、終了を読み出しの完了まで待たせない。

### Requirement 2: 届かないはずの失敗は判断の文言を使わない

**Objective:** As a 記録を読んで原因を探す開発者, I want 想定外の失敗が判断の失敗と別の文言で返ること, so that 「呼び方の誤り」と「areka の中の不整合」を取り違えない

#### Acceptance Criteria

1. If `surface` を指定した `dump_surface` で、判断がスコープを確かめた後なのに、その surface を単体で描く口がそのスコープを知らないと答える, then the areka shall 本文 `NG:the shell of this scope is not ready`・`isError: true`・画像なしで答え、`error!` を 1 件残す（裁定 4）。
2. The areka shall 判断の文言 4 つ（`NG:This ghost has no window`・`NG:No such scope in this ghost`・`NG:No such surface ID. Check get_expression_table tool`・`NG:No surface has been shown in this scope yet`）を、`dump_surface`・`dump_balloon` の判断の失敗（ERROR を出さない答え）にだけ使い、`error!` を伴う想定外の失敗の本文に使わない。

### Requirement 3: 装着の前に預けた呼び出しも UI スレッドで符号化しない

**Objective:** As a 起動の直後にエージェントから撮られる利用者, I want 最初の 1 回も UI スレッドを塞がず、符号化の panic でも決まった答えが返ること, so that 起動の直後の描画が引っかからず、エージェントが誤った「終了中」を受け取らない

#### Acceptance Criteria

1. While 窓への結線はあるが装着がまだ済んでいない, when `dump_surface`・`dump_balloon` が届く, the areka shall 装着が済んだフレームで判断と絵の写しを行い、乗算を戻す・重ね合わせ・PNG・base64 を UI スレッドの外で行う（装着の前でない呼び出しと同じ分け方）。
2. While 3.1 の符号化が仕上がっていない, the areka shall 答えを待たせ、仕上がった後のフレームで答える（答えが 1 フレーム以上後になってよい・描画は遅らせない）。
3. If 3.1 の符号化の途中で panic が起きる, then the areka shall 本文 `NG:the encoding thread panicked`・`isError: true`・画像なしで答え、`error!` を 1 件残す（装着の前でない呼び出しと同じ答え）。
4. If 3.1 の符号化を受け持つスレッドを起こせない, then the areka shall 本文 `NG:` ＋理由の英文・`isError: true`・画像なしで答え、`error!` を 1 件残す。
5. If 3.1 の答えが仕上がる前に、待つ側が居なくなる（10 秒の上限を過ぎた・終了が始まった）, then the areka shall 仕上がった答えを黙って捨て、捨てることで panic も ERROR の記録も出さない（符号化そのものが panic したときの `error!` は、待つ側の有無に依らず 3.3 どおり残す）。

### Requirement 4: 預けている間にゴーストが替わっても、別のゴーストの絵を返さない

**Objective:** As a AI エージェントの利用者, I want 返る絵が、呼び出しを解決したゴーストのものであること, so that 切り替えの途中に撮っても、別のゴーストの絵を自分のゴーストの絵と取り違えない

#### Acceptance Criteria

1. The areka shall `dump_surface`・`dump_balloon` の答えに、呼び出しを解決したゴースト以外のゴーストの絵を含めない。
2. If 絵を写す前（装着を待っている間）に、呼び出しを解決したゴーストが今動いているゴーストでなくなる, then the areka shall 絵を返さず、本文 `NG:Specified ghost is not active`・`isError: true` で答え、ERROR の記録を出さない（`debug!` までにとどめる・裁定 5）。同じゴーストかどうかは名前とルートフォルダで見分ける（同じゴーストを起こし直したときは同じと見なす。返る絵はそのゴーストのものなので 4.1 を満たす）。
3. The areka shall 件 5 が今の areka で起きうるかを確かめ、その結果（起きる道筋、または起きない根拠）を、ファイルと定義の名前で再び確かめられる形で本 spec に残す。
4. The areka shall 4.3 の結果に依らず 4.1・4.2 の振る舞いを保つ（今は起きないと分かっても、将来の変更で起きうるようになったときに別のゴーストの絵を返さない）。
5. Where 絵を写し終えた後に答えを待たせている（文字の面の読み出しや符号化を待つ）, the areka shall その間にゴーストが替わっても、写した時点の絵（呼び出しを解決したゴーストの、呼び出しを受けた時点の姿・要件 1.3）をそのまま返す（議題 1 の裁定）。

### Requirement 5: 符号化のスレッドの記録もテストで数える

**Objective:** As a 後でこのツールを触る開発者, I want 符号化のスレッドで出た ERROR がテストで赤になること, so that 「ERROR 0 件」の検査が見逃しの無い判定になる

#### Acceptance Criteria

1. The areka shall GPU を通るテストの「ERROR が無い」の検査を、UI スレッドで出たものと符号化のスレッドで出たものの両方について判定する。捕まえ方（符号化のスレッドの記録を数えるか、記録を数えずに、符号化のスレッドの失敗が必ず `NG:` の答えに現れることを答えで判定するか）は設計で決める。共有のテストの道具（`log_capture_kit`）は変えない（Boundary Context）。
2. The areka shall 5.1 の検査が符号化のスレッドの失敗を実際に捕まえることを、符号化のスレッドで意図して失敗させた場合に検査が赤になることで確かめる（捕まえる道具の較正）。
3. When `dump_surface`・`dump_balloon` が成功する, the areka shall 成功の記録を 1 件出し、その記録が `ui_us`（UI スレッドの側の時間。答えが複数のフレームにまたがるときは、1 フレームで UI スレッドを塞いだ時間の最大＝要件 1.1 で判定する値）と `encode_us`（符号化のスレッドの側の時間）の欄を数値で持つことを決定論テストで固定する（合計などの欄を足すかは設計で決める）。

### Requirement 6: 前の spec の振る舞いを保つ

**Objective:** As a AI エージェントの利用者, I want 後始末の後も 2 本の答えが変わらないこと, so that 前の spec に合わせて書いた手順がそのまま動く

#### Acceptance Criteria

1. The areka shall 前の spec（`areka-P0-mcp-dump-images`）の要件 1〜6 の振る舞い（成功の本文 3 種・判断の文言 4 つとその順・画像の形と大きさ・読み戻しで画面と台詞を変えないこと・ゴーストへイベントを送らないこと）を変えない。例外は要件 2.1 の新しい文言 1 つだけ。
2. The areka shall 前の spec が置いた決定論テストと GPU を通るテストを、本 spec の変更の後も緑のまま保つ（振る舞いを変えないので、期待する値を書き換えない。要件 2.1 の枝を固定するテストを足すのは可。答えが後のフレームになることに合わせて、答えの待ち方をフレームを回して待つ形に替えるのは、期待する値の書き換えに当たらない）。

### Requirement 7: 決定論テストと実機確認

**Objective:** As a 後でこのツールや文字の層を触る開発者, I want 後始末の各件がテストと実機の記録で固定されること, so that 戻ったときに赤で分かり、線の内に収まったことを数で示せる

#### Acceptance Criteria

1. The areka shall 次を決定論テストで固定する（GPU を要するものはこのリポジトリの定石＝常時テストに入れる条件・画面外の描画先での画素の検証に従う）: ⑴ `dump_balloon` の絵が呼び出しを受けた時点の背景と文字を含み、答えを返すまでの間に台詞を変えても後の内容を含まない（要件 1.3）、⑵ 要件 2.1 の文言と `error!` 1 件、⑶ 装着の前に預けた呼び出しが絵を返し、符号化が UI スレッドの外で行われる（要件 3.1・3.2）、⑷ 装着の前に預けた呼び出しの符号化の panic が要件 3.3 の答えになる、⑸ 待つ側が居なくなった後に仕上がった答えが panic も ERROR も出さない（要件 3.5）、⑹ 預けている間にゴーストが替わると要件 4.2 の答えになる（要件 4.3 で起きないと分かっても、状態を作って固定する）、⑺ 要件 5.1〜5.3。
2. The areka shall 実機で確かめ、結果を本 spec の `verification/signoff.md` に残す: ⑴ 既定ゴースト（emo2）・配布用のビルド・画面の拡大率 200% で、台詞を出した後に `dump_balloon` を 5 回以上（最初の 1 回を含む）呼び、成功の記録の `ui_us`（1 フレームあたりの最大・要件 5.3）がすべて 2 ms 以内（撮ったバルーンの大きさ＝物理 px も残す）、⑵ 返った絵に背景と文字が入り、前の spec の実機確認と同じく原寸で返る、⑶ 呼んでいる間もゴーストの描画と会話が止まらず、ERROR の記録が 0 件。
3. The areka shall 実機確認の展開先・記録・返った PNG を、このワークツリーの `target\` の下だけに置く（絶対パス）。
