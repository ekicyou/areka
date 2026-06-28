---
inclusion: manual
updated_at: 2026-06-28
---

# Roadmap — areka M1（最小 SSP 互換ベースウェア）

> **このロードマップは M1 のみを扱う。** M2 以降は **M1 完成後に実物を見て組み直す**（憶測で先に書かない）。
> 正本配置: 本ファイルが M1 ロードマップ正本（kiro 標準パス `.kiro/steering/roadmap.md`）。`focus.md`（`inclusion: always`）から辿る。設計判断の正本は [doc/COMPAT_ARCHITECTURE.md](../../doc/COMPAT_ARCHITECTURE.md)。M1 実物スコープは [doc/emo2-conformance-scope.md](../../doc/emo2-conformance-scope.md)。

## M1 ゴール

areka（**x64**）が最小 SSP 互換ベースウェアとして、適合対象ゴースト **emo2**（作者自作・脳=`pasta.dll`・**32bit SHIORI**）を「**そのまま**」起動→会話→撫で→メニュー→終了まで E2E 実走させる。

- emo2 が動く＝同じ汎用 32bit ブリッジで里々/YAYA も動く土台（互換＝普及の入口）。
- 「伺かっぽいマスコット」ではなく「**伺か互換系**」であること自体が長期ロードマップの起点。
- M1 スコープは emo2 が実際に使う機能で**実物定義**（[doc/emo2-conformance-scope.md](../../doc/emo2-conformance-scope.md)）。完全網羅・予測実装はしない。

## 実装規律（balloon-system の失敗から得た正）

旧ロードマップは「**spec が spec を産む工場**」と「**憶測の過剰アーキテクチャ**」で、実装まで進んでも使えない物を生んだ（実例: 旧 `wintf-P0-balloon-system` は子 spec を 8 本産んだだけで動くバルーンはゼロ。`crates` に実装なし）。同じ病を二度と入れない規律：

- **実装ファースト**: 各作業ユニットの成果物は「emo2 が実際に動く」検証済みコード。spec でも、先回りの抽象でもない。
- **spec 工場の禁止**: 成果物が子 spec になる構造を作らない。1ユニット＝1かたまりの動く振る舞い。
- **最小実装＋薄い拡張シーム**: emo2 が使う分だけ実装し、拡張は型/レジストリの口だけ残す。choice/link/text-effect・追加 SERIKO interval/method・SAORI 等は**実装しない**（emo2 未使用）。抽象は「2 例目の実物」が要求してから。
- **動く資産から建てる**: ゼロから再アーキテクトしない（下記「立つ土台」）。

## 立つ土台（completed・実コードあり）

- 透過 ULW/DComp 切替（別プロセス自動透過）、event/hit-test/alpha-mask、dola 演出ランタイム（コア〜ループ/nested）。
- **SHIORI 契約チェーン**: `areka-P0-shiori-com`（内部唯一 ABI `IShiori`）→ `-shiori-protocol` → `-shiori-protocol-split` → `-shiori-reference`（reference DLL・native 脳デモ・`shiori_create` 入口）。
- **mock-shell**（`areka-mock-shell`）: 窓表示＋縦書き Typewriter バルーン＋ドラッグ追従＝**動くレンダリング素材**。
- pasta エンジン source: vendored `vendors/pasta`（M2 の native x64 化の素・**M1 では使わない**）。

## 唯一の耐力壁（先進坑で先に潰す）

「**x64 areka が emo2 の 32bit `pasta.dll` を駆動できるか**」が M1 の生死を分ける。ここだけ先進坑（pilot・使い捨て）で先に検証し、**go 判定（開発者・人間判断）**を取ってから本実装へ。二坑モデル規律は [two-tunnel.md](two-tunnel.md)。

- 先進坑: `crates/pilot/examples/shiori-host-32/`。
- **go 基準**: x64 から 32bit `pasta.dll` を 1 往復（load→OnBoot→`Value` 受領→unload）成功 ＋ 窓持ち SHIORI のメッセージループ生存。SAORI は emo2 未使用ゆえ対象外。

## 横断データ構造（M1 着手時から組み込む）

> 後付けが高コストな基盤構造ゆえ**最初から正しく持つ**（最小実装・構造は拡張可能）。詳細は記憶 areka-unified-shell-balloon-graphics。

- **シェル/バルーン統一**: 描画エンジン上でシェルとバルーンを**区別しない**。バルーン＝シェル surface 上に被さる**文字レンダリング層**。バルーン枠も surface＝普通にアニメ可。
- **element に他サーフェス参照可**: SERIKO の element に画像だけでなく**他サーフェスを指定可**（入れ子）。surface 合成は再帰的＝入れ子アニメの基盤。
- **element 配置＝D2D 変換行列**: 基本 X,Y でなく **D2D 変換行列**を内部表現。x,y は単位行列の特例。回転・拡縮など D2D が普通に出来る構造をそのまま取り込む。emo2 は単位平行移動＋平面 overlay のみ使うが、**構造（行列＋入れ子＋統一エンジン）は最初から持つ**。
- 位置づけ: 旧「汎用シーングラフは M2 後ろ倒し」の**部分的前倒し**（データ構造のみ M1・上位演出エンジンは依然 M2）。
- **アニメーションエンジンは2つ**（記憶 areka-two-animation-engines）: ①**さくらスクリプト再生エンジン**（talk timeline・上層）→ ②**シェルアニメーションエンジン**（SERIKO ループ・中層）→ レンダリングエンジン（下層）。さくら engine が shell engine を叩き、shell engine が render を叩く。両 engine は **dola（完了・タイミング層）**上。`conductor` は SHIORI イベント循環でさくら engine に script を渡す。

## M1 実装ユニット（実現可能な粒度）

> **粒度基準**: 1ユニット＝「コードを走らせて観測できる**単一 pass/fail** を持ち、それを観測するのに別ユニットを先に作る必要がない」もの。done が複数の独立観測に割れるなら粗すぎ→分割。
> 正規名は**暫定**（着手時に確定）。**spec 工場にしない**＝下記はユニット名の登録であり brief.md 群ではない。着手時に最小 spec/task を just-in-time で切る。
> **粒度の真実**: 作業は **M-boot に前倒し集中**（11ユニット＝M1 の山）。「最初の起動」が本体で以降は薄い増分。

### M-boot ＝ `areka-P0-emo2-boot`（最初の可視結果・最重量＝11ユニット）
emo2 が起動して喋る。下記 3 トラックを結線して達成。

**host-32（耐力壁・`pilot/shiori-host-32` がトラックを gate）**
- `pilot/shiori-host-32` — 使い捨て feasibility。✔ go: 32bit pasta.dll 1往復
- `areka-P0-host32-ipc` — x64↔32bit helper＋pipe＋handshake/lifecycle。✔ 往復 echo
- `areka-P0-host32-shiori-load` — LoadLibrary pasta.dll＋load/unload/request 解決＋load(ghostdir)。✔ load 成功・無crash
- `areka-P0-host32-request` — SHIORI/3.0 build＋marshal＋response Value＋charset。✔ x64 が emo2 OnBoot の Value 受領
- `areka-P0-host32-lifecycle` — helper msg loop＋OnSecondChange poll＋unload＋crash監視。✔ N秒運転→clean unload

**parsers（並行・単体テスト可・host 不要）**
- `areka-P0-sakura-parse` — emo2 タグ subset→token。✔ boot script を token 化
- `areka-P0-shell-parse` — surfaces.txt/descript→surface モデル。✔ emo2 shell parse
- `areka-P0-balloon-parse` — balloon descript/Ns マージ→モデル。✔ emo2-kakukaku parse
- `areka-P0-package-mount` — install.txt/dir→mount。✔ emo2 layout 解決

**runtime 制御階層（上→下に駆動・両 anim engine は dola 上）**
- `areka-P0-conductor` — SHIORI イベント循環（OnSecondChange pump・host-32 送受・Value を sakura-engine へ）。✔ OnBoot→Value 受領→再生開始
- `areka-P0-sakura-engine` — **さくらスクリプト再生エンジン**（talk timeline: `\w/\_w` wait・`\s` で shell-engine へ surface 指令・text を text-layer へ・seq）。✔ boot script を時系列再生
- `areka-P0-shell-anim-engine` — **シェルアニメーションエンジン**（SERIKO ループ＋surface 状態＋MAYUNA bind・render を毎フレーム駆動）。M-boot は静的＋指令適用、ループ(blink)は M-life。✔ 指令された surface を表示

**render engine（統一・`areka-mock-shell`＋dola から増分）**
- `areka-P0-surface-engine` — **シェルもバルーンも同一の surface 合成**。element＝{画像 | 他サーフェス参照（入れ子）}、配置＝**D2D 変換行列**。✔ surface0 ＋バルーン枠を surface として表示
- `areka-P0-text-layer` — バルーン文字を **engine 上に被せる層**（token→glyph→surface 領域）。✔ script がバルーンに描画＋scroll

### 増分マイルストーン（M-boot の動く土台へ加算）
- **M-dual** `areka-P0-dual-surface` — side0/1 両立＋`\s[]`＋alias。✔ むらさき＆エモ表情切替
- **M-mayuna** `areka-P0-mayuna-compose` — MAYUNA bind 多層 overlay。✔ むらさき着せ替え合成
- **M-life** `areka-P0-shell-anim-loop`（shell-anim-engine の SERIKO ループ＝✔ まばたき random/bind+random）＋ `areka-P0-collision-touch`（✔ 撫で発火）＋ `areka-P0-idle-talk`（✔ OnSecondChange 自発会話）
- **M-dialogue** `areka-P0-menu-choice` — dblclick メニュー＋`\q`＋OnChoiceSelectEx＋`\_l`＋`\![move]`。✔ 選択対話
- **M-e2e** `areka-P0-emo2-conformance-e2e` — OnClose＋emo2 vendoring＋boot→talk→touch→menu→close 一周。✔ 適合（M1 ゴール `areka-P0-emo2-conformance` 充足）

## 制約

- Rust 2024・マルチクレート（wintf/dola/areka ＋最小依存 `shiori-abi`）。32bit 可搬性を崩さない。
- 透過は ULW/DComp 切替式（実装済み・ULW 既定）。SHIORI 内部唯一 ABI=`IShiori`(COM, HSTRING/UTF-16)。過去互換は 32bit Rust ホスト（flat-C/HGLOBAL/charset/自前 IPC）。
- 設計判断の変更は [doc/COMPAT_ARCHITECTURE.md](../../doc/COMPAT_ARCHITECTURE.md) を正本として更新。

## ポートフォリオ（2026-06-28・clean slate）

- `.kiro/specs/` 直下 active = **0**（憶測仕様を全伐採し更地化。実装ファーストで着手時に作る）。
- `completed/` = 99（歴史・M1 が立つ土台の記録）。
- 旧 active/brief（M1 憶測・M2 reference・出荷層）・backlog（P1-P3）・`_rejected/`・旧戦略メモは**削除**（git 履歴に保全。必要時に復元可）。

## M2 以降

**M1 完成後に、実物を見て組み直す。** 本ロードマップでは扱わない（pasta の native x64・`IShiori` in-proc 化、縦書き・ベクトル描画・AI、互換面拡大＝Shift_JIS/SAORI/里々・YAYA 網羅/NAR 等はその時に）。
