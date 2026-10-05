# 1 画面化と Wireless Debug 前提への再設計

## 1. 背景と目的

依頼者からの要望:

1. 設定 > デバッグモードを有効化しても、アプリ再起動で無効に戻る。
2. 一度接続しても、設定を開くためにトップ（モード選択）へ戻ると再接続が必要になる。
3. 実運用は「USB で ADB の Wireless Debug（`adb tcpip 5555`）を ON にする → 以後は保存した IP へ定期的に信号を送る」の組み合わせが前提。これに合わせて画面と状態管理を作り直す。

## 2. 現状の原因分析

| 症状 | 原因（該当箇所） |
|---|---|
| デバッグモードがリセットされる | `isDebug` はフロントの `useState(false)`（`frontend/src/hooks/useAppLogic.ts:24`）とバックエンドの `AtomicBool`（`src/state.rs:20`）にしか無く、設定ファイル `AppConfig`（`src/config.rs:8`）に項目が無い |
| トップに戻ると再接続が必要 | 右上の × が `connectionMode=null` にする（`MainView.tsx:124`）→ モード選択画面 → モード選択で `DeviceChecking`（USB で A9210 を検出するまで待つ）を必ず経由する。設定はモード選択画面からしか開けない |

上記以外に、コードを読んで見つけた不具合:

- `run_adb_device_command(serial=None)` は `host:transport-any` を使う（`src/adb_client.rs:69`）。USB と無線の両方で同じ端末が見えると「more than one device」で失敗する。`enable_tcpip`・`get_device_ip`・`get_device_model` が該当。
- 有線モードでも保存済み IP があると、スリープ防止ループが `-s <IP>:5555` を付けて送る（`src/commands.rs:168`）。有線で使うと失敗する。
- `connect_device` は ADB サーバーが `failed to connect to ...` を返しても成功扱いになる（`commands.rs:65`、`useAppLogic.ts:321`）。
- 自動接続の確認が `devices.contains("device")` という部分一致（`commands.rs:325`）。`offline`・`unauthorized` でも IP 文字列が含まれれば通る場合がある。
- `tcpip 5555` を実行してから IP を取りに行く順番のため、adbd 再起動待ちで 5 秒固定の待機と再試行が必要になっている（`useAppLogic.ts:294-315`）。
- スリープ防止中に接続が切れても再接続しない。エラーはデバッグモード時のみログに出る。
- `toggleKeepAwake` の依存配列に `keepAwakeInterval` が無く、ログの秒数が古い値になる（`useAppLogic.ts:369`）。

## 3. 設計方針

1. **接続状態はバックエンドが常時監視して持つ。** フロントは表示するだけにする。画面を切り替えても状態は消えない（症状 2 の根本対策）。
2. **「接続可能」は保存済み IP の `5555/TCP` に対する確認で判定する。** 有線モード・モード選択は廃止する。USB は「Wireless Debug を有効化するための手段」に限定する。
3. **画面は 1 枚。** 設定はその上に重ねるパネルで開き、閉じても下の状態は変わらない。
4. **設定はすべて設定ファイルに保存する**（デバッグモードを含む。症状 1 の対策）。
5. ADB 操作は**必ずシリアル番号を指定**する（USB 操作は USB のシリアル、無線操作は `IP:5555`）。

## 4. 接続状態の定義

バックエンドの監視タスクが一定間隔（3 秒）で次を行い、結果を `connection-status` イベントで通知する。

### 4.1 無線（Wireless Debug）の状態 `wireless.state`

| 値 | 判定 | 画面表示（日本語） |
|---|---|---|
| `no_ip` | IP 未保存 | IP 未設定。USB で Wireless Debug を有効化してください |
| `unreachable` | `IP:5555` への TCP 接続がタイムアウト／経路なし | ヘッドセットが見つかりません（電源・Wi-Fi・同一ネットワークを確認） |
| `refused` | TCP 接続が拒否された（ホストは居るがポートが閉じている） | Wireless Debug が OFF です（ヘッドセット再起動で OFF に戻ります）。USB で再有効化してください |
| `connecting` | TCP は通るが ADB 上で未接続、`host:connect` 実行中 | 接続中… |
| `unauthorized` / `offline` | ADB の状態がそれぞれの値 | ヘッドセットで許可してください／応答待ち |
| `ready` | ADB の状態が `device` | **接続可能** |

- TCP 確認は `tokio::net::TcpStream::connect` を 2 秒のタイムアウトで行い、接続できたら即切断する。
- TCP が通って ADB 上で未接続、または `offline` が 3 回連続したら `host:disconnect:<IP:5555>` → `host:connect:<IP:5555>` で張り直す。
- `host:connect` の応答文字列は `connected to` / `already connected` のみ成功とし、その後 `host:devices` を行単位で解析して `<IP:5555>\tdevice` を確認する。

### 4.2 USB の状態 `usb.state`

`host:devices` の行のうち、シリアルに `:` を含まないものを USB 端末とみなす。

| 値 | 判定 | 表示 |
|---|---|---|
| `none` | USB 端末なし | USB 未接続 |
| `unauthorized` | 状態が `unauthorized` | ヘッドセットで「USB デバッグを許可」を選んでください |
| `device` | 状態が `device`。初回のみ `getprop ro.product.model` を取得してシリアル単位でキャッシュ | 接続済み: `<model>`（A9210 以外なら「非対応機種の可能性」と注記） |

### 4.3 通知するデータ（`ConnectionStatus`）

```jsonc
{
  "wireless": { "state": "ready", "target": "192.168.1.10:5555", "checked_at": "12:34:56" },
  "usb": { "state": "device", "serial": "PA94...", "model": "A9210" },
  "keep_awake": { "running": true, "last_wake_at": "12:34:55", "last_error": null },
  "setup": { "running": false }
}
```

フロントは起動時に `get_status` で現在値を取り、以後はイベントで更新する。

## 5. 画面設計（400×700 固定）

```
┌──────────────────────────────┐
│ pico4u V睡ツール v0.2.0   [⚙]│ ← ヘッダー。⚙ で設定パネル
├──────────────────────────────┤
│ ● 接続可能                    │ ← 状態カード（4.1 の表示）
│   192.168.1.10:5555  12:34:56 │
│                               │
│   [ スリープ回避 開始 ]        │ ← ready 以外では押せない（理由を下に表示）
│   動作中 / 最終送信 12:34:55  │
├──────────────────────────────┤
│ ▼ Wireless Debug の設定        │ ← アコーディオン。ready 以外なら初期状態で開く
│   USB: ● 接続済み A9210        │   USB 状態をリアルタイム表示
│   [ Wireless Debug を有効化 ]  │   USB が device のときだけ押せる
│   IP: [192.168.1.10   ][保存]  │   手動入力（DHCP で IP が変わった場合用）
│   ┌ セットアップログ ───────┐ │
│   └──────────────────────┘ │
├──────────────────────────────┤
│ 動作ログ               [消去] │ ← アプリ全体のログ（常時表示）
│ ┌──────────────────────────┐ │
│ └──────────────────────────┘ │
└──────────────────────────────┘
```

- 設定パネル: 言語・テーマ・自動暗転（時間）・送信間隔（秒）・デバッグモード。「適用」で保存して閉じる。下の画面は表示したまま（アンマウントしない）。
- デバッグモードの意味を「動作ログに詳細（毎回の送信結果・監視結果）を出す」に変える。現状は「ログタブを表示する」だったが、ログは常時表示にするため。
- 「使い方」セクションはアコーディオン内の手順説明（1. USB で接続し許可 → 2. 有効化 → 3. USB を外して開始）に置き換える。

## 6. Wireless Debug 有効化の手順（バックエンド `enable_wireless_debug`）

1. `host:devices` から USB 端末を 1 台特定する（0 台／複数台ならエラー）。
2. その USB シリアルで `shell:ip -f inet addr show wlan0` を実行し IP を取得する（**tcpip より先に**取る）。
3. 同じシリアルで `tcpip:5555` を実行する。
4. 最大 10 秒、500ms 間隔で `IP:5555` への TCP 接続を試す。
5. `host:connect:IP:5555` → `host:devices` で `device` を確認する。
6. IP を設定ファイルに保存し、監視タスクに即時確認を依頼する。

各段階の結果は `log` イベント（`channel: "setup"`）で通知する。

## 7. スリープ防止ループ

- 対象は常に `IP:5555`（設定ファイルの IP）。
- 各周期で監視結果が `ready` でなければ送信をせず、「接続待ち」をログに出す（状態が変わったときだけ出す。毎周期は出さない）。監視タスク側が再接続を続ける。
- `ready` のときは現行どおり `dumpsys power` で `mWakefulness=Awake` を確認し、Awake でなければ `input keyevent 224` を送る。
- 開始は `ready` のときだけ許可する。動作中に切断されても停止はせず、復帰したら送信を再開する。
- 自動暗転は現行どおり（開始から N 時間後に `screen_brightness 1`）。

## 8. ログ

- バックエンドが `log` イベントを出す: `{ ts, channel: "app" | "setup", level: "info" | "warn" | "error" | "debug", message }`。
- バックエンドで直近 500 件をリングバッファに保持し、`get_logs` で取得できる（フロントが再描画しても消えない）。
- `debug` レベルはデバッグモードが ON のときだけ記録する。
- 画面では `app` を「動作ログ」、`setup` を「セットアップログ」に分けて表示する。

## 9. 設定ファイル（`pico4u_config.json`）

| 項目 | 型 | 既定値 | 変更 |
|---|---|---|---|
| `ip_address` | string | `""` | 既存 |
| `keep_awake_interval_secs` | u64 | 3 | 既存 |
| `dim_delay_hours` | f64 | 1.0 | 既存 |
| `debug_mode` | bool | false | **追加** |
| `last_connection_mode` | — | — | **廃止**（既存ファイルに残っていても無視される。serde は未知の項目を無視する） |

言語とテーマは現行どおり WebView の `localStorage` に保存する（再起動後も保持されることは既存挙動）。

## 10. バックエンドの API（Tauri コマンド）

| コマンド | 内容 |
|---|---|
| `get_status` | 現在の `ConnectionStatus` |
| `get_logs` | ログのリングバッファ |
| `clear_logs(channel)` | ログ消去 |
| `get_config` / `save_config_cmd(config)` | 設定の読み書き。保存時にデバッグモードと IP を状態へ即時反映し、監視タスクに再確認を依頼 |
| `enable_wireless_debug` | 6 章の手順 |
| `start_keep_awake` / `stop_keep_awake` | 7 章 |

廃止: `connect_device`・`enable_tcpip`・`set_usb_mode`・`disconnect_all_wireless`・`get_device_ip`・`get_device_model`・`try_auto_connect`・`check_connection`・`kill_adb`・`set_debug_mode`（`save_config_cmd` に統合）。

## 11. ファイル構成の変更

- Rust: `src/adb_client.rs`（シリアル必須化・`host:devices` 解析）、`src/monitor.rs`（新規: 監視タスク）、`src/logs.rs`（新規: リングバッファ＋イベント）、`src/commands.rs`・`src/state.rs`・`src/config.rs`・`src/main.rs` を書き換え。
- フロント: `MainView.tsx` を 1 画面に作り直し、`StatusCard.tsx`・`WirelessSetupPanel.tsx`・`LogView.tsx`・`SettingsPanel.tsx` に分ける。`ModeSelection.tsx`・`DeviceChecking.tsx`・`ConnectionPanel.tsx`・`HowToSection.tsx` は削除。`useAppLogic.ts` はイベント購読と設定保存だけの薄い層にする。i18n キーを整理（英日両方）。

## 12. 確認方法

- `cargo check` / `cargo clippy`、フロントの `tsc`・`oxlint`。
- `host:devices` の解析・`host:connect` 応答判定・`ip addr` 解析は純粋関数にして Rust の単体テストを付ける。
- `pnpm tauri dev` で起動し、ヘッドセット未接続時の表示（`no_ip` / `unreachable`）を確認する。実機（Pico 4 Ultra）を使った確認は依頼者に依頼する。

## 13. Fable 5.1 レビューの反映（2026-10-05）

UX・System の 2 観点でレビューを受け、以下を採用した。上の章と食い違う場合はこの章を優先する。

### 13.1 状態と判定

- `wireless.state` に `checking`（起動直後、監視の初回結果待ち）を追加。開始ボタンは無効。
- `usb.state` に `multiple`（USB 端末が複数）と `offline`（device / unauthorized 以外）を追加。`authorizing` は `unauthorized` と同じ扱い。
- `adb_server`（`ok` / `unavailable`）を追加。ADB サーバーに接続できない間は再試行間隔を 3→6→12→30 秒と延ばす。
- 判定の優先順位: IP 未保存 → ADB 上で `device` なら `ready`（TCP 確認は省略）→ TCP 確認（`refused` / `unreachable`）→ ADB 状態（`unauthorized` / `offline` / `connecting`）。
- TCP エラーの分類: `ConnectionRefused` → `refused`、それ以外（タイムアウト・`HostUnreachable`・`NetworkUnreachable` 等）→ `unreachable`。分類は純粋関数にしてテストする。
- 再接続: TCP が通り ADB 上のエントリが `offline` 等なら、回数を待たず `host:disconnect` → `host:connect`（`already connected` で張り直されない問題への対策）。TCP が通らないのに ADB 上にエントリが残っていれば `host:disconnect` して表示を一致させる。`unauthorized` のときは利用者の許可待ちなので張り直さない。
- `ready` から外す判定は 2 回連続の失敗で行う（Wi-Fi 瞬断で表示が揺れないように）。復帰は 1 回で行う。状態遷移は純粋関数 `next_state` にしてテストする。
- `unreachable` が 5 回続いたら監視間隔を 10 秒に延ばす。設定保存・セットアップ完了で即時確認し、間隔を戻す。

### 13.2 並行処理

- 状態は `Mutex<ConnectionStatus>`、即時確認の依頼は `tokio::sync::Notify`。状態を変えたら `connection-status` イベントを出す。
- セットアップ実行中（`setup_running`）は、監視タスクは観測のみ行い `connect` / `disconnect` をしない。`setup_running` は成功・失敗どちらでも必ず戻す。
- 周期処理は `interval` ではなく各周期の末尾で `sleep` する（遅延後に要求が集中しないように）。
- スリープ防止ループは毎周期、最新の接続状態と対象を読む。送信間隔・自動暗転は「次回の開始時から有効」とし、設定パネルにその旨を表示する。
- 自動暗転は 1 本のループに統合し、時刻到達後に最初に `ready` だった周期で 1 回実行して結果をログに出す。

### 13.3 設定とログ

- 起動時（`setup`）に設定を読み、デバッグモード・IP を共有状態へ入れてから監視タスクを起動する。
- 読み込み時に値を補正する（送信間隔は 1 以上、自動暗転は 0 以上の有限値）。保存時も同じ検証を行う。
- 保存は一時ファイルに書いて置き換える。解析に失敗したファイルは `.bak` に退避し、警告をログに出す。
- IP が変わったら旧 `IP:5555` に `host:disconnect` を送る。
- セットアップ手順では IP を取得した時点で保存する（以降の失敗は監視の表示に任せる）。`wlan0` で取れない場合は `ip route get 1.1.1.1` の `src` を使う。
- ログには連番 `id` を付ける。フロントは先にイベント購読を始めてから `get_logs` を呼び、`id` で重複を除く。
- ログ本文は翻訳キーと引数で送り、フロントで日英に翻訳する。
- ADB の `FAIL` 応答は理由の本文まで読み、エラー文字列に含める。プロトコルの読み書きはメモリ上の擬似ストリームでテストする。

### 13.4 画面

- 固定領域: ヘッダー＋状態カード。可変領域（1 つのスクロール領域）: アコーディオン＋ログ枠。
- ログ枠は 1 つにまとめ、「動作」「セットアップ」のタブで切り替える。セットアップ実行中は「セットアップ」タブを自動選択する。
- アコーディオンの開閉: 起動時に `ready` 以外なら開く。`ready` から `no_ip` / `refused` / `unreachable` に変わったら開く。自動では閉じない。
- 状態カード: 動作中に切断された場合は「動作中（接続待ち: <理由>）」と 2 段で表示する。停止ボタンは常に押せる。開始できないときはボタンの下に理由（4.1 の文言）を出す。
- `unreachable` の文言に「IP が変わった可能性。USB で有効化し直すと IP を取り直す」を加える。USB が `device` で無線が `ready` 以外なら、有効化ボタンを強調表示する。
- `unauthorized` の文言は USB・無線とも「ヘッドセットを装着し『USB デバッグを許可』で『常に許可』を選ぶ」に揃える。
- セットアップ中は有効化ボタンを無効にし「有効化中… USB を抜かないでください」と表示する。完了したら「USB ケーブルを外してください」を出す。
- 手動 IP 入力は IPv4（ドット区切りの 4 組）だけ受け付け、不正なら保存ボタンを無効にする。
- 使い方の手順は `no_ip` のときだけアコーディオン先頭に表示する。接続図（`connect_hmd_to_pc.png`）は削除する。
- デバッグモードの説明文を「毎回の送信結果・監視結果をログに出す」に変える。
- 未使用の依存 `adb_client` crate を `Cargo.toml` から削除する。

### 13.5 採用しなかった指摘

- `ANDROID_ADB_SERVER_PORT` 環境変数への対応: 今回の要件外のため見送り。
- `connecting` が 10 秒続いたときの補足表示: 監視の 1 周が `host:connect` のタイムアウト（5 秒）で抑えられるため見送り。
