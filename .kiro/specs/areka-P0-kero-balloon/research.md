# ギャップ分析: areka-P0-kero-balloon

> 実施: 2026-07-31 / worktree `claude/kiro-start-areka-p0-kero-1f9ab7`（HEAD `969a9b3`）
> 入力: `requirements.md`（確定）・`brief.md`・`.kiro/steering/{product,tech,structure,logging,roadmap}.md`
> 本文の `path:line` は **本 worktree で本日実読した実測値**（brief 追記(52) のアンカーを再検証・差異は本文で明示）。
> 本書は **情報提供であり決定ではない**。判断は「9. 設計判断項目」へ集約し、要件ディスカッション／設計フェーズへ送る。

---

## 1. 分析サマリ

- **既存パターンは 3 層に整っている**——(a) パーサ層（`areka-parsers::balloon`）は `balloonk0s.txt` の 2 層マージを**既に檻化済み**（`validation_tests.rs:137`）、(b) 起動時資産（`assets.rs`）は既に **scope ごとにループして** balloon target を構築、(c) 採寸（`measure.rs`）と文字層再追従（`frame.rs:928`）は W4 が **per-scope の席を明示コメント付きで保全済み**（`measure.rs:127-128` `:227-231`・`frame.rs:194-205` `balloon_models: HashMap<u32, BalloonModel>`）。**構造の新設ではなく、既存ループへ「scope→系列」という引数を 1 本通す作業**が本仕様の主形。
- **欠けているのは 3 つだけ**——① 画像列挙の prefix が `balloons` **定数固定**（`areka-emo-present/src/balloon.rs:39`・`frame_id`/`enumerate_frames` は private・公開口は `build_balloon_target` の 1 本のみ）、② 面別上書きファイル名が `"balloons0s.txt"` **定数固定**（`assets.rs:79`）＋ `BalloonModel` が全 scope 共有 1 本（`assets.rs:122`）、③ 採寸が `balloons0.png` **文字列固定**（`measure.rs:402` `:409`）かつ scope ループの**外**で 1 回だけ（`measure.rs:179-180`）。
- **実測で「効く」ことが確定**——fixture `emo2-kakukaku` の PNG IHDR 実読: `balloons0.png` = **400×224**、`balloonk0.png` = **288×203**。すなわち per-scope 採寸化は **scope1 のバルーン窓寸を実際に変える**（見た目に出る）。同時に `measure.rs` の共有寸前提テスト（`:564-567` `:1165-1168`・定数 `:534-536`）は**必ず赤くなる**＝R7.2 の「矛盾テスト更新」は必須作業として確定。
- **未着手の新規能力が 1 つ紛れている（最大の争点）**——R3.2 は「相方側定義の `windowposition` から相対位置を決定する」と書くが、**現行 placement は `windowposition` を一切消費していない**。バルーン位置は shell descript の `balloon.alignment` ＋ `balloon.offsetx/offsety` による **DD7 暫定規則**（`placement/resolver.rs:109-113` `:181-203`・`placement/config.rs:47-50`）で決まっており、`windowposition()` アクセサの本番消費点は**ゼロ**（`assets.rs:447` のテスト assert のみ）。これは「per-scope 化」ではなく**正典配置規則の新規実装**であり、規模とリスクの支配項になる。
- **W4 申し送りの穴は実在・箇所も 1 行で特定済み**——`areka-emo-text/src/actor.rs:367-375` が `k_old == k_new` **だけ**で再構築を打ち切る。`binding.image_size`／`surface_size`／`model` 由来の文字描画領域が変わっても k が同値なら `false` を返す＝R4.4 が指す穴そのもの。per-scope model 化はまさにこの経路を踏む（同じ k で model と image_size が scope ごとに違う）ため、**本仕様が直さないと per-scope 定義が文字層へ届かないフレームが生じ得る**。

---

## 2. 正典（ukadoc）確認 — 本日 MCP で実取得

出典: `ukadoc:manual_balloon`（UKADOC Project バルーン）／`ukadoc:descript_balloon` の `windowposition.x` / `windowposition.y` / `windowposition.limit`。

| 正典事項 | 原文要旨 | 本仕様での扱い |
|---|---|---|
| `balloonk*.png` | 「相方側（または三人目以降）の吹き出し。**このファイルは省略可。省略時は本体側の対応するIDのものが使われる**」 | R1.1〜R1.4 の根拠。「**対応するID**」＝ID 単位フォールバック（R1.3）を正典が明示している＝areka 裁量ではない |
| `balloonp*def*.png` | 「省略時は `balloonk`、さらになければ `balloons`」＝3 段連鎖 | R1.6 で Out（M1 は二人立ち）。**正典の 3 段連鎖の存在自体は語彙として記録すべき**（記憶: 先送りは完全語彙＋縮退シーム） |
| `balloons*s.txt` / `balloonk*s.txt` | 「**対応するIDのサーフェス**（balloons1.png に対して balloons1s.txt）について用意すると、**そのサーフェスに対して** descript.txt の記述を上書きする形で適応される」 | R2.1/R2.2 の根拠。**注目**: 正典は上書き層を「画像ファイルに対応するもの」と定義している——ゆえに R2.3（フォールバックで本体側画像を採ったら本体側 `balloons{ID}s.txt` を上書き層に使う）は**正典沈黙ではなく正典の自然な帰結**と読める。要件は「areka 裁量」と分類しているが、対応表（R7.4）へは「正典文言に整合」と添えて書ける（→ 判断項目 D5） |
| 偶数/奇数 ID | 「偶数番のIDは左向き、奇数番のIDは右向きとして二つセット。ただし**自由**」 | R2.6/Out of scope。正典自身が「自由」と言うため、面 0 固定は互換上安全 |
| `windowposition.x` | 「バルーンの**基本位置からの** x 方向の位置調整。数値指定の場合**シェル側が +**、シェルから離れる側が −。`center`/`top`/`bottom` キーワード可（SSP）」 | R3.2 の意味論。**符号がシェル基準**である点が重要（左右どちらのキャラでも「+ がシェル寄り」） |
| `windowposition.y` | 「下が + で上が −。基本位置は x の指定で変わる: 数値指定……**バルーンとシェル画像の上端が重なる位置**」 | R3.2。数値指定時の基本位置＝**上端揃え**＝現行 DD7 の `balloon_y = char_y` と同義 |
| `windowposition.limit` | 画面内維持の 0/1（既定 1） | 要件未言及。現行も未実装（→ 判断項目 D4 の縮退記録候補） |
| `balloon.defaultsurface` / `kero.balloon.defaultsurface` | ghost descript のバルーン既定サーフェス番号（既定 0）。`char*.balloon.defaultsurface` も存在 | R2.6 が「面 ID 0」と固定するのは**既定値としては正典一致**。ただし宣言があれば従うのが正典。emo2 fixture は**両キーとも無宣言**（`ghost/master/descript.txt` 実測）＝現状は差が出ない（→ 判断項目 D6） |

**fixture 実測（`crates/pilot/examples/shiori-host-32/fixtures/emo2/emo2-kakukaku/`）**

```
balloons0.png  9440B  400 x 224   balloons0s.txt  354B
balloonk0.png  8590B  288 x 203   balloonk0s.txt  216B
balloonc1..4.png / arrow0,1.png / marker.png / online0..3.png / sstp.png / sstp_new.png / descript.txt / install.txt
```

- `balloons0s.txt`: `windowposition.x,266` `windowposition.y,-129` / `wordwrappoint.x,-49` / `validrect 46,-56,36,-44`
- `balloonk0s.txt`: `windowposition.x,-190` `windowposition.y,-75` / `validrect 40,-70,24,-48` / **`wordwrappoint` 行なし**（＝descript 継承 `-34` が生き証人・`validation_tests.rs:153`）
- **`balloonk1.png` は存在しない**＝R1.3 の「ID 単位フォールバック」を実 fixture で檻に入れるには **合成 fixture（TempDir）が要る**（既存 `balloon.rs` テストの `TempDir`＋`MemoryDecoder` 流儀がそのまま donor）
- `balloonc0.png` は無く `balloonc1..4` のみ（既存 `frame_id` の除外テストが `balloonc0.png` を使うのはテスト内合成名）

---

## 3. 要件 → 既存資産マップ（gap タグ付き）

タグ: **Missing**＝実装が無い／**Constraint**＝既存構造の制約が形を縛る／**Unknown**＝設計で要調査

### R1: scope 別バルーン系列の解決とフォールバック

| AC | 既存資産（実測 path:line） | gap |
|---|---|---|
| 1.1 系列割当 | `areka-emo-present/src/balloon.rs:39` `const FRAME_PREFIX: &str = "balloons"`／`:88` `fn frame_id`（private）／`:50` `fn enumerate_frames`（private）。**公開口は `:120` `build_balloon_target(balloon_dir, decoder)` のみ**（prefix も scope も引数に無い） | **Missing**（prefix パラメタ化＋公開 API 形状の決定が要る） |
| 1.2/1.3 ID 単位フォールバック | 該当ロジックなし。`enumerate_frames` は 1 パスの単純列挙＋`sort_unstable_by_key` | **Missing** |
| 1.4 `balloonk*` 皆無時の後方互換 | 現行が常に `balloons*` 一択ゆえ**この分岐が現行挙動と一致**する（回帰の基準線） | 充足可能（設計で自明に担保） |
| 1.5 非枠除外 | `:88-93` `frame_id` が `balloons{N}.png` 厳密判定＝`balloonc*`/`arrow*`/`marker*`/`online*`/`sstp*` を自然排除。テスト `:258-271` が固定 | **Constraint**: prefix 可変化後も **`balloonc` を `balloonk` と取り違えない**判定が要る（8 文字目 `c`/`k`/`s` の一致だけで分岐すると `balloonc*` を相方系列に誤採用する事故が起きる） |
| 1.6 `balloonp*def*` 非対応 | 実装なし | 設計で明示 Non-Goal＋語彙記録 |
| 1.7 面 0 皆無で起動失敗 | `balloon.rs:124-131` 枠 0 枚→`error!`＋`PresentError::Compose(EmptyComposition(0))`。上流 `assets.rs:282` が `?` で `BootWiringError::Balloon` へ。**ただし** `emo2_boot/mod.rs` の `wire_emo2_boot` は失敗時 `fallback()`＝**unwired（LogSink×2）へ縮退**し、プロセスは死なない | **Unknown**: 「起動を失敗させる」の実装上の意味（unwired 縮退で足りるか／`PlacementError` 側は `measure.rs:409` で hard `Err`→`spawn_dummy_window` 縮退、**2 経路で挙動が違う**） → 判断項目 D7 |

### R2: scope 別バルーン定義（面別上書き）の適用

| AC | 既存資産 | gap |
|---|---|---|
| 2.1 scope 専用定義の保持 | `assets.rs:122` `pub balloon_model: BalloonModel`（doc に「**全 scope 共有**」明記）／`:285` `build_balloon_model(balloon_root)` を**1 回だけ**呼ぶ／`:279` コメント「1 回組み全 scope 共有する」 | **Missing**（保持器の形＝判断項目 D2） |
| 2.2 `balloonk{ID}s.txt` 適用 | `assets.rs:79` `const BALLOON_FACE0_TXT: &str = "balloons0s.txt"`（**面 0・本体側固定**）／`:322-326` `build_balloon_model` が `descript.txt`＋`BALLOON_FACE0_TXT` を `parse_str` へ | **Missing**（ファイル名導出の関数化） |
| 2.3 フォールバック時の上書き層 | なし | **Missing**（正典整合の判断＝D5） |
| 2.4 上書きファイル不在の寛容 | `assets.rs:330-342` `read_decoded_lenient` が `warn!`＋`None`／`parse_str(descript, None)` は `Result` 無し寛容 | **充足済み**（そのまま再利用可）。ただし「相方に面別ファイルが無いのは正常」なので **warn の妥当性を再検討**（現行は読取失敗を一律 warn＝相方側で常時鳴る恐れ） → D8 |
| 2.5 未指定項目の継承 | `areka-parsers::balloon::parse_str` の 2 層後勝ちマージ。`validation_tests.rs:95`（sakura）`:137`（kero）で檻化済み。**パーサ改造不要**（要件 Adjacent expectations と一致） | **充足済み** |
| 2.6 初期表示面 = ID 0 | `frame.rs:531-540` が `ShowSurface{surface_id: 0}` を無条件発行 | **充足済み**（ただし `kero.balloon.defaultsurface` 非対応＝D6） |

### R3: 窓配置採寸の scope 別化

| AC | 既存資産 | gap |
|---|---|---|
| 3.1 scope 別採寸 | `measure.rs:179-180` `let balloon_size = measure_balloon_surface0(...)` が **scope ループ（`:183`）の外**で 1 回／`:213-217` 全 scope へ同一値を push／`:371` `measure_balloon_surface0` は `balloon_root` のみ受け取り `:402` `:409` で `"balloons0.png"` を大小無視固定名照合 | **Missing**（ただし移設は容易・ループ内へ落とすだけ） |
| 3.2 相方寸＋**`windowposition`** で相対位置 | `placement/resolver.rs:109-113`（DD7 コメント）`:181-203` が `balloon.alignment`（Left→`char_x − balloon_w`／Right→`char_x + char_w`）＋`balloon.offsetx/offsety` で `balloon_pos` を決定。`placement/config.rs:47-50` `:221-241` がその KV カスケード。**`BalloonModel::windowposition()` の本番消費点は 0 件**（全文 grep: `assets.rs:447` のテスト assert とパーサ自身のテストのみ） | **Missing（新規能力・最大の争点）** → D1 |
| 3.3 k 適用は既存権威のまま | `measure.rs:233` `apply_scaling`／`:248` `scale_scope_input`／`:279` `scale_size_px`→`ScaleRatio::scaled_extent` 単一権威。`:224-231` に「per-scope 写像ゆえ kero-balloon が改造しても**本段は構造変更を要さない**」の申し送りコメント実在 | **充足済み**（席保全が効いている） |
| 3.4 `balloonk*` 不在時の同一性 | 同上（全 scope 同一寸へ収束） | 設計で自明に担保 |
| 3.5 原点・保存/復元の不変 | `resolver.rs:74-81` の `balloon_offset ≡ balloon_pos − char_pos` 恒等式（恒久事後条件）／`placement/persist.rs` | **Constraint**: D1 で `windowposition` を導入するなら**この恒等式を壊さない**位置に入れる必要がある |

### R4: バルーン文字層の scope 別追従

| AC | 既存資産 | gap |
|---|---|---|
| 4.1 scope 定義で領域解決 | `frame.rs:549-556` `connect_balloon_text(runtime, view, ActorKey::from(scope.to_string()), &balloon_model)`（**共有 model を全 scope へ**）／`areka-emo-text/src/actor.rs:289` `register_actor_view`→`:303` `register_actor_binding`→`ResolvedBalloonText::resolve(model, binding.image_size)` | **Missing**（model 差し替えのみ・emo-text 改造不要） |
| 4.2 装着と再追従で同一写像 | `frame.rs:552-554` に警告コメント実在／`frame.rs:948` `ActorKey::from(scope.to_string())`（再追従側）と `:554`（装着側）が現状一致 | **Constraint**（**維持義務**。per-scope 化で片方だけ変えない） |
| 4.3 k 変化時の再構築 | `actor.rs:336` `refresh_actor_scale`→`:379-382` `register_actor_binding`＋`surfaces.remove(actor)` | **充足済み** |
| 4.4 **k 同値でも寸/領域変化なら再構築** | `actor.rs:367-375`: `let (k_old, k_new) = (current.scale, binding.scale); if k_old == k_new { return false; }` ← **k のみ比較**。`TextSlotBinding`（`:47-63`）は `slot`/`window`/`scale`/`surface_size`/`image_size` を持つのに **`image_size`/`surface_size` を比較していない** | **Missing（W4 申し送りの穴・本仕様担当）**。修正は「比較対象の拡張」で足りる見込みだが、`model` 由来の validrect 変化は binding に現れない（`ResolvedBalloonText` 側）ため**比較キーの選定が要設計** → D3 |
| 4.5 no-op 維持（churn 禁止） | `actor.rs:368-375` の churn ガード／テスト `actor.rs:2665` `refresh_actor_scale_with_same_k_is_noop_returning_false` | **Constraint**: D3 の比較拡張で**この檻の意味が変わる**（「k 同値なら常に false」から「k・寸・領域すべて同値なら false」へ）＝テスト名/意図の更新が要る（R7.2 の対象） |
| 4.6 未装着 actor は静穏 skip＋ログ | `actor.rs:354-363` `debug!`＋`false`／`frame.rs:949-960` の `text_scale_warned` エッジガード（scope ごと 1 回 warn） | **充足済み** |
| 4.7 純粋状態の保存 | `actor.rs:377-382` の doc/実装（`routing`＋`layout_input` のみ上書き・`TextLayerState` 不触） | **充足済み** |

### R5: 既存挙動の非回帰

| AC | 既存資産 | gap |
|---|---|---|
| 5.1/5.2 `\b` 経路無改変 | `spine.rs:941` S3（`\b[-1]`→Hide／`\b[0]`→ShowSurface の実 readback 檻）／`:1254` S4（`\b` 不在時 balloon 宛指令 leak なし） | **Constraint**: 無改変で緑維持。**S4 の doc コメント `:1249-1253` が「emo2 fixture は balloons0.png のみ」と述べており、本仕様で陳腐化する**（fixture には `balloonk0.png` が実在し、本仕様後は scope1 がそれを使う）→ doc 更新対象 |
| 5.3 可視ライフサイクル不変 | `frame.rs:531-540` の無条件 `ShowSurface` は W6 `balloon-visibility` の領分 | **Constraint**（触らない・ただし同一関数域＝W6 が後着で再突合） |
| 5.4/5.5 後方互換・本体側不変 | — | テストで担保（下記 7 章） |
| 5.6 面テーブルの系列整合 | `assets.rs:291-300` `LoopTables { shell, balloon }`＝**balloon 表は `balloons[0]`（scope0 の World）から 1 本**。消費側 `areka-seriko/src/looper.rs:43-49` `SerikoLoopConfig { shell_table, balloon_table }` も **scope 非依存の 1 本**（`:180` `:236` で `Slot::Balloon → &*balloon_table`） | **Unknown/Constraint**: per-scope 系列化すると理屈上 balloon 表も scope 別。ただし **synthetic surfaces.txt にアニメ定義は無く emo2 の balloon 表は常に空**（`looper.rs:46` doc「emo2 は空」）＝実害ゼロ。`SerikoLoopConfig` の scope 別化は **`areka-seriko` へ波及＝brief の Boundary Candidates 外** → 判断項目 D9 |

### R6: 観測性

`.kiro/steering/logging.md` の規約（`error!`/`warn!`/`info!`/`debug!`・構造化フィールド・`[scope_prefix]` 書式）に沿う。既存の同型先例が豊富: `balloon.rs:52-57`（走査失敗 error）・`assets.rs:334-339`（読取失敗 warn）・`measure.rs:159-165`／`:376-386`（採寸失敗 error）・`frame.rs:559-565`（`planned/attached/missing/unused` の集計 info）。**gap は「何を出すか」の設計のみで、機構は既存**。R6.2（フォールバックを warn）／R6.3（`windowposition`/`validrect` 実値ログ）は新規ログ点。

### R7: 検証と正典整合

- R7.1 の檻可能領域は**全て純関数化可能**（系列選択・ID 単位フォールバック・ファイル名導出・マージ実値・採寸の per-scope 写像）。GPU/COM 不要の in-source `#[cfg(test)]` で網羅可（記憶: 決定論的テスト網羅は必達／檻に入れるのは判断分岐のみ）。
- R7.3 実機サインオフ: 記憶 `areka-emo2-signoff-needs-absolute-paths` ＋ `areka-real-machine-signoff-bounded-auto-exit`（`AREKA_APP_SMOKE_EXIT_MS` ＋ `RUST_LOG` grep）。**R6 のログ設計を先に固めると、目視に加えてログ grep で決定論判定できる**（枠形状の差 = 400×224 vs 288×203 は窓寸ログで数値確認可能）。
- R7.5 workspace 緑: 記憶 `workspace-test-needs-i686-host32-artifacts`（DoD 前に i686 helper ビルドが要る）。brief 末尾の「wintf GPU クラッシュで赤」注記は**失効済み**（`wintf-gpu-test-crash` 完遂済み）。

---

## 4. 現状の欠陥連鎖（3 経路が同じ 1 つの原因を共有）

```
balloon_root（1 個のディレクトリ）
   ├─(A) assets.rs:280-284  for scope { build_balloon_target(balloon_root) }   ← 毎回同じ引数＝全 scope 同一枠
   │        assets.rs:285   build_balloon_model(balloon_root)                  ← 1 回・"balloons0s.txt" 固定
   │           └→ frame.rs:545 balloon_models.insert(scope, model.clone())     ← 席はあるが中身は同一
   │              frame.rs:549-556 connect_balloon_text(.., &balloon_model)    ← 全 scope 同一 model
   │                 └→ emo-text actor.rs:309 ResolvedBalloonText::resolve(model, image_size)
   └─(B) measure.rs:179-180 measure_balloon_surface0(balloon_root)             ← ループ外・"balloons0.png" 固定
            └→ measure.rs:213-217 全 scope へ同一 balloon_size
               └→ resolver.rs:181-203 balloon_pos = alignment 由来（windowposition 不参照）
```

**(A) と (B) は独立に balloon ディレクトリを列挙する 2 実装**（`balloon.rs:50 enumerate_frames` と `measure.rs:390-409` の最小再実装）。現状は両者が `balloons0.png` を固定名で見るため**偶然一致**している。per-scope 化で**両者の解決規則がずれると、窓寸（採寸）と実際に合成される枠（装着）が食い違う**——バルーンが窓からはみ出す／余白が出る、という実機でしか見えない欠陥になる（記憶: 実機サインオフは檻が隠す欠陥を炙り出す）。→ **単一権威化が設計の要**（判断項目 D2）。

---

## 5. 実装アプローチ（A/B/C ＋ 直交する 2 つの下位選択）

### アプローチ A: 「scope→系列」を最下流の純関数へ集約し、上位は引数を通すだけ（brief 推奨案の精緻化）

- `areka-emo-present/src/balloon.rs` に **公開の系列解決 API** を追加（例: `BalloonSeries { Sakura, Kero }` ＋ `resolve_faces(dir, series) -> Vec<(u32, String)>` 相当）。`enumerate_frames`/`frame_id` を prefix パラメタ化し、**ID 単位フォールバックをこの純関数の内側**に閉じる。`build_balloon_target` は series 引数を取る形へ拡張（既存呼び出しは `Sakura` 相当で無改変等価）。
- `assets.rs`: balloon ループ（`:280-284`）で scope から series を決め、`build_balloon_target(.., series)`＋**採用面 ID ごとの上書きファイル名**を導出して `build_balloon_model` を per-scope 化。`BootAssets.balloons`（現 `Vec<(u32, EmoWorld, AtlasTable)>`・`:120`）を **`ScopeAssets` 対称の struct** へ昇格し `model` を同梱（`balloon_model` 単数フィールド `:122` は撤去）。
- `measure.rs`: `measure_balloon_surface0` を **同じ公開 API の消費者**へ書き換え、scope ループ内へ移設（`:179-180`→ループ内）。**(A)/(B) の二重実装を解消**。
- `frame.rs`: `balloon_models.insert(scope, ...)`（`:545`）へ per-scope model を挿す。`connect_balloon_text` の第 4 引数を per-scope model へ（`:555`）。**`run_text_scale_phase`（`:928` `:964`）は無改変で per-scope 化が効く**（マップから引くだけ）。
- `areka-emo-text/src/actor.rs:367-375` の比較キーを拡張（R4.4）。

**Trade-offs**: ✅ 正典のファイル名規約に素直／✅ `\b` の面 ID 意味論を保つ（各系列が独立に 0..N を持つ）／✅ 採寸と装着が**同一関数**を通るので (A)/(B) ずれが構造的に起きない／✅ 既存 per-scope ループの形をそのまま使う。❌ `BootAssets` の形が変わり **ripple 8 箇所**（下記 6 章）／❌ `areka-emo-present` の公開 API が増える。

### アプローチ B: 単一 World へ両系列を面 ID 合成（brief B 案）

`balloons0→id0`／`balloonk0→id1`（または別 ID 空間）で 1 つの balloon World に同居させ、scope は初期面 ID だけ変える。
**Trade-offs**: ✅ World 構築 1 本のまま。❌ **正典と衝突**（`balloons*` と `balloonk*` は別系列で各々 0..N を持ち、偶奇は左右向きの意味を持つ）。`\b[N]` の N の意味が歪む＝`completed/areka-P0-balloon-face-cue` の完成領域の意味論を壊す。❌ 面別上書きファイルの対応も歪む。**非推奨**（正典乖離が受け入れ条件 R5.1/R5.2 と直接衝突）。

### アプローチ C: assets/measure の各所で個別に prefix 分岐（最小差分）

`balloon.rs` を触らず、`assets.rs`／`measure.rs` がそれぞれ「相方なら `balloonk0.png` を探す」分岐を持つ。
**Trade-offs**: ✅ 下流クレートの公開 API を増やさない。❌ **4 章の (A)/(B) 二重実装が固定化**（規則が 2 箇所に分散＝将来の面追加・`balloonp*` 拡張で必ずずれる）。❌ `frame_id`/`enumerate_frames` は private ゆえ、結局 `balloon.rs` 内に相当ロジックを再実装することになる。❌ 記憶「先送りは完全語彙＋縮退シーム」に反し、`balloonp*def*` への拡張シームが残らない。**非推奨**。

### 直交する下位選択 1: `windowposition` の扱い（R3.2）

- **C1: 本仕様で正典配置を実装**（`windowposition.x/y` の数値指定＋基本位置＝上端揃えを `resolver.rs` へ導入し、DD7 の alignment 規則を置換または上書き）。→ R3.2 を字義どおり満たす。規模 +M、リスク +（`balloon_offset` 恒等式・位置永続 `persist.rs`・W6 `balloon-visibility`／W4 position-persist との相互作用）。
- **C2: per-scope 定義の供給までを本仕様、配置規則は DD7 のまま**（＝相方が相方の**寸法**で配置される・`windowposition` は保持とログのみ）。→ 実機では「枠形状が違う」は達成、「位置規則が正典」は未達。要件文言（R3.2）との整合を要件ディスカッションで確定する必要がある。
- **C3: `center`/`top`/`bottom` キーワードは Out・数値指定のみ実装**（正典の部分実装＋縮退シーム＋対応表記録）。C1 と C2 の中間。

### 直交する下位選択 2: `BalloonModel` の保持器（R2.1）

- **S1: `BalloonScopeAssets` struct 新設**（`scope`/`emo_world`/`atlas`/`model`/`initial_surface_id`）＝`ScopeAssets`（`assets.rs:87`）と対称。可読性が高く、`balloon_model` 単数フィールド撤去で「共有 1 本」の誤用が構造的に不可能になる。
- **S2: 既存タプルへ 4 要素目を足す**（`Vec<(u32, EmoWorld, AtlasTable, BalloonModel)>`）＝差分最小だが可読性が落ち、`frame.rs:507` の分解パターンが伸びる。
- **S3: `HashMap<u32, BalloonModel>` を `BootAssets` に別途持つ**＝`balloons` と 2 箇所に scope キーが分散（片方だけ更新される欠陥の余地）。**非推奨**。

---

## 6. 規模・リスク・波及

| 項目 | 評価 | 根拠（1 行） |
|---|---|---|
| **規模（A＋C2＋S1）** | **M（3〜7 日）** | 変更ファイル 5＋テスト更新。新規パターンは無く既存ループへの引数追加が主 |
| **規模（A＋C1＋S1）** | **L（1〜2 週）** | `windowposition` 配置規則は placement の**新規能力**＋永続・追従との相互作用検証が要る |
| **リスク** | **Medium** | 単体の変更は既存パターン踏襲で低リスク。中リスク要因は (i) 採寸と装着の解決規則ずれ（4 章）、(ii) R4.4 の比較キー拡張が churn ガードを壊す可能性、(iii) W6 2 本との同一ファイル同居（先行着地義務） |

### ripple（`BootAssets` の形を変える場合の実測着地点）

`balloon_model` を参照/構築する 8 箇所（全文 grep 実測）:
`assets.rs:122`（定義）・`assets.rs:285,305`（構築）・`emo2_boot/mod.rs:322,368`・`frame.rs:422`（分解）・`frame.rs:1383`（テスト構築）・`spine.rs:525,568`・`input_events/balloon.rs:1320`（テスト構築）。
**テスト構築 2 箇所は `parse_str("", None)` のプレースホルダ**ゆえ機械的に追随可能。

### ウェーブ干渉（roadmap.md:75-80 の台帳と一致することを確認済み）

- **ker(W5) ⇄ vis(W6)**: `frame.rs` `run_attach_phase` 末尾（`:531-540` 無条件 ShowSurface ⇄ `:549-556` `connect_balloon_text`／fn `:577-596`）。**本仕様が先行**して per-scope model の実形を確定し、`balloon-visibility` が後着で再突合。
- **ker(W5) ⇄ bind(W6)**: `assets.rs` 異ハンク（本仕様 `:278-300` ⇄ bindoption `:196-210` `BindResolver` 構築）。**本仕様先行着地後に相手が rebase**。
- **ker(W5) ⇄ cage(W6.5)**: `frame.rs`＋`measure.rs`。W6.5 は後続ウェーブゆえ順序で解決。
- **W5 同居 3 本**（`dpi-window-vanish` ∥ `collision-dpi-hittest` ∥ `choice-select-events`）とはファイル集合が互いに素（2026-07-31 再実測済み・roadmap.md:80）。
- **rebase 条項**: `spine.rs` S3/S4 檻域で**新規 GPU world テストを足す場合のみ**オーナースレッド委譲へ乗せる（素の別スレッド `Compositor` は AV）。

---

## 7. 更新が必要な既存テスト（R7.2 の実測リスト）

**必ず赤くなる／意味が変わる**（放置＝矛盾テスト）:

| 場所 | 現在の主張 | 本仕様後 |
|---|---|---|
| `crates/areka/src/placement/measure.rs:564-567`（定数 `:534-536`） | 全 scope の `balloon_size` が 400×224 | scope1 は 288×203（実測）＝**per-scope 期待値へ更新必須** |
| `crates/areka/src/placement/measure.rs:1165-1168` | k 端到端でも全 scope 同一 balloon 寸 | 同上 |
| `crates/areka-emo-present/src/balloon.rs:264` | `frame_id("balloonk0.png") == None`（「相方側は枠でない」） | 系列を明示した判定へ**意味を変えて**更新（sakura 系列では None／kero 系列では Some(0)） |
| `crates/areka/src/emo2_boot/assets.rs:439-449` | 単一 `balloon_model` が sakura 値（validrect 46/-56/36/-44・windowposition 266/-129） | per-scope へ分解し **scope1 は kero 値**（40/-70/24/-48・-190/-75）を assert |
| `crates/areka-emo-text/src/actor.rs:2665` `refresh_actor_scale_with_same_k_is_noop_returning_false` | 「k 同値なら常に no-op」 | R4.4 適用後は「k・寸・領域すべて同値なら no-op」＝**テスト名と意図の更新**＋「k 同値・寸違い→再構築」の新規檻 |
| `crates/areka/src/emo2_boot/spine.rs:1249-1253`（S4 の doc） | 「emo2 fixture は balloons0.png のみ」 | 事実として陳腐化（`balloonk0.png` 実在）＝**doc 更新**（assert 自体は無改変で緑維持） |

**無改変で緑維持が受け入れ条件**（R5.1/R5.2）: `spine.rs:941` S3・`spine.rs:1254` S4 の assert 本体。
**そのまま生きる（改造不要）**: `areka-parsers/src/balloon/validation_tests.rs:95`（sakura マージ）・`:137`（**kero マージ・既に balloonk0s.txt を檻化済み**）。

**新規に要る檻（R7.1 の網羅対象・全て純関数＝GPU/COM 不要）**:
1. scope→系列の写像（0→sakura／1 以降→kero）
2. ID 単位フォールバック分岐（`balloonk0` あり・`balloonk1` なし → 面 0=kero／面 1=sakura）＝**合成 fixture（`TempDir`＋`MemoryDecoder`・`balloon.rs:184-215` が donor）が要る**（実 fixture に `balloonk1` が無いため）
3. `balloonk*` 皆無時の全 scope sakura 縮退（後方互換）
4. 非枠除外（`balloonc*` を kero 系列と誤認しない）
5. 採用面 ID → 面別上書きファイル名の導出（`balloonk0.png`→`balloonk0s.txt`／フォールバック時→`balloons{ID}s.txt`）
6. per-scope マージ実値（scope1 の `windowposition(-190,-75)`・`validrect(40,-70,24,-48)`・`wordwrappoint` は descript 継承 `-34`）
7. per-scope 採寸（scope0=400×224／scope1=288×203・k 適用後も 2 軸独立）
8. R4.4: k 同値・`image_size` 変化で `refresh_actor_binding` が `true`（再構築）を返す／全同値で `false`（churn ガード維持）

---

## 8. Research Needed（設計フェーズへ持ち越す調査項目）

1. **`windowposition` の基本位置の厳密定義**（正典 `windowposition.y` 項: 「数値指定……バルーンとシェル画像の**上端が重なる**位置」）と、areka の**キャラ窓原点＝下端中央**（記憶 `areka-character-origin-bottom-center`）の突き合わせ。`center`/`top`/`bottom` キーワードは M1 で採るか。
2. **`windowposition` の符号規約**（「シェル側が +」）と scope 左右（`balloon.alignment` left/right）の合成規則。emo2 実測値（sakura x=+266／kero x=−190）が実機でどの向きに出るかは**実機観測が最終権威**（記憶: 窓配置は本番ゴースト表示を先に）。
3. **`SerikoLoopConfig.balloon_table` の scope 別化要否**（R5.6）。emo2 では balloon 表が常に空ゆえ実害ゼロだが、正典的には系列ごとに別表。`areka-seriko` へ波及するため境界判断が要る。
4. **`build_balloon_target` の公開 API 形状**（series enum を `areka-emo-present` の公開型にするか、prefix 文字列を受けるか、面 ID→ファイル名の解決結果を返す形にするか）。`measure.rs` が消費できる粒度であることが必須条件。
5. **`balloonp*def*`（\p[2] 以降）の拡張シーム**をどこに残すか（正典 3 段連鎖 `balloonp*def*` → `balloonk*` → `balloons*`。M1 は 2 段のみ実装だが、語彙と縮退シームは残す＝記憶 `defer-canon-with-full-vocabulary-and-tracking-spec`）。
6. **`.pna` 系列**（`balloonk*.pna`）: 正典に存在するが emo2 fixture は PNG α のみ。`ElementDecoder::probe_pna` の既存 seam が受けるため本仕様の追加作業は無い見込み（要確認）。
7. **`windowposition.limit`**（画面内維持・既定 1）の未実装を対応表へ記録するか。

---

## 9. 設計判断項目（要件ディスカッション／設計フェーズへ送る）

> いずれも**本書では決めない**。選択肢と影響のみ提示する。

- **D1. `windowposition` を本仕様で実装するか（最大の争点）** — ✅ **2026-07-31 要件ディスカッションで裁定済み＝実装する（In-scope）**
  裁定の根拠（本書 15 行目の「規模とリスクの支配項」評価は下方修正する）:
  (a) `persist.rs:393`/`:424` の復元マージは「保存値があれば保存値／無ければ resolver 出力」＝**DD7 は恒久規則ではなく初期既定の種**。`windowposition` の導入は恒久ルールの置換ではなく**初期既定値の正典化**にとどまる。
  (b) `resolver.rs:112-113` が DD7 を自ら「暫定」と宣言し「**正式規則は balloon 表示系の後続へ委ねる**」と明記＝空席が用意されている。
  (c) **基本位置は正典と現行実装が一致**——ukadoc `windowposition.y`「数値指定……バルーンとシェル画像の**上端が重なる**位置」＝ `resolver.rs:111` の `balloon_y = char_y`（上端揃え）と同一。ゆえに必要なのは基本位置の作り直しではなく**調整量の加算**のみ。
  (d) 加算口は既存（`ScopeConfig.balloon_offset: Option<(i32,i32)>`・`config.rs:49-50`・`resolver.rs:186` が `unwrap_or((0,0))` で消費・**emo2 では `None`＝未使用**）。ghost descript の `balloon.offsetx/offsety` と `windowposition` は正典上いずれも存在し加算的であるため、供給元の追加として整合する。
  設計へ残る下位論点（D1'）: ① **x 方向の基本位置は正典が明示していない**（y のみ明示）＝実機観測で確定し対応表へ記録（要件 R7.6）。② 符号変換（`windowposition.x` は「シェル側が正」→ バルーンの左右いずれに置くかで画面座標符号へ変換／`.y` は「下が正」＝同符号）。③ **`resolver.rs` の P5 式へ手を入れずに `balloon_offset` 供給で足りるか**——足りれば W5 同居の `dpi-window-vanish`（配置層を境界に掲げ、診断やり直しで編集集合が未確定）との互いに素が保てる。要すると判明した場合は着手順を裁定し干渉台帳へ登記（要件 Adjacent expectations のエスケープ条項）。④ k（表示スケール）は調整量にも適用する（要件 R3.6）。
  Out へ落とした正典項目（語彙記録＋縮退シーム・要件 Out of scope へ登記済み）: `windowposition.x` のキーワード指定（`center`/`top`/`bottom`）・`windowposition.limit`（既定 1・現行はバルーン非クランプ）。

- **D1（原文・記録として保存）**
  R3.2 は字義上 `windowposition` による相対位置決定を要求するが、現行 placement は `windowposition` を一切消費せず DD7 暫定規則（alignment＋offset）で決めている。選択肢 **C1（正典配置を実装・規模 L）／C2（per-scope 寸法まで・配置は DD7 据置・規模 M）／C3（数値指定のみ実装・キーワードは Out）**。C2 を採る場合は R3.2 の文言解釈（「相対位置を相方側の実寸と定義から決定する」の "定義" の射程）を要件ディスカッションで確定する必要がある。影響: `resolver.rs:181-203` の `balloon_pos` 式・`balloon_offset` 恒等式（恒久事後条件）・`persist.rs` の位置永続・W4 position-persist との相互作用。

- **D2. 系列解決の単一権威をどこに置くか**
  現状、balloon ディレクトリの列挙は **`areka-emo-present/src/balloon.rs:50` と `measure.rs:390-409` の 2 実装**。per-scope 化で規則がずれると「採寸した窓寸 ≠ 実際に合成された枠」という実機限定の欠陥になる（4 章）。選択肢: **(a) `areka-emo-present` に公開純関数を置き `measure.rs` が消費（推奨形）／(b) `areka-parsers` 側へ置く／(c) `crates/areka` 内の共有 module／(d) 二重実装のまま規則を檻で同値固定**。

- **D3. R4.4 の「変化」判定キー**
  `actor.rs:367-375` は `scale` のみ比較。拡張候補は **(i) `scale` ＋ `image_size` ＋ `surface_size`（binding 全体の等値比較）／(ii) 上記＋解決後の `TextRegion`（`ResolvedBalloonText`）の等値比較／(iii) `slot`/`window` entity も含む完全等値**。(ii) は「validrect だけが変わった」を捉えられるが `ResolvedBalloonText` に `PartialEq` が要る。churn ガード（R4.5）を壊さないことが制約。

- **D4. per-scope `BalloonModel` の保持器の形**
  S1（`BalloonScopeAssets` struct・`ScopeAssets` 対称）／S2（タプル 4 要素化）／S3（別マップ）。`BootAssets.balloon_model`（単数）を撤去するか温存するかで ripple 8 箇所の性質が変わる。

- **D5. フォールバック時の面別上書き層（R2.3）の分類**
  要件は「正典沈黙＝areka 裁量」としているが、正典 `manual_balloon` は上書きファイルを「**対応するIDのサーフェス（画像）に対して**」適用すると述べており、本体側画像へ縮退したなら本体側 `balloons{ID}s.txt` を使うのが自然な帰結と読める。対応表（R7.4）へ「裁量」と書くか「正典整合」と書くかを確定したい。

- **D6. `balloon.defaultsurface` / `kero.balloon.defaultsurface` の扱い**
  R2.6 は初期面を ID 0 固定とするが、正典は ghost descript の宣言に従う（既定 0）。emo2 fixture は**両キーとも無宣言**ゆえ現状差は出ない。**Out of scope として語彙記録＋縮退シームを残す**か、本仕様で読むか。

- **D7. R1.7「起動を失敗させる」の実装上の意味**
  現行は 2 経路で挙動が違う——(a) 資産構築失敗 → `BootWiringError::Balloon` → `wire_emo2_boot` が **unwired（LogSink×2）へ縮退**（プロセスは生存）、(b) 採寸失敗 → `PlacementError::Measure` → main シームが `spawn_dummy_window` へ縮退。**どちらも「無言で空バルーンを表示」はしない**ので R1.7 の趣旨は満たすが、「起動を失敗させる」を文字どおり取るなら現行縮退では不足。要確認。

- **D8. 相方側で面別上書きファイルが無いときの log レベル（R2.4/R6.2）**
  現行 `assets.rs:334-339` `read_decoded_lenient` は読取失敗を一律 `warn!`。相方側は「面別ファイル不在が正常」（R2.4）なので、per-scope 化で **相方側 warn が常時鳴る**恐れがある。`debug!` へ落とすか、呼び手が層（descript／面別）で使い分けるか。

- **D9. balloon `AnimationTable` の scope 別化（R5.6）** — ✅ **2026-07-31 要件ディスカッションで裁定済み＝(b) per-scope 化する（境界を `areka-seriko/src/looper.rs` へ明示拡張）**
  裁定の根拠（本書の「emo2 では常に空表ゆえ実害ゼロ」評価は判定軸ごと差し替える）:
  (a) **本仕様自身が既存コードの前提を偽にする**——`assets.rs:287-300` は「全 scope は同一 `Shell` から build 済みゆえ**先頭 World から 1 度だけ組めば足りる**」を根拠に表を 1 本にしており、同じ論法でバルーン表も `balloons.first()`（scope 0 の World）から組んでいる。本仕様後は `balloons[0]`＝sakura 系列・`balloons[1]`＝kero 系列で**中身が異なる**ため、この根拠は成立しない。放置すれば scope 1 のバルーンが scope 0 系列のアニメ定義で駆動される＝**R5.6 が禁じる状態そのもの**であり、かつ R7.2（陳腐化した注記を放置しない）にも抵触する。
  (b) **エンジン設計上の対称性の欠損**——`looper.rs:40-42` は「`shell_table`／`balloon_table` は **surface ID 名前空間の別であり能力の仕切りではない**（面種非依存・裁定 (a)）」と明文化しており、シェルとバルーンを同種として扱う思想が既にコードにある。シェル側は「たまたま全 scope 同一データ」ゆえ露見していないだけで、表が scope で引けないことは伺か意味論の選択肢ではなく**土台の欠損**。開発者方針「シェルとバルーンは同じ意味論を共有するエンジンとして実装し、将来バルーンへ高度なアニメーションを持ち込む」に照らすと、「今は空だから」は目的地に背を向けた判断になる。
  (c) 規模は小（フィールド 1 つの型＋参照 1 箇所）。ウェーブ干渉は `bindoption-exclusivity`（W6）が触る `areka-seriko` の bind/state/actor とは別ハンクで、assets.rs と同じく**本仕様の先行着地後に相手が rebase** する既定順序に乗る。
  設計へ残る下位論点（D9'）: 表の所在——`LoopTables` という並行構造を残して scope キーを付けるか、**表は導出元の World と同じ場所（scope 別のバルーン資産・D4 の保持器）へ同梱して単一真実源にするか**。後者ならシェル側も同型になり「先頭から 1 本」という前提コメント自体が不要になる。シェル側も対称に再キーするかは規模と相談。

- **D11. 系列解決は scope 番号でパラメタ化する（2026-07-31 追加・要件 R1.8）**
  正典実文（`ukadoc:manual_balloon` 本日全文取得）: 「`balloonp*def*.png` 三人目以降の吹き出し。**`\p[2]` に当たるバルーンが `balloonp2def0.png` / `balloonp2def1.png` になる**。省略時は `balloonk`、さらになければ `balloons`」＝**第一の `*` が scope 番号・第二が面 ID**。さらに同一の番号正規化が族をまたいで一様に適用されている（`arrows`/`arrowk`/`arrowp{n}def`・`markers`/`markerk`/`markerp{n}def`・`sstp_new`/`sstp_newk`/`sstp_newp{n}def`・`clickwaits`/`clickwaitk`/`clickwaitp{n}def`）。
  ゆえに**本書 5 章アプローチ A の `BalloonSeries { Sakura, Kero }` enum 案は撤回**する（正典の構造を 2 値へ潰しており、`balloonp` 対応時に構造ごと作り直しになる）。正しい形は `scope → 接頭辞優先連鎖`で、面 ID ごとに連鎖を先頭から辿る。ID 単位フォールバックと多段連鎖が**同一コードで表現できる**。

  **連鎖の確定形（2026-07-31 開発者裁定「正規系をベースにすべき」）** — 番号形式 `p{n}def` を**正規名**、`s`/`k` を scope 0/1 の**旧名（過去互換エイリアス）**と位置づけ、3 段の連結で構成する:
  ```
  chain(s) = 自分の候補 ++ 相方系列(s≧2 のみ) ++ デフォルト定義(s≧1 のみ)
    自分の候補     : s=0 → [balloonp0def, balloons] / s=1 → [balloonp1def, balloonk] / s≧2 → [balloonp{s}def]
    相方系列       : [balloonk]
    デフォルト定義 : [balloonp0def, balloons]
  ```
  展開: scope 0 = `[balloonp0def, balloons]`／scope 1 = `[balloonp1def, balloonk, balloonp0def, balloons]`／scope n≧2 = `[balloonp{n}def, balloonk, balloonp0def, balloons]`。
  **`balloonk` の役割は連鎖内の位置で異なる**（設計時に取り違えないこと）——scope 1 の連鎖では「scope 1 自身の旧名（`balloonp1def` の過去互換エイリアス）」、n≧2 の連鎖では「正典が三人目以降の流用先として**名指しした系列**」。後者は scope 1 の解決へ再帰的に縮退するのではないため、n≧2 の連鎖に `balloonp1def` は**入らない**。**デフォルト定義の地位を持つのは scope 0 のみ**（`balloons` が全連鎖の末尾に来る理由）であり、scope 1 はその地位を持たない。
  **`balloonp0def` / `balloonp1def` の先行探索は areka 裁量の正規化拡張**（正典実文は p 系列を `\p[2]` 以降としてのみ記述——`balloonp*def*` 項の「三人目以降」に加え、`arrowp2def`**以降**／`markerp2def`**以降**／`clickwaitp2def`**以降** の 3 箇所が独立に 2 始まりを述べる）。SSP が無視するファイルを areka が拾う互換乖離の可能性を提示したうえで、開発者が「正規系をベース」の方針で採用を裁定した＝要件 R1.10・R7.7(a) で対応表へ記録。
  **語彙の二系統も記録対象**（R7.7(b)）: 同一 scope をさくらスクリプトは `\0`/`\h`・`\1`/`\u`（ukadoc `list_sakura_script`「**`\0` もしくは `\h`** 本体側のスコープに移る」で確認）、ファイル名は `s`/`k` と呼ぶ。ゆえに内部表現は **scope 番号のみ**とし 2 値列挙も `h`/`u` も正準にしない（R1.9）。
  **装飾族の旧名は一段深い**（R7.7(c)）: 正典「`arrows` が本体用（**旧バージョン対応のために `arrow` で代用を推奨**）」／`markers` ⇔ `marker` も同型。吹き出し族に接尾辞なし旧名は無いが、連鎖表を「scope ごとの**可変長**候補列」にしておけば同一構造で表現できる。
  副次利得: ① 接頭辞が厳密文字列ゆえ本書 3 章 R1.5 が警告した「`balloonc*` を相方系列と誤認する事故」が構造的に起きない。② 連鎖関数を**族名でパラメタ化**すれば `arrow*`/`marker*` 等の scope 別対応（本仕様 Out）へそのまま再利用できる。③ `balloonp` を実解決させる追加コストはほぼゼロ（連鎖が伸びるだけ）ゆえ、語彙＋シームの先送りより**素直に実装する方が安い**——先送り 4 点セットの対象から外れる。
  設計論点: 連鎖の返却形（`Vec<String>` / iterator / 族名＋scope の 2 引数）と、`measure.rs` が消費できる粒度（本書 D2 の単一権威と併せて決める）。

- **D9（原文・記録として保存）**

- **D10. アプローチ選択**
  A（下流純関数へ集約・推奨）／B（単一 World 合成・正典乖離ゆえ非推奨）／C（各所で個別分岐・二重実装固定化ゆえ非推奨）。

---

## 10. 次フェーズへの推奨

1. **要件ディスカッションで D1（`windowposition`）を最優先で裁定する**——規模が M と L で倍近く変わり、W6 `balloon-visibility` / W4 position-persist との接触面も変わる。
2. 設計フェーズ着手時に **`git log -- crates/areka-emo-present/src/balloon.rs crates/areka/src/emo2_boot/assets.rs crates/areka/src/placement/measure.rs` で再度陳腐化確認**（記憶: 並走 brief は陳腐化する・design 前に rebase）。本書のアンカーは 2026-07-31 実測。
3. 設計では **D2（単一権威）を先に固めてから** assets/measure/frame の配線を書く（4 章の (A)/(B) ずれが本仕様最大の実機限定リスク）。
4. 実機サインオフの判定を目視だけに頼らず、**R6 のログ（scope／採用系列／採用ファイル／`windowposition`・`validrect` 実値／窓寸）を grep 可能な形で設計**する（400×224 と 288×203 の差はログで決定論的に確認できる）。
