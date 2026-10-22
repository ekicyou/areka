# ギャップ分析（areka-P0-pasta-in-windows-verification）

- 作成: 2026-10-22（日本時間）・ブランチ `claude/areka-p0-pasta-windows-d4ed7a`（main `6e0ea8d8` の上・コミット `1b8c4747`）
- 入力: `requirements.md`（要件 1〜9）・`brief.md`・ステアリング（product・tech・structure）・既存のコード
- やり方: コードと文書を読んで突き合わせただけ。SSP も areka も起こしていない。書庫は取り寄せていない（通信 0）。重いテストは回していない。

## 1. 要約

- **既存の部品で大半が書ける**。検体の窓口（`crates/sample-ghost-kit/`）・取り寄せの段（`tools/sample-fetch.ps1`）・実機の試験の窓口（`crates/real-run-kit/`）・32 ビットの補助の揃え（`tools/sync-i686-helper.ps1`）・えみりの検証の記録の型（`.kiro/specs/completed/areka-P0-emily-ghost-verification/verification.md`）が、そのまま使える。新しく書くコードは、自動テスト 1 本と（決めによっては）α のしきい値の直しだけ。
- **書庫の中身は要件 2 の前提どおり**。ghost_dev の公開元の手元の書庫（読むだけ）では、`pasta_tategaki/` が書庫の根（＝入れた後のゴーストのフォルダの直下）に在り、その descript.txt が `name,窓際のぱすたさん縦書き` を名乗る。`install.txt` に `balloon.directory` は無い（入れた後の `balloon/` は 0 件になる見込み）。ただし、どのワークツリーの `target\` にも取り寄せた書庫はまだ無い。
- **α のしきい値は 3 か所**（`alpha_mask.rs` の整数 1 つと `hit_test/mod.rs` の小数 2 つ）。シェルとバルーンの当たりは前者だけを通る。後者 2 つはウィジェットの不透明度の当たり。要件 7.4 の「両方で同じしきい値」をどう持つかが設計の決めどころ。
- **測りの検体に気を付ける所がある**。公開元のフォルダには、別のゴースト名の `pasta-in-windows-test-alpha8.nar`（「SSP がシェル PNG の α 8 以下を描かない件の確認用」・影を点描にする前の版）が在る。最新の書庫は影を点描に変えたらしく、α が小さい画素が残っていない見込みがある。α の測りにどの書庫を使うかで、測りの手順が変わる。
- **網羅台帳に α の当たりの決まりを書く行が無い**（4 つの台帳で `128`・`ALPHA_THRESHOLD`・`alpha_mask` の語は 0 件）。また、ぱすたが使う項目の行には、兄弟の spec の名前がまだ書かれていない行がある（例: `seriko.sticky-window` の担当が空）。要件 9 の作業は「行の洗い出し」から要る。

## 2. 今あるもの（要件ごとの対応表）

印: **在る**＝そのまま使える／**不足**＝書き足す／**不明**＝調べが要る／**制約**＝守る決まり

| 要件 | 今あるもの | 印 | メモ |
|---|---|---|---|
| 1 SSP での測り | SSP の MCP の道具（`get_property`・`raise_event`・`sakurascript`・`dump_surface`・`get_log`・`reload`）がこのセッションから呼べる。SSP が SHIORI へ送った要求は `ghost\master\ssp_shiori_log.txt` に残る（開発の決まりの記憶） | 在る／不明 | α の当たりを SSP でどう測るか（画素の上にマウスを置いたときにイベントが来るか）の手順が無い（5 節の調べ 1・2） |
| 1.6 SSP の外の物を書き換えない | SSP の MCP は通った答えを `data\profile\mcp_*.dat` に保存する（開発の決まりの記憶） | 不明 | 測る者が書かなくても、MCP を使うだけで SSP が自分で書く。要件 1.6 の「書き換えない」に当たるかの読み方が要る（5 節の調べ 3） |
| 1.7 書庫の同定 | 置き場の `<名>.nar.sha256` を窓口が照合する（`crates/sample-ghost-kit/src/fetched.rs`） | 在る | 記録に書く値はこのファイルの中身 |
| 2 中身の自動テスト | `SampleRoot::acquire("pasta-in-windows")`（`crates/sample-ghost-kit/src/lib.rs:221`）が、照合を通った書庫を空の根へ入れ、登記（ゴースト 1・同梱 0）と食い違えば落ちる。`folder()` が `<根>/ghost/pasta-in-windows/` を返す | 在る | 新しく書くのは「`folder()` の直下の `pasta_tategaki/descript.txt` が名前を名乗る」と「`root()/balloon/` が 0 件」の判定だけ |
| 2.2 `balloon/` が 0 件 | 既存の `every_sample_nar_installs_exactly_the_elements_its_registry_row_declares`（`crates/sample-ghost-kit/src/lib_tests.rs:494`）が登記どおりの要素を見ている | 在る（間接） | 要件は明示の判定を求めている。新しいテストで `balloon/` の中を数える |
| 2.4 書庫が無いときの失敗 | `SampleError::FetchedNarMissing`／`FetchedNarMismatch` の文が取り寄せの段のコマンドを示す（`lib.rs:626`〜） | 在る | 新しい文は要らない。`acquire` の失敗をそのまま panic の文にすれば満たす |
| 2.6 登記 9・番人を緑に | 登記の数 9 は上のテストが固定。検体のパスの綴りの番人は `crates/log-capture-kit/tests/sample_path_guard_test.rs` | 制約 | 新しいテストは `crates/sample-ghost-kit/src/` の外（`tests/`）に置くので番人の見張りの中。窓口の読み口だけを使い、置き場の綴り（名前空間の語・`vendors/sample_ghost` に名前を継ぐ形）を書かない。`pasta_tategaki` は登記の同梱バルーンではないので、形 ⑷（同梱バルーンのパス組み）には当たらない見込み（番人を回して確かめる） |
| 2.7 赤の較正 | 固定入力の書庫を組む `NarBuilder`・`install_txt`（`crates/sample-ghost-kit/src/nar_writer.rs`・公開）と、空の根 `WorkDir`（公開）。`areka-nar` は窓口の通常の依存 | 在る | 判定の関数を 1 つにして、本物の書庫と固定入力の両方に当てれば「判定そのものが赤を出せる」ことを示せる |
| 3 実機の条件 | えみりの記録の起動の命令（環境変数・`Start-Process`・プロセス番号の控え・終了コードの書き出し）・`tools/sync-i686-helper.ps1`・`nar-sample-path`（`crates/sample-ghost-kit/src/bin/nar-sample-path.rs`）・`--ghost` に絶対パス（完了 `startup-options-core`・`crates/areka/src/cli/`）・机（`doc/impl-watch.md`） | 在る | そのまま写せる |
| 3.5 拡大率の違う 2 画面 | えみりの記録に、開発機の画面 1（200%）・画面 2（150%・主の左）の測り方が在る | 在る | 走行の前に測り直す |
| 4 最初の走行の流れ | えみりの記録の「流れ」「項目ごとの結果」「ERROR と WARN の数」の型 | 在る | 段が 7 つに増える。各段に SSP の基準を並べる欄が要る（えみりには無かった） |
| 4.1-1 起動のバルーン | `ghost-inner-balloon` が着地するまで、ゴーストのフォルダの中の縦書きのバルーンは名前で見つからない。えみりでは `descript_balloon_not_found` の後に次の候補のバルーンへ進んだ | 制約 | 最初の走行の根に、代わりに使われるバルーンを置くかの決めが要る（6 節の 7） |
| 5 崩れの一覧 | えみりの記録の「崩れた所の表」（症状・根拠・原因・扱い） | 在る | 列「引き取る spec」を必ず埋める形にする |
| 6 直す線 | えみりの記録の「直しの見積もりと 20 本の線」 | 在る | 同じ型 |
| 7 α の当たり | `crates/wintf/src/ecs/widget/bitmap_source/alpha_mask.rs:7`（`const ALPHA_THRESHOLD: u8 = 128`・`pack_pbgra32_alpha` が新規の作りと作り直しの両方で使う唯一の判定）。`crates/wintf/src/ecs/layout/hit_test/mod.rs:311`・`:413`（`const ALPHA_THRESHOLD: f32 = 128.0 / 255.0`・不透明度 × 塗りの α） | 不足（決め待ち） | 5 節の調べ 4・6 節の 4・5 |
| 8 何度でも回せる手順 | えみりは手順を spec の記録の中に置いた。完了のときに spec のフォルダは `completed/` へ動く | 不足 | 置き場の決めが要る（6 節の 8） |
| 9 網羅台帳 | `doc/ukadoc-coverage/ledger/*.toml`（4 本・1,749 項目）。直したら `cargo test -p ukadoc-survey` と `cargo run -p ukadoc-survey -- report`（`doc/ukadoc-coverage/README.md`） | 不足 | ぱすたが使う行の洗い出しの道具が無い。α の決まりの行が無い（6 節の 10） |

## 3. 書庫の実物の確かめ（読むだけ）

### 3.1 取り寄せた書庫（置き場）

- このワークツリーには `target\` が無い（ビルドしていない）。ほかの 8 つのワークツリーと本体のチェックアウトにも、`target\nar-samples\fetched\pasta-in-windows.nar` は無かった。
- そのため、要件 2 の自動テストも既存の登記のテストも、このワークツリーでは取り寄せの段（`pwsh -NoProfile -File C:\home\maz\git\areka\.claude\worktrees\areka-p0-pasta-windows-d4ed7a\tools\sample-fetch.ps1`）を回すまで赤になる（設計どおりの失敗）。実装の段で 1 回回す。

### 3.2 公開元の手元の書庫（`C:\home\maz\git\ghost_dev\release\pasta-in-windows\`・読むだけ）

取り寄せた物ではないが、公開する前の同じ名前の書庫が手元に在ったので、中の一覧と 3 つの文書だけを読んだ（展開はしていない）。

| 項目 | `pasta-in-windows.nar` | `pasta-in-windows-test-alpha8.nar` |
|---|---|---|
| 日時 | 2026-10-20 22:23 | 2026-10-19 21:07 |
| 大きさ | 3,324,982 バイト | 3,017,677 バイト |
| 項目の数 | 73（棚卸㉗の 68 から増えた＝要件 2.3 のとおり変わり続けている） | 59 |
| `.sha256` の値 | `ad473d5dd1f142c168092e71b436419270b1cd16331ce19cc24f92a0c04a43a6` | `01dcb0b26e92f81bfdafeb3de8dd9bce3530e686f87a5e9d02dd3875440b635a` |
| `install.txt` | `type,ghost`・`directory,pasta-in-windows`・`balloon.directory` 無し | `directory,pasta-in-windows-test-alpha8`（別のゴースト） |
| 縦書きのバルーン | 書庫の根に `pasta_tategaki/`（`descript.txt`・`balloons0.png`・`balloonk0.png`・`balloonc0.png`・`arrow0.png`・`arrow1.png`）。descript は `type,balloon`・`name,窓際のぱすたさん縦書き`・`vertical,1`・`use_self_alpha,1` | 同じ |
| ゴーストの descript | `balloon,窓際のぱすたさん縦書き`・`sakura.seriko.defaultsurface,1000`・`balloon.dontmove,true`・`balloon.syncscale,true`・`shiori,pasta.dll`・`homeurl` | `homeurl` が無い。ほかは同じ |

読み取れたこと:

- ゴーストの書庫は根がそのままゴーストのフォルダになるので、入れた後は `<根>\ghost\pasta-in-windows\pasta_tategaki\descript.txt` になる。要件 2.1 の「ゴーストのフォルダの直下」と合う。`balloon.directory` が無いので、`<根>\balloon\` にはぱすたのバルーンが入らない（要件 2.2 と合う）。
- 公開の URL の書庫がこれと同じ版かは確かめていない（取り寄せていない）。
- `test-alpha8` の readme は「SSP がシェル PNG の α 8 以下を描かない件の確認用」「影の画像を点描にする前のものに戻し」と書く。最新の書庫とは `shell/master/window/part_pasta_shadow.png`・`part_window.png`・`canvas.png` などの中身が違う。最新の書庫の影は点描（不透明な点と抜けの組み合わせ）に変わった見込みで、α が 1〜127 の画素が残っていないかもしれない。
- 最新の書庫だけが `shell/master/window/part_window_150.png`・`part_window_200.png` を持ち、`surfaces.txt` の 100 行目が `element0,overlay,window/part_window_150.png,0,0` を書く。辞書 `tategaki.pasta` の 102 行目に `REG.OnShellScaling` が在る。拡大率の違う画面への移動（要件 4.1-5）で、ぱすたが拡大率ごとに絵を選んでいる見込みがある（持ち主は `dpi-realign-remembered-chain`・倍率の連動は範囲の外）。

## 4. 要件ごとの不足・制約・調べが要る所

### 4.1 要件 2（自動テスト）

- **置き場**: brief の見立てどおり `crates/sample-ghost-kit/tests/` の新しいファイル。窓口の公開の読み口（`SampleRoot::acquire`・`folder`・`root`）と、較正用の `NarBuilder`・`WorkDir`・`areka_nar` だけで書ける。`src/lib.rs`・`src/lib_tests.rs` は触らない（兄弟の spec が触る）。
- **名前の読み方**: `pasta_tategaki/descript.txt` は `charset,UTF-8`。判定は行 `name,窓際のぱすたさん縦書き` の有無で足りる。areka の本物の descript の読み手を通すかは選べる（下の案）。
- **較正（2.7）**: 固定入力は 2 つ考えられる。⑴ 縦書きのバルーンを抜いたゴーストの書庫。⑵ 縦書きのバルーンをゴーストのフォルダの外（例えば `ghost/master/` の下、または別のバルーンの書庫）に置いた書庫。どちらか 1 つでも要件は満たす。
- **制約**: 番人（`sample_path_guard_test`）の 4 つの形を書かない。落ちたときの文は、窓口の失敗の文をそのまま出す。

### 4.2 要件 7（α）

- **シェルとバルーンの当たりの道**: 合成した絵の画素（PBGRA）から `AlphaMask::from_pbgra32` でマスクを作る（`crates/areka-emo-present/src/presenter/budget.rs:329`・`crates/areka-emo-text/src/surface_window_child.rs:36`）。判定は `alpha_mask.rs` の `pack_pbgra32_alpha` の 1 か所で、しきい値は整数 128。
- **`hit_test/mod.rs` の 2 つ**: 不透明度 × 塗りの α を小数 128/255 と比べる（ウィジェットの当たり）。シェルの道は通らない見込みだが、要件 7.4 は「描き方の違いで線がずれない」ことを求めるので、3 つを同じ値にする。今は同じ値を 3 回書いている（コメントで「同一基準」とだけ言っている）。
- **既存のテストが変わる**: `alpha_mask.rs` の中のテスト（例: 237 行目 `test_threshold_boundary_128`・190 行目と 208 行目の α < 128／≧ 128 のテスト）は、しきい値を変えると期待が古くなる。決めた値に合わせて書き直す（古くなったテストの扱い）。
- **描く側（7.6）**: areka の合成は α が 1 以上の画素をそのまま描く。アトラスの切り詰め（`crates/areka-emo-atlas/src/trim.rs:75`）は α > 0 で範囲を決める（当たりとは別の意味）。「α 8 以下を描かない」を写すなら、絵の読み込みの正規化（`areka-emo-atlas` の `normalize`）か合成のどちらかに手が入り、当たりより広い影響になる。
- **重なる spec**: しきい値の 3 か所は、`hit-id-pass`・`compose-f16-precision`・`wintf-d2d-composite-rules`（どれも v0.2.0）・`balloon-element-order`（版に縛らない）も触る。v0.1.0 のウェーブでは重ならない（brief の見立て）。

### 4.3 要件 1・3・4（測りと実機）

- 測りも走行も、道具は揃っている。足りないのは手順の文書と、α を測る具体のやり方だけ。
- ぱすたを SSP と areka で同時に起こさない（デバッグの待受 127.0.0.1:9276 がぶつかる）。測りの間は areka でぱすたを起こさない。
- 走行の根: えみりは配布物と同じ形（`emo2`・`emo2-kakukaku`・`StayseeBalloon`）の根を `nar-sample-path` で作り、メニューの「インストール…」で検体を入れた。ぱすたも同じ形で入れられる。代わりにゴーストを `--ghost` の絶対パスで直に起こす形もある。どちらにするかで、起動の段（要件 4.1-1）で何を比べるかが変わる。
- `ghost-inner-balloon` は別のワークツリーで進んでいる（`areka-p0-ghost-inner-balloon-dd21e8`）。最初の走行の時点で着地していなければ、起動の段のバルーンは「違う／回せなかった」になる見込み。

### 4.4 要件 8・9（手順と台帳）

- 手順を spec のフォルダに置くと、完了のときに `completed/` へ動いてパスが変わる。最後の確かめは本 spec の外で回すので、動かない置き場（`doc/` か `tools/`）の方が読み手に優しい。
- 台帳: ぱすたが使う項目の候補は、少なくとも次の 4 群。正確な一覧は書庫の descript・surfaces・辞書を読んで作る（手で並べるか、使い捨ての読み取りで拾うか）。
  - ゴースト・シェルの descript: `sakura.seriko.defaultsurface`・`balloon.dontmove`・`balloon.syncscale`・`homeurl`・`seriko.sticky-window`・`seriko.alignmenttodesktop`・`sakura.balloon.offsetx`／`offsety`・`menu.*`・`sakura.menu`・`sakura.cursor` ブレス
  - バルーンの descript: `vertical`・`use_self_alpha`・`origin.*`・`validrect.*`・`wordwrappoint.y`・`font.*`
  - 台本: `\_l`・`\v`・`\_u`・`\![bind]`・`\![bind-noevent]`・`\![*]`・`\![set,zorder]`／`\![reset,zorder]`・`\![move]`・`\c`・`\_w`
  - イベント: `OnMouseClick`・`OnMouseEnter`／`OnMouseLeave`・`OnMouseMove`・`OnMouseDoubleClick`・`OnMouseDragEnd`・`OnShellScaling`・`OnChoiceSelectEx`・`OnBoot`・`OnFirstBoot`・`OnSecondChange`・`OnClose`
- 台帳の行は今の時点で兄弟の spec に追いついていない所がある。例: `doc/ukadoc-coverage/ledger/assets.toml:5915` の `seriko.sticky-window` は担当が空（持ち主は `window-link-directives`）。
- α の当たりの決まりを述べる行が無い。台帳の形式は凍結されているので、新しい行は作れない。書くなら、既存の近い行（`collision*`・`seriko.use_self_alpha` など）の説明に足すか、`doc/COMPAT_ARCHITECTURE.md` の §8「沈黙ルール対応表」に行を足すかになる。

## 5. 調べが要る所（設計の段で）

1. **SSP での α の当たりの測り方**: SSP で「当たる」を何で見るか。案: 決めた画素の上にマウスを置き（開発者の許可を得て SendInput）、`ssp_shiori_log.txt` に `OnMouseMove` が来るかを見る。描かれるかは SSP の `dump_surface` の絵（SSP の描画を写すのか、元の絵を写すのか）で見られるか、画面の撮影が要るか。
2. **α が小さい画素のある場所**: 測りに使う書庫に α が 1〜8・9〜127・128 以上の画素がどこに在るか。最新の書庫に無ければ、`test-alpha8` を使うか、ほかの手段が要る（6 節の 1）。
3. **SSP が自分で書く記録と要件 1.6**: MCP の答えの保存（`data\profile\mcp_*.dat`）と `ssp_shiori_log.txt` は、測る者ではなく SSP が書く。「測りのために書き換えない」の外と読んでよいか。
4. **しきい値の値の候補**: 測りが「SSP は α 9 以上で当たる」なら、整数の 9 と小数の 9/255 を同じ値として扱う形（整数の値を 1 つ持ち、小数はそこから作る）が素直。SSP の実の線が 8／9 の間でない可能性もある（測ってから）。
5. **公開の書庫と手元の書庫が同じ版か**: 取り寄せの段を回した後に `.sha256` を比べれば分かる（`ad473d5d…` なら同じ）。
6. **拡大率ごとの絵の選び方**: ぱすたが `part_window_150.png` などをどう選ぶか（`OnShellScaling` と辞書）。SSP の基準を撮るときに、段 5 の見た目に効く。

## 6. 実装の案

### 案 A: 今あるものに足すだけ（最小）

- 自動テスト: `crates/sample-ghost-kit/tests/` に 1 本（判定の関数 1 つ＋本物の書庫に当てる 1 本＋固定入力の赤 1 本）。
- α: 測りが今と違い、開発者が変えると決めたときだけ、3 か所の定数を新しい値に書き換え、境のすぐ下・すぐ上のテストを足す（既存の境のテストを書き直す）。
- 手順と記録: えみりと同じく、手書きの PowerShell の命令を文書に書く。
- 良い所: 書くコードが最小。既存の形そのまま。
- 弱い所: 3 か所の定数がまた別々に動きうる（要件 7.4 を構造では守らない）。手順が文書だけで、回し直す人が命令を写す。

### 案 B: しきい値を 1 か所に集め、手順を道具にする

- α: `alpha_mask.rs` に値を 1 つ置き、`hit_test/mod.rs` の 2 か所はそこから小数を作る。値を変えても 3 か所がずれない。
- 手順: `real-run-kit` を使う試験（ふだんは回さない印つき）か `tools/` の台本で、起動 → MCP で撮る → 閉じる、までを道具にし、人の手は操作と目視だけにする。
- 良い所: 要件 7.4 を構造で守れる。回し直しが軽い。
- 弱い所: 触るファイルが増え、v0.2.0 の 3 本と重なる面が広がる。道具の分だけタスクが増える（20 の線に近づく）。目視が要る段は道具にしても人の手が残る。

### 案 C: 混ぜる（A を土台に、ずれやすい所だけ B）

- 自動テストと記録は A のまま。
- α を変えると決まったときだけ、値を 1 か所に集める（変えないと決まれば、コードは 0 行）。
- 手順は文書（`doc/` の下）に書き、命令は絶対パスの PowerShell。道具にするのは、えみりの記録で手間だった所（根の作り直し・終了コードの書き出し）だけ。
- 良い所: 決めの結果で作業が増減する所を、決めの後に回せる。
- 弱い所: 決めの前に設計を固めきれない所が残る。

## 7. 規模とリスク

- **規模: M（3〜7 日）**。コードは小さい（自動テスト 1 本・α の数行）。日数の大半は、SSP の測り・実機の 7 段の走行・崩れの仕分け・台帳の洗い出しで、開発者の目視と許可を待つ時間が入る。brief の 10〜14 タスクの見立てと合う。
- **リスク: 中**。技術は既知で、道具もそろっている。ただ、⑴ α の測り方がまだ決まっていない、⑵ 兄弟の spec の着地の度合いで最初の走行の結果が大きく変わる（多くの段が「回せなかった」になりうる）、⑶ α を変えると全部のゴーストの当たりが変わる、の 3 つが読み切れていない。

## 8. 設計の段へ申し送る決めごと（要件の話し合いの議題の候補）

1. **α の測りに使う書庫**: 最新の公開の書庫（α が小さい画素が無いかもしれない）・手元の `pasta-in-windows-test-alpha8.nar`（登記の外・ghost_dev から読むだけ・SSP へは開発者が入れる）・その両方、のどれで測るか。
2. **SSP で「当たる」を見るやり方**: マウスを置いてイベントの記録を見る形でよいか（SendInput の許可は測りのあいだだけ）。描かれるかを何で見るか。
3. **SSP が自分で書く記録の扱い**: MCP の答えの保存と SHIORI の記録は、要件 1.6 の「書き換えない」の外と読んでよいか。
4. **α を変える場合のしきい値の持ち方**: 1 つの値に集める（案 B）か、3 か所を同じ値に書き換えるだけ（案 A）か。
5. **描く側を写すか（要件 7.6）**: 写すなら絵の読み込みの正規化か合成に手が入り、v0.2.0 の 3 本と重なる。写さないなら「当たらないが薄く見える」画素が残る。
6. **自動テストの判定の深さ**: descript.txt の名前の行を文字列で見るだけか、areka の本物のバルーンの descript の読み手で名前を読むか。較正の固定入力は「抜いた書庫」「外に置いた書庫」のどちら（または両方）か。
7. **最初の走行の根の形**: 配布物と同じ形の根（`emo2`・`emo2-kakukaku`・`StayseeBalloon`）にメニューから入れるか、ぱすただけを `--ghost` の絶対パスで起こすか。`ghost-inner-balloon` が着地していないときに使われる代わりのバルーンを何にするか。
8. **手順の置き場**: spec のフォルダ（完了で `completed/` へ動く）か、`doc/` の下（例えば `doc/testing.md` の節か新しい文書）か、`tools/` の台本か。
9. **手順をどこまで道具にするか**: 手書きの命令（えみりの形）か、`real-run-kit` を使う試験か台本か。
10. **網羅台帳の「ぱすたが使う項目」の数え方と、α の決まりの書き場所**: 書庫から拾って一覧にするか手で並べるか。α の決まりは既存の行の説明に足すか、`doc/COMPAT_ARCHITECTURE.md` §8 の表に足すか。
11. **最初の走行の時期**: 兄弟の spec がほとんど着地していない今回すと、多くの段が「回せなかった」になる。それを承知で今回すか（要件の線どおり）、着地の進み具合を見て時期をずらすか。

### 8.1 要件の話し合いでの仕分け（2026-10-22）

- 設計の段（`/kiro-design`）で決めるもの: 8 節の 2（SSP で当たりを見るやり方）・4（しきい値の持ち方）・6（自動テストの判定の深さと較正の固定入力）・7（最初の走行の根の形と代わりのバルーン）・8（手順の置き場）・9（手順をどこまで道具にするか）・10（台帳の数え方と α の決まりの書き場所）。
- 要件の話し合いで済ませたもの: 3（要件 1.6 を直して決着）。1・5・11 と brief の議題 2・5・6 は開発者の議題に回した。

## 9. 次の段

- この分析を要件の話し合い（`/kiro-requirements-discussion areka-P0-pasta-in-windows-verification`）で読み、8 節の決めごとを決める。
- その後 `/kiro-design areka-P0-pasta-in-windows-verification` で設計へ進む。
