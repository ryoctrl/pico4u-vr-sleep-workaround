import i18n from 'i18next'
import { initReactI18next } from 'react-i18next'
import LanguageDetector from 'i18next-browser-languagedetector'

const ALLOW_PROMPT_EN =
  'Put on the headset and choose "Always allow from this computer" in the "Allow USB debugging?" dialog.'
const ALLOW_PROMPT_JA =
  'ヘッドセットを装着し、「USB デバッグを許可しますか」で「このコンピュータを常に許可」を選んでください'

const resources = {
  en: {
    translation: {
      title: 'pico4u VR Sleep Workaround',
      open_settings: 'Settings',
      wireless: {
        checking: { title: 'Checking…', hint: 'Looking for the headset.' },
        no_ip: {
          title: 'Not set up',
          hint: 'No headset IP yet. Connect via USB and enable Wireless Debug below.',
        },
        unreachable: {
          title: 'Headset not found',
          hint: 'Check power, Wi-Fi and that the PC is on the same network (a sleeping headset also looks like this). The IP may also have changed — enabling Wireless Debug over USB again picks up the new IP.',
        },
        refused: {
          title: 'Wireless Debug is OFF',
          hint: 'It turns off when the headset restarts. Connect via USB and enable it again below.',
        },
        connecting: { title: 'Connecting…', hint: 'Connecting to the headset over Wi-Fi.' },
        unauthorized: { title: 'Waiting for permission', hint: ALLOW_PROMPT_EN },
        offline: { title: 'Not responding', hint: 'Waiting for the headset to respond.' },
        ready: { title: 'Ready', hint: 'You can start sleep prevention.' },
      },
      adb_unavailable: {
        title: 'ADB is not available',
        hint: 'The bundled ADB server could not be started. Retrying automatically.',
      },
      keep_awake: {
        start: 'Start sleep prevention',
        stop: 'Stop',
        running: 'Running',
        stopped: 'Stopped',
        waiting: 'Running (waiting for connection: {{reason}})',
        last_wake: 'Last wake signal {{time}}',
        checked_at: 'Last checked {{time}}',
        last_response: 'Headset responded {{time}}',
      },
      setup: {
        header: 'Wireless Debug setup',
        steps_title: 'First time?',
        step1: 'Connect the headset to this PC with a USB cable and allow USB debugging.',
        step2: 'Press "Enable Wireless Debug".',
        step3: 'When it finishes, unplug the cable and start sleep prevention.',
        usb_label: 'USB',
        enable: 'Enable Wireless Debug',
        enabling: 'Enabling… do not unplug the USB cable',
        enable_needs_usb: 'Connect the headset via USB to enable.',
        already_enabled: 'Wireless Debug is already on and connected.',
        already_enabled_connecting:
          'Wireless Debug is already on. Waiting for the connection to complete.',
        done_unplug: 'Done. You can unplug the USB cable.',
        ip_label: 'Headset IP address',
        ip_placeholder: 'e.g. 192.168.1.10',
        ip_invalid: 'Enter an IPv4 address such as 192.168.1.10 (no port).',
        ip_save: 'Save',
      },
      usb: {
        none: 'Not connected',
        unauthorized: 'Waiting for permission',
        unauthorized_hint: ALLOW_PROMPT_EN,
        offline: 'Not responding',
        device: 'Connected: {{model}}',
        device_unknown: 'Connected',
        not_pico: 'This may not be a PICO 4 Ultra (A9210).',
        multiple: 'Several devices are connected. Unplug everything except the headset.',
      },
      logs: {
        header: 'Logs',
        tab_app: 'Activity',
        tab_setup: 'Setup',
        clear: 'Clear',
        empty: 'No logs yet.',
      },
      settings: {
        title: 'Settings',
        language: 'Language',
        theme: 'Theme',
        theme_light: 'Light',
        theme_dark: 'Dark',
        theme_system: 'System',
        dim: 'Auto-dim screen after (hours)',
        dim_note: '0 disables. Applies from the next start.',
        interval: 'Wake signal interval (seconds)',
        interval_note:
          'Lower values keep the screen from dimming; too high may reset the floor level. Applies from the next start.',
        running_note:
          'Sleep prevention is running — interval and dim changes apply from the next start.',
        debug: 'Debug mode (log every wake signal and connection check)',
        apply: 'Apply',
        cancel: 'Cancel',
      },
      error: {
        setup_already_running: 'Setup is already running.',
        usb_none: 'No USB device found. Connect the headset with a USB cable.',
        usb_multiple: 'Several USB devices are connected. Unplug everything except the headset.',
        usb_unauthorized: 'USB debugging is not allowed yet. ' + ALLOW_PROMPT_EN,
        usb_offline: 'The USB device is not responding. Reconnect the cable.',
        port_timeout:
          'Port did not open in time ({{detail}}). Check that the PC and headset are on the same network.',
        wireless_unauthorized: 'Connected, but waiting for permission. ' + ALLOW_PROMPT_EN,
        wireless_state: 'Connected, but the device state is "{{detail}}".',
        wireless_missing: 'The headset did not appear in the device list.',
        ip_not_found: 'Could not read the headset IP. Check that Wi-Fi is on.',
        not_ready: 'The headset is not ready.',
        already_enabled: 'Wireless Debug is already on.',
        keep_awake_already_running: 'Sleep prevention is already running.',
      },
      log: {
        adb_server_ok: 'ADB server is available.',
        adb_server_unavailable: 'Cannot reach the ADB server: {{error}}',
        wireless_state: 'Connection: {{state}}',
        monitor_observed: 'Check result: {{state}}',
        usb_state: 'USB: {{state}}',
        connect_result: 'adb connect: {{output}}',
        setup_started: 'Enabling Wireless Debug…',
        setup_usb_found: 'USB device found ({{serial}}).',
        setup_ip_found: 'Headset IP: {{ip}} (saved).',
        setup_tcpip: 'Switched to TCP mode: {{output}}',
        setup_port_open: 'Port is open ({{target}}).',
        setup_done:
          'Wireless Debug enabled and connected to {{target}}. You can unplug the USB cable.',
        setup_failed: 'Setup failed: {{error}}',
        ip_changed: 'Target changed to {{target}}.',
        keep_awake_started: 'Sleep prevention started (every {{interval}}s).',
        keep_awake_stopped: 'Sleep prevention stopped.',
        keep_awake_waiting:
          'Waiting for the headset ({{state}}). Sending resumes after reconnecting.',
        keep_awake_resumed: 'Reconnected. Sending resumed.',
        dim_done: 'Screen brightness set to the minimum.',
        dim_failed: 'Auto-dim failed: {{error}}',
        power_check_failed: 'Power state check failed: {{error}}',
        already_awake: 'Headset is awake, skipped.',
        wake_sent: 'Wake signal sent.',
        wake_failed: 'Wake signal failed: {{error}}',
        config_corrupted:
          'The settings file was unreadable and has been reset (backup: pico4u_config.json.bak).',
        config_sanitized: 'Invalid values in the settings file were corrected.',
      },
    },
  },
  ja: {
    translation: {
      title: 'pico4u V睡ツール',
      open_settings: '設定',
      wireless: {
        checking: { title: '確認中…', hint: 'ヘッドセットを探しています' },
        no_ip: {
          title: '未設定',
          hint: 'ヘッドセットの IP が未設定です。USB で接続し、下の「Wireless Debug を有効化」を押してください',
        },
        unreachable: {
          title: 'ヘッドセットが見つかりません',
          hint: '電源・Wi-Fi・PC と同じネットワークかを確認してください（スリープ中もこの表示になります）。IP が変わった可能性もあります。USB で有効化し直すと IP を取り直します',
        },
        refused: {
          title: 'Wireless Debug が OFF です',
          hint: 'ヘッドセットを再起動すると OFF に戻ります。USB で接続し、下で再度有効化してください',
        },
        connecting: { title: '接続中…', hint: 'Wi-Fi 経由でヘッドセットに接続しています' },
        unauthorized: { title: '許可待ち', hint: ALLOW_PROMPT_JA },
        offline: { title: '応答待ち', hint: 'ヘッドセットの応答を待っています' },
        ready: { title: '接続可能', hint: 'スリープ回避を開始できます' },
      },
      adb_unavailable: {
        title: 'ADB を起動できません',
        hint: '同梱の ADB サーバーを起動できませんでした。自動で再試行しています',
      },
      keep_awake: {
        start: 'スリープ回避 開始',
        stop: '停止',
        running: '動作中',
        stopped: '停止中',
        waiting: '動作中（接続待ち: {{reason}}）',
        last_wake: '最終送信 {{time}}',
        checked_at: '最終確認 {{time}}',
        last_response: '応答確認 {{time}}',
      },
      setup: {
        header: 'Wireless Debug の設定',
        steps_title: 'はじめて使うとき',
        step1: 'ヘッドセットを USB ケーブルで PC に接続し、USB デバッグを許可する',
        step2: '「Wireless Debug を有効化」を押す',
        step3: '完了したらケーブルを外し、スリープ回避を開始する',
        usb_label: 'USB',
        enable: 'Wireless Debug を有効化',
        enabling: '有効化中… USB を抜かないでください',
        enable_needs_usb: 'USB でヘッドセットを接続すると有効化できます',
        already_enabled: 'Wireless Debug は有効で、接続済みです',
        already_enabled_connecting: 'Wireless Debug は有効です。接続の完了を待っています',
        done_unplug: '完了しました。USB ケーブルを外してください',
        ip_label: 'ヘッドセットの IP アドレス',
        ip_placeholder: '例: 192.168.1.10',
        ip_invalid: '192.168.1.10 のような IPv4 アドレスを入力してください（ポートは不要）',
        ip_save: '保存',
      },
      usb: {
        none: '未接続',
        unauthorized: '許可待ち',
        unauthorized_hint: ALLOW_PROMPT_JA,
        offline: '応答なし',
        device: '接続済み: {{model}}',
        device_unknown: '接続済み',
        not_pico: 'PICO 4 Ultra（A9210）ではない可能性があります',
        multiple: '複数の端末が接続されています。ヘッドセット以外を外してください',
      },
      logs: {
        header: 'ログ',
        tab_app: '動作',
        tab_setup: 'セットアップ',
        clear: '消去',
        empty: 'ログはまだありません',
      },
      settings: {
        title: '設定',
        language: '言語',
        theme: 'テーマ',
        theme_light: 'ライト',
        theme_dark: 'ダーク',
        theme_system: 'システムに合わせる',
        dim: '自動画面暗転（時間後）',
        dim_note: '0 で無効。次回の開始時から有効',
        interval: '信号の送信間隔（秒）',
        interval_note:
          '小さいほど画面が暗転しにくくなりますが、大きすぎるとフロアレベルがリセットされる可能性があります。次回の開始時から有効',
        running_note:
          'スリープ回避の動作中です。送信間隔と暗転の変更は次回の開始時から有効になります',
        debug: 'デバッグモード（毎回の送信結果・接続確認の結果をログに出す）',
        apply: '適用',
        cancel: 'キャンセル',
      },
      error: {
        setup_already_running: 'セットアップは実行中です',
        usb_none: 'USB 端末が見つかりません。ヘッドセットを USB ケーブルで接続してください',
        usb_multiple: '複数の USB 端末が接続されています。ヘッドセット以外を外してください',
        usb_unauthorized: 'USB デバッグが許可されていません。' + ALLOW_PROMPT_JA,
        usb_offline: 'USB 端末が応答しません。ケーブルを挿し直してください',
        port_timeout:
          'ポートが開きませんでした（{{detail}}）。PC とヘッドセットが同じネットワークか確認してください',
        wireless_unauthorized: '接続しましたが許可待ちです。' + ALLOW_PROMPT_JA,
        wireless_state: '接続しましたが、端末の状態が「{{detail}}」です',
        wireless_missing: '端末一覧にヘッドセットが現れませんでした',
        ip_not_found: 'ヘッドセットの IP を取得できませんでした。Wi-Fi が ON か確認してください',
        not_ready: 'ヘッドセットに接続できていません',
        already_enabled: 'Wireless Debug は既に有効です',
        keep_awake_already_running: 'スリープ回避は既に動作中です',
      },
      log: {
        adb_server_ok: 'ADB サーバーに接続できました',
        adb_server_unavailable: 'ADB サーバーに接続できません: {{error}}',
        wireless_state: '接続状態: {{state}}',
        monitor_observed: '確認結果: {{state}}',
        usb_state: 'USB: {{state}}',
        connect_result: 'adb connect: {{output}}',
        setup_started: 'Wireless Debug を有効化しています…',
        setup_usb_found: 'USB 端末を検出しました（{{serial}}）',
        setup_ip_found: 'ヘッドセットの IP: {{ip}}（保存しました）',
        setup_tcpip: 'TCP モードに切り替えました: {{output}}',
        setup_port_open: 'ポートが開きました（{{target}}）',
        setup_done:
          'Wireless Debug を有効化し {{target}} に接続しました。USB ケーブルを外してください',
        setup_failed: 'セットアップに失敗しました: {{error}}',
        ip_changed: '接続先を {{target}} に変更しました',
        keep_awake_started: 'スリープ回避を開始しました（{{interval}} 秒間隔）',
        keep_awake_stopped: 'スリープ回避を停止しました',
        keep_awake_waiting:
          'ヘッドセットとの接続を待っています（{{state}}）。再接続後に送信を再開します',
        keep_awake_resumed: '再接続しました。送信を再開します',
        dim_done: '画面の明るさを最小にしました',
        dim_failed: '自動暗転に失敗しました: {{error}}',
        power_check_failed: '電源状態の確認に失敗しました: {{error}}',
        already_awake: '起動中のため送信を省略しました',
        wake_sent: 'スリープ回避信号を送信しました',
        wake_failed: 'スリープ回避信号の送信に失敗しました: {{error}}',
        config_corrupted:
          '設定ファイルを読めなかったため初期化しました（退避先: pico4u_config.json.bak）',
        config_sanitized: '設定ファイルの不正な値を補正しました',
      },
    },
  },
}

i18n
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    resources,
    fallbackLng: 'en',
    interpolation: {
      escapeValue: false,
    },
  })

export default i18n
