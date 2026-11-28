gcode-view-check-before-cutting = Проверить перед резкой
gcode-view-checks-cancelled = Проверки перед резкой отменены: нажмите «Обновить», чтобы запустить их снова.
gcode-view-checks-found = Что нашли проверки перед резкой. Что менять — решать вам: просмотр ничего не сохраняет.
gcode-view-checks-passed = ✔ Проверки перед резкой не нашли ничего, что нужно исправить.
gcode-view-checks-running = Проверка траекторий перед резкой…
gcode-view-check-the-toolpaths-and-save = Проверить траектории и сохранить файлы для станка
gcode-view-copy-all = Копировать всё
gcode-view-error = ✖ Нужно исправить: { $message }
gcode-view-damage = ⛔ Может повредить станок: { $message }
gcode-view-refused-copy = Не скопировано: проверки нашли то, что может повредить станок. Исправьте это или отключите проверку в разделе «Защита станка» Конфигурации станка
gcode-view-copy-this-file-s-g = Копировать G-code этого файла, чтобы вставить в другое место
gcode-view-don-t-save = Не сохранять
gcode-view-don-t-send = Не отправлять
gcode-view-file = Файл:
gcode-view-go-ahead-with-errors = Сохранить несмотря на ошибки: вы прочитали каждую и берёте ответственность за рез на себя. Это будет указано в начале файлов.
gcode-view-go-back = Вернуться
gcode-view-go-ahead-you-have-read = Продолжить: вы прочитали это, и так и задумано
gcode-view-make-the-g-code-again = Заново создать G-code из траекторий в их текущем виде
gcode-view-post-processor = Постпроцессор:
gcode-view-refresh = Обновить
gcode-view-save-anyway = Всё равно сохранить
gcode-view-save-toolpaths = Сохранить траектории…
gcode-view-save-nothing-to-put-the = Ничего не сохранять, чтобы сначала исправить траектории
gcode-view-save-with-these-errors = Сохранить траектории с этими ошибками? Отметьте каждую, чтобы подтвердить, что вы её прочитали.
gcode-view-save-the-toolpaths-anyway = Всё равно сохранить траектории?
gcode-view-saving-makes-these-files-run = При сохранении создаются эти файлы, запускаемые в таком порядке: выберите один, чтобы прочитать
gcode-view-send-anyway = Всё равно отправить
gcode-view-tick-each-error = Отметьте каждую ошибку, чтобы подтвердить, что прочитали её. В следующий раз при сохранении будет задан вопрос снова.
gcode-view-there-is-nothing-to-cut = В показанных траекториях нечего резать.
gcode-view-will-not-run = В нынешнем виде задание не запустится на станке. Нажмите «Вернуться», чтобы исправить, или отметьте каждую ошибку, чтобы всё равно сохранить.
gcode-view-warning = ⚠ { $message }
gcode-view-what-saving-would-write-through = Что будет записано при сохранении через постпроцессор станка. Ничего не сохраняется.
gcode-view-lines-made-at = { $count ->
    [one] { $count } строка, создано в { $time }
    [few] { $count } строки, создано в { $time }
    [many] { $count } строк, создано в { $time }
   *[other] { $count } строки, создано в { $time }
    }
