# ぱすたの実機の手順（何度でも回せる形）

ゴースト「窓際のぱすたさん」（以下ぱすた）を areka で起こし、使い方の流れを 7 段で回して、段ごとに SSP 2.9.18 での動きと比べる手順。この文書だけで、準備から片付けまでを辿れるように書く。

- 初版: 2026-10-22（spec `areka-P0-pasta-in-windows-verification` の最初の走行のために書いた）。
- 命令はすべて PowerShell 7（`pwsh`）で打つ。Git Bash では打たない（i686 のリンクが落ちる）。
- 命令の中のパスはすべて絶対パスで、先頭の 1 行 `$w = ...` だけを替えれば、どのフォルダから打っても同じに動く。

## 1. 何を確かめる手順か

- ぱすたが areka で「SSP 2.9.18 と同じレベル」で動くかを、7 段（下の 5 節の表）で確かめる。段ごとに、SSP での動き（基準）と areka の結果を並べ、「同じ」「違う」「回せなかった」のどれかを書く。
- 崩れる見込みの所の多くは、まだ着地していない兄弟の spec が引き取っている。表の「今の引き取る spec」の欄の spec が着地するたびに、その段の結果が変わるはずである。
- 「SSP と同じレベル」の最後の確かめは、兄弟の spec がすべて着地した後に、この手順をもう一度回して行う（v0.1.0 の版の門の最後の確かめ）。そのための別の spec は起こさない。
- 書庫の版は固定しない。毎回取り寄せから始め、いつも最新のぱすたで回す。回した書庫は `.sha256` の値で記録に書く。

## 2. 前に要るもの

| もの | なぜ | 揃わないとき |
|---|---|---|
| 拡大率の違う 2 つの画面（例: 200% と 150%） | 段 5（拡大率の違う画面への移動） | 段 5 を「回していない」と書く。合格にしない |
| SSP のぱすたを閉じてあること | ぱすたはデバッグの待受（127.0.0.1:9276）を開くので、SSP と areka で同時に起こすとぶつかる。起こすのは areka の 1 つのプロセスだけ | 開発者に SSP のぱすたを閉じてもらってから起こす |
| 開発者の SendInput の許可 | areka の窓はクリックが透けるので、撫で・クリック・ドラッグ・右クリックは OS の入力の送り込みで行う。許可はその走行のあいだだけ | 操作の段を回さない |
| 排他テストの机 | 画面を操作するので、ほかのセッションの実機の窓・負荷の走行と同じ時間に回さない。走行の区切りの中だけ持ち、終わったらすぐ返す | 机が取れるまで待つ |
| このセッションから areka の MCP が使えること | 段ごとの面・バルーン・記録を撮る（`tools/mcp-bridge-install.ps1` で中継を入れてある） | 画面の撮影（`shot`）だけで残し、その旨を記録に書く |

排他テストの机の取り方と返し方は `doc/impl-watch.md` の 4 節（呼び方の約束）に従う。取るのは次の形（`<識別>` は回すセッションの識別。`--wait` つきはバックグラウンドで打つ）。

```powershell
& "$env:AREKA_IMPL_WATCH_HOME\areka-impl-watch.exe" request --id <識別> --repo <リポジトリ> --desk test --purpose "ぱすたの実機の手順" --wait
```

目視の判定（重なりの順・見た目が同じか）は開発者が決める。AI は記録と撮った物を作るだけで、目視の項目を単独で「同じ」にしない。

## 3. 準備の命令

取り寄せ → 検体を開く → 根を組む → debug 版と i686 の補助 → 記憶を空に、の順。下の塊をそのまま 1 つの PowerShell に貼る。替えるのは先頭の 1 行だけ。走行ごとに回し直す（根と記憶を作り直すので、何度回しても同じ条件になる）。

```powershell
$w = 'C:\home\maz\git\areka'   # ここだけ、回すワークツリーの絶対パスに替える
$v = "$w\target\pasta-verify"

# 1. 取り寄せ（いつも最新を追う。置き場の書庫が最新の .sha256 と合わないときだけ取り直す）
pwsh -NoProfile -File "$w\tools\sample-fetch.ps1"; "fetch exit=$LASTEXITCODE"
"pasta sha256: " + ((Get-Content -LiteralPath "$w\target\nar-samples\fetched\pasta-in-windows.nar.sha256" -Raw).Trim() -split '\s+')[0]

# 2. 検体を開く（照合を通った書庫だけが target\nar-samples\manual\ の下へ開かれる）
function Open-Sample([string]$Name) {
    $lines = cargo run -q --locked --manifest-path "$w\Cargo.toml" -p sample-ghost-kit --bin nar-sample-path -- $Name
    if ($LASTEXITCODE -ne 0) { throw "nar-sample-path $Name exit=$LASTEXITCODE" }
    $h = @{}; foreach ($l in $lines) { $k, $val = $l -split '=', 2; $h[$k] = $val }; $h
}
$emo2 = Open-Sample 'emo2'
$staysee = Open-Sample 'StayseeBalloon'
$pasta = Open-Sample 'pasta-in-windows'

# 3. 根を組む（配布物と同じ形にぱすたのゴーストを足す）。記憶も空にする
Remove-Item -LiteralPath "$v\root", "$v\profile" -Recurse -Force -ErrorAction SilentlyContinue
$null = New-Item -ItemType Directory -Force -Path "$v\root\ghost", "$v\root\balloon", "$v\profile", "$v\logs", "$v\shots"
Copy-Item -LiteralPath $emo2['folder'] -Destination "$v\root\ghost\emo2" -Recurse
Copy-Item -LiteralPath $emo2['balloon.emo2-kakukaku'] -Destination "$v\root\balloon\emo2-kakukaku" -Recurse
Copy-Item -LiteralPath $staysee['folder'] -Destination "$v\root\balloon\StayseeBalloon" -Recurse
Copy-Item -LiteralPath $pasta['folder'] -Destination "$v\root\ghost\pasta-in-windows" -Recurse
Get-ChildItem -LiteralPath "$v\root\ghost", "$v\root\balloon" -Directory | Select-Object -ExpandProperty FullName

# 4. debug 版と i686 の補助（全体テストの後に x64 版へ替わっていることがあるので毎回確かめる）
cargo build --manifest-path "$w\Cargo.toml" -p areka; "build exit=$LASTEXITCODE"
cargo build --manifest-path "$w\Cargo.toml" -p shiori-host32-helper --target i686-pc-windows-msvc; "build i686 exit=$LASTEXITCODE"
pwsh -NoProfile -File "$w\tools\sync-i686-helper.ps1"; "sync exit=$LASTEXITCODE"
```

通った形:

- `fetch exit=0`・`build exit=0`・`build i686 exit=0`・`sync exit=0`（`sync` は `helper is i686: ...` か `placed i686 helper: ...` の行を出す）。
- 根の一覧が `ghost\emo2`・`ghost\pasta-in-windows`・`balloon\emo2-kakukaku`・`balloon\StayseeBalloon` の 4 つ。
- `nar-sample-path` が終了コード 3 で落ちたら、置き場の書庫か `.sha256` が無いか合わない。1 の取り寄せを回し直す。

置き場の形（根・記録・撮った物は、すべてワークツリーの `target\` の下。`C:\` の直下や `C:\tmp` は使わない）:

```text
<w>\target\pasta-verify\
├─ root\       走行の根（ghost\emo2・ghost\pasta-in-windows・balloon\emo2-kakukaku・balloon\StayseeBalloon）
├─ profile\    areka の記憶（走行ごとに空）
├─ logs\       <回>.stdout.log・<回>.stderr.log・<回>.pid.txt・<回>.exitcode.txt
├─ shots\<回>\ 撮った物
└─ ssp\        SSP で測ったときの撮った物（この手順では触らない）
```

各画面の位置（物理画素）と拡大率を読む。`mouse-input.ps1` に渡す座標はこの物理画素で書く。拡大率は画面ごとの DPI を 96 で割った値（`dpi=192` なら 200%）。どちらも記録の「機械の構成」に書く。

```powershell
pwsh -NoProfile -Command {
    Add-Type -Namespace W -Name U -MemberDefinition @'
[DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(System.IntPtr v);
[DllImport("user32.dll")] public static extern System.IntPtr MonitorFromPoint(System.Drawing.Point p, uint f);
[DllImport("shcore.dll")] public static extern int GetDpiForMonitor(System.IntPtr m, int t, out uint x, out uint y);
'@ -ReferencedAssemblies System.Drawing.Primitives
    $null = [W.U]::SetProcessDpiAwarenessContext([IntPtr]-4)
    Add-Type -AssemblyName System.Windows.Forms
    [System.Windows.Forms.Screen]::AllScreens | ForEach-Object {
        $m = [W.U]::MonitorFromPoint($_.Bounds.Location, 2); $x = 0; $y = 0
        $null = [W.U]::GetDpiForMonitor($m, 0, [ref]$x, [ref]$y)
        '{0} primary={1} bounds={2} dpi={3} scale={4}%' -f $_.DeviceName, $_.Primary, $_.Bounds, $x, ($x * 100 / 96)
    }
}
```

拡大率の違う 2 つの画面が無いときは、走らせる前に記録へ「段 5 は回していない」と書いておく。

## 4. 起動の命令

SSP のぱすたが閉じていること・SendInput の許可・排他テストの机を確かめてから打つ。準備と同じ PowerShell で打つ（`$w`・`$v` を使う）。`$run` は回ごとの名前（`run1`・`run2` …）。

この塊は areka が終わるまで戻らない（終わりを待って終了コードを書き出すため）。AI が回すときはバックグラウンドの呼び出しで打ち、段の操作は別の呼び出しで行う。

```powershell
$run = 'run1'
$null = New-Item -ItemType Directory -Force -Path "$v\logs", "$v\shots\$run"
Remove-Item Env:AREKA_NO_ALERT, Env:AREKA_USER_ROOT -ErrorAction SilentlyContinue
$env:AREKA_ROOT = "$v\root"
$env:AREKA_PROFILE_DIR = "$v\profile"
$env:RUST_LOG = 'info,areka::boot_resolve=debug,areka::install=debug,areka_nar=debug'
$env:AREKA_APP_SMOKE_EXIT_MS = '3600000'
$env:NO_COLOR = '1'
$p = Start-Process -FilePath "$w\target\debug\areka.exe" -ArgumentList @('--ghost', "`"$v\root\ghost\pasta-in-windows`"") -WorkingDirectory $v -RedirectStandardOutput "$v\logs\$run.stdout.log" -RedirectStandardError "$v\logs\$run.stderr.log" -PassThru
"areka pid=$($p.Id)" | Tee-Object -FilePath "$v\logs\$run.pid.txt"
$p.WaitForExit(); "areka exit=$($p.ExitCode)" | Tee-Object -FilePath "$v\logs\$run.exitcode.txt"
```

環境変数の意味:

| 名前 | 値 | 理由 |
|---|---|---|
| `AREKA_ROOT` | `<w>\target\pasta-verify\root` | 根 |
| `AREKA_PROFILE_DIR` | `<w>\target\pasta-verify\profile`（準備で毎回空にする） | 記憶の置き場。空なので、起動のイベントは `OnFirstBoot` になる |
| `RUST_LOG` | `info,areka::boot_resolve=debug,areka::install=debug,areka_nar=debug` | バルーンの決め方の分岐が記録に出る水準。段の判定に別の分岐が要るときは、その対象を足し、足した値を記録に書く |
| `AREKA_APP_SMOKE_EXIT_MS` | `3600000`（60 分） | 安全弁。7 段の所要（30〜45 分の見込み）より長く |
| `NO_COLOR` | `1` | 記録を色の制御文字の無い平文にする |
| `AREKA_NO_ALERT`・`AREKA_USER_ROOT` | 消す | 本物の利用者の置き場（`%LOCALAPPDATA%\areka`）に触らない |

- ゴーストは `--ghost` に絶対パスで渡して直に起こす。メニューからのインストールは通さない（7 段にインストールは無く、インストールの記録が段の判定に混ざらないように）。
- `areka pid=` の番号を控える。外から止めるのは、この番号のプロセスだけ（7 節）。
- areka が途中で止まったら、止まった時刻と記録の最後の行を控え、`$run` を `run1b` のように替えて起動の命令を打ち直し、次の段から続ける（記憶は空にしない。起こし直したことも記録に書く）。

### 補いの走り（`ghost-inner-balloon` の着地まで・任意）

`ghost-inner-balloon` が着地するまで、areka はゴーストのフォルダの中の縦書きのバルーンを名前で見つけられず、配布物の既定のバルーンで話す。縦書きのバルーンでしか見えない崩れ（列と枠の並び・縦書きの選択肢）を早く見るため、本走行の後に、縦書きのバルーンを根の `balloon\` に写した別の根で段 1〜2 だけを回してよい。結果は「参考（代わりの置き方）」として、段の 3 値の数と分けて書く。着地の後は要らない。

```powershell
$v = "$w\target\pasta-verify-alt"   # 置き場を本走行と分ける
Remove-Item -LiteralPath "$v\root", "$v\profile" -Recurse -Force -ErrorAction SilentlyContinue
$null = New-Item -ItemType Directory -Force -Path "$v\profile", "$v\logs", "$v\shots"
Copy-Item -LiteralPath "$w\target\pasta-verify\root" -Destination "$v\root" -Recurse
Copy-Item -LiteralPath "$v\root\ghost\pasta-in-windows\pasta_tategaki" -Destination "$v\root\balloon\pasta_tategaki" -Recurse
```

この後、同じ PowerShell で起動の命令を `$run = 'alt1'` にして打つ。

## 5. 7 段の表

段はこの順で回す。1 つの段が兄弟の spec の持ち分の崩れで回せないときは「回せなかった」と書いて次の段へ進む（1 つの崩れで走行全体を止めない）。

SSP の基準の欄は、SSP 2.9.18 での動きの要点。根拠（台本の原文・面の写し・撮った物）は測りの記録 `.kiro/specs/areka-P0-pasta-in-windows-verification/ssp-measurement.md` の「全体の通しの基準」の段の番号の項目にある。

| 段 | 操作 | 撮る物 | SSP の基準 | 今の引き取る spec | その着地で変わるはずの所 |
|---|---|---|---|---|---|
| 1 起動 | 起動の命令で起こし、最初の台詞が出終わるまで待つ | 画面の `shot`（重なりの順）・`dump_surface`（`\0` と `\1`）・`dump_balloon`・`get_log` | ぱすたが着せ替えの面 1000 で立ち、奥に窓の枠（`\1` の面 10）が別の窓で重なり、ゴーストのフォルダの中の縦書きのバルーンで台詞が出る。重なりは窓（奥）→ バルーン → ぱすた（手前）。SSP は記憶のある起動なので `OnBoot`、areka は空の記憶で `OnFirstBoot`（台詞の文の違いは起動イベントの違いとして分けて書く）（測りの記録 段 1） | `ghost-inner-balloon`（縦書きのバルーンを名前で見つける）・`descript-window-keys`（`sakura.seriko.defaultsurface`・`balloon.dontmove`）・`window-link-directives`（`seriko.sticky-window`）・`stay-on-top-directives`（`\1\v`）。重なりの指定は完了 `zorder-scope-inversion`（着地した結果として確かめる） | バルーンが既定のバルーンから縦書きのバルーンに替わる。最初の面が 1000 になる。窓の枠がぱすたに重なって付く。窓の枠が奥へ回る |
| 2 縦書きの会話 | 台詞の中の選択肢を 1 つ選び、選んだ後の会話が終わるまで待つ | 画面の `shot`（重なりの順・列と枠の並び）・`dump_balloon`・`get_log` | 縦書きのバルーンに、`\_l[x,y]` で並べた列と枠が崩れずに並び、選択肢が出て、選ぶと続きの会話が出る。重なりは段 1 と同じ（測りの記録 段 2） | `ghost-inner-balloon`（縦書きのバルーン）・`script-small-tags`（`\_u[0x3000]`・`\![bind-noevent]`・選択肢の前の `\![*]`） | 会話が縦書きで出る。全角の空白・選択肢の前の印・`bind-noevent` が SSP と同じに効く |
| 3 撫で・× に乗る・ボール | ぱすたを `move` で少しずつ撫でる。× の印の上に乗せる。ボールを `click` | 撫での前後と × に乗ったときの `shot`・`dump_surface`・`get_log`（`OnMouseMove`・`OnMouseEnter`・`OnMouseClick` の届き） | 撫でると反応の台詞が出る。× の印に乗ると × が赤くなる。ボールを押すとボールのトークが出る。当たり判定ごとにマウスの形が替わる（測りの記録 段 3） | `mouse-click-wheel-events`（`OnMouseClick`・`OnMouseEnter`／`OnMouseLeave`）・`shell-mouse-cursor`（`sakura.cursor` ブレス） | × が赤くなる。ボールのトークが出る。マウスの形が替わる |
| 4 2 つの窓を一緒にドラッグ | ぱすたを `drag` で同じ画面の中で動かして離す | ドラッグの前・後の `shot`・`get_log` | ぱすたと窓の枠が離れずに一緒に動き、落とした後も重なった形に付き直る（測りの記録 段 4） | `window-link-directives`（`seriko.sticky-window`・`--option=ignore-sticky-window`） | 窓の枠がぱすたについて動き、落とした後に付き直る |
| 5 拡大率の違う画面への移動 | ぱすたを `drag` で拡大率の違うもう 1 つの画面へ動かして離す | 移った後の `shot`・`dump_surface`・`get_log` | 設計・要件には SSP での見え方の要点を書いていない。測りの記録 段 5 を基準にする | `dpi-realign-remembered-chain`（拡大率の違う画面をまたぐドラッグ） | 移った先の画面でぱすたと窓の枠とバルーンの並びが保たれる。倍率の連動（`balloon.syncscale`・`OnShellScaling`）はこの手順の範囲の外なので、それに由来する見た目の違いは「違う」の数と分けて書く |
| 6 右クリックメニュー | ぱすたを `rightclick`。着せ替えのメニューを開き、1 つ選ぶ | メニューの `shot`・着せ替えの後の `shot` と `dump_surface`・`get_log` | 自前の色（`menu.*`。普通の Windows のメニューに見せる色）のメニューが出て、着せ替えのメニューから選ぶと着せ替えが替わる（測りの記録 段 6） | `popup-menu-shell-look`（`menu.*` の色と絵・`sakura.menu,auto`） | メニューの色と絵がシェルの指定どおりになる。着せ替えのメニューが出る |
| 7 × で終わる | × の印を `click`。効かなければ右クリックメニューの「終了」 | 終わりの台詞の `shot`・`get_log`・`<回>.exitcode.txt` | × を押すと終わりの台詞の後に終わる（測りの記録 段 7） | `mouse-click-wheel-events`（× の `OnMouseClick`） | × で終われる（今は右クリックメニューの「終了」で終える見込み）。どちらで終えたかと終了コードを書く。終了コードは 0 が合格 |

- 段 1 と段 2 では、重なりの順（窓が奥 → バルーン → ぱすたが手前）が見える範囲を `shot` で撮り、開発者が目視で決める。
- 段 6 でメニューが見えずに閉じたときは `popup-menu-residue` の件 12 に当たる。記録に書くだけで、直さない。
- 当たりの決まりの違い: areka は α が 0.5 以上（128 以上）の画素だけを当たりにし、SSP には合わせない（2026-10-22 開発者の裁定）。薄い影や縁を撫でても反応しない違いは、崩れでなく「方針で受けない」と書く。
- この手順の範囲の外（見た目が違っても「違う」の数に入れず、分けて書く）: `balloon.syncscale` と `OnShellScaling`／`OnBalloonScaling`・スクロールの矢印・`communicatebox`・`\![reload,shiori]`・禁則と縦中横・SSP の作りの癖。

## 6. 撮り方と数え方

### areka の MCP で撮る

最初に `get_active_ghost_list` でぱすたの名前を得て、以下の呼び出しには毎回その名前を渡す。

| 道具 | 使う所 |
|---|---|
| `get_status` | 起動が済んだか・走っているか |
| `dump_surface` | 面の写し（`\0` のぱすた・`\1` の窓の枠） |
| `dump_balloon` | バルーンの写し（どのバルーンか・列と枠の並び） |
| `raise_event` | 段の台本を起こして原文を取るとき |
| `get_log` | その段のあいだの記録 |

### 画面の操作と撮影（`tools/mouse-input.ps1`）

座標は 3 節で読んだ物理画素で書く。毎回、動かした後にカーソルの位置を読み戻して確かめ、狙いと違えば終了コード 1（ロックされた画面・スクリーンセーバーの中では送り込みが断られる）。撮った物は `$v\shots\$run\` の下に、段の番号から始まる名前で置く。

```powershell
pwsh -NoProfile -File "$w\tools\mouse-input.ps1" -Action move -X 1500 -Y 900
pwsh -NoProfile -File "$w\tools\mouse-input.ps1" -Action click -X 1500 -Y 900
pwsh -NoProfile -File "$w\tools\mouse-input.ps1" -Action rightclick -X 1500 -Y 900
pwsh -NoProfile -File "$w\tools\mouse-input.ps1" -Action drag -X 1500 -Y 900 -X2 -1200 -Y2 1000 -Steps 40
pwsh -NoProfile -File "$w\tools\mouse-input.ps1" -Action shot -X 0 -Y 0 -X2 2880 -Y2 1800 -Out "$v\shots\$run\1-boot-zorder.png"
```

- 撫では `move` を少しずつ違う点へ続けて打つ。
- 画面をまたぐドラッグも `drag` で送る（段 5）。
- 成功は終了コード 0 と `ok <動作> x=<X> y=<Y>`、引数の誤りは終了コード 2。

### 数え方

areka が終わった後に、同じ PowerShell で数える。

```powershell
$out = "$v\logs\$run.stdout.log"; $err = "$v\logs\$run.stderr.log"
"panicked: " + (Select-String -LiteralPath $out, $err -Pattern 'panicked at').Count
"ERROR: " + (Select-String -LiteralPath $out -Pattern '^\S+\s+ERROR ').Count
"WARN: " + (Select-String -LiteralPath $out -Pattern '^\S+\s+WARN ').Count
Select-String -LiteralPath $out -Pattern '^\S+\s+(ERROR|WARN) ' | Group-Object { ($_.Line -split '\s+')[1..2] -join ' ' } | Sort-Object Count -Descending | Format-Table Count, Name -AutoSize
Get-Content -LiteralPath "$v\logs\$run.exitcode.txt"
```

- panic: `panicked at` の行の数。致命: ERROR の行のうち、致命を名乗る行（areka を止めた行）の数。どちらも 7 段の全部のあいだで数える。
- ERROR と WARN は、記録の対象（モジュール）ごとに並べて読む。ぱすたの中身（SHIORI の返事・辞書）から出た行は、理由を書いて外し、外す前と外した後の両方の数を書く。
- 起こし直した回があるときは、その回の記録（`run1b.*` など）も足して数える。

## 7. 結果の書き方と片付け

### 結果の書き方

回した人の記録に書く（spec の中で回したときはその spec の検証の記録、最後の確かめでは版の門の記録）。

1. 同定: コミット（`git -C $w rev-parse --short HEAD`）・debug 版・書庫の `.sha256` の値（3 節の `pasta sha256:` の行）。
2. 機械の構成: OS の版・各画面の拡大率と位置（3 節）。
3. 環境変数と記録の水準（4 節の表。足した対象があればそれも）。
4. 走行: 起動の命令・回ごとの日時（日本時間）・流れ（記録の時刻つき）・段の表・ERROR と WARN の数（外した行と理由）・panic と致命の数・終了コード・撮った物の置き場。
5. 崩れた所の表: 番号・症状・根拠・原因（file:line か方針の裁定）・扱い（直した／方針で受けない／引き取る spec）・引き取る spec の名前か裁定の日付と出どころ。範囲の外の機能に由来する見た目の違いは別の表に書く。

段の表の型:

| 段 | SSP の基準 | areka の結果 | 根拠（撮った物の名前・記録の行の時刻つきの逐語） | 結果（同じ／違う／回せなかった） | 判定した人 | 引き取る spec | 範囲の外由来か |
|---|---|---|---|---|---|---|---|
| 1 起動 | 測りの記録 段 1 | | | | | | |

- 段 5 が拡大率の違う画面の無い机で回せなかったときだけ「回していない」と書く。
- 目視で決める項目の「判定した人」は開発者。

### 片付け

1. areka が終わっていることを確かめる。外から止める必要があるときは、控えた番号のプロセスが自分の起こした `areka.exe` であることを確かめてから止める。ほかの番号は止めない。

   ```powershell
   $id = [int]((Get-Content -LiteralPath "$v\logs\$run.pid.txt") -replace '\D', '')
   $q = Get-Process -Id $id -ErrorAction SilentlyContinue
   if ($q -and $q.Path -eq "$w\target\debug\areka.exe") { Stop-Process -Id $id } else { "not running or not ours: $id" }
   ```

2. 排他テストの机をすぐ返す: `& "$env:AREKA_IMPL_WATCH_HOME\areka-impl-watch.exe" return --id <識別> --desk test`。
3. SendInput の許可はこの走行で終わる。次の走行ではまた許可を得る。
4. 根・記憶・記録・撮った物（`<w>\target\pasta-verify\`）は消さずに残す。次の走行の準備の命令が根と記憶を作り直す。記録と撮った物は回ごとの名前で分かれる。
5. 開いた PowerShell を閉じる（4 節で設定した環境変数はその PowerShell の中だけのもの）。
