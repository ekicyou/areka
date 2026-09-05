# shiori の網羅状況

この本文は台帳から機械で作ります。手で書き換えず、食い違いは作り直して直します。

## 状態の分布

| 状態 | 件数 |
| --- | ---: |
| 実装済み | 21 |
| 語彙のみ | 161 |
| 縮退 | 3 |
| 未対応 | 320 |
| 別名 | 3 |
| 対象外 | 169 |
| 未分類 | 0 |
| 合計 | 677 |

## ページ別の状態の分布

未分類の残りがどのページに何件あるかは、この表の「未分類」の列が正です。

| ページ | 実装済み | 語彙のみ | 縮退 | 未対応 | 別名 | 対象外 | 未分類 | 合計 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| list_plugin_event | 0 | 0 | 0 | 19 | 0 | 0 | 0 | 19 |
| list_shiori_event | 11 | 3 | 0 | 273 | 3 | 0 | 0 | 290 |
| list_shiori_event_ex | 0 | 0 | 0 | 0 | 0 | 168 | 0 | 168 |
| list_shiori_resource | 1 | 158 | 0 | 0 | 0 | 0 | 0 | 159 |
| memo_shiorievent | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 1 |
| spec_dll | 0 | 0 | 1 | 0 | 0 | 0 | 0 | 1 |
| spec_fmo_mutex | 0 | 0 | 0 | 6 | 0 | 0 | 0 | 6 |
| spec_headline | 0 | 0 | 0 | 1 | 0 | 0 | 0 | 1 |
| spec_plugin | 0 | 0 | 0 | 1 | 0 | 0 | 0 | 1 |
| spec_shiori3 | 9 | 0 | 2 | 15 | 0 | 0 | 0 | 26 |
| spec_sstp | 0 | 0 | 0 | 2 | 0 | 0 | 0 | 2 |
| spec_web | 0 | 0 | 0 | 3 | 0 | 0 | 0 | 3 |

## SSP 世代別の対応表

| 世代 | 実装済み | 語彙のみ | 縮退 | 未対応 | 別名 | 対象外 | 未分類 | 合計 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 2.3 | 0 | 0 | 0 | 5 | 0 | 0 | 0 | 5 |
| 2.4 | 0 | 2 | 0 | 10 | 0 | 0 | 0 | 12 |
| 2.5 | 0 | 4 | 0 | 24 | 0 | 0 | 0 | 28 |
| 2.6 | 0 | 0 | 1 | 17 | 0 | 0 | 0 | 18 |
| 2.7 | 0 | 3 | 0 | 18 | 3 | 0 | 0 | 24 |
| 2.8 | 0 | 0 | 0 | 11 | 0 | 0 | 0 | 11 |
| 世代不明 | 21 | 152 | 2 | 235 | 0 | 169 | 0 | 579 |

## 別名の一覧

| 別名の id | 指す先の id |
| --- | --- |
| ukadoc:list_shiori_event:OnFileDrop:1 | ukadoc:list_shiori_event:OnFileDrop2:1 |
| ukadoc:list_shiori_event:OnFileDropEx:1 | ukadoc:list_shiori_event:OnFileDrop2:1 |
| ukadoc:list_shiori_event:OnFileDropped:1 | ukadoc:list_shiori_event:OnFileDrop2:1 |

## テーマ別の状態分布

| テーマ | 実装済み | 語彙のみ | 縮退 | 未対応 | 別名 | 対象外 | 未分類 | 合計 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 交わり | 0 | 0 | 0 | 30 | 0 | 0 | 0 | 30 |
| 掛け合い | 3 | 1 | 0 | 12 | 0 | 0 | 0 | 16 |
| 更新 | 0 | 3 | 0 | 26 | 0 | 0 | 0 | 29 |
| 気配 | 3 | 0 | 0 | 8 | 0 | 0 | 0 | 11 |
| 気配り | 0 | 3 | 0 | 46 | 0 | 0 | 0 | 49 |
| 装い | 0 | 131 | 0 | 15 | 0 | 0 | 0 | 146 |
| 触れ合い | 2 | 1 | 0 | 37 | 0 | 0 | 0 | 40 |
| 記憶 | 3 | 2 | 0 | 22 | 0 | 0 | 0 | 27 |

## ドメイン内で関連が閉じている束

| 束 id | 構成 id |
| --- | --- |
| ukadoc:list_plugin_event:OnInstallComplete:1 | ukadoc:list_plugin_event:OnInstallComplete:1, ukadoc:list_shiori_event:OnInstallComplete:1 |
| ukadoc:list_plugin_event:balloonpathlist:1 | ukadoc:list_plugin_event:balloonpathlist:1, ukadoc:list_shiori_event:balloonpathlist:1 |
| ukadoc:list_plugin_event:ghostpathlist:1 | ukadoc:list_plugin_event:ghostpathlist:1, ukadoc:list_shiori_event:ghostpathlist:1 |
| ukadoc:list_plugin_event:headlinepathlist:1 | ukadoc:list_plugin_event:headlinepathlist:1, ukadoc:list_shiori_event:headlinepathlist:1 |
| ukadoc:list_plugin_event:installedballoonname:1 | ukadoc:list_plugin_event:installedballoonname:1, ukadoc:list_shiori_event:installedballoonname:1 |
| ukadoc:list_plugin_event:installedghostname:1 | ukadoc:list_plugin_event:installedghostname:1, ukadoc:list_shiori_event:installedghostname:1 |
| ukadoc:list_plugin_event:installedplugin:1 | ukadoc:list_plugin_event:installedplugin:1, ukadoc:list_shiori_event:installedplugin:1 |
| ukadoc:list_plugin_event:pluginpathlist:1 | ukadoc:list_plugin_event:pluginpathlist:1, ukadoc:list_shiori_event:pluginpathlist:1 |
| ukadoc:list_shiori_event:OnBatteryCritical:1 | ukadoc:list_shiori_event:OnBatteryCritical:1, ukadoc:list_shiori_event_ex:OnBatteryCritical:1 |
| ukadoc:list_shiori_event:OnBatteryLow:1 | ukadoc:list_shiori_event:OnBatteryLow:1, ukadoc:list_shiori_event_ex:OnBatteryLow:1 |
| ukadoc:list_shiori_event:OnCommunicate:1 | ukadoc:list_shiori_event:OnCommunicate:1, ukadoc:list_shiori_event_ex:OnGetValues:1, ukadoc:list_shiori_event_ex:OnKanadeTeaPartyInfomationRequest:1, ukadoc:list_shiori_event_ex:OnMahjong:1, ukadoc:list_shiori_event_ex:OnPoker:1, ukadoc:list_shiori_event_ex:OnRequestValues:1, ukadoc:list_shiori_event_ex:Send60stair_GetStatus:1, ukadoc:list_shiori_event_ex:_53ef_5909_540d_306e_8fd4_4fe1_30a4_30d9_30f3_30c8:1 |
| ukadoc:list_shiori_event:OnMouseClick:1 | ukadoc:list_shiori_event:OnMouseClick:1, ukadoc:list_shiori_event:OnMouseClickEx:1 |
| ukadoc:list_shiori_event:OnMusicPlay:1 | ukadoc:list_shiori_event:OnMusicPlay:1, ukadoc:list_shiori_event_ex:OnMusicPlay:1 |
| ukadoc:list_shiori_event:OnUpdate.OnDownloadBegin:1 | ukadoc:list_shiori_event:OnUpdate.OnDownloadBegin:1, ukadoc:list_shiori_event:OnUpdate.OnMD5CompareBegin:1, ukadoc:list_shiori_event:OnUpdate.OnMD5CompareComplete:1, ukadoc:list_shiori_event:OnUpdate.OnMD5CompareFailure:1, ukadoc:list_shiori_event:OnUpdateBegin:1, ukadoc:list_shiori_event:OnUpdateComplete:1, ukadoc:list_shiori_event:OnUpdateFailure:1, ukadoc:list_shiori_event:OnUpdateOther.OnDownloadBegin:1, ukadoc:list_shiori_event:OnUpdateOther.OnMD5CompareBegin:1, ukadoc:list_shiori_event:OnUpdateOther.OnMD5CompareComplete:1, ukadoc:list_shiori_event:OnUpdateOther.OnMD5CompareFailure:1, ukadoc:list_shiori_event:OnUpdateOtherBegin:1, ukadoc:list_shiori_event:OnUpdateOtherComplete:1, ukadoc:list_shiori_event:OnUpdateOtherFailure:1, ukadoc:list_shiori_event:OnUpdateOtherReady:1, ukadoc:list_shiori_event:OnUpdateProcessExec:1, ukadoc:list_shiori_event:OnUpdateReady:1, ukadoc:list_shiori_event:OnUpdatedataCreated:1, ukadoc:list_shiori_event:OnUpdatedataCreating:1 |
| ukadoc:list_shiori_event:OnUpdateCheckComplete:1 | ukadoc:list_shiori_event:OnUpdateCheckComplete:1, ukadoc:list_shiori_event:OnUpdateCheckFailure:1 |
| ukadoc:list_shiori_event:OnUpdateCheckResult:1 | ukadoc:list_shiori_event:OnUpdateCheckResult:1, ukadoc:list_shiori_event:OnUpdateCheckResultEx:1, ukadoc:list_shiori_event:OnUpdateResult:1, ukadoc:list_shiori_event:OnUpdateResultEx:1, ukadoc:list_shiori_event:OnUpdateResultExplorer:1 |
| ukadoc:list_shiori_resource:char_2a.defaultleft:1 | ukadoc:list_shiori_resource:char_2a.defaultleft:1, ukadoc:list_shiori_resource:kero.defaultleft:1, ukadoc:list_shiori_resource:sakura.defaultleft:1 |
| ukadoc:list_shiori_resource:char_2a.defaulttop:1 | ukadoc:list_shiori_resource:char_2a.defaulttop:1, ukadoc:list_shiori_resource:kero.defaulttop:1, ukadoc:list_shiori_resource:sakura.defaulttop:1 |
| ukadoc:list_shiori_resource:char_2a.defaultx:1 | ukadoc:list_shiori_resource:char_2a.defaultx:1, ukadoc:list_shiori_resource:kero.defaultx:1, ukadoc:list_shiori_resource:sakura.defaultx:1 |
| ukadoc:list_shiori_resource:char_2a.defaulty:1 | ukadoc:list_shiori_resource:char_2a.defaulty:1, ukadoc:list_shiori_resource:kero.defaulty:1, ukadoc:list_shiori_resource:sakura.defaulty:1 |
| ukadoc:list_shiori_resource:char_2a.popupmenu.applybindtoself:1 | ukadoc:list_shiori_resource:char_2a.popupmenu.applybindtoself:1, ukadoc:list_shiori_resource:kero.popupmenu.applybindtoself:1, ukadoc:list_shiori_resource:sakura.popupmenu.applybindtoself:1 |
| ukadoc:list_shiori_resource:char_2a.popupmenu.type:1 | ukadoc:list_shiori_resource:char_2a.popupmenu.type:1, ukadoc:list_shiori_resource:kero.popupmenu.type:1, ukadoc:list_shiori_resource:sakura.popupmenu.type:1 |
| ukadoc:list_shiori_resource:char_2a.popupmenu.visible:1 | ukadoc:list_shiori_resource:char_2a.popupmenu.visible:1, ukadoc:list_shiori_resource:kero.popupmenu.visible:1, ukadoc:list_shiori_resource:sakura.popupmenu.visible:1 |
| ukadoc:list_shiori_resource:char_2a.recommendbuttoncaption:1 | ukadoc:list_shiori_resource:char_2a.recommendbuttoncaption:1, ukadoc:list_shiori_resource:kero.recommendbuttoncaption:1, ukadoc:list_shiori_resource:sakura.recommendbuttoncaption:1 |
| ukadoc:list_shiori_resource:char_2a.recommendsites:1 | ukadoc:list_shiori_resource:char_2a.recommendsites:1, ukadoc:list_shiori_resource:kero.recommendsites:1, ukadoc:list_shiori_resource:sakura.recommendsites:1 |
