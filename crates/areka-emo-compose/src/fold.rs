//! single-pass fold: parser の登場順定義ストリームを `EmoWorld` へ畳み込む。
//!
//! plain `surfaceN,M`／`N-M` は全 id を新設・`surface.append` は既存 id のみへ追記
//! （存在条件付き・ukadoc 意味論）。ターゲット記述子の単一・列挙・範囲を展開し、除外指定
//! （`!N`／`!a-b`）を展開時に減算適用する。複数定義が同一 surface に効く場合は登場順を保った
//! 順序で決定的に適用し、append ブロックが持つ element・collision・animation を対象 surface へ
//! 反映しつつ alias を収集する。参照 id が存在しない場合はパニックせず `warn` 以上で観測可能に扱う。
