# ディスカッション記録 2026-06-28 — M1 ロードマップ再設計

> ⚠️ **これは 2026-06-28 時点の議論スナップショット**（陳腐化し得る）。
> **正本は [.kiro/steering/roadmap.md](../.kiro/steering/roadmap.md)**（M1 専用）／設計判断は [doc/COMPAT_ARCHITECTURE.md](COMPAT_ARCHITECTURE.md)（旧 framing 残存・追従保留）／実物スコープは [doc/emo2-conformance-scope.md](emo2-conformance-scope.md)。
> 食い違ったら**この文書でなく正本を見ること**。
>
> 添付図: [roadmap-tree.svg](discussion-260628-roadmap-tree.svg) ／ [concurrency-model.svg](discussion-260628-concurrency-model.svg)

## 0. この日やったこと（要約）

ロードマップの大整理と M1 アーキテクチャの確定。きっかけは「未着手ロードマップが過剰で実現性が薄い」という問題提起。

1. **棚卸し**: 孤児ファイル除去・件数整合・二坑モデル反映。
2. **M1 ゴール再設定**: 「伺かっぽいマスコット」→ **「最小 SSP 互換ベースウェア」**。適合対象は作者自作ゴースト **emo2**。
3. **emo2 実測解剖**: M1 スコープを推測でなく実物で定義（→ emo2-conformance-scope.md）。
4. **balloon-system が "spec 工場" と判明**（子 spec を8本産んだだけで実装ゼロ）→ **未着手仕様を全伐採し更地化**。
5. **ロードマップを M1 専用 lean 版へ全面書換**。
6. **アーキテクチャ確定**: 統一グラフィック／2アニメエンジン／構築モデル／全7トラック／実現可能粒度／エンジン別並走／クロスエンジン I/O／並行モデル／マウス所有。

## 1. M1 ゴール

areka（**x64**）が最小 SSP 互換ベースウェアとして、**emo2**（作者自作・脳=`pasta.dll`・**32bit SHIORI**）を「そのまま」起動→会話→撫で→メニュー→終了まで E2E 実走させる。
emo2 が動く＝同じ汎用 32bit ブリッジで里々/YAYA も動く土台（互換＝普及の入口）。M1 スコープは emo2 が実際に使う機能で実物定義（完全網羅・予測実装はしない）。

## 2. 実装規律（balloon-system の失敗から）

- **実装ファースト**: 成果物は「emo2 が実際に動く」検証済みコード。spec でも先回りの抽象でもない。
- **spec 工場の禁止**: 成果物が子 spec になる構造を作らない。1ユニット＝1かたまりの動く振る舞い。
- **最小実装＋薄い拡張シーム**: emo2 が使う分だけ実装、拡張は型/レジストリの口だけ。
- **動く資産から建てる**: `areka-mock-shell`（窓＋Typewriter）・`areka-P0-shiori-reference`（native 脳デモ）・透過 ULW・dola から増分。
- **brief は前もって量産しない**: 着手時に1本ずつ just-in-time（`/kiro-discovery` 再入 or `/kiro-spec-init`）。`/kiro-spec-batch` は使わない。

## 3. 基盤データ構造（M1 着手時から組み込む）

- **シェル/バルーン統一**: 描画エンジン上で区別しない。バルーン＝シェル surface 上の文字層。バルーン枠も surface＝アニメ可。
- **element に他サーフェス参照可**（入れ子）→ surface 合成は再帰的。
- **element 配置＝D2D 変換行列**（x,y は単位行列の特例・回転/拡縮そのまま）。emo2 は単位平行移動＋平面 overlay のみ使うが、構造は最初から持つ。
- 旧「汎用シーングラフは M2」の**部分的前倒し**（データ構造のみ M1・上位演出エンジンは M2）。

## 4. アニメーションエンジンは2つ（フォーク）

```
conductor（SHIORI イベント循環）
  └ ①さくらスクリプト再生エンジン（talk timeline）
       ├─(shell anim: \s 等)→ ②シェルアニメエンジン（SERIKO ループ）→ surface 合成 ┐
       └─(text: typewriter)─────────────────────────────→ text-layer ──────────────┤→ render 合成→画面
```
テキスト描画はシェルアニメではないので、さくら engine は text を render(text-layer) へ**直接**指令。両エンジンは **dola（完了・タイミング層）**上。

## 5. 構築（初期化）モデル — 各エンジンのコンストラクタ

- **root（親）**: `install.txt`（ルート定義）。
- **ghost**: `package-mount` が構築。実行時 owner ＝ **ゴーストエンジン**。
- **SHIORI/host-32**: コンストラクタ＝ゴーストフォルダ定義（`ghost/master/descript.txt`）。
- **shell-anim-engine**: コンストラクタ＝SERIKO/shell 定義（`surfaces.txt`）＋ balloon 定義（統一ゆえ両方）。
- **sakura-engine**: コンストラクタ＝さくらスクリプト（runtime・per-talk・transient）。
- 2系: load-time（root→ghost→{SHIORI, shell-anim}）と runtime（script→sakura）。

## 6. 全7トラック（→ roadmap-tree.svg）

| # | トラック | 役割 | 増分 |
|---|---|---|---|
| ⓪ | ゴーストエンジン | 最上位 owner（lifecycle/窓配置/位置永続化・統括） | position-persist |
| ① | SHIORI 通信層（host-32） | 32bit pasta.dll・耐力壁（pilot で gate） | なし（M-boot 完了） |
| ② | parser / loader | 定義→model・構築入力 | なし（M-boot 完了） |
| ③ | conductor | SHIORI イベント循環 | idle-talk / input-events |
| ④ | sakura-engine ① | さくらスクリプト再生 | sakura-dialogue-tags |
| ⑤ | shell-anim-engine ② | SERIKO ループ＋surface | dual-surface / mayuna-compose / shell-anim-loop |
| ⑥ | render-engine | surface＋text 合成＋**総合マウス制御** | collision-geometry / choice-render / dual-window |

## 7. 実現可能粒度（M-boot＝最重量・約16ユニット）

粒度基準＝「走らせて観測できる単一 pass/fail を持ち、観測に別ユニットを要さない」。M-boot（emo2 が起動して喋る）に作業が前倒し集中、以降は薄い増分。マイルストーン M-dual/M-mayuna/M-life/M-dialogue/M-e2e は**エンジン横断の統合点**であって作業単位ではない。

## 8. 並走とクロスエンジン I/O

- **並走安全（即着手可）**: idle-talk / mayuna-compose / shell-anim-loop / position-persist。
- **クロスエンジン結合（I/O 契約を先決→並列）**:
  - 撫で（M-life）: render:collision-geometry（入力）⟷ conductor:input-events（→SHIORI 出力）。契約＝region/actor。
  - 選択肢（M-dialogue）: sakura:dialogue-tags ⟷ render:choice-render ⟷ conductor:input-events。契約＝\q/選択。
  - 二人立ち（M-dual）: shell-anim:dual-surface ⟷ render:dual-window ⟷ ghost:window-placement。契約＝2窓/surface。
  - 移動（\![move]）: sakura:dialogue-tags ⟷ ghost:window-placement。
- **マウス**: 総合マウス制御＝render の責務。低レベル（窓 msg/alpha hit-test/drag）は完了基盤。M1 新規は collision-geometry のみ。
- **owner-draw 右クリックメニューは M2**（M1 メニュー＝balloon \q 選択肢）。

## 9. 並行モデル（→ concurrency-model.svg）

- 各エンジン層＝**チャンネル通信のアクターモデル**、**エンジンインスタンスごとに独立スレッド**（async 中心でなくスレッド独立）。
- **エンジン間 I/O 契約＝channel メッセージ型**（§8 の契約がそのまま channel）。
- **但し ⑥render/window は UI スレッド固定**（D2D 単一スレッド＋window アフィニティ）＝他 actor は worker から channel で render へ。
- ①host-32 は別プロセス＝天然のアクター境界（IPC）。④sakura は per-talk transient。

## 10. 次の一手

規律どおりなら **`pilot/shiori-host-32`（耐力壁の go：x64 から 32bit pasta.dll を 1 往復）** か、**結合クラスタの channel 契約を1つ定義**。brief は着手時に just-in-time で1本ずつ。

## 11. この日のコミット列（areka ブランチ）

`docs(roadmap): ロードマップ棚卸し` … →
`M1を最小SSP互換へ再carving` → `M1集中・全伐採で更地化` → `実現可能粒度（18ユニット）` →
`基盤アーキテクチャ確定（統一グラフィック＋2アニメ）` → `さくらengineフォーク` →
`エンジン構築モデル` → `増分エンジン別再帰属` → `SHIORI通信層engine格上げ・全7トラック` →
`ゴーストエンジン追加` → `クロスエンジンI/O依存チェック` → `マウス制御render所有・owner-draw M2` →
`並行モデル指針` → `着手手順（just-in-time briefing）`。

## 関連記憶（user auto-memory）

areka-roadmap-drift / areka-compat-baseware-strategy / areka-unified-shell-balloon-graphics /
areka-two-animation-engines / areka-engine-construction-model / areka-concurrency-model。
