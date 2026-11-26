#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""i18n_install.py — translations for the BraveOS installer and welcome app.
See i18n_shell.py for the format."""

INSTALLER = {
    "inst_title": {
        "ar": "تثبيت BraveOS", "zh": "安装 BraveOS", "fr": "Installer BraveOS",
        "de": "BraveOS installieren", "es": "Instalar BraveOS",
        "pt": "Instalar o BraveOS", "ru": "Установка BraveOS",
        "ja": "BraveOSをインストール", "ko": "BraveOS 설치",
        "hi": "BraveOS इंस्टॉल करें", "tr": "BraveOS kur",
        "id": "Pasang BraveOS",
    },
    "inst_welcome_t": {
        "ar": "مرحباً بك في BraveOS", "zh": "欢迎使用 BraveOS",
        "fr": "Bienvenue dans BraveOS", "de": "Willkommen bei BraveOS",
        "es": "Bienvenido a BraveOS", "pt": "Bem-vindo ao BraveOS",
        "ru": "Добро пожаловать в BraveOS", "ja": "BraveOSへようこそ",
        "ko": "BraveOS에 오신 것을 환영합니다",
        "hi": "BraveOS में आपका स्वागत है", "tr": "BraveOS'a hoş geldiniz",
        "id": "Selamat datang di BraveOS",
    },
    "inst_welcome_s": {
        "ar": "ثبّت توزيعة متصفح Brave على هذا الحاسوب.",
        "zh": "将 Brave 浏览器发行版安装到这台电脑。",
        "fr": "Installez la distribution du navigateur Brave sur cet ordinateur.",
        "de": "Die Brave-Browser-Distribution auf diesem Rechner installieren.",
        "es": "Instalar la distribución del navegador Brave en este equipo.",
        "pt": "Instalar a distribuição do navegador Brave neste computador.",
        "ru": "Установить дистрибутив браузера Brave на этот компьютер.",
        "ja": "このコンピューターにBraveブラウザのディストリビューションをインストールします。",
        "ko": "이 컴퓨터에 Brave 브라우저 배포판을 설치합니다.",
        "hi": "Brave ब्राउज़र वितरण को इस कंप्यूटर पर इंस्टॉल करें।",
        "tr": "Brave tarayıcı dağıtımını bu bilgisayara kurun.",
        "id": "Pasang distribusi peramban Brave di komputer ini.",
    },
    "inst_body": {
        "ar": "سيقوم هذا المثبّت <b>بمسح قرص واحد</b> وتثبيت BraveOS مع:\n"
              "  • متصفح Brave، الملفات، إعدادات BraveOS\n"
              "  • نواة لينكس مع تعريفات لعتاد 2008–2022\n"
              "  • محمّل الإقلاع GRUB (BIOS و UEFI)\n\n"
              "كل شيء يعمل دون اتصال بالإنترنت من هذا القرص؛ لا حاجة للشبكة.",
        "zh": "此安装器将<b>擦除一块磁盘</b>并安装 BraveOS，包含：\n"
              "  • Brave 浏览器、文件、BraveOS 设置\n"
              "  • 支持 2008–2022 年硬件驱动的 Linux 内核\n"
              "  • GRUB 引导加载器（BIOS 与 UEFI）\n\n"
              "全部离线运行，无需网络。",
        "fr": "Cet installateur <b>efface un disque</b> et installe BraveOS avec :\n"
              "  • navigateur Brave, fichiers, Paramètres BraveOS\n"
              "  • noyau Linux avec pilotes pour le matériel 2008–2022\n"
              "  • chargeur d'amorçage GRUB (BIOS et UEFI)\n\n"
              "Tout fonctionne hors ligne depuis cette clé ; le réseau n'est pas nécessaire.",
        "de": "Dieses Installationsprogramm <b>löscht einen Datenträger</b> und "
              "installiert BraveOS mit:\n"
              "  • Brave-Browser, Dateien, BraveOS-Einstellungen\n"
              "  • Linux-Kernel mit Treibern für Hardware 2008–2022\n"
              "  • GRUB-Bootloader (BIOS und UEFI)\n\n"
              "Alles läuft offline von diesem USB-Stick; kein Netzwerk nötig.",
        "es": "Este instalador <b>borra un disco</b> e instala BraveOS con:\n"
              "  • navegador Brave, Archivos, Ajustes de BraveOS\n"
              "  • kernel Linux con controladores para hardware de 2008–2022\n"
              "  • cargador de arranque GRUB (BIOS y UEFI)\n\n"
              "Todo funciona sin conexión desde este USB; no hace falta red.",
        "pt": "Este instalador <b>apaga um disco</b> e instala o BraveOS com:\n"
              "  • navegador Brave, Arquivos, Configurações do BraveOS\n"
              "  • kernel Linux com drivers para hardware de 2008–2022\n"
              "  • carregador de arranque GRUB (BIOS e UEFI)\n\n"
              "Tudo funciona offline deste USB; a rede não é necessária.",
        "ru": "Этот установщик <b>стирует один диск</b> и ставит BraveOS с:\n"
              "  • браузером Brave, Файлами, Параметрами BraveOS\n"
              "  • ядром Linux с драйверами для железа 2008–2022\n"
              "  • загрузчиком GRUB (BIOS и UEFI)\n\n"
              "Всё работает офлайн с этого USB; сеть не нужна.",
        "ja": "このインストーラーは<b>1つのディスクを消去</b>し、以下を含むBraveOSをインストールします：\n"
              "  • Braveブラウザ、ファイル、BraveOS 設定\n"
              "  • 2008–2022年のハードウェア用ドライバー入りLinuxカーネル\n"
              "  • GRUBブートローダー（BIOS・UEFI）\n\n"
              "すべてこのUSBからオフラインで動作します。ネットワークは不要です。",
        "ko": "이 설치 관리자는 <b>디스크 하나를 지우고</b> 다음을 포함한 BraveOS를 설치합니다:\n"
              "  • Brave 브라우저, 파일, BraveOS 설정\n"
              "  • 2008–2022년 하드웨어용 드라이버가 포함된 Linux 커널\n"
              "  • GRUB 부트로더(BIOS 및 UEFI)\n\n"
              "모두 이 USB에서 오프라인으로 실행됩니다. 네트워크가 필요 없습니다.",
        "hi": "यह इंस्टॉलर <b>एक डिस्क मिटाकर</b> BraveOS इंस्टॉल करेगा, जिसमें शामिल हैं:\n"
              "  • Brave ब्राउज़र, फ़ाइलें, BraveOS सेटिंग्स\n"
              "  • 2008–2022 हार्डवेयर के ड्राइवरों वाला Linux कर्नेल\n"
              "  • GRUB बूटलोडर (BIOS और UEFI)\n\n"
              "सब कुछ इस USB से ऑफ़लाइन चलता है; नेटवर्क ज़रूरी नहीं।",
        "tr": "Bu kurulum programı <b>bir diski siler</b> ve şunlarla birlikte "
              "BraveOS kurar:\n"
              "  • Brave tarayıcı, Dosyalar, BraveOS Ayarları\n"
              "  • 2008–2022 donanımı için sürücülü Linux çekirdeği\n"
              "  • GRUB önyükleyici (BIOS ve UEFI)\n\n"
              "Her şey bu USB'den çevrimdışı çalışır; ağ gerekmez.",
        "id": "Pemasang ini <b>menghapus satu disk</b> dan memasang BraveOS dengan:\n"
              "  • peramban Brave, Berkas, Pengaturan BraveOS\n"
              "  • kernel Linux dengan penggerak untuk perangkat keras 2008–2022\n"
              "  • pemuat boot GRUB (BIOS dan UEFI)\n\n"
              "Semuanya berjalan luring dari USB ini; jaringan tidak diperlukan.",
    },
    "inst_quit": {
        "ar": "إنهاء", "zh": "退出", "fr": "Quitter", "de": "Beenden",
        "es": "Salir", "pt": "Sair", "ru": "Выйти", "ja": "終了",
        "ko": "끝내기", "hi": "बाहर निकलें", "tr": "Çık", "id": "Keluar",
    },
    "inst_continue": {
        "ar": "متابعة", "zh": "继续", "fr": "Continuer", "de": "Weiter",
        "es": "Continuar", "pt": "Continuar", "ru": "Продолжить",
        "ja": "続行", "ko": "계속", "hi": "जारी रखें", "tr": "Devam",
        "id": "Lanjut",
    },
    "inst_back": {
        "ar": "رجوع", "zh": "返回", "fr": "Retour", "de": "Zurück",
        "es": "Atrás", "pt": "Voltar", "ru": "Назад", "ja": "戻る",
        "ko": "뒤로", "hi": "वापस", "tr": "Geri", "id": "Kembali",
    },
    "inst_disk_t": {
        "ar": "اختر القرص الهدف", "zh": "选择目标磁盘",
        "fr": "Choisir le disque cible", "de": "Zieldatenträger wählen",
        "es": "Elegir el disco de destino",
        "pt": "Escolher o disco de destino",
        "ru": "Выберите целевой диск",
        "ja": "インストール先ディスクを選択", "ko": "대상 디스크 선택",
        "hi": "लक्ष्य डिस्क चुनें", "tr": "Hedef diski seçin",
        "id": "Pilih disk target",
    },
    "inst_disk_s": {
        "ar": "ستُدمّر جميع البيانات على القرص المحدد.",
        "zh": "所选磁盘上的所有数据都将被销毁。",
        "fr": "TOUTES LES DONNÉES du disque sélectionné seront détruites.",
        "de": "ALLE DATEN auf dem gewählten Datenträger werden gelöscht.",
        "es": "Se destruirán TODOS los datos del disco seleccionado.",
        "pt": "TODOS os dados do disco selecionado serão destruídos.",
        "ru": "Все данные на выбранном диске будут уничтожены.",
        "ja": "選択したディスクの全データは破壊されます。",
        "ko": "선택한 디스크의 모든 데이터가 삭제됩니다.",
        "hi": "चुनी गई डिस्क का सारा डेटा मिटा दिया जाएगा।",
        "tr": "Seçilen diskteki TÜM VERİLER yok edilecek.",
        "id": "SEMUA DATA pada disk yang dipilih akan dihapus.",
    },
    "inst_disk_choose": {
        "ar": "القرص المراد مسحه وتثبيت BraveOS عليه:",
        "zh": "要擦除并安装 BraveOS 的磁盘：",
        "fr": "Disque à effacer pour installer BraveOS :",
        "de": "Zu löschender Datenträger für die BraveOS-Installation:",
        "es": "Disco a borrar e instalar BraveOS:",
        "pt": "Disco a apagar e instalar o BraveOS:",
        "ru": "Диск, который стереть и поставить BraveOS:",
        "ja": "消去してBraveOSをインストールするディスク：",
        "ko": "지우고 BraveOS를 설치할 디스크:",
        "hi": "मिटाकर BraveOS इंस्टॉल करने के लिए डिस्क:",
        "tr": "Silinip BraveOS'un kurulacağı disk:",
        "id": "Disk yang akan dihapus dan dipasangi BraveOS:",
    },
    "inst_no_disks": {
        "ar": "لم يتم العثور على أقراص.",
        "zh": "未找到磁盘。",
        "fr": "Aucun disque trouvé.",
        "de": "Keine Datenträger gefunden.",
        "es": "No se encontraron discos.",
        "pt": "Nenhum disco encontrado.",
        "ru": "Диски не найдены.",
        "ja": "ディスクが見つかりません。",
        "ko": "디스크를 찾을 수 없습니다.",
        "hi": "कोई डिस्क नहीं मिली।",
        "tr": "Disk bulunamadı.",
        "id": "Tidak ada disk yang ditemukan.",
    },
    "inst_options_t": {
        "ar": "خيارات النظام", "zh": "系统选项",
        "fr": "Options du système", "de": "Systemoptionen",
        "es": "Opciones del sistema", "pt": "Opções do sistema",
        "ru": "Параметры системы", "ja": "システムオプション",
        "ko": "시스템 옵션", "hi": "सिस्टम विकल्प",
        "tr": "Sistem seçenekleri", "id": "Opsi sistem",
    },
    "inst_options_s": {
        "ar": "اسم المضيف وكلمة المرور",
        "zh": "主机名和您的密码",
        "fr": "nom d'hôte et votre mot de passe",
        "de": "Hostname und Ihr Passwort",
        "es": "nombre del equipo y su contraseña",
        "pt": "nome da máquina e a sua senha",
        "ru": "имя хоста и ваш пароль",
        "ja": "ホスト名とパスワード",
        "ko": "호스트 이름과 비밀번호",
        "hi": "होस्टनाम और आपका पासवर्ड",
        "tr": "makine adı ve parolanız",
        "id": "nama host dan kata sandi Anda",
    },
    "inst_hostname": {
        "ar": "اسم المضيف", "zh": "主机名", "fr": "Nom d'hôte",
        "de": "Hostname", "es": "Nombre del equipo", "pt": "Nome da máquina",
        "ru": "Имя хоста", "ja": "ホスト名", "ko": "호스트 이름",
        "hi": "होस्टनाम", "tr": "Makine adı", "id": "Nama host",
    },
    "inst_pw_label": {
        "ar": "كلمة مرور المستخدم 'brave'",
        "zh": "用户 brave 的密码",
        "fr": "Mot de passe de l'utilisateur « brave »",
        "de": "Passwort für Benutzer „brave“",
        "es": "Contraseña del usuario «brave»",
        "pt": "Senha do usuário «brave»",
        "ru": "Пароль пользователя brave",
        "ja": "ユーザー「brave」のパスワード",
        "ko": "사용자 'brave'의 비밀번호",
        "hi": "उपयोगकर्ता 'brave' का पासवर्ड",
        "tr": "'brave' kullanıcısının parolası",
        "id": "Kata sandi pengguna 'brave'",
    },
    "inst_pw2": {
        "ar": "أعد كلمة المرور", "zh": "重复密码",
        "fr": "Répéter le mot de passe", "de": "Passwort wiederholen",
        "es": "Repetir contraseña", "pt": "Repetir a senha",
        "ru": "Повторите пароль", "ja": "パスワードを再入力",
        "ko": "비밀번호 확인", "hi": "पासवर्ड दोबारा",
        "tr": "Parolayı tekrarla", "id": "Ulangi kata sandi",
    },
    "inst_err_pw_short": {
        "ar": "يجب أن تكون كلمة المرور 4 أحرف على الأقل.",
        "zh": "密码至少需要 4 个字符。",
        "fr": "Le mot de passe doit contenir au moins 4 caractères.",
        "de": "Das Passwort muss mindestens 4 Zeichen haben.",
        "es": "La contraseña debe tener al menos 4 caracteres.",
        "pt": "A senha deve ter pelo menos 4 caracteres.",
        "ru": "Пароль должен быть не короче 4 символов.",
        "ja": "パスワードは4文字以上必要です。",
        "ko": "비밀번호는 4자 이상이어야 합니다.",
        "hi": "पासवर्ड कम से कम 4 अक्षरों का हो।",
        "tr": "Parola en az 4 karakter olmalı.",
        "id": "Kata sandi minimal 4 karakter.",
    },
    "inst_err_pw_match": {
        "ar": "كلمتا المرور غير متطابقتين.",
        "zh": "两次输入的密码不一致。",
        "fr": "Les mots de passe ne correspondent pas.",
        "de": "Die Passwörter stimmen nicht überein.",
        "es": "Las contraseñas no coinciden.",
        "pt": "As senhas não coincidem.",
        "ru": "Пароли не совпадают.",
        "ja": "パスワードが一致しません。",
        "ko": "비밀번호가 일치하지 않습니다.",
        "hi": "पासवर्ड मेल नहीं खाते।",
        "tr": "Parolalar eşleşmiyor.",
        "id": "Kata sandi tidak cocok.",
    },
    "inst_confirm_t": {
        "ar": "جاهز للتثبيت", "zh": "准备安装",
        "fr": "Prêt à installer", "de": "Bereit zur Installation",
        "es": "Listo para instalar", "pt": "Pronto para instalar",
        "ru": "Готово к установке", "ja": "インストール準備完了",
        "ko": "설치 준비 완료", "hi": "इंस्टॉल के लिए तैयार",
        "tr": "Kuruluma hazır", "id": "Siap memasang",
    },
    "inst_confirm_s": {
        "ar": "تحقّق أخير", "zh": "最后确认", "fr": "dernière vérification",
        "de": "ein letzter Check", "es": "una última comprobación",
        "pt": "uma última verificação",
        "ru": "последняя проверка", "ja": "最終確認",
        "ko": "마지막 확인", "hi": "अंतिम जाँच",
        "tr": "son bir kontrol", "id": "pemeriksaan terakhir",
    },
    "inst_confirm_check": {
        "ar": "أفهم أن جميع البيانات على القرص الهدف ستُمسح",
        "zh": "我了解目标磁盘上的所有数据都将被擦除",
        "fr": "Je comprends que TOUTES LES DONNÉES du disque cible seront effacées",
        "de": "Ich verstehe, dass ALLE DATEN auf dem Zieldatenträger gelöscht werden",
        "es": "Entiendo que se borrarán TODOS LOS DATOS del disco de destino",
        "pt": "Entendo que TODOS OS DADOS do disco de destino serão apagados",
        "ru": "Я понимаю, что все данные на целевом диске будут стёрты",
        "ja": "ターゲットディスクの全データが消去されることを理解しました",
        "ko": "대상 디스크의 모든 데이터가 삭제됨을 이해합니다",
        "hi": "मैं समझता/समझती हूँ कि लक्ष्य डिस्क का सारा डेटा मिट जाएगा",
        "tr": "Hedef diskteki TÜM VERİLERİN silineceğini anlıyorum",
        "id": "Saya paham bahwa SEMUA DATA pada disk target akan dihapus",
    },
    "inst_install_now": {
        "ar": "ثبّت الآن", "zh": "立即安装", "fr": "Installer maintenant",
        "de": "Jetzt installieren", "es": "Instalar ahora",
        "pt": "Instalar agora", "ru": "Установить сейчас",
        "ja": "今すぐインストール", "ko": "지금 설치",
        "hi": "अभी इंस्टॉल करें", "tr": "Şimdi kur", "id": "Pasang sekarang",
    },
    "inst_err_tick": {
        "ar": "علّم مربع التأكيد أولاً.",
        "zh": "请先勾选确认框。",
        "fr": "Cochez d'abord la case de confirmation.",
        "de": "Bitte zuerst das Bestätigungsfeld ankreuzen.",
        "es": "Marca primero la casilla de confirmación.",
        "pt": "Primeiro marque a caixa de confirmação.",
        "ru": "Сначала отметьте чекбокс подтверждения.",
        "ja": "先に確認チェックボックスにチェックを入れてください。",
        "ko": "먼저 확인 확인란을 선택하세요.",
        "hi": "पहले पुष्टि बॉक्स पर टिक करें।",
        "tr": "Önce onay kutusunu işaretleyin.",
        "id": "Centang kotak konfirmasi terlebih dahulu.",
    },
    "inst_confirm_body": {
        "ar": "القرص الهدف: <b>%s</b>\nاسم المضيف: <b>%s</b>\nالوضع: <b>%s</b>\n\n"
              "سيعاد تقسيم القرص (GPT): 512 MiB EFI + 1 MiB BIOS + الجذر. "
              "سيُثبّت GRUB لـ BIOS و UEFI.",
        "zh": "目标磁盘：<b>%s</b>\n主机名：<b>%s</b>\n模式：<b>%s</b>\n\n"
              "磁盘将重新分区（GPT）：512 MiB EFI + 1 MiB BIOS + 根分区。"
              "GRUB 将同时安装 BIOS 与 UEFI 版本。",
        "fr": "Disque cible : <b>%s</b>\nNom d'hôte : <b>%s</b>\nMode : <b>%s</b>\n\n"
              "Le disque sera repartitionné (GPT) : 512 Mio EFI + 1 Mio BIOS + "
              "racine. GRUB sera installé pour BIOS et UEFI.",
        "de": "Zieldatenträger: <b>%s</b>\nHostname: <b>%s</b>\nModus: <b>%s</b>\n\n"
              "Der Datenträger wird neu partitioniert (GPT): 512 MiB EFI + "
              "1 MiB BIOS + Root. GRUB wird für BIOS und UEFI installiert.",
        "es": "Disco de destino: <b>%s</b>\nNombre del equipo: <b>%s</b>\n"
              "Modo: <b>%s</b>\n\nEl disco se reparticionará (GPT): 512 MiB EFI + "
              "1 MiB BIOS + raíz. Se instalará GRUB para BIOS y UEFI.",
        "pt": "Disco de destino: <b>%s</b>\nNome da máquina: <b>%s</b>\n"
              "Modo: <b>%s</b>\n\nO disco será reparticionado (GPT): 512 MiB "
              "EFI + 1 MiB BIOS + raiz. O GRUB será instalado para BIOS e UEFI.",
        "ru": "Целевой диск: <b>%s</b>\nИмя хоста: <b>%s</b>\nРежим: <b>%s</b>\n\n"
              "Диск будет переразбит (GPT): 512 МиБ EFI + 1 МиБ BIOS + корень. "
              "GRUB будет установлен для BIOS и UEFI.",
        "ja": "ターゲットディスク: <b>%s</b>\nホスト名: <b>%s</b>\n"
              "モード: <b>%s</b>\n\n"
              "ディスクは再パーティション（GPT）されます：512 MiB EFI + 1 MiB BIOS + "
              "ルート。GRUBはBIOSとUEFIの両方にインストールされます。",
        "ko": "대상 디스크: <b>%s</b>\n호스트 이름: <b>%s</b>\n모드: <b>%s</b>\n\n"
              "디스크가 다시 파티션됩니다(GPT): 512 MiB EFI + 1 MiB BIOS + 루트. "
              "GRUB는 BIOS와 UEFI 모두에 설치됩니다.",
        "hi": "लक्ष्य डिस्क: <b>%s</b>\nहोस्टनाम: <b>%s</b>\nमोड: <b>%s</b>\n\n"
              "डिस्क फिर से विभाजित होगी (GPT): 512 MiB EFI + 1 MiB BIOS + रूट। "
              "GRUB BIOS और UEFI दोनों के लिए इंस्टॉल होगा।",
        "tr": "Hedef disk: <b>%s</b>\nMakine adı: <b>%s</b>\nMod: <b>%s</b>\n\n"
              "Disk yeniden bölümlenecek (GPT): 512 MiB EFI + 1 MiB BIOS + kök. "
              "GRUB hem BIOS hem UEFI için kurulacak.",
        "id": "Disk target: <b>%s</b>\nNama host: <b>%s</b>\nMode: <b>%s</b>\n\n"
              "Disk akan dipartisi ulang (GPT): 512 MiB EFI + 1 MiB BIOS + root. "
              "GRUB akan dipasang untuk BIOS dan UEFI.",
    },
    "inst_dry_run": {
        "ar": "تشغيل تجريبي (بدون تغييرات)",
        "zh": "演练模式（不做更改）",
        "fr": "ESSAI (aucune modification)",
        "de": "PROBELAUF (keine Änderungen)",
        "es": "SIMULACIÓN (sin cambios)",
        "pt": "SIMULAÇÃO (sem alterações)",
        "ru": "ПРОБНЫЙ ЗАПУСК (без изменений)",
        "ja": "ドライラン（変更なし）", "ko": "드라이런(변경 없음)",
        "hi": "ड्राई-रन (कोई बदलाव नहीं)",
        "tr": "DENEME (değişiklik yok)",
        "id": "UJI COBA (tanpa perubahan)",
    },
    "inst_real": {
        "ar": "تثبيت حقيقي", "zh": "真实安装", "fr": "INSTALLATION RÉELLE",
        "de": "ECHTE INSTALLATION", "es": "INSTALACIÓN REAL",
        "pt": "INSTALAÇÃO REAL", "ru": "РЕАЛЬНАЯ УСТАНОВКА",
        "ja": "本番インストール", "ko": "실제 설치", "hi": "वास्तविक इंस्टॉल",
        "tr": "GERÇEK KURULUM", "id": "PASANG SUNGGUHAN",
    },
    "inst_target_disk": {
        "ar": "القرص الهدف", "zh": "目标磁盘", "fr": "Disque cible",
        "de": "Zieldatenträger", "es": "Disco de destino",
        "pt": "Disco de destino", "ru": "Целевой диск",
        "ja": "ターゲットディスク", "ko": "대상 디스크",
        "hi": "लक्ष्य डिस्क", "tr": "Hedef disk", "id": "Disk target",
    },
    "inst_mode": {
        "ar": "الوضع", "zh": "模式", "fr": "Mode", "de": "Modus",
        "es": "Modo", "pt": "Modo", "ru": "Режим", "ja": "モード",
        "ko": "모드", "hi": "मोड", "tr": "Mod", "id": "Mode",
    },
    "inst_progress_t": {
        "ar": "جارٍ التثبيت…", "zh": "正在安装…", "fr": "Installation…",
        "de": "Installation…", "es": "Instalando…", "pt": "Instalando…",
        "ru": "Установка…", "ja": "インストール中…", "ko": "설치 중…",
        "hi": "इंस्टॉल हो रहा है…", "tr": "Kuruluyor…", "id": "Memasang…",
    },
    "inst_progress_s": {
        "ar": "سيستغرق هذا عدة دقائق",
        "zh": "这需要几分钟",
        "fr": "cela prend plusieurs minutes",
        "de": "dies dauert einige Minuten",
        "es": "esto tarda varios minutos",
        "pt": "isto leva vários minutos",
        "ru": "это займёт несколько минут",
        "ja": "数分かかります", "ko": "몇 분 걸립니다",
        "hi": "इसमें कुछ मिनट लगेंगे",
        "tr": "bu birkaç dakika sürer",
        "id": "ini memakan waktu beberapa menit",
    },
    "inst_starting": {
        "ar": "بدء…", "zh": "正在启动…", "fr": "Démarrage…",
        "de": "Starten…", "es": "Comenzando…", "pt": "Iniciando…",
        "ru": "Запуск…", "ja": "開始中…", "ko": "시작 중…",
        "hi": "शुरू हो रहा है…", "tr": "Başlıyor…", "id": "Memulai…",
    },
    "inst_done_t": {
        "ar": "اكتمل التثبيت", "zh": "安装完成",
        "fr": "Installation terminée", "de": "Installation abgeschlossen",
        "es": "Instalación completa", "pt": "Instalação concluída",
        "ru": "Установка завершена", "ja": "インストール完了",
        "ko": "설치 완료", "hi": "इंस्टॉल पूरा हुआ",
        "tr": "Kurulum tamamlandı", "id": "Pemasangan selesai",
    },
    "inst_done_s": {
        "ar": "تم تثبيت BraveOS وهو جاهز.",
        "zh": "BraveOS 已安装就绪。",
        "fr": "BraveOS est installé et prêt.",
        "de": "BraveOS ist installiert und bereit.",
        "es": "BraveOS está instalado y listo.",
        "pt": "O BraveOS está instalado e pronto.",
        "ru": "BraveOS установлен и готов.",
        "ja": "BraveOSがインストールされ、準備ができました。",
        "ko": "BraveOS가 설치되어 준비되었습니다.",
        "hi": "BraveOS इंस्टॉल हो गया और तैयार है।",
        "tr": "BraveOS kuruldu ve hazır.",
        "id": "BraveOS telah dipasang dan siap.",
    },
    "inst_done_body": {
        "ar": "أزل قرص USB و<b>أعد التشغيل</b> لبدء نظامك الجديد. تبقى الجلسة "
              "الحية تعمل إن أردت متابعة التجربة.",
        "zh": "移除 USB 闪存盘并<b>重启</b>以启动新系统。如想继续体验，Live 会话仍会运行。",
        "fr": "Retirez la clé USB et <b>redémarrez</b> pour démarrer votre "
              "nouveau système. La session live continue si vous voulez tester.",
        "de": "Entfernen Sie den USB-Stick und <b>starten Sie neu</b>, um Ihr "
              "neues System zu booten. Die Live-Sitzung läuft weiter, wenn Sie "
              "weiter testen wollen.",
        "es": "Retire el USB y <b>reinicie</b> para arrancar su nuevo sistema. "
              "La sesión live sigue si quiere seguir probando.",
        "pt": "Remova o USB e <b>reinicie</b> para iniciar o novo sistema. "
              "A sessão live continua se quiser continuar testando.",
        "ru": "Извлеките USB и <b>перезагрузитесь</b>, чтобы стартовать новую "
              "систему. Live-сеанс продолжит работу, если хотите тестировать.",
        "ja": "USBを取り外して<b>再起動</b>すると新しいシステムが起動します。"
              "引き続き試したい場合はLiveセッションが動き続けます。",
        "ko": "USB를 제거하고 <b>재시작</b>하면 새 시스템이 시작됩니다. "
              "계속 체험하려면 Live 세션이 유지됩니다.",
        "hi": "USB निकालें और <b>रीबूट</b> करें ताकि नया सिस्टम शुरू हो। "
              "और आज़माना चाहें तो Live सेशन चलता रहेगा।",
        "tr": "USB'yi çıkarın ve yeni sisteminizi başlatmak için "
              "<b>yeniden başlatın</b>. Denemeye devam etmek isterseniz canlı "
              "oturum açık kalır.",
        "id": "Lepaskan USB dan <b>mulai ulang</b> untuk menjalankan sistem "
              "baru Anda. Sesi live tetap berjalan jika ingin terus mencoba.",
    },
    "inst_reboot_now": {
        "ar": "أعد التشغيل الآن", "zh": "立即重启",
        "fr": "Redémarrer maintenant", "de": "Jetzt neu starten",
        "es": "Reiniciar ahora", "pt": "Reiniciar agora",
        "ru": "Перезагрузить сейчас", "ja": "今すぐ再起動",
        "ko": "지금 재시작", "hi": "अभी रीबूट करें",
        "tr": "Şimdi yeniden başlat", "id": "Mulai ulang sekarang",
    },
    "inst_close": {
        "ar": "إغلاق", "zh": "关闭", "fr": "Fermer", "de": "Schließen",
        "es": "Cerrar", "pt": "Fechar", "ru": "Закрыть", "ja": "閉じる",
        "ko": "닫기", "hi": "बंद करें", "tr": "Kapat", "id": "Tutup",
    },
    "inst_failed": {
        "ar": "فشل التثبيت:", "zh": "安装失败：", "fr": "ÉCHEC :",
        "de": "INSTALLATION FEHLGESCHLAGEN:", "es": "FALLÓ:",
        "pt": "FALHOU:", "ru": "ОШИБКА УСТАНОВКИ:", "ja": "失敗：",
        "ko": "실패:", "hi": "विफल:", "tr": "BAŞARISIZ:", "id": "GAGAL:",
    },
    "inst_done": {
        "ar": "تم.", "zh": "完成。", "fr": "Terminé.", "de": "Fertig.",
        "es": "Hecho.", "pt": "Concluído.", "ru": "Готово.",
        "ja": "完了。", "ko": "완료.", "hi": "पूरा।", "tr": "Bitti.",
        "id": "Selesai.",
    },
}

WELCOME = {
    "wel_title": {
        "ar": "مرحباً بك في BraveOS", "zh": "欢迎使用 BraveOS",
        "fr": "Bienvenue dans BraveOS", "de": "Willkommen bei BraveOS",
        "es": "Bienvenido a BraveOS", "pt": "Bem-vindo ao BraveOS",
        "ru": "Добро пожаловать в BraveOS", "ja": "BraveOSへようこそ",
        "ko": "BraveOS에 오신 것을 환영합니다",
        "hi": "BraveOS में आपका स्वागत है", "tr": "BraveOS'a hoş geldiniz",
        "id": "Selamat datang di BraveOS",
    },
    "wel_info": {
        "ar": "أنت تعمل في <b>الجلسة الحية</b>. لم يُثبَّت شيء بعد. جرّب كل "
              "شيء — Brave، الملفات، الإعدادات — ثم ثبّت عندما تكون مستعداً. "
              "التثبيت يمسح قرصاً واحداً من اختيارك.",
        "zh": "您正在运行 <b>Live 会话</b>。尚未安装任何内容。欢迎随意体验 Brave、"
              "文件、设置，准备好后再安装。安装会擦除您选择的一块磁盘。",
        "fr": "Vous êtes en <b>session live</b>. Rien n'est encore installé. "
              "Essayez tout — Brave, Fichiers, Paramètres — puis installez "
              "quand vous êtes prêt. L'installation efface un disque de votre choix.",
        "de": "Sie laufen in der <b>Live-Sitzung</b>. Es ist noch nichts "
              "installiert. Testen Sie alles — Brave, Dateien, Einstellungen — "
              "und installieren Sie, wenn Sie bereit sind. Die Installation "
              "löscht einen Datenträger Ihrer Wahl.",
        "es": "Está en la <b>sesión live</b>. Todavía no hay nada instalado. "
              "Pruebe todo — Brave, Archivos, Ajustes — e instale cuando esté "
              "listo. La instalación borra un disco de su elección.",
        "pt": "Você está na <b>sessão live</b>. Nada foi instalado ainda. "
              "Experimente tudo — Brave, Arquivos, Configurações — e instale "
              "quando estiver pronto. A instalação apaga um disco à sua escolha.",
        "ru": "Вы в <b>live-сеансе</b>. Пока ничего не установлено. Попробуйте "
              "всё — Brave, Файлы, Параметры — и ставьте систему, когда "
              "будете готовы. Установка сотрёт выбранный вами диск.",
        "ja": "<b>Liveセッション</b>で実行中です。まだ何もインストールされていませ"
              "ん。Brave・ファイル・設定をすべて試して、準備ができたらインストー"
              "ルしてください。インストールは選んだ1つのディスクを消去します。",
        "ko": "<b>Live 세션</b>에서 실행 중입니다. 아직 아무것도 설치되지 않았습니"
              "다. Brave, 파일, 설정을 모두 사용해 보고 준비되면 설치하세요. 설치"
              "는 선택한 디스크 하나를 지웁니다.",
        "hi": "आप <b>लाइव सेशन</b> चला रहे हैं। अभी कुछ भी इंस्टॉल नहीं हुआ है। "
              "सब कुछ आज़माएँ — Brave, फ़ाइलें, सेटिंग्स — और तैयार होने पर "
              "इंस्टॉल करें। इंस्टॉल करने पर आपकी चुनी एक डिस्क मिट जाएगी।",
        "tr": "<b>Canlı oturum</b> çalışıyorsunuz. Henüz hiçbir şey kurulmadı. "
              "Her şeyi deneyin — Brave, Dosyalar, Ayarlar — hazır olduğunuzda "
              "kurun. Kurulum seçtiğiniz bir diski siler.",
        "id": "Anda menjalankan <b>sesi live</b>. Belum ada yang dipasang. "
              "Coba semuanya — Brave, Berkas, Pengaturan — lalu pasang saat "
              "Anda siap. Pemasangan menghapus satu disk pilihan Anda.",
    },
    "wel_use_brave_t": {
        "ar": "استخدم Brave الآن", "zh": "立即使用 Brave",
        "fr": "Utiliser Brave maintenant", "de": "Brave jetzt benutzen",
        "es": "Usar Brave ahora", "pt": "Usar o Brave agora",
        "ru": "Открыть Brave", "ja": "今すぐBraveを使う",
        "ko": "지금 Brave 사용", "hi": "अभी Brave इस्तेमाल करें",
        "tr": "Brave'i şimdi kullan", "id": "Gunakan Brave sekarang",
    },
    "wel_use_brave_d": {
        "ar": "تطبيق النظام الأساسي، مهيأ مسبقاً",
        "zh": "系统核心应用，已预先配置",
        "fr": "l'application centrale du système, préconfigurée",
        "de": "die Kern-App des Systems, vorkonfiguriert",
        "es": "la aplicación principal del sistema, preconfigurada",
        "pt": "o aplicativo central do sistema, pré-configurado",
        "ru": "главное приложение системы, уже настроено",
        "ja": "システムの中核アプリ、事前設定済み",
        "ko": "시스템 핵심 앱, 미리 구성됨",
        "hi": "सिस्टम का मुख्य ऐप, पूर्व-कॉन्फ़िगर",
        "tr": "sistemin çekirdek uygulaması, önceden yapılandırılmış",
        "id": "aplikasi inti sistem, sudah dikonfigurasi",
    },
    "wel_files_t": {
        "ar": "تصفّح الملفات", "zh": "浏览文件",
        "fr": "Parcourir les fichiers", "de": "Dateien durchsuchen",
        "es": "Explorar archivos", "pt": "Explorar arquivos",
        "ru": "Открыть файлы", "ja": "ファイルを見る", "ko": "파일 탐색",
        "hi": "फ़ाइलें ब्राउज़ करें", "tr": "Dosyalara göz at",
        "id": "Jelajahi berkas",
    },
    "wel_files_d": {
        "ar": "مدير الملفات PCManFM", "zh": "PCManFM 文件管理器",
        "fr": "gestionnaire de fichiers PCManFM",
        "de": "PCManFM-Dateimanager",
        "es": "gestor de archivos PCManFM",
        "pt": "gerenciador de arquivos PCManFM",
        "ru": "файловый менеджер PCManFM", "ja": "PCManFMファイルマネージャー",
        "ko": "PCManFM 파일 관리자", "hi": "PCManFM फ़ाइल मैनेजर",
        "tr": "PCManFM dosya yöneticisi", "id": "pengelola berkas PCManFM",
    },
    "wel_install_t": {
        "ar": "ثبّت BraveOS على القرص", "zh": "将 BraveOS 安装到磁盘",
        "fr": "Installer BraveOS sur le disque",
        "de": "BraveOS auf Datenträger installieren",
        "es": "Instalar BraveOS en el disco",
        "pt": "Instalar o BraveOS no disco",
        "ru": "Установить BraveOS на диск",
        "ja": "BraveOSをディスクにインストール",
        "ko": "BraveOS를 디스크에 설치",
        "hi": "BraveOS को डिस्क पर इंस्टॉल करें",
        "tr": "BraveOS'u diske kur", "id": "Pasang BraveOS ke disk",
    },
    "wel_install_d": {
        "ar": "مثبّت موجّه — يمسح قرصاً واحداً",
        "zh": "引导式安装 — 会擦除一块磁盘",
        "fr": "installateur guidé — efface un disque",
        "de": "geführte Installation — löscht einen Datenträger",
        "es": "instalador guiado — borra un disco",
        "pt": "instalador guiado — apaga um disco",
        "ru": "мастер установки — стирает один диск",
        "ja": "ガイド付きインストーラー — ディスクを1つ消去",
        "ko": "안내식 설치 관리자 — 디스크 하나를 지움",
        "hi": "निर्देशित इंस्टॉलर — एक डिस्क मिटाता है",
        "tr": "rehberli kurulum — bir diski siler",
        "id": "pemasang terpandu — menghapus satu disk",
    },
    "wel_note": {
        "ar": "يمكن إعادة فتح هذه النافذة من أيقونة سطح المكتب أو الإعدادات ← الجلسة الحية.",
        "zh": "可以从桌面图标或 设置 → Live 会话 重新打开此窗口。",
        "fr": "Cette fenêtre peut être rouverte depuis l'icône du bureau ou "
              "Paramètres → Session live.",
        "de": "Dieses Fenster kann über das Desktop-Symbol oder Einstellungen → "
              "Live-Sitzung erneut geöffnet werden.",
        "es": "Esta ventana puede reabrirse desde el icono del escritorio o "
              "Ajustes → Sesión live.",
        "pt": "Esta janela pode ser reaberta pelo ícone da área de trabalho ou "
              "Configurações → Sessão live.",
        "ru": "Это окно можно снова открыть через значок на рабочем столе или "
              "Параметры → Live-сеанс.",
        "ja": "このウィンドウはデスクトップのアイコンや 設定 → Liveセッション "
              "から再度開けます。",
        "ko": "이 창은 바탕 화면 아이콘이나 설정 → Live 세션에서 다시 열 수 있습니다.",
        "hi": "इस विंडो को डेस्कटॉप आइकन या सेटिंग्स → लाइव सेशन से दोबारा खोला जा सकता है।",
        "tr": "Bu pencere masaüstü simgesinden veya Ayarlar → Canlı oturum "
              "aracılığıyla yeniden açılabilir.",
        "id": "Jendela ini dapat dibuka kembali dari ikon desktop atau "
              "Pengaturan → Sesi live.",
    },
}
