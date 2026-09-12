# 実機で 2 体のゴーストを確認した記録（タスク 6.2）

対象仕様: `areka-P0-charset-canon`
対象要件: 11.1 / 11.2 / 11.3
実施日: 2026-09-12
手順の典拠: `design.md` §Testing Strategy「実機確認（11.1〜11.3）」

見込みや引き写しではない。**emo2 の目視は 2026-09-13 に開発者が実施して合格**（気になる点なし）。
残るのは里々側の目視だけで、これは今日の areka では実施できない——理由は 8 節にある。
埋めるための手順は 7 節と 8 節にある。

---

## 0. 結論（先に）

- 文字コードの記録は **2 体とも期待どおり**。emo2 は初期＝既定 Shift_JIS → 採用 1 回
  （UTF-8）、里々標準テンプレートは初期＝既定 Shift_JIS → 採用 0 回。解決不能ラベルは
  両方 0 件。
- 「0 件」が沈黙でないことを、**その走行自身の中に debug 水準の行が実在すること**で裏づけた
  （里々は指令を 1 つ足して採り直した第 2 走行・emo2 は採用の行そのものが debug）。加えて、
  綴りを崩すと同じ行が消える較正 2 走行で、指令の綴りが点灯を支配していることも示した（6 節）。
- emo2 は本物のゴースト窓が 4 枚生え、バルーンへ文字が流れた（装着の記録あり）。
  **emo2 の目視は開発者がそのまま行える。**
- **里々標準テンプレートは窓が生えない。** 本仕様とは無関係の別の欠落
  （シェル定義ファイルが基準画像を明示せず、ファイル名の慣習に頼っている）で採寸が失敗し、
  検証用ダミー窓へ退避する。**この検体では挨拶の目視ができない**（8 節。要件 11.1 の
  目視部分は今日の areka では満たせない）。
- 検体の木は走行前と同じ状態へ戻した（`git status --porcelain` に残るのは本記録の
  ファイル 1 件だけ・9 節）。

---

## 1. 環境

| 項目 | 値 |
|---|---|
| OS | Windows 11 Pro 10.0.26200 |
| ホスト | x86_64-pc-windows-msvc |
| 作業ディレクトリ | `C:\home\maz\git\areka\.claude\worktrees\areka-p0-ukadoc-survey-33754d` |
| ブランチ | `claude/areka-p0-charset-canon-d3ab00` |
| 版 | タスク 6.1 まで着地済み＋検体追加 `bba59beb`＋数え違いの訂正 `91dff64c` |
| ビルド種別 | **dev（debug）**。release ではない |
| 実行ファイル | `target\debug\areka.exe`（23,008,768 bytes・本記録のために作り直した） |
| 32bit 補助プロセス | `target\debug\shiori-host32-helper.exe`（273,408 bytes・i686 ビルドを実行ファイルの隣へ複製） |
| 主モニタの実効 DPI | 192（起動ログの `primary_dpi=192`・拡大率 2.0） |
| 表示 | 実表示あり（emo2 は実ゴースト窓が生えた） |

ビルド（逐語・いずれも終了コード 0）:

```
cargo build -p areka
cargo build -p shiori-host32-helper --target i686-pc-windows-msvc
cp target/i686-pc-windows-msvc/debug/shiori-host32-helper.exe target/debug/shiori-host32-helper.exe
```

---

## 2. 検体 2 体

| | ⑴ 里々標準テンプレート | ⑵ emo2 |
|---|---|---|
| ghost_root（絶対パス） | `C:\home\maz\git\areka\.claude\worktrees\areka-p0-ukadoc-survey-33754d\vendors\sample_ghost\R_POST_and_KOMAINU` | `C:\home\maz\git\areka\.claude\worktrees\areka-p0-ukadoc-survey-33754d\crates\pilot\examples\shiori-host-32\fixtures\emo2` |
| balloon_root | emo2 のものを流用（`…\fixtures\emo2\emo2-kakukaku`） | 同左 |
| `ghost/master/descript.txt` の宣言 | `charset,Shift_JIS`・`shiori,satori.dll`・**`shiori.encoding` なし**・`id` なし | `charset,UTF-8`・`shiori,pasta.dll`・**`shiori.encoding` なし** |
| 識別 | `install.txt` の `directory,R_POST_and_KOMAINU`・表示名「Ｒポストと狛犬」 | fixture |
| SHIORI の DLL | `satori.dll`・PE ヘッダの machine = **0x014c（32bit）** | `pasta.dll`・同 **0x014c（32bit）** |
| 期待される初期値 | 既定の Shift_JIS（宣言が無いので） | 同左（宣言が無いので） |
| 期待される採用 | 0 回（里々は `Charset: Shift_JIS` を返す） | 1 回（`Charset: UTF-8`） |

DLL のビット数は PE ヘッダを直接読んで確かめた（`od` で `e_lfanew` を取り、その +4 の
2 バイトが machine 値）。どちらも 0x014c＝i386 で、32bit 補助プロセス経路を通る。

---

## 3. 手順（逐語コマンド）

design が定めるとおり、実バイナリを**絶対パス**で起動し、`AREKA_APP_SMOKE_EXIT_MS` で
有界の自動終了にし、`RUST_LOG` は**イベントの target 名**で指定した。

### ⑴ 里々標準テンプレート

```
RUST_LOG="info,shiori-charset=debug,ghost-boot=debug" \
AREKA_APP_SMOKE_EXIT_MS=20000 \
"C:\home\maz\git\areka\.claude\worktrees\areka-p0-ukadoc-survey-33754d\target\debug\areka.exe" \
  "C:\home\maz\git\areka\.claude\worktrees\areka-p0-ukadoc-survey-33754d\vendors\sample_ghost\R_POST_and_KOMAINU" \
  "C:\home\maz\git\areka\.claude\worktrees\areka-p0-ukadoc-survey-33754d\crates\pilot\examples\shiori-host-32\fixtures\emo2\emo2-kakukaku" \
  > satori.log 2>&1
```

終了コード **0**（自動終了が働いた）。記録は 47 行。

### ⑴′ 里々標準テンプレート（採り直し・**点灯の裏づけ用**）

上の走行は記録全体に debug 水準の行が 1 行も無く、**指令が届かなかった走行と見分けが
付かない**（6.2 節）。そこで、同じ検体を、**その走行の中で必ず出る debug の行を 1 つ足した
指令**で採り直した。違いは `RUST_LOG` に `areka::placement::diag=debug` を足したことだけで、
ほかは 1 文字も変えていない。

```
RUST_LOG="info,shiori-charset=debug,ghost-boot=debug,areka::placement::diag=debug" \
AREKA_APP_SMOKE_EXIT_MS=20000 \
"C:\home\maz\git\areka\.claude\worktrees\areka-p0-ukadoc-survey-33754d\target\debug\areka.exe" \
  "C:\home\maz\git\areka\.claude\worktrees\areka-p0-ukadoc-survey-33754d\vendors\sample_ghost\R_POST_and_KOMAINU" \
  "C:\home\maz\git\areka\.claude\worktrees\areka-p0-ukadoc-survey-33754d\crates\pilot\examples\shiori-host-32\fixtures\emo2\emo2-kakukaku" \
  > satori2.log 2>&1
```

終了コード **0**。記録は 54 行（増えた 7 行の内訳は、足した指令による debug 6 行と、
走行間の時間揺らぎで出たループ ticker の追いつき 1 行）。文字コードの件数は第 1 走行と**すべて同じ**（4.1 節の表）。

**足す指令にこれを選んだ理由と、事前の裏取り。** `areka::placement::diag` は
`crates/areka/src/placement/diag.rs` が `DIAG_TARGET` として持つ**明示の target 名**で
（モジュールパスに偶然一致する名前ではなく、リテラルで指定された target）、
`prepare_ghost_windows` がモニタ構成を**採寸の失敗より手前**で出すのに使っている。
同ファイルの呼出点のコメントが「固定物の不在等でダミー窓へ退避した運転の記録からも、
その運転がどのモニタ構成を見ていたかを再構成できることが事後診断の条件である」と書いて
おり、**まさに里々の検体が陥る退避経路でも出る**ことが設計上保証されている。実走でも
6 行出た（6.2 節）。

### ⑵ emo2

```
RUST_LOG="info,shiori-charset=debug,ghost-boot=debug" \
AREKA_APP_SMOKE_EXIT_MS=20000 \
"C:\home\maz\git\areka\.claude\worktrees\areka-p0-ukadoc-survey-33754d\target\debug\areka.exe" \
  "C:\home\maz\git\areka\.claude\worktrees\areka-p0-ukadoc-survey-33754d\crates\pilot\examples\shiori-host-32\fixtures\emo2" \
  "C:\home\maz\git\areka\.claude\worktrees\areka-p0-ukadoc-survey-33754d\crates\pilot\examples\shiori-host-32\fixtures\emo2\emo2-kakukaku" \
  > emo2.log 2>&1
```

終了コード **0**。記録は 76 行。

記録の検索はどの走行でも次の形で行った（`grep -c` は 0 件のとき終了コード 1 を返すので、
件数だけを取り出して数える形にしてある）。

```
grep -ac "<イベント名>" <記録ファイル>
```

---

## 4. ⑴ 里々標準テンプレートの結果

### 4.1 記録の件数

2 走行とも同じ値である（判定に用いるのは**点灯の裏づけを持つ第 2 走行**のほう）。

| イベント | 第 1 走行 | **第 2 走行（判定用）** | 期待 | 判定 |
|---|---:|---:|---:|---|
| `charset_initial` | 1 | **1** | 1 | ○ |
| `charset_switched` | 0 | **0** | 0 | ○ |
| `charset_label_unresolved` | 0 | **0** | 0 | ○ |
| `charset_forced_ignores_header` | 0 | 0 | — | ○ |
| `charset_unmappable_replaced` | 0 | 0 | — | ○ |
| `charset_invalid_bytes_replaced` | 0 | 0 | — | ○ |
| （参考）`ghost-boot` の行 | 2 | 2 | — | — |
| （参考）`shiori-charset` の行 | 0 | 0 | — | — |
| （参考）debug／trace 水準の行 | **0** | **6** | — | — |

該当行（逐語・色の指定だけ落とした。第 2 走行のもの。第 1 走行は時刻
`11:23:00.714491Z` で、それ以外は 1 文字も違わない）:

```
2026-09-12T11:40:30.797259Z  INFO ghost-boot: SHIORI 通信の初期の文字コードを決定した event="charset_initial" charset="Shift_JIS" source="default"
```

初期値は既定の Shift_JIS で、出どころは `default`（descript が `shiori.encoding` も
`shiori.forceencoding` も持たないため）。期待どおり。

### 4.2 SHIORI が本当に応答していること

`charset_switched` の 0 件は「相手が Shift_JIS を名乗った（＝切り替える必要が無かった）」
ためであって、「通信が成立しなかった」ためではない。同じ記録の次の行がそれを示す。

```
2026-09-12T11:23:00.714876Z  INFO actor{actor=kanade}: kanade: 起動指示を受領——OnInitialize NOTIFY を発行 event="boot_start"
2026-09-12T11:23:00.913238Z  INFO actor{actor=kanade}: areka_kanade::resource: shiori resource prefetch done id="username" outcome="value"
2026-09-12T11:23:00.913973Z  INFO actor{actor=kanade}: kanade: 起動種別にスクリプト——OnBoot をスキップし basewareversion へ event="boot_type_script"
2026-09-12T11:23:00.914023Z  INFO actor{actor=kanade}: kanade: 起動グリーティングを再生起動 event="boot_talk" talk_id=1
2026-09-12T11:23:20.760227Z  INFO actor{actor=shiori}: shiori-actor: 正規 clean shutdown 完了（unload → helper 正常終了 exit(0)） event="unload_clean"
```

- 利用者名の照会が `outcome="value"` で返っている＝32bit 補助プロセスが `satori.dll` を
  読み込み、**中身のある応答**を返した。
- 起動の挨拶が台本として返り、再生が始まっている（`boot_talk`）。
- 終了時に補助プロセスが正常終了している。

第 2 走行でも同じ 5 行が同じ順で出ている（時刻だけが違う。利用者名の照会は
11:40:30.895514 で `outcome="value"`、補助プロセスの正常終了は 11:40:50.819274）。

さらに、走行後に `satori.dll` 自身が `ghost/master/satori_savedata.txt`（484 バイト・
Shift_JIS）を書き出しており、その中に `＄起動回数 1`・`＄ゴースト起動時間累計(ms) 20000`
（＝本走行の有界時間）・`＄ユーザ名 ユーザ` がある。相手側から見ても 20 秒の対話が
成立していたことの裏づけになる（このファイルは 9 節で消した）。

### 4.3 この走行が見た起動の種別

**初回起動**（`OnFirstBoot` 側）。根拠: 走行前にこの検体には `ghost/master/profile/` が
存在せず、走行後に `profile/areka/sylphya.toml` が作られて `[boot] count = "1"` を持って
いた。記録にも台本からの `prop_set_cue applied key="areka.boot.count" value="1"` が出て
いる（`boot_gate skip_first_boot` は出ていない＝初回の分岐を通った）。

**この走行は冪等ではない。** 同じ検体を続けて 2 回目に起動すると `count` が進んで通常の
起動（`OnBoot` 側）になり、里々側も保存データを持った状態から始まる。第 2 走行を
**第 1 走行と同じ初回起動**にするため、走行の間で `profile/` と `satori_savedata.txt` を
消してから採り直した（第 2 走行にも `prop_set_cue applied key="areka.boot.count" value="1"`
が出ており、初回の分岐を通ったことが確かめられる）。9 節のとおり最後にも消してあるので、
次に起動する人はまた初回起動から始まる。

### 4.4 目視

**未実施（開発者の担当）。かつ、今日の areka ではこの検体で実施できない。** 8 節を読むこと。

---

## 5. ⑵ emo2 の結果

### 5.1 記録の件数

| イベント | 件数 | 期待 | 判定 |
|---|---:|---:|---|
| `charset_initial` | **1** | 1 | ○ |
| `charset_switched` | **1** | ちょうど 1・以後 0 | ○ |
| `charset_label_unresolved` | **0** | 0 | ○ |
| `charset_forced_ignores_header` | 0 | — | ○ |
| `charset_unmappable_replaced` | 0 | — | ○ |
| `charset_invalid_bytes_replaced` | 0 | — | ○ |

該当行（逐語）:

```
2026-09-12T11:24:31.818106Z  INFO ghost-boot: SHIORI 通信の初期の文字コードを決定した event="charset_initial" charset="Shift_JIS" source="default"
2026-09-12T11:24:32.905421Z DEBUG actor{actor=shiori}: shiori-charset: 応答の Charset ヘッダを以後の要求の文字コードとして採用した event="charset_switched" from="Shift_JIS" to="UTF-8"
```

`charset_switched` は記録全体で 1 行きり（`grep -ac` が 1）。**以後 0 行**もこれで言える
——走行は採用の 18 秒後まで続いており（自動終了は 11:24:51）、その間に台本の再生と
定常運転への復帰が起きているが、2 行目は出ていない。

### 5.2 既定で送った要求は「1 本」ではなく「2 本」であること

2026-09-12 の裁定（要件 2.7・12.1 の訂正）どおりの並びが記録に出ている。

| 順 | 何 | 記録の行（時刻） | 文字コード |
|---:|---|---|---|
| 1 | `OnInitialize`（片道の通知。要件 5.3 によりその応答は採用の根拠にしない） | `event="boot_start"` 11:24:31.818781 | 既定の Shift_JIS |
| 2 | 利用者名の照会（応答を待つ。**この応答で採用が起きる**） | 採用 11:24:32.905421 → 照会完了 11:24:32.905625 | 既定の Shift_JIS |
| 3 以降 | 起動の挨拶ほか | `event="boot_talk"` 11:24:32.909100 | **UTF-8** |

採用の行が利用者名の照会の完了行の直前（0.2 ミリ秒前）に出ていることが、採用の起点が
**2 本目の応答**であることを示している。1 本目（片道の通知）では採用していない。

なお emo2 の利用者名の照会は `outcome="no_content"`＝ステータス 204 で返っており、
**本文の無い応答でもヘッダを採用している**。これは本仕様の中核の約束（204／400／500 でも
ヘッダを採用する）が実機で働いていることの実例である。

### 5.3 この走行が見た起動の種別

**通常起動**（`OnBoot` 側）。走行前から `fixtures/emo2/ghost/master/profile/areka/sylphya.toml`
が存在し `[boot] count = "1"` を持っていた（本記録より前の別の走行が作ったもの）。記録にも
`kanade: boot_gate skip_first_boot event="boot_gate"` が出ている。

### 5.4 表示（目視の前提は成立している）

```
2026-09-12T11:24:31.823481Z  INFO actor{actor=emo-text}: areka: 本物のゴースト窓を開きました（placement シーム・スコープごとにキャラ窓＋バルーン窓） scopes=[0, 1]
2026-09-12T11:24:33.418436Z  INFO actor{actor=emo-text}: areka_emo_text::actor: テキスト供給面を予約スロットへ装着した … actor=0 slot=28v0 physical_size=(640, 244) wrap=BudouxWordWrap
2026-09-12T11:24:34.813522Z  INFO actor{actor=emo-text}: areka_emo_text::actor: テキスト供給面を予約スロットへ装着した … actor=1 slot=30v0 physical_size=(432, 186) wrap=BudouxWordWrap
2026-09-12T11:24:36.776817Z  INFO actor{actor=kanade}: kanade: talk 完了——定常運転へ復帰 event="steady_talk_done"
2026-09-12T11:24:51.823095Z  INFO actor{actor=emo-text}: areka: smoke 自動 close: 起動窓を despawn しました count=4
```

窓が 4 枚（2 つのスコープ × キャラ窓＋バルーン窓）生え、両方のバルーンに文字の供給面が
装着され、挨拶が最後まで再生されている。**emo2 の目視は開発者がそのまま行える。**

### 5.5 目視

**実施済み（2026-09-13・開発者）。合格。**「挨拶が文字化けせずにバルーンへ出ること」と
「表示が本仕様の適用前と同一であること」の 2 点を開発者が画面で確認し、**気になる点は無かった**。

---

## 6. 「0 件」が沈黙でないことの裏づけ

0 件は「見に行って無かった」であって「そもそも点いていなかった」ではない、と言える必要が
ある。2 つの target それぞれについて裏づけを置く。

### 6.1 target `ghost-boot`

両方の走行で `ghost-boot` の行が **2 行ずつ**出ている（`charset_initial` と
`ghost boot sequence completed`）。ゆえに `ghost-boot` は確かに点いており、
`charset_label_unresolved`（この target 側・警告水準）の 0 件は沈黙ではない。

### 6.2 target `shiori-charset`

こちらは 2 段で裏づける。**⑴ その走行自身の中に debug 水準の行が実在すること**（＝
debug の絞り込みが生きていた証拠）と、**⑵ この綴りが `shiori-charset` を点けること**である。
⑴ だけでは「debug は生きていたが `shiori-charset` の綴りが効いていない」可能性が、
⑵ だけでは「綴りは効くが、この走行にその指令が届いていない」可能性が残る。両方が要る。

#### ⑴ その走行の中に debug の行が実在するか

| 走行 | `RUST_LOG` の指令（逐語） | **debug／trace 行** | `charset_initial` | `charset_switched` | `ghost-boot` |
|---|---|---:|---:|---:|---:|
| 本番 里々 **第 1 走行**（3 節 ⑴） | `info,shiori-charset=debug,ghost-boot=debug` | **0** | 1 | 0 | 2 |
| 較正 C1 | `info`（指令なし） | **0** | 1 | 0 | 2 |
| **本番 里々 第 2 走行**（3 節 ⑴′・**判定用**） | 上に `,areka::placement::diag=debug` を足したもの | **6** | 1 | **0** | 2 |
| 本番 emo2（5 節） | `info,shiori-charset=debug,ghost-boot=debug` | **1**（`charset_switched` 自身） | 1 | 1 | 2 |

**第 1 走行は、指令が届かなかった走行（C1）と記録の上で 1 つも違わない。** この記録が
判定に使うどの目盛りでも区別が付かず、そのままでは 0 件を「見に行った結果」と主張できない。
`crates/areka/src/main.rs` の絞り込み構築は、`RUST_LOG` が未設定・非 UTF-8・書式不正の
いずれでも**黙って `info` へ退避し、実際に効いている絞り込みを一切表示しない**ので、
届かなかった場合の見え方がちょうどこれになる。

そこで採り直した第 2 走行では、**同じ記録の中に debug の行が 6 行実在する**
（すべて `areka::placement::diag`）。

```
2026-09-12T11:40:30.771767Z DEBUG areka::placement::diag: [diag.monitor_snapshot] context=prepare_ghost_windows count=2
2026-09-12T11:40:30.771792Z DEBUG areka::placement::diag: [diag.monitor] index=0 handle=197493 bounds=0,0,2880,1800 work_area=0,0,2880,1704 dpi=192 primary=true
2026-09-12T11:40:30.771801Z DEBUG areka::placement::diag: [diag.monitor] index=1 handle=131193 bounds=-2560,195,0,1795 work_area=-2560,195,0,1795 dpi=144 primary=false
2026-09-12T11:40:30.885836Z DEBUG actor{actor=emo-text}: areka::placement::diag: [diag.monitor_snapshot] context=work_area_sync count=2
（残り 2 行は work_area_sync 側のモニタ 2 台分）
```

ゆえに第 2 走行では **`RUST_LOG` が確かに届き、debug 水準まで絞り込みが開いていた**。
`info` への黙った退避は起きていない。指令は 1 本の文字列として渡っており、その中に
`shiori-charset=debug` が入っている。よって**同じ記録の `charset_switched` の 0 件は、
見に行った結果の 0 である**。

#### ⑵ この綴りが `shiori-charset` を点けること

| 走行 | `RUST_LOG` の指令（逐語） | `charset_switched` | 意味 |
|---|---|---:|---|
| 本番 emo2（5 節） | `info,shiori-charset=debug,ghost-boot=debug` | **1** | この綴りで `shiori-charset` の debug は**点く** |
| 較正 C1 | `info`（指令なし） | 0 | 指令が無いと同じ検体でも採用の行は消える |
| 較正 C2 | `info,shiori_host32_host=debug`（モジュールパス名） | 0 | **モジュールパス名では点かない**（偽の 0） |

較正の 2 走行（C1・C2）はどちらも emo2 を検体にし、本番 emo2 走行と検体も実行ファイルも
同一で、**違うのは `RUST_LOG` の文字列だけ**である。同じ検体が指令次第で 1 にも 0 にも
なるので、この指令が採用の行の点灯を支配していることが実証される。里々の第 2 走行が
使ったのは「1 になる側の指令」に 1 つ足したものである。

較正の逐語コマンド（有界時間だけ 8 秒に縮めた。採用は起動の約 1.7 秒後に起きるので足りる）:

```
# C1
RUST_LOG="info" AREKA_APP_SMOKE_EXIT_MS=8000 "…\target\debug\areka.exe" "…\fixtures\emo2" "…\fixtures\emo2\emo2-kakukaku" > calib_info_only.log 2>&1
# C2
RUST_LOG="info,shiori_host32_host=debug" AREKA_APP_SMOKE_EXIT_MS=8000 "…\target\debug\areka.exe" "…\fixtures\emo2" "…\fixtures\emo2\emo2-kakukaku" > calib_modpath.log 2>&1
```

どちらも終了コード 0。C2 は 6.1 の申し送り（記録 8 節の注記）が警告していた落とし穴を
**実走で再現したもの**である——モジュールパス名で指定すると採用の行が出ず、見た人は
「切り替えが起きなかった」と読み違える。

### 6.3 申し送り: 効いている絞り込みが記録に出ない（統合担当へ）

本節の遠回りの根は 1 つである。**areka は自分が実際に使っている記録の絞り込みを一度も
表示しない。** `crates/areka/src/main.rs` の絞り込み構築は既定環境変数からの読み取りに
失敗すると黙って `info` へ退避するので、指令が届かなかった走行と、届いたうえで該当の行が
無かった走行とが、**記録の上で一切区別できない**。今回は「必ず出る debug の行」を 1 つ
足すことで外から区別を作ったが、これは走行のたびに人が気を付ける必要があり、忘れれば
また偽の 0 になる。

起動時に有効な絞り込みを 1 行（情報水準）出せば、この種の取り違えは構造的に無くなる。
**本タスクはコードに触れない約束なので手を入れていない。** 引受先の判断を統合担当に委ねる。

---

## 7. 目視の欄

| 検体 | 目視で見るもの | 結果 |
|---|---|---|
| ⑴ 里々標準テンプレート | 挨拶が文字化けせずバルーンに出ること | **未実施。かつ今日の areka では実施できない**（8 節） |
| ⑵ emo2 | 挨拶が文字化けせずバルーンに出ること／表示が本仕様の適用前と同一であること | **合格**（2026-09-13・開発者が画面で確認。気になる点なし） |

画面を見る確認は人の目が要る。emo2 の分は開発者が次の 1 行をそのまま
PowerShell に貼れば行える。3 分の有界自動終了なので、観察したあと窓を閉じてもよいし、
放置すれば自動で終わる。

```powershell
$env:RUST_LOG="info,shiori-charset=debug,ghost-boot=debug"; $env:AREKA_APP_SMOKE_EXIT_MS="180000"; & "C:\home\maz\git\areka\.claude\worktrees\areka-p0-ukadoc-survey-33754d\target\debug\areka.exe" "C:\home\maz\git\areka\.claude\worktrees\areka-p0-ukadoc-survey-33754d\crates\pilot\examples\shiori-host-32\fixtures\emo2" "C:\home\maz\git\areka\.claude\worktrees\areka-p0-ukadoc-survey-33754d\crates\pilot\examples\shiori-host-32\fixtures\emo2\emo2-kakukaku"
```

里々の分も同じ形で起動はできる（ghost_root を
`…\vendors\sample_ghost\R_POST_and_KOMAINU` に替えるだけ）。ただし 8 節のとおり、今は
立ち絵もバルーンも出ずダミー窓になるので、見えるものが無い。

---

## 8. 見つかった障害: 里々標準テンプレートは窓が生えない（**本仕様とは無関係**）

### 何が起きたか

```
2026-09-12T11:23:00.684767Z ERROR areka_emo_compose: 定義層が皆無で外形 0×0 の退化データ: EmptyComposition surface_id=0
2026-09-12T11:23:00.684822Z ERROR areka::placement::measure: measure: scope0（surface id 0）の採寸合成に失敗 reason=surface 0 の合成失敗: surface 0 has no layers at all (extent 0x0)
2026-09-12T11:23:00.684921Z ERROR areka: 窓配置の準備に失敗しました——検証用ダミー窓へフォールバックします …
2026-09-12T11:23:00.718059Z  INFO actor{actor=emo-text}: areka: 検証用ダミー窓を開きました（placement フォールバック）
```

立ち絵の採寸ができず、ゴースト窓の代わりに検証用のダミー窓が開く。バルーンも出ない。
挨拶そのものは再生されている（`boot_talk` → `steady_talk_done`）が、文字の供給面を
装着する先が無いため描画は見送られ続ける（`actor の装着先（予約スロット）が未解決` の
警告が繰り返し出る）。

### 原因

シェル定義ファイルの書き方の違いである。

- emo2 の `shell/master/surfaces.txt` は各面の中身を `element0,overlay,surface0.png,0,0` と
  **明示して**いる。
- 里々標準テンプレートの `shell/master/surfaces.txt` は `element` を **1 行も持たない**
  （`grep -c element` が 0。**この 0 は検出の失敗ではない**——同じコマンドを emo2 の
  同名ファイルに掛けると **63** 返る）。当たり判定と座標だけを書き、基準となる画像は
  `surface0000.png` というファイル名の慣習で暗黙に決まる、という古くからの書き方である。
  画像ファイル自体は `surface0000.png` を含めて **10 枚**同梱されている
  （`surface0000`〜`surface0006`・`surface0010`・`surface0100`・`surface0200`）。

areka にはこの「ファイル名の慣習で基準画像を決める」経路が実装されていない
（`crates/areka-seriko`・`areka-emo-compose`・`areka-parsers/src/shell` を
`surface0000`／`{:04}` 等で走査してヒット 0 件）。ゆえに層が 1 枚も無い面として扱われ、
外形 0×0 で退避する。

### 帰属

**本仕様（文字コードの正典化）とは無関係である。** 文字コードの経路はこの検体でも
最後まで正常に働いており（4.2 節）、失敗しているのは画像の合成だけである。本仕様は
シェルの合成にも面の解決にも触っていない。

### これが要件 11.1 に与える影響

要件 11.1 は「里々の標準テンプレートで挨拶を文字化けなくバルーンに表示する」ことを
求めているが、**今日の areka ではこの検体でバルーンが出ない**ので、目視部分は満たせない。
機械で確かめられる部分（要件 11.2 の初期値の記録と採用の記録、11.3 の裏づけ）は
すべて満たしている。

引受先の判断が要る。取りうる道は 3 つある。

1. 暗黙の基準画像（`surfaceNNNN.png` の慣習）を実装する spec を起こし、要件 11.1 の
   目視部分をそこへ預ける。ukadoc の網羅台帳にこの語彙の行があるはずなので、統合担当
   （`ukadoc-coverage-roadmap`）が引受先の候補になる。
2. 検体を替える（`element` を明示している Shift_JIS のゴーストを別に用意する）。
   ただし「里々の標準テンプレート」という検体は 2026-09-11 の要件ディスカッションで
   開発者が確定したものなので、替えるなら開発者の裁定が要る。
3. 目視の対象を emo2 だけにし、里々側は機械の確認（本記録の 4 節）で足りるとする。

**この 3 択は実装者の裁量を越えるので、完了ゲートで開発者が決めること。**

---

## 9. 再現するときの注意と、後片付け

### 走行は冪等ではない

起動すると `<ghost_root>\ghost\master\profile\areka\sylphya.toml` が作られ、`[boot] count`
が書かれる。2 回目以降は起動の種別が初回から通常へ変わる。また `satori.dll` は自分で
`ghost\master\satori_savedata.txt` を書く。

### 後片付け

里々の検体が走行で得たものを、走行前の状態へ戻した。**同じ後片付けを 2 度行っている**
——第 1 走行と第 2 走行の間（第 2 走行も初回起動から始めるため。4.3 節）と、
すべての走行を終えたあとである。

```
rm -rf vendors/sample_ghost/R_POST_and_KOMAINU/ghost/master/profile
rm     vendors/sample_ghost/R_POST_and_KOMAINU/ghost/master/satori_savedata.txt
git status --porcelain      # 本記録のファイル 1 件のみ
```

`git status --porcelain` に残るのは**本記録のファイル 1 件だけ**で、ほかに差分も未追跡も
無い。
検体の木（`vendors/sample_ghost/**`）と emo2 の固定物
（`crates/pilot/examples/shiori-host-32/fixtures/**`）はどちらもきれいな状態である。

emo2 側の `profile/` は本記録より前から存在し（`crates/pilot/examples/shiori-host-32/.gitignore`
が追跡対象から外している）、本走行でも `[boot] count = "1"` のまま変わっていない。
里々の検体にはこの除外規則が無いので、次に誰かが起動したら `git status` に未追跡の
`profile/` が現れる。**走行のたびに消すか、除外規則を足すかは統合担当の判断に委ねる**
（本タスクは検体の木に手を入れない方を採った）。

---

## 10. 実行したコマンドの一覧（再現用・逐語）

```
# ビルド（いずれも終了コード 0）
cargo build -p areka
cargo build -p shiori-host32-helper --target i686-pc-windows-msvc
cp target/i686-pc-windows-msvc/debug/shiori-host32-helper.exe target/debug/shiori-host32-helper.exe

# 本番 3 走行（3 節に全文。いずれも終了コード 0）
RUST_LOG="info,shiori-charset=debug,ghost-boot=debug" AREKA_APP_SMOKE_EXIT_MS=20000 areka.exe <里々 ghost_root> <emo2 balloon_root>   > satori.log  2>&1
RUST_LOG="info,shiori-charset=debug,ghost-boot=debug" AREKA_APP_SMOKE_EXIT_MS=20000 areka.exe <emo2 ghost_root> <emo2 balloon_root>   > emo2.log    2>&1
# 里々の採り直し（点灯の裏づけ用・指令を 1 つ足しただけ。その前に profile と savedata を消してある）
RUST_LOG="info,shiori-charset=debug,ghost-boot=debug,areka::placement::diag=debug" AREKA_APP_SMOKE_EXIT_MS=20000 areka.exe <里々 ghost_root> <emo2 balloon_root> > satori2.log 2>&1

# 較正 2 走行（6.2 節・いずれも終了コード 0）
RUST_LOG="info"                            AREKA_APP_SMOKE_EXIT_MS=8000 areka.exe <emo2 ghost_root> <emo2 balloon_root> > calib_info_only.log 2>&1
RUST_LOG="info,shiori_host32_host=debug"   AREKA_APP_SMOKE_EXIT_MS=8000 areka.exe <emo2 ghost_root> <emo2 balloon_root> > calib_modpath.log   2>&1

# 記録の検索（0 件でも終了コードに引きずられない形）
grep -ac "charset_initial"                 <記録ファイル>
grep -ac "charset_switched"                <記録ファイル>
grep -ac "charset_label_unresolved"        <記録ファイル>
grep -ac "charset_forced_ignores_header"   <記録ファイル>
grep -ac "charset_unmappable_replaced"     <記録ファイル>
grep -ac "charset_invalid_bytes_replaced"  <記録ファイル>
grep -ac "ghost-boot"                      <記録ファイル>
grep -ac "shiori-charset"                  <記録ファイル>

# debug／trace 水準の行が実在するか（6.2 節 ⑴・色の指定を落としてから数える）
sed $'s/\x1b\\[[0-9;]*m//g' <記録ファイル> | grep -acE '^[0-9-]+T[0-9:.]+Z (DEBUG|TRACE)'
sed $'s/\x1b\\[[0-9;]*m//g' <記録ファイル> | grep -ac "areka::placement::diag"

# 検体の素性
grep -c element vendors/sample_ghost/R_POST_and_KOMAINU/shell/master/surfaces.txt          # 0
grep -c element crates/pilot/examples/shiori-host-32/fixtures/emo2/shell/master/surfaces.txt  # 63（較正＝上の 0 が検出漏れでないこと）
ls vendors/sample_ghost/R_POST_and_KOMAINU/shell/master/surface*.png | wc -l                  # 10

# 後片付けと確認（9 節）
rm -rf vendors/sample_ghost/R_POST_and_KOMAINU/ghost/master/profile
rm     vendors/sample_ghost/R_POST_and_KOMAINU/ghost/master/satori_savedata.txt
git status --porcelain      # 本記録のファイル 1 件のみ
```

走行の生の記録は一時ディレクトリに採ったもので、リポジトリには入れていない。本記録に
必要な行は 4 節・5 節・8 節へ逐語で写してある。
