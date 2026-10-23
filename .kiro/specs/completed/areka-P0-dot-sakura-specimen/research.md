# ギャップ分析: areka-P0-dot-sakura-specimen

> 2026-10-22・ブランチ `claude/areka-p0-dot-sakura-specimen-0f24e9`（HEAD `4a1a86c0`）で測った。要件は確定済み（`requirements.md`）。ここに書くのは情報と案で、決めごとではない。
> 書庫は調べるために `target\gap-dot-sakura\dl\dot_sakura.nar` へ取り寄せ、`target\gap-dot-sakura\x\` に展開した（ワークツリーの `target\` の下だけ・追跡しない）。areka も GUI も起こしていない。

## 1. まとめ

- **既存の形にそのまま乗る。** 検体を足す手順（書庫を 1 つ置く＋登記表 `SAMPLES` に 1 行）・名前で引く窓口 `SampleRoot`・登記の往復のテスト・見張りのテスト・配布物の許可表は全部すでに在る。新しい部品は要らない。仕事は「書庫を置く」「1 行足す」「数を直す」「文書を書く」「実機で 1 回起こして記録する」「起票する」の 6 つ。
- **入らない見込みは低い。** `install.txt` は `type`・`name`・`directory`・`balloon.directory` だけで、どれも `areka-nar` が知っている鍵。取り出し元 `bottle/` は書庫の最上位に在る（`emily4` のような「名乗るのに無い」形ではない）。要件 2.7（断られたら登記しない）と 2.8（バルーンだけ入らない）の枝は、静的に読む限り通らない。
- **要件に名前の出ていない数が 1 種類ある。** 「登記の数 9 → 10」とは別に、「書庫を置く行（`Vendored`）の数 8 → 9」が、テスト 2 か所と README の 1 か所に在る（下の 3.2）。
- **実機の 1 回で崩れそうな所が、もう静的に 3 つ見えている**: ⑴ 別名の専用ファイル `alias.txt`（`sakura.surface.alias` を含む）を areka は読まない・担当の spec が無い ⑵ `surfaces.txt` の `alternativestart` を areka は描かない（`warn!`）・担当の spec が無い ⑶ 同梱バルーンの印（矢印・オンライン・SSTP）は `balloon-markers` の持ち物。起票先の無い ⑴⑵ は、実機で症状を確かめてから起票する。
- **SHIORI の版は静的には決まらない。** `shiori.dll` は i386 の PE で UPX で圧縮されており、中の文字列が読めない。辞書には 3.0 寄りの印（`event.On*`・`system.Reference0〜7`）と 2.x 寄りの印（`system.Sentence`・`system.communicate`・「SHIORI/2.2 Notify対応」の注記）が両方在る。版は実機の 1 回の記録で決める（要件 5.7 のとおり）。

## 2. 今のコードと資産（要件ごとの対応表）

印: **在る**＝今のまま使える／**足す**＝この spec で書く／**数**＝数を直す／**不明**＝実機で決まる／**制約**＝守ること

| 要件 | 使う物 | 状態 |
|---|---|---|
| 1.1〜1.2 取り寄せて大きさと SHA-256 を比べる | `curl`／`Get-FileHash`。2026-10-22 に取り寄せて 1,425,534 バイト・`0c602c6a…b7a472` で一致を確かめた。隣の `dot_sakura.nar.sha256` は 404 のまま | 在る（手作業） |
| 1.3 受け取ったバイト列のまま置く | `vendors/sample_ghost/.gitattributes` の `* -text`（`git check-attr` で `text: unset` を確かめた）。`.gitignore` には当たらない（`git check-ignore` 終了コード 1） | 在る・制約（`fold-samples` を通さない） |
| 1.4 追跡したファイルの SHA-256 を取り出し直して確かめる | `git show HEAD:vendors/sample_ghost/dot_sakura.nar` の SHA-256、または消して `git checkout` し直して測る | 足す（手順だけ） |
| 1.5 配布物に入れない | `tools/package.ps1` は `emo2` と `StayseeBalloon` だけを名指しで引き、最上位・`ghost/`・`balloon/` 直下を許可表で判定する（500〜506 行目） | 在る（変更 0） |
| 2.1 登記の 1 行 | `crates/sample-ghost-kit/src/lib.rs` の `SAMPLES`（131〜189 行目・今 9 行）に `Sample { name: "dot_sakura", kind: Ghost, balloons: &["bottle"], source: Vendored }` | 足す |
| 2.2 名前で引く | `SampleRoot::acquire`（`devroot.rs`）。行を足せば追加の作業なしで効く | 在る |
| 2.3 数 9 → 10 | `lib_tests.rs` の 7・198・206・260・262（関数名 `..._all_nine_known_names`）・497〜498 行目と、既知の名前の一覧（268〜280 行目） | 数 |
| 2.4 往復と位置のテスト | `every_registered_sample_lands_where_its_registry_row_says`・`every_sample_nar_installs_exactly_the_elements_its_registry_row_declares`（どちらも `SAMPLES` を回すので行を足せば `dot_sakura` を含む） | 在る |
| 2.5〜2.6 見張り | `crates/log-capture-kit/tests/sample_path_guard_test.rs`。語は `SAMPLES` から組む＝`vendors/sample_ghost/dot_sakura/`・`vendors/sample_ghost/dot_sakura"`・`join("bottle")`・`bottle/`・`/bottle"`・`dot_sakura("bottle")`。走査の範囲は `crates/` の下の `.rs` だけ（`workspace_scan/mod.rs` の `walk_workspace_sources`）。今の当たりは 0 件（`crates`・`tools` を `bottle`・`dot_sakura`・`dot-sakura` で引いて 0） | 在る（赤にならない見込み） |
| 2.7〜2.8 断られたとき | `areka-nar` の `manifest.rs`。鍵は全部既知（`KNOWN_KEYS`＋`balloon.directory`）。`balloon.source.directory` が無いので取り出し元は宛先と同じ `bottle`（`collect_companions`）。`bottle/` は書庫の最上位に在る | 在る（枝は通らない見込み） |
| 3.1〜3.6 検体の README | `vendors/sample_ghost/README.md`（224 行）。表は 14〜23 行目、見出しの「今の 8 本」は 9 行目。節の手本は `konnoyayame`・`claudia`・`emily4`（受け取ったバイト列のまま置いた 3 本） | 足す |
| 4.1 `structure.md` | 421 行目「9 本＝ゴースト 6 体…SHIORI 3 系統」 | 数・足す |
| 4.2 根の `README.md` | 38〜44 行目の表（5 行・`emily4` は在り、`pasta-in-windows` は無い） | 足す（実機の結果を待つ） |
| 4.3 `product.md` | 「適合の検体＝SHIORI 3 系統の標準テンプレート」の見出しと 4 行の表 | 足す |
| 5.1〜5.5 実機の 1 回 | 手本は完了 `areka-P0-emily-ghost-verification` の `verification.md`（同定・機械の構成・環境変数・起動の命令・項目ごとの結果・ERROR と WARN の数）。道具は `nar-sample-path`（`target\nar-samples\manual\<検体>\` を作り直す）・`tools/sync-i686-helper.ps1`・自動終了 `AREKA_APP_SMOKE_EXIT_MS`・記録の水準 `RUST_LOG` | 在る（手順を写す） |
| 5.7 SHIORI の版 | areka は SHIORI/3.0 だけを話す（`shiori-abi` の `IShiori`）。3.0 として読めない応答は `areka-ghost/src/shiori_inproc.rs` の `error!`「SHIORI/3.0 として解析不能」＋`ShioriError::Parse` になる＝2.x だけなら記録にこの行が出るはず | 不明（実機で決まる） |
| 5.10 areka のコードの変更 0 | 上の全部が今のまま使える | 制約 |
| 6.1〜6.6 起票 | `/kiro-discovery`・既存 brief への「件 N」 | 足す（実機の後） |

## 3. 測ったこと

### 3.1 書庫の中身（2026-10-22 取り寄せ）

- 項目 204（ファイル 196・フォルダ 8）。圧縮は deflate 129・無圧縮 75。名前は全部 ASCII。
- 最上位: `install.txt`・`delete.txt`・`updates.txt`・`readme.txt`・`ghost/`・`shell/`・`bottle/`。
- `install.txt`（`charset` 行なし＝areka は ANSI の既定で読む）: `type,ghost`／`name,．さくら`／`directory,dot_sakura`／`balloon.directory,bottle`。
- `bottle/install.txt`: `type,balloon`・`name,ボトル（でふぉ改）`・`directory,bottle`（同梱の側の `install.txt` は `areka-nar` が普通のファイルとして置く）。
- `bottle/descript.txt` は `charset` 行が無い（Shift_JIS の想定）。`font.name,ＭＳ ゴシック`・`validrect.bottom,-20`・`wordwrappoint.x,-20`・`arrow0/1`・`onlinemarker`・`sstpmarker`・`sstpmessage`・`number.*`・`communicatebox.*` を持つ。`balloons0s.txt` など面ごとの設定ファイルを持つ。
- ゴースト `ghost/master/descript.txt`: `charset,Shift_JIS`・`shiori,shiori.dll`・`sakura.name,さくら`・`kero.name,うにゅう`・`updateurl,http://ekicyou.github.io/old/dot-sakura/download/ghost/dot_sakura/`・`menu.*.bitmap.filename`（オーナードローのメニューの絵）・`shiori.logo.*`。
- `shiori.dll`: 140,800 バイト・PE の機械の種類 `0x014c`（i386＝32 ビット）・**UPX で圧縮**（`UPX0`・`UPX1`・`UPX!` の印）。読める文字は `KAWARI.kdt/7.4.`・`load`・`unload`・`request` だけで、`SHIORI/3.0` の綴りは見えない。辞書は `kawari.ini` が並べる Shift_JIS のテキスト約 40 本（`dot\` の下を含む）。
- 辞書の中の印: `system.Reference0〜7`（多い）・`event.On*`（`OnUpdate` など多数）・`system.Sentence`・`system.communicate`・注記「SHIORI/2.2 Notify対応アプリ汎用イベント」。**3.0 の印と 2.x の印が両方在る**ので、版は静的には決めない。
- シェル `shell/master/`: `surface*.png` だけ（`.pna` なし）・`surfaces.txt`（`descript { version,1 }` の新しい書き方・`collision`・`animation*.interval,rarely`／`talk,4`・`overlay`・**`alternativestart,(0,1)`**）・**`alias.txt`（`sakura.surface.alias`・`kero.surface.alias`）**・`surfacetable.txt`・`readme.txt`。`descript.txt` は `mode,material`・`sakura.balloon.offsetx/y`・`kero.balloon.offsetx/y`。
- 絵 133 枚: **`tRNS` を持つ絵は 0 枚**＝α は無い。色の型はパレット 126 枚（`bottle` の 1 枚は 2 ビット、残りは 8 ビット）・RGB 6 枚（シェル 4・ゴーストのメニューの絵 2）。
- `delete.txt`: `shell\master\Thumbs.db`・`shell\master\surface100.png` ほか、前の版の残りを消す一覧（`shell\sakura75\…` のように今の書庫に無いシェルも指す）。
- `updates.txt`（最上位と `ghost/master/` に同じ 194 行）: 古い形の `file,<パス><MD5>size=<数>`。
- 起動の記録（`profile/`）は無い。`ghost/master/dict-keeps-savedata.txt` は辞書（保存の仕組みの定義）で、走行の残りではない。
- 取り扱い（`readme.txt`）: 「無償ですが著作権複数のためフリーには出来ません」「転載・配布: インターネット上は不可、それ以外は各自の責任において可」。シェルの `readme.txt`: 「リビルド: 日蔭 翠」「インターネット上でのアーカイブの転載不可」。要件 3.3 の文言と合う。

### 3.2 数を持つ所（直書きの 9 と 8）

**登記の数 9 → 10**（要件 2.3 が名指しする数）

| 場所 | 中身 |
|---|---|
| `crates/sample-ghost-kit/src/lib_tests.rs` 7 行目 | 冒頭の説明「既知の 9 つ」 |
| 同 198 行目 | 説明「登記した 9 つの検体」 |
| 同 206 行目 | `assert_eq!(SAMPLES.len(), 9, "登記は検体 9 つ")` |
| 同 260 行目 | 説明「既知の名前 9 つ」 |
| 同 262 行目 | 関数名 `unknown_sample_fails_with_all_nine_known_names` |
| 同 268〜280 行目 | 既知の名前の一覧（9 つ・`SAMPLES` の順） |
| 同 497〜498 行目 | `SAMPLES.len()` と 9・「登記は検体 9 つ」 |
| `.kiro/steering/structure.md` 421 行目 | 「9 本＝ゴースト 6 体」・「SHIORI 3 系統」 |

**書庫を置く行（`Vendored`）の数 8 → 9**（要件の本文に名前が無い数）

| 場所 | 中身 |
|---|---|
| `lib_tests.rs` 132 行目 | 説明「今の 8 行は `Vendored`」 |
| 同 141〜153 行目 | `Vendored` の名前の一覧（8 つ）と「今の 8 行は Vendored のまま」＝**`dot_sakura` を足さないと赤** |
| `crates/sample-ghost-kit/src/devroot_fetched_tests.rs` 67 行目 | 説明「今の 8 本の引き方は変わらない」 |
| `vendors/sample_ghost/README.md` 9 行目 | 「書庫を置く行（今の 8 本）」（要件 3.1 の「見出しの数を実数に合わせる」が当たる） |

**数ではないもの（直さない）**: `devroot_sweep_tests.rs` の「8 本」はスレッドの数。`nar_sample_path_test.rs`・`nar-sample-path.rs`・`devroot_fetched_tests.rs` の繰り返しは `SAMPLES` から導くので数を持たない。`areka-emo-atlas/src/samples_e2e_tests.rs` の `SAMPLES` は別の表（絵の検体）。

**ついでに見つけた抜け**: `unknown_sample_fails_with_all_nine_known_names` の後半（失敗の表示に名前が載るかを見るループ・283〜292 行目）は 8 つしか並べておらず、`claudia` が無い。前半の一覧は 9 つ揃っている。

### 3.3 `areka-nar` の扱い（静的に読んだ結果）

- 同梱の探索は `balloon` → `balloon0` → …の順。`balloon.directory,bottle` は無印で見つかり、`balloon.source.directory` が無いので取り出し元は `bottle`（`manifest.rs` の `collect_companions`）。
- 本体（`ghost/dot_sakura/`）からは取り出し元 `bottle/` の項目を除き、`bottle/` の中身を `balloon/bottle/` に置く（`plan.rs` 158〜168 行目）。最上位の `install.txt` はゴーストでは本体側に置かれる（除くのは `supplement` だけ）。
- 最上位の `delete.txt`・`updates.txt`・`readme.txt` は**普通のファイルとして `ghost/dot_sakura/` の直下に置かれる**。`areka-nar` は `delete.txt` を読まない（読むのはネットワーク更新の `areka-update/src/delete.rs` だけ）。ukadoc（「ネットワーク更新」のページ）でも、ゴーストの最上位に `updates2.dau`・`updates.txt`・`delete.txt` を置くのが正しい配置である。＝**インストールで警告が出る理由は見当たらない**（予想は 0 件。要件 5.7 の 1 項目目は実機で確かめる）。
- 断る理由になりうる鍵・値（宛先の 1 階層の検査など）にも当たらない。

### 3.4 見張りの語との当たり

0 件（上の表）。`bottle/` という語は短く一般的なので、**今後** `crates/` の `.rs` に `bottle/` という綴りの文字列（コメントの外）を書くと当たる。今の時点では問題にならない。

### 3.5 表示の経路で予想される崩れ（実機で確かめる）

| 予想 | 根拠 | 持ち主 |
|---|---|---|
| `alias.txt` の別名が全部落ちる（`\s[9]` などが番号で解かれず、面が出ない・黙って壊れる） | 網羅台帳 `doc/ukadoc-coverage/ledger/assets.toml` 5987 行目「⑵ 本体側の綴り `sakura.surface.alias` は無い」「⑶ `alias.txt` を読む道が無い」 | **担当なし**（担当だった `areka-P0-shell-parse` は完了） |
| まばたきの `alternativestart,(0,1)` が描かれず `warn!` が出る | 台帳 6024 行目（`status = "absent"`・`owner = ""`・記録 `warn!`） | **担当なし** |
| 矢印・オンライン（16 コマ）・SSTP の印が出ない | 要件の範囲外の宣言 | `balloon-markers` |
| 入力の箱 `balloonc0〜3`・`communicatebox.*` | 要件の範囲外の宣言 | `balloon-canon-residue`・`inputbox-user-input` |
| α の無い絵の抜き色 | シェルもバルーンも絵は `areka-emo-atlas` の `normalize`（宣言なし＝透明な画素が無ければ左上の 1 画素の色を抜く）を通る。抜けるはず。バルーンの角の赤が左上の画素と同じ色かは実機で見る | 在る |
| SHIORI/2.x だけの応答 | 3.0 として読めない応答は `error!`＋`Parse` | 担当なし（要件で対応は範囲外） |
| `GET Sentence`（2.x のゴースト間の会話） | `system.communicate` を使う辞書。areka は送らない | 方針の確認が要る（範囲外） |
| オーナードローのメニューの絵（`menu.*.bitmap.filename`） | areka のメニューは OS のメニュー | 既存の方針（`popup-menu-shell-look` などの持ち物か要確認） |

## 4. 実装の案

### 案 A: 今の部品を広げるだけ（推奨の候補）

- 書庫を置く・`SAMPLES` に 1 行・`lib_tests.rs` と `devroot_fetched_tests.rs` の数・README 3 本と steering 2 本・`verification.md` を新しく 1 本。
- 判定は既存の往復のテストと位置のテストが `dot_sakura` を含めて受け持つ。
- ✅ 新しいテストも部品も 0。`emily4` のときと同じ形。
- ❌ 「`bottle` が `balloon/bottle/` に入る」は往復のテストの中でだけ判定され、専用の名前のテストは無い（`emily4` には専用のテストが在る）。

### 案 B: 専用の判定テストを 1 本足す

- 案 A に加え、`emily4_nar_installs_...` の隣に「`dot_sakura.nar` を空の根へ入れると、ゴースト `dot_sakura` とバルーン `bottle` が入り、飛ばした同梱は 0 件・最上位の `delete.txt`・`updates.txt` は `ghost/dot_sakura/` の直下に普通のファイルとして在る」を 1 本。
- ✅ 要件 5.7 の 1 項目目（`delete.txt`・`updates.txt` の扱い）に、実機を待たない決定論の根拠が付く。
- ❌ テストが 1 本増える。`lib_tests.rs` は 726 行（上限 1,000 行の目安まで余裕はある）。

### 案 C: 実機の 1 回を実機の試験の窓口（`real-run-kit`）で書く

- `real-run-kit` の `Launch`（検体）で起こし、MCP で面とバルーンを取って判定するテストにする。
- ✅ 繰り返せる。
- ❌ 要件 5 は「1 回起こして記録する」で、繰り返す試験は求めていない。崩れが多いと予想されるゴーストで緑の条件を決められない。規模が S を超える。

## 5. 規模と危うさ

- **規模: S**（4〜6 タスク）。既存の形のなぞり＋文書＋手作業の実機 1 回。
- **危うさ: 低**（コードの側）。登記と文書は機械的。**実機の 1 回は不確か**だが、直しは範囲外なので本 spec の規模は膨らまない。膨らむのは起票の件数だけ。

## 6. 設計の段へ渡す調べ物（Research Needed）

1. **華和梨 7.4 の SHIORI の版**: UPX のため静的に読めない。展開する道具（upx）を入れるのは新しい外の道具になるので避け、実機の記録（3.0 として読めたか・`Parse` の `error!` が出たか）で決める。
2. **32 ビットの載せ台で UPX の DLL が読めるか**: `LoadLibrary` は普通に読むはずだが、実機で確かめる。
3. **`delete.txt` をインストールのときに SSP が当てるか**: ukadoc の配置の説明はネットワーク更新の文脈。新しい根に入れる限り消す物は無いので、本 spec の記録には影響しない見込み。
4. **`updateurl` が `http://` の古いサイトを指す**: 実機の 1 回でネットワーク更新を押すと開発者の古いサイトへ取りに行く。走行の手順で触らないと決めるか、確かめる項目に入れるか（要件 5.7 の項目には無い）。
5. **バルーンの `descript.txt` に `charset` 行が無い**: 既定の文字コードで読めるか（`font.name,ＭＳ ゴシック` が化けないか）を実機で見る。

## 7. 設計で決めること（要件の討議へ渡す議題の候補）

1. **登記表の行の位置**: `dot_sakura` を末尾（`pasta-in-windows` の後）に置くか、書庫を置く行の最後（`emily4` の後・`pasta-in-windows` の前）に置くか。既知の名前の一覧の順（`SAMPLES` の順）と、`Vendored` の名前の一覧の順が変わる。後に続く `default-balloon-selfmade`（10 → 11）も同じ決まりに乗る。
2. **書庫を置く行の数 8 → 9 を要件に書くか**: `lib_tests.rs` 132・141〜153 行目・`devroot_fetched_tests.rs` 67 行目・README 9 行目。要件 2.3 は登記の数 9 → 10 だけを名指しする。要件 2.3 の「数の主張・文書の注釈」に含めて読むか、1 項足すか。
3. **README の「中のファイル数」の数え方**: 書庫の項目数（204＝ファイル 196＋フォルダ 8）で書くか、ファイルだけ（196）で書くか。今の表は項目数で数えている（`emily4` は 260＝ファイル 251＋フォルダ 9、`claudia` は 135 で同じ）。brief は 204 と書く。
4. **既知の名前の表示のループに `claudia` が無い抜け**（3.2 の末尾）を、同じ所を触るこの spec で一緒に直すか。
5. **専用の判定テストを足すか**（案 A か案 B）。足すなら `delete.txt`・`updates.txt` が普通のファイルとして置かれることを決定論で固定できる。
6. **SHIORI の版の根拠の取り方**: 実機の記録の行（3.0 として読めた応答・または `Parse` の `error!`）を根拠にする案と、UPX を展開して読む案（新しい外の道具が要る）。
7. **記録の水準 `RUST_LOG` の中身**: `emily4` の手本（`info,areka::install=debug,areka::boot_resolve=debug,areka_nar=debug`）に、SHIORI の応答の読みと面の別名の解決の分岐（`areka_ghost`・`areka_seriko`・`areka_emo_compose`）を足すか。
8. **実機の走り方**: 要件 5.1 どおり「検体の窓口（`nar-sample-path dot_sakura`）が作った根から起こす」だけにするか、`emily4` の手本（`emo2` と `StayseeBalloon` だけの根で起こし、メニューの「インストール…」から `.nar` を入れた）のように、メニューからのインストールも 1 回通すか（インストールの警告の有無を画面で見られる）。
9. **ネットワーク更新を触るか**（6 の 4）。
10. **起票先の無い崩れのまとめ方**: `alias.txt`／`sakura.surface.alias`（別名）と `alternativestart` はどちらもシェルの `surfaces.txt` まわり。触る場所が同じなら 1 本にまとめるか、別名と SERIKO で分けるか（10-11 の「触る場所が同じものを 1 本に」の決まり）。
11. **作業の順**: 根の `README.md` の備考（要件 4.2）は実機の結果を書くので、実機の 1 回の後に置く。登記（要件 2）・検体の README（要件 3）は実機より前に済ませられる。要件 2.7 の枝（断られたら登記しない）の判定は、登記の前に往復のテストを先に回して確かめる順にするか。

## 8. 次の段

要件の討議で 7 節の議題を決めた後、`/kiro-design areka-P0-dot-sakura-specimen` で設計へ進む。

## 9. 設計の段の調べ（2026-10-22・`/kiro-spec-design -y`）

- **調べの型**: 軽い調べ（既存の仕組みへの追加）。新しい依存・外の道具は 0 なので、外の文書の調べ（WebSearch）は要らなかった。使った決まりは `kiro-spec-design` の `rules/`（`design-principles.md`・`design-discovery-light.md`・`design-synthesis.md`・`design-review-gate.md`）。ほかの技能（画面の設計・使いやすさ）は、画面を作らない spec なので当たらない。
- **読み直したコード**（HEAD `6270ccc6`）:
  - `crates/sample-ghost-kit/src/lib.rs` の `SAMPLES`（9 行・`pasta-in-windows` は `emily4` の後に末尾へ足された＝登記表は足した順）。
  - `crates/sample-ghost-kit/src/lib_tests.rs`: 冒頭の説明の「主張は 8 本」は**主張の数**で、専用テストを足せば 9 本になる。未登録の名前のテストは、`known` と比べる一覧（9）と失敗の表示を見る一覧（8・`claudia` 無し）を 2 つ手で書いている＝1 つの配列にまとめれば抜けが構造的に消える。
  - `crates/log-capture-kit/tests/sample_path_guard_test.rs`: 窓口の定義 `crates/sample-ghost-kit/src/` は走査から外れるが、`EXCLUSION_FORMS` は ⑵（展開形の検体フォルダ）と ⑷（同梱バルーン名でのパス組み）を `ComposedAtRuntime`（窓口の中にも字面で現れない）と宣言し、現れたら赤にする。**専用テストを `lib_tests.rs` に書くとき、`join("bottle")`・`bottle/`・`/bottle"`・`dot_sakura("bottle")` を綴ると見張りが赤になる**＝同梱の名は登記の行から引いて継ぎ足す。
  - `crates/areka/src/install/procedure.rs`: 画面からのインストールの警告のダイアログは `InstallOutcome::skipped` が空でないときだけ出る。`areka-nar` の `install` は `warnings`（manifest の行を飛ばした）・`leftovers`・`skipped` を 1 件ずつ `warn!` で出す。
  - `crates/areka/src/boot_resolve.rs`: 根にゴーストが 1 体なら経路 `Only`。同梱バルーンは既定より先に採る。根に `emo2` を足すと経路が `Default`（`emo2`）に替わる。
  - `crates/areka/src/cli/parse.rs`: 起動オプション `--install` は `Unsupported`。インストールを画面の操作なしで起こす口は台本の `\![execute,install,path,…]`（`emo2_boot/install_cue.rs`）だが、起動中のゴーストへ入れると切り替えの一周（`install/overwrite.rs`）になる。
  - `crates/areka/src/alert.rs`: `AREKA_NO_ALERT` で告知のダイアログを抑え、記録には残る。
  - 記録の的: `ghost-boot`（`areka-ghost` の SHIORI の結線・文字コード）・`shiori-host32`（補助プログラムの向き `helper_not_i686`）・`shiori-charset`・`areka::boot_resolve`・`areka_nar`。`areka-emo-atlas` は抜き色の判断を記録に出さない＝透過は MCP の `dump_surface` を文で説明する。
  - `seriko-script-triggers` の brief は `alternativestart` などのアニメーションから呼ぶメソッドを範囲の外と書く。`alias.txt`・`sakura.surface.alias` を持つ spec は無い（`makoto-dll-host` の `alias.txt` は `makoto,` の上書きの話で別物）。

### 9.1 まとめ直し（synthesis）

- **一般化**: 無し。検体を足す形はすでに一般化されている（登記の 1 行で全テストが拾う）。検体ごとの関数を足さない。
- **作るか使うか**: 全部使う。取り寄せは `curl`・`Get-FileHash`・`sha256sum`、根は `nar-sample-path`、補助プログラムは `sync-i686-helper.ps1`、観察は MCP の `areka` の道具、机は `areka-impl-watch`。SHIORI の版を読むための `upx` は入れない（新しい外の道具になる）。
- **削ったもの**: 実機の試験の窓口（`real-run-kit`）での繰り返せる試験（研究の案 C）・メニューからのインストール・画面の操作（SendInput）・時間で出る話を待つ観察・`registry_records_kind_and_bundled_balloons` への行の追加（往復のテストが同じ事を見る）。

## 10. 設計で決めたこと（7 節の議題への答え）

| 議題 | 決めたこと | 理由 |
|---|---|---|
| 1 登記表の行の位置 | 末尾（`pasta-in-windows` の後） | 登記表は足した順に並ぶ（`emily4` の後にぱすたを足した）。書庫の出どころで並べ替える決まりは無い。後に続く `default-balloon-selfmade` も末尾へ足せる |
| 5 専用の判定テスト | 足す（案 B）。名前 `dot_sakura_nar_installs_the_ghost_and_its_balloon_without_warnings`・`emily4` の専用テストの直後 | 往復のテストは `installed` だけを見て、`skipped`・`warnings`・`leftovers` と最上位の普通のファイルの置き場を見ない。要件 5.7 の 1 項目目（警告と `delete.txt`・`updates.txt` の扱い）の根拠を、メニューからのインストールなしに決定論で持てる。`installed` の一致は往復のテストに任せて繰り返さない |
| 6 SHIORI の版の根拠 | 実機の記録の行と MCP の `raise_event` の台本。`upx` は使わない | 新しい外の道具を入れない。areka は SHIORI/3.0 だけを送るので、台本が返って再生されれば 3.0 で話せた、応答が 400 やエラーばかりなら 2.x だけの見込み |
| 7 `RUST_LOG` | `info,areka::boot_resolve=debug,areka_nar=debug,ghost-boot=debug,shiori-host32=debug,shiori-charset=debug` | 判定の分岐（ゴーストとバルーンの決め方・補助プログラムの向き・文字コード）を debug まで開ける。メニューからのインストールをしないので `areka::install` は外す。分岐の行が出ていなければその水準まで開けて回し直す |
| 8 実機の走り方 | 窓口の根（`nar-sample-path dot_sakura`）から起こすだけ。メニューからのインストールはしない。終わりは MCP の `raise_event`（`OnClose`）。`AREKA_NO_ALERT=1` | 要件 5.1 が窓口の根を決めている。根に `emo2` を足すと起動の経路が替わる。画面からのインストールは `areka-nar` の同じ `install` を呼び、ダイアログは `skipped` だけで決まるので、専用テストで足りる。画面の操作（SendInput）は開発者の許しが要り、人の待ちになる |
| 9 ネットワーク更新 | 触らない（要件 5.11 で決まり済み） | — |
| 10 起票先の無い崩れのまとめ方 | 2 本に分ける見込み（`alias.txt` の別名＝シェルの設定の読みと面の番号の解き方／`alternativestart` の族＝SERIKO の動き）。実機で症状を見てから `/kiro-discovery` | 触る場所が違う（開発者の決め 10-11「触る場所が同じなら 1 本」の裏返し）。`seriko-script-triggers` はこの族を範囲の外とするので、そこへ足さない |
| 11 作業の順 | 書庫 → 登記と専用テストと数（コミットの前にテストで断られたかを決める）→ 見張り → 検体の README と steering 2 本 → 実機 → 記録と起票 → 根の `README.md` | 要件 2.7 の枝は登記をコミットする前に決める。根の `README.md` は実機の結果を書く |

議題 2・3・4 は要件の討議で決まり済み（書庫を置く行の数 8 → 9 を要件 2.3 に含める・項目の数で数える・`claudia` の抜けを直す）。

### 10.1 残る危うさ

- **インストールの 1 項目目の根拠の形**: 実機の 1 回で areka の画面からのインストールを通さないので、「インストールで出た警告」の根拠は、窓口の根の一覧・起動の記録の行（ゴーストとバルーンが見つかった行）と、専用テストの結果の 3 つになる。要件 5.7 の「記録の行の逐語・MCP の文」の形に入らないのは専用テストの部分だけで、開発者の決め（確定の証跡は実機の記録と静的な構造の証跡の二本立て）に乗る。討議で「画面からのインストールも 1 回通す」と決まれば、手順に MCP 経由ではない画面の操作が加わり、開発者の許しが要る。
- **`raise_event` の `OnClose` で終わるか**: 辞書の `OnClose` が `\-` で終わらなければ終わらない。そのときは自動終了の上限か、控えた番号の停止で終える（記録に書く）。
- **MCP が答えないとき**: SHIORI を読めないと MCP の道具が使えず、項目の多くが「確かめていない」になる（要件 5.8 のとおり）。

## 11. 設計の討議で決めたこと（2026-10-22）

- 実機の 1 回の終わらせ方: `raise_event OnClose` を送り、返った台本に `\-` が在るかを記録する。無ければ MCP の `sakurascript` で `\-` を送って終わらせる。メニューの終了の道は「確かめていない」と書き、`\-` が無いことは崩れに数えない。根拠: 本 spec の目的は古い作りの崩れを洗うことで、終了の道はゴーストに依らない。画面の操作を入れずに済む（設計の検証の指摘 1・開発者の承認）。
