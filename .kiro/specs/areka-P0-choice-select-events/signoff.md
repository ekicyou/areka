# 実機サインオフ手順書（areka-P0-choice-select-events / tasks.md 7.2）

対応要件: **Req9.3**（実 emo2・実 pasta・実 DPI でメニューが一周することを人間が確認）/ **Req9.4**（選択待ち中に消費側ゴーストの自発トークが漏れないことの実機観測）
対応設計: **Testing Strategy / E2E・実機サインオフ（Req9.3/9.4・手順の明文化）** 1〜3、**Error Handling / Monitoring**、**Error Handling / ログ語彙表**

> **このタスクの完了状態** = 「§3 メニュー一周の人間サインオフ」と「§4 ログ突合」の**双方**が §5 の記録欄に記入された状態。片方だけでは未完了。

---

## 0. 前提条件（実測で確認済みの絶対パス）

| 項目 | 絶対パス | 実測状況 |
|---|---|---|
| 実 emo2 ゴースト root（`ghost/master/descript.txt` 実在・辞書 13 本込みのフルゴースト） | `C:\home\maz\git\areka\.claude\worktrees\kiro-gpu-test-crash-ec3b84\crates\pilot\examples\shiori-host-32\fixtures\emo2` | ✅ 実在 |
| 実 balloon root | `C:\home\maz\git\areka\.claude\worktrees\kiro-gpu-test-crash-ec3b84\crates\pilot\examples\shiori-host-32\fixtures\emo2\emo2-kakukaku` | ✅ 実在 |
| 実 SHIORI（`pasta.dll`） | `C:\home\maz\git\areka\.claude\worktrees\kiro-gpu-test-crash-ec3b84\crates\pilot\examples\shiori-host-32\fixtures\emo2\ghost\master\pasta.dll` | ✅ 実在・**PE machine = 0x014c（i386＝32bit）** |
| メニュー辞書（`\q[]` の実物） | `…\fixtures\emo2\ghost\master\dic\menu.pasta` | ✅ 実在 |
| 起動実行体 | `C:\home\maz\git\areka\.claude\worktrees\kiro-gpu-test-crash-ec3b84\target\debug\areka.exe` | ✅ ビルド済（`cargo build -p areka --bin areka`） |
| 32bit SHIORI helper（**areka.exe と同一ディレクトリ必須**） | `C:\home\maz\git\areka\.claude\worktrees\kiro-gpu-test-crash-ec3b84\target\debug\shiori-host32-helper.exe` | ⚠️ **要ステージング**（下記参照） |
| helper の i686 成果物（コピー元） | `C:\home\maz\git\areka\.claude\worktrees\kiro-gpu-test-crash-ec3b84\target\i686-pc-windows-msvc\debug\shiori-host32-helper.exe` | ✅ ビルド済・PE machine = 0x014c |

### 0.1 helper のステージング（必須・毎回確認）

`crates/areka/src/main.rs` の `default_helper_exe_path()` は **`current_exe()` の隣の `shiori-host32-helper.exe`** を無条件に解決する（helper パスを差す env は**存在しない**）。
一方 `cargo build --workspace` は **x64 の** `shiori-host32-helper.exe` を `target\debug\` へ吐くため、放置すると 32bit の `pasta.dll` を LOAD できない。**workspace ビルドのたびに上書きされる**ので、実走の直前に必ず i686 版をコピーし直すこと。

```powershell
cd C:\home\maz\git\areka\.claude\worktrees\kiro-gpu-test-crash-ec3b84
cargo build -p areka --bin areka
cargo build -p shiori-host32-helper --target i686-pc-windows-msvc
Copy-Item `
  "C:\home\maz\git\areka\.claude\worktrees\kiro-gpu-test-crash-ec3b84\target\i686-pc-windows-msvc\debug\shiori-host32-helper.exe" `
  "C:\home\maz\git\areka\.claude\worktrees\kiro-gpu-test-crash-ec3b84\target\debug\shiori-host32-helper.exe" -Force
```

コピー結果の検証（`0x014c` なら OK・`0x8664` なら x64 が残っている＝実発話しない）:

```powershell
function Get-Machine($p){ $fs=[IO.File]::OpenRead($p); $br=New-Object IO.BinaryReader($fs); $fs.Seek(0x3c,'Begin')|Out-Null; $pe=$br.ReadInt32(); $fs.Seek($pe+4,'Begin')|Out-Null; $m=$br.ReadUInt16(); $fs.Close(); "0x{0:x4}" -f $m }
Get-Machine "C:\home\maz\git\areka\.claude\worktrees\kiro-gpu-test-crash-ec3b84\target\debug\shiori-host32-helper.exe"
```

### 0.2 実 DPI（≠96）

`primary_dpi=120`（125%）で実測済み。**96 のまま検証しても DPI 差を見たことにならない**（steering `areka-dpi-following-core-design`）。表示スケールを 125% か 150% にしてから起動すること。起動ログ 1 行目付近の `placement: 起動時 k₀ を導出 … primary_dpi=<値>` で確認できる。

---

## 1. 起動手順（PowerShell・絶対パス・有界 auto-exit ＋ ログ収集）

**相対パスで起動しないこと**（`pasta.dll` の LOAD が `0x8007007E` で落ちる・steering `areka-emo2-signoff-needs-absolute-paths`）。

```powershell
$log = "$env:USERPROFILE\Desktop\areka-signoff-choice.log"

$env:RUST_LOG              = "info,kanade=trace"
$env:AREKA_APP_SMOKE_EXIT_MS = "300000"   # 5 分（人間がメニューを一周する猶予・大きめ）

& "C:\home\maz\git\areka\.claude\worktrees\kiro-gpu-test-crash-ec3b84\target\debug\areka.exe" `
  "C:\home\maz\git\areka\.claude\worktrees\kiro-gpu-test-crash-ec3b84\crates\pilot\examples\shiori-host-32\fixtures\emo2" `
  "C:\home\maz\git\areka\.claude\worktrees\kiro-gpu-test-crash-ec3b84\crates\pilot\examples\shiori-host-32\fixtures\emo2\emo2-kakukaku" `
  *>&1 | Tee-Object -FilePath $log
```

- `AREKA_APP_SMOKE_EXIT_MS`（`crates/areka/src/main.rs:814`）: 指定 ms 後に全ゴースト窓を despawn して**正常終了**する有界 auto-exit。**この直接起動には番犬が無い**ので、`crates/areka/tests/emo2_real_run.rs` の `#[test]`（番犬 120s）とは値を混同しないこと。
- `RUST_LOG="info,kanade=trace"`:
  - `kanade=trace` … `shiori_request`（`crates/areka-kanade/src/actor.rs:285`・**`status=` に `choosing` が載る唯一の観測点**）と `choice_cascade_stage`（trace）を捕捉する。
  - `info` … `choice_selected`（UI 層・target=`areka::input_events::balloon`）・`choice_accepted` / `choice_waiting_established` / `steady_talk` / `choice_resolved` / `choice_timeout_*`（target=`kanade`）を捕捉する。
- 途中で終了したいときは **Ctrl+左ダブルクリック**（キャラ窓の不透明域・暫定退避 `mouse_escape_close`）。放置しても 5 分で自動終了する。

---

## 2. 実機で踏む経路（emo2 の実メニュー・`dic/menu.pasta` 実測）

```
ダブルクリック（キャラ窓）
  → ＊OnMouseDoubleClick 「ダブルクリックしたね？メニューやで。」
  → メインメニュー選択肢: [おしゃべり頻度] [エモの位置調整] [閉じる]
       ├ [おしゃべり頻度] → ＊Onおしゃべり頻度メニュー 「おしゃべり頻度、どのくらいがええ？」
       │    → サブメニュー: [しゃべくり] [ほどよく] [たまーに] [もどる]
       │         ├ [ほどよく] → 「ほどよくやね。了解や。」＋ 頻度変数を 60/90 へ更新 → 再びサブメニュー
       │         └ [もどる]   → ＊Onメインメニュー 「メニューに戻るで。」→ メインメニュー
       └ [閉じる] → ＊Onメニュー閉じる 「またね～。」（**メニューを閉じるだけ・アプリは終了しない**）
```

**重要（emo2 固有）**: emo2 の `\q[]` の ID は全て `On` 始まり（`Onおしゃべり頻度メニュー` 等）＝**任意名（`CascadePlan::Named`）1 段**。ゆえに実機ログに `OnChoiceSelectEx` / `OnChoiceSelect` は**出ない**（出たら欠陥）。正典 2 段形（Ex→無印）は決定論檻（6.1）の担当であり、本サインオフの観測対象ではない。

---

## 3. 人間が確認するチェックリスト（Req9.3）

各段で「見えれば合格」の条件を満たしたらチェックする。

- [ ] **(0) 起動**: emo2 の立ち絵が既定位置に表示され、`OnBoot` の挨拶がバルーンへ typewriter 進行で流れる。
      ※ **emo2 の surface0 はバルーンが焼き込まれた立ち絵**である。立ち絵の中に文字が見えるのは**正常**——バグと早合点しないこと（steering `areka-emo2-surface0-baked-balloon`）。
- [ ] **(1) ダブルクリック → メニュー表示**: キャラ窓の不透明域を**左ダブルクリック**（Ctrl は押さない）すると「ダブルクリックしたね？メニューやで。」に続き、**選択肢 3 件**（`おしゃべり頻度` / `エモの位置調整` / `閉じる`）がバルーンに並ぶ。
- [ ] **(2) 選択肢のハイライト**: 選択肢の上へマウスを乗せると当該行がハイライトされ、外すと戻る（ヒット判定が生きている）。
- [ ] **(3) 項目選択 → サブメニュー**: `おしゃべり頻度` をクリックすると「おしゃべり頻度、どのくらいがええ？」が流れ、**サブメニュー 4 件**（`しゃべくり` / `ほどよく` / `たまーに` / `もどる`）が並ぶ。
- [ ] **(4) サブメニュー項目の実行**: `ほどよく` をクリックすると「ほどよくやね。了解や。」が流れ、**再びサブメニュー 4 件**が並ぶ（頻度変化は以後の自発トーク間隔で判定＝遷移 talk で足りる）。
- [ ] **(5) 「もどる」**: `もどる` をクリックすると「メニューに戻るで。」が流れ、**メインメニュー 3 件**へ戻る。
- [ ] **(6) 「閉じる」**: `閉じる` をクリックすると「またね～。」が流れ、**選択肢が消えて定常運転へ戻る**（アプリは終了しない）。
- [ ] **(7) 一周中にエラーダイアログ・フリーズ・バルーン消失・立ち絵の座標破綻が無い**。
- [ ] **(8) 終了**: Ctrl+左ダブルクリック（または放置して auto-exit）でアプリが静かに正常終了する。

### 3.1 注意（30 秒タイムアウト）

`choice_timeout_default_ms = 30_000`（`crates/areka-kanade/src/msg.rs:298`）。`menu.pasta` の `\q[]` はタイムアウトを指定していないため**選択肢表示から 30 秒放置すると `OnChoiceTimeout` が発火**する。emo2 の辞書に `OnChoiceTimeout` の応答は**無い**ので 204 → 選択解除（`choice_timeout_cancelled` → メニュー talk が中断）となる。
これは**設計どおりの挙動**であり欠陥ではない。一周の観測中は各段で 30 秒以上迷わないこと。もし観測したい場合は §4.4 の grep で確認できる。

---

## 4. ログ突合手順（Req9.4）

一周を終えたら（アプリ終了後）、`$log` に対して以下を順に実行する。**すべて実コードの語彙で確認済み**（`crates/areka-kanade/src/schedule/steady.rs`・`crates/areka-kanade/src/actor.rs`・`crates/areka/src/input_events/balloon.rs`）。

```powershell
$log = "$env:USERPROFILE\Desktop\areka-signoff-choice.log"
```

### 4.1 系列突合（design Monitoring 節が指定する 4 段）

1 回の選択につき **`choice_selected` → `choice_accepted` → `choice_cascade_stage` → （応答があれば）`steady_talk` → `choice_resolved`** がこの順で並ぶこと。

```powershell
Select-String -Path $log -Pattern 'choice_selected|choice_waiting_established|choice_accepted|choice_cascade_stage|steady_talk|choice_resolved'
```

期待（`おしゃべり頻度` を 1 回選んだ場合の 1 サイクル）:

| # | event | level / target | 見るべきフィールド |
|---|---|---|---|
| 1 | `choice_waiting_established` | info / kanade | `candidates`（メインメニューなら 3）・`deadline` |
| 2 | `choice_selected` | info / `areka::input_events::balloon` | `id=Onおしゃべり頻度メニュー`・`label=おしゃべり頻度`・`scope` |
| 3 | `choice_accepted` | info / kanade | `plan=Named`・`choice_id`・`label`・`talk_id` |
| 4 | `choice_cascade_stage` | **trace** / kanade | `stage=Onおしゃべり頻度メニュー`・`has_next=false` |
| 5 | `steady_talk` | info / kanade | `origin=Onおしゃべり頻度メニュー`・`prev_talk_id`（＝選択元 talk） |
| 6 | `choice_resolved` | info / kanade | `talk_id`（旧）・`outcome=value` |

- [ ] 一周で踏んだ選択回数（`もどる`・`閉じる` を含む）と `choice_accepted` の件数が一致する。
- [ ] `choice_accepted` 1 件につき `choice_resolved` が**ちょうど 1 件**（Req5.4「1 選択＝高々 1 解決」）。
- [ ] `OnChoiceSelectEx` / `OnChoiceSelect` が**出ていない**（emo2 は全て任意名 1 段・§2 参照）。

```powershell
# 件数の突合
'choice_selected','choice_accepted','choice_cascade_stage','choice_resolved' |
  ForEach-Object { "{0,-24} {1}" -f $_, (Select-String -Path $log -SimpleMatch $_).Count }
# 正典 2 段形が出ていないこと（emo2 では 0 が正）
(Select-String -Path $log -Pattern 'OnChoiceSelectEx|id=OnChoiceSelect ').Count
```

### 4.2 `choosing` の実機観測（複合 Status wire・Req6.1/6.4）

選択肢表示中の周期リクエストの `Status` に `talking,choosing` が載り、解決後に `choosing` が消えること。

```powershell
Select-String -Path $log -Pattern 'shiori_request' | Select-String -SimpleMatch 'choosing'
```

- [ ] `status=Some("talking,choosing")` の `shiori_request`（`id=OnSecondChange` の NOTIFY）が選択肢表示中に出ている。
- [ ] カスケード段の GET（`id=Onおしゃべり頻度メニュー` 等）にも `status=Some("talking,choosing")` が載っている。
- [ ] メニューを閉じた（`choice_resolved` かつ次の選択肢が出ていない）以降の `shiori_request` から `choosing` が**消えている**。

### 4.3 【Req9.4 の中核】選択待ち中に自発トーク起動が現れないこと

自発トーク（スケジューラ周期起動）は **`steady_talk` かつ `origin=OnSecondChange`** として現れる。
選択由来のトークは `origin=Onおしゃべり頻度メニュー` 等（＝選択肢 ID）または `origin=OnChoiceTimeout` である。**`choosing` が載っている区間に `origin=OnSecondChange` の `steady_talk` が 1 件も無いこと**を確認する。

```powershell
# 自発トーク起動の全件（origin で弁別する）
Select-String -Path $log -Pattern 'event="steady_talk"' |
  ForEach-Object { $_.Line } | Select-String -SimpleMatch 'origin="OnSecondChange"'
```

- [ ] 上記の出力が**空**、または出力された時刻が「`choice_resolved` 後かつ次の `choice_waiting_established` 前」の区間にのみ存在する（＝選択待ち中の漏れが無い）。

時系列で目視突合したい場合:

```powershell
Select-String -Path $log -Pattern 'choice_waiting_established|choice_resolved|event="steady_talk"' |
  ForEach-Object { $_.Line }
```

（`choice_waiting_established` と `choice_resolved` に挟まれた区間に `origin="OnSecondChange"` の `steady_talk` が挟まっていないことを見る。）

### 4.4 棄却・失敗が正常系で出ていないこと

正常なユーザー操作でエラーレベルのログが出ないこと（Req1.6・タスク 4.6 完了状態）。

```powershell
Select-String -Path $log -Pattern 'choice_rejected_no_wait|choice_rejected_unknown_id|choice_rejected_busy|choice_unsupported_category|choice_shiori_failed_as_204|choice_forward_failed|choice_selection_send_failed|choice_waiting_stale|event_id_not_allowed'
```

- [ ] 上記が**空**である。空でない場合は §5 の特記事項へ全文を転記すること（30 秒放置による `choice_timeout_fired` / `choice_timeout_cancelled` は §3.1 のとおり設計どおりなので、その旨を明記する）。

### 4.5 起動と終了が健全であること

```powershell
Select-String -Path $log -Pattern 'primary_dpi|wire 成立|装着計画を実行|boot_complete|force_quit|id=OnClose'
```

- [ ] `primary_dpi=` が **96 以外**（実 DPI 条件・§0.2）。
- [ ] `emo2-boot: 実 sink 結線が成立しました（wire 成立）` がある。
- [ ] `emo2 attach: 装着計画を実行` がある。
- [ ] `event="boot_complete"` がある（実 pasta が `OnBoot` に応答＝辞書込みフルゴーストが生きている）。
- [ ] 終了時に `id=OnClose` の送出があり、プロセスが exit 0 で終わっている。

---

## 5. 記録欄（**このタスクの完了状態＝以下 2 つが両方埋まっていること**）

### 5.1 メニュー一周の人間サインオフ（Req9.3）

| 項目 | 記入欄 |
|---|---|
| 実施日 | |
| 実施者 | |
| 実行体（絶対パス・ビルド日時） | |
| helper の PE machine（`0x014c` を確認したか） | |
| 実 DPI（`primary_dpi=` の値） | |
| `AREKA_APP_SMOKE_EXIT_MS` の値 | |
| ログ保存先（絶対パス） | |
| **結果**: §3 チェックリスト (0)〜(8) 全通過 → **一周成功 / 失敗** | |
| 失敗した項番と症状 | |

### 5.2 ログ突合（Req9.4）

| 項目 | 記入欄 |
|---|---|
| §4.1 系列突合（3 チェック） | |
| §4.2 `choosing` の実機観測（3 チェック） | |
| §4.3 **選択待ち中の自発トーク漏れなし** | |
| §4.4 棄却・失敗ログなし | |
| §4.5 起動・終了の健全性（5 チェック） | |
| **ログ突合の結果**: 合格 / 不合格 | |

### 5.3 特記事項

（想定外のログ・環境固有の事象・タイムアウト発火の有無などを記す）

---

## 6. 参考: 自動化済みの部分（人間の実施前に確認済み）

- **起動スモークは実測済み**（人間の操作を要さない範囲）: 上記 §1 と同じ絶対パス・同じ env（`AREKA_APP_SMOKE_EXIT_MS=12000`）で `areka.exe` を起動し、**exit 0**・`primary_dpi=120`（実 DPI ≠96）・`wire 成立`・`emo2 attach: 装着計画を実行`・`wrap=BudouxWordWrap`・`boot_start`→`boot_gate`→`boot_talk`→`boot_complete`→`steady_talk_done`→`force_quit`→`OnClose` の系列を確認済み。error レベルのログは 0 件。
  タスク 5（`wire_choice_drain`）の結線を含むビルドが**実際に起動して定常運転へ到達する**ことはこれで押さえられている。
- **未実施（人間の担当）**: §3 のメニュー一周（ダブルクリック以降の実操作は自動化できない）と、それに伴う §4 の選択系ログの突合。
