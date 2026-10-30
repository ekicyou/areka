# Brief: default-balloon-selfmade

> 2026-10-10 `/kiro-discovery`（開発者「これ〔どっとさくら〕に含まれるバルーンを参考に、デフォルトバルーンを作ってほしい。設定系パラメーターは引き継ぐ、設定ファイルを utf-8 に変える。online0.png などのアニメは動画 webp などで実現する。デザインをもっとシンプルにする。オンラインなどの通信アニメーションももっとシンプルにする。サイズ感はそのまま流用する。画像ファイルなどは現代的なフォーマットにする、など」）で起票した。roadmap「自前の既定バルーンと検体『どっとさくら』」節。コードは「何の定義か」で指す。着手時に引き直す。

## Problem

areka の既定バルーンは、第三者の `StayseeBalloon`（CC0）を借りている。areka が自分の顔として配るバルーンを持っていない。

手本にしたいバルーンは在る。開発者の自作ゴースト「．さくら」に同梱の「ボトル（でふぉ改）」で、寸法と設定値は使い込まれている。ただし作りが古い（設定は Shift_JIS・絵は α の無いパレットの PNG を赤の抜き色で抜く・オンラインの印は連番 16 枚・瓶の透かしや夕日と波の絵が入る）。

## Current State

- 既定バルーンの名前は起動の解決の定数 1 つ（`crates/areka/src/boot_resolve.rs` の `DEFAULT_BALLOON_FOLDER`）。バルーンの解決の 6 段目（引数 → 記憶 → ゴーストの descript → 同梱 → 唯一 → **既定** → 無作為）で、同じ名前のフォルダが在るときだけ当たる。
- 配布物は `tools/package.ps1` が検体の窓口から `StayseeBalloon` を引いて `balloon/StayseeBalloon/` に入れる（必須の項目・許可の表・写す行の 4 か所）。
- 検体としては `vendors/sample_ghost/StayseeBalloon.nar` と登記表 `SAMPLES` の行。中身を固定するテスト一式が `crates/areka-emo-text/tests/staysee_balloon_fixture*` に在る。
- 既定の名前を直に期待するテストは `crates/areka/src/` の `boot_resolve_tests.rs`・`boot_resolve_balloon_tests.rs`・`boot_config_balloon_tests.rs`・`main_config_input_tests.rs`。
- 文書は根の `README.md`・`dist/README.txt`・`vendors/sample_ghost/README.md`・steering（`product.md`・`tech.md`・`structure.md`）が `StayseeBalloon` を既定と書いている。
- 今のエンジンでできること・できないこと:
  - 面の絵（`balloons*`・`balloonk*`）は出る。α は descript の `use_self_alpha` で使う。
  - descript は先頭に `charset,UTF-8` と書けば UTF-8 で読める（書かないと Shift_JIS として読む）。
  - 面別の設定（`balloons0s.txt` など）は面 0 だけ実際に通っている（面 1 以降は `balloon-canon-residue`）。
  - 印（矢印・オンライン・SSTP）と数字は**まだ出ない**。鍵（`arrow0.x`・`onlinemarker.x` など）は黙って読み飛ばす（`balloon-markers` が作る）。
  - 入力の箱 `balloonc*` は読まない。
  - 絵の名前は `.png` だけ（`.webp` の名前は `balloon-webp-names` が作る）。
- 動く絵の検体を `image` と std だけで作る前例が在る（`crates/areka-emo-atlas/examples/gen_animated_samples.rs`＝動く WebP を可逆のコマから組む）。

### 手本「ボトル（でふぉ改）」から引き継ぐ値（2026-10-10 に書庫から読んだ）

絵の寸法（ピクセル）:

| 絵 | 寸法 | 役 |
|---|---|---|
| `balloons0`・`balloons1` | 326×169 | 本体側・小（しっぽが右／左） |
| `balloons2`・`balloons3` | 326×348 | 本体側・大（しっぽが右／左） |
| `balloonk0`・`balloonk1` | 326×96 | 相方側（しっぽが右／左） |
| `balloonc0`〜`balloonc3` | 400×63 | 入力の箱 |
| `arrow0`・`arrow1` | 11×10 | 上／下の矢印 |
| `online0`〜`online15`（と `online`） | 77×36 | オンラインの印（16 コマ） |
| `sstp` | 7×9 | SSTP の印 |

`descript.txt`（全体）:

```
origin.x,14            origin.y,11
validrect.left,0       validrect.top,0      validrect.right,0     validrect.bottom,-20
wordwrappoint.x,-20    wordwrappoint.y,0
font.name,ＭＳ ゴシック  font.height,12       font.color.r/g/b,0/0/0
cursor.blendmethod,mergepennot              cursor.style,square
cursor.brush.color.r/g/b,22/143/231         cursor.pen.color.r/g/b,8/8/194      cursor.font.color.r/g/b,201/255/255
arrow0.x,303  arrow0.y,10                   arrow1.x,303  arrow1.y,-20
onlinemarker.x,4       onlinemarker.y,-39
sstpmarker.x,10        sstpmarker.y,-15
sstpmessage.x,20       sstpmessage.y,-17    sstpmessage.font.name,ＭＳ Ｐゴシック  sstpmessage.font.height,12  sstpmessage.font.color.r/g/b,56/163/163
number.font.name,Times New Roman            number.font.height,14  number.font.color.r/g/b,32/76/120   number.xr,-25  number.y,-18
communicatebox.x,8     communicatebox.y,21  communicatebox.width,385  communicatebox.height,20
```

面別の設定（どれも `windowpos.x,0`・`windowpos.y,0` を持つ）:

| ファイル | origin.x | origin.y | wordwrappoint.x | ほか |
|---|---|---|---|---|
| `balloons0s.txt` | 14 | 11 | -30 | `onlinemarker.x,3`・`sstpmarker.x,10`・`sstpmessage.x,20` |
| `balloons1s.txt` | 23 | 11 | -20 | `onlinemarker.x,12`・`sstpmarker.x,19`・`sstpmessage.x,29` |
| `balloons2s.txt` | 14 | 14 | -30 | |
| `balloons3s.txt` | 23 | 14 | -20 | |
| `balloonk0s.txt` | 14 | 14 | -30 | `validrect.bottom,-14` |
| `balloonk1s.txt` | 24 | 14 | -20 | `validrect.bottom,-14` |

## Desired Outcome

- areka の配布物に、areka 自前の既定バルーンが入っている。バルーンを指定しないゴーストは、そのバルーンで喋る。
- 寸法と設定値は手本と同じで、手本のバルーンに合わせて作られたゴースト（どっとさくら）に被せても文字の位置が同じになる。
- 設定ファイルは UTF-8（`charset,UTF-8` を名乗る）。
- 絵は α 付きで、抜き色を使わない。見た目は手本より簡素（瓶の透かしなどの飾りを持たない）。
- オンラインの印は連番でなく 1 枚の動く WebP で、絵柄も手本（夕日と波）より簡素。
- 絵の作り方がリポジトリに残っていて、同じ手順で作り直せる。権利の扱い（自作・再配布の条件）が書かれている。
- `StayseeBalloon` を既定と書いていた所（定数・配布の道具・文書・テスト）が、新しいバルーンを指している。

## Approach

絵を作る仕事と、既定を差し替える仕事を、1 本の spec で続けて行う。**絵柄は要件の段で案を出して開発者が選ぶ**（`areka-P0-app-icon` と同じ段取り）。

絵の作り方は、**コードで描く**案を推す。

- 案 A（推し）: 絵を描く小さい道具（Rust の `examples`）で、角丸の箱としっぽを α 付きで描いて書き出す。動く WebP も同じ道具で組む（`gen_animated_samples.rs` の前例＝新しい依存なし）。寸法が 1 ピクセル単位で手本と合い、何度でも同じバイト列で作り直せ、権利が自作だけで閉じる。簡素な絵柄に向く。色や角の丸みは道具の定数を変えて案を並べる。
- 案 B: fal（画像生成）で案を出す。寸法と縁を 1 ピクセル単位で合わせる後加工が要り、左右の対（しっぽが右／左）や大小の面で絵柄をそろえにくい。モデルの利用条件の確認も要る。
- 案 C: 輪郭のデータ（SVG）を手で書いてリポジトリに置き、絵に変換する。人が直しやすいが、変換の道具か依存が要る。

進め方:

1. **絵柄を決める（要件の段）**: 案 A の道具の下書きで案を 2〜3 個並べ、開発者が選ぶ（箱の色・縁・角の丸み・しっぽの形・オンラインの印の絵柄と動き）。
2. **バルーンを組む（実装）**: 面（本体側の小と大・相方側・入力の箱）と印（矢印・オンライン・SSTP）の絵、UTF-8 の `descript.txt` と面別の設定、`install.txt`・`readme.txt` を置き、検体の決まりどおり `.nar` に畳んで登記する。
3. **既定を差し替える（実装）**: 起動の解決の定数・配布の道具・文書・既定の名前を期待するテストを新しい名前へ。`StayseeBalloon` は検体として残す。
4. **確かめる（実装）**: 中身を固定するテスト（`staysee_balloon_fixture` と同じ形）と、実機で emo2 とどっとさくらに被せた見え方。

印の絵は作って入れておくが、出るようになるのは `balloon-markers` の着地の後（今は読み飛ばされるだけで害は無い）。

## 議題（要件の段で決める）

1. **名前と ID**（フォルダ名・`name`・`id`・`craftman`）。開発者が決める。
2. **絵柄**（上の 1）。瓶の透かしを無くすか、areka の印を小さく入れるか。オンラインの印は何を何コマ・何ミリ秒で動かすか（手本は 77×36・16 コマ）。
3. **絵の形式**。`.webp` の名前で持つ（`balloon-webp-names` の後に着手する）か、α 付きの PNG で持つ（今すぐ作れる）か。開発者の指示は「現代的なフォーマット」。推しは、面も印も `.webp`（オンラインの印は動く WebP 1 枚）。SSP では出ないバルーンになることを承知のうえで選ぶ。
4. **引き継ぐ値の線引き**。そのまま引き継ぐ: 寸法・`origin`・`validrect`・`wordwrappoint`・印と数字と入力の箱の位置。聞いてから決める: `font.name,ＭＳ ゴシック`／`font.height,12`（今の画面では小さく、字形も古い）・カーソルの色・SSTP と数字の字の色。`windowpos.x/y`（値は 0）は ukadoc の鍵の名前（`windowposition`）を引き直してから扱いを決める。
5. **オンラインの印を 1 枚の動く絵で持つときの書き方**。正典は `onlinemarker.filename` の名前に 0 からの連番を足して読み、`onlinemarker.interval`（既定 500 ミリ秒・50 未満は不可）で切り替える。1 枚の動く絵で持つのは areka の拡張になる。読み方は `balloon-markers` の議題に足した（こちらは絵を用意する側）。
6. **面をいくつ持つか**。手本どおり本体側 4・相方側 2・入力の箱 4 を持つ案を推す（寸法を流用するため）。面別の設定が実際に通っているのは面 0 だけなので、台本の `\b[ID]` で面 2 などへ切り替えたときの見え方は、着手時に確かめる。
7. **`StayseeBalloon` の扱い**。検体には残す。配布物から外すか、選べるバルーンとして残すか。記憶に `StayseeBalloon` が残っている利用者の次の起動がどうなるか（フォルダが無ければ既定へ落ちる）を確かめて決める。
8. **権利の書き方**。自作の絵と設定の再配布の条件（`StayseeBalloon` と同じ CC0 にするか）と、置き場（バルーンの `readme.txt`・`vendors/sample_ghost/README.md`）。手本から引き継ぐのは数値だけで、絵は写さない。
9. **拡大率の高い画面**。バルーンは画面の拡大率に合わせて拡大される。コードで描くなら 2 倍の絵も作れるが、今のエンジンに 2 倍の絵を選ぶ口は無い。この spec では等倍だけを作る案を推す。

## Scope

- **In**: 絵を描く道具と作り直しの手順／面と印の絵／UTF-8 の `descript.txt` と面別の設定／`install.txt`・`readme.txt`／`.nar` に畳んで検体に登記／既定の差し替え（定数・`tools/package.ps1`・文書・既定の名前を期待するテスト）／中身を固定するテスト／実機の確かめ（emo2・どっとさくら）／権利の記録。
- **Out**: 印と数字を出すこと・1 枚の動く絵の印の読み方（`balloon-markers`）／`.webp` の名前を拾うこと（`balloon-webp-names`）／入力の箱を出すこと（`inputbox-user-input`・`balloon-canon-residue`）／面 1 以降の面別の設定の実被覆（`balloon-canon-residue`）／縦書きの面／シェル内バルーンの絵／2 倍の絵／手本「ボトル（でふぉ改）」の絵の手直しや再配布。

## Boundary Candidates

- バルーンの資産と作る道具（置き場は要件・設計で決める。検体の保管形は `vendors/sample_ghost/<名>.nar`）。既定バルーンは配布物に必ず入る製品の資産なので、検体を URL から取り寄せる形（`areka-P0-sample-url-fetch`）へ替わった後もリポジトリが持つ。置き場を検体の置き場と分けるかは、ここで決める。
- 既定の差し替え（`crates/areka/src/boot_resolve.rs` の定数・`tools/package.ps1`・`crates/sample-ghost-kit` の登記表と数のテスト）。
- 文書と権利の記録（根の `README.md`・`dist/README.txt`・`vendors/sample_ghost/README.md`・steering）。
- 確かめ（中身を固定するテスト・実機）。

## Out of Boundary

- バルーンの解決の段の順。変えない（定数の値だけ）。
- バルーンの読み手・合成器・文字の層。直さない。足りない所は起票する。
- `StayseeBalloon.nar` の中身と、その中身を固定するテスト。残す。

## Upstream / Downstream

- **Upstream**: `areka-P0-dot-sakura-specimen`（手本を検体として引ける・登記表と数のテストを続けて触る）。`balloon-webp-names`（議題 3 で `.webp` を選ぶとき）。要件の段に、開発者と絵柄を選ぶ往復が入る。
- **Downstream**: `balloon-markers`（このバルーンの矢印・オンライン・SSTP の印が、同 spec が求める「リポジトリ内の印の検体」になる）。スキル `/release`（次の版の配布物から既定が替わる）。

## Existing Spec Touchpoints

- **Extends**: 完了 `areka-P0-default-balloon-bundle`・`areka-P0-default-balloon-nar-fold`（既定を `StayseeBalloon` に決めて保管した）の決めごとを、既定の役だけ新しいバルーンへ移す形で改める。
- **Adjacent**: カタログの鎖 `areka-P0-baseware-root-list` → `areka-P0-ghost-inner-balloon` → `shell-companion-balloon`（`boot_resolve.rs` を作り直す＝定数の差し替えを同時に走らせない）。配布と公開の列（`tools/package.ps1`・`dist/README.txt`・`README.md`＝`areka-P0-user-data-root` ほか）。`areka-P0-emily-ghost-verification`（同じ登記表）。`areka-P0-app-icon`（絵を要件の段で選ぶ段取りの前例）。

## Constraints

- 絵は開発者が選ぶ。エージェントだけで絵柄を確定しない。要件を作るサブエージェントには案を出させない（開発者と話せないので、案は討議の段で主のセッションが出す）。
- 新しい依存を足さない（案 A）。
- 手本の絵を写さない（引き継ぐのは数値だけ）。
- α 付きの絵を正しく使う原則に沿う（抜き色を使わない・`.pna` を置かない）。動く絵は WebP か APNG（動く GIF は使わない）。
- 印の絵が今は出ないことを、説明書に嘘なく書く（出るようになるのは `balloon-markers` の後）。
- 一時のファイル（案の絵・途中の絵）はワークツリーの `target\` の下。リポジトリに入れるのは、選んだ絵と道具だけ。
- 規模の見立ては M（8〜12 タスク）。


## どっとさくらで見えた事実（2026-10-23 `areka-P0-dot-sakura-specimen` の実機の 1 回）

- どっとさくらの同梱バルーン `bottle` の `descript.txt` は `validrect.left,0`・`validrect.right,0`・`validrect.bottom,-20`。areka は負の値を反対の辺からの距離と読み、0 は左端の 0 と読む（`crates/areka-emo-text/src/region.rs`）。`bottom,-20` は 149 に解けたが、`right,0` は 0 のままで、文字の描画範囲が幅 0 に潰れた（`left=0.0 top=0.0 right=0.0 bottom=149.0 image_size=(326.0, 169.0)`）。続けて「origin が validrect の外」「折返し基準が描画範囲の外」の WARN も 1 人目と 2 人目で 1 回ずつ出た。
- この spec は `bottle` の `validrect` をそのまま引き継ぐ＝同じ読みのままだと、作るバルーンでも文字が描かれない。正典で `validrect.right,0` が右端を指すか（ukadoc の `validrect.right` の既定は「不明」）は確かめていない。要件の段で SSP の MCP で測ってから決める（記憶「ukadoc で読めない点は議題の前に SSP の MCP で測る」）。正典の読みが areka と同じなら、引き継ぐ値の側を直す。
- 根拠: `.kiro/specs/completed/areka-P0-dot-sakura-specimen/verification.md` の 7 節と 8 節の件 3。
- 開発者（2026-10-23）: 「次のリリースまでに治るように」。この spec はもとから v0.1.0。

## 2026-10-24 「．さくら」が話すようになった後の `bottle`（`shiori2-protocol` のタスク 5.4）

- `shiori2-protocol` で「．さくら」が話すようになった（実機の 1 回 5.1・2026-10-24・記録 `target\s2p-5-1\logs\run2.stdout.log`）。台詞は `bottle` に来たが、文字の描画範囲は前回と同じく幅 0 に潰れた（`文字の描画範囲が潰れている … right=0.0 bottom=149.0 image_size=(326.0, 169.0)` と `bottom=82.0 image_size=(326.0, 96.0)` が 1 件ずつ・origin と折返し基準の WARN も 2 件ずつ）。areka はこのとき文字を描かない作りなので、上の「どっとさくらで見えた事実」の件は、台詞が来る今は「話しているのに文字が出ない」という見える崩れになる。画面は撮っていない。

## 2026-10-24 棚卸㉚の再測定（main 9318cdb0）
- **前提の変化**: 同じファイルを挙げていた `readonly-mode`（10-24）が着地した。向こうが `crates/areka/src/boot_resolve.rs`（714→753 行）で触ったのは書く側の関数（`write_session_mark`・`write_switch_drop`・`clear_session_mark`・最後に使ったものの記録）に readonly の断りを足しただけで、既定のバルーンの名前の定数（34 行目 `DEFAULT_BALLOON_FOLDER`＝`StayseeBalloon`）と、それを使う段 6（325〜326 行目）は変わらない。新しい readonly のテスト（`boot_resolve_readonly_tests.rs` ほか）は既定の名前を綴らない＝直すテストは増えない（既定の名前を期待するテスト 4 本の綴りの数は 15・6・3・4 のまま）。`startup-install-option` とは同じファイルが無い（向こうは `cli/`・`install/`・`boot_config.rs` の `exe_location` を呼ぶ所・説明書の別の行）＝同じウェーブに置ける。
- **触るファイル**: 定数 1 行・既定の名前を期待するテスト 4 本（`boot_resolve_tests.rs` 853・`boot_resolve_balloon_tests.rs` 613・`boot_config_balloon_tests.rs` 370・`main_config_input_tests.rs` 459）・`tools/package.ps1`（752・既定のバルーンを引く 467・496・504・570 行目＝変わらず）・登記表 `crates/sample-ghost-kit/src/{lib.rs,lib_tests.rs}`（705・783）・`vendors/sample_ghost/` の新しい書庫と README・`dist/README.txt`（460）・根の `README.md`（138）・steering・絵を描く道具と中身を固定するテスト（新）。1,000 行に近いファイルは無い。
- **規模**: 8〜12 のまま（`validrect.right,0` の読みを areka の側で直すことになったら +1〜2）。
- **先に要るもの**: 働きの上では無い＝先頭（議題 3 で `.webp` を選んだときだけ、v0.1.0 の外の `balloon-webp-names` が前提になり v0.1.0 へ戻る）。同じウェーブに置けない相手: `winget-release-automation`（根の `README.md`）・余力の枠の `multi-ghost-runtime`（`boot_resolve.rs`）。`validrect` の読み（`crates/areka-emo-text/src/region.rs` 495 行・変わらず）を直す側に倒すと、同じファイルを挙げる spec と重なる＝要件で決まったら測り直す。登記表は 10 本で、この spec で 10 → 11（書庫を置く行のまま足す）。1 本だけの席は取らない。進め方は前のとおり＝既定を差し替えるタスクを実装の最後に置き、その直前に main を取り込む。
- **中の待ち**: 要件の段で開発者が選ぶ往復＝議題 1（名前と ID）・2（絵柄）・3（形式＝`.webp` か α 付きの PNG）。`validrect.right,0` の正典の読みを SSP の MCP で測る（打つ前に許し）。静かな机・全体テストの比べは要らない。
- **優先度の区分**: A のまま（開発者「これに含まれるバルーンを参考に、デフォルトバルーンを作ってほしい」）。
- **要件定義のモデル**: Opus のまま（議題は開発者に聞く選びと SSP で測る読み）。
- **分割の案**: なし。
- **見つけた穴・古くなった記述**: `bottle` の `validrect.right,0` で文字の描画範囲が幅 0 に潰れる件（上の 2 節）は、`shiori2-protocol` で「．さくら」が話すようになって見える崩れになった＝要件の段の最初に測って決める（`balloon-canon-residue` の 14 項の折り返し基準の件と照らす）。
