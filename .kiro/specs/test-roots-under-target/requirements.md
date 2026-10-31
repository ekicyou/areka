# Requirements Document

> 現状の記述は `brief.md`（2026-10-07 起票・2026-10-31 棚卸㉜の再測定）と、本ブランチ（main `9604677a` の上・2026-10-31）での読み直しによる。`crates/temp-path-kit/src/lib.rs`（148 行）・`lib_tests.rs`（165 行）・見張り `crates/log-capture-kit/tests/temp_path_guard_test.rs`（783 行・例外表 19 ファイル）は棚卸㉜の記述のとおりで、`TempPath::new` の呼び出しは本ブランチで数え直しても **80 ファイル・240 か所**（`git grep -c "TempPath::new" -- crates`）。
>
> 本 spec はウェーブ C15 の ④（割り込み・優先度の区分 C）として、ほかの 5 本と並べて回る。ウェーブの約束（`roadmap.md` の C15 の行・2026-10-31）は「**入口の差し替えだけ**」で、触るファイルは `crates/temp-path-kit/src/{lib,lib_tests}.rs` と `crates/log-capture-kit/tests/temp_path_guard_test.rs` の 3 本。brief の Approach が要件に委ねた「入口の中身を替えるか、入口を消して呼び出しを置き換えるか」は、この約束により**入口の中身を替える**に決まっている。

## Project Description (Input)

### 誰が困っているか

- ワークスペースのテストを回す開発者と AI。特に、テストが落ちたり途中で止まったりした後に残る一時フォルダを片付ける人。
- 開発の決まり「実機の根・検体・一時フォルダはワークツリーの `target\` の下だけ（`C:\` 直下も `C:\tmp` も OS の一時フォルダも不可）」を守らせたい開発者。

### いま何が起きているか

- テスト用の一時フォルダの窓口（`temp_path_kit::TempPath`）には入口が 2 つある。`TempPath::new` は OS の一時フォルダ（`%TEMP%`）の下に、`TempPath::under_target` はワークスペースの `target\test-roots\` の下に一時フォルダを作る。名前・一意性・破棄での削除はどちらも同じ。
- `TempPath::new` を呼ぶのは 80 ファイル・240 か所（`areka` 41・`areka-ghost` 23・`areka-emo-present` 7・`areka-parsers` 4・`temp-path-kit` 2・`areka-sylphya` 1・`areka-impl-watch` 1・`log-capture-kit` 1）。テストが落ちたり途中で止まったりすると、破棄が走らずに一時フォルダが OS の一時フォルダに残り、ワークツリーを消しても掃除されない。
- 新しいテストを止める見張りが無い。10-23 から 10-31 のあいだにも、完了 `placement-measure-bake-once` の新しいテスト（`crates/areka-emo-present/src/shell_target_quiet_tests.rs`）が `TempPath::new` を 6 か所足した。
- 見張り `temp_path_guard_test.rs` は、窓口の外で OS の一時フォルダを**直に**引く呼び出しを例外表（19 ファイル）の外で赤にするが、窓口の入口 `TempPath::new` そのものが OS の一時フォルダを使うことは見ていない（窓口の中に OS の一時フォルダを引く呼び出しが 1 か所あることを、むしろ確かめている）。

### 何を変えるか

1. `TempPath::new` が作る一時フォルダが、`under_target` と同じ `target\test-roots\` の下にできる。呼び出しの 240 か所は書き換えずに、一度に移る。
2. テストが落ちたり途中で止まったりしても、窓口が作った一時フォルダは `target\` の下にだけ残り、OS の一時フォルダには何も残らない。
3. 以後、窓口の入口が OS の一時フォルダへ戻されたり、窓口の中で OS の一時フォルダを引く呼び出しが足されたりしたら、見張りが赤になる。

## Introduction

本書は、テスト用の一時フォルダの窓口が作る一時フォルダの置き場を、OS の一時フォルダからワークツリーの `target\test-roots\` へ移す働きと、それを守らせる見張りの要件を定める。要件の主語は次のとおり。

- **窓口**: `temp-path-kit` の `TempPath`（入口 `TempPath::new`・`TempPath::under_target`、名前の組み立て、破棄での削除）。
- **見張り**: 窓口の自己テスト（`crates/temp-path-kit/src/lib_tests.rs`）と、窓口の迂回を見張るテスト（`crates/log-capture-kit/tests/temp_path_guard_test.rs`）。
- **本 spec**: 本 spec の成果物（コードとテスト）の全体。

「ワークツリーの根」とは、窓口のクレートの 2 つ上のフォルダ（ワークスペースの根。ハーネスのワークツリーで回すときはそのワークツリー）をいう。「置き場」とは、ワークツリーの根の `target\test-roots\` をいう。

## Boundary Context

- **In scope**:
  - `TempPath::new` の置き場を、`under_target` と同じ置き場へ替えること（入口の中身だけ。名前・一意性・破棄は今のまま）。
  - 窓口の説明（`lib.rs` の利用手順・入口の説明）を新しい置き場に合わせること。
  - 見張りの手当て: 窓口の自己テストに「`new` も置き場の下に作る」確かめを足すこと、見張りの「窓口の中に OS の一時フォルダを引く呼び出しが 1 か所ある」確かめを「0 件である」確かめに改めること。
- **Out of scope**:
  - **`TempPath::new` の呼び出しの書き換え**（80 ファイル・240 か所。書き換え 0 か所）。`TempPath::new` を消すこと・`under_target` へ名前を寄せること（入口は 2 つのまま。ウェーブ C15 の約束）。
  - **見張りの例外表の 19 ファイル**（OS の一時フォルダを直に引くファイル。テストが使う 18 ファイル＝`shiori-host32-*`・`areka-ghost`・`areka` の `placement/` のテスト・`areka-emo-present`（うち `shiori_proxy.rs`・`process_host.rs` の 2 つは本番のファイルの中のテストの部分）、本番のダウンロードの置き場 `crates/areka/src/install/fetch_url.rs` の 1 ファイル）。本 spec は 1 ファイルも動かさない（足す 0・外す 0・書き換え 0）。この 18 ファイルのテストは本 spec の後も OS の一時フォルダを使う（下の「未決の点」の 3）。
  - ほかのクレートのテスト・本番のコード（変更 0）。テストの主張・期待値・本数（変更 0）。
  - 一時フォルダを作らないテストの書き換え・`sample-ghost-kit` の `devroot::fresh_root`（既に `target\nar-samples` の下）・`real-run-kit`（既に `under_target`）・実機の手順書。
  - 1,000 行の番人などほかの見張りの変更。
- **Adjacent expectations**:
  - **同じウェーブ C15 のほかの 5 本**（`shell-scale-linkage`・`emo-text-line-gap`・`alert-dialog-per-monitor-dpi`・`emo-bake-log-overlap`・`popup-menu-extra-resources`）: 上の 3 本のファイルを挙げる spec は無い（重なり 0）。ただし、本 spec が着地すると、ほかの 5 本のテストが `TempPath::new` で作る一時フォルダも置き場の下へ移る（働きは同じ）。
  - **`clippy-199-lints`**: 例外表の `shiori-host32-host/tests/lifecycle_{cyclic,kill}_e2e.rs` と重なるので、本 spec は例外表に触らない（上の Out of scope）。
  - **`areka-test-binary-stability`（ウェーブの外・静かな机で測ることが本体）**: 本 spec が着地すると全部のテストの一時フォルダの置き場が動き、測りの基準が変わる。そちらが静かな机で測っているあいだは本 spec を着地させない（要件 5.1）。
  - **`audit-test-kits`（v0.1.0 の後の見直し）**: `temp-path-kit` を受け持つ。本 spec が先に着地しても困らない。
  - **steering `structure.md` の「Temp Path Kit Crate」の節**: 「置き場は 2 つ＝`TempPath::new`（OS の一時フォルダ）…」の記述が古くなる。完了の手順の文書の更新で直す（本 spec の 3 本のファイルの外だが、完了の手順の持ち物）。

## Requirements

### Requirement 1: 窓口が作る一時フォルダはワークツリーの `target\` の下にできる

**Objective:** テストを回す開発者として、窓口が作る一時フォルダがどの入口からでもワークツリーの `target\test-roots\` の下にできてほしい。そうすれば、テストが落ちたり止まったりして残った一時フォルダが、ワークツリーを消すか `target` を消すだけで片付き、OS の一時フォルダに積み上がらない。

#### Acceptance Criteria

1. When テストが `TempPath::new` で一時フォルダを求めた, the 窓口 shall その一時フォルダを置き場（ワークツリーの根の `target\test-roots\`）の直下に作る（`under_target` と同じ置き場）。
2. The 窓口 shall OS の一時フォルダの下に一時フォルダを作らない（`new` と `under_target` のどちらから求めても 0 件）。
3. If テストが落ちた・途中で止まった・破棄が走らずに終わった, then the 窓口 shall 残る一時フォルダを置き場の下にだけ残す（OS の一時フォルダに残る窓口の一時フォルダは 0 件）。
4. If 置き場（`target\` または `target\test-roots\`）がまだ無い（新しいワークツリー・`target` を消した後）, then the 窓口 shall 置き場を自分で作ってから一時フォルダを配る（呼び手に前準備を求めない）。
5. While ビルドの成果物の置き場を変える設定（環境変数 `CARGO_TARGET_DIR`）が有る, the 窓口 shall 今の `under_target` と同じく、ワークツリーの根の `target\test-roots\` の下に作る（ワークツリーの外へは作らない。下の「未決の点」の 1）。
6. The 窓口 shall 名前の形 `areka-{札}-{プロセス識別子}-{連番}`・プロセス内とプロセス間の一意性・破棄で中身ごと消えること・`child` の働きを、今と同じに保つ。
7. If 一時フォルダを作れない, then the 窓口 shall 今と同じく作れなかったパスとエラーを添えて panic し、OS の一時フォルダなど別の置き場へ黙って逃げない。

### Requirement 2: 呼び出し側は書き換えずに一度に移る

**Objective:** ワークスペースのテストの持ち主として、置き場が移っても、自分のテストのファイルが書き換えられず、テストの結果が変わらないでほしい。そうすれば、並べて回るほかの spec と触るファイルがぶつからず、置き場の移動がテストの色を変えない。

#### Acceptance Criteria

1. The 本 spec shall `TempPath::new` の呼び出し（80 ファイル・240 か所）を 1 か所も書き換えない（書き換え 0 か所）。呼び出しはそのままで、全部が要件 1 の置き場へ移る。
2. The 本 spec shall `TempPath::under_target` の呼び出し（その名前が出るファイル 96）の置き場・名前・働きを変えない（変更 0）。
3. The 本 spec shall 触るファイルを `crates/temp-path-kit/src/lib.rs`・`crates/temp-path-kit/src/lib_tests.rs`・`crates/log-capture-kit/tests/temp_path_guard_test.rs` の 3 本に限り、ほかのクレートのテスト・本番のコード・`Cargo.toml`・`Cargo.lock` を変えない（変更 0）。
4. When 本 spec の実装の後に全体テストを回した, the ワークスペースのテスト shall 前に緑だったテストを、置き場の移動によって赤に変えない（主張・期待値・本数は変更 0）。
5. If 置き場の移動によって赤に変わるテストが見つかった（パスの長さ・ワークスペースの中にあることに頼る判定など）, then the 本 spec shall 3 本のファイルの外の呼び出し側を書き換える前に、その件を開発者の裁定へ上げる（ウェーブの約束の外に黙って手を広げない）。
6. The 窓口 shall 説明（利用手順と入口の説明）で、`new` と `under_target` のどちらも置き場の下に作ることを述べ、利用手順の例が今と同じく doctest として通る。

### Requirement 3: パスが長くなってもテストが通る

**Objective:** 長いパスに弱いテスト（実物の DLL を読む・深いフォルダを組む）の持ち主として、置き場の移動で一時フォルダのパスが長くなっても、テストがパスの長さで失敗しないでほしい。そうすれば、ワークツリーの名前の長さでテストの色が変わらない。

#### Acceptance Criteria

1. The 窓口 shall 置き場の移動でパスが長くなる分（このワークツリーで、根 81 字＋`\target\test-roots\` で約 100 字。今の OS の一時フォルダ `C:\Users\<名>\AppData\Local\Temp\` は約 34 字）を見込んでも、既存のテストがパスの長さで失敗しない長さの一時フォルダを配る。
2. When 実物の `pasta.dll` を読むテスト（`crates/areka-ghost/tests/ghost/real_pasta_test.rs`・環境変数 `HOST32_PASTA_DLL` を与えた走行）を本 spec の後に回した, the テスト shall パスの長さで失敗しない。
3. When 札の長いテスト（直書きの札で最長 39 字・`format!` で札を組むテストは 60 字を超えるものがある＝`areka-ghost` の `runtime_tests.rs` ほか）を本 spec の後に回した, the テスト shall パスの長さで失敗しない。
4. If パスの長さのために名前を短くする必要が出た, then the 窓口 shall 呼び出し側の札を書き換えずに（要件 2.1）、失敗の後に残ったフォルダ名からどのテスト群の一時フォルダかが分かることを保つ。

### Requirement 4: 以後、OS の一時フォルダを使う一時フォルダが黙って入らない

**Objective:** 開発の決まりを守らせたい開発者として、以後、窓口の入口が OS の一時フォルダへ戻されたり、OS の一時フォルダを直に引く新しいテストが足されたりしたら、見張りが赤になってほしい。そうすれば、`placement-measure-bake-once` の 6 か所のように、新しいテストが黙って OS の一時フォルダに一時フォルダを作ることが起きない。

#### Acceptance Criteria

1. The 見張り shall `TempPath::new` が配る一時フォルダが置き場（ワークツリーの根の `target\test-roots\`）の直下にあることを確かめ、置き場が OS の一時フォルダなどへ戻されたら赤になる（`under_target` の今の自己テストと同じく、ワークツリーの根は置き場を求める式とは別の手がかりで確かめる）。
2. The 見張り shall 窓口の定義（`crates/temp-path-kit/src/`）の中に、OS の一時フォルダを引く呼び出しが 0 件であることを確かめる（今の「窓口の中に 1 か所ある」を確かめる検査を改める）。
3. The 見張り shall 2 の「0 件」の主張が、走査が空振りしていても緑になる恒真の主張にならないよう、陽性側の確かめ（窓口の中に OS の一時フォルダを引く呼び出しが有れば当たること）を対で持つ。
4. The 見張り shall 窓口の外で OS の一時フォルダを直に引く呼び出しが、例外表の 19 ファイルの外に 0 件であることを、今と同じに確かめ続ける（例外表の件数 19・内訳＝プロセス識別子で一意化 16・読み出しのみ 2・固定名が仕様 1 は変えない。足す 0・外す 0）。
5. If 新しいテストが窓口の外で OS の一時フォルダを直に引いた, then the 見張り shall 今と同じく赤になり、そのファイルと行を示す。
6. The 本 spec shall 1 ファイル 1,000 行の番人を守る（触る 3 本のどれも 1,000 行を超えない）。

### Requirement 5: 着地の条件

**Objective:** ウェーブを回す開発者として、本 spec の着地が、並べて回る測りや確かめの基準を黙って動かさないでほしい。そうすれば、ほかの spec の測りの結果が、本 spec の着地の前後で読み違えられない。

#### Acceptance Criteria

1. While `areka-test-binary-stability` が静かな机で測っている, the 本 spec shall 着地しない（全部のテストの一時フォルダの置き場が動き、測りの基準が変わるため）。
2. When 本 spec の実装が済んだ, the 本 spec shall 全体テスト 1 回で、要件 2.4 と要件 4 の見張りが緑であることを確かめる（静かな机・前後の全体テストの比べ・長い負荷の走行・人の待ちは要らない）。OS の一時フォルダの中身を走行の前後で比べる確かめは置かない（並べて回るほかのワークツリーの古いコードが同じ形の名前のフォルダを作るので、本 spec の働きを測れない。要件 1.2 は要件 4.1・4.2 の見張りで確かめる）。

## 未決の点（要件討議の議題の候補・どれも要件を止めない）

1. **`CARGO_TARGET_DIR` を見るか**: 今の `under_target` はこの設定を見ず、いつもワークツリーの根の `target` を使う（開発の決まり「ワークツリーの `target\` の下だけ」には合う。設定がワークツリーの外を指すと、見る形では決まりを破る）。要件 1.5 は今の働きのまま（見ない）とした。`sample-ghost-kit` の `devroot` は設定を見る（設定が無ければ実行ファイルの祖先の `target`）、`real-run-kit` は見ない。brief の Approach の「`OUT_DIR` や `CARGO_TARGET_DIR` を使うビルドでも `target` の位置を正しく引ける」は、この違いを決める話になる。見る形を選ぶなら要件 1.5 を改める。
2. **パスの長さをどう確かめるか**: 要件 3.2 の実物の `pasta.dll` のテストは `HOST32_PASTA_DLL` が無いと何もせずに抜けるので、全体テスト 1 回では確かめにならない。実装の中で 1 度その走行を回すか、パスの長さを静的に見積もって記録するかを決める。この機械は OS の長いパスの許可（`LongPathsEnabled`）が 1 だが、DLL を読む側（`pasta.dll` の中の Lua）がそれに従うかは確かめていない。名前を短くする要が出たときの形（要件 3.4）も設計で決める。
3. **例外表の 18 ファイルのテストは OS の一時フォルダを使い続ける**: 本 spec は例外表に触らない（ウェーブ C15 の約束・`clippy-199-lints` との重なり）。そのため、本 spec の後も「ワークスペースのテストが作る一時の根はすべて `target\` の下」（brief の Desired Outcome の 1）は、窓口が作る分についてだけ成り立つ。残りの 18 ファイルを移す追跡の spec は今は無い（`audit-test-kits` は `temp-path-kit` を受け持つが、例外表の各ファイルは受け持たない）。完了の棚卸で `/kiro-discovery` により起票するか、本 spec の brief に残すかを決める。
4. **見張りの置き場**: 要件 4.1 の確かめを窓口の自己テスト（`lib_tests.rs`）に置くか、迂回の見張り（`temp_path_guard_test.rs`）にも置くかは設計で決める（どちらも約束の 3 本の中）。
