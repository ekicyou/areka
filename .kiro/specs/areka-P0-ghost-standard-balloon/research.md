# ギャップ分析: areka-P0-ghost-standard-balloon

> 2026-10-05 `/kiro-validate-gap`。対象は確定済みの `requirements.md`（Requirement 1〜7）。調べたのはワークツリーの今のソース（ブランチの先頭 `c8a6479e`）。ビルドとテストは走らせていない（読んで確かめただけ）。この文書は選択肢と事実を並べるもので、決めるのは設計の段である。

## 1. まとめ

- **足りないものは 3 つだけ**である。⑴ 同梱の番号付きの探索（無印 → `balloon0`）、⑵ ゴーストの descript.txt の `default.balloon.path`・`balloon` の読み手、⑶ バルーンを決める鎖の「descript の段」。鎖そのもの・記録の形・切替からの呼び出しはすでに在り、切替の配線（`ghost_switch.rs`）は無改変で済む見込み。
- **今の読み手は「行は在るが値が空」を見分けられない**。`catalog.rs` の `lowercased`（鍵を小文字化した表を作る関数）が空の値を鍵ごと落とすからである。要件 1 の 14 項（空の無印も「見つかった」に数えて探索を止める）は、この関数を通さない読み方が要る。
- **`balloon` を名前で引くには、鎖へ渡す列挙の形が変わる**。今 `boot_config.rs` の `resolve_balloon_for_ghost` はフォルダ名の列だけを渡している。バルーンの `name` は `catalog.rs` の `Identity.name` に在るので、取り直す I/O は要らない。
- **要件と今のコード・テストの間に、実装できない矛盾は見つからなかった**。読み方に幅のある所が 3 つある（§6）。brief の「`catalog_tests.rs` は `balloon0` を無視することを確かめている」は、実際には無印と `balloon0` の両方を書いた検体で無印が返ることを確かめているだけで、要件 1 の 3 項と両立する（テストの書き換えは不要・コメントだけ古くなる）。
- 規模は **S〜M**（brief の見立てどおり）、リスクは **Low**。行数の番人（1 ファイル 1,000 行）に近いテストファイルが 2 本あるので、テストは兄弟の新しいファイルへ置くことになる。

## 2. 今の姿

### 2.1 バルーンを決める鎖

| 部品 | 置き場 | 今していること |
|---|---|---|
| 鎖の判断 | `crates/areka/src/boot_resolve.rs` の `resolve_balloon`（純粋な関数・I/O なし） | 引数 → 記憶 → 同梱 → 0 個なら失敗 → 唯一 → 既定 → 無作為。記憶と同梱は「列挙に在るか」を `find`（フォルダ名の完全一致・大文字と小文字を区別）で確かめ、無ければ `warn!` を 1 件出して次へ進む |
| 鎖の入力 | 同ファイルの `BalloonInputs` | `root`・`argv`・`memory`・`companion: Option<&str>`・`listed: &[String]`（フォルダ名だけ） |
| 決まった段 | 同ファイルの `BalloonRoute` | `Argv`・`Memory`・`Companion`・`Only`・`Default`・`Random` の 6 つ |
| 入力を集める口 | `crates/areka/src/boot_config.rs` の `resolve_balloon_for_ghost` | 記憶を `read_last_balloon` で読み、同梱を `catalog::companion_balloon` で読み、`catalog::list_balloons` の結果を**フォルダ名だけへ落として**渡す |
| 起動の入口 | 同ファイルの `resolve_boot_from` | 引数のバルーンが在れば `resolve_balloon_for_ghost` を呼ばずに鎖を直接呼ぶ（記憶も同梱も読まない）。決まった後に `balloon_resolved`（info・`route` と `dir`）を 1 件出す |
| 切替の入口 | `crates/areka/src/emo2_boot/ghost_switch.rs`（切替先を起こす関数の中） | `resolve_balloon_for_ghost(&root, &ghost.dir, pick_index)` を呼ぶだけ。**`balloon_resolved` に当たる記録は出していない**（失敗のときだけ `ghost_switch_boot_failed`） |

当たらなかったときの今の記録は 2 種。

- `last_balloon_not_found`（欄は `memory`・`balloon_store`）
- `companion_balloon_not_found`（欄は `companion`・`balloon_store`）

### 2.2 同梱の読み手

`crates/areka-ghost/src/catalog.rs` の `companion_balloon`。

- `<ゴースト>/install.txt` を読む。無ければ黙って無し。読めなければ `catalog_install_unreadable` を 1 件出して無し（要件 1 の 11・12 項は今のまま満たす）。
- 表を `lowercased` で作り、`balloon.directory` の 1 鍵だけを引く。番号付きは読まない。
- `lowercased` は**空の値の鍵を落とす**（コメントに「どの鍵でも『無し』と同じに扱う」）。そのため `balloon.directory,`（空）と行なしを区別できない。

### 2.3 インストールの側の探索（完了 `install-companion-reading`）

`crates/areka-nar/src/manifest.rs` の `search_balloons`（非公開の関数・8 行ほど）。

- 無印 → `balloon0` → `balloon1` … の順に `<接頭辞>.directory` の鍵が**表に在るか**（`contains_key`）で判定する。値が空でも在れば見つかった。
- 番号は `format!` で作るので、先頭に 0 を付けた綴り（`balloon00`）には当たらない。
- `areka-nar` の表は空の値を落とさない（落とすのは `non_empty` を通したときだけ）。鍵は ASCII 小文字化して引く。
- `areka-ghost` は `areka-nar` に依存していない（`catalog.rs` の冒頭のコメントが「依存しない」と明記）。両方とも `areka-parsers` には依存する。

### 2.4 ゴーストの descript.txt の読み手

`catalog.rs` に同じ型の読み手が 2 つ在る（`sakura_name`・`install_accept`）。どちらも `master_descript_keys`（`ghost/master/descript.txt` を鍵を小文字化した表で読む関数）を通す。

- 無い → 黙って無し。読めない → `catalog_descript_unreadable` を 1 件出して無し。
- 空の値は `lowercased` が落とすので「書かれていない」と同じ（要件 2 の 2 項と一致）。
- `default.balloon.path`・`balloon`・`recommended.balloon`・`recommended.balloon.path` を読むコードは、ワークスペースのどこにも無い（検索で 0 件）。

### 2.5 名前 → フォルダ名の決め方（実行中の切替）

`crates/areka/src/emo2_boot/shell_balloon_resolve.rs` の `resolve_skin_target`。

- 候補は `SkinCandidate`（`dir`・`folder`・`name`・`hidden`）。バルーンの候補は `balloon_candidates`（`list_balloons` の写し・並びはフォルダ名のバイト順）。
- 指し方が名前（`SkinSpec::Name`）のとき、**`random` と `lastinstalled` を先に特別に解き**、その後で「`name` が一致する最初の候補 → 無ければフォルダ名が一致する候補」。
- 要件 2 の 6 項は「同じ決め方」、8 項は「`random`・`lastinstalled` は特別に解かない」。つまり共有できるのは最後の腕（`name` → フォルダ名）だけで、関数を丸ごと呼ぶと 8 項に反する。

### 2.6 今の動きを固定しているテストと見張り

| 置き場 | 固定していること | 本 spec の後 |
|---|---|---|
| `crates/areka-ghost/src/catalog_tests.rs` の `companion_balloon_reads_one_key` | `Balloon.Directory, kaku` と `balloon0.directory,other` の両方 → `kaku` | そのまま通る（要件 1 の 3 項）。テスト名とコメントだけ古くなる |
| 同ファイルの空の値のテスト（`balloon.directory,` → 無し） | 空の値は無し | 戻り値を今の `Option<String>` のままにするなら通る |
| 同ファイルの `unreadable_install_warns_and_yields_none` | 読めない → 警告 1 件＋無し | そのまま |
| `crates/areka/src/boot_resolve_tests.rs`（824 行）の `balloon` 補助関数とバルーンの 10 本ほど | `BalloonInputs` の 5 欄を直接組む | 欄を足す・`listed` の形を変えると補助関数 1 か所と呼び出しが追随する |
| `crates/areka/src/main_config_input_tests.rs`（509 行）の `resolve_balloon_for_ghost` の 5 本 | 記憶 > 同梱 > 既定を実ファイルで | そのまま通る見込み。descript・番号付きの場面を足す置き場の候補 |
| `crates/areka/tests/smoke_boot_loop_exit.rs`・`tools/package.ps1`（`$LOG_MARKER_BALLOON_ROUTE = 'route=Companion'`） | emo2 の初回のバルーンは `route=Companion`・`emo2-kakukaku` | emo2 の descript.txt に `balloon`・`default.balloon.path` は無い（`vendors/sample_ghost/emo2.nar` の中身で確認）。**`BalloonRoute::Companion` の名前を変えると両方が赤になる** |
| `crates/log-capture-kit/tests/file_length_guard_test.rs` | 1 ファイル 1,000 行 | `ghost_switch_tests.rs` は 988 行・`boot_resolve_tests.rs` は 824 行。足す場面（要件 7 で 30 前後）は兄弟の新しいテストファイルへ |
| `ukadoc-survey` の検査 | 台帳の `status = "implemented"` には、ソースの定義箇所に正典 URL 1 行のコメントが要る（無ければ `ImplementedWithoutEvidence`） | §4.6 |

### 2.7 検体

- `vendors/sample_ghost/claudia.nar` の `install.txt`: `balloon0.directory,claudia`・`balloon1.directory,claudia_vertical`（無印なし）。
- `vendors/sample_ghost/emo2.nar` の `install.txt`: `balloon.directory,emo2-kakukaku`（無印）。
- 同梱の全ゴースト（emo2・claudia・konnoyayame・R_POST_and_KOMAINU）の `ghost/master/descript.txt` に `balloon` を含む行は 0 件。descript の段を実機で確かめるには、手で 1 行足した検体が要る。

## 3. 要件と今の資産の対応

印: **在る**＝今のコードで満たす／**足す**＝無いので作る／**直す**＝在るが変える／**注意**＝読み方か制約に気を付ける。

| 要件 | 今の資産 | 印 | 何が要るか |
|---|---|---|---|
| 1 の 1〜6（探索の順・行で判定・先頭 0 を数えない） | `companion_balloon`（無印だけ）／`areka-nar` の `search_balloons`（非公開） | 足す | 「最初の 1 個」を返す読み方。`lowercased` を通さず、行の有無で判定 |
| 1 の 7〜10（完全一致・無ければ警告・繰り下げない） | `resolve_balloon` の同梱の段と `find` | 在る | 読み手が 1 個だけ返せば、段は無改変 |
| 1 の 11〜13（`install.txt` が無い・読めない・無印は今のまま） | `companion_balloon` | 在る | 変えない |
| 1 の 14（空の値でも「見つかった」・記録 0 件） | `lowercased` が空を落とす | 直す・注意 | 行の有無と値を別々に見る。返すのは「無し」 |
| 1 の 15（区切りを含む値は読み替えない・警告 1 件） | `find` はフォルダ名との完全一致 | 在る | 区切りを含むフォルダ名は列挙に現れないので、足すコードは 0 行で「当たらない＋`companion_balloon_not_found`」になる |
| 2 の 1・2・13（2 鍵を読む・空は無し） | `master_descript_keys` | 足す | 2 鍵を **1 回の読み**で取る読み手（§5 の D4） |
| 2 の 3〜5（`default.balloon.path` はフォルダ名 1 段・ゴーストの中は見ない） | `find` | 足す | 鎖に段を足す。`..`・絶対パス・区切りは列挙のフォルダ名と一致し得ないので、専用の検査は書かなくても当たらない |
| 2 の 6〜8（`balloon` は `name` → フォルダ名・特別な語は解かない） | `resolve_skin_target` の最後の腕／`Identity.name` | 足す・直す | 列挙を「フォルダ名と `name` の組」で渡す。決め方の共有の仕方は §5 の D2 |
| 2 の 9〜12（`default.balloon.path` が先・鍵ごとの記録） | 無し | 足す | 段の中の順と、鍵ごとの `warn!` |
| 2 の 14（`recommended.*` を使わない） | 読むコードが無い | 在る | 変えない |
| 3 の 1〜8（段の並び・記憶が最優先） | `resolve_balloon` | 直す | 記憶と同梱の間に段を 1 つ挟む。他の段は無改変 |
| 4 の 1（切替も同じ並び） | `ghost_switch.rs` が `resolve_balloon_for_ghost` を呼ぶ | 在る | 入力を集める口を直せば付いてくる |
| 4 の 2・3（実行中の切替・シェルの切替は変更 0） | `shell_balloon_switch.rs`・`frame/switch.rs` | 在る | 触らない。D2 で共有の関数を切り出す場合だけ `shell_balloon_resolve.rs` に差分が出る |
| 5 の 1（決まった段を区別） | `BalloonRoute`・`balloon_resolved` | 足す・注意 | 腕を 1 つ足す。切替では今そもそも記録が出ていない（§6 の 1） |
| 5 の 2・5・6（鍵ごとの警告・件数の上限） | 無し | 足す | 新しい記録の名前と欄（鍵・値・置き場） |
| 5 の 3（同梱は今と同じ形） | `companion_balloon_not_found` | 在る | 変えない |
| 5 の 4（descript が読めない → 1 件） | `catalog_descript_unreadable` | 在る・注意 | 2 鍵を別々の読み手にすると 2 件になる（D4） |
| 5 の 7（記録の無い失敗の経路 0 本） | — | 注意 | 読み手の `None` の出口を全部数える |
| 6 の 1〜3・6（§8 に記す） | `doc/COMPAT_ARCHITECTURE.md` §8 の表（「【上書き】」の行の型が在る） | 足す | 行を足す。完了 `baseware-root-layout` 要件 5.3 の上書きは「【上書き】」の型で |
| 6 の 4（台帳の `balloon`・`default.balloon.path`） | `assets.toml` の 2 行（`absent`・担当なし・注記はもう無い `boot_config::default_balloon_root` と `boot_config::resolve_config_inputs` を指す） | 直す・注意 | 状態と担当と注記。`implemented` にするなら証拠の URL 行が要る（D6） |
| 6 の 5（`descript_install` の `*.directory`） | 同ファイルの 1 行（`implemented`・担当 `areka-P0-nar-install`） | 直す | 注記に 1 文足す |
| 6 の 7（`recommended.*` の 2 行は注記だけ） | 同ファイルの 2 行 | 直す | 同じ古い文が在る。`absent` のまま |
| 7 の 1〜7（決定論のテスト） | 既存のテストの型（`log_capture_kit::capture`・`temp_path_kit::TempPath`） | 足す | 兄弟の新しいテストファイル。排他で開いて「読めない」を作る補助 `hold_exclusive` は `catalog_tests.rs` の中にしか無い（`areka` 側のテストからは使えない） |

## 4. 作りの選択肢

### 4.1 案 A: 今の部品を伸ばす

- `catalog.rs`
  - `companion_balloon` の中身を「無印 → `balloon0` の最初の 1 個」へ差し替える。名前と戻り値（`Option<String>`）は変えない。呼び手 1 か所と既存のテストはそのまま。
  - descript の 2 鍵を 1 回で読む読み手を 1 つ足す（`sakura_name` と同じ型）。
- `boot_resolve.rs`
  - `BalloonInputs` に descript の 2 欄を足し、`listed` を「フォルダ名と `name` の組」の列へ変える。`BalloonRoute` に腕を 1 つ足す。`resolve_balloon` の記憶と同梱の間に段を足す。
- `boot_config.rs`
  - `resolve_balloon_for_ghost` が 2 欄を読んで渡す。引数の経路（`resolve_boot_from` の中で鎖を直接呼ぶ所）は空を渡す。
- 良い点: 触る本番ファイルは 3 本。判断は今どおり純粋な関数 1 つに集まり、要件 7 の 2〜4 項の大半を I/O なしで踏める。
- 悪い点: `listed` の形を変えると `boot_resolve_tests.rs` の補助関数と既存のバルーンのテストが追随する。`resolve_skin_target` と同じ「`name` → フォルダ名」が 2 か所に書かれる（数行）。

### 4.2 案 B: 新しい部品へ分ける

- `areka-ghost` に「ゴーストの標準のバルーンの宣言」を読む新しいモジュール（同梱の最初の 1 個＋descript の 2 鍵を 1 つの値で返す）を置く。
- `areka` に「名前からバルーンを決める」共有の関数を切り出し、`resolve_skin_target` の最後の腕と descript の段の両方がそれを呼ぶ。
- 良い点: 要件 2 の 6 項の「同じ決め方」がコードの上でも 1 か所になる。後続の `shell-companion-balloon`・`ghost-inner-balloon` が同じ読み手へ足しやすい。
- 悪い点: ファイルが増える。`shell_balloon_resolve.rs` に差分が出る（要件 4 の 2 項は「決め方を変えない」なので動きは変わらないが、完了 spec のファイルを触る）。`catalog.rs` は 377 行で、分けるほど長くない。

### 4.3 案 C: 混ぜる

- 読み手は案 A（`catalog.rs` へ足す）。
- 「`name` → フォルダ名」だけを小さな共有の関数にして、`resolve_skin_target` と鎖の両方から呼ぶ（案 B の後半だけ）。
- 探索の順は、`areka-nar` の `search_balloons` と同じ形を `catalog.rs` に書く（D1 の ⒜）か、`areka-parsers` へ移して両方が呼ぶ（D1 の ⒝）。
- 良い点: 規則の二重書きを 1 つ（名前の決め方）に減らしつつ、ファイルは増やさない。
- 悪い点: 共有の関数の引数の型を、`SkinCandidate` と鎖の列挙の両方に合う形（たとえばフォルダ名と `name` の借用の組）にする手間。

### 4.4 どの案でも変わらない所

- `ghost_switch.rs`・`frame/switch.rs`・`shell_balloon_switch.rs`・`ghost_session.rs`・`crates/areka-nar/` の動き（brief の Out of Boundary）。
- `BalloonRoute::Companion` の名前（`tools/package.ps1` と `smoke_boot_loop_exit.rs` が `route=Companion` の綴りを見ている）。
- 記憶の書き方（`boot_resolve.rs` の `LastUsed::record`）。

## 5. 設計で決めること

**D1. 探索の順をどこに書くか。**
- ⒜ `catalog.rs` に `search_balloons` と同じ形を書く（最初の 1 個だけ要るので数行）。規則が 2 か所になる。要件の Adjacent expectations は「受け取るのは探索の順だけ・共有する読み替えの規則は 0 個」と書くので、⒜ でも要件には反しない。
- ⒝ `areka-parsers` へ関数を移し、`areka-nar` と `areka-ghost` の両方が呼ぶ。規則は 1 か所。ただし完了 spec の `manifest.rs` を触る（本 spec の Out of Boundary は `crates/areka-nar/`）。
- ⒞ `areka-ghost` が `areka-nar` に依存する。`catalog.rs` の冒頭の「依存しない」に反する。

**D2. `balloon` の決め方を `resolve_skin_target` とどう揃えるか。**
- 丸ごと呼ぶと `random`・`lastinstalled` が特別に解かれ、要件 2 の 8 項に反する。共有できるのは最後の腕だけ。
- ⒜ 鎖の中に同じ 2 行を書き、「同じ決め方」をテストで固定する。
- ⒝ 最後の腕を関数に切り出して両方から呼ぶ（`shell_balloon_resolve.rs` に動きの変わらない差分）。
- どちらでも、`name` が複数一致したときは列挙の並びで最初（`list_balloons` はフォルダ名のバイト順に返すので、今の `resolve_skin_target` と同じ結果になる）。

**D3. 鎖へ渡す列挙の形。**
- 今は `&[String]`（フォルダ名）。`name` を持たせる形は、⒜ フォルダ名と `name` の組の列、⒝ `catalog::BalloonEntry` の列をそのまま、⒞ `SkinCandidate` の列（`balloon_candidates` を使い回す）。
- `resolve_balloon` のコメントは「列挙の並びは判断に使わない（裁定 3）」と書くが、要件 2 の 6 項の 4 つ目は並びを使う。コメントと §8 の記述を合わせて直す所である。
- 無作為の段は `listed[pick(n)]` で添字を引くので、並びと件数を今と変えない形が安全。

**D4. descript の 2 鍵の読み方と「読めない」の件数。**
- `sakura_name` 型の読み手を鍵ごとに 2 つ作ると、descript が読めないとき `catalog_descript_unreadable` が 2 件出て、要件 5 の 4 項（1 件）に反する。1 回の読みで 2 鍵を返す形が要る。
- 起動の経路では、その前に `list_ghosts` が同じ descript.txt を読んでいる（読めないゴーストは列挙から落ちる）。「読めない」に届くのは、引数でゴーストだけを渡した起動か、列挙の後で読めなくなった場合に限られる。テストは排他で開く形で作れる（`catalog_tests.rs` の `hold_exclusive` と同じ手）。

**D5. 記録の名前と欄。**
- 要件 5 の 2 項は「鍵・書かれていた値・バルーンの置き場」。今の 2 つの警告は段ごとに別の名前で、欄は値と置き場。descript の段は、⒜ 鍵ごとに別の名前、⒝ 1 つの名前に鍵の欄、のどちらでも要件を満たす。
- 値は作者の書いた文字列をそのまま載せることになる。`areka-nar` は記録に載せる値を `bounded_value` で切り詰めているが、今の `companion_balloon_not_found` は切り詰めていない。揃えるかどうか。
- `BalloonRoute` に足す腕は 1 つで足りる（要件 5 の 1 項は「descript の段で決まったこと」の区別だけを求め、どちらの鍵かは求めていない）。

**D6. 台帳の 2 行の状態と証拠の置き場。**
- `implemented` と書くなら、ソースの定義箇所に `// ukadoc: <URL>` の 1 行が要る（`doc/ukadoc-coverage/README.md` の 3 章）。置き場は読み手の関数の上（`catalog::homeurl` と同じ型）が素直。
- `default.balloon.path` は「相対パス」を「フォルダ名 1 段」に狭めて読む。`implemented` と `degraded`（動くが正典どおりではない・違いを注記に書く）のどちらが合うかは、`ghost-inner-balloon` が起点を広げる前提での判断になる。
- 担当（`owner`）・優先度・束の文（注記の末尾の「束: …」）は `ukadoc-survey` の `priority-apply` と報告の作り直しに関わる。台帳を触った後は `cargo run -p ukadoc-survey -- report` と `cargo test -p ukadoc-survey`（README の手順）。`doc/ukadoc-coverage/linkage.md` の「descript の転記」の束に、この 2 行が載っている。

**D7. テストの置き場。**
- 純粋な判断（要件 7 の 2〜4 項の大半）→ `boot_resolve.rs` の兄弟の新しいテストファイル（今の `boot_resolve_tests.rs` は 824 行）。
- 読み手（要件 7 の 1・5 項と「読めない」）→ `catalog.rs` の兄弟の新しいテストファイル（`catalog_tests.rs` は 642 行。補助は `catalog_test_support.rs`）。
- 切替（要件 7 の 6 項）→ `ghost_switch_tests.rs` は 988 行で足せない。`main_config_input_tests.rs` に在る `resolve_balloon_for_ghost` のテスト群（コメントが「切替先のバルーンを argv 無しの分岐で解く」と明記）へ足すか、切替を実際に回すテスト（`ghost_switch_test_support.rs` を使う兄弟ファイル）にするか。前者は「入力を集める口」だけを踏み、後者は切替の経路を踏む。
- 一時フォルダは `temp_path_kit::TempPath`（ワークツリーの `target\` の下へ作る・要件 7 の 7 項）。

## 6. 要件の読み方に幅がある所

実装できない矛盾ではないが、設計の前に読みを 1 つに決めておくと手戻りが無い。

1. **要件 5 の 1 項「今の記録と同じ形で残し」と切替（要件 4）**。`balloon_resolved` を出しているのは起動の入口（`resolve_boot_from`）だけで、切替の経路は今、どの段で決まったかを記録していない。「今と同じ形」を文字どおりに読むと、切替では記録が無いままでも満たす。切替でも段を記録に残したいなら、記録を `resolve_balloon_for_ghost` の側へ寄せる等の変更になる（その場合 `tools/package.ps1` が見る「バルーンを決めました」の行が起動で 1 件のままであることを保つ）。要件 7 の 3・6 項の「決まった段」は、戻り値の `route` で確かめられるので、テストはどちらの読みでも書ける。
2. **要件 2 の 6 項「同じ決め方」と 8 項「特別に解かない」**。実行中の決め方は `random`・`lastinstalled` を先に解く。6 項の箇条書きは `name` → フォルダ名だけを挙げているので、「同じ」が指すのはその 2 段だと読める（D2）。
3. **Introduction の「利用者が一度でもバルーンを選んだゴースト」と、記憶の実際の書かれ方**。`boot_resolve.rs` の `LastUsed::record` は、起動が成功するたびに、そのとき決まったバルーンを記憶へ書く（引数で渡した場合だけ書かない）。つまり記憶は「利用者が選んだ」ときだけでなく、**一度起動しただけ**でも出来る。帰結は 2 つ。
   - すでに一度起動したゴースト（開発機の `claudia` を含む）は、本 spec の後も今のバルーンのまま（要件 3 の 8 項のとおり）。実機で確かめるときは、記憶（`ghost/master/profile/areka/sylphya.toml` の `[last] balloon`）の無い状態から始める必要がある。
   - 作者が後から descript.txt や同梱を書き換えても、一度起動した利用者には効かない。要件 3 はこれを意図どおりとしている（「記憶を持たないゴースト（初めて起動したゴースト・記憶の先のバルーンが消えたゴースト）」）ので矛盾ではないが、§8 の記述に 1 文あると読み手が迷わない。

要件 1 の 15 項・2 の 4 項（区切り・`..`・絶対パス）は、列挙のフォルダ名との完全一致という今の作りのおかげで、専用の検査を足さなくても「当たらない＋警告 1 件」になる。値の検査を足すと、かえって記録の出口が増える（要件 5 の 5 項の上限に注意）。

## 7. 規模とリスク

- **規模: S〜M**（brief の 6〜10 タスクの見立てと合う）。本番の差分は 3 ファイル（`catalog.rs`・`boot_resolve.rs`・`boot_config.rs`）で小さく、量の大半は要件 7 のテスト（30 場面前後）と、文書 2 本（§8・台帳の 5 行と報告の作り直し）。
- **リスク: Low**。今の型（純粋な鎖・`catalog.rs` の単独の鍵の読み手・§8 の表・台帳）を伸ばすだけで、新しい依存も新しい仕組みも無い。
- 気を付ける所:
  - `BalloonRoute::Companion` の綴りを保つ（配布物の検査と煙テストが見ている）。
  - `lowercased` を通すと空の無印を見落とす（要件 1 の 14 項）。
  - descript が読めないときの記録を 1 件に保つ（D4）。
  - テストは兄弟の新しいファイルへ（1,000 行の番人）。
  - 台帳を触ったら報告を作り直す（`ukadoc-survey`）。
  - 後続の `shell-companion-balloon` は同じ 3 ファイルを触る（roadmap どおり直列）。

## 8. 設計へ持ち越す調べもの

- `ukadoc-survey` の `priority-apply` と報告が、`absent` から状態を変えた 2 行（束「既定で着せる吹き出し・読む経路が無い」・`linkage.md` の「descript の転記」）をどう扱うか。束の残りの行（`recommended.*` の 2 行）の順位の文が変わるか。
- ゴーストの descript.txt の文字コード: `lowercased` は `charset::decode`（既定は ANSI）を通す。`balloon,バルーン名` の値が日本語のとき、バルーンの descript.txt の `name`（同じ関数で読む）と同じ文字列になることを、文字コードの違う 2 ファイル（Shift_JIS のゴーストと UTF-8 のバルーン）で 1 場面確かめる価値があるか。
- 実機の確認の段取り（設計か実装の段）: `claudia` を記憶なしで起こして `claudia` のバルーンで出ること・手で `balloon,…` を足した検体で descript の段が効くこと。どちらも根はワークツリーの `target\` の下に作る。
- 外部の依存の調べものは無い（新しいクレートを足さない）。
