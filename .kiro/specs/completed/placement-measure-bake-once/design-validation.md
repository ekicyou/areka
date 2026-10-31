# 設計の検証: placement-measure-bake-once

> 2026-10-31・ブランチ `claude/kiro-start-placement-measure-bake-once-80df62`（`51b3149c`）で、design.md の主張をコードと突き合わせて読んだ。対話なしで判定まで出す。

## 要約

設計は、要件討議で決めた形（採寸だけ動く絵を 1 枚目で読む読み手で包む・採寸の道で読み込みの記録を出さない）をそのまま、触る所の少ない形に落としている。コードと突き合わせた主張はほぼ正しく、実装に進める。ただし「焼く段の記録の行の数は今と同じ」という主張には 1 つ穴があり、もう 1 つ「descript が読めない」の記録が 1 行にならない場合が残る。どちらも設計の議論で決めれば直せる。

## コードで確かめたこと

| 設計の主張 | 確かめた結果 |
|---|---|
| `ElementDecoder` は `&self` の 5 つの口（`decode`・`probe_pna`・`probe_animation`・`decode_frames`・`decode_first_frame`）で、後ろ 3 つと `probe_pna` は既定の答えを持つ | 正しい（`areka-emo-atlas/src/decode.rs:104-143`）。`FirstFrameDecoder` が `decode` と `probe_pna` だけを書けば組める |
| 焼く段は `probe_animation` が `None` なら `decode` だけを呼ぶ（動く絵の 3 つの `warn!` は出ない） | 正しい（`animated.rs:42-43` の `still`） |
| 窓の大きさは焼いた表の元の寸法（`original`）で決まり、1 枚目だけで焼いても変わらない | 正しい。外形の計算（`areka-emo-compose/src/plan_extent.rs:117-126`）は `original` を使い、`placement` が `None`（全透明）でも数える。設計の Risks にある「1 枚目が全透明だと寸法が変わり得る」は起きない |
| 配置の scope は 1 まで、資産の scope は `[0, 1]` | 正しい（`placement/config.rs:142-158` の `detect_scopes`・`emo2_boot/mod.rs:306` の `derive_scopes`） |
| `build_shell_assets`・`build_balloon_assets` を `&impl ElementDecoder` にしても `switch_assets.rs` は変更 0 で通る | 正しい。本文で読み手を渡す先（`load_shell_target`・`resolve_box_markers`・`build_balloon_target_from_faces`・`load_scope_balloon_definitions`）はどれもすでに `&impl ElementDecoder` を受ける。`switch_assets.rs:219`・`251` は `&WicDecoderArm` を渡すだけ。ほかの呼び手（`assets_shell_tests.rs`・`assets_tests.rs`・`offline_dump_tests.rs`）も同じ |
| 記録を出さない入口の上でも、`Err` で終える失敗の `error!` は残る | 正しい。シェルは一覧の失敗（`shell_target.rs:763`）・`surfaces.txt` が読めない（348）・面が無い（366）の 3 つ。バルーンは走査の失敗（`balloon.rs:322` の中）と面 0 が無い（`resolve_balloon_faces` の中）。`load_balloon_use_self_alpha` は走査の失敗で `error!` を出して続け、続く系列の解決が `Err` を返す |
| `placement/mod.rs`（970 行）は呼ぶ所の付け替えだけで行数が変わらない | 正しい見込み。97 行目の `use` は付け替え後も 100 桁に収まる（約 89 桁）ので、整形で行が割れない。803 行目も同じ |
| `ghost_session.rs`（940）・`measure_tests.rs`（1,000）は変更 0 | 設計どおりに作れば守れる。`measure_tests.rs` から見える関数の形は変わらない |
| 本数が増えるファイルの行数 | `measure.rs` 465・`balloon.rs` 701・`assets.rs` 540・`boot_shell_tests.rs` 218 は余裕がある。`shell_target.rs` は 874 行で、記録の行を `if records.emits()` で囲むと 30〜60 行増える見込み（上限 1,000 の内）。末尾の記録の塊（`shell_target.rs:401-468`）は 1 つの `if` でまとめて囲めば増え方が小さい |

## 重要な問題（3 件）

### 問題 1: 1 枚目が全透明の動く絵で、採寸の道に「全透明」の `warn!` が新しく出る

- **何が起きるか**: 焼く段の「全透明」の `warn!`（`areka-emo-atlas/src/lib.rs:179` の条件 `trim.placement.is_none() && frames.as_ref().is_none_or(|f| f.all_transparent())`）は、動く絵なら全部のコマが透明なときだけ出る。採寸の読み手は動く絵を静止画（1 枚目）として見せるので、1 枚目だけが透明（出てくる・ふわっと現れる動きなど）の動く絵では、今は出ない警告が採寸の道で起動のたびに 1 行出る。
- **なぜ困るか**: 要件 3.7（焼く段の記録の行の数を変えない）と設計の決めごと D4（「今と同じ数だけ出る」）に反する。正しい絵に作者のミスを疑う警告を出すことになる。窓の大きさは変わらない（`original` で数えるため）。
- **直し方の案**: 設計の議論で次のどちらかに決める。(a) 増える 1 行を受け入れ、D4 と要件 3.7 の扱いに書き足し、完了の棚卸で起票する焼く段の記録の件に入れる。檻には 1 枚目が透明な動く絵の検体を足して行数を固定する（今の検体 `animated-playback` は 3 番のコマが透明なだけで、この場合を通らない）。(b) 焼く段に手を入れずに防ぐ手は無いので、防ぐなら `areka-emo-atlas` を境界に入れ直す（議題 1 の「atlas は変えない」と衝突する）。推奨は (a)。
- **要件**: 3.7・2.2（Risks の書き直し）
- **設計の場所**: 決めごと D4・FirstFrameDecoder の Risks・採寸の付け替えの Risks

### 問題 2: バルーンの「descript が読めない」の記録が、壊れたバルーンでは 1 行にならない

- **何が起きるか**: バルーンの `descript.txt` が読めないときの `warn!` は、`load_balloon_use_self_alpha` と `load_scope_balloon_model`（2 層マージ）が同じ関数 `read_descript_layer`（`balloon.rs:438`）から同じ文言で出す。設計（D3）は前者だけを 1 行にし、後者は `windowposition` の読み（scope ごと）と資産の面ごとの定義の読み（`balloon_definitions.rs:80`・面ごと）から今と同じ数だけ出す。
- **なぜ困るか**: 要件 3.2 は「descript が読めない」を 1 行にする記録に挙げている。同じ文言の行が scope と面の数だけ並ぶので、要件の目的（警告の数をそのまま問題の数として数えられる）がこの記録では果たせない。壊れたバルーンのときだけの話で、起動の結果には効かない。
- **直し方の案**: D3 の読み方を設計の議論で確かめる。(a) D3 のまま受け入れ、残る重なりとして完了の棚卸の起票の件（要件の「決まったこと」議題 2 の最後の項）に書き足す。(b) `windowposition` の読みが呼ぶ `load_scope_balloon_model` にも記録を出さない入口を足す（2 層マージの `info!` を変えないこととの兼ね合いを決める必要がある）。推奨は (a)。
- **要件**: 3.2・3.7
- **設計の場所**: 決めごと D3・Out of Boundary の 2 点目

### 問題 3: バルーンの焼く回数の檻が、scope ごとの数を区別できない

- **何が起きるか**: 数える読み手はファイルごとに数えるが、読み手には scope が渡らない。動く絵の検体のバルーン（`balloons0.png`・`balloons1.png` の `s` の系列だけ）では scope 1 が本体側の系列へ縮退し、scope 0 と 1 が同じファイルを焼く。さらに採寸の scope は `detect_scopes`（`kero.*` が無ければ `[0]`）、資産は `[0, 1]` で、数が食い違う。設計の書く「バルーンの面の絵は scope ごとに採寸 1・資産 1（計 2）」は、ファイルごとの数としてはそのまま出ない。
- **なぜ困るか**: 要件 4.1 の「バルーンを scope ごとに 2 回」を固定する檻の期待値が、実装の段で決め直しになる。数え方を誤ると、回数が増えても赤にならない（要件 4.6）。
- **直し方の案**: 檻の書き方を設計に足す。採寸に渡す scope を `[0, 1]` に揃え、期待値を「ファイルごとに 採寸 = そのファイルを使う scope の数・資産 = 同じ数」と書く。または scope ごとに系列が分かれる emo2 のバルーン（`placement_shared_test_support::emo2_root`。設計の書く `shared_test_support::emo2_root` はこの名前）で scope ごとの数を確かめる。
- **要件**: 4.1・4.6
- **設計の場所**: Testing Strategy の Integration Tests 1・2

## 設計のよい点

- **触る所が少なく、境界がはっきりしている**: 読み込みの中身は 1 本のまま、入口だけを 2 つにする。焼く段・畳み込み・着替えの道・配置の決まりは変えず、上限に近い `placement/mod.rs` と `ghost_session.rs` は行数 0 の付け替えか変更 0 に収めている。資産の組み立ての読み手の口も、引数の型を広げるだけで呼び手を変えずに済むことをコードで確かめた。
- **記録の無い失敗の道を作らない**: 記録を出さない入口の上でも残す `error!` を表で入口ごとに挙げ、どれも今のコードの `Err` の道と一致していた。準備の後・資産の前に起動が止まった場合も、採寸の失敗は採寸の道で `error!` が出ることを書いている。

## 判定

**GO**

- **理由**: 既存の構造と衝突する所は無く、要件 1〜5 のすべてに実装の道がある。3 件の問題は記録の行の数と檻の書き方の細部で、設計の議論で扱いを決めれば、構造を変えずに直せる。
- **次の手順**: 設計の議論（`/kiro-design-discussion placement-measure-bake-once`）で問題 1〜3 の扱いを決め、design.md の D3・D4・Testing Strategy に書き足す。その後 `/kiro-spec-tasks placement-measure-bake-once`。
