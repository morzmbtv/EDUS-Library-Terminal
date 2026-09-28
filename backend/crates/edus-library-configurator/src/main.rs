#![cfg_attr(not(test), windows_subsystem = "windows")]
use edus_library_configurator::ipc;

use serde_json::{json, Value};
use std::{
    mem::size_of,
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
};
use windows::{
    core::{w, BOOL, PCWSTR},
    Win32::{
        Foundation::*,
        Graphics::Gdi::{GetStockObject, COLOR_WINDOW, DEFAULT_GUI_FONT, HBRUSH, HFONT},
        Security::*,
        System::{
            LibraryLoader::GetModuleHandleW,
            Threading::{GetCurrentProcess, OpenProcessToken},
        },
        UI::{
            Controls::EM_SETLIMITTEXT,
            Input::KeyboardAndMouse::{EnableWindow, GetFocus},
            WindowsAndMessaging::*,
        },
    },
};

const ACTION: usize = 100;
const EXECUTE: usize = 101;
const APPLY: usize = 102;
const ACTIONS: &[(&str, &str)] = &[
    ("Служба и версия", "GetServiceStatus"),
    ("Текущая рабочая область", "GetWorkspaceStatus"),
    ("Открыть UAT (без Cloud)", "UAT"),
    ("Открыть настроенный Production", "PRODUCTION"),
    ("Подключить Production к Cloud", "EnrollDevice"),
    ("Список тестовых читателей и книг", "ListUatData"),
    ("Добавить тестового читателя", "CreateUatReader"),
    ("Добавить тестовый экземпляр книги", "CreateUatBook"),
    ("Импорт CSV — предварительная проверка", "PreviewUatReaders"),
    ("Привязать физическую карту", "BindUatCard"),
    ("Привязать сканерный код", "BindUatScannerCode"),
    ("Создать зашифрованную резервную копию", "CreateBackup"),
    ("Список резервных копий текущей области", "ListBackups"),
    ("Восстановить UAT из проверенной копии", "RestoreBackup"),
    ("Статус оборудования / Face boundary", "GetHardwareStatus"),
    ("Безопасная диагностика", "CollectDiagnostics"),
    ("Сбросить только UAT — два подтверждения", "ResetUatData"),
    ("Восстановление / миграция старой версии", "UNAVAILABLE"),
];

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
unsafe fn set_text(hwnd: HWND, text: &str) {
    let text = wide(text);
    let _ = SetWindowTextW(hwnd, PCWSTR(text.as_ptr()));
}
unsafe fn window_text(hwnd: HWND) -> String {
    let len = GetWindowTextLengthW(hwnd).max(0) as usize;
    let mut buf = vec![0; len.min(256 * 1024) + 1];
    let n = GetWindowTextW(hwnd, &mut buf).max(0) as usize;
    String::from_utf16_lossy(&buf[..n])
}
unsafe fn control(
    parent: HWND,
    class: PCWSTR,
    text: &str,
    id: usize,
    rect: (i32, i32, i32, i32),
    style: WINDOW_STYLE,
) -> HWND {
    let title = wide(text);
    let hwnd = CreateWindowExW(
        WINDOW_EX_STYLE(0),
        class,
        PCWSTR(title.as_ptr()),
        WS_CHILD | WS_VISIBLE | style,
        rect.0,
        rect.1,
        rect.2,
        rect.3,
        Some(parent),
        Some(HMENU(id as *mut _)),
        None,
        None,
    )
    .unwrap_or_default();
    let font: HFONT = HFONT(GetStockObject(DEFAULT_GUI_FONT).0);
    SendMessageW(
        hwnd,
        WM_SETFONT,
        Some(WPARAM(font.0 as usize)),
        Some(LPARAM(1)),
    );
    hwnd
}
struct Field {
    name: &'static str,
    hwnd: HWND,
    label: HWND,
    options: Vec<String>,
}
struct Pending {
    command: String,
    receiver: Receiver<Result<Value, String>>,
}
struct App {
    hwnd: HWND,
    action: HWND,
    output: HWND,
    submit: HWND,
    apply: HWND,
    fields: Vec<Field>,
    pending: Option<Pending>,
    data: Option<Value>,
    backups: Option<Value>,
    csv_preview: Option<Value>,
}
impl App {
    unsafe fn new(hwnd: HWND) -> Self {
        control(
            hwnd,
            w!("STATIC"),
            "EDUS Terminal Configurator 2.0.0-rc.1",
            0,
            (20, 15, 1000, 28),
            WINDOW_STYLE(0),
        );
        control(hwnd,w!("STATIC"),"Администрирование EDUS через защищённый канал службы. Windows kiosk настраивается вне EDUS.",0,(20,44,1000,25),WINDOW_STYLE(0));
        let action = control(
            hwnd,
            w!("COMBOBOX"),
            "",
            ACTION,
            (20, 78, 1010, 500),
            WS_TABSTOP | WINDOW_STYLE(CBS_DROPDOWNLIST as u32) | WS_VSCROLL,
        );
        for (label, _) in ACTIONS {
            let label = wide(label);
            SendMessageW(
                action,
                CB_ADDSTRING,
                None,
                Some(LPARAM(label.as_ptr() as isize)),
            );
        }
        SendMessageW(action, CB_SETCURSEL, Some(WPARAM(0)), None);
        let submit = control(
            hwnd,
            w!("BUTTON"),
            "Выполнить",
            EXECUTE,
            (20, 438, 270, 40),
            WS_TABSTOP | WINDOW_STYLE(BS_PUSHBUTTON as u32),
        );
        let apply = control(
            hwnd,
            w!("BUTTON"),
            "Применить проверенный CSV",
            APPLY,
            (310, 438, 340, 40),
            WS_TABSTOP | WINDOW_STYLE(BS_PUSHBUTTON as u32),
        );
        let _ = EnableWindow(apply, false);
        control(
            hwnd,
            w!("STATIC"),
            "Результат (операции выполняет служба; пароли Windows не запрашиваются)",
            0,
            (20, 492, 1010, 22),
            WINDOW_STYLE(0),
        );
        let output = control(
            hwnd,
            w!("EDIT"),
            "Выберите действие. Нажмите «Выполнить».",
            0,
            (20, 520, 1010, 170),
            WS_VSCROLL
                | WS_BORDER
                | WINDOW_STYLE((ES_MULTILINE | ES_READONLY | ES_AUTOVSCROLL) as u32),
        );
        Self {
            hwnd,
            action,
            output,
            submit,
            apply,
            fields: vec![],
            pending: None,
            data: None,
            backups: None,
            csv_preview: None,
        }
    }
    unsafe fn selected(&self) -> &'static str {
        let index = SendMessageW(self.action, CB_GETCURSEL, None, None).0;
        ACTIONS
            .get(index.max(0) as usize)
            .map(|a| a.1)
            .unwrap_or("GetServiceStatus")
    }
    unsafe fn add_field(
        &mut self,
        name: &'static str,
        label: &str,
        default: &str,
        options: Vec<(String, String)>,
        secret: bool,
        multiline: bool,
    ) {
        let y = 126 + self.fields.len() as i32 * 48;
        let label = control(
            self.hwnd,
            w!("STATIC"),
            label,
            0,
            (20, y + 4, 365, 35),
            WINDOW_STYLE(0),
        );
        let hwnd = if options.is_empty() {
            let flags = if multiline {
                ES_MULTILINE | ES_AUTOVSCROLL | ES_WANTRETURN
            } else if secret {
                ES_PASSWORD | ES_AUTOHSCROLL
            } else {
                ES_AUTOHSCROLL
            };
            control(
                self.hwnd,
                w!("EDIT"),
                default,
                200 + self.fields.len(),
                (390, y, 640, if multiline { 275 } else { 32 }),
                WS_TABSTOP
                    | WS_BORDER
                    | WINDOW_STYLE(flags as u32)
                    | if multiline {
                        WS_VSCROLL
                    } else {
                        WINDOW_STYLE(0)
                    },
            )
        } else {
            let hwnd = control(
                self.hwnd,
                w!("COMBOBOX"),
                "",
                200 + self.fields.len(),
                (390, y, 640, 300),
                WS_TABSTOP | WS_VSCROLL | WINDOW_STYLE(CBS_DROPDOWNLIST as u32),
            );
            for (_, label) in &options {
                let label = wide(label);
                SendMessageW(
                    hwnd,
                    CB_ADDSTRING,
                    None,
                    Some(LPARAM(label.as_ptr() as isize)),
                );
            }
            SendMessageW(hwnd, CB_SETCURSEL, Some(WPARAM(0)), None);
            hwnd
        };
        if options.is_empty() {
            SendMessageW(
                hwnd,
                EM_SETLIMITTEXT,
                Some(WPARAM(if multiline { 240 * 1024 } else { 2048 })),
                None,
            );
        }
        self.fields.push(Field {
            name,
            hwnd,
            label,
            options: options.into_iter().map(|(value, _)| value).collect(),
        });
    }
    unsafe fn form(&mut self) {
        for f in self.fields.drain(..) {
            set_text(f.hwnd, "");
            let _ = DestroyWindow(f.hwnd);
            let _ = DestroyWindow(f.label);
        }
        self.csv_preview = None;
        let _ = EnableWindow(self.apply, false);
        let command = self.selected();
        match command {
            "EnrollDevice" => { self.add_field("cloudUrl","Cloud API URL (HTTPS)","",vec![],false,false); self.add_field("enrollmentCode","Код подключения (не пароль Windows)","",vec![],true,false); self.add_field("deviceName","Название терминала","",vec![],false,false); }
            "CreateUatReader" => {
                self.add_field("externalId","Внешний ID (уникальный)","",vec![],false,false);
                self.add_field("fullName","Вымышленное имя","",vec![],false,false);
                self.add_field("personType","Тип читателя","",options(&["STUDENT","TEACHER","STAFF"]),false,false);
                self.add_field("className","Класс (обязателен для STUDENT)","",vec![],false,false);
                self.add_field("positionName","Должность (необязательно)","",vec![],false,false);
                self.add_field("status","Статус","",options(&["ACTIVE","INACTIVE"]),false,false);
            }
            "CreateUatBook" => { for (name,label) in [("name","Название"),("author","Автор"),("isbn","ISBN (необязательно)"),("year","Год издания"),("code","Инвентарный / сканерный код")] { self.add_field(name,label,"",vec![],false,false); } }
            "PreviewUatReaders" => { self.add_field("csv","Вставьте UTF-8 CSV из шаблона ниже.\r\nexternal_id,full_name,person_type,\r\nclass_name,position_name,status","external_id,full_name,person_type,class_name,position_name,status\r\n",vec![],false,true); }
            "BindUatCard" | "BindUatScannerCode" => {
                let reader=command=="BindUatCard";
                let records=self.data.as_ref().and_then(|v|v[if reader{"readers"}else{"books"}].as_array()).cloned().unwrap_or_default();
                let choices: Vec<_>=records.iter().filter_map(|r|Some((r["id"].as_str()?.to_string(),format!("{} · {}",r[if reader{"fullName"}else{"title"}].as_str().unwrap_or(""),r[if reader{"externalId"}else{"inventoryNumber"}].as_str().unwrap_or(""))))).collect();
                if choices.is_empty() { set_text(self.output,"Сначала загрузите «Список тестовых читателей и книг». Создайте запись, если список пуст."); }
                self.add_field(if reader{"readerId"}else{"copyId"},if reader{"Читатель (сначала загрузите список)"}else{"Экземпляр (сначала загрузите список)"},"",choices,false,false);
                self.add_field(if reader{"card"}else{"code"},"Нажмите поле и приложите карту / сканируйте","",vec![],reader,false);
            }
            "RestoreBackup"=>{
                let records=self.backups.as_ref().and_then(Value::as_array).cloned().unwrap_or_default();
                let choices:Vec<_>=records.iter().filter_map(|r|Some((r["fileName"].as_str()?.to_string(),format!("{} · {} bytes",r["fileName"].as_str()?,r["sizeBytes"].as_u64().unwrap_or(0))))).collect();
                if choices.is_empty(){set_text(self.output,"Сначала загрузите список резервных копий. Восстановление разрешено только в UAT. Production restore недоступен из-за требований согласованности Cloud cursor.");}
                self.add_field("fileName","Резервная копия (из списка службы)","",choices,false,false);
            }
            "UNAVAILABLE"=>set_text(self.output,"Миграция старой Tauri-базы не выполняется автоматически. Production restore недоступен до реализации Cloud-safe reconciliation. Старая база не изменяется. Для чистой проверки используйте отдельную UAT-область; перенос данных для неё не обязателен."),
            _=>{}
        }
    }
    unsafe fn values(&self) -> Value {
        let mut values = serde_json::Map::new();
        for f in &self.fields {
            let value = if f.options.is_empty() {
                window_text(f.hwnd)
            } else {
                let i = SendMessageW(f.hwnd, CB_GETCURSEL, None, None).0;
                f.options
                    .get(i.max(0) as usize)
                    .cloned()
                    .unwrap_or_default()
            };
            values.insert(f.name.into(), Value::String(value));
        }
        Value::Object(values)
    }
    unsafe fn confirm(&self, text: &str) -> bool {
        let text = wide(text);
        MessageBoxW(
            Some(self.hwnd),
            PCWSTR(text.as_ptr()),
            w!("Подтверждение EDUS"),
            MB_YESNO | MB_ICONWARNING | MB_DEFBUTTON2,
        ) == IDYES
    }
    unsafe fn start(&mut self, command: String, payload: Value) {
        if self.pending.is_some() {
            return;
        }
        let (sender, receiver) = mpsc::channel();
        let worker_command = command.clone();
        thread::spawn(move || {
            let result = ipc::request(worker_command, payload);
            let _ = sender.send(result);
        });
        self.pending = Some(Pending { command, receiver });
        let _ = EnableWindow(self.submit, false);
        let _ = EnableWindow(self.action, false);
        let _ = EnableWindow(self.apply, false);
        for field in &self.fields {
            let _ = EnableWindow(field.hwnd, false);
        }
        let seconds = self
            .pending
            .as_ref()
            .map(|pending| ipc::deadline(&pending.command).as_secs())
            .unwrap_or(35);
        set_text(self.output, &format!("Ожидание службы… До {seconds} секунд. При неизвестном результате не повторяйте мутацию до проверки данных."));
    }
    unsafe fn execute(&mut self) {
        if self.pending.is_some() {
            return;
        }
        let selected = self.selected();
        let mut command = selected.to_owned();
        let mut payload = self.values();
        match selected {
            "UNAVAILABLE" => {
                self.form();
                return;
            }
            "UAT" | "PRODUCTION" => {
                if !self.confirm("Переключить рабочую область? Завершите библиотечные операции. Данные областей не объединяются."){return;}
                command = "SetActiveWorkspace".into();
                payload = json!({"mode":selected,"cloudUrl":null,"schoolId":null,"terminalId":null,"deviceName":"EDUS Library"});
            }
            "ResetUatData" => {
                if !self.confirm("Удалить тестовые книги, читателей, карты, выдачи и очередь UAT? Production не изменяется.")||!self.confirm("Повторное подтверждение: сброс UAT необратим. Выполнить?"){return;}
                payload = json!({"confirmation":"RESET UAT"});
            }
            "RestoreBackup" => {
                let file = payload["fileName"].as_str().unwrap_or("");
                let exists = self
                    .backups
                    .as_ref()
                    .and_then(Value::as_array)
                    .is_some_and(|rows| rows.iter().any(|r| r["fileName"].as_str() == Some(file)));
                if !exists {
                    set_text(self.output,"Выберите резервную копию из актуального списка службы. Произвольные пути запрещены.");
                    return;
                }
                if !self.confirm("Восстановить UAT? Текущее состояние тестовых данных заменится данными копии. Служба сохранит recovery-копию текущей базы.")||!self.confirm("Завершите все библиотечные операции. Повторно подтвердите восстановление UAT."){return;}
                payload = json!({"fileName":file,"confirmation":"RESTORE UAT"});
            }
            "CreateUatBook" => {
                let year = match payload["year"].as_str().unwrap_or("").parse::<u32>() {
                    Ok(year) => year,
                    Err(_) => {
                        set_text(self.output, "Введите год издания числом.");
                        return;
                    }
                };
                payload = json!({"title":{"id":"","name":payload["name"],"author":payload["author"],"isbn":payload["isbn"],"publisher":"","year":year,"language":"ru","subject":"","grade":""},"mode":"COPY","codes":[payload["code"]],"quantity":1,"operationId":uuid::Uuid::new_v4().to_string()});
            }
            "PreviewUatReaders" => {
                self.csv_preview = Some(payload.clone());
            }
            "BindUatCard" | "BindUatScannerCode" | "EnrollDevice" => {
                if selected != "EnrollDevice" {
                    let reader = selected == "BindUatCard";
                    let id = payload[if reader { "readerId" } else { "copyId" }]
                        .as_str()
                        .unwrap_or("");
                    let listed = self
                        .data
                        .as_ref()
                        .and_then(|v| v[if reader { "readers" } else { "books" }].as_array())
                        .is_some_and(|rows| rows.iter().any(|r| r["id"].as_str() == Some(id)));
                    if !listed {
                        set_text(
                            self.output,
                            "Сначала загрузите список и выберите существующую тестовую запись.",
                        );
                        return;
                    }
                }
                for f in &self.fields {
                    if f.name == "card" || f.name == "enrollmentCode" {
                        set_text(f.hwnd, "");
                    }
                }
            }
            _ => {}
        }
        self.start(command, payload);
    }
    unsafe fn poll(&mut self) {
        let result = self
            .pending
            .as_ref()
            .and_then(|p| match p.receiver.try_recv() {
                Ok(result) => Some(result),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => Some(Err(
                    "ADMIN_IPC_INTERRUPTED: Результат неизвестен. Проверьте данные перед повтором."
                        .into(),
                )),
            });
        if let Some(result) = result {
            let command = self.pending.take().map(|p| p.command).unwrap_or_default();
            let _ = EnableWindow(self.submit, true);
            let _ = EnableWindow(self.action, true);
            for field in &self.fields {
                let _ = EnableWindow(field.hwnd, true);
            }
            match result {
                Ok(value) => {
                    if command == "ListUatData" {
                        self.data = Some(value.clone());
                    }
                    if command == "ListBackups" {
                        self.backups = Some(value.clone());
                    }
                    if command == "SetActiveWorkspace"
                        || command == "RestoreBackup"
                        || command == "ResetUatData"
                    {
                        self.data = None;
                        self.backups = None;
                    }
                    if command == "PreviewUatReaders" {
                        let _ = EnableWindow(self.apply, value["errorCount"].as_u64() == Some(0));
                    }
                    set_text(
                        self.output,
                        &serde_json::to_string_pretty(&value)
                            .unwrap_or_else(|_| "Невозможно отобразить результат".into()),
                    );
                }
                Err(error) => {
                    self.csv_preview = None;
                    set_text(self.output, &error);
                }
            }
        }
    }
}
fn options(values: &[&str]) -> Vec<(String, String)> {
    values
        .iter()
        .map(|s| (s.to_string(), s.to_string()))
        .collect()
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if msg == WM_CREATE {
        let app = Box::new(App::new(hwnd));
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(app) as isize);
        SetTimer(Some(hwnd), 1, 100, None);
        return LRESULT(0);
    }
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut App;
    if !ptr.is_null() {
        let app = &mut *ptr;
        match msg {
            WM_COMMAND => {
                let id = wparam.0 & 0xffff;
                let notification = (wparam.0 >> 16) & 0xffff;
                if id == ACTION && notification == CBN_SELCHANGE as usize {
                    app.form();
                }
                if id == EXECUTE && notification == BN_CLICKED as usize {
                    app.execute();
                }
                if id == APPLY
                    && notification == BN_CLICKED as usize
                    && app.pending.is_none()
                    && app.confirm("Проверьте результат preview. Применить только проверенный CSV одной транзакцией?")
                {
                    if let Some(payload) = app.csv_preview.take() {
                        app.start("ImportUatReaders".into(), payload);
                    }
                }
                return LRESULT(0);
            }
            WM_TIMER => {
                app.poll();
                return LRESULT(0);
            }
            WM_CLOSE => {
                if app.pending.is_some()&&!app.confirm("Операция ещё выполняется. Закрытие окна не отменяет commit службы. Закрыть и проверить результат позже?"){return LRESULT(0);}
                let _ = DestroyWindow(hwnd);
                return LRESULT(0);
            }
            WM_DESTROY => {
                let _ = KillTimer(Some(hwnd), 1);
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                drop(Box::from_raw(ptr));
                PostQuitMessage(0);
                return LRESULT(0);
            }
            _ => {}
        }
    }
    DefWindowProcW(hwnd, msg, wparam, lparam)
}
unsafe fn elevated_administrator() -> bool {
    let mut token = HANDLE::default();
    if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
        return false;
    }
    let mut elevation = TOKEN_ELEVATION::default();
    let mut len = 0;
    let elevated = GetTokenInformation(
        token,
        TokenElevation,
        Some((&mut elevation as *mut TOKEN_ELEVATION).cast()),
        size_of::<TOKEN_ELEVATION>() as u32,
        &mut len,
    )
    .is_ok()
        && elevation.TokenIsElevated != 0;
    let _ = CloseHandle(token);
    let mut sid = [0u8; SECURITY_MAX_SID_SIZE as usize];
    let mut size = sid.len() as u32;
    let mut member = BOOL(0);
    elevated
        && CreateWellKnownSid(
            WinBuiltinAdministratorsSid,
            None,
            Some(PSID(sid.as_mut_ptr().cast())),
            &mut size,
        )
        .is_ok()
        && CheckTokenMembership(None, PSID(sid.as_mut_ptr().cast()), &mut member).is_ok()
        && member.as_bool()
}
fn main() {
    unsafe {
        if !elevated_administrator() {
            MessageBoxW(None,w!("Запустите Configurator от имени администратора Windows. Обычный kiosk-пользователь не имеет доступа."),w!("EDUS Terminal Configurator"),MB_OK|MB_ICONERROR);
            return;
        }
        let instance = GetModuleHandleW(None).unwrap_or_default();
        let class = WNDCLASSW {
            lpfnWndProc: Some(wndproc),
            hInstance: instance.into(),
            lpszClassName: w!("EDUSNativeConfigurator"),
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            hbrBackground: HBRUSH((COLOR_WINDOW.0 + 1) as usize as *mut _),
            ..Default::default()
        };
        if RegisterClassW(&class) == 0 {
            return;
        }
        let hwnd = match CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("EDUSNativeConfigurator"),
            w!("EDUS Terminal Configurator"),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            1080,
            750,
            None,
            None,
            Some(instance.into()),
            None,
        ) {
            Ok(h) => h,
            Err(_) => return,
        };
        let _ = ShowWindow(hwnd, SW_SHOW);
        let mut msg = MSG::default();
        loop {
            let status = GetMessageW(&mut msg, None, 0, 0).0;
            if status <= 0 {
                break;
            }
            // A keyboard-wedge Enter suffix completes capture, not authorization
            // of a mutation. Binding always requires an explicit button click.
            let state = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut App;
            if msg.message == WM_KEYDOWN && msg.wParam.0 == 13 && !state.is_null() {
                let focused = GetFocus();
                if (&*state)
                    .fields
                    .iter()
                    .any(|f| matches!(f.name, "card" | "code") && f.hwnd == focused)
                {
                    continue;
                }
            }
            if !IsDialogMessageW(hwnd, &msg).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }
}
