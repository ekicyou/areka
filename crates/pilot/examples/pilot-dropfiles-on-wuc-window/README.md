# 先進坑: pilot-dropfiles-on-wuc-window

> この README は先進坑の**一次記録（正本）**である。本坑 spec の design はここの検証結果を参照し、
> 同じ結果を二重化しない。
> go／違う／直す の判定は開発者（人間）がこの README を見て下す。Claude Code は判定を書かない。
> 成果物はコードではなく知見（go／違う／直す＋学び）である。コードは使い捨てで、本坑へ写さない。

## 動機（なぜ掘るか）

- 対応する本坑 spec: `areka-P0-ghost-install`。本坑はこの先進坑の go 判定を前提依存（`_Depends(confirmed): pilot-dropfiles-on-wuc-window`）として持つ。
- 何が怪しいか: α の一周は「`.nar` をゴーストの窓へ落とす」から始まる。ところが areka のゴースト窓は WUC で合成し（`WS_EX_NOREDIRECTIONBITMAP`）、クリック透過のために `WS_EX_TRANSPARENT` を付けたり外したりしている。そういう窓に、エクスプローラからの落とし物のメッセージ `WM_DROPFILES` が本当に届くかは、まだ誰も測っていない（2026-09-26 時点で `WM_DROPFILES`／`DragAcceptFiles`／`DragQueryFile`／`IDropTarget` は `crates/` のこの先進坑の外に 0 件）。
- 届かなかったときに変わること: 本坑は「投げ込みは `WM_DROPFILES` で受ける」前提で設計する予定である。届かなければ、本坑の設計は OLE の受け口 `IDropTarget` へ変わる。`IDropTarget` は受け口を登録するスレッドが STA（`OleInitialize` 済み）であることを求めるが、wintf の UI スレッドは WUC のために MTA（`COINIT_MULTITHREADED`）で動いているので、そのままでは登録できない。この衝突をどこで解くかが本坑の設計の中心になってしまう。本坑の要件を書く前に、ここで答えを出す。
- 確かめたい 1 点: **本番と同じ拡張スタイルで建てた WUC 合成・クリック透過つきの窓へ、エクスプローラから `.nar` を落としたとき、落とし物のメッセージが届いてファイルのパスが取れるか。絵の外（透過している所）へ落としたときは、窓に届かず背後の窓へ抜けるか**。
- 合否基準（要件より転記）:
  - **go**: 絵の上で落とすと落とし物のメッセージが届き、ファイルのパスが取れる。絵の外では届かない（背後の窓へ抜ける）。
  - **直す**: 届くが条件つき（例: 透過の付け外しの時機・受け入れの宣言の付け方）で、本坑の中で直せる。直し方をこの README に書く。絵の上では届くが絵の外でも窓が受け取ってしまう（背後の窓へ抜けない）場合もここに入れる（本坑は絵の外の落とし物を捨て、背後へ抜けないことを既知の制限とする）。
  - 「違う」に入れるのは絵の上で届かない場合だけ。絵の外で抜けないことは `IDropTarget` へ倒しても直らない（どの窓へ落とすかは OS が透過のビットで決め、受け口の方式に依らない）ため。
  - **違う**: どう組んでも届かない。`IDropTarget` へ倒す。その場合の STA の置き場所の見立ても学びに書く。

## 概要（何を作ったか）

### 作った物

コードは `crates/pilot/examples/pilot-dropfiles-on-wuc-window/` の下だけにある（`crates/pilot/Cargo.toml` も他の crate も変えていない）。

| ファイル | 役割 |
|---|---|
| `main.rs` | 起動と終了。窓 1 枚（本番のゴースト窓の様式＋受け入れの宣言）と赤い矩形 1 つを建て、窓が作られた直後に「素の拡張スタイルの読み戻し → クリック透過の機構への登録 → 手当て → 受け口の設置」を行い、上限時間で窓を消して終わる。 |
| `dropfiles.rs` | 受け口。窓へ `SetWindowSubclass` で手続きを重ね、`WM_DROPFILES` だけを受けて位置・絵の上か外か・到着時の拡張スタイル・ファイル数・各パスをログへ出し、必ず `DragFinish` で片付ける。拡張スタイルの読み戻し・手当ての切替・管理者の判定もここ。 |

窓と絵:

- 窓は枠なしのポップアップ（`WS_POPUP | WS_VISIBLE`）で、拡張スタイルは本番の `WS_EX_LAYERED | WS_EX_TOOLWINDOW` に受け入れの宣言 `WS_EX_ACCEPTFILES` を足しただけ。`WS_EX_LAYERED` を外して `WS_EX_NOREDIRECTIONBITMAP` を足すのと、クリック透過の `WS_EX_TRANSPARENT` の付け外しは wintf に任せている。最前面にはしない（本番に無いので）。ドラッグで動かすこともできない。
- 窓は画面の (160,160)（物理 px）に出る。大きさは 320×320（論理 px）で、窓そのものには当たりが無い（透明な余白）。その中に不透明な赤い矩形が 1 つ、左上 (100,100)・120×120（論理 px）で置いてある。拡大率 200% なら窓は 640×640・矩形は 240×240（物理 px）で、余白は各辺 200 物理 px ある。**この矩形が「絵の上」、余白が「絵の外」**。
- 生成直後の読み戻しの一例（タスク 3.1 の短い走行で見た行・拡大率 200%）: `[dropfiles] ex-style when="created" accept_files=true transparent=false layered=false noredirect=true raw="0x200090"`。つまり受け入れの宣言は窓の生成を通って残っていた。この直後にクリック透過の機構が `clickthrough: WS_EX_LAYERED 同伴フラグ適用` で `WS_EX_LAYERED` を付け直すので、到着時（`when="drop"`）の行では `layered=true` になっていても不思議ではない。実際の検証の結果は下の「検証結果」に書く。

### 実行法

```sh
cargo run -p pilot --example pilot-dropfiles-on-wuc-window
```

- x64 専用（wintf は i686 で組めないので、`crates/pilot/Cargo.toml` の dev-dependencies は x86 を除いてある）。
- 部品の単体テスト: `cargo test -p pilot --example pilot-dropfiles-on-wuc-window`。
- 終了コード: **0**＝上限時間に到達して終了（この example の正常な終わり方）／**2**＝初期化の失敗／**3**＝窓が外から閉じられた（Alt＋F4 など）。3 の走行は判定に使わない。

環境変数:

| 名前 | 既定 | 意味 |
|---|---|---|
| `AREKA_APP_SMOKE_EXIT_MS` | `180000`（180 秒） | 上限時間（ミリ秒）。未設定・空は既定を使い、数でない値は警告を出して既定を使う。上限に達すると `[dropfiles] 終了: 上限時間に到達` を出して自分で終わる。 |
| `PILOT_DROPFILES_FIX` | 未設定（手当てなし） | 受け入れの宣言の手当て。`reapply`＝窓の生成後に `GWL_EXSTYLE` へ `WS_EX_ACCEPTFILES` を付け直して `SWP_FRAMECHANGED`。`dragaccept`＝`DragAcceptFiles(hwnd, true)` で宣言する。それ以外の値は警告を出して手当てなしで走る。手当てを適用すると `ex-style when="fix"` の行がもう 1 行出る。 |
| `RUST_LOG` | 未設定なら `info,wintf::ecs::clickthrough=debug` | ログの水準。**自分で設定すると既定の `wintf::ecs::clickthrough=debug` が消え、透過の切り替えの行（`clickthrough: ex-style トグル適用`）が見えなくなる**。設定するなら必ず `info,wintf::ecs::clickthrough=debug` を含める。 |
| `NO_COLOR` | 未設定 | `1` にするとログの色付けを止める。**パイプやファイルへ流しても色の制御文字は付いたまま**なので、ログを保存して検索するときは `NO_COLOR=1` にする（色付きだと `accept_files=false` のような語で検索しても当たらない）。 |

- `PILOT_DROPFILES_FIX=reapply` が意味を持つのは、生成直後の `ex-style when="created"` の行で `accept_files=false` だったときだけ。`accept_files=true` ならビットは既に立っているので、何も変えない手当てになる。
- 手当ては 1 走行に 1 種。試すときは環境変数を変えて起動し直す。

ログの読み方（1 行の形）:

| 行 | 出るとき | 主な項目 |
|---|---|---|
| `[dropfiles] 起動` | 起動時に 1 回 | `admin`（管理者として動いているか）・`fix`（`None`／`Reapply`／`DragAccept`）・`limit_ms` |
| `[dropfiles] 窓を生成` | 起動時に 1 回 | `window`・`x`・`y` |
| `[dropfiles] ex-style` | 生成直後（`when="created"`）・手当ての後（`when="fix"`）・到着ごと（`when="drop"`） | `accept_files`・`transparent`・`layered`・`noredirect`・`raw`（生の値） |
| `[dropfiles] 受け口を設置` | 生成直後に 1 回 | `entity`・`hwnd` |
| `clickthrough: ex-style トグル適用` | 透過が付いた・外れたとき（wintf の debug の行） | `desired=Transparent`（透過を付けた）／`desired=Opaque`（外した） |
| `[dropfiles] 到着` | 落とし物のメッセージが届くたび | `seq`（通し番号）・`x`／`y`（窓の中の位置・物理 px）・`in_client`・`opaque`（`"true"`＝絵の上／`"false"`＝絵の外／`"unknown"`＝判定できなかった）・`transparent`・`accept_files`・`layered`・`noredirect`・`n`（ファイル数） |
| `[dropfiles] ファイル` | 届いたファイル 1 本ごと | `seq`・`i`・`path` |
| `[dropfiles] 終了: 上限時間に到達` | 上限時間で終わるとき | `drops`（受け取った回数）・`limit_ms` |
| `[dropfiles] 終了コード` | 最後に 1 回 | `reason`・`code` |

- 1 回の到着につき、`ex-style when="drop"` → `到着` → `ファイル`×n の順で続けて出る。
- `到着` の行の `transparent`・`accept_files` などは**到着した時点で読んだ値**である。落とした瞬間から届くまでの数 ms の間に、カーソルの監視（12ms 周期）が透過を付け外しすることはありうる。
- `opaque="unknown"` は、到着の場で wintf の World を借りられなかった（tick の途中で同期に届いた、または終了処理の途中）ことを示す。**`unknown` を「同期に届く経路がある」の証拠と読めるのは、`[dropfiles] 終了` の行より前に出たときだけ**。終了処理は World を借りたまま待ち行列を配送するので、終了の瞬間に落としたものは同期でなくても `unknown` になる。
- 失敗の行も同じ目印で出る（`[dropfiles] 手当ての適用に失敗`・`[dropfiles] 受け口の設置に失敗`・`[dropfiles] パスの取り出しに失敗`・`[dropfiles] 受け口の中で panic — 片付けて続行`）。受け口の設置に失敗した走行は無効として扱う。

ログを保存して検索する例（ログは標準出力に出る）:

```powershell
# PowerShell（日本語が化けるなら 1 行目で出力の文字コードを UTF-8 にする）
[Console]::OutputEncoding = [Text.Encoding]::UTF8
$env:NO_COLOR = '1'
cargo run -p pilot --example pilot-dropfiles-on-wuc-window | Tee-Object -FilePath dropfiles.log
Select-String -Path dropfiles.log -Pattern '\[dropfiles\]|clickthrough: ex-style'
```

```sh
# bash
NO_COLOR=1 cargo run -p pilot --example pilot-dropfiles-on-wuc-window | tee dropfiles.log
grep -E '\[dropfiles\]|clickthrough: ex-style' dropfiles.log
```

### 検証の準備

1. **写した `.nar` を使う**。捨ててよい作業用フォルダを 2 つ作る（例: 落とす側 `drop-src`・背後の受け手 `drop-dst`）。`drop-src` へ `.nar` を **2 本**写す（例: リポジトリの `vendors/sample_ghost/StayseeBalloon.nar` を `a.nar`・`b.nar` として写す）。元のファイルは使わない。同じドライブのフォルダの窓へ落とすと既定で**移動**になるので、ⓑで 1 本が `drop-dst` へ移る。ⓒにはもう 1 本を使う。
2. **管理者でないターミナル**から走らせる。起動の行が `admin=false` であることを確かめる。`admin=true` の走行は判定に使わず、ふつうの権限で走り直す（管理者で動かしたときの手当ては扱わない）。
3. 画面の拡大率は変えなくてよい（開発機の 200% のまま）。
4. エクスプローラの窓を 2 枚開いて置く:
   - **落とす側（`drop-src`）の窓は、先進坑の窓と重ならない所に置く**。ドラッグを始めるためにクリックした瞬間にその窓が前面へ来て、ドラッグ中は先進坑の窓を前に出せないため。
   - **背後の受け手（`drop-dst`）の窓は、赤い矩形が `drop-dst` の窓の真ん中あたりに重なるように広げて置く**。先進坑の窓は矩形よりひと回り大きい正方形だが、矩形のまわりは透明で見えない（これを「余白」と呼ぶ。矩形の各辺から外へ矩形の幅の 8 割ほど）。矩形のまわりに `drop-dst` の空のファイル一覧が見えていればよい。
5. example を起動する。**窓がエクスプローラに隠れたら、赤い矩形をクリックして前に出す**（先進坑の窓は最前面ではない）。起動直後に `clickthrough: ex-style トグル適用 … desired=Transparent` が 1 行出る（カーソルが絵の外にあるので透過が付いた）。
6. 既定の 180 秒のうちにⓐ〜ⓒを 1 走行で行う。途中で窓を閉じない（閉じると終了コード 3 で、その走行は判定に使わない）。

### 手順ⓐ〜ⓒと期待する行

| 手順 | 落とし方 | 期待する行（go の場合） |
|---|---|---|
| ⓐ 絵の上 | `drop-src` の `a.nar` をつかみ、先進坑の赤い矩形の上で離す（わざと矩形の内外を出し入れしない）。 | `ex-style when="drop"` に続いて `[dropfiles] 到着 seq=1 … in_client=true opaque="true" transparent=false accept_files=true … n=1`、続けて `[dropfiles] ファイル seq=1 i=0 path=…\a.nar`。位置 `x`・`y` は矩形の中（論理 100〜220 × 拡大率。200% なら物理 px で 200〜440）。 |
| ⓑ 絵の外 | ⓐで使った `drop-src` の `a.nar` をもう一度つかみ、赤い矩形のすぐ外側、辺から矩形の幅の半分くらい離れた所（見えない余白の内側で、下に `drop-dst` の窓が見えている所）で離す。離れすぎると余白の外に出て、ただ `drop-dst` に落としただけになる。 | **`[dropfiles] 到着` の行が出ない**。`a.nar` が `drop-dst` の窓へ移った（または写った）ことを目で確かめる。 |
| ⓒ 付け外しの後に絵の上 | 先にカーソルを矩形の内外へ数回出し入れし、`clickthrough: ex-style トグル適用` の行が `desired=Opaque`／`desired=Transparent` で数回出たことを確かめる。それから `drop-src` の `b.nar` を矩形の上で離す。 | `[dropfiles] 到着 seq=2 … opaque="true" transparent=false accept_files=true …` と `[dropfiles] ファイル seq=2 i=0 path=…\b.nar`。**`accept_files=true` が、付け外しの後も受け入れの宣言が残っている証拠**。 |

- 絵の上に落としても**画面の上では何も起きない**（ファイルも動かない）。この先進坑は受け取ったことをログに書くだけなので、結果はログの行で確かめる。
- ⓐは「起動時の 1 回と、矩形へ入るときの付け外ししか起きていない」状態、ⓒは「わざと何度も付け外しさせた後」の状態で、透過の付け外しの前後で受け取りが変わるかを比べる。
- 上限時間が来ると `[dropfiles] 終了: 上限時間に到達 drops=…` が出る。ⓐ〜ⓒを行えば `drops=2` のはず（ⓑは数えられない）。
- ⓐかⓒで届かなかったら、`PILOT_DROPFILES_FIX=reapply` と `PILOT_DROPFILES_FIX=dragaccept` の走行を 1 本ずつ足し、同じ手順で試す。それでも届かないときに次に足すのは、スレッドのメッセージ取得フック（`SetWindowsHookExW(WH_GETMESSAGE, …)`）で「そもそも待ち行列に `WM_DROPFILES` が来ているか」を切り分けることである（今回のコードには入れていない）。

## 検証結果

- 判定（go／違う／直す）: **go**（2026-09-26・開発者）
- 日付: 2026-09-26（走行 1＝22:40〜22:43・走行 2＝22:45〜22:48、いずれも日本時間）

### 見立ての表（判定ではない）

実走の結果を次の表に当てはめて見立てを書く。判定は開発者が下す。

| 観測 | 見立て |
|---|---|
| ⓐ・ⓒで届きパスが取れ、ⓑで届かず背後へ渡る | **go** |
| 手当て（`reapply`／`dragaccept`）のどれかで届くようになった | **直す**（効いた手当てを書く） |
| ⓐ・ⓒで届くがⓑで窓が受け取る（背後へ抜けない） | **直す**（本坑は落ちた位置を当たり判定にかけ、絵の外なら捨てる。背後へ抜けないことを既知の制限として書く） |
| 手当てをすべて試してもⓐかⓒで届かない | **違う**（`IDropTarget` へ倒す。STA の置き場所の見立ては下記） |

「違う」になったときの STA の置き場所の見立て（試作はしない・`.kiro/specs/pilot-dropfiles-on-wuc-window/research.md` §6 より）:

1. **UI スレッドを STA にする**: `WinApp` の初期化を `COINIT_APARTMENTTHREADED` へ変える。WUC は STA でも動く道がある（`crates/wintf/src/com/wuc.rs` の注記）が、WIC の背景復号（MTA 前提）と `crates/areka-emo-*` の `CoInitializeEx(COINIT_MULTITHREADED)` 呼び出し群の見直しが要る＝変更が広い。
2. **STA の別スレッドに受け口の窓を置く**: ゴースト窓と同じ位置・大きさに透明な受け口の窓を重ね、`RegisterDragDrop` はそのスレッドで行う。重なり順・クリック透過の付け外しとの二重管理・DPI への追従が新たに要る＝複雑。
3. **`WM_DROPFILES` で行けるところまで行く**: `IDropTarget` が要るのはテキスト・URL の投げ込み（α の後）なので、α の `.nar` は `WM_DROPFILES` で足りる、という切り分け。この先進坑が go なら本坑はこの線。

「違う」のときは、1〜3 のどれが `areka-P0-ghost-install` の規模に収まるかの見立てまでを学びに書く。

見立て: **go**。ⓐ・ⓒで届いてパスが取れ、ⓑでは窓に届かず背後のエクスプローラの窓へ渡った（表の 1 行目）。手当ては要らなかった。本坑 `areka-P0-ghost-install` は上の 3 の線（`WM_DROPFILES` で `.nar` を受ける）で設計できる見込み。

### 走行の条件

- コマンドと環境変数: `NO_COLOR=1 cargo run -q -p pilot --example pilot-dropfiles-on-wuc-window`（`AREKA_APP_SMOKE_EXIT_MS`・`PILOT_DROPFILES_FIX`・`RUST_LOG` は未設定＝上限 180 秒・手当てなし・既定のフィルタ）。管理者でないシェルから 2 走行。走行 1 はⓐだけ（ⓑの位置取りの説明が分かりにくく、2 回目も絵の上に落ちた）、走行 2 でⓐ〜ⓒを通した。下の表は走行 2 の値。
- 画面の拡大率: 200%（開発機のまま）
- 起動の行（`admin=false` の確認）: 両走行とも `[dropfiles] 起動 admin=false fix=None limit_ms=180000`
- 生成直後の `ex-style when="created"` の行: 両走行とも `accept_files=true transparent=false layered=false noredirect=true raw="0x200090"`。直後に wintf が `clickthrough: WS_EX_LAYERED 同伴フラグ適用` を出し、以後の到着時の値は `raw="0x280090"`（`layered=true`）
- 終了の行と終了コード: 走行 1＝`[dropfiles] 終了: 上限時間に到達 drops=2 limit_ms=180000`・走行 2＝`… drops=3 …`。どちらも `終了コード reason=Deadline code=0`（プロセスの終了コード 0）

### ⓐ〜ⓒの結果

| 手順 | 届いたか | 取れたパス | 位置（`x`,`y`）・`opaque` | 到着時の `transparent` | 到着時の `accept_files` |
|---|---|---|---|---|---|
| ⓐ 絵の上 | 届いた（22:45:15・`seq=1`・`n=1`） | `C:\Users\maz-o\Downloads\x\drop-src\a.nar` | `433`,`336`（矩形は物理 px で 200〜440）・`opaque="true"` | `false` | `true` |
| ⓑ 絵の外 | 届かなかった（`到着` の行なし） | —（背後の `drop-dst` の窓へ渡った。`a.nar` が 22:45:26.74 に `drop-dst` へ移動。その直前 22:45:25.97 に `desired=Transparent`） | — | — | — |
| ⓒ 付け外しの後に絵の上 | 届いた（22:46:04・`seq=2`／22:46:16・`seq=3`） | `C:\Users\maz-o\Downloads\x\drop-src\b.nar`（2 回とも） | `393`,`385`／`404`,`403`・どちらも `opaque="true"` | `false` | `true` |

- ⓒの前に出た `clickthrough: ex-style トグル適用` の行の数: `seq=1` から `seq=2` までに 18 行（ⓑのドラッグを含む）、`seq=2` から `seq=3` までにさらに 18 行
- 走行 1 も同じ形で 2 回届いた（`seq=1` `x=326 y=277`・`seq=2` `x=332 y=320`、どちらも `a.nar`・`opaque="true"`・`transparent=false`・`accept_files=true`）

### 試した手当てと結果

- 手当てなし（既定）: ⓐ〜ⓒすべて期待どおり
- `PILOT_DROPFILES_FIX=reapply`: 走らせていない（届かない項が無かった。生成直後に `accept_files=true` なので、この手当ては何も変えない）
- `PILOT_DROPFILES_FIX=dragaccept`: 走らせていない（届かない項が無かった）

### 学び

- 落とし物のメッセージは wintf の窓手続きまで届いたか（`SetWindowSubclass` で重ねた手続きで受けられた＝本坑は wintf の窓手続きの振り分けの表に `WM_DROPFILES` の 1 分岐を足すだけで足りるか）: **届いた**。`WM_DROPFILES` は窓スレッドの待ち行列経由で、`SetWindowSubclass` で重ねた手続きに 5 回とも届いた（`opaque` が 5 回とも判定できた＝tick の途中の同期配送ではない）。本坑は wintf の窓手続きの振り分け（`dispatch_window_message`）に 1 分岐足せば足りる見込み。宣言は `WS_EX_ACCEPTFILES` のビットだけで足り、`DragAcceptFiles` は要らなかった。
- 透過の付け外しとの噛み合い（付け外しの前後で受け取りが変わったか・受け入れの宣言が付け外しの後も残ったか・ドラッグ中に透過が追随したか）: 変わらなかった。`seq=1` から数えて 18 行・36 行の付け外しの後も到着時の `accept_files=true`（wintf の付け外しは `WS_EX_TRANSPARENT` と `WS_EX_LAYERED` 以外のビットを保つ）。ドラッグ中もカーソル監視は動き続け、絵の上に来ると `desired=Opaque`・外へ出ると `desired=Transparent` が出た。そのため落とした瞬間の透過の状態は落とした位置と一致し、絵の上の到着は 5 回とも `transparent=false`、絵の外では窓に届かなかった。
- `opaque="unknown"` が出たか。出たなら `[dropfiles] 終了` の行より前か後か: 出なかった（両走行とも 0 件）
- 終了時の警告の有無（重ねた手続きの後片付けで警告やエラーの行が出たか）: なし（両走行とも `WARN`・`ERROR` の行 0 件。`[App] Last window closed.` → `終了コード … code=0` まで通常どおり）
- 判定の方式（矩形か α マスクか）が結果に効かない理由（設計の見込み: どの窓へ落とすかを決めるのは OS で、OS が見るのは `WS_EX_TRANSPARENT` のビットだけ。当たり判定の方式は wintf の中でそのビットを決める段の話）: 裏付けあり。ⓑの落としは直前に `desired=Transparent` が出た状態で背後の窓へ渡り、ⓐ・ⓒの到着はどれも `transparent=false` で、wintf の当たり判定（`opaque="true"`）と一致した。落とし先はビットの状態で決まり、そのビットを矩形で決めるか α で決めるかは結果に関わらない。
- 本坑への注意: ドラッグ中の付け外しはカーソル監視の周期（12ms）に追随する。絵の縁ぎりぎりに素早く落とすと、ビットが切り替わる前の状態で落とし先が決まる可能性はある（今回は縁から 7 物理 px 内側の `x=433` でも正しく届いた）。
