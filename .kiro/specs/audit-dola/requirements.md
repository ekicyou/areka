# Requirements Document

## Project Description (Input)

開発者は、areka のコードに残っているバグ・危ない所・負荷に弱い所・テストの抜け・遅い所・要らないコードを、一度まとめて直したい。今までは、各 spec が自分の触る所だけを見てきた。全体を同じ物差しで見直したことはない。物差しは ponytail のプラグイン（5.3.2・利用者全体に入れてある）の `/ponytail:ponytail-audit`。この spec は **dola（絶対時刻の台本の再生）** を受け持つ（受け持つファイル＝`crates/dola/` 全部。まだ audit をかけていない）。

直した後の姿: 受け持つ範囲に audit をかけて出た指摘が、3 つの組（必ず直す・直したほうがよい・できれば）とも全部直っている（開発者「とにかく指摘事項は全部直す方針」）。直した後に同じ範囲へもう一度 audit をかけて、新しい指摘が 0 件。全体テストが緑で、直したバグ・危ない所には、それが壊れたら落ちるテストが 1 本ずつある。

あわせて、ほかの spec から申し送られた 3 件（件 1: トークの終わりの時刻が ms の格子から外れうる・件 2: `CuePlayer::stop` を呼ぶ本番のコードが無い・件 3: dola の clippy が `-D warnings` で赤）を、この spec で片づける。

## Introduction

この spec は、dola（絶対時刻の台本の再生）を受け持つクレート `crates/dola/` を、ponytail の audit で見直し、出た指摘を全部直すためのものである。要件は「受け持つ範囲の指摘を、3 つの組とも全部直す」という形だけを決める。指摘の中身はここには書かない。指摘はギャップ分析で audit をかけて洗い出し、要件の討議で直す対象として確定する（2026-10-19 開発者「原則、ギャップ分析でauditを実施して、設計以降で修正」）。

申し送りの 3 件は中身が決まっているので、要件 7〜9 に外から見た結果として書く。直し方は要件の討議と設計で決める。

この spec はウェーブ C18 で、ほかの 7 本（`wintf-visual-tree`・`placement-crate-split`・`audit-wintf-layout-widget`・`audit-emo-text-state-actor`・`audit-emo-text-layout-draw`・`audit-kanade`・`audit-sylphya-actor`）と並んで走る。並走の約束（ロードマップの「ウェーブ編成」の C18 の行・特に約束 ⑴・⑶・⑹）を守る。

## Boundary Context

- **In scope**:
  - `crates/dola/` の全ファイル（本番のコード・テスト・テストの道具・fixture・`Cargo.toml`）に audit をかけて出た指摘の直し。
  - 直しのために要るテストの追加と直し。
  - 直しのために要る、範囲の外の呼び出し元の直し（要件 3.2 と要件 6 の約束に従う。C18 のあいだは要件 6.3 で触れない所がある）。
  - 申し送りの件 1〜3（要件 7〜9）。
- **Out of scope**:
  - 範囲の外のファイルの見直し（ほかの 32 本の見直しの持ち物）。
  - 新しい機能。
  - `vendors/` の下。
  - clippy の指摘の機械的な直しのうち、`crates/dola/` の外のもの（`clippy-199-lints` の持ち物）。`crates/dola/` の中の分は件 3 としてこの spec が直す。
  - dola の YAML の読み書きの土台の取り替え（`dola-yaml-backend` の持ち物）と、時刻の型を秒の `f64` から別の型へ替えること（`dola-interrupt-ticks` の持ち物）。この 2 本は本 spec の着地を待つ。
  - 開発者の裁定（ロードマップの「生きている決まり」・記憶の決めごと・完了した spec の要件）を覆す直し。直さずに議題にする。
- **Adjacent expectations**:
  - dola を呼ぶ範囲の外のコード（本体 `crates/areka` の 54 ファイル・`crates/areka-sakura`・`crates/areka-emo-text` の `sink.rs`／`actor_lookahead*`・wintf の `ecs/dola/mod.rs`・`ecs/mod.rs`・`ecs/world/mod.rs`・`ecs/graphics/core.rs`）は、ほかの spec の持ち物である。C18 では dola の公開の形を変えず、これらを書き換えない（要件 6）。
  - C18 で並ぶほかの 7 本とは、受け持つソースファイルが重ならない（棚卸㉟の測り）。
  - `clippy-199-lints` は dola の clippy の分も受け持ちに数えている。本 spec が件 3 で直した分を、`clippy-199-lints` が直し直さなくてよいように記録を残す（要件 9.4）。

## Requirements

### Requirement 1: 受け持つ範囲の指摘をもれなく洗い出す

**Objective:** As a 開発者, I want 受け持つ範囲の指摘を、省かず、読み残しなく一覧にしたい, so that 直す対象をもれなく決められる

#### Acceptance Criteria

1. When ギャップ分析を行うとき, the ギャップ分析 shall `crates/dola/` だけを対象にして `/ponytail:ponytail-audit` をかける
2. When audit をかける前, the ギャップ分析 shall その時点の `main` を取り込んだうえで、受け持つ範囲のファイルの数と行数（本番のコードとテストを分けて）を測り直す
3. The ギャップ分析 shall audit の「最大 20 件」の上限を外し、出た指摘を全件、省かずに報告へ残す
4. The ギャップ分析 shall 受け持つ範囲のファイルを全部読み、「読まなかった所」を残さない
5. The ギャップ分析 shall 各指摘に通し番号（範囲を分けて読んだ結果をまとめた後の、3 つの組を通した番号。以後の段はこの番号で指す）を付け、3 つの組（必ず直す・直したほうがよい・できれば）のどれに当たるかと、指している場所を報告に書く
6. When audit の指摘が申し送りの件 1〜3 と同じ所を指すとき, the ギャップ分析 shall その指摘と件の番号の対応を報告に書く（同じ直しを 2 回数えない）
7. When audit の指摘を直すのに dola の公開の形を変える必要があるとき, the ギャップ分析 shall その指摘にその旨と、範囲の外のどの呼び出し元に響くかを書く（要件 6.4 の議題の材料）

### Requirement 2: 直す対象を要件の討議で確定する

**Objective:** As a 開発者, I want 指摘の一覧を見て、直す対象と直さずに残すものを自分で確定したい, so that 方針「指摘事項は全部直す」から外れるものが私の裁定なしに残らない

#### Acceptance Criteria

1. When 要件の討議を行うとき, the 要件の討議 shall ギャップ分析の指摘を全件、3 つの組とも直す対象として要件に落とす
2. The 本 spec shall 直さずに残す指摘を、次の 3 種類のどれかに当たり、かつ要件の討議で開発者が残すと裁定したものだけに限る
   - ⑴ 開発者の裁定に反する直し
   - ⑵ わざと置いた先送りの印（実在の理由を書いた説明つきで使われていないことを許している語彙の口、および `shortcut:`・`ponytail:` の注釈で、書いた限界が今の負荷を越えていないもの）
   - ⑶ 外から見た振る舞いが変わる直しのうち、SSP との互換や ukadoc の正典に関わるもの
3. If 指摘が ⑶ に当たるかもしれないとき, then the 要件の討議 shall ukadoc で正典を確かめた結果（出どころと引用）を添えて議題にする
4. If 指摘の直しが開発者の裁定を覆すとき, then the 本 spec shall その指摘を直さず、議題にする
5. The 本 spec shall 直さずに残すと裁定された指摘ごとに、当たる種類と開発者の裁定を記録に残す
6. If 直す対象のタスクが 20 を越えそうなとき, then the 要件の討議 shall 指摘の組ごとに、同じ範囲の 2 本目の spec へ切り分けることを議題にしてよい

### Requirement 3: 確定した指摘を全部直し切る

**Objective:** As a 開発者, I want 確定した指摘が 3 つの組とも、半端なく直っていてほしい, so that 受け持つ範囲のバグ・危ない所・要らないコードが残らない

#### Acceptance Criteria

1. The 本 spec の直し shall 直す対象として確定した指摘を、3 つの組（必ず直す・直したほうがよい・できれば）とも全部直す
2. When 指摘を直すとき, the 本 spec の直し shall その直しで壊れる呼び出し元・テスト・fixture・設定を、範囲の外も含めてツリー全体で探し、全部直し切る
3. When コードやファイルを消すとき, the 本 spec の直し shall 消す前に、ツリー全体（テスト・fixture・設定・文字列での参照を含む）でそれが使われていないことを確かめる
4. The 本 spec の直し shall 直す対象として確定していない新しい機能を足さない

### Requirement 4: 直したバグと危ない所をテストで縛る

**Objective:** As a 開発者, I want 直したバグや危ない所が、また壊れたらテストで分かってほしい, so that 直しが後の変更で黙って戻らない

#### Acceptance Criteria

1. The 本 spec の直し shall 直したバグと危ない所の 1 件ごとに、それが壊れたら落ちるテストを 1 本ずつ持たせる
2. The 本 spec のテスト shall 決定論で動き、回すたびに同じ結果になる
3. Where テストが実機の根や一時フォルダを使うところでは, the 本 spec のテスト shall それをワークツリーの `target` の下に置く
4. Where テストが GPU を使うところでは, the 本 spec のテスト shall 実 GPU で回す
5. When 既存のテストが直しで古びたとき, the 本 spec の直し shall そのテストを直しに合わせて改める（直しの的を外して緑にしない）

### Requirement 5: 直しの中でも守る決まり

**Objective:** As a 開発者, I want 直しがプロジェクトの決まりを崩さないでほしい, so that 見直しで新しい問題が生まれない

#### Acceptance Criteria

1. The 本 spec の直し shall どのファイルも 1,000 行の番人を越えさせない（足すなら新しいファイルへ）
2. The 本 spec の直し shall 記録の無い失敗の道を作らない（失敗は `error!` の記録とエラーの戻り値の両方で表す。dola には記録の依存が無いので、dola の中では失敗を戻り値（`Err`・`TriggerResult::Error`）で返し、`error!` は呼び手の境目で出す＝要件 11.6）
3. The 本 spec の直し shall 終了の道を正規の実装にする（終了を省く小細工をしない）
4. The 本 spec の直し shall 文字化けをコンソールの文字コードの切り替えで逃げず、読む側を直す
5. The 本 spec の直し shall 時刻を正確に扱う（待ち時間や時刻を、直しの都合で丸めない）
6. The 本 spec の直し shall 依存を足さない（C18 の約束 ⑹）
7. If 直しが新しい依存なしには成り立たないとき, then the 本 spec shall 依存を足さずに、`tech.md` への登記の案を添えて開発者の承認を仰ぐ議題にする

### Requirement 6: 範囲の外へ直しが及ぶとき・C18 の並走の約束

**Objective:** As a 開発者, I want 範囲の外への直しが、並んで走るほかの spec とぶつからないでほしい, so that 並走しても互いの作業を壊さない

#### Acceptance Criteria

1. While 範囲の外のファイルを受け持つ spec のセッションが走っているあいだ, when 直しがそのファイルへ及ぶとき, the 本 spec shall そのセッションと申し合わせてから触る
2. The 本 spec の直し shall 範囲の外のファイルについて、直しのために要る変更のほかは手を入れない（範囲の外の指摘を見つけても、その範囲の spec の持ち物として残す）
3. While C18 のあいだ, the 本 spec の直し shall 次のファイルに手を入れない: 本体 `crates/areka/` の全部・`crates/areka-emo-text` の `sink*`・`actor*`・wintf の `ecs/mod.rs`・`ecs/world/mod.rs`・`placement-crate-split` が触る外のファイル（`log-capture-kit` のテスト 2 つ・wintf の `ecs/window_proc/` の 2 つ・`tools/perf/fixtures/`・`tools/perf/fixtures-loop/`）
4. While C18 のあいだ, the 本 spec の直し shall dola の公開の形（範囲の外から呼べる型・関数・欄の名前と形）を、足すことのほかは変えない（C18 の約束 ⑶）
5. If 確定した指摘が dola の公開の形を変えずには直せないとき, then the 本 spec shall C18 のあいだはその指摘を直さず、C18 のほかの 7 本がすべて `main` に着地した後の段で、本 spec の中で直す（2026-11-06 討議・開発者「分けない」＝要件 11.1・11.2）
6. The 本 spec の直し shall `vendors/` の下に手を入れない

### Requirement 7: トークの終わりの時刻を ms の格子に載せる（件 1）

**Objective:** As a 開発者, I want 整数 ms で組んだ台本の終わりの時刻が、同じ ms の値と寸分違わず一致してほしい, so that トークの終わりの合図（`TalkDone`）と後始末が 1 刻み遅れない

ここで「ms の格子に載る値」は、整数の ms を 1000 で割って秒にした値をいう（呼び手の Tick が今の時刻を出すのと同じ形）。

#### Acceptance Criteria

1. When 台本の合図の始まりと長さがどちらも ms の格子に載る値のとき, the dola shall 台本の終わりの時刻（`CueSheet::absolute_end_time` の値）を、ms で足した終わりを秒にした値そのものとして出す（例: 150 ms に始まる 800 ms の合図の終わりは 950 ms を秒にした値と等しく、0.9500000000000001 にならない）
2. When 台本の合図の始まりと長さがどちらも ms の格子に載る値のとき, the dola shall 台本から再生の予定へ直すときに焼き込む占有の終わりも、7.1 と同じ値にする
3. When 再生の時刻が、ms の格子に載った終わりの時刻にちょうど届いたとき, the dola の再生 shall その刻みで完了とする（呼び手の `TalkDone` と後始末が 1 刻み遅れない）
4. If 合図の始まりか長さが ms の格子に載らない値のとき, then the dola shall 終わりの時刻を格子へ丸めず、今と同じ値を出す（時刻は正確に扱う）
5. The 本 spec の直し shall 件 1 を dola の中で閉じて直し、`absolute_end_time` の形と、それを呼ぶ範囲の外のコード（`crates/areka-sakura/src/compile.rs` の `append_epilogue` など）を変えない（合図の長さを整数 ms で持つ形は、呼び手が多いので C18 では取らない）
6. The 本 spec のテスト shall 7.1〜7.4 のそれぞれを、壊れたら落ちるテストで縛る（7.1 は 150 ms＋800 ms の例を含める）

### Requirement 8: 本番に呼び手の無い `CuePlayer::stop` の扱いを決める（件 2）

**Objective:** As a 開発者, I want 本番に呼び手の無い `CuePlayer::stop` が、理由の分からないまま残らないでほしい, so that 要らないコードか、わざと置いた口かが読み手に分かる

#### Acceptance Criteria

1. While C18 のあいだ, the 本 spec の直し shall `CuePlayer::stop` を消さない（範囲の外の `crates/areka/src/emo2_boot/frame/window_state_tests.rs` が呼び、C18 では本体に触れない）
2. When C18 のほかの 7 本がすべて `main` に着地した後, the 本 spec の直し shall `CuePlayer::stop` を消し、本体の呼び手 `crates/areka/src/emo2_boot/frame/window_state_tests.rs:496` を、player を捨てる形（本番の `drive.rs` の `drop(player)` と同じ）に直す（2026-11-06 討議・推奨案で確定）
3. When 8.2 を直すとき, the 本 spec の直し shall 同じ形の `CuePlayer::skip_barrier`（`research.md` #15・本番から呼べない）も消し、「選択肢 0 件の区切りは `skip_barrier` で抜ける」と書いた説明を「上流が player を捨てて抜ける」に改める
4. The 本 spec のテスト shall 8.2 の後も、中断した台本で状態が変わらないこと（`window_state_tests.rs` の檻の的）を、player を捨てる形で縛り続ける
5. If audit が `CuePlayer::stop` を要らないコードとして指したとき, then the ギャップ分析 shall その指摘を件 2 と同じものとして扱う

### Requirement 9: dola の clippy を `-D warnings` で緑にする（件 3）

**Objective:** As a 開発者, I want dola のクレートが今の clippy の `-D warnings` で通ってほしい, so that dola の警告がほかの赤に紛れず、`clippy-199-lints` の手間が減る

#### Acceptance Criteria

1. When 直しを全部終えたとき, the dola の静的検査（`cargo clippy -p dola --all-targets --no-deps -- -D warnings`） shall 警告 0 で通る
2. The 本 spec の直し shall clippy の各指摘を、振る舞いを変えずに直すか、要件の討議で許すと決めたものだけを、その場所に許す理由を書いて許す
3. The 本 spec の直し shall clippy の指摘の直しでも要件 6.4（公開の形を変えない）を守り、公開の関数の引数の数などを変える直しは要件 6.5 の議題にする
4. When 件 3 を直し終えたとき, the 本 spec shall `clippy-199-lints` の受け持ちのうち dola の分が済んだことを記録に残す

### Requirement 10: 仕上げの確かめ

**Objective:** As a 開発者, I want 直し終わったことを、測り直した結果で確かめたい, so that 「全部直した」を報告だけでなく事実で受け取れる

#### Acceptance Criteria

1. When 直しを全部終えたとき, the 検証 shall 同じ範囲へもう一度 `/ponytail:ponytail-audit` を、要件 1 と同じ作法（上限なし・全部読む）でかける
2. When もう一度かけた audit が指摘を出したとき, the 本 spec の直し shall 直しが生んだ指摘も含めて直し、新しい指摘が 0 件になるまで繰り返す（要件 2 で直さずに残すと裁定された指摘は数えない。打ち切りは 10.6）
3. The 検証 shall 新しい指摘が 0 件であることを、かけ直した audit の結果として記録に残す
4. When 直しを全部終えたとき, the 全体テスト（`pwsh -NoProfile -File tools/test-all.ps1`） shall 緑で終わる
5. When 直しを全部終えたとき, the 検証 shall 要件 9.1 の静的検査が警告 0 で通ることを、走らせた結果として記録に残す
6. If かけ直しが 5 回目になっても 0 件に収まらず、5 回目の指摘に「必ず直す」と本番の振る舞いに関わるものが 1 件も無いとき, then the 本 spec shall 開発者に一言知らせたうえで、5 回目の指摘を直したところで打ち切り、検証の記録に「5 回目で打ち切った・残りは直し済み」と書いて全体テストへ進む（2026-11-06 討議・audit-seriko と同じ形）

### Requirement 11: 要件の討議で確定した直す対象と残す指摘（2026-11-06・議題 1 は開発者「分けない」・議題 3〜10 は推奨案）

**Objective:** As a 開発者, I want 討議で決めた直す対象と、残すと裁定した指摘と、議題ごとの直し方の決めを 1 か所で引きたい, so that 設計とタスクが決めから外れない

指摘の番号は `research.md` §3 の通し番号（#1〜#88）。

#### Acceptance Criteria

1. The 本 spec shall 1 本の spec のまま進め、`research.md` §3 の 88 件のうち 11.3 で残すと決めた分を除く全部を、3 つの組とも直す（2026-11-06 開発者「分けない」。タスクが 20 を越えても同じ範囲の 2 本目へは切らない）
2. The 本 spec shall 直しを 2 つの段に分ける。前の段＝公開の形を変えない直しと件 1・件 3（C18 のあいだに取る）。後の段＝C18 のほかの 7 本がすべて `main` に着地した後に取る直し＝公開の形を変える 18 件（#11〜#19・#25・#26・#52・#56・#66・#70・#73・#77・#78）と件 2（要件 8.2・8.3）
3. The 本 spec shall 次のものだけを直さずに残す（要件 2.5 の記録）
   - #4 の付随（最後の区切りより後に置いたトリガーは、親が先に終わって発火しない）: ⑴ 開発者の裁定＝完了 `dola-nested-storyboard` 要件 3.3「トリガーを長さに足さない」を守る。#4 の本体（台本の長さに先頭の遅れが入らない）は直す
   - #11 の `CueCommand::WaitForInput` と `CuePlayer::resolve_click`: ⑵ わざと置いた口＝`talk-fast-forward` の `\x` の受け皿。残す理由を説明に書く（11.4）
   - #25 の機能 `yaml`: ⑵ 持ち主の spec＝`dola-yaml-backend` が決まっている
4. When #11 を直すとき, the 本 spec の直し shall 後の段で、効かない時間切れ（`WaitForInput` の `timeout` の欄・`BarrierKind::Timeout`・その時間切れの枝）を消し、残す `WaitForInput` と `resolve_click` の説明に「`talk-fast-forward` の `\x` の受け皿」という実在の理由を書く。ukadoc の `\x`・`\x[noclear]` には時間切れの引数が無い（`ukadoc:list/sakura-script/wait/x`）＝消しても正典に反しない
5. When #12・#16・#17・#26 を直すとき, the 本 spec の直し shall 後の段で、配送の制御（Routing）・`CueTarget::Window`・`CueCommand::EntityRef`・`PlaybackState`／`ScheduleRequest` を、それを受ける範囲の外の枝とテストごと消す（完了した spec が置いた理由＝M-dialogue の受け皿・退役した相手の関数は、もう過ぎている）
6. When #29・#38 の失敗を記録に残すとき, the 本 spec の直し shall dola に記録の依存を足さず、dola は戻り値で失敗を返し、`error!` は呼び手の境目（wintf の `tick_dola_animators`）で出す（`cue/sink.rs` の「記録は呼び手の境目」と同じ作り）
7. When #5 を直すとき, the dola のコンパイル shall 「前のエントリに続く」を配列の順で決める。変数を持つ遷移は同じ変数の配列上の直前のエントリに続け（完了 `dola-compiled-transition` 要件 3.5 に合わせる）、変数を持たないエントリ（純粋なキーフレーム・トリガー）は配列の 1 つ前のエントリに続ける
8. When #7 を直すとき, the dola の競合の判定 shall 「同じ変数に生きた項目があれば競合」とし、インスタンスごとの相対の時刻どうしを比べる重なりの判定を消す
9. When #29 を直すとき, the dola の検証 shall 負の delay を拒む（delay・duration は 0 以上、time_scale は 0 より大きい、数値はどれも有限、loop_count は 1 以上か −1）
10. When #31 を直すとき, the dola のコンパイル shall トリガーと純粋なキーフレームに書いた `between` を、`between` の始まりの時刻で発火・配置するものとして扱う（完了 `dola-nested-storyboard` の「トリガーでも at/between/keyframe を使える」を守る）。トリガー以外のエントリに書いた `trigger_start_offset` は検証で拒む
11. When #33 を直すとき, the dola shall `relative_to` から出した終わりの値も値域と比べて `ValueOutOfRange` を出し、コンパイル結果の `min_value`／`max_value` を、完了 `dola-compiled-transition` 要件 5.6 の用途どおりランタイムでの値のクランプに使う
12. When 件 3 の `too_many_arguments` を直すとき, the 本 spec の直し shall `runtime/instance_manager/mod.rs` の分は #60 の直し（`create_instance` を消す）で消し、`compile/resolve.rs` の分は理由を書いた `#[expect(clippy::too_many_arguments, reason = ...)]` で許す（引数の束の型は作らない）
