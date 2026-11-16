# Implementation Plan

> 触ってよいのは `crates/areka-placement/` の中と、`crates/areka/src/ghost_session.rs` の登録の 1 行、`doc/COMPAT_ARCHITECTURE.md`、この spec の `research.md` だけ。`crates/areka/src/runtime/` の下は同じウェーブの `runtime-crate-split` の持ち場なので触らない。テストはどれも実機の窓を使わない決定論のテストで、直す前の形で赤になることを確かめてから直す（要件 5.2）。

- [x] 1. 並べ替えの判定にほかのスコープのキャラ窓との重なりを入れる（件 1 の核）
- [x] 1.1 判定の核に「映した先がほかのキャラ窓に重なるなら押し込みへ倒す」段を足す
  - 判定の戻りに「重なったので押し込んだ」という変種を足す。位置は今の押し込みの位置そのまま、重なった最初のスコープを持つ。記録の語は今の `pushed` のまま、重なったスコープを読む口を足す。
  - 判定に「判定に入れるキャラ窓の矩形の列」を渡す引数を足す。今の分岐（収まる・キャラ窓がはみ出す・映した先が横に入らない・映しても x が変わらない）は変えず、並べ替えを返す直前にだけ段を足す。映しの式は変えない。
  - 数える重なりは「映して縦だけ収めた矩形と相手の窓の交わりが空でなく、自分のキャラ窓に収まり切らない」とき。矩形は半開で、接するだけは重ならない。列に自分の窓が入っていても数えない。
  - 引数が増えて壊れる今の呼び出しを全部直す: 起動の関門（`balloon_limit.rs` の 1 か所・中身は 1.2 で入れる）と書き込みの関門（`window_move.rs` の 1 か所・中身は 3.2 で入れる）はこの段では空の列を渡してコンパイルを通すだけ。`balloon_limit_flip_tests.rs` の今の呼び出しにも空の列を渡す。
  - 先に檻を書いて赤を確かめる: 縦にも横にも重なる合成の形（`PushedOverlap`・scope 1・直す前は `Flipped`）／4.3 ⑴ の形（`Flipped(232, 352)`）／本体と相方が同じ位置に重なる形（`Flipped`）／接するだけの形（`Flipped`）／3 スコープの列で 3 番目だけ重なる形（scope 2）／押し込んだ位置がなお重なる形（位置は押し込みのまま）／空の列（今と同一）。
  - 済んだ姿: 上の檻が `balloon_limit_flip_tests.rs` で緑、既存の並べ替えの檻もそのまま緑で、`cargo test -p areka-placement` が通る。
  - _Dispatch: standard — 判定の分岐と交わりの計算を新しく書く_
  - _Requirements: 2.1, 2.2, 2.4, 2.5, 2.8, 2.9, 3.8, 5.1, 5.2, 5.3, 6.1, 6.2, 6.4_

- [x] 1.2 起動の関門で全スコープのキャラ窓の矩形を判定へ渡し、押し込みの理由を記録する
  - 起動の関門の公開の署名は変えない。各スコープを判定する前に、置くスコープ全部の (スコープ, キャラ窓の矩形) の列を 1 度組み、同じ列を各スコープの判定へ渡す。キャラ窓の矩形を組む式は、列を組む側と 1 つの私有関数で共有する。
  - `[balloon-limit] Clamp` の行に欄 `overlap_scope` を足す（重なって押し込んだときだけ値がある）。収まるときは今どおり記録しない。
  - 済んだ姿: 2 スコープで本体の映した先が相方の窓に重なる檻で、本体のバルーンの位置が押し込みの位置になり、記録が `how="pushed"`・`overlap_scope=Some(1)`・`context="boot-gate"` の 1 行になる。重ならない形は `how="flipped"`・`overlap_scope=None`。どちらも `balloon_limit_flip_tests.rs` で緑。
  - _Dispatch: mechanical — 列の組み立てと記録の欄は設計が形を与えている_
  - _Requirements: 2.3, 2.6, 2.8, 4.1, 4.2, 4.3, 4.4, 5.1_

- [x] 2. (P) 右に出すバルーンの descript の横のずらしの向きを直す（件 2）
  - 1・3・4 と触るファイルが重ならないので、並べて進めてよい。
  - _Boundary: DescriptOffsetField, P5RightArm_
- [x] 2.1 合流の前の descript の `balloon.offsetx` の分を、配置の設定に別の欄で運ぶ
  - 配置の設定に「descript の `balloon.offsetx` の分」の欄を足す（既定 0）。設定を解くところで生の値を入れ、作者の拡大率の換算の後は換算後の値を入れる。`windowposition` の合流はこの欄に触らない。
  - 全欄を並べて設定を組む 2 か所（設定を解く関数と、キーワードのバルーンのテスト）に欄を足す。
  - 済んだ姿: 供給の檻（`balloon_offset_supply_tests.rs`）で、換算と合流の後の欄が descript の分の換算値に等しく `windowposition.x` の分を含まないこと、`sakura.`・`kero.`・無印のどの鍵から来ても同じ欄に入ることが緑。
  - _Dispatch: mechanical — 既存の型に欄を 1 つ足して既存の流れで値を写す_
  - _Requirements: 3.1, 3.4, 6.1_

- [x] 2.2 右に出す腕で descript の分だけをシェルに近づく向きに当てる
  - 右に決まったとき（`Right`・`Auto` で右）だけ、横の位置を「キャラ窓の右端 ＋（`windowposition.x` の分）− descript の分」にする。左の腕・縦・キーワードの腕は変えない。descript の分が 0 なら今の式と同一。
  - 先に檻を書いて赤を確かめる（`resolver_resolve_balloon_tests.rs`）: `Right`・descript の分 30 → 右端 − 30（直す前は +30）／`Left`・同じ値 → 今と同じ／`Auto` で右に決まる形 → 右端 − 30／`Right`・descript の分 0・`windowposition.x` の分 −237 → 右端 − 237。
  - 右に出す sakura（`offsetx,30`）の追従のずらしを期待する `reseed_tests.rs` の 4 か所を、`CHAR0.w + 30` から `CHAR0.w − 30` へ書き換える。記憶の値の道は P5 を通らないので変えない。
  - 済んだ姿: 上の檻と書き換えた 4 か所が緑で、`placement_windowposition_tests.rs` の左右同一の檻もそのまま緑。
  - _Dispatch: mechanical — 式は設計が与えており、変わる期待値も設計が挙げている_
  - _Requirements: 3.1, 3.2, 3.5, 3.6, 5.1, 5.2, 6.1, 6.4_

- [x] 3. 書き込みの関門に表示しているキャラ窓の矩形を渡す
- [x] 3.1 World から「判定に入れるキャラ窓の矩形の列」を組む
  - `follow/` の下に新しいファイルを置き、全スコープを昇順に回して、キャラ窓があり・窓の矩形が決まり・表示している、の 3 つが揃ったスコープだけを列に入れる。自分のスコープも外さない。
  - 表示しているかは、キャラ窓の子の描画面の `Visual.is_visible` のどれかが真かで読む。子が無い・全部不可視なら入れない。窓の一式が無ければ空の列で、記録はしない。
  - 窓の矩形を読む既存の私有関数を `follow/` の中から使えるようにし、ファサードに新しいモジュールとテストの接続を足す。
  - 済んだ姿: 新設の `follow_other_chars_tests.rs` で、可視の窓は入る・子が全部不可視の窓は入らない・寸の決まらない窓は入らない・3 スコープで昇順、が緑。
  - _Dispatch: standard — 子の描画面を引いて表示の有無を読む新しい読み口を書く_
  - _Requirements: 2.6, 2.8, 2.9, 6.1_

- [x] 3.2 書き込みの関門の判定へ矩形の列を渡し、押し込みの理由を記録する
  - 書き込みの関門の判定の直前に 3.1 の列を組んで渡す（1.1 で置いた空の列を置き換える）。押し込みの変種を受ける形を足し、`[balloon-limit] Clamp` に欄 `overlap_scope` を足す。ほかの `Unresolved` の分岐は変えない。`window_move.rs` の増分は数行に抑える。
  - 済んだ姿: `balloon_limit_gate_tests.rs` で、相方の子の `Visual` が可視で重なる → 押し込み＋`overlap_scope=Some(1)`、同じ形で子を全部不可視 → 並べ替え、相方の寸が未確定 → 並べ替え、が緑。
  - _Depends: 1.1_
  - _Dispatch: mechanical — 呼び出しへ引数を渡し記録の欄を足すだけで形は設計が与えている_
  - _Requirements: 2.3, 2.6, 4.1, 4.2, 4.3, 4.4, 5.1_

- [x] 4. 窓を並べ直した後に全スコープのバルーンを判定し直す（要件 2.10）
- [x] 4.1 全スコープのバルーンを今のキャラ窓の位置で判定し直す関数と、起動の並べ直しの後に 1 度走る系を作る
  - 判定し直しの本体は、`BalloonLimit(true)` のバルーンを持つスコープだけを先に集めてから、各キャラ窓の今の位置で随伴をし直す素の関数にする。随伴の引き金は並べ直しの書き込みと同じ既存の経路の語を使い、新しい語を作らない。
  - キャラ窓の位置が決まらないスコープは `[balloon-limit] Unresolved` の警告を残して飛ばす。
  - 起動の並べ直しの終わりの印が入った巡に 1 度だけ走る系を `FrameFinalize` に置き、プロセスに 1 回だけ載せる公開の登録の口を作る。
  - ファサード（`follow.rs`）で、登録の口を `pub` に、判定し直しの関数を crate の中（4.2 の `reset_pos.rs`・`chain_realign.rs`）から呼べるよう `pub(crate)` に再束縛する。
  - 済んだ姿: `follow_other_chars_tests.rs` で、(a) 相方の窓を本体の映した先に重なる位置へ動かしてから印を入れて系を回すと本体のバルーンが押し込みの位置に書かれる（直す前は並べ替えた位置のまま＝赤）、(b) 印を入れずに相方を動かすと本体のバルーンは動かず、印の後の 2 回目の巡では系が走らない、(c) どの窓も重ならない形では全バルーンの位置がビット同一で記録の行が増えない、(d) `windowposition.limit,0` のバルーンには書き込みが出ない、が緑。
  - _Dispatch: standard — 新しい系と引き金と対象の絞り込みを書く_
  - _Requirements: 2.7, 2.10, 4.2, 4.4, 5.1, 5.2, 6.4_

- [x] 4.2 シェル位置初期化と拡大率の解き直しの後にも判定し直す
  - シェル位置初期化と、拡大率が変わった後の鎖の解き直しが全スコープの窓を書き終えた後に、4.1 の関数を 1 行ずつ呼ぶ。
  - 檻はどれも `areka-placement` の兄弟のファイルに置く: シェル位置初期化は `reset_pos_tests.rs`、拡大率の解き直しは `chain_realign_script_moved_tests.rs`（解き直しの関数が取る窓の集め方の閉包はテストが自分で渡す）。`runtime/` の下の檻には足さない。
  - 済んだ姿: 本体を先に動かす順で相方の窓が本体のバルーンの映した先に重なる位置へ戻る形を、2 つの場面で 1 回ずつ回す檻が緑で、本体のバルーンが押し込みの位置に書かれる。呼び出しを外すと並べ替えた位置のまま（赤）。
  - _Dispatch: standard — 2 つの場面の重なる形を組む檻に考えが要る_
  - _Requirements: 2.10, 5.1, 5.2_

- [x] 4.3 アプリの登録に判定し直しの系を 1 行で載せる
  - 足す前に、同じウェーブの `runtime-crate-split` のセッションへ `crates/areka/src/ghost_session.rs` の `register_systems` の末尾に 1 行足すことを知らせる。
  - `register_systems` の末尾に 4.1 の登録の口を呼ぶ 1 行を足す。`runtime/` の下は触らない。
  - 済んだ姿: `cargo build -p areka` と `cargo test -p areka` の `zorder_wiring_tests` が緑で、`git diff --stat` に `crates/areka/src/runtime/` が出ない。
  - _Dispatch: mechanical — 登録の 1 行と知らせだけ_
  - _Requirements: 2.10_

- [x] 5. 記録の表と調べの記録を直す
- [x] 5.1 (P) `windowposition.x` の古い注記と互換対応表の ⚠ を今の ukadoc に合わせる
  - `windowposition.rs` の冒頭の説明と符号規約の節の「正典はシェル側が+と記述する／乖離は記録対象」を、2026-09-20 の ukadoc の改め（ukatech/ukadoc の f92e4ff9・「左右どちらに出していても常に右方向が+」）と SSP の実測が一致した、という文に改める。式と表は変えない。
  - `doc/COMPAT_ARCHITECTURE.md` の `windowposition.x` の符号規約の行の ⚠ を同じ事実に改める。
  - 済んだ姿: 2 か所の文が改まり、`windowposition.rs` の差分がコメントだけで、`cargo test -p areka-placement` の `placement_windowposition_tests` が緑。
  - _Boundary: CompatDoc_
  - _Dispatch: mechanical — 文の中身は設計の決めごと D5 が与えている_
  - _Requirements: 5.4_

- [x] 5.2 互換対応表 §8 の「バルーンだけが作業領域からはみ出すとき」の行に、2 つの件と SSP の測りを書き足す
  - 設計の CompatDoc の ⑴〜⑷（重なれば押し込み・押し込んだ後の重なりは許す・並べ直しの後に判定し直す／右の descript の分は左向きが＋・`windowposition.x` の数値は左右とも右向き／SSP 2.9.34 は並べ替えも押し込みもしないので、どれも areka の裁量／記録の欄 `overlap_scope`）を足し、「測っていない」の記述を置き換え、並べ替えを残すかの見直しは `balloon-flip-ssp-review` が持つと書く。
  - 済んだ姿: §8 の該当の行に 4 つが載り、`research.md` 9 節を指している。
  - 5.1 と同じ `doc/COMPAT_ARCHITECTURE.md` を触るので、5.1 の後に回す。
  - _Depends: 1.2, 2.2, 3.2, 4.2, 5.1_
  - _Dispatch: mechanical — 書く中身は設計が与えている_
  - _Requirements: 5.4_

- [x] 5.3 要件 3.7 の数えを確定し、測りの記録を確かめる
  - 配布版のぱすた（リポジトリの外）の kero の `balloon.alignment` と `balloon.offsetx` を読み（写すなら `target\` の下）、`research.md` 10.5 節に書き足して、右に出すバルーンで横のずらしが 0 でない形が何本あるかを確定する。読めなければ「読めなかった」とその理由を残す。
  - `research.md` 9 節に、測りの版・検体・手順・値・測れなかった理由・知らせ・戻しが残っており、決めごとを覆していないことを確かめる（足りなければ足す）。SSP は新しく測らない。
  - 済んだ姿: 10.5 節に配布版のぱすたの kero の値（または読めなかった理由）と数えの結論が書かれている。
  - _Dispatch: mechanical — 値を読んで表に書き足すだけ_
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 3.7_

- [ ] 6. 件 1 と件 2 がそろうことの確かめと、変えないものの見張り
- [ ] 6.1 初めから右に出したバルーンと、左から並べ替えて右に来たバルーンの位置が descript の分についてそろうことを固定する
  - `Right`・descript の分 d で初めから出した位置と、`Left`・同じ d で出して判定の核で左から右へ映した位置の x が同じになる檻を足す。`windowposition.x` の分が 0 でない形では、映しの結果が今の式のままであることも固定する。
  - 済んだ姿: `balloon_limit_flip_tests.rs` のこの 2 つの檻が緑で、2.2 の右の腕を戻すと 1 つ目が赤になる。
  - _Depends: 1.1, 2.2_
  - _Dispatch: mechanical — 期待値は設計の式の確かめが与えている_
  - _Requirements: 3.3, 3.8, 5.1_

- [ ] 6.2 全体を回して、変えないものが変わっていないことを確かめる
  - `cargo test -p areka-placement` と `cargo test -p areka` を回し、既存の並べ替え・関門・随伴・`windowposition` の檻と、`runtime/` の下の鎖の仕上げ・解き直しの檻（`frame_chain_finalize_*`・`frame_chain_realign_*`）が緑のままであることを確かめる（書き換えたのは `reseed_tests.rs` の 4 か所だけ・`runtime/` は読むだけで触らない）。
  - `git diff` で、鎖の寄せ（`follow/work_area_clamp.rs`）・押し込みの式・ドラッグを離したときの補正・キーワードの横の位置・記憶の合流・`crates/areka/src/runtime/` に差分が無いこと、新しく足したファイルが 1,000 行以下であること、ゴーストの名前や初回の起動を見る分岐が無いことを確かめる。
  - 済んだ姿: 2 つのコマンドが緑で、差分の見張りの 3 点が満たされている。
  - _Dispatch: mechanical — 回して差分を読むだけ_
  - _Requirements: 5.1, 6.1, 6.2, 6.3, 6.4_

## Implementation Notes
- `cargo clippy -p areka-placement --all-targets -- -D warnings` は着手前から赤（`wintf` と、この spec が触らないファイルの指摘）。各タスクでは「差分の行に新しい指摘を足さない」を `--no-deps` の出力で確かめる。
- 2.1: descript の `balloon.offsetx` は `sakura.`/`char0.`（scope 0）と `kero.`/`char1.`（scope 1）の鍵だけが読まれ、無印の `balloon.offsetx` は surfaces.txt の surface ブレスの鍵（要件 3.4 で範囲外）。requirements.md 3.4・design.md の Unit Tests・research.md・このタスクの 2.1 にある「無印」の語は不正確（完了の棚卸で文を直す）。
- 2.2: 右の腕の直しで `reseed_tests.rs` は 4 か所でなく 5 か所（192 DPI の `+60`→`-60` を含む）、さらに設計が見落とした `crates/areka/src/runtime/frame/switch_finish_face_tests.rs` の対照の期待値 1 行（`434 + 30 + 40`→`434 - 30 + 40`）が変わった。要件 3.1 の直接の帰結なので、テストの期待値だけ直した（ロジックは触らない）。`runtime-crate-split` に知らせ済みで、向こうはこのファイルを `crates/areka/src/runtime_tests/frame/switch_finish_face_tests.rs` へ `git mv` する（期待値の行は触らない）。後から着地した側が rebase で合わせる。6.2 の「`runtime/` に差分が無い」は、このテストの 1 行を除いて読む。
- 4.3: `runtime-crate-split` は `ghost_session.rs` を動かさないが、段 4 で `crate::runtime::` の道を `areka_runtime::` へ機械で置き換える。足す 1 行は `runtime::` の道を含めない形にする。
- 3.1: `follow.rs` の `mod other_chars;` の `#[allow(dead_code)]` と `shown_char_rects` の use の `#[allow(unused_imports)]` は仮置き。3.2 で呼び出しを足したら外す。「表示している」は子の `Visual.is_visible` で読むが、`emo-text-box-surface`（`Visual::default()`＝可視）が窓を隠したときに箱の同期で外れる、という設計の前提はまだ確かめていない（6.2 か完了の棚卸で確かめる）。
- 3.2: 書き込みの関門の檻は `balloon_limit_gate_tests.rs`（起動の関門の檻）でなく、既存の `write_gate_*` と道具がある `balloon_limit_flip_tests.rs` に置いた。
- 4.1: 判定し直しの系と関数は `follow/other_chars.rs` に置いた。`follow.rs` の `pub(crate) use ... rejudge_all_balloons` の `#[allow(unused_imports)]` は仮置きで、4.2 で呼び出しを足したら外す。判定は状態を持たないので、並べ替えたままのバルーンは判定し直しのたびに `how="flipped"` の行をもう 1 度残す。
