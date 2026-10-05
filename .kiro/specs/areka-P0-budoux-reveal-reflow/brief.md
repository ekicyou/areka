# Brief: areka-P0-budoux-reveal-reflow

> 2026-10-05 `/kiro-discovery` で起票（開発者「emo2 の初回起動トークで、バルーンに不自然な改行が見られる。原因を調査し、バグ修正 spec を立ち上げよ」）。段＝**バグ**。本文の file:line は起票時（main `fa12dba7`）の実測＝着手時に引き直すこと。

## Problem

- **利用者**: `budoux_newline,1` のバルーン（emo2 同梱の `emo2-kakukaku` が該当）で台詞を読む人。1 字ずつ表示している途中で、**もう表示されていた字が次の行へ飛ぶ**。読んでいた位置が動くので目が追えない。
- emo2 の初回起動トーク（`dic/boot.pasta` の `＊OnFirstBoot`）のエモ側で実際に起きる: 「イイジャン！‥」「イイジャン！‥‥」と 1 行で出たあと、「ええと、」が届いた瞬間に「イイジャン！ ／ ‥‥ええと、」へ組み直され、出ていた「‥‥」が 2 行目の頭へ移る。
- 完了済みの `areka-P0-budoux-newline` の要件 7.2「While typewriter リビールが進行中である, the emo テキスト層 shall 一度配置したグリフの所属行を、後続グリフのリビールによって移動させない（リフロー跳びを起こさない）」を、**実際の届き方では満たしていない**。

## Current State

- 文節の区切りは `crates/areka-emo-text/src/actor_present.rs` の `present_frame` の中（`WrapMode::BudouxWordWrap` の腕・起票時 251〜253 行）で、毎フレーム `segment_plan(actor_state.items())` として計算し直している。
- `items()`（`crates/areka-emo-text/src/state.rs` の `TextLayerState::items`）は**その時点までに届いた字だけ**。台本の字は「タグとタグの間」ごとに 1 つのテキストの合図になり（`crates/areka-sakura/src/compile.rs` の `Instruction::Text` の腕）、dola は合図を予定の時刻が来てから配る（`crates/dola/src/cue/runtime.rs` の `tick` の `sink.emit`）。
- pasta は句読点や「‥」ごとに `\_w[…]` を入れるので、1 つの台詞が多数のテキストの合図に分かれ、少しずつ届く。後から届いた字で BudouX の区切りが変わり、配置済みの字の行が変わる。
- `budoux-newline` の設計（design.md の INV-1「入力は常に全 `items`（可視 prefix ではない）」・`segment.rs` の冒頭の注記）は「全 items＝台詞の全文」を前提にしているが、合図が分かれて届くとこの前提が崩れる。同 spec の決定論テストは 1 つの合図に全文が入った形しか踏んでいない見込み（要件の段で確かめる）。
- 再現の計算（起票時・調査用の一時テストで実行済み・削除済み）: 本物の `parse` → `compile` → `TextLayerState::apply_cue` → `segment_plan` → `LayoutEngine::layout` を、実フォント Yu Gothic UI・`emo2-kakukaku` の 2 層の定義で回した。エモ側は合図が届くたびに上記の飛びが出た。さくら側の最終形には飛びが無かった。

### 実機の撮影（2026-10-05・main `fa12dba7` のデバッグ版・DPI 192・emo2 を `target\lb-repro3` へ写して profile を消した初回起動）

- `evidence/01-before-jump.jpg`: エモの欄が「可愛い娘。／イイジャン！‥」＝「‥」は「イイジャン！」と同じ行に出ている。
- `evidence/02-after-jump.jpg`: 0.3 秒ほど後に「‥‥ええと、／僕は日常か」＝**出ていた「‥」が次の行の頭へ移った**（欄が 3 行しか入らないので上へ送られている）。本 spec の飛びを実機で確認。
- `evidence/03-final-leading-dots.jpg`: 全部出た後も「イイジャン！／‥‥ええと、」＝行頭の「‥‥」（禁則・範囲外）。
- `evidence/04-pasta-blank-lines.jpg`: さくらの欄の「そこ自分でいう‥‥。」と「つまり、、」の間の大きな空き（pasta の余分な改行・範囲外）。
- `evidence/05-ok-alone.jpg`: 「つまり、、エモを弄ってれば／OK？」＝「OK？」が 1 つで行に残る。BudouX の文節「弄ってれば｜OK？」で幅が足りないための正しい折り返しで、本 spec の対象ではない（禁則でも直らない）。

### SSP との比べ（2026-10-05・開発者が起動した SSP の「えも2DEBUG」へ MCP の `raise_event` で OnFirstBoot を投げて撮影）

- 返った台本の逐語は `evidence/ssp-onfirstboot-script.txt`。送り手が SSP なので、pasta は自前の BudouX 改行 `\n` を入れている（「イイジャン！\_w[450]\n‥」「エモを弄ってれば\nOK？」など）。areka では `apply_baseware_policy` がこれを外し、areka 自身が `budoux_newline,1` で折り返す。
- `evidence/11-ssp-leading-dots.jpg`: SSP でも「イイジャン！／‥‥ええと、」＝**行頭の「‥‥」は SSP でも同じ**（pasta が「‥」の前に `\n` を入れるため）。
- `evidence/12-ssp-pasta-blank-lines.jpg`: SSP でも「そこ自分でいう‥‥。」と「つまり、、」の間に同じ空き＝**pasta の余分な改行は SSP でも出る**（台本にも `\p[0]\n[150]\s[1000]…\p[1]…\p[0]\n[150]…つまり` と 2 つ並ぶ）。
- `evidence/13-ssp-ok-alone.jpg`: SSP でも「つまり、、エモを弄ってれば／OK？」。
- **SSP では字が飛ばない**: 改行は台本に `\n` として先に書かれていて、表示中に折り返しを計算し直さないため。**areka だけの症状は本 spec の飛びだけ**で、最終形の見た目は SSP とほぼ同じ（エモ側の「クール系の可愛い娘。」が SSP では 1 行・areka では「クール系の／可愛い娘。」になる差は、pasta の幅の設定と areka の実測の幅の違いで、本 spec の対象外）。
- 注意: 撮影した SSP のゴーストは開発版（えも2DEBUG）で、areka の検体（emo2.nar・pasta.dll 0.3.7）と版が同じとは限らない。

## Desired Outcome

- 合図がいくつに分かれて届いても、**一度表示された字の行は動かない**（`budoux-newline` 要件 7.2 を実際の届き方で満たす）。
- 合図が 1 つで全文が届く場合の結果は今と変わらない（既存の決定論テストを 1 本も落とさない）。
- 「分かれて届く」形の決定論テストを置く（emo2 の「イイジャン！‥‥ええと、」を写した入力で、修正前に赤・修正後に緑）。

## Approach

要件・設計の段で決める。起票時に見えている選択肢:

- **案 1（推奨）＝出した字の行を固定する**: 文字の層の中で、すでに配置した（または表示した）字の行の割り当てを覚えておき、後から届いた字で区切りが変わっても、固定済みの字より前へは組み直さない。文字の層の中だけで閉じる。副作用として「‥‥」の例は「イイジャン！‥‥ ／ ええと、」になり、行頭に「‥‥」が来なくなる。
  - 決めること: 固定する単位（届いた字か・表示された字か）、`\c`・あふれのスクロール・選択肢の再配置との関係、要件 7.3「可視化されるグリフの行位置が最終レイアウトと一致」をどう読み替えるか。
- **案 2＝台詞の全文を先に文字の層へ渡す**: compile の段では台詞全体の字が分かっているので、それを先に渡して全文で区切る。**`budoux-newline` の要件の範囲外の節「sakura compile／cue 語彙の改変（境界ヒント cue 案は層違反として棄却済み——折返し可否はバルーン幅依存＝emo の関心）」と衝突する**＝採るなら過去の裁定を覆す議題になる。
- **案 3＝区切りの計算に「まだ届いていない続き」を待つ**: 届いた字の末尾の文節だけは確定させずに保留する。表示の遅れや見た目の揺れが出るので見込みは薄い。

## Scope

- **In**: `budoux_newline,1` のときの、分かれて届く字に対する行の割り当ての安定化。分かれて届く形の決定論テスト。`budoux-newline` の INV-1 の前提の書き直し（`segment.rs` の注記など）。実機での確かめ（emo2 の初回起動トーク）。
- **Out**:
  - 禁則（行頭に「‥」「、」などを置かない規則）＝`text-typesetting` の担当（brief「禁則は無い」「既定は `anywhere`＝SSP と同じ折り返しのまま」）。本 spec の案 1 で「‥‥」の例がたまたま直っても、禁則を入れたことにはしない。
  - 1 字ずつの折り返し（`budoux_newline` 無し）。こちらは届いた字だけで決めても行が動かない（要件の段で確かめる）。
  - pasta の段落の改行（下の「調査で分かった別の原因」）。

## 調査で分かった別の原因（本 spec の範囲外・記録）

開発者が見た「不自然な改行」には、本 spec の飛びのほかに次の 2 つが混じっている見込み。

1. **空行が余分に入る（pasta 側）**: pasta の `sakura_builder.lua`（ゴースト同梱の `profile/pasta/pasta_scripts/pasta/shiori/sakura_builder.lua`）の S3 の分岐が、`＠表情` の単語が返す**タグだけの本文**も「字がある」と数えて `spot_has_text` を真にする。そのため、
   - エモの最初の `＠通常　\1\![move,…]` だけでエモの欄が「字あり」になり、最初の台詞「僕はエモ。」の前に `\n[150]` が付く（エモの欄の頭に 1.5 行の空き・3 行しか入らない欄なので 2 行目であふれてスクロールする）。
   - 字の無い「むらさき：＠通常」の行が `\n[150]` を 1 つ余分に出し、さくら側の「つまり、、」の前で 2 つ重なる（3 行ぶんの空き）。
   - areka は書かれたとおりに描いているだけ。直す場所は pasta（別リポジトリ `ekicyou/pasta`）。なお pasta は送り手が areka のとき自前の BudouX 改行を外している（`scripts/pasta/shiori/event/boot.lua` の `apply_baseware_policy`・`OnFirstBoot` でも呼ばれる・実機の `pasta.log` で確認）。
   - SSP でも同じ空きになる（上の「SSP との比べ」で撮影済み）。
   - **pasta 側で起票済み（2026-10-05）**: spec `paragraph-break-tag-only-talk`（ekicyou/pasta#66・マージ待ち）。pasta の現行 main でも S3 の分岐は同じで、マニュアル `reference/pasta-toml.md` の `spot_newlines`（「すでに台詞を出したスポット」）とも食い違うと確認された。「＠単語」は正規の書き方なので、ゴースト（emo2）側の回避は不要。
2. **行頭の「‥‥」（禁則）**: BudouX が「‥‥ええと、」をひとまとめにするため、最終形でも「‥‥」が行頭に来る。UAX #14 の LB22（「‥」「…」の前では切らない）に当たる。`text-typesetting` の担当で、既定 `anywhere` のままでは直らない。

## Boundary Candidates

- 区切りの計算（`segment.rs`・純関数）と、行の割り当ての記憶（文字の層の状態・`state.rs` か `actor_present.rs`）と、配置（`layout*.rs`）の 3 つ。案 1 なら主に 2 つ目に足し、`segment.rs` と layout の純関数は形を保つのが望ましい。

## Out of Boundary

- sakura compile と dola の合図の語彙（案 2 を採らない限り触らない）。
- 禁則・ぶら下げ・縦中横（`text-typesetting`）。
- pasta のスクリプト生成。

## Upstream / Downstream

- **Upstream**: 完了済み `areka-P0-budoux-newline`（区切りの計算と配置）・`areka-P0-emo-text-*` の配置の土台・dola の合図の配信。
- **Downstream**: `text-typesetting`（禁則を同じ折り返しの判定に足す＝本 spec の「固定」の約束の上に乗る）・`text-reveal-fade`・`talk-fast-forward`（早送りで残りの字が一度に届くときも行が動かないこと）。

## Existing Spec Touchpoints

- **Extends**: なし（`budoux-newline` は完了済み＝その要件 7.2 を実際の届き方で満たし直す）。
- **Adjacent**: `text-typesetting`（`wrap.rs`・layout の折り返し点を触る）・`talk-fast-forward`・`balloon-scroll-fade`（あふれのスクロール）。いずれも文字とバルーンの列。

## Constraints

- 合図 1 つで全文が届く場合の出力を変えない（既存テスト全部緑）。
- 1 フレーム遅らせて直す手は取らない（記憶 no-frame-delay-fixes）。
- 決定論テストは「分かれて届く」経路を踏ませる（`apply_cue` を複数回・時刻を進めて）。表示するだけでなく行の割り当てを判定する。
- 実機の根・検体はワークツリーの `target\` の下だけ。emo2 は絶対パスで起動。
- 規模の見立て: S（4〜7 タスク）。
