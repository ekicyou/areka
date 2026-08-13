# 実機サインオフ 実行手順書（タスク 6.2）

この手順で S1〜S4 を判定し、結果を同じディレクトリの `signoff.md` に判定表として書き出す。
判定表が合格で確定すると、本仕様は `/kiro-complete` へ進める状態になる。

**判定は「所感」ではなくログか目視の一次事実で行う。** 迷ったら FAIL ではなく「判定不能」として、
何が足りなかったかを書くこと。判定不能のまま先へ進めるより差し戻すほうが安い。

---

## 0. 前提

### 必要なもの

- **拡大率の異なる 2 台のディスプレイ**。S1 は 2 画面間の往復そのものが観測対象なので、
  同じ拡大率の 2 台では意味を持たない。
- ゴースト窓をクリック・ドラッグできる操作環境（DPI 仮想化が挟まると測定が濁るので実機推奨）。
- S4 用に**常時最前面にできるアプリ**を 1 つ（タスクマネージャーの「常に手前に表示」など）。

### ビルド ★ 32bit ヘルパを忘れないこと

```bash
cargo build -p areka
cargo build -p shiori-host32-helper --target i686-pc-windows-msvc
cp target/i686-pc-windows-msvc/debug/shiori-host32-helper.exe target/debug/
```

`areka.exe` は**自分と同じディレクトリ**の `shiori-host32-helper.exe` を起動する。これが無いと
SHIORI が接続できず `OnFirstBoot` が来ないため、キャラ窓に出すサーフェスが決まらない。
バルーン窓は背景画像を伺かパッケージから直接読むので、**「バルーンしか表示されない」**という
症状になる。タスク 4 でこれを踏み、危うく重なり保証の不成立と誤判定するところだった。

`release` ではなく `debug` を使う（`release` はコンソールを持たずログ判定ができない）。

### 起動コマンド（雛形）

fixture は**必ず絶対パス**で渡す。相対パスだと `pasta.dll` の LOAD が `0x8007007E` で落ちる。

```bash
AREKA_APP_SMOKE_EXIT_MS=300000 RUST_LOG=info,wintf::ecs::window::zorder_pair=debug,areka=debug ./target/debug/areka.exe "C:/home/maz/git/areka/.claude/worktrees/ghost-window-zorder-0055fb/crates/pilot/examples/shiori-host-32/fixtures/emo2" "C:/home/maz/git/areka/.claude/worktrees/ghost-window-zorder-0055fb/crates/pilot/examples/shiori-host-32/fixtures/emo2/emo2-kakukaku" > signoff.out 2> signoff.err
```

- `AREKA_APP_SMOKE_EXIT_MS` は**必ず付ける**（有界自動終了）。S1〜S4 を 1 回の起動で通すなら
  300000（5 分）程度を見ておく。
- ログは必ずファイルへ落とす。端末のスクロールバックで判定しないこと。
- S1〜S4 は**同じ 1 回の起動**で通してよい。分ける場合はそれぞれのログを別ファイルにする。

---

## 1. ログの読み方

### 起動直後に出る並び

```bash
grep '\[zorder-pair\]' signoff.out | head -20
```

```
[zorder-pair] strategy-selected plan=A mechanism=owner-link raise_assist=false
[zorder-pair] declared scope=0 char_entity=<A> balloon_entity=<B>
[zorder-pair] declared scope=1 char_entity=<C> balloon_entity=<D>
[zorder-pair] owner-established entity=<B> peer=<A> owned_hwnd=… owner_hwnd=… measured_prev=…
[zorder-pair] owner-established entity=<D> peer=<C> owned_hwnd=… owner_hwnd=… measured_prev=…
[zorder-pair] fix   entity=… peer=… insert_after=… measured_next_after_fix=…
[zorder-pair] skip  entity=… peer=… reason=AlreadyAdjacent
```

**`strategy-selected` が `plan=A mechanism=owner-link raise_assist=false` であることを最初に確かめる。**
ここが違っていれば以降の判定は別の構成を測っていることになる。

### 2 段 grep（scope ↔ entity の対応）

`scope` は wintf 側のレコードに載らない。`declared`（areka 側）で scope ↔ entity を引いてから、
wintf 側のレコードを entity で突き合わせる。まずこの対応表を作ること。

| scope | キャラ entity | バルーン entity | キャラ HWND | バルーン HWND |
|---|---|---|---|---|
| 0 | | | | |
| 1 | | | | |

HWND は `owner-established` の `owned_hwnd`（バルーン）と `owner_hwnd`（キャラ）から引ける。

### ★ 案 A では「キャラをクリックしても `fix` は出ない」

**タスク 4 の手順書に書いてあった「各キャラ窓を活性化して `fix` を出させる」は、案 A では成立しない。**
隣接の維持は OS の owner 保証が担っており、キャラ窓のクリックで再断行の要求を挿す供給者が
居ないためである（重なり変化の検知は案 B ／補助浮上でのみ結線する設計）。**指令が出ないことが正常。**

よって本サインオフの肯定的証跡は **`sink-observed` の `adjacency_ok`** で採る。

```
[zorder-pair] sink-observed entity=<B> adjacency_ok=true foreground=0x… behind_foreground=true
```

`adjacency_ok` は隣接の正準判定（手前側の最も近い可視の背後が背後側か）そのものであり、
実測値に基づく。**両スコープについて記録が揃うこと**を確かめる（entity 欄で見分ける）。

`sink-observed` は自窓が非活性化された巡に目印を付け、**次の巡で実測する**（活性化の途中で
測ると実装が正しくても偽の失敗を記録するため）。したがって記録を出させるには
**他アプリかもう一方のスコープを活性化する**必要がある。

### 隣接は「最も近い可視の隣」で測る

不可視の窓（スレッド既定の IME 窓など）は読み飛ばして判定する。実装がすでにそうなっているので、
ログの実測値をそのまま信じてよい。

---

## 2. 判定項目

### S1 拡大率の異なる 2 画面の間でキャラ窓を往復させる（要件 7.2／7.3）

**やること**: キャラ窓をドラッグして画面 A → 画面 B → 画面 A と往復させる。
**各移動のあとに、他アプリを 1 回活性化する**（`sink-observed` を出させるため）。
両スコープのキャラ窓それぞれについて行う。

**PASS 基準**:

- 各移動後の `sink-observed` が **`adjacency_ok=true`**。両スコープとも。
- `verify-failed` が 1 本も出ていない。
- 目視で、移動後もバルーンがキャラの手前に居て、追従が壊れていない。

**判定式**:

```bash
grep '\[zorder-pair\] sink-observed' signoff.out
grep -c '\[zorder-pair\] verify-failed' signoff.out   # 0 であること
```

---

### S2 バルーン窓を操作してもキャラ窓が埋もれず、位置も動かない（要件 7.4）

**やること**: 他のアプリを活性化してゴーストを背面へ回してから、**バルーン窓をクリックする**。
続けてバルーン窓を単独でドラッグする。

**PASS 基準**:

- バルーン窓だけでなく**キャラ窓も一緒に手前へ出る**（キャラ窓が他アプリの背後に残らない）。要件 1.3。
- **キャラ窓の位置が動いていない**。バルーンの操作でキャラが移動してはならない。
- 上記のあと他アプリを活性化すると `sink-observed` が `adjacency_ok=true` で出る。

**判定式**（キャラ窓が動いていないことの機械判定）:

窓の移動は `[diag.window_move]` に `route= entity= kind= scope= x= y= w= h= dpi=` の形で出る。
`kind=char` の行だけを抜き、**バルーン操作の前後で座標が変わっていないこと**を見る。

```bash
grep '\[diag.window_move\]' signoff.out | grep 'kind=char'
```

バルーンを掴んだ区間に `kind=char` の座標変化が現れたら FAIL。バルーンの追従による
`kind=balloon` の行は正常なので混同しないこと。

---

### S3 他アプリの活性化でゴースト一式が背面かつ隣接を保つ（要件 4.1／4.2／4.4／7.5）

**やること**: メモ帳などを活性化し、ゴースト一式を背面へ回す。数回繰り返す。
**もう一方のスコープのキャラ窓を活性化する**ケースも含めること（沈む側の観測が採れる）。

**PASS 基準**: 非活性化ごとの最後の観測記録で

- `behind_foreground=true`（ゴーストの窓が前面の窓より背面）
- `adjacency_ok=true`（ペアの隣接が保たれている）

が**両スコープについて**揃う。

**判定式**:

```bash
grep '\[zorder-pair\] sink-observed' signoff.out | grep -c 'adjacency_ok=true'
grep '\[zorder-pair\] sink-observed' signoff.out | grep -c 'adjacency_ok=false'   # 0 であること
grep '\[zorder-pair\] sink-observed' signoff.out | grep 'behind_foreground=false'
```

**★ `behind_foreground=false` を機械的に FAIL と数えてはならない。** `foreground=` 欄を見ること。

- `foreground` が**そのペア自身のキャラ窓**なら**正常**である。バルーンは自分のキャラのすぐ手前に
  居るのだから、自分のキャラより背面であってはならない——`true` だったら要件 1.1 の違反になる。
  この行は不変条件が立っていることの証跡として数える。
- `foreground` が**他アプリ**の窓なのに `false` なら**それが FAIL** である（沈んでいない）。

---

### S4 常時最前面の窓が居てもバルーンが帯へ入らない（要件 8.1・タスク 6.4 の是正）★新規

**この項目はタスク 6.4 の是正を実機で踏ませるためのものである。** 決定論的テストでは
自前の窓で作った帯しか試せていない。

**やること**:

1. **常時最前面のアプリを 1 つ出す**（タスクマネージャーを開き「オプション → 常に手前に表示」）。
   ゴーストと重ならない位置に置いてよい。
2. 他アプリを活性化してゴーストを背面へ回す。
3. **キャラ窓をクリックして手前へ出す**——このとき、キャラ窓は「可視の通常窓のうち最前面」に
   なる。すなわちキャラ窓の最も近い可視の手前は**常時最前面の窓**になり、6.4 の分岐が踏まれうる。
4. さらに、バルーン窓を単独でドラッグしてから同じことを繰り返す（再断行の契機を作るため）。

**PASS 基準**:

- **バルーン窓に常時最前面の印が付いていない。**
  判定は Alt+Tab や他アプリの重なりで見るのが確実——**常時最前面のアプリを動かして
  バルーン窓の上に重ねたとき、常時最前面アプリのほうが手前に来ること**。
  バルーンが常時最前面アプリより手前に居座ったら FAIL。
- `sink-observed` の `adjacency_ok=true` が保たれている。
- `verify-failed` が出ていない。

**もし `fix` が出た場合**は `insert_after=` の欄を見る。`top-of-normal-band` と出ていれば
6.4 の分岐が実際に踏まれた証跡である（`top-edge` は「キャラより手前に可視の窓が無かった」場合で
別物）。**出ていなくても FAIL ではない**——案 A では是正の契機が乏しく、踏まれないことのほうが多い。

**FAIL の帰結**: 6.4 の是正が実機で効いていないことになる。判定表に記録し、差し戻す。

---

## 3. 判定表の書き出し

`verification/signoff.md` に以下の形で記録する。**根拠ログの実際の行を貼ること**
（「PASS した」だけでは後から再検証できない）。

```markdown
# 実機サインオフ 判定表（タスク 6.2）

- 実施日:
- 実施環境: ディスプレイ 2 台（拡大率 __% / __%）・解像度 __ / __
- ビルド: debug / コミット <hash>（＋ i686 ヘルパ）
- fixture: emo2（emo2-kakukaku）・2 スコープ

## scope ↔ entity ↔ HWND の対応

| scope | キャラ entity | バルーン entity | キャラ HWND | バルーン HWND |
|---|---|---|---|---|

## 判定

| # | 判定項目 | 結果 | 根拠 |
|---|---|---|---|
| S1 | 2 画面往復で隣接を保つ | PASS / FAIL / 判定不能 | |
| S2 | バルーン操作でキャラが埋もれず位置も不動 | | |
| S3 | 他アプリ活性化で背面かつ隣接 | | |
| S4 | 常時最前面窓が居ても帯へ入らない | | |

- `strategy-selected` の実測:
- `sink-observed` 総数 / `adjacency_ok=true` の数:
- `verify-failed` の数（0 であること）:
- ERROR の総数:

## 根拠ログ（抜粋）
```

---

## 4. 既知の落とし穴

- **32bit ヘルパを建て忘れると「バルーンしか見えない」。** 重なりとは無関係の症状である。
  実機で「見えない」を観測したら、まず ERROR 行を見ること。
- **`skip AlreadyAdjacent` を肯定的証跡として数えない。** このレコードは実測値を持たず、
  隣接がどちら向きで成立したかも復元できない。証跡は `sink-observed` の `adjacency_ok` で採る。
- **案 A では `fix` がほとんど出ない。** 出ないことが正常であり、`fix` の本数で判定しない。
- **不可視窓の割り込みは判定に影響しない**（実装が読み飛ばす）。ログの実測値に身に覚えのない
  HWND が現れたら、その窓のクラス名・可視性・owner を調べてから判断すること——過去に
  スレッド既定の IME 窓を「隣接保証の不成立」と読み違えかけた事例がある。
- **DPI 仮想化の罠。** 窓の寸法を外部ツールで測ると、DPI 非対応プロセスからの読み取りは
  全モニタ 96dpi に丸められて実値の半分などになる。**アプリ自身のログの値を正とすること。**
- **`target/` の成果物キャッシュを疑うこと。** 古い成果物で健全なソースに対し決定論的な赤が
  8 回連続再現した事例がある。驚く結果を見たら `cargo clean -p areka` から測り直す。
- **単一モニタでは S1 が意味を持たない。** 拡大率の異なる 2 台を用意すること。
