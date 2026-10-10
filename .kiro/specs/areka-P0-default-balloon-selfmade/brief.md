# Brief: areka-P0-default-balloon-selfmade

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
  - 面別の設定（`balloons0s.txt` など）は面 0 だけ実際に通っている（面 1 以降は `areka-P0-balloon-canon-residue`）。
  - 印（矢印・オンライン・SSTP）と数字は**まだ出ない**。鍵（`arrow0.x`・`onlinemarker.x` など）は黙って読み飛ばす（`areka-P0-balloon-markers` が作る）。
  - 入力の箱 `balloonc*` は読まない。
  - 絵の名前は `.png` だけ（`.webp` の名前は `areka-P0-balloon-webp-names` が作る）。
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

印の絵は作って入れておくが、出るようになるのは `areka-P0-balloon-markers` の着地の後（今は読み飛ばされるだけで害は無い）。

## 議題（要件の段で決める）

1. **名前と ID**（フォルダ名・`name`・`id`・`craftman`）。開発者が決める。
2. **絵柄**（上の 1）。瓶の透かしを無くすか、areka の印を小さく入れるか。オンラインの印は何を何コマ・何ミリ秒で動かすか（手本は 77×36・16 コマ）。
3. **絵の形式**。`.webp` の名前で持つ（`areka-P0-balloon-webp-names` の後に着手する）か、α 付きの PNG で持つ（今すぐ作れる）か。開発者の指示は「現代的なフォーマット」。推しは、面も印も `.webp`（オンラインの印は動く WebP 1 枚）。SSP では出ないバルーンになることを承知のうえで選ぶ。
4. **引き継ぐ値の線引き**。そのまま引き継ぐ: 寸法・`origin`・`validrect`・`wordwrappoint`・印と数字と入力の箱の位置。聞いてから決める: `font.name,ＭＳ ゴシック`／`font.height,12`（今の画面では小さく、字形も古い）・カーソルの色・SSTP と数字の字の色。`windowpos.x/y`（値は 0）は ukadoc の鍵の名前（`windowposition`）を引き直してから扱いを決める。
5. **オンラインの印を 1 枚の動く絵で持つときの書き方**。正典は `onlinemarker.filename` の名前に 0 からの連番を足して読み、`onlinemarker.interval`（既定 500 ミリ秒・50 未満は不可）で切り替える。1 枚の動く絵で持つのは areka の拡張になる。読み方は `areka-P0-balloon-markers` の議題に足した（こちらは絵を用意する側）。
6. **面をいくつ持つか**。手本どおり本体側 4・相方側 2・入力の箱 4 を持つ案を推す（寸法を流用するため）。面別の設定が実際に通っているのは面 0 だけなので、台本の `\b[ID]` で面 2 などへ切り替えたときの見え方は、着手時に確かめる。
7. **`StayseeBalloon` の扱い**。検体には残す。配布物から外すか、選べるバルーンとして残すか。記憶に `StayseeBalloon` が残っている利用者の次の起動がどうなるか（フォルダが無ければ既定へ落ちる）を確かめて決める。
8. **権利の書き方**。自作の絵と設定の再配布の条件（`StayseeBalloon` と同じ CC0 にするか）と、置き場（バルーンの `readme.txt`・`vendors/sample_ghost/README.md`）。手本から引き継ぐのは数値だけで、絵は写さない。
9. **拡大率の高い画面**。バルーンは画面の拡大率に合わせて拡大される。コードで描くなら 2 倍の絵も作れるが、今のエンジンに 2 倍の絵を選ぶ口は無い。この spec では等倍だけを作る案を推す。

## Scope

- **In**: 絵を描く道具と作り直しの手順／面と印の絵／UTF-8 の `descript.txt` と面別の設定／`install.txt`・`readme.txt`／`.nar` に畳んで検体に登記／既定の差し替え（定数・`tools/package.ps1`・文書・既定の名前を期待するテスト）／中身を固定するテスト／実機の確かめ（emo2・どっとさくら）／権利の記録。
- **Out**: 印と数字を出すこと・1 枚の動く絵の印の読み方（`areka-P0-balloon-markers`）／`.webp` の名前を拾うこと（`areka-P0-balloon-webp-names`）／入力の箱を出すこと（`areka-P0-inputbox-user-input`・`areka-P0-balloon-canon-residue`）／面 1 以降の面別の設定の実被覆（`areka-P0-balloon-canon-residue`）／縦書きの面／シェル内バルーンの絵／2 倍の絵／手本「ボトル（でふぉ改）」の絵の手直しや再配布。

## Boundary Candidates

- バルーンの資産と作る道具（置き場は要件・設計で決める。検体の保管形は `vendors/sample_ghost/<名>.nar`）。
- 既定の差し替え（`crates/areka/src/boot_resolve.rs` の定数・`tools/package.ps1`・`crates/sample-ghost-kit` の登記表と数のテスト）。
- 文書と権利の記録（根の `README.md`・`dist/README.txt`・`vendors/sample_ghost/README.md`・steering）。
- 確かめ（中身を固定するテスト・実機）。

## Out of Boundary

- バルーンの解決の段の順。変えない（定数の値だけ）。
- バルーンの読み手・合成器・文字の層。直さない。足りない所は起票する。
- `StayseeBalloon.nar` の中身と、その中身を固定するテスト。残す。

## Upstream / Downstream

- **Upstream**: `areka-P0-dot-sakura-specimen`（手本を検体として引ける・登記表と数のテストを続けて触る）。`areka-P0-balloon-webp-names`（議題 3 で `.webp` を選ぶとき）。要件の段に、開発者と絵柄を選ぶ往復が入る。
- **Downstream**: `areka-P0-balloon-markers`（このバルーンの矢印・オンライン・SSTP の印が、同 spec が求める「リポジトリ内の印の検体」になる）。`areka-P0-release-cycle`（次の版の配布物から既定が替わる）。

## Existing Spec Touchpoints

- **Extends**: 完了 `areka-P0-default-balloon-bundle`・`areka-P0-default-balloon-nar-fold`（既定を `StayseeBalloon` に決めて保管した）の決めごとを、既定の役だけ新しいバルーンへ移す形で改める。
- **Adjacent**: カタログの鎖 `areka-P0-baseware-root-list` → `areka-P0-ghost-inner-balloon` → `areka-P0-shell-companion-balloon`（`boot_resolve.rs` を作り直す＝定数の差し替えを同時に走らせない）。配布と公開の列（`tools/package.ps1`・`dist/README.txt`・`README.md`＝`areka-P0-user-data-root` ほか）。`areka-P0-emily-ghost-verification`（同じ登記表）。`areka-P0-app-icon`（絵を要件の段で選ぶ段取りの前例）。

## Constraints

- 絵は開発者が選ぶ。エージェントだけで絵柄を確定しない。要件を作るサブエージェントには案を出させない（開発者と話せないので、案は討議の段で主のセッションが出す）。
- 新しい依存を足さない（案 A）。
- 手本の絵を写さない（引き継ぐのは数値だけ）。
- α 付きの絵を正しく使う原則に沿う（抜き色を使わない・`.pna` を置かない）。動く絵は WebP か APNG（動く GIF は使わない）。
- 印の絵が今は出ないことを、説明書に嘘なく書く（出るようになるのは `balloon-markers` の後）。
- 一時のファイル（案の絵・途中の絵）はワークツリーの `target\` の下。リポジトリに入れるのは、選んだ絵と道具だけ。
- 規模の見立ては M（8〜12 タスク）。
