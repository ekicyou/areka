# 案 A 実機可否ゲート 判定表

- **実施日**: 2026-08-13
- **実施環境**: ディスプレイ 2 台・**拡大率が異なる**
  - primary: `bounds=0,0,2880,1800` `dpi=192`（200%）
  - secondary: `bounds=-2560,195,0,1795` `dpi=144`（150%）
- **ビルド**: debug ／ コミット `16b633b`（＋ i686 ヘルパ）
- **fixture**: emo2（emo2-kakukaku）・2 スコープ（むらさき／エモ）
- **実行**: `AREKA_APP_SMOKE_EXIT_MS` 有界自動終了 ／ `RUST_LOG=info,wintf::ecs::window::zorder_pair=debug,areka=debug`
- **ログ**: 手動操作を含む本走 `gate2-manual.out`（約 3 分半・全項目の操作を実施）／破棄経路の確認 `g8.out`

## 判定

| # | 判定項目 | 結果 | 根拠 |
|---|---|---|---|
| G1 | 描画の生存 | **PASS** | 対照実験で確定（下記「G1 の対照実験」）。owner の有無を唯一の変数にして症状が同一 |
| G2 | クリック透過の生存 | **PASS** | 目視。透明部のクリックが背後アプリへ届き、不透明部はゴーストが受ける |
| G3 | 透過トグルの生存 | **PASS** | 目視。透明部 ⇄ 不透明部の往復で固着しない |
| G4 | タスクバー／Alt+Tab 非露出 | **PASS** | 目視。owner 付与後もゴースト窓は現れない |
| G5 | ドラッグ＋バルーン追従 | **PASS** | 目視。キャラ単独・バルーン単独・追従とも現行どおり |
| G6 | owner 活性化でペア浮上＋隣接 | **PASS** | 目視（キャラをクリックするとバルーンも一緒に浮上）＋ ログ実測（下記） |
| G7 | owned 活性化で owner も浮上 | **PASS** | 目視。バルーンをクリックしてもキャラが他アプリの背後に残らない |
| G8 | 破棄順序の双方向で異常終了なし | **PASS** | `exit 0`・4 窓を 1 バッチで despawn・SHIORI も `unload_clean`・ERROR 0 件 |

**ERROR 総数 0 件・`verify-failed` 0 件**（両走とも）。

## 結論

**G1〜G7 全 PASS。案 A（Win32 owner）を確定とし、補助浮上は不要。**

- タスク 5.1 は**既定のまま**（`ZOrderPairStrategy::OwnerLink { raise_assist: false }`）。
- タスク 5.2（切離しの適用点を早める）・5.3〜5.7（補助浮上／案 B）は**実施しない**。
- よってタスク 6.1 の前提には 5.1 のみを含める（実施されなかった分岐は前提に含めない）。

## 根拠ログ

### 窓の対応（本走）

| entity | 窓 | HWND |
|---|---|---|
| 4v0 | むらさき キャラ | `0x4A0AFC` |
| 3v0 | むらさき バルーン | `0x670AD0` |
| 6v0 | エモ キャラ | `0xBA09CE` |
| 5v0 | エモ バルーン | `0x410AF2` |

### 起動直後（宣言 → 確立 → 是正）

```
[zorder-pair] declared scope=0 char_entity=4v0 balloon_entity=3v0
[zorder-pair] declared scope=1 char_entity=6v0 balloon_entity=5v0
[zorder-pair] owner-established entity=3v0 peer=4v0 owned_hwnd=0x670AD0 owner_hwnd=0x4A0AFC measured_prev=0xBA09CE
[zorder-pair] owner-established entity=5v0 peer=6v0 owned_hwnd=0x410AF2 owner_hwnd=0xBA09CE measured_prev=0x670AD0
[zorder-pair] skip entity=5v0 peer=6v0 reason=AlreadyAdjacent
[zorder-pair] fix  entity=3v0 peer=4v0 insert_after=0xBA09CE measured_next_after_fix=0x4A0AFC
```

`fix` の実測 `measured_next_after_fix=0x4A0AFC` は **scope 0 のキャラ窓そのもの**＝指令が意図どおり効いたことの実測。

### 操作中の隣接（G6・要件 4.4）

手動操作の全期間で `sink-observed` が **15 本**出ており、**すべて `adjacency_ok=true` かつ `behind_foreground=true`**。両スコープのバルーン窓（entity 3v0 が 7 本・5v0 が 8 本）について記録がある。

```
[zorder-pair] sink-observed entity=5v0 adjacency_ok=true foreground=0x4A0AFC behind_foreground=true
[zorder-pair] sink-observed entity=3v0 adjacency_ok=true foreground=0xBA09CE behind_foreground=true
[zorder-pair] sink-observed entity=3v0 adjacency_ok=true foreground=0x2049A  behind_foreground=true
（以下同型・計 15 本）
```

`adjacency_ok` は隣接の正準判定（手前側の最も近い可視の背後が背後側か）そのものである。`foreground=0x4A0AFC`（むらさきキャラ）や `0xBA09CE`（エモキャラ）の行は、**相方スコープのキャラをクリックした直後に、沈んだ側のペアが隣接を保っていた**ことを示す。他アプリ（`0x2049A` 等）を活性化した場合も同じ。

**G6 で `fix` が起動時の 1 本だけである理由**: 案 A では隣接の維持を OS の owner 保証が担い、キャラ窓のクリックで再断行の要求を挿す供給者が居ない（z 変化検知は案 B ／補助浮上でのみ結線する設計）。よって是正の指令は出ない——出ないことが正常であり、隣接が保たれていることは `sink-observed` の `adjacency_ok=true` と目視が示す。

### 破棄経路（G8）

```
areka: smoke 自動 close: 起動窓（ダミー窓／ゴースト窓）を despawn しました count=4
areka: seriko: loop ticker を Close しました（終了順序①・SERIKO 再生ループ停止）
shiori-actor: 正規 clean shutdown 完了（unload → helper 正常終了 exit(0)） event="unload_clean"
EXIT=0
```

`owner-detached` は 0 本。**これは FAIL ではない**——4 窓が同一巡で despawn するため標準機構（相手消滅の検知）は走らない。G8 の PASS 基準は「プロセスが異常終了しないこと」であり、それは満たされている。

## G1 の対照実験（重要）

初回起動で「**バルーンしか表示されない**」という症状が出た。これを G1 FAIL（owner が合成を壊した）と読むと案 A が丸ごと撤去されるため、対照実験で切り分けた。

- **対照**: `cfb645b`（タスク 3.1・`wire_zorder_pair` がまだ無く owner を一切張らない）を同じ機械・同じ fixture でビルドして実行。全窓 `owner=0x0` を実測で確認。
- **結果**: **症状は同一**（バルーンしか見えない）。よって owner は原因ではない。

**真因**: `shiori-host32-helper.exe`（32bit SHIORI ヘルパ）が未ビルドで `areka.exe` の隣に存在せず、SHIORI が接続できていなかった。

```
ERROR shiori-actor: SHIORI 接続確立に失敗——死活報告（ShioriDown）し受信ループに入らず終了
  reason=i686 helper の spawn に失敗: failed to spawn helper process:
         指定されたファイルが見つかりません。(os error 2)
```

SHIORI が死ぬと `OnFirstBoot` が来ずキャラ窓に出すサーフェスが決まらない。一方バルーン窓は背景画像を伺かパッケージから直接読む（`balloons0.png` / `balloonk0.png` の解決がログにある）ため、**バルーンだけ見える**という症状になる。

ヘルパを建てて配置したところ **ERROR 0 件**になり、キャラも表示された。

```bash
cargo build -p shiori-host32-helper --target i686-pc-windows-msvc
cp target/i686-pc-windows-msvc/debug/shiori-host32-helper.exe target/debug/
```

**副産物**: 対照版（owner を一切張らない）でも既定 IME 窓の owner はキャラ窓だった（`class='IME' owner=<char0>`）。**あれは我々が owner を張ったせいではなく Windows の素の挙動**であることが、これで独立に裏付けられた。
