# What a tool says when it cannot run, keyed by its English words (`error-`, its first words and a
# check of the whole text). The app starts each with a capital and ends it with a full stop,
# so write them as the English is. Each line says which tool says it.

## border-tile/src/lib.rs
error-the-depth-map-is-too-eab97d = карта глубины слишком большая

## commands/3d-finishing-toolpath/src/lib.rs
error-choose-at-least-one-bit-a1ac27 = выберите хотя бы одну фрезу
error-the-bit-s-finishing-stepover-217283 = чистовой шаг фрезы должен быть больше 0% и не больше 100%: задайте его в Библиотеке фрез

## commands/3d-roughing-toolpath/src/lib.rs
error-set-the-bit-s-clearing-13c7c3 = задайте шаг выборки фрезы больше 0% и не больше 100% в Библиотеке фрез
error-set-the-allowance-to-0-dcc459 = задайте припуск 0 или больше

## commands/add-draft/src/lib.rs
error-there-is-no-visible-model-48ad9f = нет видимой модели, к которой можно добавить уклон. Покажите компонент или создайте его инструментом моделирования
error-set-the-draft-angle-above-cbb20b = задайте угол уклона больше 0° и не больше 45°

## commands/array-copy-toolpath/src/lib.rs
error-show-the-toolpaths-to-copy-46f456 = покажите в списке траекторий те траектории, которые нужно скопировать
error-the-toolpaths-to-copy-have-bbf7aa = в копируемых траекториях нет резов. Измените их и сначала нажмите «Рассчитать»

## commands/array-copy/src/lib.rs
error-select-vectors-to-copy-10add3 = выделите векторы для копирования
error-set-rows-and-columns-to-cd6d0a = задайте число строк и столбцов 1 или больше
error-one-row-of-one-column-2d41dd = одна строка и один столбец не дают копий. Задайте 2 или больше строк или столбцов

## commands/auto-layout-text/src/lib.rs
error-select-a-closed-vector-to-9a26d4 = выделите замкнутый вектор, в который нужно вписать текст
error-set-the-border-from-0-9636b9 = задайте отступ от 0 до менее чем 50 %
error-enter-some-text-4d3e12 = введите текст
error-enter-some-text-the-font-ef7b92 = введите текст, который может отобразить этот шрифт

## commands/bake-components/src/lib.rs
error-show-at-least-two-components-0bdcf2 = покажите хотя бы два компонента, чтобы запечь их вместе

## commands/bitmap-properties/src/lib.rs
error-there-are-no-pictures-in-dfcb60 = в задании нет изображений. Сначала импортируйте изображение

## commands/celtic-weave/src/lib.rs
error-give-the-knot-grid-at-587f45 = задайте сетке узла хотя бы одну ячейку и размер ячейки больше 0

## commands/chamfer-toolpath/src/lib.rs
error-select-the-closed-vectors-to-8ec819 = выделите замкнутые векторы для снятия фаски
error-the-chamfer-does-not-fit-bac422 = фаска не помещается внутри выделенных фигур. Уменьшите ширину или выберите «Снаружи»
error-pick-a-v-bit-or-aae50c = выберите V-образную или сферическую фрезу: для фаски нужен наклонный кончик
error-set-the-angle-between-0-88df37 = задайте угол от 0° до 90°
error-set-start-depth-and-overcut-010ff8 = задайте начальную глубину и перерез 0 или больше

## commands/circular-copy/src/lib.rs
error-set-items-to-2-or-6a2708 = задайте 2 или больше элементов: оригинал и хотя бы одна копия
error-set-the-angle-to-something-c22429 = задайте угол, отличный от 0
error-set-the-total-angle-to-524a51 = задайте общий угол 360° или меньше

## commands/close-extend-vector/src/lib.rs
error-select-open-vectors-to-close-ffdbe0 = выделите открытые векторы, которые нужно замкнуть
error-no-selected-vector-s-ends-8930ce = концы выделенных векторов не укладываются в максимальный зазор. Увеличьте максимальный зазор или сдвиньте концы ближе
error-set-max-gap-to-0-eb0718 = задайте максимальный зазор 0 или больше

## commands/close-with-line/src/lib.rs
error-that-vector-is-already-closed-ce668c = этот вектор уже замкнут. Выделите открытый вектор, чтобы замкнуть его

## commands/component-boundary/src/lib.rs
error-select-components-to-draw-a-76f3a1 = выделите компоненты, вокруг которых нужно нарисовать границу
error-the-components-have-no-shape-22e869 = у компонентов нет формы. Выделите компоненты, у которых есть высота

## commands/component-properties/src/lib.rs
error-set-shape-height-to-0-03822c = задайте высоту формы 0 или больше

## commands/construction-lines/src/lib.rs
error-the-selected-vectors-are-construction-8e3909 = выделенные векторы уже являются вспомогательными линиями. Выделите сплошную линию, чтобы сделать её вспомогательной
error-the-selected-vectors-are-real-01e0a3 = выделенные векторы уже являются обычными линиями. Выделите пунктирную вспомогательную линию, чтобы сделать её обычной

## commands/convert-text-to-curves/src/lib.rs
error-select-the-text-to-convert-860efd = выделите текст для преобразования

## commands/copy-along-vectors/src/lib.rs
error-select-the-vectors-to-copy-faea24 = выделите векторы, вдоль которых нужно копировать окружности
error-select-the-object-to-copy-932e59 = сначала выделите объект для копирования, затем векторы, вдоль которых его копировать
error-a-vector-to-copy-along-e2e22d = у вектора, вдоль которого нужно копировать, нулевая длина. Нарисуйте его линией или кривой
error-set-copies-to-1-or-6f49bc = задайте 1 или больше копий

## commands/copy-to-other-side/src/lib.rs
error-only-a-double-sided-job-79ab7f = другая сторона есть только у двустороннего задания. Сначала сделайте задание двусторонним в Настройке задания

## commands/copy-toolpath-to-sheet/src/lib.rs
error-pick-another-sheet-to-copy-467757 = выберите другой лист, на который нужно скопировать траекторию

## commands/create-component-from-bitmap/src/lib.rs
error-import-the-picture-first-555987 = сначала импортируйте изображение

## commands/create-component-from-visible-model/src/lib.rs
error-there-is-no-visible-model-e61039 = нет видимой модели. Покажите компонент или создайте его инструментом моделирования

## commands/create-fillets/src/lib.rs
error-select-vectors-whose-corners-to-f332bc = выделите векторы, углы которых нужно скруглить
error-the-selected-vectors-have-no-aa723b = у выделенных векторов нет острых углов такого типа для скругления. Выберите другой тип угла или выделите другие векторы
error-the-radius-is-too-large-4ea300 = радиус слишком большой для всех углов: попробуйте меньше
error-no-fillet-fits-that-corner-854c0e = в этот угол не помещается скругление. Попробуйте радиус поменьше
error-only-a-fillet-between-two-970051 = убрать можно только скругление между двумя прямыми сторонами. Щёлкните по скруглению между двумя прямыми сторонами
error-click-on-a-sharp-corner-e64550 = щёлкните по острому углу, чтобы скруглить его, или по скруглению, чтобы убрать его

## commands/create-shape/src/lib.rs
error-select-an-open-vector-for-97abab = выделите вместе с замкнутыми векторами открытый вектор для своего профиля
error-select-the-closed-vectors-to-d6185a = выделите замкнутые векторы для формования
error-set-the-angle-between-1-670798 = задайте угол от 1° до 89°
error-a-flat-shape-needs-a-e79168 = плоской форме нужна высота. Задайте высоту больше 0

## commands/crop-bitmap/src/lib.rs
error-select-the-picture-to-crop-15cc13 = сначала выделите изображение для обрезки
error-select-closed-vectors-to-crop-a83e9c = выделите замкнутые векторы, по которым нужно обрезать изображение
error-the-vectors-do-not-cover-f77de5 = векторы не перекрывают изображение. Переместите замкнутый вектор на ту часть, которую нужно оставить

## commands/cross-section/src/lib.rs
error-there-is-no-visible-model-d7d297 = нет видимой модели, через которую можно провести сечение. Покажите компонент или создайте его инструментом моделирования
error-drag-a-line-across-the-6714ed = протяните линию через модель

## commands/cut-vector/src/lib.rs
error-cut-between-the-ends-of-29adc5 = разрезайте между концами открытого вектора

## commands/decorative-relief/src/lib.rs
error-the-area-is-too-small-476bc9 = область слишком мала для узора. Уменьшите размер узора или нарисуйте область побольше

## commands/delete-items/src/lib.rs
error-only-layers-can-be-empty-a5ea40 = пустыми бывают только слои. Выберите «Слои», чтобы удалить пустые

## commands/delete-layer/src/lib.rs
error-a-job-keeps-at-least-af5f60 = в задании должен остаться хотя бы один слой. Добавьте другой слой, прежде чем удалять этот

## commands/delete-level/src/lib.rs
error-a-job-keeps-at-least-0df7c2 = в задании должен остаться хотя бы один уровень. Добавьте другой уровень, прежде чем удалять этот

## commands/delete-point/src/lib.rs
error-the-vector-needs-the-nodes-1a5aa0 = вектору нужны оставшиеся у него узлы. Удалите весь вектор целиком

## commands/delete-sheet/src/lib.rs
error-a-job-keeps-at-least-caf928 = в задании должен остаться хотя бы один лист. Добавьте другой лист, прежде чем удалять этот

## commands/dimensions/src/lib.rs
error-drag-from-an-arc-or-41a29d = протяните от дуги или окружности туда, где будет её радиус
error-drag-between-two-different-points-4a070f = протяните между двумя разными точками
error-drag-from-the-corner-to-11cec8 = протяните от угла туда, где будут дуга и её текст

## commands/distort-object/src/lib.rs
error-pick-the-envelope-last-a-ae96bd = выберите оболочку последней: замкнутый вектор с узлом в каждом углу
error-pick-the-curve-to-distort-fe73bb = последней выберите кривую, вдоль которой искажать: открытый вектор
error-vectors-and-components-cannot-be-97153d = векторы и компоненты нельзя искажать вместе. Выделите только векторы или только компоненты
error-the-objects-have-no-width-b2d7c1 = у объектов нет ширины или высоты. Выделите объекты, которые не являются плоскими линиями

## commands/drag-knife-toolpath/src/lib.rs
error-select-the-vectors-to-cut-d72baf = выделите векторы для резки
error-set-the-pressure-between-1-48d2a2 = задайте давление от 1 до 100% хода пружины
error-set-the-swivel-pressure-less-22bab7 = задайте давление поворота меньше давления резки
error-set-passes-to-1-or-165a48 = задайте 1 или больше проходов

## commands/draw-arc/src/lib.rs
error-start-and-end-angle-must-5cbd4e = начальный и конечный углы должны различаться (для полной окружности используйте «Нарисовать окружность»)

## commands/draw-curve/src/lib.rs
error-a-curve-needs-at-least-cb3b88 = для кривой нужно не меньше 2 точек (3, чтобы замкнуть)

## commands/draw-freehand/src/lib.rs
error-drag-to-draw-5dd4fa = протяните, чтобы рисовать

## commands/draw-polygon/src/lib.rs
error-set-sides-to-3-or-0cf077 = задайте 3 или больше сторон

## commands/draw-polyline/src/lib.rs
error-click-at-least-2-points-b2eb0e = щёлкните хотя бы в 2 точках, чтобы нарисовать линию
error-click-at-least-3-points-bf69be = щёлкните хотя бы в 3 точках, чтобы нарисовать замкнутую фигуру

## commands/draw-star/src/lib.rs
error-set-points-to-3-or-79da98 = задайте 3 или больше лучей
error-set-inner-radius-smaller-than-9c2249 = задайте внутренний радиус меньше внешнего

## commands/draw-text/src/lib.rs
error-select-a-block-of-text-063370 = выделите блок текста, который нужно разбить на строки
error-text-in-a-box-is-b28b90 = инструмент «Текст в рамке» создаёт один блок: отдельных строк у него нет. Выделите текст, нарисованный строками
error-the-text-is-one-line-7d07f0 = текст уже в одну строку. Выделите текст из нескольких строк

## commands/drill-bank-toolpath/src/lib.rs
error-select-the-circles-to-drill-aef85e = выделите окружности для сверления: каждая сверлится в центре сверлом своего размера
error-set-the-dwell-and-retract-f8e99b = задайте выдержку и зазор отвода 0 или больше

## commands/drilling-toolpath/src/lib.rs
error-select-closed-vectors-to-drill-0aaa3a = выделите замкнутые векторы, которые нужно просверлить в центрах

## commands/edit-picture/src/lib.rs
error-set-gamma-above-0-7b48e3 = задайте гамму больше 0

## commands/embellish/src/lib.rs
error-select-a-vector-to-embellish-5fa8ce = выделите вектор для украшения: замкнутая фигура получает орнамент, подогнанный под неё, а открытая линия — орнамент вдоль неё

## commands/emboss/src/lib.rs
error-select-a-component-to-emboss-c0e89d = выделите компонент для тиснения
error-set-scale-height-between-1-ead544 = задайте масштаб высоты от 1 до 50 %

## commands/engrave-part-names/src/lib.rs
error-there-are-no-parts-on-ebb394 = на этом листе нет деталей, которым можно дать имена. Сначала нарисуйте или импортируйте замкнутые фигуры

## commands/extrude-and-weave/src/lib.rs
error-a-drive-rail-is-a-55cfeb = направляющая состоит из одной точки. Нарисуйте каждую направляющую линией или кривой
error-the-cross-section-makes-no-736b5a = сечение не образует формы. Нарисуйте его открытой кривой, которая поднимается от линии между её концами
error-select-the-component-to-extrude-c9b530 = выделите компонент, который нужно вытянуть вдоль направляющих
error-select-the-drive-rail-s-5ac698 = выделите направляющую (или несколько), затем одно или несколько открытых сечений
error-the-cross-sections-must-be-85aa97 = сечения должны быть открытыми
error-the-cross-section-starts-and-0858e9 = сечение начинается и заканчивается в одной точке. Нарисуйте его так, чтобы концы были разнесены

## commands/fit-curves-to-vectors/src/lib.rs
error-select-vectors-to-fit-curves-ea3da4 = выделите векторы, к которым нужно подогнать кривые

## commands/fluting-toolpath/src/lib.rs
error-select-the-vectors-to-flute-a28ab5 = выделите векторы для каннелюр
error-set-start-depth-to-0-1f86ed = задайте начальную глубину 0 или больше
error-set-ramp-above-0-and-4435c0 = задайте % врезания по наклонной больше 0 и меньше 100

## commands/generate-border/src/lib.rs
error-describe-the-border-under-details-f250bd = опишите рамку в разделе «Подробности» или выберите стиль
error-set-the-depth-curve-between-5813f0 = задайте кривую глубины от 0.2 до 5

## commands/group-components/src/lib.rs
error-select-two-or-more-components-a3c21b = выделите два или больше компонентов для группировки
error-components-on-different-levels-cannot-b38d6c = компоненты на разных уровнях нельзя сгруппировать. Сначала переместите их на один уровень

## commands/group/src/lib.rs
error-select-at-least-two-objects-697425 = выделите хотя бы два объекта для группировки

## commands/import-toolpaths/src/lib.rs
error-pick-the-g-code-file-f0d13d = выберите файл G-code для импорта (Файл ▸ Импорт ▸ Импорт траекторий…)
error-the-file-has-no-moves-33d53c = в файле нет перемещений: это точно G-code?

## commands/inlay-toolpath/src/lib.rs
error-select-the-closed-vectors-to-57c730 = выделите замкнутые векторы для инкрустации
error-set-the-step-depth-less-fdcedd = задайте глубину ступени меньше глубины реза

## commands/insert-point/src/lib.rs
error-there-is-no-span-there-fa0e89 = здесь нет участка вектора. Щёлкните по линии вектора

## commands/interactive-vector-trim/src/lib.rs
error-the-drag-crosses-no-vector-3934cf = протянутая линия не пересекает ни одного вектора для обрезки
error-click-on-the-piece-of-a9c7cb = щёлкните по части вектора, которую нужно обрезать
error-nothing-crosses-that-vector-click-372aea = этот вектор ничто не пересекает. Щёлкните по части между двумя пересечениями

## commands/job-setup/src/lib.rs
error-set-plunge-z2-between-0-b43db3 = задайте врезание (Z2) от 0 до безопасного зазора (Z1)
error-set-centre-stock-diameter-less-11121b = задайте диаметр центральной части меньше диаметра материала или 0, чтобы не оставлять центральную часть

## commands/join-close-vectors/src/lib.rs
error-select-one-open-vector-to-5934a3 = выделите один открытый вектор, чтобы замкнуть его, или два, чтобы соединить

## commands/join-open-ends/src/lib.rs
error-click-the-end-of-an-e8b961 = щёлкните по концу открытого вектора (на каждом есть кружок), затем по концу, с которым его соединить
error-these-ends-never-meet-if-8d3ca1 = эти концы не встретятся при продолжении: они направлены в разные стороны или параллельно. Соедините их прямой линией или дугой

## commands/join-vectors/src/lib.rs
error-no-open-vector-ends-are-20fc85 = нет концов открытых векторов в пределах допуска. Увеличьте допуск или сдвиньте концы ближе друг к другу

## commands/keep-out-zones/src/lib.rs
error-select-closed-vectors-to-make-16862e = выделите замкнутые векторы, из которых нужно сделать зоны

## commands/keyhole-toolpath/src/lib.rs
error-select-circles-where-the-slots-a2bb48 = выделите окружности в местах, где будут пазы

## commands/laser-cut-fill-toolpath/src/lib.rs
error-select-the-closed-vectors-to-9255ad = выделите замкнутые векторы для резки или заливки
error-the-kerf-leaves-nothing-to-7e6dda = из-за ширины реза внутри фигур нечего резать. Уменьшите ширину реза
error-set-power-to-more-than-2d5d46 = задайте мощность больше 0 и не больше 100 %

## commands/laser-picture-toolpath/src/lib.rs
error-select-the-vectors-to-burn-cbeec9 = выделите векторы, вдоль которых нужно выжигать
error-import-the-picture-to-burn-9e83dd = сначала импортируйте изображение для выжигания
error-the-selected-vectors-don-t-d0b602 = выделенные векторы не пересекают изображение: переместите их на изображение или выберите «Растр» или «Штриховка»

## commands/level-clipping/src/lib.rs
error-select-the-closed-vectors-to-2cb8e3 = выделите замкнутые векторы, по которым нужно обрезать уровень

## commands/level-properties/src/lib.rs
error-wrapping-is-for-rotary-jobs-b5933c = обёртывание доступно только для заданий с поворотной осью. Сначала включите поворотную ось в Настройке задания

## commands/make-start-point/src/lib.rs
error-only-a-closed-vector-s-db4238 = перемещать начало можно только у замкнутого вектора

## commands/merge-visible-layers/src/lib.rs
error-there-is-no-other-visible-5259f7 = нет другого видимого слоя для объединения. Покажите слои, которые нужно объединить

## commands/merged-toolpath/src/lib.rs
error-show-the-toolpaths-to-merge-ed1e3d = покажите в списке траекторий те траектории, которые нужно объединить

## commands/mirror/src/lib.rs
error-the-vector-picked-last-has-829062 = у последнего выбранного вектора оба конца совпадают. Выберите прямую линию, относительно которой отражать
error-pick-the-objects-then-last-d63646 = выберите объекты, а последней — линию, относительно которой их отразить

## commands/moulding-toolpath/src/lib.rs
error-select-the-drive-rails-and-702bd8 = выделите направляющие и открытый профиль сечения
error-select-a-drive-rail-as-68d510 = выделите не только профиль, но и направляющую

## commands/move-node/src/lib.rs
error-there-is-no-handle-there-bf9c70 = здесь нет маркера. Сначала сделайте участок кривой: щёлкните по нему правой кнопкой и выберите «Преобразовать участок в кривую Безье»

## commands/move-nodes/src/lib.rs
error-pick-the-nodes-to-move-1cfce4 = выберите узлы для перемещения

## commands/name-part/src/lib.rs
error-select-a-closed-shape-to-31d51f = выделите замкнутую фигуру, чтобы дать ей имя. Открытые линии не являются деталями

## commands/nest-parts/src/lib.rs
error-no-part-fits-on-the-ce1f55 = ни одна деталь не помещается на материале. Увеличьте материал в Настройке задания или уменьшите детали или отступ от края

## commands/offset-model/src/lib.rs
error-there-is-no-visible-model-cb169d = нет видимой модели для смещения. Покажите компонент или создайте его инструментом моделирования
error-set-distance-to-something-other-3dba34 = задайте расстояние, отличное от 0

## commands/offset-vectors/src/lib.rs
error-select-closed-vectors-to-offset-aa8f97 = выделите замкнутые векторы для смещения
error-the-inward-offset-left-nothing-e85bd1 = после смещения внутрь ничего не осталось. Задайте меньшее расстояние

## commands/orientate-3d-model/src/lib.rs
error-nothing-of-the-model-is-d299a5 = никакая часть модели не находится выше нулевой плоскости. Поднимите модель или опустите нулевую плоскость
error-this-file-has-no-triangles-d9e47e = в этом файле нет треугольников. Экспортируйте его заново из программы, в которой он создан
error-the-model-has-no-area-8b9bf7 = у модели нет площади при виде сверху. Поверните её широкой стороной вверх
error-the-model-has-no-length-52a2fb = у модели нет длины или ширины. Поверните её широкой стороной вверх
error-choose-an-stl-file-3fb278 = выберите файл STL

## commands/overlap-intersect/src/lib.rs
error-select-at-least-two-closed-abcd9b = выделите хотя бы два замкнутых вектора для пересечения
error-the-selected-vectors-do-not-252152 = выделенные векторы не перекрываются. Переместите их так, чтобы они перекрывались, и повторите

## commands/photo-vcarve-toolpath/src/lib.rs
error-import-the-picture-to-carve-a8660f = сначала импортируйте изображение для гравировки
error-select-the-vectors-to-carve-3dfdd9 = выделите векторы, вдоль которых нужно гравировать
error-pick-a-v-bit-photo-e57f19 = выберите V-образную фрезу: она нужна для V-гравировки фото

## commands/plate-production/src/lib.rs
error-select-the-vectors-of-the-860690 = выделите векторы основного шаблона
error-a-plate-does-not-fit-21092c = табличка не помещается на материале внутри полей. Уменьшите табличку или поля
error-set-plates-to-1-or-7b1884 = задайте 1 или больше табличек

## commands/pocket-toolpath/src/lib.rs
error-select-the-closed-vectors-to-a95053 = выделите замкнутые векторы для выборки кармана
error-the-pocket-could-not-be-79a780 = не удалось оценить время обработки кармана

## commands/prism-toolpath/src/lib.rs
error-select-the-closed-vectors-to-0b80a1 = выделите замкнутые векторы для призматической резьбы
error-pick-a-v-bit-prism-440f7f = выберите V-образную фрезу: она нужна для призматической резьбы

## commands/profile-toolpath/src/lib.rs
error-select-the-vectors-to-profile-ec4f8a = выделите векторы для профильной обработки
error-click-on-the-vectors-in-cba6b9 = щёлкните по векторам в 2D-виде, чтобы расставить перемычки
error-cut-outside-instead-190532 = вместо этого режьте снаружи («Снаружи»)

## commands/quick-engrave-toolpath/src/lib.rs
error-select-the-closed-vectors-to-b7f734 = выделите замкнутые векторы для заливки
error-select-the-vectors-to-engrave-cedd97 = выделите векторы для гравировки

## commands/quick-trace/src/lib.rs
error-import-a-bitmap-to-trace-e1f9ce = импортируйте растровое изображение для трассировки

## commands/relief-from-photo/src/lib.rs
error-import-the-photo-first-539d35 = сначала импортируйте фото
error-the-picture-is-empty-pick-827719 = изображение пустое. Выберите другое изображение
error-this-photo-has-no-depth-4a83cc = у этого фото ещё нет глубины: сначала примените к нему «Оценить глубину» или выберите «Изображение» в поле «Глубина из», если изображение уже является картой глубины
error-set-the-lift-to-0-0d2e4c = задайте подъём 0 или больше и меньше максимальной высоты
error-set-the-soft-edge-and-058804 = задайте мягкий край и основание 0 или больше
error-pick-the-bit-you-will-ab2118 = выберите фрезу, которой будете резать: «Выбрать…» в разделе «Инструмент», или снимите флажок «Только то, что может вырезать фреза»

## commands/reorder-component/src/lib.rs
error-it-is-already-at-the-ff9a66 = он уже на самом верху своего уровня
error-it-is-already-at-the-7083ca = он уже в самом низу своего уровня

## commands/reorder-layer/src/lib.rs
error-it-is-already-at-the-da6de7 = он уже в начале списка
error-it-is-already-at-the-096963 = он уже в конце списка

## commands/reorder-level/src/lib.rs
error-it-is-already-at-the-472725 = он уже в конце списка

## commands/reorder-toolpath/src/lib.rs
error-the-new-order-must-name-493f4f = в новом порядке каждая траектория должна быть указана ровно один раз

## commands/replace-below/src/lib.rs
error-select-a-component-cd9375 = выделите компонент
error-that-would-remove-the-whole-6153f1 = это удалит весь компонент. Задайте меньшую высоту, ниже которой заменять

## commands/reset-inside-selected-vectors/src/lib.rs
error-select-the-closed-vectors-to-a33d38 = выделите замкнутые векторы, внутри которых нужно сбросить

## commands/reset-outside-selected-vectors/src/lib.rs
error-select-the-closed-vectors-to-7c823b = выделите замкнутые векторы, снаружи которых нужно сбросить

## commands/rotary-details-toolpath/src/lib.rs
error-this-toolpath-turns-the-material-f20ce9 = эта траектория поворачивает материал: включите поворотную ось для задания в Настройке задания

## commands/rotary-finishing-toolpath/src/lib.rs
error-no-room-for-the-registration-29c30f = нет места для метки совмещения вдали от патрона и задней бабки, поэтому её нет: перед началом проведите карандашом линию вдоль материала при A=0 или отодвиньте «Конец» от задней бабки

## commands/rotary-lathe-toolpath/src/lib.rs
error-the-lathe-spins-the-material-5ba114 = токарная обработка вращает материал: включите поворотную ось для задания в Настройке задания
error-the-bit-s-stepover-must-7f947e = шаг фрезы должен быть больше 0% и не больше 100%: задайте его в Библиотеке фрез
error-part-off-s-start-is-e8e2fe = параметр «Начало» отрезки находится за пределом обработки: переместите «Начало» туда, куда предел доходит вдоль детали, или выберите «Материал» в качестве предела

## commands/rounding-toolpath/src/lib.rs
error-rounding-needs-a-rotary-job-d51669 = для скругления нужно задание с поворотной осью: настройте его в Настройке задания
error-the-blank-is-no-bigger-86dd90 = заготовка не больше готового цилиндра. Увеличьте заготовку или уменьшите припуск
error-the-optimized-raster-is-for-42e540 = оптимизированный растр предназначен для квадратных заготовок. Выберите другую стратегию

## commands/scale-model-z-height/src/lib.rs
error-the-model-is-flat-so-cc4772 = модель плоская, поэтому масштабировать нечего. Сначала придайте ей высоту инструментом моделирования

## commands/sculpt/src/lib.rs
error-there-is-no-component-to-a07711 = нет компонента для лепки: сначала создайте его
error-the-brush-has-no-height-a8d16c = у кисти нет высоты, чтобы давить
error-drag-over-the-component-to-2db6af = проведите по компоненту, чтобы лепить

## commands/segment-3d-model/src/lib.rs
error-the-plane-misses-the-model-e8c990 = плоскость не проходит через модель: переместите её внутрь
error-set-the-plane-between-0-fbf26f = задайте плоскость от 0 до 100 %

## commands/select/src/lib.rs
error-select-a-closed-shape-first-19fdd7 = сначала выделите замкнутую фигуру, затем выберите «Выделить векторы внутри», чтобы добавить всё, что внутри неё (или просто щёлкните внутри фигуры)

## commands/send-to-sheet/src/lib.rs
error-the-vectors-are-on-that-3cb610 = векторы уже на этом листе. Выберите другой лист

## commands/set-size/src/lib.rs
error-the-selection-has-no-area-5845c1 = у выделения нет площади для изменения размера. Выделите объекты, которые не являются плоскими линиями

## commands/sketch-carving-toolpath/src/lib.rs
error-import-the-picture-to-sketch-d3e3ec = сначала импортируйте изображение для эскиза
error-there-is-no-3d-model-a9e397 = нет 3D-модели для эскиза. Сначала создайте её инструментом моделирования
error-the-sketch-has-no-lines-7d6256 = в эскизе нет линий. Используйте линию потолще
error-select-the-closed-boundary-vectors-024190 = выделите замкнутые векторы границы

## commands/slice/src/lib.rs
error-there-is-no-visible-model-15fb46 = нет видимой модели для нарезки. Покажите компонент или создайте его инструментом моделирования
error-the-model-has-no-thickness-7e455e = у модели нет толщины выше нулевой плоскости. Покажите компонент, у которого есть высота
error-make-at-least-one-slice-39eb5b = сделайте хотя бы один срез

## commands/smooth-point/src/lib.rs
error-the-end-of-an-open-b80809 = конец открытого вектора не с чем сгладить

## commands/smooth-selected-components/src/lib.rs
error-there-are-no-components-to-4f721b = нет компонентов для сглаживания. Сначала создайте компонент инструментом моделирования

## commands/split/src/lib.rs
error-select-a-component-and-one-e5c734 = выделите компонент и один вектор, по которому его разделить
error-select-a-closed-vector-to-446226 = выделите замкнутый вектор, которым нужно разделить
error-select-a-component-to-split-e12616 = выделите компонент для разделения
error-the-vector-does-not-cross-405c08 = вектор не пересекает компонент. Переместите его так, чтобы он пересекал

## commands/subtract-vectors/src/lib.rs
error-select-at-least-two-closed-8a37fc = выделите хотя бы два замкнутых вектора для вычитания

## commands/surfacing-toolpath/src/lib.rs
error-set-the-bit-s-stepover-b4f207 = задайте шаг фрезы от 1% до 100% её ширины («Изменить…» в разделе «Фреза»)
error-there-is-nothing-to-flatten-a592d9 = нечего выравнивать
error-enter-the-spoilboard-s-width-f2c7df = введите ширину и высоту жертвенного стола (задайте размер стола станка в Станок ▸ Станки, и они заполнятся сами)
error-select-the-closed-outline-of-8132c8 = выделите замкнутый контур того, что нужно выровнять, или выровняйте весь материал

## commands/text-on-a-curve/src/lib.rs
error-select-a-single-line-of-a3b429 = выделите одну строку текста и кривую, вдоль которой её расположить

## commands/texture-area/src/lib.rs
error-select-the-component-to-tile-46a104 = выделите компонент-плитку в дереве компонентов последним, после замкнутых векторов для заполнения (если их нет — всё задание), потому что щелчок по вектору снимает выделение с компонента; или выберите плитку, затем откройте этот инструмент
error-the-component-used-as-the-09170d = компонент, используемый как плитка, удалён. Выделите другой компонент для плитки
error-check-the-tile-settings-the-c887c4 = проверьте настройки плитки: у плитки должна быть форма, интервал должен оставлять каждой плитке место, и отражений должно быть четыре

## commands/texture-toolpath/src/lib.rs
error-select-the-vectors-to-use-f4f59d = выделите векторы, которые будут узором
error-the-boundary-offset-leaves-nothing-c8cf1e = при таком смещении границы текстурировать нечего. Уменьшите смещение границы
error-not-a-texture-settings-file-ddc043 = это не файл настроек текстуры

## commands/thread-milling-toolpath/src/lib.rs
error-select-the-vectors-to-thread-d37c59 = выделите векторы, вокруг центров которых нужно фрезеровать резьбу
error-set-the-thread-mill-s-42c34f = задайте смещение зуба резьбовой фрезы не меньше половины размера зуба
error-set-the-max-depth-at-5d8a17 = задайте максимальную глубину хотя бы на один шаг резьбы ниже смещения зуба
error-the-thread-mill-is-too-9e0153 = резьбовая фреза слишком большая для этой резьбы. Выберите резьбовую фрезу поменьше

## commands/trace-bitmap/src/lib.rs
error-nothing-to-trace-tick-more-427fe0 = нечего трассировать. Отметьте больше цветов или ослабьте фильтр шума
error-nothing-to-trace-set-another-f93ed0 = нечего трассировать. Задайте другой порог или цвет трассировки
error-this-picture-has-no-colours-fd9e95 = в этом изображении нет цветов. Переключите трассировку на чёрно-белую

## commands/trim-objects/src/lib.rs
error-the-object-picked-last-the-37dae3 = последний выбранный объект, граница, должен быть замкнутым. Выберите последним замкнутый вектор
error-pick-the-objects-to-trim-06a68c = выберите объекты для обрезки, а последней — границу

## commands/turn-and-spin/src/lib.rs
error-select-one-open-vector-as-eb0964 = выделите один открытый вектор в качестве профиля
error-the-profile-makes-no-shape-466e07 = профиль не образует формы. Нарисуйте его открытой кривой, отходящей от прямой линии, например как половину контура вазы
error-the-profile-starts-and-ends-d5e78e = профиль начинается и заканчивается в одной точке. Нарисуйте его так, чтобы концы были разнесены: он вращается вокруг линии между ними
error-the-profile-must-run-to-9ced10 = профиль должен идти вправо от своего левого конца

## commands/two-rail-sweep/src/lib.rs
error-a-rail-is-a-single-b4d73b = направляющая состоит из одной точки. Нарисуйте каждую направляющую линией или кривой
error-the-cross-sections-need-width-ef7866 = сечениям нужна ширина: рисуйте их вертикально, поперёк от конца до конца
error-select-two-rails-then-one-e4aecc = выделите две направляющие, затем одно или несколько открытых сечений

## commands/ungroup/src/lib.rs
error-nothing-selected-is-in-a-d6b845 = ничто из выделенного не входит в группу. Выделите группу, чтобы разгруппировать её

## commands/vcarve-engrave-toolpath/src/lib.rs
error-select-the-closed-vectors-to-6fa22c = выделите замкнутые векторы для гравировки
error-clearance-bits-clear-a-fill-e94c6e = фрезы для выборки выбирают плоское дно заливки: задайте «Резьба» как «Заливка» или уберите их
error-clearance-bits-clear-the-flat-eda53f = фрезы для выборки выбирают плоское дно: задайте глубину плоского дна
error-pick-a-v-bit-v-d942c5 = выберите V-образную фрезу: она нужна для V-гравировки

## commands/vector-boundary/src/lib.rs
error-select-closed-vectors-or-tick-4c14d6 = выделите замкнутые векторы или отметьте «Граница-резинка» для открытых

## commands/vector-selector/src/lib.rs
error-no-vectors-pass-those-filters-a3fb79 = ни один вектор не проходит эти фильтры. Отключите часть фильтров или ослабьте их

## commands/vector-texture/src/lib.rs
error-the-area-is-too-small-e8fad0 = область слишком мала для текстуры. Уменьшите текстуру или нарисуйте область побольше
error-the-pattern-is-too-fine-956c94 = узор слишком мелкий для такой большой области. Увеличьте размер узора или наименьший зазор

## commands/vector-unwrapper/src/lib.rs
error-the-vector-unwrapper-is-for-5771ec = инструмент «Развёртка векторов» работает только с заданиями с поворотной осью. Сначала включите поворотную ось в Настройке задания
error-select-one-closed-vector-the-11b94d = выделите один замкнутый вектор: сечение для развёртки

## commands/vector-validator/src/lib.rs
error-no-zero-length-spans-to-395a5d = в проверенных векторах нет участков нулевой длины для исправления

## commands/weld-vectors/src/lib.rs
error-select-at-least-two-closed-abd570 = выделите хотя бы два замкнутых вектора для сварки

## commands/wrapped-fluting-layout/src/lib.rs
error-this-gadget-lays-out-a-5f7842 = этот гаджет размечает задание с поворотной осью. Сначала включите поворотную ось в Настройке задания
error-the-offsets-leave-nothing-of-bee874 = при таких смещениях от заготовки ничего не остаётся. Уменьшите смещения
error-set-flutes-to-1-or-f57712 = задайте 1 или больше каннелюр

## commands/wrapped-spiral-layout/src/lib.rs
error-set-strands-to-1-or-712767 = задайте 1 или больше прядей, а шаг или интервал больше 0

## core/src/change.rs
error-select-the-objects-to-space-5fd609 = выделите объекты для распределения, затем вектор, внутри которого их распределить
error-select-at-least-three-objects-9a936a = выделите хотя бы три объекта для распределения

## core/src/command.rs
error-select-what-to-work-on-e0e677 = выделите то, с чем нужно работать: щёлкните по вектору в 2D-виде, с Shift — чтобы добавить ещё
error-only-construction-lines-are-selected-bad7b0 = выделены только вспомогательные линии, а траектории никогда не режут по ним. Выделите обычные векторы или щёлкните правой кнопкой по вспомогательной линии и выберите «Сделать обычной линией», чтобы резать по ней
error-type-a-name-cf6c71 = введите имя

## core/src/component.rs
error-select-the-closed-vectors-to-100822 = выделите замкнутые векторы, внутри которых нужно обрабатывать
error-the-machining-boundary-is-empty-5c006d = граница обработки пуста
error-there-is-no-3d-model-29926e = нет 3D-модели для обработки: сначала создайте форму
error-the-model-does-not-fit-b5cfb0 = модель не помещается в материал там, где её располагает Настройка материала. Измените положение в Настройке материала или сделайте модель ниже

## core/src/document.rs
error-the-picture-this-toolpath-was-1d4208 = изображение, из которого создана эта траектория, удалено. Импортируйте его снова и создайте из него новую траекторию

## core/src/locked_face.rs
error-this-toolpath-already-turns-a-03cd29 = эта траектория уже поворачивает ось A во время резки: «Плоско на зафиксированной грани» подходит только для плоских траекторий
error-a-drill-bank-s-holes-bfcbb2 = отверстия сверлильной головки находятся там, где её свёрла: «Плоско на зафиксированной грани» подходит только для траекторий, которые фреза режет сама

## core/src/nodes.rs
error-that-vector-cannot-be-edited-a6e9cd = этот вектор нельзя изменить: его слой скрыт или заблокирован, или вектор удалён. Покажите или разблокируйте его слой или выберите другой вектор
error-there-is-no-node-there-622bc8 = здесь нет узла. Щёлкните по узлу

## core/src/vfont.rs
error-not-a-vectric-font-2925f6 = это не файл векторного шрифта этого формата
error-the-font-shares-an-object-0fe45e = в шрифте есть общий объект; в шрифтах такого быть не должно
error-a-glyph-holds-something-other-005b08 = глиф содержит что-то кроме контуров

## formats/cut-list/src/lib.rs
error-the-job-has-no-parts-b74f41 = в задании нет деталей для списка. Сначала нарисуйте или импортируйте замкнутые фигуры
error-the-cut-list-is-empty-256301 = список раскроя пуст
error-the-cut-list-has-no-a3d745 = в списке раскроя нет деталей (все количества равны 0 или нет строк)

## formats/export-as-3d-clipart/src/lib.rs
error-not-a-clipart-file-this-3f98e8 = это не файл клипарта, который может прочитать эта версия PowerWhittle
error-the-clipart-file-is-damaged-d6ef42 = файл клипарта повреждён
error-there-is-no-component-to-efe33c = нет компонента для экспорта: сначала создайте его

## formats/export-as-grayscale-bitmap/src/lib.rs
error-there-is-no-3d-model-ffdf55 = нет 3D-модели для экспорта
error-the-model-is-too-big-0a64ed = модель слишком большая для растрового изображения. Задайте разрешение пониже

## formats/export-as-stl/src/lib.rs
error-there-is-no-3d-model-698ad7 = нет 3D-модели для экспорта: сначала создайте форму
error-closing-with-the-other-side-f46ed4 = для замыкания с другой стороной нужно двустороннее задание с моделью на другой стороне. Отключите этот параметр или сделайте задание двусторонним в Настройке задания
error-the-job-is-too-small-caffed = задание слишком маленькое для экспорта

## formats/export-dxf/src/lib.rs
error-there-are-no-vectors-to-66410f = нет векторов для экспорта. Сначала нарисуйте или импортируйте векторы

## formats/import-crv3d/src/lib.rs
error-not-a-vectric-job-no-869605 = это не файл задания CRV3D: нет 2D-данных
error-the-job-holds-no-vectors-2f83fc = в задании нет векторов

## formats/import-dxf/src/lib.rs
error-the-dxf-has-no-lines-0ad344 = в DXF нет линий, дуг, окружностей, полилиний, эллипсов или сплайнов. Экспортируйте его заново из программы, в которой он создан

## formats/import-stl/src/lib.rs
error-the-model-has-no-area-4a8078 = у модели нет площади при виде сверху. Экспортируйте её заново из программы, в которой она создана
error-not-an-stl-file-e67a9e = это не файл STL
error-the-stl-file-has-a-762c91 = в файле STL есть повреждённый треугольник

## formats/import-svg/src/lib.rs
error-the-svg-has-no-vector-d67023 = в SVG нет векторных контуров. Экспортируйте его заново, преобразовав фигуры в контуры

## formats/part-labels/src/lib.rs
error-the-job-has-no-parts-e61b41 = в задании нет деталей для подписей. Сначала нарисуйте или импортируйте замкнутые фигуры
