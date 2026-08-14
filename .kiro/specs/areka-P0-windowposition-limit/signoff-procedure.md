# 実機サインオフ手順書（areka-P0-windowposition-limit / tasks.md 4.4）

対応要件: **7.5**（実 emo2・実 pasta・実 DPI 2 水準でバルーンが作業領域内へ収まることの実機確認）
対応設計: **Testing Strategy / E2E・実機サインオフ（7.5）**

> **このタスクの完了状態** = 「§5 目視チェックリスト」と「§4 ログ突合（合否判定）」が**拡大率 2 水準ぶん**
> §7 の記録欄に記入された状態。片方だけ・1 水準だけでは未完了。

## この手順書の分担

| 区分 | 内容 | 担当 | 状況 |
|---|---|---|---|
| A | 検証用バルーン fixture の作成 | 実装（自動） | ✅ 完了（§0.3） |
| B | 水準①（現在の拡大率）での実走とログ突合 | 実装（自動） | ✅ 完了（§7.1・**dpi=192／200%**） |
| C | 水準②（**100%＝dpi=96** 固定）での実走とログ突合 | **開発者** | ✅ 完了（§7.2） |
| D | 画面端・モニタ境界へのドラッグと**目視**確認 | **開発者** | ✅ 完了（§7.3） |

> **この表の ✅ は「実施済み」であって「無条件に有効」ではない。** 一部の記入は
> task 4.5 の是正**より前**の実行体で採られており、**キーワード基本位置に関する水準①の判定は
> 失効**している（§6 の但し書き・§7.1 冒頭・§7.3 冒頭の開示）。また **水準②の観測は
> 2 つの走行にまたがる**（§7.3 の「観測された走行」行）。読む前に §6 と §7.3 の開示を通すこと。

C は Windows の表示スケール変更（システム設定の変更）を伴うため、D は人間の目を要するため、
いずれも自動化の対象外である。

**水準②は 100%（dpi=96）に固定であり、選択の余地は無い。** タスク 4.4 が求めるのは
「拡大率 **100%** と 100% 以外（125% または 200%）の 2 水準」であり、水準①の実走が既に
**200%（dpi=192）**＝「100% 以外」の枠を埋めている（§7.1）。ゆえに残る枠は **100%** ただ 1 つで、
ここで 125% を選ぶと {200%, 125%} となり **k=1.0 を一度も踏まないまま**タスクが終わってしまう。
k=1.0 は「DPI 追従が基本設計で k=1.0 は途中状態」という前提そのものを検める水準なので、
省いてはならない。

---

## 0. 前提条件（実測で確認済み）

### 0.1 絶対パス

| 項目 | 絶対パス | 実測状況 |
|---|---|---|
| ゴースト root（辞書 13 本込みのフルゴースト） | `C:\home\maz\git\areka\.claude\worktrees\areka-p0-scope-chain-gap-004c39\crates\pilot\examples\shiori-host-32\fixtures\emo2` | ✅ 実在 |
| **検証用** balloon root（本タスクで新設） | `C:\home\maz\git\areka\.claude\worktrees\areka-p0-scope-chain-gap-004c39\crates\pilot\examples\shiori-host-32\fixtures\emo2-kakukaku-wplimit` | ✅ 実在 |
| 実 SHIORI（`pasta.dll`） | `…\fixtures\emo2\ghost\master\pasta.dll` | ✅ 実在（32bit） |
| 起動実行体 | `C:\home\maz\git\areka\.claude\worktrees\areka-p0-scope-chain-gap-004c39\target\debug\areka.exe` | ✅ ビルド済（PE machine `0x8664`） |
| 32bit helper（**areka.exe と同一ディレクトリ必須**） | `…\target\debug\shiori-host32-helper.exe` | ⚠️ **要ステージング**（§1） |
| helper の i686 成果物（コピー元） | `…\target\i686-pc-windows-msvc\debug\shiori-host32-helper.exe` | ✅ ビルド済（PE machine `0x014C`） |

**相対パスで起動しないこと。** `pasta.dll` の LOAD が `0x8007007E`（ERROR_MOD_NOT_FOUND）で
落ち、SHIORI が接続しない（steering `areka-emo2-signoff-needs-absolute-paths`）。

### 0.2 実 DPI（拡大率）

`crates/areka/src/main.rs` の `default_helper_exe_path()` と同じく、**表示スケールを指す env は無い**
——Windows の「システム > ディスプレイ > 拡大縮小」で変更し、**areka を起動し直す**。
実効 DPI は**外部ツールで推定せず、areka 自身のログ**で読むこと
（steering `effective-dpi-must-be-read-from-dpi-aware-process`）:

```
placement: 起動時 k₀ を導出（…） primary_dpi=192 shell_author_dpi=96 balloon_author_dpi=96 k_shell=2.0 k_balloon=2.0
[diag.monitor] index=0 handle=… bounds=0,0,2880,1800 work_area=0,0,2880,1704 dpi=192 primary=true
```

| 拡大率 | `primary_dpi` | `k_balloon` |
|---|---|---|
| 100% | 96 | 1.0 |
| 125% | 120 | 1.25 |
| 200% | 192 | 2.0 |

### 0.3 検証用バルーン fixture（本タスクで新設）

原本 `fixtures/emo2/emo2-kakukaku` の複製に、**面別上書き層のみ**を書き換えたもの
（画像・既定層は原本と同一。Paint.NET の作業ファイル `online.pdn` だけは実行に不要なので落とした）。

| ファイル | scope | 追加・変更した行 | 期待する基本位置 |
|---|---|---|---|
| `balloons0s.txt` | 0（sakura） | `windowposition.x,center`／`windowposition.limit,1`／`windowposition.y,0`（**検証の途中で変更**） | **中央上**（`CenterTop`） |
| `balloonk0s.txt` | 1（kero） | `windowposition.x,bottom`／`windowposition.limit,1`／`windowposition.y,0`（**検証の途中で変更**） | **中央下**（`CenterBottom`） |

- `windowposition.y` は**検証の途中で `0` へ変更した**（2026-08-14・当初は原本のまま
  sakura `-129`／kero `-75`）。理由: 原本の `y` は**数値指定用に作られた値**で、数値指定の
  基本位置（バルーン上端＝キャラ上端）を前提にしている。キーワードの基本位置は
  （中央上なら）バルーン**下端**がキャラ上端に接する位置なので、同じ `y` を流用すると
  バルーン高さぶん余計に浮く。§5 の ⑥（基本位置が意図どおりか）は**この状態でも観測できる**
  ——浮いた量が宣言値どおりかを頭の中で差し引けばよいからで、実際に水準①はその状態で
  ⑥ を PASS と記録している（§7.3）。**なおこの水準①の ⑥ は、後に「是正前の走行によるもの＝
  失効」として開示されている**（§7.3 冒頭）——ここで援用しているのは「非ゼロでも観測はできた」
  という観測可能性の事実だけであって、現行の幾何を認証する合格印としては読まないこと。
  `y,0` へ変えたのは**判定から交絡を取り除いて基本位置を
  素で見えるようにするため**であり、非ゼロでは観測不能だからではない。
  **要件 4.4（キーワード指定でも調整量の加算が続く）の実機証跡は、`y` が原本のままだった
  水準①（200%・§7.1・`adjust_dy=-258`／`-150`）が持つ。** 決定論檻
  （`t_k3_numeric_y_is_added_to_keyword_base_position`・4 DPI 水準 × 両モード）も併せて固定済み。
- 既定層（`descript.txt`）は原本と 1 バイトも違わない。**面別上書き層が勝つ**ことが
  観測できる形になっている（要件 1.4）。
- 要件 4.1 は `top` ≡ `center` を定めるので、キーワード幾何は `center` と `bottom` の
  2 種で全被覆になる。
- **原本を書き換えていない**理由: `fixtures/emo2` を読む既存テスト（採寸・合成・起動の約 40 本）が
  原本の数値指定 `x=266`／`x=-190` を期待値に持つ。ゆえに複製は `fixtures/emo2` ツリーの
  **外**（兄弟ディレクトリ）へ置いてある——ゴーストツリーの列挙にも影響しない。

---

## 1. 事前準備（i686 helper のステージング・毎回）

`main.rs` の `default_helper_exe_path()` は **`current_exe()` の隣**の
`shiori-host32-helper.exe` を無条件に解決する（helper パスを差す env は**存在しない**）。
`cargo build/test --workspace` は **x64 版**を `target\debug\` へ吐いて上書きするので、
実走の直前に必ず i686 版をコピーし直すこと。

i686 ビルドは**必ず PowerShell** で行う（Git Bash の coreutils `link.exe` が MSVC の
`link.exe` を遮蔽する）。

```powershell
cd C:\home\maz\git\areka\.claude\worktrees\areka-p0-scope-chain-gap-004c39
cargo build -p areka
cargo build -p shiori-host32-helper --target i686-pc-windows-msvc
Copy-Item target\i686-pc-windows-msvc\debug\shiori-host32-helper.exe target\debug\ -Force
```

検証（`0x14C` なら OK・`0x8664` なら x64 が残っている＝SHIORI が繋がらない）:

```powershell
function Get-Machine($p){ $b=[IO.File]::ReadAllBytes($p); $o=[BitConverter]::ToInt32($b,0x3C); "0x{0:X}" -f [BitConverter]::ToUInt16($b,$o+4) }
Get-Machine target\debug\shiori-host32-helper.exe   # 期待: 0x14C
Get-Machine target\debug\areka.exe                  # 期待: 0x8664
```

---

## 2. 起動（PowerShell・絶対パス・有界 auto-exit）

```powershell
cd C:\home\maz\git\areka\.claude\worktrees\areka-p0-scope-chain-gap-004c39
$ghost   = (Resolve-Path "crates\pilot\examples\shiori-host-32\fixtures\emo2").Path
$balloon = (Resolve-Path "crates\pilot\examples\shiori-host-32\fixtures\emo2-kakukaku-wplimit").Path
$exe     = (Resolve-Path "target\debug\areka.exe").Path
$dpi     = 96                                              # 水準②＝100%。実測 DPI に合わせる
$log     = "$env:USERPROFILE\Desktop\wplimit-signoff-dpi$dpi.log"

$env:AREKA_APP_SMOKE_EXIT_MS = "600000"                    # 10 分・目視とドラッグの猶予
$env:RUST_LOG                = "info,areka::placement::diag=debug"
& $exe $ghost $balloon *> $log
```

- `AREKA_APP_SMOKE_EXIT_MS`（`main.rs` の `SMOKE_EXIT_ENV`）: 指定 ms 後に全ゴースト窓を
  despawn して**正常終了**する有界 auto-exit。目視とドラッグを行う水準②では**大きめ**にすること
  （水準①の自動実走は 120000＝2 分だった）。
- `RUST_LOG` の `areka::placement::diag=debug` は**必須**。位置ログ `[diag.window_move]` は
  `debug!`（`diag.rs::log_window_move`）ゆえ、`info` だけでは突合の片側が採れない。
  補正ログ `[balloon-limit] Clamp` と縮退警告 `[balloon-limit] Unresolved`、
  解決値ログ（観測点 4）はいずれも `info`／`warn` なので `info` で足りる。
- 途中終了は **Ctrl+左ダブルクリック**（キャラ窓の不透明域）。放置しても指定 ms で自動終了する。

### 2.1 ログの保全

```powershell
$dst = "$env:LOCALAPPDATA\areka-diag\wplimit-signoff-$(Get-Date -Format yyyyMMdd-HHmmss)"
New-Item -ItemType Directory -Force $dst | Out-Null
Copy-Item $log $dst\
Get-FileHash $dst\*.log -Algorithm MD5
```

---

## 3. grep パターン（3 語＋2 語）

第 1 段は接頭辞 1 語で本機能の記録を全部拾える。第 2 段の `context=` で 3 関門を弁別する
（`balloon_limit.rs` の `BALLOON_LIMIT_*_CONTEXT` 定数）。

```powershell
$L = "<保全したログのパス>"

# 第1段: 本機能の記録すべて（補正＋縮退）
Select-String -Path $L -Pattern '\[balloon-limit\]'

# 第2段: 関門の弁別
Select-String -Path $L -Pattern 'context="boot-gate"'      # 起動時関門
Select-String -Path $L -Pattern 'context="runtime-gate"'   # 実行時関門（enqueue_window_set_pos 内）
Select-String -Path $L -Pattern 'context="release-gate"'   # バルーンドラッグ解放時補正

# 縮退（1 件でも出たら要調査。作業領域が解決できず補正を諦めた印）
Select-String -Path $L -Pattern '\[balloon-limit\] Unresolved'

# 位置ログ（最終位置・route つき）
Select-String -Path $L -Pattern '\[diag\.window_move\]'

# 解決値ログ（観測点 4・limit と x_mode の実値）
Select-String -Path $L -Pattern 'windowposition を初期既定位置の調整量へ変換した'

# 実効 DPI（areka 自身の申告）
Select-String -Path $L -Pattern '起動時 k₀ を導出|\[diag\.monitor\] '
```

Git Bash なら `grep -n '\[balloon-limit\]' "$L"` 等で同じ。

---

## 4. 合否判定（ログ突合・機械判定できる部分）

### J1: 解決値ログが fixture のキーワードどおりであること（要件 1.4／4.1／6.2）

観測点 4 の行が scope 0／1 の**両方**出ており、かつ:

| scope | 期待 `limit` | 期待 `x_mode` | 期待 `windowposition_x` |
|---|---|---|---|
| 0 | `true` | `CenterTop` | `None`（キーワードなので数値 x は存在しない） |
| 1 | `true` | `CenterBottom` | `None` |

`windowposition_y` は**その時点の fixture が宣言している値**と一致すること
（`adjusted=true`・`adjust_dx=0`・`adjust_dy = 宣言値×k`。丸めは `ScaleRatio::scale_len`＝
0 から遠い側へ half away from zero）。**現行 fixture は `y,0` ゆえ `Some(0)`／`adjust_dy=0`**。
`y` を原本の `-129`／`-75` に戻して走らせた場合は `adjust_dy = -129×k`／`-75×k` になる
（水準①＝§7.1 がその状態での実測）。合格判定は「宣言値と実測が一致すること」であって
特定の数値ではない——fixture を変えたら基準も動く。

> **行が出ない scope があっても即「未実装」と読まないこと。** `scope_windowposition` が
> `None` を返す 3 経路（scope が u32 に収まらない／系列解決失敗／面 0 不在）では観測点 4 の行は
> 出ない。ただしその場合は**必ず直前に warn が出ている**ので、warn の有無で切り分ける。

### J2: キーワード基本位置の幾何（要件 4.2／4.3／4.4）

`merge_scope restore` 行（`char_x`／`char_y`／`char_w`／`char_h`）と
`[balloon-limit] Clamp context="boot-gate"` 行の `from`／`balloon_size` から、
**補正前**の基本位置が式どおりであることを検算する。

- 共通（水平中央・DD8 の整数除算＝0 方向切り捨て）:
  `from.x == char_x + (char_w - balloon_w) / 2`
- `CenterTop`（scope 0）: `from.y == char_y - balloon_h + adjust_dy`
- `CenterBottom`（scope 1）: `from.y == char_y + char_h + adjust_dy`

補正が起きない（＝基本位置が既に作業領域内）ときは `Clamp` 行が出ないので、
`from` の代わりに `[diag.window_move]` の `x`／`y` を使う。

### J3: 補正ログと位置ログの一致（要件 6.1）

各 `[balloon-limit] Clamp` 行の直後（同一 `entity=` / 同一 `scope=`）に
`[diag.window_move] … kind=balloon` が出ており、**`to=PointPx { x, y }` と
位置ログの `x=`／`y=` が完全一致**すること。

> 起動時関門（`boot-gate`）は窓書込より前のデータ変換なので**対になる位置ログを持たない**。
> 対応づけるのは `runtime-gate`／`release-gate` の 2 種だけである。

### J4: 補正結果が作業領域に内包されること（要件 2.1／2.3／2.4）

`Clamp` 行の `to`・`balloon_size`・`work_area` から算出する。`right`／`bottom` は**排他**:

```
to.x >= work_area.left  かつ  to.x + balloon_size.w <= work_area.right
to.y >= work_area.top   かつ  to.y + balloon_size.h <= work_area.bottom
```

バルーンが作業領域より大きい逆転区間では `to.x == work_area.left`／`to.y == work_area.top`
（左辺・上辺優先）になり、はみ出したままが**正しい**（要件 2.4）。

### J5: 縮退が起きていないこと

`[balloon-limit] Unresolved` が **0 件**。1 件でも出ていたら、その scope は補正されずに
素通ししているので、目視の合否をその scope について主張してはならない。

### J6: キャラ窓が動いていないこと（要件 2.8）

`[balloon-limit] Clamp` の前後で `[diag.window_move] … kind=char` の値が変わらないこと
（補正はバルーン窓だけに作用する）。

---

## 5. 目視チェックリスト（開発者の担当・要件 7.5）

**各拡大率で以下の 5 点を行う。** 実走中（§2 の auto-exit の猶予内）に行うこと。

1. **左端**: キャラ窓をドラッグして画面の**左端**へ寄せる。バルーンの左辺が
   作業領域の左端で止まり、画面外へ切れないこと。
2. **右端**: 同じく**右端**へ寄せる。バルーンの右辺が作業領域の右端で止まること。
3. **上端**: **上端**へ寄せる。scope 0 は中央上（キャラの上）なので、ここが最も
   はみ出しやすい。バルーンの上辺が作業領域の上端で止まること。
4. **下端**: **下端**へ寄せる。scope 1 は中央下（キャラの下）なので、ここが最も
   はみ出しやすい。バルーンの下辺が作業領域の下端（タスクバーの上）で止まること。
5. **モニタ境界**: キャラ窓を**もう一方のモニタ**へまたがせる。基準は
   **キャラ窓が属するモニタ**の作業領域（要件 5.5）なので、キャラの中心が属する側の
   作業領域で止まること。副モニタは作業領域＝境界（タスクバーなし）のことがあるので、
   `[diag.monitor]` 行の `work_area=` を先に読んでから判定する。

**加えて、基本位置が意図どおりであること**（キーワードの目視・要件 4.2／4.3）:

6. 画面中央付近にキャラを置いた状態で、**sakura（scope 0）のバルーンがキャラ画像の
   真上・水平中央**に、**kero（scope 1）のバルーンがキャラ画像の真下・水平中央**に
   出ていること。**`windowposition.y` が非ゼロの fixture なら**その値ぶん上へずれているのが
   正しい（現行 fixture は `y,0` ゆえずれ無し＝基本位置が素で見える）。

**任意（`release-gate` を踏みたいとき）**: バルーン窓そのものを掴んで画面外へ引っ張り、
**そこで手を離す**。解放位置が作業領域外なら `context="release-gate"` の `Clamp` が
1 行だけ出て、表示位置だけが引き戻される（保存される相対位置は生値のまま）。
バルーン単独ドラッグをしなければこの語は 0 件で正常。

---

## 6. 完了判定

| 判定 | 内容 | 水準①（済） | 水準②（済） |
|---|---|---|---|
| J1〜J6 | §4 のログ突合が全て PASS | ✅（**J2 のみ是正前＝失効**） | ✅ |
| §5 の 1〜5 | 画面端・モニタ境界での目視 | ✅（**是正前**・封じ込めは有効） | ✅（**是正前**・封じ込めは有効） |
| §5 の 6 | キーワード基本位置の目視 | ✅（**是正前＝失効**） | ✅（是正後） |

**2 水準とも**全項目が埋まってはじめてタスク 4.4 は完了である。

> 水準①の記入は task 4.5 の是正**より前**のビルドで採られている（§7.1・§7.3 冒頭の開示）。
> limit の封じ込め（J1・J3〜J6・§5 の 1〜5）は是正の影響を受けず有効だが、
> **キーワード基本位置に関する水準①の記入（J2・§5 の 6）は是正後の幾何を認証しない。**
>
> 水準②も**1 つの走行では埋まっていない**。§5 の 1〜5（画面端・モニタ境界）は**是正前**の
> 走行 `wplimit-signoff-dpi96-20260814-192617` の観測で、§5 の 6 と J1〜J6 は**是正後**の
> 走行 `wplimit-dpi96-fixed-20260814-203002` の観測である（§7.3 の「観測された走行」行）。
> 前者が是正前であることは封じ込めの判定を変えない——理由は §7.3 冒頭の開示に書いた。

---

## 7. 記録欄

### 7.1 水準①（自動実走・2026-08-14）— ログ突合のみ完了

> ## ⚠ 開示: この走行は**是正前**のビルドである
>
> 本走行は task 4.5（コミット `d6cb7a52`・キーワード基本位置を実表示寸から導出し直す是正）
> **より前**の実行体で採られている。task 4.5 が直した欠陥そのものが、本走行の記録の中に
> 現れている:
>
> - 本走行が記録した起動時のキャラ寸は `char 868x1374`。これは**採寸値** `434x687` の
>   ちょうど 2 倍（k=2.0）である。
> - §7.2 が後に実測で示したとおり、実際に表示されるキャラ寸は `382x547`（k=1.0）＝
>   k=2.0 なら `764x1094` であり、採寸値とは食い違う。
> - キーワードの水平中央は `(char_w − balloon_w) / 2` ゆえ、`868` と `764` の差の半分
>   **52px** だけ、本走行のバルーン基本位置は中央から右へずれていた。
>
> **この開示によって無効になる記録／ならない記録:**
>
> | 区分 | 項目 | 理由 |
> |---|---|---|
> | **有効（是正の影響を受けない）** | J3・J4・J5・J6、および「作業領域への内包」という limit の主題そのもの | 関門はバルーンの**提示位置がどこであれ**それを作業領域へ収める。基本位置が 52px ずれていたことは、収まったかどうかの判定を変えない |
> | **有効** | J1（解決値ログ＝`limit`／`x_mode`／`adjust_dy` の実値） | 語彙分類と調整量の scale は task 4.5 が触っていない |
> | **有効** | DD6（補正を相対位置へ焼き付けない）の実測 | 生 offset が保たれることの検算であり、基本位置の正しさに依存しない |
> | **失効（是正前の幾何を証明しているだけ）** | J2（キーワード基本位置の幾何） | 検算が通っているのは「**是正前の実装が採寸寸どおりに計算していた**」ことであって、出荷される幾何ではない |
>
> **帰結: キーワード基本位置について、k≠1 での是正後の実機証跡は存在しない。**
> 是正後に採れているのは §7.2（k=1.0・100%）だけである。これは本仕様の**未解決の残件**として
> ここに登記する（是正後の 200% 再走を行えば埋まる）。

| 項目 | 値 |
|---|---|
| 実行日時 | 2026-08-14 07:03:03Z 〜 07:05:04Z（121.0 秒・`AREKA_APP_SMOKE_EXIT_MS=120000`） |
| 終了 | exit 0（`shiori-actor: 正規 clean shutdown 完了` → `ghost shutdown sequence completed`） |
| **areka 自身が申告した実効 DPI** | `primary_dpi=192`（**200%**）・`k_shell=2.0`・`k_balloon=2.0` |
| モニタ構成 | 2 台。index0 `bounds=0,0,2880,1800` `work_area=0,0,2880,1704` `dpi=192` primary／index1 `bounds=-2560,195,0,1795` `work_area` 同値 `dpi=144` |
| ログ | `%LOCALAPPDATA%\areka-diag\wplimit-signoff-20260814-160303\wplimit-dpi192.log`（101,253 bytes・md5 `CA3D795DA2CD14A593476E570A1F95C6`・389 行） |

**J1（解決値ログ）: PASS**

```
scope=0 windowposition_x=None windowposition_y=Some(-129) balloon_side=Left  adjust_dx=0 adjust_dy=-258 adjusted=true k=2.0 limit=true x_mode=CenterTop
scope=1 windowposition_x=None windowposition_y=Some(-75)  balloon_side=Right adjust_dx=0 adjust_dy=-150 adjusted=true k=2.0 limit=true x_mode=CenterBottom
```

面別上書き層のマージも `file=balloons0s.txt`／`file=balloonk0s.txt` で確定している。
`-129×2 = -258`・`-75×2 = -150` で調整量も一致（キーワード指定でも加算が続いている＝要件 4.4）。

**J2（キーワード基本位置の幾何）: PASS**

| scope | 実測（`merge_scope restore` ＋ `Clamp` の `from`） | 検算 |
|---|---|---|
| 0 | `char=(2012,330) 868x1374`・`balloon 800x448`・`from=(2046,-376)` | x: `2012 + (868-800)/2 = 2046` ✅／y: `330 - 448 - 258 = -376` ✅（CenterTop） |
| 1 | `char=(1340,904) 672x800`・`balloon 576x406`・`from=(1388,1554)` | x: `1340 + (672-576)/2 = 1388` ✅（是正前）／y: `904 + 800 - 150 = 1554` ✅（CenterBottom） |

> **この ✅ が意味するのは「是正前の実装が採寸寸どおりに計算していた」ことに限られる**
> （§7.1 冒頭の開示）。入力の `char_w` が採寸値（scope 0 は `868`・実表示は `764` 相当）
> であるため、検算が通っていても出荷される幾何を証明しない。task 4.5 の是正後は同じ入力
> 状況で `x = 2012 + (764−800)/2 = 1994` 側へ動く。**是正後の幾何の実機証跡は §7.2（k=1.0）
> のみ**であり、k≠1 では未取得である。

**J3（補正ログ ⇄ 位置ログ）: PASS**

`Clamp` は 4 件（`boot-gate` 2・`runtime-gate` 2・`release-gate` 0）。
`runtime-gate` の 2 件がいずれも直後の位置ログと逐語一致した。

```
[balloon-limit] Clamp runtime 関門 … scope=0 context="runtime-gate" entity=3v0 route=Some(BalloonFollow)
    from=PointPx { x: 2098, y: -96 } to=PointPx { x: 2080, y: 0 } balloon_size=SizePx { w: 800, h: 448 }
[diag.window_move] route=BalloonFollow entity=3v0 kind=balloon scope=0 x=2080 y=0 w=- h=- dpi=192      ← to と一致

[balloon-limit] Clamp runtime 関門 … scope=1 context="runtime-gate" entity=5v0 route=Some(BalloonFollow)
    from=PointPx { x: 1440, y: 1554 } to=PointPx { x: 1440, y: 1298 } balloon_size=SizePx { w: 576, h: 406 }
[diag.window_move] route=BalloonFollow entity=5v0 kind=balloon scope=1 x=1440 y=1298 w=- h=- dpi=192    ← to と一致
```

起動時関門の 2 件（対になる位置ログを持たない）:

```
scope=0 context="boot-gate" from=(2046,-376) → to=(2046,0)    balloon 800x448
scope=1 context="boot-gate" from=(1388,1554) → to=(1388,1298) balloon 576x406
```

**相対位置が焼き付いていないこと（DD6）も実測で確認できた**: 起動時関門が scope 0 の表示位置を
`y=-376 → 0` へ引き戻したにもかかわらず、その後の追従書込は
`char(2064,610) + 生 offset(34,-706) = (2098,-96)` を素で提示している（＝補正後の `0` ではなく
生値の `-706` が保たれている）。scope 1 も同様（`char(1392,904) + 生 offset(48,650) = (1440,1554)`）。

**J4（作業領域への内包）: PASS**（`work_area=0,0,2880,1704`・右下は排他）

| scope | `to` | 右辺 | 下辺 | 判定 |
|---|---|---|---|---|
| 0（runtime） | (2080, 0) | `2080+800 = 2880 <= 2880` | `0+448 = 448 <= 1704` | ✅ |
| 1（runtime） | (1440, 1298) | `1440+576 = 2016 <= 2880` | `1298+406 = 1704 <= 1704` | ✅ |

**J5（縮退なし）: PASS** — `[balloon-limit] Unresolved` 0 件。

**J6（キャラ窓が動かない）: PASS** — `kind=char` の書込は**ログ全体で 2 件のみ**
（L65 `scope=0 (2064,610) route=ReportedSizeReconcile`／L77 `scope=1 (1392,904) route=MoveCue`）で、
いずれも関門とは無関係な経路である。runtime 関門 L66 の直後に現れるのはバルーン書込 L67 だけ、
同じく L78 の直後は L79 だけで、**補正に起因するキャラ窓の書込は 1 件も無い**
（関門がキャラ窓へ波及していない）。

> 上記 2 件の `kind=char` は、時系列では起動時関門 L25／L26（07:03:04.267）の**後**に位置する
> （L65 = 07:03:05.742・L77 = 07:03:07.936）。ただし両者は `ReportedSizeReconcile`／`MoveCue`
> という別経路の書込であり、起動時関門の結果ではない。「補正の後にキャラ窓の書込が一切無い」
> という読み方は事実に反するので採らないこと。

なお本実走はドラッグを含まないため、キャラ窓の書込そのものが 2 件しかない——
「端へ寄せてもキャラが跳ねない」ことの確認は水準②の目視（§5）の担当である。

**未実施**: §5 の目視（1〜6）。この実走は自動実行のため、ドラッグも目視も行っていない。

### 7.2 水準②（開発者・拡大率 **100%（dpi=96）固定**）

**完了（2026-08-14）。** 実走中に要件 4.2 の欠陥を 1 件発見し、本仕様内で是正したうえで再走した
（詳細は tasks.md のタスク 4.5）。

> **本節で引用する走行は 1 本ではない。** 直下の表と J1〜J6・再導出は**是正後**の走行
> `wplimit-dpi96-fixed-20260814-203002` の実測である。末尾の「保存・復元の実機確認」だけは
> 別の 3 起動（`wplimit-final-20260814-215251`）と、DD6 の 1 点のみ 200% 側の走行
> （`wplimit-signoff-visual-20260814-191549`・**是正前**）を出所とする——各項目に明記した。
> **画面端・モニタ境界の目視（§5 の 1〜5）は本節の走行では行っていない**。それは別走行
> `wplimit-signoff-dpi96-20260814-192617` の担当で、記録は §7.3 にある。

| 項目 | 値 |
|---|---|
| areka 自身が申告した実効 DPI | `primary_dpi=96`・`k_shell=1.0`・`k_balloon=1.0`（＝**k=1.0 を踏んだ**） |
| モニタ | index0 `work_area=0,0,2880,1752`（タスクバー 48px）／index1 `work_area=-2560,195,0,1795` `dpi=144`（タスクバー無し） |
| 解決値 | scope0 `limit=true x_mode=CenterTop`／scope1 `limit=true x_mode=CenterBottom`（面別上書き層が勝っている） |
| ログ | `%LOCALAPPDATA%\areka-diag\wplimit-dpi96-fixed-20260814-203002\wplimit-dpi96-fixed.log`（266,010 bytes・md5 `0A3168BD606CA88ACC42101D122DD410`） |

**J1 解決値ログ: PASS** — 両 scope とも `limit` と `x_mode` の実値が出ている。
**この走行の fixture は `windowposition.y,0`**（§0.3 参照）ゆえ `windowposition_y=Some(0)`・
`adjust_dy=0` で、宣言値と実測が一致している。`y` が非ゼロの状態での実機証跡（要件 4.4）は
水準①＝§7.1（`adjust_dy=-258`／`-150`）が持つ。

**J2 キーワード基本位置の幾何: PASS** — `char=(2472,1205) 382x547`・`balloon 400x224` に対し
`x = 2472 + (382−400)/2 = 2463`・`y = 1205 − 224 = 981`。実測 `balloon x=2463 y=981` と完全一致。

**J3 補正ログ ⇄ 位置ログ: PASS** — `boot-gate` 1 件・`runtime-gate` 1 件、いずれも直後の
`[diag.window_move]` と値が一致。**この 2 件はどちらも scope 1（中央下）が作業領域の
下辺で止められたもの**（`from=(…,1752) → to=(…,1549)`・`work_area` 下辺 `1752`）で、
下記 J4 が「不要になった」と述べている scope 0 の**右端**クランプとは別件である。

**J4 作業領域への内包: PASS** — **scope 0（中央上）について**、是正後は水平中央位置が最初から
領域内に収まり、**右端**クランプが不要になった（是正前は `x=2489` で 9px はみ出し、関門が
2480 へ引き戻していた）。**scope 1（中央下）の下辺クランプは是正後も必要で、実際に 2 件出ている**
（J3）——中央下の基本位置がキャラ足元の下＝作業領域の外側に落ちるのは正常な入力であり、
関門が拾うべき筋そのものである。この走行の補正は合計 2 件で、いずれも内包判定を満たす。

**J5 縮退なし: PASS** — `[balloon-limit] Unresolved` 0 件。

**J6 キャラ窓が動かない: PASS** — 補正に起因する `kind=char` の書込なし。

**キーワード基本位置の再導出（本仕様 4.5 で新設）: 実機で確認**

```
[balloon-keyword] Rederive  scope=0  mode=CenterTop
  old_offset=(+17,-224) → new_offset=(-9,-224)
  old_char_size=434x687 → new_char_size=382x547
```

発火は**1 回のみ**（要件 4.7「キーワードは初期既定位置の供給にとどめる」が実機で成立）。

**保存・復元の実機確認（追加検証）**

> **この小節の証跡は上表のログではなく、別に採った 3 起動ぶんのログ
> `%LOCALAPPDATA%\areka-diag\wplimit-final-20260814-215251\` にある**（`wplimit-dpi96-visual-final.log`
> ／`wplimit-roundtrip-final.log`／`wplimit-restore-visual.log` の 3 本＝**1 本 1 起動で計 3 サイクル**）。
> 上表の `wplimit-dpi96-fixed-20260814-203002` は**起動 1 回**しか含まないので、3 サイクルの
> 主張の出所にはならない。

- 同一 DPI 内の往復は**完全恒等**: 1 サイクル目（`wplimit-dpi96-visual-final.log`）が
  ドラッグで保存した `(-101,-158)`（scope 0）／`(154,-95)`（scope 1）を、2 サイクル目
  （`wplimit-roundtrip-final.log`）と 3 サイクル目（`wplimit-restore-visual.log`）の
  `merge_scope restore` が `saved_off_x`／`saved_off_y` として同値で読み戻している
  （1 ビットも動かず＝ドリフト無し）。
- 保存値がある scope では**再導出が発火しない**: 保存値を持った状態で起動した 2 サイクル
  （2・3 サイクル目）は `[balloon-keyword] Rederive` **0 件**。保存値がまだ無かった
  1 サイクル目では 1 件発火しており、これは「保存値が無いときは供給する」という同じ規則の表側である。
  ただしこの再起動試験が踏んでいるのは要件 4.7 の**起動時の腕だけ**である
  （`persist::merge_scope` が読み込んだ保存値で素材を落とす経路）。**セッション中の腕**
  ——バルーン単独ドラッグで保存値が生まれ、その後にキャラ窓の寸が変わる筋——には
  **実機証跡が無い**。こちらは決定論テスト
  `a_dragged_relative_position_survives_a_later_char_size_change`
  （`follow_keyword_base_tests.rs`・task 4.6）が担保している。
- **DD6 が実機で確認できた**（拡大率 200% 側・出所は §7.3 の水準①目視実走
  `wplimit-signoff-visual-20260814-191549`）: バルーンを画面外 `y=-107` で解放したとき、
  同時刻の保存は生値 `persist=(-143,-717)`、`release-gate` は表示だけを
  `from=(1983,-107) → to=(1983,0)` へ補正した。補正は相対位置へ焼き付いていない。

### 7.3 目視サインオフ（開発者・両水準）

> ## ⚠ 開示: 水準①の目視は**是正前**のビルドで行われた
>
> 下表の水準①欄は、直下の目視実走
> （ログ `wplimit-signoff-visual-20260814-191549`・19:15 開始）に基づく。この走行は
> task 4.5（コミット `d6cb7a52`）**より前**の実行体であり、当時のバルーン基本位置は
> 採寸寸から導かれていた——k=2.0 では中央から **52px** 右へずれた状態である
> （内訳は §7.1 冒頭の開示）。
>
> - **5-1〜5-5（画面端・モニタ境界での封じ込め）は有効**。limit の関門は基本位置がどこで
>   あれ提示された矩形を作業領域へ収めるので、52px のずれは合否を変えない。同走行の
>   **補正 930 件を機械的に検算してはみ出し 0 件**という証跡（本節「補正の実測」）も同様に有効。
> - **5-6（中央上・中央下の基本位置）は失効**。是正前の幾何を見た判定であり、出荷される
>   幾何を認証しない。同じ理由で §7.1 の J2（水準①）も失効している。
> - ✅ 印は当時の観測が実際に PASS だったという事実の記録なので取り消さない。**是正後の
>   認証としては読まないこと。**
>
> **未解決の残件: キーワード基本位置について、k≠1 での是正後の実機証跡が存在しない。**
> 是正後に採れているのは水準②（k=1.0・§7.2）のみ。200% で再走すれば埋まる。

> ## ⚠ 開示: 水準②の目視も**1 つの走行では採れていない**（2 走行にまたがる）
>
> 下表の水準②欄は、**別々の 2 走行**の観測である。
>
> - **5-1〜5-5（画面端・モニタ境界での封じ込め）** は
>   `wplimit-signoff-dpi96-20260814-192617`（19:18:58Z 開始・約 7 分・`primary_dpi=96`・
>   `work_area=0,0,2880,1752`）で観測した。この走行は task 4.5（`d6cb7a52`）**より前**＝
>   **是正前**である（`merge_scope restore` のキャラ寸が採寸値 `434x687` のまま・
>   `Rederive` 0 件がその印）。
> - **5-6（中央上・中央下の基本位置）** は §7.2 の**是正後**の走行
>   `wplimit-dpi96-fixed-20260814-203002` で観測した。こちらは是正後の認証として有効である。
>
> **5-1〜5-5 が是正前であっても封じ込めの判定は有効**——理由は水準①と同じで、
> 関門はバルーンの提示位置が**どこであれ**それを作業領域へ収めるからである。加えて本件は
> **引用の誤りであって証跡の誤りではない**: 関門の純関数 `balloon_limit.rs` は task 3.4
> （`b2d18e9a`）以降 1 バイトも変わっておらず、4.5／4.6 は関門の**手前**に工程を挿しただけで
> 関門の呼び出し点を動かしていない。ゆえに是正前後で封じ込めの挙動は同一である。
>
> **是正前の走行で封じ込めが成立していることは機械的に検算済み**: `Clamp` 517 件
> （`runtime-gate` 513・`release-gate` 4・`boot-gate` 0）の全件について、各行が自分で
> 記録している `work_area=` に対し補正後の矩形が完全内包であることを確認した（はみ出し **0 件**・
> `Unresolved` **0 件**）。
>
> **なぜ取り違えたと判るか（算数）**: 5-1〜5-5 は幅 2880 の作業領域で四辺すべてへドラッグする
> 項目である。`wplimit-dpi96-fixed-20260814-203002` の `runtime-gate` は **1 件**しかなく、
> これはドラッグを一度も伴わない起動直後の 1 回ぶんに相当する。比較対象の 200% 目視実走が
> 同じ 5 項目で **921 件**を出していることと突き合わせれば、1 件の走行が
> 5-1〜5-5 を産んだはずがないことは件数だけで判る。
>
> ✅ 印は当時の観測が実際に PASS だったという事実の記録なので取り消さない。**どちらの走行の
> 観測かを見てから読むこと。**

| 項目 | 水準①（200%） | 水準②（100%） |
|---|---|---|
| 5-1 左端 | ✅ PASS（**是正前**） | ✅ PASS（**是正前**） |
| 5-2 右端 | ✅ PASS（**是正前**） | ✅ PASS（**是正前**） |
| 5-3 上端 | ✅ PASS（**是正前**） | ✅ PASS（**是正前**） |
| 5-4 下端 | ✅ PASS（**是正前**） | ✅ PASS（**是正前**） |
| 5-5 モニタ境界 | ✅ PASS（**是正前**） | ✅ PASS（**是正前**） |
| 5-6 中央上・中央下の基本位置 | ✅ PASS（**是正前＝失効**・上の開示） | ✅ PASS（**是正後**） |
| 観測された走行（5-1〜5-5） | **是正前**・下記「水準①の目視実走」（`wplimit-signoff-visual-20260814-191549`） | **是正前**・`wplimit-signoff-dpi96-20260814-192617`（下記「水準②の端ドラッグ実走」） |
| 観測された走行（5-6） | 同上（**是正前ゆえ失効**） | **是正後**・§7.2 の走行（`wplimit-dpi96-fixed-20260814-203002`） |
| サインオフ者 / 日付 | 開発者 / 2026-08-14 | 開発者 / 2026-08-14 |

> **5-1〜5-5 の「是正前」注記は失効を意味しない。** 失効しているのは水準①の 5-6 と J2
> （キーワード基本位置）だけである。封じ込めの 5 項目は上の 2 つの開示に書いたとおり、
> 是正の影響を受けないので**両水準とも有効**である。

#### 水準①の目視実走（`primary_dpi=192`・k=2.0）

| 項目 | 値 |
|---|---|
| 起動 | `AREKA_APP_SMOKE_EXIT_MS=600000`・`RUST_LOG=info,areka::placement::diag=debug`・絶対パス・検証 fixture `emo2-kakukaku-wplimit` |
| areka 自身が申告した実効 DPI | `primary_dpi=192`・`k_shell=2.0`・`k_balloon=2.0` |
| モニタ | index0 `work_area=0,0,2880,1704`（タスクバー 96px）／index1 `work_area=-2560,195,0,1795`（**タスクバー無し＝作業領域＝画面全体**） |
| 解決値 | scope0 `limit=true x_mode=CenterTop adjust_dy=-258`／scope1 `limit=true x_mode=CenterBottom adjust_dy=-150` |
| ログ | `%LOCALAPPDATA%\areka-diag\wplimit-signoff-visual-20260814-191549\wplimit-signoff-visual.log`（1,122,588 bytes・md5 `B7B209F1C79AF3227F5386BCB313E6E4`・4,364 行） |

**補正の実測（実ドラッグを伴う走行）**

| 関門 | 件数 |
|---|---|
| `boot-gate` | 2 |
| `runtime-gate` | 921（`route=BalloonFollow` 919・`KeepPositionResize` 2） |
| **`release-gate`** | **7** |
| `Unresolved`（縮退） | **0** |

- **補正 930 件すべてについて、補正後の矩形が当該 `work_area` へ完全内包されていることを
  機械的に検算した（はみ出し 0 件）。** 判定は各行が自分で記録している `work_area=` に対して行い、
  モニタを跨いだ分も取り違えないようにした。
- **`release-gate` に初めて実機証跡が付いた**（自動実走 §7.1 では 0 件だった）。
  バルーン単独ドラッグを画面外で解放し、表示位置だけが引き戻されることを目視で確認済み。
- `Unresolved` が 1 件も出ていない＝縮退経路を一度も踏まずに全補正が成立している。

#### 水準②の端ドラッグ実走（`primary_dpi=96`・k=1.0・**是正前**）

上表の水準② 5-1〜5-5 はこの走行の観測である（5-6 は §7.2 の是正後走行が持つ）。

| 項目 | 値 |
|---|---|
| 起動 | `RUST_LOG=info,areka::placement::diag=debug`・絶対パス・検証 fixture `emo2-kakukaku-wplimit` |
| 実行時間 | 2026-08-14 10:18:58Z 〜 10:25:54Z（約 7 分・`ghost shutdown sequence completed` で正常終了） |
| areka 自身が申告した実効 DPI | `primary_dpi=96`・`k_shell=1.0`・`k_balloon=1.0`（＝**k=1.0 を踏んでいる**） |
| モニタ | index0 `work_area=0,0,2880,1752`（タスクバー 48px）／index1 `work_area=-2560,195,0,1795` `dpi=144`（タスクバー無し） |
| fixture の `windowposition.y` | **原本のまま**（sakura `-129`／kero `-75`）——`y,0` へ変えたのはこの走行より後（§0.3） |
| 解決値 | scope0 `limit=true x_mode=CenterTop adjust_dy=-129`／scope1 `limit=true x_mode=CenterBottom adjust_dy=-75`（`k=1.0` ゆえ宣言値と同値） |
| ビルド世代 | **是正前**（task 4.5 `d6cb7a52` より前）。`merge_scope restore` のキャラ寸が採寸値 `434x687`・`[balloon-keyword] Rederive` 0 件 |
| ログ | `%LOCALAPPDATA%\areka-diag\wplimit-signoff-dpi96-20260814-192617\wplimit-signoff-dpi96.log`（1,000,087 bytes・md5 `0ABDED2ED9A9656153A695ED0B5BB40B`） |

**補正の実測（実ドラッグを伴う走行）**

| 関門 | 件数 |
|---|---|
| `boot-gate` | 0（起動時は保存済みの相対位置が効いており、基本位置が最初から作業領域内だった） |
| `runtime-gate` | 513 |
| **`release-gate`** | **4** |
| `Unresolved`（縮退） | **0** |

- **補正 517 件すべてについて、補正後の矩形が当該 `work_area` へ完全内包されていることを
  機械的に検算した（はみ出し 0 件）。** 判定は各行が自分で記録している `work_area=` に対して
  行っており、副モニタ（`work_area=-2560,195,0,1795`）を跨いだ分も取り違えていない。
- `release-gate` の 4 件は上下左右いずれの向きにも出ている（例: `from=(2604,-27) → to=(2480,0)`
  ＝右上／`from=(-914,1534) → to=(0,1416)`＝左下）。
- **この走行が是正前であることは封じ込めの判定を変えない**（§7.3 冒頭・2 つめの開示）。
  失効しているのはキーワード基本位置の判定だけで、それは水準①側の話である。

---

## 8. 申し送り

- **未解決の残件（記録の欠落）: キーワード基本位置について、k≠1 での是正後の実機証跡が無い。**
  水準①の走行はいずれも task 4.5（`d6cb7a52`）より前の実行体で採られており、キーワード
  基本位置について是正後に採れているのは水準②の `wplimit-dpi96-fixed-20260814-203002`
  （k=1.0）だけである（§7.1・§7.3 の開示）。limit の封じ込め自体は両水準とも有効なので
  4.4 の主題は満たされているが、基本位置の実機認証は 200% で 1 回再走すれば埋まる。
- **サインオフ記録には「どのビルドで採ったか」を必ず書くこと。** この欠落を作ったのは
  §7.1／§7.3 がコミットを記していなかったことで、後から入った是正が証明書を静かに
  無効化しても誰も気付けない形になっていた。
- **併せて「どの走行で観測したか」も項目ごとに書き、書いた引用をその走行の中身と突き合わせること。**
  §7.3 の水準②欄は当初、6 項目すべてを `wplimit-dpi96-fixed-20260814-203002` 1 本に
  帰していたが、その走行の `runtime-gate` は 1 件しかなく、四辺へのドラッグを含む
  5-1〜5-5 を産めるはずがなかった（比較対象の 200% 目視実走は同じ 5 項目で 921 件）。
  実際の出所は別走行 `wplimit-signoff-dpi96-20260814-192617` である。**どの走行も裏付けられない
  帰属は、陳腐化したビルドを開示しないのと同じ欠陥である**——どちらも「読む限り全項目が
  合格に見える」状態を作る。件数・時間帯・`work_area`・保存値の有無といった、走行が自分で
  記録している量で引用を検算できる。
- **実走のたびに i686 helper を貼り直すこと。** `cargo test --workspace` を挟むと
  `target\debug\shiori-host32-helper.exe` が x64 で上書きされ、SHIORI が繋がらなくなる
  （症状は「窓は出るが一言も喋らない」）。
- **永続状態**: `fixtures/emo2/ghost/master/profile/areka/sylphya.toml`（gitignore 済み）。
  §7.1 の**自動実走**（ドラッグを含まない）の後は `[boot] count = "1"` のままで、
  窓位置は保存されていなかった。
  ただし**バルーンやキャラをドラッグすると相対位置が保存され、次回起動の初期位置が変わる**
  ——実際、目視走行の後は保存値が残り、§7.2 の「保存・復元の実機確認」はそれを読み戻している。
  水準を変えて比較するときは、実走前にこのファイルの窓位置の項目を消すか、
  ファイルごと削除してから起動すること（削除しても boot count が 0 に戻るだけで無害）。
- **`[diag.window_move]` の `w=` / `h=` は `-`（未知）になることがある。** 内包判定
  （J4）に使う寸法は `[balloon-limit] Clamp` 行の `balloon_size=` から採ること。
- **`Clamp` が 0 件でも即 FAIL ではない。** `limit_correction` の `None` は
  「クランプしても位置が動かない」の意であり、基本位置が最初から作業領域内なら補正は起きない。
  その場合は `[diag.window_move]` の最終位置に対して J4 の内包判定を行う。
- **本 fixture の `windowposition.limit,1` は正典既定と同値**である。`limit=0` の素通し経路は
  決定論テスト（`follow_balloon_limit_tests.rs` ほか）の担当で、実機サインオフの対象ではない。
  実機で `limit=0` を見たければ fixture の 2 行を `0` へ書き換えて再走し、
  `Clamp` が 0 件になり**バルーンが画面外へはみ出したままになる**ことを確認すればよい。
- `cargo test --workspace` には本仕様と因果独立な既知間欠赤が 1 本ある
  （`areka-emo-atlas` の `warn_fires_on_all_transparent_element`・W6.9 `test-cage-determinism` 所有）。
  実機サインオフの判定には無関係。
