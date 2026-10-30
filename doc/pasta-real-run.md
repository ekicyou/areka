# ぱすたの実機の手順（何度でも回せる形）

ゴースト「窓際のぱすたさん」（以下ぱすた）を areka で起こし、使い方の流れを 7 段で回して、段ごとに SSP での動きと比べる手順。SSP の基準は、測りの記録（`ssp-measurement.md` の 1 節）に書いた版で測った値とする（初回の測りは 2026-10-23 の SSP 2.9.23。要件は 2.9.18 と書いていたが、開発者の機械の SSP が上がっていた）。この文書だけで、準備から片付けまでを辿れるように書く。

- 初版: 2026-10-22（spec `areka-P0-pasta-in-windows-verification` の最初の走行のために書いた）。
- 清書: 2026-10-23（同じ spec のタスク 8.1。最初の走行 run1・補いの走り alt1・alt2・直しの後の回し直し rerun1 で分かった命令の直しを戻した）。
- 命令はすべて PowerShell 7（`pwsh`）で打つ。Git Bash では打たない（i686 のリンクが落ちる）。
- 命令の中のパスはすべて絶対パスで、先頭の 1 行 `$w = ...` だけを替えれば、どのフォルダから打っても同じに動く。

## 1. 何を確かめる手順か

- ぱすたが areka で「SSP と同じレベル」で動くかを、7 段（下の 5 節の表）で確かめる。段ごとに、SSP での動き（基準）と areka の結果を並べ、「同じ」「違う」「回せなかった」のどれかを書く。
- 崩れる見込みの所の多くは、まだ着地していない兄弟の spec が引き取っている。表の「今の引き取る spec」の欄の spec が着地するたびに、その段の結果が変わるはずである。
- 「SSP と同じレベル」の最後の確かめは、兄弟の spec がすべて着地した後に、この手順をもう一度回して行う（v0.1.0 の版の門の最後の確かめ）。そのための別の spec は起こさない。
- 書庫の版は固定しない。毎回取り寄せから始め、いつも最新のぱすたで回す。回した書庫は `.sha256` の値で記録に書く。

## 2. 前に要るもの

| もの | なぜ | 揃わないとき |
|---|---|---|
| 拡大率の違う 2 つの画面（例: 200% と 150%） | 段 5（拡大率の違う画面への移動） | 段 5 を「回していない」と書く。合格にしない |
| ぱすたのデバッグの待受がぶつからないこと | 開発者の PC には利用者の環境変数 `PASTA_DEBUG=1`・`PASTA_DEBUG_PORT=9276` が在り、ぱすたで作ったゴーストはどれも 127.0.0.1:9276 で待ち受けようとする（配られた `pasta.toml` だけなら待受は開かない。2026-10-23「ぱすた開発」の答え）。4 節の起動の塊は、その PowerShell の中だけで `PASTA_DEBUG` を消し、areka のぱすたに待ち受けさせない。確かめ済み（2026-10-23・`target\pasta-verify-live\notes.md`）: `PASTA_DEBUG=1` のままの SSP が 9276 を持ち、この形で起こした areka は 9276 を開かなかった | 起動の塊を使わずに起こすときだけ、開発者に SSP のぱすたを閉じてもらう |
| 開発者の SendInput の許可 | areka の窓はクリックが透けるので、撫で・クリック・ドラッグ・右クリックは OS の入力の送り込みで行う。許可はその走行のあいだだけ | 操作の段を回さない |
| 排他テストの机 | 画面を操作するので、ほかのセッションの実機の窓・負荷の走行と同じ時間に回さない。走行の区切りの中だけ持ち、終わったらすぐ返す | 机が取れるまで待つ |
| このセッションから areka の MCP が使えること | 段ごとの面・バルーン・記録を撮る（`tools/mcp-bridge-install.ps1` で中継を入れてある）。SSP が先に 9801 を取っていると、areka の MCP の待受は 9802 になる（2026-10-23 の live1）。台本を送る前に、areka の MCP の道具が areka を指していることを、areka にしか無い `check_script` が答えることで確かめる | 画面の撮影（`shot`）だけで残し、その旨を記録に書く |

排他テストの机の取り方と返し方は `doc/impl-watch.md` の 4 節（呼び方の約束）に従う。識別（作業木の最上位のフォルダの名前）とリポジトリ（共通の git フォルダ `.git` の親の名前）は、3 節の準備の塊を貼った同じ PowerShell（先頭の `$w` を使う）で、下の 1 行目で求める。準備（取り寄せとビルド）は机が要らないので、机は 4 節の起動の前に取ればよい。`request` の前に、回すセッションの `watch` が立っていること（立っていなければ先に立てる）。`watch` と `--wait` つきはバックグラウンドで打つ。別の呼び出しで打つときは、`$id`・`$repo` を 1 行目が出した値に字のまま替える。

```powershell
$id = Split-Path -Leaf (git -C $w rev-parse --show-toplevel); $repo = Split-Path -Leaf (Split-Path -Parent (git -C $w rev-parse --path-format=absolute --git-common-dir)); "desk: id=$id repo=$repo"
& "$env:AREKA_IMPL_WATCH_HOME\areka-impl-watch.exe" watch --id $id --repo $repo   # 立っていないときだけ・バックグラウンド
& "$env:AREKA_IMPL_WATCH_HOME\areka-impl-watch.exe" request --id $id --repo $repo --desk test --purpose "ぱすたの実機の手順" --wait   # バックグラウンド
```

目視の判定（重なりの順・見た目が同じか）は開発者が決める。AI は記録と撮った物を作るだけで、目視の項目を単独で「同じ」にしない。

## 3. 準備の命令

取り寄せ → 検体を開く → 根を組む → debug 版と i686 の補助 → 記憶を空に、の順。下の塊をそのまま 1 つの PowerShell に貼る。替えるのは先頭の 1 行だけ。走行ごとに回し直す（根と記憶を作り直すので、何度回しても同じ条件になる）。根はいつも置き場の展開（`target\nar-samples\manual\` の下）から組み直すので、前の走行で areka とぱすたがゴーストのフォルダの中に作った記憶（`ghost\pasta-in-windows\ghost\master\profile\`）は残らない（取り寄せた書庫に `profile` のフォルダは無い）。

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
$arekaBalloon = Open-Sample 'areka-balloon'
$staysee = Open-Sample 'StayseeBalloon'
$pasta = Open-Sample 'pasta-in-windows'

# 3. 根を組む（配布物と同じ形にぱすたのゴーストを足す）。記憶も空にする
Remove-Item -LiteralPath "$v\root", "$v\profile" -Recurse -Force -ErrorAction SilentlyContinue
$null = New-Item -ItemType Directory -Force -Path "$v\root\ghost", "$v\root\balloon", "$v\profile", "$v\logs", "$v\shots"
Copy-Item -LiteralPath $emo2['folder'] -Destination "$v\root\ghost\emo2" -Recurse
Copy-Item -LiteralPath $emo2['balloon.emo2-kakukaku'] -Destination "$v\root\balloon\emo2-kakukaku" -Recurse
Copy-Item -LiteralPath $arekaBalloon['folder'] -Destination "$v\root\balloon\areka-balloon" -Recurse
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
- 根の一覧が `ghost\emo2`・`ghost\pasta-in-windows`・`balloon\areka-balloon`・`balloon\emo2-kakukaku`・`balloon\StayseeBalloon` の 5 つ（`areka-balloon` は既定のバルーン、`StayseeBalloon` は配布物に同梱する選べるバルーン）。
- `nar-sample-path` が終了コード 3 で落ちたら、置き場の書庫か `.sha256` が無いか合わない。1 の取り寄せを回し直す。

置き場の形（根・記録・撮った物は、すべてワークツリーの `target\` の下。`C:\` の直下や `C:\tmp` は使わない）:

```text
<w>\target\pasta-verify\
├─ root\       走行の根（ghost\emo2・ghost\pasta-in-windows・balloon\areka-balloon・balloon\emo2-kakukaku・balloon\StayseeBalloon）
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

2 節の前に要るもの（ぱすたの待受・SendInput の許可・排他テストの机）を確かめてから打つ。準備と同じ PowerShell で打つ（`$w`・`$v` を使う）。`$run` は回ごとの名前（`run1`・`run2` …）。

この塊は areka が終わるまで戻らない（終わりを待って終了コードを書き出すため）。AI が回すときはバックグラウンドの呼び出しで打ち、段の操作は別の呼び出しで行う。

```powershell
$run = 'run1'
$null = New-Item -ItemType Directory -Force -Path "$v\logs", "$v\shots\$run"
Remove-Item Env:AREKA_NO_ALERT, Env:AREKA_USER_ROOT -ErrorAction SilentlyContinue
Remove-Item Env:PASTA_DEBUG -ErrorAction SilentlyContinue   # ぱすたのデバッグの待受を開かせない（2 節）
$env:AREKA_ROOT = "$v\root"
$env:AREKA_PROFILE_DIR = "$v\profile"
$env:RUST_LOG = 'info,areka::boot_resolve=debug,areka::install=debug,areka_nar=debug,areka::input_events=debug'
$env:AREKA_APP_SMOKE_EXIT_MS = '3600000'
$env:NO_COLOR = '1'
$p = Start-Process -FilePath "$w\target\debug\areka.exe" -ArgumentList @('--ghost', "`"$v\root\ghost\pasta-in-windows`"") -WorkingDirectory $v -WindowStyle Hidden -RedirectStandardOutput "$v\logs\$run.stdout.log" -RedirectStandardError "$v\logs\$run.stderr.log" -PassThru
"areka pid=$($p.Id)" | Tee-Object -FilePath "$v\logs\$run.pid.txt"
$p.WaitForExit(); "areka exit=$($p.ExitCode)" | Tee-Object -FilePath "$v\logs\$run.exitcode.txt"
```

環境変数の意味:

| 名前 | 値 | 理由 |
|---|---|---|
| `AREKA_ROOT` | `<w>\target\pasta-verify\root` | 根 |
| `AREKA_PROFILE_DIR` | `<w>\target\pasta-verify\profile`（準備で毎回空にする） | 記憶の置き場。空なので、起動のイベントは `OnFirstBoot` になる |
| `RUST_LOG` | `info,areka::boot_resolve=debug,areka::install=debug,areka_nar=debug,areka::input_events=debug` | バルーンの決め方の分岐と、マウスの形が替わったこと（段 3）が記録に出る水準。段の判定に別の分岐が要るときは、その対象を足し、足した値を記録に書く |
| `AREKA_APP_SMOKE_EXIT_MS` | `3600000`（60 分） | 安全弁。7 段の所要（30〜45 分の見込み）より長く |
| `NO_COLOR` | `1` | 記録を色の制御文字の無い平文にする |
| `AREKA_NO_ALERT`・`AREKA_USER_ROOT` | 消す | 本物の利用者の置き場（`%LOCALAPPDATA%\areka`）に触らない |
| `PASTA_DEBUG` | 消す（その PowerShell の中だけ） | areka のぱすたにデバッグの待受（127.0.0.1:9276）を開かせず、SSP のぱすたとぶつからないようにする（2 節） |

- ゴーストは `--ghost` に絶対パスで渡して直に起こす。メニューからのインストールは通さない（7 段にインストールは無く、インストールの記録が段の判定に混ざらないように）。
- `Start-Process` の `-WindowStyle Hidden` は、areka.exe の空のコンソールの窓を出さないため（run1 では、この窓がぱすたの手前に開いて覆った）。
- `areka pid=` の番号を控える。外から止めるのは、この番号のプロセスだけ（7 節）。
- areka が途中で止まったら、止まった時刻と記録の最後の行を控え、`$run` を `run1b` のように替えて起動の命令を打ち直し、次の段から続ける（記憶は空にしない。起こし直したことも記録に書く）。

9276 の待受の持ち主を確かめたいときは、起動の後に別の PowerShell で次を打つ（2026-10-23 の live1 では、持ち主は SSP だけで、areka が開いたのは MCP の 9802 だけだった）。行が出ないか、出た行がどれも areka の側のプロセス（`areka.exe`・`shiori-host32-helper.exe`）でなければ、取り合っていない。

```powershell
Get-NetTCPConnection -LocalPort 9276 -State Listen -ErrorAction SilentlyContinue | ForEach-Object { 'listen 9276 pid={0} path={1}' -f $_.OwningProcess, (Get-Process -Id $_.OwningProcess -ErrorAction SilentlyContinue).Path }
```

### 補いの走り（ぱすたの新しい書庫まで・任意）

取り寄せる書庫（10-20 版）は `install.txt` にバルーンの鍵が無く、縦書きのバルーンはゴーストのフォルダの直下（`ghost\pasta-in-windows\pasta_tategaki`）に置かれたままになる。areka も SSP もそこをバルーンとして探さないので、配布物の既定のバルーンで話す（ぱすたの側が `install.txt` を直した・次のリリースの書庫から入る）。縦書きのバルーンでしか見えない崩れ（列と枠の並び・縦書きの選択肢）を早く見るため、本走行の後に、縦書きのバルーンを根の `balloon\` に写した別の根で段 1〜2 だけを回してよい。結果は「参考（代わりの置き方）」として、段の 3 値の数と分けて書く。直した書庫で回すときは要らない。

本走行の根を写すと、本走行で areka とぱすたがゴーストのフォルダの中に作った記憶（`ghost\master\profile\`）まで写る。そのままだと起動が `OnBoot`、バルーンが記憶のバルーン（本走行で使った既定の `areka-balloon`。2026-10 までは `StayseeBalloon`）になり、縦書きのバルーンで `OnFirstBoot` を見られない（alt1）。下の塊の最後の行で、写した記憶を消す。

```powershell
$v = "$w\target\pasta-verify-alt"   # 置き場を本走行と分ける
Remove-Item -LiteralPath "$v\root", "$v\profile" -Recurse -Force -ErrorAction SilentlyContinue
$null = New-Item -ItemType Directory -Force -Path "$v\profile", "$v\logs", "$v\shots"
Copy-Item -LiteralPath "$w\target\pasta-verify\root" -Destination "$v\root" -Recurse
Copy-Item -LiteralPath "$v\root\ghost\pasta-in-windows\pasta_tategaki" -Destination "$v\root\balloon\pasta_tategaki" -Recurse
Remove-Item -LiteralPath "$v\root\ghost\pasta-in-windows\ghost\master\profile" -Recurse -Force -ErrorAction SilentlyContinue   # 本走行が作ったゴーストの記憶を写さない
```

この後、同じ PowerShell で起動の命令を `$run = 'alt1'` にして打つ。

## 5. 7 段の表

段はこの順で回す。1 つの段が兄弟の spec の持ち分の崩れで回せないときは「回せなかった」と書いて次の段へ進む（1 つの崩れで走行全体を止めない）。

SSP の基準の欄は、SSP での動きの要点（初回の測りは SSP 2.9.23）。根拠（台本の原文・面の写し・撮った物）は測りの記録 `.kiro/specs/completed/areka-P0-pasta-in-windows-verification/ssp-measurement.md` の「全体の通しの基準」の段の番号の項目にある。

| 段 | 操作 | 撮る物 | SSP の基準 | 今の引き取る spec | その着地で変わるはずの所 |
|---|---|---|---|---|---|
| 1 起動 | 起動の命令で起こし、最初の台詞が出終わるまで待つ | 画面の `shot`（重なりの順）・`dump_surface`（`\0` と `\1`）・`dump_balloon`・`get_log` | ぱすたが着せ替えの面 1000 で立ち、奥に窓の枠（`\1` の面 10）が別の窓で重なり、縦書きのバルーンで台詞が出る（測った SSP の縦書きのバルーンは、SSP の根の `balloon\` に置いた写しから出た。配布の書庫〔10-20 版〕は `install.txt` にバルーンの鍵が無く、入れただけでは縦書きのバルーンが入らない。ぱすたの側が直した・未リリース）。重なりは窓（奥）→ バルーン → ぱすた（手前）。SSP の基準は、ゴーストをぱすたへ替えて SSP を起こし直した起動で測ったので `OnBoot`（2026-10-23 07:59:49 に測った。台本の頭は `\0\s[1000]\1\s[10]`）。areka は空の記憶で `OnFirstBoot`（台詞の文の違いは起動イベントの違いとして分けて書く）（測りの記録 段 1） | ぱすたの側（書庫の `install.txt` に縦書きのバルーンの鍵が無い・直した・未リリース）・`descript-window-keys`（`sakura.seriko.defaultsurface`・`balloon.dontmove`）・`stay-on-top-directives`（`\1\v`・`\![set,windowstate,!stayontop]`）・`script-small-tags`（`\_u[0x3000]`）・`mouse-click-wheel-events`（初回の台本がボールのクリックを待つので、身体が映らない）・`mcp-scope-rect-property`（位置を `get_property` で読む・着地済み・確かめる）・`animated-surface-log-flood`（記録が `apply(ShowSurface)` の行で埋もれる）。重なりの指定は完了 `zorder-scope-inversion`（着地した結果として確かめる）。着地済み: `window-link-directives`（`seriko.sticky-window`・main の PR#360）。起動の並べ直しで窓の枠が別の画面へ送られた件も #360 が直した（台本の `\![move]` が置いた窓の印 `ScriptPlaced`）。どちらも着地した結果として確かめる | バルーンが既定のバルーンから縦書きのバルーンに替わる。最初の面が 1000 になる。ぱすたの窓が画面の下端を越えない。窓の枠がぱすたに重なって付く（#360 で着地済み・確かめる）。窓の枠が奥へ回る。ボールのクリックで身体が映る。位置を `get_property` で読める（着地済み・確かめる） |
| 2 縦書きの会話 | `raise_event` で `選択肢テスト` を起こし、出た選択肢を 1 つ選び、選んだ後の会話が終わるまで待つ（30 秒の時間切れに注意・表の下の注） | 画面の `shot`（重なりの順・列と枠の並び）・`dump_balloon`・`get_log` | 縦書きのバルーンに、`\_l[x,y]` で並べた列と枠が崩れずに並び、選択肢が出て、選ぶと続きの会話が出る。重なりは段 1 と同じ（測りの記録 段 2） | ぱすたの側（縦書きのバルーン・段 1 と同じ）・`script-small-tags`（`\_u[0x3000]`・`\![bind-noevent]`・選択肢の前の `\![*]`）・`vertical-choice-hover-band`（縦書きの選択肢の反転の帯） | 会話が縦書きで出る。全角の空白・選択肢の前の印・`bind-noevent` が SSP と同じに効く。縦書きの選択肢の反転の帯が列に合う |
| 3 撫で・× に乗る・ボール | ぱすたを `move` で少しずつ撫でる。× の印の上に乗せる。ボールを `click` | 撫での前後と × に乗ったときの `shot`・`dump_surface`・`get_log`（`OnMouseMove`・`OnMouseEnter`・`OnMouseClick` の届き）・マウスの形を `cursor` で読んだ行（表の下の「段 3 のマウスの形の読み方」） | 撫でると反応の台詞が出る。× の印に乗ると × が赤くなる。ボールを押すとボールのトークが出る。当たり判定ごとにマウスの形が替わる（測りの記録 段 3） | `mouse-click-wheel-events`（`OnMouseClick`・`OnMouseEnter`／`OnMouseLeave`）・`shell-mouse-cursor`（`sakura.cursor` ブレス） | × が赤くなる。ボールのトークが出る。マウスの形が替わる |
| 4 2 つの窓を一緒にドラッグ | ぱすたを `drag` で同じ画面の中で動かして離す | ドラッグの前・後の `shot`・`get_log` | ぱすたと窓の枠が離れずに一緒に動き、落とした後も重なった形に付き直る（測りの記録 段 4） | `mcp-scope-rect-property`（位置を `get_property` で読む・着地済み・確かめる）。着地済み: `window-link-directives`（`seriko.sticky-window`・`--option=ignore-sticky-window`・main の PR#360。組の相手をドラッグと `\![move]` で連れて動かす） | 窓の枠がぱすたについて動き、落とした後に付き直る（#360 で着地済み・確かめる。付いてこなければ起票する） |
| 5 拡大率の違う画面への移動 | ぱすたを `drag` で拡大率の違うもう 1 つの画面へ動かして離す | 移った後の `shot`・`dump_surface`・`get_log` | 200% の画面へ移っても大きさは変わらない（831x1026 のまま・`scaling` は 150,150）。ぱすたと窓の枠は同じ rect のまま重なって並び、バルーンはぱすたの左上から 60,87 の所のまま。移動に伴うイベント（`OnShellScaling` など）は届かない（測りの記録 段 5） | `dpi-realign-remembered-chain`（拡大率の違う画面をまたぐドラッグ・落とした点のずれ）・`mcp-scope-rect-property`（位置を `get_property` で読む・着地済み・確かめる）・`shell-scale-linkage`（倍率の連動。この手順の範囲の外）。着地済み: `window-link-directives`（窓の枠が付いてくる・main の PR#360） | 移った先の画面でぱすたと窓の枠とバルーンの並びが保たれ、落とした点に置かれる（窓の枠が付いてくることは #360 で着地済み・確かめる）。大きさが画面の拡大率に合わせて変わるのは areka の方針（DPI 追従・2026-07-18 開発者）で、「方針で受けない」と書く。倍率の連動（`balloon.syncscale`・`OnShellScaling`）はこの手順の範囲の外なので、それに由来する見た目の違いは「違う」の数と分けて書く |
| 6 右クリックメニュー | ぱすたを `rightclick`。着せ替えのメニューを開き、1 つ選ぶ | メニューの `shot`・着せ替えの後の `shot` と `dump_surface`・`get_log` | 自前の色（`menu.*`。普通の Windows のメニューに見せる色）のメニューが出て、着せ替えのメニューから選ぶと着せ替えが替わる（測りの記録 段 6） | `popup-menu-shell-look`（`menu.*` の色と絵・`sakura.menu,auto`） | メニューの色と絵がシェルの指定どおりになる。着せ替えのメニューが出る |
| 7 × で終わる | × の印を `click`。効かなければ右クリックメニューの「終了」 | 終わりの台詞の `shot`・`get_log`・`<回>.exitcode.txt` | × を押すと終わりの台詞の後に終わる（測りの記録 段 7） | `mouse-click-wheel-events`（× の `OnMouseClick`） | × で終われる（今は右クリックメニューの「終了」で終える見込み）。どちらで終えたかと終了コードを書く。終了コードは 0 が合格 |

- 段 1 と段 2 では、重なりの順（窓が奥 → バルーン → ぱすたが手前）が見える範囲を `shot` で撮り、開発者が目視で決める。
- 段 2 の選択肢は、表示が終わってから 30 秒で `OnChoiceTimeout` になって消える。`raise_event` の前に、カーソルを選ぶ選択肢の出る位置へ `move` で置いておき、選択肢が出たら 30 秒のうちに `click` する（alt2 の 1 回目は 4 秒遅れて時間切れになった）。
- 段 3 のマウスの形は、表の下の「段 3 のマウスの形の読み方」の手順で読む。
- 段 6 でメニューが見えずに閉じたときは `popup-menu-residue` の件 12 に当たる。記録に書くだけで、直さない。
- 当たりの決まりの違い: areka は α が 0.5 以上（128 以上）の画素だけを当たりにし、SSP には合わせない（2026-10-22 開発者の裁定）。薄い影や縁を撫でても反応しない違いは、崩れでなく「方針で受けない」と書く。
- この手順の範囲の外（見た目が違っても「違う」の数に入れず、分けて書く）: `balloon.syncscale` と `OnShellScaling`／`OnBalloonScaling`（引き取る spec `shell-scale-linkage`。`\![get,property]` で聞く倍率と DPI も同じ）・縦書きのバルーンのスクロールの矢印（`balloon-scroll-arrows`）・`communicatebox`（`balloon-communicatebox`）・`\![reload,shiori]`・禁則と縦中横・SSP の作りの癖。

### 段 3 のマウスの形の読み方

マウスの形は目で見ず、`tools/mouse-input.ps1` の `cursor` で読む（6 節）。`cursor` は点へ動かして位置を読み戻した後、今のマウスの形を Windows の標準の形 14 種と比べて、`ok cursor x=<X> y=<Y> shape=<名前|custom> handle=0x<16進>` の 1 行を出す。標準の形のどれでもなければ `custom`。SSP の測り（`.kiro/specs/completed/shell-mouse-cursor/research.md` の 8 節）と同じ読み方。

点はぱすたの窓の位置（6 節の「位置の読み方」）から、測りの記録の点を左上からの差で写して決める。

```powershell
pwsh -NoProfile -File "$w\tools\mouse-input.ps1" -Action cursor -X <身体の点の X> -Y <身体の点の Y>
pwsh -NoProfile -File "$w\tools\mouse-input.ps1" -Action cursor -X <× の点の X> -Y <× の点の Y>
pwsh -NoProfile -File "$w\tools\mouse-input.ps1" -Action cursor -X <ボールの点の X> -Y <ボールの点の Y>
pwsh -NoProfile -File "$w\tools\mouse-input.ps1" -Action cursor -X <白地の点の X> -Y <白地の点の Y>
```

| 点 | 期待の `shape` | わけ |
|---|---|---|
| ぱすたの身体 | `custom` | シェルの指定（`system:hand`）で、areka 同梱の手を開いた形が出る |
| × の印 | `HAND` | 指定の無い当たり判定は指差しの形（SSP と同じ） |
| ボール | `HAND` | × と同じ |
| 当たり判定の無い窓の白地 | `ARROW` | 当たり判定の外は矢印 |

押したときと放したときの形:

- 身体の上で左を押すと指差しの形（`HAND`）、放すと手を開いた形（`custom`）に戻る。ぱすたのシェルには押したときの行（`mousedown`）が無く、当たり判定の名前も `bust` ではないので、押したときは指差しの形になる（要件 3.1。握った形は、当たり判定の名前が `bust` のときか、押したときの行があるときだけ出る）。`cursor` に `-Press` を付けると、点へ動かして左を押し、押したままの形を読んで ` pressed=1` の付いた行を出し、左を放してもう一度読んだ行を出す（読むのに失敗しても、左は必ず放す）。

```powershell
pwsh -NoProfile -File "$w\tools\mouse-input.ps1" -Action cursor -X <身体の点の X> -Y <身体の点の Y> -Press
```

- 1 行目（`pressed=1`）が `shape=HAND`（指差しの形）で、`handle` が手を開いた形の値と違うこと。2 行目が `shape=custom`（手を開いた形）で、`handle` が押す前に同じ点で読んだ値と同じであること。
- 2 行目は放した直後に、動かさずに読んだ形。手を開いた形に戻っていること。
- 記録の `areka::input_events` の行でも、押したときと放したときに形が替わったことを見て、2 つ目の証跡にする。
- ぱすたのシェルには止まったときとホイールのときの行が無いので、止めてもホイールを回しても形は替わらない（SSP と同じ）。

記録: 起動の命令の `RUST_LOG` に `areka::input_events=debug` を入れてあるので、形が替わったときの行が `<回>.stderr.log` に出る。読んだ `ok cursor` の行と一緒に段 3 の結果へ写す。

### 最後の確かめで決める項目（案 B）

最初の走行で開発者の判定が付かなかった 7 項目（`.kiro/specs/completed/areka-P0-pasta-in-windows-verification/verification.md` の 6 節「申し送りの漏れが無いことの確かめ」の判定待ちの 6 欄と、5 節の崩れの 15）と、ぱすたの側が直した崩れの 1・2 の確かめの、合わせて 8 項目。v0.1.0 の版の門の最後の確かめで、書いた spec の着地の後に回し、開発者が「同じ／違う」を決める。「違う」になったら、その時に v0.1.0 の持ち主か新しい起票へ渡す。

| 項目 | 見比べる物 | 決める時 |
|---|---|---|
| シェル `seriko.alignmenttodesktop`（`free`）・`sakura.defaultx`（`0`） | 段 1 の起動の位置（手で動かす前の `shot`）。SSP の基準は記憶のある起動（`OnBoot`）の位置なので、起動の種類の違いを分けて書く | `descript-window-keys` の着地の後 |
| シェル `sakura.balloon.offsetx`（`476`）・`offsety`（`58`） | 段 1・段 2 のバルーンのぱすたからのずれ（SSP は等倍で 40,58） | ぱすたの新しい書庫の後（縦書きのバルーンが手当てなしで出る形で見る） |
| リソース `sakura.recommendsites` | 段 6 のメニュー（崩れ 15 と一緒に見る） | `popup-menu-shell-look` の着地の後 |
| バルーン `anchor.font.color.*` | リンク（`\_a`）の字の色。7 段のどれにもリンクの字が出なければ「見た目・振る舞いに効かない」へ移す | 兄弟の spec の着地の後の回し直しで |
| `collision*` | 段 3 の、顔を撫でたときの当たり（身体が映る起動で見る） | `mouse-click-wheel-events` の着地の後 |
| `\![move,…]` | 段 1 の窓の枠（`\1`）の位置（手で動かす前の `shot`。`\0` の左上に重なって出るか） | 着地済みの `window-link-directives`（main の PR#360）の上で、次の回し直しで |
| 右クリックメニューの見た目（崩れ 15） | 段 6 のメニューの `shot` を SSP の `stage6-menu-shot.png` と比べる | `popup-menu-shell-look` の着地の後 |
| 縦書きのバルーンが書庫から入って出るか（崩れ 1・2） | 新しい書庫（`install.txt` に `balloon.directory,pasta_tategaki`・ghost_dev の `c32229e`）を入れて段 1・段 2 を回す。縦書きのバルーンで起動し（`balloon_resolved route=Descript`・`descript_balloon_not_found` が 0）、`\_l[x,0]` の後の段と選択肢が前の行に重ならないこと。入れ方は表の下 | ぱすたの新しい書庫のリリースの後（リリースは開発者が決める） |

崩れ 1・2 の確かめの準備: 3 節の根を組む塊は書庫を写すだけで `install.txt` を読まない。新しいぱすたの書庫は、areka 自身のインストールで入れる（3 節の根で起こした areka のゴーストの窓へ `.nar` を投げ込む）。同梱のバルーンは、main #364 の決めのとおりゴーストの中の `ghost\pasta-in-windows\balloon\pasta_tategaki\` に入る。この入れ方は案 B の前に実機で確かめる。

### 代わりの操作（`mouse-click-wheel-events` の着地まで・任意）

`mouse-click-wheel-events` が着地するまで、areka はボールのクリック（`OnMouseClick`）をぱすたへ送らない。ぱすたの初回の起動の台本（`OnFirstBoot`）は最後に `\0\s[1002]`（ボールと × だけの面）を出してボールのクリックを待つので、身体が最後まで映らない。身体と窓の枠を見るために、段 1 を撮り終えた後に、areka の MCP の `sakurascript` で次の台本を送ってよい（2026-10-23 開発者の承認の案 A）。

```text
\0\s[1000]\1\s[10]
```

- `ghost_name` は `get_active_ghost_list` で得た名前。送る前に `check_script` で確かめる。
- 段 1 の判定は、代わりの操作の前の姿で決める。代わりの操作の後の結果（撮った物の名前は `1s-subst-` から始める）は「代わりの操作」として別に書き、段の 3 値の数には入れない。
- `mouse-click-wheel-events` の着地の後は要らない（ボールのクリックで身体が映る）。この小節ごと外す。

## 6. 撮り方と数え方

### areka の MCP で撮る

最初に `get_active_ghost_list` でぱすたの名前を得て、以下の呼び出しには毎回その名前を渡す。

| 道具 | 使う所 |
|---|---|
| `get_status` | 起動が済んだか・走っているか |
| `dump_surface` | 面の写し（`\0` のぱすた・`\1` の窓の枠） |
| `dump_balloon` | バルーンの写し（どのバルーンか・列と枠の並び） |
| `raise_event` | 段の台本を起こして原文を取るとき |
| `get_property` | 窓の位置（表の下の「位置の読み方」） |
| `get_log` | その段のあいだの記録 |
| `sakurascript`・`check_script` | 5 節の代わりの操作 |

位置の読み方: 窓の位置は `get_property` で読み、測りの記録の rect と比べる。`currentghost.scope(0).rect` がぱすた、`currentghost.scope(1).rect` が窓の枠、`currentghost.balloon.scope(0).rect` がぱすたのバルーン。答えは「左,上,右,下」の物理の画素で、右と下はその画素を含まない。窓の無い番号は、`currentghost.scope(n).rect` が `0,0,0,0` で、`currentghost.balloon.scope(n).rect` が値なし（`NG`）で答える。

### 画面の操作と撮影（`tools/mouse-input.ps1`）

座標は 3 節で読んだ物理画素で書く。毎回、動かした後にカーソルの位置を読み戻して確かめ、狙いと違えば終了コード 1（ロックされた画面・スクリーンセーバーの中では送り込みが断られる）。撮った物は `$v\shots\$run\` の下に、段の番号から始まる名前で置く。

```powershell
pwsh -NoProfile -File "$w\tools\mouse-input.ps1" -Action move -X 1500 -Y 900
pwsh -NoProfile -File "$w\tools\mouse-input.ps1" -Action click -X 1500 -Y 900
pwsh -NoProfile -File "$w\tools\mouse-input.ps1" -Action rightclick -X 1500 -Y 900
pwsh -NoProfile -File "$w\tools\mouse-input.ps1" -Action drag -X 1500 -Y 900 -X2 -1200 -Y2 1000 -Steps 40
pwsh -NoProfile -File "$w\tools\mouse-input.ps1" -Action shot -X 0 -Y 0 -X2 2880 -Y2 1800 -Out "$v\shots\$run\1-boot-zorder.png"
pwsh -NoProfile -File "$w\tools\mouse-input.ps1" -Action cursor -X 1500 -Y 900
```

- 撫では `move` を少しずつ違う点へ続けて打つ。
- 画面をまたぐドラッグも `drag` で送る（段 5）。
- マウスの形は `cursor` で読む（段 3）。
- 成功は終了コード 0 と `ok <動作> x=<X> y=<Y>`（`cursor` は後ろに `shape=<名前|custom> handle=0x<16進>` が付く。`-Press` のときは ` pressed=1` の付いた押したままの行と、放した後の行の 2 行）、引数の誤りは終了コード 2。

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
- `animated-surface-log-flood` が着地するまで、記録の 9 割が `apply(ShowSurface)` の INFO 行になる。記録を目で読むときは `Select-String -LiteralPath $out -Pattern 'apply\(ShowSurface\)' -NotMatch` で外して読む（数えるのは ERROR と WARN だけなので、上の数には効かない）。

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
   $apid = [int]([regex]::Match((Get-Content -LiteralPath "$v\logs\$run.pid.txt" -Raw), 'pid=(\d+)').Groups[1].Value)
   $q = Get-Process -Id $apid -ErrorAction SilentlyContinue
   if ($q -and $q.Path -eq "$w\target\debug\areka.exe") { Stop-Process -Id $apid } else { "not running or not ours: $apid" }
   ```

2. 排他テストの机をすぐ返す: `& "$env:AREKA_IMPL_WATCH_HOME\areka-impl-watch.exe" return --id $id --desk test`（`$id` は 2 節の 1 行目の値）。
3. SendInput の許可はこの走行で終わる。次の走行ではまた許可を得る。
4. 根・記憶・記録・撮った物（`<w>\target\pasta-verify\`）は消さずに残す。次の走行の準備の命令が根と記憶を作り直す。記録と撮った物は回ごとの名前で分かれる。
5. 開いた PowerShell を閉じる（4 節で設定した環境変数はその PowerShell の中だけのもの）。
