# areka.exe のアイコン

`areka.exe` に入れるアプリのアイコンと通知領域用のアイコンの、絵と作り直しの手順と権利の記録です。
組み立て（`crates/areka/build.rs`）はこのフォルダの `icons.res` をリンカに渡すだけで、絵は組みません。

## 中身の一覧

| ファイル | 役目 |
| --- | --- |
| `app.svg` | アプリ用の焼き方。2 人（女の子とうにゅう）の輪郭の線・置く位置・二重の縁取り・塗りの色を書いた SVG（外の画像を読まない）。焼き直しのときだけ使う |
| `tray.svg` | 通知領域用の焼き方（うにゅうだけ）。焼き直しのときだけ使う |
| `app.png` | アプリ用の原画（1024×1024・透明の地）。`app.svg` を焼いたもの。`-Seed` の入力 |
| `tray.png` | 通知領域用の原画（1024×1024・透明の地）。`tray.svg` を焼いたもの。`-Seed` の入力 |
| `app-16.png`〜`app-256.png` | アプリ用の寸法ごとの絵 8 枚（16・20・24・32・40・48・64・256）。16〜24 はうにゅうだけのドット絵、32〜48 は 2 人のドット絵、64・256 は原画から縮めた 2 人。作り直しの道具の入力 |
| `tray-16.png`〜`tray-256.png` | 通知領域用の寸法ごとの絵 8 枚（同じ寸法）。16〜48 はうにゅうのドット絵、64・256 は原画から縮めたもの。作り直しの道具の入力 |
| `app.ico` | 道具の出力。アプリのアイコンのファイル（8 寸法・PNG） |
| `tray.ico` | 道具の出力。通知領域用のアイコンのファイル（8 寸法・PNG） |
| `icons.res` | 道具の出力。`areka.exe` に渡す資源のファイル（アイコンの組 2 つ・絵 16 枚） |
| `README.md` | この文書 |

- 寸法ごとの PNG 16 枚が正本です。手で整えた小さい寸法も、`-Seed` で縮めて作った寸法も、コミットした PNG がそのまま `areka.exe` に入ります。
- 出力の 3 つ（`app.ico`・`tray.ico`・`icons.res`）は手で直さないでください。PNG を直して道具を回し直します。
- PNG を直したのに道具を回し忘れると、結合テスト `crates/areka/tests/embedded_icon_test.rs` が寸法を名指して失敗します。

## 資源の番号の表

`areka.exe` の中のアイコンの番号です。**変えないこと**（通知領域のアイコンを出す側が組の 2 番を番号で引きます）。

| 資源の種別 | 番号 | 中身 | 言語 |
| --- | --- | --- | --- |
| `RT_GROUP_ICON`（14） | **1** | アプリのアイコン（exe の絵＝エクスプローラー・タスクマネージャー・ショートカット） | 0 |
| `RT_GROUP_ICON`（14） | **2** | 通知領域用のアイコン（`areka-P0-tasktray-icon` が引く） | 0 |
| `RT_ICON`（3） | 1〜8 | アプリの絵（16・20・24・32・40・48・64・256 の順） | 0 |
| `RT_ICON`（3） | 9〜16 | 通知領域用の絵（同じ順） | 0 |
| `RT_MANIFEST`（24） | 1 | 今のマニフェスト（`link.exe` の `/MANIFEST:EMBED` が入れる。`icons.res` には入れない） | リンカの既定 |

- 名前の付いた資源は使いません。エクスプローラーは番号の小さい組を exe の絵にするので、組の 1 番がアプリの絵になります。
- 言語 0 は「言語に依らない」の意味で、表示言語によらず同じ絵が引かれます。
- x64 と arm64 の `areka.exe` には同じ `icons.res` を渡すので、番号も絵も同じです。

## 作り直し方

下のコマンドの `<リポジトリの絶対パス>` は、このリポジトリを置いた場所（例 `C:\home\maz\git\areka`）に置き換えてください。どのフォルダから打っても同じに動きます（道具は自分の在るリポジトリの `crates\areka\icon\` だけを読み書きします）。

### 寸法ごとの PNG を直したとき

1. 直す寸法の `<名>-<寸法>.png`（`<名>` は `app` か `tray`）を直します。1 枚ずつ、その寸法の幅と高さ・8 ビットの RGBA（透明あり）の PNG にします。
2. 道具を回します。

   ```powershell
   pwsh -NoProfile -File <リポジトリの絶対パス>\tools\app-icon.ps1
   ```

   入力を全部確かめてから `app.ico`・`tray.ico`・`icons.res` を作り直します。足りない・寸法が違う・形式が違う PNG があれば、ファイルごとに `FAIL input ...` の行を出して終了コード 1 で止まり、今の 3 つは書き換えません。同じ入力からは同じバイトのファイルができます。
3. 直した PNG と出力の 3 つを一緒にコミットします。

### 原画（`app.png`・`tray.png`）を変えたとき

1. 原画から作り直したい寸法の `<名>-<寸法>.png` を消します（`-Seed` は在るファイルを上書きしません）。
2. 縮めて種をまきます。

   ```powershell
   pwsh -NoProfile -File <リポジトリの絶対パス>\tools\app-icon.ps1 -Seed
   ```

   無い寸法の PNG だけを原画から縮めて書きます（アイコンのファイルは組みません）。
3. 小さい寸法（16〜48）は縮めただけだと潰れやすいので、画素を手で整えます。
4. 上の「寸法ごとの PNG を直したとき」の 2・3 を行います（PNG と出力の 3 つを一緒にコミット）。

### 道具そのものを直したとき

```powershell
pwsh -NoProfile -File <リポジトリの絶対パス>\tools\app-icon.ps1 -SelfTest
```

このフォルダを写したもの（ワークツリーの `target\app-icon-selftest\` の下）で、同じバイトになること・入力の欠けや寸法違いや形式違いで止まること・`-Seed` が在るファイルを上書きしないことを確かめ、判定ごとに `ok selftest ...` か `FAIL selftest ...` を 1 行出します。このフォルダは書きません。

## 焼き直し方（絵そのものを変えるときだけ）

原画 `app.png`・`tray.png` は `app.svg`・`tray.svg` を Microsoft Edge で焼いたものです。輪郭の線を変えるときは、SVG の中の `path` を直してから、Edge を窓なしで動かして 1024×1024・透明の地で撮ります。

```powershell
msedge --headless=new --default-background-color=00000000 --window-size=1024,1024 --screenshot=<書き出す PNG の絶対パス> <SVG の file URL>
```

焼いた原画を置いたら、上の「原画（`app.png`・`tray.png`）を変えたとき」の手順に進みます。

なぞりの元と生成の記録は、仕様のフォルダの `art/` に在ります（このフォルダには写していません。`app.svg`・`tray.svg` が同じ線を中に持つので、焼き直しには要りません）。

- 置き場: `.kiro/specs/completed/areka-P0-app-icon/art/`
- `girl.svg`: 女の子の輪郭の線。`girl-g0.png` の頭を 1.3 倍にして 8 度かしげ、太ももで切った影をなぞったもの
- `unyuu.svg`: うにゅうの輪郭の線
- `girl-g0.png`: 女の子の素の絵（生成した影絵を 1024 ピクセルで焼いたもの）
- 生成と選んだ経緯の記録: 同じ仕様のフォルダの `requirements.md` の「選んだ原画」の節

## 出どころと権利

### 女の子の素の絵の出どころ

| 項目 | 内容 |
| --- | --- |
| 使ったモデル | fal の Recraft V3（`fal-ai/recraft/v3/text-to-image`） |
| 画風と指定 | 画風 `vector_illustration/cutout`、色の指定は黒 1 色、寸法 `square_hd` |
| 生成した日 | 2026-10-16 |
| request の番号 | `01a14290-c13b-7890-a1dc-b6f717158ec1` |
| 選んだ案 | 2 回目の生成（女の子の影絵 8 枚）の g0。その頭を手元で 1.3 倍にして 8 度かしげた案 `h13t` を開発者が選んだ（2026-10-16） |

指示の文:

> Pictogram: one single solid black silhouette of a girl standing, full body, facing slightly left, short bob hair with a large ribbon bow on top, knee-length dress, hands clasped in front. Flat filled shape only: no interior lines, no face, no details, no outline. Plain pure white background, nothing else in the image.

モデルの利用条件:

- モデルのページ <https://fal.ai/models/fal-ai/recraft/v3/text-to-image> に「Commercial use」の印と「Commercial use licensed for production applications」の記載がある。
- fal の FAQ <https://fal.ai/docs/documentation/model-apis/faq> は、許諾はモデルごとで、`Commercial use` の印のモデルは商用に使えるとしている。
- どちらにも、作った絵に出どころの表示を求める記載は無い。そのため配布の zip の `README.txt`（`dist/README.txt`）には書き足していない。
- 確かめた日: 2026-10-16（要件の段で確かめ、この文書を書くときにもう一度確かめた）。

### 権利

権利は 2 つに分けて扱います。依存のライセンスから作る `THIRD-PARTY-NOTICES.md` には載せません。

**areka 本体（MIT）の部分**

- 女の子の絵（上のモデルで生成した素の絵をなぞって整えたもの）・2 人の組み合わせ・色・縁取りは、モデルの利用条件（商用の利用可）の下で、areka 本体の一部として MIT（リポジトリの根の `LICENSE-MIT`）で配ります。

**うにゅうの輪郭（MIT の許諾が及ばない）**

- うにゅうの輪郭は、元祖の伺かの相方の形に由来します。開発者の判断（2026-10-16）で areka のアイコンに使っています。
- areka はその元の形の権利を持ちません。MIT の許諾は、元の形の権利には及びません。
- このリポジトリが持つのは輪郭の形だけです。元の絵（色・顔）は入れないでください。

## 実機の確かめ方

### 見え方（エクスプローラー・タスクマネージャー・ショートカット）

それぞれを、Windows の明るい表示設定と暗い表示設定の両方、画面の拡大率 100% と 200% 以上の両方で見ます。

- エクスプローラー: Windows はアイコンを覚えておくので、古い絵が出ないよう、組んだ `areka.exe` をワークツリーの `target\` の下に、まだ使ったことのない名前で写してから見ます（覚えを消す道もありますが、写す方が他の表示を乱しません）。
- タスクマネージャー: areka を動かして、プロセスの行の絵を見ます。
- ショートカット: exe へのショートカットを作って、その絵を見ます。

### 16 ピクセルの輪郭

エクスプローラーの「小さいアイコン」か「詳細」の表示（16 ピクセルで出ます）で、写した `areka.exe`（アプリの絵）と、このフォルダの `tray.ico`（`.ico` のファイルは自分の絵で出るので、通知領域用の絵が見えます）を、明るい表示設定と暗い表示設定の両方で見ます。輪郭が見分けにくければ、その寸法の `<名>-<寸法>.png` を手で整えて、道具を回し直します。

通知領域に実際に出して見るのは、通知領域のアイコンを出す仕様（`areka-P0-tasktray-icon`）の仕事です。

### arm64 の exe

arm64 の `areka.exe` は動かさず、ファイルとして開いて資源を読みます（x64 の機械で確かめられます）。

1. arm64 の `areka.exe` を組みます。どちらか一方で組みます。

   ```powershell
   pwsh -NoProfile -File <リポジトリの絶対パス>\tools\package.ps1 -Arch arm64
   ```

   組んだ exe は `<リポジトリの絶対パス>\target\package\aarch64-pc-windows-msvc\release\areka.exe` にできます。

   ```powershell
   cargo build --release -p areka --target aarch64-pc-windows-msvc --manifest-path <リポジトリの絶対パス>\Cargo.toml
   ```

   組んだ exe は `<リポジトリの絶対パス>\target\aarch64-pc-windows-msvc\release\areka.exe` にできます。
2. PowerShell で、組んだ exe の絶対パスを環境変数に入れて、結合テストを回します。

   ```powershell
   $env:AREKA_ICON_CHECK_EXE = '<リポジトリの絶対パス>\target\package\aarch64-pc-windows-msvc\release\areka.exe'
   cargo test -p areka --test embedded_icon_test --manifest-path <リポジトリの絶対パス>\Cargo.toml
   ```

   x64 の `areka.exe` と同じ検査（組 1・組 2 の 8 寸法・絵の中身・マニフェスト）が arm64 の exe にも通れば合格です。環境変数を立てないときは、arm64 の検査を飛ばしたことを 1 行出します（テストが通ったときの出力は隠れるので、見るときは末尾に `-- --nocapture` を付けます）。
