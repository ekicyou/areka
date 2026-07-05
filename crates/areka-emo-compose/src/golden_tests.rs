//! emo2 fixture の pixel 観測 golden テスト（in-source `#[cfg(test)]`・`MemoryDecoder`+bake 経路）。
//!
//! 実上流（実行時エンジン）非依存に、fixture／正規化モデル直入力で合成結果を観測する。
//! surface1000 への有効 bind 集合合成・トリム前後の見た目等価・全透明退化の正常返却などを
//! バイト等価で検証する（後続タスクで具体ケースを充填）。
