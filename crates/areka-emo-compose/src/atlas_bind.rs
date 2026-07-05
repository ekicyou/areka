//! `AtlasBinder`: 正規化定義の `ElementPath` を `AtlasTable` の `ElementId` へ一度きり解決する。
//!
//! 構築時（load-time・ゴーストごと1回）に `ElementPath`→`ElementId` を resolve し、結果を
//! `AtlasBinding` コンポーネントとして World へ挿入する。以後の毎フレーム参照は O(1) の `entry`
//! 引きに帰着する。解決不能なパスはパニックせず `warn` 以上で観測可能に扱う。
