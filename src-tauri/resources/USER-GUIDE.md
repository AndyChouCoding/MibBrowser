# MIB Browser 1.0.0 候選版操作手冊

## 安裝與啟動

macOS 安裝目標為 13 以上的 Intel 與 Apple Silicon；開啟通過發行簽章驗證的 DMG，將 MIB Browser 拖至 Applications。Windows 安裝目標為 Windows 10/11 x64；執行通過簽章驗證的 NSIS 安裝程式，缺少 WebView2 時由隨附的離線安裝程式補裝。桌面程式使用內建 Rust 引擎，使用者端無須安裝 Node、Rust 或 Net-SNMP。

## 瀏覽 MIB

首次啟動載入 mib-rs 內建的七個 SMIv1 與 SMIv2 基礎模組。用「＋」或拖放加入設備的 MIB 與相依檔案。缺少 IMPORTS 時畫面會顯示模組名稱，可再選取缺檔。成功後才儲存路徑與切換工作區。移動檔案前請保留原始檔與其相依模組。

點模組名稱切換顯示。以 Cmd+F 或 Ctrl+F 跳至搜尋，輸入名稱或數字 OID。樹狀項目可用 Tab 聚焦、Enter 選取、左右鍵收合與展開。選取物件可看型別、存取權、索引、列舉值與描述，Copy OID 會複製數字 OID。

## 查詢及 Profile

輸入設備位址與 UDP port。v1/v2c 使用 community。v3 展開 USM credentials，填寫使用者、context 與對應的密語，authNoPriv 使用 SHA-256，authPriv 加上 AES-128。Settings 可調整每次請求的 timeout 與 retry，或將目前連線存成具名 Profile。下次從 Settings 載入。資料庫保存連線設定，驗證資料由 macOS Keychain 或 Windows Credential Manager 保存。

GET 查詢一個 instance，scalar 預設使用 .0。要指定表格實例，可以直接在 Query OID 輸入完整 OID。Get next 取得下一項；Walk 與 Bulk walk 在子樹終點停止，後者適用 v2c 和 v3。查詢中可用 Cancel 中止。結果及最近 100 筆成功讀取的歷史摘要可選 CSV 或 JSON 匯出。

## SET 與讀回

先匯入定義可寫物件的 MIB。選 Set、目標物件、instance suffix 與新值，再執行預覽。整數使用十進位，列舉輸入數值，OCTET STRING 輸入 UTF-8，IP 使用 IPv4，OID 使用數字表示。程式檢查可寫性、型別與適用的範圍、長度、列舉或表格索引。

核對預覽的設備、OID、目前值及新值。Cancel 會撤銷預覽。Confirm write 會再次讀取原值，確認後寫入一次並讀回。預覽有效期為 60 秒。若顯示結果未知，請以 GET 查該完整 instance OID，再決定是否需寫入。還原設備值也需重新預覽與確認。

## 候選版變更與回退

本版整合離線 MIB 工作區、批次匯入與重載、v1/v2c/v3、分批及取消查詢、受確認的 SET、系統憑證儲存、歷史與匯出。第三方套件授權清單位於同目錄 THIRD-PARTY.txt。發行資格取決於實體設備及跨平台安裝驗收。

更換或回退版本前，先關閉應用程式並保留 MIB 原始檔、已匯出的紀錄、原安裝檔與應用程式資料。保留同一 OS 帳戶中的系統憑證。SQLite 使用 workspace.sqlite，首次開啟 Profile 或 History 時建立資料表。回退時安裝先前已驗證版本，保留目前資料以便再次升級。舊版本對新功能及資料的相容性仍需驗收。

內建 SMI 基礎定義取自鎖定的 mib-rs 0.10.0，該套件標示 MIT 或 Apache-2.0。定義為 RFC 衍生版本；發行審查應核對適用的 RFC 原始聲明與 IETF Trust 條款。用於驗收的 Apple、UCD 與 LM-SENSORS 樣本由測試環境提供。[mib-rs 套件](https://crates.io/crates/mib-rs/0.10.0) · [RFC 2578](https://www.rfc-editor.org/rfc/rfc2578) · [IETF Trust 條款](https://trustee.ietf.org/documents/trust-legal-provisions/)
