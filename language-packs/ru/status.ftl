# The status line along the window's foot: what the app says it did, or why it did not.
# A value put in ({ $tool }, { $path }, { $error }) is filled in by PowerWhittle: keep it as it is.

## Undo, the preview, gadgets and imports
# $problems: what went wrong with each, separated by semicolons.
status-gadgets-not-loaded = Не удалось загрузить некоторые гаджеты: { $problems }
status-undone = Отменено
status-redone = Повторено
status-no-machine-yet = Станок ещё не добавлен: добавьте его в Станок ▸ Конфигурация станка
status-undid-last-preview = Последний предпросмотр отменён
status-preview-reset = Предпросмотр сброшен: «Отменить последний предпросмотр» вернёт его
# $path: the file it was saved in.
status-saved-brush = Кисть для лепки сохранена: { $path }
# $path: the imported file.
status-import-changed = Внимание: файл { $path } изменился после импорта; он импортирован заново в нынешнем виде
# $name: the toolpath's name.
status-notes-kept = Заметки сохранены вместе с { $name }

## Work done in the background
# $what: what was being worked out, such as a toolpath's name.
status-not-used-job-changed = { $what }: результат не использован, потому что задание изменилось во время расчёта. Попробуйте ещё раз
# $what: what was being worked out.
status-cancelled-nothing-changed = { $what }: отменено, ничего не изменилось
status-already-up-to-date = Уже актуально: ничего не изменилось, поэтому ничего не рассчитывалось

## Tool forms
# $error: what was wrong.
status-suggest-could-not-read = Не удалось прочитать форму для подсказки: { $error }
status-saved-texture = Текстура сохранена: { $path }
status-loaded-texture = Текстура загружена: { $path }
status-depth-estimated = Глубина оценена
status-cut-depth-is-thickness = Глубина реза равна толщине материала
# $path: the file written.
status-exported = Экспортировано: { $path }

## Drawing
status-shift-click-to-edit = Щёлкните с Shift по фигуре, нарисованной этим инструментом, чтобы изменить её
status-drawing-curves = Рисование кривых (S переключает)
status-drawing-lines = Рисование прямых линий (S переключает)
# $typed: what has been typed so far.
status-next-point = Следующая точка: { $typed }
status-type-a-point = Введите длину, x,y или угол A длину L
status-drawing-on-from-end = Продолжение от конца: Пробел или Esc завершает, Tab замыкает

## Tools applied, changed and cancelled
# $count: how many were saved.
status-saved-job-sheets = { $count ->
        [one] Сохранён { $count } лист задания
        [few] Сохранено { $count } листа задания
        [many] Сохранено { $count } листов задания
       *[other] Сохранено { $count } листа задания
    }
# $tool: the tool that drew it.
status-editing-the-shape = Изменение фигуры в инструменте { $tool }: меняйте форму, чтобы изменить фигуру
# After a tool's name and "applied" (status-applied). $level: the level's name.
status-placed-on-level = : новый компонент на уровне { $level }
status-placed-on-hidden-level = : новый компонент на уровне { $level } (уровень скрыт: покажите его, чтобы увидеть компонент)
# $tool: the tool. $placed: where what it made went (status-placed-on-level), or nothing.
status-applied = { $tool }: применено{ $placed }
# $turns: a number with two decimals.
status-strand-revolutions = Каждая прядь делает оборотов: { $turns }
# $tool: the tool whose settings changed.
status-changed = { $tool }: изменено
# $warnings: each warning, separated by semicolons.
status-warning = Внимание: { $warnings }
status-model-imported = Модель импортирована
# $tool: the tool whose form was cancelled.
status-cancelled = { $tool }: отменено

## Saving and sending toolpaths
status-nothing-saved-job-changed = Ничего не сохранено: задание изменилось во время сохранения. Сохраните ещё раз, чтобы проверки перед резкой видели его в нынешнем виде
status-toolpaths-not-sent = Траектории не отправлены
status-toolpaths-not-saved = Траектории не сохранены

## Selecting and snapping
# $kind: what it snapped to, such as midpoint.
status-snap-picked = Привязка выбрана: эта точка удерживается на объекте «{ $kind }». Нажмите Delete, чтобы отпустить её.
status-stepped-out-to-group = Выход на уровень группы, в которую входит объект
status-stepped-out-of-group = Выход из группы: щелчок снова выделяет группы целиком

## Gadgets
# $file: the gadget's file. $error: what was wrong.
status-gadget-not-used = Не удалось использовать { $file }: { $error }
status-no-gadgets-folder = У PowerWhittle нет папки настроек на этом компьютере для хранения гаджетов
status-could-not-open-gadgets-folder = Не удалось открыть папку «Мои гаджеты», { $path } ({ $error })

## Jobs and files
# $material: the material's size and kind.
status-new-job = Новое задание: { $material }
status-saved-template = Шаблон сохранён: { $path }
status-not-made-yet = { $tool }: пока не реализовано
# $file: the file to import.
status-set-up-then-import = Настройте материал, затем ОК создаст задание и импортирует в него { $file }
# $path: the folder recovery copies go in.
status-no-recovery-copies = Внимание: несохранённую работу нельзя копировать для восстановления, потому что в { $path } нельзя записывать. Сохраняйте почаще
status-recovery-copy-not-kept = не удалось сохранить копию для восстановления: { $error }
status-new-jobs-start-so = Новые задания будут начинаться с этими настройками
status-new-job-created = Новое задание создано

## Keys
status-end-let-go = Конец отпущен
status-preview-stopped = Предпросмотр остановлен: уже вырезанное сохранено
# $tool: the tool whose form is open again.
status-back-to = Снова { $tool }, как вы его оставили
status-could-not-print = не удалось напечатать: { $error }
status-could-not-open-folder = не удалось открыть папку: { $error }
status-recovered-save = Восстановлена несохранённая работа с прошлого запуска: сохраните её (Файл ▸ Сохранить) или закройте
status-recovered-save-as = Восстановлена несохранённая работа с прошлого запуска: сохраните её (Файл ▸ Сохранить как) или закройте

## First-time setup
# $machine: the machine's name and a comma, or nothing. $size: new jobs' material size.
status-set-up-blank = Настроено: { $machine }новые задания { $size }. Рисуйте на листе или нажмите «Открыть пример проекта». Это можно изменить в любой момент в Станок ▸ Конфигурация станка и Правка ▸ Параметры ▸ Настройки задания по умолчанию.
status-set-up-start = Настроено: { $machine }новые задания { $size }. Нажмите «Создать новый файл», чтобы начать, или «Открыть пример проекта». Это можно изменить в любой момент в Станок ▸ Конфигурация станка и Настройка задания.
status-setup-skipped = Настройка пропущена. Справка ▸ Повторить первоначальную настройку задаст вопросы снова.

## Options
status-title-bar-next-start = Заголовок окна изменится при следующем запуске PowerWhittle.

## The 3D view's handles
status-let-go-off-material = Отпущено за пределами материала: ничего не изменено
status-pointer-off-material = Указатель вне материала: верните его или отпустите, чтобы отменить
status-cancelled-nothing-done = Отменено: ничего не изменено
status-removed-piece-cut-free = Вырезанная деталь убрана: «Отменить последний предпросмотр» на странице «Траектории» вернёт её

## The Library
status-could-not-show = Не удалось показать { $path }: { $error }
status-file-gone = Не удалось добавить { $path }: файла больше нет.

## The preview
status-preview-closed-job-changed = Задание изменилось, поэтому элементы управления предпросмотром закрыты: запустите предпросмотр снова, чтобы воспроизвести его
status-preview-new-material = Настройка задания изменила материал, поэтому предпросмотр начинается заново на новой заготовке

## The 3D view
# $number: the button's number.
status-spacemouse-button-unused = Кнопка SpaceMouse { $number } пока ничего не делает в PowerWhittle

## The sample project
status-sample-opened = Пример вывески открыт с готовыми траекториями. Нажмите «Предпросмотр всех траекторий» на странице «Траектории», чтобы посмотреть, как он вырезается
status-sample-toolpaths-not-made = Не удалось создать траектории примера: { $error }

## Saving toolpaths
status-still-checking = Траектории ещё проверяются перед резкой: сохраните снова, когда проверки закончатся
status-checked-after-work = Траектории будут проверены перед резкой, когда завершится текущая работа: тогда сохраните снова
status-checks-cancelled = Проверки перед резкой отменены: нажмите «Проверить снова», затем сохраните

## The Bit Library and the Materials Library
# $error: why the library could not be read.
status-materials-not-saved = { $error }; изменения материалов не будут сохранены
status-tools-not-saved = { $error }; изменения фрез не будут сохранены
# $first: the first bit joined. $more: how many more.
status-bits-joined = Фрезы, указанные отдельно для каждого материала, теперь объединены в одну фрезу. { $first }{ $more ->
        [0] {""}
        [one] {" "}(и ещё { $more }; см. журнал)
       *[other] {" "}(и ещё { $more }; см. журнал)
    }

## Importing
status-import-cancelled = Импорт отменён
status-open-a-job-to-import = Откройте или создайте задание, чтобы импортировать в него { $file }

## After a crash
status-closed-with-error = В прошлый раз PowerWhittle завершился с ошибкой. Чтобы отправлять отчёты о сбоях, измените Правка ▸ Параметры ▸ Отчёты об ошибках.
# $path: where its crash log is kept.
status-closed-with-error-log = В прошлый раз PowerWhittle завершился с ошибкой. Журнал сбоя хранится в { $path }. Чтобы отправлять отчёты о сбоях, измените Правка ▸ Параметры ▸ Отчёты об ошибках.
status-select-to-zoom = выделите что-нибудь, чтобы приблизить

## The Web API
status-new-web-api-token = Создан новый токен Web API: теперь скриптам нужен новый токен
status-copied = Скопировано

## Tracing, tabs, joining ends and measuring
status-tracing-whole-picture = Трассировка всего изображения
status-no-colours-traced = Цвета не трассируются: щёлкните по цвету на изображении или отметьте его в окне трассировки
# $colours: the colours, separated by commas.
status-colours-to-trace = Цвета для трассировки: { $colours }
status-drop-tab-on-vector = перетащите перемычку на выделенный вектор, чтобы переместить её туда
status-tab-moved = Перемычка перемещена
status-tab-removed = Перемычка удалена
status-tab-added = Перемычка добавлена
status-click-to-add-tab = щёлкните по выделенному вектору, чтобы добавить перемычку, или по перемычке, чтобы удалить её
status-no-open-end = Здесь нет открытого конца: щёлкните по одному из кружков на концах открытых векторов
status-end-picked = Конец выбран: теперь щёлкните по концу, с которым его соединить
status-click-to-measure = щёлкните по вектору или протяните между двумя точками

## Drawing tools with no step of their own
# $about: what the tool does (its tooltip).
status-drag-in-the-view = { $about }: протяните мышью в окне вида
