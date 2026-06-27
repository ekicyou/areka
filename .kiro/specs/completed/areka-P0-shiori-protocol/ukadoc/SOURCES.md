# ukadoc 一次資料スナップショット（典拠ピン）

> 本フォルダは互換契約の正典である ukadoc の該当ページを、契約抽出の一次資料として**ピン留め**したもの。
> 本仕様（`areka-P0-shiori-protocol`）の実装＝これらを解析して正準イベントカタログ・フィールドスキーマ・対応表を抽出する作業。
> ukadoc 更新時は再取得し、ハッシュ差分で契約への影響をレビューする（COMPAT §2 沈黙ルールの是正フック）。

## 取得情報

| ファイル | 出典 URL | 取得日 | sha256 |
|---------|---------|--------|--------|
| `list_shiori_event.html` | https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html | 2026-06-27 | `0e7f119f537a782ac551ab942c97879b6a748f0ade55c730ab3ea3649bd15da4` |
| `list_shiori_resource.html` | https://ssp.shillest.net/ukadoc/manual/list_shiori_resource.html | 2026-06-27 | `30da3a5878d6a26ecc3a3fae9ab9c11e5fee583fd1bada80e3c7f9eb9c574bb0` |
| `spec_shiori3.html` | https://ssp.shillest.net/ukadoc/manual/spec_shiori3.html | 2026-06-27 | `358a52519539a72847839ace792f5f333b7793e1823bfb2d5894af0d2bff7c27` |

## 役割（要件への対応）
- **Requirement 1**（正準イベントカタログ）: `list_shiori_event.html` が全 SHIORI イベントの列挙元。
- **Requirement 2/3**（フィールドスキーマ・対応表）: 各イベントの `Reference*` 定義と意味を両ページから抽出。
- **Requirement 6**（予約ヘッダ）: `spec_shiori3.html`（SHIORI/3.0 規約本体）が予約ヘッダ集合の主典拠。ukadoc で使われる範疇のヘッダを母集合として確定する。
- **Requirement 7**（沈黙ルール典拠追跡）: 対応表の「典拠」列は本スナップショットの記述有無を参照する。

## 注意
- これらは UKADOC Project の著作物。リポジトリへの同梱可否（再配布ライセンス）は別途確認のこと。必要なら本フォルダを `.gitignore` 化し、`SOURCES.md`（URL＋ハッシュ）のみ追跡する運用に切替可能。
