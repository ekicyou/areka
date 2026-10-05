# 設計の検証レポート: areka-P0-mcp-expression-table

作成: 2026-10-04（ブランチ `claude/areka-p0-mcp-expression-table-440874`）。
対象: `design.md`（生成済み・未承認）。突き合わせた相手: `requirements.md`・`research.md`・`ssp-measurements.md`・`.kiro/steering/`・今のコード。
進め方: 対話なし。SSP は操作していない。

## 1. まとめ

**判定: GO**（設計のやり直しは要らない。下の 3 点は設計の討議で文言を足せば済む）。

設計が今のコードについて言っていることは、すべて実物で確かめられた。行の読み分けの規則を `ssp-measurements.md` の 9 つの検体すべてに当てたところ、SSP の答えと違う行は 0 件だった。同じウェーブの約束（触らないファイル）も守れている。残るのは「読み分けの表の細かい決め漏れ」「1 本のテストの揺れの芽」「期待値の出どころの書き直し」の 3 つで、どれも設計の形を変えない。

## 2. 確かめたこと

### 2.1 今のコードについての主張（すべて実在）

| 設計の主張 | 確かめた場所 | 結果 |
|---|---|---|
| `handle(world, ghost, args, reply)` の並びと `#[path]` のテスト接続 | `crates/areka/src/mcp/get_expression_table.rs` | そのとおり。今は `outcome::ng("not implemented yet")` を返すだけ |
| `dispatch` が `Omitted::Reject` で解決してから `handle` を呼ぶ・`drain` が 1 件ずつ回す | `crates/areka/src/mcp/mod.rs` の `dispatch`・`drain` | そのとおり（6.3・6.4 は触らずに満たす） |
| `ActiveGhost` は `name` と `root` だけ | `crates/areka/src/mcp/resolve.rs` の `ActiveGhost` | そのとおり |
| `GhostSlot` → `GhostSession::runtime()` → `mount()` → `shell.dir` が `mcp` の下から届く | `crates/areka/src/ghost_session.rs` の `GhostSlot`（`pub(crate)`・中身の欄も `pub(crate)`）・`GhostSession::runtime`（`pub(crate)`）、`crates/areka-ghost/src/runtime.rs` の `GhostRuntime::mount`（`pub`）、`crates/areka-parsers/src/package/model.rs` の `MountModel.shell`・`ShellMount.dir`（`pub`） | 届く。同じ引き方の前例が `resolve.rs` の `active` と `boot_shell_tests.rs` の `mounted_shell` にある |
| 切替の後は `set_shell_dir` が `shell.dir` を書き換える | `GhostRuntime::set_shell_dir`・`GhostSession::set_shell_dir`（`pub(crate)`）・呼び手は `emo2_boot/frame/switch.rs` | そのとおり（6.2 は毎回読めば満たす） |
| `charset::decode` は綴りの大小を無視・BOM を読み飛ばす・知らない名前は `debug!` で既定へ戻る | `crates/areka-parsers/src/charset/decode.rs` の `decode`・`prescan.rs` の `prescan_charset` | そのとおり。名前の判定は `encoding_rs::Encoding::for_label` |
| `encoding_rs` を読み手で使える（`Cargo.toml` を触らない） | `crates/areka-parsers/Cargo.toml` の `[dependencies]` に `encoding_rs` | 使える。`crates/areka` には直接の依存が無いが、設計は `crates/areka` の側では使わない（Shift_JIS のテストはバイト列を直に書く） |
| テストの道具が `crates/areka` に既にある | `crates/areka/Cargo.toml` の `log-capture-kit`・`temp-path-kit` | ある |
| `SwitchRig` の経路 | `crates/areka/src/emo2_boot/ghost_switch_test_support.rs` の `SwitchRig`（`pub(crate)`・`world` の欄も `pub(crate)`）。使用例は `mcp_tests.rs` の `real_unit_answers_get_active_ghost_list_in_one_frame` | ある。見本のゴーストを一時の根へ写して起こすので、`shell.dir` に `surfacetable.txt` を書いても見本の元は汚れない |
| `outcome::value`・`Args { ghost_name }` | `crates/areka-mcp/src/tools/outcome.rs`・`tools/get_expression_table.rs` | ある |
| 2 つ目の読み手の前例 | `crates/areka-parsers/src/shell/boxes.rs` と `shell/mod.rs` の 3 か所（`mod`・テストの `mod`・`pub use`） | そのとおり |
| `surfacetable.txt` という名前のフォルダは「開けない」になる | Windows の `std::fs::read` はフォルダに対して「見つからない」以外の失敗を返す | 設計のテストの置き方で 5.8 の枝を踏める |

### 2.2 検体の机上の通し（`ssp-measurements.md` の 1〜9）

設計の「行の読み分け」と `render` の規則 1〜5 を、検体ごとに手で当てた。

| 検体 | 見どころ | 設計の結果 | SSP と |
|---|---|---|---|
| 1 えも2DEBUG | `group,0`（見出し語が先に当たるので ID の行と取り違えない）・タブの字下げ・既定の 10・11・19 が 9 と 20 の間、25 が 21 の後 | 同じ並び・同じ行 | 一致 |
| 2 ファイル無し | 既定の 15 件だけ | 空の転記 → 15 行 | 一致 |
| 3 平たいファイル | `group` の外の行はスコープ 0・キャラクタ名は空。20 がシェルの名前に替わり、30・40・50 が末尾 | 同じ | 一致 |
| 4 名前の省略・閉じていない `{` | `0,` が `\|\0\|本体基本\|\|\s[0]\|` で載り、既定の 0 は消える | 同じ | 一致 |
| 5 `__disabled` | 10 を `__disabled` の中に書くと既定の 10 が消え、11・19・20・25 は残る | 同じ（書かれた ID の集合を全行から作る） | 一致 |
| 6 グループ名が空・`scope` 無し | キャラクタ名は空・スコープ 0。既定は 11 と 20 だけが ID の位置に入る | 同じ | 一致 |
| 7 スコープ 2・同じスコープの `group` が 2 つ | `\p[2]`・`エミリオ` と `その他` が ID の順に混ざる・スコープ 1 に書いた 10・11・19 が既定を消す・25 は残る | 同じ | 一致 |
| 8 `option` | 定義の無い 19・20・25 も載る。グループ名 `\0` がそのまま出る | 同じ（`option` の行は読み飛ばす） | 一致 |
| 9 行末の `}` | `100,黒塗り}` は名前 `黒塗り}` で載る | 同じ（閉じるのは `}` だけの行） | 一致 |

違いの出た検体: **0 件**。

### 2.3 同じウェーブの約束

設計が書き換えるのは `crates/areka-parsers/src/shell/{surfacetable.rs, surfacetable_tests.rs, mod.rs}` と `crates/areka/src/mcp/{get_expression_table.rs, get_expression_table_tests.rs}` の 5 本だけ。`mcp/mod.rs`・`resolve.rs`・`crates/areka-mcp/src/**`・すべての `Cargo.toml`・`shell/model.rs`・`shell/decode.rs` に触る箇所は **0 件**。
`roadmap.md` の C3 の行は「`shell/mod.rs` の 1 行」と書いているが、設計は 3 か所（`boxes` と同じ形）。同じウェーブで `shell/` を触る `surface-element-nesting` は `shell/mod.rs` に触らない約束なので、ぶつからない。

## 3. 討議に持ち込む点（最大 3 つ）

### 1. 行の読み分けの表に、実装者ごとに答えが割れる決め漏れが 3 つある

- **名前の前後の空白**。表の前書きは「見出し語と値は…それぞれ前後の空白を落とす」と言い、`数値,名前` の行は「名前は最初の `,` より後ろの全部」と言う。`10, 素` の名前が `素` か ` 素` かが読み手によって変わる。グループ名（`group, 名前`）も同じ。
- **「空白」の範囲**。Rust の `trim` は全角の空白（U+3000）も落とす。名前やグループ名の端に全角の空白を置いたシェルでは、要件 1.6（元の字のとおり）と食い違う。落とすのを ASCII の空白とタブ（と行末の `\r`）に限るかを書く。
- **ID の数値の読み方**。「`u32` として読める 10 進数」を Rust の `parse::<u32>` で書くと `+5,名前` が ID 5 として通る。「ASCII の数字だけ」と書けば割れない。

どれも SSP の実測は無く、9 つの検体の結果は変えない。表に 1 行ずつ決めを足し、読み手のテストに 1 本ずつ置けば済む。

### 2. 「SHIORI の呼出の記録が前後で増えない」のテストは、そのままだと揺れうる

配線のテスト（6.1）は `SwitchRig` で起こした直後に呼出の記録の数を比べる。起動の一連の呼出（`OnBoot` など）は別スレッドから遅れて届くので、起こしてすぐ比べると、本ツールと関係なく数が増える回がありうる。`SwitchRig::wait_steady`（定常に着くまで待つ口）を先に呼んでから「前」を採る、と設計に書く。

合わせて、`handle` は World を読むだけで kanade への送り口に触れる道が無い。この 1 点を本物の単位で見張る値打ちが、揺れの芽に見合うかも討議で決めたい（外すなら 6.1 は「`handle` が送り口を持つ型に触れない」ことの説明で足りる）。

### 3. 期待値の出どころが古いまま（軽い直し）＋削れるもの

- `design.md` の末尾「実装の前に要るもの」は、検体の全文がリポジトリに無いと書いているが、`ssp-measurements.md` が後から入った。この節を「期待値は `ssp-measurements.md` から起こす」に書き直す。Testing Strategy の 7.1・7.2 も、検体 1 の全文に加えて、検体 3（平たい Shift_JIS・既定の 20 が替わる）・5・6・8（グループ名 `\0`）・9 を、同ファイルの文面と答えのまま使うと書けば、期待値を手で作り直す余地が無くなる。
- `UnreadableLine.text` は使い手が居ない（`load` の記録はパス・件数・行番号だけ）。`unreadable` を行番号の列にすれば、公開の型が 1 つ減る。記録に行の文面も載せたいなら、そう書いて残す。
- 読み手の `charset,名前` の確かめは、`charset::decode` の読み方（冒頭の ASCII の部分・最初の 1 つだけ）と範囲が少し違う（読み手はファイルのどこにあっても見る）。害は無いが、「2 つは同じ判定ではない」と 1 行書いておくと後で迷わない。

## 4. 良い点

- **読み手は全行を転記し、載せる・載せないは `render` の 1 か所**。「書かれている ID」の判定（要件 3.4）に `__disabled`・`__parts`・名前の省略・別スコープの 4 つの場合があるが、旗を足さずに「全行から ID の集合を作る」1 手で済んでいる。検体 4・5・7 がこの 1 手で通る。
- **足さなかったものが明確**。別スレッドの読み取り・読んだ結果の溜め置き・`surfaces.txt` の読み取り・既定の名前のファイル・`Shell` 型への欄を、理由つきで外している。新しい依存も `Cargo.toml` の変更も無く、同じウェーブの 10 本と触るファイルが重ならない。

## 5. 判定と次の一歩

**GO**。理由: 今のコードへの主張はすべて実在し、9 つの検体で SSP と違う行が 0 件、約束のファイルへの接触が 0 件。上の 3 点は表とテストの文言の追記で閉じ、設計の形（読み手は転記・組み立てはツールの側・その場で読む）は変わらない。

次: 設計の討議で 3 点を決めて `design.md` に反映 → `/kiro-spec-tasks areka-P0-mcp-expression-table`。
