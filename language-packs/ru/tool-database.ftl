tool-database-no-cutting-data = Нет режимов резания для { $material }: используются данные её класса или данные по умолчанию
tool-database-tool-changer = Смена инструмента: { $machine }
tool-database-back-to-default = Вернуть значение по умолчанию для этого типа: { $default }
tool-database-chip-load = Нагрузка на зуб: { $load }
tool-database-loaded-in = Установлена в { $machine }
tool-database-no-data-yet = Режимов резания для { $material } пока нет: показаны данные по умолчанию, а изменение сохранится как собственные данные для { $material }
tool-database-points = { $count ->
    [one] { $count } точка
    [few] { $count } точки
    [many] { $count } точек
   *[other] { $count } точки
    }
tool-database-from-slot = из гнезда T{ $slot } устройства смены инструмента станка { $machine }
tool-database-taken-as = не задано: принято как { $shown }
tool-database-from-class-range = { $tool }: из диапазона класса «{ $class }», подача { $feed } mm/min при { $speed } об/мин
tool-database-class-span = От { $soft } ({ $soft_hardness }) до { $hard } ({ $hard_hardness }), { $scale }: промежуточный материал режется с промежуточными подачами по его твёрдости, если у него нет собственных подач
