#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""i18n_shell.py — BraveOS shell translations (BraveWM menus/bar, dock, lock).

Single source of truth; scripts/gen_i18n.py emits /usr/share/bravos/i18n/
<app>/<lang>.conf from these tables. English is the compiled-in fallback in
each component; these are the overrides.

Languages: ar zh fr de es pt ru ja ko hi tr id (regional variants fall back
to the base language; any further language can be added as a new dict).
"""

# keys used by BraveWM (i18n.cpp) and the BraveOS bar
WM = {
    "menu_browser": {
        "ar": "متصفح Brave", "zh": "Brave 浏览器", "fr": "Navigateur Brave",
        "de": "Brave-Browser", "es": "Navegador Brave", "pt": "Navegador Brave",
        "ru": "Браузер Brave", "ja": "Braveブラウザ", "ko": "Brave 브라우저",
        "hi": "Brave ब्राउज़र", "tr": "Brave Tarayıcı", "id": "Peramban Brave",
    },
    "menu_files": {
        "ar": "الملفات", "zh": "文件", "fr": "Fichiers", "de": "Dateien",
        "es": "Archivos", "pt": "Arquivos", "ru": "Файлы", "ja": "ファイル",
        "ko": "파일", "hi": "फ़ाइलें", "tr": "Dosyalar", "id": "Berkas",
    },
    "menu_terminal": {
        "ar": "الطرفية", "zh": "终端", "fr": "Terminal", "de": "Terminal",
        "es": "Terminal", "pt": "Terminal", "ru": "Терминал",
        "ja": "ターミナル", "ko": "터미널", "hi": "टर्मिनल", "tr": "Uçbirim",
        "id": "Terminal",
    },
    "menu_settings": {
        "ar": "الإعدادات", "zh": "设置", "fr": "Paramètres",
        "de": "Einstellungen", "es": "Ajustes", "pt": "Configurações",
        "ru": "Параметры", "ja": "設定", "ko": "설정", "hi": "सेटिंग्स",
        "tr": "Ayarlar", "id": "Pengaturan",
    },
    "menu_appgrid": {
        "ar": "شبكة التطبيقات", "zh": "应用网格", "fr": "Grille d'applications",
        "de": "App-Raster", "es": "Cuadrícula de apps",
        "pt": "Grade de apps", "ru": "Сетка приложений", "ja": "アプリグリッド",
        "ko": "앱 그리드", "hi": "ऐप ग्रिड", "tr": "Uygulama ızgarası",
        "id": "Kisi aplikasi",
    },
    "menu_shortcuts": {
        "ar": "اختصارات لوحة المفاتيح", "zh": "键盘快捷键",
        "fr": "Raccourcis clavier", "de": "Tastaturkürzel",
        "es": "Atajos de teclado", "pt": "Atalhos de teclado",
        "ru": "Горячие клавиши", "ja": "キーボードショートカット",
        "ko": "키보드 단축키", "hi": "कीबोर्ड शॉर्टकट",
        "tr": "Klavye kısayolları", "id": "Pintasan papan ketik",
    },
    "menu_lock": {
        "ar": "قفل الشاشة", "zh": "锁定屏幕", "fr": "Verrouiller l'écran",
        "de": "Bildschirm sperren", "es": "Bloquear pantalla",
        "pt": "Bloquear tela", "ru": "Заблокировать экран",
        "ja": "画面をロック", "ko": "화면 잠금", "hi": "स्क्रीन लॉक",
        "tr": "Ekranı kilitle", "id": "Kunci layar",
    },
    "menu_restart_wm": {
        "ar": "إعادة تشغيل BraveWM", "zh": "重启 BraveWM",
        "fr": "Redémarrer BraveWM", "de": "BraveWM neu starten",
        "es": "Reiniciar BraveWM", "pt": "Reiniciar BraveWM",
        "ru": "Перезапустить BraveWM", "ja": "BraveWMを再起動",
        "ko": "BraveWM 재시작", "hi": "BraveWM पुनः आरंभ",
        "tr": "BraveWM'yi yeniden başlat", "id": "Mulai ulang BraveWM",
    },
    "menu_maximize": {
        "ar": "تكبير", "zh": "最大化", "fr": "Agrandir", "de": "Maximieren",
        "es": "Maximizar", "pt": "Maximizar", "ru": "Развернуть",
        "ja": "最大化", "ko": "최대화", "hi": "अधिकतम", "tr": "Büyüt",
        "id": "Maksimalkan",
    },
    "menu_unmaximize": {
        "ar": "إلغاء التكبير", "zh": "取消最大化", "fr": "Restaurer",
        "de": "Wiederherstellen", "es": "Desmaximizar", "pt": "Restaurar",
        "ru": "Восстановить", "ja": "元に戻す", "ko": "최대화 해제",
        "hi": "अधिकतम रद्द", "tr": "Geri yükle", "id": "Pulihkan",
    },
    "menu_minimize": {
        "ar": "تصغير", "zh": "最小化", "fr": "Réduire", "de": "Minimieren",
        "es": "Minimizar", "pt": "Minimizar", "ru": "Свернуть",
        "ja": "最小化", "ko": "최소화", "hi": "न्यूनतम", "tr": "Küçült",
        "id": "Minimalkan",
    },
    "menu_fullscreen": {
        "ar": "ملء الشاشة", "zh": "全屏", "fr": "Plein écran",
        "de": "Vollbild", "es": "Pantalla completa", "pt": "Tela cheia",
        "ru": "Полный экран", "ja": "全画面表示", "ko": "전체 화면",
        "hi": "पूर्ण स्क्रीन", "tr": "Tam ekran", "id": "Layar penuh",
    },
    "menu_exit_fullscreen": {
        "ar": "الخروج من ملء الشاشة", "zh": "退出全屏",
        "fr": "Quitter le plein écran", "de": "Vollbild beenden",
        "es": "Salir de pantalla completa", "pt": "Sair da tela cheia",
        "ru": "Выйти из полного экрана", "ja": "全画面を終了",
        "ko": "전체 화면 종료", "hi": "पूर्ण स्क्रीन से बाहर",
        "tr": "Tam ekrandan çık", "id": "Keluar layar penuh",
    },
    "menu_always_top": {
        "ar": "دائماً في الأعلى", "zh": "总在最前",
        "fr": "Toujours au premier plan", "de": "Immer im Vordergrund",
        "es": "Siempre encima", "pt": "Sempre no topo",
        "ru": "Поверх всех окон", "ja": "常に最前面",
        "ko": "항상 위", "hi": "हमेशा ऊपर", "tr": "Her zaman üstte",
        "id": "Selalu di atas",
    },
    "menu_unpin": {
        "ar": "إلغاء التثبيت في الأعلى", "zh": "取消置顶",
        "fr": "Annuler (premier plan)", "de": "Nicht mehr im Vordergrund",
        "es": "Quitar «siempre encima»", "pt": "Desativar «sempre no topo»",
        "ru": "Открепить (не поверх всех)",
        "ja": "最前面表示を解除", "ko": "항상 위 해제",
        "hi": "हमेशा ऊपर रद्द करें", "tr": "Her zaman üstteyi kapat",
        "id": "Batal pin di atas",
    },
    "menu_move_ws": {
        "ar": "انقل إلى مساحة العمل %d", "zh": "移动到工作区 %d",
        "fr": "Déplacer vers le bureau %d", "de": "Zu Arbeitsfläche %d verschieben",
        "es": "Mover al espacio %d", "pt": "Mover para a área %d",
        "ru": "Перенести на рабочий стол %d", "ja": "ワークスペース %d へ移動",
        "ko": "작업 공간 %d(으)로 이동", "hi": "वर्कस्पेस %d में ले जाएँ",
        "tr": "%d. çalışma alanına taşı", "id": "Pindah ke ruang kerja %d",
    },
    "menu_close": {
        "ar": "إغلاق", "zh": "关闭", "fr": "Fermer", "de": "Schließen",
        "es": "Cerrar", "pt": "Fechar", "ru": "Закрыть", "ja": "閉じる",
        "ko": "닫기", "hi": "बंद करें", "tr": "Kapat", "id": "Tutup",
    },
    "power_lock": None,  # alias of menu_lock (written by generator)
    "power_logout": {
        "ar": "تسجيل الخروج", "zh": "注销", "fr": "Se déconnecter",
        "de": "Abmelden", "es": "Cerrar sesión", "pt": "Sair",
        "ru": "Завершить сеанс", "ja": "ログアウト", "ko": "로그아웃",
        "hi": "लॉग आउट", "tr": "Oturumu kapat", "id": "Keluar",
    },
    "power_suspend": {
        "ar": "تعليق", "zh": "挂起", "fr": "Mettre en veille",
        "de": "Ruhezustand", "es": "Suspender", "pt": "Suspender",
        "ru": "Ждущий режим", "ja": "サスペンド", "ko": "절전 모드",
        "hi": "निलंबित", "tr": "Askıya al", "id": "Tangguhkan",
    },
    "power_restart": {
        "ar": "إعادة التشغيل", "zh": "重启", "fr": "Redémarrer",
        "de": "Neustart", "es": "Reiniciar", "pt": "Reiniciar",
        "ru": "Перезагрузить", "ja": "再起動", "ko": "재시작",
        "hi": "पुनः आरंभ", "tr": "Yeniden başlat", "id": "Mulai ulang",
    },
    "power_shutdown": {
        "ar": "إيقاف التشغيل", "zh": "关机", "fr": "Éteindre",
        "de": "Herunterfahren", "es": "Apagar", "pt": "Desligar",
        "ru": "Выключить", "ja": "シャットダウン", "ko": "종료",
        "hi": "बंद करें", "tr": "Bilgisayarı kapat", "id": "Matikan",
    },
    "am": {
        "ar": "ص", "zh": "上午", "fr": "AM", "de": "AM", "es": "a.m.",
        "pt": "AM", "ru": "AM", "ja": "午前", "ko": "오전", "hi": "AM",
        "tr": "ÖÖ", "id": "AM",
    },
    "pm": {
        "ar": "م", "zh": "下午", "fr": "PM", "de": "PM", "es": "p.m.",
        "pt": "PM", "ru": "PM", "ja": "午後", "ko": "오후", "hi": "PM",
        "tr": "ÖS", "id": "PM",
    },
}

# languages that prefer a 24-hour clock on the bar (spec default: 12h)
TIME_24H = {"ru", "zh", "ja", "ko", "de", "id", "tr", "hi"}

# keys used by bravos-dock (tooltips + popups). Many reuse WM translations.
DOCK = {
    "tip_files": None,          # -> WM["menu_files"]
    "tip_settings": None,       # -> WM["menu_settings"]
    "tip_brave": None,          # -> WM["menu_browser"]
    "tip_terminal": None,       # -> WM["menu_terminal"]
    "tip_appgrid": None,        # -> WM["menu_appgrid"]
    "tip_bluetooth": {
        "ar": "بلوتوث", "zh": "蓝牙", "fr": "Bluetooth", "de": "Bluetooth",
        "es": "Bluetooth", "pt": "Bluetooth", "ru": "Bluetooth",
        "ja": "Bluetooth", "ko": "블루투스", "hi": "ब्लूटूथ",
        "tr": "Bluetooth", "id": "Bluetooth",
    },
    "tip_nightlight": {
        "ar": "الإضاءة الليلية", "zh": "夜间模式",
        "fr": "Lumière nocturne", "de": "Nachtlicht",
        "es": "Luz nocturna", "pt": "Luz noturna",
        "ru": "Ночной свет", "ja": "夜間モード", "ko": "야간 모드",
        "hi": "नाइट लाइट", "tr": "Gece ışığı", "id": "Cahaya malam",
    },
    "tip_wifi": {
        "ar": "Wi-Fi", "zh": "Wi-Fi", "fr": "Wi-Fi", "de": "WLAN",
        "es": "Wi-Fi", "pt": "Wi-Fi", "ru": "Wi-Fi", "ja": "Wi-Fi",
        "ko": "Wi-Fi", "hi": "Wi-Fi", "tr": "Wi-Fi", "id": "Wi-Fi",
    },
    "tip_power": {
        "ar": "الطاقة", "zh": "电源", "fr": "Alimentation",
        "de": "Energie", "es": "Energía", "pt": "Energia",
        "ru": "Питание", "ja": "電源", "ko": "전원", "hi": "पावर",
        "tr": "Güç", "id": "Daya",
    },
    "tip_lock": None,           # -> WM["menu_lock"]
    "power_lock": None, "power_logout": None, "power_suspend": None,
    "power_restart": None, "power_shutdown": None,   # -> WM
    "app_brave": None,          # -> WM["menu_browser"]
    "app_install": {
        "ar": "تثبيت BraveOS", "zh": "安装 BraveOS", "fr": "Installer BraveOS",
        "de": "BraveOS installieren", "es": "Instalar BraveOS",
        "pt": "Instalar o BraveOS", "ru": "Установить BraveOS",
        "ja": "BraveOSをインストール", "ko": "BraveOS 설치",
        "hi": "BraveOS इंस्टॉल करें", "tr": "BraveOS'ı kur",
        "id": "Instal BraveOS",
    },
    "app_welcome": {
        "ar": "مرحباً بك", "zh": "欢迎", "fr": "Bienvenue",
        "de": "Willkommen", "es": "Bienvenida", "pt": "Bem-vindo",
        "ru": "Добро пожаловать", "ja": "ようこそ", "ko": "환영합니다",
        "hi": "स्वागत है", "tr": "Hoş geldiniz", "id": "Selamat datang",
    },
    "app_tools": {
        "ar": "أدوات النظام", "zh": "系统工具", "fr": "Outils système",
        "de": "Systemwerkzeuge", "es": "Herramientas del sistema",
        "pt": "Ferramentas do sistema",
        "ru": "Системные инструменты", "ja": "システムツール",
        "ko": "시스템 도구", "hi": "सिस्टम टूल्स", "tr": "Sistem araçları",
        "id": "Alat sistem",
    },
    "app_settings": None, "app_terminal": None, "app_files": None,
    "app_shortcuts": None,     # -> WM equivalents
}

# keys used by bravos-lock
LOCK = {
    "lock_password": {
        "ar": "كلمة المرور", "zh": "密码", "fr": "Mot de passe",
        "de": "Passwort", "es": "Contraseña", "pt": "Senha",
        "ru": "Пароль", "ja": "パスワード", "ko": "비밀번호",
        "hi": "पासवर्ड", "tr": "Şifre", "id": "Kata sandi",
    },
    "lock_wrong": {
        "ar": "كلمة المرور غير صحيحة", "zh": "密码错误",
        "fr": "Mot de passe incorrect", "de": "Falsches Passwort",
        "es": "Contraseña incorrecta", "pt": "Senha incorreta",
        "ru": "Неверный пароль", "ja": "パスワードが違います",
        "ko": "비밀번호가 틀렸습니다", "hi": "गलत पासवर्ड",
        "tr": "Yanlış şifre", "id": "Kata sandi salah",
    },
}

SHELL_ALIASES = {
    # dock/lock -> wm source key
    ("dock", "tip_files"): ("wm", "menu_files"),
    ("dock", "tip_settings"): ("wm", "menu_settings"),
    ("dock", "tip_brave"): ("wm", "menu_browser"),
    ("dock", "tip_terminal"): ("wm", "menu_terminal"),
    ("dock", "tip_appgrid"): ("wm", "menu_appgrid"),
    ("dock", "tip_lock"): ("wm", "menu_lock"),
    ("dock", "power_lock"): ("wm", "menu_lock"),
    ("dock", "power_logout"): ("wm", "power_logout"),
    ("dock", "power_suspend"): ("wm", "power_suspend"),
    ("dock", "power_restart"): ("wm", "power_restart"),
    ("dock", "power_shutdown"): ("wm", "power_shutdown"),
    ("dock", "app_brave"): ("wm", "menu_browser"),
    ("dock", "app_settings"): ("wm", "menu_settings"),
    ("dock", "app_terminal"): ("wm", "menu_terminal"),
    ("dock", "app_files"): ("wm", "menu_files"),
    ("dock", "app_shortcuts"): ("wm", "menu_shortcuts"),
    ("wm", "power_lock"): ("wm", "menu_lock"),
}
